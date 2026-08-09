//! Connection-settings drawer — port of `ConnectionSettings.vue`.
//!
//! Slide-over panel with a HOSxP / INVS tab, per-side connection forms,
//! password visibility toggles, the ทดสอบ (test) / บันทึก (save) actions and
//! the save feedback line.  All state lives in the [`DbConfigContext`]; the
//! drawer only owns the password-visibility toggles.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

use crate::components::icons::{Icon, IconKind};
use crate::contexts::{DbConfigContext, SettingsTab};

/// Props for [`ConnectionSettings`].
#[component]
pub fn ConnectionSettings(
  /// Whether the drawer is open.
  visible: RwSignal<bool>,
  /// Emitted when the drawer asks to close.
  on_close: Callback<()>,
) -> impl IntoView {
  let db = expect_context::<DbConfigContext>();

  let show_hosxp_pw = RwSignal::new(false);
  let show_invs_pw = RwSignal::new(false);

  let close = move || on_close.run(());

  let connect_hosxp = {
    let db = db;
    let close = close;
    move |_| {
      let db = db;
      let close = close;
      spawn_local(async move {
        // The original drawer closes itself on a successful test.
        if db.connect_hosxp().await {
          close();
        }
      });
    }
  };

  let connect_invs = {
    let db = db;
    let close = close;
    move |_| {
      let db = db;
      let close = close;
      spawn_local(async move {
        if db.connect_invs().await {
          close();
        }
      });
    }
  };

  let save_hosxp = {
    let db = db;
    move |_| {
      let db = db;
      spawn_local(async move {
        let _ = db.save_settings().await;
      });
    }
  };

  let save_invs = save_hosxp.clone();

  let on_overlay = move |ev: web_sys::MouseEvent| {
    // Click on the dimmed backdrop (not the panel) closes the drawer.
    if let (Some(target), Some(current)) = (ev.target(), ev.current_target()) {
      if target == current {
        close();
      }
    }
  };

  view! {
      <Show when=move || visible.get()>
          <div class="drawer-overlay" on:click=on_overlay>
              <div class="drawer-panel">
                  <div class="drawer-header">
                      <span class="drawer-title">
                          <Icon kind=IconKind::Settings2 size=16 />
                          "ตั้งค่าการเชื่อมต่อฐานข้อมูล"
                      </span>
                      <button class="btn-icon" on:click=move |_| close()>
                          <Icon kind=IconKind::X size=16 />
                      </button>
                  </div>

                  <div class="tab-bar">
                      <button
                          class="tab-btn"
                          class:active=move || db.active_tab.get() == SettingsTab::Hosxp
                          on:click=move |_| db.active_tab.set(SettingsTab::Hosxp)
                      >
                          <Icon kind=IconKind::Database size=14 />
                          "HOSxP (MySQL)"
                      </button>
                      <button
                          class="tab-btn"
                          class:active=move || db.active_tab.get() == SettingsTab::Invs
                          on:click=move |_| db.active_tab.set(SettingsTab::Invs)
                      >
                          <Icon kind=IconKind::Database size=14 />
                          "INVS (SQL Server)"
                      </button>
                  </div>

                  <Show when=move || db.active_tab.get() == SettingsTab::Hosxp>
                      <div class="form-section">
                          <div class="status-row">
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
                                  {move || {
                                      if db.hosxp_connected.get() {
                                          "เชื่อมต่อแล้ว"
                                      } else {
                                          "ยังไม่ได้เชื่อมต่อ"
                                      }
                                  }}
                              </span>
                          </div>

                          <div class="form-grid">
                              <div class="form-group">
                                  <label class="form-label">"Host / IP"</label>
                                  <input
                                      class="input"
                                      placeholder="localhost"
                                      autocomplete="off"
                                      prop:value=move || db.hosxp_config.get().host
                                      on:input=bind_text(db.hosxp_config, |c, v| c.host = v)
                                  />
                              </div>
                              <div class="form-group form-group--half">
                                  <label class="form-label">"Port"</label>
                                  <input
                                      class="input"
                                      type="number"
                                      placeholder="3306"
                                      prop:value=move || db.hosxp_config.get().port
                                      on:input=bind_text(db.hosxp_config, |c, v| c.port = v)
                                  />
                              </div>
                              <div class="form-group form-group--half">
                                  <label class="form-label">"Database"</label>
                                  <input
                                      class="input"
                                      placeholder="hospdb"
                                      prop:value=move || db.hosxp_config.get().database
                                      on:input=bind_text(db.hosxp_config, |c, v| c.database = v)
                                  />
                              </div>
                              <div class="form-group">
                                  <label class="form-label">"Username"</label>
                                  <input
                                      class="input"
                                      placeholder="hosxp_user"
                                      autocomplete="username"
                                      prop:value=move || db.hosxp_config.get().user
                                      on:input=bind_text(db.hosxp_config, |c, v| c.user = v)
                                  />
                              </div>
                              <div class="form-group">
                                  <label class="form-label">"Password"</label>
                                  <div class="password-wrap">
                                      <input
                                          class="input"
                                          placeholder="••••••••"
                                          autocomplete="current-password"
                                          type=move || if show_hosxp_pw.get() { "text" } else { "password" }
                                          prop:value=move || db.hosxp_config.get().password
                                          on:input=bind_text(db.hosxp_config, |c, v| c.password = v)
                                      />
                                      <button
                                          class="btn-icon small"
                                          on:click=move |_| show_hosxp_pw.update(|v| *v = !*v)
                                      >
                                          <Show when=move || show_hosxp_pw.get()>
                                              <Icon kind=IconKind::EyeOff size=14 />
                                          </Show>
                                          <Show when=move || !show_hosxp_pw.get()>
                                              <Icon kind=IconKind::Eye size=14 />
                                          </Show>
                                      </button>
                                  </div>
                              </div>
                          </div>

                          <Show when=move || db.hosxp_error.get().is_some()>
                              <div class="error-box">
                                  <Icon kind=IconKind::AlertTriangle size=14 />
                                  {move || db.hosxp_error.get().unwrap_or_default()}
                              </div>
                          </Show>

                          <div class="drawer-actions">
                              <button class="btn btn-ghost" on:click=move |_| close()>
                                  "ยกเลิก"
                              </button>
                              <button
                                  class="btn btn-secondary"
                                  disabled=move || db.hosxp_connecting.get() || db.saving.get()
                                  on:click=save_hosxp
                              >
                                  <Icon kind=IconKind::Save size=14 />
                                  "บันทึก"
                              </button>
                              <button
                                  class="btn btn-primary"
                                  disabled=move || db.hosxp_connecting.get()
                                  on:click=connect_hosxp
                              >
                                  <Show when=move || db.hosxp_connecting.get()>
                                      <span class="animate-pulse">"กำลังเชื่อมต่อ…"</span>
                                  </Show>
                                  <Show when=move || !db.hosxp_connecting.get()>
                                      <Icon kind=IconKind::PlugZap size=14 />
                                      "ทดสอบ"
                                  </Show>
                              </button>
                          </div>
                          <SaveFeedback db=db />
                      </div>
                  </Show>

                  <Show when=move || db.active_tab.get() == SettingsTab::Invs>
                      <div class="form-section">
                          <div class="status-row">
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
                                  {move || {
                                      if db.invs_connected.get() {
                                          "เชื่อมต่อแล้ว"
                                      } else {
                                          "ยังไม่ได้เชื่อมต่อ"
                                      }
                                  }}
                              </span>
                          </div>

                          <div class="form-grid">
                              <div class="form-group">
                                  <label class="form-label">"Server / Host IP"</label>
                                  <input
                                      class="input"
                                      placeholder="192.168.1.10"
                                      autocomplete="off"
                                      prop:value=move || db.invs_config.get().host
                                      on:input=bind_text(db.invs_config, |c, v| c.host = v)
                                  />
                              </div>
                              <div class="form-group form-group--half">
                                  <label class="form-label">"Port"</label>
                                  <input
                                      class="input"
                                      type="number"
                                      placeholder="1433"
                                      prop:value=move || db.invs_config.get().port
                                      on:input=bind_text(db.invs_config, |c, v| c.port = v)
                                  />
                              </div>
                              <div class="form-group form-group--half">
                                  <label class="form-label">"Named Instance"</label>
                                  <input
                                      class="input"
                                      placeholder="(เว้นว่างถ้าไม่มี)"
                                      autocomplete="off"
                                      prop:value=move || db.invs_config.get().instance
                                      on:input=bind_text(db.invs_config, |c, v| c.instance = v)
                                  />
                              </div>
                              <div class="form-group">
                                  <label class="form-label">"Database"</label>
                                  <input
                                      class="input"
                                      placeholder="INVS"
                                      prop:value=move || db.invs_config.get().database
                                      on:input=bind_text(db.invs_config, |c, v| c.database = v)
                                  />
                              </div>
                              <div class="form-group">
                                  <label class="form-label">"Username"</label>
                                  <input
                                      class="input"
                                      placeholder="sa"
                                      autocomplete="username"
                                      prop:value=move || db.invs_config.get().user
                                      on:input=bind_text(db.invs_config, |c, v| c.user = v)
                                  />
                              </div>
                              <div class="form-group">
                                  <label class="form-label">"Password"</label>
                                  <div class="password-wrap">
                                      <input
                                          class="input"
                                          placeholder="••••••••"
                                          autocomplete="current-password"
                                          type=move || if show_invs_pw.get() { "text" } else { "password" }
                                          prop:value=move || db.invs_config.get().password
                                          on:input=bind_text(db.invs_config, |c, v| c.password = v)
                                      />
                                      <button
                                          class="btn-icon small"
                                          on:click=move |_| show_invs_pw.update(|v| *v = !*v)
                                      >
                                          <Show when=move || show_invs_pw.get()>
                                              <Icon kind=IconKind::EyeOff size=14 />
                                          </Show>
                                          <Show when=move || !show_invs_pw.get()>
                                              <Icon kind=IconKind::Eye size=14 />
                                          </Show>
                                      </button>
                                  </div>
                              </div>
                          </div>

                          <Show when=move || db.invs_error.get().is_some()>
                              <div class="error-box">
                                  <Icon kind=IconKind::AlertTriangle size=14 />
                                  {move || db.invs_error.get().unwrap_or_default()}
                              </div>
                          </Show>

                          <div class="drawer-actions">
                              <button class="btn btn-ghost" on:click=move |_| close()>
                                  "ยกเลิก"
                              </button>
                              <button
                                  class="btn btn-secondary"
                                  disabled=move || db.invs_connecting.get() || db.saving.get()
                                  on:click=save_invs
                              >
                                  <Icon kind=IconKind::Save size=14 />
                                  "บันทึก"
                              </button>
                              <button
                                  class="btn btn-primary"
                                  disabled=move || db.invs_connecting.get()
                                  on:click=connect_invs
                              >
                                  <Show when=move || db.invs_connecting.get()>
                                      <span class="animate-pulse">"กำลังเชื่อมต่อ…"</span>
                                  </Show>
                                  <Show when=move || !db.invs_connecting.get()>
                                      <Icon kind=IconKind::PlugZap size=14 />
                                      "ทดสอบ"
                                  </Show>
                              </button>
                          </div>
                          <SaveFeedback db=db />
                      </div>
                  </Show>
              </div>
          </div>
      </Show>
  }
}

/// The shared save-feedback line (`บันทึกสำเร็จ` or the backend error),
/// styled green on success and red otherwise — exactly like the original.
#[component]
fn SaveFeedback(db: DbConfigContext) -> impl IntoView {
  view! {
      <Show when=move || db.save_message.get().is_some()>
          <div
              class="save-feedback"
              class:save-ok=move || db.save_message.get().as_deref() == Some("บันทึกสำเร็จ")
              class:save-err=move || db.save_message.get().as_deref() != Some("บันทึกสำเร็จ")
          >
              {move || db.save_message.get().unwrap_or_default()}
          </div>
      </Show>
  }
}

/// Bind a text input to one string field of a config signal.
fn bind_text<C: Send + Sync + 'static>(
  sig: RwSignal<C>,
  mut set: impl FnMut(&mut C, String) + 'static,
) -> impl FnMut(web_sys::Event) + 'static {
  move |ev: web_sys::Event| {
    if let Some(value) = input_value(&ev) {
      sig.update(|cfg| set(cfg, value));
    }
  }
}

/// Read the current value of an `<input>` from an event.
fn input_value(ev: &web_sys::Event) -> Option<String> {
  let target = ev.target()?;
  target.dyn_into::<HtmlInputElement>().ok().map(|el| el.value())
}
