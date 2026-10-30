# VPNC – Installationsanleitung

Diese Anleitung bringt eine frische VPNC-Installation von Null bis zum ersten
WireGuard-Profil. Empfohlen für den Einstieg: **Docker Compose** (Abschnitt 2).
Für Entwicklung oder Bare-Metal siehe Abschnitt 3 und 4.

> **Hinweis zum aktuellen Stand:** Control-Plane (API, Auth, Profile, gRPC) und
> Dashboard-Login sind vorhanden. Server-Provisioning per SSH, vollständige
> Verwaltungsseiten und Kernel-WireGuard auf Nodes folgen den nächsten
> Roadmap-Schritten. Die UI-Vorschaubilder unten zeigen das **Zielbild**.

---

## Inhaltsverzeichnis

1. [Voraussetzungen](#1-voraussetzungen)
2. [Schnellstart mit Docker Compose](#2-schnellstart-mit-docker-compose)
3. [Installation aus dem Quellcode (Entwicklung)](#3-installation-aus-dem-quellcode-entwicklung)
4. [Bare-Metal / systemd (Produktionsskizze)](#4-bare-metal--systemd-produktionsskizze)
5. [Erster Admin und Login](#5-erster-admin-und-login)
6. [Server hinzufügen](#6-server-hinzufügen)
7. [Gerät und WireGuard-Profil](#7-gerät-und-wireguard-profil)
8. [Dashboard-Vorschau](#8-dashboard-vorschau)
9. [Konfiguration und Umgebungsvariablen](#9-konfiguration-und-umgebungsvariablen)
10. [Update, Backup, Troubleshooting](#10-update-backup-troubleshooting)

---

## 1. Voraussetzungen

### Produktion / Compose

| Komponente | Minimum |
|---|---|
| Host-OS | Linux x86_64 oder aarch64 (Debian/Ubuntu empfohlen) |
| Docker | 24+ mit Compose-Plugin |
| RAM | 2 GB (Control + Postgres) |
| Ports | `8080` (HTTP/Dashboard), `50051` (gRPC), `5432` nur intern |
| DNS | Optional GeoDNS für VPNC-Tunnel (siehe [geodns.md](geodns.md)) |

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
(oder per `.env`-Datei / Compose-Override).

### 2.3 Stack starten

```bash
docker compose up -d postgres control
docker compose ps
```

Warten, bis Postgres healthy und Control läuft:

```bash
docker compose logs -f control
```

Erreichbarkeit prüfen:

```bash
curl -s http://127.0.0.1:8080/api/health
# {"status":"ok"}
```

### 2.4 Ersten Admin anlegen

```bash
docker compose exec control vpnc-control bootstrap-admin \
  --email admin@example.com \
  --password 'mindestens-10-zeichen'
```

Passwort idealerweise über Env:

```bash
docker compose exec -e VPNC_BOOTSTRAP_PASSWORD='mindestens-10-zeichen' \
  control vpnc-control bootstrap-admin --email admin@example.com
```

`bootstrap-admin` schlägt fehl, wenn bereits Benutzer existieren (Absicht).

### 2.5 Dashboard öffnen

Browser: **http://127.0.0.1:8080** (sobald Control das Static-Dashboard ausliefert)
bzw. parallel Dev-Dashboard: siehe Abschnitt 3.

Login mit der Admin-E-Mail und dem gesetzten Passwort.

### 2.6 Optional: Node-Container

```bash
# Join-Token zuvor per API / später Dashboard erzeugen
export VPNC_JOIN_TOKEN='...'
docker compose --profile node up -d node
```

Der Node-Profil-Service braucht `NET_ADMIN` und `/dev/net/tun` (nur Linux).

---

## 3. Installation aus dem Quellcode (Entwicklung)

### 3.1 Abhängigkeiten

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Node + pnpm
npm install -g pnpm@10

# optional
cargo install just
```

Windows: [Build Tools for Visual Studio](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
mit Workload „Desktopentwicklung mit C++“ installieren.

### 3.2 Bauen und testen

```bash
pnpm install
cargo build --workspace
cargo test --workspace
# oder:
just build && just test && just lint
```

### 3.3 Control lokal mit SQLite

```powershell
# PowerShell (Windows)
$env:VPNC_DATABASE_URL = "sqlite://./vpnc-dev.db?mode=rwc"
$env:VPNC_MASTER_KEY   = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
$env:VPNC_HTTP_BIND    = "127.0.0.1:8080"
$env:VPNC_GRPC_BIND    = "127.0.0.1:50051"
$env:VPNC_COOKIE_SECURE = "false"

cargo run -p vpnc-control -- bootstrap-admin `
  --email admin@example.com --password "dev-password-ok"

cargo run -p vpnc-control
```

```bash
# Bash
export VPNC_DATABASE_URL="sqlite://./vpnc-dev.db?mode=rwc"
export VPNC_MASTER_KEY="$(openssl rand -hex 32)"
export VPNC_HTTP_BIND="127.0.0.1:8080"
export VPNC_GRPC_BIND="127.0.0.1:50051"
export VPNC_COOKIE_SECURE=false

cargo run -p vpnc-control -- bootstrap-admin \
  --email admin@example.com --password 'dev-password-ok'

cargo run -p vpnc-control
```

### 3.4 Control mit lokalem PostgreSQL

```bash
createdb vpnc   # oder pgAdmin / SQL
export VPNC_DATABASE_URL="postgres://postgres:GEHEIM@localhost:5432/vpnc"
export VPNC_TEST_DATABASE_URL="$VPNC_DATABASE_URL"   # für Integrationstests
cargo run -p vpnc-control
```

### 3.5 Dashboard (Dev-Server)

```bash
# API-Basis auf lokalen Control zeigen
# Windows PowerShell:
$env:VITE_API_BASE = "http://127.0.0.1:8080"
pnpm --filter @vpnc/dashboard dev
```

Standardmäßig öffnet Vite z. B. **http://127.0.0.1:5173**. Cookies funktionieren
nur, wenn Control `VPNC_COOKIE_SECURE=false` setzt und CORS/Same-Site für Dev
passen (aktuell SameSite=Strict auf gleicher Site bzw. Proxy empfohlen).

---

## 4. Bare-Metal / systemd (Produktionsskizze)

Einheiten liegen unter `deploy/systemd/`:

- `vpnc-control.service`
- `vpnc-node.service`

Beispiel:

```bash
sudo useradd --system --home /var/lib/vpnc --shell /usr/sbin/nologin vpnc
sudo mkdir -p /etc/vpnc /var/lib/vpnc
sudo cp target/release/vpnc-control /usr/local/bin/
sudo cp deploy/systemd/vpnc-control.service /etc/systemd/system/
# Config: /etc/vpnc/control.toml + Env VPNC_*
sudo systemctl daemon-reload
sudo systemctl enable --now vpnc-control
```

Beispiel `control.toml`:

```toml
http_bind = "0.0.0.0:8080"
grpc_bind = "0.0.0.0:50051"
database_url = "postgres://vpnc@/vpnc?host=/var/run/postgresql"
session_hours = 24
cookie_secure = true

[telemetry]
filter = "info"
json = true
```

---

## 5. Erster Admin und Login

1. `bootstrap-admin` ausführen (Compose oder lokal).
2. Browser öffnen → Login-Seite.
3. E-Mail + Passwort eingeben → **Einloggen**.
4. Session-Cookie: `HttpOnly`, `SameSite=Strict`, optional `Secure`.

### So sieht der Login aus (Zielbild)

**Hell:**

![Login Hellmodus](images/vpnc-login-light.jpg)

**Dunkel:**

![Login Dunkelmodus](images/vpnc-login-dark.jpg)

Nach dem Login landet man auf der Übersicht:

![Dashboard Übersicht](images/vpnc-dashboard-home.jpg)

---

## 6. Server hinzufügen

### 6.1 Über die API (jetzt verfügbar)

Join-Token erzeugen (als eingeloggter Admin, Cookie mitsenden):

```bash
curl -s -X POST http://127.0.0.1:8080/api/nodes/join-tokens \
  -H 'content-type: application/json' \
  -b 'vpnc_session=...' \
  -d '{"role":"classic","ttl_minutes":60}'
```

Antwort enthält einmaliges `token`. Auf dem Server:

```bash
vpnc-node run \
  --control http://CONTROL_IP:50051 \
  --join-token "$TOKEN" \
  --role classic \
  --name edge-1
```

Node-CRUD und Status: `GET/POST /api/nodes`.

### 6.2 Über das Dashboard (Zielbild)

Server-Liste:

![Server-Liste](images/vpnc-nodes-list.jpg)

Assistent „Server hinzufügen“ inkl. Host-Key-Bestätigung:

![Server hinzufügen](images/vpnc-add-server-wizard.jpg)

Ablauf Zielbild:

1. Host/IP, Port, Benutzer, Passwort **oder** SSH-Key, Rolle, Region
2. Host-Key-Fingerprint anzeigen und bestätigen
3. Preflight (Distro, Architektur, sudo, WireGuard-Modul, Ports)
4. Installation mit Live-Log (SSE)
5. Node erscheint **online**

SSH-Zugangsdaten werden **nie** in der DB gespeichert (nur RAM des Jobs).

---

## 7. Gerät und WireGuard-Profil

### 7.1 Per API (jetzt)

1. Gerät anlegen: `POST /api/devices` `{"name":"Laptop"}`
2. Classic-Node anlegen (mit `tunnel_cidr_v4`, z. B. `10.66.0.0/24`)
3. Profil: `POST /api/profiles` `{"device_id":"...","node_id":"..."}`
4. Einmal-Download: `GET /api/profiles/{id}/download` → `.conf` + QR (PNG Base64)
5. In die offizielle WireGuard-App importieren

Private Keys werden **nicht** in der Datenbank gespeichert; der Download
funktioniert genau einmal.

### 7.2 Zielbild im Dashboard

![Profil mit QR-Code](images/vpnc-profile-qr.jpg)

---

## 8. Dashboard-Vorschau

| Ansicht | Datei |
|---|---|
| Login hell | [`images/vpnc-login-light.jpg`](images/vpnc-login-light.jpg) |
| Login dunkel | [`images/vpnc-login-dark.jpg`](images/vpnc-login-dark.jpg) |
| Übersicht | [`images/vpnc-dashboard-home.jpg`](images/vpnc-dashboard-home.jpg) |
| Server-Liste | [`images/vpnc-nodes-list.jpg`](images/vpnc-nodes-list.jpg) |
| Server-Assistent | [`images/vpnc-add-server-wizard.jpg`](images/vpnc-add-server-wizard.jpg) |
| Profil + QR | [`images/vpnc-profile-qr.jpg`](images/vpnc-profile-qr.jpg) |

Rollen:

- **Admin:** Benutzer, Geräte, Server, Profile, Audit, Einstellungen
- **User:** nur eigene Geräte und Profile

---

## 9. Konfiguration und Umgebungsvariablen

Präfix `VPNC_`, Nested Keys mit `__` (figment).

| Variable | Bedeutung | Beispiel |
|---|---|---|
| `VPNC_DATABASE_URL` | SQLite oder Postgres | `postgres://vpnc:vpnc@db/vpnc` |
| `VPNC_MASTER_KEY` | 32 Byte Hex/raw | `openssl rand -hex 32` |
| `VPNC_HTTP_BIND` | REST + später Static UI | `0.0.0.0:8080` |
| `VPNC_GRPC_BIND` | Node-gRPC | `0.0.0.0:50051` |
| `VPNC_COOKIE_SECURE` | Secure-Flag Session-Cookie | `true` in Prod |
| `VPNC_SESSION_HOURS` | Session-Lebensdauer | `24` |
| `VPNC_TELEMETRY__JSON` | JSON-Logs | `true` |
| `VPNC_TELEMETRY__FILTER` | tracing-Filter | `info` |
| `VPNC_BOOTSTRAP_PASSWORD` | nur für CLI bootstrap | — |
| `VPNC_JOIN_TOKEN` | Node-Registrierung | einmalig |
| `VPNC_TEST_DATABASE_URL` | Postgres-Integrationstests | — |

Datei-Fallback: `/etc/vpnc/control.toml` bzw. `--config pfad.toml`.

---

## 10. Update, Backup, Troubleshooting

### Update

Siehe [update.md](update.md). Kurz: neues Control-Image/Binary, Migrationen
laufen beim Start; Nodes später per signiertem Self-Update.

### Backup

Siehe [backup-restore.md](backup-restore.md). **Immer** auch `VPNC_MASTER_KEY`
sichern.

### Häufige Probleme

| Symptom | Prüfung |
|---|---|
| `linker link.exe not found` | VS Build Tools C++ |
| Login 401 nach Dev-Reload | Cookie Secure/`SameSite`, API-Host |
| `bootstrap refused: users already exist` | DB zurücksetzen oder Admin-Passwort-Reset (noch manuell) |
| Postgres-Test skipped | `VPNC_TEST_DATABASE_URL` setzen |
| Node registriert nicht | Join-Token Ablauf/Hash, gRPC-Port, Firewall |
| WireGuard verbindet nicht | Endpoint, Server-Public-Key, UDP 51820, NAT |

Health:

```bash
curl -s http://127.0.0.1:8080/api/health
curl -s http://127.0.0.1:8080/api/openapi.json | head
```

---

## Checkliste „frisch installiert“

- [ ] Postgres (oder SQLite Dev) erreichbar
- [ ] `VPNC_MASTER_KEY` gesetzt und gesichert
- [ ] `vpnc-control` läuft, `/api/health` = ok
- [ ] Admin per `bootstrap-admin` angelegt
- [ ] Login im Browser funktioniert
- [ ] Join-Token erzeugt, Node registriert (oder Classic-Node in API)
- [ ] Gerät + WireGuard-Profil erzeugt und in der offiziellen App importiert
- [ ] Backup-Plan für DB + Master-Key dokumentiert

Weiterführend: [geodns.md](geodns.md) · [backup-restore.md](backup-restore.md) · [update.md](update.md) · ADRs in `docs/adr/`.
