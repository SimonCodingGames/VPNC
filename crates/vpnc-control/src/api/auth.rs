//! Authentication endpoints.

use crate::api::error::ApiError;
use crate::app::AppState;
use crate::services::auth::SESSION_COOKIE;
use crate::services::users::{UserRecord, UserRole};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};

/// Builds auth routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct MeResponse {
    id: String,
    email: String,
    role: UserRole,
}

impl From<UserRecord> for MeResponse {
    fn from(user: UserRecord) -> Self {
        Self {
            id: user.id.to_string(),
            email: user.email,
            role: user.role,
        }
    }
}

async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let (token, user) = state
        .auth
        .login(&body.email, &body.password)
        .await
        .map_err(|_| ApiError::unauthorized("invalid credentials"))?;

    let cookie = state.auth.session_cookie(&token);
    let jar = jar.add(cookie);
    Ok((jar, Json(MeResponse::from(user))))
}

async fn logout(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        let _ = state.auth.logout(cookie.value()).await;
    }
    let jar = jar.add(state.auth.clear_session_cookie());
    (jar, StatusCode::NO_CONTENT)
}

async fn me(State(state): State<AppState>, jar: CookieJar) -> Result<Json<MeResponse>, ApiError> {
    let Some(cookie) = jar.get(SESSION_COOKIE) else {
        return Err(ApiError::unauthorized("not authenticated"));
    };
    let user = state
        .auth
        .user_from_token(cookie.value())
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::unauthorized("not authenticated"))?;
    Ok(Json(MeResponse::from(user)))
}
