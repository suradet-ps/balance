//! Tauri application entry-point.
//!
//! Wires up the plugin stack, registers IPC command handlers for both
//! HOSxP (MySQL) and INVS (SQL Server) backends.

mod hosxp;
mod invs;

use hosxp::db::HosxpDbState;
use invs::db::InvsDbState;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(HosxpDbState::new())
        .manage(InvsDbState(Arc::new(Mutex::new(None))))
        .invoke_handler(tauri::generate_handler![
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
