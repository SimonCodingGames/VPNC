//! Session-based authentication.

use crate::db::Db;
use crate::services::users::{self, UserRecord};
use anyhow::{bail, Context};
use axum_extra::extract::cookie::{Cookie, SameSite};
use chrono::Utc;
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const SESSION_COOKIE: &str = "vpnc_session";

/// Authentication and session management.
#[derive(Clone)]
pub struct AuthService {
    db: Db,
    session_hours: i64,
    cookie_secure: bool,
}

impl AuthService {
    /// Creates an auth service.
    pub fn new(db: Db, session_hours: i64, cookie_secure: bool) -> Self {
        Self {
            db,
            session_hours,
            cookie_secure,
        }
    }

    /// Validates credentials and creates a server-side session.
    /// Returns the session cookie value (raw token) and user.
    pub async fn login(&self, email: &str, password: &str) -> anyhow::Result<(String, UserRecord)> {
        let Some((user, password_hash)) = users::find_by_email(&self.db, email).await? else {
            bail!("invalid credentials");
        };
        if !users::verify_password(password, &password_hash)? {
            bail!("invalid credentials");
        }

        let raw_token = generate_token();
        let token_hash = hash_token(&raw_token);
        let session_id = Uuid::new_v4();
        let now = Utc::now();
        let expires = users::session_expiry(self.session_hours);

        match &self.db {
            Db::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO sessions (id, user_id, token_hash, expires_at, created_at)
                    VALUES (?, ?, ?, ?, ?)
                    "#,
                )
                .bind(session_id.to_string())
                .bind(user.id.to_string())
                .bind(&token_hash)
                .bind(expires.to_rfc3339())
                .bind(now.to_rfc3339())
                .execute(pool)
                .await
                .context("insert session")?;
            }
            Db::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO sessions (id, user_id, token_hash, expires_at, created_at)
                    VALUES ($1, $2, $3, $4, $5)
                    "#,
                )
                .bind(session_id.to_string())
                .bind(user.id.to_string())
                .bind(&token_hash)
                .bind(expires.to_rfc3339())
                .bind(now.to_rfc3339())
                .execute(pool)
                .await
                .context("insert session")?;
            }
        }

        Ok((raw_token, user))
    }

    /// Deletes the session identified by the raw cookie token.
    pub async fn logout(&self, raw_token: &str) -> anyhow::Result<()> {
        let token_hash = hash_token(raw_token);
        match &self.db {
            Db::Sqlite(pool) => {
                sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
                    .bind(&token_hash)
                    .execute(pool)
                    .await?;
            }
            Db::Postgres(pool) => {
                sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
                    .bind(&token_hash)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    /// Resolves the current user from a raw session token.
    pub async fn user_from_token(&self, raw_token: &str) -> anyhow::Result<Option<UserRecord>> {
        let token_hash = hash_token(raw_token);
        let now = Utc::now().to_rfc3339();

        let user_id: Option<String> = match &self.db {
            Db::Sqlite(pool) => {
                sqlx::query_scalar(
                    r#"
                    SELECT user_id FROM sessions
                    WHERE token_hash = ? AND expires_at > ?
                    "#,
                )
                .bind(&token_hash)
                .bind(&now)
                .fetch_optional(pool)
                .await?
            }
            Db::Postgres(pool) => {
                sqlx::query_scalar(
                    r#"
                    SELECT user_id FROM sessions
                    WHERE token_hash = $1 AND expires_at > $2
                    "#,
                )
                .bind(&token_hash)
                .bind(&now)
                .fetch_optional(pool)
                .await?
            }
        };

        let Some(user_id) = user_id else {
            return Ok(None);
        };
        let id = Uuid::parse_str(&user_id)?;
        users::find_by_id(&self.db, id).await
    }

    /// Builds the session cookie.
    pub fn session_cookie(&self, raw_token: &str) -> Cookie<'static> {
        let mut builder = Cookie::build((SESSION_COOKIE, raw_token.to_string()))
            .http_only(true)
            .same_site(SameSite::Lax)
            .path("/")
            .max_age(cookie::time::Duration::hours(self.session_hours));
        if self.cookie_secure {
            builder = builder.secure(true);
        }
        builder.build()
    }

    /// Builds a clearing cookie.
    pub fn clear_session_cookie(&self) -> Cookie<'static> {
        let mut builder = Cookie::build((SESSION_COOKIE, ""))
            .http_only(true)
            .same_site(SameSite::Lax)
            .path("/")
            .max_age(cookie::time::Duration::seconds(0));
        if self.cookie_secure {
            builder = builder.secure(true);
        }
        builder.build()
    }
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn hash_token(raw: &str) -> String {
    let digest = Sha256::digest(raw.as_bytes());
    hex::encode(digest)
}
