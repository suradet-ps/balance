//! Service layer: Tauri IPC glue and typed command wrappers.
//!
//! [`tauri`] reaches `window.__TAURI__` (loaded at runtime by `index.html`),
//! [`commands`] exposes one typed wrapper per backend command, and [`timers`]
//! provides one-shot timer helpers.  No UI code talks to `invoke` directly.

pub mod commands;
pub mod tauri;
pub mod timers;
