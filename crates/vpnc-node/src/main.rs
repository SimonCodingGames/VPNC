//! VPNC node daemon binary.

mod classic;

mod pb {
    pub mod node {
        tonic::include_proto!("vpnc.node.v1");
    }
}

use anyhow::Context;
use classic::nat::{ensure_ip_forward, ensure_masquerade};
use classic::wireguard::{apply_interface, DesiredInterface};
use clap::{Parser, Subcommand};
use pb::node::node_service_client::NodeServiceClient;
use pb::node::{HealthReport, RegisterRequest, StreamConfigRequest};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, info, warn};
use vpnc_common::ClassicWgConfig;

#[derive(Debug, Parser)]
#[command(name = "vpnc-node", about = "VPNC node daemon")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Register with control using a join token, then stream config / health.
    Run {
        /// Control gRPC endpoint, e.g. http://127.0.0.1:50051
        #[arg(long, default_value = "http://127.0.0.1:50051")]
        control: String,
        /// One-time join token.
        #[arg(long, env = "VPNC_JOIN_TOKEN")]
        join_token: String,
        /// Optional node name.
        #[arg(long, default_value = "")]
        name: String,
        /// Node role.
        #[arg(long, default_value = "classic")]
        role: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Run {
            control,
            join_token,
            name,
            role,
        }) => run(&control, &join_token, &name, &role).await,
        None => {
            println!("vpnc-node: use `vpnc-node run --join-token ...`");
            Ok(())
        }
    }
}

async fn run(control: &str, join_token: &str, name: &str, role: &str) -> anyhow::Result<()> {
    let mut client = NodeServiceClient::connect(control.to_string())
        .await
        .with_context(|| format!("connect {control}"))?;

    let reg = client
        .register(RegisterRequest {
            join_token: join_token.to_string(),
            node_name: name.to_string(),
            role: role.to_string(),
            public_key_pem: String::new(),
        })
        .await?
        .into_inner();
    info!(node_id = %reg.node_id, "registered");

    let mut version = 0u64;
    loop {
        match client
            .stream_config(StreamConfigRequest {
                node_id: reg.node_id.clone(),
                last_applied_version: version,
            })
            .await
        {
            Ok(stream) => {
                let mut stream = stream.into_inner();
                while let Some(update) = stream.message().await? {
                    info!(
                        version = update.version,
                        full = update.full_snapshot,
                        payload_len = update.payload.len(),
                        "config update"
                    );
                    match apply_config_payload(&update.payload) {
                        Ok(()) => {
                            version = update.version;
                            let _ = client
                                .ack_config(pb::node::AckConfigRequest {
                                    node_id: reg.node_id.clone(),
                                    applied_version: version,
                                })
                                .await;
                        }
                        Err(err) => {
                            error!(error = %err, "failed to apply config");
                        }
                    }
                }
            }
            Err(err) => {
                warn!(error = %err, "config stream error; reconnecting");
                tokio::time::sleep(jitter_backoff()).await;
            }
        }

        let _ = client
            .report_health(HealthReport {
                node_id: reg.node_id.clone(),
                unix_ts: chrono_like_now(),
                metrics: HashMap::new(),
            })
            .await;
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

fn apply_config_payload(payload: &[u8]) -> anyhow::Result<()> {
    let cfg: ClassicWgConfig =
        serde_json::from_slice(payload).context("parse classic config JSON")?;
    if cfg.role != "classic" {
        warn!(role = %cfg.role, "ignoring non-classic config");
        return Ok(());
    }
    let desired = DesiredInterface::from_classic(&cfg);
    apply_interface(&desired)?;

    ensure_ip_forward()?;
    let mut cidrs: Vec<&str> = Vec::new();
    if let Some(c) = cfg.tunnel_cidr_v4.as_deref() {
        cidrs.push(c);
    }
    if let Some(c) = cfg.tunnel_cidr_v6.as_deref() {
        cidrs.push(c);
    }
    if !cidrs.is_empty() {
        ensure_masquerade(&cfg.interface, &cidrs)?;
    }
    Ok(())
}

fn jitter_backoff() -> Duration {
    use rand::Rng;
    let ms = rand::thread_rng().gen_range(500..2500);
    Duration::from_millis(ms)
}

fn chrono_like_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
