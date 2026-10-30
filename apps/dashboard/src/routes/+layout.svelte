<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { fetchMe, logout, type Me } from '$lib/auth';
  import { de } from '$lib/i18n/de';

  let { children } = $props();
  let me = $state<Me | null>(null);
  let ready = $state(false);
  let dark = $state(false);

  onMount(async () => {
    dark = localStorage.getItem('vpnc-theme') === 'dark';
    document.documentElement.classList.toggle('dark', dark);
    if ($page.url.pathname !== '/login') {
      me = await fetchMe();
      if (!me) {
        await goto('/login');
        ready = true;
        return;
      }
    }
    ready = true;
  });

  function toggleTheme() {
    dark = !dark;
    localStorage.setItem('vpnc-theme', dark ? 'dark' : 'light');
    document.documentElement.classList.toggle('dark', dark);
  }

  async function onLogout() {
    await logout();
    me = null;
    await goto('/login');
  }
</script>

{#if $page.url.pathname === '/login'}
  {@render children()}
{:else if ready && me}
  <div class="min-h-screen bg-slate-50 text-slate-900 dark:bg-slate-900 dark:text-slate-100">
    <header
      class="border-b border-slate-200 dark:border-slate-700 px-4 py-3 flex items-center gap-4"
    >
      <strong class="text-lg tracking-tight">{de.brand}</strong>
      <nav class="flex gap-3 text-sm">
        <a href="/devices">{de.nav.devices}</a>
        <a href="/profiles">{de.nav.profiles}</a>
        {#if me.role === 'admin'}
          <a href="/users">{de.nav.users}</a>
          <a href="/nodes">{de.nav.nodes}</a>
          <a href="/audit">{de.nav.audit}</a>
          <a href="/settings">{de.nav.settings}</a>
        {/if}
      </nav>
      <div class="ml-auto flex items-center gap-3 text-sm">
        <button type="button" onclick={toggleTheme}>{de.themeToggle}</button>
        <span>{me.email}</span>
        <button type="button" onclick={onLogout}>{de.logout}</button>
      </div>
    </header>
    <main class="p-4">{@render children()}</main>
  </div>
{:else if ready}
  {@render children()}
{:else}
  <p class="p-4 text-sm text-slate-500">…</p>
{/if}
