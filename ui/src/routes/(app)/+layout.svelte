<script lang="ts">
  import { onMount } from 'svelte';
  import { browser } from '$app/environment';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import { currentUser, selectedSources } from '$lib/stores';
  import type { Source } from '$lib/types';
  import {
    LayoutList,
    BarChart3,
    Settings,
    LogOut,
    Database,
    User,
    ChevronDown
  } from 'lucide-svelte';

  let { children } = $props();

  let sources = $state<Source[]>([]);
  let sourcesValue = $state<string[]>([]);
  let initialized = $state(false);

  // Sync store → rune for template reactivity
  $effect(() => {
    const unsub = selectedSources.subscribe(v => { sourcesValue = v; });
    return unsub;
  });

  // Persist to sessionStorage (only after initialization to avoid overwriting restored state)
  $effect(() => {
    if (!initialized || !browser) return;
    sessionStorage.setItem('evlog_selected_sources', JSON.stringify(sourcesValue));
  });

  onMount(async () => {
    if (browser) {
      const email = localStorage.getItem('email');
      const role = localStorage.getItem('role');
      if (email && role) {
        currentUser.set({ email, role });
      }

      try {
        const fetchedSources = await api.get<Source[]>('/api/sources');
        sources = fetchedSources;

        // Restore selection from sessionStorage
        const saved = sessionStorage.getItem('evlog_selected_sources');
        if (saved) {
          try {
            const parsed = JSON.parse(saved) as string[];
            if (Array.isArray(parsed)) {
              selectedSources.set(parsed);
            }
          } catch { /* ignore */ }
        }
        // Default: [] = all sources (no explicit filter)
      } catch (e) {
        // Fallback or silent error handled by UI state
      }

      initialized = true;
    }
  });

  function handleLogout() {
    localStorage.removeItem('token');
    localStorage.removeItem('role');
    localStorage.removeItem('email');
    currentUser.set(null);
    goto('/login');
  }

  function selectAll() {
    selectedSources.set([]);
  }

  function selectSource(name: string, event: MouseEvent) {
    if (event.ctrlKey || event.metaKey) {
      // Multi-select: toggle source
      if (sourcesValue.includes(name)) {
        selectedSources.set(sourcesValue.filter(s => s !== name));
      } else {
        selectedSources.set([...sourcesValue, name]);
      }
    } else {
      selectedSources.set([name]);
    }
  }

  const navLinks = [
    { href: '/logs', label: 'Logs', icon: LayoutList },
    { href: '/analytics', label: 'Analytics', icon: BarChart3 },
    { href: '/settings', label: 'Settings', icon: Settings, adminOnly: true },
  ];
</script>

<div class="flex h-screen bg-background text-white overflow-hidden relative">
  <!-- Sidebar -->
  <aside class="w-[240px] flex-shrink-0 glass border-r border-white/5 flex flex-col z-20">
    <div class="p-6">
      <h1 class="text-xl font-bold tracking-tighter">
        evlog<span class="text-brand-primary">.</span>studio
      </h1>
    </div>

    <!-- Sources selector -->
    <div class="px-4 mb-4">
      <p class="text-[10px] font-mono uppercase tracking-widest text-text-muted mb-1 px-3">Sources</p>
      <div class="sources-list {sources.length > 8 ? 'max-h-[280px] overflow-y-auto' : ''}">
        <!-- All sources -->
        <button
          onclick={selectAll}
          class="w-full text-left px-3 py-2 text-[12px] font-mono transition-all duration-150 border-l-2
            {sourcesValue.length === 0
              ? 'border-brand-primary bg-surface-elevated text-text-primary'
              : 'border-transparent text-text-secondary hover:bg-surface-elevated hover:text-text-primary'}"
        >
          All sources
        </button>
        <!-- Individual sources -->
        {#each sources as source}
          <button
            onclick={(e) => selectSource(source.name, e)}
            class="w-full text-left flex items-center gap-2 px-3 py-2 text-[12px] font-mono transition-all duration-150 border-l-2
              {sourcesValue.includes(source.name)
                ? 'border-brand-primary bg-surface-elevated text-text-primary'
                : 'border-transparent text-text-secondary hover:bg-surface-elevated hover:text-text-primary'}"
          >
            <span class="w-1.5 h-1.5 bg-brand-success flex-shrink-0"></span>
            <span class="truncate">{source.name}</span>
          </button>
        {/each}
      </div>
    </div>

    <nav class="flex-1 px-3 space-y-1">
      {#each navLinks as link}
        {#if !link.adminOnly || $currentUser?.role === 'admin'}
          {@const Icon = link.icon}
          <a
            href={link.href}
            class="flex items-center gap-3 px-3 py-2.5 rounded-none text-sm transition-all duration-200 {$page.url.pathname.startsWith(link.href) ? 'bg-brand-primary/10 text-brand-primary font-bold border border-brand-primary/20' : 'text-text-secondary hover:bg-white/5 hover:text-white border border-transparent'}"
          >
            <Icon size={18} strokeWidth={2.5} class="opacity-80" />
            {link.label}
          </a>
        {/if}
      {/each}
    </nav>

    <div class="p-4 border-t border-white/5 glass mt-auto">
      <div class="flex flex-col gap-3">
        <div class="flex items-center gap-3 px-2 py-1">
          <div class="w-9 h-9 rounded-none bg-brand-primary/20 border border-brand-primary/30 flex items-center justify-center text-xs font-bold uppercase text-brand-primary shadow-lg shadow-brand-primary/10">
            {$currentUser?.email?.[0] || 'U'}
          </div>
          <div class="flex-1 min-w-0">
            <p class="text-xs font-bold truncate">{$currentUser?.email}</p>
            <p class="text-[9px] text-text-muted uppercase tracking-widest leading-none mt-1">{$currentUser?.role}</p>
          </div>
        </div>
        <button
          onclick={handleLogout}
          class="w-full px-3 py-2 rounded-none text-xs font-bold text-brand-danger bg-brand-danger/5 border border-brand-danger/10 hover:bg-brand-danger/10 transition-all active:scale-[0.98] flex items-center justify-center gap-2"
        >
          <LogOut size={14} />
          Logout
        </button>
      </div>
    </div>
  </aside>

  <!-- Main Content Area with Terminal Motif -->
  <main class="flex-1 flex flex-col min-w-0 overflow-hidden relative z-10 p-4">
    <!-- Background Glow -->
    <div class="absolute -top-[10%] -right-[10%] w-[40%] h-[40%] bg-brand-primary/5 blur-[120px] rounded-full pointer-events-none"></div>

    <div class="flex-1 flex flex-col glass rounded-none border border-white/5 overflow-hidden shadow-2xl">
      <div class="flex-1 flex flex-col overflow-hidden relative">
        {@render children()}
      </div>
    </div>
  </main>
</div>

<style>
  .sources-list::-webkit-scrollbar { width: 4px; }
  .sources-list::-webkit-scrollbar-track { background: transparent; }
  .sources-list::-webkit-scrollbar-thumb { background: var(--color-border-dim); }
  .sources-list::-webkit-scrollbar-thumb:hover { background: var(--color-text-muted); }

  button { border-radius: 0 !important; }
</style>
