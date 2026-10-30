<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchMe } from '$lib/auth';
  import { api, ApiError, type JoinToken, type Node } from '$lib/api';
  import Forbidden from '$lib/components/Forbidden.svelte';
  import { de } from '$lib/i18n/de';

  let allowed = $state<boolean | null>(null);
  let items = $state<Node[]>([]);
  let error = $state('');
  let busy = $state(false);
  let name = $state('');
  let role = $state('classic');
  let cidrV4 = $state('10.66.0.0/24');
  let cidrV6 = $state('');
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

  async function onCreate(e: Event) {
    e.preventDefault();
    busy = true;
    try {
      await api.createNode({
        name: name.trim(),
        role,
        tunnel_cidr_v4: cidrV4.trim() || undefined,
        tunnel_cidr_v6: cidrV6.trim() || undefined
      });
      name = '';
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
    } finally {
      busy = false;
    }
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

    <form class="flex flex-wrap gap-3 items-end" onsubmit={onCreate}>
      <label class="space-y-1">
        <span class="text-sm">{de.name}</span>
        <input
          class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
          bind:value={name}
          required
        />
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
        <span class="text-sm">{de.nodes.cidrV4}</span>
        <input
          class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
          bind:value={cidrV4}
          placeholder="10.66.0.0/24"
        />
      </label>
      <label class="space-y-1">
        <span class="text-sm">{de.nodes.cidrV6}</span>
        <input
          class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
          bind:value={cidrV6}
        />
      </label>
      <button
        class="bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 rounded px-4 py-2"
        type="submit"
        disabled={busy}>{de.nodes.new}</button
      >
    </form>

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
            <button type="button" class="underline shrink-0" onclick={() => copy(token!.token, 't')}
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
          <div>
            <div class="text-xs text-slate-500 mb-1">{de.nodes.dockerCmd}</div>
            <div class="flex gap-2 items-start">
              <code class="flex-1 break-all bg-slate-100 dark:bg-slate-800 p-2 rounded"
                >{token.docker_command}</code
              >
              <button
                type="button"
                class="underline shrink-0"
                onclick={() => copy(token!.docker_command, 'd')}
                >{copied === 'd' ? de.copied : de.copy}</button
              >
            </div>
          </div>
        </div>
      {/if}
    </form>

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
