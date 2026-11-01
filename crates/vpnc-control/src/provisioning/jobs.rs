//! DB-backed provisioning job queue.

use crate::db::Db;
use anyhow::Context;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroize;

/// Job lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Waiting to start / be claimed.
    Pending,
    /// Waiting for admin to confirm SSH host key.
    AwaitingHostkey,
    /// Installation running.
    Running,
    /// Finished successfully.
    Succeeded,
    /// Finished with error.
    Failed,
    /// Cancelled by admin.
    Cancelled,
}

impl JobStatus {
    /// Database string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::AwaitingHostkey => "awaiting_hostkey",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse from DB.
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        Ok(match s {
            "pending" => Self::Pending,
            "awaiting_hostkey" => Self::AwaitingHostkey,
            "running" => Self::Running,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            other => anyhow::bail!("unknown job status: {other}"),
        })
    }
}

/// Persisted provisioning job (no SSH secrets).
#[derive(Debug, Clone, Serialize)]
pub struct ProvisioningJob {
    /// Job id.
    pub id: Uuid,
    /// Status.
    pub status: JobStatus,
    /// Target host.
    pub host: String,
    /// SSH port.
    pub ssh_port: u16,
    /// SSH username.
    pub ssh_user: String,
    /// Desired node role.
    pub role: String,
    /// Optional display name for the node.
    pub node_name: Option<String>,
    /// Optional tunnel CIDR v4.
    pub tunnel_cidr_v4: Option<String>,
    /// Optional tunnel CIDR v6.
    pub tunnel_cidr_v6: Option<String>,
    /// Observed / confirmed host-key fingerprint.
    pub host_key_fingerprint: Option<String>,
    /// Error message if failed (never contains passwords).
    pub error_message: Option<String>,
    /// Accumulated log text.
    pub log_text: String,
}

/// Ephemeral SSH credentials — RAM only, zeroized on drop.
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct SshSecrets {
    /// Password auth (optional).
    pub password: Option<String>,
    /// Private key PEM (optional).
    pub private_key_pem: Option<String>,
}

/// Provisioning service.
#[derive(Clone)]
pub struct ProvisioningService {
    db: Db,
}

impl ProvisioningService {
    /// Creates the service.
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Enqueues a job in `awaiting_hostkey` after probe. Secrets must never be written to the DB.
    pub async fn insert_awaiting_hostkey(
        &self,
        host: &str,
        ssh_port: u16,
        ssh_user: &str,
        role: &str,
        node_name: Option<&str>,
        tunnel_cidr_v4: Option<&str>,
        tunnel_cidr_v6: Option<&str>,
        host_key_fingerprint: &str,
    ) -> anyhow::Result<ProvisioningJob> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        match &self.db {
            Db::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO provisioning_jobs
                      (id, status, host, ssh_port, ssh_user, role, node_name, tunnel_cidr_v4, tunnel_cidr_v6,
                       host_key_fingerprint, observed_host_key, created_at, updated_at)
                    VALUES (?, 'awaiting_hostkey', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(id.to_string())
                .bind(host)
                .bind(i64::from(ssh_port))
                .bind(ssh_user)
                .bind(role)
                .bind(node_name)
                .bind(tunnel_cidr_v4)
                .bind(tunnel_cidr_v6)
                .bind(host_key_fingerprint)
                .bind(host_key_fingerprint)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await
                .context("insert provisioning job")?;
            }
            Db::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO provisioning_jobs
                      (id, status, host, ssh_port, ssh_user, role, node_name, tunnel_cidr_v4, tunnel_cidr_v6,
                       host_key_fingerprint, observed_host_key, created_at, updated_at)
                    VALUES ($1, 'awaiting_hostkey', $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                    "#,
                )
                .bind(id.to_string())
                .bind(host)
                .bind(i64::from(ssh_port))
                .bind(ssh_user)
                .bind(role)
                .bind(node_name)
                .bind(tunnel_cidr_v4)
                .bind(tunnel_cidr_v6)
                .bind(host_key_fingerprint)
                .bind(host_key_fingerprint)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await
                .context("insert provisioning job")?;
            }
        }
        Ok(ProvisioningJob {
            id,
            status: JobStatus::AwaitingHostkey,
            host: host.to_string(),
            ssh_port,
            ssh_user: ssh_user.to_string(),
            role: role.to_string(),
            node_name: node_name.map(str::to_string),
            tunnel_cidr_v4: tunnel_cidr_v4.map(str::to_string),
            tunnel_cidr_v6: tunnel_cidr_v6.map(str::to_string),
            host_key_fingerprint: Some(host_key_fingerprint.to_string()),
            error_message: None,
            log_text: String::new(),
        })
    }

    /// Loads a job by id.
    pub async fn get(&self, id: Uuid) -> anyhow::Result<Option<ProvisioningJob>> {
        fetch_job(&self.db, id).await
    }

    /// Lists jobs (cursor = last id, ascending).
    pub async fn list(
        &self,
        cursor: Option<&str>,
        limit: i64,
    ) -> anyhow::Result<Vec<ProvisioningJob>> {
        fetch_jobs(&self.db, cursor, limit).await
    }

    /// Confirms host key and marks job `pending` for the worker.
    pub async fn confirm_hostkey(&self, id: Uuid, fingerprint: &str) -> anyhow::Result<()> {
        let job = fetch_job(&self.db, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("job not found"))?;
        if job.status != JobStatus::AwaitingHostkey {
            anyhow::bail!("job is not awaiting host key");
        }
        let observed = job
            .host_key_fingerprint
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("no observed host key"))?;
        if observed != fingerprint {
            anyhow::bail!("host key fingerprint mismatch");
        }
        set_status(&self.db, id, JobStatus::Pending, None).await?;
        append_log(&self.db, id, "host key confirmed; queued for install").await?;
        Ok(())
    }

    /// Cancels a job if not finished.
    pub async fn cancel(&self, id: Uuid) -> anyhow::Result<()> {
        let job = fetch_job(&self.db, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("job not found"))?;
        if matches!(
            job.status,
            JobStatus::Succeeded | JobStatus::Failed | JobStatus::Cancelled
        ) {
            anyhow::bail!("job already finished");
        }
        set_status(&self.db, id, JobStatus::Cancelled, None).await?;
        append_log(&self.db, id, "cancelled by admin").await?;
        Ok(())
    }

    /// Claims next pending job (marks running). SQLite: simple update; Postgres: SKIP LOCKED.
    pub async fn claim_next(&self) -> anyhow::Result<Option<ProvisioningJob>> {
        claim_next_job(&self.db).await
    }

    /// Appends a log line.
    pub async fn append_log(&self, id: Uuid, line: &str) -> anyhow::Result<()> {
        append_log(&self.db, id, line).await
    }

    /// Marks job succeeded/failed.
    pub async fn finish(
        &self,
        id: Uuid,
        status: JobStatus,
        error: Option<&str>,
    ) -> anyhow::Result<()> {
        set_status(&self.db, id, status, error).await
    }

    /// Database handle for wait helpers.
    pub fn db(&self) -> &Db {
        &self.db
    }
}

async fn fetch_job(db: &Db, id: Uuid) -> anyhow::Result<Option<ProvisioningJob>> {
    let row: Option<(
        String,
        String,
        String,
        i64,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    )> = match db {
        Db::Sqlite(pool) => {
            sqlx::query_as(
                r#"
                SELECT id, status, host, ssh_port, ssh_user, role, node_name, tunnel_cidr_v4, tunnel_cidr_v6,
                       host_key_fingerprint, error_message, log_text
                FROM provisioning_jobs WHERE id = ?
                "#,
            )
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?
        }
        Db::Postgres(pool) => {
            sqlx::query_as(
                r#"
                SELECT id, status, host, ssh_port, ssh_user, role, node_name, tunnel_cidr_v4, tunnel_cidr_v6,
                       host_key_fingerprint, error_message, log_text
                FROM provisioning_jobs WHERE id = $1
                "#,
            )
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?
        }
    };
    Ok(row.map(map_job).transpose()?.flatten())
}

fn map_job(
    row: (
        String,
        String,
        String,
        i64,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    ),
) -> anyhow::Result<Option<ProvisioningJob>> {
    let (
        id,
        status,
        host,
        ssh_port,
        ssh_user,
        role,
        node_name,
        tunnel_cidr_v4,
        tunnel_cidr_v6,
        host_key_fingerprint,
        error_message,
        log_text,
    ) = row;
    Ok(Some(ProvisioningJob {
        id: Uuid::parse_str(&id)?,
        status: JobStatus::parse(&status)?,
        host,
        ssh_port: ssh_port as u16,
        ssh_user,
        role,
        node_name,
        tunnel_cidr_v4,
        tunnel_cidr_v6,
        host_key_fingerprint,
        error_message,
        log_text,
    }))
}

async fn fetch_jobs(
    db: &Db,
    cursor: Option<&str>,
    limit: i64,
) -> anyhow::Result<Vec<ProvisioningJob>> {
    let rows: Vec<(
        String,
        String,
        String,
        i64,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    )> = match db {
        Db::Sqlite(pool) => {
            sqlx::query_as(
                r#"
                SELECT id, status, host, ssh_port, ssh_user, role, node_name, tunnel_cidr_v4, tunnel_cidr_v6,
                       host_key_fingerprint, error_message, log_text
                FROM provisioning_jobs
                WHERE (?1 IS NULL OR id > ?1)
                ORDER BY id ASC LIMIT ?2
                "#,
            )
            .bind(cursor)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        Db::Postgres(pool) => {
            sqlx::query_as(
                r#"
                SELECT id, status, host, ssh_port, ssh_user, role, node_name, tunnel_cidr_v4, tunnel_cidr_v6,
                       host_key_fingerprint, error_message, log_text
                FROM provisioning_jobs
                WHERE ($1::text IS NULL OR id > $1)
                ORDER BY id ASC LIMIT $2
                "#,
            )
            .bind(cursor)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
    };
    rows.into_iter()
        .map(|r| map_job(r).and_then(|o| o.ok_or_else(|| anyhow::anyhow!("empty job"))))
        .collect()
}

async fn set_status(
    db: &Db,
    id: Uuid,
    status: JobStatus,
    error: Option<&str>,
) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    match db {
        Db::Sqlite(pool) => {
            sqlx::query(
                "UPDATE provisioning_jobs SET status = ?, error_message = COALESCE(?, error_message), updated_at = ?, finished_at = CASE WHEN ? IN ('succeeded','failed','cancelled') THEN ? ELSE finished_at END, started_at = CASE WHEN ? = 'running' AND started_at IS NULL THEN ? ELSE started_at END WHERE id = ?",
            )
            .bind(status.as_str())
            .bind(error)
            .bind(&now)
            .bind(status.as_str())
            .bind(&now)
            .bind(status.as_str())
            .bind(&now)
            .bind(id.to_string())
            .execute(pool)
            .await?;
        }
        Db::Postgres(pool) => {
            sqlx::query(
                "UPDATE provisioning_jobs SET status = $1, error_message = COALESCE($2, error_message), updated_at = $3, finished_at = CASE WHEN $1 IN ('succeeded','failed','cancelled') THEN $3 ELSE finished_at END, started_at = CASE WHEN $1 = 'running' AND started_at IS NULL THEN $3 ELSE started_at END WHERE id = $4",
            )
            .bind(status.as_str())
            .bind(error)
            .bind(&now)
            .bind(id.to_string())
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

async fn append_log(db: &Db, id: Uuid, line: &str) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    let stamped = format!("[{now}] {line}\n");
    match db {
        Db::Sqlite(pool) => {
            sqlx::query(
                "UPDATE provisioning_jobs SET log_text = log_text || ?, updated_at = ? WHERE id = ?",
            )
            .bind(&stamped)
            .bind(&now)
            .bind(id.to_string())
            .execute(pool)
            .await?;
        }
        Db::Postgres(pool) => {
            sqlx::query(
                "UPDATE provisioning_jobs SET log_text = log_text || $1, updated_at = $2 WHERE id = $3",
            )
            .bind(&stamped)
            .bind(&now)
            .bind(id.to_string())
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

async fn claim_next_job(db: &Db) -> anyhow::Result<Option<ProvisioningJob>> {
    let now = Utc::now().to_rfc3339();
    match db {
        Db::Sqlite(pool) => {
            let id: Option<String> = sqlx::query_scalar(
                "SELECT id FROM provisioning_jobs WHERE status = 'pending' ORDER BY id ASC LIMIT 1",
            )
            .fetch_optional(pool)
            .await?;
            let Some(id) = id else {
                return Ok(None);
            };
            let updated = sqlx::query(
                "UPDATE provisioning_jobs SET status = 'running', started_at = ?, updated_at = ? WHERE id = ? AND status = 'pending'",
            )
            .bind(&now)
            .bind(&now)
            .bind(&id)
            .execute(pool)
            .await?
            .rows_affected();
            if updated == 0 {
                return Ok(None);
            }
            fetch_job(db, Uuid::parse_str(&id)?).await
        }
        Db::Postgres(pool) => {
            let id: Option<String> = sqlx::query_scalar(
                r#"
                UPDATE provisioning_jobs SET status = 'running', started_at = $1, updated_at = $1
                WHERE id = (
                  SELECT id FROM provisioning_jobs WHERE status = 'pending'
                  ORDER BY id ASC LIMIT 1 FOR UPDATE SKIP LOCKED
                )
                RETURNING id
                "#,
            )
            .bind(&now)
            .fetch_optional(pool)
            .await?;
            match id {
                Some(id) => fetch_job(db, Uuid::parse_str(&id)?).await,
                None => Ok(None),
            }
        }
    }
}
