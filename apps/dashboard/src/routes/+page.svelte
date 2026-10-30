<script lang="ts">
  import { de } from '$lib/i18n/de';
</script>

<svelte:head>
  <title>{de.brand}</title>
</svelte:head>

<section class="space-y-2">
  <h1 class="text-2xl font-semibold">{de.brand}</h1>
  <p class="text-slate-600 dark:text-slate-300">{de.placeholder}</p>
</section>
