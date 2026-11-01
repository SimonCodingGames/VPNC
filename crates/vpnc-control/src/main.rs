//! VPNC control plane binary.

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;
use tonic::transport::Server;
use vpnc_common::init_telemetry;
use vpnc_control::config::ControlConfig;
use vpnc_control::db::Db;
use vpnc_control::grpc::node::mark_offline_nodes;
use vpnc_control::grpc::pb::node::node_service_server::NodeServiceServer;
use vpnc_control::grpc::{NodeGrpcService, NodeRuntimeState};
use vpnc_control::services::users;
use vpnc_control::{bind_addr, build_app, build_state};

#[derive(Debug, Parser)]
#[command(name = "vpnc-control", about = "VPNC control plane")]
struct Cli {
    /// Optional path to a TOML config file (default: /etc/vpnc/control.toml).
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create the first admin user.
    BootstrapAdmin {
        /// Admin email address.
        #[arg(long)]
        email: String,
        /// Admin password (min. 10 characters). Prefer env `VPNC_BOOTSTRAP_PASSWORD`.
        #[arg(long, env = "VPNC_BOOTSTRAP_PASSWORD")]
        password: String,
    },
    /// Exit 0 if `/api/health` would succeed (used by Docker HEALTHCHECK).
    Healthcheck {
        /// HTTP base URL of this process.
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        url: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(Command::Healthcheck { url }) = &cli.command {
        let health_url = format!("{}/api/health", url.trim_end_matches('/'));
        let body = reqwest_get_health(&health_url).await?;
        if body.contains("ok") {
            return Ok(());
        }
        anyhow::bail!("health check failed: {body}");
    }

    let mut config = ControlConfig::load(cli.config.as_deref()).context("load config")?;
    // Force JSON logs in production-style defaults unless explicitly overridden via file/env.
    init_telemetry(&config.telemetry);

    let db = Db::connect_with_retry(&config.database_url, 30)
        .await
        .context("connect database")?;

    match cli.command {
        Some(Command::BootstrapAdmin { email, password }) => {
            let user = users::bootstrap_admin(&db, &email, &password).await?;
            tracing::info!(user_id = %user.id, email = %user.email, "admin bootstrapped");
            println!("bootstrapped admin {}", user.email);
            Ok(())
        }
        Some(Command::Healthcheck { .. }) => unreachable!("handled above"),
        None => {
            maybe_auto_bootstrap(&db, &config).await?;

            // Default static dir in container if unset but present.
            if config.static_dir.is_none() {
                let candidate = PathBuf::from("/app/static");
                if candidate.is_dir() {
                    config.static_dir = Some(candidate);
                }
            }
            if config.node_binaries_dir.is_none() {
                let candidate = PathBuf::from("/app/binaries");
                if candidate.is_dir() {
                    config.node_binaries_dir = Some(candidate);
                }
            }

            let runtime = NodeRuntimeState::default();
            let grpc_svc = NodeGrpcService::new(db.clone(), runtime.clone());
            let grpc_addr = bind_addr(&config.grpc_bind);
            let state = build_state(config.clone(), db.clone())?;
            let binaries_dir = state
                .config
                .node_binaries_dir
                .clone()
                .unwrap_or_else(|| PathBuf::from("/app/binaries"));
            let public_grpc = state
                .config
                .public_grpc
                .clone()
                .unwrap_or_else(|| format!("http://{}", state.config.grpc_bind));
            vpnc_control::provisioning::spawn_worker(
                state.provisioning.clone(),
                state.secret_store.clone(),
                binaries_dir,
                public_grpc,
            );
            let app = build_app(state);
            let http_addr = bind_addr(&config.http_bind);

            tracing::info!(%http_addr, "vpnc-control HTTP listening");
            tracing::info!(%grpc_addr, "vpnc-control gRPC listening");
            if let Some(dir) = &config.static_dir {
                tracing::info!(?dir, "serving dashboard static files");
            }

            let listener = tokio::net::TcpListener::bind(http_addr)
                .await
                .with_context(|| format!("bind {http_addr}"))?;

            let offline_db = db.clone();
            let offline_runtime = runtime.clone();
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(15));
                loop {
                    ticker.tick().await;
                    mark_offline_nodes(&offline_db, &offline_runtime, 45).await;
                }
            });

            let grpc = Server::builder()
                .add_service(NodeServiceServer::new(grpc_svc))
                .serve(grpc_addr);

            let http = axum::serve(listener, app);

            tokio::select! {
                res = http => res.context("http serve")?,
                res = grpc => res.context("grpc serve")?,
            }
            Ok(())
        }
    }
}

async fn maybe_auto_bootstrap(db: &Db, config: &ControlConfig) -> anyhow::Result<()> {
    let Some(email) = config.bootstrap_email.as_deref() else {
        return Ok(());
    };
    let Some(password) = config.bootstrap_password.as_deref() else {
        tracing::warn!("VPNC_BOOTSTRAP_EMAIL set without VPNC_BOOTSTRAP_PASSWORD; skipping");
        return Ok(());
    };
    let count = db.user_count().await?;
    if count > 0 {
        tracing::info!("users already exist; ignoring VPNC_BOOTSTRAP_*");
        return Ok(());
    }
    match users::bootstrap_admin(db, email, password).await {
        Ok(user) => {
            tracing::info!(user_id = %user.id, email = %user.email, "auto-bootstrapped admin");
            Ok(())
        }
        Err(err) => {
            tracing::error!(error = %err, "auto-bootstrap failed");
            Err(err)
        }
    }
}

async fn reqwest_get_health(url: &str) -> anyhow::Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let without_scheme = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(url);
    let (host_port, path) = match without_scheme.split_once('/') {
        Some((hp, rest)) => (hp, format!("/{rest}")),
        None => (without_scheme, "/api/health".to_string()),
    };
    let (host, port) = match host_port.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().unwrap_or(8080)),
        None => (host_port, 8080),
    };
    let mut stream = tokio::net::TcpStream::connect((host, port))
        .await
        .with_context(|| format!("connect {host}:{port}"))?;
    let req = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).await?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}
