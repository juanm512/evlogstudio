<script lang="ts">
  import { Loader2 } from 'lucide-svelte';
  import Modal from './Modal.svelte';

  interface Props {
    open: boolean;
    title: string;
    description: string;
    impactLabel?: string;
    impactDescription?: string;
    confirmLabel: string;
    isSubmitting?: boolean;
    variant?: 'danger' | 'warning';
    onConfirm: () => void;
    onCancel: () => void;
  }

  let { 
    open, 
    title, 
    description, 
    impactLabel = 'Warning',
    impactDescription,
    confirmLabel, 
    isSubmitting = false,
    variant = 'danger',
    onConfirm, 
    onCancel 
  }: Props = $props();

  const variantStyles = {
    danger: {
      button: 'bg-brand-danger hover:bg-red-600',
      box: 'bg-brand-danger/5 border-brand-danger/20',
      label: 'text-brand-danger'
    },
    warning: {
      button: 'bg-brand-warning hover:bg-orange-600',
      box: 'bg-brand-warning/5 border-brand-warning/20',
      label: 'text-brand-warning'
    }
  };

  const styles = $derived(variantStyles[variant]);
</script>

<Modal {open} {title} onClose={onCancel}>
  <div class="space-y-6">
    <div class="space-y-3">
      <p class="text-[11px] text-text-secondary leading-relaxed">
        {description}
      </p>
      
      {#if impactDescription}
        <div class="border p-4 {styles.box}">
          <p class="text-[10px] font-bold uppercase tracking-widest mb-1 {styles.label}">
            {impactLabel}
          </p>
          <p class="text-[10px] text-text-secondary leading-relaxed font-sans">
            {impactDescription}
          </p>
        </div>
      {/if}
    </div>

    <div class="flex flex-col gap-3">
      <button 
        onclick={onConfirm}
        disabled={isSubmitting}
        class="w-full text-white text-[11px] font-bold uppercase tracking-[2px] py-3.5 transition-all disabled:opacity-50 flex items-center justify-center gap-2 {styles.button}"
      >
        {#if isSubmitting}
          <Loader2 size={16} class="animate-spin" />
          Processing...
        {:else}
          {confirmLabel}
        {/if}
      </button>
      <button 
        onclick={onCancel}
        class="w-full bg-surface-elevated border border-border-dim text-text-primary text-[11px] font-bold uppercase tracking-[2px] py-3.5 hover:bg-surface transition-all"
      >
        Cancel
      </button>
    </div>
  </div>
</Modal>

<style>
  button, div {
    border-radius: 0 !important;
  }
</style>
