<script lang="ts">
  import { Check, ChevronDown } from 'lucide-svelte';
  import type { Source } from '$lib/types';

  interface Props {
    sources: Source[];
    value: string[];
    label?: string;
    compact?: boolean;
    onSelect: (s: string[]) => void;
  }

  let { sources, value, label, compact = false, onSelect }: Props = $props();

  let isOpen = $state(false);
  let wrapRef: HTMLDivElement | null = $state(null);
  let buttonRef: HTMLButtonElement | null = $state(null);
  let dropdownStyles = $state({ top: '0px', left: '0px', width: '0px' });

  function updateDropdownPosition() {
    if (!buttonRef) return;
    const rect = buttonRef.getBoundingClientRect();
    dropdownStyles = {
      top: `${rect.bottom + window.scrollY + 1}px`,
      left: `${rect.left + window.scrollX}px`,
      width: `${Math.max(rect.width, 180)}px`,
    };
  }

  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    updateDropdownPosition();
    return {
      destroy() {
        if (node.parentNode) node.parentNode.removeChild(node);
      }
    };
  }

  function toggle(e: MouseEvent) {
    e.stopPropagation();
    if (!isOpen) updateDropdownPosition();
    isOpen = !isOpen;
  }

  function handleClickOutside(e: MouseEvent) {
    if (wrapRef && !wrapRef.contains(e.target as Node)) {
      isOpen = false;
    }
  }

  $effect(() => {
    if (!isOpen) return;
    window.addEventListener('click', handleClickOutside);
    window.addEventListener('scroll', updateDropdownPosition, true);
    window.addEventListener('resize', updateDropdownPosition);
    return () => {
      window.removeEventListener('click', handleClickOutside);
      window.removeEventListener('scroll', updateDropdownPosition, true);
      window.removeEventListener('resize', updateDropdownPosition);
    };
  });

  function selectAll() {
    onSelect([]);
  }

  function toggleSource(name: string) {
    if (value.includes(name)) {
      onSelect(value.filter(s => s !== name));
    } else {
      onSelect([...value, name]);
    }
  }

  let displayLabel = $derived(
    value.length === 0 ? 'All Sources' :
    value.length === 1 ? value[0] :
    `${value.length} Sources`
  );
</script>

<div class="space-y-1.5 w-full relative" bind:this={wrapRef}>
  {#if label}
    <span class="text-[10px] font-bold uppercase tracking-widest text-text-muted block">{label}</span>
  {/if}

  <div class="relative">
    <button
      type="button"
      bind:this={buttonRef}
      onclick={toggle}
      class="w-full bg-surface-elevated border border-border-dim px-3
        {compact ? 'h-[30px] text-[10px]' : 'min-h-[40px] py-2 text-[11px]'}
        font-mono font-bold uppercase tracking-widest flex items-center justify-between hover:bg-surface transition-colors gap-2"
    >
      <span class="flex items-center gap-2 min-w-0">
        {#if value.length > 0}
          <span class="w-1.5 h-1.5 bg-brand-success flex-shrink-0"></span>
        {/if}
        <span class="truncate {value.length > 0 ? 'text-text-primary' : 'text-text-muted'}">{displayLabel}</span>
      </span>
      <ChevronDown size={compact ? 12 : 14} class="flex-shrink-0 transition-transform duration-200 {isOpen ? 'rotate-180' : ''}" />
    </button>

    {#if isOpen}
      <div
        use:portal
        class="absolute z-[9999] bg-surface border border-border-dim shadow-2xl overflow-hidden"
        style="top: {dropdownStyles.top}; left: {dropdownStyles.left}; width: {dropdownStyles.width};"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="max-h-[240px] overflow-y-auto custom-scrollbar">
          <button
            type="button"
            onclick={selectAll}
            class="w-full px-4 py-3 text-left text-[11px] font-mono uppercase tracking-widest transition-all flex items-center justify-between
              {value.length === 0 ? 'bg-brand-primary/10 text-brand-primary' : 'text-text-secondary hover:bg-surface-elevated hover:text-text-primary'}"
          >
            <span>All Sources</span>
            {#if value.length === 0}
              <Check size={12} />
            {/if}
          </button>
          {#each sources as source}
            <button
              type="button"
              onclick={() => toggleSource(source.name)}
              class="w-full px-4 py-3 text-left text-[11px] font-mono uppercase tracking-widest transition-all flex items-center justify-between gap-2
                {value.includes(source.name) ? 'bg-brand-primary/10 text-brand-primary' : 'text-text-secondary hover:bg-surface-elevated hover:text-text-primary'}"
            >
              <span class="flex items-center gap-2 min-w-0">
                <span class="w-1.5 h-1.5 bg-brand-success flex-shrink-0"></span>
                <span class="truncate">{source.name}</span>
              </span>
              {#if value.includes(source.name)}
                <Check size={12} class="flex-shrink-0" />
              {/if}
            </button>
          {/each}
          {#if sources.length === 0}
            <div class="px-4 py-6 text-[10px] text-text-muted uppercase text-center font-mono">
              [ NO SOURCES ]
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  div, button { border-radius: 0 !important; }
  .custom-scrollbar::-webkit-scrollbar { width: 4px; }
  .custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
  .custom-scrollbar::-webkit-scrollbar-thumb { background: var(--color-border-dim); }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover { background: var(--color-text-muted); }
</style>
