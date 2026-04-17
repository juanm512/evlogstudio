export interface Log {
  id: string;
  timestamp: string;
  source: string;
  level: string | null;
  message: string | null;
  fields: Record<string, unknown>;
  ingested_at: string;
}

export interface Source {
  id: string;
  name: string;
  description: string | null;
  retention_days: number;
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
