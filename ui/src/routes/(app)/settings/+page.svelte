<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { currentUser } from '$lib/stores';
  import SourcesPanel from '$lib/components/settings/SourcesPanel.svelte';
  import TokensPanel from '$lib/components/settings/TokensPanel.svelte';
  import UsersPanel from '$lib/components/settings/UsersPanel.svelte';
  import ConfigPanel from '$lib/components/settings/ConfigPanel.svelte';

  let activeTab = $state('sources');

  onMount(() => {
    // Role guard: if not admin, redirect immediately
    const role = $currentUser?.role || localStorage.getItem('role');
    if (role !== 'admin') {
      goto('/logs');
    }
  });

  const tabs = [
    { id: 'sources', label: 'Sources' },
    { id: 'tokens', label: 'Tokens' },
    { id: 'users', label: 'Users' },
    { id: 'config', label: 'Config' }
  ];
</script>

<svelte:head>
  <title>Settings | evlogstudio</title>
</svelte:head>

<div class="h-full flex flex-col overflow-hidden">
  <!-- Page Header -->
  <div class="flex flex-col gap-1 p-6 border-b border-border-dim bg-surface-elevated shrink-0">
    <h1 class="text-xl font-bold text-text-primary uppercase tracking-tight font-sans">System Settings</h1>
    <p class="text-[11px] text-text-muted font-mono uppercase tracking-wider">Platform Administration & Identity Management</p>
  </div>

  <!-- Tab Navigation -->
  <div class="flex border-b border-border-dim bg-surface px-6 shrink-0">
    {#each tabs as tab}
      <button
        onclick={() => activeTab = tab.id}
        class="px-8 py-4 text-[11px] font-bold uppercase tracking-widest transition-all relative outline-none
          {activeTab === tab.id ? 'text-brand-primary' : 'text-text-muted hover:text-text-secondary'}"
      >
        {tab.label}
        {#if activeTab === tab.id}
          <div class="absolute bottom-0 left-0 right-0 h-[2px] bg-brand-primary"></div>
        {/if}
      </button>
    {/each}
  </div>

  <!-- Content Area -->
  <div class="flex-1 overflow-y-auto bg-[#0c0c0e]">
    <div class="max-w-6xl mx-auto p-6 lg:p-10">
      {#if activeTab === 'sources'}
        <SourcesPanel />
      {:else if activeTab === 'tokens'}
        <TokensPanel />
      {:else if activeTab === 'users'}
        <UsersPanel />
      {:else if activeTab === 'config'}
        <ConfigPanel />
      {/if}
    </div>
  </div>
</div>

<style>
  button {
    border-radius: 0;
  }
</style>
