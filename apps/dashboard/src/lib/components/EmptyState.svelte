<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon, { type IconName } from './Icon.svelte';

  let {
    icon,
    title,
    text = '',
    children
  }: { icon: IconName; title: string; text?: string; children?: Snippet } = $props();
</script>

<div class="flex flex-col items-center justify-center px-6 py-14 text-center">
  <div
    class="mb-4 grid h-12 w-12 place-items-center rounded-xl bg-brand-50 text-brand-600 ring-1 ring-inset ring-brand-100 dark:bg-brand-500/10 dark:text-brand-400 dark:ring-brand-500/20"
  >
    <Icon name={icon} size={22} />
  </div>
  <h3 class="text-sm font-semibold text-ink-900 dark:text-white">{title}</h3>
  {#if text}
    <p class="mt-1 max-w-sm text-sm text-ink-500 dark:text-ink-400">{text}</p>
  {/if}
  {#if children}<div class="mt-5">{@render children()}</div>{/if}
</div>
