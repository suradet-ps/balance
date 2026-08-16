//! Root application view — port of `App.vue`.
//!
//! Provides the shared contexts, owns the settings-drawer visibility and the
//! connection / year watchers, and lays out the two-panel dashboard, the KPI
//! bar and the banners.

use std::cell::Cell;
use std::rc::Rc;

use leptos::html::Div;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::app_header::AppHeader;
use crate::components::connection_settings::ConnectionSettings;
use crate::components::discrepancy_view::DiscrepancyView;
use crate::components::drug_search_panel::DrugSearchPanel;
use crate::components::drug_trend_chart::DrugTrendChart;
use crate::components::icons::{Icon, IconKind};
use crate::components::mapping_panel::MappingPanel;
use crate::components::mapping_status_chip::MappingStatusChip;
use crate::components::summary_kpi_bar::SummaryKpiBar;
use crate::contexts::{DashboardContext, DbConfigContext, MappingContext};
use crate::models::Side;

/// The root component of the frontend.
#[component]
pub fn App() -> impl IntoView {
  let dash = DashboardContext::provide();
  let db = DbConfigContext::provide();
  let mapping = MappingContext::provide();
  let any_connected = db.any_connected();
  let show_settings = RwSignal::new(false);
  let show_mapping = RwSignal::new(false);

  // Reload every side for the selected year — but only the connected ones,
  // matching the original `refreshAll`.
  let refresh_all = move || {
    spawn_local(async move {
      dash.loading.set(true);
      dash.error.set(None);
      let year = dash.selected_year.get_untracked();
      if db.hosxp_connected.get_untracked() {
        let _ = dash.refresh_hosxp(year).await;
      }
      if db.invs_connected.get_untracked() {
        let _ = dash.refresh_invs(year).await;
      }
      dash.loading.set(false);
    });
  };

  // HOSxP connection → fetch available years, correcting the selected year.
  Effect::new(move |_| {
    if db.hosxp_connected.get() {
      let dash = dash;
      spawn_local(async move {
        let years = dash.fetch_hosxp_years().await;
        let selected = dash.selected_year.get_untracked();
        if !years.is_empty() && !years.contains(&selected) {
          dash.set_year(years[0]);
        }
      });
    }
  });

  // INVS connection → fetch available years + the yearly grand totals.
  Effect::new(move |_| {
    if db.invs_connected.get() {
      let dash = dash;
      spawn_local(async move {
        let years = dash.fetch_invs_years().await;
        let selected = dash.selected_year.get_untracked();
        if !years.is_empty() && !years.contains(&selected) {
          dash.set_year(years[0]);
        }
        let _ = dash
          .fetch_invs_year_summary(dash.selected_year.get_untracked())
          .await;
      });
    }
  });

  // Year change → reload everything.
  Effect::new(move |_| {
    let _ = dash.selected_year.get();
    refresh_all();
  });

  // First connection (boot auto-connect or a manual test in the drawer) →
  // reload everything.  Replacing the original's blind 500 ms timer avoids the
  // race where a slow DB connect finishes *after* the timer-fired refresh has
  // already failed with "not connected" and left the dashboard empty.
  let was_connected = Rc::new(Cell::new(false));
  Effect::new(move |_| {
    let connected = db.any_connected().get();
    if connected && !was_connected.replace(connected) {
      refresh_all();
    }
  });

  // Boot: load persisted settings (auto-connects), then let the connect
  // watcher above trigger the first refresh.  Mapping stats are loaded too
  // so the KPI bar's mapping card has numbers without opening the view.
  // The health-poll loop starts here as well.
  let root_ref = NodeRef::<Div>::new();
  root_ref.on_load(move |_root| {
    spawn_local(async move {
      db.init_from_storage().await;
      let _ = mapping.load_stats().await;
      db.start_health_polling(15_000);
    });
  });

  let reconnect_hosxp = move |_| {
    let db = db;
    spawn_local(async move {
      let _ = db.connect_hosxp().await;
    });
  };

  let reconnect_invs = move |_| {
    let db = db;
    spawn_local(async move {
      let _ = db.connect_invs().await;
    });
  };

  let on_hosxp_select = Callback::new(move |code: String| {
    dash.select_hosxp_drug(code.clone());
    let dash = dash;
    let mapping = mapping;
    spawn_local(async move {
      let year = dash.selected_year.get_untracked();
      let _ = dash.fetch_hosxp_monthly(year, code.clone()).await;
      // If the selected HOSxP drug is mapped, pull its INVS counterpart
      // into the right panel too.
      mapping.follow_link_to_invs(year, &code).await;
      mapping.refresh_links().await;
    });
  });

  let on_invs_select = Callback::new(move |code: String| {
    dash.select_invs_drug(code.clone());
    let dash = dash;
    let mapping = mapping;
    spawn_local(async move {
      let year = dash.selected_year.get_untracked();
      let _ = dash.fetch_invs_monthly(year, code.clone()).await;
      // If the selected INVS drug is mapped, pull its HOSxP counterpart
      // into the left panel too.
      mapping.follow_link_to_hosxp(year, &code).await;
      mapping.refresh_links().await;
    });
  });

  view! {
      <div class="app-shell" node_ref=root_ref>
          <AppHeader
              on_open_settings=Callback::new(move |_| show_settings.set(true))
              on_open_mapping=Callback::new(move |_| show_mapping.set(true))
          />

          <Show when=move || dash.error.get().is_some()>
              <div class="error-banner">
                  <Icon kind=IconKind::AlertTriangle size=14 />
                  <span>{move || dash.error.get().unwrap_or_default()}</span>
                  <button class="btn-dismiss" on:click=move |_| dash.error.set(None)>
                      <Icon kind=IconKind::X size=12 />
                  </button>
              </div>
          </Show>

          <Show when=move || db.any_lost().get()>
              <div class="lost-banner">
                  <Icon kind=IconKind::AlertTriangle size=14 />
                  <span>
                      "การเชื่อมต่อฐานข้อมูลหลุด — ข้อมูลที่แสดงเป็นชุดล่าสุด ระบบจะพยายามเชื่อมต่อใหม่โดยอัตโนมัติ"
                  </span>
                  <span class="lost-actions">
                      <Show when=move || db.hosxp_lost.get()>
                          <button class="btn btn-ghost lost-btn" on:click=reconnect_hosxp>
                              "เชื่อมต่อ MySQL ใหม่"
                          </button>
                      </Show>
                      <Show when=move || db.invs_lost.get()>
                          <button class="btn btn-ghost lost-btn" on:click=reconnect_invs>
                              "เชื่อมต่อ MSSQL ใหม่"
                          </button>
                      </Show>
                  </span>
              </div>
          </Show>

          <Show
              when=move || {
                  !any_connected.get()
                      && !db.hosxp_connecting.get()
                      && !db.invs_connecting.get()
                      && !db.any_lost().get()
              }
          >
              <div class="no-conn-banner">
                  <Icon kind=IconKind::PlugZap size=14 />
                  "ยังไม่ได้เชื่อมต่อฐานข้อมูล —"
                  <button class="link-btn" on:click=move |_| show_settings.set(true)>
                      "คลิกเพื่อตั้งค่าการเชื่อมต่อ"
                  </button>
              </div>
          </Show>

          <main class="main-grid">
              <section class="panel panel-hosxp">
                  <div class="panel-label">
                      <span class="panel-dot dot-purple"></span>
                      "HOSxP — ปริมาณการจ่ายยา"
                  </div>
                  <DrugSearchPanel
                      side=Side::Hosxp
                      placeholder="ค้นหายา HOSxP (รหัส / ชื่อ)..."
                      on_select=on_hosxp_select
                  />
                  <MappingStatusChip side=Side::Hosxp />
                  <div class="chart-card card">
                      <DrugTrendChart
                          side=Side::Hosxp
                          data=dash.hosxp_chart_data
                          loading=dash.hosxp_loading_chart
                      />
                  </div>
              </section>

              <div class="panel-divider"></div>

              <section class="panel panel-invs">
                  <div class="panel-label">
                      <span class="panel-dot dot-green"></span>
                      "INVS — มูลค่าการสั่งซื้อ"
                  </div>
                  <DrugSearchPanel
                      side=Side::Invs
                      placeholder="ค้นหายา INVS (รหัส / ชื่อ)..."
                      on_select=on_invs_select
                  />
                  <MappingStatusChip side=Side::Invs />
                  <div class="chart-card card">
                      <DrugTrendChart
                          side=Side::Invs
                          data=dash.invs_chart_data
                          loading=dash.invs_loading_chart
                      />
                  </div>
              </section>
          </main>

          <DiscrepancyView />

          <SummaryKpiBar />

          <ConnectionSettings
              visible=show_settings
              on_close=Callback::new(move |_| show_settings.set(false))
          />

          <MappingPanel
              visible=show_mapping
              on_close=Callback::new(move |_| show_mapping.set(false))
          />
      </div>
  }
}
