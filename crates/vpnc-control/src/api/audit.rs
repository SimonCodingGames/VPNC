//! Audit log API.

use crate::api::error::ApiError;
use crate::api::middleware::{require_admin, AuthUser};
use crate::app::AppState;
use crate::db::Db;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};

/// Builds audit routes (admin-only).
pub fn router() -> Router<AppState> {
    Router::new().route("/api/audit", get(list_audit))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct AuditEvent {
    id: String,
    actor_user_id: Option<String>,
    action: String,
    resource_type: Option<String>,
    resource_id: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct ListResponse {
    items: Vec<AuditEvent>,
    next_cursor: Option<String>,
}

async fn list_audit(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, ApiError> {
    require_admin(&actor)?;
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let rows: Vec<(
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        String,
    )> = match &state.db {
        Db::Sqlite(pool) => {
            if let Some(cursor) = &q.cursor {
                sqlx::query_as(
                    r#"
                    SELECT id, actor_user_id, action, resource_type, resource_id, created_at
                    FROM audit_events
                    WHERE id < ? ORDER BY id DESC LIMIT ?
                    "#,
                )
                .bind(cursor)
                .bind(limit + 1)
                .fetch_all(pool)
                .await
            } else {
                sqlx::query_as(
                    r#"
                    SELECT id, actor_user_id, action, resource_type, resource_id, created_at
                    FROM audit_events
                    ORDER BY id DESC LIMIT ?
                    "#,
                )
                .bind(limit + 1)
                .fetch_all(pool)
                .await
            }
        }
        Db::Postgres(pool) => {
            if let Some(cursor) = &q.cursor {
                sqlx::query_as(
                    r#"
                    SELECT id, actor_user_id, action, resource_type, resource_id, created_at
                    FROM audit_events
                    WHERE id < $1 ORDER BY id DESC LIMIT $2
                    "#,
                )
                .bind(cursor)
                .bind(limit + 1)
                .fetch_all(pool)
                .await
            } else {
                sqlx::query_as(
                    r#"
                    SELECT id, actor_user_id, action, resource_type, resource_id, created_at
                    FROM audit_events
                    ORDER BY id DESC LIMIT $1
                    "#,
                )
                .bind(limit + 1)
                .fetch_all(pool)
                .await
            }
        }
    }
    .map_err(ApiError::internal)?;

    let mut items: Vec<AuditEvent> = rows
        .into_iter()
        .map(
            |(id, actor_user_id, action, resource_type, resource_id, created_at)| AuditEvent {
                id,
                actor_user_id,
                action,
                resource_type,
                resource_id,
                created_at,
            },
        )
        .collect();
    let mut next_cursor = None;
    if (items.len() as i64) > limit {
        items.truncate(limit as usize);
        next_cursor = items.last().map(|e| e.id.clone());
    }
    Ok(Json(ListResponse { items, next_cursor }))
}
