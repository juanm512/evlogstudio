<script lang="ts">
  import { createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { api } from '$lib/api';
  import type { Source } from '$lib/types';
  import CustomSelect from '$lib/components/common/CustomSelect.svelte';
  import EmptyState from '$lib/components/common/EmptyState.svelte';
  import { Save, Loader2, Trash2, Settings2, ShieldAlert, Activity, Database } from 'lucide-svelte';

  const queryClient = useQueryClient();

  // ── Sources ────────────────────────────────────────────────────────────────
  const sourcesQuery = createQuery(() => ({
    queryKey: ['sources'],
    queryFn: () => api.get<Source[]>('/api/sources')
  }));

  let selectedSourceId = $state<string | null>(null);

  $effect(() => {
    if (sourcesQuery.data && sourcesQuery.data.length > 0 && !selectedSourceId) {
      selectedSourceId = sourcesQuery.data[0].id;
    }
  });

  let selectedSource = $derived(
    (sourcesQuery.data ?? []).find((s: Source) => s.id === selectedSourceId) ?? null
  );

  // ── Retention ──────────────────────────────────────────────────────────────
  const RETENTION_RE = /^\d+(d|h|m)$/;
  let retentionValue    = $state('30d');
  let retentionError    = $state('');
  let isSavingRetention = $state(false);
  let retentionMsg      = $state({ text: '', type: '' });

  $effect(() => {
    retentionValue = selectedSource?.retention ?? '30d';
    retentionError = '';
    retentionMsg   = { text: '', type: '' };
  });

  async function saveRetention() {
    if (!selectedSourceId) return;
    retentionError = '';
    if (!RETENTION_RE.test(retentionValue)) {
      retentionError = "Formato inválido. Usar: '30d', '24h', '60m'";
      return;
    }
    isSavingRetention = true;
    retentionMsg = { text: '', type: '' };
    try {
      await api.patch(`/api/sources/${selectedSourceId}`, { retention: retentionValue });
      retentionMsg = { text: 'Retention updated', type: 'success' };
      queryClient.invalidateQueries({ queryKey: ['sources'] });
    } catch (e: any) {
      retentionMsg = { text: e.message, type: 'error' };
    } finally {
      isSavingRetention = false;
    }
  }

  // ── Sampling ───────────────────────────────────────────────────────────────
  let samplingEnabled  = $state(false);
  let samplingRates    = $state({ debug: 10, info: 100, warn: 100 });
  let isSavingSampling = $state(false);
  let samplingMsg      = $state({ text: '', type: '' });

  $effect(() => {
    samplingEnabled = selectedSource?.sampling_enabled ?? false;
    samplingRates   = {
      debug: selectedSource?.sampling_debug_rate ?? 10,
      info:  selectedSource?.sampling_info_rate  ?? 100,
      warn:  selectedSource?.sampling_warn_rate  ?? 100,
    };
    samplingMsg = { text: '', type: '' };
  });

  async function saveSampling() {
    if (!selectedSourceId) return;
    isSavingSampling = true;
    samplingMsg = { text: '', type: '' };
    try {
      await api.patch(`/api/sources/${selectedSourceId}`, {
        sampling_enabled:    samplingEnabled,
        sampling_debug_rate: samplingRates.debug,
        sampling_info_rate:  samplingRates.info,
        sampling_warn_rate:  samplingRates.warn,
      });
      samplingMsg = { text: 'Sampling configuration updated', type: 'success' };
      queryClient.invalidateQueries({ queryKey: ['sources'] });
    } catch (e: any) {
      samplingMsg = { text: e.message, type: 'error' };
    } finally {
      isSavingSampling = false;
    }
  }

  // ── Danger zone ────────────────────────────────────────────────────────────
  let isDeletingLogs = $state(false);
  let deleteMsg      = $state({ text: '', type: '' });

  $effect(() => {
    if (selectedSource) deleteMsg = { text: '', type: '' };
  });

  async function deleteSourceLogs() {
    if (!selectedSourceId || !selectedSource) return;
    if (!confirm(`¿Eliminar TODOS los logs de "${selectedSource.name}"? Esta acción no se puede deshacer.`)) return;
    isDeletingLogs = true;
    deleteMsg = { text: '', type: '' };
    try {
      const res = await api.delete<any>(`/api/sources/${selectedSourceId}/logs`);
      deleteMsg = { text: `Deleted ${res.deleted} logs from ${selectedSource.name}`, type: 'success' };
    } catch (e: any) {
      deleteMsg = { text: e.message, type: 'error' };
    } finally {
      isDeletingLogs = false;
    }
  }
</script>

<div class="space-y-8 pb-20">

  <!-- ── Source selector ───────────────────────────────────────────────────── -->
  <div class="flex items-center gap-4 pb-6 border-b border-border-dim">
    <span class="text-[10px] font-bold uppercase tracking-widest text-text-muted whitespace-nowrap">Configure Source</span>
    <div class="w-64">
      <CustomSelect
        options={(sourcesQuery.data ?? []).map((s: Source) => ({ id: s.id, label: s.name }))}
        value={selectedSourceId}
        placeholder="Select source..."
        onSelect={(id) => selectedSourceId = id}
      />
    </div>
    {#if selectedSource}
      <span class="text-[10px] font-mono text-text-muted">
        {selectedSource.retention ?? '30d'} retention
      </span>
    {/if}
  </div>

  {#if !selectedSourceId}
    <!-- Empty state -->
    <div class="border border-border-dim bg-surface">
      <EmptyState
        title="No Source Selected"
        description="Select a source from the dropdown above to view and edit its configuration."
        icon={Database}
      />
    </div>

  {:else}

    <!-- ── Retention ────────────────────────────────────────────────────────── -->
    <section class="space-y-4">
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 bg-brand-primary/10 flex items-center justify-center rounded-none">
          <Settings2 size={16} class="text-brand-primary" />
        </div>
        <div>
          <h2 class="text-xs font-bold uppercase tracking-widest text-text-primary">Log Retention</h2>
          <p class="text-[10px] text-text-muted font-mono uppercase tracking-tight">How long logs are kept for <span class="text-text-secondary">{selectedSource?.name}</span></p>
        </div>
      </div>

      <div class="bg-surface border border-border-dim p-6">
        <div class="max-w-md space-y-2">
          <label for="retention" class="text-[10px] font-bold uppercase tracking-widest text-text-muted block">Retention Period</label>
          <div class="flex gap-0">
            <input
              id="retention"
              type="text"
              bind:value={retentionValue}
              placeholder="ej: 30d, 24h, 60m"
              class="flex-1 bg-surface-elevated border border-border-dim p-3 text-[13px] font-mono text-text-primary focus:border-brand-primary outline-none transition-colors {retentionError ? 'border-brand-danger' : ''}"
            />
            <button
              onclick={saveRetention}
              disabled={isSavingRetention}
              class="bg-brand-primary text-white text-[11px] font-bold uppercase tracking-widest px-6 hover:bg-blue-600 transition-all disabled:opacity-50 flex items-center gap-2"
            >
              {#if isSavingRetention}<Loader2 size={14} class="animate-spin" />{:else}<Save size={14} />{/if}
              Save
            </button>
          </div>
          <p class="text-[10px] font-mono text-text-muted">d = días &nbsp;·&nbsp; h = horas &nbsp;·&nbsp; m = minutos</p>
          {#if retentionError}
            <div class="text-[10px] font-mono uppercase p-2 border-l-2 bg-brand-danger/5 text-brand-danger border-brand-danger">
              &gt; ERROR: {retentionError}
            </div>
          {/if}
          {#if retentionMsg.text}
            <div class="text-[10px] font-mono uppercase p-3 border-l-2 bg-surface-elevated {retentionMsg.type === 'error' ? 'text-brand-danger border-brand-danger' : 'text-brand-primary border-brand-primary'}">
              &gt; {retentionMsg.type === 'error' ? 'ERROR:' : 'STATUS:'} {retentionMsg.text}
            </div>
          {/if}
        </div>
      </div>
    </section>

    <!-- ── Sampling ──────────────────────────────────────────────────────────── -->
    <section class="space-y-4">
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 bg-brand-primary/10 flex items-center justify-center rounded-none">
          <Activity size={16} class="text-brand-primary" />
        </div>
        <div>
          <h2 class="text-xs font-bold uppercase tracking-widest text-text-primary">Sampling</h2>
          <p class="text-[10px] text-text-muted font-mono uppercase tracking-tight">Reduce storage by sampling logs for <span class="text-text-secondary">{selectedSource?.name}</span>. Errors always kept at 100%.</p>
        </div>
      </div>

      <div class="bg-surface border border-border-dim p-6 space-y-8">
        <div class="flex items-center justify-between">
          <div class="space-y-1">
            <span class="text-[11px] font-bold uppercase tracking-widest text-text-primary">Enable Sampling</span>
            <p class="text-[10px] text-text-muted font-mono">STATUS: {samplingEnabled ? 'ACTIVE' : 'INACTIVE'}</p>
          </div>
          <button
            onclick={() => samplingEnabled = !samplingEnabled}
            class="w-12 h-6 flex items-center p-0.5 border border-border-dim transition-colors {samplingEnabled ? 'bg-brand-primary/20 border-brand-primary' : 'bg-surface-elevated'}"
          >
            <div class="w-5 h-5 transition-transform {samplingEnabled ? 'translate-x-6 bg-brand-primary' : 'translate-x-0 bg-text-muted'} rounded-none"></div>
          </button>
        </div>

        {#if samplingEnabled}
          <div class="grid grid-cols-1 md:grid-cols-2 gap-8 pt-8 border-t border-border-dim">
            <div class="space-y-3">
              <div class="flex justify-between items-end">
                <label class="text-[10px] font-bold uppercase tracking-widest text-text-muted">Debug Logs</label>
                <span class="text-[11px] font-mono text-brand-primary">{samplingRates.debug}%</span>
              </div>
              <input type="range" bind:value={samplingRates.debug} min="0" max="100" class="custom-range w-full bg-transparent cursor-pointer" />
            </div>
            <div class="space-y-3">
              <div class="flex justify-between items-end">
                <label class="text-[10px] font-bold uppercase tracking-widest text-text-muted">Info Logs</label>
                <span class="text-[11px] font-mono text-brand-primary">{samplingRates.info}%</span>
              </div>
              <input type="range" bind:value={samplingRates.info} min="0" max="100" class="custom-range w-full bg-transparent cursor-pointer" />
            </div>
            <div class="space-y-3">
              <div class="flex justify-between items-end">
                <label class="text-[10px] font-bold uppercase tracking-widest text-text-muted">Warning Logs</label>
                <span class="text-[11px] font-mono text-brand-primary">{samplingRates.warn}%</span>
              </div>
              <input type="range" bind:value={samplingRates.warn} min="0" max="100" class="custom-range w-full bg-transparent cursor-pointer" />
            </div>
            <div class="space-y-3 opacity-40">
              <div class="flex justify-between items-end">
                <label class="text-[10px] font-bold uppercase tracking-widest text-text-muted">Error / Fatal</label>
                <span class="text-[11px] font-mono text-text-muted">100% (LOCKED)</span>
              </div>
              <input type="range" value="100" disabled class="custom-range w-full bg-transparent cursor-not-allowed" />
            </div>
          </div>
        {/if}

        <div class="pt-4 space-y-4">
          <button
            onclick={saveSampling}
            disabled={isSavingSampling}
            class="bg-brand-primary text-white text-[11px] font-bold uppercase tracking-widest px-8 py-3.5 hover:bg-blue-600 transition-all disabled:opacity-50 flex items-center gap-2"
          >
            {#if isSavingSampling}<Loader2 size={14} class="animate-spin" />{:else}<Save size={14} />{/if}
            Apply Sampling Configuration
          </button>
          {#if samplingMsg.text}
            <div class="text-[10px] font-mono uppercase p-3 border-l-2 bg-surface-elevated {samplingMsg.type === 'error' ? 'text-brand-danger border-brand-danger' : 'text-brand-primary border-brand-primary'}">
              &gt; {samplingMsg.type === 'error' ? 'ERROR:' : 'STATUS:'} {samplingMsg.text}
            </div>
          {/if}
        </div>
      </div>
    </section>

    <!-- ── Danger zone ────────────────────────────────────────────────────────── -->
    <section class="space-y-4">
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 bg-brand-danger/10 flex items-center justify-center rounded-none">
          <ShieldAlert size={16} class="text-brand-danger" />
        </div>
        <div>
          <h2 class="text-xs font-bold uppercase tracking-widest text-brand-danger">Danger Zone</h2>
          <p class="text-[10px] text-text-muted font-mono uppercase tracking-tight">Irreversible destructive operations</p>
        </div>
      </div>

      <div class="bg-surface border border-brand-danger/30 p-6 flex flex-col md:flex-row items-start md:items-center justify-between gap-6">
        <div class="space-y-1">
          <h3 class="text-[11px] font-bold uppercase tracking-widest text-text-primary">Purge Source Logs</h3>
          <p class="text-[10px] text-text-muted font-mono max-w-md uppercase tracking-tight">
            Drop all log data for <span class="text-text-secondary">{selectedSource?.name}</span>. The source itself is not deleted.
          </p>
        </div>
        <div class="flex flex-col items-end gap-3 min-w-[200px]">
          <button
            onclick={deleteSourceLogs}
            disabled={isDeletingLogs}
            class="w-full bg-brand-danger text-white text-[11px] font-bold uppercase tracking-widest px-8 py-3.5 hover:bg-red-700 transition-all disabled:opacity-50 flex items-center justify-center gap-2"
          >
            {#if isDeletingLogs}<Loader2 size={14} class="animate-spin" />{:else}<Trash2 size={14} />{/if}
            Purge Logs
          </button>
          {#if deleteMsg.text}
            <div class="w-full text-[10px] font-mono uppercase p-2 border-l-2 bg-brand-danger/5 {deleteMsg.type === 'error' ? 'text-brand-danger border-brand-danger' : 'text-brand-primary border-brand-primary'}">
              &gt; {deleteMsg.text}
            </div>
          {/if}
        </div>
      </div>
    </section>

  {/if}
</div>

<style>
  button, input {
    border-radius: 0 !important;
  }

  input[type="range"].custom-range {
    -webkit-appearance: none;
    appearance: none;
    height: 40px;
    background: transparent;
    overflow: visible;
  }
  input[type="range"].custom-range::-webkit-slider-runnable-track {
    width: 100%;
    height: 4px;
    background: var(--color-surface-elevated);
    border: 1px solid var(--color-border-dim);
    cursor: pointer;
  }
  input[type="range"].custom-range::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 12px;
    height: 24px;
    background: var(--color-brand-primary);
    cursor: pointer;
    margin-top: -11px;
    border: none;
  }
  input[type="range"].custom-range::-moz-range-track {
    width: 100%;
    height: 4px;
    background: var(--color-surface-elevated);
    border: 1px solid var(--color-border-dim);
    cursor: pointer;
  }
  input[type="range"].custom-range::-moz-range-thumb {
    width: 12px;
    height: 24px;
    background: var(--color-brand-primary);
    cursor: pointer;
    border: none;
    border-radius: 0;
  }
  input[type="range"].custom-range:focus {
    outline: none;
  }
  input[type="range"].custom-range:focus::-webkit-slider-runnable-track {
    border-color: var(--color-brand-primary);
  }
</style>
