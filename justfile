set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]
set dotenv-load := false

# Build all Rust workspace members
build:
    cargo build --workspace

# Run Rust tests
test:
    cargo test --workspace

# Format Rust sources
fmt:
    cargo fmt --all

# Lint Rust (clippy) and frontend
lint: lint-rust lint-frontend

lint-rust:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings

lint-frontend:
    pnpm lint
    pnpm check

# Control plane (placeholder until step 3)
dev-control:
    cargo run -p vpnc-control

# Dashboard (placeholder until step 10)
dev-dashboard:
    pnpm --filter @vpnc/dashboard dev
