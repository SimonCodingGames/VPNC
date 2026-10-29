# ADR 0002: VPNC-Tunnel sind reines WireGuard
Status: angenommen (ersetzt den Plan eines eigenen Protokolls)

Anforderung: VPNC-Profile müssen in der offiziellen WireGuard-App funktionieren.
Daher kein eigenes Wire-Protokoll. Konsequenzen:
- Ingress-Wahl erfolgt serverseitig: Anycast (eigenes AS + IP-Block, beste Lösung)
  oder GeoDNS (günstig, Wahl nur beim Aktivieren des Tunnels).
- Alle Ingress-Knoten eines Clusters teilen den Server-Private-Key.
  Risiko: Kompromittierung eines Knotens betrifft den Cluster.
  Rotation erfordert neue Profile für Nutzer der Standard-App.
- Wechselt ein Client den Ingress, fehlt dem neuen Knoten die Session;
  WireGuard handshaked automatisch neu (Unterbrechung im Sekundenbereich).
- Egress muss wissen, wo ein Peer gerade ist: Ingress meldet Peer-Location nach Handshake.
- Der VPNC-Client ist optionaler Komfort (Latenzwahl, schnelleres Failover), keine Pflicht.
