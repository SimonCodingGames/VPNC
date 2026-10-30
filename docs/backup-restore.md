# Backup / Restore

## PostgreSQL
```bash
pg_dump -Fc vpnc > vpnc.dump
pg_restore -d vpnc vpnc.dump
```

## SQLite
Datei `vpnc.db` (bzw. konfigurierter Pfad) im Stillstand kopieren.

Master-Key (`VPNC_MASTER_KEY`) getrennt sichern — ohne ihn sind verschlüsselte
Server-/Cluster-Keys nicht wiederherstellbar.
