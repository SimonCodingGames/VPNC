<script lang="ts">
  import { resolve } from '$app/paths';
  import { de } from '$lib/i18n/de';
</script>

<section class="space-y-2 py-8">
  <h1 class="text-2xl font-semibold">{de.forbidden}</h1>
  <p class="text-slate-600 dark:text-slate-300">{de.forbiddenHint}</p>
  <a class="text-sm underline" href={resolve('/devices')}>{de.nav.devices}</a>
</section>
