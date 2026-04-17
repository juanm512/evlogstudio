<script lang="ts">
  import type { ComponentType } from 'svelte';
  import type { Icon as IconType } from 'lucide-svelte';

  interface Props {
    title: string;
    subtitle?: string;
    actionIcon?: ComponentType<IconType>;
    actionLabel?: string;
    actionDisabled?: boolean;
    onAction?: () => void;
    children?: import('svelte').Snippet; // For custom filters/elements in the header area
  }

  let { 
    title, 
    subtitle, 
    actionIcon: Icon, 
    actionLabel, 
    actionDisabled = false,
    onAction, 
    children 
  }: Props = $props();
</script>

<div class="flex flex-col md:flex-row justify-between items-start md:items-center gap-4 px-1">
  <div class="flex flex-col gap-1">
    <h2 class="text-[12px] font-bold text-text-secondary uppercase tracking-[3px]">{title}</h2>
    {#if subtitle}
      <p class="text-[10px] text-text-muted font-mono uppercase">{subtitle}</p>
    {/if}
  </div>
  
  <div class="flex items-center gap-3 w-full md:w-auto">
    {#if children}
      {@render children()}
    {/if}

    {#if actionLabel && onAction}
      <button 
        onclick={onAction}
        disabled={actionDisabled}
        class="bg-brand-primary text-white text-[11px] font-bold uppercase tracking-widest px-4 py-2 hover:bg-blue-600 transition-colors flex items-center gap-2 disabled:opacity-50 h-[40px] whitespace-nowrap"
      >
        {#if Icon}
          <Icon size={14} />
        {/if}
        {actionLabel}
      </button>
    {/if}
  </div>
</div>

<style>
  button {
    border-radius: 0 !important;
  }
</style>
