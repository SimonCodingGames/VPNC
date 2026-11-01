<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchMe } from '$lib/auth';
  import { api, ApiError, type AuditEvent } from '$lib/api';
  import { de } from '$lib/i18n/de';
  import { formatDate, formatRelative, shortId, type Tone } from '$lib/format';
  import Forbidden from '$lib/components/Forbidden.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Alert from '$lib/components/Alert.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import LoadingState from '$lib/components/LoadingState.svelte';

  let allowed = $state<boolean | null>(null);
  let items = $state<AuditEvent[]>([]);
  let loaded = $state(false);
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
      error = e instanceof ApiError ? e.message : de.genericError;
    } finally {
      busy = false;
      loaded = true;
    }
  }

  function actionTone(action: string): Tone {
    const a = action.toLowerCase();
    if (/(delete|revoke|disable|fail|remove)/.test(a)) return 'danger';
    if (/(create|add|register|enable|succeed)/.test(a)) return 'success';
    if (/(login|logout|auth)/.test(a)) return 'info';
    if (/(update|rename|patch|reset|confirm)/.test(a)) return 'warning';
    return 'neutral';
  }
</script>

<svelte:head>
  <title>{de.audit.title} · {de.brand}</title>
</svelte:head>

{#if allowed === null}
  <LoadingState />
{:else if !allowed}
  <Forbidden />
{:else}
  <PageHeader title={de.audit.title} subtitle={de.audit.subtitle}>
    {#snippet actions()}
      <button type="button" class="btn-secondary" disabled={busy} onclick={() => load(null)}>
        <Icon name="refresh" size={16} />
        {de.refresh}
      </button>
    {/snippet}
  </PageHeader>

  <div class="space-y-6">
    {#if error}
      <Alert tone="danger">{error}</Alert>
    {/if}

    <div class="card overflow-hidden">
      {#if !loaded}
        <LoadingState />
      {:else if items.length === 0}
        <EmptyState icon="scroll" title={de.audit.emptyTitle} />
      {:else}
        <div class="overflow-x-auto">
          <table class="table">
            <thead>
              <tr>
                <th>{de.audit.time}</th>
                <th>{de.audit.action}</th>
                <th>{de.audit.actor}</th>
                <th>{de.audit.resource}</th>
              </tr>
            </thead>
            <tbody>
              {#each items as e (e.id)}
                <tr>
                  <td class="whitespace-nowrap">
                    <div class="text-ink-900 dark:text-ink-100">{formatRelative(e.created_at)}</div>
                    <div class="text-xs text-ink-400" title={e.created_at}>
                      {formatDate(e.created_at)}
                    </div>
                  </td>
                  <td>
                    <Badge tone={actionTone(e.action)}>
                      <span class="font-mono">{e.action}</span>
                    </Badge>
                  </td>
                  <td class="font-mono text-xs text-ink-600 dark:text-ink-300">
                    {#if e.actor_user_id}
                      <span title={e.actor_user_id}>{shortId(e.actor_user_id)}</span>
                    {:else}
                      <span class="text-ink-400">{de.audit.system}</span>
                    {/if}
                  </td>
                  <td class="font-mono text-xs">
                    {#if e.resource_type}
                      <span class="text-ink-500">{e.resource_type}</span>
                    {/if}
                    {#if e.resource_id}
                      <span class="text-ink-700 dark:text-ink-200" title={e.resource_id}
                        >{shortId(e.resource_id)}</span
                      >
                    {/if}
                    {#if !e.resource_type && !e.resource_id}<span class="text-ink-400">—</span>{/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        {#if next}
          <div class="border-t border-ink-100 p-3 text-center dark:border-ink-800">
            <button type="button" class="btn-ghost" disabled={busy} onclick={() => load(next)}>
              {#if busy}<Spinner size={14} />{:else}<Icon name="chevronDown" size={16} />{/if}
              {de.audit.loadMore}
            </button>
          </div>
        {/if}
      {/if}
    </div>
  </div>
{/if}
