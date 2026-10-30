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
    /// Optional host-key fingerprint (after confirm).
    pub host_key_fingerprint: Option<String>,
    /// Error message if failed (never contains passwords).
    pub error_message: Option<String>,
}

/// Ephemeral SSH credentials — RAM only, zeroized on drop.
#[derive(Zeroize)]
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

    /// Enqueues a job. Secrets must never be written to the DB.
    pub async fn enqueue(
        &self,
        host: &str,
        ssh_port: u16,
        ssh_user: &str,
        role: &str,
        _secrets: &SshSecrets,
    ) -> anyhow::Result<ProvisioningJob> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        match &self.db {
            Db::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO provisioning_jobs
                      (id, status, host, ssh_port, ssh_user, role, created_at, updated_at)
                    VALUES (?, 'awaiting_hostkey', ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(id.to_string())
                .bind(host)
                .bind(i64::from(ssh_port))
                .bind(ssh_user)
                .bind(role)
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
                      (id, status, host, ssh_port, ssh_user, role, created_at, updated_at)
                    VALUES ($1, 'awaiting_hostkey', $2, $3, $4, $5, $6, $7)
                    "#,
                )
                .bind(id.to_string())
                .bind(host)
                .bind(i64::from(ssh_port))
                .bind(ssh_user)
                .bind(role)
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
            host_key_fingerprint: None,
            error_message: None,
        })
    }
}
