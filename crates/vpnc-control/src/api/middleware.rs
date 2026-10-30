//! Authn/authz and audit middleware helpers.

use crate::api::error::ApiError;
use crate::app::AppState;
use crate::db::Db;
use crate::services::auth::SESSION_COOKIE;
use crate::services::users::{UserRecord, UserRole};
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::extract::cookie::CookieJar;
use chrono::Utc;
use uuid::Uuid;

/// Request extension holding the authenticated user.
#[derive(Clone, Debug)]
pub struct AuthUser(pub UserRecord);

/// Requires a valid session cookie and inserts [`AuthUser`].
pub async fn require_auth(
    State(state): State<AppState>,
    jar: CookieJar,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let Some(cookie) = jar.get(SESSION_COOKIE) else {
        return Err(ApiError::unauthorized("not authenticated"));
    };
    let user = state
        .auth
        .user_from_token(cookie.value())
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::unauthorized("not authenticated"))?;
    request.extensions_mut().insert(AuthUser(user));
    Ok(next.run(request).await)
}

/// Requires the authenticated user to be an admin.
pub fn require_admin(user: &UserRecord) -> Result<(), ApiError> {
    if user.role != UserRole::Admin {
        return Err(ApiError::forbidden("admin role required"));
    }
    Ok(())
}

/// Writes an audit event (best-effort; errors are logged).
pub async fn write_audit(
    db: &Db,
    actor: Option<Uuid>,
    action: &str,
    resource_type: Option<&str>,
    resource_id: Option<&str>,
    metadata_json: Option<&str>,
) {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let result = match db {
        Db::Sqlite(pool) => sqlx::query(
            r#"
                INSERT INTO audit_events
                    (id, actor_user_id, action, resource_type, resource_id, metadata_json, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
        )
        .bind(id.to_string())
        .bind(actor.map(|u| u.to_string()))
        .bind(action)
        .bind(resource_type)
        .bind(resource_id)
        .bind(metadata_json)
        .bind(now)
        .execute(pool)
        .await
        .map(|_| ()),
        Db::Postgres(pool) => sqlx::query(
            r#"
                INSERT INTO audit_events
                    (id, actor_user_id, action, resource_type, resource_id, metadata_json, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
        )
        .bind(id.to_string())
        .bind(actor.map(|u| u.to_string()))
        .bind(action)
        .bind(resource_type)
        .bind(resource_id)
        .bind(metadata_json)
        .bind(now)
        .execute(pool)
        .await
        .map(|_| ()),
    };
    if let Err(err) = result {
        tracing::error!(error = %err, "failed to write audit event");
    }
}
