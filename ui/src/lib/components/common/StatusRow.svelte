<script lang="ts">
  import { Loader2 } from 'lucide-svelte';

  interface Props {
    colspan: number;
    message?: string;
  }

  let { colspan, message = 'Fetching data...' }: Props = $props();
</script>

<tr>
  <td {colspan} class="px-6 py-16 text-center">
    <div class="flex flex-col items-center gap-3">
      <Loader2 class="animate-spin text-brand-primary" size={24} />
      <span class="text-[10px] font-mono text-text-muted uppercase tracking-[2px]">
        {message}
      </span>
    </div>
  </td>
</tr>
