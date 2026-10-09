export type DeveloperOutcome = 'success' | 'warn' | 'failure';
export type DeveloperLevel = 'warn' | 'error';

export interface DeveloperEvent {
  id: string;
  correlation_id: string | null;
  operation: string;
  target: string;
  timestamp: string; // ISO 8601
  duration_ms: number | null;
  outcome: DeveloperOutcome;
  level: DeveloperLevel;
  message: string;
}

export interface DeveloperStatus {
  enabled: boolean;
  count: number;
  capacity: number;
}

export interface HealthSnapshot {
  database: string;
  server_version: string;
  latency_ms: number;
  /** ISO-8601 UTC */
  db_time: string;
  db_timezone: string;
  pool_size: number;
  pool_idle: number;
  pool_max: number;
  app_version: string;
  profile: string;
  log_filter: string;
}