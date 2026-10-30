//! VPNC control plane library (HTTP API, database, services).

pub mod api;
pub mod app;
pub mod config;
pub mod db;
pub mod grpc;
pub mod provisioning;
pub mod services;

pub use app::{bind_addr, build_app, build_state, AppState};
pub use config::ControlConfig;
