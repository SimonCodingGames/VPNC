/** German UI strings (centralized for later i18n). */
export const de = {
  brand: 'VPNC',
  loginTitle: 'Anmelden',
  email: 'E-Mail',
  password: 'Passwort',
  submitLogin: 'Einloggen',
  logout: 'Abmelden',
  nav: {
    devices: 'Geräte',
    profiles: 'Profile',
    users: 'Benutzer',
    nodes: 'Server',
    audit: 'Audit',
    settings: 'Einstellungen'
  },
  themeToggle: 'Darstellung umschalten',
  forbidden: 'Keine Berechtigung',
  forbiddenHint: 'Diese Seite ist nur für Administratoren sichtbar.',
  notFound: 'Seite nicht gefunden',
  loading: 'Laden…',
  save: 'Speichern',
  cancel: 'Abbrechen',
  create: 'Anlegen',
  delete: 'Löschen',
  rename: 'Umbenennen',
  copy: 'Kopieren',
  copied: 'Kopiert',
  actions: 'Aktionen',
  name: 'Name',
  role: 'Rolle',
  status: 'Status',
  empty: 'Noch keine Einträge.',
  devices: {
    title: 'Geräte',
    new: 'Neues Gerät',
    namePlaceholder: 'z. B. Laptop',
    confirmDelete: 'Gerät wirklich löschen?'
  },
  profiles: {
    title: 'Profile',
    new: 'Profil anlegen',
    device: 'Gerät',
    node: 'Server',
    download: 'Konfiguration laden',
    downloadOnce:
      'Der Download der .conf und des QR-Codes funktioniert nur einmal. Speichere die Datei sicher.',
    downloadGone: 'Privater Schlüssel bereits heruntergeladen oder nicht mehr verfügbar.',
    revoke: 'Widerrufen',
    confirmRevoke: 'Profil widerrufen? Der Client verliert den Zugang.',
    qr: 'QR-Code',
    conf: 'WireGuard-Konfiguration',
    downloadFile: '.conf herunterladen'
  },
  nodes: {
    title: 'Server',
    new: 'Server anlegen',
    joinToken: 'Join-Token erzeugen',
    tokenOnce:
      'Dieser Token wird nur einmal angezeigt. Sofort kopieren und auf dem Node verwenden.',
    ttl: 'Gültigkeit (Minuten)',
    cidrV4: 'Tunnel-CIDR IPv4',
    cidrV6: 'Tunnel-CIDR IPv6',
    installCmd: 'CLI-Befehl',
    dockerCmd: 'Docker-Befehl',
    confirmDelete: 'Server wirklich löschen?'
  },
  users: {
    title: 'Benutzer',
    new: 'Benutzer anlegen',
    disable: 'Deaktivieren',
    enable: 'Aktivieren',
    resetPassword: 'Passwort setzen',
    confirmDelete: 'Benutzer wirklich löschen?',
    disabled: 'deaktiviert'
  },
  audit: {
    title: 'Audit',
    loadMore: 'Weitere laden',
    actor: 'Akteur',
    action: 'Aktion',
    resource: 'Ressource',
    time: 'Zeit'
  },
  settings: {
    title: 'Einstellungen',
    readOnly: 'Aktuell nur lesend. Schreibende Einstellungen folgen später.'
  }
} as const;
