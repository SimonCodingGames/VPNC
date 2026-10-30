//! Configuration loading via figment (TOML file + `VPNC_*` env overrides).

use figment::providers::{Env, Format, Serialized, Toml};
use figment::Figment;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors while loading configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Figment failed to merge or extract configuration.
    #[error("failed to load config: {0}")]
    Figment(Box<figment::Error>),
}

impl From<figment::Error> for ConfigError {
    fn from(value: figment::Error) -> Self {
        Self::Figment(Box::new(value))
    }
}

/// Returns the default config path for a service: `/etc/vpnc/<service>.toml`.
pub fn default_config_path(service: &str) -> PathBuf {
    PathBuf::from("/etc/vpnc").join(format!("{service}.toml"))
}

/// Loads configuration for `service`.
///
/// Merge order (later wins):
/// 1. `defaults`
/// 2. TOML file at `path` (if it exists; otherwise skipped)
/// 3. Environment variables with prefix `VPNC_` (nested keys use `__`)
///
/// Example: `VPNC_HTTP__BIND=0.0.0.0:8080` overrides `http.bind`.
pub fn load_config<T>(service: &str, path: Option<&Path>, defaults: T) -> Result<T, ConfigError>
where
    T: DeserializeOwned + Serialize,
{
    let config_path = path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_config_path(service));

    let mut figment = Figment::new().merge(Serialized::defaults(defaults));

    if config_path.exists() {
        figment = figment.merge(Toml::file(&config_path));
    }

    figment = figment.merge(Env::prefixed("VPNC_").split("__"));

    Ok(figment.extract()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct SampleConfig {
        bind: String,
        workers: u32,
    }

    impl Default for SampleConfig {
        fn default() -> Self {
            Self {
                bind: "127.0.0.1:8080".into(),
                workers: 2,
            }
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn loads_defaults_when_file_missing() {
        let _guard = env_lock().lock().unwrap();
        std::env::remove_var("VPNC_BIND");
        std::env::remove_var("VPNC_WORKERS");

        let cfg = load_config("missing-service", None, SampleConfig::default()).unwrap();
        assert_eq!(cfg, SampleConfig::default());
    }

    #[test]
    fn loads_from_toml_file() {
        let _guard = env_lock().lock().unwrap();
        std::env::remove_var("VPNC_BIND");
        std::env::remove_var("VPNC_WORKERS");

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("control.toml");
        fs::write(
            &path,
            r#"
bind = "0.0.0.0:9000"
workers = 8
"#,
        )
        .unwrap();

        let cfg = load_config("control", Some(&path), SampleConfig::default()).unwrap();
        assert_eq!(cfg.bind, "0.0.0.0:9000");
        assert_eq!(cfg.workers, 8);
    }

    #[test]
    fn env_overrides_file() {
        let _guard = env_lock().lock().unwrap();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("control.toml");
        fs::write(
            &path,
            r#"
bind = "0.0.0.0:9000"
workers = 8
"#,
        )
        .unwrap();

        std::env::set_var("VPNC_BIND", "10.0.0.1:4444");
        std::env::set_var("VPNC_WORKERS", "16");

        let cfg = load_config("control", Some(&path), SampleConfig::default()).unwrap();
        assert_eq!(cfg.bind, "10.0.0.1:4444");
        assert_eq!(cfg.workers, 16);

        std::env::remove_var("VPNC_BIND");
        std::env::remove_var("VPNC_WORKERS");
    }
}
