<script lang="ts">
  import type { WidgetConfig, SchemaField, Source } from '$lib/types';
  import { api } from '$lib/api';
  import { onMount } from 'svelte';
  import CustomSelect from '$lib/components/common/CustomSelect.svelte';
  import SourceMultiSelect from '$lib/components/common/SourceMultiSelect.svelte';

  interface Props {
    config: WidgetConfig;
    onchange?: (cfg: WidgetConfig) => void;
  }

  let { config = $bindable(), onchange }: Props = $props();

  // ─── Remote data ─────────────────────────────────────────────────────────
  let allSources   = $state<Source[]>([]);
  let schemaFields = $state<SchemaField[]>([]);

  onMount(async () => {
    try {
      const [srcRes, schemaRes] = await Promise.all([
        api.get<Source[]>('/api/sources'),
        api.get<{ fields: SchemaField[] }>('/api/schema'),
      ]);
      allSources   = srcRes ?? [];
      schemaFields = schemaRes.fields ?? [];
    } catch { /* non-fatal */ }
  });

  // ─── Derived field lists ──────────────────────────────────────────────────
  const DIRECT_FIELDS = [
    'timestamp', 'service', 'source', 'level', 'method',
    'path', 'status', 'environment', 'duration', 'request_id',
  ];

  // Filter schema fields by selected sources (if any)
  let filteredSchemaFields = $derived.by(() => {
    const sel = config.sources ?? [];
    if (sel.length === 0) return schemaFields;
    return schemaFields.filter(f => sel.includes(f.source));
  });

  // Unique field paths from filtered schema (deduplicated)
  let schemaFieldPaths = $derived(
    [...new Set(filteredSchemaFields.map(f => f.field_path))]
      .filter(fp => !DIRECT_FIELDS.includes(fp))
  );

  let allFields = $derived([...DIRECT_FIELDS, ...schemaFieldPaths]);

  // ─── Option helpers for CustomSelect ─────────────────────────────────────
  const METRIC_OPTIONS = [
    { id: 'count',        label: 'Count — total events' },
    { id: 'count_errors', label: 'Count errors' },
    { id: 'error_rate',   label: 'Error rate (%)' },
    { id: 'avg',          label: 'Average of field' },
    { id: 'p50',          label: 'P50 of field' },
    { id: 'p95',          label: 'P95 of field' },
    { id: 'p99',          label: 'P99 of field' },
    { id: 'sum',          label: 'Sum of field' },
  ];

  const OP_OPTIONS = [
    { id: 'eq',       label: '= equals' },
    { id: 'neq',      label: '≠ not equals' },
    { id: 'gt',       label: '> greater than' },
    { id: 'lt',       label: '< less than' },
    { id: 'contains', label: '~ contains' },
  ];

  const INTERVAL_OPTIONS = [
    { id: '1m',  label: '1 min' },
    { id: '5m',  label: '5 min' },
    { id: '30m', label: '30 min' },
    { id: '1h',  label: '1 hour' },
    { id: '1d',  label: '1 day' },
  ];

  const PERIOD_OPTIONS = [
    { id: '1h',   label: 'Last hour' },
    { id: '24h',  label: 'Last 24h' },
    { id: '7d',   label: 'Last 7 days' },
    { id: '30d',  label: 'Last 30 days' },
    { id: 'custom', label: 'Custom' },
  ];

  let selectedPeriod = $state('24h');
  let groupEnabled   = $state((config.group_by ?? null) !== null);

  const NEEDS_FIELD = ['avg', 'p50', 'p95', 'p99', 'sum'];

  function fieldOptions(fields: string[]) {
    return fields.map(f => ({ id: f, label: f }));
  }

  function emit() { onchange?.(config); }

  // ─── Setters ──────────────────────────────────────────────────────────────
  function setSources(sel: string[]) {
    config = { ...config, sources: sel };
    emit();
  }

  function setMetric(m: string) {
    config = {
      ...config,
      metric: m,
      field: NEEDS_FIELD.includes(m) ? (config.field ?? '') : null,
    };
    emit();
  }

  function setField(f: string) {
    config = { ...config, field: f };
    emit();
  }

  function toggleGroup(enabled: boolean) {
    groupEnabled = enabled;
    config = {
      ...config,
      group_by: enabled ? { field: 'timestamp', interval: '1h' } : null,
    };
    emit();
  }

  function setGroupField(f: string) {
    config = {
      ...config,
      group_by: { field: f, interval: f === 'timestamp' ? (config.group_by?.interval ?? '1h') : null },
    };
    emit();
  }

  function setGroupInterval(i: string) {
    config = {
      ...config,
      group_by: config.group_by ? { ...config.group_by, interval: i } : null,
    };
    emit();
  }

  function addFilter() {
    config = {
      ...config,
      filters: [...(config.filters ?? []), { field: 'level', op: 'eq', value: '' }],
    };
    emit();
  }

  function removeFilter(idx: number) {
    config = { ...config, filters: (config.filters ?? []).filter((_, i) => i !== idx) };
    emit();
  }

  function updateFilter(idx: number, key: 'field' | 'op' | 'value', val: string) {
    const filters = (config.filters ?? []).map((f, i) => i === idx ? { ...f, [key]: val } : f);
    config = { ...config, filters };
    emit();
  }

  function nowIso() { return new Date().toISOString(); }
  function relIso(offset: number, unit: 'hour' | 'day') {
    const d = new Date();
    if (unit === 'hour') d.setHours(d.getHours() + offset);
    else d.setDate(d.getDate() + offset);
    return d.toISOString();
  }

  function setPeriod(id: string) {
    selectedPeriod = id;
    const fromTo: Record<string, { from: string; to: string } | null> = {
      '1h':   { from: relIso(-1,  'hour'), to: nowIso() },
      '24h':  { from: relIso(-24, 'hour'), to: nowIso() },
      '7d':   { from: relIso(-7,  'day'),  to: nowIso() },
      '30d':  { from: relIso(-30, 'day'),  to: nowIso() },
      'custom': null,
    };
    const range = fromTo[id];
    if (range) {
      config = { ...config, from: range.from, to: range.to };
      emit();
    }
  }

  function setCustomFrom(v: string) {
    config = { ...config, from: v ? new Date(v).toISOString() : null };
    emit();
  }

  function setCustomTo(v: string) {
    config = { ...config, to: v ? new Date(v).toISOString() : null };
    emit();
  }
</script>

<div class="flex flex-col gap-3 text-[12px]">

  <!-- BLOQUE 0: Sources -->
  <fieldset class="border border-border-dim p-3">
    <legend class="text-[10px] font-mono text-text-muted uppercase tracking-widest px-1">Sources</legend>
    <SourceMultiSelect
      sources={allSources}
      value={config.sources ?? []}
      compact={true}
      onSelect={setSources}
    />
    {#if (config.sources ?? []).length > 0}
      <p class="text-[10px] font-mono text-text-muted mt-1.5">
        Fields filtered to {(config.sources ?? []).length} source{(config.sources ?? []).length !== 1 ? 's' : ''}
      </p>
    {/if}
  </fieldset>

  <!-- BLOQUE 1: Métrica -->
  <fieldset class="border border-border-dim p-3">
    <legend class="text-[10px] font-mono text-text-muted uppercase tracking-widest px-1">Metric</legend>
    <div class="flex gap-2 flex-wrap">
      <div class="flex-1 min-w-[160px]">
        <CustomSelect
          options={METRIC_OPTIONS}
          value={config.metric}
          compact={true}
          onSelect={setMetric}
        />
      </div>
      {#if NEEDS_FIELD.includes(config.metric)}
        <div class="flex-1 min-w-[140px]">
          <CustomSelect
            options={fieldOptions(allFields)}
            value={config.field ?? null}
            placeholder="Select field"
            compact={true}
            onSelect={setField}
          />
        </div>
      {/if}
    </div>
  </fieldset>

  <!-- BLOQUE 2: Group By -->
  <fieldset class="border border-border-dim p-3">
    <legend class="text-[10px] font-mono text-text-muted uppercase tracking-widest px-1">Group By</legend>
    <label class="flex items-center gap-2 cursor-pointer mb-2">
      <input
        type="checkbox"
        checked={groupEnabled}
        onchange={e => toggleGroup((e.target as HTMLInputElement).checked)}
        class="accent-brand-primary"
      />
      <span class="text-text-secondary text-[12px]">Group results</span>
    </label>
    {#if groupEnabled}
      <div class="flex gap-2 flex-wrap">
        <div class="flex-1 min-w-[140px]">
          <CustomSelect
            options={fieldOptions(allFields)}
            value={config.group_by?.field ?? 'timestamp'}
            compact={true}
            onSelect={setGroupField}
          />
        </div>
        {#if config.group_by?.field === 'timestamp'}
          <div class="w-28">
            <CustomSelect
              options={INTERVAL_OPTIONS}
              value={config.group_by?.interval ?? '1h'}
              compact={true}
              onSelect={setGroupInterval}
            />
          </div>
        {/if}
      </div>
    {/if}
  </fieldset>

  <!-- BLOQUE 3: Filtros -->
  <fieldset class="border border-border-dim p-3">
    <legend class="text-[10px] font-mono text-text-muted uppercase tracking-widest px-1">Filters</legend>
    <div class="flex flex-col gap-2">
      {#each (config.filters ?? []) as filter, idx}
        <div class="flex items-center gap-1.5">
          <div class="flex-1 min-w-0">
            <CustomSelect
              options={fieldOptions(allFields)}
              value={filter.field}
              compact={true}
              onSelect={v => updateFilter(idx, 'field', v)}
            />
          </div>
          <div class="w-32 flex-shrink-0">
            <CustomSelect
              options={OP_OPTIONS}
              value={filter.op}
              compact={true}
              onSelect={v => updateFilter(idx, 'op', v)}
            />
          </div>
          <input
            type="text"
            class="flex-1 min-w-0 bg-surface border border-border-dim text-text-primary font-mono text-[11px]
                   px-2 h-[30px] focus:outline-none focus:border-brand-primary transition-colors"
            placeholder="value"
            value={filter.value}
            oninput={e => updateFilter(idx, 'value', (e.target as HTMLInputElement).value)}
          />
          <button
            onclick={() => removeFilter(idx)}
            class="flex-shrink-0 w-6 h-[30px] flex items-center justify-center text-text-muted
                   hover:text-brand-danger transition-colors border border-transparent
                   hover:border-brand-danger/30"
            aria-label="Remove filter"
          >×</button>
        </div>
      {/each}
    </div>
    <button
      onclick={addFilter}
      class="mt-2 text-[11px] font-mono text-brand-primary hover:underline"
    >
      + Add filter
    </button>
  </fieldset>

  <!-- BLOQUE 4: Período -->
  <fieldset class="border border-border-dim p-3">
    <legend class="text-[10px] font-mono text-text-muted uppercase tracking-widest px-1">Period</legend>
    <div class="flex flex-wrap gap-1 mb-2">
      {#each PERIOD_OPTIONS as opt}
        <button
          onclick={() => setPeriod(opt.id)}
          class="text-[11px] font-mono px-2.5 py-1 border transition-colors
            {selectedPeriod === opt.id
              ? 'border-brand-primary text-brand-primary bg-brand-primary/10'
              : 'border-border-dim text-text-muted hover:text-text-primary'}"
        >
          {opt.label}
        </button>
      {/each}
    </div>
    {#if selectedPeriod === 'custom'}
      <div class="flex gap-2 mt-1">
        <div class="flex flex-col gap-1 flex-1">
          <label class="text-[10px] font-mono text-text-muted uppercase tracking-wider">From</label>
          <input
            type="datetime-local"
            class="bg-surface border border-border-dim text-text-primary font-mono text-[11px]
                   px-2 py-1 focus:outline-none focus:border-brand-primary transition-colors"
            onchange={e => setCustomFrom((e.target as HTMLInputElement).value)}
          />
        </div>
        <div class="flex flex-col gap-1 flex-1">
          <label class="text-[10px] font-mono text-text-muted uppercase tracking-wider">To</label>
          <input
            type="datetime-local"
            class="bg-surface border border-border-dim text-text-primary font-mono text-[11px]
                   px-2 py-1 focus:outline-none focus:border-brand-primary transition-colors"
            onchange={e => setCustomTo((e.target as HTMLInputElement).value)}
          />
        </div>
      </div>
    {/if}
  </fieldset>
</div>

<style>
  input[type="text"],
  input[type="datetime-local"] { border-radius: 0 !important; }
  button { border-radius: 0 !important; }
  fieldset { border-radius: 0 !important; }
</style>
