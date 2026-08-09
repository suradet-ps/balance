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
  BackendError, HosxpDbConfig, HosxpDrugItem, HosxpDrugMonthly, InvsDbConfig, InvsDrugItem,
  InvsDrugMonthlyValue, InvsYearSummary, SettingsFile,
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
