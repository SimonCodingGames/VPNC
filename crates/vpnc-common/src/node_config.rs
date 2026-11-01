//! Config payloads pushed from control to nodes over the gRPC stream.

use serde::{Deserialize, Serialize};

/// Classic WireGuard peer (client) as seen by the node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClassicPeerConfig {
    /// WireGuard public key (base64).
    pub public_key: String,
    /// Allowed tunnel addresses for this peer (usually /32 and/or /128).
    pub allowed_ips: Vec<String>,
}

/// Full classic-role configuration for `vpnc-node`.
///
/// Contains the server private key — never log this struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassicWgConfig {
    /// Role marker (`classic`).
    pub role: String,
    /// Interface name, e.g. `wg0`.
    pub interface: String,
    /// UDP listen port.
    pub listen_port: u16,
    /// Server private key (WireGuard base64).
    pub private_key: String,
    /// Server tunnel IPv4 CIDR on the interface, e.g. `10.66.0.1/24`.
    #[serde(default)]
    pub address_v4: Option<String>,
    /// Server tunnel IPv6 CIDR on the interface, e.g. `fd66::1/64`.
    #[serde(default)]
    pub address_v6: Option<String>,
    /// Full tunnel pool used for NAT source matching.
    #[serde(default)]
    pub tunnel_cidr_v4: Option<String>,
    /// Full tunnel IPv6 pool used for NAT source matching.
    #[serde(default)]
    pub tunnel_cidr_v6: Option<String>,
    /// Active client peers.
    pub peers: Vec<ClassicPeerConfig>,
}

impl ClassicWgConfig {
    /// Stable config version derived from content (not secret-safe to log the key).
    pub fn content_version(&self) -> u64 {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(self.role.as_bytes());
        h.update(self.interface.as_bytes());
        h.update(self.listen_port.to_le_bytes());
        h.update(self.private_key.as_bytes());
        if let Some(a) = &self.address_v4 {
            h.update(a.as_bytes());
        }
        if let Some(a) = &self.address_v6 {
            h.update(a.as_bytes());
        }
        if let Some(a) = &self.tunnel_cidr_v4 {
            h.update(a.as_bytes());
        }
        if let Some(a) = &self.tunnel_cidr_v6 {
            h.update(a.as_bytes());
        }
        for p in &self.peers {
            h.update(p.public_key.as_bytes());
            for ip in &p.allowed_ips {
                h.update(ip.as_bytes());
            }
        }
        let dig = h.finalize();
        u64::from_be_bytes(dig[0..8].try_into().unwrap_or([0; 8]))
    }
}
