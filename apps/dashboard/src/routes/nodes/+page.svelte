<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { fetchMe } from '$lib/auth';
  import { api, ApiError, type JoinToken, type Node, type ProvisioningJob } from '$lib/api';
  import Forbidden from '$lib/components/Forbidden.svelte';
  import { de } from '$lib/i18n/de';

  let allowed = $state<boolean | null>(null);
  let tab = $state<'wizard' | 'manual'>('wizard');
  let items = $state<Node[]>([]);
  let error = $state('');
  let busy = $state(false);

  // Wizard
  let host = $state('');
  let sshPort = $state(22);
  let sshUser = $state('root');
  let sshPassword = $state('');
  let sshKey = $state('');
  let role = $state('classic');
  let nodeName = $state('');
  let cidrV4 = $state('10.66.0.0/24');
  let cidrV6 = $state('');
  let job = $state<ProvisioningJob | null>(null);
  let logText = $state('');
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  // Manual
  let tokenRole = $state('classic');
  let tokenName = $state('');
  let ttl = $state(60);
  let token = $state<JoinToken | null>(null);
  let copied = $state('');

  onMount(async () => {
    const me = await fetchMe();
    allowed = me?.role === 'admin';
    if (allowed) await load();
  });

  onDestroy(() => {
    if (pollTimer) clearInterval(pollTimer);
  });

  async function load() {
    error = '';
    try {
      const res = await api.listNodes();
      items = res.items;
    } catch (e) {
      if (e instanceof ApiError && e.status === 403) {
        allowed = false;
        return;
      }
      error = e instanceof ApiError ? e.message : 'Fehler';
    }
  }

  function clearSecrets() {
    sshPassword = '';
    sshKey = '';
  }

  async function onWizardStart(e: Event) {
    e.preventDefault();
    busy = true;
    error = '';
    job = null;
    logText = '';
    try {
      const body: Parameters<typeof api.createProvisioningJob>[0] = {
        host: host.trim(),
        ssh_port: sshPort,
        ssh_user: sshUser.trim(),
        role,
        node_name: nodeName.trim() || undefined,
        tunnel_cidr_v4: cidrV4.trim() || undefined,
        tunnel_cidr_v6: cidrV6.trim() || undefined
      };
      if (sshKey.trim()) {
        body.private_key_pem = sshKey;
      } else {
        body.password = sshPassword;
      }
      job = await api.createProvisioningJob(body);
      clearSecrets();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
      clearSecrets();
    } finally {
      busy = false;
    }
  }

  async function onConfirmHostKey() {
    if (!job?.host_key_fingerprint) return;
    busy = true;
    error = '';
    try {
      job = await api.confirmHostKey(job.id, job.host_key_fingerprint);
      startPolling(job.id);
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
    } finally {
      busy = false;
    }
  }

  function startPolling(id: string) {
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = setInterval(async () => {
      try {
        const j = await api.getProvisioningJob(id);
        job = j;
        logText = j.log_text;
        if (j.status === 'succeeded' || j.status === 'failed' || j.status === 'cancelled') {
          if (pollTimer) clearInterval(pollTimer);
          pollTimer = null;
          await load();
        }
      } catch {
        /* ignore transient */
      }
    }, 1500);
  }

  async function onDelete(id: string) {
    if (!confirm(de.nodes.confirmDelete)) return;
    busy = true;
    try {
      await api.deleteNode(id);
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
    } finally {
      busy = false;
    }
  }

  async function onToken(e: Event) {
    e.preventDefault();
    busy = true;
    token = null;
    try {
      token = await api.createJoinToken({
        role: tokenRole,
        node_name: tokenName.trim() || undefined,
        ttl_minutes: ttl
      });
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
    } finally {
      busy = false;
    }
  }

  async function copy(text: string, key: string) {
    await navigator.clipboard.writeText(text);
    copied = key;
    setTimeout(() => {
      if (copied === key) copied = '';
    }, 2000);
  }
</script>

<svelte:head>
  <title>{de.nodes.title} · {de.brand}</title>
</svelte:head>

{#if allowed === null}
  <p class="text-sm text-slate-500">{de.loading}</p>
{:else if !allowed}
  <Forbidden />
{:else}
  <section class="space-y-8">
    <h1 class="text-2xl font-semibold">{de.nodes.title}</h1>

    <div class="flex gap-4 text-sm border-b border-slate-200 dark:border-slate-700 pb-2">
      <button
        type="button"
        class={tab === 'wizard' ? 'font-semibold underline' : 'hover:underline'}
        onclick={() => (tab = 'wizard')}>{de.nodes.tabWizard}</button
      >
      <button
        type="button"
        class={tab === 'manual' ? 'font-semibold underline' : 'hover:underline'}
        onclick={() => (tab = 'manual')}>{de.nodes.tabManual}</button
      >
    </div>

    {#if tab === 'wizard'}
      <p class="text-sm text-slate-600 dark:text-slate-300">{de.nodes.wizardHint}</p>

      <form class="flex flex-wrap gap-3 items-end" onsubmit={onWizardStart}>
        <label class="space-y-1">
          <span class="text-sm">{de.nodes.host}</span>
          <input
            class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
            bind:value={host}
            required
            placeholder="203.0.113.10"
          />
        </label>
        <label class="space-y-1">
          <span class="text-sm">{de.nodes.sshPort}</span>
          <input
            class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800 w-24"
            type="number"
            min="1"
            max="65535"
            bind:value={sshPort}
          />
        </label>
        <label class="space-y-1">
          <span class="text-sm">{de.nodes.sshUser}</span>
          <input
            class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
            bind:value={sshUser}
            required
          />
        </label>
        <label class="space-y-1">
          <span class="text-sm">{de.nodes.sshPassword}</span>
          <input
            class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
            type="password"
            autocomplete="new-password"
            bind:value={sshPassword}
          />
        </label>
        <label class="space-y-1 w-full max-w-xl">
          <span class="text-sm">{de.nodes.sshKey}</span>
          <textarea
            class="block w-full border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800 font-mono text-xs"
            rows="3"
            bind:value={sshKey}
          ></textarea>
        </label>
        <label class="space-y-1">
          <span class="text-sm">{de.role}</span>
          <select
            class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
            bind:value={role}
          >
            <option value="classic">classic</option>
            <option value="ingress">ingress</option>
            <option value="egress">egress</option>
          </select>
        </label>
        <label class="space-y-1">
          <span class="text-sm">{de.name}</span>
          <input
            class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
            bind:value={nodeName}
            placeholder="edge-1"
          />
        </label>
        <label class="space-y-1">
          <span class="text-sm">{de.nodes.cidrV4}</span>
          <input
            class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
            bind:value={cidrV4}
          />
        </label>
        <button
          class="bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 rounded px-4 py-2"
          type="submit"
          disabled={busy}>{de.nodes.startInstall}</button
        >
      </form>

      {#if job}
        <div class="border border-slate-200 dark:border-slate-700 rounded p-4 space-y-3">
          <div class="text-sm">
            Status: <strong>{job.status}</strong> · {job.host}:{job.ssh_port}
          </div>
          {#if job.status === 'awaiting_hostkey' && job.host_key_fingerprint}
            <h2 class="font-medium">{de.nodes.hostKeyTitle}</h2>
            <p class="text-sm text-slate-600 dark:text-slate-300">{de.nodes.hostKeyHint}</p>
            <code class="block break-all bg-slate-100 dark:bg-slate-800 p-2 rounded text-sm"
              >{job.host_key_fingerprint}</code
            >
            <button
              type="button"
              class="bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 rounded px-4 py-2"
              onclick={onConfirmHostKey}
              disabled={busy}>{de.nodes.confirmHostKey}</button
            >
          {/if}
          {#if logText || job.log_text}
            <h2 class="font-medium">{de.nodes.liveLog}</h2>
            <pre
              class="text-xs overflow-auto bg-slate-100 dark:bg-slate-800 p-3 rounded max-h-64 whitespace-pre-wrap">{logText ||
                job.log_text}</pre>
          {/if}
          {#if job.error_message}
            <p class="text-sm text-red-600">{job.error_message}</p>
          {/if}
        </div>
      {/if}
    {:else}
      <form
        class="space-y-3 border border-slate-200 dark:border-slate-700 rounded p-4"
        onsubmit={onToken}
      >
        <h2 class="font-medium">{de.nodes.joinToken}</h2>
        <div class="flex flex-wrap gap-3 items-end">
          <label class="space-y-1">
            <span class="text-sm">{de.role}</span>
            <select
              class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
              bind:value={tokenRole}
            >
              <option value="classic">classic</option>
              <option value="ingress">ingress</option>
              <option value="egress">egress</option>
            </select>
          </label>
          <label class="space-y-1">
            <span class="text-sm">{de.name}</span>
            <input
              class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
              bind:value={tokenName}
            />
          </label>
          <label class="space-y-1">
            <span class="text-sm">{de.nodes.ttl}</span>
            <input
              class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800 w-24"
              type="number"
              min="1"
              max="1440"
              bind:value={ttl}
            />
          </label>
          <button
            class="bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 rounded px-4 py-2"
            type="submit"
            disabled={busy}>{de.nodes.joinToken}</button
          >
        </div>
        {#if token}
          <p class="text-sm text-amber-700 dark:text-amber-300">{de.nodes.tokenOnce}</p>
          <div class="space-y-2 text-sm">
            <div class="flex gap-2 items-start">
              <code class="flex-1 break-all bg-slate-100 dark:bg-slate-800 p-2 rounded"
                >{token.token}</code
              >
              <button
                type="button"
                class="underline shrink-0"
                onclick={() => copy(token!.token, 't')}
                >{copied === 't' ? de.copied : de.copy}</button
              >
            </div>
            <div>
              <div class="text-xs text-slate-500 mb-1">{de.nodes.installCmd}</div>
              <div class="flex gap-2 items-start">
                <code class="flex-1 break-all bg-slate-100 dark:bg-slate-800 p-2 rounded"
                  >{token.install_command}</code
                >
                <button
                  type="button"
                  class="underline shrink-0"
                  onclick={() => copy(token!.install_command, 'i')}
                  >{copied === 'i' ? de.copied : de.copy}</button
                >
              </div>
            </div>
          </div>
        {/if}
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
        {#each items as n (n.id)}
          <li class="px-3 py-3 flex flex-wrap gap-2 items-center">
            <div>
              <div class="font-medium">{n.name}</div>
              <div class="text-xs text-slate-500">
                {n.role} · {n.status}{#if n.public_ip}
                  · {n.public_ip}{/if}
              </div>
            </div>
            <button
              type="button"
              class="ml-auto text-sm underline text-red-600"
              onclick={() => onDelete(n.id)}>{de.delete}</button
            >
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}
