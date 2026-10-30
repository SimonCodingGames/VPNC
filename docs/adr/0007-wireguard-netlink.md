# ADR 0007: Kernel-WireGuard-Verwaltung per Netlink
Status: angenommen (Schritt 7)

## Kontext
vpnc-node muss WireGuard-Interfaces und Peers idempotent verwalten.

## Entscheidung
Primär: Crate `wireguard-control` (bzw. netlink-basiert) für Kernel-WireGuard.
Fallback: `boringtun` Userspace, wenn kein Kernelmodul verfügbar.
Die konkrete Crate-Version wird in `crates/vpnc-node` gepinnt und kann ohne
Schema-Migration gewechselt werden.
