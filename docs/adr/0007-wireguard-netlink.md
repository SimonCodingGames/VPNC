# ADR 0007: Kernel-WireGuard-Verwaltung per Netlink
Status: angenommen (Schritt 7)

## Kontext
vpnc-node muss WireGuard-Interfaces und Peers idempotent verwalten.

## Entscheidung
Primär: Kernel-WireGuard über Netlink. Umsetzung in `vpnc-node` mit den
festen Tools `ip` und `wg setconf` (Argumentliste, keine Shell-Strings) — das
ist netlink-basiert und vermeidet C-Library-Linking im Cross-Build.
Optional später: Crate `wireguard-control` als Direktbindung.
Fallback: `boringtun` Userspace, wenn kein Kernelmodul verfügbar (Apply dann
Userspace-Pfad; derzeit wird Kernel bevorzugt und `wg` verlangt).
