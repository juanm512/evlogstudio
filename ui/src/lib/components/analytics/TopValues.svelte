<script lang="ts">
  import { createQuery } from '@tanstack/svelte-query';
  import { api } from '$lib/api';

  interface Props {
    field: string;
    source: string | null;
    from: string;
    to: string;
  }

  let { field, source, from, to }: Props = $props();

  function sanitize(s: string): string {
    return s.replace(/'/g, "''");
  }

  function buildSQL(): string {
    const f = sanitize(field);
    let where = `fields->>'${f}' IS NOT NULL`;
    if (source) where += ` AND source = '${sanitize(source)}'`;
    if (from) where += ` AND timestamp >= '${sanitize(from)}'`;
    if (to) where += ` AND timestamp <= '${sanitize(to)}'`;
    return `SELECT fields->>'${f}' as value, COUNT(*) as count FROM logs WHERE ${where} GROUP BY value ORDER BY count DESC LIMIT 10`;
  }

  interface TopValueRow { value: string | null; count: number; }
  interface JsonQueryResponse { rows: TopValueRow[]; }

  const query = createQuery<JsonQueryResponse>(() => ({
    queryKey: ['analytics', 'top-values', field, source, from, to],
    queryFn: () => api.post<JsonQueryResponse>('/api/query/json', { sql: buildSQL() }),
    enabled: !!field,
  }));

  let maxCount = $derived(Math.max(1, ...(query.data?.rows.map(r => r.count) ?? [1])));
</script>

<div class="space-y-3">
  <div class="flex items-center gap-3 px-1">
    <h3 class="text-[12px] font-bold text-text-secondary uppercase tracking-[3px]">Top Values — <span class="text-brand-primary font-mono">{field}</span></h3>
    {#if query.isFetching}
      <div class="w-2 h-[2px] bg-brand-primary animate-pulse"></div>
    {/if}
  </div>

  {#if query.isLoading}
    <div class="border border-border-dim bg-surface p-6 flex items-center justify-center">
      <span class="text-text-muted text-[11px] font-mono uppercase tracking-[3px]">Loading...</span>
    </div>
  {:else if query.isError}
    <div class="border border-brand-danger/30 bg-brand-danger/5 p-4">
      <p class="text-brand-danger text-[11px] font-mono uppercase">Field not available</p>
    </div>
  {:else if !query.data?.rows.length}
    <div class="border border-border-dim bg-surface p-6">
      <p class="text-text-muted text-[11px] font-mono uppercase tracking-[3px]">No data for this field</p>
    </div>
  {:else}
    <div class="border border-border-dim bg-surface divide-y divide-border-dim">
      {#each query.data.rows as row}
        <div class="flex items-center gap-3 px-4 py-2.5 hover:bg-surface-elevated transition-colors">
          <span
            class="text-[12px] font-mono text-text-primary w-36 truncate flex-shrink-0"
            title={row.value ?? ''}
          >{row.value ?? '(null)'}</span>
          <div class="flex-1 bg-surface-elevated h-1.5 relative overflow-hidden">
            <div
              class="h-full bg-brand-primary/60 absolute left-0 top-0 transition-all duration-300"
              style="width: {(row.count / maxCount) * 100}%"
            ></div>
          </div>
          <span class="text-[12px] font-mono text-text-secondary w-14 text-right flex-shrink-0 tabular-nums">
            {row.count.toLocaleString()}
          </span>
        </div>
      {/each}
    </div>
  {/if}
</div>
