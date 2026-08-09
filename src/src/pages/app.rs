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
use crate::components::drug_search_panel::DrugSearchPanel;
use crate::components::drug_trend_chart::DrugTrendChart;
use crate::components::icons::{Icon, IconKind};
use crate::components::summary_kpi_bar::SummaryKpiBar;
use crate::contexts::{DashboardContext, DbConfigContext};
use crate::models::Side;

/// The root component of the frontend.
#[component]
pub fn App() -> impl IntoView {
  let dash = DashboardContext::provide();
  let db = DbConfigContext::provide();
  let any_connected = db.any_connected();
  let show_settings = RwSignal::new(false);

  // Reload every side for the selected year — but only the connected ones,
  // matching the original `refreshAll`.
  let refresh_all = {
    let dash = dash;
    let db = db;
    move || {
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
    }
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
  let refresh_on_connect = {
    let db = db;
    let refresh_all = refresh_all;
    let was_connected = Rc::new(Cell::new(false));
    Effect::new(move |_| {
      let connected = db.any_connected().get();
      if connected && !was_connected.replace(connected) {
        refresh_all();
      }
    })
  };
  let _ = refresh_on_connect;

  // Boot: load persisted settings (auto-connects), then let the connect
  // watcher above trigger the first refresh.
  let root_ref = NodeRef::<Div>::new();
  root_ref.on_load(move |_root| {
    spawn_local(async move {
      db.init_from_storage().await;
    });
  });

  let on_hosxp_select = Callback::new(move |code: String| {
    dash.select_hosxp_drug(code.clone());
    let dash = dash;
    spawn_local(async move {
      let _ = dash
        .fetch_hosxp_monthly(dash.selected_year.get_untracked(), code)
        .await;
    });
  });

  let on_invs_select = Callback::new(move |code: String| {
    dash.select_invs_drug(code.clone());
    let dash = dash;
    spawn_local(async move {
      let _ = dash
        .fetch_invs_monthly(dash.selected_year.get_untracked(), code)
        .await;
    });
  });

  view! {
      <div class="app-shell" node_ref=root_ref>
          <AppHeader
              on_open_settings=Callback::new(move |_| show_settings.set(true))
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

          <Show
              when=move || {
                  !any_connected.get() && !db.hosxp_connecting.get() && !db.invs_connecting.get()
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
                  <div class="chart-card card">
                      <DrugTrendChart
                          side=Side::Invs
                          data=dash.invs_chart_data
                          loading=dash.invs_loading_chart
                      />
                  </div>
              </section>
          </main>

          <SummaryKpiBar />

          <ConnectionSettings
              visible=show_settings
              on_close=Callback::new(move |_| show_settings.set(false))
          />
      </div>
  }
}
