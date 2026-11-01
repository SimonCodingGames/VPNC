<script lang="ts">
  import Icon, { type IconName } from './Icon.svelte';

  let {
    label,
    value,
    icon,
    hint = '',
    href
  }: {
    label: string;
    value: string | number;
    icon: IconName;
    hint?: string;
    href?: string;
  } = $props();
</script>

<svelte:element
  this={href ? 'a' : 'div'}
  {href}
  class="card group relative overflow-hidden p-5 transition {href
    ? 'hover:border-brand-300 hover:shadow-pop dark:hover:border-brand-500/40'
    : ''}"
>
  <div class="flex items-start justify-between">
    <div>
      <p class="text-xs font-medium uppercase tracking-wider text-ink-500 dark:text-ink-400">
        {label}
      </p>
      <p
        class="mt-2 text-3xl font-semibold tabular-nums tracking-tight text-ink-900 dark:text-white"
      >
        {value}
      </p>
      {#if hint}<p class="mt-1 text-xs text-ink-500 dark:text-ink-400">{hint}</p>{/if}
    </div>
    <div
      class="grid h-10 w-10 place-items-center rounded-lg bg-brand-50 text-brand-600 ring-1 ring-inset ring-brand-100 dark:bg-brand-500/10 dark:text-brand-400 dark:ring-brand-500/20"
    >
      <Icon name={icon} size={20} />
    </div>
  </div>
</svelte:element>
