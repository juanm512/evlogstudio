<script lang="ts">
  import { onMount } from 'svelte';
  import { fly } from 'svelte/transition';
  import { 
    X, Maximize2, Minimize2, Copy, 
    ChevronDown, ChevronUp, ChevronRight, Search,
    Check
  } from 'lucide-svelte';
  import { format, formatDistanceToNow } from 'date-fns';
  import type { Log } from '$lib/types';

  interface Props {
    log: Log;
    onclose: () => void;
  }

  let { log, onclose }: Props = $props();

  // ─── State ────────────────────────────────────────────────────────────────────
  let width = $state(420);
  let isResizing = $state(false);
  let isMaximized = $state(false);
  let searchQuery = $state('');
  let isRawExpanded = $state(false);
  let copiedField = $state<string | null>(null);
  let expandedPaths = $state<Record<string, boolean>>({});
  
  // Truncation state for long strings
  let expandedStrings = $state<Record<string, boolean>>({});

  // ─── Persistence ──────────────────────────────────────────────────────────────
  onMount(() => {
    const savedWidth = localStorage.getItem('evlog_detail_width');
    if (savedWidth) width = Math.min(Math.max(parseInt(savedWidth), 420), 600);

    const savedExpanded = localStorage.getItem('evlog_detail_expanded');
    if (savedExpanded) {
      try {
        expandedPaths = JSON.parse(savedExpanded);
      } catch { /* ignore */ }
    }
  });

  $effect(() => {
    if (!isMaximized && width) localStorage.setItem('evlog_detail_width', width.toString());
  });

  $effect(() => {
    localStorage.setItem('evlog_detail_expanded', JSON.stringify(expandedPaths));
  });

  // ─── Resize Logic ─────────────────────────────────────────────────────────────
  function startResizing(e: MouseEvent) {
    e.preventDefault();
    isResizing = true;
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', stopResizing);
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isResizing) return;
    const newWidth = window.innerWidth - e.clientX;
    width = Math.min(Math.max(newWidth, 420), 600); 
  }

  function stopResizing() {
    isResizing = false;
    window.removeEventListener('mousemove', handleMouseMove);
    window.removeEventListener('mouseup', stopResizing);
  }

  // ─── Helpers ──────────────────────────────────────────────────────────────────
  function getNestedValue(obj: any, path: string): any {
    if (obj[path] !== undefined && obj[path] !== null) return obj[path];
    if (path === 'requestId' && (obj.request_id !== undefined && obj.request_id !== null)) return obj.request_id;
    if (obj.fields && obj.fields[path] !== undefined && obj.fields[path] !== null) return obj.fields[path];
    return null;
  }

  const KNOWN_FIELDS = [
    { label: 'service', path: 'service' },
    { label: 'environment', path: 'environment' },
    { label: 'userId', path: 'userId' },
    { label: 'organizationId', path: 'organizationId' },
    { label: 'requestId', path: 'requestId' },
    { label: 'ipAddress', path: 'ipAddress' },
    { label: 'userAgent', path: 'userAgent' },
    { label: 'procedure', path: 'procedure' },
    { label: 'type', path: 'type' },
    { label: 'ok', path: 'ok' },
  ];

  function formatTimestamp(ts: string) {
    try {
      const d = new Date(ts);
      const relative = formatDistanceToNow(d, { addSuffix: true });
      const absolute = format(d, 'yyyy-MM-dd HH:mm:ss.SSS XXX');
      return `${absolute} (${relative})`;
    } catch {
      return ts;
    }
  }

  async function copyToClipboard(text: string, key: string) {
    try {
      await navigator.clipboard.writeText(text);
      copiedField = key;
      setTimeout(() => { if (copiedField === key) copiedField = null; }, 2000);
    } catch { /* clip fail */ }
  }

  // ─── JSON Rendering Logic ─────────────────────────────────────────────────────
  function togglePath(path: string) {
    expandedPaths[path] = !expandedPaths[path];
  }

  function isExpanded(path: string, depth: number) {
    if (expandedPaths[path] !== undefined) return expandedPaths[path];
    return depth === 0; 
  }

  function filterFields(obj: any, query: string, path = ''): boolean {
    if (!query) return true;
    const q = query.toLowerCase();
    for (const key in obj) {
      const fullPath = path ? `${path}.${key}` : key;
      if (key.toLowerCase().includes(q) || fullPath.toLowerCase().includes(q)) return true;
      const val = obj[key];
      if (typeof val === 'object' && val !== null) {
        if (filterFields(val, query, fullPath)) return true;
      } else if (String(val).toLowerCase().includes(q)) {
        return true;
      }
    }
    return false;
  }

  const methodColors: Record<string, string> = {
    GET: 'blue', POST: 'green', PUT: 'yellow', PATCH: 'yellow', DELETE: 'red'
  };
  const statusColors = (s: number | null) => {
    if (!s) return 'gray';
    if (s < 300) return 'green';
    if (s < 400) return 'blue';
    if (s < 500) return 'yellow';
    return 'red';
  };
  const levelColors: Record<string, string> = {
    debug: 'blue', info: 'green', warn: 'yellow', error: 'red', fatal: 'red'
  };

  function highlightJson(val: any): string {
    const json = JSON.stringify(val, null, 2);
    return json.replace(
      /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g,
      (match) => {
        let cls = 'json-number';
        if (/^"/.test(match)) {
          cls = /:$/.test(match) ? 'json-key' : 'json-string';
        } else if (/true|false/.test(match)) {
          cls = 'json-bool';
        } else if (/null/.test(match)) {
          cls = 'json-null';
        }
        return `<span class="${cls}">${match}</span>`;
      }
    );
  }
</script>

{#snippet fieldRow(key: string, value: any, fullPath: string, depth: number)}
  {@const isObj = typeof value === 'object' && value !== null}
  {@const isArr = Array.isArray(value)}
  {@const expanded = isExpanded(fullPath, depth)}
  {@const showRow = !searchQuery || 
    key.toLowerCase().includes(searchQuery.toLowerCase()) || 
    fullPath.toLowerCase().includes(searchQuery.toLowerCase()) || 
    (!isObj && String(value).toLowerCase().includes(searchQuery.toLowerCase())) || 
    (isObj && filterFields(value, searchQuery, fullPath))}

  {#if showRow}
    <div 
      class="field-row-wrapper" 
      style="--depth: {depth};"
    >
      <div class="field-row group">
        <div class="field-key-area">
          {#if isObj}
            <button class="expand-toggle" onclick={() => togglePath(fullPath)}>
              {#if expanded} <ChevronDown size={14} /> {:else} <ChevronRight size={14} /> {/if}
            </button>
          {:else}
            <div class="expand-placeholder"></div>
          {/if}
          <span class="field-key" title={fullPath}>{key}</span>
        </div>

        <div class="field-value-area">
          {#if isObj}
            <span class="field-type-hint" onclick={() => togglePath(fullPath)}>
              {#if isArr}[{value.length} items]{:else}&#123;...&#125;{/if}
            </span>
          {:else}
            {@const strVal = String(value)}
            {@const isLong = strVal.length > 120}
            {@const isExpandedStr = expandedStrings[fullPath]}
            <div class="value-text-container">
              <span class="field-value type-{typeof value}" class:is-null={value === null}>
                {#if value === null}
                  null
                {:else if typeof value === 'string'}
                  <span class="quotes">"</span>{isLong && !isExpandedStr ? strVal.slice(0, 120) + '...' : strVal}<span class="quotes">"</span>
                  {#if isLong}
                    <button class="see-more" onclick={() => expandedStrings[fullPath] = !expandedStrings[fullPath]}>
                      {isExpandedStr ? 'see less' : 'see more'}
                    </button>
                  {/if}
                {:else}
                  {strVal}
                {/if}
              </span>
            </div>
          {/if}
        </div>

        {#if !isObj}
          <div class="row-actions">
            <button 
              class="action-btn" 
              onclick={() => copyToClipboard(String(value), fullPath)}
              title="Copy value"
            >
              {#if copiedField === fullPath} <Check size={12} class="text-green-400" /> {:else} <Copy size={12} /> {/if}
            </button>
          </div>
        {/if}
      </div>

      {#if isObj && expanded}
        <div class="nested-fields">
          {#each Object.entries(value) as [k, v]}
            {@render fieldRow(k, v, `${fullPath}.${k}`, depth + 1)}
          {/each}
        </div>
      {/if}
    </div>
  {/if}
{/snippet}

<aside 
  class="log-detail" 
  class:maximized={isMaximized}
  style={isMaximized ? '' : `width: ${width}px;`}
  transition:fly={{ x: 40, duration: 200 }}
>
  {#if !isMaximized}
    <div class="resize-handle" onmousedown={startResizing}></div>
  {/if}

  <header class="detail-header">
    <div class="header-top">
      <span class="label">LOG DETAIL</span>
      <div class="header-actions">
        <button class="icon-btn" onclick={() => isMaximized = !isMaximized} title={isMaximized ? 'Restore' : 'Maximize'}>
          {#if isMaximized} <Minimize2 size={16} /> {:else} <Maximize2 size={16} /> {/if}
        </button>
        <button class="icon-btn" onclick={onclose} title="Close"><X size={16} /></button>
      </div>
    </div>

    {#if log.method && log.path}
      <div class="request-row">
        <span class="badge badge-method-{methodColors[log.method] || 'gray'}">{log.method}</span>
        <span class="path font-mono" title={log.path}>{log.path}</span>
        {#if log.status}
          <span class="badge badge-status-{statusColors(log.status)}">{log.status}</span>
        {/if}
      </div>
    {/if}

    <div class="metrics-row">
      {#if log.duration !== null}
        <div class="metric">
          <span class="m-label">DURATION</span>
          <span class="m-val">{log.duration < 1000 ? `${log.duration}ms` : `${(log.duration / 1000).toFixed(2)}s`}</span>
        </div>
      {/if}
      <div class="metric">
        <span class="m-label">SOURCE</span>
        <span class="m-val">{log.source}</span>
      </div>
      {#if log.level}
        <div class="metric">
          <span class="m-label">LEVEL</span>
          <span class="badge-level badge-level-{levelColors[log.level] || 'gray'}">{log.level}</span>
        </div>
      {/if}
      {#if log.request_id}
        <div class="metric group">
          <span class="m-label">REQUEST ID</span>
          <div class="flex items-center gap-1 min-w-0">
            <span class="m-val font-mono truncate" title={log.request_id}>{log.request_id}</span>
            <button 
              class="copy-mini" 
              onclick={() => copyToClipboard(log.request_id!, 'request_id')}
            >
              {#if copiedField === 'request_id'} <Check size={10} /> {:else} <Copy size={10} /> {/if}
            </button>
          </div>
        </div>
      {/if}
    </div>

    <div class="timestamp-row">
      {formatTimestamp(log.timestamp)}
    </div>
  </header>

  <section class="search-area">
    <div class="search-input-wrapper">
      <div class="search-icon">
        <Search size={14} />
      </div>
      <input 
        type="text" 
        placeholder="Filter fields..." 
        bind:value={searchQuery}
        class="search-input"
      />
    </div>
  </section>

  <main class="detail-content scrollbar-custom">
    <div class="sub-section">
      <h3 class="sub-title">KNOWN FIELDS</h3>
      <div class="known-table">
        {#each KNOWN_FIELDS as field}
          {@const val = getNestedValue(log, field.path)}
          {#if val !== null}
            <div class="known-row group">
              <span class="known-key">{field.label}</span>
              <span class="known-val font-mono">{val}</span>
              <div class="row-actions">
                <button class="action-btn" onclick={() => copyToClipboard(String(val), field.path)}>
                  {#if copiedField === field.path} <Check size={12} /> {:else} <Copy size={12} /> {/if}
                </button>
              </div>
            </div>
          {/if}
        {/each}
      </div>
    </div>

    <div class="sub-section">
      <h3 class="sub-title">ALL FIELDS</h3>
      <div class="fields-tree">
        {#each Object.entries(log.fields) as [key, value]}
          {@render fieldRow(key, value, key, 0)}
        {/each}
      </div>
    </div>

    <div class="raw-section" class:expanded={isRawExpanded}>
      <button class="raw-toggle" onclick={() => isRawExpanded = !isRawExpanded}>
        {#if isRawExpanded} <ChevronUp size={16} /> {:else} <ChevronDown size={16} /> {/if}
        <span>Raw JSON</span>
      </button>
      {#if isRawExpanded}
        <div class="raw-box">
          <div class="raw-header">
            <button class="copy-json" onclick={() => copyToClipboard(JSON.stringify(log, null, 2), 'raw_json')}>
              {#if copiedField === 'raw_json'} <Check size={14} /> Copied! {:else} <Copy size={14} /> Copy JSON {/if}
            </button>
          </div>
          <pre class="raw-pre">{@html highlightJson(log)}</pre>
        </div>
      {/if}
    </div>
  </main>

  {#if !isRawExpanded}
    <footer class="detail-footer">
      <button class="footer-btn" onclick={() => copyToClipboard(JSON.stringify(log, null, 2), 'raw_json')}>
        {#if copiedField === 'raw_json'} <Check size={14} /> Copied! {:else} <Copy size={14} /> Copy JSON {/if}
      </button>
    </footer>
  {/if}
</aside>

<style>
  .log-detail {
    position: relative;
    height: 100%;
    background: var(--color-surface);
    border-left: 1px solid var(--color-border-dim);
    display: flex;
    flex-direction: column;
    z-index: 50;
    overflow: hidden;
  }
  .log-detail.maximized {
    position: absolute;
    top: 0;
    right: 0;
    left: 0;
    width: auto !important;
    z-index: 100;
  }

  .resize-handle {
    position: absolute;
    left: -2px;
    top: 0;
    bottom: 0;
    width: 4px;
    cursor: ew-resize;
    z-index: 60;
    transition: background 0.2s;
  }
  .resize-handle:hover { background: var(--color-brand-primary); }

  .detail-header {
    padding: 16px;
    border-bottom: 1px solid var(--color-border-dim);
    background: color-mix(in srgb, var(--color-surface-elevated) 40%, transparent);
  }

  .header-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }
  .label {
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.1em;
    color: var(--color-text-muted);
  }
  .header-actions { display: flex; gap: 4px; }
  .icon-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-muted);
    background: transparent;
    border-radius: 4px;
    transition: all 0.15s;
    cursor: pointer;
  }
  .icon-btn:hover { background: var(--color-surface-elevated); color: var(--color-text-primary); }

  .request-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
    min-width: 0;
  }
  .path {
    font-size: 13px;
    color: var(--color-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .metrics-row {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    margin-bottom: 12px;
  }
  .metric { display: flex; flex-direction: column; min-width: 0; }
  .m-label { font-size: 9px; font-weight: 700; color: var(--color-text-muted); margin-bottom: 1px; }
  .m-val { font-size: 12px; color: var(--color-text-secondary); }

  .timestamp-row {
    font-size: 11px;
    color: var(--color-text-muted);
  }

  /* Badges */
  .badge {
    padding: 2px 6px;
    font-size: 10px;
    font-weight: 700;
    border-radius: 4px;
    font-family: var(--font-mono);
  }
  .badge-method-blue   { background: #1e3a8a; color: #60a5fa; }
  .badge-method-green  { background: #064e3b; color: #34d399; }
  .badge-method-yellow { background: #451a03; color: #fbbf24; }
  .badge-method-red    { background: #7f1d1d; color: #f87171; }
  
  .badge-status-green  { color: #10b981; }
  .badge-status-blue   { color: #3b82f6; }
  .badge-status-yellow { color: #f59e0b; }
  .badge-status-red    { color: #ef4444; }

  .badge-level {
    padding: 2px 6px;
    font-size: 10px;
    font-weight: 700;
    border-radius: 4px;
    text-transform: uppercase;
  }
  .badge-level-blue   { background: rgba(59, 130, 246, 0.15); color: #60a5fa; }
  .badge-level-green  { background: rgba(16, 185, 129, 0.15); color: #34d399; }
  .badge-level-yellow { background: rgba(245, 158, 11, 0.15); color: #fbbf24; }
  .badge-level-red    { background: rgba(239, 68, 68, 0.15); color: #f87171; }

  .search-area {
    padding: 12px 16px;
    border-bottom: 1px solid var(--color-border-dim);
  }
  .search-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }
  .search-icon {
    position: absolute;
    left: 10px;
    color: var(--color-text-muted);
  }
  .search-input {
    width: 100%;
    background: var(--color-background);
    border: 1px solid var(--color-border-dim);
    border-radius: 6px;
    padding: 6px 10px 6px 32px;
    font-size: 13px;
    color: var(--color-text-primary);
    transition: border-color 0.15s;
  }
  .search-input:focus { outline: none; border-color: var(--color-brand-primary); }

  .detail-content {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .sub-section {
    padding: 16px 0;
    border-bottom: 1px solid var(--color-border-dim);
  }
  .sub-title {
    padding: 0 16px;
    font-size: 10px;
    font-weight: 800;
    color: var(--color-text-muted);
    margin-bottom: 8px;
    letter-spacing: 0.05em;
  }

  .known-table { display: flex; flex-direction: column; }
  .known-row {
    display: flex;
    align-items: center;
    padding: 6px 16px;
    gap: 12px;
    transition: background 0.1s;
  }
  .known-row:hover { background: var(--color-surface-elevated); }
  .known-key { width: 120px; font-size: 12px; color: var(--color-text-muted); flex-shrink: 0; }
  .known-val { font-size: 12px; color: var(--color-text-secondary); flex: 1; word-break: break-all; }

  .fields-tree { display: flex; flex-direction: column; }
  .field-row-wrapper { display: flex; flex-direction: column; }
  
  .field-row {
    display: flex;
    align-items: center;
    padding: 4px 16px;
    gap: 8px;
    min-height: 28px;
    transition: background 0.1s;
    padding-left: calc(16px + (var(--depth) * 16px));
  }
  .field-row:hover { background: var(--color-surface-elevated); }

  .field-key-area { display: flex; align-items: center; gap: 4px; min-width: 140px; }
  .expand-toggle {
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-muted);
    background: transparent;
    cursor: pointer;
  }
  .expand-placeholder { width: 16px; }
  .field-key {
    font-size: 13px;
    color: var(--color-text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .field-value-area { flex: 1; display: flex; align-items: center; min-width: 0; }
  .field-type-hint { font-size: 12px; color: var(--color-text-muted); font-style: italic; cursor: pointer; }
  .value-text-container { display: flex; align-items: center; min-width: 0; }
  .field-value { font-family: var(--font-mono); font-size: 12px; word-break: break-all; }
  .field-value.type-string { color: #86efac; }
  .field-value.type-number { color: #93c5fd; }
  .field-value.type-boolean { color: #fde047; }
  .field-value.is-null { color: var(--color-text-muted); }
  .quotes { color: var(--color-text-muted); opacity: 0.6; }

  .see-more { 
    margin-left: 8px; 
    font-size: 10px; 
    color: var(--color-brand-primary); 
    text-decoration: underline; 
    cursor: pointer;
    background: transparent;
  }

  .row-actions {
    display: flex;
    gap: 4px;
    opacity: 0;
    transition: opacity 0.1s;
    margin-left: auto;
  }
  .known-row:hover .row-actions,
  .field-row:hover .row-actions { opacity: 1; }
  .action-btn {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-muted);
    background: transparent;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.1s;
  }
  .action-btn:hover { background: var(--color-background); color: var(--color-text-primary); }

  .raw-section { padding: 16px; }
  .raw-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    font-weight: 700;
    color: var(--color-text-muted);
    background: transparent;
    cursor: pointer;
    width: 100%;
    padding: 8px 0;
    transition: color 0.15s;
  }
  .raw-toggle:hover { color: var(--color-text-secondary); }

  .raw-box {
    margin-top: 8px;
    border: 1px solid var(--color-border-dim);
    border-radius: 8px;
    background: var(--color-background);
    overflow: hidden;
  }
  .raw-header {
    background: var(--color-surface-elevated);
    padding: 6px 12px;
    display: flex;
    justify-content: flex-end;
  }
  .copy-json {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text-secondary);
    background: transparent;
    cursor: pointer;
  }
  .raw-pre {
    padding: 16px;
    font-size: 12px;
    font-family: var(--font-mono);
    line-height: 1.5;
    margin: 0;
    overflow-x: auto;
  }

  .detail-footer {
    padding: 12px 16px;
    border-top: 1px solid var(--color-border-dim);
    background: color-mix(in srgb, var(--color-surface-elevated) 40%, transparent);
  }
  .footer-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 8px;
    font-size: 13px;
    font-weight: 600;
    border: 1px solid var(--color-border-dim);
    background: var(--color-background);
    color: var(--color-text-secondary);
    border-radius: 6px;
    transition: all 0.15s;
    cursor: pointer;
  }
  .footer-btn:hover { background: var(--color-surface-elevated); color: var(--color-text-primary); }

  :global(.json-key)    { color: #60a5fa; }
  :global(.json-string) { color: #86efac; }
  :global(.json-number) { color: #93c5fd; }
  :global(.json-bool)   { color: #fde047; }
  :global(.json-null)   { color: #9ca3af; }

  .copy-mini {
    padding: 2px;
    color: var(--color-text-muted);
    opacity: 0;
    transition: opacity 0.1s;
    background: transparent;
    cursor: pointer;
  }
  .metric:hover .copy-mini { opacity: 1; }
  .copy-mini:hover { color: var(--color-text-secondary); }

  .scrollbar-custom::-webkit-scrollbar { width: 6px; }
  .scrollbar-custom::-webkit-scrollbar-thumb { background: var(--color-border-dim); border-radius: 10px; }
  .scrollbar-custom::-webkit-scrollbar-thumb:hover { background: var(--color-text-muted); }
</style>
