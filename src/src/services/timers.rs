//! Sidebar/utility helpers over `web-sys` (non-Tauri DOM glue).

use js_sys::Function;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;

/// Schedule `cb` to run after `ms` milliseconds.
///
/// The underlying `setTimeout` handler is created once per call and leaked
/// intentionally: the JS timer keeps a reference to the wasm trampoline, and a
/// `Closure` dropped while the timer is still pending would panic when the
/// timer fires.  For the few one-shot timers used by this app (save-feedback
/// auto-clear, the post-mount refresh) the leak is negligible and matches the
/// pdf-splitter precedent of intentionally-leaked page-lifetime closures.
pub fn set_timeout_ms(cb: impl FnMut() + 'static, ms: i32) {
  let closure = Closure::wrap(Box::new(cb) as Box<dyn FnMut()>);
  if let Some(win) = web_sys::window() {
    let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
      closure.as_ref().unchecked_ref::<Function>(),
      ms,
    );
  }
  closure.forget();
}
