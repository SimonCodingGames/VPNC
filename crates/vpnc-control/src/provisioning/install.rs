//! Remote installation script and orchestration (fixed scripts only).

use super::binaries::{read_node_binary, resolve_node_binary};
use super::detect::{detect_public_ips, merge_with_ssh_host, DetectedPublicIps};
use super::jobs::SshSecrets;
use super::preflight::PreflightReport;
use super::ssh::SshSession;
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

const INSTALL_SH: &str = r#"#!/bin/bash
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
if command -v apt-get >/dev/null 2>&1; then
  apt-get update -qq
  apt-get install -y -qq wireguard-tools iproute2 nftables
fi
modprobe wireguard 2>/dev/null || true
install -d -m 755 /usr/local/bin /etc/vpnc /etc/sysctl.d /etc/modules-load.d
cat >/etc/modules-load.d/vpnc-wireguard.conf <<'EOF'
wireguard
EOF
cat >/etc/sysctl.d/99-vpnc.conf <<'EOF'
net.ipv4.ip_forward=1
net.ipv6.conf.all.forwarding=1
net.ipv4.conf.all.rp_filter=2
net.ipv4.conf.default.rp_filter=2
EOF
sysctl --system >/dev/null 2>&1 || true
# Open WireGuard UDP if a host firewall is active (cloud SG still needed).
if command -v ufw >/dev/null 2>&1; then
  ufw allow 51820/udp || true
fi
if command -v firewall-cmd >/dev/null 2>&1; then
  firewall-cmd --permanent --add-port=51820/udp || true
  firewall-cmd --reload || true
fi
if command -v nft >/dev/null 2>&1; then
  nft delete table inet vpnc_input 2>/dev/null || true
  nft -f - <<'EOF'
table inet vpnc_input {
  chain input {
    type filter hook input priority filter; policy accept;
    udp dport 51820 accept
  }
}
EOF
fi
install -m 755 /tmp/vpnc-node-new /usr/local/bin/vpnc-node
install -m 755 /tmp/vpnc-node-run.sh /usr/local/bin/vpnc-node-run
install -m 600 /tmp/vpnc-node.toml /etc/vpnc/node.toml
install -m 644 /tmp/vpnc-node.service /etc/systemd/system/vpnc-node.service
systemctl daemon-reload
systemctl enable --now vpnc-node.service
"#;

const ROLLBACK_SH: &str = r#"#!/bin/bash
set -euo pipefail
systemctl disable --now vpnc-node.service 2>/dev/null || true
rm -f /usr/local/bin/vpnc-node /usr/local/bin/vpnc-node-run /etc/vpnc/node.toml /etc/systemd/system/vpnc-node.service
systemctl daemon-reload 2>/dev/null || true
rm -f /tmp/vpnc-node-new /tmp/vpnc-node.toml /tmp/vpnc-node.service /tmp/vpnc-node-run.sh /tmp/vpnc-install.sh /tmp/vpnc-rollback.sh
"#;

const SERVICE_UNIT: &str =
    "[Unit]\nDescription=VPNC node\nAfter=network-online.target\nWants=network-online.target\n\n\
[Service]\nType=simple\nExecStart=/usr/local/bin/vpnc-node-run\nRestart=on-failure\nRestartSec=5\n\
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW\nCapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW\n\
# Root so nft/ip/wg can manage interfaces without capability gaps.\n\n\
[Install]\nWantedBy=multi-user.target\n";

/// Parameters for a remote install (no SSH secrets).
pub struct InstallParams<'a> {
    /// Path to binaries dir.
    pub binaries_dir: &'a Path,
    /// Control gRPC URL for the node.
    pub control_grpc: &'a str,
    /// One-time join token (plaintext, once).
    pub join_token: &'a str,
    /// Node role.
    pub role: &'a str,
    /// Optional node name.
    pub node_name: &'a str,
    /// Expected host-key fingerprint.
    pub host_key_fp: &'a str,
}

/// Cloneable log sink.
#[derive(Clone)]
pub struct ArcLog(pub Arc<dyn Fn(&str) + Send + Sync>);

impl ArcLog {
    /// Emits a line.
    pub fn line(&self, msg: &str) {
        (self.0)(msg);
    }
}

/// Runs preflight over an open session.
pub async fn run_preflight(session: &mut SshSession) -> Result<PreflightReport> {
    let mut report = PreflightReport::default();

    let (code, out, _) = session
        .exec("cat /etc/os-release 2>/dev/null || true")
        .await?;
    if code == 0 {
        for line in out.lines() {
            if let Some(id) = line.strip_prefix("ID=") {
                let id = id.trim_matches('"');
                report.distro = Some(id.to_string());
                if id != "debian" && id != "ubuntu" {
                    return Ok(super::preflight::unsupported_distro(id));
                }
            }
        }
    }
    if report.distro.is_none() {
        report
            .issues
            .push("could not detect distribution (need debian/ubuntu)".into());
    }

    let (_, arch_out, _) = session.exec("uname -m").await?;
    report.arch = Some(arch_out.trim().to_string());

    let (_, id_out, _) = session.exec("id -u").await?;
    let uid = id_out.trim();
    report.privilege_ok = uid == "0";
    if !report.privilege_ok {
        let (scode, _, _) = session.exec("sudo -n true 2>/dev/null").await.unwrap_or((
            1,
            String::new(),
            String::new(),
        ));
        report.privilege_ok = scode == 0;
        if !report.privilege_ok {
            report
                .issues
                .push("root or passwordless sudo required".into());
        }
    }

    // Soft check: module may load later; do not block install.
    report.wireguard_ok = true;
    report.ports_ok = true;

    let detected = detect_public_ips(session).await.unwrap_or_default();
    report.public_ip = detected.ipv4;
    report.public_ipv6 = detected.ipv6;

    Ok(report)
}

/// Result of a successful remote install (for control-plane bookkeeping).
#[derive(Debug, Clone, Default)]
pub struct InstallOutcome {
    /// Public IPs observed on the host (plus SSH host fallback).
    pub public_ips: DetectedPublicIps,
}

/// Full install over SSH.
pub async fn install_node(
    host: &str,
    port: u16,
    user: &str,
    secrets: &SshSecrets,
    params: &InstallParams<'_>,
    log: &ArcLog,
) -> Result<InstallOutcome> {
    log.line("connecting via SSH…");
    let (mut session, _fp) =
        SshSession::connect(host, port, user, secrets, Some(params.host_key_fp)).await?;

    log.line("running preflight…");
    let report = run_preflight(&mut session).await?;
    if !report.ok() {
        bail!("preflight failed: {}", report.issues.join("; "));
    }
    let arch = report.arch.as_deref().unwrap_or("x86_64");
    let public_ips = merge_with_ssh_host(
        DetectedPublicIps {
            ipv4: report.public_ip.clone(),
            ipv6: report.public_ipv6.clone(),
        },
        host,
    );
    log.line(&format!(
        "preflight ok (distro={:?}, arch={arch}, ipv4={:?}, ipv6={:?})",
        report.distro, public_ips.ipv4, public_ips.ipv6
    ));

    let (bin_path, sha) = resolve_node_binary(params.binaries_dir, arch)?;
    log.line(&format!("uploading vpnc-node ({})", bin_path.display()));
    let bytes = read_node_binary(&bin_path)?;
    session.upload("/tmp/vpnc-node-new", &bytes).await?;

    let (c, out, _) = session
        .exec("sha256sum /tmp/vpnc-node-new | awk '{print $1}'")
        .await?;
    if c != 0 || out.trim() != sha {
        let _ = rollback(&mut session).await;
        bail!("remote checksum mismatch");
    }
    log.line("checksum ok");

    let toml = format!(
        "control = \"{}\"\njoin_token = \"{}\"\nrole = \"{}\"\nname = \"{}\"\n",
        sanitize_url(params.control_grpc),
        params.join_token,
        sanitize_token(params.role),
        sanitize_name(params.node_name)
    );
    session
        .upload_str("/tmp/vpnc-node.toml", &toml)
        .await
        .context("upload node.toml")?;

    const WRAPPER: &str = "#!/bin/bash\nset -euo pipefail\n\
CONTROL=$(grep -E '^control' /etc/vpnc/node.toml | cut -d'\"' -f2)\n\
TOKEN=$(grep -E '^join_token' /etc/vpnc/node.toml | cut -d'\"' -f2)\n\
ROLE=$(grep -E '^role' /etc/vpnc/node.toml | cut -d'\"' -f2)\n\
NAME=$(grep -E '^name' /etc/vpnc/node.toml | cut -d'\"' -f2)\n\
exec /usr/local/bin/vpnc-node run --control \"$CONTROL\" --join-token \"$TOKEN\" --role \"$ROLE\" --name \"$NAME\"\n";
    session.upload_str("/tmp/vpnc-node-run.sh", WRAPPER).await?;
    session
        .upload_str("/tmp/vpnc-node.service", SERVICE_UNIT)
        .await?;
    session
        .upload_str("/tmp/vpnc-install.sh", INSTALL_SH)
        .await?;
    session
        .upload_str("/tmp/vpnc-rollback.sh", ROLLBACK_SH)
        .await?;

    log.line("running install script…");
    let (code, out, err) = session.exec("bash /tmp/vpnc-install.sh").await?;
    if code != 0 {
        let _ = rollback(&mut session).await;
        bail!("install failed (exit {code}): {err} {out}");
    }
    log.line("systemd unit started");
    Ok(InstallOutcome { public_ips })
}

async fn rollback(session: &mut SshSession) -> Result<()> {
    let _ = session
        .upload_str("/tmp/vpnc-rollback.sh", ROLLBACK_SH)
        .await;
    let _ = session.exec("bash /tmp/vpnc-rollback.sh").await;
    Ok(())
}

fn sanitize_url(url: &str) -> String {
    url.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '/' | '.' | '-' | '_' | '@'))
        .collect()
}

fn sanitize_token(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .collect()
}

fn sanitize_name(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .take(64)
        .collect()
}

/// Polls until `check` returns true or timeout.
pub async fn wait_until(
    mut check: impl FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>>,
    timeout: Duration,
    log: &ArcLog,
) -> Result<()> {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if check().await {
            log.line("node registered and online");
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    bail!("timeout waiting for node registration");
}
