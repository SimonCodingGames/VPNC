<script lang="ts">
  import { page } from '$app/stores';
  import { resolve } from '$app/paths';
  import { de } from '$lib/i18n/de';
  import Icon from '$lib/components/Icon.svelte';
</script>

<section class="grid min-h-[60vh] place-items-center px-4 py-12">
  <div class="text-center">
    <p
      class="bg-gradient-to-br from-brand-400 to-brand-700 bg-clip-text text-6xl font-bold tracking-tight text-transparent"
    >
      {$page.status}
    </p>
    <h1 class="mt-4 text-xl font-semibold text-ink-900 dark:text-white">
      {$page.status === 404 ? de.notFound : `${de.errorTitle} ${$page.status}`}
    </h1>
    {#if $page.error?.message}
      <p class="mt-2 text-sm text-ink-500 dark:text-ink-400">{$page.error.message}</p>
    {/if}
    <a class="btn-primary mt-8" href={resolve('/')}>
      <Icon name="home" size={16} />
      {de.backHome}
    </a>
  </div>
</section>
