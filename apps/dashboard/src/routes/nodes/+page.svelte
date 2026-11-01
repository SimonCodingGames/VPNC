<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { fetchMe } from '$lib/auth';
  import { api, ApiError, type JoinToken, type Node, type ProvisioningJob } from '$lib/api';
  import { de } from '$lib/i18n/de';
  import { formatRelative, roleTone } from '$lib/format';
  import { confirmAction, toast } from '$lib/ui.svelte';
  import Forbidden from '$lib/components/Forbidden.svelte';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Alert from '$lib/components/Alert.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import CopyField from '$lib/components/CopyField.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import LoadingState from '$lib/components/LoadingState.svelte';
  import PasswordInput from '$lib/components/PasswordInput.svelte';

  type Role = 'classic' | 'ingress' | 'egress';
  const roles: { value: Role; icon: IconName }[] = [
    { value: 'classic', icon: 'server' },
    { value: 'ingress', icon: 'globe' },
    { value: 'egress', icon: 'network' }
  ];

  let allowed = $state<boolean | null>(null);
  let addOpen = $state(false);
  let tab = $state<'wizard' | 'manual'>('wizard');
  let items = $state<Node[]>([]);
  let loaded = $state(false);
  let error = $state('');
  let busy = $state(false);

  // Wizard
  let host = $state('');
  let sshPort = $state(22);
  let sshUser = $state('root');
  let sshPassword = $state('');
  let sshKey = $state('');
  let role = $state<Role>('classic');
  let nodeName = $state('');
  let cidrV4 = $state('10.66.0.0/24');
  let cidrV6 = $state('');
  let job = $state<ProvisioningJob | null>(null);
  let logText = $state('');
  let logEl = $state<HTMLPreElement | null>(null);
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  // Manual
  let tokenRole = $state<Role>('classic');
  let tokenName = $state('');
  let ttl = $state(60);
  let token = $state<JoinToken | null>(null);

  const jobDone = $derived(
    job !== null && ['succeeded', 'failed', 'cancelled'].includes(job.status)
  );
  const jobStep = $derived(
    !job
      ? 0
      : job.status === 'awaiting_hostkey' || job.status === 'pending'
        ? 1
        : job.status === 'running'
          ? 2
          : 3
  );

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
      error = e instanceof ApiError ? e.message : de.genericError;
    } finally {
      loaded = true;
    }
  }

  function clearSecrets() {
    sshPassword = '';
    sshKey = '';
  }

  function closeAdd() {
    addOpen = false;
    clearSecrets();
    // One-time token must not linger in memory.
    token = null;
    if (jobDone) {
      job = null;
      logText = '';
    }
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
      error = err instanceof ApiError ? err.message : de.genericError;
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
      error = err instanceof ApiError ? err.message : de.genericError;
    } finally {
      busy = false;
    }
  }

  async function onCancelJob() {
    if (!job) return;
    busy = true;
    try {
      job = await api.cancelProvisioningJob(job.id);
    } catch (err) {
      error = err instanceof ApiError ? err.message : de.genericError;
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
        await tick();
        if (logEl) logEl.scrollTop = logEl.scrollHeight;
        if (j.status === 'succeeded' || j.status === 'failed' || j.status === 'cancelled') {
          if (pollTimer) clearInterval(pollTimer);
          pollTimer = null;
          if (j.status === 'succeeded') toast(`${j.node_name ?? j.host} installiert`);
          await load();
        }
      } catch {
        /* ignore transient */
      }
    }, 1500);
  }

  function resetWizard() {
    job = null;
    logText = '';
    error = '';
  }

  async function onDelete(n: Node) {
    const ok = await confirmAction({
      title: de.nodes.confirmDelete,
      message: `${n.name} – ${de.nodes.confirmDeleteText}`,
      confirmLabel: de.delete,
      danger: true
    });
    if (!ok) return;
    busy = true;
    try {
      await api.deleteNode(n.id);
      toast(`${n.name} gelöscht`);
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : de.genericError;
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
      error = err instanceof ApiError ? err.message : de.genericError;
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head>
  <title>{de.nodes.title} · {de.brand}</title>
</svelte:head>

{#snippet rolePicker(current: Role, set: (r: Role) => void, name: string)}
  <div class="grid gap-2 sm:grid-cols-3" role="radiogroup" aria-label={de.role}>
    {#each roles as r (r.value)}
      {@const active = current === r.value}
      <label
        class="relative flex cursor-pointer items-start gap-3 rounded-lg border p-3 transition {active
          ? 'border-brand-500 bg-brand-50/60 ring-4 ring-brand-500/10 dark:border-brand-400 dark:bg-brand-500/5'
          : 'border-ink-200 hover:border-ink-300 dark:border-ink-700 dark:hover:border-ink-600'}"
      >
        <input
          type="radio"
          class="sr-only"
          {name}
          value={r.value}
          checked={active}
          onchange={() => set(r.value)}
        />
        <Icon
          name={r.icon}
          size={18}
          class={active ? 'mt-0.5 text-brand-600 dark:text-brand-400' : 'mt-0.5 text-ink-400'}
        />
        <span class="min-w-0">
          <span class="block text-sm font-semibold capitalize">{r.value}</span>
          <span class="block text-xs text-ink-500 dark:text-ink-400"
            >{de.nodes.roleHint[r.value]}</span
          >
        </span>
      </label>
    {/each}
  </div>
{/snippet}

{#if allowed === null}
  <LoadingState />
{:else if !allowed}
  <Forbidden />
{:else}
  <PageHeader title={de.nodes.title} subtitle={de.nodes.subtitle}>
    {#snippet actions()}
      <button type="button" class="btn-secondary" onclick={() => load()} aria-label={de.refresh}>
        <Icon name="refresh" size={16} />
      </button>
      <button type="button" class="btn-primary" onclick={() => (addOpen = true)}>
        <Icon name="plus" size={16} />
        {de.nodes.add}
      </button>
    {/snippet}
  </PageHeader>

  <div class="space-y-6">
    {#if job && !addOpen}
      <button
        type="button"
        class="card flex w-full items-center gap-3 px-4 py-3 text-left text-sm transition hover:border-brand-300"
        onclick={() => (addOpen = true)}
      >
        {#if jobDone}<Icon name="check" size={16} class="text-brand-500" />{:else}<Spinner
            size={16}
            class="text-brand-500"
          />{/if}
        <span class="flex-1">
          <strong>{de.nodes.jobTitle}</strong> · {job.host}:{job.ssh_port}
        </span>
        <StatusBadge status={job.status} />
        <Icon name="chevronRight" size={16} class="text-ink-400" />
      </button>
    {/if}

    {#if error && !addOpen}
      <Alert tone="danger">{error}</Alert>
    {/if}

    <div class="card overflow-hidden">
      {#if !loaded}
        <LoadingState />
      {:else if items.length === 0}
        <EmptyState icon="server" title={de.nodes.emptyTitle} text={de.nodes.emptyText}>
          <button type="button" class="btn-primary" onclick={() => (addOpen = true)}>
            <Icon name="plus" size={16} />
            {de.nodes.add}
          </button>
        </EmptyState>
      {:else}
        <div class="overflow-x-auto">
          <table class="table">
            <thead>
              <tr>
                <th>{de.name}</th>
                <th>{de.role}</th>
                <th class="hidden md:table-cell">{de.nodes.publicIp}</th>
                <th>{de.status}</th>
                <th class="hidden lg:table-cell">{de.nodes.lastSeen}</th>
                <th class="text-right"><span class="sr-only">{de.actions}</span></th>
              </tr>
            </thead>
            <tbody>
              {#each items as n (n.id)}
                <tr>
                  <td>
                    <div class="flex items-center gap-3">
                      <span
                        class="hidden h-9 w-9 shrink-0 place-items-center rounded-lg sm:grid bg-ink-100 text-ink-600 dark:bg-ink-800 dark:text-ink-300"
                      >
                        <Icon name="server" size={16} />
                      </span>
                      <div>
                        <div
                          class="whitespace-nowrap font-mono text-sm font-medium text-ink-900 dark:text-white"
                        >
                          {n.name}
                        </div>
                        <div class="font-mono text-xs text-ink-400">{n.id.slice(0, 8)}</div>
                      </div>
                    </div>
                  </td>
                  <td><Badge tone={roleTone(n.role)}>{n.role}</Badge></td>
                  <td class="hidden font-mono text-xs text-ink-600 dark:text-ink-300 md:table-cell"
                    >{n.public_ip ?? '—'}</td
                  >
                  <td><StatusBadge status={n.status} /></td>
                  <td class="hidden whitespace-nowrap text-ink-500 lg:table-cell">
                    {n.last_seen_at ? formatRelative(n.last_seen_at) : de.nodes.never}
                  </td>
                  <td class="text-right">
                    <button
                      type="button"
                      class="btn-icon hover:!bg-red-50 hover:!text-red-600 dark:hover:!bg-red-500/10 dark:hover:!text-red-400"
                      title={de.delete}
                      aria-label={`${de.delete}: ${n.name}`}
                      onclick={() => onDelete(n)}
                    >
                      <Icon name="trash" size={16} />
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>
  </div>

  <Modal open={addOpen} title={de.nodes.add} onclose={closeAdd} size="lg">
    <div class="space-y-5">
      <div class="tabs" role="tablist">
        <button
          type="button"
          role="tab"
          class="tab"
          aria-selected={tab === 'wizard'}
          onclick={() => (tab = 'wizard')}
        >
          <Icon name="terminal" size={15} />
          {de.nodes.tabWizard}
        </button>
        <button
          type="button"
          role="tab"
          class="tab"
          aria-selected={tab === 'manual'}
          onclick={() => (tab = 'manual')}
        >
          <Icon name="key" size={15} />
          {de.nodes.tabManual}
        </button>
      </div>

      {#if error}
        <Alert tone="danger">{error}</Alert>
      {/if}

      {#if tab === 'wizard'}
        <!-- Stepper -->
        <ol class="flex items-center gap-2 text-xs font-medium">
          {#each ['Verbindung', 'Host-Key', 'Installation'] as s, i (s)}
            <li class="flex items-center gap-2">
              <span
                class="grid h-6 w-6 place-items-center rounded-full text-[11px] {jobStep > i
                  ? 'bg-brand-500 text-white dark:text-ink-950'
                  : jobStep === i
                    ? 'bg-ink-900 text-white dark:bg-white dark:text-ink-900'
                    : 'bg-ink-100 text-ink-500 dark:bg-ink-800 dark:text-ink-400'}"
              >
                {#if jobStep > i}<Icon name="check" size={12} strokeWidth={3} />{:else}{i + 1}{/if}
              </span>
              <span class={jobStep >= i ? 'text-ink-900 dark:text-white' : 'text-ink-400'}>{s}</span
              >
              {#if i < 2}<span class="h-px w-6 bg-ink-200 dark:bg-ink-700 sm:w-10"></span>{/if}
            </li>
          {/each}
        </ol>

        {#if !job}
          <p class="text-sm text-ink-500 dark:text-ink-400">{de.nodes.wizardHint}</p>
          <form id="wizard" class="space-y-6" onsubmit={onWizardStart}>
            <fieldset class="space-y-3">
              <legend class="mb-2 text-sm font-semibold">{de.nodes.sectionConnection}</legend>
              <div class="grid gap-3 sm:grid-cols-[1fr_7rem_10rem]">
                <div class="field">
                  <label class="label" for="w-host">{de.nodes.host}</label>
                  <input
                    id="w-host"
                    class="input font-mono"
                    bind:value={host}
                    required
                    placeholder="203.0.113.10"
                  />
                </div>
                <div class="field">
                  <label class="label" for="w-port">{de.nodes.sshPort}</label>
                  <input
                    id="w-port"
                    class="input font-mono"
                    type="number"
                    min="1"
                    max="65535"
                    bind:value={sshPort}
                  />
                </div>
                <div class="field">
                  <label class="label" for="w-user">{de.nodes.sshUser}</label>
                  <input id="w-user" class="input font-mono" bind:value={sshUser} required />
                </div>
              </div>
            </fieldset>

            <fieldset class="space-y-3">
              <legend class="mb-2 text-sm font-semibold">{de.nodes.sectionAuth}</legend>
              <div class="field">
                <label class="label" for="w-pw">{de.nodes.sshPassword}</label>
                <PasswordInput id="w-pw" bind:value={sshPassword} autocomplete="new-password" />
              </div>
              <div class="field">
                <label class="label" for="w-key">{de.nodes.sshKey}</label>
                <textarea
                  id="w-key"
                  class="input font-mono text-xs"
                  rows="3"
                  placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
                  bind:value={sshKey}
                ></textarea>
              </div>
            </fieldset>

            <fieldset class="space-y-3">
              <legend class="mb-2 text-sm font-semibold">{de.nodes.sectionNode}</legend>
              {@render rolePicker(role, (r) => (role = r), 'wizard-role')}
              <div class="grid gap-3 sm:grid-cols-3">
                <div class="field">
                  <label class="label" for="w-name">{de.name}</label>
                  <input
                    id="w-name"
                    class="input font-mono"
                    bind:value={nodeName}
                    placeholder="edge-1"
                  />
                </div>
                <div class="field">
                  <label class="label" for="w-v4">{de.nodes.cidrV4}</label>
                  <input id="w-v4" class="input font-mono" bind:value={cidrV4} />
                </div>
                <div class="field">
                  <label class="label" for="w-v6">{de.nodes.cidrV6}</label>
                  <input
                    id="w-v6"
                    class="input font-mono"
                    bind:value={cidrV6}
                    placeholder="fd00:66::/64"
                  />
                </div>
              </div>
            </fieldset>
          </form>
        {:else}
          <div class="space-y-4">
            <div
              class="flex flex-wrap items-center gap-3 rounded-lg border border-ink-200 px-4 py-3 text-sm dark:border-ink-700"
            >
              <Icon name="server" size={16} class="text-ink-400" />
              <span class="font-mono">{job.ssh_user}@{job.host}:{job.ssh_port}</span>
              <Badge tone={roleTone(job.role)}>{job.role}</Badge>
              <span class="ml-auto"><StatusBadge status={job.status} /></span>
            </div>

            {#if job.status === 'awaiting_hostkey' && job.host_key_fingerprint}
              <div
                class="space-y-3 rounded-xl border border-amber-200 bg-amber-50/60 p-4 dark:border-amber-500/20 dark:bg-amber-500/5"
              >
                <div class="flex items-center gap-2">
                  <Icon name="fingerprint" size={18} class="text-amber-600 dark:text-amber-400" />
                  <h3 class="text-sm font-semibold">{de.nodes.hostKeyTitle}</h3>
                </div>
                <p class="text-sm text-ink-600 dark:text-ink-300">{de.nodes.hostKeyHint}</p>
                <CopyField value={job.host_key_fingerprint} />
              </div>
            {/if}

            {#if logText || job.log_text}
              <div class="space-y-1.5">
                <div class="label flex items-center gap-2">
                  <Icon name="terminal" size={13} />
                  {de.nodes.liveLog}
                </div>
                <div
                  class="overflow-hidden rounded-lg border border-ink-800 bg-ink-950 shadow-inner"
                >
                  <div class="flex gap-1.5 border-b border-white/5 px-3 py-2">
                    <span class="h-2.5 w-2.5 rounded-full bg-red-400/70"></span>
                    <span class="h-2.5 w-2.5 rounded-full bg-amber-400/70"></span>
                    <span class="h-2.5 w-2.5 rounded-full bg-brand-400/70"></span>
                  </div>
                  <pre
                    bind:this={logEl}
                    class="max-h-72 overflow-auto whitespace-pre-wrap p-3 font-mono text-xs leading-relaxed text-ink-200">{logText ||
                      job.log_text}</pre>
                </div>
              </div>
            {/if}

            {#if job.error_message}
              <Alert tone="danger">{job.error_message}</Alert>
            {/if}
            {#if job.status === 'succeeded'}
              <Alert tone="success">{job.node_name ?? job.host} ist einsatzbereit.</Alert>
            {/if}
          </div>
        {/if}
      {:else}
        <p class="text-sm text-ink-500 dark:text-ink-400">{de.nodes.joinTokenHint}</p>
        <form id="token" class="space-y-4" onsubmit={onToken}>
          {@render rolePicker(tokenRole, (r) => (tokenRole = r), 'token-role')}
          <div class="grid gap-3 sm:grid-cols-[1fr_10rem]">
            <div class="field">
              <label class="label" for="t-name">{de.name}</label>
              <input
                id="t-name"
                class="input font-mono"
                bind:value={tokenName}
                placeholder="edge-1"
              />
            </div>
            <div class="field">
              <label class="label" for="t-ttl">{de.nodes.ttl}</label>
              <input id="t-ttl" class="input" type="number" min="1" max="1440" bind:value={ttl} />
            </div>
          </div>
        </form>
        {#if token}
          <div class="space-y-4 border-t border-ink-100 pt-5 dark:border-ink-800">
            <Alert tone="warning">{de.nodes.tokenOnce}</Alert>
            <CopyField label={de.nodes.token} value={token.token} />
            <CopyField label={de.nodes.installCmd} value={token.install_command} />
            {#if token.docker_command}
              <CopyField label={de.nodes.dockerCmd} value={token.docker_command} />
            {/if}
          </div>
        {/if}
      {/if}
    </div>

    {#snippet footer()}
      {#if tab === 'wizard'}
        {#if !job}
          <button type="button" class="btn-secondary" onclick={closeAdd}>{de.cancel}</button>
          <button type="submit" form="wizard" class="btn-primary" disabled={busy}>
            {#if busy}<Spinner size={16} />{:else}<Icon name="arrowRight" size={16} />{/if}
            {de.nodes.startInstall}
          </button>
        {:else if job.status === 'awaiting_hostkey'}
          <button type="button" class="btn-secondary" disabled={busy} onclick={onCancelJob}
            >{de.cancel}</button
          >
          <button type="button" class="btn-primary" onclick={onConfirmHostKey} disabled={busy}>
            <Icon name="shield" size={16} />
            {de.nodes.confirmHostKey}
          </button>
        {:else if jobDone}
          <button type="button" class="btn-secondary" onclick={resetWizard}>
            <Icon name="plus" size={16} />
            {de.nodes.add}
          </button>
          <button type="button" class="btn-primary" onclick={closeAdd}>{de.close}</button>
        {:else}
          <button type="button" class="btn-secondary" onclick={closeAdd}>{de.close}</button>
        {/if}
      {:else}
        <button type="button" class="btn-secondary" onclick={closeAdd}>{de.close}</button>
        <button type="submit" form="token" class="btn-primary" disabled={busy}>
          <Icon name="key" size={16} />
          {de.nodes.joinToken}
        </button>
      {/if}
    {/snippet}
  </Modal>
{/if}
