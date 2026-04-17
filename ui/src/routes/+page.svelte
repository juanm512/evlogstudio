<script lang="ts">
  import { browser } from '$app/environment';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { api } from '$lib/api';

  onMount(async () => {
    if (!browser) return;
    
    try {
      const health = await api.get<{ setup_required: boolean }>('/health');
      if (health.setup_required) {
        goto('/setup', { replaceState: true });
        return;
      }
      
      const token = localStorage.getItem('token');
      goto(token ? '/logs' : '/login', { replaceState: true });
    } catch (e) {
      // Fallback to login if connection fails
      goto('/login', { replaceState: true });
    }
  });
</script>
