<script lang="ts">
  import { Plus, X, Trash2, Save, BookOpen } from 'lucide-svelte';
  import CustomSelect from '$lib/components/common/CustomSelect.svelte';
  import FieldPicker from '$lib/components/common/FieldPicker.svelte';
  import Modal from '$lib/components/common/Modal.svelte';
  import type { SchemaField, FilterCondition, Operator } from '$lib/types';

  interface Props {
    schemaFields: SchemaField[];
    conditions: FilterCondition[];
    onconditionschange: (conditions: FilterCondition[]) => void;
  }

  let { schemaFields, conditions, onconditionschange }: Props = $props();

  // ─── Operator metadata ────────────────────────────────────────────────────
  const ALL_OPERATORS: { id: Operator; label: string }[] = [
    { id: 'eq',       label: '= equals' },
    { id: 'neq',      label: '≠ not equals' },
    { id: 'contains', label: '∋ contains' },
    { id: 'starts',   label: '⊏ starts with' },
    { id: 'gt',       label: '> greater than' },
    { id: 'lt',       label: '< less than' },
    { id: 'exists',   label: '∃ exists' },
  ];

  const TYPE_OPS: Record<string, Operator[]> = {
    string:  ['eq', 'neq', 'contains', 'starts', 'exists'],
    number:  ['eq', 'neq', 'gt', 'lt', 'exists'],
    boolean: ['eq', 'exists'],
    object:  ['exists'],
    array:   ['exists'],
  };

  function opsForField(fieldPath: string): Operator[] {
    const f = schemaFields.find(f => f.field_path === fieldPath);
    if (!f) return ['eq', 'neq', 'contains', 'exists'];
    return TYPE_OPS[f.field_type] ?? ['eq', 'neq', 'contains', 'exists'];
  }

  function defaultOp(fieldPath: string): Operator {
    return opsForField(fieldPath)[0] ?? 'eq';
  }

  // ─── Field options (flat, prefixed by source if multiple) ─────────────────
  let sources = $derived([...new Set(schemaFields.map(f => f.source))].sort());


  function opOptions(fieldPath: string) {
    const allowed = opsForField(fieldPath);
    return ALL_OPERATORS.filter(o => allowed.includes(o.id));
  }

  // ─── Condition mutations ──────────────────────────────────────────────────
  function addCondition() {
    const firstField = schemaFields[0]?.field_path ?? '';
    onconditionschange([
      ...conditions,
      { id: crypto.randomUUID(), field: firstField, operator: defaultOp(firstField), value: '' },
    ]);
  }

  function removeCondition(id: string) {
    onconditionschange(conditions.filter(c => c.id !== id));
  }

  function clearAll() {
    onconditionschange([]);
  }

  function updateField(id: string, field: string) {
    onconditionschange(conditions.map(c =>
      c.id === id ? { ...c, field, operator: defaultOp(field), value: '' } : c
    ));
  }

  function updateOperator(id: string, operator: string) {
    onconditionschange(conditions.map(c =>
      c.id === id ? { ...c, operator: operator as Operator } : c
    ));
  }

  function updateValue(id: string, value: string) {
    onconditionschange(conditions.map(c =>
      c.id === id ? { ...c, value } : c
    ));
  }

  // ─── Presets ──────────────────────────────────────────────────────────────
  const PRESETS_KEY = 'evlog_filter_presets';

  interface FilterPreset {
    name: string;
    conditions: FilterCondition[];
  }

  function loadPresetsFromStorage(): FilterPreset[] {
    try {
      const raw = localStorage.getItem(PRESETS_KEY);
      if (!raw) return [];
      return JSON.parse(raw) as FilterPreset[];
    } catch {
      return [];
    }
  }

  function savePresetsToStorage(p: FilterPreset[]) {
    localStorage.setItem(PRESETS_KEY, JSON.stringify(p));
  }

  let presets = $state<FilterPreset[]>(loadPresetsFromStorage());

  let selectedPresetName = $state<string | null>(null);

  // Modals
  let showSaveModal    = $state(false);
  let showPresetsModal = $state(false);
  let presetNameInput  = $state('');

  function openSaveModal() {
    presetNameInput = '';
    showSaveModal = true;
  }

  function confirmSave() {
    const name = presetNameInput.trim();
    if (!name) return;
    const updated = [
      ...presets.filter(p => p.name !== name),
      { name, conditions: JSON.parse(JSON.stringify(conditions)) as FilterCondition[] },
    ];
    presets = updated;
    savePresetsToStorage(updated);
    selectedPresetName = name;
    showSaveModal = false;
  }

  function loadPreset(name: string) {
    const preset = presets.find(p => p.name === name);
    if (!preset) return;
    selectedPresetName = name;
    onconditionschange(JSON.parse(JSON.stringify(preset.conditions)) as FilterCondition[]);
  }

  function deletePreset(name: string) {
    const updated = presets.filter(p => p.name !== name);
    presets = updated;
    savePresetsToStorage(updated);
    if (selectedPresetName === name) selectedPresetName = null;
  }
</script>

<div class="af-panel" role="region" aria-label="Advanced filters">
  {#if schemaFields.length === 0}
    <p class="af-empty">
      Send logs first to see available fields.
    </p>
  {:else if conditions.length === 0}
    <p class="af-empty">
      No filters active. Click <strong>+ Add condition</strong> to start.
    </p>
  {:else}
    <ul class="af-list" role="list" aria-label="Active filter conditions">
      {#each conditions as cond (cond.id)}
        {@const availableOps = opOptions(cond.field)}
        <li class="af-row">
          <!-- Field selector -->
          <div class="af-select-wrap af-select-field">
            <FieldPicker
              availableFields={schemaFields}
              selectedField={cond.field}
              compact={true}
              placeholder="Field"
              onSelect={(id) => updateField(cond.id, id)}
            />
          </div>

          <!-- Operator selector -->
          <div class="af-select-wrap af-select-op">
            <CustomSelect
              options={availableOps}
              value={cond.operator}
              compact={true}
              placeholder="operator"
              onSelect={(id) => updateOperator(cond.id, id)}
            />
          </div>

          <!-- Value input (hidden for 'exists') -->
          {#if cond.operator !== 'exists'}
            <input
              type={cond.operator === 'gt' || cond.operator === 'lt' ? 'number' : 'text'}
              class="af-input"
              value={cond.value}
              oninput={(e) => updateValue(cond.id, (e.target as HTMLInputElement).value)}
              placeholder="value"
              aria-label="Filter value"
            />
          {:else}
            <span class="af-exists-gap" aria-hidden="true"></span>
          {/if}

          <!-- Remove -->
          <button
            class="af-remove-btn"
            onclick={() => removeCondition(cond.id)}
            aria-label="Remove filter condition"
            title="Remove condition"
          >
            <X size={12} />
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <!-- Footer -->
  <div class="af-footer">
    <div class="af-footer-left">
      {#if schemaFields.length > 0}
        <button class="af-btn af-btn-add" onclick={addCondition} aria-label="Add filter condition">
          <Plus size={12} />
          Add condition
        </button>
      {/if}
      {#if conditions.length > 0}
        <button class="af-btn af-btn-clear" onclick={clearAll} aria-label="Clear all conditions">
          <Trash2 size={12} />
          Clear all
        </button>
      {/if}
    </div>

    <div class="af-footer-right">
      <!-- Active preset chip -->
      {#if selectedPresetName}
        <span class="af-preset-chip">
          {selectedPresetName}
          <button
            class="af-preset-chip-remove"
            onclick={() => { selectedPresetName = null; }}
            aria-label="Unload preset {selectedPresetName}"
            title="Unload preset"
          ><X size={11} /></button>
        </span>
      {/if}

      <!-- Save preset -->
      {#if conditions.length > 0}
        <button class="af-btn af-btn-save" onclick={openSaveModal} aria-label="Save conditions as preset">
          <Save size={12} />
          Save
        </button>
      {/if}

      <!-- Open presets manager -->
      {#if presets.length > 0}
        <button class="af-btn af-btn-presets" onclick={() => { showPresetsModal = true; }} aria-label="Manage filter presets">
          <BookOpen size={12} />
          Presets ({presets.length})
        </button>
      {/if}
    </div>
  </div>
</div>

<!-- Save preset modal -->
<Modal
  open={showSaveModal}
  title="Save filter preset"
  onClose={() => { showSaveModal = false; }}
>
  <div class="modal-body">
    <label for="preset-name" class="modal-label">Preset name</label>
    <input
      id="preset-name"
      type="text"
      class="modal-input"
      bind:value={presetNameInput}
      placeholder="e.g. API errors, Slow requests…"
      onkeydown={(e) => { if (e.key === 'Enter') confirmSave(); }}
      autofocus
    />
    {#if presets.find(p => p.name === presetNameInput.trim())}
      <p class="modal-hint">A preset with this name already exists — it will be overwritten.</p>
    {/if}
    <div class="modal-actions">
      <button class="modal-btn modal-btn-cancel" onclick={() => { showSaveModal = false; }}>
        Cancel
      </button>
      <button
        class="modal-btn modal-btn-confirm"
        onclick={confirmSave}
        disabled={!presetNameInput.trim()}
      >
        Save
      </button>
    </div>
  </div>
</Modal>

<!-- Presets manager modal -->
<Modal
  open={showPresetsModal}
  title="Filter presets"
  onClose={() => { showPresetsModal = false; }}
>
  <div class="modal-body">
    {#if presets.length === 0}
      <p class="modal-empty">No presets saved yet.</p>
    {:else}
      <ul class="presets-list">
        {#each presets as preset (preset.name)}
          <li class="presets-row">
            <span class="presets-name">{preset.name}</span>
            <div class="presets-actions">
              <button
                class="modal-btn modal-btn-load"
                onclick={() => { loadPreset(preset.name); showPresetsModal = false; }}
              >
                Load
              </button>
              <button
                class="modal-btn modal-btn-delete"
                onclick={() => deletePreset(preset.name)}
                aria-label="Delete preset {preset.name}"
              >
                <Trash2 size={13} />
              </button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</Modal>

<style>
  .af-panel {
    padding: 8px 12px;
    background-color: var(--color-surface);
    border-bottom: 1px solid var(--color-border-dim);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .af-empty {
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--color-text-muted);
    padding: 4px 0;
    margin: 0;
  }

  .af-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .af-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  /* Wrapper to constrain CustomSelect width in a flex row */
  .af-select-wrap {
    flex-shrink: 0;
  }
  .af-select-wrap :global(.space-y-1\.5) {
    margin: 0;
  }

  .af-select-field  { width: 200px; }
  .af-select-op     { width: 150px; }
  .af-select-preset { width: 180px; }

  .af-input {
    height: 30px;
    min-width: 120px;
    flex: 1;
    background-color: var(--color-background);
    border: 1px solid var(--color-border-dim);
    color: var(--color-text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 0 8px;
    outline: none;
    transition: border-color 0.15s;
  }

  .af-input:focus {
    border-color: var(--color-brand-primary);
    outline: 2px solid color-mix(in srgb, var(--color-brand-primary) 30%, transparent);
    outline-offset: 0;
  }

  /* Chrome number input arrows */
  .af-input[type='number']::-webkit-inner-spin-button,
  .af-input[type='number']::-webkit-outer-spin-button { opacity: 0.4; }

  .af-exists-gap {
    flex: 1;
    min-width: 120px;
    height: 30px;
  }

  .af-remove-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 30px;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    flex-shrink: 0;
    transition: color 0.15s, border-color 0.15s;
  }
  .af-remove-btn:hover {
    color: var(--color-brand-danger);
    border-color: color-mix(in srgb, var(--color-brand-danger) 25%, transparent);
  }
  .af-remove-btn:focus-visible { outline: 2px solid var(--color-brand-primary); outline-offset: 2px; }

  /* Footer */
  .af-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    padding-top: 2px;
  }

  .af-footer-left,
  .af-footer-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .af-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 10px;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-mono);
    background: transparent;
    border: 1px solid var(--color-border-dim);
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.15s, color 0.15s, border-color 0.15s;
  }
  .af-btn:focus-visible { outline: 2px solid var(--color-brand-primary); outline-offset: 2px; }

  .af-btn-add {
    color: var(--color-brand-primary);
    border-color: color-mix(in srgb, var(--color-brand-primary) 30%, transparent);
  }
  .af-btn-add:hover { background: color-mix(in srgb, var(--color-brand-primary) 8%, transparent); }

  .af-btn-clear {
    color: var(--color-text-muted);
  }
  .af-btn-clear:hover {
    color: var(--color-brand-danger);
    border-color: color-mix(in srgb, var(--color-brand-danger) 25%, transparent);
    background: color-mix(in srgb, var(--color-brand-danger) 6%, transparent);
  }

  .af-btn-save {
    color: var(--color-brand-success);
    border-color: color-mix(in srgb, var(--color-brand-success) 30%, transparent);
  }
  .af-btn-save:hover { background: color-mix(in srgb, var(--color-brand-success) 8%, transparent); }

  .af-btn-presets {
    color: var(--color-text-secondary);
  }
  .af-btn-presets:hover { background: var(--color-surface-elevated); color: var(--color-text-primary); }

  /* Active preset chip */
  .af-preset-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 8px 0 10px;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--color-brand-primary);
    background: color-mix(in srgb, var(--color-brand-primary) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-brand-primary) 30%, transparent);
    white-space: nowrap;
  }

  .af-preset-chip-remove {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
    opacity: 0.7;
    transition: opacity 0.15s;
  }
  .af-preset-chip-remove:hover { opacity: 1; }
  .af-preset-chip-remove:focus-visible { outline: 2px solid var(--color-brand-primary); outline-offset: 1px; }

  /* Presets manager modal list */
  .presets-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .presets-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 10px;
    background: var(--color-background);
    border: 1px solid var(--color-border-dim);
  }

  .presets-name {
    font-size: 13px;
    font-family: var(--font-mono);
    color: var(--color-text-primary);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .presets-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .modal-empty {
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--color-text-muted);
    margin: 0;
    padding: 8px 0;
  }

  .modal-btn-load {
    background: color-mix(in srgb, var(--color-brand-primary) 10%, transparent);
    color: var(--color-brand-primary);
    border-color: color-mix(in srgb, var(--color-brand-primary) 30%, transparent);
  }
  .modal-btn-load:hover { background: color-mix(in srgb, var(--color-brand-primary) 18%, transparent); }

  .modal-btn-delete {
    background: transparent;
    color: var(--color-text-muted);
    border-color: transparent;
    padding: 0 6px;
  }
  .modal-btn-delete:hover {
    color: var(--color-brand-danger);
    border-color: color-mix(in srgb, var(--color-brand-danger) 25%, transparent);
    background: color-mix(in srgb, var(--color-brand-danger) 6%, transparent);
  }

  /* Save modal */
  .modal-body { display: flex; flex-direction: column; gap: 12px; }

  .modal-label {
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--color-text-secondary);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .modal-input {
    height: 36px;
    width: 100%;
    background: var(--color-background);
    border: 1px solid var(--color-border-dim);
    color: var(--color-text-primary);
    font-family: var(--font-mono);
    font-size: 13px;
    padding: 0 10px;
    outline: none;
    transition: border-color 0.15s;
  }
  .modal-input:focus {
    border-color: var(--color-brand-primary);
    outline: 2px solid color-mix(in srgb, var(--color-brand-primary) 30%, transparent);
    outline-offset: 0;
  }

  .modal-hint {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--color-warning);
    margin: 0;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 4px;
  }

  .modal-btn {
    height: 32px;
    padding: 0 16px;
    font-size: 12px;
    font-weight: 600;
    font-family: var(--font-mono);
    border: 1px solid var(--color-border-dim);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .modal-btn:focus-visible { outline: 2px solid var(--color-brand-primary); outline-offset: 2px; }
  .modal-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .modal-btn-cancel {
    background: transparent;
    color: var(--color-text-secondary);
  }
  .modal-btn-cancel:hover { background: var(--color-surface-elevated); color: var(--color-text-primary); }

  .modal-btn-confirm {
    background: var(--color-brand-primary);
    color: #fff;
    border-color: var(--color-brand-primary);
  }
  .modal-btn-confirm:not(:disabled):hover { background: color-mix(in srgb, var(--color-brand-primary) 80%, #000); }
</style>
