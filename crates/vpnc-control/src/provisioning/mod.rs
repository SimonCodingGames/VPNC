//! SSH-based node provisioning.

pub mod binaries;
pub mod detect;
pub mod install;
pub mod jobs;
pub mod preflight;
pub mod secrets;
pub mod ssh;
pub mod worker;

pub use jobs::{JobStatus, ProvisioningJob, ProvisioningService, SshSecrets};
pub use secrets::SecretStore;
pub use worker::spawn_worker;
