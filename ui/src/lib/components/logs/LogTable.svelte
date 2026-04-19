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
    onsort?: (detail: { field: string | null; dir: 'asc' | 'desc' | null }) => void;
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
    duration_ms: 'Duration',
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
  let sortField = $state<string | null>(null);
  let sortDir   = $state<'asc' | 'desc' | null>(null);

  function handleHeaderClick(col: string) {
    if (sortField !== col) {
      sortField = col;
      sortDir   = 'asc';
    } else if (sortDir === 'asc') {
      sortDir = 'desc';
    } else {
      sortField = null;
      sortDir   = null;
    }
    onsort?.({ field: sortField, dir: sortDir });
  }

  function getFieldType(col: string): string {
    return schemaFields.find(f => f.field_path === col)?.field_type ?? 'string';
  }

  const TOP_LEVEL_LOG_COLS = new Set([
    'timestamp','source','level','message',
    'service','environment','method','path','status','duration_ms','request_id','error',
  ]);

  function getSortValue(log: Log, col: string): unknown {
    if (TOP_LEVEL_LOG_COLS.has(col)) return (log as Record<string, unknown>)[col] ?? '';
    const parts = col.split('.');
    let current: unknown = log.fields ?? {};
    for (const part of parts) {
      if (current == null || typeof current !== 'object') return '';
      current = (current as Record<string, unknown>)[part];
    }
    return current ?? '';
  }

  let sortedLogs = $derived.by<Log[]>(() => {
    if (!sortField || !sortDir) return logs;
    const field     = sortField;
    const dir       = sortDir;
    const isNumeric = getFieldType(field) === 'number';
    return [...logs].sort((a, b) => {
      const av = getSortValue(a, field);
      const bv = getSortValue(b, field);
      let cmp: number;
      if (isNumeric) {
        cmp = (Number(av) || 0) - (Number(bv) || 0);
      } else {
        cmp = String(av).localeCompare(String(bv));
      }
      return dir === 'asc' ? cmp : -cmp;
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
              <th
                class="col-header {col === 'message' ? 'col-message' : col === 'timestamp' ? 'col-ts' : ''}"
                onclick={() => handleHeaderClick(col)}
                aria-sort={sortField === col ? (sortDir === 'asc' ? 'ascending' : 'descending') : 'none'}
              >
                <span class="col-header-inner">
                  {headerFor(col)}
                  {#if sortField === col && sortDir === 'asc'}
                    <span class="sort-icon" aria-hidden="true">↑</span>
                  {:else if sortField === col && sortDir === 'desc'}
                    <span class="sort-icon" aria-hidden="true">↓</span>
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
