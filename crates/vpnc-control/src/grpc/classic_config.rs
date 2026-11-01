//! Build classic WireGuard config payloads for the node config stream.

use crate::db::Db;
use crate::services::crypto::CryptoService;
use crate::services::ipam::IpAllocator;
use crate::services::wireguard::WgKeypair;
use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chrono::Utc;
use vpnc_common::{ClassicPeerConfig, ClassicWgConfig};

const DEFAULT_CIDR_V4: &str = "10.66.0.0/24";
const DEFAULT_CIDR_V6: &str = "fd66::/64";
const LISTEN_PORT: u16 = 51820;
const IFACE: &str = "wg0";

/// Builds a full classic config for `node_id`, ensuring a server keypair exists.
pub async fn build_classic_config(
    db: &Db,
    crypto: &CryptoService,
    node_id: &str,
) -> Result<ClassicWgConfig> {
    let row = fetch_node(db, node_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("node not found"))?;
    if row.role != "classic" {
        bail!("node role is not classic");
    }

    let cidr_v4 = row
        .tunnel_cidr_v4
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_CIDR_V4)
        .to_string();
    let cidr_v6 = row
        .tunnel_cidr_v6
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_CIDR_V6)
        .to_string();

    let private_b64 = ensure_private_key(db, crypto, node_id, &row).await?;

    let peers = load_peers(db, node_id).await?;
    let address_v4 = IpAllocator::gateway_v4_cidr(&cidr_v4).ok();
    let address_v6 = IpAllocator::gateway_v6_cidr(&cidr_v6).ok();

    // Persist defaults so subsequent API reads see them.
    persist_cidrs(db, node_id, &cidr_v4, &cidr_v6).await?;

    Ok(ClassicWgConfig {
        role: "classic".into(),
        interface: IFACE.into(),
        listen_port: LISTEN_PORT,
        private_key: private_b64,
        address_v4,
        address_v6,
        tunnel_cidr_v4: Some(cidr_v4),
        tunnel_cidr_v6: Some(cidr_v6),
        peers,
    })
}

struct NodeKeys {
    role: String,
    tunnel_cidr_v4: Option<String>,
    tunnel_cidr_v6: Option<String>,
    server_public_key: Option<String>,
    encrypted_server_private_key: Option<String>,
}

async fn fetch_node(db: &Db, node_id: &str) -> Result<Option<NodeKeys>> {
    let row: Option<(
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = match db {
        Db::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT role, tunnel_cidr_v4, tunnel_cidr_v6, server_public_key, encrypted_server_private_key FROM nodes WHERE id = ?",
            )
            .bind(node_id)
            .fetch_optional(pool)
            .await?
        }
        Db::Postgres(pool) => {
            sqlx::query_as(
                "SELECT role, tunnel_cidr_v4, tunnel_cidr_v6, server_public_key, encrypted_server_private_key FROM nodes WHERE id = $1",
            )
            .bind(node_id)
            .fetch_optional(pool)
            .await?
        }
    };
    Ok(row.map(|(role, v4, v6, pubk, enc)| NodeKeys {
        role,
        tunnel_cidr_v4: v4,
        tunnel_cidr_v6: v6,
        server_public_key: pubk,
        encrypted_server_private_key: enc,
    }))
}

async fn ensure_private_key(
    db: &Db,
    crypto: &CryptoService,
    node_id: &str,
    row: &NodeKeys,
) -> Result<String> {
    if let Some(enc) = &row.encrypted_server_private_key {
        let secret = crypto.decrypt(enc).context("decrypt server private key")?;
        let bytes = secret.expose();
        if bytes.len() == 32 {
            return Ok(B64.encode(bytes));
        }
        // Already stored as base64 string?
        if let Ok(s) = std::str::from_utf8(bytes) {
            if s.len() >= 40 {
                return Ok(s.to_string());
            }
        }
        bail!("unexpected server private key encoding");
    }

    let kp = WgKeypair::generate();
    let enc = crypto.encrypt(kp.private.expose())?;
    let pub_b64 = kp.public_b64();
    let now = Utc::now().to_rfc3339();
    match db {
        Db::Sqlite(pool) => {
            sqlx::query(
                "UPDATE nodes SET encrypted_server_private_key = ?, server_public_key = ?, updated_at = ? WHERE id = ?",
            )
            .bind(&enc)
            .bind(&pub_b64)
            .bind(&now)
            .bind(node_id)
            .execute(pool)
            .await?;
        }
        Db::Postgres(pool) => {
            sqlx::query(
                "UPDATE nodes SET encrypted_server_private_key = $1, server_public_key = $2, updated_at = $3 WHERE id = $4",
            )
            .bind(&enc)
            .bind(&pub_b64)
            .bind(&now)
            .bind(node_id)
            .execute(pool)
            .await?;
        }
    }
    let _ = row.server_public_key;
    Ok(kp.private_b64())
}

async fn load_peers(db: &Db, node_id: &str) -> Result<Vec<ClassicPeerConfig>> {
    let rows: Vec<(Option<String>, Option<String>, Option<String>)> = match db {
        Db::Sqlite(pool) => {
            sqlx::query_as(
                r#"
                SELECT client_public_key, assigned_ipv4, assigned_ipv6
                FROM profiles
                WHERE node_id = ? AND revoked_at IS NULL AND client_public_key IS NOT NULL
                "#,
            )
            .bind(node_id)
            .fetch_all(pool)
            .await?
        }
        Db::Postgres(pool) => {
            sqlx::query_as(
                r#"
                SELECT client_public_key, assigned_ipv4, assigned_ipv6
                FROM profiles
                WHERE node_id = $1 AND revoked_at IS NULL AND client_public_key IS NOT NULL
                "#,
            )
            .bind(node_id)
            .fetch_all(pool)
            .await?
        }
    };
    let mut peers = Vec::new();
    for (pubk, v4, v6) in rows {
        let Some(public_key) = pubk.filter(|s| !s.is_empty()) else {
            continue;
        };
        let mut allowed_ips = Vec::new();
        if let Some(a) = v4.filter(|s| !s.is_empty()) {
            allowed_ips.push(a);
        }
        if let Some(a) = v6.filter(|s| !s.is_empty()) {
            allowed_ips.push(a);
        }
        if allowed_ips.is_empty() {
            continue;
        }
        peers.push(ClassicPeerConfig {
            public_key,
            allowed_ips,
        });
    }
    Ok(peers)
}

async fn persist_cidrs(db: &Db, node_id: &str, v4: &str, v6: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    match db {
        Db::Sqlite(pool) => {
            sqlx::query(
                "UPDATE nodes SET tunnel_cidr_v4 = COALESCE(tunnel_cidr_v4, ?), tunnel_cidr_v6 = COALESCE(tunnel_cidr_v6, ?), updated_at = ? WHERE id = ?",
            )
            .bind(v4)
            .bind(v6)
            .bind(&now)
            .bind(node_id)
            .execute(pool)
            .await?;
        }
        Db::Postgres(pool) => {
            sqlx::query(
                "UPDATE nodes SET tunnel_cidr_v4 = COALESCE(tunnel_cidr_v4, $1), tunnel_cidr_v6 = COALESCE(tunnel_cidr_v6, $2), updated_at = $3 WHERE id = $4",
            )
            .bind(v4)
            .bind(v6)
            .bind(&now)
            .bind(node_id)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}
