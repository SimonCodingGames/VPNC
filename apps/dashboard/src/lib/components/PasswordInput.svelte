<script lang="ts">
  import Icon from './Icon.svelte';
  import { de } from '$lib/i18n/de';

  let {
    value = $bindable(''),
    id,
    autocomplete = 'current-password',
    required = false,
    minlength
  }: {
    value?: string;
    id: string;
    autocomplete?: 'current-password' | 'new-password' | 'off';
    required?: boolean;
    minlength?: number;
  } = $props();
  let visible = $state(false);
</script>

<div class="relative">
  <input
    {id}
    class="input pr-10"
    type={visible ? 'text' : 'password'}
    {autocomplete}
    {required}
    {minlength}
    bind:value
  />
  <button
    type="button"
    class="absolute inset-y-0 right-0 grid w-10 place-items-center text-ink-400 hover:text-ink-700 dark:hover:text-ink-200"
    onclick={() => (visible = !visible)}
    aria-label={visible ? de.hidePassword : de.showPassword}
  >
    <Icon name={visible ? 'eyeOff' : 'eye'} size={16} />
  </button>
</div>
