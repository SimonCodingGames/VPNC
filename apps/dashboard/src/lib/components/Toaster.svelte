<script lang="ts">
  import Icon from './Icon.svelte';
  import { ui, dismissToast } from '$lib/ui.svelte';
  import { de } from '$lib/i18n/de';
</script>

<div
  class="pointer-events-none fixed bottom-4 right-4 z-[70] flex w-full max-w-sm flex-col gap-2"
  aria-live="polite"
>
  {#each ui.toasts as t (t.id)}
    <div
      class="card pointer-events-auto flex items-center gap-3 px-4 py-3 text-sm shadow-pop animate-fade-in"
    >
      <span
        class="grid h-6 w-6 place-items-center rounded-full {t.kind === 'error'
          ? 'bg-red-50 text-red-600 dark:bg-red-500/10 dark:text-red-400'
          : t.kind === 'info'
            ? 'bg-sky-50 text-sky-600 dark:bg-sky-500/10 dark:text-sky-400'
            : 'bg-brand-50 text-brand-600 dark:bg-brand-500/10 dark:text-brand-400'}"
      >
        <Icon
          name={t.kind === 'error' ? 'alert' : t.kind === 'info' ? 'info' : 'check'}
          size={14}
        />
      </span>
      <span class="flex-1 text-ink-800 dark:text-ink-100">{t.text}</span>
      <button
        type="button"
        class="btn-icon h-6 w-6"
        onclick={() => dismissToast(t.id)}
        aria-label={de.close}
      >
        <Icon name="x" size={14} />
      </button>
    </div>
  {/each}
</div>
