//! Dashboard state and data actions.
//!
//! Mirrors the Pinia `dashboard` store plus the `useHosxpData` /
//! `useInvsData` composables: it owns the year, per-side chart data and
//! loading flags, and performs the async backend fetches.  All
//! backend communication goes through [`crate::services`].

use leptos::prelude::*;
use wasm_bindgen::JsValue;

use crate::models::{current_fiscal_year, ChartSeries, DrugResult, InvsYearSummary, Side};
use crate::services::commands;

/// Log a swallowed backend error (the original composables `console.error`).
fn log_err(tag: &str, message: &str) {
  web_sys::console::error_1(&JsValue::from_str(&format!("{tag}: {message}")));
}

/// Shared dashboard state, exposed through Leptos context.
///
/// Every field is a plain `RwSignal`: the struct itself is `Copy`, so it can
/// be passed to child components by value.
#[derive(Clone, Copy, Debug)]
pub struct DashboardContext {
  /// Selected Thai fiscal year (CE).
  pub selected_year: RwSignal<i32>,
  /// Years available from HOSxP, newest first.
  pub hosxp_years: RwSignal<Vec<i32>>,
  /// Icode of the drug currently selected on the HOSxP side.
  pub hosxp_selected_icode: RwSignal<Option<String>>,
  /// Text shown in the HOSxP search box when a *mapped* INVS selection
  /// pulled that drug in (e.g. `"041234 — Amoxicillin"`).
  pub hosxp_search_display: RwSignal<String>,
  /// Monthly trend of the selected HOSxP drug (or `None`).
  pub hosxp_chart_data: RwSignal<Option<ChartSeries>>,
  /// Whether a HOSxP chart fetch is in flight (shows that side's skeleton).
  pub hosxp_loading_chart: RwSignal<bool>,
  /// Years available from INVS, newest first.
  pub invs_years: RwSignal<Vec<i32>>,
  /// Working code of the drug currently selected on the INVS side.
  pub invs_selected_code: RwSignal<Option<String>>,
  /// Text shown in the INVS search box when a *mapped* HOSxP selection
  /// pulled that drug in (e.g. `"WA001 — Amoxicillin"`).
  pub invs_search_display: RwSignal<String>,
  /// Monthly trend of the selected INVS drug (or `None`).
  pub invs_chart_data: RwSignal<Option<ChartSeries>>,
  /// Whether an INVS chart fetch is in flight (shows that side's skeleton).
  pub invs_loading_chart: RwSignal<bool>,
  /// INVS yearly grand totals (or `None`).
  pub invs_year_summary: RwSignal<Option<InvsYearSummary>>,
  /// Whether a full refresh is in flight.
  pub loading: RwSignal<bool>,
  /// Last dashboard-level error (displayed as a banner).
  pub error: RwSignal<Option<String>>,
}

impl DashboardContext {
  /// Create the signals, register them in context, and return the handle.
  #[must_use]
  pub fn provide() -> Self {
    let ctx = Self {
      selected_year: RwSignal::new(current_fiscal_year()),
      hosxp_years: RwSignal::new(Vec::new()),
      hosxp_selected_icode: RwSignal::new(None),
      hosxp_search_display: RwSignal::new(String::new()),
      hosxp_chart_data: RwSignal::new(None),
      hosxp_loading_chart: RwSignal::new(false),
      invs_years: RwSignal::new(Vec::new()),
      invs_selected_code: RwSignal::new(None),
      invs_search_display: RwSignal::new(String::new()),
      invs_chart_data: RwSignal::new(None),
      invs_loading_chart: RwSignal::new(false),
      invs_year_summary: RwSignal::new(None),
      loading: RwSignal::new(false),
      error: RwSignal::new(None),
    };
    provide_context(ctx);
    ctx
  }

  /// Change the selected fiscal year.
  pub fn set_year(self, year: i32) {
    self.selected_year.set(year);
  }

  /// Remember the HOSxP drug selected by the user.
  pub fn select_hosxp_drug(self, icode: String) {
    self.hosxp_selected_icode.set(Some(icode));
  }

  /// Remember the INVS drug selected by the user.
  pub fn select_invs_drug(self, code: String) {
    self.invs_selected_code.set(Some(code));
  }

  // ── HOSxP data ────────────────────────────────────────────────────

  /// Fetch the HOSxP years, store them, and return them.
  pub async fn fetch_hosxp_years(self) -> Vec<i32> {
    match commands::hosxp_get_available_years().await {
      Ok(years) => {
        self.hosxp_years.set(years.clone());
        years
      }
      // Don't set the global error — HOSxP may not be connected.
      Err(e) => {
        log_err("HOSxP fetchAvailableYears", &e.message);
        Vec::new()
      }
    }
  }

  /// Whether the HOSxP chart fetch for `(year, icode)` is still current
  /// (a newer selection or year change may have superseded it).
  #[must_use]
  fn is_current_hosxp(self, year: i32, icode: &str) -> bool {
    self.selected_year.get_untracked() == year
      && self.hosxp_selected_icode.get_untracked().as_deref() == Some(icode)
  }

  /// Whether the INVS chart fetch for `(year, working_code)` is still current.
  #[must_use]
  fn is_current_invs(self, year: i32, code: &str) -> bool {
    self.selected_year.get_untracked() == year
      && self.invs_selected_code.get_untracked().as_deref() == Some(code)
  }

  /// Fetch the 12-month quantities for `icode`, store them, return them.
  ///
  /// A response that arrives after the user has already switched year or
  /// selected a different drug is dropped instead of overwriting the newer
  /// data.
  pub async fn fetch_hosxp_monthly(self, year: i32, icode: String) -> Option<ChartSeries> {
    self.hosxp_loading_chart.set(true);
    let result = match commands::hosxp_get_drug_monthly_qty(year, &icode).await {
      Ok(mut list) => list.drain(..).next().map(ChartSeries::Hosxp),
      Err(e) => {
        log_err("HOSxP fetchDrugMonthly", &e.message);
        None
      }
    };
    if self.is_current_hosxp(year, &icode) {
      self.hosxp_chart_data.set(result.clone());
      self.hosxp_loading_chart.set(false);
    }
    result
  }

  /// Search HOSxP drugs by code / name.
  pub async fn search_hosxp_drugs(self, query: String) -> Vec<DrugResult> {
    if query.trim().is_empty() {
      return Vec::new();
    }
    match commands::hosxp_get_drug_list(&query).await {
      Ok(items) => items.into_iter().map(DrugResult::Hosxp).collect(),
      Err(e) => {
        log_err("HOSxP searchDrugs", &e.message);
        Vec::new()
      }
    }
  }

  // ── INVS data ─────────────────────────────────────────────────────

  /// Fetch the INVS fiscal years, store them, and return them.
  pub async fn fetch_invs_years(self) -> Vec<i32> {
    match commands::invs_get_available_years().await {
      Ok(years) => {
        self.invs_years.set(years.clone());
        years
      }
      Err(e) => {
        log_err("INVS fetchAvailableYears", &e.message);
        Vec::new()
      }
    }
  }

  /// Fetch the 12 fiscal-month values for `working_code`, store, return.
  ///
  /// A response that arrives after the user has already switched year or
  /// selected a different drug is dropped instead of overwriting the newer
  /// data.
  pub async fn fetch_invs_monthly(self, year: i32, working_code: String) -> Option<ChartSeries> {
    self.invs_loading_chart.set(true);
    let result = match commands::invs_get_drug_monthly_value(year, &working_code).await {
      Ok(data) => Some(ChartSeries::Invs(data)),
      Err(e) => {
        log_err("INVS fetchDrugMonthlyValue", &e.message);
        None
      }
    };
    if self.is_current_invs(year, &working_code) {
      self.invs_chart_data.set(result.clone());
      self.invs_loading_chart.set(false);
    }
    result
  }

  /// Fetch the INVS yearly summary, store it, and return it.
  pub async fn fetch_invs_year_summary(self, year: i32) -> Option<InvsYearSummary> {
    match commands::invs_get_year_summary(year).await {
      Ok(summary) => {
        self.invs_year_summary.set(Some(summary.clone()));
        Some(summary)
      }
      Err(e) => {
        log_err("INVS fetchYearSummary", &e.message);
        None
      }
    }
  }

  /// Search INVS drugs by code / name.
  pub async fn search_invs_drugs(self, query: String) -> Vec<DrugResult> {
    if query.trim().is_empty() {
      return Vec::new();
    }
    match commands::invs_get_drug_list(&query).await {
      Ok(items) => items.into_iter().map(DrugResult::Invs).collect(),
      Err(e) => {
        log_err("INVS searchDrugs", &e.message);
        Vec::new()
      }
    }
  }

  /// Search either database according to `side`.
  pub async fn search_drugs(self, side: Side, query: String) -> Vec<DrugResult> {
    match side {
      Side::Hosxp => self.search_hosxp_drugs(query).await,
      Side::Invs => self.search_invs_drugs(query).await,
    }
  }

  // ── Combined refresh ──────────────────────────────────────────────

  /// Refresh the HOSxP side: the selected drug's chart.
  pub async fn refresh_hosxp(self, year: i32) {
    if let Some(icode) = self.hosxp_selected_icode.get_untracked() {
      let _ = self.fetch_hosxp_monthly(year, icode).await;
    }
  }

  /// Refresh the INVS side: year summary + the selected drug's chart.
  pub async fn refresh_invs(self, year: i32) {
    let summary = self.fetch_invs_year_summary(year);
    let chart = async {
      if let Some(code) = self.invs_selected_code.get_untracked() {
        let _ = self.fetch_invs_monthly(year, code).await;
      }
    };
    let _ = futures::join!(summary, chart);
  }
}
