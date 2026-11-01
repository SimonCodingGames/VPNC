# VPNC – Roadmap

Jeder Schritt wird einzeln umgesetzt. Ein Schritt ist erst fertig, wenn alle
Akzeptanzkriterien erfüllt sind.

## Schritt 1 – Workspace und Tooling
**Ziel:** Leeres, aber vollständig baubares Monorepo mit CI.
**Anlegen/Ändern:**
- `Cargo.toml` als Workspace mit allen Crates aus `README.md` anlegen, jede Crate
  mit minimalem `lib.rs`/`main.rs`, damit alles kompiliert.
- `rust-toolchain.toml`, `rustfmt.toml`, `.editorconfig`, `.gitignore`.
- `pnpm-workspace.yaml` mit `apps/dashboard`, `apps/client-desktop`, `packages/*`.
- `justfile` mit `build`, `test`, `lint`, `fmt`, `dev-control`, `dev-dashboard`.
- `.github/workflows/ci.yml`: Rust (fmt, clippy, test) und Frontend (lint, check).
**Akzeptanz:** `just build`, `just lint`, `just test` laufen lokal und in CI grün.
**Nicht hier:** Keine Fachlogik.

## Schritt 2 – vpnc-common
**Ziel:** Gemeinsame Basis für alle Rust-Binaries.
**Anlegen:**
- `config.rs`: Laden via figment aus `/etc/vpnc/<dienst>.toml` und `VPNC_*`.
- `error.rs`: gemeinsame Fehlertypen.
- `telemetry.rs`: tracing-Setup (pretty in Dev, JSON in Prod).
- `types.rs`: `NodeId`, `UserId`, `DeviceId` (UUID-Newtypes), `NodeRole`
  (`Classic`, `Ingress`, `Egress`), `ProfileKind` (`Wireguard`, `Openvpn`, `Vpnc`).
- `secret.rs`: `Secret<T>`-Wrapper, dessen `Debug`/`Display` den Inhalt maskiert.
**Akzeptanz:** Unit-Tests für Config-Laden (Datei + Env-Override) und
Secret-Maskierung.

## Schritt 3 – vpnc-control: Grundgerüst, DB, Auth
**Ziel:** Lauffähiger API-Server mit Login.
**Anlegen in `crates/vpnc-control/src/`:**
- `main.rs`: Start, Config, Migrationen ausführen, Router.
- `db/`: Pool-Setup für SQLite und Postgres.
- `api/health.rs`: `GET /api/health`.
- `api/auth.rs`: `POST /api/auth/login`, `POST /api/auth/logout`, `GET /api/auth/me`.
  Sessions als HttpOnly-, Secure-, SameSite=Strict-Cookies, serverseitig gespeichert.
- `services/crypto.rs`: Ver-/Entschlüsselung ruhender Secrets mit Master-Key.
- CLI-Unterbefehl `vpnc-control bootstrap-admin --email ...` legt ersten Admin an.
- Migration `0001_init.sql` anlegen mit: `users` (inkl. `password_hash`, `role`),
  `sessions`, `devices`, `nodes`, `profiles` (`kind` in wireguard/openvpn/vpnc),
  `audit_events`, `ingress_clusters` (inkl. `key_version`), `cluster_members`
  (inkl. `unicast_endpoint`), `egress_pools`, `pool_members`.
- Rate-Limit auf Login.
**Akzeptanz:** Integrationstest: Admin bootstrappen, einloggen, `/me` abrufen,
ausloggen. Läuft mit SQLite und Postgres.

## Schritt 4 – vpnc-control: Benutzer, Geräte, Nodes, Audit
**Ziel:** Vollständige Verwaltungs-API.
**Anlegen:**
- `api/users.rs`, `api/devices.rs`, `api/nodes.rs`, `api/audit.rs` mit CRUD.
- Autorisierungs-Middleware: Rollenprüfung, User nur eigene Ressourcen.
- Audit-Middleware für alle schreibenden Endpunkte.
- OpenAPI-Spec per utoipa unter `GET /api/openapi.json`.
- Endpunkt zum Erzeugen von Node-Join-Tokens (einmalig, Ablaufzeit, gehasht).
**Akzeptanz:** Tests für Berechtigungen (User sieht keine fremden Geräte),
Audit-Einträge werden geschrieben, OpenAPI-Spec ist valide.

## Schritt 5 – Classic-WireGuard-Profile
**Ziel:** Dashboard-fähige Erzeugung von WireGuard-Profilen.
**Anlegen:**
- `services/ipam.rs`: IP-Vergabe aus konfiguriertem CIDR pro Node, IPv4 und IPv6,
  kollisionsfrei, Freigabe beim Löschen.
- `services/wireguard.rs`: Keypairs, `.conf`-Generierung, QR-Code als PNG/SVG.
- `api/profiles.rs`: Profil anlegen, einmaliger Download (`.conf` + QR), widerrufen.
- Server-Keypair pro Classic-Node, verschlüsselt gespeichert.
**Akzeptanz:** Generierte `.conf` wird von `wg-quick` akzeptiert (Test parst die
Datei), IPAM-Tests inkl. Erschöpfung des Bereichs, Private Key wird nicht persistiert.

## Schritt 6 – gRPC: Node-Registrierung und Config-Stream
**Ziel:** Nodes melden sich an und erhalten ihre Konfiguration.
**Ändern/Anlegen:**
- `proto/node.proto` und `proto/client.proto` anlegen (Register, StreamConfig, ReportHealth,
  später ReportPeerLocation).
- `build.rs` mit tonic-build in vpnc-control und vpnc-node.
- Registrierung: Join-Token wird gegen ein dauerhaftes Node-Credential getauscht
  (Node erzeugt Keypair, Control signiert ein Client-Zertifikat, danach mTLS).
- Config-Stream mit Versionsnummer; Node bestätigt angewendete Version.
- Health-Reports in DB/Speicher, Status „online/offline“ nach Timeout.
**Akzeptanz:** Integrationstest mit Control und Node im selben Prozess:
Registrierung, Config-Update kommt an, Node geht nach Timeout offline.

## Schritt 7 – vpnc-node: Rolle `classic` für WireGuard
**Ziel:** Node setzt Classic-WireGuard-Konfiguration real um.
**Anlegen in `crates/vpnc-node/src/classic/`:**
- `wireguard.rs`: Kernel-WireGuard-Interface per Netlink verwalten (geeignete Crate
  prüfen und als ADR festhalten), Peers idempotent abgleichen (hinzufügen,
  entfernen, ändern), Fallback auf boringtun, wenn kein Kernelmodul.
- `nat.rs`: IP-Forwarding und Masquerading per nftables, eigene Tabelle `vpnc`.
- Handshake- und Traffic-Statistiken an Control melden.
**Akzeptanz:** Test in Linux-Network-Namespace (Skript in `scripts/netns-test.sh`):
Client-Namespace verbindet mit generiertem Profil und erreicht ein Ziel dahinter.

## Schritt 8 – Node-Provisioning per SSH (Backend)
**Ziel:** Ein Admin gibt IP und Zugangsdaten ein, der Server wird vollautomatisch
Teil des Systems. **Primärpfad** ist das Dashboard (kein SSH-Login durch den
Admin auf dem Zielserver). Manueller Join-Token/`curl | sh` ist nur Fallback.
**Anlegen in `crates/vpnc-control/src/provisioning/`:**
- `jobs.rs`: Job-Queue auf Basis der Tabelle `provisioning_jobs`
  (Zustände: `pending`, `awaiting_hostkey`, `running`, `succeeded`, `failed`,
  `cancelled`), abarbeitbar von jeder Control-Instanz.
- `ssh.rs`: Verbindung mit russh, Passwort- oder Key-Auth, sudo-Unterstützung,
  Host-Key-Prüfung gegen bestätigten Fingerprint.
- `preflight.rs`: prüft Distribution (zuerst Debian/Ubuntu, Struktur offen für
  weitere), Architektur (x86_64, aarch64), root/sudo, WireGuard-Kernelmodul,
  freie Ports, öffentliche IP, Uhrzeit-Abweichung. Ergebnis als strukturierter
  Bericht.
- `install.rs`: lädt das passende signierte `vpnc-node`-Binary aus dem
  Release-Store von vpnc-control hoch, prüft Checksumme auf dem Ziel, schreibt
  `/etc/vpnc/node.toml` mit einmaligem Join-Token, installiert systemd-Unit,
  startet den Dienst, wartet auf erfolgreiche gRPC-Registrierung (Timeout).
- Bei Fehlern: Rollback (Dienst stoppen, Dateien entfernen) und verständliche
  Fehlermeldung.
- `api/provisioning.rs`: Job anlegen, Host-Key bestätigen, Job abbrechen,
  Live-Log per SSE (`GET /api/provisioning/{id}/events`), Liste paginiert.
- Bulk: mehrere Hosts in einem Request (CSV-Import später im Dashboard),
  parallel mit konfigurierbarem Limit.
- Einzeiler-Alternative: `GET /api/nodes/install-command` liefert
  `curl ... | sh`-Befehl mit einmaligem, kurzlebigem Join-Token.
- `vpnc-node`: Unterbefehl `self-update` (signiertes Binary von Control holen,
  Signatur prüfen, atomar ersetzen, Neustart) und `uninstall`, beide über den
  Config-Stream auslösbar.
- Migration `0002_provisioning.sql` (Tabellen `provisioning_jobs`, `regions`,
  Spalten `region_id` und `ssh_host_key` in `nodes`).
**Akzeptanz:** Integrationstest gegen einen SSH-Testcontainer: Job mit Passwort
läuft durch, Node registriert sich, Passwort taucht weder in DB, Logs noch
  API-Antworten auf (Test durchsucht alle drei). Falscher Host-Key bricht ab.
Fehlgeschlagene Installation hinterlässt keine Reste.

## Schritt 9 – OpenVPN Classic
**Ziel:** OpenVPN-Server und `.ovpn`-Profile.
**Anlegen:**
- `vpnc-control/services/pki.rs`: eigene CA pro Installation (rcgen),
  Client-Zertifikate, CRL bei Widerruf.
- `.ovpn`-Generierung mit eingebetteten Zertifikaten.
- `vpnc-node/classic/openvpn.rs`: OpenVPN-2.6-Prozess verwalten, Config schreiben,
  Status über Management-Interface lesen.
**Akzeptanz:** netns-Test mit OpenVPN-Client verbindet erfolgreich; widerrufenes
Zertifikat wird abgelehnt.

## Schritt 10 – Dashboard: Grundgerüst
**Ziel:** Login und Navigation.
**Anlegen in `apps/dashboard/`:**
- SvelteKit mit adapter-static, Tailwind, TypeScript strict.
- `packages/api-client` aus `/api/openapi.json` generieren (Skript in `justfile`).
- Login-Seite, geschützte Layout-Route, Navigation nach Rolle.
- Hell/Dunkel-Modus, responsive.
- vpnc-control liefert den statischen Build unter `/` aus.
**Akzeptanz:** Login/Logout funktioniert gegen lokalen Control-Server;
`pnpm check` grün.

## Schritt 11 – Dashboard: Verwaltungsseiten
**Ziel:** Alle Classic-Funktionen per Oberfläche bedienbar.
**Anlegen unter `src/routes/`:**
- `users`, `devices`,
- `nodes`: Liste mit Filter nach Region/Rolle/Status (paginiert, für tausende
  Nodes). **Primär:** Assistent „Server hinzufügen“: Host/IP, Port, Benutzer,
  Passwort oder SSH-Key, Rolle → Host-Key im Dashboard bestätigen → Preflight →
  Installation mit Live-Log (SSE) → Node online — **ohne** dass der Admin sich
  auf dem Server einloggen muss. **Sekundär:** Tab „Manuell installieren“ mit
  Join-Token/Einzeiler. Bulk-Hinzufügen per CSV (später).
  Aktionen pro Node: Update, Wartungsmodus (keine neuen Sessions), Entfernen.
- `regions`: Regionen anlegen und Fallback-Regionen festlegen.
  `profiles` (Erzeugen, einmaliger Download, QR-Code, Widerrufen), `audit`, `settings`.
- User sehen nur `devices` und `profiles`.
**Akzeptanz:** Ein Admin fügt einen frischen Server nur per IP und Passwort
hinzu, ohne die Kommandozeile zu benutzen. Ein neuer User kann sich einloggen,
ein Gerät anlegen, ein WireGuard-Profil per QR-Code in die offizielle App
importieren und verbinden.

## Schritt 12 – VPNC-Cluster: Ingress mit Cluster-Key
**Ziel:** VPNC-Profile, die gegen jeden Ingress-Knoten funktionieren.
**Anlegen:**
- `vpnc-control`: CRUD für `ingress_clusters`, Cluster-Keypair erzeugen und
  verschlüsselt speichern, Mitglieder mit Unicast-Endpoint verwalten,
  VPNC-Profilgenerierung mit GeoDNS-Endpoint.
- `vpnc-wg`: Cluster-Key und Peer-Liste auf allen Ingress-Mitgliedern synchron
  halten (über den Config-Stream).
- `vpnc-node/ingress/`: WireGuard-Interface mit Cluster-Key.
- Key-Rotation vorbereiten (`key_version`), aber noch nicht automatisieren.
**Akzeptanz:** netns-Test mit zwei Ingress-Knoten: dasselbe Profil verbindet mit
beiden; Wechsel des Endpoints führt nach Re-Handshake zu funktionierendem Tunnel.

## Schritt 13 – Backbone, Egress-Pool, Peer-Location
**Ziel:** Traffic läuft Ingress → Backbone → Egress → Internet und zurück.
**Anlegen:**
- Backbone: regionale WireGuard-Topologie (Ingress ↔ Egress derselben Region plus
  Fallback-Regionen, kein Full-Mesh). Keys und Peers verteilt Control automatisch.
  Neue Nodes werden ohne manuellen Eingriff eingebunden, entfernte sauber
  ausgetragen.
- `vpnc-wg`: nach Handshake `ReportPeerLocation` an Control; Control verteilt die
  Zuordnung Tunnel-IP → Ingress an die Egress-Knoten.
- `vpnc-node/egress/`: NAT ins Internet, Rückroute pro Tunnel-IP zum aktuellen
  Ingress, Auswahl des Egress pro Flow per konsistentem Hashing auf dem Ingress.
- Dashboard `vpnc-network`: Cluster, Mitglieder, Pools, Live-Status.
**Akzeptanz:** netns-Test mit 2 Ingress + 2 Egress: Verbindungen bleiben stabil,
Flows verteilen sich, Ausfall eines Egress verschiebt neue Flows. Ein dritter
Egress, der während des Tests per Provisioning hinzukommt, erhält ohne Neustart
anderer Nodes Traffic.
**Hinweis:** Gemeinsame öffentliche Egress-IP über mehrere Knoten wird hier nur
vorbereitet (ADR schreiben, Optionen: fester Egress pro Nutzer, Floating IP mit
conntrackd, Anycast/ECMP). Umsetzung erst nach Rückfrage.

## Schritt 14 – Client-Core und Desktop-Client
**Ziel:** Eigener Client mit WireGuard- und VPNC-Backend.
**Anlegen:**
- `vpnc-core`: `TunnelBackend`-Trait (connect, disconnect, send, stats), Backend `wireguard` (boringtun),
  TUN-Abstraktion pro Plattform (Linux/macOS: tun, Windows: Wintun).
- Backend `vpnc`: nutzt dasselbe WireGuard-Profil, holt per gRPC die
  Unicast-Endpoints des Clusters, misst Latenz, wählt den besten, wechselt bei
  Ausfall.
- `apps/client-desktop`: Tauri v2, Profile importieren (`.conf`, QR, Link aus
  Dashboard), verbinden, Statistiken. Privilegierter Teil als separater
  Hilfsdienst, UI läuft ohne Admin-Rechte.
**Akzeptanz:** Desktop-Client verbindet mit Classic-WireGuard und VPNC auf Linux
und Windows; VPNC-Failover funktioniert im Test.

## Schritt 15 – OpenVPN im Client (Lizenzprüfung zuerst)
**Ziel:** OpenVPN-Profile im eigenen Client.
- Zuerst ADR zur Lizenz (OpenVPN3-Core ist AGPL bzw. kommerziell) – anhalten
  und Entscheidung einholen, bevor Code entsteht.
- Danach Backend `openvpn` per FFI in vpnc-core.

## Schritt 16 – Deployment und Dokumentation
**Ziel:** Selfhosting in wenigen Minuten.
**Anlegen:**
- Multi-Stage-Dockerfiles für control und node (Root-`Dockerfile` liefert Control
  + Static-Dashboard + `vpnc-node`-Binaries unter `/app/binaries`).
- `docker-compose.yml` (lokal) und `docker-compose.coolify.yml` (Traefik, gRPC 50051).
- systemd-Units in `deploy/systemd/`, Installationsskript für Nodes.
- `docs/`: Installation, Coolify (`deploy-coolify.md`), GeoDNS, Backup/Restore, Update.
- Auto-Bootstrap (`VPNC_BOOTSTRAP_*`), `VPNC_STATIC_DIR`, JSON-Logs.
- `tests/load/`: simulierte Nodes (nur gRPC, kein echtes WireGuard), um
  Control mit z. B. 5 000 Nodes und zwei Control-Instanzen zu testen.
**Akzeptanz:** Frische VM/Coolify: Anleitung befolgen, Dashboard erreichbar, Node
per Dashboard-Assistent hinzugefügt, Profil funktioniert in der offiziellen
WireGuard-App. Lasttest zeigt stabile Config-Verteilung und Reconnects ohne
Lastspitzen.

## Classic-Selfhost „fertig“ (Quercheck)

Nicht behaupten, bis: Login → Server per Assistent → Node online → Gerät →
Profil/QR → WireGuard-App, ohne CLI auf dem Node. Offen bleiben bewusst:
Cluster/Backbone (12–13), Desktop-Client (14), OpenVPN (9/15), Lasttest.
