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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = ControlConfig::load(cli.config.as_deref()).context("load config")?;
    init_telemetry(&config.telemetry);

    let db = Db::connect(&config.database_url)
        .await
        .context("connect database")?;

    match cli.command {
        Some(Command::BootstrapAdmin { email, password }) => {
            let user = users::bootstrap_admin(&db, &email, &password).await?;
            tracing::info!(user_id = %user.id, email = %user.email, "admin bootstrapped");
            println!("bootstrapped admin {}", user.email);
            Ok(())
        }
        None => {
            let runtime = NodeRuntimeState::default();
            let grpc_svc = NodeGrpcService::new(db.clone(), runtime.clone());
            let grpc_addr = bind_addr(&config.grpc_bind);
            let state = build_state(config.clone(), db.clone())?;
            let app = build_app(state);
            let http_addr = bind_addr(&config.http_bind);

            tracing::info!(%http_addr, "vpnc-control HTTP listening");
            tracing::info!(%grpc_addr, "vpnc-control gRPC listening");

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
