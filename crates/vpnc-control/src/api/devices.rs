//! Device management API.

use crate::api::error::ApiError;
use crate::api::middleware::{write_audit, AuthUser};
use crate::app::AppState;
use crate::db::Db;
use crate::services::users::UserRole;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Extension, Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Builds device routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/devices", get(list_devices).post(create_device))
        .route(
            "/api/devices/{id}",
            get(get_device).patch(update_device).delete(delete_device),
        )
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    cursor: Option<String>,
    limit: Option<i64>,
    /// Admin-only: filter by owner.
    user_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct CreateDeviceRequest {
    name: String,
    /// Admin may create a device for another user.
    user_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct UpdateDeviceRequest {
    name: String,
}

#[derive(Debug, Serialize)]
struct DeviceResponse {
    id: String,
    user_id: String,
    name: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct ListResponse {
    items: Vec<DeviceResponse>,
    next_cursor: Option<String>,
}

async fn list_devices(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, ApiError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let owner = match actor.role {
        UserRole::Admin => q.user_id,
        UserRole::User => {
            if q.user_id.is_some_and(|id| id != actor.id) {
                return Err(ApiError::forbidden("cannot list other users' devices"));
            }
            Some(actor.id)
        }
    };
    let mut items = fetch_devices(&state.db, owner, q.cursor.as_deref(), limit + 1)
        .await
        .map_err(ApiError::internal)?;
    let mut next_cursor = None;
    if (items.len() as i64) > limit {
        items.truncate(limit as usize);
        next_cursor = items.last().map(|d| d.id.clone());
    }
    Ok(Json(ListResponse { items, next_cursor }))
}

async fn create_device(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Json(body): Json<CreateDeviceRequest>,
) -> Result<(axum::http::StatusCode, Json<DeviceResponse>), ApiError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("name required"));
    }
    let owner = match (actor.role, body.user_id) {
        (UserRole::Admin, Some(uid)) => uid,
        (UserRole::Admin, None) => actor.id,
        (UserRole::User, Some(uid)) if uid != actor.id => {
            return Err(ApiError::forbidden("cannot create device for another user"));
        }
        (UserRole::User, _) => actor.id,
    };
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    insert_device(&state.db, id, owner, name, &now)
        .await
        .map_err(ApiError::internal)?;
    write_audit(
        &state.db,
        Some(actor.id),
        "device.create",
        Some("device"),
        Some(&id.to_string()),
        None,
    )
    .await;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(DeviceResponse {
            id: id.to_string(),
            user_id: owner.to_string(),
            name: name.to_string(),
            created_at: now,
        }),
    ))
}

async fn get_device(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeviceResponse>, ApiError> {
    let device = fetch_device(&state.db, id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("device not found"))?;
    ensure_device_access(&actor, &device)?;
    Ok(Json(device))
}

async fn update_device(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateDeviceRequest>,
) -> Result<Json<DeviceResponse>, ApiError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("name required"));
    }
    let mut device = fetch_device(&state.db, id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("device not found"))?;
    ensure_device_access(&actor, &device)?;
    let now = Utc::now().to_rfc3339();
    match &state.db {
        Db::Sqlite(pool) => sqlx::query("UPDATE devices SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(&now)
            .bind(id.to_string())
            .execute(pool)
            .await
            .map(|_| ()),
        Db::Postgres(pool) => {
            sqlx::query("UPDATE devices SET name = $1, updated_at = $2 WHERE id = $3")
                .bind(name)
                .bind(&now)
                .bind(id.to_string())
                .execute(pool)
                .await
                .map(|_| ())
        }
    }
    .map_err(ApiError::internal)?;
    write_audit(
        &state.db,
        Some(actor.id),
        "device.update",
        Some("device"),
        Some(&id.to_string()),
        None,
    )
    .await;
    device.name = name.to_string();
    Ok(Json(device))
}

async fn delete_device(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, ApiError> {
    let device = fetch_device(&state.db, id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("device not found"))?;
    ensure_device_access(&actor, &device)?;
    delete_device_row(&state.db, id)
        .await
        .map_err(ApiError::internal)?;
    write_audit(
        &state.db,
        Some(actor.id),
        "device.delete",
        Some("device"),
        Some(&id.to_string()),
        None,
    )
    .await;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

fn ensure_device_access(
    actor: &crate::services::users::UserRecord,
    device: &DeviceResponse,
) -> Result<(), ApiError> {
    if actor.role == UserRole::Admin {
        return Ok(());
    }
    if device.user_id != actor.id.to_string() {
        return Err(ApiError::forbidden("not your device"));
    }
    Ok(())
}

async fn fetch_devices(
    db: &Db,
    owner: Option<Uuid>,
    cursor: Option<&str>,
    limit: i64,
) -> anyhow::Result<Vec<DeviceResponse>> {
    let rows: Vec<(String, String, String, String)> = match db {
        Db::Sqlite(pool) => match (owner, cursor) {
            (Some(uid), Some(c)) => {
                sqlx::query_as(
                    "SELECT id, user_id, name, created_at FROM devices WHERE user_id = ? AND id > ? ORDER BY id ASC LIMIT ?",
                )
                .bind(uid.to_string())
                .bind(c)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            (Some(uid), None) => {
                sqlx::query_as(
                    "SELECT id, user_id, name, created_at FROM devices WHERE user_id = ? ORDER BY id ASC LIMIT ?",
                )
                .bind(uid.to_string())
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            (None, Some(c)) => {
                sqlx::query_as(
                    "SELECT id, user_id, name, created_at FROM devices WHERE id > ? ORDER BY id ASC LIMIT ?",
                )
                .bind(c)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            (None, None) => {
                sqlx::query_as(
                    "SELECT id, user_id, name, created_at FROM devices ORDER BY id ASC LIMIT ?",
                )
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
        },
        Db::Postgres(pool) => match (owner, cursor) {
            (Some(uid), Some(c)) => {
                sqlx::query_as(
                    "SELECT id, user_id, name, created_at FROM devices WHERE user_id = $1 AND id > $2 ORDER BY id ASC LIMIT $3",
                )
                .bind(uid.to_string())
                .bind(c)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            (Some(uid), None) => {
                sqlx::query_as(
                    "SELECT id, user_id, name, created_at FROM devices WHERE user_id = $1 ORDER BY id ASC LIMIT $2",
                )
                .bind(uid.to_string())
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            (None, Some(c)) => {
                sqlx::query_as(
                    "SELECT id, user_id, name, created_at FROM devices WHERE id > $1 ORDER BY id ASC LIMIT $2",
                )
                .bind(c)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            (None, None) => {
                sqlx::query_as(
                    "SELECT id, user_id, name, created_at FROM devices ORDER BY id ASC LIMIT $1",
                )
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
        },
    };
    Ok(rows
        .into_iter()
        .map(|(id, user_id, name, created_at)| DeviceResponse {
            id,
            user_id,
            name,
            created_at,
        })
        .collect())
}

async fn fetch_device(db: &Db, id: Uuid) -> anyhow::Result<Option<DeviceResponse>> {
    let row: Option<(String, String, String, String)> = match db {
        Db::Sqlite(pool) => {
            sqlx::query_as("SELECT id, user_id, name, created_at FROM devices WHERE id = ?")
                .bind(id.to_string())
                .fetch_optional(pool)
                .await?
        }
        Db::Postgres(pool) => {
            sqlx::query_as("SELECT id, user_id, name, created_at FROM devices WHERE id = $1")
                .bind(id.to_string())
                .fetch_optional(pool)
                .await?
        }
    };
    Ok(row.map(|(id, user_id, name, created_at)| DeviceResponse {
        id,
        user_id,
        name,
        created_at,
    }))
}

async fn insert_device(
    db: &Db,
    id: Uuid,
    user_id: Uuid,
    name: &str,
    now: &str,
) -> anyhow::Result<()> {
    match db {
        Db::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO devices (id, user_id, name, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(id.to_string())
            .bind(user_id.to_string())
            .bind(name)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }
        Db::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO devices (id, user_id, name, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(id.to_string())
            .bind(user_id.to_string())
            .bind(name)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

async fn delete_device_row(db: &Db, id: Uuid) -> anyhow::Result<()> {
    match db {
        Db::Sqlite(pool) => {
            sqlx::query("DELETE FROM devices WHERE id = ?")
                .bind(id.to_string())
                .execute(pool)
                .await?;
        }
        Db::Postgres(pool) => {
            sqlx::query("DELETE FROM devices WHERE id = $1")
                .bind(id.to_string())
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}
