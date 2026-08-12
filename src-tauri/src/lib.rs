//! Tauri application entry-point.
//!
//! Wires up the plugin stack, registers IPC command handlers for both
//! HOSxP (MySQL) and INVS (SQL Server) backends.

mod hosxp;
mod invs;
mod mapping;
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
            let store = store::open_store(app.handle())
                .map_err(|e| std::io::Error::other(e))?;
            app.manage(store);
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
            // INVS (SQL Server) commands
            invs::commands::invs_connect,
            invs::commands::invs_get_available_years,
            invs::commands::invs_get_top_drugs_by_value,
            invs::commands::invs_get_drug_monthly_value,
            invs::commands::invs_get_drug_list,
            invs::commands::invs_get_year_summary,
        ])
        .run(tauri::generate_context!())
        .expect("invariant: tauri context is generated at compile time and is always valid");
}
