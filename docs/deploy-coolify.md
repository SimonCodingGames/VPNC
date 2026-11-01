# VPNC auf Coolify deployen

Schritt für Schritt: Repository anbinden, Docker Compose bauen, Domain setzen,
Pflicht-Umgebungsvariablen, Auto-Deploy und ersten Node verbinden.

## Voraussetzungen

- Coolify-Instanz mit Traefik
- GitHub-Repository (dieses Monorepo)
- Domain für das Dashboard/API (HTTPS)
- Firewall: UDP `51820` auf Nodes, TCP `50051` zum Control für gRPC

## 1. Repo in Coolify verbinden

1. Coolify → **Sources** → GitHub App verbinden und das VPNC-Repo freigeben
2. **New Resource** → **Docker Compose**
3. Repository und Branch (`main`/`master`) wählen
4. Compose-Datei: `docker-compose.coolify.yml`
5. Build Pack bleibt **Docker Compose** (Build im Repo-Root-`Dockerfile`)

## 2. Domain

Unter dem Service `control` die Domain setzen. Coolify setzt dafür typischerweise
`SERVICE_FQDN_CONTROL_8080` (Magic Variable). **Keine** `ports:` für HTTP – Traefik
routet auf Container-Port `8080`.

Dashboard und API laufen unter **derselben Domain** (Static + `/api/*`).

## 3. Pflicht-Umgebungsvariablen

| Variable | Pflicht | Hinweis |
|---|---|---|
| `SERVICE_PASSWORD_POSTGRES` | ja | Coolify Magic; Passwort für Postgres |
| `VPNC_MASTER_KEY` | ja | `openssl rand -hex 32` (64 Hex-Zeichen) **oder** Base64 von genau 32 Bytes. `SERVICE_BASE64_64_*` ist **nicht** geeignet (ergibt ~48 Bytes). |
| `VPNC_BOOTSTRAP_EMAIL` | empfohlen beim Erststart | Admin-E-Mail, nur wenn noch keine User existieren |
| `VPNC_BOOTSTRAP_PASSWORD` | empfohlen beim Erststart | min. 10 Zeichen; nach erstem Login entfernen/rotieren |
| `VPNC_COOKIE_SECURE` | nein (Default `true`) | hinter HTTPS belassen |
| `VPNC_TRUST_FORWARDED_HEADERS` | nein (Default `true`) | Traefik setzt `X-Forwarded-*` |
| `VPNC_PUBLIC_GRPC` | **ja** (Compose) | z. B. `http://DEINE-IP:50051` – Nodes und SSH-Install nutzen diese Adresse |
| `VPNC_SESSION_HOURS` | nein | Default `24` |

`SERVICE_FQDN_CONTROL_8080` wird als `VPNC_PUBLIC_URL` durchgereicht (Install-/Doku-Links).

## 4. Ports und gRPC

- **HTTP/HTTPS:** nur über Traefik / `SERVICE_FQDN_CONTROL_8080`
- **gRPC `50051`:** in der Compose-Datei als `ports: ["50051:50051"]` veröffentlicht, damit Nodes **direkt** verbinden

### Alternative: gRPC über Traefik (h2c)

Möglich mit Traefik-Router `proto=h2c` auf Port 50051. Nachteile:

- zusätzliche Traefik-Konfiguration und Timeouts
- Nodes müssen die Traefik-URL nutzen (TLS/SNI)
- Debugging schwieriger als freigegebener Host-Port

Für den Start ist der veröffentlichte Port `50051` die einfachere Variante.

## 5. Auto-Deploy

### Variante A: Coolify Auto-Deploy (GitHub App)

In der Resource **Auto Deploy** bei Push auf den Produktionsbranch aktivieren.

### Variante B: nur getestete Commits (empfohlen)

CI (`.github/workflows/ci.yml`) baut und testet; bei Push auf `main`/`master` optional:

1. Image nach GHCR pushen
2. Coolify-Deploy-Webhook aufrufen

GitHub Secrets:

- `COOLIFY_WEBHOOK_URL` – Deploy-Webhook der Coolify-Resource
- `COOLIFY_TOKEN` – optional, falls der Webhook Auth verlangt

## 6. Erster Login

1. Deployment abwarten, Healthcheck grün
2. `https://<deine-domain>/` öffnen → Login
3. Mit `VPNC_BOOTSTRAP_*` (oder manuell `bootstrap-admin`) anmelden
4. Bootstrap-Passwort danach ändern bzw. Env entfernen (weitere Starts ignorieren `VPNC_BOOTSTRAP_*`, wenn User existieren)

## 7. Backup

Vor Produktionsbetrieb sichern:

- Postgres-Volume / `pg_dump`
- **`VPNC_MASTER_KEY`** (ohne Key sind verschlüsselte Secrets verloren)

Siehe auch [backup-restore.md](backup-restore.md).

## 8. Node verbinden

### Primär: Dashboard-Assistent (kein Login auf dem Server)

1. Firewall: Control erreichbar auf **TCP 50051** vom Node-Host; Control muss den Node auf **SSH (22)** erreichen
2. Dashboard **Server** → Tab **Per SSH installieren**
3. Host/IP, User, Passwort (oder Key), Rolle `classic`, CIDR z. B. `10.66.0.0/24`
4. Host-Key-Fingerprint prüfen → **bestätigen** → Live-Log bis `succeeded`
5. Node erscheint **online**

### Fallback: Join-Token manuell

Tab **Manuell / Join-Token** → Befehl auf dem Server ausführen.

Danach: Gerät → Profil → einmalig `.conf`/QR → WireGuard-App.

## Checkliste

- [ ] Compose-Datei `docker-compose.coolify.yml`
- [ ] Domain / `SERVICE_FQDN_CONTROL_8080`
- [ ] `SERVICE_PASSWORD_POSTGRES` + `VPNC_MASTER_KEY`
- [ ] Bootstrap-Admin gesetzt oder CLI ausgeführt
- [ ] Login unter derselben Domain wie `/api/health`
- [ ] Port 50051 erreichbar, Join-Token getestet
- [ ] Backup von DB + Master-Key
