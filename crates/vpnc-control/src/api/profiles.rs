//! Classic WireGuard profile API.

use crate::api::error::ApiError;
use crate::api::middleware::{write_audit, AuthUser};
use crate::app::AppState;
use crate::db::Db;
use crate::services::ipam::IpAllocator;
use crate::services::users::UserRole;
use crate::services::wireguard::{
    conf_qr_png, render_client_conf, validate_conf, ClientConfParams, WgKeypair,
};
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

/// Builds profile routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/profiles", post(create_profile))
        .route("/api/profiles/{id}/download", get(download_profile))
        .route("/api/profiles/{id}/revoke", post(revoke_profile))
}

#[derive(Debug, Deserialize)]
struct CreateProfileRequest {
    device_id: Uuid,
    node_id: Uuid,
}

#[derive(Debug, Serialize)]
struct CreateProfileResponse {
    id: String,
    client_public_key: String,
    assigned_ipv4: Option<String>,
    assigned_ipv6: Option<String>,
    /// One-time download token hint — actual private key only in download once.
    download_once: bool,
}

// Process-local one-time private key store (not persisted).
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn pending_keys() -> &'static Mutex<HashMap<Uuid, String>> {
    static PENDING_KEYS: OnceLock<Mutex<HashMap<Uuid, String>>> = OnceLock::new();
    PENDING_KEYS.get_or_init(|| Mutex::new(HashMap::new()))
}

async fn create_profile(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Json(body): Json<CreateProfileRequest>,
) -> Result<(StatusCode, Json<CreateProfileResponse>), ApiError> {
    let device = fetch_device_owner(&state.db, body.device_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("device not found"))?;
    if actor.role != UserRole::Admin && device != actor.id {
        return Err(ApiError::forbidden("not your device"));
    }

    let node = fetch_classic_node(&state.db, body.node_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("node not found"))?;

    // Ensure server keypair exists (encrypted).
    let server_public = ensure_server_keypair(&state, body.node_id, &node)
        .await
        .map_err(ApiError::internal)?;

    let mut ipam = load_ipam(&state.db, body.node_id)
        .await
        .map_err(ApiError::internal)?;
    let v4 = match &node.tunnel_cidr_v4 {
        Some(cidr) => Some(
            ipam.allocate_v4(cidr)
                .map_err(|e| ApiError::bad_request(e.to_string()))?,
        ),
        None => None,
    };
    let v6 = match &node.tunnel_cidr_v6 {
        Some(cidr) => Some(
            ipam.allocate_v6(cidr)
                .map_err(|e| ApiError::bad_request(e.to_string()))?,
        ),
        None => None,
    };

    let client = WgKeypair::generate();
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    insert_profile(
        &state.db,
        id,
        body.device_id,
        body.node_id,
        &client.public_b64(),
        v4.map(|a| format!("{a}/32")).as_deref(),
        v6.map(|a| format!("{a}/128")).as_deref(),
        &now,
    )
    .await
    .map_err(ApiError::internal)?;

    pending_keys()
        .lock()
        .expect("pending keys")
        .insert(id, client.private_b64());

    write_audit(
        &state.db,
        Some(actor.id),
        "profile.create",
        Some("profile"),
        Some(&id.to_string()),
        None,
    )
    .await;

    let _ = server_public;
    Ok((
        StatusCode::CREATED,
        Json(CreateProfileResponse {
            id: id.to_string(),
            client_public_key: client.public_b64(),
            assigned_ipv4: v4.map(|a| format!("{a}/32")),
            assigned_ipv6: v6.map(|a| format!("{a}/128")),
            download_once: true,
        }),
    ))
}

async fn download_profile(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<(HeaderMap, Vec<u8>), ApiError> {
    let profile = fetch_profile(&state.db, id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("profile not found"))?;
    if profile.revoked {
        return Err(ApiError::bad_request("profile revoked"));
    }
    let owner = fetch_device_owner(&state.db, profile.device_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("device not found"))?;
    if actor.role != UserRole::Admin && owner != actor.id {
        return Err(ApiError::forbidden("not your profile"));
    }

    let private = pending_keys()
        .lock()
        .expect("pending keys")
        .remove(&id)
        .ok_or_else(|| ApiError::bad_request("private key already downloaded or unavailable"))?;

    let node = fetch_classic_node(&state.db, profile.node_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("node not found"))?;
    let server_public = node
        .server_public_key
        .ok_or_else(|| ApiError::internal("node missing server public key"))?;
    let endpoint = node
        .public_ip
        .map(|ip| format!("{ip}:51820"))
        .unwrap_or_else(|| "CHANGE_ME:51820".into());

    let conf = render_client_conf(&ClientConfParams {
        client_private_b64: &private,
        address_v4: profile.assigned_ipv4.as_deref(),
        address_v6: profile.assigned_ipv6.as_deref(),
        server_public_b64: &server_public,
        endpoint: &endpoint,
        allowed_ips: "0.0.0.0/0, ::/0",
        dns: Some("1.1.1.1"),
    });
    validate_conf(&conf).map_err(|e| ApiError::internal(e))?;
    let qr = conf_qr_png(&conf).map_err(ApiError::internal)?;

    // Multipart-ish simple JSON envelope with conf + qr base64 (one response).
    #[derive(Serialize)]
    struct DownloadBody {
        conf: String,
        qr_png_base64: String,
    }
    use base64::Engine;
    let body = serde_json::to_vec(&DownloadBody {
        conf,
        qr_png_base64: base64::engine::general_purpose::STANDARD.encode(qr),
    })
    .map_err(ApiError::internal)?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    Ok((headers, body))
}

async fn revoke_profile(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let profile = fetch_profile(&state.db, id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("profile not found"))?;
    let owner = fetch_device_owner(&state.db, profile.device_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("device not found"))?;
    if actor.role != UserRole::Admin && owner != actor.id {
        return Err(ApiError::forbidden("not your profile"));
    }
    let now = Utc::now().to_rfc3339();
    match &state.db {
        Db::Sqlite(pool) => sqlx::query("UPDATE profiles SET revoked_at = ? WHERE id = ?")
            .bind(&now)
            .bind(id.to_string())
            .execute(pool)
            .await
            .map(|_| ()),
        Db::Postgres(pool) => sqlx::query("UPDATE profiles SET revoked_at = $1 WHERE id = $2")
            .bind(&now)
            .bind(id.to_string())
            .execute(pool)
            .await
            .map(|_| ()),
    }
    .map_err(ApiError::internal)?;
    pending_keys().lock().expect("pending keys").remove(&id);
    if let Some(v4) = profile.assigned_ipv4.as_deref() {
        let _ = v4; // release handled by not reusing until IPAM reload skips revoked
    }
    write_audit(
        &state.db,
        Some(actor.id),
        "profile.revoke",
        Some("profile"),
        Some(&id.to_string()),
        None,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

struct NodeRow {
    tunnel_cidr_v4: Option<String>,
    tunnel_cidr_v6: Option<String>,
    server_public_key: Option<String>,
    encrypted_server_private_key: Option<String>,
    public_ip: Option<String>,
}

struct ProfileRow {
    device_id: Uuid,
    node_id: Uuid,
    assigned_ipv4: Option<String>,
    assigned_ipv6: Option<String>,
    revoked: bool,
}

async fn fetch_device_owner(db: &Db, id: Uuid) -> anyhow::Result<Option<Uuid>> {
    let row: Option<String> = match db {
        Db::Sqlite(pool) => {
            sqlx::query_scalar("SELECT user_id FROM devices WHERE id = ?")
                .bind(id.to_string())
                .fetch_optional(pool)
                .await?
        }
        Db::Postgres(pool) => {
            sqlx::query_scalar("SELECT user_id FROM devices WHERE id = $1")
                .bind(id.to_string())
                .fetch_optional(pool)
                .await?
        }
    };
    Ok(row.map(|s| Uuid::parse_str(&s)).transpose()?)
}

async fn fetch_classic_node(db: &Db, id: Uuid) -> anyhow::Result<Option<NodeRow>> {
    let row: Option<(
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    )> = match db {
        Db::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT tunnel_cidr_v4, tunnel_cidr_v6, server_public_key, encrypted_server_private_key, public_ip, role FROM nodes WHERE id = ?",
            )
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?
        }
        Db::Postgres(pool) => {
            sqlx::query_as(
                "SELECT tunnel_cidr_v4, tunnel_cidr_v6, server_public_key, encrypted_server_private_key, public_ip, role FROM nodes WHERE id = $1",
            )
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?
        }
    };
    Ok(match row {
        Some((v4, v6, pubk, enc, ip, role)) if role == "classic" => Some(NodeRow {
            tunnel_cidr_v4: v4,
            tunnel_cidr_v6: v6,
            server_public_key: pubk,
            encrypted_server_private_key: enc,
            public_ip: ip,
        }),
        Some(_) => None,
        None => None,
    })
}

async fn ensure_server_keypair(
    state: &AppState,
    node_id: Uuid,
    node: &NodeRow,
) -> anyhow::Result<String> {
    if let Some(pubk) = &node.server_public_key {
        return Ok(pubk.clone());
    }
    let kp = WgKeypair::generate();
    let enc = state.crypto.encrypt(kp.private.expose())?;
    let pub_b64 = kp.public_b64();
    match &state.db {
        Db::Sqlite(pool) => {
            sqlx::query(
                "UPDATE nodes SET encrypted_server_private_key = ?, server_public_key = ?, updated_at = ? WHERE id = ?",
            )
            .bind(&enc)
            .bind(&pub_b64)
            .bind(Utc::now().to_rfc3339())
            .bind(node_id.to_string())
            .execute(pool)
            .await?;
        }
        Db::Postgres(pool) => {
            sqlx::query(
                "UPDATE nodes SET encrypted_server_private_key = $1, server_public_key = $2, updated_at = $3 WHERE id = $4",
            )
            .bind(&enc)
            .bind(&pub_b64)
            .bind(Utc::now().to_rfc3339())
            .bind(node_id.to_string())
            .execute(pool)
            .await?;
        }
    }
    Ok(pub_b64)
}

async fn load_ipam(db: &Db, node_id: Uuid) -> anyhow::Result<IpAllocator> {
    let rows: Vec<(Option<String>, Option<String>)> = match db {
        Db::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT assigned_ipv4, assigned_ipv6 FROM profiles WHERE node_id = ? AND revoked_at IS NULL",
            )
            .bind(node_id.to_string())
            .fetch_all(pool)
            .await?
        }
        Db::Postgres(pool) => {
            sqlx::query_as(
                "SELECT assigned_ipv4, assigned_ipv6 FROM profiles WHERE node_id = $1 AND revoked_at IS NULL",
            )
            .bind(node_id.to_string())
            .fetch_all(pool)
            .await?
        }
    };
    let mut ipam = IpAllocator::new();
    for (v4, v6) in rows {
        if let Some(v4) = v4 {
            if let Ok(ip) = v4.split('/').next().unwrap_or(&v4).parse::<IpAddr>() {
                ipam.mark_used(ip);
            }
        }
        if let Some(v6) = v6 {
            if let Ok(ip) = v6.split('/').next().unwrap_or(&v6).parse::<IpAddr>() {
                ipam.mark_used(ip);
            }
        }
    }
    Ok(ipam)
}

async fn insert_profile(
    db: &Db,
    id: Uuid,
    device_id: Uuid,
    node_id: Uuid,
    client_public: &str,
    v4: Option<&str>,
    v6: Option<&str>,
    now: &str,
) -> anyhow::Result<()> {
    match db {
        Db::Sqlite(pool) => {
            sqlx::query(
                r#"
                INSERT INTO profiles (id, device_id, node_id, kind, client_public_key, assigned_ipv4, assigned_ipv6, created_at)
                VALUES (?, ?, ?, 'wireguard', ?, ?, ?, ?)
                "#,
            )
            .bind(id.to_string())
            .bind(device_id.to_string())
            .bind(node_id.to_string())
            .bind(client_public)
            .bind(v4)
            .bind(v6)
            .bind(now)
            .execute(pool)
            .await?;
        }
        Db::Postgres(pool) => {
            sqlx::query(
                r#"
                INSERT INTO profiles (id, device_id, node_id, kind, client_public_key, assigned_ipv4, assigned_ipv6, created_at)
                VALUES ($1, $2, $3, 'wireguard', $4, $5, $6, $7)
                "#,
            )
            .bind(id.to_string())
            .bind(device_id.to_string())
            .bind(node_id.to_string())
            .bind(client_public)
            .bind(v4)
            .bind(v6)
            .bind(now)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

async fn fetch_profile(db: &Db, id: Uuid) -> anyhow::Result<Option<ProfileRow>> {
    let row: Option<(String, String, Option<String>, Option<String>, Option<String>)> = match db {
        Db::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT device_id, node_id, assigned_ipv4, assigned_ipv6, revoked_at FROM profiles WHERE id = ?",
            )
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?
        }
        Db::Postgres(pool) => {
            sqlx::query_as(
                "SELECT device_id, node_id, assigned_ipv4, assigned_ipv6, revoked_at FROM profiles WHERE id = $1",
            )
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?
        }
    };
    Ok(match row {
        Some((device_id, node_id, v4, v6, revoked_at)) => Some(ProfileRow {
            device_id: Uuid::parse_str(&device_id)?,
            node_id: Uuid::parse_str(&node_id)?,
            assigned_ipv4: v4,
            assigned_ipv6: v6,
            revoked: revoked_at.is_some(),
        }),
        None => None,
    })
}
