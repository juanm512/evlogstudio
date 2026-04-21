<script lang="ts">
  import { onMount } from 'svelte';
  import { createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { api } from '$lib/api';
  import { currentUser } from '$lib/stores';
  import type { VolumeResponse, ErrorRateResponse, LogsResponse, SchemaResponse, Source, Log } from '$lib/types';
  import MetricCard from '$lib/components/analytics/MetricCard.svelte';
  import VolumeChart from '$lib/components/analytics/VolumeChart.svelte';
  import CustomSelect from '$lib/components/common/CustomSelect.svelte';
  import SourceMultiSelect from '$lib/components/common/SourceMultiSelect.svelte';
  import TopValues from '$lib/components/analytics/TopValues.svelte';
  import { toCSV, downloadCSV } from '$lib/utils';
  import { subHours, subDays, formatISO } from 'date-fns';
  import { Download, Loader2 } from 'lucide-svelte';

  const queryClient = useQueryClient();

  // ─── Filters state ────────────────────────────────────────────────────────────
  let interval = $state('hour');
  let timeRange = $state('24h');
  let selectedField = $state('');
  let isExporting = $state(false);

  // ─── Sources state ────────────────────────────────────────────────────────────
  let sourcesValue = $state<string[]>([]);

  const sourcesQuery = createQuery(() => ({
    queryKey: ['sources'],
    queryFn: () => api.get<Source[]>('/api/sources'),
    staleTime: 60_000,
  }));
  let availableSources = $derived(sourcesQuery.data ?? []);

  let userValue = $state<{ email: string; role: string } | null>(null);
  $effect(() => {
    const unsub = currentUser.subscribe(v => { userValue = v; });
    return unsub;
  });

  let isAdmin = $derived(userValue?.role === 'admin');

  // ─── Options ──────────────────────────────────────────────────────────────────
  const intervalOptions = [
    { id: 'minute', label: 'Minute' },
    { id: 'hour', label: 'Hour' },
    { id: 'day', label: 'Day' },
    { id: 'week', label: 'Week' }
  ];

  const rangeOptions = [
    { id: '1h', label: 'Last hour' },
    { id: '24h', label: 'Last 24h' },
    { id: '7d', label: 'Last 7 days' },
    { id: '30d', label: 'Last 30 days' }
  ];

  // ─── Time range ───────────────────────────────────────────────────────────────
  let now = $state(new Date());

  onMount(() => {
    const timer = setInterval(() => {
      now = new Date();
    }, 30_000);

    const handleVisibility = () => {
      if (!document.hidden) {
        queryClient.invalidateQueries({ queryKey: ['analytics'] });
      }
    };
    document.addEventListener('visibilitychange', handleVisibility);

    return () => {
      clearInterval(timer);
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  });

  let to = $derived(formatISO(now));
  let from = $derived(() => {
    switch (timeRange) {
      case '1h':  return formatISO(subHours(now, 1));
      case '24h': return formatISO(subDays(now, 1));
      case '7d':  return formatISO(subDays(now, 7));
      case '30d': return formatISO(subDays(now, 30));
      default:    return formatISO(subDays(now, 1));
    }
  });

  // ─── SQL sanitizer ────────────────────────────────────────────────────────────
  function sanitize(s: string): string {
    return s.replace(/'/g, "''");
  }

  // ─── Queries ──────────────────────────────────────────────────────────────────
  const volumeQuery = createQuery(() => ({
    queryKey: ['analytics', 'volume', interval, from(), to, ...sourcesValue],
    queryFn: () => {
      const params = new URLSearchParams({
        interval,
        from: from(),
        to,
        ...(sourcesValue.length === 1 ? { source: sourcesValue[0] } : {})
      });
      return api.get<VolumeResponse>(`/api/analytics/volume?${params}`);
    },
    refetchInterval: 30_000,
    refetchIntervalInBackground: false,
  }));

  const errorRateQuery = createQuery(() => ({
    queryKey: ['analytics', 'errors', from(), to, ...sourcesValue],
    queryFn: () => {
      const params = new URLSearchParams({
        from: from(),
        to,
        ...(sourcesValue.length === 1 ? { source: sourcesValue[0] } : {})
      });
      return api.get<ErrorRateResponse>(`/api/analytics/errors?${params}`);
    },
    refetchInterval: 30_000,
    refetchIntervalInBackground: false,
  }));

  const schemaQuery = createQuery(() => ({
    queryKey: ['schema', ...sourcesValue],
    queryFn: () => {
      const params = new URLSearchParams();
      if (sourcesValue.length === 1) params.set('source', sourcesValue[0]);
      return api.get<SchemaResponse>(`/api/schema?${params.toString()}`);
    },
    staleTime: 60_000,
  }));

  let fieldPaths = $derived(schemaQuery.data?.fields.map(f => f.field_path) ?? []);
  let hasDuration = $derived(fieldPaths.includes('duration'));

  interface PercentileResult { p50: number | null; p95: number | null; p99: number | null; }

  const percentileQuery = createQuery(() => ({
    queryKey: ['analytics', 'percentiles', from(), to, ...sourcesValue],
    queryFn: () => {
      const params = new URLSearchParams({
        from: from(),
        to,
        ...(sourcesValue.length === 1 ? { source: sourcesValue[0] } : {})
      });
      return api.get<PercentileResult>(`/api/analytics/percentiles?${params}`);
    },
    enabled: isAdmin,
    refetchInterval: 30_000,
    refetchIntervalInBackground: false,
  }));

  function formatMs(v: number | null | undefined): string {
    if (v == null || isNaN(v)) return '—';
    return `${Math.round(v)}ms`;
  }

  let p50 = $derived(formatMs(percentileQuery.data?.p50));
  let p95 = $derived(formatMs(percentileQuery.data?.p95));
  let p99 = $derived(formatMs(percentileQuery.data?.p99));

  // ─── Derived metric values ────────────────────────────────────────────────────
  let totalLogs = $derived(volumeQuery.data?.data.reduce((acc, p) => acc + p.count, 0) ?? '—');
  let errorsCount = $derived(errorRateQuery.data?.errors ?? '—');
  let errorRate = $derived(errorRateQuery.data ? `${(errorRateQuery.data.rate * 100).toFixed(1)}%` : '—');
  let activeSourcesCount = $derived(sourcesValue.length > 0 ? sourcesValue.length : '—');

  // ─── Source prop for new components ──────────────────────────────────────────
  let singleSource = $derived(sourcesValue.length === 1 ? sourcesValue[0] : null);

  // ─── Export CSV ───────────────────────────────────────────────────────────────
  async function exportCSV() {
    isExporting = true;
    try {
      const payload = {
        source: sourcesValue.length === 1 ? sourcesValue[0] : null,
        from: from(),
        to,
        conditions: []
      };
      const logs = await api.post<Log[]>('/api/logs/export', payload);
      const csv = toCSV(logs);
      downloadCSV(csv, `logs_export_${Date.now()}.csv`);
    } finally {
      isExporting = false;
    }
  }
</script>

<div class="h-full overflow-y-auto space-y-6 max-w-full">

  <!-- 2. Header & Controls -->
  <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-4 bg-surface-elevated p-6 border-b border-border-dim">
    <div>
      <h1 class="text-xl font-bold text-text-primary uppercase tracking-tight font-sans">Analytics</h1>
      <p class="text-[11px] text-text-muted font-mono uppercase tracking-wider mt-1">Event Distribution and Error Metrics</p>
    </div>

    <div class="flex flex-wrap items-center gap-6">
      <div class="min-w-[160px]">
        <SourceMultiSelect
          label="Source"
          sources={availableSources}
          value={sourcesValue}
          onSelect={(s) => (sourcesValue = s)}
        />
      </div>

      <div class="min-w-[140px]">
        <CustomSelect
          label="Interval"
          options={intervalOptions}
          value={interval}
          onSelect={(id) => interval = id}
        />
      </div>

      <div class="min-w-[160px]">
        <CustomSelect
          label="Time Range"
          options={rangeOptions}
          value={timeRange}
          onSelect={(id) => timeRange = id}
        />
      </div>

      {#if isAdmin}
        <div class="min-w-[200px]">
          <CustomSelect
            label="Top values for field"
            options={fieldPaths.map(f => ({ id: f, label: f }))}
            value={selectedField || null}
            placeholder="Select a field..."
            onSelect={(id) => selectedField = id}
          />
        </div>
      {/if}

      {#if isAdmin}
        <div class="flex flex-col gap-1.5">
          <label class="text-[10px] text-text-muted uppercase font-bold tracking-widest">Export</label>
          <button
            onclick={exportCSV}
            disabled={isExporting}
            class="flex items-center gap-2 bg-surface border border-border-dim px-3 py-2 text-[10px] font-mono font-bold uppercase tracking-widest text-text-secondary hover:bg-surface-elevated hover:text-text-primary transition-colors disabled:opacity-50 disabled:cursor-not-allowed min-h-[40px]"
          >
            {#if isExporting}
              <Loader2 size={12} class="animate-spin" />
              <span>Exporting...</span>
            {:else}
              <Download size={12} />
              <span>Export CSV</span>
            {/if}
          </button>
        </div>
      {/if}

      <div class="flex flex-col gap-1.5">
        <label class="text-[10px] text-text-muted uppercase font-bold tracking-widest">Status</label>
        <div class="flex items-center gap-2 bg-surface border border-border-dim p-2 px-3 min-h-[40px]">
          <div class="w-1.5 h-1.5 bg-brand-success rounded-none"></div>
          <span class="text-[10px] text-text-primary font-mono uppercase">Live</span>
        </div>
      </div>
    </div>
  </div>

  <div class="px-6 pb-12 space-y-8">
    <!-- 3. MetricCards Grid -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-0 border-t border-l border-border-dim">
      <div class="border-r border-b border-border-dim">
        <MetricCard
          label="Total Events"
          value={totalLogs}
          sublabel={volumeQuery.isSuccess ? "[ STREAM: ACTIVE ]" : "[ STREAM: OFFLINE ]"}
        />
      </div>
      <div class="border-r border-b border-border-dim">
        <MetricCard
          label="System Errors"
          value={errorsCount}
          sublabel={Number(errorsCount) > 0 ? "[ STATUS: ALERT ]" : "[ STATUS: NOMINAL ]"}
        />
      </div>
      <div class="border-r border-b border-border-dim">
        <MetricCard
          label="Error Rate"
          value={errorRate}
          sublabel="[ METRIC: STABILITY ]"
        />
      </div>
      <div class="border-r border-b border-border-dim">
        <MetricCard
          label="Active Sources"
          value={activeSourcesCount}
          sublabel="[ SCOPE: FILTERED ]"
        />
      </div>
    </div>

    <!-- Duration Percentiles (admin + duration in schema) -->
    {#if hasDuration}
      <div>
        <div class="flex items-center gap-3 px-1 mb-3">
          <h2 class="text-[12px] font-bold text-text-secondary uppercase tracking-[3px]">Duration Percentiles</h2>
          {#if percentileQuery.isFetching}
            <div class="w-2 h-[2px] bg-brand-primary animate-pulse"></div>
          {/if}
        </div>
        <div class="grid grid-cols-3 gap-0 border-t border-l border-border-dim">
          <div class="border-r border-b border-border-dim">
            <MetricCard label="p50 Duration" value={p50} sublabel="[ PERCENTILE: MEDIAN ]" />
          </div>
          <div class="border-r border-b border-border-dim">
            <MetricCard label="p95 Duration" value={p95} sublabel="[ PERCENTILE: 95TH ]" />
          </div>
          <div class="border-r border-b border-border-dim">
            <MetricCard label="p99 Duration" value={p99} sublabel="[ PERCENTILE: 99TH ]" />
          </div>
        </div>
      </div>
    {/if}

    <!-- 4. VolumeChart -->
    <div class="space-y-4">
      <div class="flex items-center justify-between px-1">
        <div class="flex items-center gap-3">
          <h2 class="text-[12px] font-bold text-text-secondary uppercase tracking-[3px]">Ingestion Volume</h2>
          {#if volumeQuery.isFetching || errorRateQuery.isFetching}
            <div class="flex items-center gap-2">
              <div class="w-2 h-[2px] bg-brand-primary animate-pulse"></div>
              <span class="text-[9px] text-text-muted uppercase font-mono">Syncing...</span>
            </div>
          {/if}
        </div>
      </div>

      {#if volumeQuery.isLoading}
        <div class="w-full h-[320px] bg-surface-elevated border border-border-dim flex flex-col items-center justify-center gap-4">
          <div class="w-12 h-1 bg-brand-primary/20 overflow-hidden">
            <div class="w-full h-full bg-brand-primary animate-[loading_1.5s_infinite]"></div>
          </div>
          <span class="text-text-muted text-[10px] uppercase tracking-[3px] font-mono">Loading Data Streams</span>
        </div>
      {:else if volumeQuery.error}
        <div class="w-full h-[320px] border border-brand-danger/30 bg-brand-danger/5 flex items-center justify-center p-6">
          <div class="text-center space-y-2">
            <p class="text-brand-danger text-[11px] font-mono uppercase tracking-widest font-bold">Inbound Stream Error</p>
            <p class="text-text-muted text-[10px] font-mono">{volumeQuery.error.message}</p>
          </div>
        </div>
      {:else}
        <div class="glow-blue">
          <VolumeChart data={volumeQuery.data?.data ?? []} {interval} />
        </div>
      {/if}
    </div>

    <!-- 6. Grid 2 cols: TopPaths | TopValues (admin only) -->
    {#if isAdmin}
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
        {#if selectedField}
          <TopValues
            field={selectedField}
            source={singleSource}
            from={from()}
            {to}
          />
        {:else}
          <div class="space-y-3">
            <div class="flex items-center gap-3 px-1">
              <h3 class="text-[12px] font-bold text-text-secondary uppercase tracking-[3px]">Top Values</h3>
            </div>
            <div class="border border-border-dim bg-surface p-6 flex items-center justify-center">
              <p class="text-text-muted text-[11px] font-mono uppercase tracking-[2px]">Select a field above to see top values</p>
            </div>
          </div>
        {/if}
      </div>
    {/if}

  </div>
</div>

<style>
  select {
    appearance: none;
    border-radius: 0;
  }

  @keyframes loading {
    0%   { transform: translateX(-100%); }
    100% { transform: translateX(100%); }
  }

  :global(body) {
    overflow-x: hidden;
  }
</style>
