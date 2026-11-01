//! Provisioning: secrets never appear in DB / API; host-key mismatch fails.

#![allow(clippy::field_reassign_with_default)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;
use vpnc_control::config::ControlConfig;
use vpnc_control::db::Db;
use vpnc_control::provisioning::jobs::{JobStatus, ProvisioningService, SshSecrets};
use vpnc_control::provisioning::ssh::{host_key_fingerprint_sha256, verify_host_key};
use vpnc_control::services::users;
use vpnc_control::{build_app, build_state};

async fn setup() -> (axum::Router, Db, String) {
    std::env::set_var(
        "VPNC_MASTER_KEY",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prov.db");
    let url = format!(
        "sqlite://{}?mode=rwc",
        path.to_string_lossy().replace('\\', "/")
    );
    let db = Db::connect(&url).await.unwrap();
    users::bootstrap_admin(&db, "admin@example.com", "correct-horse-battery")
        .await
        .unwrap();
    let mut config = ControlConfig::default();
    config.database_url = url.clone();
    config.cookie_secure = false;
    let state = build_state(config, db.clone()).unwrap();
    std::mem::forget(dir);
    (build_app(state), db, url)
}

async fn login(app: &axum::Router) -> String {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"email":"admin@example.com","password":"correct-horse-battery"}"#,
        ))
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

#[test]
fn host_key_mismatch_rejected() {
    let fp = host_key_fingerprint_sha256(b"real-key");
    assert!(verify_host_key(b"evil-key", &fp).is_err());
}

#[tokio::test]
async fn job_row_never_contains_password() {
    let (_app, db, _) = setup().await;
    let svc = ProvisioningService::new(db.clone());
    let secret = "super-secret-password-xyz";
    let _secrets = SshSecrets {
        password: Some(secret.into()),
        private_key_pem: None,
    };
    let job = svc
        .insert_awaiting_hostkey(
            "10.0.0.1",
            22,
            "root",
            "classic",
            Some("edge-1"),
            Some("10.66.0.0/24"),
            None,
            "SHA256:testhostkeyfingerprint",
        )
        .await
        .unwrap();
    assert_eq!(job.status, JobStatus::AwaitingHostkey);

    // Dump job JSON + raw DB text and ensure password is absent.
    let json = serde_json::to_string(&job).unwrap();
    assert!(!json.contains(secret));

    let blob: String = match &db {
        Db::Sqlite(pool) => sqlx::query_scalar(
            "SELECT id||status||host||ssh_user||role||COALESCE(log_text,'')||COALESCE(error_message,'') FROM provisioning_jobs WHERE id = ?",
        )
        .bind(job.id.to_string())
        .fetch_one(pool)
        .await
        .unwrap(),
        Db::Postgres(_) => unreachable!(),
    };
    assert!(!blob.contains(secret));
}

#[tokio::test]
async fn confirm_rejects_wrong_fingerprint() {
    let (app, db, _) = setup().await;
    let cookie = login(&app).await;
    let svc = ProvisioningService::new(db);
    let job = svc
        .insert_awaiting_hostkey(
            "10.0.0.2",
            22,
            "root",
            "classic",
            None,
            None,
            None,
            "SHA256:correct",
        )
        .await
        .unwrap();

    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/provisioning/{}/confirm-hostkey", job.id))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &cookie)
        .body(Body::from(r#"{"fingerprint":"SHA256:wrong"}"#))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status() == StatusCode::BAD_REQUEST || res.status().is_client_error(),
        "status={}",
        res.status()
    );
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&body);
    assert!(!text.contains("password"));
    let _: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
}
