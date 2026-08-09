//! App header — port of `AppHeader.vue`.
//!
//! Shows the brand, the fiscal-year selector, the per-database connection
//! badges and the settings button.  Presentational: all state comes from the
//! provided contexts; the settings button emits `on_open_settings`.

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlSelectElement};

use crate::components::icons::{Icon, IconKind};
use crate::contexts::{DashboardContext, DbConfigContext};

/// Props for [`AppHeader`].
#[component]
pub fn AppHeader(on_open_settings: Callback<()>) -> impl IntoView {
  let db = expect_context::<DbConfigContext>();
  let dash = expect_context::<DashboardContext>();

  // Union of both sides' years, newest first; fall back to the selected year.
  let merged_years = Memo::new(move |_| {
    let mut all: Vec<i32> = dash.hosxp_years.get();
    all.extend(dash.invs_years.get());
    all.sort_unstable_by(|a, b| b.cmp(a));
    all.dedup();
    if all.is_empty() {
      vec![dash.selected_year.get()]
    } else {
      all
    }
  });

  let on_year_change = move |ev: Event| {
    let Some(target) = ev.target() else { return };
    let Ok(select) = target.dyn_into::<HtmlSelectElement>() else { return };
    if let Ok(year) = select.value().parse::<i32>() {
      dash.set_year(year);
    }
  };

  let open_settings = move || on_open_settings.run(());

  view! {
      <header class="app-header">
          <div class="header-brand">
              <img class="brand-icon" src="/logo.svg" alt="Balance Logo" />
              <div class="brand-text">
                  <span class="brand-title">"Balance"</span>
                  <span class="brand-sub">"โรงพยาบาลสระโบสถ์"</span>
              </div>
          </div>

          <div class="header-controls">
              <div class="year-selector">
                  <label>"ปีงบประมาณ"</label>
                  <select
                      prop:value=move || dash.selected_year.get()
                      on:change=on_year_change
                  >
                      <For each=move || merged_years.get() key=|y| *y let:year>
                          <option value=year>{year}</option>
                      </For>
                  </select>
              </div>

              <span
                  class="badge"
                  class:badge-connected=move || db.hosxp_connected.get()
                  class:badge-disconnected=move || !db.hosxp_connected.get()
              >
                  <span
                      class="status-dot"
                      class:dot-green=move || db.hosxp_connected.get()
                      class:dot-red=move || !db.hosxp_connected.get()
                  />
                  "MySQL"
              </span>

              <span
                  class="badge"
                  class:badge-connected=move || db.invs_connected.get()
                  class:badge-disconnected=move || !db.invs_connected.get()
              >
                  <span
                      class="status-dot"
                      class:dot-green=move || db.invs_connected.get()
                      class:dot-red=move || !db.invs_connected.get()
                  />
                  "MSSQL"
              </span>

              <button class="btn btn-ghost settings-btn" on:click=move |_| open_settings()>
                  <Icon kind=IconKind::Settings size=14 />
                  "ตั้งค่า"
              </button>
          </div>
      </header>
  }
}
