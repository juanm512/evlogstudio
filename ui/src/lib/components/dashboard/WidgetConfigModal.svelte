<script lang="ts">
  import type { Widget, WidgetConfig } from '$lib/types';
  import { api } from '$lib/api';
  import QueryBuilder from './QueryBuilder.svelte';
  import WidgetRenderer from './WidgetRenderer.svelte';

  interface Props {
    open: boolean;
    dashboardId: string;
    widget: Widget | null;
    onsave?: () => void;
    onclose?: () => void;
  }

  let { open, dashboardId, widget, onsave, onclose }: Props = $props();

  let title = $state(widget?.title ?? '');
  let widgetType = $state<Widget['type']>(widget?.type ?? 'bar');
  let width = $state<Widget['width']>(widget?.width ?? 'half');
  let saving = $state(false);
  let errorMsg = $state('');

  // Parse or default config
  function parseConfig(): WidgetConfig {
    if (widget?.config) {
      try {
        const parsed = JSON.parse(widget.config);
        return {
          ...parsed,
          filters: Array.isArray(parsed.filters) ? parsed.filters : [],
          sources: Array.isArray(parsed.sources) ? parsed.sources : [],
        };
      } catch { /* ignore */ }
    }
    return {
      metric: 'count',
      field: null,
      group_by: { field: 'timestamp', interval: '1h' },
      filters: [],
      sources: [],
      from: null,
      to: null,
    };
  }

  let config = $state<WidgetConfig>(parseConfig());

  // Preview widget (fake Widget object for WidgetRenderer)
  let previewWidget = $derived<Widget>({
    id: '_preview',
    dashboard_id: dashboardId,
    title,
    type: widgetType,
    width,
    position: 0,
    config: JSON.stringify(config),
  });

  // Debounced preview trigger
  let previewKey = $state(0);
  let debounceTimer: ReturnType<typeof setTimeout>;

  function onConfigChange(cfg: WidgetConfig) {
    config = cfg;
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => { previewKey++; }, 500);
  }

  async function save() {
    if (!title.trim()) { errorMsg = 'Title is required'; return; }
    saving = true;
    errorMsg = '';
    try {
      const body = {
        title: title.trim(),
        type: widgetType,
        width,
        config: JSON.stringify(config),
      };
      if (widget) {
        await api.put(`/api/widgets/${widget.id}`, body);
      } else {
        await api.post(`/api/dashboards/${dashboardId}/widgets`, body);
      }
      onsave?.();
    } catch (e: unknown) {
      errorMsg = e instanceof Error ? e.message : 'Error saving widget';
    } finally {
      saving = false;
    }
  }
</script>

{#if open}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4"
    role="dialog"
    aria-modal="true"
    aria-label="Widget configuration"
  >
    <div class="bg-surface border border-border-dim w-full max-w-5xl max-h-[90vh] flex flex-col shadow-2xl">
      <!-- Header -->
      <div class="flex items-center justify-between px-5 py-3.5 border-b border-border-dim flex-shrink-0">
        <h2 class="text-[13px] font-bold text-text-primary uppercase tracking-wider">
          {widget ? 'Edit Widget' : 'New Widget'}
        </h2>
        <button
          onclick={onclose}
          class="text-text-muted hover:text-text-primary transition-colors text-lg leading-none px-1"
          aria-label="Close"
        >×</button>
      </div>

      <!-- Body: columnas fijas lado a lado -->
      <div class="flex-1 min-h-0 overflow-hidden grid grid-cols-[1fr_320px]">
        <!-- Left: Config (scrollable) -->
        <div class="overflow-y-auto p-5 border-r border-border-dim flex flex-col gap-4 min-w-0">
          <!-- Basic fields -->
          <div class="grid grid-cols-3 gap-3">
            <div class="col-span-1 flex flex-col gap-1">
              <label class="text-[10px] font-mono text-text-muted uppercase tracking-wider">Title</label>
              <input
                type="text"
                class="bg-surface-elevated border border-border-dim text-text-primary font-mono text-[12px]
                       px-3 py-2 focus:outline-none focus:border-brand-primary transition-colors w-full"
                placeholder="Widget title"
                bind:value={title}
              />
            </div>
            <div class="flex flex-col gap-1">
              <label class="text-[10px] font-mono text-text-muted uppercase tracking-wider">Type</label>
              <select
                class="bg-surface-elevated border border-border-dim text-text-primary font-mono text-[12px]
                       px-3 py-2 focus:outline-none focus:border-brand-primary transition-colors w-full"
                bind:value={widgetType}
              >
                <option value="bar">Bar Chart</option>
                <option value="line">Line Chart</option>
                <option value="number">Number</option>
                <option value="table">Table</option>
              </select>
            </div>
            <div class="flex flex-col gap-1">
              <label class="text-[10px] font-mono text-text-muted uppercase tracking-wider">Width</label>
              <select
                class="bg-surface-elevated border border-border-dim text-text-primary font-mono text-[12px]
                       px-3 py-2 focus:outline-none focus:border-brand-primary transition-colors w-full"
                bind:value={width}
              >
                <option value="half">Half (50%)</option>
                <option value="full">Full (100%)</option>
              </select>
            </div>
          </div>

          <QueryBuilder bind:config onchange={onConfigChange} />
        </div>

        <!-- Right: Preview (sticky, non-scroll) -->
        <div class="flex flex-col gap-2 p-5 overflow-hidden">
          <p class="text-[10px] font-mono text-text-muted uppercase tracking-wider flex-shrink-0">Preview</p>
          {#key previewKey}
            <div class="border border-border-dim bg-surface-elevated flex-1 min-h-0 p-3 overflow-hidden">
              <WidgetRenderer widget={previewWidget} />
            </div>
          {/key}
        </div>
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-between px-5 py-3.5 border-t border-border-dim flex-shrink-0">
        {#if errorMsg}
          <p class="text-[12px] font-mono text-brand-danger">{errorMsg}</p>
        {:else}
          <div></div>
        {/if}
        <div class="flex gap-2">
          <button
            onclick={onclose}
            class="text-[12px] font-mono px-4 py-2 border border-border-dim text-text-muted
                   hover:text-text-primary hover:border-brand-primary/40 transition-colors"
          >
            Cancel
          </button>
          <button
            onclick={save}
            disabled={saving}
            class="text-[12px] font-mono px-4 py-2 bg-brand-primary text-white
                   hover:bg-brand-primary/90 disabled:opacity-50 transition-colors"
          >
            {saving ? 'Saving…' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  select, input { border-radius: 0 !important; }
</style>
