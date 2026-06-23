//! HOSxP MySQL connection pool management.
//!
//! Uses a global `OnceLock<RwLock<Option<MySqlPool>>>` so multiple Tauri
//! commands can share the pool concurrently.

use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::RwLock;

static POOL: OnceLock<RwLock<Option<MySqlPool>>> = OnceLock::new();

fn get_pool_lock() -> &'static RwLock<Option<MySqlPool>> {
    POOL.get_or_init(|| RwLock::new(None))
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct HosxpDbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

/// Managed state wrapper — empty struct, commands use `with_pool()` instead.
pub struct HosxpDbState;

impl HosxpDbState {
    pub fn new() -> Self {
        Self
    }
}

/// Percent-encode characters meaningful in a MySQL DSN.
fn urlencoding_simple(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '@' => out.push_str("%40"),
            ':' => out.push_str("%3A"),
            '/' => out.push_str("%2F"),
            ' ' => out.push_str("%20"),
            '#' => out.push_str("%23"),
            _ => out.push(c),
        }
    }
    out
}

/// Initialise (or replace) the global MySQL connection pool.
pub async fn init_pool(config: HosxpDbConfig) -> Result<(), String> {
    let url = format!(
        "mysql://{}:{}@{}:{}/{}",
        urlencoding_simple(&config.user),
        urlencoding_simple(&config.password),
        config.host,
        config.port,
        config.database,
    );

    let pool = MySqlPoolOptions::new()
        .max_connections(12)
        .min_connections(3)
        .acquire_timeout(Duration::from_secs(20))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(3600))
        .test_before_acquire(false)
        .connect(&url)
        .await
        .map_err(|e| format!("HOSxP DB connection failed: {}", e))?;

    let mut guard = get_pool_lock().write().await;
    *guard = Some(pool);
    Ok(())
}

/// Acquire a read-lock on the pool, then call `f` with a reference to it.
pub async fn with_pool<F, T, E>(f: F) -> Result<T, String>
where
    F: for<'a> FnOnce(
        &'a MySqlPool,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'a>,
    >,
    E: std::fmt::Display,
{
    let guard = get_pool_lock().read().await;
    match guard.as_ref() {
        Some(pool) => f(pool).await.map_err(|e| e.to_string()),
        None => Err("HOSxP database not connected. Please configure connection settings.".to_string()),
    }
}
