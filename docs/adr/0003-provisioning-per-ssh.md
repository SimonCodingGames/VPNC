# ADR 0003: Node-Provisioning per SSH aus dem Dashboard
Status: angenommen
Admins fügen Server per IP + Passwort/SSH-Key hinzu. VPNC installiert vpnc-node
per SSH und registriert ihn mit einem einmaligen Join-Token.
- Zugangsdaten nur im RAM des Jobs, nie persistiert.
- Host-Key muss bestätigt werden (Schutz gegen Man-in-the-Middle).
- Nach der Installation kein SSH mehr nötig; Updates über den Agenten mit
  signierten Binaries.
- Risiko: vpnc-control ist während eines Jobs ein hochprivilegiertes System.
  Control-Server entsprechend absichern (2FA fürs Dashboard, Netzwerkzugriff
  einschränken).
