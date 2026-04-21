<script lang="ts">
  import { onMount } from 'svelte';
  import { SlidersHorizontal, Search, X, ChevronRight, ChevronDown } from 'lucide-svelte';
  import type { SchemaField } from '$lib/types';
  import { browser } from '$app/environment';
  import { 
    type TreeNode, 
    buildTree, 
    filterTree, 
    saveExpansionState, 
    getSavedExpansionState,
    countSelectedLeaves
  } from '$lib/treeUtils';

  interface Props {
    availableFields: SchemaField[];
    activeColumns: string[];
    anchorTop: number;
    anchorRight: number;
    onchange: (cols: string[]) => void;
    onclose: () => void;
  }

  let { availableFields, activeColumns = $bindable(), anchorTop, anchorRight, onchange, onclose }: Props = $props();

  const BASE_COLUMNS = ['timestamp', 'source', 'level', 'message'];
  const STORAGE_KEY = 'evlog_column_tree_state';

  // --- Local State ---
  let tree = $state<TreeNode[]>([]);
  let searchQuery = $state('');
  let panelRef = $state<HTMLElement>();
  let expansionState = $state<Record<string, boolean>>({});

  // --- Reactive Tree ---

  $effect(() => {
    expansionState = getSavedExpansionState(STORAGE_KEY);
    tree = buildTree(availableFields, STORAGE_KEY, { excludeFields: BASE_COLUMNS });
  });

  let filteredTree = $derived(filterTree(tree, searchQuery));

  // --- Interaction ---

  function toggleColumn(path: string) {
    if (BASE_COLUMNS.includes(path)) return;
    const next = activeColumns.includes(path)
      ? activeColumns.filter(c => c !== path)
      : [...activeColumns, path];
    activeColumns = next;
    onchange(next);
  }

  function toggleExpand(node: TreeNode, path: string) {
    node.expanded = !node.expanded;
    saveExpansionState(STORAGE_KEY, path, node.expanded);
  }

  // Click outside logic
  $effect(() => {
    const handler = (e: MouseEvent) => {
      if (panelRef && !panelRef.contains(e.target as Node)) {
        onclose();
      }
    };
    const timer = setTimeout(() => {
      window.addEventListener('click', handler);
    }, 10);
    return () => {
      clearTimeout(timer);
      window.removeEventListener('click', handler);
    };
  });

  // Calculate position (up or down)
  let openingDirection = $state<'down' | 'up'>('down');
  $effect(() => {
    if (panelRef) {
      const rect = panelRef.getBoundingClientRect();
      const spaceBelow = window.innerHeight - anchorTop;
      if (spaceBelow < rect.height && anchorTop > rect.height) {
        openingDirection = 'up';
      } else {
        openingDirection = 'down';
      }
    }
  });

  function highlightText(text: string, query: string) {
    if (!query) return text;
    const parts = text.split(new RegExp(`(${query})`, 'gi'));
    return parts.map(part => 
      part.toLowerCase() === query.toLowerCase() 
        ? `<span class="highlight">${part}</span>` 
        : part
    ).join('');
  }
</script>

<div
  bind:this={panelRef}
  class="picker-panel"
  class:opening-up={openingDirection === 'up'}
  role="dialog"
  aria-label="Column picker"
  style="top: {openingDirection === 'down' ? anchorTop + 'px' : 'auto'}; 
         bottom: {openingDirection === 'up' ? (window.innerHeight - (anchorTop - 8)) + 'px' : 'auto'}; 
         right: {anchorRight}px;"
>
  <div class="picker-search">
    <Search size={14} class="search-icon" />
    <input
      type="text"
      placeholder="Search fields..."
      bind:value={searchQuery}
      class="search-input"
    />
    {#if searchQuery}
      <button onclick={() => searchQuery = ''} class="clear-btn">
        <X size={14} />
      </button>
    {/if}
  </div>

  <div class="picker-content">
    <div class="section-label">FIXED</div>
    <div class="fixed-section">
      {#each BASE_COLUMNS as col}
        <div class="row locked">
          <input type="checkbox" checked={true} disabled class="checkbox" />
          <span class="label">{col}</span>
        </div>
      {/each}
    </div>

    <div class="divider"></div>
    <div class="section-label">SCHEMA FIELDS</div>
    <div class="tree-section">
      {#snippet renderNode(node: TreeNode, depth: number, parentPath: string)}
        {@const currentPath = parentPath ? `${parentPath}.${node.key}` : node.key}
        {@const isLeaf = node.children.length === 0}
        
        <div class="tree-row-container">
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_interactive_supports_focus -->
          <div 
            class="row" 
            style="padding-left: {depth * 16 + (isLeaf ? 16 : 0)}px"
            onclick={() => isLeaf ? toggleColumn(node.fullPath!) : toggleExpand(node, currentPath)}
            role="button"
          >
            {#if !isLeaf}
              <span class="icon-toggle">
                {#if node.expanded}
                  <ChevronDown size={14} />
                {:else}
                  <ChevronRight size={14} />
                {/if}
              </span>
            {/if}

            {#if isLeaf}
              <input 
                type="checkbox" 
                checked={activeColumns.includes(node.fullPath!)} 
                class="checkbox" 
                tabindex="-1"
              />
            {/if}

            <span class="label truncate" title={isLeaf ? node.fullPath : undefined}>
              {@html highlightText(node.key, searchQuery)}
            </span>

            {#if isLeaf && node.fieldType}
              <span class="field-type">{node.fieldType}</span>
            {/if}

            {#if !isLeaf}
              {@const selectedCount = countSelectedLeaves(node, activeColumns)}
              {#if selectedCount > 0}
                <span class="badge badge-selected">({selectedCount} selected)</span>
              {/if}
              {#if !node.expanded && selectedCount === 0}
                <span class="badge badge-fields">({node.children.length} fields)</span>
              {/if}
            {/if}
          </div>

          {#if !isLeaf && node.expanded}
            <div class="children">
              {#each node.children as child}
                {@render renderNode(child, depth + 1, currentPath)}
              {/each}
            </div>
          {/if}
        </div>
      {/snippet}

      {#each filteredTree as node}
        {@render renderNode(node, 0, '')}
      {/each}

      {#if filteredTree.length === 0 && searchQuery}
        <div class="no-results">No fields match "{searchQuery}"</div>
      {/if}
    </div>
  </div>
</div>

<style>
  .picker-panel {
    position: fixed;
    z-index: 9999;
    width: 320px;
    max-height: 480px;
    background-color: var(--color-surface-elevated);
    border: 1px solid var(--color-border-dim);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .picker-search {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    border-bottom: 1px solid var(--color-border-dim);
    gap: 8px;
    background: var(--color-surface);
  }

  .search-icon {
    color: var(--color-text-muted);
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--color-text-primary);
    font-size: 13px;
    outline: none;
    height: 24px;
  }

  .clear-btn {
    background: transparent;
    border: none;
    color: var(--color-text-muted);
    cursor: pointer;
    display: flex;
    padding: 2px;
  }

  .clear-btn:hover {
    color: var(--color-text-primary);
  }

  .picker-content {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .section-label {
    padding: 10px 12px 6px;
    font-size: 10px;
    font-weight: 700;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    cursor: pointer;
    transition: background 0.1s;
    user-select: none;
  }

  .row:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .row.locked {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .checkbox {
    width: 14px;
    height: 14px;
    accent-color: var(--color-brand-primary);
    flex-shrink: 0;
    pointer-events: none;
  }

  .label {
    font-size: 13px;
    color: #fff;
    flex: 1;
    font-family: var(--font-mono);
  }

  .truncate {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .field-type {
    font-size: 11px;
    color: var(--color-text-muted);
    margin-left: 4px;
  }

  .icon-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    color: var(--color-text-muted);
    flex-shrink: 0;
  }

  .badge {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 4px;
    border-radius: 2px;
    margin-left: 6px;
  }

  .badge-selected {
    color: var(--color-brand-primary);
    background: rgba(var(--color-brand-primary-rgb), 0.1);
  }

  .badge-fields {
    color: var(--color-text-muted);
    background: rgba(255, 255, 255, 0.05);
  }

  .divider {
    height: 1px;
    background: var(--color-border-dim);
    margin: 4px 0;
  }

  .no-results {
    padding: 24px;
    text-align: center;
    color: var(--color-text-muted);
    font-size: 13px;
  }

  :global(.highlight) {
    color: var(--color-brand-primary);
    font-weight: 700;
  }

  /* Custom scrollbar */
  .picker-content::-webkit-scrollbar {
    width: 6px;
  }
  .picker-content::-webkit-scrollbar-thumb {
    background: var(--color-border-dim);
    border-radius: 10px;
  }
</style>
