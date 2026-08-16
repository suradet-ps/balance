//! Drug-mapping Tauri command handlers (Phase 1).
//!
//! The mapping engine wires three things together: the local store
//! ([`crate::store::StoreState`]), the HOSxP catalog query, and the INVS
//! catalog query.  All *scoring* happens in pure Rust ([`super::normalizer`]);
//! the databases only ever supply candidate names.

use crate::hosxp::db::with_pool;
use crate::invs::db::InvsDbState;
use crate::mapping::normalizer;
use crate::mapping::{bulk, repo};
use crate::store::StoreState;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tiberius::QueryItem;

// ─── Wire types ───────────────────────────────────────────────────────────

/// A full mapping link (both codes + the context it was made in).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MappingLink {
    pub icode: String,
    pub working_code: String,
    pub drug_name_hosxp: String,
    pub drug_name_invs: String,
    pub match_method: String,
    pub match_score: Option<f64>,
}

/// The resolved state of one drug on either side.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DrugMappingStatus {
    /// `mapped` | `no_invs` | `unmapped`
    pub status: String,
    pub link: Option<MappingLink>,
    /// Exclusion reason when `status == "no_invs"`.
    pub reason: Option<String>,
}

/// One HOSxP row in the mapping list view.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MappingRow {
    pub icode: String,
    pub drug_name: String,
    /// `mapped` | `no_invs` | `unmapped`
    pub status: String,
    pub working_code: Option<String>,
    pub no_invs_reason: Option<String>,
}

/// A scored INVS candidate for a HOSxP drug.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MappingCandidate {
    pub working_code: String,
    pub drug_name: String,
    pub score: f64,
}

/// One entry of the batch auto-match result.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AutoMatchPreview {
    pub icode: String,
    pub drug_name: String,
    pub working_code: String,
    pub drug_name_invs: String,
    pub score: f64,
}

/// Batch auto-match result: `to_match` is always filled (the preview);
/// `applied` counts how many were actually written when `dry_run` was false.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AutoMatchResult {
    pub to_match: Vec<AutoMatchPreview>,
    pub applied: usize,
}

/// A row that conflicts with an existing mapping (same icode, other code).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BulkConflict {
    /// 1-based line number in the pasted text.
    pub line: usize,
    pub icode: String,
    pub working_code: String,
    pub existing: String,
}

/// Bulk-import outcome / preview.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BulkImportResult {
    pub total: usize,
    /// Rows that will be / were applied.
    pub added: usize,
    /// Rows that would overwrite a different existing link — never applied
    /// without a review, always listed.
    pub conflicts: Vec<BulkConflict>,
    /// Rows with an empty icode or working_code.
    pub skipped: usize,
    /// Unparsable lines.
    pub errors: Vec<String>,
}

/// Header counts for the mapping view.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MappingStats {
    pub total: u64,
    pub by_method: std::collections::HashMap<String, u64>,
    pub exclusions: u64,
}

// ─── Row helpers (mirror the hosxp/invs command modules) ──────────────────

fn col_string(row: &sqlx::mysql::MySqlRow, col: &str) -> String {
    if let Ok(v) = row.try_get::<String, _>(col) {
        return v;
    }
    if let Ok(Some(v)) = row.try_get::<Option<String>, _>(col) {
        return v;
    }
    String::new()
}

fn get_str(row: &tiberius::Row, idx: usize) -> String {
    row.get::<&str, usize>(idx).unwrap_or("").trim().to_string()
}

fn link_to_wire(
    (icode, working_code, name_h, name_i, method, score): repo::LinkRow,
) -> MappingLink {
    MappingLink {
        icode,
        working_code,
        drug_name_hosxp: name_h,
        drug_name_invs: name_i,
        match_method: method,
        match_score: score,
    }
}

/// Escape LIKE wildcards so the search term is matched literally.
/// `~` is the escape character (usable on both MySQL and SQL Server —
/// unlike `\`, whose meaning depends on the MySQL `NO_BACKSLASH_ESCAPES`
/// mode), so it must be escaped first.
fn escape_like(s: &str) -> String {
    s.replace('~', "~~")
        .replace('%', "~%")
        .replace('_', "~_")
}

/// A short, loose term for the INVS candidate query: the first word, joined
/// with the second when the first is very short, capped at 16 chars.
/// Empty for a blank input — callers must not query with `LIKE '%%'`.
fn suggest_search_term(name: &str) -> String {
    let mut words = name.split_whitespace();
    let first = words.next().unwrap_or("");
    let mut term = first.to_owned();
    if first.chars().count() < 3
        && let Some(second) = words.next()
    {
        term.push(' ');
        term.push_str(second);
    }
    term.chars().take(16).collect()
}

// ─── Status commands ──────────────────────────────────────────────────────

/// Resolved state of a HOSxP drug: mapped / no-INVS / unmapped.
#[tauri::command]
pub async fn mapping_status_by_icode(
    store: tauri::State<'_, StoreState>,
    icode: String,
) -> Result<DrugMappingStatus, String> {
    let conn = store.lock()?;
    if let Some(link) = repo::link_by_icode(&conn, &icode)? {
        return Ok(DrugMappingStatus {
            status: "mapped".to_string(),
            link: Some(link_to_wire(link)),
            reason: None,
        });
    }
    if let Some(reason) = repo::excluded_map(&conn)?.get(&icode) {
        return Ok(DrugMappingStatus {
            status: "no_invs".to_string(),
            link: None,
            reason: Some(reason.clone()),
        });
    }
    Ok(DrugMappingStatus {
        status: "unmapped".to_string(),
        link: None,
        reason: None,
    })
}

/// Resolved state of an INVS drug: mapped / unmapped.
#[tauri::command]
pub async fn mapping_status_by_working_code(
    store: tauri::State<'_, StoreState>,
    working_code: String,
) -> Result<DrugMappingStatus, String> {
    let conn = store.lock()?;
    if let Some(link) = repo::link_by_working_code(&conn, &working_code)? {
        return Ok(DrugMappingStatus {
            status: "mapped".to_string(),
            link: Some(link_to_wire(link)),
            reason: None,
        });
    }
    Ok(DrugMappingStatus {
        status: "unmapped".to_string(),
        link: None,
        reason: None,
    })
}

// ─── List view ────────────────────────────────────────────────────────────

/// Search the HOSxP catalog and enrich each row with its mapping state.
#[tauri::command]
pub async fn mapping_list_rows(
    store: tauri::State<'_, StoreState>,
    query: String,
    limit: u8,
) -> Result<Vec<MappingRow>, String> {
    let limit = limit.clamp(1, 100) as i64;
    let escaped = escape_like(&query);
    let pattern = format!("%{escaped}%");

    let drugs: Vec<(String, String)> = with_pool(move |pool| {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT icode, COALESCE(name, icode) AS drug_name
                FROM drugitems
                WHERE icode LIKE ? ESCAPE '~' OR name LIKE ? ESCAPE '~'
                ORDER BY name
                LIMIT ?
                "#,
            )
            .bind(pattern.as_str())
            .bind(pattern.as_str())
            .bind(limit)
            .fetch_all(pool)
            .await?;

            let out: Vec<(String, String)> = rows
                .iter()
                .map(|r| (col_string(r, "icode"), col_string(r, "drug_name")))
                .collect();
            Ok::<Vec<(String, String)>, sqlx::Error>(out)
        })
    })
    .await?;

    let conn = store.lock()?;
    let icodes: Vec<&str> = drugs.iter().map(|(i, _)| i.as_str()).collect();
    let links = repo::links_for_icodes(&conn, &icodes)?;
    let exclusions = repo::excluded_map(&conn)?;

    Ok(drugs
        .into_iter()
        .map(|(icode, drug_name)| {
            if let Some((working_code, _method)) = links.get(&icode) {
                MappingRow {
                    icode,
                    drug_name,
                    status: "mapped".to_string(),
                    working_code: Some(working_code.clone()),
                    no_invs_reason: None,
                }
            } else if let Some(reason) = exclusions.get(&icode) {
                MappingRow {
                    icode,
                    drug_name,
                    status: "no_invs".to_string(),
                    working_code: None,
                    no_invs_reason: Some(reason.clone()),
                }
            } else {
                MappingRow {
                    icode,
                    drug_name,
                    status: "unmapped".to_string(),
                    working_code: None,
                    no_invs_reason: None,
                }
            }
        })
        .collect())
}

/// Headline counts for the mapping view.
#[tauri::command]
pub async fn mapping_stats(store: tauri::State<'_, StoreState>) -> Result<MappingStats, String> {
    let conn = store.lock()?;
    let (total, by_method, exclusions) = repo::stats(&conn)?;
    Ok(MappingStats {
        total: total.max(0) as u64,
        by_method: by_method
            .into_iter()
            .map(|(k, v)| (k, v.max(0) as u64))
            .collect(),
        exclusions: exclusions.max(0) as u64,
    })
}

// ─── Suggest + match ──────────────────────────────────────────────────────

/// Score the top INVS candidates for a HOSxP drug name.
#[tauri::command]
pub async fn mapping_suggest(
    state: tauri::State<'_, InvsDbState>,
    drug_name: String,
    limit: u8,
) -> Result<Vec<MappingCandidate>, String> {
    fetch_and_score_candidates(state.inner(), &drug_name, limit).await
}

/// The suggest logic, shared by the command and the batch auto-match path.
async fn fetch_and_score_candidates(
    invs: &InvsDbState,
    drug_name: &str,
    limit: u8,
) -> Result<Vec<MappingCandidate>, String> {
    let limit_i32 = limit.clamp(1, 20) as i32;
    let search_term = suggest_search_term(drug_name);
    if search_term.is_empty() {
        // No term → nothing comparable; return no candidates instead of
        // `LIKE '%%'` matching the whole catalog and scoring everything 0.
        return Ok(Vec::new());
    }
    let escaped = escape_like(&search_term);

    let mut guard = invs.0.lock().await;
    let client = guard
        .as_mut()
        .ok_or_else(|| "ยังไม่ได้เชื่อมต่อฐานข้อมูล INVS".to_string())?;

    let query = "
        SELECT TOP (@P1)
            g.WORKING_CODE,
            ISNULL(g.DRUG_NAME, '')
        FROM DRUG_GN g
        WHERE
            g.WORKING_CODE = @P2
            OR g.WORKING_CODE LIKE @P3 + '%' ESCAPE '~'
            OR g.DRUG_NAME LIKE '%' + @P4 + '%' ESCAPE '~'
        ORDER BY
            CASE WHEN g.WORKING_CODE = @P2 THEN 0
                 WHEN g.WORKING_CODE LIKE @P3 + '%' ESCAPE '~' THEN 1
                 ELSE 2 END,
            g.DRUG_NAME
    ";
    let mut stream = client
        .query(
            query,
            &[
                &limit_i32,
                &search_term.as_str(),
                &escaped.as_str(),
                &escaped.as_str(),
            ],
        )
        .await
        .map_err(|e| format!("Query error: {e}"))?;

    let mut candidates: Vec<(String, String)> = Vec::new();
    while let Some(item) = stream
        .try_next()
        .await
        .map_err(|e| format!("Row error: {e}"))?
    {
        if let QueryItem::Row(row) = item {
            candidates.push((get_str(&row, 0), get_str(&row, 1)));
        }
    }
    drop(stream);
    drop(guard); // scoring is pure CPU — release the INVS client first

    // DRUG_GN can hold several name rows per WORKING_CODE; a code must
    // appear once, scored by its best matching name.
    let mut best_by_code: std::collections::HashMap<String, (String, f64)> =
        std::collections::HashMap::new();
    for (working_code, name) in candidates {
        let score = normalizer::similarity(drug_name, &name);
        best_by_code
            .entry(working_code)
            .and_modify(|(n, s)| {
                if score > *s {
                    *n = name.clone();
                    *s = score;
                }
            })
            .or_insert((name, score));
    }

    let mut scored: Vec<MappingCandidate> = best_by_code
        .into_iter()
        .map(|(working_code, (drug_name, score))| MappingCandidate {
            working_code,
            drug_name,
            score,
        })
        .collect();
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.working_code.cmp(&b.working_code))
    });
    scored.truncate(limit.clamp(1, 20) as usize);
    Ok(scored)
}

/// Create or update a link.  `method` is `auto` (machine, score-gated),
/// `manual` (pharmacist forced it) or `approved` (pharmacist confirmed a
/// suggestion).  Any exclusion for the icode is cleared.
#[tauri::command]
pub async fn mapping_set(
    store: tauri::State<'_, StoreState>,
    icode: String,
    drug_name_hosxp: String,
    working_code: String,
    drug_name_invs: String,
    method: String,
    score: Option<f64>,
) -> Result<(), String> {
    if icode.trim().is_empty() || working_code.trim().is_empty() {
        return Err("icode และ working_code ต้องไม่ว่าง".to_string());
    }
    if !matches!(method.as_str(), "auto" | "manual" | "approved") {
        return Err(format!("match method '{method}' is invalid"));
    }
    let mut conn = store.lock()?;
    repo::upsert(
        &mut conn,
        icode.trim(),
        working_code.trim(),
        &drug_name_hosxp,
        &drug_name_invs,
        &method,
        score,
    )
}

/// Break a link.
#[tauri::command]
pub async fn mapping_remove(
    store: tauri::State<'_, StoreState>,
    icode: String,
    working_code: String,
) -> Result<(), String> {
    let conn = store.lock()?;
    repo::remove(&conn, &icode, &working_code)
}

/// Mark a HOSxP drug as having no INVS equivalent (drops its links).
#[tauri::command]
pub async fn mapping_mark_no_invs(
    store: tauri::State<'_, StoreState>,
    icode: String,
    reason: String,
) -> Result<(), String> {
    let mut conn = store.lock()?;
    repo::set_no_invs(&mut conn, &icode, reason.trim())
}

/// Clear the "no INVS equivalent" mark.
#[tauri::command]
pub async fn mapping_unmark_no_invs(
    store: tauri::State<'_, StoreState>,
    icode: String,
) -> Result<(), String> {
    let conn = store.lock()?;
    repo::unset_no_invs(&conn, &icode)
}

// ─── Batch auto-match ─────────────────────────────────────────────────────

/// Score candidates for one HOSxP drug against the INVS catalog and return
/// the best above `min_score` (if any).  Helper shared with the batch path.
async fn best_candidate(
    invs: &InvsDbState,
    drug_name: &str,
    min_score: f64,
) -> Result<Option<MappingCandidate>, String> {
    let candidates = fetch_and_score_candidates(invs, drug_name, 10).await?;
    Ok(candidates.into_iter().find(|c| c.score >= min_score))
}

/// Auto-match every unmapped, non-excluded HOSxP drug in the current list
/// whose best candidate scores at least `min_score`.  A `min_score` of `0`
/// (or below) falls back to the engine's [`normalizer::AUTO_MATCH_THRESHOLD`].
/// `dry_run` returns the preview without writing; the frontend shows it
/// before confirming.
#[tauri::command]
pub async fn mapping_auto_match(
    store: tauri::State<'_, StoreState>,
    invs: tauri::State<'_, InvsDbState>,
    query: String,
    limit: u8,
    min_score: f64,
    dry_run: bool,
) -> Result<AutoMatchResult, String> {
    let min_score = if min_score > 0.0 {
        min_score
    } else {
        normalizer::AUTO_MATCH_THRESHOLD
    };
    let list_limit = limit.clamp(1, 50);
    let rows = mapping_list_rows(store.clone(), query, list_limit).await?;

    let mut to_match: Vec<AutoMatchPreview> = Vec::new();
    for row in rows {
        if row.status != "unmapped" {
            continue;
        }
        let Some(candidate) = best_candidate(invs.inner(), &row.drug_name, min_score).await? else {
            continue;
        };
        to_match.push(AutoMatchPreview {
            icode: row.icode,
            drug_name: row.drug_name,
            working_code: candidate.working_code,
            drug_name_invs: candidate.drug_name,
            score: candidate.score,
        });
    }

    let mut applied = 0usize;
    if !dry_run {
        let mut conn = store.lock()?;
        for m in &to_match {
            repo::upsert(
                &mut conn,
                &m.icode,
                &m.working_code,
                &m.drug_name,
                &m.drug_name_invs,
                "auto",
                Some(m.score),
            )?;
            applied += 1;
        }
    }

    Ok(AutoMatchResult { to_match, applied })
}

// ─── Bulk CSV import ──────────────────────────────────────────────────────

/// Parse the pasted CSV and either preview or apply it.  Conflicting rows
/// (icode already mapped to a *different* working_code) are never written
/// silently: in `dry_run` they are listed, in apply mode they are skipped
/// and listed for the pharmacist to resolve in the list view.
#[tauri::command]
pub async fn mapping_bulk_import(
    store: tauri::State<'_, StoreState>,
    csv_text: String,
    dry_run: bool,
) -> Result<BulkImportResult, String> {
    let (rows, errors) = bulk::parse_bulk_csv(&csv_text);
    let mut result = BulkImportResult {
        total: rows.len(),
        added: 0,
        conflicts: Vec::new(),
        skipped: 0,
        errors,
    };

    let mut conn = store.lock()?;
    for (index, row) in rows.iter().enumerate() {
        let icode = row.icode.trim();
        let working_code = row.working_code.trim();
        if icode.is_empty() || working_code.is_empty() {
            result.skipped += 1;
            continue;
        }
        match repo::link_by_icode(&conn, icode)? {
            Some((_i, existing, _n, _m, _method, _score)) if existing != working_code => {
                result.conflicts.push(BulkConflict {
                    line: index + 1,
                    icode: icode.to_string(),
                    working_code: working_code.to_string(),
                    existing,
                });
            }
            _ => {
                if !dry_run {
                    repo::upsert(
                        &mut conn,
                        icode,
                        working_code,
                        &row.drug_name_hosxp,
                        &row.drug_name_invs,
                        "approved",
                        None,
                    )?;
                }
                result.added += 1;
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{escape_like, suggest_search_term};

    #[test]
    fn escape_like_escapes_wildcards_and_the_escape_char() {
        assert_eq!(escape_like("amox"), "amox");
        assert_eq!(escape_like("50%"), "50~%");
        assert_eq!(escape_like("a_b"), "a~_b");
        assert_eq!(escape_like("100~"), "100~~");
        assert_eq!(escape_like("%_~%"), "~%~_~~~%");
    }

    #[test]
    fn suggest_term_shortens_and_guards_empty() {
        assert_eq!(suggest_search_term("Amoxicillin 500 mg"), "Amoxicillin");
        assert_eq!(suggest_search_term("Ab 500 mg"), "Ab 500");
        assert_eq!(suggest_search_term("a very long name here"), "a very");
        assert!(suggest_search_term("").is_empty());
        assert!(suggest_search_term("   ").is_empty());
    }
}
