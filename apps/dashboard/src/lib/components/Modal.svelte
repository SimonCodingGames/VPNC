<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import { de } from '$lib/i18n/de';

  let {
    open,
    title,
    onclose,
    size = 'md',
    children,
    footer
  }: {
    open: boolean;
    title: string;
    onclose: () => void;
    size?: 'sm' | 'md' | 'lg';
    children: Snippet;
    footer?: Snippet;
  } = $props();

  let dialog = $state<HTMLDivElement | null>(null);
  const widths = { sm: 'max-w-md', md: 'max-w-lg', lg: 'max-w-2xl' };

  $effect(() => {
    if (open && dialog) {
      const el = dialog.querySelector<HTMLElement>(
        'input, select, textarea, button[data-autofocus], button'
      );
      el?.focus();
    }
  });

  function onkeydown(e: KeyboardEvent) {
    if (open && e.key === 'Escape') onclose();
  }
</script>

<svelte:window {onkeydown} />

{#if open}
  <div class="fixed inset-0 z-50 flex items-end justify-center p-4 sm:items-center">
    <button
      type="button"
      class="absolute inset-0 cursor-default bg-ink-950/50 backdrop-blur-[2px] animate-fade-in"
      aria-label={de.close}
      tabindex="-1"
      onclick={onclose}
    ></button>
    <div
      bind:this={dialog}
      role="dialog"
      aria-modal="true"
      aria-label={title}
      class="card relative flex max-h-[90vh] w-full flex-col overflow-hidden shadow-pop animate-scale-in {widths[
        size
      ]}"
    >
      <div class="card-header">
        <h2 class="text-base font-semibold text-ink-900 dark:text-white">{title}</h2>
        <button type="button" class="btn-icon -mr-2" onclick={onclose} aria-label={de.close}>
          <Icon name="x" size={18} />
        </button>
      </div>
      <div class="overflow-y-auto p-5">{@render children()}</div>
      {#if footer}
        <div
          class="flex flex-wrap justify-end gap-2 border-t border-ink-100 bg-ink-50/60 px-5 py-3 dark:border-ink-800 dark:bg-ink-950/40"
        >
          {@render footer()}
        </div>
      {/if}
    </div>
  </div>
{/if}
