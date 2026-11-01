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
                "get": { "summary": "List users (admin)", "responses": { "200": { "description": "OK" } } },
                "post": { "summary": "Create user (admin)", "responses": { "201": { "description": "Created" } } }
            },
            "/api/users/{id}": {
                "patch": { "summary": "Update user role/password/disabled (admin)", "responses": { "200": { "description": "OK" } } },
                "delete": { "summary": "Delete user (admin)", "responses": { "204": { "description": "Deleted" } } }
            },
            "/api/devices": {
                "get": { "summary": "List devices", "responses": { "200": { "description": "OK" } } },
                "post": { "summary": "Create device", "responses": { "201": { "description": "Created" } } }
            },
            "/api/devices/{id}": {
                "patch": { "summary": "Rename device", "responses": { "200": { "description": "OK" } } },
                "delete": { "summary": "Delete device", "responses": { "204": { "description": "Deleted" } } }
            },
            "/api/nodes": {
                "get": { "summary": "List nodes", "responses": { "200": { "description": "OK" } } },
                "post": { "summary": "Create node", "responses": { "201": { "description": "Created" } } }
            },
            "/api/nodes/{id}": {
                "delete": { "summary": "Delete node", "responses": { "204": { "description": "Deleted" } } }
            },
            "/api/nodes/join-tokens": {
                "post": {
                    "summary": "Create node join token (once-visible)",
                    "responses": { "200": { "description": "Token + install commands" } }
                }
            },
            "/api/profiles": {
                "get": { "summary": "List profiles", "responses": { "200": { "description": "OK" } } },
                "post": { "summary": "Create profile (private key once)", "responses": { "201": { "description": "Created" } } }
            },
            "/api/profiles/{id}": {
                "delete": { "summary": "Revoke/delete profile", "responses": { "204": { "description": "Deleted" } } }
            },
            "/api/profiles/{id}/download": {
                "get": { "summary": "One-time WireGuard config download", "responses": { "200": { "description": "conf + QR" } } }
            },
            "/api/audit": {
                "get": { "summary": "List audit events (admin)", "responses": { "200": { "description": "OK" } } }
            },
            "/api/settings": {
                "get": { "summary": "Read-only settings (admin)", "responses": { "200": { "description": "OK" } } }
            },
            "/api/provisioning": {
                "get": { "summary": "List provisioning jobs (admin)", "responses": { "200": { "description": "OK" } } },
                "post": { "summary": "Start SSH provisioning (admin)", "responses": { "201": { "description": "Created" } } }
            },
            "/api/provisioning/{id}/confirm-hostkey": {
                "post": { "summary": "Confirm SSH host key", "responses": { "200": { "description": "OK" } } }
            },
            "/api/provisioning/{id}/events": {
                "get": { "summary": "SSE live log", "responses": { "200": { "description": "SSE" } } }
            }
        }
    }))
}
