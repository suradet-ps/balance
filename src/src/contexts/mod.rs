//! Application state contexts (the Leptos equivalent of the Pinia stores).

pub mod dashboard;
pub mod db_config;

pub use dashboard::DashboardContext;
pub use db_config::{DbConfigContext, SettingsTab};
