<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchMe } from '$lib/auth';
  import { api, ApiError, type User } from '$lib/api';
  import Forbidden from '$lib/components/Forbidden.svelte';
  import { de } from '$lib/i18n/de';

  let allowed = $state<boolean | null>(null);
  let items = $state<User[]>([]);
  let error = $state('');
  let busy = $state(false);
  let email = $state('');
  let password = $state('');
  let role = $state('user');
  let resetId = $state<string | null>(null);
  let resetPw = $state('');

  onMount(async () => {
    const me = await fetchMe();
    allowed = me?.role === 'admin';
    if (allowed) await load();
  });

  async function load() {
    error = '';
    try {
      const res = await api.listUsers();
      items = res.items;
    } catch (e) {
      if (e instanceof ApiError && e.status === 403) {
        allowed = false;
        return;
      }
      error = e instanceof ApiError ? e.message : 'Fehler';
    }
  }

  async function onCreate(e: Event) {
    e.preventDefault();
    busy = true;
    try {
      await api.createUser({ email: email.trim(), password, role });
      email = '';
      password = '';
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
    } finally {
      busy = false;
      password = '';
    }
  }

  async function setRole(id: string, next: string) {
    busy = true;
    try {
      await api.updateUser(id, { role: next });
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
    } finally {
      busy = false;
    }
  }

  async function toggleDisabled(u: User) {
    busy = true;
    try {
      await api.updateUser(u.id, { disabled: !u.disabled });
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
    } finally {
      busy = false;
    }
  }

  async function onReset(e: Event) {
    e.preventDefault();
    if (!resetId || resetPw.length < 10) return;
    busy = true;
    try {
      await api.updateUser(resetId, { password: resetPw });
      resetId = null;
      resetPw = '';
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
    } finally {
      busy = false;
      resetPw = '';
    }
  }

  async function onDelete(id: string) {
    if (!confirm(de.users.confirmDelete)) return;
    busy = true;
    try {
      await api.deleteUser(id);
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head>
  <title>{de.users.title} · {de.brand}</title>
</svelte:head>

{#if allowed === null}
  <p class="text-sm text-slate-500">{de.loading}</p>
{:else if !allowed}
  <Forbidden />
{:else}
  <section class="space-y-6">
    <h1 class="text-2xl font-semibold">{de.users.title}</h1>

    <form class="flex flex-wrap gap-3 items-end" onsubmit={onCreate}>
      <label class="space-y-1">
        <span class="text-sm">{de.email}</span>
        <input
          class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
          type="email"
          bind:value={email}
          required
        />
      </label>
      <label class="space-y-1">
        <span class="text-sm">{de.password}</span>
        <input
          class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
          type="password"
          bind:value={password}
          minlength="10"
          required
        />
      </label>
      <label class="space-y-1">
        <span class="text-sm">{de.role}</span>
        <select
          class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
          bind:value={role}
        >
          <option value="user">user</option>
          <option value="admin">admin</option>
        </select>
      </label>
      <button
        class="bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 rounded px-4 py-2"
        type="submit"
        disabled={busy}>{de.users.new}</button
      >
    </form>

    {#if resetId}
      <form class="flex flex-wrap gap-3 items-end border p-3 rounded" onsubmit={onReset}>
        <label class="space-y-1">
          <span class="text-sm">{de.users.resetPassword}</span>
          <input
            class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
            type="password"
            bind:value={resetPw}
            minlength="10"
            required
          />
        </label>
        <button type="submit" class="underline text-sm" disabled={busy}>{de.save}</button>
        <button type="button" class="underline text-sm" onclick={() => (resetId = null)}
          >{de.cancel}</button
        >
      </form>
    {/if}

    {#if error}
      <p class="text-sm text-red-600">{error}</p>
    {/if}

    {#if items.length === 0}
      <p class="text-sm text-slate-500">{de.empty}</p>
    {:else}
      <ul
        class="divide-y divide-slate-200 dark:divide-slate-700 border border-slate-200 dark:border-slate-700 rounded"
      >
        {#each items as u (u.id)}
          <li class="px-3 py-3 flex flex-wrap gap-2 items-center">
            <div>
              <div class="font-medium">
                {u.email}
                {#if u.disabled}
                  <span class="text-xs text-red-600">({de.users.disabled})</span>
                {/if}
              </div>
              <div class="text-xs text-slate-500">{u.role}</div>
            </div>
            <div class="ml-auto flex flex-wrap gap-3 text-sm">
              <select
                class="border border-slate-300 dark:border-slate-600 rounded px-2 py-1 bg-white dark:bg-slate-800"
                value={u.role}
                onchange={(e) => setRole(u.id, (e.currentTarget as HTMLSelectElement).value)}
              >
                <option value="user">user</option>
                <option value="admin">admin</option>
              </select>
              <button type="button" class="underline" onclick={() => (resetId = u.id)}
                >{de.users.resetPassword}</button
              >
              <button type="button" class="underline" onclick={() => toggleDisabled(u)}>
                {u.disabled ? de.users.enable : de.users.disable}
              </button>
              <button type="button" class="underline text-red-600" onclick={() => onDelete(u.id)}
                >{de.delete}</button
              >
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}
