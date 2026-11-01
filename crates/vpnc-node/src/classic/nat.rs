//! IP forwarding and nftables masquerading (table `vpnc`).

use anyhow::{bail, Context, Result};
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

/// Ensures IPv4/IPv6 forwarding and loose rp_filter (Linux).
pub fn ensure_ip_forward() -> Result<()> {
    if !cfg!(target_os = "linux") {
        return Ok(());
    }
    write_proc("/proc/sys/net/ipv4/ip_forward", b"1")?;
    let _ = write_proc("/proc/sys/net/ipv6/conf/all/forwarding", b"1");
    // Strict rp_filter drops return traffic for VPN clients on many VPS images.
    let _ = write_proc("/proc/sys/net/ipv4/conf/all/rp_filter", b"2");
    let _ = write_proc("/proc/sys/net/ipv4/conf/default/rp_filter", b"2");
    tracing::info!("ip forwarding enabled");
    Ok(())
}

fn write_proc(path: &str, value: &[u8]) -> Result<()> {
    fs::write(path, value).with_context(|| format!("write {path}"))
}

/// Ensures nftables table/chain `vpnc` with masquerade + MSS clamp for tunnel clients.
///
/// Invokes `nft` with an argv list and a temp ruleset file — never a shell string.
pub fn ensure_masquerade(iface: &str, source_cidrs: &[&str]) -> Result<()> {
    if iface.is_empty() {
        bail!("iface required");
    }
    if source_cidrs.is_empty() {
        bail!("source_cidrs required");
    }
    if !iface
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!("invalid iface name");
    }
    for cidr in source_cidrs {
        validate_cidr(cidr)?;
    }

    if !cfg!(target_os = "linux") {
        tracing::debug!(%iface, "nft masquerade skipped (non-linux)");
        return Ok(());
    }

    // Loose rp_filter on the tunnel iface specifically.
    let rp = format!("/proc/sys/net/ipv4/conf/{iface}/rp_filter");
    let _ = write_proc(&rp, b"2");

    let ruleset = render_ruleset(iface, source_cidrs);
    let mut tmp = NamedTempFile::new().context("temp nft ruleset")?;
    tmp.write_all(ruleset.as_bytes())?;
    tmp.flush()?;

    // Replace table idempotently: delete (ignore miss) then load.
    let _ = Command::new("nft")
        .args(["delete", "table", "inet", "vpnc"])
        .status();
    let status = Command::new("nft")
        .args(["-f"])
        .arg(tmp.path())
        .status()
        .context("nft -f")?;
    drop(tmp);
    if !status.success() {
        bail!("nft apply failed: {status}");
    }
    tracing::info!(%iface, cidrs = ?source_cidrs, "nft vpnc masquerade applied");
    Ok(())
}

fn validate_cidr(cidr: &str) -> Result<()> {
    if cidr.is_empty() || cidr.contains(|c: char| c.is_whitespace()) || !cidr.contains('/') {
        bail!("invalid cidr");
    }
    Ok(())
}

fn render_ruleset(iface: &str, source_cidrs: &[&str]) -> String {
    let mut out = String::new();
    out.push_str("table inet vpnc {\n");
    out.push_str("  chain forward {\n");
    out.push_str("    type filter hook forward priority filter; policy accept;\n");
    out.push_str("    ct state established,related accept\n");
    out.push_str(&format!("    iifname \"{iface}\" accept\n"));
    out.push_str(&format!("    oifname \"{iface}\" accept\n"));
    // Clamp MSS so full-tunnel TCP works behind WireGuard overhead.
    out.push_str(&format!(
        "    iifname \"{iface}\" tcp flags syn tcp option maxseg size set rt mtu\n"
    ));
    out.push_str(&format!(
        "    oifname \"{iface}\" tcp flags syn tcp option maxseg size set rt mtu\n"
    ));
    out.push_str("  }\n");
    out.push_str("  chain postrouting {\n");
    out.push_str("    type nat hook postrouting priority srcnat; policy accept;\n");
    for cidr in source_cidrs {
        // Masquerade any egress that is not back into the tunnel.
        if cidr.contains(':') {
            out.push_str(&format!(
                "    ip6 saddr {cidr} oifname != \"{iface}\" masquerade\n"
            ));
        } else {
            out.push_str(&format!(
                "    ip saddr {cidr} oifname != \"{iface}\" masquerade\n"
            ));
        }
    }
    out.push_str("  }\n");
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruleset_contains_masquerade_and_mss() {
        let r = render_ruleset("wg0", &["10.66.0.0/24", "fd66::/64"]);
        assert!(r.contains("table inet vpnc"));
        assert!(r.contains("ip saddr 10.66.0.0/24"));
        assert!(r.contains("masquerade"));
        assert!(r.contains("ip6 saddr fd66::/64"));
        assert!(r.contains("iifname \"wg0\" accept"));
        assert!(r.contains("tcp option maxseg size set rt mtu"));
        assert!(r.contains("ct state established,related accept"));
    }
}
