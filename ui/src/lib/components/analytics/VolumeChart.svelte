<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Chart, type ChartConfiguration } from 'chart.js/auto';
  import { format, parseISO } from 'date-fns';
  import type { VolumePoint } from '$lib/types';

  interface Props {
    data: VolumePoint[];
    interval: string;
  }

  let { data, interval }: Props = $props();

  let canvas: HTMLCanvasElement;
  let chart: Chart | null = null;

  function getDateFormat(interval: string) {
    switch (interval) {
      case 'minute': return 'HH:mm';
      case 'hour': return 'MMM dd HH:mm';
      case 'day': return 'MMM dd';
      case 'week': return 'MMM dd';
      default: return 'MMM dd HH:mm';
    }
  }

  function createChart() {
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Computed styles for chart colors
    const textColor = '#A1A1AA'; // var(--color-text-secondary)
    const borderColor = '#27272A'; // var(--color-border-dim)
    const primaryColor = '#2853FF'; // var(--color-brand-primary)
    const surfaceElevated = '#18181B'; // var(--color-surface-elevated)

    const config: ChartConfiguration = {
      type: 'bar',
      data: {
        labels: data.map(p => format(parseISO(p.bucket), getDateFormat(interval))),
        datasets: [{
          data: data.map(p => p.count),
          backgroundColor: primaryColor,
          borderWidth: 0,
          borderRadius: 0,
          barPercentage: 0.9,
          categoryPercentage: 0.9,
        }]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        animation: false,
        plugins: {
          legend: { display: false },
          tooltip: {
            backgroundColor: surfaceElevated,
            titleColor: '#FAFAFA',
            bodyColor: '#A1A1AA',
            borderColor: borderColor,
            borderWidth: 1,
            padding: 8,
            cornerRadius: 0,
            displayColors: false,
            titleFont: { family: 'Geist Sans', size: 12, weight: 'bold' },
            bodyFont: { family: 'Geist Mono', size: 11 },
            callbacks: {
              title: (items) => {
                const index = items[0].dataIndex;
                return format(parseISO(data[index].bucket), 'PPpp').toUpperCase();
              },
              label: (item) => `LOGS: ${item.formattedValue}`
            }
          }
        },
        scales: {
          x: {
            grid: { display: false },
            ticks: {
              color: textColor,
              font: { family: 'Geist Mono', size: 10 },
              maxRotation: 0,
              autoSkip: true,
            }
          },
          y: {
            beginAtZero: true,
            grid: { color: borderColor, drawTicks: false },
            border: { display: false },
            ticks: {
              color: textColor,
              font: { family: 'Geist Mono', size: 10 },
              callback: (value) => value.toLocaleString(),
              padding: 8
            }
          }
        }
      }
    };

    chart = new Chart(ctx, config);
  }

  $effect(() => {
    if (chart && data) {
      chart.data.labels = data.map(p => format(parseISO(p.bucket), getDateFormat(interval)));
      chart.data.datasets[0].data = data.map(p => p.count);
      chart.update('none');
    }
  });

  onMount(() => {
    createChart();
  });

  onDestroy(() => {
    if (chart) {
      chart.destroy();
      chart = null;
    }
  });
</script>

<div class="relative w-full h-[320px] bg-surface border border-border-dim p-6">
  {#if data.length === 0}
    <div class="absolute inset-0 flex items-center justify-center bg-surface/50 backdrop-blur-sm z-10">
      <p class="text-text-muted text-[11px] font-mono uppercase tracking-[2px]">No events found for this range</p>
    </div>
  {/if}
  <canvas bind:this={canvas}></canvas>
</div>

<style>
  canvas {
    width: 100% !important;
    height: 100% !important;
  }
</style>
