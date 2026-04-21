<script lang="ts">
  import { goto } from '$app/navigation';
  import { AlertCircle, LogIn } from 'lucide-svelte';
  import { api } from '$lib/api';
  import { currentUser } from '$lib/stores';
  import { onMount } from 'svelte';

  onMount(async () => {
    // 1. Check for setup
    try {
      const health = await api.get<{ setup_required: boolean }>('/health');
      if (health.setup_required) {
        goto('/setup');
        return;
      }
    } catch (e) {}

    // 2. Check if already logged in
    const token = localStorage.getItem('token');
    if (token) {
      try {
        const user = await api.get<{ email: string; role: string }>('/auth/verify');
        currentUser.set(user);
        goto('/logs');
      } catch (e) {
        // Token invalid, clear it
        localStorage.removeItem('token');
        localStorage.removeItem('role');
        localStorage.removeItem('email');
        currentUser.set(null);
      }
    }
  });

  let email = '';
  let password = '';
  let error = '';
  let loading = false;

  async function handleLogin() {
    error = '';
    loading = true;
    try {
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
      error = e.message || 'Invalid credentials';
    } finally {
      loading = false;
    }
  }
</script>

<svelte:head>
  <title>Login | evlogstudio</title>
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
        <p class="text-text-secondary text-sm mt-2">Sign in to your account</p>
      </div>

      <form onsubmit={handleLogin} class="space-y-6">
        <div>
          <label for="email" class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">Email address</label>
          <input
            id="email"
            type="email"
            bind:value={email}
            required
            autocomplete="email"
            class="w-full px-4 py-3 bg-white/5 border border-white/10 rounded-none text-white focus:outline-none focus:ring-2 focus:ring-brand-primary/50 focus:border-brand-primary transition-all placeholder:text-text-muted/50"
            placeholder="name@company.com"
          />
        </div>

        <div>
          <label for="password" class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">Password</label>
          <input
            id="password"
            type="password"
            bind:value={password}
            required
            autocomplete="current-password"
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
            <LogIn size={18} />
          {/if}
          {loading ? 'Authenticating...' : 'Sign in'}
        </button>
      </form>
    </div>
    
    <div class="px-8 py-5 bg-white/5 border-t border-white/5 text-center">
      <p class="text-[10px] text-text-muted uppercase tracking-widest">
        &copy; 2026 evlogstudio &bull; Protected Access
      </p>
    </div>
  </div>
</div>
