<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Chart, type ChartConfiguration } from 'chart.js/auto';

  interface DataPoint { group_key: string | null; value: number; }

  interface Props { data: DataPoint[]; }

  let { data }: Props = $props();

  let canvas: HTMLCanvasElement;
  let chart: Chart | null = null;

  const TEXT_COLOR   = '#A1A1AA';
  const BORDER_COLOR = '#27272A';
  const SURFACE_EL   = '#18181B';
  const PRIMARY      = '#3B82F6';

  function makeConfig(pts: DataPoint[]): ChartConfiguration {
    return {
      type: 'line',
      data: {
        labels: pts.map(p => p.group_key ?? ''),
        datasets: [{
          data: pts.map(p => p.value),
          borderColor: PRIMARY,
          backgroundColor: `${PRIMARY}18`,
          borderWidth: 1.5,
          pointRadius: 0,
          pointHoverRadius: 4,
          fill: true,
          tension: 0.3,
        }],
      },
      options: {
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
            grid: { display: false },
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
    chart.data.labels = data.map(p => p.group_key ?? '');
    chart.data.datasets[0].data = data.map(p => p.value);
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
