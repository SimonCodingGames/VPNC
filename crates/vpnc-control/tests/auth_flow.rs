//! Integration test: bootstrap admin, login, /me, logout (SQLite + optional Postgres).

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;
use vpnc_control::config::ControlConfig;
use vpnc_control::db::Db;
use vpnc_control::services::users;
use vpnc_control::{build_app, build_state};

async fn run_auth_flow(database_url: &str) {
    std::env::set_var(
        "VPNC_MASTER_KEY",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );

    let db = Db::connect(database_url)
        .await
        .expect("connect db");
    let user = users::bootstrap_admin(&db, "admin@example.com", "correct-horse-battery")
        .await
        .expect("bootstrap");
    assert_eq!(user.email, "admin@example.com");

    let mut config = ControlConfig::default();
    config.database_url = database_url.to_string();
    config.cookie_secure = false;
    let state = build_state(config, db).expect("state");
    let app = build_app(state);

    // Login
    let login_req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"email":"admin@example.com","password":"correct-horse-battery"}"#,
        ))
        .unwrap();
    let login_res = app.clone().oneshot(login_req).await.unwrap();
    assert_eq!(login_res.status(), StatusCode::OK);
    let set_cookie = login_res
        .headers()
        .get(header::SET_COOKIE)
        .expect("set-cookie")
        .to_str()
        .unwrap()
        .to_string();
    assert!(set_cookie.contains("vpnc_session="));
    assert!(set_cookie.to_ascii_lowercase().contains("httponly"));
    assert!(set_cookie.to_ascii_lowercase().contains("samesite=strict"));

    let body = login_res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["email"], "admin@example.com");
    assert_eq!(json["role"], "admin");

    let session_cookie = set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string();

    // /me
    let me_req = Request::builder()
        .method(Method::GET)
        .uri("/api/auth/me")
        .header(header::COOKIE, &session_cookie)
        .body(Body::empty())
        .unwrap();
    let me_res = app.clone().oneshot(me_req).await.unwrap();
    assert_eq!(me_res.status(), StatusCode::OK);
    let me_body = me_res.into_body().collect().await.unwrap().to_bytes();
    let me_json: Value = serde_json::from_slice(&me_body).unwrap();
    assert_eq!(me_json["email"], "admin@example.com");

    // Logout
    let logout_req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/logout")
        .header(header::COOKIE, &session_cookie)
        .body(Body::empty())
        .unwrap();
    let logout_res = app.clone().oneshot(logout_req).await.unwrap();
    assert_eq!(logout_res.status(), StatusCode::NO_CONTENT);

    // /me after logout
    let me2_req = Request::builder()
        .method(Method::GET)
        .uri("/api/auth/me")
        .header(header::COOKIE, &session_cookie)
        .body(Body::empty())
        .unwrap();
    let me2_res = app.oneshot(me2_req).await.unwrap();
    assert_eq!(me2_res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn auth_flow_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let url = format!("sqlite://{}?mode=rwc", path.display());
    // sqlx on Windows wants forward slashes sometimes; normalize:
    let url = url.replace('\\', "/");
    run_auth_flow(&url).await;
}

#[tokio::test]
async fn auth_flow_postgres() {
    let Ok(url) = std::env::var("VPNC_TEST_DATABASE_URL") else {
        eprintln!("skipping postgres test: VPNC_TEST_DATABASE_URL not set");
        return;
    };
    if !(url.starts_with("postgres://") || url.starts_with("postgresql://")) {
        eprintln!("skipping postgres test: URL is not postgres");
        return;
    }
    run_auth_flow(&url).await;
}
