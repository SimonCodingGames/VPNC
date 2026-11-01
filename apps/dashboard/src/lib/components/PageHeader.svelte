<script lang="ts">
  import type { Snippet } from 'svelte';
  let {
    title,
    subtitle = '',
    actions
  }: { title: string; subtitle?: string; actions?: Snippet } = $props();
</script>

<header class="mb-6 flex flex-wrap items-end justify-between gap-4">
  <div class="min-w-0">
    <h1 class="text-2xl font-semibold tracking-tight text-ink-900 dark:text-white">{title}</h1>
    {#if subtitle}
      <p class="mt-1 text-sm text-ink-500 dark:text-ink-400">{subtitle}</p>
    {/if}
  </div>
  {#if actions}
    <div class="flex flex-wrap items-center gap-2">{@render actions()}</div>
  {/if}
</header>
