//! Database-connection configuration state and actions.
//!
//! Mirrors the Pinia `dbConfig` store: owns the HOSxP / INVS connection
//! settings, the per-side connection state, the settings-drawer tab, and the
//! save feedback.  All backend communication goes through [`crate::services`];
//! this module never touches `invoke` directly.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::models::{HosxpDbConfig, InvsDbConfig};
use crate::services::commands;
use crate::services::timers::set_timeout_ms;

/// Which database tab is active inside the settings drawer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
  Hosxp,
  Invs,
}

/// Shared connection-config state, exposed through Leptos context.
///
/// Every field is a plain `RwSignal`: the struct itself is `Copy`, so it can
/// be passed to child components by value.
#[derive(Clone, Copy, Debug)]
pub struct DbConfigContext {
  /// HOSxP (MySQL) connection settings (editable in the drawer).
  pub hosxp_config: RwSignal<HosxpDbConfig>,
  /// Whether a HOSxP connection is currently established.
  pub hosxp_connected: RwSignal<bool>,
  /// Whether a HOSxP connection attempt is in flight.
  pub hosxp_connecting: RwSignal<bool>,
  /// Last HOSxP connect error (displayed in the drawer).
  pub hosxp_error: RwSignal<Option<String>>,
  /// HOSxP was connected at least once and a health ping just failed.
  pub hosxp_lost: RwSignal<bool>,
  /// INVS (SQL Server) connection settings (editable in the drawer).
  pub invs_config: RwSignal<InvsDbConfig>,
  /// Whether an INVS connection is currently established.
  pub invs_connected: RwSignal<bool>,
  /// Whether an INVS connection attempt is in flight.
  pub invs_connecting: RwSignal<bool>,
  /// Last INVS connect error (displayed in the drawer).
  pub invs_error: RwSignal<Option<String>>,
  /// INVS was connected at least once and a health ping just failed.
  pub invs_lost: RwSignal<bool>,
  /// Active tab of the settings drawer.
  pub active_tab: RwSignal<SettingsTab>,
  /// Whether a save operation is in flight (disables the save buttons).
  pub saving: RwSignal<bool>,
  /// Save feedback message (`บันทึกสำเร็จ` or the backend error), auto-cleared.
  pub save_message: RwSignal<Option<String>>,
  /// Whether the health-poll loop is running (guards against double-start).
  polling: RwSignal<bool>,
  /// Whether a poll tick is still in flight (a slow INVS reconnect must not
  /// overlap the next tick).
  tick_running: RwSignal<bool>,
  /// Whether each side has ever been connected (only those sides are pinged).
  ever_hosxp: RwSignal<bool>,
  /// See [`Self::ever_hosxp`].
  ever_invs: RwSignal<bool>,
}

impl DbConfigContext {
  /// Create the signals, register them in context, and return the handle.
  #[must_use]
  pub fn provide() -> Self {
    let ctx = Self {
      hosxp_config: RwSignal::new(HosxpDbConfig::default()),
      hosxp_connected: RwSignal::new(false),
      hosxp_connecting: RwSignal::new(false),
      hosxp_error: RwSignal::new(None),
      hosxp_lost: RwSignal::new(false),
      invs_config: RwSignal::new(InvsDbConfig::default()),
      invs_connected: RwSignal::new(false),
      invs_connecting: RwSignal::new(false),
      invs_error: RwSignal::new(None),
      invs_lost: RwSignal::new(false),
      active_tab: RwSignal::new(SettingsTab::Hosxp),
      saving: RwSignal::new(false),
      save_message: RwSignal::new(None),
      polling: RwSignal::new(false),
      tick_running: RwSignal::new(false),
      ever_hosxp: RwSignal::new(false),
      ever_invs: RwSignal::new(false),
    };
    provide_context(ctx);
    ctx
  }

  /// Whether at least one database is connected (drives the no-connection banner).
  #[must_use]
  pub fn any_connected(self) -> Memo<bool> {
    Memo::new(move |_| self.hosxp_connected.get() || self.invs_connected.get())
  }

  /// Whether a previously-good connection has just failed a health ping
  /// (drives the "connection lost" banner).
  #[must_use]
  pub fn any_lost(self) -> Memo<bool> {
    Memo::new(move |_| self.hosxp_lost.get() || self.invs_lost.get())
  }

  /// Test the HOSxP connection with the current settings.
  ///
  /// Returns `true` on success and stores the outcome in the signals.
  pub async fn connect_hosxp(self) -> bool {
    self.hosxp_connecting.set(true);
    self.hosxp_error.set(None);
    let cfg = self.hosxp_config.get_untracked();
    let ok = match commands::hosxp_connect(&cfg).await {
      Ok(()) => {
        self.ever_hosxp.set(true);
        self.hosxp_lost.set(false);
        self.hosxp_connected.set(true);
        true
      }
      Err(e) => {
        self.hosxp_connected.set(false);
        self.hosxp_error.set(Some(e.message));
        false
      }
    };
    self.hosxp_connecting.set(false);
    ok
  }

  /// Test the INVS connection with the current settings.
  ///
  /// Returns `true` on success and stores the outcome in the signals.
  pub async fn connect_invs(self) -> bool {
    self.invs_connecting.set(true);
    self.invs_error.set(None);
    let cfg = self.invs_config.get_untracked();
    let ok = match commands::invs_connect(&cfg).await {
      Ok(()) => {
        self.ever_invs.set(true);
        self.invs_lost.set(false);
        self.invs_connected.set(true);
        true
      }
      Err(e) => {
        self.invs_connected.set(false);
        self.invs_error.set(Some(e.message));
        false
      }
    };
    self.invs_connecting.set(false);
    ok
  }

  // ── Connection health ──────────────────────────────────────────────

  /// Start the background health-poll loop: pings every connected side
  /// every `interval_ms`; a failed ping flips the side to "lost" (badge +
  /// banner), a successful one restores it.  The MySQL pool heals itself
  /// (test_before_acquire), so the HOSxP side recovers by itself; the INVS
  /// side is a single TCP client that cannot heal, so a failed ping also
  /// retries `connect_invs` with the saved config.  Idempotent.
  pub fn start_health_polling(self, interval_ms: i32) {
    if self.polling.get_untracked() {
      return;
    }
    self.polling.set(true);
    self.poll_tick(interval_ms);
  }

  fn poll_tick(self, interval_ms: i32) {
    set_timeout_ms(
      move || {
        spawn_local(async move {
          if !self.tick_running.get_untracked() {
            self.tick_running.set(true);
            self.poll_once().await;
            self.tick_running.set(false);
          }
          self.poll_tick(interval_ms);
        });
      },
      interval_ms,
    );
  }

  async fn poll_once(self) {
    if self.ever_hosxp.get_untracked() {
      match commands::hosxp_ping().await {
        Ok(()) => {
          if self.hosxp_lost.get_untracked() {
            // The pool healed itself — connection is back.
            self.hosxp_lost.set(false);
            self.hosxp_connected.set(true);
          }
        }
        Err(e) => {
          if !self.hosxp_lost.get_untracked() {
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
              "HOSxP connection lost: {}",
              e.message
            )));
          }
          self.hosxp_lost.set(true);
          self.hosxp_connected.set(false);
        }
      }
    }

    if self.ever_invs.get_untracked() {
      match commands::invs_ping().await {
        Ok(()) => {
          if self.invs_lost.get_untracked() {
            self.invs_lost.set(false);
            self.invs_connected.set(true);
          }
        }
        Err(e) => {
          if !self.invs_lost.get_untracked() {
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
              "INVS connection lost: {}",
              e.message
            )));
          }
          self.invs_lost.set(true);
          self.invs_connected.set(false);
          // The single tiberius client is dead; reconnect from saved config.
          let cfg = self.invs_config.get_untracked();
          if !cfg.user.trim().is_empty() && !self.invs_connecting.get_untracked() {
            let _ = self.connect_invs().await;
          }
        }
      }
    }
  }

  /// Persist both configs via the encrypted Tauri settings, then show the
  /// appropriate feedback message (auto-cleared after 3s on success, 5s on
  /// error — matching the original Vue store).
  pub async fn save_settings(self) -> bool {
    self.saving.set(true);
    self.save_message.set(None);
    let hosxp = self.hosxp_config.get_untracked();
    let invs = self.invs_config.get_untracked();
    let invs = if invs.user.trim().is_empty() {
      None
    } else {
      Some(invs)
    };
    let ok = match commands::save_settings(&hosxp, invs.as_ref()).await {
      Ok(()) => {
        self.save_message.set(Some("บันทึกสำเร็จ".to_owned()));
        true
      }
      Err(e) => {
        self.save_message.set(Some(e.message));
        false
      }
    };
    self.saving.set(false);
    let msg = self.save_message.get_untracked().unwrap_or_default();
    let clear_ms = if ok { 3000 } else { 5000 };
    set_timeout_ms(
      move || {
        // Only clear if the message is still the one we just set (a newer
        // save may have replaced it while the timer was pending).
        if self.save_message.get_untracked().as_deref() == Some(msg.as_str()) {
          self.save_message.set(None);
        }
      },
      clear_ms,
    );
    ok
  }

  /// Load persisted settings from storage, then auto-connect both databases
  /// for which a username is stored.
  pub async fn init_from_storage(self) {
    match commands::load_settings().await {
      Ok(settings) => {
        self.hosxp_config.set(settings.hosxp);
        if let Some(invs) = settings.invs {
          self.invs_config.set(invs);
        }
      }
      // No saved settings yet — keep the defaults.
      Err(e) => {
        web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
          "load_settings: {}",
          e.message
        )));
      }
    }

    if !self.hosxp_config.get_untracked().user.trim().is_empty() {
      let _ = self.connect_hosxp().await;
    }
    if !self.invs_config.get_untracked().user.trim().is_empty() {
      let _ = self.connect_invs().await;
    }
  }
}
