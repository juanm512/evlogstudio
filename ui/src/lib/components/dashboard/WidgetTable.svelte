<script lang="ts">
  interface DataPoint { group_key: string | null; value: number; }

  interface Props { data: DataPoint[]; }

  let { data }: Props = $props();

  let maxVal = $derived(Math.max(...data.map(d => d.value), 1));
</script>

{#if data.length === 0}
  <div class="flex items-center justify-center h-full min-h-[100px]">
    <p class="text-[11px] font-mono text-text-muted uppercase tracking-widest">No data for this period</p>
  </div>
{:else}
  <div class="overflow-auto">
    <table class="w-full text-[12px] font-mono border-collapse">
      <thead>
        <tr class="border-b border-border-dim">
          <th class="text-left py-1.5 px-2 text-text-muted font-normal uppercase tracking-wider text-[10px]">Group</th>
          <th class="text-right py-1.5 px-2 text-text-muted font-normal uppercase tracking-wider text-[10px]">Value</th>
        </tr>
      </thead>
      <tbody>
        {#each data as row}
          <tr class="border-b border-border-dim/40 hover:bg-surface-elevated transition-colors">
            <td class="py-1.5 px-2 text-text-secondary truncate max-w-[200px]">
              {row.group_key ?? '—'}
            </td>
            <td class="py-1.5 px-2 text-right">
              <div class="flex items-center justify-end gap-2">
                <div class="w-24 h-1.5 bg-surface-elevated overflow-hidden flex-shrink-0">
                  <div
                    class="h-full bg-brand-primary"
                    style="width: {Math.round((row.value / maxVal) * 100)}%"
                  ></div>
                </div>
                <span class="text-text-primary text-[11px] tabular-nums w-16 text-right">
                  {typeof row.value === 'number' ? row.value.toLocaleString() : row.value}
                </span>
              </div>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}
