//! Encrypted settings persistence.
//!
//! Provides [`VaultState`] for Tauri state management and
//! `save_settings` / `load_settings` IPC commands that encrypt
//! config values to disk via the OS keychain-backed master key.

use crate::hosxp::db::HosxpDbConfig;
use crate::invs::db::InvsDbConfig;
use encryptman_keyring::Vault;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

pub struct VaultState(pub Vault);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsFile {
    pub hosxp: HosxpDbConfig,
    pub invs: Option<InvsDbConfig>,
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("cannot resolve app data dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create app data dir: {e}"))?;
    dir.push("settings.json");
    Ok(dir)
}

#[tauri::command]
pub async fn save_settings(
    app: tauri::AppHandle,
    vault: tauri::State<'_, VaultState>,
    hosxp: HosxpDbConfig,
    invs: Option<InvsDbConfig>,
) -> Result<(), String> {
    let encrypted = SettingsFile {
        hosxp: hosxp.encrypt(&vault.0)?,
        invs: invs
            .map(|c| c.encrypt(&vault.0))
            .transpose()?,
    };
    let path = settings_path(&app)?;
    let json = serde_json::to_string_pretty(&encrypted)
        .map_err(|e| format!("serialization failed: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("write failed: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn load_settings(
    app: tauri::AppHandle,
    vault: tauri::State<'_, VaultState>,
) -> Result<SettingsFile, String> {
    let path = settings_path(&app)?;
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read settings file: {e}"))?;
    let encrypted: SettingsFile =
        serde_json::from_str(&json).map_err(|e| format!("parse failed: {e}"))?;
    let plain = SettingsFile {
        hosxp: encrypted.hosxp.decrypt(&vault.0)?,
        invs: encrypted
            .invs
            .map(|c| c.decrypt(&vault.0))
            .transpose()?,
    };
    Ok(plain)
}
