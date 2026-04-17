<script lang="ts">
  import { ChevronDown, Check } from 'lucide-svelte';
  import { onMount } from 'svelte';

  interface Option {
    id: string;
    label: string;
  }

  interface Props {
    options: Option[];
    value: string | null;
    placeholder?: string;
    label?: string;
    compact?: boolean;
    onSelect: (id: string) => void;
  }

  let { options, value, placeholder = 'Select an option', label, compact = false, onSelect }: Props = $props();

  let isOpen = $state(false);
  let selectRef: HTMLDivElement | null = $state(null);
  let buttonRef: HTMLButtonElement | null = $state(null);
  let dropdownStyles = $state({ top: '0px', left: '0px', width: '0px' });

  function updateDropdownPosition() {
    if (!buttonRef) return;
    const rect = buttonRef.getBoundingClientRect();
    dropdownStyles = {
      top: `${rect.bottom + window.scrollY + 1}px`,
      left: `${rect.left + window.scrollX}px`,
      width: `${rect.width}px`
    };
  }

  // Portal action to move the element to body
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
    if (!isOpen) {
      updateDropdownPosition();
    }
    isOpen = !isOpen;
  }

  function handleSelect(id: string) {
    onSelect(id);
    isOpen = false;
  }

  function handleClickOutside(e: MouseEvent) {
    if (selectRef && !selectRef.contains(e.target as Node)) {
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

  let selectedOption = $derived(options.find(o => o.id === value));
</script>

<div class="space-y-1.5 w-full relative" bind:this={selectRef}>
  {#if label}
    <span class="text-[10px] font-bold uppercase tracking-widest text-text-muted block">{label}</span>
  {/if}
  
  <div class="relative">
    <button
      type="button"
      bind:this={buttonRef}
      onclick={toggle}
      class="w-full bg-surface-elevated border border-border-dim px-3 {compact ? 'py-1 text-[10px]' : 'py-2 text-[11px]'} font-mono font-bold uppercase tracking-widest flex items-center justify-between hover:bg-surface transition-colors {compact ? 'h-[30px]' : 'min-h-[40px]'}"
    >
      <span class={selectedOption ? 'text-text-primary' : 'text-text-muted'}>
        {selectedOption ? selectedOption.label : placeholder}
      </span>
      <ChevronDown size={compact ? 12 : 14} class="transition-transform duration-200 {isOpen ? 'rotate-180' : ''}" />
    </button>

    {#if isOpen}
      <div
        use:portal
        class="absolute z-[9999] bg-surface border border-border-dim shadow-2xl overflow-hidden flex flex-col"
        style="top: {dropdownStyles.top}; left: {dropdownStyles.left}; width: {dropdownStyles.width};"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="max-h-[240px] overflow-y-auto custom-scrollbar">
          {#each options as option}
            <button
              type="button"
              onclick={() => handleSelect(option.id)}
              class="w-full px-4 py-3 text-left text-[11px] font-mono uppercase tracking-widest transition-all flex items-center justify-between group
                {value === option.id ? 'bg-brand-primary/10 text-brand-primary' : 'text-text-secondary hover:bg-surface-elevated hover:text-text-primary'}"
            >
              <span>{option.label}</span>
              {#if value === option.id}
                <Check size={12} />
              {/if}
            </button>
          {/each}
          {#if options.length === 0}
            <div class="px-4 py-6 text-[10px] text-text-muted uppercase text-center font-mono">
              [ NO DATA AVAILABLE ]
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  div, button {
    border-radius: 0 !important;
  }
  
  .custom-scrollbar::-webkit-scrollbar {
    width: 4px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: var(--color-border-dim);
  }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover {
    background: var(--color-text-muted);
  }
</style>
