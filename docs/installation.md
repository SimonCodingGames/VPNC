# VPNC – Installationsanleitung

Diese Anleitung bringt eine frische VPNC-Installation von Null bis zum ersten
WireGuard-Profil. Empfohlen für den Einstieg: **Docker Compose** (Abschnitt 2)
oder **Coolify** ([deploy-coolify.md](deploy-coolify.md)).

Control liefert API **und** Dashboard unter derselben Origin aus (Static-Dateien +
`/api/*`). Ein separater Reverse-Proxy nur für das Frontend ist nicht nötig.

---

## Inhaltsverzeichnis

1. [Voraussetzungen](#1-voraussetzungen)
2. [Schnellstart mit Docker Compose](#2-schnellstart-mit-docker-compose)
3. [Installation aus dem Quellcode (Entwicklung)](#3-installation-aus-dem-quellcode-entwicklung)
4. [Bare-Metal / systemd (Produktionsskizze)](#4-bare-metal--systemd-produktionsskizze)
5. [Erster Admin und Login](#5-erster-admin-und-login)
6. [Server hinzufügen](#6-server-hinzufügen)
7. [Gerät und WireGuard-Profil](#7-gerät-und-wireguard-profil)
8. [Dashboard](#8-dashboard)
9. [Konfiguration und Umgebungsvariablen](#9-konfiguration-und-umgebungsvariablen)
10. [Update, Backup, Troubleshooting](#10-update-backup-troubleshooting)

---

## 1. Voraussetzungen

### Produktion / Compose

| Komponente | Minimum |
|---|---|
| Host-OS | Linux x86_64 (arm64 nice-to-have) |
| Docker | 24+ mit Compose-Plugin |
| RAM | 2 GB (Control + Postgres) |
| Ports | `8080` (HTTP/Dashboard+API), `50051` (gRPC); `5432` nur intern |
| DNS | Optional GeoDNS für VPNC-Tunnel (siehe [geodns.md](geodns.md)) |

Coolify: siehe [deploy-coolify.md](deploy-coolify.md) (`docker-compose.coolify.yml`).

### Entwicklung (Windows / macOS / Linux)

| Tool | Version |
|---|---|
| Rust | stable (siehe `rust-toolchain.toml`) |
| Node.js | ≥ 20 |
| pnpm | 10.x |
| just | optional, für `just build` / `just test` |
| Visual Studio Build Tools | Windows: C++-Workload (`link.exe`) |
| PostgreSQL 16/17 | optional; sonst SQLite für Dev |

---

## 2. Schnellstart mit Docker Compose

### 2.1 Repository holen

```bash
git clone <dein-repo-url> vpnc
cd vpnc
```

### 2.2 Master-Key erzeugen

Der Master-Key verschlüsselt ruhende Secrets (Server-/Cluster-Keys). **Einmalig
sichern** – ohne ihn sind verschlüsselte Keys nicht wiederherstellbar.

```bash
# 32 Bytes als 64 Hex-Zeichen
openssl rand -hex 32
```

In `docker-compose.yml` unter `control.environment.VPNC_MASTER_KEY` einsetzen
(oder per `.env`-Datei / Compose-Override). Alternativ Base64 von genau 32 Bytes.

### 2.3 Stack starten

Das Root-`Dockerfile` baut Dashboard und `vpnc-control` in einem Image.

```bash
docker compose up -d --build postgres control
docker compose ps
```

```bash
curl -s http://127.0.0.1:8080/api/health
# {"status":"ok"}
```

### 2.4 Ersten Admin anlegen

Lokal setzt Compose optional `VPNC_BOOTSTRAP_EMAIL` / `VPNC_BOOTSTRAP_PASSWORD`
(nur wenn noch keine Benutzer existieren). Alternativ CLI:

```bash
docker compose exec control vpnc-control bootstrap-admin \
  --email admin@example.com \
  --password 'mindestens-10-zeichen'
```

### 2.5 Dashboard öffnen

Browser: **http://127.0.0.1:8080** – Login, Geräte, Profile, Server usw. laufen
auf derselben Domain wie die API (`/api/...`).

### 2.6 Optional: Node-Container

```bash
export VPNC_JOIN_TOKEN='...'
docker compose --profile node up -d node
```

Der Node braucht `NET_ADMIN` und `/dev/net/tun` (nur Linux).

---

## 3. Installation aus dem Quellcode (Entwicklung)

### 3.1 Abhängigkeiten

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
npm install -g pnpm@10
```

Windows: [Build Tools for Visual Studio](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
mit Workload „Desktopentwicklung mit C++“.

### 3.2 Bauen und testen

```bash
pnpm install
cargo build --workspace
cargo test --workspace
pnpm --filter @vpnc/dashboard build
```

### 3.3 Control lokal mit SQLite

```powershell
$env:VPNC_DATABASE_URL = "sqlite://./vpnc-dev.db?mode=rwc"
$env:VPNC_MASTER_KEY   = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
$env:VPNC_HTTP_BIND    = "127.0.0.1:8080"
$env:VPNC_GRPC_BIND    = "127.0.0.1:50051"
$env:VPNC_COOKIE_SECURE = "false"
$env:VPNC_STATIC_DIR   = "apps/dashboard/build"   # nach pnpm build

cargo run -p vpnc-control -- bootstrap-admin `
  --email admin@example.com --password "dev-password-ok"

cargo run -p vpnc-control
```

### 3.4 Dashboard-Dev-Server (optional)

Nur nötig, wenn du die UI heiß neu lädst. API weiter über Control:

```powershell
$env:VITE_API_BASE = "http://127.0.0.1:8080"
pnpm --filter @vpnc/dashboard dev
```

`VITE_API_BASE` ist optional und in Produktion leer (relative `/api/...`).
Für Dev: Control mit `VPNC_COOKIE_SECURE=false`; Session-Cookie ist `SameSite=Lax`.

---

## 4. Bare-Metal / systemd (Produktionsskizze)

Einheiten unter `deploy/systemd/`. Binary + gebautes Dashboard nach
`VPNC_STATIC_DIR` (z. B. `/var/lib/vpnc/static`) legen.

```toml
http_bind = "0.0.0.0:8080"
grpc_bind = "0.0.0.0:50051"
database_url = "postgres://vpnc@/vpnc?host=/var/run/postgresql"
session_hours = 24
cookie_secure = true
static_dir = "/var/lib/vpnc/static"
trust_forwarded_headers = true

[telemetry]
filter = "info"
json = true
```

---

## 5. Erster Admin und Login

1. Auto-Bootstrap (`VPNC_BOOTSTRAP_*`) oder `bootstrap-admin`
2. Browser → `https://<host>/login`
3. Session-Cookie: `HttpOnly`, `SameSite=Lax`, `Secure` in Produktion

Vorschaubilder: [Abschnitt 8](#8-dashboard).

---

## 6. Server hinzufügen

### 6.1 Dashboard

**Server** → anlegen (Rolle `classic`, CIDR z. B. `10.66.0.0/24`) → **Join-Token
erzeugen** → Token und fertigen `vpnc-node`-/`docker`-Befehl einmalig kopieren.

### 6.2 API

```bash
curl -s -X POST http://127.0.0.1:8080/api/nodes/join-tokens \
  -H 'content-type: application/json' \
  -b 'vpnc_session=...' \
  -d '{"role":"classic","ttl_minutes":60}'
```

Node:

```bash
vpnc-node run \
  --control http://CONTROL_IP:50051 \
  --join-token "$TOKEN" \
  --role classic \
  --name edge-1
```

---

## 7. Gerät und WireGuard-Profil

1. **Geräte** → anlegen
2. **Profile** → Gerät + Server wählen → anlegen
3. Einmal-Download: `.conf` + QR (Hinweis in der UI)
4. In die offizielle WireGuard-App importieren

Die `.conf` enthält automatisch Endpoint (öffentliche IP), `MTU = 1420`, DNS und
Full-Tunnel (`AllowedIPs = 0.0.0.0/0`). Am Node: Forwarding, NAT und UDP 51820
werden bei SSH-Install bzw. vom Daemon gesetzt — in der **Cloud-Firewall**
muss UDP **51820** zum Node trotzdem freigegeben sein.

Private Keys werden **nicht** in der DB gespeichert; der Download funktioniert
genau einmal.

---

## 8. Dashboard

Navigation: Geräte, Profile; für Admins zusätzlich Benutzer, Server, Audit,
Einstellungen. Unbekannte Pfade → echte 404; fehlende Admin-Rechte → „Keine
Berechtigung“ (kein 404).

| Ansicht | Datei |
|---|---|
| Login hell | [`images/vpnc-login-light.jpg`](images/vpnc-login-light.jpg) |
| Login dunkel | [`images/vpnc-login-dark.jpg`](images/vpnc-login-dark.jpg) |
| Übersicht | [`images/vpnc-dashboard-home.jpg`](images/vpnc-dashboard-home.jpg) |
| Server-Liste | [`images/vpnc-nodes-list.jpg`](images/vpnc-nodes-list.jpg) |
| Profil + QR | [`images/vpnc-profile-qr.jpg`](images/vpnc-profile-qr.jpg) |

---

## 9. Konfiguration und Umgebungsvariablen

Präfix `VPNC_`, Nested Keys mit `__` (figment).

| Variable | Bedeutung | Beispiel |
|---|---|---|
| `VPNC_DATABASE_URL` | SQLite oder Postgres | `postgres://vpnc:vpnc@db/vpnc` |
| `VPNC_MASTER_KEY` | 64 Hex oder Base64(32 Bytes) | `openssl rand -hex 32` |
| `VPNC_HTTP_BIND` | REST + Static UI | `0.0.0.0:8080` |
| `VPNC_GRPC_BIND` | Node-gRPC | `0.0.0.0:50051` |
| `VPNC_STATIC_DIR` | Dashboard-Build | `/app/static` |
| `VPNC_COOKIE_SECURE` | Secure-Flag Session-Cookie | `true` in Prod |
| `VPNC_TRUST_FORWARDED_HEADERS` | `X-Forwarded-*` von Proxy | `true` hinter Traefik |
| `VPNC_PUBLIC_URL` | öffentliche HTTPS-URL | `https://vpnc.example.com` |
| `VPNC_PUBLIC_GRPC` | gRPC-URL in Join-Befehlen | `http://host:50051` |
| `VPNC_SESSION_HOURS` | Session-Lebensdauer | `24` |
| `VPNC_BOOTSTRAP_EMAIL` | Auto-Admin (nur leere DB) | — |
| `VPNC_BOOTSTRAP_PASSWORD` | Auto-Admin-Passwort | — |
| `VPNC_TELEMETRY__JSON` | JSON-Logs | `true` |
| `VPNC_TELEMETRY__FILTER` | tracing-Filter | `info` |
| `VPNC_JOIN_TOKEN` | Node-Registrierung | einmalig |
| `VPNC_TEST_DATABASE_URL` | Postgres-Integrationstests | — |

Migrationen laufen beim Start; bei nicht bereiter DB wird mit Backoff erneut versucht.

---

## 10. Update, Backup, Troubleshooting

Siehe [update.md](update.md), [backup-restore.md](backup-restore.md). **Immer**
`VPNC_MASTER_KEY` sichern.

| Symptom | Prüfung |
|---|---|
| Login 401 | Cookie Secure/`SameSite`, HTTPS, `VPNC_TRUST_FORWARDED_HEADERS` |
| `bootstrap refused` / Bootstrap ignoriert | User existieren bereits |
| Dashboard 404 auf `/` | `VPNC_STATIC_DIR`, Image neu bauen |
| Node registriert nicht | Join-Token, gRPC-Port 50051, Firewall |
| WireGuard verbindet nicht | Endpoint, Public-Key, UDP 51820 |

```bash
curl -s http://127.0.0.1:8080/api/health
curl -s http://127.0.0.1:8080/api/openapi.json | head
```

Weiterführend: [deploy-coolify.md](deploy-coolify.md) · [geodns.md](geodns.md) · ADRs in `docs/adr/`.
