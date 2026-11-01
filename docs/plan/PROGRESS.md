# VPNC – Fortschritt

- [x] Schritt 0 – Bootstrap (Regeln, Plan, Ordnerstruktur)
- [x] Schritt 1 – Workspace und Tooling
- [x] Schritt 2 – vpnc-common
- [x] Schritt 3 – vpnc-control: Grundgerüst, DB, Auth
- [x] Schritt 4 – vpnc-control: Benutzer, Geräte, Nodes, Audit
- [x] Schritt 5 – Classic-WireGuard-Profile
- [x] Schritt 6 – gRPC: Node-Registrierung und Config-Stream
- [ ] Schritt 7 – vpnc-node: Rolle `classic` für WireGuard (Struktur + Diff-Tests; Kernel-Apply/netns offen)
- [x] Schritt 8 – Node-Provisioning per SSH (Backend: API, russh, Install, Worker, Secrets-Tests)
- [ ] Schritt 9 – OpenVPN Classic
- [x] Schritt 10 – Dashboard: Grundgerüst (Login, Static-Serve durch Control, Hell/Dunkel)
- [x] Schritt 11 – Dashboard: Verwaltungsseiten + Assistent „Server hinzufügen“ (SSH)
- [ ] Schritt 12 – VPNC-Cluster: Ingress mit Cluster-Key
- [ ] Schritt 13 – Backbone, Egress-Pool, Peer-Location
- [ ] Schritt 14 – Client-Core und Desktop-Client
- [ ] Schritt 15 – OpenVPN im Client (Lizenzprüfung zuerst)
- [ ] Schritt 16 – Deployment und Dokumentation (Compose/Coolify/Docs/Binary-Store erledigt; Lasttest offen)

## Classic-Selfhost-Checkliste (Ziel)

- [x] Login unter derselben Domain wie `/api`
- [x] Server-Assistent im Dashboard (IP + SSH → Host-Key → Install)
- [ ] E2E auf Linux: Job succeeded → Node online (braucht erreichbaren SSH-Host + Binary)
- [ ] Gerät + WireGuard-Profil → Import in offizielle App (braucht Schritt 7 Kernel-WG)
- [x] Kein manuelles Einloggen auf dem Node nötig (Primärpfad)

## Notizen

### Schritt 8
- `POST /api/provisioning`, Host-Key-Bestätigung, SSE/Polling-Log, Worker, Join-Token, systemd.
- Secrets nur RAM (`SecretStore`), nie in DB/API.
- Binary-Store: `VPNC_NODE_BINARIES_DIR` (`linux-amd64/vpnc-node` im Image).
- SSH-E2E gegen echten Host: auf Linux/Coolify manuell; Unit-Tests für Secrets/Host-Key.

### Schritt 7 (offen für Tunnel)
- Node registriert sich; WireGuard-Interface/nft Apply noch nicht produktiv.

### Blocker / Entscheidungen nötig
- **Schritt 15:** ADR 0005 – OpenVPN-Client-Lizenz.
- **Schritt 13:** ADR 0006 – gemeinsame Egress-IP erst nach Rückfrage.
