<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchMe } from '$lib/auth';
  import { api, ApiError, type AuditEvent } from '$lib/api';
  import Forbidden from '$lib/components/Forbidden.svelte';
  import { de } from '$lib/i18n/de';

  let allowed = $state<boolean | null>(null);
  let items = $state<AuditEvent[]>([]);
  let next = $state<string | null>(null);
  let error = $state('');
  let busy = $state(false);

  onMount(async () => {
    const me = await fetchMe();
    allowed = me?.role === 'admin';
    if (allowed) await load(null);
  });

  async function load(cursor: string | null) {
    busy = true;
    error = '';
    try {
      const res = await api.listAudit(cursor);
      if (cursor) {
        items = [...items, ...res.items];
      } else {
        items = res.items;
      }
      next = res.next_cursor;
    } catch (e) {
      if (e instanceof ApiError && e.status === 403) {
        allowed = false;
        return;
      }
      error = e instanceof ApiError ? e.message : 'Fehler';
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head>
  <title>{de.audit.title} · {de.brand}</title>
</svelte:head>

{#if allowed === null}
  <p class="text-sm text-slate-500">{de.loading}</p>
{:else if !allowed}
  <Forbidden />
{:else}
  <section class="space-y-4">
    <h1 class="text-2xl font-semibold">{de.audit.title}</h1>
    {#if error}
      <p class="text-sm text-red-600">{error}</p>
    {/if}
    {#if items.length === 0}
      <p class="text-sm text-slate-500">{de.empty}</p>
    {:else}
      <div class="overflow-x-auto border border-slate-200 dark:border-slate-700 rounded">
        <table class="w-full text-sm text-left">
          <thead class="bg-slate-100 dark:bg-slate-800">
            <tr>
              <th class="px-3 py-2">{de.audit.time}</th>
              <th class="px-3 py-2">{de.audit.action}</th>
              <th class="px-3 py-2">{de.audit.actor}</th>
              <th class="px-3 py-2">{de.audit.resource}</th>
            </tr>
          </thead>
          <tbody>
            {#each items as e (e.id)}
              <tr class="border-t border-slate-200 dark:border-slate-700">
                <td class="px-3 py-2 whitespace-nowrap">{e.created_at}</td>
                <td class="px-3 py-2">{e.action}</td>
                <td class="px-3 py-2 font-mono text-xs">{e.actor_user_id ?? '—'}</td>
                <td class="px-3 py-2 font-mono text-xs">
                  {e.resource_type ?? ''}
                  {e.resource_id ?? ''}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
    {#if next}
      <button type="button" class="underline text-sm" disabled={busy} onclick={() => load(next)}
        >{de.audit.loadMore}</button
      >
    {/if}
  </section>
{/if}
