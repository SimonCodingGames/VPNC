//! Classic WireGuard interface management.
//!
//! On Linux: create/configure via `ip` + `wg setconf` (netlink under the hood;
//! see ADR 0007). Fallback path is documented for boringtun when no kernel
//! module is present.
//!
//! Full netns acceptance tests require Linux (`scripts/netns-test.sh`).

use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;
use vpnc_common::{ClassicWgConfig, WG_MTU};

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
    /// Interface addresses (CIDR strings).
    pub addresses: Vec<String>,
    /// Peers keyed by public key.
    pub peers: BTreeMap<String, DesiredPeer>,
}

impl DesiredInterface {
    /// Converts a streamed classic config into apply input.
    pub fn from_classic(cfg: &ClassicWgConfig) -> Self {
        let mut peers = BTreeMap::new();
        for p in &cfg.peers {
            peers.insert(
                p.public_key.clone(),
                DesiredPeer {
                    public_key: p.public_key.clone(),
                    allowed_ips: p.allowed_ips.clone(),
                    endpoint: None,
                },
            );
        }
        let mut addresses = Vec::new();
        if let Some(a) = &cfg.address_v4 {
            addresses.push(a.clone());
        }
        if let Some(a) = &cfg.address_v6 {
            addresses.push(a.clone());
        }
        Self {
            name: cfg.interface.clone(),
            private_key_b64: cfg.private_key.clone(),
            listen_port: cfg.listen_port,
            addresses,
            peers,
        }
    }
}

/// Backend used to apply WireGuard state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WgBackendKind {
    /// Kernel WireGuard via `ip`/`wg` (netlink).
    Kernel,
    /// Userspace boringtun (not yet applied in-process).
    Userspace,
}

/// Applies desired WireGuard state on Linux; no-op elsewhere.
pub fn apply_interface(desired: &DesiredInterface) -> Result<WgBackendKind> {
    if desired.private_key_b64.is_empty() {
        bail!("private key required");
    }
    if desired.name.is_empty()
        || desired
            .name
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
    {
        bail!("invalid interface name");
    }

    if !cfg!(target_os = "linux") {
        tracing::warn!(
            iface = %desired.name,
            "WireGuard apply is a no-op on this OS; use Linux for step 7 acceptance"
        );
        return Ok(WgBackendKind::Userspace);
    }

    if !kernel_module_available() {
        tracing::warn!("wireguard kernel module not detected; trying wg anyway");
    }

    ensure_link(&desired.name)?;
    apply_setconf(desired)?;
    for addr in &desired.addresses {
        replace_address(&desired.name, addr)?;
    }
    let mtu = WG_MTU.to_string();
    run_ok(
        "ip",
        &["link", "set", "dev", &desired.name, "mtu", &mtu, "up"],
    )?;
    tracing::info!(
        iface = %desired.name,
        peers = desired.peers.len(),
        port = desired.listen_port,
        "wireguard interface applied"
    );
    Ok(WgBackendKind::Kernel)
}

fn kernel_module_available() -> bool {
    PathBuf::from("/sys/module/wireguard").exists()
        || Command::new("modprobe")
            .args(["wireguard"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
}

fn ensure_link(name: &str) -> Result<()> {
    let listed = Command::new("ip")
        .args(["-o", "link", "show", "dev", name])
        .output()
        .context("ip link show")?;
    if listed.status.success() {
        return Ok(());
    }
    run_ok("ip", &["link", "add", "dev", name, "type", "wireguard"])
        .context("ip link add wireguard")?;
    Ok(())
}

fn apply_setconf(desired: &DesiredInterface) -> Result<()> {
    let conf = render_setconf(desired);
    let mut tmp = NamedTempFile::new().context("temp wg conf")?;
    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(tmp.path())?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(tmp.path(), perms)?;
    }
    tmp.write_all(conf.as_bytes()).context("write wg conf")?;
    tmp.flush()?;
    let path = tmp.path();
    let status = Command::new("wg")
        .args(["setconf", &desired.name])
        .arg(path)
        .status()
        .context("wg setconf")?;
    // Drop tempfile (and private key) as soon as wg has read it.
    drop(tmp);
    if !status.success() {
        bail!("wg setconf failed with {status}");
    }
    Ok(())
}

fn render_setconf(desired: &DesiredInterface) -> String {
    let mut out = String::new();
    out.push_str("[Interface]\n");
    out.push_str(&format!("PrivateKey = {}\n", desired.private_key_b64));
    out.push_str(&format!("ListenPort = {}\n", desired.listen_port));
    for peer in desired.peers.values() {
        out.push('\n');
        out.push_str("[Peer]\n");
        out.push_str(&format!("PublicKey = {}\n", peer.public_key));
        if !peer.allowed_ips.is_empty() {
            out.push_str(&format!("AllowedIPs = {}\n", peer.allowed_ips.join(", ")));
        }
        if let Some(ep) = &peer.endpoint {
            out.push_str(&format!("Endpoint = {ep}\n"));
        }
    }
    out
}

fn replace_address(iface: &str, cidr: &str) -> Result<()> {
    if cidr.contains(':') {
        // IPv6
        let _ = Command::new("ip")
            .args(["-6", "addr", "flush", "dev", iface])
            .status();
        run_ok("ip", &["-6", "addr", "replace", cidr, "dev", iface])
    } else {
        run_ok("ip", &["addr", "replace", cidr, "dev", iface])
    }
}

fn run_ok(bin: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(bin)
        .args(args)
        .status()
        .with_context(|| format!("exec {bin}"))?;
    if !status.success() {
        bail!("{bin} {:?} failed: {status}", args);
    }
    Ok(())
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

    #[test]
    fn setconf_shape_omits_nothing_critical() {
        let mut peers = BTreeMap::new();
        peers.insert(
            "pk".into(),
            DesiredPeer {
                public_key: "pk".into(),
                allowed_ips: vec!["10.66.0.2/32".into()],
                endpoint: None,
            },
        );
        let d = DesiredInterface {
            name: "wg0".into(),
            private_key_b64: "PRIV".into(),
            listen_port: 51820,
            addresses: vec!["10.66.0.1/24".into()],
            peers,
        };
        let conf = render_setconf(&d);
        assert!(conf.contains("ListenPort = 51820"));
        assert!(conf.contains("PublicKey = pk"));
        assert!(conf.contains("AllowedIPs = 10.66.0.2/32"));
    }
}
