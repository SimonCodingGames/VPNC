<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Tone } from '$lib/format';

  let {
    tone = 'neutral',
    dot = false,
    pulse = false,
    children
  }: { tone?: Tone; dot?: boolean; pulse?: boolean; children: Snippet } = $props();

  const cls: Record<Tone, string> = {
    neutral: 'badge-neutral',
    success: 'badge-success',
    warning: 'badge-warning',
    danger: 'badge-danger',
    info: 'badge-info',
    violet: 'badge-violet'
  };
  const dotCls: Record<Tone, string> = {
    neutral: 'bg-ink-400',
    success: 'bg-brand-500',
    warning: 'bg-amber-500',
    danger: 'bg-red-500',
    info: 'bg-sky-500',
    violet: 'bg-violet-500'
  };
</script>

<span class={cls[tone]}>
  {#if dot}
    <span class="relative flex h-1.5 w-1.5">
      {#if pulse}
        <span class="absolute inline-flex h-full w-full animate-ping2 rounded-full {dotCls[tone]}"
        ></span>
      {/if}
      <span class="relative inline-flex h-1.5 w-1.5 rounded-full {dotCls[tone]}"></span>
    </span>
  {/if}
  {@render children()}
</span>
