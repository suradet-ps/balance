//! Drug-mapping view (Phase 1) — full-screen master–detail.
//!
//! Replaces the old narrow drawer + suggestion sub-panel.  Layout:
//!
//! - **Header** — title, headline stats, the prominent auto-match button,
//!   the CSV import button, close.
//! - **Left pane (master)** — debounced search, status-filter chips
//!   (ทั้งหมด / ยังไม่แมป / แมปแล้ว / ไม่มีใน INVS) and the row list.
//!   Clicking a row selects it.
//! - **Right pane (detail)** — the selected drug: status, scored INVS
//!   candidates with a แมป action each, a manual INVS search, and the
//!   "ไม่มีใน INVS" action.  After a successful change the selection
//!   auto-advances to the next unmapped row, so batch sessions never
//!   require returning to the list by hand.
//!
//! The CSV import and the auto-match confirmation are small modal dialogs
//! on top of the view, not tabs.
//!
//! All state lives in [`MappingContext`]; the view owns only local UI state
//! (the no-INVS reason prompt, the manual-match search box, modal
//! visibility).

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};

use crate::components::icons::{Icon, IconKind};
use crate::contexts::{DashboardContext, DetailSession, MappingContext, MappingFilter};
use crate::models::{InvsDrugItem, MappingCandidate, MappingRow};

/// Props for [`MappingPanel`].
#[component]
pub fn MappingPanel(
  /// Whether the view is open.
  visible: RwSignal<bool>,
  /// Emitted when the view asks to close.
  on_close: Callback<()>,
) -> impl IntoView {
  let mapping = expect_context::<MappingContext>();

  // ── Local UI state ──────────────────────────────────────────────────

  // The icode currently being marked as "no INVS equivalent" (reason prompt).
  let no_invs_prompt = RwSignal::new(None::<String>);
  let no_invs_reason = RwSignal::new(String::new());
  // The manual INVS search inside the detail pane.
  let manual_query = RwSignal::new(String::new());
  let manual_results = RwSignal::new(Vec::new());
  let manual_loading = RwSignal::new(false);
  // Modal dialogs.
  let csv_open = RwSignal::new(false);
  let auto_open = RwSignal::new(false);

  // Reload the view whenever it opens.
  Effect::new(move |_| {
    if visible.get() {
      let mapping = mapping;
      spawn_local(async move {
        mapping.open().await;
      });
    }
  });

  // Per-row UI state (reason prompt) belongs to one drug: drop it whenever
  // the selection moves on (incl. auto-advance after a match).
  Effect::new(move |_| {
    let _ = mapping.selected_icode.get();
    no_invs_prompt.set(None);
    no_invs_reason.set(String::new());
  });

  let close = move || on_close.run(());

  let on_overlay = move |ev: web_sys::MouseEvent| {
    if let (Some(target), Some(current)) = (ev.target(), ev.current_target()) {
      if target == current {
        close();
      }
    }
  };

  let run_auto_preview = move |_ev: web_sys::MouseEvent| {
    let mapping = mapping;
    spawn_local(async move {
      mapping.auto_preview().await;
      auto_open.set(mapping.auto_preview.get_untracked().is_some());
    });
  };

  let run_auto_apply = move |_| {
    let mapping = mapping;
    spawn_local(async move {
      mapping.auto_apply().await;
      auto_open.set(false);
    });
  };

  let close_auto = move |_ev: web_sys::MouseEvent| {
    mapping.auto_preview.set(None);
    auto_open.set(false);
  };

  let run_bulk_preview = move |_| {
    let mapping = mapping;
    spawn_local(async move {
      mapping.bulk_preview().await;
    });
  };

  let run_bulk_apply = move |_| {
    let mapping = mapping;
    spawn_local(async move {
      mapping.bulk_apply().await;
      csv_open.set(false);
      mapping.csv_text.set(String::new());
      mapping.bulk_preview.set(None);
    });
  };

  let close_csv = move |_ev: web_sys::MouseEvent| {
    csv_open.set(false);
    mapping.csv_text.set(String::new());
    mapping.bulk_preview.set(None);
  };

  view! {
      <Show when=move || visible.get()>
          <div class="mapping-overlay" on:click=on_overlay>
              <div class="mapping-view">
                  <div class="mapping-header">
                      <div class="mapping-title">
                          <Icon kind=IconKind::Link2 size=18 />
                          "แมปยา HOSxP ↔ INVS"
                          <MappingStatsBar />
                      </div>
                      <div class="mapping-header-actions">
                          <button
                              class="btn btn-primary"
                              disabled=move || mapping.auto_loading.get()
                              on:click=run_auto_preview
                          >
                              <Icon kind=IconKind::PlugZap size=14 />
                              "แมปอัตโนมัติ"
                          </button>
                          <button class="btn btn-ghost" on:click=move |_ev: web_sys::MouseEvent| {
                              csv_open.set(true)
                          }>
                              <Icon kind=IconKind::Upload size=14 />
                              "นำเข้า CSV"
                          </button>
                          <button class="btn-icon" on:click=move |_ev: web_sys::MouseEvent| close()>
                              <Icon kind=IconKind::X size=18 />
                          </button>
                      </div>
                  </div>

                  <Show when=move || mapping.feedback.get().is_some()>
                      <div
                          class="save-feedback"
                          class:save-ok=move || mapping.feedback_ok.get()
                          class:save-err=move || !mapping.feedback_ok.get()
                      >
                          {move || mapping.feedback.get().unwrap_or_default()}
                      </div>
                  </Show>

                  <div class="mapping-body">
                      <MappingListPane />
                      <MappingDetailPane
                          no_invs_prompt=no_invs_prompt
                          no_invs_reason=no_invs_reason
                          manual_query=manual_query
                          manual_results=manual_results
                          manual_loading=manual_loading
                      />
                  </div>
              </div>

              <Show when=move || auto_open.get()>
                  <div class="mapping-modal-overlay" on:click=close_auto>
                      <div class="mapping-modal" on:click=move |ev: web_sys::MouseEvent| {
                          ev.stop_propagation();
                      }>
                          <AutoMatchConfirm
                              on_apply=Callback::new(run_auto_apply)
                              on_close=Callback::new(move |_| close_auto(web_sys::MouseEvent::new("click").expect("event")))
                          />
                      </div>
                  </div>
              </Show>

              <Show when=move || csv_open.get()>
                  <div class="mapping-modal-overlay" on:click=close_csv>
                      <div class="mapping-modal" on:click=move |ev: web_sys::MouseEvent| {
                          ev.stop_propagation();
                      }>
                          <CsvImport
                              on_preview=Callback::new(run_bulk_preview)
                              on_apply=Callback::new(run_bulk_apply)
                              on_close=Callback::new(move |_| close_csv(web_sys::MouseEvent::new("click").expect("event")))
                          />
                      </div>
                  </div>
              </Show>
          </div>
      </Show>
  }
}

// ─── Header stats ────────────────────────────────────────────────────────

/// The headline counts row (แมปแล้ว / อัตโนมัติ / ตรวจสอบแล้ว / ด้วยมือ /
/// ไม่มีใน INVS).
#[component]
fn MappingStatsBar() -> impl IntoView {
  let mapping = expect_context::<MappingContext>();
  let auto = move || {
    mapping
      .stats
      .get()
      .map_or(0, |s| s.by_method.get("auto").copied().unwrap_or(0))
  };
  let manual = move || {
    mapping
      .stats
      .get()
      .map_or(0, |s| s.by_method.get("manual").copied().unwrap_or(0))
  };
  let approved = move || {
    mapping
      .stats
      .get()
      .map_or(0, |s| s.by_method.get("approved").copied().unwrap_or(0))
  };
  let excluded = move || mapping.stats.get().map_or(0, |s| s.exclusions);
  let total = move || mapping.stats.get().map_or(0, |s| s.total);

  view! {
      <span class="mapping-stats">
          <span class="mapping-stat"><strong>{move || total()}</strong> " รายการ"</span>
          <span class="mapping-stat">"อัตโนมัติ "{move || auto()}</span>
          <span class="mapping-stat">"ตรวจสอบแล้ว "{move || approved()}</span>
          <span class="mapping-stat">"ด้วยมือ "{move || manual()}</span>
          <span class="mapping-stat">"ไม่มีใน INVS "{move || excluded()}</span>
      </span>
  }
}

// ─── Left pane: search + filter + list ──────────────────────────────────

#[component]
fn MappingListPane() -> impl IntoView {
  let mapping = expect_context::<MappingContext>();

  // Debounced search: one long-lived, intentionally-leaked `setTimeout`
  // handler reads the latest query on every fire; each keystroke cancels the
  // pending timer and schedules a fresh one (same pattern as
  // `drug_search_panel`).  The generation counter lives in the context.
  let timer_handle: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
  let run_search = move || {
    let mapping = mapping;
    spawn_local(async move {
      mapping.search_rows().await;
    });
  };
  let fire = Closure::wrap(Box::new(run_search) as Box<dyn FnMut()>);
  let fire_fn: js_sys::Function = fire.as_ref().unchecked_ref::<js_sys::Function>().clone();
  fire.forget();
  let schedule_search = {
    let timer_handle = timer_handle.clone();
    let fire_fn = fire_fn;
    move || {
      let Some(win) = web_sys::window() else { return };
      if let Some(handle) = timer_handle.borrow_mut().take() {
        win.clear_timeout_with_handle(handle);
      }
      if let Ok(handle) = win.set_timeout_with_callback_and_timeout_and_arguments_0(&fire_fn, 300) {
        *timer_handle.borrow_mut() = Some(handle);
      }
    }
  };

  let on_input = move |ev: web_sys::Event| {
    if let Some(v) = input_value(&ev) {
      mapping.query.set(v);
      schedule_search();
    }
  };

  let on_search_keydown = move |ev: web_sys::KeyboardEvent| {
    if ev.key() == "Enter" {
      ev.prevent_default();
      let mapping = mapping;
      spawn_local(async move {
        mapping.search_rows().await;
      });
    }
  };

  let select = move |row: MappingRow| {
    let mapping = mapping;
    spawn_local(async move {
      mapping.select_row(row).await;
    });
  };

  let filters = [
    MappingFilter::All,
    MappingFilter::Unmapped,
    MappingFilter::Mapped,
    MappingFilter::NoInvs,
  ];

  let visible_rows = move || {
    let filter = mapping.filter.get();
    mapping
      .rows
      .get()
      .into_iter()
      .filter(|r| filter.matches(r))
      .collect::<Vec<_>>()
  };

  view! {
      <div class="mapping-pane mapping-list-pane">
          <div class="mapping-search-row">
              <div class="search-input-wrap">
                  <Icon kind=IconKind::Search class="search-icon" size=14 />
                  <input
                      class="input search-input"
                      placeholder="ค้นหายา HOSxP (รหัส / ชื่อ)..."
                      autocomplete="off"
                      prop:value=move || mapping.query.get()
                      on:input=on_input
                      on:keydown=on_search_keydown
                  />
              </div>
          </div>

          <div class="mapping-filters">
              <For each=move || filters key=|f| *f let:filter>
                  <button
                      class="chip"
                      class:chip-active=move || mapping.filter.get() == filter
                      on:click=move |_ev: web_sys::MouseEvent| mapping.filter.set(filter)
                  >
                      {filter.label()}
                  </button>
              </For>
          </div>

          <div class="mapping-rows">
              <Show when=move || mapping.rows_loading.get()>
                  <div class="mapping-empty">
                      <span class="animate-pulse">"กำลังโหลดรายการ…"</span>
                  </div>
              </Show>

              <Show when=move || { !mapping.rows_loading.get() && mapping.rows.get().is_empty() }>
                  <div class="mapping-empty">
                      "ไม่พบรายการยา — ค้นหาด้วยรหัสหรือชื่อ (หรือตรวจสอบการเชื่อมต่อ HOSxP)"
                  </div>
              </Show>

              <Show
                  when=move || {
                      !mapping.rows_loading.get()
                          && !mapping.rows.get().is_empty()
                          && visible_rows().is_empty()
                  }
              >
                  <div class="mapping-empty">"ไม่มีรายการที่ตรงกับตัวกรองนี้"</div>
              </Show>

              <For each=visible_rows key=|r| r.icode.clone() let:row>
                  <MappingRowView row=row on_select=Callback::new(select) />
              </For>
          </div>
      </div>
  }
}

/// One HOSxP row in the list; clicking selects it for the detail pane.
///
/// Leptos 0.8 keyed `<For>` calls the children closure only *once per key*
/// (tachys `keyed` re-runs `view_fn` only for added items), so a captured
/// `row` prop would go stale the moment the list refreshes after a match.
/// The row content is therefore read reactively from `mapping.rows` (found
/// by its icode — the key), with the initially captured row only as a
/// fallback for the instant before the first list arrives.
#[component]
fn MappingRowView(row: MappingRow, on_select: Callback<MappingRow>) -> impl IntoView {
  let mapping = expect_context::<MappingContext>();
  let initial = StoredValue::new(row);
  let live_row = move || {
    let icode = initial.get_value().icode.clone();
    mapping
      .rows
      .get()
      .into_iter()
      .find(|r| r.icode == icode)
      .unwrap_or_else(|| initial.get_value().clone())
  };
  let is_selected =
    move || mapping.selected_icode.get().as_deref() == Some(initial.get_value().icode.as_str());

  view! {
      <button
          class="mapping-row"
          class:row-selected=is_selected
          on:click=move |_ev: web_sys::MouseEvent| on_select.run(live_row())
      >
          <span class="drug-code font-mono">{move || live_row().icode.clone()}</span>
          <span class="drug-name">{move || live_row().drug_name.clone()}</span>
          <Show when=move || live_row().status == "mapped">
              <span class="badge badge-connected">
                  <Icon kind=IconKind::Link2 size=12 />
                  {move || {
                      let wc = live_row().working_code.clone().unwrap_or_default();
                      format!("INVS: {wc}")
                  }}
              </span>
          </Show>
          <Show when=move || live_row().status == "no_invs">
              <span
                  class="badge badge-muted"
                  title=move || live_row().no_invs_reason.clone().unwrap_or_default()
              >
                  <Icon kind=IconKind::XCircle size=12 />
                  "ไม่มีใน INVS"
              </span>
          </Show>
          <Show when=move || live_row().status == "unmapped">
              <span class="badge badge-unmapped">"ยังไม่แมป"</span>
          </Show>
      </button>
  }
}

// ─── Right pane: detail ─────────────────────────────────────────────────

/// The detail pane for the selected drug: status, candidates, manual search,
/// no-INVS action.
#[component]
fn MappingDetailPane(
  no_invs_prompt: RwSignal<Option<String>>,
  no_invs_reason: RwSignal<String>,
  manual_query: RwSignal<String>,
  manual_results: RwSignal<Vec<crate::models::DrugResult>>,
  manual_loading: RwSignal<bool>,
) -> impl IntoView {
  let mapping = expect_context::<MappingContext>();
  let dash = expect_context::<DashboardContext>();

  // Manual-search results belong to one drug: clear them whenever the
  // selection moves on (incl. auto-advance after a match).
  Effect::new(move |_| {
    let _ = mapping.selected_icode.get();
    manual_query.set(String::new());
    manual_results.set(Vec::new());
    manual_loading.set(false);
  });

  let run_manual_search = move |_| {
    let q = manual_query.get_untracked();
    if q.trim().is_empty() {
      manual_results.set(Vec::new());
      return;
    }
    manual_loading.set(true);
    let dash = dash;
    spawn_local(async move {
      let hits = dash.search_invs_drugs(q).await;
      manual_results.set(hits);
      manual_loading.set(false);
    });
  };

  let confirm_manual = move |item: InvsDrugItem| {
    if let Some(d) = mapping.detail.get_untracked() {
      let mapping = mapping;
      spawn_local(async move {
        mapping
          .manual_match(&d.row.icode, &d.row.drug_name, &item)
          .await;
        manual_query.set(String::new());
        manual_results.set(Vec::new());
      });
    }
  };

  let confirm_candidate = move |candidate: MappingCandidate| {
    if let Some(d) = mapping.detail.get_untracked() {
      let mapping = mapping;
      spawn_local(async move {
        mapping
          .match_candidate(&d.row.icode, &d.row.drug_name, &candidate)
          .await;
      });
    }
  };

  let confirm_no_invs = move |_| {
    if let Some(d) = mapping.detail.get_untracked() {
      let mapping = mapping;
      let reason = no_invs_reason.get_untracked();
      spawn_local(async move {
        mapping.mark_no_invs(&d.row, &reason).await;
      });
      no_invs_prompt.set(None);
    }
  };

  view! {
      <div class="mapping-pane mapping-detail-pane">
          <Show
              when=move || mapping.detail.get().is_none()
              fallback=move || {
                  let session = mapping.detail.get().unwrap();
                  view! {
                      <DetailContent
                          session=session
                          no_invs_prompt=no_invs_prompt
                          no_invs_reason=no_invs_reason
                          manual_query=manual_query
                          manual_results=manual_results
                          manual_loading=manual_loading
                          on_confirm_candidate=Callback::new(confirm_candidate)
                          on_confirm_manual=Callback::new(confirm_manual)
                          on_manual_search=Callback::new(run_manual_search)
                          on_confirm_no_invs=Callback::new(confirm_no_invs)
                      />
                  }
              }
          >
              <div class="mapping-detail-empty">
                  <Icon kind=IconKind::Link2 size=28 />
                  <span>"เลือกยาทางซ้ายเพื่อเริ่มแมป"</span>
                  <span class="mapping-detail-hint">
                      "กด 'แมปอัตโนมัติ' เพื่อจับคู่คะแนนสูงทั้งหมดในรายการ"
                  </span>
              </div>
          </Show>
      </div>
  }
}

/// The content of the detail pane for one selected drug.
///
/// The `session` prop is captured only as the initial snapshot: the pane's
/// `Show` fallback is created once per Some-period, and a `Some → Some`
/// change (auto-advance to the next drug) never re-runs it.  Every piece of
/// displayed data is therefore read reactively from `mapping.detail` (the
/// live session) and `mapping.rows` (the refreshed list), so the pane
/// follows selection changes without being re-created.
#[allow(clippy::too_many_arguments)]
#[component]
fn DetailContent(
  session: DetailSession,
  no_invs_prompt: RwSignal<Option<String>>,
  no_invs_reason: RwSignal<String>,
  manual_query: RwSignal<String>,
  manual_results: RwSignal<Vec<crate::models::DrugResult>>,
  manual_loading: RwSignal<bool>,
  on_confirm_candidate: Callback<MappingCandidate>,
  on_confirm_manual: Callback<InvsDrugItem>,
  on_manual_search: Callback<()>,
  on_confirm_no_invs: Callback<()>,
) -> impl IntoView {
  let mapping = expect_context::<MappingContext>();
  let initial = StoredValue::new(session.row.clone());

  // The current drug: whatever the live detail session points at, resolved
  // against the refreshed list (the session snapshot goes stale after a
  // change; the list row carries the fresh status/working code).
  let current = move || {
    let Some(snap) = mapping.detail.get().map(|d| d.row.clone()) else {
      return initial.get_value().clone();
    };
    mapping
      .rows
      .get()
      .into_iter()
      .find(|r| r.icode == snap.icode)
      .unwrap_or(snap)
  };
  let status = move || current().status;
  let is_prompt = move || no_invs_prompt.get().as_deref() == Some(current().icode.as_str());

  let remove = move |_ev: web_sys::MouseEvent| {
    let row = current();
    let mapping = mapping;
    spawn_local(async move {
      mapping.remove_link(&row).await;
    });
  };

  let unmark = move |_ev: web_sys::MouseEvent| {
    let row = current();
    let mapping = mapping;
    spawn_local(async move {
      mapping.unmark_no_invs(&row).await;
    });
  };

  let start_no_invs = move |_ev: web_sys::MouseEvent| {
    no_invs_reason.set(String::new());
    no_invs_prompt.set(Some(current().icode.clone()));
  };

  view! {
      <div class="mapping-detail">
          <div class="mapping-detail-head">
              <span class="drug-code font-mono">{move || current().icode.clone()}</span>
              <span class="drug-name">{move || current().drug_name.clone()}</span>
              <Show when=move || status() == "mapped">
                  <span class="badge badge-connected">
                      <Icon kind=IconKind::Link2 size=12 />
                      {move || {
                          let wc = current().working_code.clone().unwrap_or_default();
                          format!("แมปแล้ว ↔ INVS: {wc}")
                      }}
                  </span>
              </Show>
              <Show when=move || status() == "no_invs">
                  <span
                      class="badge badge-muted"
                      title=move || current().no_invs_reason.clone().unwrap_or_default()
                  >
                      <Icon kind=IconKind::XCircle size=12 />
                      "ไม่มีใน INVS"
                  </span>
              </Show>
              <Show when=move || status() == "unmapped">
                  <span class="badge badge-unmapped">"ยังไม่แมป"</span>
              </Show>
          </div>

          // Mapped → show the link with an undo action.
          <Show when=move || status() == "mapped">
              <div class="detail-card detail-current">
                  <div class="detail-card-title">"ลิงก์ปัจจุบัน"</div>
                  <div class="detail-current-row">
                      <span class="drug-code font-mono">{move || current().icode.clone()}</span>
                      <Icon kind=IconKind::Link2 size=14 />
                      <span class="drug-code font-mono">
                          {move || current().working_code.clone().unwrap_or_default()}
                      </span>
                      <button class="btn btn-ghost detail-btn" on:click=remove>
                          "ยกเลิกการแมป"
                      </button>
                  </div>
              </div>
          </Show>

          // Unmapped → suggest candidates.
          <Show when=move || status() == "unmapped">
              <div class="detail-card">
                  <div class="detail-card-title">
                      <Icon kind=IconKind::Search size=13 />
                      "คู่เทียบใน INVS"
                  </div>
                  <Show when=move || {
                      mapping.detail.get().is_some_and(|d| d.candidates_loading)
                  }>
                      <div class="detail-loading">
                          <span class="animate-pulse">"กำลังค้นหาคู่เทียบ…"</span>
                      </div>
                  </Show>
                  <Show when=move || {
                      mapping
                          .detail
                          .get()
                          .is_some_and(|d| !d.candidates_loading && d.candidates.is_empty())
                  }>
                      <div class="mapping-empty">"ไม่พบคู่เทียบที่ใกล้เคียง — ลองแมปเองด้านล่าง"</div>
                  </Show>
                  <div class="detail-candidates">
                      <For
                          each=move || mapping.detail.get().map_or(Vec::new(), |d| d.candidates)
                          key=|c| c.working_code.clone()
                          let:cand
                      >
                          <div class="candidate-row">
                              <span class="drug-code font-mono">{cand.working_code.clone()}</span>
                              <span class="drug-name">{cand.drug_name.clone()}</span>
                              <span class="score-pill">
                                  {move || format!("{:.0}%", cand.score * 100.0)}
                              </span>
                              <button
                                  class="btn btn-primary candidate-btn"
                                  on:click=move |_ev: web_sys::MouseEvent| {
                                      on_confirm_candidate.run(cand.clone())
                                  }
                              >
                                  <Icon kind=IconKind::Check size=13 />
                                  "แมป"
                              </button>
                          </div>
                      </For>
                  </div>
              </div>

              <div class="detail-card">
                  <div class="detail-card-title">
                      <Icon kind=IconKind::Search size=13 />
                      "แมปเอง — ค้นหายา INVS"
                  </div>
                  <div class="manual-search">
                      <input
                          class="input"
                          placeholder="รหัส INVS / ชื่อยา..."
                          autocomplete="off"
                          prop:value=move || manual_query.get()
                          on:input=move |ev: web_sys::Event| {
                              if let Some(v) = input_value(&ev) {
                                  manual_query.set(v);
                              }
                          }
                          on:keydown=move |ev: web_sys::KeyboardEvent| {
                              if ev.key() == "Enter" {
                                  ev.prevent_default();
                                  on_manual_search.run(());
                              }
                          }
                      />
                      <button class="btn btn-ghost" on:click=move |_ev: web_sys::MouseEvent| {
                          on_manual_search.run(())
                      }>
                          "ค้นหา"
                      </button>
                  </div>
                  <Show when=move || manual_loading.get()>
                      <div class="detail-loading">
                          <span class="animate-pulse">"กำลังค้นหา…"</span>
                      </div>
                  </Show>
                  <Show when=move || !manual_results.get().is_empty()>
                      <div class="detail-candidates">
                          <For
                              each=move || manual_results.get()
                              key=|d| d.code().to_owned()
                              let:item
                          >
                              <div class="candidate-row">
                                  <span class="drug-code font-mono">{item.code().to_owned()}</span>
                                  <span class="drug-name">{item.name().to_owned()}</span>
                                  <button
                                      class="btn btn-primary candidate-btn"
                                      on:click=move |_ev: web_sys::MouseEvent| {
                                          on_confirm_manual.run(InvsDrugItem {
                                              working_code: item.code().to_owned(),
                                              drug_name: item.name().to_owned(),
                                          })
                                      }
                                  >
                                      <Icon kind=IconKind::Check size=13 />
                                      "แมปด้วยรหัสนี้"
                                  </button>
                              </div>
                          </For>
                      </div>
                  </Show>
              </div>

              // No-INVS action (with reason prompt).
              <div class="detail-card">
                  <Show when=move || !is_prompt()>
                      <button class="btn btn-ghost" on:click=start_no_invs>
                          <Icon kind=IconKind::XCircle size=13 />
                          "ยานี้ไม่มีใน INVS (เช่น เลิกจัดซื้อแล้ว)"
                      </button>
                  </Show>
                  <Show when=move || is_prompt()>
                      <div class="no-invs-prompt">
                          <input
                              class="input"
                              placeholder="เหตุผล (เช่น เลิกจัดซื้อแล้ว)..."
                              autocomplete="off"
                              prop:value=move || no_invs_reason.get()
                              on:input=move |ev: web_sys::Event| {
                                  if let Some(v) = input_value(&ev) {
                                      no_invs_reason.set(v);
                                  }
                              }
                              on:keydown=move |ev: web_sys::KeyboardEvent| {
                                  if ev.key() == "Enter" {
                                      ev.prevent_default();
                                      on_confirm_no_invs.run(());
                                  }
                              }
                          />
                          <button
                              class="btn btn-primary"
                              on:click=move |_ev: web_sys::MouseEvent| on_confirm_no_invs.run(())
                          >
                              "ยืนยัน"
                          </button>
                          <button
                              class="btn btn-ghost"
                              on:click=move |_ev: web_sys::MouseEvent| no_invs_prompt.set(None)
                          >
                              "ยกเลิก"
                          </button>
                      </div>
                  </Show>
              </div>
          </Show>

          // No-INVS → offer to undo.
          <Show when=move || status() == "no_invs">
              <div class="detail-card">
                  <div class="detail-card-title">"สถานะ 'ไม่มีใน INVS'"</div>
                  <div class="detail-current-row">
                      <span class="detail-no-invs-reason">
                          {move || {
                              let reason = current().no_invs_reason.clone().unwrap_or_default();
                              if reason.is_empty() {
                                  "ไม่ระบุเหตุผล".to_owned()
                              } else {
                                  format!("เหตุผล: {reason}")
                              }
                          }}
                      </span>
                      <button class="btn btn-ghost detail-btn" on:click=unmark>
                          "ยกเลิกสถานะนี้"
                      </button>
                  </div>
              </div>
          </Show>
      </div>
  }
}

// ─── Auto-match confirm modal ────────────────────────────────────────────

#[component]
fn AutoMatchConfirm(on_apply: Callback<()>, on_close: Callback<()>) -> impl IntoView {
  let mapping = expect_context::<MappingContext>();
  let count = move || mapping.auto_preview.get().map_or(0, |p| p.to_match.len());
  view! {
      <div class="modal-head">
          <span class="modal-title">
              <Icon kind=IconKind::PlugZap size=14 />
              "แมปอัตโนมัติ"
          </span>
          <button class="btn-icon" on:click=move |_ev: web_sys::MouseEvent| on_close.run(())>
              <Icon kind=IconKind::X size=14 />
          </button>
      </div>
      <div class="auto-preview">
          <div class="auto-preview-title">
              {move || format!("พบ {} รายการที่คะแนนตรงกันสูง (≥ 95%)", count())}
          </div>
          <div class="auto-preview-list">
              <For
                  each=move || mapping.auto_preview.get().map_or(Vec::new(), |p| p.to_match)
                  key=|m| m.icode.clone()
                  let:m
              >
                  <div class="auto-preview-item">
                      <span class="drug-code font-mono">{m.icode.clone()}</span>
                      <span class="drug-name">{m.drug_name.clone()}</span>
                      <span class="auto-preview-arrow">"↔"</span>
                      <span class="drug-code font-mono">{m.working_code.clone()}</span>
                      <span class="drug-name">{m.drug_name_invs.clone()}</span>
                      <span class="score-pill">{move || format!("{:.0}%", m.score * 100.0)}</span>
                  </div>
              </For>
          </div>
          <div class="auto-preview-actions">
              <button class="btn btn-ghost" on:click=move |_ev: web_sys::MouseEvent| {
                  on_close.run(())
              }>
                  "ยกเลิก"
              </button>
              <button class="btn btn-primary" on:click=move |_ev: web_sys::MouseEvent| {
                  on_apply.run(())
              }>
                  <Icon kind=IconKind::Check size=14 />
                  {move || format!("ยืนยันการแมป {} รายการ", count())}
              </button>
          </div>
      </div>
  }
}

// ─── CSV import modal ────────────────────────────────────────────────────

#[component]
fn CsvImport(
  on_preview: Callback<()>,
  on_apply: Callback<()>,
  on_close: Callback<()>,
) -> impl IntoView {
  let mapping = expect_context::<MappingContext>();
  view! {
      <div class="modal-head">
          <span class="modal-title">
              <Icon kind=IconKind::Upload size=14 />
              "นำเข้า CSV"
          </span>
          <button class="btn-icon" on:click=move |_ev: web_sys::MouseEvent| on_close.run(())>
              <Icon kind=IconKind::X size=14 />
          </button>
      </div>
      <div class="csv-hint">
          "วางรายการแมปในรูปแบบ: "
          <code>"icode,working_code[,drug_name_hosxp,drug_name_invs]"</code>
      </div>
      <textarea
          class="input csv-input"
          rows=9
          placeholder="041234,WA001,Amoxicillin 500 mg,Amoxicillin (แคปซูล)&#10;041235,WA002"
          prop:value=move || mapping.csv_text.get()
          on:input=bind_textarea(mapping.csv_text)
      ></textarea>
      <div class="csv-actions">
          <button
              class="btn btn-secondary"
              disabled=move || mapping.bulk_loading.get()
              on:click=move |_ev: web_sys::MouseEvent| on_preview.run(())
          >
              <Show when=move || mapping.bulk_loading.get()>
                  <span class="animate-pulse">"กำลังตรวจสอบ…"</span>
              </Show>
              <Show when=move || !mapping.bulk_loading.get()>
                  "ตรวจสอบ (ดูตัวอย่าง)"
              </Show>
          </button>
          <Show
              when=move || mapping
                  .bulk_preview
                  .get()
                  .as_ref()
                  .is_some_and(|p| p.added > 0)
          >
              <button
                  class="btn btn-primary"
                  disabled=move || mapping.bulk_loading.get()
                  on:click=move |_ev: web_sys::MouseEvent| on_apply.run(())
              >
                  <Icon kind=IconKind::Check size=14 />
                  {move || {
                      format!(
                          "นำเข้า {} รายการ",
                          mapping.bulk_preview.get().map_or(0, |p| p.added)
                      )
                  }}
              </button>
          </Show>
      </div>

      <Show when=move || mapping.bulk_preview.get().is_some()>
          <BulkPreviewSummary />
      </Show>
  }
}

/// The dry-run result summary (ทั้งหมด / จะนำเข้า / ขัดแย้ง / ข้าม / ข้อผิดพลาด).
#[component]
fn BulkPreviewSummary() -> impl IntoView {
  let mapping = expect_context::<MappingContext>();
  view! {
      <div class="bulk-preview">
          <Show when=move || {
              mapping
                  .bulk_preview
                  .get()
                  .as_ref()
                  .is_some_and(|p| p.added == 0 && p.conflicts.is_empty() && p.errors.is_empty())
          }>
              <div class="mapping-empty">"ไม่พบรายการที่นำเข้าได้ — ตรวจสอบรูปแบบ CSV"</div>
          </Show>
          <div class="bulk-preview-line">
              {move || {
                  let p = mapping.bulk_preview.get();
                  let Some(p) = p else { return String::new() };
                  format!(
                      "ทั้งหมด {} · จะนำเข้า {} · ขัดแย้ง {} · ข้าม {} · ข้อผิดพลาด {}",
                      p.total,
                      p.added,
                      p.conflicts.len(),
                      p.skipped,
                      p.errors.len(),
                  )
              }}
          </div>
          <Show when=move || !mapping.bulk_preview.get().map_or(Vec::new(), |p| p.conflicts).is_empty()>
              <div class="bulk-conflicts">
                  <div class="bulk-subtitle">"รายการขัดแย้ง (ต้องแมปด้วยมือ):"</div>
                  <For
                      each=move || {
                          mapping
                              .bulk_preview
                              .get()
                              .map_or(Vec::new(), |p| p.conflicts.clone())
                      }
                      // Line numbers are unique within one import, and a
                      // re-preview must re-key rows instead of reusing stale
                      // DOM (Leptos 0.8 keyed <For> never re-runs children
                      // for existing keys).
                      key=|c| c.line
                      let:c
                  >
                      <div class="bulk-conflict-row">
                          <span class="drug-code font-mono">{c.icode.clone()}</span>
                          "→ "
                          <span class="drug-code font-mono">{c.working_code.clone()}</span>
                          " (เดิม: "
                          <span class="drug-code font-mono">{c.existing.clone()}</span>
                          ") — บรรทัด "
                          {c.line}
                      </div>
                  </For>
              </div>
          </Show>
          <Show when=move || !mapping.bulk_preview.get().map_or(Vec::new(), |p| p.errors).is_empty()>
              <div class="bulk-errors">
                  <div class="bulk-subtitle">"ข้อผิดพลาด:"</div>
                  <For
                      each=move || {
                          mapping
                              .bulk_preview
                              .get()
                              .map_or(Vec::new(), |p| p.errors.clone())
                      }
                      key=|e| e.clone()
                      let:e
                  >
                      <div class="bulk-error-row">{e}</div>
                  </For>
              </div>
          </Show>
      </div>
  }
}

// ─── Input helpers ────────────────────────────────────────────────────────

/// Bind a textarea to a string signal.
fn bind_textarea(sig: RwSignal<String>) -> impl FnMut(web_sys::Event) + 'static {
  move |ev: web_sys::Event| {
    if let Some(target) = ev.target() {
      if let Ok(el) = target.dyn_into::<HtmlTextAreaElement>() {
        sig.set(el.value());
      }
    }
  }
}

/// Read the current value of an `<input>` from an event.
fn input_value(ev: &web_sys::Event) -> Option<String> {
  let target = ev.target()?;
  target
    .dyn_into::<HtmlInputElement>()
    .ok()
    .map(|el| el.value())
}
