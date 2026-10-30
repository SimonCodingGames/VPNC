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
    /// Connects using a `sqlite:` / `sqlite://` or `postgres://` / `postgresql://` URL
    /// and runs migrations.
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
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
            Ok(Self::Sqlite(pool))
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
            Ok(Self::Postgres(pool))
        } else {
            bail!("unsupported database URL (expected sqlite: or postgres://): {database_url}");
        }
    }
}
