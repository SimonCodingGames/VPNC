import { browser } from '$app/environment';

const API_BASE = import.meta.env.VITE_API_BASE ?? '';

export type Me = {
  id: string;
  email: string;
  role: 'admin' | 'user';
};

/** Calls `/api/auth/me` using session cookie (credentials include). */
export async function fetchMe(): Promise<Me | null> {
  if (!browser) return null;
  const res = await fetch(`${API_BASE}/api/auth/me`, {
    credentials: 'include'
  });
  if (!res.ok) return null;
  return (await res.json()) as Me;
}

/** Logs in; password must be cleared by the caller after await. */
export async function login(email: string, password: string): Promise<Me> {
  const res = await fetch(`${API_BASE}/api/auth/login`, {
    method: 'POST',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email, password })
  });
  if (!res.ok) {
    throw new Error('login failed');
  }
  return (await res.json()) as Me;
}

/** Logs out. */
export async function logout(): Promise<void> {
  await fetch(`${API_BASE}/api/auth/logout`, {
    method: 'POST',
    credentials: 'include'
  });
}
