//! Typed wrappers around the individual Tauri commands.
//!
//! Each function maps the raw `invoke` result/error into a domain type or a
//! [`models::BackendError`].  No UI code calls `invoke` directly.  Argument
//! names use the camelCase JS keys that Tauri v2 bridges to the snake_case
//! Rust parameters (e.g. `workingCode` → `working_code`).

use serde::Serialize;
use wasm_bindgen::JsValue;

use super::tauri::{build_args, invoke};
use crate::models::{
  AutoMatchResult, BackendError, BulkImportResult, DrugMappingStatus, HosxpDbConfig, HosxpDrugItem,
  HosxpDrugMonthly, HosxpYearSummary, InvsDbConfig, InvsDrugItem, InvsDrugMonthlyValue,
  InvsYearSummary, MappingCandidate, MappingRow, MappingStats, ReconcileReport, SettingsFile,
};

fn arg<T: Serialize>(value: &T) -> JsValue {
  serde_wasm_bindgen::to_value(value).unwrap_or(JsValue::UNDEFINED)
}

/// Connect to the HOSxP MySQL database with `config`.
pub async fn hosxp_connect(config: &HosxpDbConfig) -> Result<(), BackendError> {
  let args = build_args(&[("config", &arg(config))]);
  invoke::<()>("hosxp_connect", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Fetch distinct years present in `opitemrece`, newest first.
pub async fn hosxp_get_available_years() -> Result<Vec<i32>, BackendError> {
  invoke::<Vec<i32>>("hosxp_get_available_years", &JsValue::NULL)
    .await
    .map_err(BackendError::from_js)
}

/// Fetch the 12-month dispensing quantities for `icode` in `year`.
pub async fn hosxp_get_drug_monthly_qty(
  year: i32,
  icode: &str,
) -> Result<Vec<HosxpDrugMonthly>, BackendError> {
  let args = build_args(&[
    ("year", &JsValue::from(year)),
    ("icode", &JsValue::from_str(icode)),
  ]);
  invoke::<Vec<HosxpDrugMonthly>>("hosxp_get_drug_monthly_qty", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Search HOSxP drugs by `icode` prefix or name substring.
pub async fn hosxp_get_drug_list(search: &str) -> Result<Vec<HosxpDrugItem>, BackendError> {
  let args = build_args(&[("search", &JsValue::from_str(search))]);
  invoke::<Vec<HosxpDrugItem>>("hosxp_get_drug_list", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Fetch the fiscal-year dispensing grand totals.
pub async fn hosxp_get_year_summary(year: i32) -> Result<HosxpYearSummary, BackendError> {
  let args = build_args(&[("year", &JsValue::from(year))]);
  invoke::<HosxpYearSummary>("hosxp_get_year_summary", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Connect to the INVS SQL Server with `cfg`.
pub async fn invs_connect(cfg: &InvsDbConfig) -> Result<(), BackendError> {
  let args = build_args(&[("cfg", &arg(cfg))]);
  invoke::<()>("invs_connect", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Fetch distinct Thai fiscal years available in `MS_IVO`, descending.
pub async fn invs_get_available_years() -> Result<Vec<i32>, BackendError> {
  invoke::<Vec<i32>>("invs_get_available_years", &JsValue::NULL)
    .await
    .map_err(BackendError::from_js)
}

/// Fetch the 12 fiscal-month purchase values for `working_code` in `year`.
pub async fn invs_get_drug_monthly_value(
  year: i32,
  working_code: &str,
) -> Result<InvsDrugMonthlyValue, BackendError> {
  let args = build_args(&[
    ("year", &JsValue::from(year)),
    ("workingCode", &JsValue::from_str(working_code)),
  ]);
  invoke::<InvsDrugMonthlyValue>("invs_get_drug_monthly_value", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Search INVS drugs by `working_code` prefix or name substring.
pub async fn invs_get_drug_list(search: &str) -> Result<Vec<InvsDrugItem>, BackendError> {
  let args = build_args(&[("search", &JsValue::from_str(search))]);
  invoke::<Vec<InvsDrugItem>>("invs_get_drug_list", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Fetch the grand totals for the fiscal `year`.
pub async fn invs_get_year_summary(year: i32) -> Result<InvsYearSummary, BackendError> {
  let args = build_args(&[("year", &JsValue::from(year))]);
  invoke::<InvsYearSummary>("invs_get_year_summary", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Persist both connection configs (encrypted) to disk.
pub async fn save_settings(
  hosxp: &HosxpDbConfig,
  invs: Option<&InvsDbConfig>,
) -> Result<(), BackendError> {
  let args = build_args(&[("hosxp", &arg(hosxp)), ("invs", &arg(&invs))]);
  invoke::<()>("save_settings", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Load the persisted connection configs (decrypted).
pub async fn load_settings() -> Result<SettingsFile, BackendError> {
  invoke::<SettingsFile>("load_settings", &JsValue::NULL)
    .await
    .map_err(BackendError::from_js)
}

// ─── Drug mapping (Phase 1) ───────────────────────────────────────────

/// Resolved state of a HOSxP drug: mapped / no-INVS / unmapped.
pub async fn mapping_status_by_icode(icode: &str) -> Result<DrugMappingStatus, BackendError> {
  let args = build_args(&[("icode", &JsValue::from_str(icode))]);
  invoke::<DrugMappingStatus>("mapping_status_by_icode", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Resolved state of an INVS drug: mapped / unmapped.
pub async fn mapping_status_by_working_code(
  working_code: &str,
) -> Result<DrugMappingStatus, BackendError> {
  let args = build_args(&[("workingCode", &JsValue::from_str(working_code))]);
  invoke::<DrugMappingStatus>("mapping_status_by_working_code", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Search the HOSxP catalog with mapping state per row.
pub async fn mapping_list_rows(query: &str, limit: u8) -> Result<Vec<MappingRow>, BackendError> {
  let args = build_args(&[
    ("query", &JsValue::from_str(query)),
    ("limit", &JsValue::from(limit)),
  ]);
  invoke::<Vec<MappingRow>>("mapping_list_rows", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Headline counts for the mapping view header.
pub async fn mapping_stats() -> Result<MappingStats, BackendError> {
  invoke::<MappingStats>("mapping_stats", &JsValue::NULL)
    .await
    .map_err(BackendError::from_js)
}

/// Score the top INVS candidates for a HOSxP drug name.
pub async fn mapping_suggest(
  drug_name: &str,
  limit: u8,
) -> Result<Vec<MappingCandidate>, BackendError> {
  let args = build_args(&[
    ("drugName", &JsValue::from_str(drug_name)),
    ("limit", &JsValue::from(limit)),
  ]);
  invoke::<Vec<MappingCandidate>>("mapping_suggest", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Create or update a link (`method`: `auto` | `manual` | `approved`).
pub async fn mapping_set(
  icode: &str,
  drug_name_hosxp: &str,
  working_code: &str,
  drug_name_invs: &str,
  method: &str,
  score: Option<f64>,
) -> Result<(), BackendError> {
  let args = build_args(&[
    ("icode", &JsValue::from_str(icode)),
    ("drugNameHosxp", &JsValue::from_str(drug_name_hosxp)),
    ("workingCode", &JsValue::from_str(working_code)),
    ("drugNameInvs", &JsValue::from_str(drug_name_invs)),
    ("method", &JsValue::from_str(method)),
    (
      "score",
      &serde_wasm_bindgen::to_value(&score).unwrap_or(JsValue::NULL),
    ),
  ]);
  invoke::<()>("mapping_set", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Break a specific link.
pub async fn mapping_remove(icode: &str, working_code: &str) -> Result<(), BackendError> {
  let args = build_args(&[
    ("icode", &JsValue::from_str(icode)),
    ("workingCode", &JsValue::from_str(working_code)),
  ]);
  invoke::<()>("mapping_remove", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Mark a HOSxP drug as having no INVS equivalent.
pub async fn mapping_mark_no_invs(icode: &str, reason: &str) -> Result<(), BackendError> {
  let args = build_args(&[
    ("icode", &JsValue::from_str(icode)),
    ("reason", &JsValue::from_str(reason)),
  ]);
  invoke::<()>("mapping_mark_no_invs", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Clear the "no INVS equivalent" mark.
pub async fn mapping_unmark_no_invs(icode: &str) -> Result<(), BackendError> {
  let args = build_args(&[("icode", &JsValue::from_str(icode))]);
  invoke::<()>("mapping_unmark_no_invs", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Batch auto-match.  `min_score` 0 uses the engine default (0.95).
pub async fn mapping_auto_match(
  query: &str,
  limit: u8,
  min_score: f64,
  dry_run: bool,
) -> Result<AutoMatchResult, BackendError> {
  let args = build_args(&[
    ("query", &JsValue::from_str(query)),
    ("limit", &JsValue::from(limit)),
    ("minScore", &JsValue::from(min_score)),
    ("dryRun", &JsValue::from_bool(dry_run)),
  ]);
  invoke::<AutoMatchResult>("mapping_auto_match", &args)
    .await
    .map_err(BackendError::from_js)
}

/// Preview or apply a pasted mapping CSV.
pub async fn mapping_bulk_import(
  csv_text: &str,
  dry_run: bool,
) -> Result<BulkImportResult, BackendError> {
  let args = build_args(&[
    ("csvText", &JsValue::from_str(csv_text)),
    ("dryRun", &JsValue::from_bool(dry_run)),
  ]);
  invoke::<BulkImportResult>("mapping_bulk_import", &args)
    .await
    .map_err(BackendError::from_js)
}

// ─── Reconciliation (Phase 2) ───────────────────────────────────────

/// Reconcile a mapped HOSxP drug against its INVS counterpart.
pub async fn reconcile_drug(year: i32, icode: &str) -> Result<ReconcileReport, BackendError> {
  let args = build_args(&[
    ("year", &JsValue::from(year)),
    ("icode", &JsValue::from_str(icode)),
  ]);
  invoke::<ReconcileReport>("reconcile_drug", &args)
    .await
    .map_err(BackendError::from_js)
}
