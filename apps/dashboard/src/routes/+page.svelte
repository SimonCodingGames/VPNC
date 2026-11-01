<script lang="ts">
  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';
  import { fetchMe, type Me } from '$lib/auth';
  import { api, type Device, type Node, type Profile } from '$lib/api';
  import { de } from '$lib/i18n/de';
  import { formatRelative, roleTone } from '$lib/format';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import StatCard from '$lib/components/StatCard.svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import LoadingState from '$lib/components/LoadingState.svelte';

  let me = $state<Me | null>(null);
  let devices = $state<Device[]>([]);
  let profiles = $state<Profile[]>([]);
  let nodes = $state<Node[] | null>(null);
  let userCount = $state<number | null>(null);
  let loaded = $state(false);

  onMount(async () => {
    me = await fetchMe();
    const [d, p, n, u] = await Promise.allSettled([
      api.listDevices(),
      api.listProfiles(),
      api.listNodes(),
      me?.role === 'admin' ? api.listUsers() : Promise.reject(new Error('skip'))
    ]);
    if (d.status === 'fulfilled') devices = d.value.items;
    if (p.status === 'fulfilled') profiles = p.value.items;
    if (n.status === 'fulfilled') nodes = n.value.items;
    if (u.status === 'fulfilled') userCount = u.value.items.length;
    loaded = true;
  });

  const activeProfiles = $derived(profiles.filter((p) => !p.revoked_at));
  const onlineNodes = $derived(nodes ? nodes.filter((n) => n.status === 'online').length : 0);
  const recent = $derived(
    [...profiles].sort((a, b) => b.created_at.localeCompare(a.created_at)).slice(0, 5)
  );

  function deviceName(id: string) {
    return devices.find((d) => d.id === id)?.name ?? id;
  }
  function nodeName(id: string | null) {
    if (!id) return '—';
    return nodes?.find((n) => n.id === id)?.name ?? id;
  }

  type QuickAction = {
    path: '/devices' | '/profiles' | '/nodes';
    title: string;
    hint: string;
    icon: IconName;
  };
  const quick = $derived<QuickAction[]>([
    {
      path: '/devices',
      title: de.overview.qaDevice,
      hint: de.overview.qaDeviceHint,
      icon: 'laptop'
    },
    {
      path: '/profiles',
      title: de.overview.qaProfile,
      hint: de.overview.qaProfileHint,
      icon: 'qr'
    },
    ...(me?.role === 'admin'
      ? [
          {
            path: '/nodes' as const,
            title: de.overview.qaNode,
            hint: de.overview.qaNodeHint,
            icon: 'server' as IconName
          }
        ]
      : [])
  ]);
</script>

<svelte:head>
  <title>{de.overview.title} · {de.brand}</title>
</svelte:head>

<!-- Hero -->
<section
  class="relative mb-8 overflow-hidden rounded-2xl bg-ink-950 px-6 py-7 text-white shadow-pop sm:px-8"
>
  <div
    class="pointer-events-none absolute -right-24 -top-24 h-72 w-72 rounded-full bg-brand-500/25 blur-3xl"
  ></div>
  <div
    class="pointer-events-none absolute bottom-[-6rem] left-1/3 h-56 w-56 rounded-full bg-sky-500/10 blur-3xl"
  ></div>
  <div class="relative flex flex-wrap items-center justify-between gap-6">
    <div>
      <p class="text-sm text-ink-300">{de.overview.greeting}</p>
      <h1 class="mt-1 break-all text-2xl font-semibold tracking-tight sm:text-3xl">
        {me?.email ?? ''}
      </h1>
      <p class="mt-2 text-sm text-ink-300">{de.overview.subtitle}</p>
    </div>
    {#if nodes}
      <div
        class="flex items-center gap-3 rounded-xl bg-white/5 px-4 py-3 ring-1 ring-inset ring-white/10"
      >
        <span class="relative flex h-3 w-3">
          {#if onlineNodes > 0}
            <span class="absolute inline-flex h-full w-full animate-ping2 rounded-full bg-brand-400"
            ></span>
          {/if}
          <span
            class="relative inline-flex h-3 w-3 rounded-full {onlineNodes > 0
              ? 'bg-brand-400'
              : 'bg-ink-500'}"
          ></span>
        </span>
        <div class="text-sm">
          <p class="font-semibold">{onlineNodes} / {nodes.length}</p>
          <p class="text-xs text-ink-400">{de.overview.statNodes}</p>
        </div>
      </div>
    {/if}
  </div>
</section>

{#if !loaded}
  <LoadingState />
{:else}
  <!-- Stats -->
  <div class="grid grid-cols-2 gap-3 sm:gap-4 xl:grid-cols-4">
    <StatCard
      label={de.overview.statDevices}
      value={devices.length}
      icon="laptop"
      href={resolve('/devices')}
    />
    <StatCard
      label={de.overview.statProfiles}
      value={activeProfiles.length}
      icon="key"
      href={resolve('/profiles')}
    />
    {#if nodes}
      <StatCard
        label={de.overview.statNodes}
        value={`${onlineNodes}/${nodes.length}`}
        icon="server"
        href={me?.role === 'admin' ? resolve('/nodes') : undefined}
      />
    {/if}
    {#if userCount !== null}
      <StatCard
        label={de.overview.statUsers}
        value={userCount}
        icon="users"
        href={resolve('/users')}
      />
    {/if}
  </div>

  <!-- Quick actions -->
  <h2 class="mb-3 mt-10 text-sm font-semibold text-ink-900 dark:text-white">
    {de.overview.quickActions}
  </h2>
  <div class="grid gap-4 md:grid-cols-3">
    {#each quick as q (q.path)}
      <a
        href={resolve(q.path)}
        class="card group flex items-center gap-4 p-4 transition hover:border-brand-300 hover:shadow-pop dark:hover:border-brand-500/40"
      >
        <span
          class="grid h-11 w-11 shrink-0 place-items-center rounded-xl bg-ink-900 text-white transition group-hover:bg-brand-600 dark:bg-ink-800 dark:group-hover:bg-brand-500 dark:group-hover:text-ink-950"
        >
          <Icon name={q.icon} size={20} />
        </span>
        <span class="min-w-0 flex-1">
          <span class="block text-sm font-semibold text-ink-900 dark:text-white">{q.title}</span>
          <span class="block truncate text-xs text-ink-500 dark:text-ink-400">{q.hint}</span>
        </span>
        <Icon
          name="arrowRight"
          size={16}
          class="text-ink-300 transition group-hover:translate-x-0.5 group-hover:text-brand-600 dark:text-ink-600 dark:group-hover:text-brand-400"
        />
      </a>
    {/each}
  </div>

  <div class="mt-10 grid gap-6 lg:grid-cols-5">
    <!-- Recent profiles -->
    <div class="card lg:col-span-3">
      <div class="card-header">
        <h2 class="card-title">{de.overview.recentProfiles}</h2>
        <a
          href={resolve('/profiles')}
          class="text-xs font-medium text-brand-600 hover:text-brand-700 dark:text-brand-400"
          >{de.overview.allProfiles} →</a
        >
      </div>
      {#if recent.length === 0}
        <p class="px-5 py-10 text-center text-sm text-ink-500">{de.empty}</p>
      {:else}
        <ul class="divide-y divide-ink-100 dark:divide-ink-800">
          {#each recent as p (p.id)}
            <li class="flex items-center gap-3 px-5 py-3">
              <span
                class="grid h-8 w-8 place-items-center rounded-lg bg-ink-100 text-ink-500 dark:bg-ink-800 dark:text-ink-400"
              >
                <Icon name="key" size={16} />
              </span>
              <div class="min-w-0 flex-1">
                <p class="truncate text-sm font-medium">
                  {deviceName(p.device_id)}
                  <span class="text-ink-400">→</span>
                  {nodeName(p.node_id)}
                </p>
                <p class="font-mono text-xs text-ink-500">{p.assigned_ipv4 ?? '—'}</p>
              </div>
              <div class="hidden text-xs text-ink-400 sm:block">{formatRelative(p.created_at)}</div>
              {#if p.revoked_at}
                <Badge tone="danger">{de.profiles.revoked}</Badge>
              {:else}
                <Badge tone="success" dot>{de.profiles.active}</Badge>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <!-- Node status -->
    {#if nodes}
      <div class="card lg:col-span-2">
        <div class="card-header">
          <h2 class="card-title">{de.overview.nodesTitle}</h2>
          {#if me?.role === 'admin'}
            <a
              href={resolve('/nodes')}
              class="text-xs font-medium text-brand-600 hover:text-brand-700 dark:text-brand-400"
              >{de.overview.allNodes} →</a
            >
          {/if}
        </div>
        {#if nodes.length === 0}
          <p class="px-5 py-10 text-center text-sm text-ink-500">{de.empty}</p>
        {:else}
          <ul class="divide-y divide-ink-100 dark:divide-ink-800">
            {#each nodes.slice(0, 6) as n (n.id)}
              <li class="flex items-center gap-3 px-5 py-3">
                <div class="min-w-0 flex-1">
                  <p class="truncate font-mono text-sm font-medium">{n.name}</p>
                  <p class="text-xs text-ink-500">
                    {[n.public_ip, n.public_ipv6].filter(Boolean).join(' · ') || '—'}
                  </p>
                </div>
                <Badge tone={roleTone(n.role)}>{n.role}</Badge>
                <StatusBadge status={n.status} />
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}
  </div>
{/if}
