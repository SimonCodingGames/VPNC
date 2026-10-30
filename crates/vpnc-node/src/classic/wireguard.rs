//! Classic WireGuard interface management.
//!
//! On Linux with kernel WireGuard: netlink via `wireguard-control` (see ADR 0007).
//! Fallback: boringtun userspace device.
//!
//! Full netns acceptance tests require Linux (`scripts/netns-test.sh`).

use anyhow::{bail, Result};
use std::collections::BTreeMap;

/// Desired peer state for idempotent sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredPeer {
    /// Base64 public key.
    pub public_key: String,
    /// Allowed IPs (CIDRs).
    pub allowed_ips: Vec<String>,
    /// Optional endpoint host:port.
    pub endpoint: Option<String>,
}

/// Desired WireGuard interface configuration.
#[derive(Debug, Clone)]
pub struct DesiredInterface {
    /// Interface name, e.g. `wg0`.
    pub name: String,
    /// Server private key (base64) — never logged.
    pub private_key_b64: String,
    /// Listen port.
    pub listen_port: u16,
    /// Peers keyed by public key.
    pub peers: BTreeMap<String, DesiredPeer>,
}

/// Backend used to apply WireGuard state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WgBackendKind {
    /// Kernel WireGuard via netlink.
    Kernel,
    /// Userspace boringtun.
    Userspace,
}

/// Applies desired state. Currently a dry-run/stub on non-Linux hosts.
pub fn apply_interface(desired: &DesiredInterface) -> Result<WgBackendKind> {
    if desired.private_key_b64.is_empty() {
        bail!("private key required");
    }
    if cfg!(target_os = "linux") {
        // TODO: wireguard-control netlink apply + boringtun fallback.
        tracing::info!(iface = %desired.name, peers = desired.peers.len(), "apply (linux stub)");
        Ok(WgBackendKind::Kernel)
    } else {
        tracing::warn!(
            iface = %desired.name,
            "WireGuard apply is a no-op on this OS; use Linux for step 7 acceptance"
        );
        Ok(WgBackendKind::Userspace)
    }
}

/// Computes peer add/remove/change sets for idempotent sync.
pub fn diff_peers(
    current: &BTreeMap<String, DesiredPeer>,
    desired: &BTreeMap<String, DesiredPeer>,
) -> (Vec<DesiredPeer>, Vec<String>, Vec<DesiredPeer>) {
    let mut add = Vec::new();
    let mut remove = Vec::new();
    let mut change = Vec::new();
    for (k, d) in desired {
        match current.get(k) {
            None => add.push(d.clone()),
            Some(c) if c != d => change.push(d.clone()),
            Some(_) => {}
        }
    }
    for k in current.keys() {
        if !desired.contains_key(k) {
            remove.push(k.clone());
        }
    }
    (add, remove, change)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_detects_add_remove_change() {
        let mut current = BTreeMap::new();
        current.insert(
            "a".into(),
            DesiredPeer {
                public_key: "a".into(),
                allowed_ips: vec!["10.0.0.2/32".into()],
                endpoint: None,
            },
        );
        current.insert(
            "b".into(),
            DesiredPeer {
                public_key: "b".into(),
                allowed_ips: vec!["10.0.0.3/32".into()],
                endpoint: None,
            },
        );
        let mut desired = BTreeMap::new();
        desired.insert(
            "a".into(),
            DesiredPeer {
                public_key: "a".into(),
                allowed_ips: vec!["10.0.0.2/32".into(), "10.0.0.99/32".into()],
                endpoint: None,
            },
        );
        desired.insert(
            "c".into(),
            DesiredPeer {
                public_key: "c".into(),
                allowed_ips: vec!["10.0.0.4/32".into()],
                endpoint: Some("1.2.3.4:51820".into()),
            },
        );
        let (add, remove, change) = diff_peers(&current, &desired);
        assert_eq!(add.len(), 1);
        assert_eq!(remove, vec!["b".to_string()]);
        assert_eq!(change.len(), 1);
    }
}
