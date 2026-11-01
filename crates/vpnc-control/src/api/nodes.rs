//! Node management and join-token API.

use crate::api::error::ApiError;
use crate::api::middleware::{require_admin, write_audit, AuthUser};
use crate::app::AppState;
use crate::db::Db;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use chrono::{Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use vpnc_common::NodeRole;

/// Builds node routes (admin-only).
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/nodes", get(list_nodes).post(create_node))
        .route(
            "/api/nodes/{id}",
            get(get_node).patch(update_node).delete(delete_node),
        )
        .route("/api/nodes/join-tokens", post(create_join_token))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    cursor: Option<String>,
    limit: Option<i64>,
    role: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateNodeRequest {
    name: String,
    role: NodeRole,
    tunnel_cidr_v4: Option<String>,
    tunnel_cidr_v6: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateNodeRequest {
    /// Public WireGuard endpoint host (IP or DNS name), without port.
    public_ip: Option<String>,
}

#[derive(Debug, Serialize)]
struct NodeResponse {
    id: String,
    name: String,
    role: String,
    status: String,
    public_ip: Option<String>,
    last_seen_at: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct ListResponse {
    items: Vec<NodeResponse>,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JoinTokenRequest {
    role: NodeRole,
    node_name: Option<String>,
    /// Lifetime in minutes (default 60, max 1440).
    ttl_minutes: Option<i64>,
}

#[derive(Debug, Serialize)]
struct JoinTokenResponse {
    token: String,
    expires_at: String,
    /// Ready-to-copy CLI install command.
    install_command: String,
    /// Ready-to-copy docker run example.
    docker_command: String,
}

async fn list_nodes(
    State(state): State<AppState>,
    Extension(AuthUser(_actor)): Extension<AuthUser>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, ApiError> {
    // Authenticated users may list nodes (needed to pick a node when creating a profile).
    // Mutations remain admin-only.
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let mut items = fetch_nodes(
        &state.db,
        q.role.as_deref(),
        q.status.as_deref(),
        q.cursor.as_deref(),
        limit + 1,
    )
    .await
    .map_err(ApiError::internal)?;
    let mut next_cursor = None;
    if (items.len() as i64) > limit {
        items.truncate(limit as usize);
        next_cursor = items.last().map(|n| n.id.clone());
    }
    Ok(Json(ListResponse { items, next_cursor }))
}

async fn create_node(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Json(body): Json<CreateNodeRequest>,
) -> Result<(axum::http::StatusCode, Json<NodeResponse>), ApiError> {
    require_admin(&actor)?;
    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("name required"));
    }
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    insert_node(
        &state.db,
        id,
        name,
        &body.role.to_string(),
        body.tunnel_cidr_v4.as_deref(),
        body.tunnel_cidr_v6.as_deref(),
        &now,
    )
    .await
    .map_err(ApiError::internal)?;
    write_audit(
        &state.db,
        Some(actor.id),
        "node.create",
        Some("node"),
        Some(&id.to_string()),
        None,
    )
    .await;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(NodeResponse {
            id: id.to_string(),
            name: name.to_string(),
            role: body.role.to_string(),
            status: "pending".into(),
            public_ip: None,
            last_seen_at: None,
            created_at: now,
        }),
    ))
}

async fn get_node(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<NodeResponse>, ApiError> {
    require_admin(&actor)?;
    fetch_node(&state.db, id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("node not found"))
        .map(Json)
}

async fn update_node(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateNodeRequest>,
) -> Result<Json<NodeResponse>, ApiError> {
    require_admin(&actor)?;
    let Some(public_ip) = body.public_ip.as_ref() else {
        return Err(ApiError::bad_request("public_ip required"));
    };
    let public_ip = public_ip.trim();
    if public_ip.is_empty() {
        return Err(ApiError::bad_request("public_ip required"));
    }
    if public_ip.contains(|c: char| c.is_whitespace()) || public_ip.contains('/') {
        return Err(ApiError::bad_request("public_ip must be a host or IP"));
    }
    // Reject host:port forms (IPv4 or bracketed IPv6); bare IPv6 with colons is fine.
    if public_ip
        .rsplit_once(':')
        .is_some_and(|(host, port)| !host.is_empty() && port.parse::<u16>().is_ok())
        && (public_ip.matches('.').count() == 3 || public_ip.starts_with('['))
    {
        return Err(ApiError::bad_request(
            "public_ip must be a host or IP without port",
        ));
    }
    let public_ip = public_ip.trim_start_matches('[').trim_end_matches(']');
    let now = Utc::now().to_rfc3339();
    let updated = match &state.db {
        Db::Sqlite(pool) => sqlx::query("UPDATE nodes SET public_ip = ?, updated_at = ? WHERE id = ?")
            .bind(public_ip)
            .bind(&now)
            .bind(id.to_string())
            .execute(pool)
            .await
            .map(|r| r.rows_affected()),
        Db::Postgres(pool) => {
            sqlx::query("UPDATE nodes SET public_ip = $1, updated_at = $2 WHERE id = $3")
                .bind(public_ip)
                .bind(&now)
                .bind(id.to_string())
                .execute(pool)
                .await
                .map(|r| r.rows_affected())
        }
    }
    .map_err(ApiError::internal)?;
    if updated == 0 {
        return Err(ApiError::not_found("node not found"));
    }
    write_audit(
        &state.db,
        Some(actor.id),
        "node.update",
        Some("node"),
        Some(&id.to_string()),
        None,
    )
    .await;
    fetch_node(&state.db, id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("node not found"))
        .map(Json)
}

async fn delete_node(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, ApiError> {
    require_admin(&actor)?;
    let deleted = match &state.db {
        Db::Sqlite(pool) => sqlx::query("DELETE FROM nodes WHERE id = ?")
            .bind(id.to_string())
            .execute(pool)
            .await
            .map(|r| r.rows_affected()),
        Db::Postgres(pool) => sqlx::query("DELETE FROM nodes WHERE id = $1")
            .bind(id.to_string())
            .execute(pool)
            .await
            .map(|r| r.rows_affected()),
    }
    .map_err(ApiError::internal)?;
    if deleted == 0 {
        return Err(ApiError::not_found("node not found"));
    }
    write_audit(
        &state.db,
        Some(actor.id),
        "node.delete",
        Some("node"),
        Some(&id.to_string()),
        None,
    )
    .await;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn create_join_token(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Json(body): Json<JoinTokenRequest>,
) -> Result<Json<JoinTokenResponse>, ApiError> {
    require_admin(&actor)?;
    let ttl = body.ttl_minutes.unwrap_or(60).clamp(1, 1440);
    let expires = Utc::now() + Duration::minutes(ttl);
    let mut raw = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut raw);
    let token = hex::encode(raw);
    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    match &state.db {
        Db::Sqlite(pool) => {
            sqlx::query(
                r#"
                INSERT INTO node_join_tokens (id, token_hash, node_name, role, expires_at, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(id.to_string())
            .bind(&token_hash)
            .bind(&body.node_name)
            .bind(body.role.to_string())
            .bind(expires.to_rfc3339())
            .bind(&now)
            .execute(pool)
            .await
            .map(|_| ())
        }
        Db::Postgres(pool) => {
            sqlx::query(
                r#"
                INSERT INTO node_join_tokens (id, token_hash, node_name, role, expires_at, created_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(id.to_string())
            .bind(&token_hash)
            .bind(&body.node_name)
            .bind(body.role.to_string())
            .bind(expires.to_rfc3339())
            .bind(&now)
            .execute(pool)
            .await
            .map(|_| ())
        }
    }
    .map_err(ApiError::internal)?;
    write_audit(
        &state.db,
        Some(actor.id),
        "node.join_token.create",
        Some("node_join_token"),
        Some(&id.to_string()),
        None,
    )
    .await;
    let grpc = state
        .config
        .public_grpc
        .clone()
        .unwrap_or_else(|| state.config.grpc_bind.clone());
    let grpc_url = if grpc.starts_with("http://") || grpc.starts_with("https://") {
        grpc
    } else {
        format!("http://{grpc}")
    };
    let name_flag = body
        .node_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|n| format!(" --name {n}"))
        .unwrap_or_default();
    let role = body.role.to_string();
    let install_command =
        format!("vpnc-node run --control {grpc_url} --join-token {token} --role {role}{name_flag}");
    let docker_command = format!(
        "docker run --rm --cap-add=NET_ADMIN --device=/dev/net/tun -e VPNC_JOIN_TOKEN={token} ghcr.io/vpnc/vpnc-node:latest run --control {grpc_url} --join-token {token} --role {role}{name_flag}"
    );
    Ok(Json(JoinTokenResponse {
        token,
        expires_at: expires.to_rfc3339(),
        install_command,
        docker_command,
    }))
}

async fn fetch_nodes(
    db: &Db,
    role: Option<&str>,
    status: Option<&str>,
    cursor: Option<&str>,
    limit: i64,
) -> anyhow::Result<Vec<NodeResponse>> {
    // Simplified filtered queries for SQLite; Postgres mirrors.
    let rows: Vec<(
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
    )> = match db {
        Db::Sqlite(pool) => {
            sqlx::query_as(
                r#"
                SELECT id, name, role, status, public_ip, last_seen_at, created_at
                FROM nodes
                WHERE (?1 IS NULL OR role = ?1)
                  AND (?2 IS NULL OR status = ?2)
                  AND (?3 IS NULL OR id > ?3)
                ORDER BY id ASC LIMIT ?4
                "#,
            )
            .bind(role)
            .bind(status)
            .bind(cursor)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        Db::Postgres(pool) => {
            sqlx::query_as(
                r#"
                SELECT id, name, role, status, public_ip, last_seen_at, created_at
                FROM nodes
                WHERE ($1::text IS NULL OR role = $1)
                  AND ($2::text IS NULL OR status = $2)
                  AND ($3::text IS NULL OR id > $3)
                ORDER BY id ASC LIMIT $4
                "#,
            )
            .bind(role)
            .bind(status)
            .bind(cursor)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
    };
    Ok(rows
        .into_iter()
        .map(
            |(id, name, role, status, public_ip, last_seen_at, created_at)| NodeResponse {
                id,
                name,
                role,
                status,
                public_ip,
                last_seen_at,
                created_at,
            },
        )
        .collect())
}

async fn fetch_node(db: &Db, id: Uuid) -> anyhow::Result<Option<NodeResponse>> {
    let row: Option<(
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
    )> = match db {
        Db::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, name, role, status, public_ip, last_seen_at, created_at FROM nodes WHERE id = ?",
            )
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?
        }
        Db::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, name, role, status, public_ip, last_seen_at, created_at FROM nodes WHERE id = $1",
            )
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?
        }
    };
    Ok(row.map(
        |(id, name, role, status, public_ip, last_seen_at, created_at)| NodeResponse {
            id,
            name,
            role,
            status,
            public_ip,
            last_seen_at,
            created_at,
        },
    ))
}

async fn insert_node(
    db: &Db,
    id: Uuid,
    name: &str,
    role: &str,
    cidr_v4: Option<&str>,
    cidr_v6: Option<&str>,
    now: &str,
) -> anyhow::Result<()> {
    match db {
        Db::Sqlite(pool) => {
            sqlx::query(
                r#"
                INSERT INTO nodes (id, name, role, status, tunnel_cidr_v4, tunnel_cidr_v6, created_at, updated_at)
                VALUES (?, ?, ?, 'pending', ?, ?, ?, ?)
                "#,
            )
            .bind(id.to_string())
            .bind(name)
            .bind(role)
            .bind(cidr_v4)
            .bind(cidr_v6)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }
        Db::Postgres(pool) => {
            sqlx::query(
                r#"
                INSERT INTO nodes (id, name, role, status, tunnel_cidr_v4, tunnel_cidr_v6, created_at, updated_at)
                VALUES ($1, $2, $3, 'pending', $4, $5, $6, $7)
                "#,
            )
            .bind(id.to_string())
            .bind(name)
            .bind(role)
            .bind(cidr_v4)
            .bind(cidr_v6)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}
