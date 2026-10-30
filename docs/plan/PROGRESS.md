# VPNC – Fortschritt

- [x] Schritt 0 – Bootstrap (Regeln, Plan, Ordnerstruktur)
- [x] Schritt 1 – Workspace und Tooling
- [x] Schritt 2 – vpnc-common
- [x] Schritt 3 – vpnc-control: Grundgerüst, DB, Auth
- [x] Schritt 4 – vpnc-control: Benutzer, Geräte, Nodes, Audit
- [x] Schritt 5 – Classic-WireGuard-Profile
- [x] Schritt 6 – gRPC: Node-Registrierung und Config-Stream
- [ ] Schritt 7 – vpnc-node: Rolle `classic` für WireGuard (Struktur + Diff-Tests; Linux-netns offen)
- [ ] Schritt 8 – Node-Provisioning per SSH (Backend) (Jobs/Preflight/SSH-Fingerprint; russh-Session offen)
- [ ] Schritt 9 – OpenVPN Classic
- [ ] Schritt 10 – Dashboard: Grundgerüst (Login-Shell vorhanden; Static-Serve durch control offen)
- [ ] Schritt 11 – Dashboard: Verwaltungsseiten
- [ ] Schritt 12 – VPNC-Cluster: Ingress mit Cluster-Key
- [ ] Schritt 13 – Backbone, Egress-Pool, Peer-Location
- [ ] Schritt 14 – Client-Core und Desktop-Client
- [ ] Schritt 15 – OpenVPN im Client (Lizenzprüfung zuerst)
- [ ] Schritt 16 – Deployment und Dokumentation (Compose/Docker/Docs angelegt; Lasttest offen)

## Notizen

### Schritt 1
- `just build` / `just lint` / `just test` lokal grün (MSVC Build Tools bzw. MinGW).

### Schritt 2
- `vpnc-common`: config, error, telemetry, types, secret inkl. Unit-Tests.

### Schritt 3
- Auth-Flow (bootstrap/login/me/logout) mit SQLite grün.
- Postgres-Test wenn `VPNC_TEST_DATABASE_URL` gesetzt (CI-Service vorhanden).
- Encrypted blobs als base64-TEXT für SQLite+Postgres-Kompatibilität.

### Schritt 4
- CRUD users/devices/nodes/audit, Join-Tokens, OpenAPI JSON, Auth-Middleware.
- Test: User sieht keine fremden Geräte; Audit-Einträge; openapi.json valide.

### Schritt 5
- IPAM (inkl. Exhaustion-Test), WireGuard-Keypairs/.conf/QR-PNG.
- Profile create/download-once/revoke; Private Key nur im RAM, nicht in DB.

### Schritt 6
- `proto/node.proto` + `client.proto`, tonic in control+node.
- Register (Join-Token), Config-Stream, Health, Offline-Markierung.
- Test `grpc_register` grün. mTLS-Zertifikatsausstellung noch TODO.

### Schritt 7 (teilweise)
- Module `classic/wireguard.rs` + `nat.rs`, Peer-Diff-Tests, ADR 0007.
- Kernel/netlink-Apply und netns-Akzeptanz brauchen Linux (hier Windows).

### Schritt 16 (teilweise)
- Dockerfiles, compose, systemd-Units, Install/Backup/GeoDNS/Update-Docs.

### Blocker / Entscheidungen nötig
- **Schritt 15:** ADR 0005 – OpenVPN-Client-Lizenz (AGPL / kommerziell / externer Prozess / weglassen).
- **Schritt 13:** ADR 0006 – gemeinsame Egress-IP erst nach Rückfrage.
- **Schritt 7/8/9 Akzeptanz:** Linux + Docker/SSH-Testcontainer für netns/SSH-Tests.
