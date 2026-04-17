<script lang="ts">
  import { createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { api } from '$lib/api';
  import type { Source, IngestToken } from '$lib/types';
  import Modal from '$lib/components/common/Modal.svelte';
  import CustomSelect from '$lib/components/common/CustomSelect.svelte';
  import PanelHeader from '$lib/components/common/PanelHeader.svelte';
  import ConfirmModal from '$lib/components/common/ConfirmModal.svelte';
  import Badge from '$lib/components/common/Badge.svelte';
  import StatusRow from '$lib/components/common/StatusRow.svelte';
  import EmptyState from '$lib/components/common/EmptyState.svelte';
  import { format } from 'date-fns';
  import { Trash2, Plus, Loader2, Copy, Check, ShieldAlert, KeyRound, Key } from 'lucide-svelte';

  const queryClient = useQueryClient();

  const sourcesQuery = createQuery(() => ({
    queryKey: ['sources'],
    queryFn: () => api.get<Source[]>('/api/sources')
  }));

  let selectedSourceId = $state<string | null>(null);
  
  $effect(() => {
    if (sourcesQuery.data && sourcesQuery.data.length > 0 && !selectedSourceId) {
      selectedSourceId = (sourcesQuery.data as Source[])[0].id;
    }
  });

  const tokensQuery = createQuery(() => ({
    queryKey: ['tokens', selectedSourceId],
    queryFn: () => api.get<IngestToken[]>(`/api/sources/${selectedSourceId}/tokens`),
    enabled: !!selectedSourceId
  }));

  let isModalOpen = $state(false);
  let isRevokeModalOpen = $state(false);
  let tokenToRevoke = $state<IngestToken | null>(null);
  let isSubmitting = $state(false);
  let generatedToken = $state<string | null>(null);
  let copied = $state(false);
  let tokenName = $state('');

  async function handleGenerate() {
    if (!tokenName || !selectedSourceId) return;
    isSubmitting = true;
    try {
      const res = await api.post<{ token: string }>(`/api/sources/${selectedSourceId}/tokens`, { 
        name: tokenName 
      });
      generatedToken = res.token;
    } catch (e: any) {
      alert(`Error generating token: ${e.message}`);
    } finally {
      isSubmitting = false;
    }
  }

  function handleCloseModal() {
    isModalOpen = false;
    generatedToken = null;
    tokenName = '';
    copied = false;
    if (selectedSourceId) {
      queryClient.invalidateQueries({ queryKey: ['tokens', selectedSourceId] });
    }
  }

  async function handleCopy() {
    if (generatedToken) {
      await navigator.clipboard.writeText(generatedToken);
      copied = true;
      setTimeout(() => copied = false, 2000);
    }
  }

  function confirmRevoke(token: IngestToken) {
    tokenToRevoke = token;
    isRevokeModalOpen = true;
  }

  async function handleRevoke() {
    if (!tokenToRevoke) return;
    isSubmitting = true;
    try {
      await api.delete(`/api/tokens/${tokenToRevoke.id}`);
      queryClient.invalidateQueries({ queryKey: ['tokens', selectedSourceId] });
      isRevokeModalOpen = false;
      tokenToRevoke = null;
    } catch (e: any) {
      alert(`Error: ${e.message}`);
    } finally {
      isSubmitting = false;
    }
  }
</script>

<div class="space-y-4">
  <PanelHeader 
    title="Ingest Tokens" 
    subtitle="Secure access keys for log emitters"
    actionIcon={Plus}
    actionLabel="New Token"
    actionDisabled={!selectedSourceId}
    onAction={() => isModalOpen = true}
  >
    <div class="min-w-[200px]">
      <CustomSelect 
        options={sourcesQuery.data?.map(s => ({ id: s.id, label: s.name })) ?? []}
        value={selectedSourceId}
        placeholder="Select Source"
        onSelect={(id) => selectedSourceId = id}
      />
    </div>
  </PanelHeader>

  <div class="border border-border-dim bg-surface overflow-hidden">
    <table class="w-full text-left border-collapse">
      <thead>
        <tr class="bg-surface-elevated border-b border-border-dim">
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted">Name</th>
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted text-center">Status</th>
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted">Created</th>
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted text-right">Last Used</th>
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted text-right">Actions</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-border-dim">
        {#if !selectedSourceId}
          <tr>
            <td colspan="5">
              <EmptyState 
                title="Select a Source" 
                description="Choose a source from the dropdown to manage its ingest tokens." 
                icon={Key}
              />
            </td>
          </tr>
        {:else if tokensQuery.isLoading}
          <StatusRow colspan={5} message="Verifying credentials..." />
        {:else if tokensQuery.data?.length === 0}
          <tr>
            <td colspan="5">
              <EmptyState 
                title="No Tokens Found" 
                description="No active tokens exist for this source. Create one to begin emitting logs." 
                icon={KeyRound}
              />
            </td>
          </tr>
        {:else}
          {#each tokensQuery.data ?? [] as token}
            <tr class="hover:bg-surface-elevated transition-colors group">
              <td class="px-6 py-4 text-xs font-mono font-bold text-text-primary">{token.name}</td>
              <td class="px-6 py-4 text-center">
                {#if token.revoked_at}
                  <Badge variant="muted">Revoked</Badge>
                {:else}
                  <Badge variant="success">Active</Badge>
                {/if}
              </td>
              <td class="px-6 py-4 text-[11px] font-mono text-text-muted tracking-tight">
                {format(new Date(token.created_at), 'yyyy/MM/dd HH:mm')}
              </td>
              <td class="px-6 py-4 text-xs font-mono text-text-secondary text-right">
                {token.last_used ? format(new Date(token.last_used), 'yyyy/MM/dd HH:mm') : 'Never'}
              </td>
              <td class="px-6 py-4 text-right">
                {#if !token.revoked_at}
                  <button 
                    onclick={() => confirmRevoke(token)}
                    class="text-text-muted hover:text-brand-danger transition-colors p-1"
                    aria-label="Revoke token"
                  >
                    <ShieldAlert size={16} />
                  </button>
                {/if}
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>
</div>

<!-- Create Token Modal -->
<Modal open={isModalOpen} title={generatedToken ? "Token Generated" : "Generate Ingest Token"} onClose={handleCloseModal}>
  {#if !generatedToken}
    <form onsubmit={(e) => { e.preventDefault(); handleGenerate(); }} class="space-y-6">
      <div class="space-y-2">
        <label for="tk-name" class="text-[10px] font-bold uppercase tracking-widest text-text-muted block">Token Name / Label</label>
        <input 
          id="tk-name"
          type="text"
          bind:value={tokenName}
          placeholder="e.g. k8s-cluster-a"
          required
          class="w-full bg-surface-elevated border border-border-dim p-3 text-[13px] font-mono text-text-primary focus:border-brand-primary outline-none transition-colors"
        />
      </div>

      <div class="bg-brand-warning/5 border border-brand-warning/20 p-4 space-y-2">
        <div class="flex items-center gap-2 text-brand-warning">
          <KeyRound size={14} />
          <span class="text-[10px] font-bold uppercase tracking-widest">Security Notice</span>
        </div>
        <p class="text-[10px] text-text-secondary leading-relaxed">
          The secret token will be displayed once. Store it securely in your environment variables or secrets manager.
        </p>
      </div>

      <button 
        type="submit"
        disabled={isSubmitting}
        class="w-full bg-brand-primary text-white text-[11px] font-bold uppercase tracking-[2px] py-3.5 hover:bg-blue-600 transition-all disabled:opacity-50 flex items-center justify-center gap-2"
      >
        {#if isSubmitting}
          <Loader2 size={16} class="animate-spin" />
          Issuing...
        {:else}
          Generate Access Key
        {/if}
      </button>
    </form>
  {:else}
    <div class="space-y-6">
      <div class="space-y-2">
        <label class="text-[10px] font-bold uppercase tracking-widest text-text-muted block">Your Secret Token</label>
        <div class="flex items-center gap-0">
          <code class="flex-1 bg-surface-elevated border border-border-dim p-4 text-[12px] font-mono text-brand-success break-all select-all">
            {generatedToken}
          </code>
          <button 
            onclick={handleCopy}
            class="bg-surface-elevated border-y border-r border-border-dim px-4 hover:bg-surface transition-colors flex items-center justify-center p-4 self-stretch"
            title="Copy to clipboard"
          >
            {#if copied}
              <Check size={16} class="text-brand-success" />
            {:else}
              <Copy size={16} class="text-text-muted" />
            {/if}
          </button>
        </div>
      </div>

      <div class="bg-brand-success/5 border border-brand-success/20 p-4">
        <p class="text-[10px] text-brand-success font-bold uppercase tracking-widest mb-1">Success</p>
        <p class="text-[10px] text-text-secondary leading-relaxed font-sans">
          Token issued successfully. Copy it now. It will not be shown again once you close this modal.
        </p>
      </div>

      <button 
        onclick={handleCloseModal}
        class="w-full bg-surface-elevated border border-border-dim text-text-primary text-[11px] font-bold uppercase tracking-[2px] py-3.5 hover:bg-surface transition-all"
      >
        Done
      </button>
    </div>
  {/if}
</Modal>

<!-- Revoke Confirmation Modal -->
<ConfirmModal 
  open={isRevokeModalOpen}
  title="Confirm Revocation"
  variant="warning"
  description="Are you sure you want to revoke token {tokenToRevoke?.name}?"
  impactLabel="Security Impact"
  impactDescription="Log agents using this token will lose access immediately. This action is permanent."
  confirmLabel="Revoke Access Key"
  {isSubmitting}
  onConfirm={handleRevoke}
  onCancel={() => isRevokeModalOpen = false}
/>

<style>
  input, button, code {
    border-radius: 0 !important;
  }
</style>
