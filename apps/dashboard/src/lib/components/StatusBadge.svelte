<script lang="ts">
  import Badge from './Badge.svelte';
  import { statusTone } from '$lib/format';

  let { status }: { status: string } = $props();
  const tone = $derived(statusTone(status));
  const pulse = $derived(tone === 'success' || status === 'running');
</script>

<Badge {tone} dot {pulse}>{status.replace(/_/g, ' ')}</Badge>
