//! gRPC register + config stream + offline timeout.

use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use std::time::Duration as StdDuration;
use tonic::transport::Server;
use uuid::Uuid;
use vpnc_control::db::Db;
use vpnc_control::grpc::node::mark_offline_nodes;
use vpnc_control::grpc::pb::node::node_service_client::NodeServiceClient;
use vpnc_control::grpc::pb::node::node_service_server::NodeServiceServer;
use vpnc_control::grpc::pb::node::{RegisterRequest, StreamConfigRequest};
use vpnc_control::grpc::{NodeGrpcService, NodeRuntimeState};

#[tokio::test]
async fn register_stream_and_offline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("grpc.db");
    let url = format!("sqlite://{}?mode=rwc", path.display()).replace('\\', "/");
    let db = Db::connect(&url).await.unwrap();

    let raw = "test-join-token-please";
    let hash = hex::encode(Sha256::digest(raw.as_bytes()));
    let id = Uuid::new_v4();
    let now = Utc::now();
    match &db {
        Db::Sqlite(pool) => {
            sqlx::query(
                r#"
                INSERT INTO node_join_tokens (id, token_hash, node_name, role, expires_at, created_at)
                VALUES (?, ?, 'n1', 'classic', ?, ?)
                "#,
            )
            .bind(id.to_string())
            .bind(hash)
            .bind((now + Duration::hours(1)).to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(pool)
            .await
            .unwrap();
        }
        _ => unreachable!(),
    }

    let runtime = NodeRuntimeState::default();
    let svc = NodeGrpcService::new(db.clone(), runtime.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    tokio::spawn(async move {
        Server::builder()
            .add_service(NodeServiceServer::new(svc))
            .serve(addr)
            .await
            .unwrap();
    });
    tokio::time::sleep(StdDuration::from_millis(150)).await;

    let mut client = NodeServiceClient::connect(format!("http://{addr}"))
        .await
        .unwrap();
    let reg = client
        .register(RegisterRequest {
            join_token: raw.into(),
            node_name: "node-a".into(),
            role: "classic".into(),
            public_key_pem: String::new(),
        })
        .await
        .unwrap()
        .into_inner();
    assert!(!reg.node_id.is_empty());

    let mut stream = client
        .stream_config(StreamConfigRequest {
            node_id: reg.node_id.clone(),
            last_applied_version: 0,
        })
        .await
        .unwrap()
        .into_inner();
    let update = stream.message().await.unwrap().unwrap();
    assert_eq!(update.version, 1);
    assert!(update.full_snapshot);

    tokio::time::sleep(StdDuration::from_millis(20)).await;
    mark_offline_nodes(&db, &runtime, 0).await;

    let status: String = match &db {
        Db::Sqlite(pool) => sqlx::query_scalar("SELECT status FROM nodes WHERE id = ?")
            .bind(&reg.node_id)
            .fetch_one(pool)
            .await
            .unwrap(),
        _ => unreachable!(),
    };
    assert_eq!(status, "offline");
    std::mem::forget(dir);
}
