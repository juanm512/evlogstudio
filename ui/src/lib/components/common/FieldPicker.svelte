<script lang="ts">
  import { ChevronDown, ChevronRight, Search, X, Check } from 'lucide-svelte';
  import { onMount } from 'svelte';
  import type { SchemaField } from '$lib/types';
  import { 
    type TreeNode, 
    buildTree, 
    filterTree, 
    saveExpansionState, 
    getSavedExpansionState 
  } from '$lib/treeUtils';

  interface Props {
    availableFields: SchemaField[];
    selectedField: string | null;
    placeholder?: string;
    compact?: boolean;
    onSelect: (field: string) => void;
  }

  let { 
    availableFields, 
    selectedField, 
    placeholder = 'Select field', 
    compact = false,
    onSelect 
  }: Props = $props();

  const STORAGE_KEY = 'evlog_field_picker_state';

  let isOpen = $state(false);
  let selectRef = $state<HTMLDivElement | null>(null);
  let buttonRef = $state<HTMLButtonElement | null>(null);
  let dropdownStyles = $state({ top: '0px', left: '0px', width: '0px' });
  
  let tree = $state<TreeNode[]>([]);
  let searchQuery = $state('');

  function updateDropdownPosition() {
    if (!buttonRef) return;
    const rect = buttonRef.getBoundingClientRect();
    const dropdownHeight = 320; // Max height
    const spaceBelow = window.innerHeight - rect.bottom;
    const spaceAbove = rect.top;

    let top = rect.bottom + window.scrollY + 1;
    if (spaceBelow < dropdownHeight && spaceAbove > dropdownHeight) {
      top = rect.top + window.scrollY - dropdownHeight - 1;
    }

    dropdownStyles = {
      top: `${top}px`,
      left: `${rect.left + window.scrollX}px`,
      width: `${Math.max(rect.width, 300)}px`
    };
  }

  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    updateDropdownPosition();
    return {
      destroy() {
        if (node.parentNode) node.parentNode.removeChild(node);
      }
    };
  }

  function toggle(e: MouseEvent) {
    e.stopPropagation();
    if (!isOpen) {
      updateDropdownPosition();
    }
    isOpen = !isOpen;
  }

  function handleSelect(path: string) {
    onSelect(path);
    isOpen = false;
  }

  function toggleExpand(node: TreeNode, path: string) {
    node.expanded = !node.expanded;
    saveExpansionState(STORAGE_KEY, path, node.expanded);
  }

  function handleClickOutside(e: MouseEvent) {
    if (selectRef && !selectRef.contains(e.target as Node)) {
      isOpen = false;
    }
  }

  $effect(() => {
    if (!isOpen) return;
    window.addEventListener('click', handleClickOutside);
    window.addEventListener('scroll', updateDropdownPosition, true);
    window.addEventListener('resize', updateDropdownPosition);
    return () => {
      window.removeEventListener('click', handleClickOutside);
      window.removeEventListener('scroll', updateDropdownPosition, true);
      window.removeEventListener('resize', updateDropdownPosition);
    };
  });

  $effect(() => {
    if (availableFields) {
      tree = buildTree(availableFields, STORAGE_KEY);
    }
  });

  let filteredTree = $derived(filterTree(tree, searchQuery));

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

<div class="field-picker-wrapper" bind:this={selectRef}>
  <button
    type="button"
    bind:this={buttonRef}
    onclick={toggle}
    class="picker-trigger {compact ? 'compact' : ''}"
  >
    <span class="trigger-label {selectedField ? 'active' : 'placeholder'}">
      {selectedField || placeholder}
    </span>
    <ChevronDown size={compact ? 12 : 14} class="chevron {isOpen ? 'rotated' : ''}" />
  </button>

  {#if isOpen}
    <div
      use:portal
      class="picker-dropdown"
      style="top: {dropdownStyles.top}; left: {dropdownStyles.left}; width: {dropdownStyles.width};"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="search-area">
        <Search size={14} class="search-icon" />
        <input
          type="text"
          placeholder="Filter fields..."
          bind:value={searchQuery}
          class="search-input"
          autofocus
        />
        {#if searchQuery}
          <button onclick={(e) => { e.stopPropagation(); searchQuery = ''; }} class="clear-btn">
            <X size={14} />
          </button>
        {/if}
      </div>

      <div class="tree-content custom-scrollbar">
        {#snippet renderNode(node: TreeNode, depth: number, parentPath: string)}
          {@const currentPath = parentPath ? `${parentPath}.${node.key}` : node.key}
          {@const isLeaf = node.children.length === 0}
          {@const isSelected = selectedField === node.fullPath}
          
          <div class="tree-row-container">
            <button 
              type="button"
              class="tree-row {isSelected ? 'selected' : ''}" 
              style="padding-left: {depth * 16 + (isLeaf ? 16 : 0)}px"
              onclick={(e) => { e.stopPropagation(); isLeaf ? handleSelect(node.fullPath!) : toggleExpand(node, currentPath) }}
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

              <span class="label truncate" title={isLeaf ? node.fullPath : undefined}>
                {@html highlightText(node.key, searchQuery)}
              </span>

              {#if isLeaf && node.fieldType}
                <span class="field-type">{node.fieldType}</span>
              {/if}

              {#if isSelected}
                <Check size={12} class="check-icon" />
              {/if}

              {#if !isLeaf && !node.expanded && !searchQuery}
                <span class="badge">({node.children.length})</span>
              {/if}
            </button>

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

        {#if filteredTree.length === 0}
          <div class="no-results">
            {searchQuery ? `No matches for "${searchQuery}"` : 'No fields available'}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .field-picker-wrapper {
    width: 100%;
    position: relative;
  }

  .picker-trigger {
    width: 100%;
    background: var(--color-surface-elevated);
    border: 1px solid var(--color-border-dim);
    padding: 8px 12px;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    display: flex;
    items-center: center;
    justify-content: space-between;
    transition: all 0.15s;
    cursor: pointer;
    border-radius: 0;
    height: 40px;
    text-align: left;
  }

  .picker-trigger.compact {
    height: 30px;
    padding: 8px 6px;
    font-size: 10px;
  }

  .picker-trigger:hover {
    background: var(--color-surface);
    border-color: var(--color-text-muted);
  }

  .trigger-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding-right: 8px;
  }

  .trigger-label.active {
    color: var(--color-text-primary);
  }

  .trigger-label.placeholder {
    color: var(--color-text-muted);
  }

  .chevron {
    flex-shrink: 0;
    transition: transform 0.2s;
    color: var(--color-text-muted);
  }

  .chevron.rotated {
    transform: rotate(180deg);
  }

  .picker-dropdown {
    position: absolute;
    z-index: 10000;
    background: var(--color-surface);
    border: 1px solid var(--color-border-dim);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    max-height: 320px;
    border-radius: 0;
  }

  .search-area {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    border-bottom: 1px solid var(--color-border-dim);
    gap: 8px;
    background: var(--color-surface-elevated);
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
    font-size: 12px;
    outline: none;
    height: 24px;
    font-family: var(--font-mono);
  }

  .clear-btn {
    background: transparent;
    border: none;
    color: var(--color-text-muted);
    cursor: pointer;
    padding: 2px;
  }

  .tree-content {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  .tree-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    cursor: pointer;
    transition: background 0.1s;
    user-select: none;
    text-align: left;
    background: transparent;
    border: none;
    border-radius: 0;
  }

  .tree-row:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .tree-row.selected {
    background: rgba(var(--color-brand-primary-rgb), 0.1);
    color: var(--color-brand-primary);
  }

  .icon-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    color: var(--color-text-muted);
    flex-shrink: 0;
  }

  .label {
    font-size: 12px;
    flex: 1;
    font-family: var(--font-mono);
  }

  .truncate {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .field-type {
    font-size: 10px;
    color: var(--color-text-muted);
    opacity: 0.7;
    font-style: italic;
  }

  .check-icon {
    color: var(--color-brand-primary);
    flex-shrink: 0;
  }

  .badge {
    font-size: 9px;
    color: var(--color-text-muted);
    background: rgba(255, 255, 255, 0.05);
    padding: 1px 4px;
    border-radius: 2px;
  }

  .no-results {
    padding: 24px;
    text-align: center;
    color: var(--color-text-muted);
    font-size: 12px;
    font-family: var(--font-mono);
  }

  :global(.highlight) {
    color: var(--color-brand-primary);
    font-weight: 700;
    text-decoration: underline;
  }

  .custom-scrollbar::-webkit-scrollbar {
    width: 4px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: var(--color-border-dim);
  }
</style>
