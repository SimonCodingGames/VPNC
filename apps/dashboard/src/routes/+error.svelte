<script lang="ts">
  import { page } from '$app/stores';
  import { resolve } from '$app/paths';
  import { de } from '$lib/i18n/de';
</script>

<section class="space-y-2 py-12">
  <h1 class="text-2xl font-semibold">
    {$page.status === 404 ? de.notFound : `Fehler ${$page.status}`}
  </h1>
  <p class="text-slate-600 dark:text-slate-300">{$page.error?.message ?? ''}</p>
  <a class="text-sm underline" href={resolve('/devices')}>{de.nav.devices}</a>
</section>
