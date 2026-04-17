<script lang="ts">
  import { createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { api } from '$lib/api';
  import { Save, AlertTriangle, Loader2, Trash2, Settings2, ShieldAlert, Activity } from 'lucide-svelte';

  const queryClient = useQueryClient();

  // GET /api/config
  const configQuery = createQuery(() => ({
    queryKey: ['config'],
    queryFn: () => api.get<any>('/api/config')
  }));

  // Local state for forms
  let retentionDays = $state(30);
  let samplingEnabled = $state(false);
  let samplingRates = $state({
    debug: 10,
    info: 100,
    warn: 100
  });

  // Sync local state when query finishes
  $effect(() => {
    if (configQuery.data) {
      retentionDays = configQuery.data.retention_default_days;
      samplingEnabled = configQuery.data.sampling_enabled;
      samplingRates = {
        debug: configQuery.data.sampling_debug_rate,
        info: configQuery.data.sampling_info_rate,
        warn: configQuery.data.sampling_warn_rate
      };
    }
  });

  let isSavingRetention = $state(false);
  let isSavingSampling = $state(false);
  let isDeletingLogs = $state(false);
  let retentionMsg = $state({ text: '', type: '' });
  let samplingMsg = $state({ text: '', type: '' });
  let deleteMsg = $state({ text: '', type: '' });

  async function saveRetention() {
    isSavingRetention = true;
    retentionMsg = { text: '', type: '' };
    try {
      await api.put('/api/config', { retention_default_days: retentionDays });
      retentionMsg = { text: 'Settings updated successfully', type: 'success' };
      queryClient.invalidateQueries({ queryKey: ['config'] });
    } catch (e: any) {
      retentionMsg = { text: e.message, type: 'error' };
    } finally {
      isSavingRetention = false;
    }
  }

  async function saveSampling() {
    isSavingSampling = true;
    samplingMsg = { text: '', type: '' };
    try {
      await api.put('/api/config', { 
        sampling_enabled: samplingEnabled,
        sampling_debug_rate: samplingRates.debug,
        sampling_info_rate: samplingRates.info,
        sampling_warn_rate: samplingRates.warn
      });
      samplingMsg = { text: 'Sampling configuration updated', type: 'success' };
      queryClient.invalidateQueries({ queryKey: ['config'] });
    } catch (e: any) {
      samplingMsg = { text: e.message, type: 'error' };
    } finally {
      isSavingSampling = false;
    }
  }

  async function deleteAllLogs() {
    if (!confirm("¿Eliminar TODOS los logs? Esta acción no se puede deshacer.")) return;
    isDeletingLogs = true;
    deleteMsg = { text: '', type: '' };
    try {
      const res = await api.delete<any>('/api/logs/all');
      deleteMsg = { text: `Successfully deleted ${res.deleted} logs`, type: 'success' };
    } catch (e: any) {
      deleteMsg = { text: e.message, type: 'error' };
    } finally {
      isDeletingLogs = false;
    }
  }
</script>

<div class="space-y-12 pb-20">
  <!-- Retention Section -->
  <section class="space-y-4 glow-blue">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 bg-brand-primary/10 flex items-center justify-center rounded-none">
        <Settings2 size={16} class="text-brand-primary" />
      </div>
      <div>
        <h2 class="text-xs font-bold uppercase tracking-widest text-text-primary">Log Retention</h2>
        <p class="text-[10px] text-text-muted font-mono uppercase tracking-tight">Default retention applies to sources without specific configuration</p>
      </div>
    </div>

    <div class="bg-surface border border-border-dim p-6">
      <div class="max-w-md space-y-4">
        <div class="space-y-2">
          <label for="retention" class="text-[10px] font-bold uppercase tracking-widest text-text-muted block">Default Retention (Days)</label>
          <div class="flex gap-0">
            <input 
              id="retention"
              type="number"
              bind:value={retentionDays}
              min="1"
              max="365"
              class="flex-1 bg-surface-elevated border border-border-dim p-3 text-[13px] font-mono text-text-primary focus:border-brand-primary outline-none transition-colors"
            />
            <button 
              onclick={saveRetention}
              disabled={isSavingRetention || configQuery.isLoading}
              class="bg-brand-primary text-white text-[11px] font-bold uppercase tracking-widest px-6 hover:bg-blue-600 transition-all disabled:opacity-50 flex items-center gap-2"
            >
              {#if isSavingRetention}
                <Loader2 size={14} class="animate-spin" />
              {:else}
                <Save size={14} />
              {/if}
              Save
            </button>
          </div>
        </div>
        {#if retentionMsg.text}
          <div class="text-[10px] font-mono uppercase p-3 border-l-2 bg-surface-elevated {retentionMsg.type === 'error' ? 'text-brand-danger border-brand-danger' : 'text-brand-primary border-brand-primary'}">
            &gt; {retentionMsg.type === 'error' ? 'ERROR:' : 'STATUS:'} {retentionMsg.text}
          </div>
        {/if}
      </div>
    </div>
  </section>

  <!-- Sampling Section -->
  <section class="space-y-4 glow-blue">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 bg-brand-primary/10 flex items-center justify-center rounded-none">
        <Activity size={16} class="text-brand-primary" />
      </div>
      <div>
        <h2 class="text-xs font-bold uppercase tracking-widest text-text-primary">Sampling</h2>
        <p class="text-[10px] text-text-muted font-mono uppercase tracking-tight">Reduce storage by sampling logs. Errors are always kept at 100%.</p>
      </div>
    </div>

    <div class="bg-surface border border-border-dim p-6 space-y-8">
      <!-- Toggle -->
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
          <!-- Debug Slider -->
          <div class="space-y-3">
            <div class="flex justify-between items-end">
              <label class="text-[10px] font-bold uppercase tracking-widest text-text-muted">Debug Logs</label>
              <span class="text-[11px] font-mono text-brand-primary">{samplingRates.debug}%</span>
            </div>
            <input 
              type="range" 
              bind:value={samplingRates.debug} 
              min="0" 
              max="100" 
              class="custom-range w-full bg-transparent cursor-pointer" 
            />
          </div>

          <!-- Info Slider -->
          <div class="space-y-3">
            <div class="flex justify-between items-end">
              <label class="text-[10px] font-bold uppercase tracking-widest text-text-muted">Info Logs</label>
              <span class="text-[11px] font-mono text-brand-primary">{samplingRates.info}%</span>
            </div>
            <input 
              type="range" 
              bind:value={samplingRates.info} 
              min="0" 
              max="100" 
              class="custom-range w-full bg-transparent cursor-pointer" 
            />
          </div>

          <!-- Warn Slider -->
          <div class="space-y-3">
            <div class="flex justify-between items-end">
              <label class="text-[10px] font-bold uppercase tracking-widest text-text-muted">Warning Logs</label>
              <span class="text-[11px] font-mono text-brand-primary">{samplingRates.warn}%</span>
            </div>
            <input 
              type="range" 
              bind:value={samplingRates.warn} 
              min="0" 
              max="100" 
              class="custom-range w-full bg-transparent cursor-pointer" 
            />
          </div>

          <!-- Error (Fixed) -->
          <div class="space-y-3 opacity-40">
            <div class="flex justify-between items-end">
              <label class="text-[10px] font-bold uppercase tracking-widest text-text-muted">Error / Fatal</label>
              <span class="text-[11px] font-mono text-text-muted">100% (LOCKED)</span>
            </div>
            <input 
              type="range" 
              value="100" 
              disabled 
              class="custom-range w-full bg-transparent cursor-not-allowed" 
            />
          </div>
        </div>
      {/if}

      <div class="pt-4 space-y-4">
        <button 
          onclick={saveSampling}
          disabled={isSavingSampling || configQuery.isLoading}
          class="bg-brand-primary text-white text-[11px] font-bold uppercase tracking-widest px-8 py-3.5 hover:bg-blue-600 transition-all disabled:opacity-50 flex items-center gap-2"
        >
          {#if isSavingSampling}
            <Loader2 size={14} class="animate-spin" />
          {:else}
            <Save size={14} />
          {/if}
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

  <!-- Danger Zone -->
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
        <h3 class="text-[11px] font-bold uppercase tracking-widest text-text-primary">Purge Data Clusters</h3>
        <p class="text-[10px] text-text-muted font-mono max-w-md uppercase tracking-tight">Drop all log collections across all provisioned sources.</p>
      </div>
      
      <div class="flex flex-col items-end gap-3 min-w-[200px]">
        <button 
          onclick={deleteAllLogs}
          disabled={isDeletingLogs}
          class="w-full bg-brand-danger text-white text-[11px] font-bold uppercase tracking-widest px-8 py-3.5 hover:bg-red-700 transition-all disabled:opacity-50 flex items-center justify-center gap-2"
        >
          {#if isDeletingLogs}
            <Loader2 size={14} class="animate-spin" />
          {:else}
            <Trash2 size={14} />
          {/if}
          Execute Purge
        </button>
        {#if deleteMsg.text}
          <div class="w-full text-[10px] font-mono uppercase p-2 border-l-2 bg-brand-danger/5 {deleteMsg.type === 'error' ? 'text-brand-danger border-brand-danger' : 'text-brand-primary border-brand-primary'}">
            &gt; {deleteMsg.text}
          </div>
        {/if}
      </div>
    </div>
  </section>
</div>

<style>
  button, input {
    border-radius: 0 !important;
  }

  /* Range input base */
  input[type="range"].custom-range {
    -webkit-appearance: none;
    appearance: none;
    height: 40px; /* Large hit area */
    background: transparent;
    overflow: visible;
  }

  /* Webkit Track */
  input[type="range"].custom-range::-webkit-slider-runnable-track {
    width: 100%;
    height: 4px;
    background: var(--color-surface-elevated);
    border: 1px solid var(--color-border-dim);
    cursor: pointer;
  }

  /* Webkit Thumb */
  input[type="range"].custom-range::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 12px;
    height: 24px;
    background: var(--color-brand-primary);
    cursor: pointer;
    margin-top: -11px; /* (Track height 4px / 2) - (Thumb height 24px / 2) - 1px border */
    border: none;
  }

  /* Firefox Track */
  input[type="range"].custom-range::-moz-range-track {
    width: 100%;
    height: 4px;
    background: var(--color-surface-elevated);
    border: 1px solid var(--color-border-dim);
    cursor: pointer;
  }

  /* Firefox Thumb */
  input[type="range"].custom-range::-moz-range-thumb {
    width: 12px;
    height: 24px;
    background: var(--color-brand-primary);
    cursor: pointer;
    border: none;
    border-radius: 0;
  }

  /* Focus states */
  input[type="range"].custom-range:focus {
    outline: none;
  }
  input[type="range"].custom-range:focus::-webkit-slider-runnable-track {
    border-color: var(--color-brand-primary);
  }
</style>
