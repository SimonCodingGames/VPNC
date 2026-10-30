//! Configuration for vpnc-control.

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
    /// Master key for encrypting secrets at rest (64 hex chars or base64 of 32 bytes).
    #[serde(default, skip_serializing)]
    pub master_key: Option<String>,
    /// Session lifetime in hours.
    pub session_hours: i64,
    /// When true, session cookies use the Secure flag.
    pub cookie_secure: bool,
    /// Directory with built dashboard static files (`index.html`, `200.html`, …).
    #[serde(default)]
    pub static_dir: Option<PathBuf>,
    /// Trust `X-Forwarded-*` headers (only behind a reverse proxy).
    #[serde(default)]
    pub trust_forwarded_headers: bool,
    /// Public base URL for install commands (optional), e.g. `https://vpnc.example.com`.
    #[serde(default)]
    pub public_url: Option<String>,
    /// Public gRPC endpoint shown in join-token install commands.
    #[serde(default)]
    pub public_grpc: Option<String>,
    /// Auto-bootstrap admin email (only if no users exist).
    #[serde(default, skip_serializing)]
    pub bootstrap_email: Option<String>,
    /// Auto-bootstrap admin password.
    #[serde(default, skip_serializing)]
    pub bootstrap_password: Option<String>,
    /// Telemetry / logging.
    pub telemetry: TelemetryConfig,
}

impl Default for ControlConfig {
    fn default() -> Self {
        Self {
            http_bind: "0.0.0.0:8080".into(),
            grpc_bind: "0.0.0.0:50051".into(),
            database_url: "sqlite://vpnc.db?mode=rwc".into(),
            master_key: None,
            session_hours: 24,
            cookie_secure: true,
            static_dir: None,
            trust_forwarded_headers: false,
            public_url: None,
            public_grpc: None,
            bootstrap_email: None,
            bootstrap_password: None,
            telemetry: TelemetryConfig {
                filter: "info".into(),
                json: true,
            },
        }
    }
}

impl ControlConfig {
    /// Loads config from optional file path and `VPNC_*` environment.
    pub fn load(path: Option<&std::path::Path>) -> Result<Self, vpnc_common::ConfigError> {
        let mut cfg = vpnc_common::load_config("control", path, Self::default())?;
        // Map common env aliases that figment may not nest automatically.
        if let Ok(v) = std::env::var("VPNC_STATIC_DIR") {
            if !v.is_empty() {
                cfg.static_dir = Some(PathBuf::from(v));
            }
        }
        if let Ok(v) = std::env::var("VPNC_BOOTSTRAP_EMAIL") {
            if !v.is_empty() {
                cfg.bootstrap_email = Some(v);
            }
        }
        if let Ok(v) = std::env::var("VPNC_BOOTSTRAP_PASSWORD") {
            if !v.is_empty() {
                cfg.bootstrap_password = Some(v);
            }
        }
        if let Ok(v) = std::env::var("VPNC_PUBLIC_URL") {
            if !v.is_empty() {
                cfg.public_url = Some(v);
            }
        }
        if let Ok(v) = std::env::var("VPNC_PUBLIC_GRPC") {
            if !v.is_empty() {
                cfg.public_grpc = Some(v);
            }
        }
        if let Ok(v) = std::env::var("VPNC_TRUST_FORWARDED_HEADERS") {
            cfg.trust_forwarded_headers =
                matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
        }
        if let Ok(v) = std::env::var("VPNC_COOKIE_SECURE") {
            cfg.cookie_secure =
                matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
        }
        Ok(cfg)
    }

    /// Returns the master key; fails hard if missing/invalid.
    pub fn require_master_key(&self) -> anyhow::Result<Secret<[u8; 32]>> {
        let raw = self
            .master_key
            .clone()
            .or_else(|| std::env::var("VPNC_MASTER_KEY").ok())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "VPNC_MASTER_KEY is required (64 hex characters or base64 encoding of 32 bytes)"
                )
            })?;
        let key = parse_master_key(&raw).context("invalid VPNC_MASTER_KEY")?;
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
    use base64::Engine;
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(trimmed) {
        if bytes.len() == 32 {
            let mut out = [0u8; 32];
            out.copy_from_slice(&bytes);
            return Ok(out);
        }
    }
    if let Ok(bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(trimmed) {
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
    anyhow::bail!(
        "VPNC_MASTER_KEY must be 64 hex chars, base64 of 32 bytes, or 32 raw bytes (got {} chars)",
        trimmed.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_and_base64() {
        let hex_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert!(parse_master_key(hex_key).is_ok());
        let raw = [9u8; 32];
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, raw);
        assert_eq!(parse_master_key(&b64).unwrap(), raw);
    }
}
