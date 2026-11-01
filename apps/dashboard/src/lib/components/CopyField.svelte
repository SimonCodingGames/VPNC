<script lang="ts">
  import Icon from './Icon.svelte';
  import { de } from '$lib/i18n/de';
  import { toast } from '$lib/ui.svelte';

  let {
    value,
    label = '',
    multiline = false
  }: { value: string; label?: string; multiline?: boolean } = $props();
  let copied = $state(false);

  async function copy() {
    await navigator.clipboard.writeText(value);
    copied = true;
    toast(de.copied);
    setTimeout(() => (copied = false), 2000);
  }
</script>

<div class="space-y-1.5">
  {#if label}<div class="label">{label}</div>{/if}
  <div class="code-block group relative flex items-start gap-2 py-2.5 pl-3 pr-2">
    <code class="min-w-0 flex-1 {multiline ? 'whitespace-pre-wrap' : ''} break-all">{value}</code>
    <button type="button" class="btn-secondary btn-sm shrink-0" onclick={copy} aria-label={de.copy}>
      <Icon name={copied ? 'check' : 'copy'} size={14} />
      {copied ? de.copied : de.copy}
    </button>
  </div>
</div>
