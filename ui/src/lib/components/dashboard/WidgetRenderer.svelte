<script lang="ts">
  import type { Widget, WidgetConfig, QueryResult } from '$lib/types';
  import { api } from '$lib/api';
  import WidgetBarChart from './WidgetBarChart.svelte';
  import WidgetLineChart from './WidgetLineChart.svelte';
  import WidgetNumber from './WidgetNumber.svelte';
  import WidgetTable from './WidgetTable.svelte';

  interface Props { widget: Widget; }

  let { widget }: Props = $props();

  type State = 'loading' | 'error' | 'ok';

  let state = $state<State>('loading');
  let result = $state<QueryResult | null>(null);
  let errorMsg = $state('');

  async function fetchData() {
    state = 'loading';
    result = null;
    errorMsg = '';
    try {
      let config: WidgetConfig;
      try {
        const parsed = JSON.parse(widget.config);
        if (!parsed || typeof parsed.metric !== 'string') {
          throw new Error('missing metric');
        }
        // Ensure filters and sources are always arrays
        config = {
          ...parsed,
          filters: Array.isArray(parsed.filters) ? parsed.filters : [],
          sources: Array.isArray(parsed.sources) ? parsed.sources : [],
        };
      } catch (e) {
        throw new Error(`Invalid widget config: ${e instanceof Error ? e.message : 'parse error'}`);
      }
      result = await api.post<QueryResult>('/api/dashboards/query', config);
      state = 'ok';
    } catch (e: unknown) {
      errorMsg = e instanceof Error ? e.message : 'Unknown error';
      state = 'error';
    }
  }

  $effect(() => {
    // Re-fetch whenever widget changes
    void widget;
    fetchData();
  });

  let config = $derived.by(() => {
    try { return JSON.parse(widget.config) as WidgetConfig; }
    catch { return null; }
  });
</script>

{#if state === 'loading'}
  <div class="flex flex-col gap-2 h-full min-h-[100px] animate-pulse">
    <div class="h-4 bg-surface-elevated w-3/4"></div>
    <div class="h-4 bg-surface-elevated w-1/2"></div>
    <div class="flex-1 bg-surface-elevated"></div>
  </div>

{:else if state === 'error'}
  <div class="flex flex-col items-center justify-center h-full min-h-[100px] gap-3">
    <p class="text-[12px] font-mono text-brand-danger">{errorMsg}</p>
    <button
      onclick={fetchData}
      class="text-[11px] font-mono text-text-muted border border-border-dim px-3 py-1
             hover:text-text-primary hover:border-brand-primary/40 transition-colors"
    >
      Retry
    </button>
  </div>

{:else if result}
  {#if widget.type === 'number'}
    <WidgetNumber
      value={result.data[0]?.value ?? 0}
      metric={result.meta.metric}
    />
  {:else if widget.type === 'bar'}
    <WidgetBarChart data={result.data} title={widget.title} />
  {:else if widget.type === 'line'}
    <WidgetLineChart data={result.data} />
  {:else if widget.type === 'table'}
    <WidgetTable data={result.data} />
  {/if}
{/if}
