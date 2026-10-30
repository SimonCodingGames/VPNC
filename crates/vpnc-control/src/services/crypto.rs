//! Encryption of secrets at rest using ChaCha20-Poly1305.

use anyhow::{anyhow, Context};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use rand::RngCore;
use vpnc_common::Secret;

/// Encrypts and decrypts sensitive blobs with the installation master key.
#[derive(Clone)]
pub struct CryptoService {
    key: Secret<[u8; 32]>,
}

impl CryptoService {
    /// Creates a crypto service from a 32-byte master key.
    pub fn new(key: Secret<[u8; 32]>) -> Self {
        Self { key }
    }

    /// Encrypts plaintext and returns `base64(nonce || ciphertext)`.
    pub fn encrypt(&self, plaintext: &[u8]) -> anyhow::Result<String> {
        let cipher = ChaCha20Poly1305::new_from_slice(self.key.expose())
            .map_err(|_| anyhow!("invalid master key length"))?;
        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| anyhow!("encryption failed"))?;
        let mut out = Vec::with_capacity(12 + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(B64.encode(out))
    }

    /// Decrypts a value produced by [`Self::encrypt`].
    pub fn decrypt(&self, encoded: &str) -> anyhow::Result<Secret<Vec<u8>>> {
        let raw = B64
            .decode(encoded.trim())
            .context("invalid base64 ciphertext")?;
        if raw.len() < 13 {
            anyhow::bail!("ciphertext too short");
        }
        let (nonce_bytes, ciphertext) = raw.split_at(12);
        let cipher = ChaCha20Poly1305::new_from_slice(self.key.expose())
            .map_err(|_| anyhow!("invalid master key length"))?;
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| anyhow!("decryption failed"))?;
        Ok(Secret::new(plaintext))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let key = Secret::new([7u8; 32]);
        let crypto = CryptoService::new(key);
        let enc = crypto.encrypt(b"cluster-private-key").unwrap();
        assert!(!enc.contains("cluster-private-key"));
        let dec = crypto.decrypt(&enc).unwrap();
        assert_eq!(dec.expose(), b"cluster-private-key");
    }
}
