<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchMe, type Me } from '$lib/auth';
  import { api, ApiError, type User } from '$lib/api';
  import { de } from '$lib/i18n/de';
  import { initials } from '$lib/format';
  import { confirmAction, toast } from '$lib/ui.svelte';
  import Forbidden from '$lib/components/Forbidden.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Alert from '$lib/components/Alert.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import LoadingState from '$lib/components/LoadingState.svelte';
  import PasswordInput from '$lib/components/PasswordInput.svelte';

  let allowed = $state<boolean | null>(null);
  let me = $state<Me | null>(null);
  let items = $state<User[]>([]);
  let loaded = $state(false);
  let error = $state('');
  let busy = $state(false);
  let createOpen = $state(false);
  let email = $state('');
  let password = $state('');
  let role = $state('user');
  let resetUser = $state<User | null>(null);
  let resetPw = $state('');

  onMount(async () => {
    me = await fetchMe();
    allowed = me?.role === 'admin';
    if (allowed) await load();
  });

  async function load() {
    error = '';
    try {
      const res = await api.listUsers();
      items = res.items;
    } catch (e) {
      if (e instanceof ApiError && e.status === 403) {
        allowed = false;
        return;
      }
      error = e instanceof ApiError ? e.message : de.genericError;
    } finally {
      loaded = true;
    }
  }

  function closeCreate() {
    createOpen = false;
    email = '';
    password = '';
    role = 'user';
  }

  async function onCreate(e: Event) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      const u = await api.createUser({ email: email.trim(), password, role });
      toast(`${u.email} angelegt`);
      closeCreate();
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : de.genericError;
    } finally {
      busy = false;
      password = '';
    }
  }

  async function setRole(id: string, next: string) {
    busy = true;
    try {
      await api.updateUser(id, { role: next });
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : de.genericError;
    } finally {
      busy = false;
    }
  }

  async function toggleDisabled(u: User) {
    busy = true;
    try {
      await api.updateUser(u.id, { disabled: !u.disabled });
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : de.genericError;
    } finally {
      busy = false;
    }
  }

  function closeReset() {
    resetUser = null;
    resetPw = '';
  }

  async function onReset(e: Event) {
    e.preventDefault();
    if (!resetUser || resetPw.length < 10) return;
    busy = true;
    try {
      await api.updateUser(resetUser.id, { password: resetPw });
      toast(`${de.users.resetPassword}: ${resetUser.email}`);
      closeReset();
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : de.genericError;
    } finally {
      busy = false;
      resetPw = '';
    }
  }

  async function onDelete(u: User) {
    const ok = await confirmAction({
      title: de.users.confirmDelete,
      message: `${u.email} – ${de.users.confirmDeleteText}`,
      confirmLabel: de.delete,
      danger: true
    });
    if (!ok) return;
    busy = true;
    try {
      await api.deleteUser(u.id);
      toast(`${u.email} gelöscht`);
      await load();
    } catch (err) {
      error = err instanceof ApiError ? err.message : de.genericError;
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head>
  <title>{de.users.title} · {de.brand}</title>
</svelte:head>

{#if allowed === null}
  <LoadingState />
{:else if !allowed}
  <Forbidden />
{:else}
  <PageHeader title={de.users.title} subtitle={de.users.subtitle}>
    {#snippet actions()}
      <button type="button" class="btn-primary" onclick={() => (createOpen = true)}>
        <Icon name="userPlus" size={16} />
        {de.users.new}
      </button>
    {/snippet}
  </PageHeader>

  <div class="space-y-6">
    {#if error}
      <Alert tone="danger">{error}</Alert>
    {/if}

    <div class="card overflow-hidden">
      {#if !loaded}
        <LoadingState />
      {:else if items.length === 0}
        <EmptyState icon="users" title={de.users.emptyTitle} />
      {:else}
        <div class="overflow-x-auto">
          <table class="table">
            <thead>
              <tr>
                <th>{de.email}</th>
                <th>{de.role}</th>
                <th>{de.status}</th>
                <th class="text-right"><span class="sr-only">{de.actions}</span></th>
              </tr>
            </thead>
            <tbody>
              {#each items as u (u.id)}
                {@const self = me?.id === u.id}
                <tr>
                  <td>
                    <div class="flex items-center gap-3">
                      <span
                        class="grid h-9 w-9 shrink-0 place-items-center rounded-full text-xs font-semibold {u.role ===
                        'admin'
                          ? 'bg-violet-100 text-violet-700 dark:bg-violet-500/15 dark:text-violet-300'
                          : 'bg-ink-100 text-ink-600 dark:bg-ink-800 dark:text-ink-300'}"
                        >{initials(u.email)}</span
                      >
                      <div class="min-w-0">
                        <div class="truncate font-medium text-ink-900 dark:text-white">
                          {u.email}
                          {#if self}<span class="ml-1 text-xs font-normal text-ink-400">(du)</span
                            >{/if}
                        </div>
                        <div class="font-mono text-xs text-ink-400">{u.id.slice(0, 8)}</div>
                      </div>
                    </div>
                  </td>
                  <td>
                    <select
                      class="input w-auto py-1.5 text-xs"
                      value={u.role}
                      aria-label={`${de.role}: ${u.email}`}
                      disabled={busy}
                      onchange={(e) => setRole(u.id, (e.currentTarget as HTMLSelectElement).value)}
                    >
                      <option value="user">{de.roles.user}</option>
                      <option value="admin">{de.roles.admin}</option>
                    </select>
                  </td>
                  <td>
                    {#if u.disabled}
                      <Badge tone="danger">{de.users.disabled}</Badge>
                    {:else}
                      <Badge tone="success" dot>{de.users.active}</Badge>
                    {/if}
                  </td>
                  <td class="text-right">
                    <div class="inline-flex gap-1">
                      <button
                        type="button"
                        class="btn-icon"
                        title={de.users.resetPassword}
                        aria-label={`${de.users.resetPassword}: ${u.email}`}
                        onclick={() => (resetUser = u)}
                      >
                        <Icon name="key" size={16} />
                      </button>
                      <button
                        type="button"
                        class="btn-icon"
                        title={u.disabled ? de.users.enable : de.users.disable}
                        aria-label={`${u.disabled ? de.users.enable : de.users.disable}: ${u.email}`}
                        disabled={busy}
                        onclick={() => toggleDisabled(u)}
                      >
                        <Icon name={u.disabled ? 'check' : 'ban'} size={16} />
                      </button>
                      <button
                        type="button"
                        class="btn-icon hover:!bg-red-50 hover:!text-red-600 dark:hover:!bg-red-500/10 dark:hover:!text-red-400"
                        title={de.delete}
                        aria-label={`${de.delete}: ${u.email}`}
                        onclick={() => onDelete(u)}
                      >
                        <Icon name="trash" size={16} />
                      </button>
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>
  </div>

  <Modal open={createOpen} title={de.users.new} onclose={closeCreate} size="sm">
    <form id="create-user" class="space-y-4" onsubmit={onCreate}>
      <div class="field">
        <label class="label" for="u-email">{de.email}</label>
        <input id="u-email" class="input" type="email" bind:value={email} required />
      </div>
      <div class="field">
        <label class="label" for="u-password">{de.password}</label>
        <PasswordInput
          id="u-password"
          bind:value={password}
          autocomplete="new-password"
          minlength={10}
          required
        />
        <p class="hint">{de.users.passwordHint}</p>
      </div>
      <div class="field">
        <label class="label" for="u-role">{de.role}</label>
        <select id="u-role" class="input" bind:value={role}>
          <option value="user">{de.roles.user}</option>
          <option value="admin">{de.roles.admin}</option>
        </select>
      </div>
    </form>
    {#snippet footer()}
      <button type="button" class="btn-secondary" onclick={closeCreate}>{de.cancel}</button>
      <button type="submit" form="create-user" class="btn-primary" disabled={busy}>
        <Icon name="plus" size={16} />
        {de.create}
      </button>
    {/snippet}
  </Modal>

  <Modal open={resetUser !== null} title={de.users.resetPassword} onclose={closeReset} size="sm">
    <form id="reset-pw" class="space-y-4" onsubmit={onReset}>
      <p class="text-sm text-ink-600 dark:text-ink-300">
        {de.users.resetPasswordFor} <strong>{resetUser?.email}</strong>
      </p>
      <div class="field">
        <label class="label" for="u-reset">{de.password}</label>
        <PasswordInput
          id="u-reset"
          bind:value={resetPw}
          autocomplete="new-password"
          minlength={10}
          required
        />
        <p class="hint">{de.users.passwordHint}</p>
      </div>
    </form>
    {#snippet footer()}
      <button type="button" class="btn-secondary" onclick={closeReset}>{de.cancel}</button>
      <button type="submit" form="reset-pw" class="btn-primary" disabled={busy}>{de.save}</button>
    {/snippet}
  </Modal>
{/if}
