//! Drug-mapping drawer (Phase 1).
//!
//! Slide-over panel with two tabs:
//!
//! - **รายการยา (List)** — search the HOSxP catalog, see each row's mapping
//!   state, open the suggestion panel for a row (scored INVS candidates +
//!   a manual INVS search), mark a drug as having no INVS equivalent, and
//!   run the batch auto-match with a preview before confirming.
//! - **นำเข้า CSV (Import)** — paste an `icode,working_code` CSV, get a
//!   dry-run preview ("N จะถูกเพิ่ม, M จะขัดแย้ง") and apply the
//!   non-conflicting rows.
//!
//! All state lives in [`MappingContext`]; the drawer owns only local UI
//! state (the no-INVS reason prompt and the manual-match search box).

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};

use crate::components::icons::{Icon, IconKind};
use crate::contexts::{DashboardContext, MappingContext, MappingTab};
use crate::models::MappingRow;

/// Props for [`MappingPanel`].
#[component]
pub fn MappingPanel(
  /// Whether the drawer is open.
  visible: RwSignal<bool>,
  /// Emitted when the drawer asks to close.
  on_close: Callback<()>,
) -> impl IntoView {
  let mapping = expect_context::<MappingContext>();
  let dash = expect_context::<DashboardContext>();

  // The icode currently being marked as "no INVS equivalent" (reason prompt).
  let no_invs_prompt = RwSignal::new(None::<String>);
  let no_invs_reason = RwSignal::new(String::new());
  // The manual INVS search inside the suggestion panel.
  let manual_query = RwSignal::new(String::new());
  let manual_results = RwSignal::new(Vec::new());
  let manual_loading = RwSignal::new(false);

  // Reload the list whenever the drawer opens.
  Effect::new(move |_| {
    if visible.get() {
      let mapping = mapping;
      spawn_local(async move {
        mapping.open().await;
      });
    }
  });

  let close = move || on_close.run(());

  let on_overlay = move |ev: web_sys::MouseEvent| {
    if let (Some(target), Some(current)) = (ev.target(), ev.current_target()) {
      if target == current {
        close();
      }
    }
  };

  // ── List tab actions ───────────────────────────────────────────────

  let run_search = move |_| {
    let mapping = mapping;
    spawn_local(async move {
      mapping.search_rows().await;
    });
  };

  let on_search_keydown = move |ev: web_sys::KeyboardEvent| {
    if ev.key() == "Enter" {
      ev.prevent_default();
      run_search(());
    }
  };

  let open_suggestion = move |row: MappingRow| {
    let mapping = mapping;
    spawn_local(async move {
      mapping.open_suggestion(&row).await;
    });
  };

  let confirm_candidate = move |(row, cand): (MappingRow, crate::models::MappingCandidate)| {
    let mapping = mapping;
    spawn_local(async move {
      mapping
        .match_candidate(&row.icode, &row.drug_name, &cand)
        .await;
    });
  };

  let confirm_manual = move |(row, item): (MappingRow, crate::models::InvsDrugItem)| {
    let mapping = mapping;
    spawn_local(async move {
      mapping
        .manual_match(&row.icode, &row.drug_name, &item)
        .await;
      manual_query.set(String::new());
      manual_results.set(Vec::new());
    });
  };

  let remove_link = move |row: MappingRow| {
    let mapping = mapping;
    spawn_local(async move {
      mapping.remove_link(&row).await;
    });
  };

  let start_no_invs = move |icode: String| {
    no_invs_reason.set(String::new());
    no_invs_prompt.set(Some(icode));
  };

  let confirm_no_invs = move |row: MappingRow| {
    let mapping = mapping;
    let reason = no_invs_reason.get_untracked();
    spawn_local(async move {
      mapping.mark_no_invs(&row, &reason).await;
    });
    no_invs_prompt.set(None);
  };

  let cancel_no_invs = move |()| no_invs_prompt.set(None);

  let unmark_no_invs = move |row: MappingRow| {
    let mapping = mapping;
    spawn_local(async move {
      mapping.unmark_no_invs(&row).await;
    });
  };

  let run_auto_preview = move |_| {
    let mapping = mapping;
    spawn_local(async move {
      mapping.auto_preview().await;
    });
  };

  let run_auto_apply = move |_| {
    let mapping = mapping;
    spawn_local(async move {
      mapping.auto_apply().await;
    });
  };

  let close_suggestion = move |()| {
    mapping.close_suggestion();
    manual_query.set(String::new());
    manual_results.set(Vec::new());
  };

  // Manual INVS search within the suggestion panel (Enter-triggered).
  let run_manual_search = move |_| {
    let dash = dash;
    let q = manual_query.get_untracked();
    if q.trim().is_empty() {
      manual_results.set(Vec::new());
      return;
    }
    manual_loading.set(true);
    spawn_local(async move {
      let hits = dash.search_invs_drugs(q).await;
      manual_results.set(hits);
      manual_loading.set(false);
    });
  };

  // ── CSV tab actions ────────────────────────────────────────────────

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
    });
  };

  view! {
      <Show when=move || visible.get()>
          <div class="drawer-overlay" on:click=on_overlay>
              <div class="drawer-panel drawer-panel--wide">
                  <div class="drawer-header">
                      <span class="drawer-title">
                          <Icon kind=IconKind::Link2 size=16 />
                          "แมปยา HOSxP ↔ INVS"
                      </span>
                      <button class="btn-icon" on:click=move |_| close()>
                          <Icon kind=IconKind::X size=16 />
                      </button>
                  </div>

                  <div class="tab-bar">
                      <button
                          class="tab-btn"
                          class:active=move || mapping.active_tab.get() == MappingTab::List
                          on:click=move |_| mapping.active_tab.set(MappingTab::List)
                      >
                          <Icon kind=IconKind::Search size=14 />
                          "รายการยา"
                      </button>
                      <button
                          class="tab-btn"
                          class:active=move || mapping.active_tab.get() == MappingTab::Csv
                          on:click=move |_| mapping.active_tab.set(MappingTab::Csv)
                      >
                          <Icon kind=IconKind::Upload size=14 />
                          "นำเข้า CSV"
                      </button>
                  </div>

                  <Show when=move || mapping.active_tab.get() == MappingTab::List>
                      <MappingList
                          no_invs_prompt=no_invs_prompt
                          no_invs_reason=no_invs_reason
                          manual_query=manual_query
                          manual_results=manual_results
                          manual_loading=manual_loading
                          on_search=Callback::new(run_search)
                          on_search_keydown=Callback::new(on_search_keydown)
                          on_open_suggestion=Callback::new(open_suggestion)
                          on_confirm_candidate=Callback::new(confirm_candidate)
                          on_confirm_manual=Callback::new(confirm_manual)
                          on_remove_link=Callback::new(remove_link)
                          on_start_no_invs=Callback::new(start_no_invs)
                          on_confirm_no_invs=Callback::new(confirm_no_invs)
                          on_cancel_no_invs=Callback::new(cancel_no_invs)
                          on_unmark_no_invs=Callback::new(unmark_no_invs)
                          on_auto_preview=Callback::new(run_auto_preview)
                          on_auto_apply=Callback::new(run_auto_apply)
                          on_close_suggestion=Callback::new(close_suggestion)
                          on_manual_search=Callback::new(run_manual_search)
                      />
                  </Show>

                  <Show when=move || mapping.active_tab.get() == MappingTab::Csv>
                      <div class="form-section">
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
                                  on:click=run_bulk_preview
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
                                      on:click=run_bulk_apply
                                  >
                                      <Icon kind=IconKind::Check size=14 />
                                      {move || {
                                          format!("นำเข้า {} รายการ", mapping.bulk_preview.get().map_or(0, |p| p.added))
                                      }}
                                  </button>
                              </Show>
                          </div>

                          <Show when=move || mapping.bulk_preview.get().is_some()>
                              <BulkPreviewSummary />
                          </Show>
                      </div>
                  </Show>

                  <Show when=move || mapping.feedback.get().is_some()>
                      <div
                          class="save-feedback"
                          class:save-ok=move || mapping.feedback_ok.get()
                          class:save-err=move || !mapping.feedback_ok.get()
                      >
                          {move || mapping.feedback.get().unwrap_or_default()}
                      </div>
                  </Show>
              </div>
          </div>
      </Show>
  }
}

// ─── List tab ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
#[component]
fn MappingList(
  no_invs_prompt: RwSignal<Option<String>>,
  no_invs_reason: RwSignal<String>,
  manual_query: RwSignal<String>,
  manual_results: RwSignal<Vec<crate::models::DrugResult>>,
  manual_loading: RwSignal<bool>,
  on_search: Callback<()>,
  on_search_keydown: Callback<web_sys::KeyboardEvent>,
  on_open_suggestion: Callback<MappingRow>,
  on_confirm_candidate: Callback<(MappingRow, crate::models::MappingCandidate)>,
  on_confirm_manual: Callback<(MappingRow, crate::models::InvsDrugItem)>,
  on_remove_link: Callback<MappingRow>,
  on_start_no_invs: Callback<String>,
  on_confirm_no_invs: Callback<MappingRow>,
  on_cancel_no_invs: Callback<()>,
  on_unmark_no_invs: Callback<MappingRow>,
  on_auto_preview: Callback<()>,
  on_auto_apply: Callback<()>,
  on_close_suggestion: Callback<()>,
  on_manual_search: Callback<()>,
) -> impl IntoView {
  let mapping = expect_context::<MappingContext>();

  view! {
      <div class="form-section mapping-list">
          <MappingStatsBar />

          <div class="mapping-toolbar">
              <div class="search-input-wrap">
                  <Icon kind=IconKind::Search class="search-icon" size=14 />
                  <input
                      class="input search-input"
                      placeholder="ค้นหายา HOSxP (รหัส / ชื่อ)..."
                      autocomplete="off"
                      prop:value=move || mapping.query.get()
                      on:input=bind_text(mapping.query)
                      on:keydown=move |ev: web_sys::KeyboardEvent| on_search_keydown.run(ev)
                  />
              </div>
              <button class="btn btn-ghost" on:click=move |_| on_search.run(())>
                  "ค้นหา"
              </button>
              <button
                  class="btn btn-primary"
                  disabled=move || mapping.auto_loading.get()
                  on:click=move |_| on_auto_preview.run(())
              >
                  <Icon kind=IconKind::PlugZap size=14 />
                  "แมปอัตโนมัติ"
              </button>
          </div>

          <Show when=move || mapping.auto_preview.get().is_some()>
              <AutoMatchPreviewBar on_apply=on_auto_apply />
          </Show>

          <Show when=move || mapping.suggestion.get().is_some()>
              <SuggestionPanel
                  manual_query=manual_query
                  manual_results=manual_results
                  manual_loading=manual_loading
                  on_confirm_candidate=on_confirm_candidate
                  on_confirm_manual=on_confirm_manual
                  on_close=on_close_suggestion
                  on_manual_search=on_manual_search
              />
          </Show>

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

              <For each=move || mapping.rows.get() key=|r| r.icode.clone() let:row>
                  <MappingRowView
                      row=row
                      no_invs_prompt=no_invs_prompt
                      no_invs_reason=no_invs_reason
                      on_open_suggestion=on_open_suggestion
                      on_remove_link=on_remove_link
                      on_start_no_invs=on_start_no_invs
                      on_confirm_no_invs=on_confirm_no_invs
                      on_cancel_no_invs=on_cancel_no_invs
                      on_unmark_no_invs=on_unmark_no_invs
                  />
              </For>
          </div>
      </div>
  }
}

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
      <div class="mapping-stats">
          <span class="mapping-stat"><strong>{move || total()}</strong>" รายการ"</span>
          <span class="mapping-stat">"อัตโนมัติ "{move || auto()}</span>
          <span class="mapping-stat">"ตรวจสอบแล้ว "{move || approved()}</span>
          <span class="mapping-stat">"ด้วยมือ "{move || manual()}</span>
          <span class="mapping-stat">"ไม่มีใน INVS "{move || excluded()}</span>
      </div>
  }
}

/// The batch auto-match preview banner with a confirm action.
#[component]
fn AutoMatchPreviewBar(on_apply: Callback<()>) -> impl IntoView {
  let mapping = expect_context::<MappingContext>();
  let count = move || mapping.auto_preview.get().map_or(0, |p| p.to_match.len());
  view! {
      <div class="auto-preview">
          <div class="auto-preview-title">
              <Icon kind=IconKind::AlertTriangle size=14 />
              {move || format!("พบ {} รายการที่คะแนนตรงกันสูง (≥ 95%) — ตรวจสอบก่อนยืนยัน", count())}
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
                      <span class="score-pill">
                          {move || format!("{:.0}%", m.score * 100.0)}
                      </span>
                  </div>
              </For>
          </div>
          <div class="auto-preview-actions">
              <button class="btn btn-primary" on:click=move |_| on_apply.run(())>
                  <Icon kind=IconKind::Check size=14 />
                  {move || format!("ยืนยันการแมป {} รายการ", count())}
              </button>
          </div>
      </div>
  }
}

/// The suggestion session for one row: scored candidates + manual search.
#[allow(clippy::too_many_arguments)]
#[component]
fn SuggestionPanel(
  manual_query: RwSignal<String>,
  manual_results: RwSignal<Vec<crate::models::DrugResult>>,
  manual_loading: RwSignal<bool>,
  on_confirm_candidate: Callback<(MappingRow, crate::models::MappingCandidate)>,
  on_confirm_manual: Callback<(MappingRow, crate::models::InvsDrugItem)>,
  on_close: Callback<()>,
  on_manual_search: Callback<()>,
) -> impl IntoView {
  let mapping = expect_context::<MappingContext>();
  let row = move || {
    mapping
      .suggestion
      .get()
      .map(|s| MappingRow {
        icode: s.icode,
        drug_name: s.drug_name,
        status: "unmapped".to_string(),
        working_code: None,
        no_invs_reason: None,
      })
      .unwrap_or(MappingRow {
        icode: String::new(),
        drug_name: String::new(),
        status: "unmapped".to_string(),
        working_code: None,
        no_invs_reason: None,
      })
  };

  view! {
      <div class="suggestion-panel">
          <div class="suggestion-header">
              <span class="suggestion-title">
                  "คำแนะนำสำหรับ "
                  <span class="drug-code font-mono">{move || row().icode.clone()}</span>
                  " — "
                  {move || row().drug_name.clone()}
              </span>
              <button class="btn-icon" on:click=move |_| on_close.run(())>
                  <Icon kind=IconKind::X size=14 />
              </button>
          </div>

          <Show when=move || mapping.suggestion.get().is_some_and(|s| s.loading)>
              <div class="suggestion-loading">
                  <span class="animate-pulse">"กำลังค้นหาคู่เทียบใน INVS…"</span>
              </div>
          </Show>

          <Show when=move || { mapping.suggestion.get().is_some_and(|s| !s.loading && s.candidates.is_empty()) }>
              <div class="mapping-empty">"ไม่พบคู่เทียบที่ใกล้เคียง — ลองแมปเองด้านล่าง"</div>
          </Show>

          <div class="suggestion-candidates">
              <For
                  each=move || {
                      mapping
                          .suggestion
                          .get()
                          .map_or(Vec::new(), |s| s.candidates)
                  }
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
                          class="btn btn-secondary candidate-btn"
                          on:click=move |_| on_confirm_candidate.run((row(), cand.clone()))
                      >
                          <Icon kind=IconKind::Check size=13 />
                          "แมป"
                      </button>
                  </div>
              </For>
          </div>

          <div class="manual-section">
              <div class="manual-label">"แมปเอง — ค้นหายา INVS:"</div>
              <div class="manual-search">
                  <input
                      class="input"
                      placeholder="รหัส INVS / ชื่อยา..."
                      autocomplete="off"
                      prop:value=move || manual_query.get()
                      on:input=bind_text(manual_query)
                      on:keydown=move |ev: web_sys::KeyboardEvent| {
                          if ev.key() == "Enter" {
                              ev.prevent_default();
                              on_manual_search.run(());
                          }
                      }
                  />
                  <button class="btn btn-ghost" on:click=move |_| on_manual_search.run(())>
                      "ค้นหา"
                  </button>
              </div>
              <Show when=move || manual_loading.get()>
                  <div class="mapping-empty"><span class="animate-pulse">"กำลังค้นหา…"</span></div>
              </Show>
              <Show when=move || !manual_results.get().is_empty()>
                  <div class="manual-results">
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
                                  on:click=move |_| {
                                      on_confirm_manual.run((
                                          row(),
                                          crate::models::InvsDrugItem {
                                              working_code: item.code().to_owned(),
                                              drug_name: item.name().to_owned(),
                                          },
                                      ))
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
      </div>
  }
}

/// One HOSxP row with its state + actions.
#[allow(clippy::too_many_arguments)]
#[component]
fn MappingRowView(
  row: MappingRow,
  no_invs_prompt: RwSignal<Option<String>>,
  no_invs_reason: RwSignal<String>,
  on_open_suggestion: Callback<MappingRow>,
  on_remove_link: Callback<MappingRow>,
  on_start_no_invs: Callback<String>,
  on_confirm_no_invs: Callback<MappingRow>,
  on_cancel_no_invs: Callback<()>,
  on_unmark_no_invs: Callback<MappingRow>,
) -> impl IntoView {
  let row_state = StoredValue::new(row);
  let is_prompt =
    move || no_invs_prompt.get().as_deref() == Some(row_state.get_value().icode.as_str());

  view! {
      <div class="mapping-row">
          <div class="mapping-row-main">
              <span class="drug-code font-mono">{row_state.get_value().icode.clone()}</span>
              <span class="drug-name">{row_state.get_value().drug_name.clone()}</span>
              <Show when=move || row_state.get_value().status == "mapped">
                  <span class="badge badge-connected">
                      <Icon kind=IconKind::Link2 size=12 />
                      {move || {
                          let wc = row_state.get_value().working_code.clone().unwrap_or_default();
                          format!("INVS: {wc}")
                      }}
                  </span>
              </Show>
              <Show when=move || row_state.get_value().status == "no_invs">
                  <span
                      class="badge badge-muted"
                      title=move || {
                          row_state
                              .get_value()
                              .no_invs_reason
                              .clone()
                              .unwrap_or_default()
                      }
                  >
                      <Icon kind=IconKind::XCircle size=12 />
                      {move || {
                          let reason = row_state.get_value().no_invs_reason.clone().unwrap_or_default();
                          if reason.is_empty() {
                              "ไม่มีใน INVS".to_owned()
                          } else {
                              format!("ไม่มีใน INVS ({reason})")
                          }
                      }}
                  </span>
              </Show>
              <Show when=move || row_state.get_value().status == "unmapped">
                  <span class="badge badge-unmapped">
                      "ยังไม่แมป"
                  </span>
              </Show>
          </div>

          <Show when=move || { row_state.get_value().status == "unmapped" && !is_prompt() }>
              <div class="mapping-row-actions">
                  <button
                      class="btn btn-ghost"
                      on:click=move |_| on_open_suggestion.run(row_state.get_value().clone())
                  >
                      <Icon kind=IconKind::Search size=12 />
                      "ดูคำแนะนำ"
                  </button>
                  <button
                      class="btn btn-ghost"
                      on:click=move |_| on_start_no_invs.run(row_state.get_value().icode.clone())
                  >
                      <Icon kind=IconKind::XCircle size=12 />
                      "ไม่มีใน INVS"
                  </button>
              </div>
          </Show>

          <Show when=move || { row_state.get_value().status == "mapped" }>
              <div class="mapping-row-actions">
                  <button
                      class="btn btn-ghost"
                      on:click=move |_| on_remove_link.run(row_state.get_value().clone())
                  >
                      "ยกเลิกการแมป"
                  </button>
              </div>
          </Show>

          <Show when=move || { row_state.get_value().status == "no_invs" }>
              <div class="mapping-row-actions">
                  <button
                      class="btn btn-ghost"
                      on:click=move |_| on_unmark_no_invs.run(row_state.get_value().clone())
                  >
                      "ยกเลิก"
                  </button>
              </div>
          </Show>

          <Show when=move || is_prompt()>
              <div class="no-invs-prompt">
                  <input
                      class="input"
                      placeholder="เหตุผล (เช่น เลิกจัดซื้อแล้ว)..."
                      autocomplete="off"
                      prop:value=move || no_invs_reason.get()
                      on:input=bind_text(no_invs_reason)
                      on:keydown=move |ev: web_sys::KeyboardEvent| {
                          if ev.key() == "Enter" {
                              ev.prevent_default();
                              on_confirm_no_invs.run(row_state.get_value().clone());
                          }
                      }
                  />
                  <button
                      class="btn btn-primary"
                      on:click=move |_| on_confirm_no_invs.run(row_state.get_value().clone())
                  >
                      "ยืนยัน"
                  </button>
                  <button class="btn btn-ghost" on:click=move |_| on_cancel_no_invs.run(())>
                      "ยกเลิก"
                  </button>
              </div>
          </Show>
      </div>
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
                      each=move || mapping.bulk_preview.get().map_or(Vec::new(), |p| p.conflicts)
                      key=|c| c.icode.clone()
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
                      each=move || mapping.bulk_preview.get().map_or(Vec::new(), |p| p.errors)
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

/// Bind a text input to a string signal.
fn bind_text(sig: RwSignal<String>) -> impl FnMut(web_sys::Event) + 'static {
  move |ev: web_sys::Event| {
    if let Some(value) = input_value(&ev) {
      sig.set(value);
    }
  }
}

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
