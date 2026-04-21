<script lang="ts">
  import type { Log, SchemaField } from '$lib/types';
  import LogRow from './LogRow.svelte';
  import EmptyState from '$lib/components/common/EmptyState.svelte';
  import { FileText } from 'lucide-svelte';

  interface Props {
    logs: Log[];
    activeColumns: string[];
    selectedLog: Log | null;
    onselect: (log: Log | null) => void;
    scrollEl?: HTMLDivElement;
    schemaFields?: SchemaField[];
    onsort?: (detail: { entries: Array<{ field: string; dir: 'asc' | 'desc' }> }) => void;
  }

  let { logs, activeColumns, selectedLog, onselect, scrollEl = $bindable(), schemaFields = [], onsort }: Props = $props();

  const COLUMN_HEADERS: Record<string, string> = {
    timestamp:   'Timestamp',
    source:      'Source',
    level:       'Level',
    message:     'Message',
    service:     'Service',
    environment: 'Env',
    method:      'Method',
    path:        'Path',
    status:      'Status',
    duration:    'Duration (ms)',
    request_id:  'Request ID',
    error:       'Error',
  };

  function headerFor(col: string): string {
    return COLUMN_HEADERS[col] ?? col;
  }

  function handleSelect(log: Log) {
    if (selectedLog?.id === log.id) {
      onselect(null);
    } else {
      onselect(log);
    }
  }

  // ─── Sort state ───────────────────────────────────────────────────────────────
  let sortEntries = $state<Array<{ field: string; dir: 'asc' | 'desc' }>>([]);

  function handleHeaderClick(col: string) {
    const existingIdx = sortEntries.findIndex(s => s.field === col);

    if (existingIdx === -1) {
      // Append to stack: First clicked is primary, next is secondary, etc.
      sortEntries.push({ field: col, dir: 'asc' });
    } else {
      // Rotate: asc -> desc -> remove
      if (sortEntries[existingIdx].dir === 'asc') {
        sortEntries[existingIdx].dir = 'desc';
      } else {
        sortEntries.splice(existingIdx, 1);
      }
    }
    onsort?.({ entries: $state.snapshot(sortEntries) });
  }

  function getFieldType(col: string): string {
    if (col === 'duration_ms' || col === 'duration' || col === 'status') return 'number';
    return schemaFields.find(f => f.field_path === col)?.field_type ?? 'string';
  }

  const TOP_LEVEL_LOG_COLS = new Set([
    'timestamp','source','level','message',
    'service','environment','method','path','status','duration_ms','duration','request_id','error',
  ]);

  /**
   * Robustly extracts a numeric value from a string like "2.34s", "404ms", or raw numbers.
   */
  function parseNumeric(val: unknown): number {
    if (typeof val === 'number') return val;
    if (val == null || val === '') return 0;
    
    const s = String(val).trim().toLowerCase();
    // Use the same scale logic as the backend normalization
    if (s.endsWith('ms')) return parseFloat(s.slice(0, -2)) || 0;
    if (s.endsWith('s')) return (parseFloat(s.slice(0, -1)) || 0) * 1000;
    if (s.endsWith('m')) return (parseFloat(s.slice(0, -1)) || 0) * 60000;
    
    return parseFloat(s) || 0;
  }

  function getSortValue(log: Log, col: string): unknown {
    // 1. Precise mappings for special UI columns
    if (col === 'duration') return log.duration_ms ?? log.fields?.duration ?? 0;
    if (col === 'duration_ms') return log.duration_ms ?? 0;
    
    // 2. Top-level columns
    if (TOP_LEVEL_LOG_COLS.has(col)) {
        const val = (log as any)[col];
        if (val !== undefined) return val;
    }

    // 3. Nested fields
    const parts = col.split('.');
    let current: any = log.fields ?? {};
    for (const part of parts) {
      if (current == null || typeof current !== 'object') return '';
      current = current[part];
    }
    return current ?? '';
  }

  let sortedLogs = $derived.by<Log[]>(() => {
    if (sortEntries.length === 0) return logs;
    
    return [...logs].sort((a, b) => {
      for (const entry of sortEntries) {
        const field = entry.field;
        const dir = entry.dir;
        const isNumeric = getFieldType(field) === 'number';
        
        const av = getSortValue(a, field);
        const bv = getSortValue(b, field);
        
        let cmp = 0;
        if (isNumeric) {
          cmp = parseNumeric(av) - parseNumeric(bv);
        } else {
          cmp = String(av).localeCompare(String(bv));
        }

        if (cmp !== 0) {
          return dir === 'asc' ? cmp : -cmp;
        }
      }
      return 0;
    });
  });

  // ─── Keyboard navigation ──────────────────────────────────────────────────────
  let tbodyEl = $state<HTMLTableSectionElement | null>(null);

  function handleTbodyKeydown(e: KeyboardEvent) {
    const rows    = Array.from(tbodyEl?.querySelectorAll<HTMLElement>('[role="button"]') ?? []);
    const focused = document.activeElement as HTMLElement;
    const idx     = rows.indexOf(focused);
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      rows[Math.min(idx + 1, rows.length - 1)]?.focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      rows[Math.max(idx - 1, 0)]?.focus();
    }
  }
</script>

<div class="table-wrapper">
  {#if logs.length === 0}
    <EmptyState
      icon={FileText}
      title="No events found"
      description="No events found for this time range. Adjust your filters or load more data."
    />
  {:else}
    <div role="grid" aria-label="Log events" class="table-scroll" tabindex="-1" onkeydown={handleTbodyKeydown} bind:this={scrollEl}>
      <table class="log-table">
        <thead>
          <tr>
            {#each activeColumns as col}
              {@const entry = sortEntries.find(s => s.field === col)}
              {@const entryIdx = sortEntries.findIndex(s => s.field === col)}
              <th
                class="col-header {col === 'message' ? 'col-message' : col === 'timestamp' ? 'col-ts' : ''}"
                onclick={() => handleHeaderClick(col)}
                aria-sort={entry ? (entry.dir === 'asc' ? 'ascending' : 'descending') : 'none'}
                title="Click to sort (Additive cycle: Asc -> Desc -> Off)"
              >
                <span class="col-header-inner">
                  {headerFor(col)}
                  {#if entry}
                    <span class="sort-icon" aria-hidden="true">
                      {entry.dir === 'asc' ? '↑' : '↓'}{#if sortEntries.length > 1}<sup class="text-[9px] ml-[-1px] opacity-70">{entryIdx + 1}</sup>{/if}
                    </span>
                  {/if}
                </span>
              </th>
            {/each}
          </tr>
        </thead>
        <tbody bind:this={tbodyEl}>
          {#each sortedLogs as log (log.id)}
            <LogRow
              {log}
              columns={activeColumns}
              selected={selectedLog?.id === log.id}
              onselect={() => handleSelect(log)}
            />
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .table-wrapper {
    flex: 1;
    overflow: hidden;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .table-scroll {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }
  .log-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    table-layout: auto;
  }
  .col-header {
    position: sticky;
    top: 0;
    z-index: 10;
    padding: 0 12px;
    height: 36px;
    text-align: left;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-text-muted);
    background-color: var(--color-surface);
    border-bottom: 1px solid var(--color-border-dim);
    white-space: nowrap;
    user-select: none;
    cursor: pointer;
  }
  .col-header:hover {
    color: var(--color-text-secondary);
    background-color: var(--color-surface-elevated);
  }
  .col-header:focus-visible {
    outline: 2px solid var(--color-brand-primary);
    outline-offset: -2px;
  }
  .col-header-inner {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .sort-icon {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-brand-primary);
    line-height: 1;
  }
  .col-ts      { min-width: 180px; }
  .col-message { width: 100%; }
</style>
