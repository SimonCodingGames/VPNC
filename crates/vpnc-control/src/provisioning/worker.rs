//! Background worker that runs confirmed provisioning jobs.

use super::detect::DetectedPublicIps;
use super::install::{install_node, wait_until, ArcLog, InstallParams};
use super::jobs::{JobStatus, ProvisioningService};
use super::secrets::SecretStore;
use crate::db::Db;
use chrono::{Duration as ChronoDuration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;

/// Spawns the provisioning worker loop.
pub fn spawn_worker(
    svc: ProvisioningService,
    secrets: SecretStore,
    binaries_dir: PathBuf,
    public_grpc: String,
) {
    tokio::spawn(async move {
        loop {
            if let Err(err) = tick(&svc, &secrets, &binaries_dir, &public_grpc).await {
                error!(error = %err, "provisioning worker tick failed");
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}

async fn tick(
    svc: &ProvisioningService,
    secrets: &SecretStore,
    binaries_dir: &PathBuf,
    public_grpc: &str,
) -> anyhow::Result<()> {
    let Some(job) = svc.claim_next().await? else {
        return Ok(());
    };
    let job_id = job.id;
    info!(%job_id, host = %job.host, "running provisioning job");

    let Some(ssh_secrets) = secrets.get_clone(job_id) else {
        svc.finish(
            job_id,
            JobStatus::Failed,
            Some("SSH credentials expired (server restarted?) — create a new job"),
        )
        .await?;
        return Ok(());
    };

    let log = ArcLog(Arc::new({
        let svc = svc.clone();
        move |line: &str| {
            let svc = svc.clone();
            let line = line.to_string();
            tokio::spawn(async move {
                let _ = svc.append_log(job_id, &line).await;
            });
        }
    }));

    let host_fp = job.host_key_fingerprint.clone().unwrap_or_default();
    if host_fp.is_empty() {
        secrets.clear(job_id);
        svc.finish(
            job_id,
            JobStatus::Failed,
            Some("missing host key fingerprint"),
        )
        .await?;
        return Ok(());
    }

    // Create one-time join token for this install.
    let join_token = match create_join_token(svc.db(), &job.role, job.node_name.as_deref()).await {
        Ok(t) => t,
        Err(err) => {
            secrets.clear(job_id);
            svc.finish(job_id, JobStatus::Failed, Some(&err.to_string()))
                .await?;
            return Ok(());
        }
    };

    let params = InstallParams {
        binaries_dir,
        control_grpc: public_grpc,
        join_token: &join_token,
        role: &job.role,
        node_name: job.node_name.as_deref().unwrap_or(""),
        host_key_fp: &host_fp,
    };

    let install_result = install_node(
        &job.host,
        job.ssh_port,
        &job.ssh_user,
        &ssh_secrets,
        &params,
        &log,
    )
    .await;

    // Drop secrets ASAP after install attempt.
    drop(ssh_secrets);
    secrets.clear(job_id);

    let outcome = match install_result {
        Ok(o) => o,
        Err(err) => {
            let msg = sanitize_error(&err.to_string());
            log.line(&format!("failed: {msg}"));
            svc.finish(job_id, JobStatus::Failed, Some(&msg)).await?;
            return Ok(());
        }
    };

    let name = job.node_name.clone();
    // Defaults when the admin left tunnel CIDRs empty in the wizard.
    let cidr_v4 = job
        .tunnel_cidr_v4
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| vpnc_common::DEFAULT_TUNNEL_CIDR_V4.into());
    let cidr_v6 = job
        .tunnel_cidr_v6
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| vpnc_common::DEFAULT_TUNNEL_CIDR_V6.into());
    let db = svc.db().clone();
    let wait = wait_until(
        || {
            let db = db.clone();
            let name = name.clone();
            Box::pin(async move { node_online(&db, name.as_deref()).await.unwrap_or(false) })
        },
        Duration::from_secs(120),
        &log,
    )
    .await;

    if let Err(err) = wait {
        let msg = sanitize_error(&err.to_string());
        svc.finish(job_id, JobStatus::Failed, Some(&msg)).await?;
        return Ok(());
    }

    if let Err(err) = apply_cidrs(svc.db(), name.as_deref(), Some(&cidr_v4), Some(&cidr_v6)).await
    {
        log.line(&format!("warning: could not set tunnel CIDR: {err}"));
    }

    if let Err(err) = apply_public_ips(svc.db(), name.as_deref(), &outcome.public_ips).await {
        log.line(&format!("warning: could not set public IPs: {err}"));
    }

    svc.finish(job_id, JobStatus::Succeeded, None).await?;
    log.line("succeeded");
    Ok(())
}

async fn create_join_token(db: &Db, role: &str, node_name: Option<&str>) -> anyhow::Result<String> {
    let mut raw = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut raw);
    let token = hex::encode(raw);
    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let expires = (Utc::now() + ChronoDuration::minutes(60)).to_rfc3339();
    match db {
        Db::Sqlite(pool) => {
            sqlx::query(
                r#"
                INSERT INTO node_join_tokens (id, token_hash, node_name, role, expires_at, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(id.to_string())
            .bind(&token_hash)
            .bind(node_name)
            .bind(role)
            .bind(&expires)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        Db::Postgres(pool) => {
            sqlx::query(
                r#"
                INSERT INTO node_join_tokens (id, token_hash, node_name, role, expires_at, created_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(id.to_string())
            .bind(&token_hash)
            .bind(node_name)
            .bind(role)
            .bind(&expires)
            .bind(&now)
            .execute(pool)
            .await?;
        }
    }
    Ok(token)
}

async fn node_online(db: &Db, name: Option<&str>) -> anyhow::Result<bool> {
    let status: Option<String> = match db {
        Db::Sqlite(pool) => {
            if let Some(n) = name.filter(|s| !s.is_empty()) {
                sqlx::query_scalar(
                    "SELECT status FROM nodes WHERE name = ? ORDER BY created_at DESC LIMIT 1",
                )
                .bind(n)
                .fetch_optional(pool)
                .await?
            } else {
                sqlx::query_scalar(
                    "SELECT status FROM nodes WHERE status = 'online' ORDER BY last_seen_at DESC LIMIT 1",
                )
                .fetch_optional(pool)
                .await?
            }
        }
        Db::Postgres(pool) => {
            if let Some(n) = name.filter(|s| !s.is_empty()) {
                sqlx::query_scalar(
                    "SELECT status FROM nodes WHERE name = $1 ORDER BY created_at DESC LIMIT 1",
                )
                .bind(n)
                .fetch_optional(pool)
                .await?
            } else {
                sqlx::query_scalar(
                    "SELECT status FROM nodes WHERE status = 'online' ORDER BY last_seen_at DESC LIMIT 1",
                )
                .fetch_optional(pool)
                .await?
            }
        }
    };
    Ok(status.as_deref() == Some("online"))
}

async fn apply_cidrs(
    db: &Db,
    name: Option<&str>,
    v4: Option<&str>,
    v6: Option<&str>,
) -> anyhow::Result<()> {
    let Some(name) = name.filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    let now = Utc::now().to_rfc3339();
    match db {
        Db::Sqlite(pool) => {
            sqlx::query(
                "UPDATE nodes SET tunnel_cidr_v4 = COALESCE(tunnel_cidr_v4, ?), tunnel_cidr_v6 = COALESCE(tunnel_cidr_v6, ?), updated_at = ? WHERE name = ?",
            )
            .bind(v4)
            .bind(v6)
            .bind(&now)
            .bind(name)
            .execute(pool)
            .await?;
        }
        Db::Postgres(pool) => {
            sqlx::query(
                "UPDATE nodes SET tunnel_cidr_v4 = COALESCE(tunnel_cidr_v4, $1), tunnel_cidr_v6 = COALESCE(tunnel_cidr_v6, $2), updated_at = $3 WHERE name = $4",
            )
            .bind(v4)
            .bind(v6)
            .bind(&now)
            .bind(name)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

async fn apply_public_ips(
    db: &Db,
    name: Option<&str>,
    ips: &DetectedPublicIps,
) -> anyhow::Result<()> {
    let Some(name) = name.filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    let now = Utc::now().to_rfc3339();
    // Only fill empty fields, and never overwrite admin edits (public_endpoint_manual=1).
    match db {
        Db::Sqlite(pool) => {
            sqlx::query(
                r#"
                UPDATE nodes SET
                  public_ip = CASE
                    WHEN COALESCE(public_endpoint_manual, 0) = 1 THEN public_ip
                    ELSE COALESCE(?, public_ip)
                  END,
                  public_ipv6 = CASE
                    WHEN COALESCE(public_endpoint_manual, 0) = 1 THEN public_ipv6
                    ELSE COALESCE(?, public_ipv6)
                  END,
                  updated_at = ?
                WHERE name = ?
                "#,
            )
            .bind(ips.ipv4.as_deref())
            .bind(ips.ipv6.as_deref())
            .bind(&now)
            .bind(name)
            .execute(pool)
            .await?;
        }
        Db::Postgres(pool) => {
            sqlx::query(
                r#"
                UPDATE nodes SET
                  public_ip = CASE
                    WHEN COALESCE(public_endpoint_manual, 0) = 1 THEN public_ip
                    ELSE COALESCE($1, public_ip)
                  END,
                  public_ipv6 = CASE
                    WHEN COALESCE(public_endpoint_manual, 0) = 1 THEN public_ipv6
                    ELSE COALESCE($2, public_ipv6)
                  END,
                  updated_at = $3
                WHERE name = $4
                "#,
            )
            .bind(ips.ipv4.as_deref())
            .bind(ips.ipv6.as_deref())
            .bind(&now)
            .bind(name)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

fn sanitize_error(msg: &str) -> String {
    // Never echo likely secrets.
    let lower = msg.to_ascii_lowercase();
    if lower.contains("password") || lower.contains("private") || lower.contains("token") {
        "installation failed (details redacted)".into()
    } else {
        msg.chars().take(500).collect()
    }
}
