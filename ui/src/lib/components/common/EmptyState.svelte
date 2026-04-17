<script lang="ts">
  import type { ComponentType } from 'svelte';
  import type { Icon as IconType } from 'lucide-svelte';

  interface Props {
    title: string;
    description?: string;
    icon?: ComponentType<IconType>;
  }

  let { title, description, icon: Icon }: Props = $props();
</script>

<div class="flex flex-col items-center justify-center py-16 px-6 text-center">
  {#if Icon}
    <div class="mb-4 text-text-muted opacity-20">
      <Icon size={48} strokeWidth={1} />
    </div>
  {/if}
  <h3 class="text-[10px] font-mono font-bold uppercase tracking-[3px] text-text-muted mb-2">
    {title}
  </h3>
  {#if description}
    <p class="text-[11px] text-text-muted/60 max-w-[280px] leading-relaxed mx-auto italic">
      {description}
    </p>
  {/if}
</div>
