//! Drug-mapping state and actions (Phase 1).
//!
//! Owns the full-screen mapping view's data: the HOSxP list with per-row
//! state and a status filter, the open detail session for the selected drug
//! (scored INVS candidates + match actions), the batch auto-match preview,
//! the CSV import flow, and the match-status chips shown on the dashboard
//! panels (driven by whichever drugs are currently selected in
//! [`DashboardContext`]).  All backend communication goes through
//! [`crate::services`].

use leptos::prelude::*;
use wasm_bindgen::JsValue;

use crate::contexts::{DashboardContext, DbConfigContext};
use crate::models::{
  AutoMatchResult, BulkImportResult, DrugMappingStatus, InvsDrugItem, MappingCandidate, MappingRow,
  MappingStats,
};
use crate::services::commands;
use crate::services::timers::set_timeout_ms;

/// Log a swallowed backend error (mirrors the dashboard context's `log_err`).
fn log_err(tag: &str, message: &str) {
  web_sys::console::error_1(&JsValue::from_str(&format!("{tag}: {message}")));
}

/// Status filter for the mapping list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MappingFilter {
  /// No filter — every loaded row.
  All,
  /// Only rows with `status == "unmapped"`.
  Unmapped,
  /// Only rows with `status == "mapped"`.
  Mapped,
  /// Only rows with `status == "no_invs"`.
  NoInvs,
}

impl MappingFilter {
  /// Whether a row passes this filter.
  #[must_use]
  pub fn matches(self, row: &MappingRow) -> bool {
    match self {
      Self::All => true,
      Self::Unmapped => row.status == "unmapped",
      Self::Mapped => row.status == "mapped",
      Self::NoInvs => row.status == "no_invs",
    }
  }

  /// Thai label for the filter chip.
  #[must_use]
  pub fn label(self) -> &'static str {
    match self {
      Self::All => "ทั้งหมด",
      Self::Unmapped => "ยังไม่แมป",
      Self::Mapped => "แมปแล้ว",
      Self::NoInvs => "ไม่มีใน INVS",
    }
  }
}

/// The open detail session for one HOSxP row: its candidates (or loading)
/// plus the row itself so the pane renders without extra lookups.
#[derive(Clone, Debug)]
pub struct DetailSession {
  pub row: MappingRow,
  pub candidates: Vec<MappingCandidate>,
  pub candidates_loading: bool,
}

/// Shared drug-mapping state, exposed through Leptos context.
///
/// Every field is a plain `RwSignal`: the struct itself is `Copy`, so it can
/// be passed to child components by value.
#[derive(Clone, Copy, Debug)]
pub struct MappingContext {
  /// The dashboard context, captured at `provide()` time.
  ///
  /// `expect_context` must never run inside a `spawn_local` task: Leptos's
  /// `spawn_local` does not inherit the reactive owner, so the lookup
  /// panics.  Holding the contexts here (captured synchronously during
  /// `provide()`, where the owner chain is intact) lets every async method
  /// use them without a lookup.
  dash: DashboardContext,
  /// The DB-connection context, captured at `provide()` time (see above).
  db: DbConfigContext,
  /// HOSxP rows of the current list view (with mapping state).
  pub rows: RwSignal<Vec<MappingRow>>,
  /// Whether a list fetch is in flight.
  pub rows_loading: RwSignal<bool>,
  /// Bumped on every `search_rows` call; stale responses are dropped.
  search_gen: RwSignal<u64>,
  /// The list-view search query.
  pub query: RwSignal<String>,
  /// Status filter applied to the loaded rows.
  pub filter: RwSignal<MappingFilter>,
  /// Headline counts (`mapping_stats`).
  pub stats: RwSignal<Option<MappingStats>>,
  /// The icode currently opened in the detail pane (or `None`).
  pub selected_icode: RwSignal<Option<String>>,
  /// The open detail session (or `None`).
  pub detail: RwSignal<Option<DetailSession>>,
  /// The pending batch auto-match preview (or `None`).
  pub auto_preview: RwSignal<Option<AutoMatchResult>>,
  /// Whether the auto-match preview is being computed.
  pub auto_loading: RwSignal<bool>,
  /// The pasted bulk-import CSV text.
  pub csv_text: RwSignal<String>,
  /// The last bulk-import preview (or `None`).
  pub bulk_preview: RwSignal<Option<BulkImportResult>>,
  /// Whether a bulk preview is being computed.
  pub bulk_loading: RwSignal<bool>,
  /// Drawer feedback line (`สำเร็จ` or the backend error), auto-cleared.
  pub feedback: RwSignal<Option<String>>,
  /// Whether the current feedback is a success (green) or error (red).
  pub feedback_ok: RwSignal<bool>,
  /// Match status of the drug currently selected on the HOSxP panel.
  pub hosxp_link: RwSignal<Option<DrugMappingStatus>>,
  /// Match status of the drug currently selected on the INVS panel.
  pub invs_link: RwSignal<Option<DrugMappingStatus>>,
}

impl MappingContext {
  /// Create the signals, register them in context, and return the handle.
  ///
  /// Must be called while the dashboard and DB contexts are already
  /// provided (App does `DashboardContext::provide()` → `DbConfigContext::provide()`
  /// → `MappingContext::provide()`), so they can be captured here.
  #[must_use]
  pub fn provide() -> Self {
    let ctx = Self {
      dash: expect_context::<DashboardContext>(),
      db: expect_context::<DbConfigContext>(),
      rows: RwSignal::new(Vec::new()),
      rows_loading: RwSignal::new(false),
      search_gen: RwSignal::new(0u64),
      query: RwSignal::new(String::new()),
      filter: RwSignal::new(MappingFilter::All),
      stats: RwSignal::new(None),
      selected_icode: RwSignal::new(None),
      detail: RwSignal::new(None),
      auto_preview: RwSignal::new(None),
      auto_loading: RwSignal::new(false),
      csv_text: RwSignal::new(String::new()),
      bulk_preview: RwSignal::new(None),
      bulk_loading: RwSignal::new(false),
      feedback: RwSignal::new(None),
      feedback_ok: RwSignal::new(true),
      hosxp_link: RwSignal::new(None),
      invs_link: RwSignal::new(None),
    };
    provide_context(ctx);
    ctx
  }

  /// Show the feedback line, auto-cleared after 4 s.
  pub fn show_feedback(self, msg: String, ok: bool) {
    self.feedback.set(Some(msg.clone()));
    self.feedback_ok.set(ok);
    set_timeout_ms(
      move || {
        if self.feedback.get_untracked().as_deref() == Some(msg.as_str()) {
          self.feedback.set(None);
        }
      },
      4000,
    );
  }

  /// Re-run the current list query and refresh stats.
  async fn reload(self) {
    self.search_rows().await;
    self.load_stats().await;
  }

  // ── List view ──────────────────────────────────────────────────────

  /// Fetch the list rows for the current query (up to 100).  A generation
  /// counter drops responses from superseded searches, so an older, slower
  /// response can never overwrite the results of a newer search.
  pub async fn search_rows(self) {
    self.search_gen.update(|g| *g += 1);
    let gen = self.search_gen.get_untracked();
    self.rows_loading.set(true);
    let query = self.query.get_untracked();
    match commands::mapping_list_rows(&query, 100).await {
      Ok(rows) => {
        if self.search_gen.get_untracked() == gen {
          self.rows.set(rows);
        }
      }
      Err(e) => {
        log_err("mapping listRows", &e.message);
        if self.search_gen.get_untracked() == gen {
          self.rows.set(Vec::new());
          self.show_feedback(e.message, false);
        }
      }
    }
    if self.search_gen.get_untracked() == gen {
      self.rows_loading.set(false);
    }
  }

  /// Fetch the mapping headline counts.
  pub async fn load_stats(self) {
    match commands::mapping_stats().await {
      Ok(stats) => self.stats.set(Some(stats)),
      Err(e) => log_err("mapping stats", &e.message),
    }
  }

  /// Open the mapping view with a fresh state: stale detail/auto/bulk
  /// sessions are cleared, then the default list and stats are loaded.
  pub async fn open(self) {
    self.selected_icode.set(None);
    self.detail.set(None);
    self.auto_preview.set(None);
    self.bulk_preview.set(None);
    self.reload().await;
  }

  // ── Panel chips (match status on both panels) ──────────────────────

  /// Refresh the panel chips for the currently selected drugs.
  pub async fn refresh_links(self) {
    let dash = self.dash;
    if let Some(icode) = dash.hosxp_selected_icode.get_untracked() {
      match commands::mapping_status_by_icode(&icode).await {
        Ok(status) => self.hosxp_link.set(Some(status)),
        Err(e) => log_err("mapping statusByIcode", &e.message),
      }
    } else {
      self.hosxp_link.set(None);
    }
    if let Some(code) = dash.invs_selected_code.get_untracked() {
      match commands::mapping_status_by_working_code(&code).await {
        Ok(status) => self.invs_link.set(Some(status)),
        Err(e) => log_err("mapping statusByWorkingCode", &e.message),
      }
    } else {
      self.invs_link.set(None);
    }
  }

  // ── Linked selection (follow the mapping across panels) ─────────────

  /// When the user selects a HOSxP drug, follow its mapping to the INVS
  /// side: select the linked `working_code` there, put it in the INVS
  /// search box, and load its chart.  A no-op for unmapped drugs (the
  /// other panel keeps whatever it was showing).
  pub async fn follow_link_to_invs(self, year: i32, icode: &str) {
    let dash = self.dash;
    if !self.db.invs_connected.get_untracked() {
      return;
    }
    let Ok(status) = commands::mapping_status_by_icode(icode).await else {
      return;
    };
    let Some(link) = status.link.filter(|_| status.status == "mapped") else {
      return;
    };
    let wc = link.working_code;
    dash.select_invs_drug(wc.clone());
    dash
      .invs_search_display
      .set(format!("{wc} — {}", link.drug_name_invs));
    let _ = dash.fetch_invs_monthly(year, wc).await;
  }

  /// Mirror of [`Self::follow_link_to_invs`] for an INVS selection.
  pub async fn follow_link_to_hosxp(self, year: i32, working_code: &str) {
    let dash = self.dash;
    if !self.db.hosxp_connected.get_untracked() {
      return;
    }
    let Ok(status) = commands::mapping_status_by_working_code(working_code).await else {
      return;
    };
    let Some(link) = status.link.filter(|_| status.status == "mapped") else {
      return;
    };
    let icode = link.icode;
    dash.select_hosxp_drug(icode.clone());
    dash
      .hosxp_search_display
      .set(format!("{icode} — {}", link.drug_name_hosxp));
    let _ = dash.fetch_hosxp_monthly(year, icode).await;
  }

  // ── Detail session (select + suggest + match) ──────────────────────

  /// Open the detail session for a row and fetch its scored candidates.
  pub async fn select_row(self, row: MappingRow) {
    self.selected_icode.set(Some(row.icode.clone()));
    self.detail.set(Some(DetailSession {
      row: row.clone(),
      candidates: Vec::new(),
      candidates_loading: true,
    }));
    self.load_candidates(&row).await;
  }

  /// Fetch (or refresh) the candidate list for the current detail row.
  pub async fn load_candidates(self, row: &MappingRow) {
    match commands::mapping_suggest(&row.drug_name, 10).await {
      Ok(candidates) => {
        self.detail.update(|d| {
          if let Some(d) = d {
            if d.row.icode == row.icode {
              d.candidates = candidates;
              d.candidates_loading = false;
            }
          }
        });
      }
      Err(e) => {
        self.detail.update(|d| {
          if let Some(d) = d {
            if d.row.icode == row.icode {
              d.candidates_loading = false;
            }
          }
        });
        self.show_feedback(e.message, false);
      }
    }
  }

  /// Confirm a suggested candidate (method `approved`).
  pub async fn match_candidate(self, icode: &str, drug_name: &str, candidate: &MappingCandidate) {
    let result = commands::mapping_set(
      icode,
      drug_name,
      &candidate.working_code,
      &candidate.drug_name,
      "approved",
      Some(candidate.score),
    )
    .await;
    self
      .after_change(
        result,
        format!("แมปแล้ว: {icode} ↔ {}", candidate.working_code),
      )
      .await;
  }

  /// Force a link to an arbitrary INVS drug picked by the pharmacist
  /// (method `manual`).
  pub async fn manual_match(self, icode: &str, drug_name: &str, item: &InvsDrugItem) {
    let result = commands::mapping_set(
      icode,
      drug_name,
      &item.working_code,
      &item.drug_name,
      "manual",
      None,
    )
    .await;
    self
      .after_change(result, format!("แมปแล้ว: {icode} ↔ {}", item.working_code))
      .await;
  }

  /// Break the detail row's current link.
  pub async fn remove_link(self, row: &MappingRow) {
    let Some(working_code) = &row.working_code else {
      return;
    };
    let result = commands::mapping_remove(&row.icode, working_code).await;
    self.after_change(result, "ยกเลิกการแมปแล้ว".to_owned()).await;
  }

  /// Mark the detail row as having no INVS equivalent.
  pub async fn mark_no_invs(self, row: &MappingRow, reason: &str) {
    let result = commands::mapping_mark_no_invs(&row.icode, reason).await;
    self
      .after_change(result, "บันทึก 'ไม่มีใน INVS' แล้ว".to_owned())
      .await;
  }

  /// Clear the "no INVS equivalent" mark.
  pub async fn unmark_no_invs(self, row: &MappingRow) {
    let result = commands::mapping_unmark_no_invs(&row.icode).await;
    self
      .after_change(result, "ยกเลิก 'ไม่มีใน INVS' แล้ว".to_owned())
      .await;
  }

  /// Common post-change handling: feedback + reload + refresh the panel
  /// chips, then auto-advance the detail selection to the next unmapped
  /// row (so a batch session never makes the pharmacist return to the
  /// list by hand).
  async fn after_change(self, result: Result<(), crate::models::BackendError>, ok_msg: String) {
    match result {
      Ok(()) => {
        self.show_feedback(ok_msg, true);
        self.reload().await;
        self.refresh_links().await;
        self.advance_selection().await;
      }
      Err(e) => self.show_feedback(e.message, false),
    }
  }

  /// If the selected row is no longer the first unmapped row in the list,
  /// select the next unmapped one after it (stays put when there is none).
  async fn advance_selection(self) {
    let Some(changed) = self.selected_icode.get_untracked() else {
      return;
    };
    let rows = self.rows.get_untracked();
    let Some(idx) = rows.iter().position(|r| r.icode == changed) else {
      return;
    };
    if let Some(next) = rows[idx + 1..].iter().find(|r| r.status == "unmapped") {
      self.select_row(next.clone()).await;
    }
  }

  // ── Batch auto-match ───────────────────────────────────────────────

  /// Compute the auto-match preview for the current list (no writes).
  pub async fn auto_preview(self) {
    self.auto_loading.set(true);
    let query = self.query.get_untracked();
    match commands::mapping_auto_match(&query, 100, 0.0, true).await {
      Ok(result) => self.auto_preview.set(Some(result)),
      Err(e) => {
        self.auto_preview.set(None);
        self.show_feedback(e.message, false);
      }
    }
    self.auto_loading.set(false);
  }

  /// Apply the auto-match (same query and threshold as the preview).
  pub async fn auto_apply(self) {
    self.auto_loading.set(true);
    let query = self.query.get_untracked();
    match commands::mapping_auto_match(&query, 100, 0.0, false).await {
      Ok(result) => {
        self.auto_preview.set(None);
        let n = result.applied;
        self.show_feedback(format!("แมปอัตโนมัติแล้ว {n} รายการ (คะแนน ≥ 95%)"), true);
        self.reload().await;
        self.refresh_links().await;
        self.advance_selection().await;
      }
      Err(e) => self.show_feedback(e.message, false),
    }
    self.auto_loading.set(false);
  }

  // ── Bulk CSV import ────────────────────────────────────────────────

  /// Preview the pasted CSV (no writes).
  pub async fn bulk_preview(self) {
    self.bulk_loading.set(true);
    let text = self.csv_text.get_untracked();
    match commands::mapping_bulk_import(&text, true).await {
      Ok(result) => self.bulk_preview.set(Some(result)),
      Err(e) => {
        self.bulk_preview.set(None);
        self.show_feedback(e.message, false);
      }
    }
    self.bulk_loading.set(false);
  }

  /// Apply the previewed CSV.
  pub async fn bulk_apply(self) {
    self.bulk_loading.set(true);
    let text = self.csv_text.get_untracked();
    match commands::mapping_bulk_import(&text, false).await {
      Ok(result) => {
        self.bulk_preview.set(Some(result.clone()));
        self.show_feedback(format!("นำเข้าแล้ว {} รายการ", result.added), true);
        self.reload().await;
        self.refresh_links().await;
        self.advance_selection().await;
      }
      Err(e) => self.show_feedback(e.message, false),
    }
    self.bulk_loading.set(false);
  }
}
