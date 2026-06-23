//! HOSxP (MySQL) Tauri command handlers.

use crate::hosxp::db::{HosxpDbConfig, init_pool, with_pool};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;

// ─── Data structs ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DrugMonthlyData {
    pub icode: String,
    pub drug_name: String,
    /// 12-element vec; index 0 = January.
    pub monthly_qty: Vec<f64>,
    pub total_qty: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DrugSummary {
    pub icode: String,
    pub drug_name: String,
    pub total_qty: f64,
    pub peak_month: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DrugItem {
    pub icode: String,
    pub name: String,
}



// ─── Row helpers ──────────────────────────────────────────────────────────

fn col_string(row: &sqlx::mysql::MySqlRow, col: &str) -> String {
    if let Ok(v) = row.try_get::<String, _>(col) {
        return v;
    }
    if let Ok(Some(v)) = row.try_get::<Option<String>, _>(col) {
        return v;
    }
    String::new()
}

fn col_u32(row: &sqlx::mysql::MySqlRow, col: &str) -> u32 {
    if let Ok(v) = row.try_get::<u32, _>(col) {
        return v;
    }
    if let Ok(v) = row.try_get::<i64, _>(col) {
        return v.clamp(0, u32::MAX as i64) as u32;
    }
    if let Ok(Some(v)) = row.try_get::<Option<u32>, _>(col) {
        return v;
    }
    if let Ok(Some(v)) = row.try_get::<Option<i64>, _>(col) {
        return v.clamp(0, u32::MAX as i64) as u32;
    }
    0
}

fn col_f64(row: &sqlx::mysql::MySqlRow, col: &str) -> f64 {
    if let Ok(v) = row.try_get::<f64, _>(col) {
        return v;
    }
    if let Ok(Some(v)) = row.try_get::<Option<f64>, _>(col) {
        return v;
    }
    if let Ok(v) = row.try_get::<i64, _>(col) {
        return v as f64;
    }
    if let Ok(Some(v)) = row.try_get::<Option<i64>, _>(col) {
        return v as f64;
    }
    if let Ok(v) = row.try_get::<u64, _>(col) {
        return v as f64;
    }
    0.0
}

// ─── Commands ────────────────────────────────────────────────────────────

/// Connect to HOSxP MySQL database.
#[tauri::command]
pub async fn hosxp_connect(config: HosxpDbConfig) -> Result<(), String> {
    init_pool(config).await
}

/// Fetch distinct years present in opitemrece.vstdate, newest first.
#[tauri::command]
pub async fn hosxp_get_available_years() -> Result<Vec<i32>, String> {
    with_pool(|pool| {
        Box::pin(async move {
            let rows = sqlx::query("SELECT DISTINCT YEAR(vstdate) AS yr FROM opitemrece ORDER BY yr DESC")
                .fetch_all(pool)
                .await?;

            let years: Vec<i32> = rows
                .iter()
                .map(|r| col_u32(r, "yr") as i32)
                .filter(|&y| y > 0)
                .collect();

            Ok::<Vec<i32>, sqlx::Error>(years)
        })
    })
    .await
}

/// Fetch top-N drugs by total dispensed quantity in a year.
#[tauri::command]
pub async fn hosxp_get_top_drugs(year: i32, limit: u8) -> Result<Vec<DrugSummary>, String> {
    with_pool(move |pool| {
        Box::pin(async move {
            // Step 1: totals only
            let total_rows = sqlx::query(
                r#"
                SELECT STRAIGHT_JOIN
                    o.icode                           AS icode,
                    COALESCE(d.name, o.icode)         AS drug_name,
                    CAST(SUM(o.qty) AS DOUBLE)        AS total_qty
                FROM opitemrece o
                LEFT JOIN drugitems d ON d.icode = o.icode
                WHERE YEAR(o.vstdate) = ?
                GROUP BY o.icode, d.name
                ORDER BY total_qty DESC
                LIMIT ?
                "#,
            )
            .bind(year)
            .bind(limit as i64)
            .fetch_all(pool)
            .await?;

            if total_rows.is_empty() {
                return Ok::<Vec<DrugSummary>, sqlx::Error>(vec![]);
            }

            let icodes: Vec<String> = total_rows.iter().map(|r| col_string(r, "icode")).collect();

            // Step 2: monthly breakdown for those icodes
            let placeholders = icodes.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let monthly_sql = format!(
                r#"
                SELECT
                    o.icode                      AS icode,
                    MONTH(o.vstdate)             AS month,
                    CAST(SUM(o.qty) AS DOUBLE)   AS qty
                FROM opitemrece o
                WHERE YEAR(o.vstdate) = ?
                  AND o.icode IN ({})
                GROUP BY o.icode, MONTH(o.vstdate)
                "#,
                placeholders
            );

            let mut q = sqlx::query(&monthly_sql).bind(year);
            for ic in &icodes {
                q = q.bind(ic.as_str());
            }
            let monthly_rows = q.fetch_all(pool).await?;

            // Step 3: pivot into per-drug [f64; 12]
            let mut monthly_map: HashMap<String, [f64; 12]> = HashMap::with_capacity(icodes.len());
            for row in &monthly_rows {
                let ic = col_string(row, "icode");
                let mo = col_u32(row, "month");
                let qty = col_f64(row, "qty");
                if (1..=12).contains(&mo) {
                    monthly_map.entry(ic).or_insert([0.0; 12])[(mo - 1) as usize] += qty;
                }
            }

            // Step 4: assemble DrugSummary
            let result: Vec<DrugSummary> = total_rows
                .iter()
                .map(|row| {
                    let ic = col_string(row, "icode");
                    let dn = {
                        let v = col_string(row, "drug_name");
                        if v.is_empty() { ic.clone() } else { v }
                    };
                    let total = col_f64(row, "total_qty");
                    let peak = if let Some(months) = monthly_map.get(&ic) {
                        months
                            .iter()
                            .enumerate()
                            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                            .map(|(i, _)| (i + 1) as u32)
                            .unwrap_or(1)
                    } else {
                        1
                    };
                    DrugSummary {
                        icode: ic,
                        drug_name: dn,
                        total_qty: total,
                        peak_month: peak,
                    }
                })
                .collect();

            Ok::<Vec<DrugSummary>, sqlx::Error>(result)
        })
    })
    .await
}

/// Get monthly dispensing quantities for a specific drug.
#[tauri::command]
pub async fn hosxp_get_drug_monthly_qty(
    year: i32,
    icode: String,
) -> Result<Vec<DrugMonthlyData>, String> {
    with_pool(move |pool| {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT STRAIGHT_JOIN
                    o.icode                      AS icode,
                    COALESCE(d.name, o.icode)    AS drug_name,
                    MONTH(o.vstdate)             AS month,
                    CAST(SUM(o.qty) AS DOUBLE)   AS total_qty
                FROM opitemrece o
                LEFT JOIN drugitems d ON d.icode = o.icode
                WHERE YEAR(o.vstdate) = ?
                  AND o.icode = ?
                GROUP BY o.icode, d.name, MONTH(o.vstdate)
                ORDER BY month
                "#,
            )
            .bind(year)
            .bind(icode.as_str())
            .fetch_all(pool)
            .await?;

            let mut map: HashMap<String, DrugMonthlyData> = HashMap::new();
            for row in &rows {
                let ic = col_string(row, "icode");
                let dn = {
                    let v = col_string(row, "drug_name");
                    if v.is_empty() { ic.clone() } else { v }
                };
                let mo = col_u32(row, "month");
                let qty = col_f64(row, "total_qty");

                let entry = map.entry(ic.clone()).or_insert_with(|| DrugMonthlyData {
                    icode: ic.clone(),
                    drug_name: dn,
                    monthly_qty: vec![0.0; 12],
                    total_qty: 0.0,
                });
                if (1..=12).contains(&mo) {
                    entry.monthly_qty[(mo - 1) as usize] = qty;
                    entry.total_qty += qty;
                }
            }

            let mut result: Vec<DrugMonthlyData> = map.into_values().collect();
            result.sort_by(|a, b| {
                b.total_qty
                    .partial_cmp(&a.total_qty)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            Ok::<Vec<DrugMonthlyData>, sqlx::Error>(result)
        })
    })
    .await
}

/// Search drugitems by icode prefix or name substring.
#[tauri::command]
pub async fn hosxp_get_drug_list(search: String) -> Result<Vec<DrugItem>, String> {
    let escaped = search
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let pattern = format!("%{}%", escaped);

    with_pool(move |pool| {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT
                    icode,
                    COALESCE(name, icode) AS drug_name
                FROM drugitems
                WHERE icode LIKE ?
                   OR name  LIKE ?
                ORDER BY name
                LIMIT 50
                "#,
            )
            .bind(pattern.as_str())
            .bind(pattern.as_str())
            .fetch_all(pool)
            .await?;

            let result: Vec<DrugItem> = rows
                .iter()
                .map(|r| {
                    let icode = col_string(r, "icode");
                    let name = {
                        let v = col_string(r, "drug_name");
                        if v.is_empty() { icode.clone() } else { v }
                    };
                    DrugItem { icode, name }
                })
                .collect();

            Ok::<Vec<DrugItem>, sqlx::Error>(result)
        })
    })
    .await
}
