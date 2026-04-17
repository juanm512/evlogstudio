<script lang="ts">
  interface Props {
    label: string;
    value: string | number;
    sublabel?: string;
  }

  let { label, value, sublabel }: Props = $props();
</script>

<div class="bg-surface border border-border-dim p-4 flex flex-col gap-1 transition-colors hover:bg-surface-elevated group">
  <span class="text-[11px] font-bold text-text-muted uppercase tracking-wider font-sans">
    {label}
  </span>
  
  <div class="flex items-baseline gap-2">
    <span class="text-2xl font-bold text-text-primary tracking-tight font-sans">
      {value}
    </span>
  </div>

  {#if sublabel}
    <span class="text-[10px] text-text-muted mt-1 font-mono uppercase tracking-tight">
      {sublabel}
    </span>
  {/if}
</div>

<style>
  div {
    border-radius: 0;
  }
</style>
