<script lang="ts">
  import Spinner from './Spinner.svelte';
  import { de } from '$lib/i18n/de';
</script>

<div class="flex items-center justify-center gap-2 py-16 text-sm text-ink-500 dark:text-ink-400">
  <Spinner />
  {de.loading}
</div>
