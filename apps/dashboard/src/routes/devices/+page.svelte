<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError, type Device } from '$lib/api';
  import { de } from '$lib/i18n/de';

  let items = $state<Device[]>([]);
  let name = $state('');
  let error = $state('');
  let busy = $state(false);
  let renamingId = $state<string | null>(null);
  let renameValue = $state('');

  async function load() {
    error = '';
    try {
      const res = await api.listDevices();
      items = res.items;
    } catch (e) {
      error = e instanceof ApiError ? e.message : 'Fehler';
    }
  }

  onMount(() => {
    void load();
  });

  async function onCreate(e: Event) {
    e.preventDefault();
    if (!name.trim()) return;
    busy = true;
    error = '';
    try {
      await api.createDevice(name.trim());
      name = '';
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
    } finally {
      busy = false;
    }
  }

  async function onRename(id: string) {
    if (!renameValue.trim()) return;
    busy = true;
    try {
      await api.renameDevice(id, renameValue.trim());
      renamingId = null;
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
    } finally {
      busy = false;
    }
  }

  async function onDelete(id: string) {
    if (!confirm(de.devices.confirmDelete)) return;
    busy = true;
    try {
      await api.deleteDevice(id);
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : 'Fehler';
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head>
  <title>{de.devices.title} · {de.brand}</title>
</svelte:head>

<section class="space-y-6">
  <h1 class="text-2xl font-semibold">{de.devices.title}</h1>

  <form class="flex flex-wrap gap-2 items-end" onsubmit={onCreate}>
    <label class="space-y-1">
      <span class="text-sm">{de.name}</span>
      <input
        class="block border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
        bind:value={name}
        placeholder={de.devices.namePlaceholder}
        required
      />
    </label>
    <button
      class="bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 rounded px-4 py-2 disabled:opacity-50"
      type="submit"
      disabled={busy}>{de.devices.new}</button
    >
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
      {#each items as d (d.id)}
        <li class="px-3 py-3 flex flex-wrap items-center gap-3">
          {#if renamingId === d.id}
            <input
              class="border border-slate-300 dark:border-slate-600 rounded px-2 py-1 bg-white dark:bg-slate-800"
              bind:value={renameValue}
            />
            <button type="button" class="text-sm underline" onclick={() => onRename(d.id)}
              >{de.save}</button
            >
            <button type="button" class="text-sm underline" onclick={() => (renamingId = null)}
              >{de.cancel}</button
            >
          {:else}
            <span class="font-medium">{d.name}</span>
            <span class="text-xs text-slate-500">{d.id}</span>
            <div class="ml-auto flex gap-3 text-sm">
              <button
                type="button"
                class="underline"
                onclick={() => {
                  renamingId = d.id;
                  renameValue = d.name;
                }}>{de.rename}</button
              >
              <button type="button" class="underline text-red-600" onclick={() => onDelete(d.id)}
                >{de.delete}</button
              >
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>
