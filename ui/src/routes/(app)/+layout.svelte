<script lang="ts">
  import { onMount } from 'svelte';
  import { browser } from '$app/environment';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { currentUser } from '$lib/stores';
  import {
    LayoutList,
    BarChart3,
    LayoutDashboard,
    Settings,
    LogOut,
    ChevronLeft,
    ChevronRight,
    Menu,
  } from 'lucide-svelte';

  let { children } = $props();

  let collapsed = $state(false);
  let mobileOpen = $state(false);

  onMount(() => {
    if (browser) {
      const email = localStorage.getItem('email');
      const role = localStorage.getItem('role');
      if (email && role) {
        currentUser.set({ email, role });
      }
      const savedCollapsed = localStorage.getItem('evlog_sidebar_collapsed');
      if (savedCollapsed === 'true') collapsed = true;
    }
  });

  $effect(() => {
    if (!browser) return;
    localStorage.setItem('evlog_sidebar_collapsed', String(collapsed));
  });

  function handleLogout() {
    localStorage.removeItem('token');
    localStorage.removeItem('role');
    localStorage.removeItem('email');
    currentUser.set(null);
    goto('/login');
  }

  const navLinks = [
    { href: '/logs',        label: 'Logs',        icon: LayoutList },
    { href: '/analytics',   label: 'Analytics',   icon: BarChart3 },
    { href: '/dashboards',  label: 'Dashboards',  icon: LayoutDashboard },
    { href: '/settings',    label: 'Settings',    icon: Settings, adminOnly: true },
  ];
</script>

<div class="flex h-screen bg-background text-white overflow-hidden relative">

  <!-- Mobile overlay -->
  {#if mobileOpen}
    <div
      class="fixed inset-0 bg-black/50 z-40 lg:hidden"
      onclick={() => (mobileOpen = false)}
      role="presentation"
    ></div>
  {/if}

  <!-- Sidebar -->
  <aside
    class="
      flex-shrink-0 glass border-r border-white/5 flex flex-col z-50
      fixed inset-y-0 left-0 lg:relative lg:inset-auto lg:translate-x-0
      transition-[width,transform] duration-200
      {collapsed ? 'w-[56px]' : 'w-[240px]'}
      {mobileOpen ? 'translate-x-0' : '-translate-x-full'}
    "
  >
    <!-- Header -->
    {#if collapsed}
      <div class="flex justify-center p-3 border-b border-white/5">
        <button
          onclick={() => (collapsed = false)}
          class="sidebar-icon-btn"
          title="Expand sidebar"
          aria-label="Expand sidebar"
        >
          <ChevronRight size={18} />
        </button>
      </div>
    {:else}
      <div class="px-5 py-4 flex items-center justify-between border-b border-white/5">
        <h1 class="text-xl font-bold tracking-tighter">
          evlog<span class="text-brand-primary">.</span>studio
        </h1>
        <button
          onclick={() => (collapsed = true)}
          class="sidebar-icon-btn"
          title="Collapse sidebar"
          aria-label="Collapse sidebar"
        >
          <ChevronLeft size={18} />
        </button>
      </div>
    {/if}

    <!-- Nav -->
    <nav class="flex-1 px-2 py-3 space-y-1 overflow-y-auto">
      {#each navLinks as link}
        {#if !link.adminOnly || $currentUser?.role === 'admin'}
          {@const Icon = link.icon}
          <a
            href={link.href}
            title={collapsed ? link.label : undefined}
            onclick={() => (mobileOpen = false)}
            class="flex items-center gap-3 py-2.5 text-sm transition-all duration-200 border
              {collapsed ? 'justify-center px-2' : 'px-3'}
              {$page.url.pathname.startsWith(link.href)
                ? 'bg-brand-primary/10 text-brand-primary font-bold border-brand-primary/20'
                : 'text-text-secondary hover:bg-white/5 hover:text-white border-transparent'}"
          >
            <Icon size={18} strokeWidth={2.5} class="opacity-80 flex-shrink-0" />
            {#if !collapsed}
              <span>{link.label}</span>
            {/if}
          </a>
        {/if}
      {/each}
    </nav>

    <!-- User info -->
    <div class="border-t border-white/5 glass">
      {#if collapsed}
        <div class="flex flex-col items-center gap-2 p-3">
          <div class="w-8 h-8 bg-brand-primary/20 border border-brand-primary/30 flex items-center justify-center text-xs font-bold uppercase text-brand-primary">
            {$currentUser?.email?.[0] || 'U'}
          </div>
          <button
            onclick={handleLogout}
            title="Logout"
            class="w-8 h-8 flex items-center justify-center text-brand-danger bg-brand-danger/5 border border-brand-danger/10 hover:bg-brand-danger/10 transition-all"
          >
            <LogOut size={14} />
          </button>
        </div>
      {:else}
        <div class="p-4 flex flex-col gap-3">
          <div class="flex items-center gap-3 px-2 py-1">
            <div class="w-9 h-9 flex-shrink-0 bg-brand-primary/20 border border-brand-primary/30 flex items-center justify-center text-xs font-bold uppercase text-brand-primary shadow-lg shadow-brand-primary/10">
              {$currentUser?.email?.[0] || 'U'}
            </div>
            <div class="flex-1 min-w-0">
              <p class="text-xs font-bold truncate">{$currentUser?.email}</p>
              <p class="text-[9px] text-text-muted uppercase tracking-widest leading-none mt-1">{$currentUser?.role}</p>
            </div>
          </div>
          <button
            onclick={handleLogout}
            class="w-full px-3 py-2 text-xs font-bold text-brand-danger bg-brand-danger/5 border border-brand-danger/10 hover:bg-brand-danger/10 transition-all active:scale-[0.98] flex items-center justify-center gap-2"
          >
            <LogOut size={14} />
            Logout
          </button>
        </div>
      {/if}
    </div>
  </aside>

  <!-- Main Content Area -->
  <main class="flex-1 flex flex-col min-w-0 overflow-hidden relative z-10 p-4">
    <!-- Background Glow -->
    <div class="absolute -top-[10%] -right-[10%] w-[40%] h-[40%] bg-brand-primary/5 blur-[120px] rounded-full pointer-events-none"></div>

    <div class="flex-1 flex flex-col glass border border-white/5 overflow-hidden shadow-2xl min-h-0">
      <!-- Mobile topbar -->
      <div class="lg:hidden flex items-center gap-3 px-4 py-3 border-b border-white/5 flex-shrink-0">
        <button
          onclick={() => (mobileOpen = true)}
          class="mobile-menu-btn"
          aria-label="Open menu"
        >
          <Menu size={20} />
        </button>
        <h1 class="text-xl font-bold tracking-tighter">
          evlog<span class="text-brand-primary">.</span>studio
        </h1>
      </div>

      <div class="flex-1 flex flex-col overflow-hidden relative">
        {@render children()}
      </div>
    </div>
  </main>
</div>

<style>
  button { border-radius: 0 !important; }
  a { border-radius: 0 !important; }

  .sidebar-icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: color 0.15s, background 0.15s, border-color 0.15s;
    flex-shrink: 0;
  }
  .sidebar-icon-btn:hover {
    color: var(--color-text-primary);
    background: var(--color-surface-elevated);
    border-color: var(--color-border-dim);
  }
  .sidebar-icon-btn:focus-visible {
    outline: 2px solid var(--color-brand-primary);
    outline-offset: 2px;
  }

  .mobile-menu-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    padding: 0;
    background: transparent;
    border: 1px solid var(--color-border-dim);
    color: var(--color-text-secondary);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
    flex-shrink: 0;
  }
  .mobile-menu-btn:hover {
    background: var(--color-surface-elevated);
    color: var(--color-text-primary);
  }
  .mobile-menu-btn:focus-visible {
    outline: 2px solid var(--color-brand-primary);
    outline-offset: 2px;
  }
</style>
