//! Drug-mapping state and actions (Phase 1).
//!
//! Owns the mapping drawer's data: the HOSxP list with per-row state, the
//! open suggestion session, the batch auto-match preview, the CSV import
//! flow, and the match-status chips shown on the dashboard panels (driven by
//! whichever drugs are currently selected in [`DashboardContext`]).  All
//! backend communication goes through [`crate::services`].

use leptos::prelude::*;
use wasm_bindgen::JsValue;

use crate::contexts::DashboardContext;
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

/// Which section of the mapping drawer is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MappingTab {
  /// Search + list + suggest + auto-match.
  List,
  /// Bulk CSV import.
  Csv,
}

/// An open suggestion session for one HOSxP row.
#[derive(Clone, Debug)]
pub struct Suggestion {
  pub icode: String,
  pub drug_name: String,
  pub candidates: Vec<MappingCandidate>,
  pub loading: bool,
}

/// Shared drug-mapping state, exposed through Leptos context.
///
/// Every field is a plain `RwSignal`: the struct itself is `Copy`, so it can
/// be passed to child components by value.
#[derive(Clone, Copy, Debug)]
pub struct MappingContext {
  /// HOSxP rows of the current list view (with mapping state).
  pub rows: RwSignal<Vec<MappingRow>>,
  /// Whether a list fetch is in flight.
  pub rows_loading: RwSignal<bool>,
  /// Bumped on every `search_rows` call; stale responses are dropped.
  search_gen: RwSignal<u64>,
  /// The list-view search query.
  pub query: RwSignal<String>,
  /// Headline counts (`mapping_stats`).
  pub stats: RwSignal<Option<MappingStats>>,
  /// Active drawer section.
  pub active_tab: RwSignal<MappingTab>,
  /// The open suggestion session (or `None`).
  pub suggestion: RwSignal<Option<Suggestion>>,
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
  #[must_use]
  pub fn provide() -> Self {
    let ctx = Self {
      rows: RwSignal::new(Vec::new()),
      rows_loading: RwSignal::new(false),
      search_gen: RwSignal::new(0u64),
      query: RwSignal::new(String::new()),
      stats: RwSignal::new(None),
      active_tab: RwSignal::new(MappingTab::List),
      suggestion: RwSignal::new(None),
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

  /// Fetch the list rows for the current query.  A generation counter drops
  /// responses from superseded searches (Enter pressed twice in a row, or a
  /// query edited mid-flight), so an older, slower response can never
  /// overwrite the results of a newer search.
  pub async fn search_rows(self) {
    self.search_gen.update(|g| *g += 1);
    let gen = self.search_gen.get_untracked();
    self.rows_loading.set(true);
    let query = self.query.get_untracked();
    match commands::mapping_list_rows(&query, 30).await {
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

  /// Open the drawer with a fresh query and load it.  Session state from a
  /// previous visit (open suggestion, bulk preview, auto-match preview) is
  /// cleared so a reopened drawer never shows stale candidates.
  pub async fn open(self) {
    self.active_tab.set(MappingTab::List);
    self.suggestion.set(None);
    self.bulk_preview.set(None);
    self.auto_preview.set(None);
    self.reload().await;
  }

  // ── Panel chips (match status on both panels) ──────────────────────

  /// Refresh the panel chips for the currently selected drugs.
  pub async fn refresh_links(self) {
    let dash = expect_context::<DashboardContext>();
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

  // ── Suggest + match ────────────────────────────────────────────────

  /// Open the suggestion session for a row and fetch its candidates.
  pub async fn open_suggestion(self, row: &MappingRow) {
    self.suggestion.set(Some(Suggestion {
      icode: row.icode.clone(),
      drug_name: row.drug_name.clone(),
      candidates: Vec::new(),
      loading: true,
    }));
    match commands::mapping_suggest(&row.drug_name, 10).await {
      Ok(candidates) => {
        self.suggestion.update(|s| {
          if let Some(s) = s {
            s.candidates = candidates;
            s.loading = false;
          }
        });
      }
      Err(e) => {
        self.suggestion.update(|s| {
          if let Some(s) = s {
            s.loading = false;
          }
        });
        self.show_feedback(e.message, false);
      }
    }
  }

  /// Close the suggestion session.
  pub fn close_suggestion(self) {
    self.suggestion.set(None);
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

  /// Break the row's current link.
  pub async fn remove_link(self, row: &MappingRow) {
    let Some(working_code) = &row.working_code else {
      return;
    };
    let result = commands::mapping_remove(&row.icode, working_code).await;
    self.after_change(result, "ยกเลิกการแมปแล้ว".to_owned()).await;
  }

  /// Mark the row as having no INVS equivalent.
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
  /// chips (the currently selected drug may have changed state).
  async fn after_change(self, result: Result<(), crate::models::BackendError>, ok_msg: String) {
    match result {
      Ok(()) => {
        self.show_feedback(ok_msg, true);
        self.reload().await;
        self.refresh_links().await;
      }
      Err(e) => self.show_feedback(e.message, false),
    }
  }

  // ── Batch auto-match ───────────────────────────────────────────────

  /// Compute the auto-match preview for the current list (no writes).
  pub async fn auto_preview(self) {
    self.auto_loading.set(true);
    let query = self.query.get_untracked();
    match commands::mapping_auto_match(&query, 50, 0.0, true).await {
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
    match commands::mapping_auto_match(&query, 50, 0.0, false).await {
      Ok(result) => {
        self.auto_preview.set(None);
        let n = result.applied;
        self.show_feedback(format!("แมปอัตโนมัติแล้ว {n} รายการ (คะแนน ≥ 95%)"), true);
        self.reload().await;
        self.refresh_links().await;
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
      }
      Err(e) => self.show_feedback(e.message, false),
    }
    self.bulk_loading.set(false);
  }
}
