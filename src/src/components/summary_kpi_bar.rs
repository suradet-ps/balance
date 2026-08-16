//! Summary KPI bar — the three year-level cards at the bottom.
//!
//! Cards:
//!
//! 1. **HOSxP ยอดจ่ายรวม** — total dispensed quantity for the selected
//!    fiscal year (+ distinct drug count).
//! 2. **INVS ยอดซื้อรวม** — total purchase value for the fiscal year.
//! 3. **แมปยาแล้ว** — mapping progress from the local store (mapped links +
//!    "no INVS equivalent" count), i.e. how complete the comparison layer is.
//!
//! The old connection-status card is gone (the header badges already say
//! that); every card now carries numbers, symmetric across both systems.
//! Values come from the shared contexts.

use leptos::prelude::*;

use crate::components::icons::{Icon, IconKind};
use crate::contexts::{DashboardContext, DbConfigContext, MappingContext};
use crate::models::{format_baht, format_number, format_qty};

/// Props for [`SummaryKpiBar`] — none.
#[component]
pub fn SummaryKpiBar() -> impl IntoView {
  let db = expect_context::<DbConfigContext>();
  let dash = expect_context::<DashboardContext>();
  let mapping = expect_context::<MappingContext>();

  let hosxp_ready = move || db.hosxp_connected.get() && dash.hosxp_year_summary.get().is_some();
  let invs_ready = move || db.invs_connected.get() && dash.invs_year_summary.get().is_some();

  view! {
      <div class="kpi-bar">
          <div class="kpi-card animate-fade-up" style="animation-delay:0ms">
              <div class="kpi-icon kpi-icon--hosxp">
                  <Icon kind=IconKind::Pill size=22 />
              </div>
              <div class="kpi-body">
                  <div class="kpi-label">"HOSxP ยอดจ่ายรวม"</div>
                  <Show when=move || hosxp_ready()>
                      <div class="kpi-value value-hosxp">
                          {move || {
                              dash
                                  .hosxp_year_summary
                                  .get()
                                  .map(|s| format_qty(s.total_qty))
                                  .unwrap_or_default()
                          }}
                          <span class="kpi-unit">"หน่วย"</span>
                      </div>
                      <div class="kpi-sub">
                          {move || {
                              dash
                                  .hosxp_year_summary
                                  .get()
                                  .map(|s| format!("{} รายการที่จ่าย", s.unique_drug_count))
                                  .unwrap_or_default()
                          }}
                      </div>
                  </Show>
                  <Show when=move || !hosxp_ready()>
                      <div class="kpi-value kpi-na">"—"</div>
                  </Show>
              </div>
          </div>

          <div class="kpi-divider"></div>

          <div class="kpi-card animate-fade-up" style="animation-delay:120ms">
              <div class="kpi-icon kpi-icon--invs">
                  <Icon kind=IconKind::Banknote size=22 />
              </div>
              <div class="kpi-body">
                  <div class="kpi-label">"INVS ยอดซื้อรวม"</div>
                  <Show when=move || invs_ready()>
                      <div class="kpi-value value-invs">
                          {move || {
                              dash
                                  .invs_year_summary
                                  .get()
                                  .map(|s| format_baht(s.total_value, 0))
                                  .unwrap_or_default()
                          }}
                      </div>
                      <div class="kpi-sub">
                          {move || {
                              dash
                                  .invs_year_summary
                                  .get()
                                  .map(|s| format!("{} รายการที่สั่งซื้อ", s.unique_drug_count))
                                  .unwrap_or_default()
                          }}
                      </div>
                  </Show>
                  <Show when=move || !invs_ready()>
                      <div class="kpi-value kpi-na">"—"</div>
                  </Show>
              </div>
          </div>

          <div class="kpi-divider"></div>

          <div class="kpi-card animate-fade-up" style="animation-delay:180ms">
              <div class="kpi-icon kpi-icon--link">
                  <Icon kind=IconKind::Link2 size=22 />
              </div>
              <div class="kpi-body">
                  <div class="kpi-label">"แมปยาแล้ว"</div>
                  <Show when=move || mapping.stats.get().is_some()>
                      <div class="kpi-value value-link">
                          {move || {
                              mapping
                                  .stats
                                  .get()
                                  .map(|s| format_number(s.total as f64, 0))
                                  .unwrap_or_default()
                          }}
                          <span class="kpi-unit">"รายการ"</span>
                      </div>
                      <div class="kpi-sub">
                          {move || {
                              mapping
                                  .stats
                                  .get()
                                  .map(|s| format!("ไม่มีใน INVS {} รายการ", s.exclusions))
                                  .unwrap_or_default()
                          }}
                      </div>
                  </Show>
                  <Show when=move || mapping.stats.get().is_none()>
                      <div class="kpi-value kpi-na">"—"</div>
                  </Show>
              </div>
          </div>
      </div>
  }
}
