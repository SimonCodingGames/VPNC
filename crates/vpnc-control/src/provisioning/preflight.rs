//! Remote preflight checks (structure ready for Debian/Ubuntu first).

use serde::Serialize;

/// Structured preflight report returned to the dashboard.
#[derive(Debug, Clone, Serialize, Default)]
pub struct PreflightReport {
    /// Detected OS id, e.g. `debian`, `ubuntu`.
    pub distro: Option<String>,
    /// Architecture, e.g. `x86_64`, `aarch64`.
    pub arch: Option<String>,
    /// Whether root or passwordless sudo is available.
    pub privilege_ok: bool,
    /// Whether WireGuard kernel module appears available.
    pub wireguard_ok: bool,
    /// Whether required ports appear free.
    pub ports_ok: bool,
    /// Observed public IPv4 if detected.
    pub public_ip: Option<String>,
    /// Observed public IPv6 if detected.
    pub public_ipv6: Option<String>,
    /// Clock skew vs control in seconds, if measured.
    pub clock_skew_secs: Option<i64>,
    /// Human-readable issues.
    pub issues: Vec<String>,
}

impl PreflightReport {
    /// Returns true when installation may proceed.
    pub fn ok(&self) -> bool {
        self.privilege_ok && self.wireguard_ok && self.ports_ok && self.issues.is_empty()
    }
}

/// Builds a failing report for unsupported distros (extensible later).
pub fn unsupported_distro(id: &str) -> PreflightReport {
    PreflightReport {
        distro: Some(id.to_string()),
        issues: vec![format!(
            "unsupported distribution: {id} (supported: debian, ubuntu)"
        )],
        ..Default::default()
    }
}
