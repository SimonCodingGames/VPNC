<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { login } from '$lib/auth';
  import { de } from '$lib/i18n/de';

  let email = $state('');
  let password = $state('');
  let error = $state('');
  let busy = $state(false);

  async function onSubmit(e: Event) {
    e.preventDefault();
    error = '';
    busy = true;
    try {
      await login(email, password);
      password = '';
      await goto(resolve('/devices'));
    } catch {
      error = 'Anmeldung fehlgeschlagen';
      password = '';
    } finally {
      busy = false;
    }
  }
</script>

<main class="min-h-screen flex items-center justify-center bg-slate-50 dark:bg-slate-900 px-4">
  <form class="w-full max-w-sm space-y-4" onsubmit={onSubmit}>
    <h1 class="text-2xl font-semibold text-slate-900 dark:text-slate-100">{de.loginTitle}</h1>
    <label class="block space-y-1">
      <span class="text-sm text-slate-600 dark:text-slate-300">{de.email}</span>
      <input
        class="w-full border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
        type="email"
        autocomplete="username"
        bind:value={email}
        required
      />
    </label>
    <label class="block space-y-1">
      <span class="text-sm text-slate-600 dark:text-slate-300">{de.password}</span>
      <input
        class="w-full border border-slate-300 dark:border-slate-600 rounded px-3 py-2 bg-white dark:bg-slate-800"
        type="password"
        autocomplete="current-password"
        bind:value={password}
        required
      />
    </label>
    {#if error}
      <p class="text-sm text-red-600">{error}</p>
    {/if}
    <button
      class="w-full bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 rounded py-2 disabled:opacity-50"
      type="submit"
      disabled={busy}
    >
      {de.submitLogin}
    </button>
  </form>
</main>
