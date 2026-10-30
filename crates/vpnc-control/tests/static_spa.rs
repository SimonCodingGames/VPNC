//! Static dashboard serving and API catch-all behaviour.

#![allow(clippy::field_reassign_with_default)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use std::fs;
use tower::ServiceExt;
use vpnc_control::config::ControlConfig;
use vpnc_control::db::Db;
use vpnc_control::{build_app, build_state};

async fn setup_with_static() -> axum::Router {
    std::env::set_var(
        "VPNC_MASTER_KEY",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    let dir = tempfile::tempdir().unwrap();
    let static_dir = dir.path().join("static");
    fs::create_dir_all(&static_dir).unwrap();
    fs::write(
        static_dir.join("index.html"),
        "<!doctype html><html><body>home</body></html>",
    )
    .unwrap();
    fs::write(
        static_dir.join("200.html"),
        "<!doctype html><html><body>spa</body></html>",
    )
    .unwrap();
    fs::create_dir_all(static_dir.join("_app/immutable")).unwrap();
    fs::write(static_dir.join("_app/immutable/x.js"), "export {};").unwrap();

    let db_path = dir.path().join("spa.db");
    let url = format!(
        "sqlite://{}?mode=rwc",
        db_path.to_string_lossy().replace('\\', "/")
    );
    let db = Db::connect(&url).await.unwrap();
    let mut config = ControlConfig::default();
    config.database_url = url;
    config.cookie_secure = false;
    config.static_dir = Some(static_dir);
    let state = build_state(config, db).unwrap();
    std::mem::forget(dir);
    build_app(state)
}

#[tokio::test]
async fn root_serves_html() {
    let app = setup_with_static().await;
    let res = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ctype = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ctype.contains("text/html"), "ctype={ctype}");
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("home") || text.contains("spa"), "{text}");
}

#[tokio::test]
async fn spa_route_serves_html() {
    let app = setup_with_static().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("spa") || text.contains("html"), "{text}");
}

#[tokio::test]
async fn api_unknown_is_json_404() {
    let app = setup_with_static().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/unknown")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let ctype = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ctype.contains("json"), "ctype={ctype}");
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("not found"), "{text}");
}

#[tokio::test]
async fn api_health_ok() {
    let app = setup_with_static().await;
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn immutable_assets_get_long_cache() {
    let app = setup_with_static().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/_app/immutable/x.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cache = res
        .headers()
        .get(header::CACHE_CONTROL)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(cache.contains("immutable"), "cache={cache}");
}
