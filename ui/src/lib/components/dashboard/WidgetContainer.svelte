<script lang="ts">
  import type { Widget } from '$lib/types';
  import { Pencil, Trash2, ChevronUp, ChevronDown, ArrowLeftRight } from 'lucide-svelte';

  interface Props {
    widget: Widget;
    isAdmin: boolean;
    isFirst?: boolean;
    isLast?: boolean;
    children?: import('svelte').Snippet;
    onedit?: () => void;
    ondelete?: () => void;
    onmoveup?: () => void;
    onmovedown?: () => void;
    ontogglewidth?: () => void;
  }

  let {
    widget,
    isAdmin,
    isFirst = false,
    isLast = false,
    children,
    onedit,
    ondelete,
    onmoveup,
    onmovedown,
    ontogglewidth,
  }: Props = $props();
</script>

<div class="bg-surface border border-border-dim flex flex-col min-h-[200px]">
  <!-- Header -->
  <div class="flex items-center justify-between px-4 py-2.5 border-b border-border-dim flex-shrink-0">
    <h3 class="text-[13px] font-bold text-text-primary truncate">{widget.title}</h3>
    {#if isAdmin}
      <div class="flex items-center gap-1 flex-shrink-0">
        <button
          onclick={onmoveup}
          disabled={isFirst}
          class="widget-ctrl-btn"
          title="Move up"
          aria-label="Move widget up"
        >
          <ChevronUp size={13} />
        </button>
        <button
          onclick={onmovedown}
          disabled={isLast}
          class="widget-ctrl-btn"
          title="Move down"
          aria-label="Move widget down"
        >
          <ChevronDown size={13} />
        </button>
        <button
          onclick={ontogglewidth}
          class="widget-ctrl-btn"
          title="Toggle width ({widget.width === 'half' ? 'expand to full' : 'shrink to half'})"
          aria-label="Toggle widget width"
        >
          <ArrowLeftRight size={13} />
        </button>
        <button
          onclick={onedit}
          class="widget-ctrl-btn"
          title="Edit widget"
          aria-label="Edit widget"
        >
          <Pencil size={13} />
        </button>
        <button
          onclick={ondelete}
          class="widget-ctrl-btn danger"
          title="Delete widget"
          aria-label="Delete widget"
        >
          <Trash2 size={13} />
        </button>
      </div>
    {/if}
  </div>

  <!-- Body -->
  <div class="flex-1 p-4 min-h-0">
    {@render children?.()}
  </div>
</div>

<style>
  .widget-ctrl-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    background: transparent;
    border: 1px solid transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: color 0.12s, background 0.12s, border-color 0.12s;
    border-radius: 0;
  }
  .widget-ctrl-btn:hover {
    color: var(--color-text-primary);
    background: var(--color-surface-elevated);
    border-color: var(--color-border-dim);
  }
  .widget-ctrl-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
  .widget-ctrl-btn.danger:hover {
    color: var(--color-brand-danger);
    border-color: var(--color-brand-danger, #EF4444);
    background: rgba(239, 68, 68, 0.08);
  }
  .widget-ctrl-btn:focus-visible {
    outline: 2px solid var(--color-brand-primary);
    outline-offset: 1px;
  }
</style>
