//! OpenAPI document endpoint.

use crate::app::AppState;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

/// Builds OpenAPI routes.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/openapi.json", get(openapi_json))
}

async fn openapi_json() -> Json<Value> {
    Json(json!({
        "openapi": "3.0.3",
        "info": {
            "title": "VPNC Control API",
            "version": "0.1.0"
        },
        "paths": {
            "/api/health": {
                "get": {
                    "summary": "Health check",
                    "responses": { "200": { "description": "OK" } }
                }
            },
            "/api/auth/login": {
                "post": {
                    "summary": "Login",
                    "responses": {
                        "200": { "description": "Authenticated" },
                        "401": { "description": "Invalid credentials" }
                    }
                }
            },
            "/api/auth/logout": {
                "post": {
                    "summary": "Logout",
                    "responses": { "204": { "description": "Logged out" } }
                }
            },
            "/api/auth/me": {
                "get": {
                    "summary": "Current user",
                    "responses": {
                        "200": { "description": "User" },
                        "401": { "description": "Unauthorized" }
                    }
                }
            },
            "/api/users": {
                "get": { "summary": "List users", "responses": { "200": { "description": "OK" } } },
                "post": { "summary": "Create user", "responses": { "201": { "description": "Created" } } }
            },
            "/api/devices": {
                "get": { "summary": "List devices", "responses": { "200": { "description": "OK" } } },
                "post": { "summary": "Create device", "responses": { "201": { "description": "Created" } } }
            },
            "/api/nodes": {
                "get": { "summary": "List nodes", "responses": { "200": { "description": "OK" } } },
                "post": { "summary": "Create node", "responses": { "201": { "description": "Created" } } }
            },
            "/api/nodes/join-tokens": {
                "post": { "summary": "Create node join token", "responses": { "200": { "description": "Token" } } }
            },
            "/api/audit": {
                "get": { "summary": "List audit events", "responses": { "200": { "description": "OK" } } }
            }
        }
    }))
}
