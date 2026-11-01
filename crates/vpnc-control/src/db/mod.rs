//! Database pool setup for SQLite and PostgreSQL.

use anyhow::{bail, Context};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{PgPool, SqlitePool};
use std::str::FromStr;
use std::time::Duration;

/// Database backend handle.
#[derive(Clone)]
pub enum Db {
    /// SQLite pool (single-instance installs).
    Sqlite(SqlitePool),
    /// PostgreSQL pool (production / multi-instance).
    Postgres(PgPool),
}

impl Db {
    /// Connects and migrates, retrying with exponential backoff if the DB is not ready.
    pub async fn connect_with_retry(database_url: &str, max_attempts: u32) -> anyhow::Result<Self> {
        let mut delay = Duration::from_secs(1);
        let mut last_err = None;
        for attempt in 1..=max_attempts.max(1) {
            match Self::connect_once(database_url).await {
                Ok(db) => return Ok(db),
                Err(err) => {
                    tracing::warn!(
                        attempt,
                        max_attempts,
                        error = %err,
                        "database not ready; retrying"
                    );
                    last_err = Some(err);
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(30));
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("database connection failed")))
    }

    /// Connects using a `sqlite:` / `sqlite://` or `postgres://` / `postgresql://` URL
    /// and runs migrations.
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        Self::connect_once(database_url).await
    }

    async fn connect_once(database_url: &str) -> anyhow::Result<Self> {
        if database_url.starts_with("sqlite:") {
            let opts = SqliteConnectOptions::from_str(database_url)
                .context("invalid sqlite URL")?
                .create_if_missing(true)
                .foreign_keys(true);
            let pool = SqlitePoolOptions::new()
                .max_connections(5)
                .acquire_timeout(Duration::from_secs(10))
                .connect_with(opts)
                .await
                .context("connect sqlite")?;
            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .context("sqlite migrations")?;
            let db = Self::Sqlite(pool);
            db.ensure_node_endpoint_columns().await?;
            Ok(db)
        } else if database_url.starts_with("postgres://")
            || database_url.starts_with("postgresql://")
        {
            let pool = PgPoolOptions::new()
                .max_connections(10)
                .acquire_timeout(Duration::from_secs(10))
                .connect(database_url)
                .await
                .context("connect postgres")?;
            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .context("postgres migrations")?;
            let db = Self::Postgres(pool);
            db.ensure_node_endpoint_columns().await?;
            Ok(db)
        } else {
            bail!("unsupported database URL (expected sqlite: or postgres://): {database_url}");
        }
    }

    /// Idempotent schema guards for endpoint columns (survives partial deploys).
    async fn ensure_node_endpoint_columns(&self) -> anyhow::Result<()> {
        match self {
            Db::Sqlite(pool) => {
                let _ = sqlx::query(
                    "ALTER TABLE nodes ADD COLUMN public_ipv6 TEXT",
                )
                .execute(pool)
                .await;
                let _ = sqlx::query(
                    "ALTER TABLE nodes ADD COLUMN public_endpoint_manual INTEGER NOT NULL DEFAULT 0",
                )
                .execute(pool)
                .await;
            }
            Db::Postgres(pool) => {
                sqlx::query(
                    "ALTER TABLE nodes ADD COLUMN IF NOT EXISTS public_ipv6 TEXT",
                )
                .execute(pool)
                .await
                .context("ensure public_ipv6")?;
                sqlx::query(
                    "ALTER TABLE nodes ADD COLUMN IF NOT EXISTS public_endpoint_manual INTEGER NOT NULL DEFAULT 0",
                )
                .execute(pool)
                .await
                .context("ensure public_endpoint_manual")?;
            }
        }
        Ok(())
    }

    /// Returns the number of users (for bootstrap checks).
    pub async fn user_count(&self) -> anyhow::Result<i64> {
        Ok(match self {
            Db::Sqlite(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM users")
                    .fetch_one(pool)
                    .await?
            }
            Db::Postgres(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM users")
                    .fetch_one(pool)
                    .await?
            }
        })
    }
}
