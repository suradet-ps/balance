//! Data models shared with the Tauri backend.
//!
//! These types mirror the JSON contract serialised by the command layer in
//! `src-tauri`.  They are intentionally *re-declared* here rather than
//! re-exported from the backend crate: the frontend runs on `wasm32` and must
//! stay lean, and the wire format (not the Rust types) is the real contract.
//! The field names and serde conventions match the backend exactly (both sides
//! use snake_case JSON keys), so (de)serialisation is lossless.

use serde::{Deserialize, Serialize};

// ─── HOSxP (MySQL) types ──────────────────────────────────────────────

/// Summary row from `hosxp_get_top_drugs`.
#[derive(Clone, Debug, Deserialize)]
pub struct HosxpDrugSummary {
  pub icode: String,
  pub drug_name: String,
  pub total_qty: f64,
  pub peak_month: u32,
}

/// 12-month breakdown from `hosxp_get_drug_monthly_qty`
/// (`monthly_qty` index 0 = January, calendar order).
#[derive(Clone, Debug, Deserialize)]
pub struct HosxpDrugMonthly {
  pub icode: String,
  pub drug_name: String,
  pub monthly_qty: Vec<f64>,
  pub total_qty: f64,
}

/// Autocomplete hit from `hosxp_get_drug_list`.
#[derive(Clone, Debug, Deserialize)]
pub struct HosxpDrugItem {
  pub icode: String,
  pub name: String,
}

// ─── INVS (SQL Server) types ──────────────────────────────────────────

/// Summary row from `invs_get_top_drugs_by_value`.
#[derive(Clone, Debug, Deserialize)]
pub struct InvsDrugValueSummary {
  pub working_code: String,
  pub drug_name: String,
  pub total_value: f64,
  pub peak_month: u8,
  pub peak_month_value: f64,
}

/// 12-month breakdown from `invs_get_drug_monthly_value`
/// (`monthly_value` index 0 = ต.ค. — fiscal order).
#[derive(Clone, Debug, Deserialize)]
pub struct InvsDrugMonthlyValue {
  pub working_code: String,
  pub drug_name: String,
  pub monthly_value: Vec<f64>,
  pub total_value: f64,
  pub peak_month: u8,
}

/// Grand totals from `invs_get_year_summary`.
#[derive(Clone, Debug, Deserialize)]
pub struct InvsYearSummary {
  pub total_value: f64,
  pub unique_drug_count: i32,
  pub peak_month: u8,
  pub peak_month_value: f64,
}

/// Autocomplete hit from `invs_get_drug_list`.
#[derive(Clone, Debug, Deserialize)]
pub struct InvsDrugItem {
  pub working_code: String,
  pub drug_name: String,
}

// ─── Connection configs ───────────────────────────────────────────────

/// HOSxP MySQL connection settings (wire shape of `HosxpDbConfig`).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct HosxpDbConfig {
  pub host: String,
  pub port: String,
  pub user: String,
  pub password: String,
  pub database: String,
}

impl Default for HosxpDbConfig {
  fn default() -> Self {
    Self {
      host: "localhost".to_owned(),
      port: "3306".to_owned(),
      user: String::new(),
      password: String::new(),
      database: "hospdb".to_owned(),
    }
  }
}

/// INVS SQL Server connection settings (wire shape of `InvsDbConfig`).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct InvsDbConfig {
  pub host: String,
  pub port: String,
  pub user: String,
  pub password: String,
  pub database: String,
  pub instance: String,
}

impl Default for InvsDbConfig {
  fn default() -> Self {
    Self {
      host: "localhost".to_owned(),
      port: "1433".to_owned(),
      user: String::new(),
      password: String::new(),
      database: "INVS".to_owned(),
      instance: String::new(),
    }
  }
}

/// Persisted settings file shape (`load_settings` result).
#[derive(Clone, Debug, Deserialize)]
pub struct SettingsFile {
  pub hosxp: HosxpDbConfig,
  pub invs: Option<InvsDbConfig>,
}

// ─── Unified search result ────────────────────────────────────────────

/// A drug autocomplete hit from either database, decoded from its wire shape.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum DrugResult {
  Hosxp(HosxpDrugItem),
  Invs(InvsDrugItem),
}

impl DrugResult {
  /// The searchable code: `icode` for HOSxP, `working_code` for INVS.
  #[must_use]
  pub fn code(&self) -> &str {
    match self {
      Self::Hosxp(d) => &d.icode,
      Self::Invs(d) => &d.working_code,
    }
  }

  /// The display name (`name` for HOSxP, `drug_name` for INVS).
  #[must_use]
  pub fn name(&self) -> &str {
    match self {
      Self::Hosxp(d) => &d.name,
      Self::Invs(d) => &d.drug_name,
    }
  }
}

// ─── Backend error ────────────────────────────────────────────────────

/// A rejected Tauri command.  The backend commands return `Result<_, String>`,
/// so the rejection payload is a plain message.
#[derive(Clone, Debug)]
pub struct BackendError {
  /// Human-readable description suitable for display.
  pub message: String,
}

impl BackendError {
  /// Decode a rejected `invoke` payload.
  #[must_use]
  pub fn from_js(raw: wasm_bindgen::JsValue) -> Self {
    Self {
      message: raw.as_string().unwrap_or_else(|| format!("{raw:?}")),
    }
  }
}

// ─── Chart series ─────────────────────────────────────────────────────

/// Which database panel a component belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
  /// HOSxP (MySQL) — quantities, calendar months, purple palette.
  Hosxp,
  /// INVS (SQL Server) — values, fiscal months, green palette.
  Invs,
}

/// Monthly trend data ready for the chart renderer.
#[derive(Clone, Debug)]
pub enum ChartSeries {
  Hosxp(HosxpDrugMonthly),
  Invs(InvsDrugMonthlyValue),
}

impl ChartSeries {
  /// The drug code (`icode` / `working_code`).
  #[must_use]
  pub fn code(&self) -> &str {
    match self {
      Self::Hosxp(d) => &d.icode,
      Self::Invs(d) => &d.working_code,
    }
  }

  /// The drug display name.
  #[must_use]
  pub fn name(&self) -> &str {
    match self {
      Self::Hosxp(d) => &d.drug_name,
      Self::Invs(d) => &d.drug_name,
    }
  }

  /// The 12 monthly values (calendar order for HOSxP, fiscal for INVS).
  #[must_use]
  pub fn values(&self) -> &[f64] {
    match self {
      Self::Hosxp(d) => &d.monthly_qty,
      Self::Invs(d) => &d.monthly_value,
    }
  }

  /// The annual total (quantity / value).
  #[must_use]
  pub fn total(&self) -> f64 {
    match self {
      Self::Hosxp(d) => d.total_qty,
      Self::Invs(d) => d.total_value,
    }
  }

  /// Whether the series is a monetary value (INVS) vs. a plain quantity.
  #[must_use]
  pub fn is_value(&self) -> bool {
    matches!(self, Self::Invs(_))
  }

  /// The x-axis month labels (calendar Thai months for HOSxP, fiscal for INVS).
  #[must_use]
  pub fn months(&self) -> &'static [&'static str; 12] {
    match self {
      Self::Hosxp(_) => &THAI_MONTHS_SHORT,
      Self::Invs(_) => &FISCAL_MONTHS_SHORT,
    }
  }

  /// The bar-series legend label.
  #[must_use]
  pub fn series_label(&self) -> &'static str {
    if self.is_value() {
      "มูลค่ารายเดือน"
    } else {
      "จำนวนจ่าย"
    }
  }
}

// ─── Fiscal year utilities ────────────────────────────────────────────

/// Calendar month arrays (index 0 = January).
pub const THAI_MONTHS_SHORT: [&str; 12] = [
  "ม.ค.",
  "ก.พ.",
  "มี.ค.",
  "เม.ย.",
  "พ.ค.",
  "มิ.ย.",
  "ก.ค.",
  "ส.ค.",
  "ก.ย.",
  "ต.ค.",
  "พ.ย.",
  "ธ.ค.",
];

pub const THAI_MONTHS_FULL: [&str; 12] = [
  "มกราคม",
  "กุมภาพันธ์",
  "มีนาคม",
  "เมษายน",
  "พฤษภาคม",
  "มิถุนายน",
  "กรกฎาคม",
  "สิงหาคม",
  "กันยายน",
  "ตุลาคม",
  "พฤศจิกายน",
  "ธันวาคม",
];

/// Fiscal month arrays (index 0 = fiscal month 1 = ต.ค.).
pub const FISCAL_MONTHS_SHORT: [&str; 12] = [
  "ต.ค.",
  "พ.ย.",
  "ธ.ค.",
  "ม.ค.",
  "ก.พ.",
  "มี.ค.",
  "เม.ย.",
  "พ.ค.",
  "มิ.ย.",
  "ก.ค.",
  "ส.ค.",
  "ก.ย.",
];

pub const FISCAL_MONTHS_FULL: [&str; 12] = [
  "ตุลาคม",
  "พฤศจิกายน",
  "ธันวาคม",
  "มกราคม",
  "กุมภาพันธ์",
  "มีนาคม",
  "เมษายน",
  "พฤษภาคม",
  "มิถุนายน",
  "กรกฎาคม",
  "สิงหาคม",
  "กันยายน",
];

/// Thai fiscal year: FY N = 1 Oct (N−1) to 30 Sep N.
///
/// Pure helper (unit-testable); [`current_fiscal_year`] feeds it the live
/// clock values.
#[must_use]
pub fn fiscal_year_from(month: u32, year: i32) -> i32 {
  if month >= 10 { year + 1 } else { year }
}

/// Return the current Thai fiscal year (CE).
#[must_use]
pub fn current_fiscal_year() -> i32 {
  let now = js_sys::Date::new_0();
  fiscal_year_from(now.get_month() + 1, now.get_full_year() as i32)
}

/// Convert a calendar month (1–12) to a 0-based fiscal index (Oct=0, …, Sep=11).
#[must_use]
pub fn cal_month_to_fiscal_idx(cal_month: u32) -> usize {
  if cal_month >= 10 {
    (cal_month - 10) as usize
  } else {
    (cal_month + 2) as usize
  }
}

/// Convert a 0-based fiscal index back to a calendar month (1–12).
#[must_use]
pub fn fiscal_idx_to_cal_month(f_idx: usize) -> u32 {
  if f_idx < 3 {
    (f_idx + 10) as u32
  } else {
    (f_idx - 2) as u32
  }
}

/// Convert a CE year to Buddhist Era.
#[must_use]
pub fn ce_to_be(year: i32) -> i32 {
  year + 543
}

// ─── Formatting helpers ───────────────────────────────────────────────

/// Format a number with Thai thousands separators and at most `max_decimals`
/// decimal places (`1,234.5`-style; trailing zeros are dropped).
#[must_use]
pub fn format_number(value: f64, max_decimals: u32) -> String {
  let rounded = format!("{value:.prec$}", prec = max_decimals as usize);
  let (int_part, frac_part) = rounded.split_once('.').unwrap_or((&rounded, ""));
  let frac = frac_part.trim_end_matches('0');
  if frac.is_empty() {
    group_digits(int_part)
  } else {
    format!("{}.{frac}", group_digits(int_part))
  }
}

/// Format a number as Thai Baht with thousands separator: `฿1,234,567`.
#[must_use]
pub fn format_baht(value: f64, decimals: u32) -> String {
  format!("฿{}", format_number(value, decimals))
}

/// Format a quantity with Thai thousands separator (max 2 decimals).
#[must_use]
pub fn format_qty(qty: f64) -> String {
  format_number(qty, 2)
}

/// Group the integer part of a decimal string with `,` separators.
fn group_digits(int_part: &str) -> String {
  let (negative, digits) = match int_part.strip_prefix('-') {
    Some(rest) => (true, rest),
    None => (false, int_part),
  };
  let mut out = String::with_capacity(digits.len() + digits.len() / 3);
  for (i, ch) in digits.chars().enumerate() {
    if i > 0 && (digits.len() - i) % 3 == 0 {
      out.push(',');
    }
    out.push(ch);
  }
  if negative {
    format!("-{out}")
  } else {
    out
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use wasm_bindgen_test::wasm_bindgen_test;

  #[wasm_bindgen_test]
  fn fiscal_year_flips_in_october() {
    assert_eq!(fiscal_year_from(9, 2025), 2025);
    assert_eq!(fiscal_year_from(10, 2025), 2026);
    assert_eq!(fiscal_year_from(12, 2024), 2025);
    assert_eq!(fiscal_year_from(1, 2025), 2025);
  }

  #[wasm_bindgen_test]
  fn fiscal_index_roundtrips() {
    assert_eq!(cal_month_to_fiscal_idx(10), 0);
    assert_eq!(cal_month_to_fiscal_idx(12), 2);
    assert_eq!(cal_month_to_fiscal_idx(1), 3);
    assert_eq!(cal_month_to_fiscal_idx(9), 11);
    for idx in 0..12 {
      assert_eq!(cal_month_to_fiscal_idx(fiscal_idx_to_cal_month(idx)), idx);
    }
  }

  #[wasm_bindgen_test]
  fn month_arrays_have_twelve_entries() {
    assert_eq!(THAI_MONTHS_SHORT.len(), 12);
    assert_eq!(THAI_MONTHS_FULL.len(), 12);
    assert_eq!(FISCAL_MONTHS_SHORT.len(), 12);
    assert_eq!(FISCAL_MONTHS_FULL.len(), 12);
  }

  #[wasm_bindgen_test]
  fn format_qty_groups_thousands_and_trims_zeros() {
    assert_eq!(format_qty(0.0), "0");
    assert_eq!(format_qty(1000.0), "1,000");
    assert_eq!(format_qty(1234.5), "1,234.5");
    assert_eq!(format_qty(1234.56), "1,234.56");
    assert_eq!(format_qty(12.345), "12.35");
  }

  #[wasm_bindgen_test]
  fn format_baht_prefixes_symbol() {
    assert_eq!(format_baht(1_500_000.0, 0), "฿1,500,000");
    assert_eq!(format_baht(1500.0, 2), "฿1,500");
    assert_eq!(format_baht(1500.5, 2), "฿1,500.5");
  }

  #[wasm_bindgen_test]
  fn buddhist_era_conversion() {
    assert_eq!(ce_to_be(2025), 2568);
  }

  #[wasm_bindgen_test]
  fn chart_series_extracts_fields() {
    let hosxp = ChartSeries::Hosxp(HosxpDrugMonthly {
      icode: "1234".to_owned(),
      drug_name: "พารา".to_owned(),
      monthly_qty: vec![0.0; 12],
      total_qty: 42.0,
    });
    assert_eq!(hosxp.code(), "1234");
    assert_eq!(hosxp.name(), "พารา");
    assert_eq!(hosxp.total(), 42.0);
    assert!(!hosxp.is_value());
    assert_eq!(hosxp.series_label(), "จำนวนจ่าย");
    assert_eq!(hosxp.months()[0], "ม.ค.");

    let invs = ChartSeries::Invs(InvsDrugMonthlyValue {
      working_code: "A1".to_owned(),
      drug_name: "ยา X".to_owned(),
      monthly_value: vec![0.0; 12],
      total_value: 99.0,
      peak_month: 1,
    });
    assert_eq!(invs.code(), "A1");
    assert!(invs.is_value());
    assert_eq!(invs.series_label(), "มูลค่ารายเดือน");
    assert_eq!(invs.months()[0], "ต.ค.");
  }
}
