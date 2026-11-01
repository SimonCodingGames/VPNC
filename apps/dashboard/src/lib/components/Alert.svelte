<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon, { type IconName } from './Icon.svelte';

  let {
    tone = 'info',
    title = '',
    children
  }: {
    tone?: 'info' | 'warning' | 'danger' | 'success';
    title?: string;
    children: Snippet;
  } = $props();

  const styles = {
    info: 'border-sky-200 bg-sky-50 text-sky-900 dark:border-sky-500/20 dark:bg-sky-500/5 dark:text-sky-200',
    warning:
      'border-amber-200 bg-amber-50 text-amber-900 dark:border-amber-500/20 dark:bg-amber-500/5 dark:text-amber-200',
    danger:
      'border-red-200 bg-red-50 text-red-900 dark:border-red-500/20 dark:bg-red-500/5 dark:text-red-200',
    success:
      'border-brand-200 bg-brand-50 text-brand-900 dark:border-brand-500/20 dark:bg-brand-500/5 dark:text-brand-200'
  };
  const icons: Record<string, IconName> = {
    info: 'info',
    warning: 'alert',
    danger: 'alert',
    success: 'check'
  };
</script>

<div
  class="flex gap-3 rounded-xl border px-4 py-3 text-sm {styles[tone]}"
  role={tone === 'danger' ? 'alert' : 'status'}
>
  <Icon name={icons[tone]} size={18} class="mt-0.5 opacity-80" />
  <div class="min-w-0 space-y-0.5">
    {#if title}<p class="font-semibold">{title}</p>{/if}
    <div class="opacity-90">{@render children()}</div>
  </div>
</div>
