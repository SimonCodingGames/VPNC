//! IP forwarding and nftables masquerading (table `vpnc`).

use anyhow::Result;

/// Ensures IPv4 forwarding is enabled (Linux).
pub fn ensure_ip_forward() -> Result<()> {
    if cfg!(target_os = "linux") {
        // TODO: write /proc/sys/net/ipv4/ip_forward = 1 without shell strings.
        tracing::info!("ip_forward ensure (linux stub)");
    }
    Ok(())
}

/// Ensures nftables table/chain `vpnc` with masquerade for the tunnel subnet.
pub fn ensure_masquerade(iface: &str, source_cidr: &str) -> Result<()> {
    if iface.is_empty() || source_cidr.is_empty() {
        anyhow::bail!("iface and source_cidr required");
    }
    if cfg!(target_os = "linux") {
        // TODO: invoke `nft` with argv list (never shell-concat user input).
        tracing::info!(%iface, %source_cidr, "nft masquerade ensure (linux stub)");
    }
    Ok(())
}
