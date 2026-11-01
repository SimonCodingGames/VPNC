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
  import { formatDate, roleTone } from '$lib/format';
  import { confirmAction, toast } from '$lib/ui.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Alert from '$lib/components/Alert.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import LoadingState from '$lib/components/LoadingState.svelte';

  let profiles = $state<Profile[]>([]);
  let devices = $state<Device[]>([]);
  let nodes = $state<Node[]>([]);
  let loaded = $state(false);
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
      error = e instanceof ApiError ? e.message : de.genericError;
    } finally {
      loaded = true;
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
      error = err instanceof ApiError ? err.message : de.genericError;
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

  function closeDownload() {
    // Drop the private key from memory as soon as the dialog closes.
    download = null;
    downloadId = null;
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
    toast(de.profiles.downloadFile);
  }

  async function onRevoke(p: Profile) {
    const ok = await confirmAction({
      title: de.profiles.confirmRevokeTitle,
      message: de.profiles.confirmRevoke,
      confirmLabel: de.profiles.revoke,
      danger: true
    });
    if (!ok) return;
    busy = true;
    try {
      await api.revokeProfile(p.id);
      if (downloadId === p.id) closeDownload();
      toast(`${de.profiles.title}: ${de.profiles.revoked}`);
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : de.genericError;
    } finally {
      busy = false;
    }
  }

  function deviceName(id: string) {
    return devices.find((d) => d.id === id)?.name ?? id;
  }

  function node(id: string | null) {
    if (!id) return null;
    return nodes.find((n) => n.id === id) ?? null;
  }
</script>

<svelte:head>
  <title>{de.profiles.title} · {de.brand}</title>
</svelte:head>

<PageHeader title={de.profiles.title} subtitle={de.profiles.subtitle} />

<div class="space-y-6">
  <form class="card p-4" onsubmit={onCreate}>
    <div class="grid gap-3 sm:grid-cols-[1fr_1fr_auto] sm:items-end">
      <div class="field">
        <label class="label" for="p-device">{de.profiles.device}</label>
        <select id="p-device" class="input" bind:value={deviceId} required>
          {#each devices as d (d.id)}
            <option value={d.id}>{d.name}</option>
          {/each}
        </select>
      </div>
      <div class="field">
        <label class="label" for="p-node">{de.profiles.node}</label>
        <select id="p-node" class="input" bind:value={nodeId} required>
          {#each nodes as n (n.id)}
            <option value={n.id}>{n.name} ({n.role})</option>
          {/each}
        </select>
      </div>
      <button class="btn-primary" type="submit" disabled={busy || !deviceId || !nodeId}>
        {#if busy}<Spinner size={16} />{:else}<Icon name="plus" size={16} />{/if}
        {de.profiles.new}
      </button>
    </div>
    {#if loaded && devices.length === 0}
      <p class="hint mt-3">{de.profiles.needDevice}</p>
    {:else if loaded && nodes.length === 0}
      <p class="hint mt-3">{de.profiles.needNode}</p>
    {/if}
  </form>

  <Alert tone="warning">{de.profiles.downloadOnce}</Alert>

  {#if error}
    <Alert tone="danger">{error}</Alert>
  {/if}

  <div class="card overflow-hidden">
    {#if !loaded}
      <LoadingState />
    {:else if profiles.length === 0}
      <EmptyState icon="key" title={de.profiles.emptyTitle} text={de.profiles.emptyText} />
    {:else}
      <div class="overflow-x-auto">
        <table class="table">
          <thead>
            <tr>
              <th>{de.profiles.device}</th>
              <th>{de.profiles.node}</th>
              <th class="hidden md:table-cell">{de.profiles.address}</th>
              <th>{de.status}</th>
              <th class="hidden lg:table-cell">{de.created}</th>
              <th class="text-right"><span class="sr-only">{de.actions}</span></th>
            </tr>
          </thead>
          <tbody>
            {#each profiles as p (p.id)}
              {@const n = node(p.node_id)}
              <tr class={p.revoked_at ? 'opacity-60' : ''}>
                <td>
                  <div class="flex items-center gap-3">
                    <span
                      class="hidden h-9 w-9 shrink-0 place-items-center rounded-lg sm:grid bg-ink-100 text-ink-600 dark:bg-ink-800 dark:text-ink-300"
                    >
                      <Icon name="key" size={16} />
                    </span>
                    <div>
                      <div class="whitespace-nowrap font-medium text-ink-900 dark:text-white">
                        {deviceName(p.device_id)}
                      </div>
                      <div class="text-xs uppercase tracking-wide text-ink-400">{p.kind}</div>
                    </div>
                  </div>
                </td>
                <td>
                  {#if n}
                    <div class="flex items-center gap-2">
                      <span class="whitespace-nowrap font-mono text-sm">{n.name}</span>
                      <Badge tone={roleTone(n.role)}>{n.role}</Badge>
                    </div>
                  {:else}
                    <span class="font-mono text-xs text-ink-400">{p.node_id ?? '—'}</span>
                  {/if}
                </td>
                <td class="hidden md:table-cell">
                  <div class="font-mono text-xs text-ink-600 dark:text-ink-300">
                    {p.assigned_ipv4 ?? '—'}
                  </div>
                  {#if p.assigned_ipv6}
                    <div class="font-mono text-xs text-ink-400">{p.assigned_ipv6}</div>
                  {/if}
                </td>
                <td>
                  {#if p.revoked_at}
                    <Badge tone="danger">{de.profiles.revoked}</Badge>
                  {:else}
                    <Badge tone="success" dot>{de.profiles.active}</Badge>
                  {/if}
                </td>
                <td class="hidden whitespace-nowrap text-ink-500 lg:table-cell"
                  >{formatDate(p.created_at)}</td
                >
                <td class="whitespace-nowrap text-right">
                  <div class="inline-flex gap-1">
                    {#if p.download_available}
                      <button
                        type="button"
                        class="btn-secondary btn-sm"
                        title={de.profiles.download}
                        aria-label={de.profiles.download}
                        disabled={busy}
                        onclick={() => doDownload(p.id)}
                      >
                        <Icon name="download" size={14} />
                        <span class="hidden 2xl:inline">{de.profiles.download}</span>
                      </button>
                    {/if}
                    {#if !p.revoked_at}
                      <button
                        type="button"
                        class="btn-danger-ghost btn-sm"
                        title={de.profiles.revoke}
                        aria-label={de.profiles.revoke}
                        disabled={busy}
                        onclick={() => onRevoke(p)}
                      >
                        <Icon name="ban" size={14} />
                        <span class="hidden 2xl:inline">{de.profiles.revoke}</span>
                      </button>
                    {/if}
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
</div>

<Modal open={download !== null} title={de.profiles.readyTitle} onclose={closeDownload} size="lg">
  {#if download}
    <div class="space-y-5">
      <Alert tone="warning">{de.profiles.downloadOnce}</Alert>
      <div class="grid gap-5 md:grid-cols-[auto_1fr]">
        <div class="flex flex-col items-center gap-2">
          <div class="rounded-xl border border-ink-200 bg-white p-3 shadow-sm dark:border-ink-700">
            <img
              class="h-48 w-48"
              alt={de.profiles.qr}
              src={`data:image/png;base64,${download.qr_png_base64}`}
            />
          </div>
          <p class="hint flex items-center gap-1.5">
            <Icon name="qr" size={13} />
            {de.profiles.qrHint}
          </p>
        </div>
        <div class="min-w-0 space-y-1.5">
          <div class="label">{de.profiles.conf}</div>
          <pre class="code-block max-h-64 overflow-auto p-3">{download.conf}</pre>
        </div>
      </div>
    </div>
  {/if}
  {#snippet footer()}
    <button type="button" class="btn-secondary" onclick={closeDownload}>{de.close}</button>
    <button type="button" class="btn-primary" onclick={saveConf}>
      <Icon name="download" size={16} />
      {de.profiles.downloadFile}
    </button>
  {/snippet}
</Modal>
