<script lang="ts">
  interface Props {
    value: number;
    metric: string;
  }

  let { value, metric }: Props = $props();

  const METRIC_LABELS: Record<string, string> = {
    count: 'Total Events',
    count_errors: 'Total Errors',
    error_rate: 'Error Rate %',
    avg: 'Average',
    p50: 'P50',
    p95: 'P95',
    p99: 'P99',
    sum: 'Sum',
  };

  let color = $derived.by(() => {
    if (metric !== 'error_rate') return 'text-text-primary';
    if (value < 1) return 'text-brand-success';
    if (value <= 5) return 'text-brand-warning';
    return 'text-brand-danger';
  });

  function fmt(v: number): string {
    if (metric === 'error_rate') return `${v.toFixed(2)}%`;
    if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(1)}M`;
    if (v >= 1_000) return `${(v / 1_000).toFixed(1)}K`;
    if (Number.isInteger(v)) return v.toLocaleString();
    return v.toFixed(2);
  }
</script>

<div class="flex flex-col items-center justify-center h-full min-h-[120px] gap-2">
  <span class="text-[48px] font-medium leading-none font-mono {color}">
    {fmt(value)}
  </span>
  <span class="text-[11px] font-mono text-text-muted uppercase tracking-widest">
    {METRIC_LABELS[metric] ?? metric}
  </span>
</div>
