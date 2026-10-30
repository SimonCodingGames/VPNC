<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchMe } from '$lib/auth';
  import { api, ApiError, type Settings } from '$lib/api';
  import Forbidden from '$lib/components/Forbidden.svelte';
  import { de } from '$lib/i18n/de';

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
      error = e instanceof ApiError ? e.message : 'Fehler';
    }
  });
</script>

<svelte:head>
  <title>{de.settings.title} · {de.brand}</title>
</svelte:head>

{#if allowed === null}
  <p class="text-sm text-slate-500">{de.loading}</p>
{:else if !allowed}
  <Forbidden />
{:else}
  <section class="space-y-4">
    <h1 class="text-2xl font-semibold">{de.settings.title}</h1>
    <p class="text-sm text-slate-600 dark:text-slate-300">{de.settings.readOnly}</p>
    {#if error}
      <p class="text-sm text-red-600">{error}</p>
    {/if}
    {#if settings}
      <dl class="grid gap-3 text-sm sm:grid-cols-2">
        <div>
          <dt class="text-slate-500">Version</dt>
          <dd>{settings.version}</dd>
        </div>
        <div>
          <dt class="text-slate-500">HTTP</dt>
          <dd class="font-mono">{settings.http_bind}</dd>
        </div>
        <div>
          <dt class="text-slate-500">gRPC</dt>
          <dd class="font-mono">{settings.grpc_bind}</dd>
        </div>
        <div>
          <dt class="text-slate-500">Public URL</dt>
          <dd class="font-mono">{settings.public_url ?? '—'}</dd>
        </div>
        <div>
          <dt class="text-slate-500">Public gRPC</dt>
          <dd class="font-mono">{settings.public_grpc ?? '—'}</dd>
        </div>
        <div>
          <dt class="text-slate-500">Cookie Secure</dt>
          <dd>{settings.cookie_secure ? 'ja' : 'nein'}</dd>
        </div>
        <div>
          <dt class="text-slate-500">Trust X-Forwarded-*</dt>
          <dd>{settings.trust_forwarded_headers ? 'ja' : 'nein'}</dd>
        </div>
        <div>
          <dt class="text-slate-500">Static Dashboard</dt>
          <dd>{settings.static_dir_configured ? 'aktiv' : 'nicht konfiguriert'}</dd>
        </div>
      </dl>
      <p class="text-xs text-slate-500">{settings.note}</p>
    {/if}
  </section>
{/if}
