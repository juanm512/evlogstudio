<script lang="ts">
  import type { Log } from '$lib/types';
  import { format } from 'date-fns';

  interface Props {
    log: Log;
    columns: string[];
    selected: boolean;
    onselect: () => void;
  }

  let { log, columns, selected, onselect }: Props = $props();

  function formatTimestamp(ts: string): string {
    try {
      return format(new Date(ts), 'MMM dd HH:mm:ss.SSS');
    } catch {
      return ts;
    }
  }

  function getNestedValue(obj: Record<string, unknown>, path: string): string {
    const parts = path.split('.');
    let current: unknown = obj;
    for (const part of parts) {
      if (current == null || typeof current !== 'object') return '';
      current = (current as Record<string, unknown>)[part];
    }
    if (current == null) return '';
    const str = typeof current === 'object' ? JSON.stringify(current) : String(current);
    return str.length > 40 ? str.slice(0, 40) + '…' : str;
  }

  const TOP_LEVEL_LOG_COLS = new Set([
    'service','environment','method','path','status','duration','request_id','error',
  ]);

  function getCellValue(col: string): string {
    if (col === 'timestamp') return formatTimestamp(log.timestamp);
    if (col === 'source') return log.source ?? '';
    if (col === 'level') return log.level ?? '';
    if (col === 'message') {
      const msg = log.message ?? '';
      return msg.length > 80 ? msg.slice(0, 80) + '…' : msg;
    }
    if (col === 'duration' || col === 'duration_ms') {
      return log.duration != null ? `${log.duration}ms` : '';
    }
    if (TOP_LEVEL_LOG_COLS.has(col)) {
      const val = (log)[col as keyof Log];
      if (val == null) return '';
      return String(val);
    }
    return getNestedValue(log.fields ?? {}, col);
  }

  function statusClass(status: number | null): string {
    if (status == null) return '';
    if (status >= 500) return 'status-5xx';
    if (status >= 400) return 'status-4xx';
    if (status >= 300) return 'status-3xx';
    if (status >= 200) return 'status-2xx';
    return '';
  }

  const levelBadge: Record<string, string> = {
    debug: 'badge-debug',
    info:  'badge-info',
    warn:  'badge-warn',
    error: 'badge-error',
    fatal: 'badge-fatal',
  };

  const levelBorder: Record<string, string> = {
    debug: 'border-l-[var(--color-event-debug)]',
    info:  'border-l-[var(--color-event-info)]',
    warn:  'border-l-[var(--color-brand-warning)]',
    error: 'border-l-[var(--color-brand-danger)]',
    fatal: 'border-l-[var(--color-brand-danger)]',
  };

  let rowBorder = $derived(levelBorder[log.level ?? ''] ?? 'border-l-transparent');
</script>

<!-- svelte-ignore a11y_interactive_supports_focus -->
<tr
  role="button"
  aria-expanded={selected}
  aria-label="Log entry from {log.source} at {formatTimestamp(log.timestamp)}"
  tabindex="0"
  onclick={onselect}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onselect(); } }}
  class="log-row border-l-2 {rowBorder} {selected ? 'row-selected' : 'row-default'}"
>
  {#each columns as col}
    <td class="log-cell">
      {#if col === 'level'}
        <span class="level-badge {levelBadge[log.level ?? ''] ?? 'badge-unknown'}">
          {log.level ?? '—'}
        </span>
      {:else if col === 'timestamp'}
        <span class="font-mono text-text-muted">{getCellValue(col)}</span>
      {:else if col === 'message'}
        <span class="font-mono text-text-secondary truncate max-w-[400px] block" title={log.message ?? ''}>{getCellValue(col)}</span>
      {:else if col === 'status'}
        {#if log.status != null}
          <span class="status-badge {statusClass(log.status)}">{log.status}</span>
        {:else}
          <span class="font-mono text-text-muted">—</span>
        {/if}
      {:else if col === 'error'}
        {#if log.error != null}
          <span class="error-badge">error</span>
        {/if}
      {:else}
        <span class="font-mono text-text-muted truncate max-w-[200px] block" title={getCellValue(col)}>{getCellValue(col) || '—'}</span>
      {/if}
    </td>
  {/each}
</tr>

<style>
  .log-row {
    height: 36px;
    cursor: pointer;
    transition: background-color 0.1s ease;
    border-bottom: 1px solid var(--color-border-dim);
    outline: none;
  }
  .log-row:focus-visible {
    outline: 2px solid var(--color-brand-primary);
    outline-offset: -2px;
  }
  .row-default {
    background-color: transparent;
  }
  .row-default:hover {
    background-color: var(--color-surface-elevated);
  }
  .row-selected {
    background-color: color-mix(in srgb, var(--color-brand-primary) 8%, transparent);
  }
  .log-cell {
    padding: 0 12px;
    font-size: 13px;
    white-space: nowrap;
    vertical-align: middle;
  }

  .level-badge {
    display: inline-block;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 0;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .badge-debug  { background: color-mix(in srgb, #A78BFA 12%, transparent); color: #A78BFA; }
  .badge-info   { background: color-mix(in srgb, #60A5FA 12%, transparent); color: #60A5FA; }
  .badge-warn   { background: color-mix(in srgb, #F59E0B 12%, transparent); color: #F59E0B; }
  .badge-error  { background: color-mix(in srgb, #EF4444 12%, transparent); color: #EF4444; }
  .badge-fatal  { background: color-mix(in srgb, #991B1B 20%, transparent); color: #FCA5A5; }
  .badge-unknown { background: color-mix(in srgb, #71717A 12%, transparent); color: #71717A; }

  .status-badge {
    display: inline-block;
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 600;
    padding: 1px 6px;
  }
  .status-2xx { background: color-mix(in srgb, #22C55E 12%, transparent); color: #22C55E; }
  .status-3xx { background: color-mix(in srgb, #60A5FA 12%, transparent); color: #60A5FA; }
  .status-4xx { background: color-mix(in srgb, #F59E0B 12%, transparent); color: #F59E0B; }
  .status-5xx { background: color-mix(in srgb, #EF4444 12%, transparent); color: #EF4444; }

  .error-badge {
    display: inline-block;
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 1px 6px;
    background: color-mix(in srgb, #EF4444 12%, transparent);
    color: #EF4444;
  }
</style>
