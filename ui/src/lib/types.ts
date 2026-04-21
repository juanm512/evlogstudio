export interface Log {
  id: string;
  timestamp: string;
  source: string;
  service: string | null;
  environment: string | null;
  method: string | null;
  path: string | null;
  status: number | null;
  duration: number | null;
  request_id: string | null;
  error: string | null;
  level: string | null;
  message: string | null;
  fields: Record<string, unknown>;
  ingested_at: string;
}

export interface Source {
  id: string;
  name: string;
  description: string | null;
  retention: string | null;
  sampling_enabled: boolean | null;
  sampling_debug_rate: number | null;
  sampling_info_rate: number | null;
  sampling_warn_rate: number | null;
  created_at: string;
}

export interface SchemaField {
  source: string;
  field_path: string;
  field_type: string;
  seen_count: number;
  last_seen: string;
}

export interface User {
  id: string;
  email: string;
  role: string;
  created_at: string;
  last_login: string | null;
}

export interface IngestToken {
  id: string;
  name: string;
  source: string;
  created_by: string;
  created_at: string;
  last_used: string | null;
  revoked_at: string | null;
}

export type Operator = 'eq' | 'neq' | 'contains' | 'starts' | 'gt' | 'lt' | 'exists';

export interface FilterCondition {
  id: string;
  field: string;
  operator: Operator;
  value: string;
}

export interface LogsResponse {
  logs: Log[];
  next_cursor: string | null;
}

export interface PollResponse {
  logs: Log[];
  last_id: string | null;
  last_timestamp: string | null;
  count: number;
}

export interface SchemaResponse {
  fields: SchemaField[];
  sources: string[];
}

export interface VolumePoint {
  bucket: string;
  count: number;
}

export interface VolumeResponse {
  interval: string;
  from: string;
  to: string;
  data: VolumePoint[];
}

export interface ErrorRateResponse {
  from: string;
  to: string;
  total: number;
  errors: number;
  rate: number;
}

export interface Dashboard {
  id: string;
  name: string;
  description: string | null;
  created_by: string;
  created_at: string;
  updated_at: string;
  widget_count?: number;
  widgets?: Widget[];
}

export interface Widget {
  id: string;
  dashboard_id: string;
  title: string;
  type: 'bar' | 'line' | 'number' | 'table';
  width: 'half' | 'full';
  position: number;
  config: string;
}

export interface WidgetConfig {
  metric: string;
  field: string | null;
  group_by: { field: string; interval: string | null } | null;
  filters: Array<{ field: string; op: string; value: string }>;
  sources: string[];
  from: string | null;
  to: string | null;
}

export interface QueryResult {
  data: Array<{ group_key: string | null; value: number }>;
  meta: { metric: string; group_by: string | null; total_rows: number };
}
