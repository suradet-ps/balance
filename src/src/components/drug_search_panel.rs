//! Drug autocomplete search panel — port of `DrugSearchPanel.vue`.
//!
//! Debounced (300 ms) backend search, keyboard navigation (↑/↓/Enter/Escape),
//! click-outside dismissal and a loading state.  The search implementation is
//! resolved per-side through the [`DashboardContext`]; the panel itself owns
//! only local UI state.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::html::Div;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, KeyboardEvent, MouseEvent};

use crate::components::icons::{Icon, IconKind};
use crate::contexts::DashboardContext;
use crate::models::{DrugResult, Side};

/// Props for [`DrugSearchPanel`].
#[component]
pub fn DrugSearchPanel(
  /// Which database panel this search belongs to.
  side: Side,
  /// Placeholder shown inside the input.
  placeholder: &'static str,
  /// Emitted with the drug code when the user selects a result.
  on_select: Callback<String>,
) -> impl IntoView {
  let dash = expect_context::<DashboardContext>();

  let query = RwSignal::new(String::new());
  let results = RwSignal::new(Vec::<DrugResult>::new());
  let loading = RwSignal::new(false);
  let show_dropdown = RwSignal::new(false);
  let cursor = RwSignal::new(0usize);

  let root_ref = NodeRef::<Div>::new();

  // ── Debounced search ────────────────────────────────────────────────
  // One long-lived, intentionally-leaked `setTimeout` handler reads the latest
  // query on every fire; each keystroke cancels the pending timer handle and
  // schedules a fresh one, so only the final keystroke inside a 300 ms window
  // triggers a search (same debounce semantics as the original `watch`).
  let timer_handle: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
  // Generation counter: every keystroke, clear and selection bumps it, so an
  // in-flight search response that belongs to an older query is dropped
  // instead of overwriting newer results (or popping the dropdown back open
  // after a clear).  A signal rather than a cell because it is captured by
  // handlers inside the `view!` closure (which requires `Send + Sync`).
  let search_gen = RwSignal::new(0u64);

  let run_search = {
    move || {
      let gen = search_gen.get_untracked();
      let q = query.get_untracked();
      cursor.set(0);
      if q.trim().is_empty() {
        results.set(Vec::new());
        show_dropdown.set(false);
        return;
      }
      spawn_local(async move {
        loading.set(true);
        let hits = dash.search_drugs(side, q).await;
        if search_gen.get_untracked() == gen {
          results.set(hits);
          show_dropdown.set(true);
          loading.set(false);
        }
      });
    }
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
      search_gen.update(|g| *g += 1);
    }
  };

  // ── Input handlers ──────────────────────────────────────────────────
  let on_input = {
    let schedule_search = schedule_search.clone();
    move |ev: web_sys::Event| {
      if let Some(v) = input_value(&ev) {
        query.set(v);
        schedule_search();
      }
    }
  };

  let move_cursor = move |dir: i32| {
    let len = results.get_untracked().len() as i32;
    let next = cursor.get_untracked() as i32 + dir;
    cursor.set(next.clamp(0, (len - 1).max(0)) as usize);
  };

  let select_drug = move |drug: DrugResult| {
    search_gen.update(|g| *g += 1);
    query.set(format!("{} — {}", drug.code(), drug.name()));
    on_select.run(drug.code().to_owned());
    show_dropdown.set(false);
  };

  let select_current = move || {
    if let Some(drug) = results.get_untracked().get(cursor.get_untracked()) {
      select_drug(drug.clone());
    }
  };

  let on_keydown = move |ev: KeyboardEvent| match ev.key().as_str() {
    "Escape" => show_dropdown.set(false),
    "ArrowDown" => {
      ev.prevent_default();
      move_cursor(1);
    }
    "ArrowUp" => {
      ev.prevent_default();
      move_cursor(-1);
    }
    "Enter" => {
      ev.prevent_default();
      select_current();
    }
    _ => {}
  };

  let clear = move |_| {
    search_gen.update(|g| *g += 1);
    query.set(String::new());
    results.set(Vec::new());
    show_dropdown.set(false);
  };

  // ── Click-outside dismissal ─────────────────────────────────────────
  // The listener lives for the whole app (the panel is always mounted), so
  // the closure is intentionally leaked, matching the timers.rs precedent.
  {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
      let closure = Closure::wrap({
        Box::new(move |ev: MouseEvent| {
          let Some(target) = ev.target() else { return };
          let Some(root) = root_ref.get_untracked() else {
            return;
          };
          let Ok(node) = target.dyn_into::<web_sys::Node>() else {
            return;
          };
          if !root.contains(Some(&node)) {
            show_dropdown.set(false);
          }
        }) as Box<dyn FnMut(MouseEvent)>
      });
      let _ = doc.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref());
      closure.forget();
    }
  }

  view! {
      <div class="search-panel" node_ref=root_ref>
          <div class="search-input-wrap">
              <Icon kind=IconKind::Search class="search-icon" size=14 />
              <input
                  class="input search-input"
                  placeholder=placeholder
                  autocomplete="off"
                  prop:value=move || query.get()
                  on:input=on_input
                  on:focus=move |_| show_dropdown.set(true)
                  on:keydown=on_keydown
              />
              <Show when=move || !query.get().is_empty()>
                  <button class="btn-clear" on:click=clear>
                      <Icon kind=IconKind::X size=12 />
                  </button>
              </Show>
          </div>

          <Show when=move || { show_dropdown.get() && (!results.get().is_empty() || loading.get()) }>
              <div class="dropdown">
                  <Show when=move || loading.get()>
                      <div class="dropdown-loading">
                          <span class="animate-pulse">"กำลังค้นหา…"</span>
                      </div>
                  </Show>
                  <Show when=move || !loading.get()>
                      <ForEnumerate
                          each=move || results.get()
                          key=|d| d.code().to_owned()
                          let(index, item)
                      >
                          <button
                              class="dropdown-item"
                              class:active=move || cursor.get() == index.get()
                              on:mouseenter=move |_| cursor.set(index.get_untracked())
                              on:click=move |_| select_drug(item.clone())
                          >
                              <span class="drug-code font-mono">{item.code().to_owned()}</span>
                              <span class="drug-name">{item.name().to_owned()}</span>
                          </button>
                      </ForEnumerate>
                  </Show>
              </div>
          </Show>
      </div>
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
