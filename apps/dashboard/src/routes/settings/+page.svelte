<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchMe } from '$lib/auth';
  import { api, ApiError, type Settings } from '$lib/api';
  import { de } from '$lib/i18n/de';
  import Forbidden from '$lib/components/Forbidden.svelte';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import Alert from '$lib/components/Alert.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import LoadingState from '$lib/components/LoadingState.svelte';

  let allowed = $state<boolean | null>(null);
  let settings = $state<Settings | null>(null);
  let error = $state('');

  onMount(async () => {
    const me = await fetchMe();
    allowed = me?.role === 'admin';
    if (!allowed) return;
    try {
      settings = await api.getSettings();
    } catch (e) {
      if (e instanceof ApiError && e.status === 403) {
        allowed = false;
        return;
      }
      error = e instanceof ApiError ? e.message : de.genericError;
    }
  });

  type Row = { label: string; value: string | null; mono?: boolean };
  type Flag = { label: string; on: boolean; onText?: string; offText?: string };

  const serverRows = $derived<Row[]>(
    settings
      ? [
          { label: 'HTTP', value: settings.http_bind, mono: true },
          { label: 'gRPC', value: settings.grpc_bind, mono: true },
          { label: de.settings.publicUrl, value: settings.public_url, mono: true },
          { label: de.settings.publicGrpc, value: settings.public_grpc, mono: true }
        ]
      : []
  );
  const flags = $derived<Flag[]>(
    settings
      ? [
          { label: de.settings.cookieSecure, on: settings.cookie_secure },
          { label: de.settings.trustForwarded, on: settings.trust_forwarded_headers },
          {
            label: de.settings.staticDashboard,
            on: settings.static_dir_configured,
            onText: de.settings.active,
            offText: de.settings.notConfigured
          }
        ]
      : []
  );
</script>

<svelte:head>
  <title>{de.settings.title} · {de.brand}</title>
</svelte:head>

{#snippet section(title: string, icon: IconName, body: import('svelte').Snippet)}
  <div class="card overflow-hidden">
    <div class="card-header">
      <h2 class="card-title flex items-center gap-2">
        <Icon name={icon} size={16} class="text-ink-400" />
        {title}
      </h2>
    </div>
    {@render body()}
  </div>
{/snippet}

{#if allowed === null}
  <LoadingState />
{:else if !allowed}
  <Forbidden />
{:else}
  <PageHeader title={de.settings.title} subtitle={de.settings.subtitle}>
    {#snippet actions()}
      {#if settings}
        <Badge tone="neutral">
          <Icon name="info" size={12} />
          {de.settings.version}
          <span class="font-mono">{settings.version}</span>
        </Badge>
      {/if}
    {/snippet}
  </PageHeader>

  <div class="space-y-6">
    <Alert tone="info">{de.settings.readOnly}</Alert>

    {#if error}
      <Alert tone="danger">{error}</Alert>
    {/if}

    {#if !settings && !error}
      <LoadingState />
    {:else if settings}
      <div class="grid gap-6 lg:grid-cols-2">
        {#snippet serverBody()}
          <dl class="divide-y divide-ink-100 dark:divide-ink-800">
            {#each serverRows as r (r.label)}
              <div class="flex items-center justify-between gap-4 px-5 py-3.5 text-sm">
                <dt class="text-ink-500 dark:text-ink-400">{r.label}</dt>
                <dd
                  class="truncate text-right {r.mono ? 'font-mono text-xs' : ''} {r.value
                    ? 'text-ink-900 dark:text-ink-100'
                    : 'text-ink-400'}"
                >
                  {r.value ?? '—'}
                </dd>
              </div>
            {/each}
          </dl>
        {/snippet}
        {@render section(de.settings.sectionServer, 'globe', serverBody)}

        {#snippet securityBody()}
          <dl class="divide-y divide-ink-100 dark:divide-ink-800">
            {#each flags as f (f.label)}
              <div class="flex items-center justify-between gap-4 px-5 py-3.5 text-sm">
                <dt class="text-ink-500 dark:text-ink-400">{f.label}</dt>
                <dd>
                  <Badge tone={f.on ? 'success' : 'neutral'} dot>
                    {f.on ? (f.onText ?? de.yes) : (f.offText ?? de.no)}
                  </Badge>
                </dd>
              </div>
            {/each}
          </dl>
        {/snippet}
        {@render section(de.settings.sectionSecurity, 'shield', securityBody)}
      </div>

      {#if settings.note}
        <p class="text-xs text-ink-500 dark:text-ink-400">{settings.note}</p>
      {/if}
    {/if}
  </div>
{/if}
