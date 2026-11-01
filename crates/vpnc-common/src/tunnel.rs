//! Shared classic-tunnel defaults (single source of truth for Control + Node + profiles).

/// Default WireGuard listen / client endpoint port.
pub const WG_LISTEN_PORT: u16 = 51820;

/// Default interface name on classic nodes.
pub const WG_INTERFACE: &str = "wg0";

/// MTU that fits WireGuard + IPv4 outer header on typical VPS paths.
pub const WG_MTU: u16 = 1420;

/// Default IPv4 tunnel pool (server uses `.1`, clients from `.2`).
pub const DEFAULT_TUNNEL_CIDR_V4: &str = "10.66.0.0/24";

/// Default IPv6-ULA tunnel pool.
pub const DEFAULT_TUNNEL_CIDR_V6: &str = "fd66::/64";

/// Client AllowedIPs for a reliable full tunnel (IPv4).
/// `::/0` is omitted by default — broken IPv6 NAT blackholes Windows/macOS clients.
pub const CLIENT_ALLOWED_IPS: &str = "0.0.0.0/0";

/// DNS servers pushed into client profiles (resolved through the tunnel).
pub const CLIENT_DNS: &str = "1.1.1.1, 1.0.0.1";
