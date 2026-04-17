<script lang="ts">
  import { goto } from '$app/navigation';
  import { AlertCircle, UserPlus } from 'lucide-svelte';
  import { api } from '$lib/api';
  import { currentUser } from '$lib/stores';
  import { onMount } from 'svelte';

  let email = '';
  let password = '';
  let confirmPassword = '';
  let error = '';
  let loading = false;

  onMount(async () => {
    try {
      const health = await api.get<{ setup_required: boolean }>('/health');
      if (!health.setup_required) {
        goto('/login');
      }
    } catch (e) {
      goto('/login');
    }
  });

  async function handleSetup(e: Event) {
    e.preventDefault();
    error = '';
    
    if (password !== confirmPassword) {
      error = 'Passwords do not match';
      return;
    }
    
    if (password.length < 8) {
      error = 'Password must be at least 8 characters';
      return;
    }

    loading = true;
    try {
      // 1. Create admin
      await api.post('/api/setup', { email, password });
      
      // 2. Auto login
      const res = await api.post<{ token: string; user: { email: string; role: string } }>('/auth/login', {
        email,
        password
      });
      
      localStorage.setItem('token', res.token);
      localStorage.setItem('role', res.user.role);
      localStorage.setItem('email', res.user.email);
      
      currentUser.set(res.user);
      
      goto('/logs');
    } catch (e: any) {
      error = e.message || 'Failed to initialize setup';
    } finally {
      loading = false;
    }
  }
</script>

<svelte:head>
  <title>Setup | evlogstudio</title>
</svelte:head>

<div class="min-h-screen flex items-center justify-center bg-background p-4 relative overflow-hidden">
  <!-- Subtle brand background glow -->
  <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[500px] h-[500px] bg-brand-primary/10 blur-[120px] rounded-full pointer-events-none"></div>

  <div class="w-full max-w-md glass rounded-none shadow-2xl overflow-hidden border border-white/10 relative z-10">
    <div class="p-8">
      <div class="flex flex-col items-center mb-8">
        <h1 class="text-3xl font-bold tracking-tighter text-white">
          evlog<span class="text-brand-primary">.</span>studio
        </h1>
        <h2 class="text-white text-xl font-bold mt-4">Welcome to evlogstudio</h2>
        <p class="text-text-secondary text-sm mt-1 text-center">Create your admin account to get started</p>
      </div>

      <form onsubmit={handleSetup} class="space-y-4">
        <div>
          <label for="email" class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">Admin Email</label>
          <input
            id="email"
            type="email"
            bind:value={email}
            required
            class="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-none text-white focus:outline-none focus:ring-2 focus:ring-brand-primary/50 focus:border-brand-primary transition-all placeholder:text-text-muted/50"
            placeholder="admin@example.com"
          />
        </div>

        <div>
          <label for="password" class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">Password (Min 8 characters)</label>
          <input
            id="password"
            type="password"
            bind:value={password}
            required
            class="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-none text-white focus:outline-none focus:ring-2 focus:ring-brand-primary/50 focus:border-brand-primary transition-all placeholder:text-text-muted/50"
            placeholder="••••••••"
          />
        </div>

        <div>
          <label for="confirm" class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">Confirm Password</label>
          <input
            id="confirm"
            type="password"
            bind:value={confirmPassword}
            required
            class="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-none text-white focus:outline-none focus:ring-2 focus:ring-brand-primary/50 focus:border-brand-primary transition-all placeholder:text-text-muted/50"
            placeholder="••••••••"
          />
        </div>

        {#if error}
          <div class="p-3 bg-brand-danger/10 border border-brand-danger/30 rounded-none text-brand-danger text-sm flex items-center gap-2">
            <AlertCircle size={16} /> {error}
          </div>
        {/if}

        <button
          type="submit"
          disabled={loading}
          class="w-full py-3.5 px-4 bg-brand-primary hover:bg-brand-primary/90 disabled:opacity-50 disabled:cursor-not-allowed text-white font-bold rounded-none shadow-lg shadow-brand-primary/20 transition-all active:scale-[0.98] flex items-center justify-center gap-2"
        >
          {#if loading}
            <div class="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></div>
          {:else}
            <UserPlus size={18} />
          {/if}
          {loading ? 'Initializing evlog...' : 'Create admin account'}
        </button>
      </form>
    </div>
    
    <div class="px-8 py-5 bg-white/5 border-t border-white/5 text-center">
      <p class="text-[10px] text-text-muted uppercase tracking-widest">
        &copy; 2026 evlogstudio &bull; Initial Setup
      </p>
    </div>
  </div>
</div>
