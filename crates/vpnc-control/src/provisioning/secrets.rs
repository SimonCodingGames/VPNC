//! In-memory SSH secrets for provisioning jobs (never persisted).

use super::jobs::SshSecrets;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Process-local store for SSH credentials keyed by job id.
#[derive(Clone, Default)]
pub struct SecretStore {
    inner: Arc<Mutex<HashMap<Uuid, SshSecrets>>>,
}

impl SecretStore {
    /// Inserts secrets for a job (replaces any previous entry).
    pub fn insert(&self, job_id: Uuid, secrets: SshSecrets) {
        self.inner
            .lock()
            .expect("secret store")
            .insert(job_id, secrets);
    }

    /// Takes secrets out of the store (caller owns them; zeroized on drop).
    pub fn take(&self, job_id: Uuid) -> Option<SshSecrets> {
        self.inner.lock().expect("secret store").remove(&job_id)
    }

    /// Borrows a clone for a short-lived operation, keeping the original in store.
    pub fn get_clone(&self, job_id: Uuid) -> Option<SshSecrets> {
        let guard = self.inner.lock().expect("secret store");
        guard.get(&job_id).map(|s| SshSecrets {
            password: s.password.clone(),
            private_key_pem: s.private_key_pem.clone(),
        })
    }

    /// Drops secrets for a job.
    pub fn clear(&self, job_id: Uuid) {
        let _ = self.take(job_id);
    }
}
