//! HTTP API routes.

pub mod audit;
pub mod auth;
pub mod devices;
pub mod error;
pub mod health;
pub mod middleware;
pub mod nodes;
pub mod openapi;
pub mod profiles;
pub mod users;

use crate::app::AppState;
use axum::middleware::from_fn_with_state;
use axum::Router;

/// Builds the full HTTP API router.
pub fn router(state: AppState) -> Router<AppState> {
    let protected = Router::new()
        .merge(users::router())
        .merge(devices::router())
        .merge(nodes::router())
        .merge(profiles::router())
        .merge(audit::router())
        .layer(from_fn_with_state(
            state.clone(),
            middleware::require_auth,
        ));

    Router::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(openapi::router())
        .merge(protected)
}
