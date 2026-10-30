import { browser } from '$app/environment';

const API_BASE = import.meta.env.VITE_API_BASE ?? '';

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

type ListResponse<T> = { items: T[]; next_cursor: string | null };

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  if (!browser) {
    throw new ApiError(0, 'not in browser');
  }
  const res = await fetch(`${API_BASE}${path}`, {
    credentials: 'include',
    ...init,
    headers: {
      ...(init?.body ? { 'content-type': 'application/json' } : {}),
      ...init?.headers
    }
  });
  if (res.status === 204) {
    return undefined as T;
  }
  const text = await res.text();
  let body: unknown = null;
  if (text) {
    try {
      body = JSON.parse(text);
    } catch {
      body = text;
    }
  }
  if (!res.ok) {
    const msg =
      typeof body === 'object' && body && 'error' in body
        ? String((body as { error: unknown }).error)
        : res.statusText || 'request failed';
    throw new ApiError(res.status, msg);
  }
  return body as T;
}

export type Device = {
  id: string;
  user_id: string;
  name: string;
  created_at: string;
};

export type Profile = {
  id: string;
  device_id: string;
  node_id: string | null;
  kind: string;
  client_public_key: string | null;
  assigned_ipv4: string | null;
  assigned_ipv6: string | null;
  revoked_at: string | null;
  created_at: string;
  download_available: boolean;
};

export type Node = {
  id: string;
  name: string;
  role: string;
  status: string;
  public_ip: string | null;
  last_seen_at: string | null;
  created_at: string;
};

export type User = {
  id: string;
  email: string;
  role: 'admin' | 'user';
  disabled: boolean;
};

export type AuditEvent = {
  id: string;
  actor_user_id: string | null;
  action: string;
  resource_type: string | null;
  resource_id: string | null;
  created_at: string;
};

export type JoinToken = {
  token: string;
  expires_at: string;
  install_command: string;
  docker_command: string;
};

export type ProfileDownload = {
  conf: string;
  qr_png_base64: string;
};

export type Settings = {
  version: string;
  http_bind: string;
  grpc_bind: string;
  public_url: string | null;
  public_grpc: string | null;
  cookie_secure: boolean;
  trust_forwarded_headers: boolean;
  static_dir_configured: boolean;
  note: string;
};

export const api = {
  listDevices: () => request<ListResponse<Device>>('/api/devices'),
  createDevice: (name: string) =>
    request<Device>('/api/devices', { method: 'POST', body: JSON.stringify({ name }) }),
  renameDevice: (id: string, name: string) =>
    request<Device>(`/api/devices/${id}`, {
      method: 'PATCH',
      body: JSON.stringify({ name })
    }),
  deleteDevice: (id: string) => request<void>(`/api/devices/${id}`, { method: 'DELETE' }),

  listProfiles: () => request<ListResponse<Profile>>('/api/profiles'),
  createProfile: (device_id: string, node_id: string) =>
    request<{ id: string; download_once: boolean }>('/api/profiles', {
      method: 'POST',
      body: JSON.stringify({ device_id, node_id })
    }),
  downloadProfile: (id: string) => request<ProfileDownload>(`/api/profiles/${id}/download`),
  revokeProfile: (id: string) => request<void>(`/api/profiles/${id}/revoke`, { method: 'POST' }),

  listNodes: () => request<ListResponse<Node>>('/api/nodes'),
  createNode: (body: {
    name: string;
    role: string;
    tunnel_cidr_v4?: string;
    tunnel_cidr_v6?: string;
  }) => request<Node>('/api/nodes', { method: 'POST', body: JSON.stringify(body) }),
  deleteNode: (id: string) => request<void>(`/api/nodes/${id}`, { method: 'DELETE' }),
  createJoinToken: (body: { role: string; node_name?: string; ttl_minutes?: number }) =>
    request<JoinToken>('/api/nodes/join-tokens', {
      method: 'POST',
      body: JSON.stringify(body)
    }),

  listUsers: () => request<ListResponse<User>>('/api/users'),
  createUser: (body: { email: string; password: string; role: string }) =>
    request<User>('/api/users', { method: 'POST', body: JSON.stringify(body) }),
  updateUser: (id: string, body: { role?: string; password?: string; disabled?: boolean }) =>
    request<User>(`/api/users/${id}`, { method: 'PATCH', body: JSON.stringify(body) }),
  deleteUser: (id: string) => request<void>(`/api/users/${id}`, { method: 'DELETE' }),

  listAudit: (cursor?: string | null) => {
    const q = cursor ? `?cursor=${encodeURIComponent(cursor)}` : '';
    return request<ListResponse<AuditEvent>>(`/api/audit${q}`);
  },

  getSettings: () => request<Settings>('/api/settings')
};
