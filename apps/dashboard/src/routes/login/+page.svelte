<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { login } from '$lib/auth';
  import { de } from '$lib/i18n/de';
  import Icon from '$lib/components/Icon.svelte';
  import Logo from '$lib/components/Logo.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import PasswordInput from '$lib/components/PasswordInput.svelte';

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
      await goto(resolve('/'));
    } catch {
      error = de.loginFailed;
      password = '';
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head>
  <title>{de.loginTitle} · {de.brand}</title>
</svelte:head>

<main class="grid min-h-screen lg:grid-cols-[1.05fr_1fr]">
  <!-- Brand / hero panel -->
  <section
    class="relative hidden overflow-hidden bg-ink-950 px-12 py-12 text-white lg:flex lg:flex-col"
  >
    <div
      class="pointer-events-none absolute -left-40 -top-40 h-[32rem] w-[32rem] rounded-full bg-brand-500/20 blur-3xl"
    ></div>
    <div
      class="pointer-events-none absolute -bottom-48 right-[-10rem] h-[30rem] w-[30rem] rounded-full bg-sky-500/10 blur-3xl"
    ></div>
    <svg
      class="pointer-events-none absolute inset-0 h-full w-full opacity-[0.07]"
      aria-hidden="true"
    >
      <defs>
        <pattern id="grid" width="32" height="32" patternUnits="userSpaceOnUse">
          <path d="M32 0H0V32" fill="none" stroke="white" stroke-width="1" />
        </pattern>
      </defs>
      <rect width="100%" height="100%" fill="url(#grid)" />
    </svg>

    <div class="relative [&_span]:!text-white">
      <Logo />
    </div>

    <div class="relative my-auto max-w-lg">
      <!-- Network illustration -->
      <svg viewBox="0 0 360 150" class="mb-10 w-full max-w-md" fill="none" aria-hidden="true">
        <g
          stroke="currentColor"
          class="text-brand-400/40"
          stroke-width="1.2"
          stroke-dasharray="3 5"
        >
          <path d="M40 75 C110 20, 150 20, 180 75" />
          <path d="M40 75 C110 130, 150 130, 180 75" />
          <path d="M180 75 L320 30" />
          <path d="M180 75 L320 75" />
          <path d="M180 75 L320 120" />
        </g>
        <circle cx="40" cy="75" r="16" class="fill-white/5 stroke-white/30" />
        <rect
          x="32"
          y="69"
          width="16"
          height="11"
          rx="2"
          class="stroke-white/80"
          stroke-width="1.4"
        />
        <circle
          cx="180"
          cy="75"
          r="24"
          class="fill-brand-500/15 stroke-brand-400"
          stroke-width="1.4"
        />
        <path
          d="M180 62l9 3.4v6.4c0 5-3.6 9.2-9 10.8-5.4-1.6-9-5.8-9-10.8v-6.4l9-3.4z"
          class="stroke-brand-300"
          stroke-width="1.4"
        />
        {#each [30, 75, 120] as y (y)}
          <circle cx="320" cy={y} r="11" class="fill-white/5 stroke-white/30" />
          <circle cx="320" cy={y} r="3" class="fill-brand-400" />
        {/each}
      </svg>

      <h2 class="text-4xl font-semibold leading-tight tracking-tight">{de.loginHeroTitle}</h2>
      <p class="mt-4 text-base leading-relaxed text-ink-300">{de.loginHeroText}</p>
      <ul class="mt-8 space-y-3">
        {#each de.loginFeatures as f (f)}
          <li class="flex items-center gap-3 text-sm text-ink-200">
            <span
              class="grid h-6 w-6 place-items-center rounded-full bg-brand-500/15 text-brand-300 ring-1 ring-inset ring-brand-400/30"
            >
              <Icon name="check" size={13} strokeWidth={2.4} />
            </span>
            {f}
          </li>
        {/each}
      </ul>
    </div>

    <p class="relative text-xs text-ink-500">© {new Date().getFullYear()} {de.brand}</p>
  </section>

  <!-- Form panel -->
  <section class="flex items-center justify-center px-4 py-12 sm:px-8">
    <div class="w-full max-w-sm animate-fade-in">
      <div class="mb-10 lg:hidden"><Logo /></div>

      <h1 class="text-2xl font-semibold tracking-tight text-ink-900 dark:text-white">
        {de.loginTitle}
      </h1>
      <p class="mt-2 text-sm text-ink-500 dark:text-ink-400">{de.loginSubtitle}</p>

      <form class="mt-8 space-y-5" onsubmit={onSubmit}>
        <div class="field">
          <label class="label" for="email">{de.email}</label>
          <div class="relative">
            <Icon
              name="mail"
              size={16}
              class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-ink-400"
            />
            <input
              id="email"
              class="input pl-9"
              type="email"
              autocomplete="username"
              placeholder="name@firma.de"
              bind:value={email}
              required
            />
          </div>
        </div>
        <div class="field">
          <label class="label" for="password">{de.password}</label>
          <PasswordInput id="password" bind:value={password} required />
        </div>

        {#if error}
          <div
            class="flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 px-3 py-2.5 text-sm text-red-700 dark:border-red-500/20 dark:bg-red-500/10 dark:text-red-300"
            role="alert"
          >
            <Icon name="alert" size={16} class="mt-0.5" />
            {error}
          </div>
        {/if}

        <button class="btn-primary w-full py-2.5" type="submit" disabled={busy}>
          {#if busy}<Spinner size={16} />{:else}<Icon name="lock" size={16} />{/if}
          {de.submitLogin}
        </button>
      </form>
    </div>
  </section>
</main>
