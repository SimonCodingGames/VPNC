//! SSH helpers via russh (no shelling out to `ssh`/`sshpass`).

use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

/// Computes a SHA256 fingerprint for an OpenSSH host key blob (raw bytes).
pub fn host_key_fingerprint_sha256(host_key: &[u8]) -> String {
    let digest = Sha256::digest(host_key);
    format!(
        "SHA256:{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD_NO_PAD, digest,)
    )
}

/// Validates that a user-confirmed fingerprint matches the observed key.
pub fn verify_host_key(observed: &[u8], expected_fingerprint: &str) -> Result<()> {
    let got = host_key_fingerprint_sha256(observed);
    if got != expected_fingerprint {
        bail!("host key fingerprint mismatch");
    }
    Ok(())
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
