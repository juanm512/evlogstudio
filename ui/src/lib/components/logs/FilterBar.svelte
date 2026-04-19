<script lang="ts">
  import { SlidersHorizontal, X, Search, ListFilter, ChevronDown, ChevronRight } from 'lucide-svelte';
  import ColumnPicker from './ColumnPicker.svelte';
  import AdvancedFilters from './AdvancedFilters.svelte';
  import CustomSelect from '../common/CustomSelect.svelte';
  import Modal from '../common/Modal.svelte';
  import type { SchemaField, FilterCondition } from '$lib/types';

  const LEVEL_OPTIONS = [
    { id: '',      label: 'All' },
    { id: 'debug', label: 'debug' },
    { id: 'info',  label: 'info' },
    { id: 'warn',  label: 'warn' },
    { id: 'error', label: 'error' },
    { id: 'fatal', label: 'fatal' },
  ];

  const METHOD_OPTIONS = [
    { id: '',        label: 'All' },
    { id: 'GET',     label: 'GET' },
    { id: 'POST',    label: 'POST' },
    { id: 'PUT',     label: 'PUT' },
    { id: 'PATCH',   label: 'PATCH' },
    { id: 'DELETE',  label: 'DELETE' },
  ];

  const ENV_OPTIONS = [
    { id: '',             label: 'All' },
    { id: 'production',   label: 'production' },
    { id: 'staging',      label: 'staging' },
    { id: 'development',  label: 'development' },
  ];

  const DATE_PRESETS = [
    { id: '',       label: 'All time' },
    { id: '15m',    label: 'Last 15 min' },
    { id: '1h',     label: 'Last 1 hour' },
    { id: '6h',     label: 'Last 6 hours' },
    { id: '24h',    label: 'Last 24 hours' },
    { id: '7d',     label: 'Last 7 days' },
    { id: '30d',    label: 'Last 30 days' },
    { id: 'custom', label: 'Custom range…' },
  ];

  const DATE_PRESET_MS: Record<string, number> = {
    '15m': 15 * 60_000,
    '1h':  3_600_000,
    '6h':  6 * 3_600_000,
    '24h': 24 * 3_600_000,
    '7d':  7 * 86_400_000,
    '30d': 30 * 86_400_000,
  };

  interface Filters {
    search: string;
    from: string;
    to: string;
    level: string;
    method: string;
    environment: string;
    status: string;
  }

  interface Props {
    filters: Filters;
    availableColumns: string[];
    activeColumns: string[];
    schemaFields?: SchemaField[];
    conditions?: FilterCondition[];
    onchange: (filters: Filters) => void;
    oncolumnschange: (cols: string[]) => void;
    onconditionschange?: (conditions: FilterCondition[]) => void;
    isLive?: boolean;
    ontoggleLive?: () => void;
  }

  let {
    filters,
    availableColumns,
    activeColumns,
    schemaFields = [],
    conditions = [],
    onchange,
    oncolumnschange,
    onconditionschange,
    isLive = false,
    ontoggleLive,
  }: Props = $props();

  let showColumnPicker = $state(false);
  let showAdvanced     = $state(false);
  let showSecondBar    = $state(true);
  let showCustomModal  = $state(false);
  let searchDebounce: ReturnType<typeof setTimeout>;

  let localSearch  = $state('');
  let datePreset   = $state('');
  let customFrom   = $state('');
  let customTo     = $state('');
  let localLevel   = $state('');
  let localMethod  = $state('');
  let localEnv     = $state('');
  let localStatus  = $state('');
  // Temporary state used only inside the modal
  let pendingFrom  = $state('');
  let pendingTo    = $state('');

  // Sync from parent resets
  $effect(() => {
    localSearch  = filters.search;
    localLevel   = filters.level ?? '';
    localMethod  = filters.method ?? '';
    localEnv     = filters.environment ?? '';
    localStatus  = filters.status ?? '';
    if (!filters.from && !filters.to) {
      datePreset = '';
      customFrom = '';
      customTo   = '';
    }
  });

  function computeDateRange(): { from: string; to: string } {
    if (datePreset === 'custom') return { from: customFrom, to: customTo };
    if (!datePreset) return { from: '', to: '' };
    const ms = DATE_PRESET_MS[datePreset];
    return { from: new Date(Date.now() - ms).toISOString(), to: '' };
  }

  function emit() {
    const { from, to } = computeDateRange();
    onchange({
      search:      localSearch,
      from,
      to,
      level:       localLevel,
      method:      localMethod,
      environment: localEnv,
      status:      localStatus,
    });
  }

  function onSearchInput() {
    clearTimeout(searchDebounce);
    searchDebounce = setTimeout(emit, 300);
  }

  function clearSearch() {
    localSearch = '';
    clearTimeout(searchDebounce);
    emit();
  }

  function onLevelChange(id: string)  { localLevel  = id; emit(); }
  function onMethodChange(id: string) { localMethod = id; emit(); }
  function onEnvChange(id: string)    { localEnv    = id; emit(); }

  function onDatePresetChange(id: string) {
    if (id === 'custom') {
      pendingFrom = customFrom;
      pendingTo   = customTo;
      showCustomModal = true;
      return;
    }
    datePreset = id;
    customFrom = '';
    customTo   = '';
    emit();
  }

  function applyCustomRange() {
    customFrom      = pendingFrom;
    customTo        = pendingTo;
    datePreset      = 'custom';
    showCustomModal = false;
    emit();
  }

  function cancelCustomRange() {
    showCustomModal = false;
  }

  // Keyboard: "/" focuses search
  let searchEl: HTMLInputElement;
  let columnsBtn: HTMLButtonElement;

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === '/' && document.activeElement !== searchEl) {
      e.preventDefault();
      searchEl?.focus();
    } else if (e.key === 'Escape' && document.activeElement === searchEl) {
      localSearch = '';
      clearTimeout(searchDebounce);
      emit();
      searchEl?.blur();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="filter-bar-wrap">
  <!-- ── Top bar ────────────────────────────────────────────────── -->
  <div class="filter-bar" role="toolbar" aria-label="Log filters">

    <!-- Chevron: colapsa/expande segunda barra -->
    <button
      onclick={() => { showSecondBar = !showSecondBar; }}
      class="chevron-btn"
      aria-label={showSecondBar ? 'Collapse filter bar' : 'Expand filter bar'}
      aria-expanded={showSecondBar}
      title={showSecondBar ? 'Collapse filters' : 'Expand filters'}
    >
      {#if showSecondBar}
        <ChevronDown size={14} />
      {:else}
        <ChevronRight size={14} />
      {/if}
    </button>

    <!-- Advanced filters toggle -->
    {#if onconditionschange}
      <button
        onclick={() => { showAdvanced = !showAdvanced; }}
        class="filter-btn {showAdvanced || conditions.length > 0 ? 'filter-btn-active' : ''}"
        aria-label="Toggle advanced filters"
        aria-expanded={showAdvanced}
      >
        <ListFilter size={13} />
        {#if conditions.length > 0}
          Filters ({conditions.length})
        {:else}
          Add filter
        {/if}
      </button>
    {/if}

    <!-- Search -->
    <div class="filter-group search-group">
      <span class="search-icon-wrap" aria-hidden="true"><Search size={13} /></span>
      <label for="filter-search" class="sr-only">Search events</label>
      <input
        id="filter-search"
        bind:this={searchEl}
        type="text"
        placeholder="Search events… (press / to focus)"
        bind:value={localSearch}
        oninput={onSearchInput}
        class="filter-input {localSearch ? 'has-clear' : ''}"
      />
      {#if localSearch}
        <button
          class="search-clear-btn"
          onclick={clearSearch}
          aria-label="Clear search"
          tabindex="-1"
        >
          <X size={12} />
        </button>
      {/if}
    </div>

    <!-- Spacer -->
    <div class="filter-spacer" aria-hidden="true"></div>

    <!-- Column picker toggle -->
    <button
      bind:this={columnsBtn}
      onclick={() => { showColumnPicker = !showColumnPicker; }}
      class="filter-btn {showColumnPicker ? 'filter-btn-active' : ''}"
      aria-label="Toggle column picker"
      aria-expanded={showColumnPicker}
    >
      <SlidersHorizontal size={13} />
      Columns
    </button>

    <!-- Live mode toggle -->
    {#if ontoggleLive}
      <button
        onclick={ontoggleLive}
        class="filter-btn live-btn {isLive ? 'live-btn-active' : ''}"
        aria-label={isLive ? 'Disable live mode' : 'Enable live mode'}
        aria-pressed={isLive}
      >
        <span class="live-dot {isLive ? 'live-dot-pulse' : ''}"></span>
        Live
      </button>
    {/if}

    <!-- Column picker dropdown -->
    {#if showColumnPicker}
      {@const rect = columnsBtn?.getBoundingClientRect()}
      <ColumnPicker
        {availableColumns}
        {activeColumns}
        anchorTop={rect ? rect.bottom + 4 : 0}
        anchorRight={rect ? window.innerWidth - rect.right : 0}
        onchange={oncolumnschange}
        onclose={() => { showColumnPicker = false; }}
      />
    {/if}
  </div>

  <!-- ── Segunda barra (colapsable) ────────────────────────────── -->
  {#if showSecondBar}
    <div class="second-bar" role="group" aria-label="Quick filters">

      <!-- Tiempo -->
      <div class="sb-field sb-field-date">
        <CustomSelect
          options={DATE_PRESETS}
          value={datePreset}
          compact={true}
          placeholder="All time"
          onSelect={onDatePresetChange}
        />
      </div>

      <div class="sb-divider" aria-hidden="true"></div>

      <!-- Level -->
      <div class="sb-field">
        <span class="sb-label">Level</span>
        <div class="sb-select-wrap">
          <CustomSelect
            options={LEVEL_OPTIONS}
            value={localLevel}
            compact={true}
            placeholder="All"
            onSelect={onLevelChange}
          />
        </div>
      </div>

      <!-- Method -->
      <div class="sb-field">
        <span class="sb-label">Method</span>
        <div class="sb-select-wrap">
          <CustomSelect
            options={METHOD_OPTIONS}
            value={localMethod}
            compact={true}
            placeholder="All"
            onSelect={onMethodChange}
          />
        </div>
      </div>

      <!-- Environment -->
      <div class="sb-field">
        <span class="sb-label">Env</span>
        <div class="sb-select-wrap sb-select-wrap-env">
          <CustomSelect
            options={ENV_OPTIONS}
            value={localEnv}
            compact={true}
            placeholder="All"
            onSelect={onEnvChange}
          />
        </div>
      </div>

      <!-- Status -->
      <div class="sb-field">
        <label for="sb-status" class="sb-label">Status</label>
        <input
          id="sb-status"
          type="text"
          inputmode="numeric"
          class="sb-input"
          placeholder="200"
          bind:value={localStatus}
          oninput={emit}
        />
      </div>

    </div>
  {/if}

  <!-- ── Advanced filters ───────────────────────────────────────── -->
  {#if showAdvanced && onconditionschange}
    <AdvancedFilters
      {schemaFields}
      {conditions}
      onconditionschange={onconditionschange}
    />
  {/if}
</div>

<!-- Custom date range modal -->
<Modal
  open={showCustomModal}
  title="Custom date range"
  onClose={cancelCustomRange}
>
  <div class="cr-body">
    <div class="cr-field">
      <label for="cr-from" class="cr-label">From</label>
      <input
        id="cr-from"
        type="datetime-local"
        class="cr-input"
        bind:value={pendingFrom}
        color-scheme="dark"
      />
    </div>
    <div class="cr-field">
      <label for="cr-to" class="cr-label">To</label>
      <input
        id="cr-to"
        type="datetime-local"
        class="cr-input"
        bind:value={pendingTo}
        color-scheme="dark"
      />
    </div>
    <div class="cr-actions">
      <button class="cr-btn cr-btn-cancel" onclick={cancelCustomRange}>Cancel</button>
      <button class="cr-btn cr-btn-apply" onclick={applyCustomRange}>Apply</button>
    </div>
  </div>
</Modal>

<style>
  .filter-bar-wrap {
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  /* ── Top bar ────────────────────────────────────────────────── */
  .filter-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--color-border-dim);
    background-color: var(--color-surface);
    flex-shrink: 0;
    flex-wrap: nowrap;
    overflow-x: auto;
  }

  .chevron-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    flex-shrink: 0;
    background: transparent;
    border: none;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: color 0.15s;
    padding: 0;
  }
  .chevron-btn:hover { color: var(--color-text-primary); }
  .chevron-btn:focus-visible {
    outline: 2px solid var(--color-brand-primary);
    outline-offset: 2px;
  }

  .filter-group {
    display: flex;
    align-items: center;
    position: relative;
  }

  .search-group {
    flex: 1;
    min-width: 180px;
  }

  .search-icon-wrap {
    position: absolute;
    left: 9px;
    pointer-events: none;
    z-index: 1;
    display: flex;
    align-items: center;
    color: var(--color-text-muted);
  }

  .search-clear-btn {
    position: absolute;
    right: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    padding: 0;
    background: transparent;
    border: none;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: color 0.15s;
    z-index: 1;
  }
  .search-clear-btn:hover { color: var(--color-text-primary); }

  .filter-spacer {
    flex: 0 1 auto;
    min-width: 4px;
  }

  .filter-input {
    height: 28px;
    background-color: var(--color-background);
    border: 1px solid var(--color-border-dim);
    color: var(--color-text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 0 10px 0 28px;
    outline: none;
    width: 100%;
    transition: border-color 0.15s;
  }
  .filter-input.has-clear { padding-right: 28px; }
  .filter-input:focus {
    border-color: var(--color-brand-primary);
    outline: 2px solid color-mix(in srgb, var(--color-brand-primary) 30%, transparent);
    outline-offset: 0;
  }

  .filter-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 28px;
    padding: 0 10px;
    font-size: 12px;
    font-weight: 600;
    color: var(--color-text-secondary);
    background: transparent;
    border: 1px solid var(--color-border-dim);
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.15s, color 0.15s, border-color 0.15s;
    flex-shrink: 0;
  }
  .filter-btn:hover {
    background: var(--color-surface-elevated);
    color: var(--color-text-primary);
  }
  .filter-btn:focus-visible {
    outline: 2px solid var(--color-brand-primary);
    outline-offset: 2px;
  }
  .filter-btn-active {
    background: color-mix(in srgb, var(--color-brand-primary) 10%, transparent);
    border-color: color-mix(in srgb, var(--color-brand-primary) 30%, transparent);
    color: var(--color-brand-primary);
  }

  .sr-only {
    position: absolute; width: 1px; height: 1px;
    padding: 0; margin: -1px; overflow: hidden;
    clip: rect(0,0,0,0); white-space: nowrap; border: 0;
  }

  .live-btn { gap: 7px; }
  .live-btn-active {
    background: color-mix(in srgb, var(--color-brand-success) 10%, transparent);
    border-color: color-mix(in srgb, var(--color-brand-success) 35%, transparent);
    color: var(--color-brand-success);
  }
  .live-btn-active:hover {
    background: color-mix(in srgb, var(--color-brand-success) 16%, transparent);
    color: var(--color-brand-success);
  }
  .live-dot {
    display: inline-block;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-text-muted);
    flex-shrink: 0;
  }
  .live-btn-active .live-dot { background: var(--color-brand-success); }
  .live-dot-pulse { animation: live-pulse 1.4s ease-in-out infinite; }
  @keyframes live-pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50%       { opacity: 0.4; transform: scale(0.75); }
  }

  /* ── Segunda barra ──────────────────────────────────────────── */
  .second-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 5px 8px;
    border-bottom: 1px solid var(--color-border-dim);
    background-color: var(--color-surface);
    flex-shrink: 0;
    flex-wrap: wrap;
  }

  .sb-divider {
    width: 1px;
    height: 16px;
    background: var(--color-border-dim);
    flex-shrink: 0;
    margin: 0 4px;
  }

  .sb-field {
    display: flex;
    align-items: center;
    gap: 5px;
    flex-shrink: 0;
  }

  .sb-label {
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-muted);
    white-space: nowrap;
  }

  .sb-input {
    height: 26px;
    background-color: var(--color-background);
    border: 1px solid var(--color-border-dim);
    color: var(--color-text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 0 6px;
    outline: none;
    transition: border-color 0.15s;
    cursor: text;
  }
  .sb-input  { width: 64px; }
  .sb-input:focus {
    border-color: var(--color-brand-primary);
    outline: 2px solid color-mix(in srgb, var(--color-brand-primary) 30%, transparent);
    outline-offset: 0;
  }

  /* CustomSelect wrappers inside second bar */
  .sb-field-date      { min-width: 130px; }
  .sb-select-wrap     { width: 90px; }
  .sb-select-wrap-env { width: 120px; }
  .sb-field :global(.space-y-1\.5) { margin: 0; }

  /* ── Custom date range modal ────────────────────────────────── */
  .cr-body {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .cr-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .cr-label {
    font-size: 11px;
    font-family: var(--font-mono);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-secondary);
  }

  .cr-input {
    height: 36px;
    width: 100%;
    background: var(--color-background);
    border: 1px solid var(--color-border-dim);
    color: var(--color-text-primary);
    font-family: var(--font-mono);
    font-size: 13px;
    padding: 0 10px;
    outline: none;
    color-scheme: dark;
    transition: border-color 0.15s;
  }
  .cr-input:focus {
    border-color: var(--color-brand-primary);
    outline: 2px solid color-mix(in srgb, var(--color-brand-primary) 30%, transparent);
    outline-offset: 0;
  }

  .cr-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 4px;
  }

  .cr-btn {
    height: 32px;
    padding: 0 18px;
    font-size: 12px;
    font-weight: 600;
    font-family: var(--font-mono);
    border: 1px solid var(--color-border-dim);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .cr-btn:focus-visible { outline: 2px solid var(--color-brand-primary); outline-offset: 2px; }

  .cr-btn-cancel {
    background: transparent;
    color: var(--color-text-secondary);
  }
  .cr-btn-cancel:hover {
    background: var(--color-surface-elevated);
    color: var(--color-text-primary);
  }

  .cr-btn-apply {
    background: var(--color-brand-primary);
    color: #fff;
    border-color: var(--color-brand-primary);
  }
  .cr-btn-apply:hover {
    background: color-mix(in srgb, var(--color-brand-primary) 80%, #000);
  }
</style>
