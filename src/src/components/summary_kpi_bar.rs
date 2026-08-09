//! Summary KPI bar — port of `SummaryKpiBar.vue`.
//!
//! Three cards at the bottom of the app: the HOSxP connection status, the
//! INVS yearly purchase total and the INVS drug count.  Presentational: all
//! values come from the shared contexts.

use leptos::prelude::*;

use crate::components::icons::{Icon, IconKind};
use crate::contexts::{DashboardContext, DbConfigContext};
use crate::models::{format_baht, format_number};

/// Props for [`SummaryKpiBar`] — none.
#[component]
pub fn SummaryKpiBar() -> impl IntoView {
  let db = expect_context::<DbConfigContext>();
  let dash = expect_context::<DashboardContext>();

  // The two INVS cards only render values when INVS is connected *and* the
  // yearly grand totals have been fetched (same condition as the original).
  let invs_ready = move || db.invs_connected.get() && dash.invs_year_summary.get().is_some();

  view! {
      <div class="kpi-bar">
          <div class="kpi-card animate-fade-up" style="animation-delay:0ms">
              <div class="kpi-icon kpi-icon--hosxp">
                  <Icon kind=IconKind::Pill size=22 />
              </div>
              <div class="kpi-body">
                  <div class="kpi-label">"HOSxP สถานะ"</div>
                  <Show when=move || db.hosxp_connected.get()>
                      <div class="kpi-value value-hosxp">"เชื่อมต่อแล้ว"</div>
                  </Show>
                  <Show when=move || !db.hosxp_connected.get()>
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
                  <div class="kpi-label">"INVS สั่งซื้อรวม"</div>
                  <Show when=move || invs_ready()>
                      <div class="kpi-value value-invs">
                          {move || {
                              let summary = dash.invs_year_summary.get();
                              summary
                                  .as_ref()
                                  .map(|s| format_baht(s.total_value, 0))
                                  .unwrap_or_default()
                          }}
                      </div>
                  </Show>
                  <Show when=move || !invs_ready()>
                      <div class="kpi-value kpi-na">"—"</div>
                  </Show>
              </div>
          </div>

          <div class="kpi-card animate-fade-up" style="animation-delay:180ms">
              <div class="kpi-icon kpi-icon--invs">
                  <Icon kind=IconKind::Package size=22 />
              </div>
              <div class="kpi-body">
                  <div class="kpi-label">"INVS รายการยา"</div>
                  <Show when=move || invs_ready()>
                      <div class="kpi-value value-invs">
                          {move || {
                              let summary = dash.invs_year_summary.get();
                              summary
                                  .as_ref()
                                  .map(|s| format_number(s.unique_drug_count as f64, 0))
                                  .unwrap_or_default()
                          }}
                          <span class="kpi-unit">"รายการ"</span>
                      </div>
                  </Show>
                  <Show when=move || !invs_ready()>
                      <div class="kpi-value kpi-na">"—"</div>
                  </Show>
              </div>
          </div>
      </div>
  }
}
