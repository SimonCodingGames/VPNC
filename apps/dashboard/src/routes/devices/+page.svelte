<script lang="ts">
  import { onMount } from 'svelte';
  import { api, ApiError, type Device } from '$lib/api';
  import { de } from '$lib/i18n/de';
  import { formatDate } from '$lib/format';
  import { confirmAction, toast } from '$lib/ui.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Alert from '$lib/components/Alert.svelte';
  import LoadingState from '$lib/components/LoadingState.svelte';

  let items = $state<Device[]>([]);
  let loaded = $state(false);
  let name = $state('');
  let error = $state('');
  let busy = $state(false);
  let renamingId = $state<string | null>(null);
  let renameValue = $state('');
  let nameInput = $state<HTMLInputElement | null>(null);

  async function load() {
    error = '';
    try {
      const res = await api.listDevices();
      items = res.items;
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
    if (!name.trim()) return;
    busy = true;
    error = '';
    try {
      const created = await api.createDevice(name.trim());
      name = '';
      toast(`„${created.name}“ angelegt`);
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : de.genericError;
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
      error = err instanceof ApiError ? err.message : de.genericError;
    } finally {
      busy = false;
    }
  }

  async function onDelete(d: Device) {
    const ok = await confirmAction({
      title: de.devices.confirmDelete,
      message: `„${d.name}“ – ${de.devices.confirmDeleteText}`,
      confirmLabel: de.delete,
      danger: true
    });
    if (!ok) return;
    busy = true;
    try {
      await api.deleteDevice(d.id);
      toast(`„${d.name}“ gelöscht`);
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : de.genericError;
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head>
  <title>{de.devices.title} · {de.brand}</title>
</svelte:head>

<PageHeader title={de.devices.title} subtitle={de.devices.subtitle} />

<div class="space-y-6">
  <form class="card flex flex-col gap-3 p-4 sm:flex-row sm:items-end" onsubmit={onCreate}>
    <div class="field flex-1">
      <label class="label" for="device-name">{de.devices.new}</label>
      <div class="relative">
        <Icon
          name="laptop"
          size={16}
          class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-ink-400"
        />
        <input
          id="device-name"
          bind:this={nameInput}
          class="input pl-9"
          bind:value={name}
          placeholder={de.devices.namePlaceholder}
          required
        />
      </div>
    </div>
    <button class="btn-primary" type="submit" disabled={busy || !name.trim()}>
      <Icon name="plus" size={16} />
      {de.create}
    </button>
  </form>

  {#if error}
    <Alert tone="danger">{error}</Alert>
  {/if}

  <div class="card overflow-hidden">
    {#if !loaded}
      <LoadingState />
    {:else if items.length === 0}
      <EmptyState icon="laptop" title={de.devices.emptyTitle} text={de.devices.emptyText}>
        <button type="button" class="btn-primary" onclick={() => nameInput?.focus()}>
          <Icon name="plus" size={16} />
          {de.devices.new}
        </button>
      </EmptyState>
    {:else}
      <div class="overflow-x-auto">
        <table class="table">
          <thead>
            <tr>
              <th>{de.name}</th>
              <th class="hidden md:table-cell">ID</th>
              <th class="hidden sm:table-cell">{de.created}</th>
              <th class="text-right"><span class="sr-only">{de.actions}</span></th>
            </tr>
          </thead>
          <tbody>
            {#each items as d (d.id)}
              <tr>
                <td>
                  {#if renamingId === d.id}
                    <form
                      class="flex items-center gap-2"
                      onsubmit={(e) => {
                        e.preventDefault();
                        void onRename(d.id);
                      }}
                    >
                      <!-- svelte-ignore a11y_autofocus -->
                      <input
                        class="input max-w-xs py-1.5"
                        bind:value={renameValue}
                        aria-label={de.rename}
                        autofocus
                        onkeydown={(e) => e.key === 'Escape' && (renamingId = null)}
                      />
                      <button type="submit" class="btn-primary btn-sm" disabled={busy}>
                        <Icon name="check" size={14} />
                        {de.save}
                      </button>
                      <button
                        type="button"
                        class="btn-ghost btn-sm"
                        onclick={() => (renamingId = null)}>{de.cancel}</button
                      >
                    </form>
                  {:else}
                    <div class="flex items-center gap-3">
                      <span
                        class="hidden h-9 w-9 shrink-0 place-items-center rounded-lg sm:grid bg-ink-100 text-ink-600 dark:bg-ink-800 dark:text-ink-300"
                      >
                        <Icon name="laptop" size={17} />
                      </span>
                      <span class="font-medium text-ink-900 dark:text-white">{d.name}</span>
                    </div>
                  {/if}
                </td>
                <td class="hidden font-mono text-xs text-ink-500 md:table-cell">{d.id}</td>
                <td class="hidden whitespace-nowrap text-ink-500 sm:table-cell"
                  >{formatDate(d.created_at)}</td
                >
                <td class="text-right">
                  {#if renamingId !== d.id}
                    <div class="inline-flex gap-1">
                      <button
                        type="button"
                        class="btn-icon"
                        title={de.rename}
                        aria-label={`${de.rename}: ${d.name}`}
                        onclick={() => {
                          renamingId = d.id;
                          renameValue = d.name;
                        }}
                      >
                        <Icon name="pencil" size={16} />
                      </button>
                      <button
                        type="button"
                        class="btn-icon hover:!bg-red-50 hover:!text-red-600 dark:hover:!bg-red-500/10 dark:hover:!text-red-400"
                        title={de.delete}
                        aria-label={`${de.delete}: ${d.name}`}
                        onclick={() => onDelete(d)}
                      >
                        <Icon name="trash" size={16} />
                      </button>
                    </div>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <div
        class="border-t border-ink-100 px-5 py-3 text-xs text-ink-500 dark:border-ink-800 dark:text-ink-400"
      >
        {items.length}
        {de.devices.count}
      </div>
    {/if}
  </div>
</div>
