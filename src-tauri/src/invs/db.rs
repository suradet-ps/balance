//! INVS SQL Server connection management.
//!
//! Uses `tiberius::Client` wrapped in `Arc<Mutex<Option<...>>>` stored as
//! a Tauri managed state.

use encryptman_keyring::Vault;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tiberius::{AuthMethod, Client, Config, EncryptionLevel};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InvsDbConfig {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub database: String,
    pub instance: Option<String>,
}

#[allow(dead_code)]
impl InvsDbConfig {
    pub fn encrypt(&self, vault: &Vault) -> Result<Self, String> {
        Ok(Self {
            host: vault.encrypt(&self.host).map_err(|e| e.to_string())?,
            port: vault.encrypt(&self.port).map_err(|e| e.to_string())?,
            user: vault.encrypt(&self.user).map_err(|e| e.to_string())?,
            password: vault.encrypt(&self.password).map_err(|e| e.to_string())?,
            database: vault.encrypt(&self.database).map_err(|e| e.to_string())?,
            instance: self
                .instance
                .as_ref()
                .map(|v| vault.encrypt(v))
                .transpose()
                .map_err(|e| e.to_string())?,
        })
    }

    pub fn decrypt(&self, vault: &Vault) -> Result<Self, String> {
        Ok(Self {
            host: vault.decrypt(&self.host).map_err(|e| e.to_string())?,
            port: vault.decrypt(&self.port).map_err(|e| e.to_string())?,
            user: vault.decrypt(&self.user).map_err(|e| e.to_string())?,
            password: vault.decrypt(&self.password).map_err(|e| e.to_string())?,
            database: vault.decrypt(&self.database).map_err(|e| e.to_string())?,
            instance: self
                .instance
                .as_ref()
                .map(|v| vault.decrypt(v))
                .transpose()
                .map_err(|e| e.to_string())?,
        })
    }
}

pub struct InvsDbState(pub Arc<Mutex<Option<Client<Compat<TcpStream>>>>>);

pub async fn connect(cfg: &InvsDbConfig) -> Result<Client<Compat<TcpStream>>, String> {
    let mut config = Config::new();

    config.host(&cfg.host);
    config.port(cfg.port.parse().map_err(|e| format!("invalid port: {e}"))?);
    config.authentication(AuthMethod::sql_server(&cfg.user, &cfg.password));
    config.database(&cfg.database);
    config.encryption(EncryptionLevel::NotSupported);
    config.trust_cert();

    if let Some(inst) = cfg.instance.as_ref().filter(|i| !i.is_empty()) {
        config.instance_name(inst);
    }

    let tcp = TcpStream::connect(config.get_addr())
        .await
        .map_err(|e| format!("TCP connect failed: {e}"))?;

    tcp.set_nodelay(true)
        .map_err(|e| format!("set_nodelay failed: {e}"))?;

    let client = Client::connect(config, tcp.compat_write())
        .await
        .map_err(|e| format!("SQL Server connect failed: {e}"))?;

    Ok(client)
}
