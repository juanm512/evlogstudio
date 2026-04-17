<script lang="ts">
  import { X } from 'lucide-svelte';
  import { onMount } from 'svelte';

  interface Props {
    open: boolean;
    title: string;
    onClose: () => void;
    children?: import('svelte').Snippet;
  }

  let { open, title, onClose, children }: Props = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) {
      onClose();
    }
  }

  // Prevent scroll when modal is open
  $effect(() => {
    if (open) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div 
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm"
    onclick={onClose}
  >
    <div 
      class="bg-surface border border-border-dim w-full max-w-[480px] shadow-2xl relative"
      onclick={e => e.stopPropagation()}
    >
      <div class="flex items-center justify-between p-4 border-b border-border-dim bg-surface-elevated">
        <h3 class="text-sm font-bold uppercase tracking-wider text-text-primary font-sans">
          {title}
        </h3>
        <button 
          onclick={onClose}
          class="text-text-muted hover:text-text-primary transition-colors p-1"
          aria-label="Close modal"
        >
          <X size={18} />
        </button>
      </div>
      
      <div class="p-6">
        {@render children?.()}
      </div>
    </div>
  </div>
{/if}

<style>
  div, button {
    border-radius: 0 !important;
  }
</style>
