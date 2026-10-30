//! User management API.

use crate::api::error::ApiError;
use crate::api::middleware::{require_admin, write_audit, AuthUser};
use crate::app::AppState;
use crate::db::Db;
use crate::services::users::{self, UserRole};
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Extension, Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Builds user routes (admin-only).
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/users", get(list_users).post(create_user))
        .route(
            "/api/users/{id}",
            get(get_user).patch(update_user).delete(delete_user),
        )
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CreateUserRequest {
    email: String,
    password: String,
    role: UserRole,
}

#[derive(Debug, Deserialize)]
struct UpdateUserRequest {
    role: Option<UserRole>,
    password: Option<String>,
    /// Soft-disable (`true`) or re-enable (`false`).
    disabled: Option<bool>,
}

#[derive(Debug, Serialize)]
struct UserResponse {
    id: String,
    email: String,
    role: UserRole,
    disabled: bool,
}

#[derive(Debug, Serialize)]
struct ListResponse {
    items: Vec<UserResponse>,
    next_cursor: Option<String>,
}

async fn list_users(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, ApiError> {
    require_admin(&actor)?;
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let mut items = fetch_users(&state.db, q.cursor.as_deref(), limit + 1)
        .await
        .map_err(ApiError::internal)?;
    let mut next_cursor = None;
    if (items.len() as i64) > limit {
        items.truncate(limit as usize);
        next_cursor = items.last().map(|u| u.id.clone());
    }
    Ok(Json(ListResponse { items, next_cursor }))
}

async fn create_user(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Json(body): Json<CreateUserRequest>,
) -> Result<(axum::http::StatusCode, Json<UserResponse>), ApiError> {
    require_admin(&actor)?;
    if body.password.len() < 10 {
        return Err(ApiError::bad_request(
            "password must be at least 10 characters",
        ));
    }
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let email = body.email.trim().to_lowercase();
    let password_hash = users::hash_password(&body.password).map_err(ApiError::internal)?;
    insert_user(
        &state.db,
        id,
        &email,
        &password_hash,
        body.role.as_str(),
        &now,
    )
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("UNIQUE") || msg.contains("unique") {
            ApiError::bad_request("email already exists")
        } else {
            ApiError::internal(e)
        }
    })?;
    write_audit(
        &state.db,
        Some(actor.id),
        "user.create",
        Some("user"),
        Some(&id.to_string()),
        None,
    )
    .await;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(UserResponse {
            id: id.to_string(),
            email,
            role: body.role,
            disabled: false,
        }),
    ))
}

async fn get_user(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, ApiError> {
    require_admin(&actor)?;
    fetch_user_response(&state.db, id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("user not found"))
        .map(Json)
}

async fn update_user(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    require_admin(&actor)?;
    let _existing = fetch_user_response(&state.db, id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("user not found"))?;
    let now = Utc::now().to_rfc3339();
    if let Some(role) = body.role {
        match &state.db {
            Db::Sqlite(pool) => {
                sqlx::query("UPDATE users SET role = ?, updated_at = ? WHERE id = ?")
                    .bind(role.as_str())
                    .bind(&now)
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map(|_| ())
            }
            Db::Postgres(pool) => {
                sqlx::query("UPDATE users SET role = $1, updated_at = $2 WHERE id = $3")
                    .bind(role.as_str())
                    .bind(&now)
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map(|_| ())
            }
        }
        .map_err(ApiError::internal)?;
    }
    if let Some(password) = body.password {
        if password.len() < 10 {
            return Err(ApiError::bad_request(
                "password must be at least 10 characters",
            ));
        }
        let hash = users::hash_password(&password).map_err(ApiError::internal)?;
        match &state.db {
            Db::Sqlite(pool) => {
                sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
                    .bind(&hash)
                    .bind(&now)
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map(|_| ())
            }
            Db::Postgres(pool) => {
                sqlx::query("UPDATE users SET password_hash = $1, updated_at = $2 WHERE id = $3")
                    .bind(&hash)
                    .bind(&now)
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map(|_| ())
            }
        }
        .map_err(ApiError::internal)?;
    }
    if let Some(disabled) = body.disabled {
        if disabled && actor.id == id {
            return Err(ApiError::bad_request("cannot disable yourself"));
        }
        let disabled_at: Option<String> = disabled.then(|| now.clone());
        match &state.db {
            Db::Sqlite(pool) => {
                sqlx::query("UPDATE users SET disabled_at = ?, updated_at = ? WHERE id = ?")
                    .bind(&disabled_at)
                    .bind(&now)
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map(|_| ())
            }
            Db::Postgres(pool) => {
                sqlx::query("UPDATE users SET disabled_at = $1, updated_at = $2 WHERE id = $3")
                    .bind(&disabled_at)
                    .bind(&now)
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map(|_| ())
            }
        }
        .map_err(ApiError::internal)?;
    }
    write_audit(
        &state.db,
        Some(actor.id),
        "user.update",
        Some("user"),
        Some(&id.to_string()),
        None,
    )
    .await;
    fetch_user_response(&state.db, id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("user not found"))
        .map(Json)
}

async fn delete_user(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, ApiError> {
    require_admin(&actor)?;
    if actor.id == id {
        return Err(ApiError::bad_request("cannot delete yourself"));
    }
    let deleted = delete_user_row(&state.db, id)
        .await
        .map_err(ApiError::internal)?;
    if !deleted {
        return Err(ApiError::not_found("user not found"));
    }
    write_audit(
        &state.db,
        Some(actor.id),
        "user.delete",
        Some("user"),
        Some(&id.to_string()),
        None,
    )
    .await;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn fetch_user_response(db: &Db, id: Uuid) -> anyhow::Result<Option<UserResponse>> {
    let row: Option<(String, String, String, Option<String>)> = match db {
        Db::Sqlite(pool) => {
            sqlx::query_as("SELECT id, email, role, disabled_at FROM users WHERE id = ?")
                .bind(id.to_string())
                .fetch_optional(pool)
                .await?
        }
        Db::Postgres(pool) => {
            sqlx::query_as("SELECT id, email, role, disabled_at FROM users WHERE id = $1")
                .bind(id.to_string())
                .fetch_optional(pool)
                .await?
        }
    };
    Ok(match row {
        Some((id, email, role, disabled_at)) => Some(UserResponse {
            id,
            email,
            role: UserRole::parse(&role)?,
            disabled: disabled_at.is_some(),
        }),
        None => None,
    })
}

async fn fetch_users(
    db: &Db,
    cursor: Option<&str>,
    limit: i64,
) -> anyhow::Result<Vec<UserResponse>> {
    let rows: Vec<(String, String, String, Option<String>)> = match db {
        Db::Sqlite(pool) => {
            if let Some(cursor) = cursor {
                sqlx::query_as(
                    r#"
                    SELECT id, email, role, disabled_at FROM users
                    WHERE id > ? ORDER BY id ASC LIMIT ?
                    "#,
                )
                .bind(cursor)
                .bind(limit)
                .fetch_all(pool)
                .await?
            } else {
                sqlx::query_as(
                    r#"
                    SELECT id, email, role, disabled_at FROM users
                    ORDER BY id ASC LIMIT ?
                    "#,
                )
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
        }
        Db::Postgres(pool) => {
            if let Some(cursor) = cursor {
                sqlx::query_as(
                    r#"
                    SELECT id, email, role, disabled_at FROM users
                    WHERE id > $1 ORDER BY id ASC LIMIT $2
                    "#,
                )
                .bind(cursor)
                .bind(limit)
                .fetch_all(pool)
                .await?
            } else {
                sqlx::query_as(
                    r#"
                    SELECT id, email, role, disabled_at FROM users
                    ORDER BY id ASC LIMIT $1
                    "#,
                )
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
        }
    };
    rows.into_iter()
        .map(|(id, email, role, disabled_at)| {
            Ok(UserResponse {
                id,
                email,
                role: UserRole::parse(&role)?,
                disabled: disabled_at.is_some(),
            })
        })
        .collect()
}

async fn insert_user(
    db: &Db,
    id: Uuid,
    email: &str,
    password_hash: &str,
    role: &str,
    now: &str,
) -> anyhow::Result<()> {
    match db {
        Db::Sqlite(pool) => {
            sqlx::query(
                r#"
                INSERT INTO users (id, email, password_hash, role, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(id.to_string())
            .bind(email)
            .bind(password_hash)
            .bind(role)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }
        Db::Postgres(pool) => {
            sqlx::query(
                r#"
                INSERT INTO users (id, email, password_hash, role, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(id.to_string())
            .bind(email)
            .bind(password_hash)
            .bind(role)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

async fn delete_user_row(db: &Db, id: Uuid) -> anyhow::Result<bool> {
    let affected = match db {
        Db::Sqlite(pool) => sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id.to_string())
            .execute(pool)
            .await?
            .rows_affected(),
        Db::Postgres(pool) => sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id.to_string())
            .execute(pool)
            .await?
            .rows_affected(),
    };
    Ok(affected > 0)
}
