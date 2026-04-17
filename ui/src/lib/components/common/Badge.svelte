<script lang="ts">
  interface Props {
    variant?: 'success' | 'danger' | 'warning' | 'info' | 'muted';
    children: import('svelte').Snippet;
    class?: string;
  }

  let { variant = 'info', children, class: className = '' }: Props = $props();

  const variants = {
    success: 'badge-success',
    danger: 'bg-brand-danger/10 text-brand-danger border border-brand-danger/20',
    warning: 'bg-brand-warning/10 text-brand-warning border border-brand-warning/20',
    info: 'badge-info',
    muted: 'bg-surface-elevated text-text-muted border border-border-dim'
  };
</script>

<span class="inline-flex items-center px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-widest {variants[variant]} {className}">
  {@render children()}
</span>

<style>
  span {
    border-radius: 0 !important;
  }
</style>
