//! Host hardening for classic VPN nodes (sysctl, module, UDP allow).

use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;
use vpnc_common::WG_LISTEN_PORT;

/// Applies durable host settings so a freshly installed node routes VPN traffic.
pub fn ensure_host_ready(iface: &str, listen_port: u16) -> Result<()> {
    if !cfg!(target_os = "linux") {
        return Ok(());
    }
    let _ = Command::new("modprobe").args(["wireguard"]).status();
    ensure_sysctl_persist()?;
    apply_sysctl_runtime()?;
    ensure_udp_input(listen_port)?;
    // Per-iface rp_filter after wg exists (best-effort if iface not up yet).
    if !iface.is_empty() {
        let path = format!("/proc/sys/net/ipv4/conf/{iface}/rp_filter");
        let _ = fs::write(&path, b"2");
    }
    Ok(())
}

fn ensure_sysctl_persist() -> Result<()> {
    let path = Path::new("/etc/sysctl.d/99-vpnc.conf");
    let body = "\
# Managed by vpnc-node — classic WireGuard forwarding
net.ipv4.ip_forward=1
net.ipv6.conf.all.forwarding=1
net.ipv4.conf.all.rp_filter=2
net.ipv4.conf.default.rp_filter=2
";
    if let Err(err) = fs::write(path, body) {
        tracing::warn!(error = %err, "could not write sysctl.d (non-root?)");
        return Ok(());
    }
    let modules = Path::new("/etc/modules-load.d/vpnc-wireguard.conf");
    let _ = fs::write(modules, "wireguard\n");
    let _ = Command::new("sysctl").args(["--system"]).status();
    Ok(())
}

fn apply_sysctl_runtime() -> Result<()> {
    for (path, val) in [
        ("/proc/sys/net/ipv4/ip_forward", "1"),
        ("/proc/sys/net/ipv6/conf/all/forwarding", "1"),
        ("/proc/sys/net/ipv4/conf/all/rp_filter", "2"),
        ("/proc/sys/net/ipv4/conf/default/rp_filter", "2"),
    ] {
        let _ = fs::write(path, val.as_bytes());
    }
    Ok(())
}

/// Opens UDP listen port in a dedicated nftables table (idempotent).
fn ensure_udp_input(port: u16) -> Result<()> {
    let port = if port == 0 { WG_LISTEN_PORT } else { port };
    let ruleset = format!(
        "table inet vpnc_input {{\n\
         \x20 chain input {{\n\
         \x20\x20  type filter hook input priority filter; policy accept;\n\
         \x20\x20  udp dport {port} accept\n\
         \x20 }}\n\
         }}\n"
    );
    let mut tmp = NamedTempFile::new().context("temp nft input")?;
    tmp.write_all(ruleset.as_bytes())?;
    tmp.flush()?;
    let _ = Command::new("nft")
        .args(["delete", "table", "inet", "vpnc_input"])
        .status();
    let status = Command::new("nft")
        .args(["-f"])
        .arg(tmp.path())
        .status()
        .context("nft input")?;
    drop(tmp);
    if !status.success() {
        tracing::warn!(%port, "nft UDP allow failed — open UDP {port} in the host firewall / cloud SG");
    } else {
        tracing::info!(%port, "nft UDP input allow applied");
    }
    Ok(())
}
