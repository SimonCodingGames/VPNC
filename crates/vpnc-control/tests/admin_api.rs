//! Step 4: permissions, audit, openapi.

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;
use vpnc_control::config::ControlConfig;
use vpnc_control::db::Db;
use vpnc_control::services::users::{self, UserRole};
use vpnc_control::{build_app, build_state};

async fn setup() -> (axum::Router, String) {
    std::env::set_var(
        "VPNC_MASTER_KEY",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("admin.db");
    // leak tempdir for test duration via into_path
    let path = path.to_string_lossy().replace('\\', "/");
    let url = format!("sqlite://{path}?mode=rwc");
    let db = Db::connect(&url).await.unwrap();
    let admin = users::bootstrap_admin(&db, "admin@example.com", "correct-horse-battery")
        .await
        .unwrap();
    // create second user directly
    let uid = uuid::Uuid::new_v4();
    let now = chrono::Utc::now().to_rfc3339();
    let hash = users::hash_password("user-password-ok").unwrap();
    match &db {
        Db::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO users (id, email, password_hash, role, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(uid.to_string())
            .bind("user@example.com")
            .bind(hash)
            .bind(UserRole::User.as_str())
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await
            .unwrap();
        }
        Db::Postgres(_) => unreachable!(),
    }
    let _ = admin;
    let mut config = ControlConfig::default();
    config.database_url = url.clone();
    let state = build_state(config, db).unwrap();
    // Keep db file alive: store path in unused
    std::mem::forget(dir);
    (build_app(state), url)
}

async fn login(app: &axum::Router, email: &str, password: &str) -> String {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            r#"{{"email":"{email}","password":"{password}"}}"#
        )))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    res.headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn user_cannot_see_foreign_devices_and_audit_is_written() {
    let (app, _) = setup().await;
    let admin_cookie = login(&app, "admin@example.com", "correct-horse-battery").await;
    let user_cookie = login(&app, "user@example.com", "user-password-ok").await;

    // Admin creates a device for themselves
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/devices")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &admin_cookie)
        .body(Body::from(r#"{"name":"admin-laptop"}"#))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // User lists devices — must not see admin's device
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/devices")
        .header(header::COOKIE, &user_cookie)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["items"].as_array().unwrap().is_empty());

    // User creates own device
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/devices")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &user_cookie)
        .body(Body::from(r#"{"name":"phone"}"#))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // Audit contains device.create
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/audit")
        .header(header::COOKIE, &admin_cookie)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let actions: Vec<&str> = json["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["action"].as_str().unwrap())
        .collect();
    assert!(actions.iter().any(|a| *a == "device.create"));

    // OpenAPI
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/openapi.json")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["openapi"], "3.0.3");
    assert!(json["paths"].is_object());
}
