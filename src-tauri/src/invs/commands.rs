//! INVS (SQL Server) Tauri command handlers.

use crate::invs::db::{connect, InvsDbConfig, InvsDbState};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use tiberius::{QueryItem, Row};

// ─── Response Types ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DrugMonthlyValue {
    pub working_code: String,
    pub drug_name: String,
    /// 12 elements in FISCAL order: index 0 = ต.ค. (Oct), 11 = ก.ย. (Sep)
    pub monthly_value: [f64; 12],
    pub total_value: f64,
    /// 1-based fiscal month (1 = ต.ค.)
    pub peak_month: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DrugValueSummary {
    pub working_code: String,
    pub drug_name: String,
    pub total_value: f64,
    pub peak_month: u8,
    pub peak_month_value: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DrugItem {
    pub working_code: String,
    pub drug_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct YearSummary {
    pub total_value: f64,
    pub unique_drug_count: i32,
    pub peak_month: u8,
    pub peak_month_value: f64,
}

// ─── Fiscal Year Helpers ─────────────────────────────────────────────────

fn fiscal_year_range(fy: u16) -> (i32, i32) {
    let start = (fy as i32 - 1) * 10_000 + 1001;
    let end = fy as i32 * 10_000 + 930;
    (start, end)
}

fn cal_to_fiscal_idx(cal_month: i32) -> usize {
    if cal_month >= 10 {
        (cal_month - 10) as usize
    } else {
        (cal_month + 2) as usize
    }
}

// ─── Row Helpers ─────────────────────────────────────────────────────────

fn get_str(row: &Row, idx: usize) -> String {
    row.get::<&str, usize>(idx)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn get_f64(row: &Row, idx: usize) -> f64 {
    if let Some(v) = row.get::<f64, usize>(idx) {
        return v;
    }
    if let Some(v) = row.get::<f32, usize>(idx) {
        return v as f64;
    }
    0.0
}

fn get_i32(row: &Row, idx: usize) -> i32 {
    row.get::<i32, usize>(idx).unwrap_or(0)
}

// ─── Commands ────────────────────────────────────────────────────────────

/// Connect to INVS SQL Server and store client in managed state.
#[tauri::command]
pub async fn invs_connect(
    cfg: InvsDbConfig,
    state: tauri::State<'_, InvsDbState>,
) -> Result<(), String> {
    let client = connect(&cfg).await?;
    let mut guard = state.0.lock().await;
    *guard = Some(client);
    Ok(())
}

/// Return monthly purchase values for a drug in fiscal-year order (ต.ค.–ก.ย.).
#[tauri::command]
pub async fn invs_get_drug_monthly_value(
    year: u16,
    working_code: String,
    state: tauri::State<'_, InvsDbState>,
) -> Result<DrugMonthlyValue, String> {
    let mut guard = state.0.lock().await;
    let client = guard
        .as_mut()
        .ok_or_else(|| "ยังไม่ได้เชื่อมต่อฐานข้อมูล INVS".to_string())?;

    let (start_date, end_date) = fiscal_year_range(year);

    let query = "
        SELECT
            c.WORKING_CODE,
            ISNULL(g.DRUG_NAME, ''),
            MONTH(CAST(CAST(h.RECEIVE_DATE AS VARCHAR(8)) AS DATE)) AS cal_month,
            SUM(c.VALUE) AS total_value
        FROM MS_IVO_C c
        JOIN MS_IVO h ON c.INVOICE_NO = h.INVOICE_NO
        LEFT JOIN DRUG_GN g ON c.WORKING_CODE = g.WORKING_CODE
        WHERE
            c.WORKING_CODE = @P1
            AND h.RECEIVE_DATE >= @P2
            AND h.RECEIVE_DATE <= @P3
        GROUP BY
            c.WORKING_CODE,
            g.DRUG_NAME,
            MONTH(CAST(CAST(h.RECEIVE_DATE AS VARCHAR(8)) AS DATE))
        ORDER BY cal_month
    ";

    let mut stream = client
        .query(query, &[&working_code.as_str(), &start_date, &end_date])
        .await
        .map_err(|e| format!("Query error: {e}"))?;

    let mut monthly_value = [0.0f64; 12];
    let mut drug_name = String::new();

    while let Some(item) = stream.try_next().await.map_err(|e| format!("Row error: {e}"))? {
        if let QueryItem::Row(row) = item {
            if drug_name.is_empty() {
                drug_name = get_str(&row, 1);
            }
            let cal_month = get_i32(&row, 2);
            let value = get_f64(&row, 3);
            if (1..=12).contains(&cal_month) {
                monthly_value[cal_to_fiscal_idx(cal_month)] = value;
            }
        }
    }

    let total_value: f64 = monthly_value.iter().sum();
    let peak_month = monthly_value
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| (i + 1) as u8)
        .unwrap_or(1);

    Ok(DrugMonthlyValue {
        working_code,
        drug_name,
        monthly_value,
        total_value,
        peak_month,
    })
}

/// Return top N drugs ranked by total purchase value for the given fiscal year.
#[tauri::command]
pub async fn invs_get_top_drugs_by_value(
    year: u16,
    limit: u8,
    state: tauri::State<'_, InvsDbState>,
) -> Result<Vec<DrugValueSummary>, String> {
    let mut guard = state.0.lock().await;
    let client = guard
        .as_mut()
        .ok_or_else(|| "ยังไม่ได้เชื่อมต่อฐานข้อมูล INVS".to_string())?;

    let (start_date, end_date) = fiscal_year_range(year);
    let limit_i32 = limit as i32;

    let query = "
        SELECT TOP (@P1)
            c.WORKING_CODE,
            ISNULL(g.DRUG_NAME, ''),
            SUM(c.VALUE) AS total_value
        FROM MS_IVO_C c
        JOIN MS_IVO h ON c.INVOICE_NO = h.INVOICE_NO
        LEFT JOIN DRUG_GN g ON c.WORKING_CODE = g.WORKING_CODE
        WHERE h.RECEIVE_DATE >= @P2
          AND h.RECEIVE_DATE <= @P3
        GROUP BY c.WORKING_CODE, g.DRUG_NAME
        ORDER BY total_value DESC
    ";

    let mut stream = client
        .query(query, &[&limit_i32, &start_date, &end_date])
        .await
        .map_err(|e| format!("Query error: {e}"))?;

    let mut results: Vec<DrugValueSummary> = Vec::new();

    while let Some(item) = stream.try_next().await.map_err(|e| format!("Row error: {e}"))? {
        if let QueryItem::Row(row) = item {
            results.push(DrugValueSummary {
                working_code: get_str(&row, 0),
                drug_name: get_str(&row, 1),
                total_value: get_f64(&row, 2),
                peak_month: 0,
                peak_month_value: 0.0,
            });
        }
    }
    drop(stream);

    // Second pass: get peak fiscal month per drug
    for item in results.iter_mut() {
        let peak_query = "
            SELECT TOP 1
                MONTH(CAST(CAST(h.RECEIVE_DATE AS VARCHAR(8)) AS DATE)) AS cal_month,
                SUM(c.VALUE) AS month_value
            FROM MS_IVO_C c
            JOIN MS_IVO h ON c.INVOICE_NO = h.INVOICE_NO
            WHERE
                c.WORKING_CODE = @P1
                AND h.RECEIVE_DATE >= @P2
                AND h.RECEIVE_DATE <= @P3
            GROUP BY MONTH(CAST(CAST(h.RECEIVE_DATE AS VARCHAR(8)) AS DATE))
            ORDER BY month_value DESC
        ";

        let mut peak_stream = client
            .query(peak_query, &[&item.working_code.as_str(), &start_date, &end_date])
            .await
            .map_err(|e| format!("Peak query error: {e}"))?;

        while let Some(peak_item) = peak_stream
            .try_next()
            .await
            .map_err(|e| format!("Peak row error: {e}"))?
        {
            if let QueryItem::Row(peak_row) = peak_item {
                let cal_month = get_i32(&peak_row, 0);
                item.peak_month = (cal_to_fiscal_idx(cal_month) + 1) as u8;
                item.peak_month_value = get_f64(&peak_row, 1);
                break;
            }
        }
    }

    Ok(results)
}

/// Return distinct Thai fiscal years available in MS_IVO (descending).
#[tauri::command]
pub async fn invs_get_available_years(
    state: tauri::State<'_, InvsDbState>,
) -> Result<Vec<u16>, String> {
    let mut guard = state.0.lock().await;
    let client = guard
        .as_mut()
        .ok_or_else(|| "ยังไม่ได้เชื่อมต่อฐานข้อมูล INVS".to_string())?;

    let query = "
        SELECT DISTINCT
            CASE
                WHEN CAST(SUBSTRING(CAST(RECEIVE_DATE AS VARCHAR(8)), 5, 2) AS INT) >= 10
                    THEN CAST(LEFT(CAST(RECEIVE_DATE AS VARCHAR(8)), 4) AS INT) + 1
                ELSE
                    CAST(LEFT(CAST(RECEIVE_DATE AS VARCHAR(8)), 4) AS INT)
            END AS fiscal_year
        FROM MS_IVO
        WHERE RECEIVE_DATE > 0
        ORDER BY fiscal_year DESC
    ";

    let mut stream = client
        .query(query, &[])
        .await
        .map_err(|e| format!("Query error: {e}"))?;

    let mut years: Vec<u16> = Vec::new();

    while let Some(item) = stream.try_next().await.map_err(|e| format!("Row error: {e}"))? {
        if let QueryItem::Row(row) = item {
            let fy = get_i32(&row, 0);
            if fy > 0 {
                years.push(fy as u16);
            }
        }
    }

    Ok(years)
}

/// Drug autocomplete: search by WORKING_CODE prefix or DRUG_NAME contains.
#[tauri::command]
pub async fn invs_get_drug_list(
    search: String,
    state: tauri::State<'_, InvsDbState>,
) -> Result<Vec<DrugItem>, String> {
    let mut guard = state.0.lock().await;
    let client = guard
        .as_mut()
        .ok_or_else(|| "ยังไม่ได้เชื่อมต่อฐานข้อมูล INVS".to_string())?;

    let query = "
        SELECT TOP 30
            g.WORKING_CODE,
            ISNULL(g.DRUG_NAME, '')
        FROM DRUG_GN g
        WHERE
            g.WORKING_CODE LIKE @P1 + '%'
            OR g.DRUG_NAME LIKE '%' + @P1 + '%'
        ORDER BY g.WORKING_CODE
    ";

    let mut stream = client
        .query(query, &[&search.as_str()])
        .await
        .map_err(|e| format!("Query error: {e}"))?;

    let mut drugs: Vec<DrugItem> = Vec::new();

    while let Some(item) = stream.try_next().await.map_err(|e| format!("Row error: {e}"))? {
        if let QueryItem::Row(row) = item {
            drugs.push(DrugItem {
                working_code: get_str(&row, 0),
                drug_name: get_str(&row, 1),
            });
        }
    }

    Ok(drugs)
}

/// Return grand total + unique drug count + peak fiscal month for a fiscal year.
#[tauri::command]
pub async fn invs_get_year_summary(
    year: u16,
    state: tauri::State<'_, InvsDbState>,
) -> Result<YearSummary, String> {
    let mut guard = state.0.lock().await;
    let client = guard
        .as_mut()
        .ok_or_else(|| "ยังไม่ได้เชื่อมต่อฐานข้อมูล INVS".to_string())?;

    let (start_date, end_date) = fiscal_year_range(year);

    let query = "
        SELECT
            MONTH(CAST(CAST(h.RECEIVE_DATE AS VARCHAR(8)) AS DATE)) AS cal_month,
            SUM(c.VALUE) AS total_value
        FROM MS_IVO_C c
        JOIN MS_IVO h ON c.INVOICE_NO = h.INVOICE_NO
        WHERE h.RECEIVE_DATE >= @P1
          AND h.RECEIVE_DATE <= @P2
        GROUP BY MONTH(CAST(CAST(h.RECEIVE_DATE AS VARCHAR(8)) AS DATE))
        ORDER BY cal_month
    ";

    let mut stream = client
        .query(query, &[&start_date, &end_date])
        .await
        .map_err(|e| format!("Query error: {e}"))?;

    let mut fiscal_totals = [0.0f64; 12];
    let mut total_value = 0.0f64;

    while let Some(item) = stream
        .try_next()
        .await
        .map_err(|e| format!("Row error: {e}"))?
    {
        if let QueryItem::Row(row) = item {
            let cal_month = get_i32(&row, 0);
            let month_total = get_f64(&row, 1);
            if (1..=12).contains(&cal_month) {
                fiscal_totals[cal_to_fiscal_idx(cal_month)] = month_total;
                total_value += month_total;
            }
        }
    }
    drop(stream);

    // Unique drug count
    let drug_count_query = "
        SELECT COUNT(DISTINCT c.WORKING_CODE)
        FROM MS_IVO_C c
        JOIN MS_IVO h ON c.INVOICE_NO = h.INVOICE_NO
        WHERE h.RECEIVE_DATE >= @P1
          AND h.RECEIVE_DATE <= @P2
          AND c.VALUE > 0
    ";

    let mut dc_stream = client
        .query(drug_count_query, &[&start_date, &end_date])
        .await
        .map_err(|e| format!("Drug count query error: {e}"))?;

    let mut unique_drug_count = 0i32;
    while let Some(dc_item) = dc_stream
        .try_next()
        .await
        .map_err(|e| format!("Drug count row error: {e}"))?
    {
        if let QueryItem::Row(dc_row) = dc_item {
            unique_drug_count = get_i32(&dc_row, 0);
            break;
        }
    }

    let peak_month = fiscal_totals
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| (i + 1) as u8)
        .unwrap_or(1);

    let peak_month_value = fiscal_totals[(peak_month as usize).saturating_sub(1)];

    Ok(YearSummary {
        total_value,
        unique_drug_count,
        peak_month,
        peak_month_value,
    })
}
