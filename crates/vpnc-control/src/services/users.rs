//! User persistence helpers.

use crate::db::Db;
use anyhow::{bail, Context};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use chrono::{Duration, Utc};
use rand::rngs::OsRng;
use uuid::Uuid;

/// Application role stored in `users.role`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    /// Full administrative access.
    Admin,
    /// Standard end-user.
    User,
}

impl UserRole {
    /// Parses a role from the database string.
    pub fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "admin" => Ok(Self::Admin),
            "user" => Ok(Self::User),
            other => bail!("unknown role: {other}"),
        }
    }

    /// Returns the database representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::User => "user",
        }
    }
}

/// User row used by the API layer.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UserRecord {
    /// User id.
    pub id: Uuid,
    /// Login email.
    pub email: String,
    /// Role.
    pub role: UserRole,
}

/// Hashes a password with argon2id.
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("hash password: {e}"))?
        .to_string();
    Ok(hash)
}

/// Verifies a password against an argon2id hash.
pub fn verify_password(password: &str, password_hash: &str) -> anyhow::Result<bool> {
    let parsed = PasswordHash::new(password_hash)
        .map_err(|e| anyhow::anyhow!("parse password hash: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Creates the first admin user. Fails if any user already exists.
pub async fn bootstrap_admin(db: &Db, email: &str, password: &str) -> anyhow::Result<UserRecord> {
    if email.trim().is_empty() {
        bail!("email must not be empty");
    }
    if password.len() < 10 {
        bail!("password must be at least 10 characters");
    }

    let count: i64 = match db {
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
    };
    if count > 0 {
        bail!("bootstrap refused: users already exist");
    }

    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let password_hash = hash_password(password)?;

    match db {
        Db::Sqlite(pool) => {
            sqlx::query(
                r#"
                INSERT INTO users (id, email, password_hash, role, created_at, updated_at)
                VALUES (?, ?, ?, 'admin', ?, ?)
                "#,
            )
            .bind(id.to_string())
            .bind(email.trim().to_lowercase())
            .bind(&password_hash)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await
            .context("insert admin")?;
        }
        Db::Postgres(pool) => {
            sqlx::query(
                r#"
                INSERT INTO users (id, email, password_hash, role, created_at, updated_at)
                VALUES ($1, $2, $3, 'admin', $4, $5)
                "#,
            )
            .bind(id.to_string())
            .bind(email.trim().to_lowercase())
            .bind(&password_hash)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await
            .context("insert admin")?;
        }
    }

    Ok(UserRecord {
        id,
        email: email.trim().to_lowercase(),
        role: UserRole::Admin,
    })
}

/// Looks up a user by email. Disabled accounts are treated as missing.
pub async fn find_by_email(db: &Db, email: &str) -> anyhow::Result<Option<(UserRecord, String)>> {
    let email = email.trim().to_lowercase();
    let row = match db {
        Db::Sqlite(pool) => {
            sqlx::query_as::<_, (String, String, String, String, Option<String>)>(
                "SELECT id, email, password_hash, role, disabled_at FROM users WHERE email = ?",
            )
            .bind(&email)
            .fetch_optional(pool)
            .await?
        }
        Db::Postgres(pool) => {
            sqlx::query_as::<_, (String, String, String, String, Option<String>)>(
                "SELECT id, email, password_hash, role, disabled_at FROM users WHERE email = $1",
            )
            .bind(&email)
            .fetch_optional(pool)
            .await?
        }
    };

    Ok(match row {
        Some((_, _, _, _, Some(_))) => None,
        Some((id, email, password_hash, role, None)) => Some((
            UserRecord {
                id: Uuid::parse_str(&id)?,
                email,
                role: UserRole::parse(&role)?,
            },
            password_hash,
        )),
        None => None,
    })
}

/// Looks up a user by id.
pub async fn find_by_id(db: &Db, id: Uuid) -> anyhow::Result<Option<UserRecord>> {
    let row = match db {
        Db::Sqlite(pool) => {
            sqlx::query_as::<_, (String, String, String)>(
                "SELECT id, email, role FROM users WHERE id = ?",
            )
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?
        }
        Db::Postgres(pool) => {
            sqlx::query_as::<_, (String, String, String)>(
                "SELECT id, email, role FROM users WHERE id = $1",
            )
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?
        }
    };

    Ok(match row {
        Some((id, email, role)) => Some(UserRecord {
            id: Uuid::parse_str(&id)?,
            email,
            role: UserRole::parse(&role)?,
        }),
        None => None,
    })
}

/// Session lifetime helper.
pub fn session_expiry(hours: i64) -> chrono::DateTime<Utc> {
    Utc::now() + Duration::hours(hours)
}
