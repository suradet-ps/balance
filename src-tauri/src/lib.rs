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
