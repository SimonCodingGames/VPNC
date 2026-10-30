//! NodeService implementation.

use crate::db::Db;
use crate::grpc::pb::node::node_service_server::NodeService;
use crate::grpc::pb::node::{
    AckConfigRequest, AckConfigResponse, ConfigUpdate, HealthAck, HealthReport,
    PeerLocationAck, PeerLocationReport, RegisterRequest, RegisterResponse, StreamConfigRequest,
};
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// In-memory health / config version tracking (also persisted partially in DB).
#[derive(Default, Clone)]
pub struct NodeRuntimeState {
    inner: Arc<Mutex<HashMap<String, NodeLive>>>,
}

#[derive(Clone, Debug)]
struct NodeLive {
    last_seen: chrono::DateTime<Utc>,
    config_version: u64,
    applied_version: u64,
}

impl NodeRuntimeState {
    /// Marks a node offline if last_seen older than `timeout`.
    pub fn reap_offline(&self, timeout: Duration) -> Vec<String> {
        let now = Utc::now();
        let guard = self.inner.lock().expect("node runtime");
        let mut offline = Vec::new();
        for (id, live) in guard.iter() {
            if now - live.last_seen > timeout {
                offline.push(id.clone());
            }
        }
        offline
    }

    fn touch(&self, node_id: &str) {
        let mut guard = self.inner.lock().expect("node runtime");
        let entry = guard.entry(node_id.to_string()).or_insert(NodeLive {
            last_seen: Utc::now(),
            config_version: 1,
            applied_version: 0,
        });
        entry.last_seen = Utc::now();
    }

    fn set_applied(&self, node_id: &str, version: u64) {
        let mut guard = self.inner.lock().expect("node runtime");
        if let Some(entry) = guard.get_mut(node_id) {
            entry.applied_version = version;
            entry.last_seen = Utc::now();
        }
    }

    fn current_version(&self, node_id: &str) -> u64 {
        let mut guard = self.inner.lock().expect("node runtime");
        let entry = guard.entry(node_id.to_string()).or_insert(NodeLive {
            last_seen: Utc::now(),
            config_version: 1,
            applied_version: 0,
        });
        entry.config_version
    }
}

/// gRPC NodeService backed by the control database.
pub struct NodeGrpcService {
    db: Db,
    runtime: NodeRuntimeState,
}

impl NodeGrpcService {
    /// Creates the service.
    pub fn new(db: Db, runtime: NodeRuntimeState) -> Self {
        Self { db, runtime }
    }
}

#[tonic::async_trait]
impl NodeService for NodeGrpcService {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        if req.join_token.is_empty() {
            return Err(Status::invalid_argument("join_token required"));
        }
        let token_hash = hex::encode(Sha256::digest(req.join_token.as_bytes()));
        let now = Utc::now().to_rfc3339();

        let row: Option<(String, String, Option<String>, String)> = match &self.db {
            Db::Sqlite(pool) => sqlx::query_as(
                r#"
                SELECT id, role, node_name, expires_at FROM node_join_tokens
                WHERE token_hash = ? AND used_at IS NULL
                "#,
            )
            .bind(&token_hash)
            .fetch_optional(pool)
            .await
            .map_err(|e| Status::internal(e.to_string()))?,
            Db::Postgres(pool) => sqlx::query_as(
                r#"
                SELECT id, role, node_name, expires_at FROM node_join_tokens
                WHERE token_hash = $1 AND used_at IS NULL
                "#,
            )
            .bind(&token_hash)
            .fetch_optional(pool)
            .await
            .map_err(|e| Status::internal(e.to_string()))?,
        };

        let Some((token_id, role, token_name, expires_at)) = row else {
            return Err(Status::unauthenticated("invalid join token"));
        };
        let expires = chrono::DateTime::parse_from_rfc3339(&expires_at)
            .map_err(|_| Status::internal("bad expires_at"))?
            .with_timezone(&Utc);
        if expires < Utc::now() {
            return Err(Status::unauthenticated("join token expired"));
        }

        let node_id = Uuid::new_v4();
        let name = if req.node_name.trim().is_empty() {
            token_name.unwrap_or_else(|| format!("node-{node_id}"))
        } else {
            req.node_name.clone()
        };
        let role = if req.role.is_empty() { role } else { req.role };

        // TODO(step-6 follow-up): issue mTLS client cert signed by control CA.
        // For now, return a durable opaque credential (hashed at rest later).
        let credential = hex::encode(Sha256::digest(format!("{node_id}:{}", req.join_token).as_bytes()));

        match &self.db {
            Db::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO nodes (id, name, role, status, created_at, updated_at, last_seen_at)
                    VALUES (?, ?, ?, 'online', ?, ?, ?)
                    "#,
                )
                .bind(node_id.to_string())
                .bind(&name)
                .bind(&role)
                .bind(&now)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
                sqlx::query("UPDATE node_join_tokens SET used_at = ? WHERE id = ?")
                    .bind(&now)
                    .bind(&token_id)
                    .execute(pool)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
            }
            Db::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO nodes (id, name, role, status, created_at, updated_at, last_seen_at)
                    VALUES ($1, $2, $3, 'online', $4, $5, $6)
                    "#,
                )
                .bind(node_id.to_string())
                .bind(&name)
                .bind(&role)
                .bind(&now)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await
                .map_err(|e| Status::internal(e.to_string()))?;
                sqlx::query("UPDATE node_join_tokens SET used_at = $1 WHERE id = $2")
                    .bind(&now)
                    .bind(&token_id)
                    .execute(pool)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
            }
        }

        self.runtime.touch(&node_id.to_string());
        Ok(Response::new(RegisterResponse {
            node_id: node_id.to_string(),
            credential,
            initial_config_version: 1,
        }))
    }

    type StreamConfigStream =
        Pin<Box<dyn Stream<Item = Result<ConfigUpdate, Status>> + Send + 'static>>;

    async fn stream_config(
        &self,
        request: Request<StreamConfigRequest>,
    ) -> Result<Response<Self::StreamConfigStream>, Status> {
        let req = request.into_inner();
        self.runtime.touch(&req.node_id);
        let version = self.runtime.current_version(&req.node_id);
        let (tx, rx) = mpsc::channel(4);
        let full = req.last_applied_version == 0 || req.last_applied_version < version;
        let update = ConfigUpdate {
            version,
            full_snapshot: full,
            payload: br#"{"role":"classic","peers":[]}"#.to_vec(),
        };
        tx.send(Ok(update))
            .await
            .map_err(|_| Status::internal("config stream closed"))?;
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn report_health(
        &self,
        request: Request<HealthReport>,
    ) -> Result<Response<HealthAck>, Status> {
        let req = request.into_inner();
        self.runtime.touch(&req.node_id);
        let now = Utc::now().to_rfc3339();
        match &self.db {
            Db::Sqlite(pool) => sqlx::query(
                "UPDATE nodes SET last_seen_at = ?, status = 'online', updated_at = ? WHERE id = ?",
            )
            .bind(&now)
            .bind(&now)
            .bind(&req.node_id)
            .execute(pool)
            .await
            .map(|_| ()),
            Db::Postgres(pool) => sqlx::query(
                "UPDATE nodes SET last_seen_at = $1, status = 'online', updated_at = $2 WHERE id = $3",
            )
            .bind(&now)
            .bind(&now)
            .bind(&req.node_id)
            .execute(pool)
            .await
            .map(|_| ()),
        }
        .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(HealthAck {}))
    }

    async fn report_peer_location(
        &self,
        _request: Request<PeerLocationReport>,
    ) -> Result<Response<PeerLocationAck>, Status> {
        // Used from step 13 onwards.
        Ok(Response::new(PeerLocationAck {}))
    }

    async fn ack_config(
        &self,
        request: Request<AckConfigRequest>,
    ) -> Result<Response<AckConfigResponse>, Status> {
        let req = request.into_inner();
        self.runtime.set_applied(&req.node_id, req.applied_version);
        Ok(Response::new(AckConfigResponse {}))
    }
}

/// Marks nodes offline after timeout based on runtime + DB.
pub async fn mark_offline_nodes(db: &Db, runtime: &NodeRuntimeState, timeout_secs: i64) {
    let offline = runtime.reap_offline(Duration::seconds(timeout_secs));
    let now = Utc::now().to_rfc3339();
    for id in offline {
        let _ = match db {
            Db::Sqlite(pool) => sqlx::query(
                "UPDATE nodes SET status = 'offline', updated_at = ? WHERE id = ?",
            )
            .bind(&now)
            .bind(&id)
            .execute(pool)
            .await
            .map(|_| ()),
            Db::Postgres(pool) => sqlx::query(
                "UPDATE nodes SET status = 'offline', updated_at = $1 WHERE id = $2",
            )
            .bind(&now)
            .bind(&id)
            .execute(pool)
            .await
            .map(|_| ()),
        };
    }
}
