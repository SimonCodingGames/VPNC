//! Local release store for `vpnc-node` binaries.

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Resolves a node binary for a remote architecture string (`x86_64` / `amd64` / `aarch64` / `arm64`).
pub fn resolve_node_binary(binaries_dir: &Path, arch: &str) -> Result<(PathBuf, String)> {
    let folder = match arch {
        "x86_64" | "amd64" | "x86-64" => "linux-amd64",
        "aarch64" | "arm64" => "linux-arm64",
        other => bail!("unsupported architecture for node binary: {other}"),
    };
    let path = binaries_dir.join(folder).join("vpnc-node");
    if !path.is_file() {
        bail!(
            "node binary missing at {} (set VPNC_NODE_BINARIES_DIR and ship linux-amd64/vpnc-node)",
            path.display()
        );
    }
    let bytes = std::fs::read(&path).with_context(|| format!("read {}", path.display()))?;
    let digest = hex::encode(Sha256::digest(&bytes));
    Ok((path, digest))
}

/// Reads the binary bytes for upload.
pub fn read_node_binary(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).with_context(|| format!("read {}", path.display()))
}
