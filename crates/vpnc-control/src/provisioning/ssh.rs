//! SSH helpers via russh (no shelling out to `ssh`/`sshpass`).

use super::jobs::SshSecrets;
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use russh::client::{self, Config, Handle, Handler};
use russh::ChannelMsg;
use russh_keys::key::PrivateKeyWithHashAlg;
use russh_keys::{decode_secret_key, PrivateKey};
use sha2::{Digest, Sha256};
use ssh_key::{HashAlg, PublicKey};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Computes a SHA256 fingerprint for raw key bytes (tests / legacy).
pub fn host_key_fingerprint_sha256(host_key: &[u8]) -> String {
    let digest = Sha256::digest(host_key);
    format!(
        "SHA256:{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD_NO_PAD, digest)
    )
}

/// OpenSSH-style fingerprint of a server public key.
pub fn public_key_fingerprint(key: &PublicKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}

/// Validates that a user-confirmed fingerprint matches the observed key.
pub fn verify_host_key_fingerprint(observed: &str, expected: &str) -> Result<()> {
    if observed != expected {
        bail!("host key fingerprint mismatch");
    }
    Ok(())
}

/// Validates that a user-confirmed fingerprint matches the observed key bytes.
pub fn verify_host_key(observed: &[u8], expected_fingerprint: &str) -> Result<()> {
    verify_host_key_fingerprint(&host_key_fingerprint_sha256(observed), expected_fingerprint)
}

struct CaptureHandler {
    observed: Arc<Mutex<Option<PublicKey>>>,
    expected_fp: Option<String>,
}

#[async_trait]
impl Handler for CaptureHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let fp = public_key_fingerprint(server_public_key);
        if let Some(expected) = &self.expected_fp {
            if &fp != expected {
                return Ok(false);
            }
        }
        *self.observed.lock().expect("host key lock") = Some(server_public_key.clone());
        Ok(true)
    }
}

/// Connected SSH session with helpers for exec and file upload.
pub struct SshSession {
    handle: Handle<CaptureHandler>,
}

impl SshSession {
    /// Connects and authenticates. If `expected_fp` is set, host key must match.
    pub async fn connect(
        host: &str,
        port: u16,
        user: &str,
        secrets: &SshSecrets,
        expected_fp: Option<&str>,
    ) -> Result<(Self, String)> {
        let observed = Arc::new(Mutex::new(None));
        let config = Arc::new(Config {
            inactivity_timeout: Some(Duration::from_secs(120)),
            ..Default::default()
        });
        let handler = CaptureHandler {
            observed: observed.clone(),
            expected_fp: expected_fp.map(str::to_string),
        };
        let mut handle = client::connect(config, (host, port), handler)
            .await
            .with_context(|| format!("ssh connect {host}:{port}"))?;

        let authed = if let Some(pem) = secrets.private_key_pem.as_deref() {
            let key: PrivateKey = decode_secret_key(pem, None).context("decode SSH private key")?;
            let with_hash =
                PrivateKeyWithHashAlg::new(Arc::new(key), None).context("prepare private key")?;
            handle
                .authenticate_publickey(user, with_hash)
                .await
                .context("ssh publickey auth")?
        } else if let Some(password) = secrets.password.as_deref() {
            handle
                .authenticate_password(user, password)
                .await
                .context("ssh password auth")?
        } else {
            bail!("password or private_key_pem required");
        };

        if !authed {
            bail!("ssh authentication failed");
        }

        let fp = {
            let guard = observed.lock().expect("host key lock");
            let key = guard
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("server host key not observed"))?;
            public_key_fingerprint(key)
        };

        Ok((Self { handle }, fp))
    }

    /// Runs a remote command.
    pub async fn exec(&mut self, command: &str) -> Result<(i32, String, String)> {
        let mut channel = self
            .handle
            .channel_open_session()
            .await
            .context("open session channel")?;
        channel.exec(true, command).await.context("exec command")?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut code = 0i32;
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { ref data } => stdout.extend_from_slice(data),
                ChannelMsg::ExtendedData { ref data, .. } => stderr.extend_from_slice(data),
                ChannelMsg::ExitStatus { exit_status } => code = exit_status as i32,
                _ => {}
            }
        }
        Ok((
            code,
            String::from_utf8_lossy(&stdout).into_owned(),
            String::from_utf8_lossy(&stderr).into_owned(),
        ))
    }

    /// Uploads bytes to a remote absolute path via `dd` on stdin.
    pub async fn upload(&mut self, remote_path: &str, data: &[u8]) -> Result<()> {
        if !remote_path.starts_with('/') || remote_path.contains(' ') || remote_path.contains('\'')
        {
            bail!("invalid remote path");
        }
        let cmd = format!("dd of={remote_path} bs=65536 status=none");
        let mut channel = self
            .handle
            .channel_open_session()
            .await
            .context("open upload channel")?;
        channel.exec(true, cmd.as_str()).await.context("exec dd")?;
        let mut offset = 0;
        while offset < data.len() {
            let end = (offset + 32 * 1024).min(data.len());
            channel
                .data(&data[offset..end])
                .await
                .context("upload data")?;
            offset = end;
        }
        channel.eof().await.context("upload eof")?;
        let mut code = 1i32;
        while let Some(msg) = channel.wait().await {
            if let ChannelMsg::ExitStatus { exit_status } = msg {
                code = exit_status as i32;
            }
        }
        if code != 0 {
            bail!("remote dd failed with exit {code}");
        }
        Ok(())
    }

    /// Convenience: write UTF-8 text file.
    pub async fn upload_str(&mut self, remote_path: &str, text: &str) -> Result<()> {
        self.upload(remote_path, text.as_bytes()).await
    }
}

/// Probes host key + auth without installing anything.
pub async fn probe_host(host: &str, port: u16, user: &str, secrets: &SshSecrets) -> Result<String> {
    let (_session, fp) = SshSession::connect(host, port, user, secrets, None).await?;
    Ok(fp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_stable() {
        let fp = host_key_fingerprint_sha256(b"test-key");
        assert!(fp.starts_with("SHA256:"));
        assert_eq!(fp, host_key_fingerprint_sha256(b"test-key"));
    }

    #[test]
    fn verify_rejects_mismatch() {
        let fp = host_key_fingerprint_sha256(b"a");
        assert!(verify_host_key(b"b", &fp).is_err());
    }
}
