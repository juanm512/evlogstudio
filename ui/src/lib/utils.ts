import type { Log } from './types';

export function toCSV(logs: Log[]): string {
  if (logs.length === 0) return '';

  const fieldKeys = new Set<string>();
  for (const log of logs) {
    if (log.fields && typeof log.fields === 'object') {
      collectKeys(log.fields as Record<string, unknown>, '', fieldKeys);
    }
  }

  const baseHeaders = ['id', 'timestamp', 'source', 'level', 'message'];
  const fieldHeaders = [...fieldKeys].sort();
  const headers = [...baseHeaders, ...fieldHeaders];

  const rows: string[] = [headers.map(csvCell).join(',')];

  for (const log of logs) {
    const flatFields: Record<string, string> = {};
    if (log.fields && typeof log.fields === 'object') {
      flattenObject(log.fields as Record<string, unknown>, '', flatFields);
    }

    const row = [
      csvCell(log.id),
      csvCell(log.timestamp),
      csvCell(log.source),
      csvCell(log.level ?? ''),
      csvCell(log.message ?? ''),
      ...fieldHeaders.map(k => csvCell(flatFields[k] ?? '')),
    ];
    rows.push(row.join(','));
  }

  return rows.join('\n');
}

function collectKeys(obj: Record<string, unknown>, prefix: string, out: Set<string>): void {
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (v !== null && typeof v === 'object' && !Array.isArray(v)) {
      collectKeys(v as Record<string, unknown>, key, out);
    } else {
      out.add(key);
    }
  }
}

function flattenObject(obj: Record<string, unknown>, prefix: string, out: Record<string, string>): void {
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (v !== null && typeof v === 'object' && !Array.isArray(v)) {
      flattenObject(v as Record<string, unknown>, key, out);
    } else {
      out[key] = v === null ? '' : String(v);
    }
  }
}

function csvCell(value: string): string {
  if (value.includes(',') || value.includes('"') || value.includes('\n')) {
    return '"' + value.replace(/"/g, '""') + '"';
  }
  return value;
}

export function downloadCSV(content: string, filename: string): void {
  const blob = new Blob([content], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  link.style.display = 'none';
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}
