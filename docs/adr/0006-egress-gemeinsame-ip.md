# ADR 0006: Gemeinsame öffentliche Egress-IP
Status: vorbereitet (Umsetzung erst nach Rückfrage, siehe Roadmap Schritt 13)

## Kontext
VPNC-Tunnel sollen über einen Egress-Pool mit gemeinsamer öffentlicher IP
austreten. Mehrere Knoten hinter einer IP erfordern Session-/Conntrack-Sync
oder Anycast.

## Optionen
1. **Fester Egress pro Nutzer** – einfach, keine gemeinsame IP im strengen Sinn;
   Sticky über Control-Zuweisung.
2. **Floating IP + conntrackd** – klassisches HA; komplexer Betrieb, shared state.
3. **Anycast/ECMP** – beste Skalierung, braucht eigenes AS / Provider-Support.

## Entscheidung
Noch nicht getroffen. Schritt 13 implementiert Backbone, Peer-Location und
Flow-Hashing; gemeinsame Egress-IP erst nach expliziter Freigabe.
