<script lang="ts">
  import { resolve } from '$app/paths';
  import Icon from './Icon.svelte';
  import { de } from '$lib/i18n/de';
</script>

<section class="card mx-auto mt-10 max-w-md px-6 py-12 text-center">
  <div
    class="mx-auto mb-4 grid h-12 w-12 place-items-center rounded-xl bg-red-50 text-red-600 ring-1 ring-inset ring-red-100 dark:bg-red-500/10 dark:text-red-400 dark:ring-red-500/20"
  >
    <Icon name="lock" size={22} />
  </div>
  <h1 class="text-lg font-semibold text-ink-900 dark:text-white">{de.forbidden}</h1>
  <p class="mt-1 text-sm text-ink-500 dark:text-ink-400">{de.forbiddenHint}</p>
  <a class="btn-secondary mt-6" href={resolve('/')}>
    <Icon name="home" size={16} />
    {de.backHome}
  </a>
</section>
