<script lang="ts">
  let { size = 16, class: klass = '' }: { size?: number; class?: string } = $props();
</script>

<svg
  class="animate-spin {klass}"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  aria-hidden="true"
>
  <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-opacity=".2" stroke-width="3" />
  <path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" stroke-width="3" stroke-linecap="round" />
</svg>
