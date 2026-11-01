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

## 3. Umgebungsvariablen

Postgres startet mit der Compose-Datei automatisch (User/DB/Passwort sind
**fest in** `docker-compose.coolify.yml`, Port nicht öffentlich). In Coolify
brauchst du nur:

| Variable | Pflicht | Hinweis |
|---|---|---|
| `VPNC_MASTER_KEY` | **ja** | `openssl rand -hex 32` — einmalig erzeugen und sichern |
| `VPNC_PUBLIC_GRPC` | **ja** | z. B. `http://DEINE-IP:50051` (für Nodes / SSH-Install) |
| `VPNC_BOOTSTRAP_EMAIL` | empfohlen Erststart | Admin-E-Mail |
| `VPNC_BOOTSTRAP_PASSWORD` | empfohlen Erststart | min. 10 Zeichen |

Optional (Defaults ok): `VPNC_COOKIE_SECURE`, `VPNC_TRUST_FORWARDED_HEADERS`,
`VPNC_SESSION_HOURS`. Domain: Coolify setzt `SERVICE_FQDN_CONTROL_8080` →
`VPNC_PUBLIC_URL`.

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

1. **Netzwerk / Cloud-Firewall (wichtig):**
   - Control ← Node: **TCP 50051**
   - Control → Node: **SSH 22** (nur fürs Provisioning)
   - Clients → Node: **UDP 51820** (WireGuard; Security Group / Hetzner Firewall / ufw)
2. Dashboard **Server** → Tab **Per SSH installieren**
3. Host/IP, User, Passwort (oder Key), Rolle `classic` (CIDRs haben Defaults)
4. Host-Key-Fingerprint prüfen → **bestätigen** → Live-Log bis `succeeded`
5. Node erscheint **online** — Install setzt automatisch: WireGuard-Tools, sysctl-Forwarding,
   nft/ufw UDP 51820, `wg0`+NAT sobald Profile existieren

### Fallback: Join-Token manuell

Tab **Manuell / Join-Token** → Befehl auf dem Server ausführen.

Danach: **Geräte** → Gerät anlegen → **Profile** → Profil → einmalig `.conf`/QR in die
offizielle WireGuard-App. Profile enthalten bereits `MTU = 1420`, DNS und Full-Tunnel.

## Checkliste

- [ ] Compose-Datei `docker-compose.coolify.yml`
- [ ] Domain / `SERVICE_FQDN_CONTROL_8080`
- [ ] Cloud-Firewall: **UDP 51820** zum Node
- [ ] `SERVICE_PASSWORD_POSTGRES` + `VPNC_MASTER_KEY`
- [ ] Bootstrap-Admin gesetzt oder CLI ausgeführt
- [ ] Login unter derselben Domain wie `/api/health`
- [ ] Port 50051 erreichbar, Join-Token getestet
- [ ] Backup von DB + Master-Key
