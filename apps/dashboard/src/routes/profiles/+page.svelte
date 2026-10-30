<script lang="ts">
  import { onMount } from 'svelte';
  import {
    api,
    ApiError,
    type Device,
    type Node,
    type Profile,
    type ProfileDownload
  } from '$lib/api';
  import { de } from '$lib/i18n/de';

  let profiles = $state<Profile[]>([]);
  let devices = $state<Device[]>([]);
  let nodes = $state<Node[]>([]);
  let deviceId = $state('');
  let nodeId = $state('');
  let error = $state('');
  let busy = $state(false);
  let download = $state<ProfileDownload | null>(null);
  let downloadId = $state<string | null>(null);

  async function load() {
    error = '';
    try {
      const [p, d, n] = await Promise.all([api.listProfiles(), api.listDevices(), api.listNodes()]);
      profiles = p.items;
      devices = d.items;
      nodes = n.items;
      if (!deviceId && devices[0]) deviceId = devices[0].id;
      if (!nodeId && nodes[0]) nodeId = nodes[0].id;
    } catch (e) {
      error = e instanceof ApiError ? e.message : 'Fehler';
    }
  }

  onMount(() => {
    void load();
  });

  async function onCreate(e: Event) {
    e.preventDefault();
    if (!deviceId || !nodeId) return;
    busy = true;
    error = '';
    download = null;
    try {
      const created = await api.createProfile(deviceId, nodeId);
      await load();
      if (created.download_once) {
        await doDownload(created.id);
      }
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
    } finally {
      busy = false;
    }
  }

  async function doDownload(id: string) {
    busy = true;
    error = '';
    try {
      download = await api.downloadProfile(id);
      downloadId = id;
      await load();
    } catch (err) {
      error =
        err instanceof ApiError
          ? err.message || de.profiles.downloadGone
          : de.profiles.downloadGone;
      download = null;
    } finally {
      busy = false;
    }
  }

  function saveConf() {
    if (!download) return;
    const blob = new Blob([download.conf], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `vpnc-${downloadId ?? 'profile'}.conf`;
    a.click();
    URL.revokeObjectURL(url);
  }

  async function onRevoke(id: string) {
    if (!confirm(de.profiles.confirmRevoke)) return;
    busy = true;
    try {
      await api.revokeProfile(id);
      if (downloadId === id) download = null;
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
    } finally {
      busy = false;
    }
  }

  function deviceName(id: string) {
    return devices.find((d) => d.id === id)?.name ?? id;
  }

  function nodeName(id: string | null) {
    if (!id) return '—';
    return nodes.find((n) => n.id === id)?.name ?? id;
  }
</script>

<svelte:head>
  <title>{de.profiles.title} · {de.brand}</title>
</svelte:head>

<section class="space-y-6">
  <h1 class="text-2xl font-semibold">{de.profiles.title}</h1>
  <p
    class="text-sm text-amber-700 dark:text-amber-300 bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-800 rounded px-3 py-2"
  >
    {de.profiles.downloadOnce}
  </p>

  <form class="flex flex-wrap gap-3 items-end" onsubmit={onCreate}>
    <label class="space-y-1">
      <span class="text-sm">{de.profiles.device}</span>
      <select
        class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800 min-w-[12rem]"
        bind:value={deviceId}
        required
      >
        {#each devices as d (d.id)}
          <option value={d.id}>{d.name}</option>
        {/each}
      </select>
    </label>
    <label class="space-y-1">
      <span class="text-sm">{de.profiles.node}</span>
      <select
        class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800 min-w-[12rem]"
        bind:value={nodeId}
        required
      >
        {#each nodes as n (n.id)}
          <option value={n.id}>{n.name} ({n.role})</option>
        {/each}
      </select>
    </label>
    <button
      class="bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 rounded px-4 py-2 disabled:opacity-50"
      type="submit"
      disabled={busy || !deviceId || !nodeId}>{de.profiles.new}</button
    >
  </form>

  {#if error}
    <p class="text-sm text-red-600">{error}</p>
  {/if}

  {#if download}
    <div class="border border-slate-200 dark:border-slate-700 rounded p-4 space-y-3">
      <h2 class="font-medium">{de.profiles.conf}</h2>
      <pre
        class="text-xs overflow-auto bg-slate-100 dark:bg-slate-800 p-3 rounded max-h-48">{download.conf}</pre>
      <button type="button" class="underline text-sm" onclick={saveConf}
        >{de.profiles.downloadFile}</button
      >
      <h2 class="font-medium">{de.profiles.qr}</h2>
      <img
        class="w-48 h-48 bg-white p-2 rounded"
        alt="QR"
        src={`data:image/png;base64,${download.qr_png_base64}`}
      />
    </div>
  {/if}

  {#if profiles.length === 0}
    <p class="text-sm text-slate-500">{de.empty}</p>
  {:else}
    <ul
      class="divide-y divide-slate-200 dark:divide-slate-700 border border-slate-200 dark:border-slate-700 rounded"
    >
      {#each profiles as p (p.id)}
        <li class="px-3 py-3 flex flex-wrap gap-2 items-center">
          <div class="min-w-0">
            <div class="font-medium">{deviceName(p.device_id)} → {nodeName(p.node_id)}</div>
            <div class="text-xs text-slate-500">
              {p.assigned_ipv4 ?? ''}
              {p.revoked_at ? '· widerrufen' : ''}
            </div>
          </div>
          <div class="ml-auto flex gap-3 text-sm">
            {#if p.download_available}
              <button type="button" class="underline" onclick={() => doDownload(p.id)}
                >{de.profiles.download}</button
              >
            {/if}
            {#if !p.revoked_at}
              <button type="button" class="underline text-red-600" onclick={() => onRevoke(p.id)}
                >{de.profiles.revoke}</button
              >
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>
