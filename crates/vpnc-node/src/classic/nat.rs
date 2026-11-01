//! IP forwarding and nftables masquerading (table `vpnc`).

use anyhow::{bail, Context, Result};
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

/// Ensures IPv4/IPv6 forwarding is enabled (Linux).
pub fn ensure_ip_forward() -> Result<()> {
    if !cfg!(target_os = "linux") {
        return Ok(());
    }
    write_proc("/proc/sys/net/ipv4/ip_forward", b"1")?;
    // Best-effort IPv6 forwarding on all interfaces.
    let _ = write_proc("/proc/sys/net/ipv6/conf/all/forwarding", b"1");
    tracing::info!("ip forwarding enabled");
    Ok(())
}

fn write_proc(path: &str, value: &[u8]) -> Result<()> {
    fs::write(path, value).with_context(|| format!("write {path}"))
}

/// Ensures nftables table/chain `vpnc` with masquerade for the tunnel subnet(s).
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
    out.push_str(&format!("    iifname \"{iface}\" accept\n"));
    out.push_str(&format!("    oifname \"{iface}\" accept\n"));
    out.push_str("  }\n");
    out.push_str("  chain postrouting {\n");
    out.push_str("    type nat hook postrouting priority srcnat; policy accept;\n");
    for cidr in source_cidrs {
        if cidr.contains(':') {
            out.push_str(&format!(
                "    oifname != \"{iface}\" ip6 saddr {cidr} masquerade\n"
            ));
        } else {
            out.push_str(&format!(
                "    oifname != \"{iface}\" ip saddr {cidr} masquerade\n"
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
    fn ruleset_contains_masquerade() {
        let r = render_ruleset("wg0", &["10.66.0.0/24", "fd66::/64"]);
        assert!(r.contains("table inet vpnc"));
        assert!(r.contains("ip saddr 10.66.0.0/24 masquerade"));
        assert!(r.contains("ip6 saddr fd66::/64 masquerade"));
        assert!(r.contains("iifname \"wg0\" accept"));
    }
}
