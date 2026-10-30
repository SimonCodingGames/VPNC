//! Configuration for vpnc-control.

use anyhow::Context;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use vpnc_common::{Secret, TelemetryConfig};

/// Top-level control plane configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlConfig {
    /// HTTP listen address.
    pub http_bind: String,
    /// gRPC listen address.
    pub grpc_bind: String,
    /// Database URL (`sqlite:...` or `postgres://...`).
    pub database_url: String,
    /// Master key for encrypting secrets at rest (32 bytes hex or raw).
    #[serde(default, skip_serializing)]
    pub master_key: Option<String>,
    /// Session lifetime in hours.
    pub session_hours: i64,
    /// When true, session cookies use the Secure flag.
    pub cookie_secure: bool,
    /// Telemetry / logging.
    pub telemetry: TelemetryConfig,
}

impl Default for ControlConfig {
    fn default() -> Self {
        Self {
            http_bind: "127.0.0.1:8080".into(),
            grpc_bind: "127.0.0.1:50051".into(),
            database_url: "sqlite://vpnc.db?mode=rwc".into(),
            master_key: None,
            session_hours: 24,
            cookie_secure: false,
            telemetry: TelemetryConfig::default(),
        }
    }
}

impl ControlConfig {
    /// Loads config from optional file path and `VPNC_*` environment.
    pub fn load(path: Option<&std::path::Path>) -> Result<Self, vpnc_common::ConfigError> {
        vpnc_common::load_config("control", path, Self::default())
    }

    /// Returns the master key as a [`Secret`].
    pub fn require_master_key(&self) -> anyhow::Result<Secret<[u8; 32]>> {
        if let Some(raw) = &self.master_key {
            let key = parse_master_key(raw).context("invalid master_key in config")?;
            return Ok(Secret::new(key));
        }
        if let Ok(env) = std::env::var("VPNC_MASTER_KEY") {
            let key = parse_master_key(&env).context("invalid VPNC_MASTER_KEY")?;
            return Ok(Secret::new(key));
        }
        tracing::warn!("VPNC_MASTER_KEY unset; using ephemeral in-memory key (dev only)");
        let mut key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        Ok(Secret::new(key))
    }
}

fn parse_master_key(raw: &str) -> anyhow::Result<[u8; 32]> {
    let trimmed = raw.trim();
    if let Ok(bytes) = hex::decode(trimmed) {
        if bytes.len() == 32 {
            let mut out = [0u8; 32];
            out.copy_from_slice(&bytes);
            return Ok(out);
        }
    }
    let bytes = trimmed.as_bytes();
    if bytes.len() == 32 {
        let mut out = [0u8; 32];
        out.copy_from_slice(bytes);
        return Ok(out);
    }
    anyhow::bail!("VPNC_MASTER_KEY must be 32 raw bytes or 64 hex chars");
}
