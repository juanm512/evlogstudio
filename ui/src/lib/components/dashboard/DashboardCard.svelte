<script lang="ts">
  import type { Dashboard } from '$lib/types';
  import { format, parseISO } from 'date-fns';

  interface Props {
    dashboard: Dashboard;
    onclick?: () => void;
  }

  let { dashboard, onclick }: Props = $props();

  let widgetCount = $derived(dashboard.widget_count ?? dashboard.widgets?.length ?? 0);

  function fmt(dateStr: string) {
    try { return format(parseISO(dateStr), 'MMM dd, yyyy'); }
    catch { return dateStr; }
  }
</script>

<button
  class="w-full text-left bg-surface border border-border-dim hover:border-brand-primary/40
         hover:bg-surface-elevated transition-all duration-150 p-5 focus-visible:outline-2
         focus-visible:outline-brand-primary"
  {onclick}
>
  <div class="flex items-start justify-between gap-3 mb-3">
    <h3 class="text-sm font-bold text-text-primary truncate">{dashboard.name}</h3>
    <span class="flex-shrink-0 text-[11px] font-mono text-text-muted bg-surface px-2 py-0.5 border border-border-dim">
      {widgetCount} widget{widgetCount !== 1 ? 's' : ''}
    </span>
  </div>
  {#if dashboard.description}
    <p class="text-[12px] text-text-secondary mb-3 line-clamp-2">{dashboard.description}</p>
  {:else}
    <p class="text-[12px] text-text-muted mb-3 italic">No description</p>
  {/if}
  <p class="text-[11px] font-mono text-text-muted">
    {fmt(dashboard.updated_at)}
  </p>
</button>
