//! Settings / installation info (read-only for now).

use crate::api::error::ApiError;
use crate::api::middleware::{require_admin, AuthUser};
use crate::app::AppState;
use axum::extract::State;
use axum::routing::get;
use axum::{Extension, Json, Router};
use serde::Serialize;

/// Builds settings routes.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/settings", get(get_settings))
}

#[derive(Debug, Serialize)]
struct SettingsResponse {
    version: &'static str,
    http_bind: String,
    grpc_bind: String,
    public_url: Option<String>,
    public_grpc: Option<String>,
    cookie_secure: bool,
    trust_forwarded_headers: bool,
    static_dir_configured: bool,
    note: &'static str,
}

async fn get_settings(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
) -> Result<Json<SettingsResponse>, ApiError> {
    require_admin(&actor)?;
    Ok(Json(SettingsResponse {
        version: env!("CARGO_PKG_VERSION"),
        http_bind: state.config.http_bind.clone(),
        grpc_bind: state.config.grpc_bind.clone(),
        public_url: state.config.public_url.clone(),
        public_grpc: state.config.public_grpc.clone(),
        cookie_secure: state.config.cookie_secure,
        trust_forwarded_headers: state.config.trust_forwarded_headers,
        static_dir_configured: state.config.static_dir.as_ref().is_some_and(|p| p.exists()),
        note: "Schreibende Einstellungen folgen in späteren Schritten.",
    }))
}
