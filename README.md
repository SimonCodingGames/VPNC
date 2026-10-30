# VPNC

Selfhosted VPN-Plattform mit Multi-Protokoll-Client und Web-Dashboard.

**Installation:** siehe **[docs/installation.md](docs/installation.md)**  
(inkl. Docker-Schnellstart, Quellcode-Setup und UI-Vorschaubilder).

## Grundsatz: Auf dem Draht ist alles WireGuard

Ein VPNC-Tunnel ist ein normales WireGuard-Profil. Es funktioniert in der offiziellen
WireGuard-App auf jeder Plattform. Die gesamte Intelligenz (Ingress-Wahl,
Load-Balancing, Egress-Pools) liegt auf der Serverseite.

**Classic** – WireGuard- und OpenVPN-Profile zu genau einem Server, ohne Lastverteilung.

**VPNC-Tunnel** – WireGuard-Profil mit Endpoint auf einer Anycast-IP oder einem
GeoDNS-Namen. Alle Ingress-Knoten teilen sich den Cluster-Key, der Traffic läuft
übers Backbone zum Egress-Pool (Flow-Hashing, gemeinsame Ausgangs-IP).
Der eigene VPNC-Client nutzt dasselbe Profil, kann aber zusätzlich selbst den
schnellsten Ingress wählen und schneller umschalten.

## Komponenten

| Pfad | Zweck |
|---|---|
| crates/vpnc-wg | WireGuard-Clusterlogik: Cluster-Key, Peer-Sync, Peer-Location |
| crates/vpnc-core | Client-Core: TunnelBackend-Trait, Backends, Userspace-Router |
| crates/vpnc-node | Daemon auf Servern, Rollen: ingress, egress, classic |
| crates/vpnc-control | Control Plane: REST fürs Dashboard, gRPC für Nodes/Clients |
| crates/vpnc-common | Geteilte Typen, Config, Logging |
| apps/dashboard | Web-Dashboard (SvelteKit + TypeScript) |
| apps/client-desktop | Desktop-Client (Tauri, nutzt vpnc-core) |
| apps/client-android, apps/client-ios | Mobile-Clients (später) |
| packages/ui | Geteilte UI-Komponenten (Dashboard + Desktop-Client) |
| packages/api-client | Generierter TypeScript-Client für die REST-API |
| proto | gRPC-Definitionen |
| deploy | docker-compose, systemd, Helm |
| docs/adr | Architecture Decision Records |
