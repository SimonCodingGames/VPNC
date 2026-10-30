//! VPNC control plane library (HTTP API, database, services).

#![allow(clippy::result_large_err)] // tonic::Status / generated gRPC traits
#![allow(clippy::type_complexity)] // sqlx query_as row tuples
#![allow(clippy::too_many_arguments)]
#![allow(clippy::field_reassign_with_default)]

pub mod api;
pub mod app;
pub mod config;
pub mod db;
pub mod grpc;
pub mod provisioning;
pub mod services;

pub use app::{bind_addr, build_app, build_state, AppState};
pub use config::ControlConfig;
