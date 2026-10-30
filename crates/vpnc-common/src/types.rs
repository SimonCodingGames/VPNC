//! Shared domain identifiers and enums.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

macro_rules! uuid_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Creates a new random identifier.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Wraps an existing UUID.
            pub fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            /// Returns the inner UUID.
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }

        impl From<$name> for Uuid {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }
    };
}

uuid_newtype!(
    /// Identifier of a VPN node.
    NodeId
);
uuid_newtype!(
    /// Identifier of a user account.
    UserId
);
uuid_newtype!(
    /// Identifier of a user device.
    DeviceId
);

/// Role of a server node in the VPNC topology.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    /// Classic single-server endpoint (WireGuard/OpenVPN).
    Classic,
    /// Ingress node accepting VPNC-Tunnel clients.
    Ingress,
    /// Egress node forwarding traffic to the internet.
    Egress,
}

impl fmt::Display for NodeRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Classic => write!(f, "classic"),
            Self::Ingress => write!(f, "ingress"),
            Self::Egress => write!(f, "egress"),
        }
    }
}

impl FromStr for NodeRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "classic" => Ok(Self::Classic),
            "ingress" => Ok(Self::Ingress),
            "egress" => Ok(Self::Egress),
            other => Err(format!("unknown node role: {other}")),
        }
    }
}

/// Kind of VPN profile issued to a device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileKind {
    /// Classic WireGuard profile to a single node.
    Wireguard,
    /// Classic OpenVPN profile to a single node.
    Openvpn,
    /// VPNC-Tunnel profile (WireGuard on the wire, cluster endpoint).
    Vpnc,
}

impl fmt::Display for ProfileKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wireguard => write!(f, "wireguard"),
            Self::Openvpn => write!(f, "openvpn"),
            Self::Vpnc => write!(f, "vpnc"),
        }
    }
}

impl FromStr for ProfileKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "wireguard" => Ok(Self::Wireguard),
            "openvpn" => Ok(Self::Openvpn),
            "vpnc" => Ok(Self::Vpnc),
            other => Err(format!("unknown profile kind: {other}")),
        }
    }
}
