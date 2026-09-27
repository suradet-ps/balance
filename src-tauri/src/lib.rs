//! Tauri application entry-point.
//!
//! Wires up the plugin stack, registers IPC command handlers for both
//! HOSxP (MySQL) and INVS (SQL Server) backends.

mod fiscal;
mod hosxp;
mod invs;
mod mapping;
mod reconcile;
mod settings;
mod store;

use hosxp::db::HosxpDbState;
use invs::db::InvsDbState;
use settings::VaultState;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let vault =
        encryptman_keyring::Vault::new("balance").expect("failed to initialize OS keychain vault");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        // Open + migrate the local store before the UI mounts (Phase 1).
        .setup(|app| {
            let store = store::open_store(app.handle()).map_err(std::io::Error::other)?;
            app.manage(store);

            // The window starts hidden (`visible: false` in tauri.conf.json).
            // An oversized window gets clamped by the OS when it is finally
            // shown — which throws the `center` config off — so shrink the
            // window to the primary monitor's work area first (subtracting
            // the window chrome, since `set_size` sets the inner size),
            // center it, and only then reveal it.
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(Some(monitor)) = app.primary_monitor() {
                    let scale = monitor.scale_factor();
                    let work = monitor.work_area();
                    if let (Ok(outer), Ok(inner)) = (window.outer_size(), window.inner_size()) {
                        // Work area and window sizes are physical pixels; the
                        // chrome (title bar + borders) is outer − inner.
                        let chrome_w = outer.width.saturating_sub(inner.width) as f64 / scale;
                        let chrome_h = outer.height.saturating_sub(inner.height) as f64 / scale;
                        let inner_w = inner.width as f64 / scale;
                        let inner_h = inner.height as f64 / scale;
                        let max_w = work.size.width as f64 / scale - chrome_w;
                        let max_h = work.size.height as f64 / scale - chrome_h;
                        if inner_w > max_w || inner_h > max_h {
                            let width = inner_w.min(max_w).max(1.0);
                            let height = inner_h.min(max_h).max(1.0);
                            window.set_size(tauri::LogicalSize::new(width, height))?;
                        }
                    }
                }
                window.center()?;
                window.show()?;
            }
            Ok(())
        })
        .manage(HosxpDbState::new())
        .manage(InvsDbState(Arc::new(Mutex::new(None))))
        .manage(VaultState(vault))
        .invoke_handler(tauri::generate_handler![
            // Settings persistence
            settings::save_settings,
            settings::load_settings,
            // HOSxP (MySQL) commands
            hosxp::commands::hosxp_connect,
            hosxp::commands::hosxp_get_available_years,
            hosxp::commands::hosxp_get_top_drugs,
            hosxp::commands::hosxp_get_drug_monthly_qty,
            hosxp::commands::hosxp_get_drug_list,
            hosxp::commands::hosxp_get_year_summary,
            hosxp::commands::hosxp_ping,
            // INVS (SQL Server) commands
            invs::commands::invs_connect,
            invs::commands::invs_get_available_years,
            invs::commands::invs_get_top_drugs_by_value,
            invs::commands::invs_get_drug_monthly_value,
            invs::commands::invs_get_drug_list,
            invs::commands::invs_get_year_summary,
            invs::commands::invs_ping,
            // Drug mapping (Phase 1) — local store + matching workflow
            mapping::commands::mapping_status_by_icode,
            mapping::commands::mapping_status_by_working_code,
            mapping::commands::mapping_list_rows,
            mapping::commands::mapping_stats,
            mapping::commands::mapping_suggest,
            mapping::commands::mapping_set,
            mapping::commands::mapping_remove,
            mapping::commands::mapping_mark_no_invs,
            mapping::commands::mapping_unmark_no_invs,
            mapping::commands::mapping_auto_match,
            mapping::commands::mapping_bulk_import,
            // Reconciliation (Phase 2)
            reconcile::commands::reconcile_drug,
        ])
        .run(tauri::generate_context!())
        .expect("invariant: tauri context is generated at compile time and is always valid");
}
