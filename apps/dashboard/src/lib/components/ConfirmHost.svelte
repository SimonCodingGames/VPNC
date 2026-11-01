<script lang="ts">
  import Icon from './Icon.svelte';
  import { ui, settleConfirm } from '$lib/ui.svelte';
  import { de } from '$lib/i18n/de';

  let confirmBtn = $state<HTMLButtonElement | null>(null);

  $effect(() => {
    if (ui.confirm && confirmBtn) confirmBtn.focus();
  });

  function onkeydown(e: KeyboardEvent) {
    if (ui.confirm && e.key === 'Escape') settleConfirm(false);
  }
</script>

<svelte:window {onkeydown} />

{#if ui.confirm}
  {@const c = ui.confirm}
  <div class="fixed inset-0 z-[60] flex items-end justify-center p-4 sm:items-center">
    <button
      type="button"
      class="absolute inset-0 cursor-default bg-ink-950/50 backdrop-blur-[2px] animate-fade-in"
      aria-label={de.cancel}
      tabindex="-1"
      onclick={() => settleConfirm(false)}
    ></button>
    <div
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="confirm-title"
      class="card relative w-full max-w-md overflow-hidden shadow-pop animate-scale-in"
    >
      <div class="flex gap-4 p-5">
        <div
          class="grid h-10 w-10 shrink-0 place-items-center rounded-full {c.danger
            ? 'bg-red-50 text-red-600 dark:bg-red-500/10 dark:text-red-400'
            : 'bg-brand-50 text-brand-600 dark:bg-brand-500/10 dark:text-brand-400'}"
        >
          <Icon name={c.danger ? 'alert' : 'info'} size={20} />
        </div>
        <div class="min-w-0 pt-1">
          <h2 id="confirm-title" class="text-base font-semibold text-ink-900 dark:text-white">
            {c.title}
          </h2>
          {#if c.message}
            <p class="mt-1 text-sm text-ink-500 dark:text-ink-400">{c.message}</p>
          {/if}
        </div>
      </div>
      <div
        class="flex justify-end gap-2 border-t border-ink-100 bg-ink-50/60 px-5 py-3 dark:border-ink-800 dark:bg-ink-950/40"
      >
        <button type="button" class="btn-secondary" onclick={() => settleConfirm(false)}
          >{de.cancel}</button
        >
        <button
          bind:this={confirmBtn}
          type="button"
          class={c.danger ? 'btn-danger' : 'btn-primary'}
          onclick={() => settleConfirm(true)}>{c.confirmLabel ?? de.confirm}</button
        >
      </div>
    </div>
  </div>
{/if}
