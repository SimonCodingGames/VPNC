const dtf = new Intl.DateTimeFormat('de-DE', { dateStyle: 'medium', timeStyle: 'short' });
const rtf = new Intl.RelativeTimeFormat('de-DE', { numeric: 'auto' });

/** Formats an ISO timestamp as a localized date/time; falls back to the raw string. */
export function formatDate(iso: string | null | undefined): string {
  if (!iso) return '—';
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : dtf.format(d);
}

/** Formats an ISO timestamp relative to now (e.g. "vor 5 Minuten"). */
export function formatRelative(iso: string | null | undefined): string {
  if (!iso) return '—';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const sec = Math.round((d.getTime() - Date.now()) / 1000);
  const abs = Math.abs(sec);
  if (abs < 60) return rtf.format(sec, 'second');
  if (abs < 3600) return rtf.format(Math.round(sec / 60), 'minute');
  if (abs < 86400) return rtf.format(Math.round(sec / 3600), 'hour');
  if (abs < 86400 * 30) return rtf.format(Math.round(sec / 86400), 'day');
  return dtf.format(d);
}

/** Shortens a UUID for display. */
export function shortId(id: string | null | undefined): string {
  if (!id) return '—';
  return id.length > 12 ? `${id.slice(0, 8)}…` : id;
}

export type Tone = 'success' | 'warning' | 'danger' | 'info' | 'neutral' | 'violet';

/** Maps a node / job status string to a visual tone. */
export function statusTone(status: string): Tone {
  const s = status.toLowerCase();
  if (['online', 'active', 'running', 'succeeded', 'healthy', 'ok'].includes(s)) return 'success';
  if (['pending', 'awaiting_hostkey', 'provisioning', 'degraded', 'installing'].includes(s))
    return 'warning';
  if (['offline', 'failed', 'error', 'down', 'disabled'].includes(s)) return 'danger';
  if (['cancelled', 'unknown'].includes(s)) return 'neutral';
  return 'info';
}

/** Maps a node role to a visual tone. */
export function roleTone(role: string): Tone {
  if (role === 'ingress') return 'info';
  if (role === 'egress') return 'violet';
  if (role === 'admin') return 'violet';
  return 'neutral';
}

/** Up to two initials from an e-mail address. */
export function initials(email: string): string {
  const local = email.split('@')[0] ?? '';
  const parts = local.split(/[._-]+/).filter(Boolean);
  const s = parts.length >= 2 ? parts[0][0] + parts[1][0] : local.slice(0, 2);
  return s.toUpperCase();
}
