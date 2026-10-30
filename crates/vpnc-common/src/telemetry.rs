//! Tracing setup: pretty logs in development, JSON in production.

use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

/// Telemetry / logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Log filter directive, e.g. `info,vpnc_control=debug`.
    pub filter: String,
    /// When true, emit JSON logs (production). Otherwise pretty (development).
    pub json: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            filter: "info".into(),
            json: false,
        }
    }
}

/// Initializes the global tracing subscriber.
///
/// Safe to call once at process start. Subsequent calls return an error from
/// `tracing_subscriber` if a subscriber is already set; that error is ignored
/// so tests can call this repeatedly.
pub fn init_telemetry(config: &TelemetryConfig) {
    let filter = EnvFilter::try_new(&config.filter)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let result = if config.json {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .try_init()
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .pretty()
            .try_init()
    };

    if let Err(err) = result {
        tracing::debug!(error = %err, "tracing subscriber already initialized");
    }
}
