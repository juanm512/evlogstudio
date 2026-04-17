<script lang="ts">
  import type { Log } from '$lib/types';
  import { format } from 'date-fns';
  import { X, Copy, Check } from 'lucide-svelte';

  interface Props {
    log: Log;
    onclose: () => void;
  }

  let { log, onclose }: Props = $props();

  let copied = $state(false);

  function formatFull(ts: string): string {
    try {
      return format(new Date(ts), 'yyyy-MM-dd HH:mm:ss.SSS zzz');
    } catch {
      return ts;
    }
  }

  async function copyJson() {
    try {
      await navigator.clipboard.writeText(JSON.stringify(log, null, 2));
      copied = true;
      setTimeout(() => { copied = false; }, 2000);
    } catch {
      // clipboard unavailable
    }
  }

  const levelBadge: Record<string, string> = {
    debug: 'badge-debug',
    info:  'badge-info',
    warn:  'badge-warn',
    error: 'badge-error',
    fatal: 'badge-fatal',
  };

  // Syntax-highlight JSON fields
  function highlightJson(obj: Record<string, unknown>): string {
    const json = JSON.stringify(obj, null, 2);
    return json.replace(
      /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g,
      (match) => {
        let cls = 'json-number';
        if (/^"/.test(match)) {
          cls = /:$/.test(match) ? 'json-key' : 'json-string';
        } else if (/true|false/.test(match)) {
          cls = 'json-bool';
        } else if (/null/.test(match)) {
          cls = 'json-null';
        }
        return `<span class="${cls}">${match}</span>`;
      }
    );
  }
</script>

<aside class="detail-panel" aria-label="Log detail">
  <!-- Header -->
  <div class="detail-header">
    <div class="flex flex-col gap-1 min-w-0">
      <span class="text-[10px] font-bold uppercase tracking-widest text-text-muted">Log Detail</span>
      <span class="font-mono text-[11px] text-text-secondary truncate">{log.id}</span>
    </div>
    <button
      onclick={onclose}
      aria-label="Close log detail"
      class="close-btn"
    >
      <X size={16} />
    </button>
  </div>

  <!-- Body -->
  <div class="detail-body">
    <!-- Meta row -->
    <div class="detail-section">
      <div class="meta-grid">
        <div class="meta-item">
          <span class="meta-label">Timestamp</span>
          <span class="meta-value font-mono">{formatFull(log.timestamp)}</span>
        </div>
        <div class="meta-item">
          <span class="meta-label">Source</span>
          <span class="meta-value font-mono">{log.source}</span>
        </div>
        <div class="meta-item">
          <span class="meta-label">Level</span>
          <span class="level-badge {levelBadge[log.level ?? ''] ?? 'badge-unknown'}">
            {log.level ?? '—'}
          </span>
        </div>
        <div class="meta-item">
          <span class="meta-label">Ingested</span>
          <span class="meta-value font-mono">{formatFull(log.ingested_at)}</span>
        </div>
      </div>
    </div>

    <!-- Message -->
    <div class="detail-section">
      <div class="section-title">Message</div>
      <p class="font-mono text-[13px] text-text-primary leading-relaxed whitespace-pre-wrap break-words">
        {log.message ?? '(no message)'}
      </p>
    </div>

    <!-- Fields JSON -->
    {#if log.fields && Object.keys(log.fields).length > 0}
      <div class="detail-section flex-1 flex flex-col min-h-0">
        <div class="section-title">Fields</div>
        <div class="json-wrapper">
          <pre class="json-pre" aria-label="Log fields JSON">{@html highlightJson(log.fields)}</pre>
        </div>
      </div>
    {/if}
  </div>

  <!-- Footer -->
  <div class="detail-footer">
    <button onclick={copyJson} class="copy-btn" aria-label="Copy full log as JSON">
      {#if copied}
        <Check size={14} class="text-brand-success" />
        <span class="text-brand-success">Copied!</span>
      {:else}
        <Copy size={14} />
        <span>Copy JSON</span>
      {/if}
    </button>
  </div>
</aside>

<style>
  .detail-panel {
    width: 380px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--color-border-dim);
    background-color: var(--color-surface);
    overflow: hidden;
    min-height: 0;
  }

  .detail-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--color-border-dim);
    background-color: color-mix(in srgb, var(--color-surface-elevated) 80%, transparent);
    flex-shrink: 0;
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    flex-shrink: 0;
    background: transparent;
    border: 1px solid var(--color-border-dim);
    color: var(--color-text-muted);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .close-btn:hover { background: var(--color-surface-elevated); color: var(--color-text-primary); }
  .close-btn:focus-visible { outline: 2px solid var(--color-brand-primary); outline-offset: 2px; }

  .detail-body {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0;
    min-height: 0;
  }

  .detail-section {
    padding: 14px 16px;
    border-bottom: 1px solid var(--color-border-dim);
  }

  .section-title {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--color-text-muted);
    margin-bottom: 8px;
  }

  .meta-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .meta-item { display: flex; flex-direction: column; gap: 3px; }
  .meta-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-text-muted);
  }
  .meta-value {
    font-size: 12px;
    color: var(--color-text-secondary);
    word-break: break-all;
  }

  .level-badge {
    display: inline-block;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 0;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    width: fit-content;
  }
  .badge-debug  { background: color-mix(in srgb, #A78BFA 12%, transparent); color: #A78BFA; }
  .badge-info   { background: color-mix(in srgb, #60A5FA 12%, transparent); color: #60A5FA; }
  .badge-warn   { background: color-mix(in srgb, #F59E0B 12%, transparent); color: #F59E0B; }
  .badge-error  { background: color-mix(in srgb, #EF4444 12%, transparent); color: #EF4444; }
  .badge-fatal  { background: color-mix(in srgb, #991B1B 20%, transparent); color: #FCA5A5; }
  .badge-unknown { background: color-mix(in srgb, #71717A 12%, transparent); color: #71717A; }

  /* JSON viewer */
  .json-wrapper {
    overflow: auto;
    background-color: var(--color-background);
    border: 1px solid var(--color-border-dim);
    max-height: 340px;
  }
  .json-pre {
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    padding: 12px 14px;
    margin: 0;
    white-space: pre;
    color: var(--color-text-secondary);
    min-width: max-content;
  }
  :global(.json-key)    { color: #60A5FA; }
  :global(.json-string) { color: #86EFAC; }
  :global(.json-number) { color: #FCA5A5; }
  :global(.json-bool)   { color: #F59E0B; }
  :global(.json-null)   { color: #71717A; }

  .detail-footer {
    padding: 10px 16px;
    border-top: 1px solid var(--color-border-dim);
    background-color: color-mix(in srgb, var(--color-surface-elevated) 60%, transparent);
    flex-shrink: 0;
  }
  .copy-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 600;
    color: var(--color-text-secondary);
    background: transparent;
    border: 1px solid var(--color-border-dim);
    cursor: pointer;
    transition: background 0.15s, color 0.15s, border-color 0.15s;
  }
  .copy-btn:hover {
    background: var(--color-surface-elevated);
    color: var(--color-text-primary);
    border-color: var(--color-text-muted);
  }
  .copy-btn:focus-visible { outline: 2px solid var(--color-brand-primary); outline-offset: 2px; }
</style>
