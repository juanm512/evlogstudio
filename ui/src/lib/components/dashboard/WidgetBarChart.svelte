<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Chart, type ChartConfiguration } from 'chart.js/auto';

  interface DataPoint { group_key: string | null; value: number; }

  interface Props {
    data: DataPoint[];
    title?: string;
  }

  let { data, title = '' }: Props = $props();

  let canvas: HTMLCanvasElement;
  let chart: Chart | null = null;

  const TEXT_COLOR   = '#A1A1AA';
  const BORDER_COLOR = '#27272A';
  const SURFACE_EL   = '#18181B';
  const PRIMARY      = '#3B82F6';

  function isTimeSeries(pts: DataPoint[]): boolean {
    if (!pts.length) return false;
    return /^\d{4}-\d{2}-\d{2}/.test(pts[0].group_key ?? '');
  }

  function makeConfig(pts: DataPoint[]): ChartConfiguration {
    const labels = pts.map(p => p.group_key ?? '');
    const values = pts.map(p => p.value);
    const timeSeries = isTimeSeries(pts);

    return {
      type: 'bar',
      data: {
        labels,
        datasets: [{
          data: values,
          backgroundColor: PRIMARY,
          borderWidth: 0,
          borderRadius: 0,
          barPercentage: 0.85,
          categoryPercentage: 0.85,
        }],
      },
      options: {
        indexAxis: timeSeries ? 'x' : 'y',
        responsive: true,
        maintainAspectRatio: false,
        animation: false,
        plugins: {
          legend: { display: false },
          tooltip: {
            backgroundColor: SURFACE_EL,
            titleColor: '#FAFAFA',
            bodyColor: TEXT_COLOR,
            borderColor: BORDER_COLOR,
            borderWidth: 1,
            padding: 8,
            cornerRadius: 0,
            displayColors: false,
            titleFont: { family: 'Geist Sans', size: 12 },
            bodyFont: { family: 'Geist Mono', size: 11 },
          },
        },
        scales: {
          x: {
            grid: { display: timeSeries, color: BORDER_COLOR, drawTicks: false },
            ticks: { color: TEXT_COLOR, font: { family: 'Geist Mono', size: 10 }, maxRotation: 0, autoSkip: true },
            border: { display: false },
          },
          y: {
            beginAtZero: true,
            grid: { color: BORDER_COLOR, drawTicks: false },
            border: { display: false },
            ticks: { color: TEXT_COLOR, font: { family: 'Geist Mono', size: 10 }, padding: 8 },
          },
        },
      },
    };
  }

  onMount(() => {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    chart = new Chart(ctx, makeConfig(data));
  });

  $effect(() => {
    if (!chart) return;
    const pts = data;
    chart.data.labels = pts.map(p => p.group_key ?? '');
    chart.data.datasets[0].data = pts.map(p => p.value);
    chart.update('none');
  });

  onDestroy(() => { chart?.destroy(); chart = null; });
</script>

<div class="relative w-full h-full min-h-[160px]">
  {#if data.length === 0}
    <div class="absolute inset-0 flex items-center justify-center">
      <p class="text-[11px] font-mono text-text-muted uppercase tracking-widest">No data for this period</p>
    </div>
  {/if}
  <canvas bind:this={canvas} class="w-full h-full"></canvas>
</div>
