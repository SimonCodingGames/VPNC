<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto, afterNavigate } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { fetchMe, logout, type Me } from '$lib/auth';
  import { de } from '$lib/i18n/de';
  import { initials } from '$lib/format';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import Logo from '$lib/components/Logo.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import ConfirmHost from '$lib/components/ConfirmHost.svelte';
  import Toaster from '$lib/components/Toaster.svelte';

  type NavPath = '/' | '/devices' | '/profiles' | '/users' | '/nodes' | '/audit' | '/settings';
  type NavItem = { path: NavPath; label: string; icon: IconName };

  const mainNav: NavItem[] = [
    { path: '/', label: de.nav.overview, icon: 'home' },
    { path: '/devices', label: de.nav.devices, icon: 'laptop' },
    { path: '/profiles', label: de.nav.profiles, icon: 'key' }
  ];
  const adminNav: NavItem[] = [
    { path: '/nodes', label: de.nav.nodes, icon: 'server' },
    { path: '/users', label: de.nav.users, icon: 'users' },
    { path: '/audit', label: de.nav.audit, icon: 'scroll' },
    { path: '/settings', label: de.nav.settings, icon: 'settings' }
  ];

  let { children } = $props();
  let me = $state<Me | null>(null);
  let ready = $state(false);
  let dark = $state(false);
  let mobileOpen = $state(false);

  onMount(async () => {
    dark = document.documentElement.classList.contains('dark');
    if ($page.url.pathname !== '/login') {
      me = await fetchMe();
      if (!me) {
        await goto(resolve('/login'));
        ready = true;
        return;
      }
    }
    ready = true;
  });

  afterNavigate(() => {
    mobileOpen = false;
  });

  function toggleTheme() {
    dark = !dark;
    try {
      localStorage.setItem('vpnc-theme', dark ? 'dark' : 'light');
    } catch {
      /* storage unavailable */
    }
    document.documentElement.classList.toggle('dark', dark);
  }

  async function onLogout() {
    await logout();
    me = null;
    await goto(resolve('/login'));
  }

  function isActive(path: NavPath, current: string) {
    return path === '/' ? current === '/' : current === path || current.startsWith(path + '/');
  }
</script>

{#snippet navLink(item: NavItem)}
  {@const active = isActive(item.path, $page.url.pathname)}
  <a
    href={resolve(item.path)}
    aria-current={active ? 'page' : undefined}
    class="group relative flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors {active
      ? 'bg-brand-500/10 text-brand-700 dark:bg-brand-400/10 dark:text-brand-300'
      : 'text-ink-600 hover:bg-ink-100 hover:text-ink-900 dark:text-ink-400 dark:hover:bg-white/5 dark:hover:text-white'}"
  >
    {#if active}
      <span class="absolute inset-y-1.5 left-0 w-0.5 rounded-full bg-brand-500"></span>
    {/if}
    <Icon
      name={item.icon}
      size={18}
      class={active
        ? 'text-brand-600 dark:text-brand-400'
        : 'text-ink-400 group-hover:text-ink-600 dark:text-ink-500 dark:group-hover:text-ink-300'}
    />
    {item.label}
  </a>
{/snippet}

{#snippet sidebar(user: Me)}
  <div class="flex h-full flex-col">
    <div class="flex h-16 items-center px-5">
      <a href={resolve('/')} class="rounded-lg" aria-label={de.brand}><Logo /></a>
    </div>

    <nav class="flex-1 space-y-6 overflow-y-auto px-3 py-4" aria-label="Hauptnavigation">
      <div class="space-y-0.5">
        <p class="px-3 pb-2 text-[11px] font-semibold uppercase tracking-wider text-ink-400">
          {de.nav.sectionMain}
        </p>
        {#each mainNav as item (item.path)}
          {@render navLink(item)}
        {/each}
      </div>
      {#if user.role === 'admin'}
        <div class="space-y-0.5">
          <p class="px-3 pb-2 text-[11px] font-semibold uppercase tracking-wider text-ink-400">
            {de.nav.sectionAdmin}
          </p>
          {#each adminNav as item (item.path)}
            {@render navLink(item)}
          {/each}
        </div>
      {/if}
    </nav>

    <div class="border-t border-ink-200/70 p-3 dark:border-ink-800">
      <button
        type="button"
        class="mb-2 flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium text-ink-600 transition hover:bg-ink-100 hover:text-ink-900 dark:text-ink-400 dark:hover:bg-white/5 dark:hover:text-white"
        onclick={toggleTheme}
        aria-label={de.themeToggle}
      >
        <Icon name={dark ? 'sun' : 'moon'} size={18} class="text-ink-400" />
        {dark ? de.themeLight : de.themeDark}
      </button>
      <div class="flex items-center gap-3 rounded-lg px-2 py-2">
        <span
          class="grid h-9 w-9 shrink-0 place-items-center rounded-full bg-gradient-to-br from-ink-700 to-ink-900 text-xs font-semibold text-white ring-2 ring-white dark:from-brand-500 dark:to-brand-700 dark:ring-ink-900"
          >{initials(user.email)}</span
        >
        <div class="min-w-0 flex-1">
          <p class="truncate text-sm font-medium text-ink-900 dark:text-white" title={user.email}>
            {user.email}
          </p>
          <p class="text-xs text-ink-500 dark:text-ink-400">{de.roles[user.role]}</p>
        </div>
        <button
          type="button"
          class="btn-icon"
          onclick={onLogout}
          aria-label={de.logout}
          title={de.logout}
        >
          <Icon name="logout" size={17} />
        </button>
      </div>
    </div>
  </div>
{/snippet}

{#if $page.url.pathname === '/login'}
  {@render children()}
{:else if ready && me}
  <div class="min-h-screen">
    <!-- Desktop sidebar -->
    <aside
      class="fixed inset-y-0 left-0 z-30 hidden w-64 border-r border-ink-200/70 bg-white/80 backdrop-blur dark:border-ink-800 dark:bg-ink-900/60 lg:block"
    >
      {@render sidebar(me)}
    </aside>

    <!-- Mobile drawer -->
    {#if mobileOpen}
      <div class="fixed inset-0 z-40 lg:hidden">
        <button
          type="button"
          class="absolute inset-0 bg-ink-950/50 backdrop-blur-[2px] animate-fade-in"
          aria-label={de.closeMenu}
          onclick={() => (mobileOpen = false)}
        ></button>
        <aside
          class="relative h-full w-72 max-w-[85vw] border-r border-ink-200 bg-white shadow-pop dark:border-ink-800 dark:bg-ink-900"
        >
          {@render sidebar(me)}
        </aside>
      </div>
    {/if}

    <div class="lg:pl-64">
      <!-- Mobile top bar -->
      <header
        class="sticky top-0 z-20 flex h-14 items-center gap-3 border-b border-ink-200/70 bg-white/80 px-4 backdrop-blur dark:border-ink-800 dark:bg-ink-950/80 lg:hidden"
      >
        <button
          type="button"
          class="btn-icon"
          onclick={() => (mobileOpen = true)}
          aria-label={de.openMenu}
        >
          <Icon name="menu" size={20} />
        </button>
        <Logo compact />
        <span class="font-semibold tracking-tight">{de.brand}</span>
      </header>

      <main class="mx-auto w-full max-w-6xl px-4 py-6 sm:px-6 lg:px-10 lg:py-10">
        <div class="animate-fade-in">{@render children()}</div>
      </main>
    </div>
  </div>
{:else if ready}
  {@render children()}
{:else}
  <div class="grid min-h-screen place-items-center">
    <div class="flex flex-col items-center gap-4 text-ink-500">
      <Logo compact />
      <Spinner size={18} />
    </div>
  </div>
{/if}

<ConfirmHost />
<Toaster />
