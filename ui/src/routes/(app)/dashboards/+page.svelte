<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { currentUser } from '$lib/stores';
  import { api } from '$lib/api';
  import type { Dashboard } from '$lib/types';
  import DashboardCard from '$lib/components/dashboard/DashboardCard.svelte';
  import { LayoutDashboard, Plus } from 'lucide-svelte';

  let dashboards = $state<Dashboard[]>([]);
  let loading = $state(true);
  let errorMsg = $state('');

  // New dashboard modal state
  let showCreate = $state(false);
  let newName = $state('');
  let newDesc = $state('');
  let creating = $state(false);
  let createError = $state('');

  let isAdmin = $derived($currentUser?.role === 'admin');

  onMount(() => { load(); });

  async function load() {
    loading = true;
    errorMsg = '';
    try {
      dashboards = await api.get<Dashboard[]>('/api/dashboards');
    } catch (e: unknown) {
      errorMsg = e instanceof Error ? e.message : 'Failed to load dashboards';
    } finally {
      loading = false;
    }
  }

  async function create() {
    if (!newName.trim()) { createError = 'Name is required'; return; }
    creating = true;
    createError = '';
    try {
      const res = await api.post<{ id: string }>('/api/dashboards', {
        name: newName.trim(),
        description: newDesc.trim() || null,
      });
      showCreate = false;
      newName = '';
      newDesc = '';
      goto(`/dashboards/${res.id}`);
    } catch (e: unknown) {
      createError = e instanceof Error ? e.message : 'Failed to create dashboard';
    } finally {
      creating = false;
    }
  }
</script>

<div class="flex flex-col h-full">
  <!-- Top bar -->
  <div class="flex items-center justify-between px-5 py-3.5 border-b border-white/5 flex-shrink-0">
    <div class="flex items-center gap-2.5">
      <LayoutDashboard size={16} class="text-text-muted" />
      <h1 class="text-[13px] font-bold text-text-primary uppercase tracking-wider">Dashboards</h1>
    </div>
    {#if isAdmin}
      <button
        onclick={() => (showCreate = true)}
        class="flex items-center gap-1.5 text-[12px] font-mono px-3 py-1.5
               bg-brand-primary text-white hover:bg-brand-primary/90 transition-colors"
      >
        <Plus size={13} />
        New dashboard
      </button>
    {/if}
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-y-auto p-5">
    {#if loading}
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {#each Array(6) as _}
          <div class="h-32 bg-surface border border-border-dim animate-pulse"></div>
        {/each}
      </div>

    {:else if errorMsg}
      <div class="flex items-center justify-center h-full">
        <p class="text-[13px] font-mono text-brand-danger">{errorMsg}</p>
      </div>

    {:else if dashboards.length === 0}
      <div class="flex flex-col items-center justify-center h-full gap-3 text-center">
        <LayoutDashboard size={32} class="text-text-muted opacity-40" />
        <p class="text-[13px] text-text-muted">No dashboards yet</p>
        {#if isAdmin}
          <button
            onclick={() => (showCreate = true)}
            class="text-[12px] font-mono text-brand-primary hover:underline"
          >
            Create your first dashboard
          </button>
        {/if}
      </div>

    {:else}
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {#each dashboards as db}
          <DashboardCard
            dashboard={db}
            onclick={() => goto(`/dashboards/${db.id}`)}
          />
        {/each}
      </div>
    {/if}
  </div>
</div>

<!-- Create modal -->
{#if showCreate}
  <div class="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4">
    <div class="bg-surface border border-border-dim w-full max-w-md shadow-2xl">
      <div class="flex items-center justify-between px-5 py-3.5 border-b border-border-dim">
        <h2 class="text-[13px] font-bold uppercase tracking-wider">New Dashboard</h2>
        <button
          onclick={() => { showCreate = false; createError = ''; }}
          class="text-text-muted hover:text-text-primary transition-colors text-lg px-1"
          aria-label="Close"
        >×</button>
      </div>
      <div class="p-5 flex flex-col gap-3">
        <div class="flex flex-col gap-1">
          <label class="text-[10px] font-mono text-text-muted uppercase tracking-wider">Name *</label>
          <input
            type="text"
            class="bg-surface-elevated border border-border-dim text-text-primary font-mono text-[12px]
                   px-3 py-2 focus:outline-none focus:border-brand-primary transition-colors"
            placeholder="My Dashboard"
            bind:value={newName}
            onkeydown={e => e.key === 'Enter' && create()}
          />
        </div>
        <div class="flex flex-col gap-1">
          <label class="text-[10px] font-mono text-text-muted uppercase tracking-wider">Description</label>
          <input
            type="text"
            class="bg-surface-elevated border border-border-dim text-text-primary font-mono text-[12px]
                   px-3 py-2 focus:outline-none focus:border-brand-primary transition-colors"
            placeholder="Optional description"
            bind:value={newDesc}
          />
        </div>
        {#if createError}
          <p class="text-[12px] font-mono text-brand-danger">{createError}</p>
        {/if}
      </div>
      <div class="flex justify-end gap-2 px-5 py-3.5 border-t border-border-dim">
        <button
          onclick={() => { showCreate = false; createError = ''; }}
          class="text-[12px] font-mono px-4 py-2 border border-border-dim text-text-muted
                 hover:text-text-primary hover:border-brand-primary/40 transition-colors"
        >Cancel</button>
        <button
          onclick={create}
          disabled={creating}
          class="text-[12px] font-mono px-4 py-2 bg-brand-primary text-white
                 hover:bg-brand-primary/90 disabled:opacity-50 transition-colors"
        >{creating ? 'Creating…' : 'Create'}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  input, select { border-radius: 0 !important; }
  button { border-radius: 0 !important; }
</style>
