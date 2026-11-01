//! Detect public IPv4/IPv6 on a remote host (fixed commands only).

use super::ssh::SshSession;
use anyhow::Result;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Observed public addresses (may be incomplete).
#[derive(Debug, Clone, Default)]
pub struct DetectedPublicIps {
    /// Globally routable IPv4, if any.
    pub ipv4: Option<String>,
    /// Globally routable IPv6, if any.
    pub ipv6: Option<String>,
}

/// Detects source addresses via `ip route get` (no user input in the command).
pub async fn detect_public_ips(session: &mut SshSession) -> Result<DetectedPublicIps> {
    let mut out = DetectedPublicIps::default();

    let (_, v4_out, _) = session
        .exec("ip -4 route get 1.1.1.1 2>/dev/null || true")
        .await?;
    if let Some(ip) = parse_route_src(&v4_out).and_then(|s| s.parse::<Ipv4Addr>().ok()) {
        if is_public_v4(ip) {
            out.ipv4 = Some(ip.to_string());
        }
    }

    let (_, v6_out, _) = session
        .exec("ip -6 route get 2606:4700:4700::1111 2>/dev/null || true")
        .await?;
    if let Some(ip) = parse_route_src(&v6_out).and_then(|s| s.parse::<Ipv6Addr>().ok()) {
        if is_public_v6(ip) {
            out.ipv6 = Some(ip.to_string());
        }
    }

    Ok(out)
}

/// Prefer SSH target host when it is already a public IP and detection missed a family.
pub fn merge_with_ssh_host(mut detected: DetectedPublicIps, host: &str) -> DetectedPublicIps {
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = host.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(v4) if detected.ipv4.is_none() && is_public_v4(v4) => {
                detected.ipv4 = Some(v4.to_string());
            }
            IpAddr::V6(v6) if detected.ipv6.is_none() && is_public_v6(v6) => {
                detected.ipv6 = Some(v6.to_string());
            }
            _ => {}
        }
    }
    detected
}

/// Formats a WireGuard `Endpoint` host:port (IPv6 bracketed).
pub fn format_wg_endpoint(ipv4: Option<&str>, ipv6: Option<&str>, port: u16) -> String {
    if let Some(v4) = ipv4.map(str::trim).filter(|s| !s.is_empty()) {
        return format!("{v4}:{port}");
    }
    if let Some(v6) = ipv6.map(str::trim).filter(|s| !s.is_empty()) {
        let bare = v6.trim_start_matches('[').trim_end_matches(']');
        return format!("[{bare}]:{port}");
    }
    format!("CHANGE_ME:{port}")
}

fn parse_route_src(out: &str) -> Option<String> {
    let mut parts = out.split_whitespace();
    while let Some(p) = parts.next() {
        if p == "src" {
            return parts.next().map(str::to_string);
        }
    }
    None
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    !(ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_unspecified()
        || ip.is_multicast())
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_unicast_link_local()
        || (ip.segments()[0] & 0xfe00) == 0xfc00) // ULA fc00::/7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_src_from_ip_route() {
        let sample = "1.1.1.1 via 10.0.0.1 dev eth0 src 203.0.113.10 uid 0";
        assert_eq!(parse_route_src(sample).as_deref(), Some("203.0.113.10"));
    }

    #[test]
    fn endpoint_prefers_v4() {
        assert_eq!(
            format_wg_endpoint(Some("203.0.113.10"), Some("2001:db8::1"), 51820),
            "203.0.113.10:51820"
        );
        assert_eq!(
            format_wg_endpoint(None, Some("2001:db8::1"), 51820),
            "[2001:db8::1]:51820"
        );
        assert_eq!(format_wg_endpoint(None, None, 51820), "CHANGE_ME:51820");
    }

    #[test]
    fn merge_ssh_host_ipv4() {
        let d = merge_with_ssh_host(DetectedPublicIps::default(), "198.51.100.20");
        assert_eq!(d.ipv4.as_deref(), Some("198.51.100.20"));
    }
}
