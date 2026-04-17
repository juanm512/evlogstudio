<script lang="ts">
  import { SlidersHorizontal } from 'lucide-svelte';

  interface Props {
    availableColumns: string[];
    activeColumns: string[];
    anchorTop: number;
    anchorRight: number;
    onchange: (cols: string[]) => void;
    onclose: () => void;
  }

  const BASE_COLUMNS = ['timestamp', 'source', 'level', 'message'];

  let { availableColumns, activeColumns, anchorTop, anchorRight, onchange, onclose }: Props = $props();

  function toggle(col: string) {
    if (BASE_COLUMNS.includes(col)) return; // locked
    const next = activeColumns.includes(col)
      ? activeColumns.filter(c => c !== col)
      : [...activeColumns, col];
    onchange(next);
  }

  const COLUMN_LABELS: Record<string, string> = {
    timestamp: 'Timestamp',
    source: 'Source',
    level: 'Level',
    message: 'Message',
  };

  function labelFor(col: string): string {
    return COLUMN_LABELS[col] ?? col;
  }

  // All displayed: base + extras
  let allColumns = $derived([
    ...BASE_COLUMNS,
    ...availableColumns.filter(c => !BASE_COLUMNS.includes(c)),
  ]);

  let panelRef = $state<HTMLElement>();

  $effect(() => {
    const handler = (e: MouseEvent) => {
      if (panelRef && !panelRef.contains(e.target as Node)) {
        onclose();
      }
    };
    // Delay to avoid immediate close from the click that opened it
    const timer = setTimeout(() => {
      window.addEventListener('click', handler);
    }, 10);
    
    return () => {
      clearTimeout(timer);
      window.removeEventListener('click', handler);
    };
  });
</script>

<div
  bind:this={panelRef}
  class="picker-panel"
  role="dialog"
  aria-label="Column picker"
  style="top: {anchorTop}px; right: {anchorRight}px;"
>
  <div class="picker-header">
    <SlidersHorizontal size={13} class="text-text-muted" />
    <span>Visible Columns</span>
  </div>
  <div class="picker-list">
    {#each allColumns as col}
      {@const locked = BASE_COLUMNS.includes(col)}
      {@const active = activeColumns.includes(col)}
      <label class="picker-row {locked ? 'locked' : ''}">
        <input
          type="checkbox"
          checked={active}
          disabled={locked}
          onchange={() => toggle(col)}
          class="picker-checkbox"
          aria-label="Toggle column {labelFor(col)}"
        />
        <span class="picker-label">{labelFor(col)}</span>
        {#if locked}
          <span class="picker-lock">required</span>
        {/if}
      </label>
    {/each}
  </div>
  <div class="picker-footer">
    <button onclick={onclose} class="picker-close-btn" aria-label="Close column picker">Done</button>
  </div>
</div>

<style>
  .picker-panel {
    position: fixed;
    z-index: 9999;
    width: 220px;
    background-color: var(--color-surface-elevated);
    border: 1px solid var(--color-border-dim);
    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
    display: flex;
    flex-direction: column;
  }
  .picker-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--color-border-dim);
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-text-muted);
  }
  .picker-list {
    display: flex;
    flex-direction: column;
    max-height: 280px;
    overflow-y: auto;
    padding: 4px 0;
  }
  .picker-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 12px;
    cursor: pointer;
    font-size: 13px;
    color: var(--color-text-secondary);
    transition: background 0.1s;
  }
  .picker-row:hover:not(.locked) {
    background: var(--color-surface);
    color: var(--color-text-primary);
  }
  .picker-row.locked {
    cursor: default;
    opacity: 0.7;
  }
  .picker-checkbox {
    width: 14px;
    height: 14px;
    accent-color: var(--color-brand-primary);
    flex-shrink: 0;
  }
  .picker-label { flex: 1; font-family: var(--font-mono); font-size: 12px; }
  .picker-lock {
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-muted);
    border: 1px solid var(--color-border-dim);
    padding: 1px 5px;
  }
  .picker-footer {
    padding: 8px 12px;
    border-top: 1px solid var(--color-border-dim);
  }
  .picker-close-btn {
    width: 100%;
    padding: 5px 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--color-text-secondary);
    background: transparent;
    border: 1px solid var(--color-border-dim);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .picker-close-btn:hover {
    background: var(--color-surface);
    color: var(--color-text-primary);
  }
  .picker-close-btn:focus-visible { outline: 2px solid var(--color-brand-primary); outline-offset: 2px; }
</style>
