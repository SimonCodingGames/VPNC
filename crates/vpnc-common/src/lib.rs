//! Shared types, config, errors, and tracing setup for VPNC.

#![deny(missing_docs)]

pub mod config;
pub mod error;
pub mod node_config;
pub mod secret;
pub mod telemetry;
pub mod tunnel;
pub mod types;

pub use config::{load_config, ConfigError};
pub use error::Error;
pub use node_config::{ClassicPeerConfig, ClassicWgConfig};
pub use secret::Secret;
pub use telemetry::{init_telemetry, TelemetryConfig};
pub use tunnel::{
    CLIENT_ALLOWED_IPS, CLIENT_DNS, DEFAULT_TUNNEL_CIDR_V4, DEFAULT_TUNNEL_CIDR_V6, WG_INTERFACE,
    WG_LISTEN_PORT, WG_MTU,
};
pub use types::{DeviceId, NodeId, NodeRole, ProfileKind, UserId};
