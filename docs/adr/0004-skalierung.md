# ADR 0004: Keine festen Obergrenzen
Status: angenommen
- vpnc-control zustandslos, horizontal skalierbar, PostgreSQL + LISTEN/NOTIFY.
- Config-Deltas statt Vollabgleich, Reconnect mit Jitter.
- Regionales Backbone statt Full-Mesh (Full-Mesh wächst quadratisch).
- Große Adressbereiche (100.64.0.0/10, IPv6-ULA), aufgeteilt pro Cluster.
- Praktische Grenzen entstehen dann durch Hardware und Peers pro Ingress,
  nicht durch die Architektur.
