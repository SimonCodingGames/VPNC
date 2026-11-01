<script lang="ts">
  import { de } from '$lib/i18n/de';
  let { compact = false, class: klass = '' }: { compact?: boolean; class?: string } = $props();
</script>

<span class="inline-flex items-center gap-2.5 {klass}">
  <span
    class="relative grid h-8 w-8 place-items-center rounded-lg bg-gradient-to-br from-brand-400 to-brand-700 text-white shadow-sm ring-1 ring-inset ring-white/20"
  >
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" aria-hidden="true">
      <path
        d="M12 2.5 19.5 5.3v6.1c0 4.6-3.2 8.6-7.5 10.1-4.3-1.5-7.5-5.5-7.5-10.1V5.3L12 2.5Z"
        fill="currentColor"
        fill-opacity=".18"
        stroke="currentColor"
        stroke-width="1.6"
        stroke-linejoin="round"
      />
      <path
        d="m8.8 12.2 2.3 2.3 4.2-4.6"
        stroke="currentColor"
        stroke-width="1.9"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
  </span>
  {#if !compact}
    <span class="flex flex-col leading-none">
      <span class="text-[15px] font-bold tracking-tight text-ink-900 dark:text-white"
        >{de.brand}</span
      >
      <span
        class="mt-1 text-[10px] font-medium uppercase whitespace-nowrap tracking-[0.08em] text-ink-400"
        >{de.brandTagline}</span
      >
    </span>
  {/if}
</span>
