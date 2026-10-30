# ADR 0005: OpenVPN im eigenen Client – Lizenzfrage
Status: ausstehend (Entscheidung erforderlich vor Schritt 15)

## Kontext
Schritt 15 soll OpenVPN-Profile im VPNC-Client unterstützen. OpenVPN3-Core
steht unter AGPL bzw. kommerzieller Lizenz. Einbindung per FFI kann die
AGPL-Verpflichtungen auf den gesamten Client ausweiten.

## Optionen
1. **OpenVPN3-Core (AGPL)** – Client wird AGPL; Distributionspflichten beachten.
2. **Kommerzielle Lizenz von OpenVPN Inc.** – proprietärer Client möglich.
3. **Externer OpenVPN-2.x-Prozess** – VPNC startet System-`openvpn`, keine
   Linkage mit AGPL-Code; weniger integriert, Abhängigkeit vom OS-Paket.
4. **Kein OpenVPN im eigenen Client** – nur Profil-Download für offizielle
   OpenVPN-Apps; Classic-OpenVPN auf dem Server bleibt (Schritt 9).

## Entscheidung
Noch nicht getroffen. Umsetzung von Schritt 15 ist blockiert, bis eine Option
gewählt ist.
