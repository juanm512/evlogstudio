<script lang="ts">
  import { createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { api } from '$lib/api';
  import type { Source } from '$lib/types';
  import Modal from '$lib/components/common/Modal.svelte';
  import PanelHeader from '$lib/components/common/PanelHeader.svelte';
  import ConfirmModal from '$lib/components/common/ConfirmModal.svelte';
  import StatusRow from '$lib/components/common/StatusRow.svelte';
  import EmptyState from '$lib/components/common/EmptyState.svelte';
  import { format } from 'date-fns';
  import { Trash2, Plus, Loader2, Database } from 'lucide-svelte';

  const queryClient = useQueryClient();

  const sourcesQuery = createQuery(() => ({
    queryKey: ['sources'],
    queryFn: () => api.get<Source[]>('/api/sources')
  }));

  const RETENTION_RE = /^\d+(d|h|m)$/;

  let isModalOpen = $state(false);
  let isSubmitting = $state(false);
  let errorMsg = $state('');
  let retentionError = $state('');

  let formData = $state({
    name: '',
    description: '',
    retention: '30d'
  });

  function resetForm() {
    formData = { name: '', description: '', retention: '30d' };
    errorMsg = '';
    retentionError = '';
  }

  let isDeleteModalOpen = $state(false);
  let sourceToDelete = $state<Source | null>(null);

  async function handleCreate() {
    if (!formData.name) return;
    retentionError = '';
    if (!RETENTION_RE.test(formData.retention)) {
      retentionError = "Formato inválido. Usar: '30d', '24h', '60m'";
      return;
    }
    isSubmitting = true;
    errorMsg = '';
    try {
      await api.post('/api/sources', formData);
      queryClient.invalidateQueries({ queryKey: ['sources'] });
      isModalOpen = false;
      resetForm();
    } catch (e: any) {
      if (e.message.includes('409') || e.message.toLowerCase().includes('already exists')) {
        errorMsg = 'Source name already exists';
      } else {
        errorMsg = e.message;
      }
    } finally {
      isSubmitting = false;
    }
  }

  function confirmDelete(source: Source) {
    sourceToDelete = source;
    isDeleteModalOpen = true;
  }

  async function handleDelete() {
    if (!sourceToDelete) return;
    isSubmitting = true;
    try {
      await api.delete(`/api/sources/${sourceToDelete.id}`);
      queryClient.invalidateQueries({ queryKey: ['sources'] });
      isDeleteModalOpen = false;
      sourceToDelete = null;
    } catch (e: any) {
      alert(`Error: ${e.message}`);
    } finally {
      isSubmitting = false;
    }
  }
</script>

<div class="space-y-4">
  <PanelHeader 
    title="Managed Sources" 
    subtitle="Log destinations and storage policies"
    actionIcon={Plus}
    actionLabel="New Source"
    onAction={() => { isModalOpen = true; resetForm(); }}
  />

  <div class="border border-border-dim bg-surface overflow-hidden">
    <table class="w-full text-left border-collapse">
      <thead>
        <tr class="bg-surface-elevated border-b border-border-dim">
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted w-[20%]">Name</th>
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted">Description</th>
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted text-right w-[10%]">Retention</th>
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted w-[15%]">Created</th>
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted text-right w-[10%]">Actions</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-border-dim">
        {#if sourcesQuery.isLoading}
          <StatusRow colspan={5} message="Fetching metadata..." />
        {:else if sourcesQuery.isError}
          <tr>
            <td colspan="5" class="px-6 py-16 text-center">
              <div class="space-y-3">
                <p class="text-brand-danger text-[11px] font-mono uppercase tracking-widest">Connection Error</p>
                <button 
                  onclick={() => sourcesQuery.refetch()}
                  class="text-brand-primary text-[10px] font-bold uppercase underline tracking-widest"
                >
                  Reconnect
                </button>
              </div>
            </td>
          </tr>
        {:else if sourcesQuery.data?.length === 0}
          <tr>
            <td colspan="5">
              <EmptyState 
                title="Infrastructure Empty" 
                description="Create a source to begin receiving and storing log data." 
                icon={Database}
              />
            </td>
          </tr>
        {:else}
          {#each sourcesQuery.data ?? [] as source}
            <tr class="hover:bg-surface-elevated transition-colors group">
              <td class="px-6 py-4 text-xs font-mono font-bold text-brand-primary">{source.name}</td>
              <td class="px-6 py-4 text-xs text-text-secondary truncate max-w-[300px]" title={source.description || ''}>
                {source.description || '—'}
              </td>
              <td class="px-6 py-4 text-xs font-mono text-text-secondary text-right">{source.retention ?? '30d'}</td>
              <td class="px-6 py-4 text-[11px] font-mono text-text-muted tracking-tight">
                {format(new Date(source.created_at), 'yyyy/MM/dd HH:mm')}
              </td>
              <td class="px-6 py-4 text-right">
                <button 
                  onclick={() => confirmDelete(source)}
                  class="text-text-muted hover:text-brand-danger transition-colors p-1"
                  aria-label="Delete source"
                >
                  <Trash2 size={16} />
                </button>
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>
</div>

<!-- Create Modal -->
<Modal open={isModalOpen} title="Provision New Source" onClose={() => isModalOpen = false}>
  <form onsubmit={(e) => { e.preventDefault(); handleCreate(); }} class="space-y-6">
    <div class="space-y-2">
      <label for="name" class="text-[10px] font-bold uppercase tracking-widest text-text-muted block">Identifier (Slug)</label>
      <input 
        id="name"
        type="text"
        bind:value={formData.name}
        placeholder="e.g. production-api"
        required
        class="w-full bg-surface-elevated border border-border-dim p-3 text-[13px] font-mono text-text-primary focus:border-brand-primary outline-none transition-colors"
      />
    </div>

    <div class="space-y-2">
      <label for="desc" class="text-[10px] font-bold uppercase tracking-widest text-text-muted block">Metadata Description</label>
      <input 
        id="desc"
        type="text"
        bind:value={formData.description}
        placeholder="Primary backend services"
        class="w-full bg-surface-elevated border border-border-dim p-3 text-[13px] text-text-primary focus:border-brand-primary outline-none transition-colors"
      />
    </div>

    <div class="space-y-2">
      <label for="retention" class="text-[10px] font-bold uppercase tracking-widest text-text-muted block">Retention Policy</label>
      <input
        id="retention"
        type="text"
        bind:value={formData.retention}
        placeholder="ej: 30d, 24h, 60m"
        class="w-full bg-surface-elevated border border-border-dim p-3 text-[13px] font-mono text-text-primary focus:border-brand-primary outline-none transition-colors {retentionError ? 'border-brand-danger' : ''}"
      />
      <p class="text-[10px] font-mono text-text-muted">d = días, h = horas, m = minutos</p>
      {#if retentionError}
        <div class="text-brand-danger text-[10px] font-bold uppercase bg-brand-danger/5 p-2 border-l-2 border-brand-danger font-mono">
          Error: {retentionError}
        </div>
      {/if}
    </div>

    {#if errorMsg}
      <div class="text-brand-danger text-[10px] font-bold uppercase bg-brand-danger/5 p-3 border-l-2 border-brand-danger font-mono">
        Error_Code: 409 // {errorMsg}
      </div>
    {/if}

    <div class="pt-4">
      <button 
        type="submit"
        disabled={isSubmitting}
        class="w-full bg-brand-primary text-white text-[11px] font-bold uppercase tracking-[2px] py-3.5 hover:bg-blue-600 transition-all disabled:opacity-50 flex items-center justify-center gap-2"
      >
        {#if isSubmitting}
          <Loader2 size={16} class="animate-spin" />
          Provisioning...
        {:else}
          Initialize Source
        {/if}
      </button>
    </div>
  </form>
</Modal>

<!-- Delete Confirmation Modal -->
<ConfirmModal 
  open={isDeleteModalOpen}
  title="Confirm Deletion"
  description="Are you sure you want to delete source {sourceToDelete?.name}?"
  impactDescription="This will permanently remove all associated log data. This action cannot be undone."
  confirmLabel="Delete Permanently"
  {isSubmitting}
  onConfirm={handleDelete}
  onCancel={() => isDeleteModalOpen = false}
/>

<style>
  input, button {
    border-radius: 0 !important;
  }
</style>
