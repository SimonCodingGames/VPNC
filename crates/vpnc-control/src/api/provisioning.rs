//! Provisioning API (admin-only): SSH install jobs + SSE logs.

use crate::api::error::ApiError;
use crate::api::middleware::{require_admin, write_audit, AuthUser};
use crate::app::AppState;
use crate::provisioning::jobs::{JobStatus, ProvisioningJob, SshSecrets};
use crate::provisioning::ssh::probe_host;
use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

/// Builds provisioning routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/provisioning", get(list_jobs).post(create_job))
        .route(
            "/api/provisioning/{id}/confirm-hostkey",
            post(confirm_hostkey),
        )
        .route("/api/provisioning/{id}/cancel", post(cancel_job))
        .route("/api/provisioning/{id}/events", get(job_events))
        .route("/api/provisioning/{id}", get(get_job))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CreateJobRequest {
    host: String,
    #[serde(default = "default_port")]
    ssh_port: u16,
    ssh_user: String,
    password: Option<String>,
    private_key_pem: Option<String>,
    role: String,
    node_name: Option<String>,
    tunnel_cidr_v4: Option<String>,
    tunnel_cidr_v6: Option<String>,
}

fn default_port() -> u16 {
    22
}

#[derive(Debug, Serialize)]
struct ListResponse {
    items: Vec<JobResponse>,
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
struct JobResponse {
    id: String,
    status: JobStatus,
    host: String,
    ssh_port: u16,
    ssh_user: String,
    role: String,
    node_name: Option<String>,
    tunnel_cidr_v4: Option<String>,
    tunnel_cidr_v6: Option<String>,
    host_key_fingerprint: Option<String>,
    error_message: Option<String>,
    log_text: String,
}

impl From<ProvisioningJob> for JobResponse {
    fn from(j: ProvisioningJob) -> Self {
        Self {
            id: j.id.to_string(),
            status: j.status,
            host: j.host,
            ssh_port: j.ssh_port,
            ssh_user: j.ssh_user,
            role: j.role,
            node_name: j.node_name,
            tunnel_cidr_v4: j.tunnel_cidr_v4,
            tunnel_cidr_v6: j.tunnel_cidr_v6,
            host_key_fingerprint: j.host_key_fingerprint,
            error_message: j.error_message,
            log_text: j.log_text,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ConfirmRequest {
    fingerprint: String,
}

async fn list_jobs(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, ApiError> {
    require_admin(&actor)?;
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let mut items = state
        .provisioning
        .list(q.cursor.as_deref(), limit + 1)
        .await
        .map_err(ApiError::internal)?;
    let mut next_cursor = None;
    if (items.len() as i64) > limit {
        items.truncate(limit as usize);
        next_cursor = items.last().map(|j| j.id.to_string());
    }
    Ok(Json(ListResponse {
        items: items.into_iter().map(JobResponse::from).collect(),
        next_cursor,
    }))
}

async fn get_job(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<JobResponse>, ApiError> {
    require_admin(&actor)?;
    let job = state
        .provisioning
        .get(id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("job not found"))?;
    Ok(Json(JobResponse::from(job)))
}

async fn create_job(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Json(mut body): Json<CreateJobRequest>,
) -> Result<(axum::http::StatusCode, Json<JobResponse>), ApiError> {
    require_admin(&actor)?;
    let host = body.host.trim().to_string();
    let user = body.ssh_user.trim().to_string();
    if host.is_empty() || user.is_empty() {
        return Err(ApiError::bad_request("host and ssh_user required"));
    }
    if body.password.as_ref().is_none_or(|p| p.is_empty())
        && body.private_key_pem.as_ref().is_none_or(|k| k.is_empty())
    {
        return Err(ApiError::bad_request(
            "password or private_key_pem required",
        ));
    }
    let role = body.role.trim().to_string();
    if !matches!(role.as_str(), "classic" | "ingress" | "egress") {
        return Err(ApiError::bad_request("invalid role"));
    }

    let secrets = SshSecrets {
        password: body.password.take().filter(|s| !s.is_empty()),
        private_key_pem: body.private_key_pem.take().filter(|s| !s.is_empty()),
    };
    // Clear request copies (already moved).
    body.password = None;
    body.private_key_pem = None;

    let fingerprint = probe_host(&host, body.ssh_port, &user, &secrets)
        .await
        .map_err(|e| ApiError::bad_request(format!("SSH probe failed: {e}")))?;

    let job = state
        .provisioning
        .insert_awaiting_hostkey(
            &host,
            body.ssh_port,
            &user,
            &role,
            body.node_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty()),
            body.tunnel_cidr_v4
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty()),
            body.tunnel_cidr_v6
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty()),
            &fingerprint,
        )
        .await
        .map_err(ApiError::internal)?;

    state.secret_store.insert(job.id, secrets);

    write_audit(
        &state.db,
        Some(actor.id),
        "provisioning.create",
        Some("provisioning_job"),
        Some(&job.id.to_string()),
        None,
    )
    .await;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(JobResponse::from(job)),
    ))
}

async fn confirm_hostkey(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<ConfirmRequest>,
) -> Result<Json<JobResponse>, ApiError> {
    require_admin(&actor)?;
    state
        .provisioning
        .confirm_hostkey(id, body.fingerprint.trim())
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    write_audit(
        &state.db,
        Some(actor.id),
        "provisioning.confirm_hostkey",
        Some("provisioning_job"),
        Some(&id.to_string()),
        None,
    )
    .await;
    let job = state
        .provisioning
        .get(id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("job not found"))?;
    Ok(Json(JobResponse::from(job)))
}

async fn cancel_job(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<JobResponse>, ApiError> {
    require_admin(&actor)?;
    state
        .provisioning
        .cancel(id)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    state.secret_store.clear(id);
    write_audit(
        &state.db,
        Some(actor.id),
        "provisioning.cancel",
        Some("provisioning_job"),
        Some(&id.to_string()),
        None,
    )
    .await;
    let job = state
        .provisioning
        .get(id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("job not found"))?;
    Ok(Json(JobResponse::from(job)))
}

async fn job_events(
    State(state): State<AppState>,
    Extension(AuthUser(actor)): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    if require_admin(&actor).is_err() {
        return Sse::new(ReceiverStream::new(
            tokio::sync::mpsc::channel::<Result<Event, Infallible>>(1).1,
        ))
        .into_response();
    }
    if state.provisioning.get(id).await.ok().flatten().is_none() {
        return Sse::new(ReceiverStream::new(
            tokio::sync::mpsc::channel::<Result<Event, Infallible>>(1).1,
        ))
        .into_response();
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(16);
    let provisioning = state.provisioning.clone();
    tokio::spawn(async move {
        let mut last_len = 0usize;
        loop {
            match provisioning.get(id).await {
                Ok(Some(job)) => {
                    if job.log_text.len() > last_len {
                        let chunk = job.log_text[last_len..].to_string();
                        last_len = job.log_text.len();
                        if tx
                            .send(Ok(Event::default().event("log").data(chunk)))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    let status = serde_json::json!({
                        "status": job.status,
                        "error_message": job.error_message,
                    });
                    if tx
                        .send(Ok(Event::default()
                            .event("status")
                            .data(status.to_string())))
                        .await
                        .is_err()
                    {
                        break;
                    }
                    if matches!(
                        job.status,
                        JobStatus::Succeeded | JobStatus::Failed | JobStatus::Cancelled
                    ) {
                        break;
                    }
                }
                _ => break,
            }
            tokio::time::sleep(Duration::from_millis(800)).await;
        }
    });

    Sse::new(ReceiverStream::new(rx)).into_response()
}
