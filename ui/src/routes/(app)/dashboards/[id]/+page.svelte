<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { currentUser } from '$lib/stores';
  import { api } from '$lib/api';
  import type { Dashboard, Widget } from '$lib/types';
  import WidgetContainer from '$lib/components/dashboard/WidgetContainer.svelte';
  import WidgetRenderer from '$lib/components/dashboard/WidgetRenderer.svelte';
  import WidgetConfigModal from '$lib/components/dashboard/WidgetConfigModal.svelte';
  import { LayoutDashboard, Plus, ArrowLeft } from 'lucide-svelte';

  let id = $derived($page.params.id);

  let dashboard = $state<Dashboard | null>(null);
  let widgets = $state<Widget[]>([]);
  let loading = $state(true);
  let errorMsg = $state('');

  let isAdmin = $derived($currentUser?.role === 'admin');

  // Modal state
  let modalOpen = $state(false);
  let editingWidget = $state<Widget | null>(null);

  onMount(() => { load(); });

  // Reload when id changes
  $effect(() => { id; load(); });

  async function load() {
    loading = true;
    errorMsg = '';
    try {
      const res = await api.get<Dashboard & { widgets: Widget[] }>(`/api/dashboards/${id}`);
      dashboard = res;
      widgets = (res.widgets ?? []).slice().sort((a, b) => a.position - b.position);
    } catch (e: unknown) {
      errorMsg = e instanceof Error ? e.message : 'Failed to load dashboard';
    } finally {
      loading = false;
    }
  }

  function openCreate() {
    editingWidget = null;
    modalOpen = true;
  }

  function openEdit(w: Widget) {
    editingWidget = w;
    modalOpen = true;
  }

  async function handleDelete(w: Widget) {
    if (!confirm(`Delete widget "${w.title}"?`)) return;
    try {
      await api.delete(`/api/widgets/${w.id}`);
      load();
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : 'Failed to delete widget');
    }
  }

  async function handleMoveUp(w: Widget) {
    const idx = widgets.findIndex(x => x.id === w.id);
    if (idx <= 0) return;
    const above = widgets[idx - 1];
    const positions = [
      { id: w.id, position: above.position },
      { id: above.id, position: w.position },
    ];
    try {
      await api.put(`/api/dashboards/${id}/positions`, positions);
      load();
    } catch { /* ignore */ }
  }

  async function handleMoveDown(w: Widget) {
    const idx = widgets.findIndex(x => x.id === w.id);
    if (idx < 0 || idx >= widgets.length - 1) return;
    const below = widgets[idx + 1];
    const positions = [
      { id: w.id, position: below.position },
      { id: below.id, position: w.position },
    ];
    try {
      await api.put(`/api/dashboards/${id}/positions`, positions);
      load();
    } catch { /* ignore */ }
  }

  async function handleToggleWidth(w: Widget) {
    const newWidth = w.width === 'half' ? 'full' : 'half';
    try {
      await api.put(`/api/widgets/${w.id}`, {
        title: w.title,
        type: w.type,
        width: newWidth,
        config: w.config,
      });
      load();
    } catch { /* ignore */ }
  }

  function onModalSave() {
    modalOpen = false;
    editingWidget = null;
    load();
  }

  function onModalClose() {
    modalOpen = false;
    editingWidget = null;
  }
</script>

{#if loading}
  <div class="flex items-center justify-center h-full">
    <div class="text-[12px] font-mono text-text-muted">Loading dashboard…</div>
  </div>

{:else if errorMsg}
  <div class="flex items-center justify-center h-full">
    <p class="text-[13px] font-mono text-brand-danger">{errorMsg}</p>
  </div>

{:else if dashboard}
  <div class="flex flex-col h-full">
    <!-- Top bar -->
    <div class="flex items-center justify-between px-5 py-3.5 border-b border-white/5 flex-shrink-0">
      <div class="flex items-center gap-3">
        <button
          onclick={() => goto('/dashboards')}
          class="text-text-muted hover:text-text-primary transition-colors"
          aria-label="Back to dashboards"
        >
          <ArrowLeft size={16} />
        </button>
        <div class="flex items-center gap-2">
          <LayoutDashboard size={15} class="text-text-muted" />
          <h1 class="text-[13px] font-bold text-text-primary">{dashboard.name}</h1>
          {#if dashboard.description}
            <span class="text-[12px] text-text-muted hidden sm:inline">— {dashboard.description}</span>
          {/if}
        </div>
      </div>
      {#if isAdmin}
        <button
          onclick={openCreate}
          class="flex items-center gap-1.5 text-[12px] font-mono px-3 py-1.5
                 bg-brand-primary text-white hover:bg-brand-primary/90 transition-colors"
        >
          <Plus size={13} />
          Add widget
        </button>
      {/if}
    </div>

    <!-- Grid -->
    <div class="flex-1 overflow-y-auto p-4">
      {#if widgets.length === 0}
        <div class="flex flex-col items-center justify-center h-full gap-3">
          <p class="text-[13px] text-text-muted">No widgets yet</p>
          {#if isAdmin}
            <button
              onclick={openCreate}
              class="text-[12px] font-mono text-brand-primary hover:underline"
            >Add your first widget</button>
          {/if}
        </div>
      {:else}
        <div class="grid grid-cols-2 gap-4">
          {#each widgets as w, idx}
            <div class="{w.width === 'full' ? 'col-span-2' : 'col-span-1'}">
              <WidgetContainer
                widget={w}
                {isAdmin}
                isFirst={idx === 0}
                isLast={idx === widgets.length - 1}
                onedit={() => openEdit(w)}
                ondelete={() => handleDelete(w)}
                onmoveup={() => handleMoveUp(w)}
                onmovedown={() => handleMoveDown(w)}
                ontogglewidth={() => handleToggleWidth(w)}
              >
                {#snippet children()}
                  <WidgetRenderer widget={w} />
                {/snippet}
              </WidgetContainer>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}

<!-- Widget config modal -->
<WidgetConfigModal
  open={modalOpen}
  dashboardId={id}
  widget={editingWidget}
  onsave={onModalSave}
  onclose={onModalClose}
/>

<style>
  button { border-radius: 0 !important; }
</style>
