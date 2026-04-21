<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import { createQuery, keepPreviousData } from '@tanstack/svelte-query';
  import { api } from '$lib/api';
  import type { Log, LogsResponse, PollResponse, SchemaResponse, FilterCondition, Operator, Source } from '$lib/types';
  import FilterBar from '$lib/components/logs/FilterBar.svelte';
  import LogTable from '$lib/components/logs/LogTable.svelte';
  import LogDetail from '$lib/components/logs/LogDetail.svelte';
  import { ChevronDown, Loader2 } from 'lucide-svelte';

  // ─── Constants ────────────────────────────────────────────────────────────────
  const COLUMNS_KEY = 'evlog_columns';
  const DEFAULT_COLUMNS = ['timestamp', 'source', 'level', 'duration', 'message'];

  // ─── State ────────────────────────────────────────────────────────────────────
  interface Filters {
    search: string;
    from: string;
    to: string;
    level: string;
    method: string;
    environment: string;
    status: string;
  }

  let filters = $state<Filters>({ search: '', from: '', to: '', level: '', method: '', environment: '', status: '' });
  let conditions    = $state<FilterCondition[]>([]);
  let activeColumns = $state<string[]>(DEFAULT_COLUMNS);
  let selectedLog   = $state<Log | null>(null);
  let cursor        = $state<string | null>(null);
  let extraLogs     = $state<Log[]>([]);
  let loadingMore   = $state(false);

  // ─── Live mode state ──────────────────────────────────────────────────────────
  let isLive         = $state(false);
  let lastId         = $state<string | null>(null);
  let newLogsBuffer  = $state<Log[]>([]);
  let liveLogs       = $state<Log[]>([]);
  let lastUpdated    = $state<Date | null>(null);
  let secondsAgo     = $state(0);
  let atTop          = $state(true);
  let tableScrollEl  = $state<HTMLDivElement | undefined>(undefined);

  let pollInterval: ReturnType<typeof setInterval> | null = null;
  let tickInterval: ReturnType<typeof setInterval> | null = null;

  // ─── Persist columns ─────────────────────────────────────────────────────────
  onMount(() => {
    if (browser) {
      const saved = localStorage.getItem(COLUMNS_KEY);
      if (saved) {
        try {
          const parsed = JSON.parse(saved) as string[];
          if (Array.isArray(parsed) && parsed.length > 0) activeColumns = parsed;
        } catch { /* ignore */ }
      }
    }
  });

  onDestroy(() => {
    clearLiveIntervals();
  });

  function persistColumns(cols: string[]) {
    activeColumns = cols;
    if (browser) localStorage.setItem(COLUMNS_KEY, JSON.stringify(cols));
  }

  // ─── Build query URL ──────────────────────────────────────────────────────────
  // function buildLogsUrl(f: Filters, sources: string[], cur: string | null): string {
  //   const params = new URLSearchParams();
  //   if (f.search)      params.set('search', f.search);
  //   if (f.from)        params.set('from', new Date(f.from).toISOString());
  //   if (f.to)          params.set('to',   new Date(f.to).toISOString());
  //   if (sources.length === 1) params.set('source', sources[0]);
  //   if (f.level)       params.set('level', f.level);
  //   if (f.method)      params.set('method', f.method);
  //   if (f.environment) params.set('environment', f.environment);
  //   if (f.status)      params.set('status', f.status);
  //   if (cur) params.set('cursor', cur);
  //   params.set('limit', '100');
  //   return `/api/logs?${params.toString()}`;
  // }

  function buildPollUrl(): string {
    const params = new URLSearchParams();
    if (lastId) params.set('since_id', lastId);
    if (sourcesValue.length === 1) params.set('source', sourcesValue[0]);
    if (filters.search) params.set('search', filters.search);
    return `/api/logs/poll?${params.toString()}`;
  }

  // function esc(v: string): string {
  //   return v.replace(/'/g, "''");
  // }

  // Top-level DB columns — referenced directly, not via json_extract_string
  const TOP_LEVEL_COLS = new Set([
    'level', 'source', 'message', 'duration', 'timestamp',
    'service', 'environment', 'method', 'path', 'status', 'request_id', 'error',
  ]);

  function buildSearchRequest(f: Filters, sources: string[], conds: FilterCondition[], cur: string | null = null, lim: number = 100) {
    const apiConds = [];
    if (f.level) apiConds.push({ field: 'level', operator: '=', value: f.level });
    if (f.method) apiConds.push({ field: 'method', operator: '=', value: f.method });
    if (f.environment) apiConds.push({ field: 'environment', operator: '=', value: f.environment });
    if (f.status) apiConds.push({ field: 'status', operator: '=', value: f.status });

    for (const c of conds) {
      if (!c.field) continue;
      if (c.operator !== 'exists' && c.value.trim() === '') continue;
      
      let operator = '=';
      switch (c.operator) {
        case 'neq': operator = '!='; break;
        case 'contains': operator = 'contains'; break;
        case 'starts': operator = 'starts_with'; break;
        case 'gt': operator = '>'; break;
        case 'lt': operator = '<'; break;
        case 'exists': operator = 'exists'; break;
      }

      apiConds.push({
        field: c.field,
        operator,
        value: c.value
      });
    }

    return {
      source: sources.length === 1 ? sources[0] : null,
      from: f.from ? new Date(f.from).toISOString() : null,
      to: f.to ? new Date(f.to).toISOString() : null,
      conditions: apiConds,
      limit: lim,
      cursor: cur
    };
  }

  // ─── Sources state ────────────────────────────────────────────────────────────
  let sourcesValue = $state<string[]>([]);

  const sourcesQuery = createQuery(() => ({
    queryKey: ['sources'],
    queryFn: () => api.get<Source[]>('/api/sources'),
    staleTime: 60_000,
  }));
  let availableSources = $derived(sourcesQuery.data ?? []);

  // Reset pagination + live buffers whenever query params change
  $effect(() => {
    void filters.search; void filters.from; void filters.to;
    void filters.level; void filters.method; void filters.environment; void filters.status;
    void sourcesValue.length;
    void conditions.length;
    cursor        = null;
    extraLogs     = [];
    newLogsBuffer = [];
    liveLogs      = [];
    lastId        = null;
  });

  // Track scroll position for banner logic
  $effect(() => {
    const el = tableScrollEl;
    if (!el) return;
    function onScroll() { atTop = (el as HTMLDivElement).scrollTop < 5; }
    el.addEventListener('scroll', onScroll, { passive: true });
    return () => el.removeEventListener('scroll', onScroll);
  });

  // ─── Schema query ─────────────────────────────────────────────────────────────
  const schemaQuery = createQuery(() => ({
    queryKey: ['schema', ...sourcesValue],
    queryFn: async () => {
      const params = new URLSearchParams();
      if (sourcesValue.length > 0) params.set('source', sourcesValue.join(','));
      return api.get<SchemaResponse>(`/api/schema?${params.toString()}`);
    },
    staleTime: 60_000,
  }));

  let availableColumns = $derived(
    (schemaQuery.data?.fields ?? [])
      .map((f) => f.field_path)
      .filter((v, i, arr) => arr.indexOf(v) === i)
  );

  // Synthetic top-level column fields exposed in AdvancedFilters
  const BUILTIN_FIELDS: import('$lib/types').SchemaField[] = [
    { source: '_builtin', field_path: 'level',   field_type: 'string', seen_count: 0, last_seen: '' },
    { source: '_builtin', field_path: 'source',  field_type: 'string', seen_count: 0, last_seen: '' },
    { source: '_builtin', field_path: 'duration', field_type: 'number', seen_count: 0, last_seen: '' },
    { source: '_builtin', field_path: 'message', field_type: 'string', seen_count: 0, last_seen: '' },
  ];

  let allSchemaFields = $derived([
    ...BUILTIN_FIELDS,
    ...(schemaQuery.data?.fields ?? []).map(f => {
      if (f.field_path === 'duration' || f.field_path === 'status') {
        return { ...f, field_type: 'number' };
      }
      return f;
    }),
  ]);

  // ─── Logs query ───────────────────────────────────────────────────────────────
  const logsQuery = createQuery(() => ({
    queryKey: ['logs', filters.search, filters.from, filters.to, filters.level, filters.method, filters.environment, filters.status, ...sourcesValue, ...conditions.map(c => `${c.id}:${c.field}:${c.operator}:${c.value}`)],
    queryFn: async () => {
      const payload = buildSearchRequest(filters, sourcesValue, conditions);
      return api.post<LogsResponse>('/api/logs/search', payload);
    },
    staleTime: 30_000,
    placeholderData: keepPreviousData,
  }));

  let allLogs = $derived.by<Log[]>(() => {
    const logs = [...liveLogs, ...(logsQuery.data?.logs ?? []), ...extraLogs];
    const filtered = sourcesValue.length > 1 ? logs.filter(l => sourcesValue.includes(l.source)) : logs;
    const seen = new Set<string>();
    return filtered.filter(l => { if (seen.has(l.id)) return false; seen.add(l.id); return true; });
  });

  let nextCursor = $derived<string | null>(
    extraLogs.length > 0 ? cursor : (logsQuery.data?.next_cursor ?? null)
  );

  // ─── Load more ────────────────────────────────────────────────────────────────
  async function loadMore() {
    const cur = nextCursor;
    if (!cur || loadingMore) return;
    loadingMore = true;
    try {
      const payload = buildSearchRequest(filters, sourcesValue, conditions, cur);
      const res = await api.post<LogsResponse>('/api/logs/search', payload);
      extraLogs = [...extraLogs, ...res.logs];
      cursor    = res.next_cursor;
    } finally {
      loadingMore = false;
    }
  }

  // Close detail panel if the log is filtered out
  $effect(() => {
    if (selectedLog && allLogs.length > 0) {
      if (!allLogs.find(l => l.id === selectedLog!.id)) selectedLog = null;
    }
  });

  // ─── Live mode ────────────────────────────────────────────────────────────────

  function matchesConditions(log: Log): boolean {
    for (const c of conditions) {
      if (!c.field) continue;
      if (c.operator !== 'exists' && c.value.trim() === '') continue;
      const raw = TOP_LEVEL_COLS.has(c.field)
        ? (log)[c.field as keyof Log]
        : (log.fields as Record<string, unknown>)?.[c.field];
      const strVal = raw == null ? null : String(raw);
      switch (c.operator as Operator) {
        case 'eq':       if (strVal?.toLowerCase() !== c.value.toLowerCase()) return false; break;
        case 'neq':      if (strVal?.toLowerCase() === c.value.toLowerCase()) return false; break;
        case 'contains': if (!strVal?.toLowerCase().includes(c.value.toLowerCase())) return false; break;
        case 'starts':   if (!strVal?.toLowerCase().startsWith(c.value.toLowerCase())) return false; break;
        case 'gt': { const n = parseFloat(strVal ?? ''); if (isNaN(n) || n <= parseFloat(c.value)) return false; break; }
        case 'lt': { const n = parseFloat(strVal ?? ''); if (isNaN(n) || n >= parseFloat(c.value)) return false; break; }
        case 'exists':   if (raw == null) return false; break;
      }
    }
    return true;
  }

  function clearLiveIntervals() {
    if (pollInterval) { clearInterval(pollInterval); pollInterval = null; }
    if (tickInterval) { clearInterval(tickInterval); tickInterval = null; }
  }

  async function poll() {
    try {
      const res = await api.get<PollResponse>(buildPollUrl());
      lastUpdated = new Date();
      secondsAgo  = 0;
      if (res.count > 0) {
        lastId = res.last_id;
        const incoming = [...res.logs].reverse().filter(matchesConditions);
        if (atTop) {
          liveLogs = [...incoming, ...liveLogs];
          tableScrollEl?.scrollTo({ top: 0, behavior: 'smooth' });
        } else {
          newLogsBuffer = [...incoming, ...newLogsBuffer];
        }
      }
    } catch { /* silent */ }
  }

  function flushBuffer() {
    liveLogs      = [...newLogsBuffer, ...liveLogs];
    newLogsBuffer = [];
    tableScrollEl?.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function toggleLive() {
    isLive = !isLive;
    if (isLive) {
      lastId      = allLogs[0]?.id ?? null;
      lastUpdated = null;
      secondsAgo  = 0;
      pollInterval = setInterval(poll, 5_000);
      tickInterval = setInterval(() => {
        if (lastUpdated) {
          secondsAgo = Math.floor((Date.now() - lastUpdated.getTime()) / 1000);
        }
      }, 1_000);
    } else {
      clearLiveIntervals();
      newLogsBuffer = [];
    }
  }
</script>

<div class="logs-page">
  <FilterBar
    {filters}
    {availableColumns}
    {activeColumns}
    {isLive}
    schemaFields={allSchemaFields}
    {conditions}
    sources={availableSources}
    selectedSources={sourcesValue}
    onchange={(f) => { filters = { ...f }; }}
    oncolumnschange={persistColumns}
    onconditionschange={(c) => { conditions = c; }}
    onsourceschange={(s) => { sourcesValue = s; cursor = null; }}
    ontoggleLive={toggleLive}
  />

  <div class="logs-body">
    <div class="table-area">
      {#if logsQuery.isPending}
        <div class="status-center">
          <Loader2 size={22} class="spin-icon" />
          <span class="text-sm text-muted mt-2">Loading logs…</span>
        </div>
      {:else if logsQuery.isError}
        <div class="status-center">
          <p class="err-text">
            Error: {logsQuery.error instanceof Error ? logsQuery.error.message : 'Unknown error'}
          </p>
          <p class="hint-text">Check your backend connection and filters.</p>
        </div>
      {:else}
        {#if newLogsBuffer.length > 0 && !atTop}
          <button class="new-logs-banner" onclick={flushBuffer} aria-live="polite">
            ↑ {newLogsBuffer.length} new log{newLogsBuffer.length !== 1 ? 's' : ''} — click to view
          </button>
        {/if}

        <LogTable
          logs={allLogs}
          {activeColumns}
          {selectedLog}
          schemaFields={schemaQuery.data?.fields ?? []}
          bind:scrollEl={tableScrollEl}
          onselect={(log) => { selectedLog = log; }}
          onsort={(_detail) => { /* sorting */ }}
        />

        {#if nextCursor}
          <div class="load-more-bar">
            <button
              onclick={loadMore}
              disabled={isLive || loadingMore || conditions.length > 0}
              class="load-more-btn"
              aria-label="Load more log entries"
              title={isLive ? 'Disabled in live mode' : conditions.length > 0 ? 'Disabled with advanced filters' : undefined}
            >
              {#if loadingMore}
                <Loader2 size={14} class="spin-icon" />
                Loading…
              {:else}
                <ChevronDown size={14} />
                Load more
              {/if}
            </button>
            <span class="count-label">Showing {allLogs.length} event{allLogs.length !== 1 ? 's' : ''}</span>
            {#if isLive && lastUpdated}
              <span class="update-ts">Updated {secondsAgo}s ago</span>
            {/if}
          </div>
        {:else if allLogs.length > 0}
          <div class="load-more-bar justify-end">
            {#if isLive && lastUpdated}
              <span class="update-ts">Updated {secondsAgo}s ago</span>
            {/if}
            <span class="count-label">{allLogs.length} event{allLogs.length !== 1 ? 's' : ''} — end of results</span>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Detail panel -->
    {#if selectedLog}
      <LogDetail
        log={selectedLog}
        onclose={() => { selectedLog = null; }}
      />
    {/if}
  </div>
</div>

<style>
  .logs-page { display: flex; flex-direction: column; flex: 1; min-height: 0; height: 100%; }
  .logs-body { display: flex; flex: 1; min-height: 0; overflow: hidden; }
  .table-area { display: flex; flex-direction: column; flex: 1; min-width: 0; overflow: hidden; position: relative; }
  .status-center { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 64px 24px; color: var(--color-text-muted); }
  .text-muted { color: var(--color-text-muted); font-size: 14px; }
  .err-text { font-size: 13px; font-family: var(--font-mono); color: var(--color-brand-danger); }
  .hint-text { font-size: 12px; color: var(--color-text-muted); margin-top: 4px; }
  .count-label { font-size: 12px; color: var(--color-text-muted); }
  .update-ts { font-size: 12px; font-family: var(--font-mono); color: var(--color-brand-success); margin-left: auto; }
  :global(.spin-icon) { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .load-more-bar { display: flex; align-items: center; gap: 12px; padding: 8px 12px; border-top: 1px solid var(--color-border-dim); background-color: var(--color-surface); flex-shrink: 0; }
  .justify-end { justify-content: flex-end; }
  .load-more-btn { display: flex; align-items: center; gap: 6px; padding: 5px 14px; font-size: 12px; font-weight: 600; color: var(--color-text-secondary); background: transparent; border: 1px solid var(--color-border-dim); cursor: pointer; transition: background 0.15s, color 0.15s; }
  .load-more-btn:hover:not(:disabled) { background: var(--color-surface-elevated); color: var(--color-text-primary); }
  .load-more-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .new-logs-banner { position: absolute; top: 8px; left: 50%; transform: translateX(-50%); z-index: 10; display: flex; align-items: center; gap: 6px; padding: 6px 16px; font-size: 12px; font-weight: 600; font-family: var(--font-mono); color: var(--color-brand-success); background: color-mix(in srgb, var(--color-brand-success) 12%, var(--color-surface)); border: 1px solid color-mix(in srgb, var(--color-brand-success) 35%, transparent); cursor: pointer; white-space: nowrap; transition: background 0.15s; }
  .new-logs-banner:hover { background: color-mix(in srgb, var(--color-brand-success) 20%, var(--color-surface)); }
</style>
