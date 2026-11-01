# syntax=docker/dockerfile:1

# --- Dashboard ---
FROM node:22-bookworm AS dashboard
WORKDIR /src
RUN corepack enable && corepack prepare pnpm@10.34.7 --activate
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY apps/dashboard/package.json apps/dashboard/
COPY packages/ui/package.json packages/ui/
COPY packages/api-client/package.json packages/api-client/
RUN pnpm install --frozen-lockfile
COPY apps/dashboard apps/dashboard
COPY packages packages
RUN pnpm --filter @vpnc/dashboard build

# --- Rust (cargo-chef for dependency caching) ---
# Cargo.lock needs rustc >= 1.89 (uuid 1.29). Track current stable (rust-toolchain.toml).
FROM lukemathwalker/cargo-chef:latest-rust-1.99.1-bookworm AS chef
WORKDIR /src

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS rust-builder
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release -p vpnc-control -p vpnc-node \
    && mkdir -p /out/binaries/linux-amd64 \
    && cp target/release/vpnc-control /out/vpnc-control \
    && cp target/release/vpnc-node /out/binaries/linux-amd64/vpnc-node

# --- Runtime ---
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home /app --shell /usr/sbin/nologin vpnc \
    && mkdir -p /app/static /app/binaries \
    && chown -R vpnc:vpnc /app

COPY --from=rust-builder /out/vpnc-control /usr/local/bin/vpnc-control
COPY --from=rust-builder /out/binaries /app/binaries
COPY --from=dashboard /src/apps/dashboard/build /app/static

USER vpnc
WORKDIR /app
ENV VPNC_STATIC_DIR=/app/static \
    VPNC_NODE_BINARIES_DIR=/app/binaries \
    VPNC_HTTP_BIND=0.0.0.0:8080 \
    VPNC_GRPC_BIND=0.0.0.0:50051 \
    VPNC_TELEMETRY__JSON=true \
    VPNC_TELEMETRY__FILTER=info

EXPOSE 8080 50051
HEALTHCHECK --interval=15s --timeout=5s --start-period=30s --retries=5 \
  CMD ["vpnc-control", "healthcheck", "--url", "http://127.0.0.1:8080"]

ENTRYPOINT ["vpnc-control"]
