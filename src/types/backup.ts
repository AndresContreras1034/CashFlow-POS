export interface BackupSettings {
  id: number;
  local_dir: string | null;
  pg_bin_dir: string | null;
  keep_daily: number;
  keep_weekly: number;
  keep_monthly: number;
  auto_enabled: boolean;
  schedule_time: string;   // "21:00:00"
  schedule_days: number;   // bits: lunes = 1 ... domingo = 64
  warn_after_days: number;
  critical_after_days: number;
  encrypt_enabled: boolean;
  smtp_enabled: boolean;
  smtp_host: string;
  smtp_port: number;
  smtp_security: 'tls' | 'starttls';
  smtp_username: string | null;
  smtp_from_name: string | null;
  email_on_failure: boolean;
  created_at: string;
  updated_at: string;
}

export interface UpdateBackupSettingsDto {
  local_dir: string | null;
  pg_bin_dir: string | null;
  keep_daily?: number;
  keep_weekly?: number;
  keep_monthly?: number;
  auto_enabled?: boolean;
  schedule_time?: string;  // "HH:MM"
  schedule_days?: number;
  warn_after_days?: number;
  critical_after_days?: number;
  encrypt_enabled?: boolean;
  smtp_enabled?: boolean;
  smtp_host?: string;
  smtp_port?: number;
  smtp_security?: 'tls' | 'starttls';
  smtp_username: string | null;      // se reemplaza siempre
  smtp_from_name: string | null;     // se reemplaza siempre
  email_on_failure?: boolean;
}

export type BackupAlertLevel = 'ok' | 'notice' | 'warning' | 'critical';

export interface BackupStatus {
  level: BackupAlertLevel;
  auto_enabled: boolean;
  running: boolean;
  last_success_at: string | null;
  hours_since_success: number | null;
  days_since_success: number | null;
  last_attempt_failed: boolean;
  last_error: string | null;
  failures_since_success: number;
  next_run_at: string | null;
  warn_after_days: number;
  critical_after_days: number;
}

export type BackupKind = 'manual' | 'scheduled' | 'catchup' | 'pre_restore' | 'pre_import';

export interface BackupRecord {
  id: number;
  kind: BackupKind;
  status: 'success' | 'failed';
  started_at: string;
  finished_at: string;
  duration_ms: number;
  file_name: string | null;
  file_path: string | null;
  size_bytes: number | null;
  sha256: string | null;
  pg_server_version: string | null;
  last_migration: number | null;
  error_message: string | null;
  created_by: string;
  file_deleted_at: string | null;
  encrypted: boolean;
  created_at: string;
}

export interface BackupRecipient {
  id: number;
  email: string;
  name: string | null;
  active: boolean;
  send_attachment: boolean;
  created_at: string;
  updated_at: string;
}

export interface BackupDelivery {
  id: number;
  backup_id: number | null;
  notice: 'success' | 'failure' | 'test';
  recipient_email: string;
  attached: boolean;
  status: 'sent' | 'failed';
  error_message: string | null;
  created_at: string;
}

export interface BackupSecretsStatus {
  encryption_password_set: boolean;
  smtp_password_set: boolean;
}

export interface EmailTestResult {
  email: string;
  ok: boolean;
  error: string | null;
}

export interface BackupEnvironment {
  database: string;
  server_version: string;
  database_size_bytes: number;
  default_dir: string;
  effective_dir: string;
  dir_writable: boolean;
  pg_dump_path: string | null;
  pg_dump_version: string | null;
  tools_error: string | null;
}

export interface CountItem {
  label: string;
  count: number;
}

export interface ArchiveDetails {
  dumped_server_version: string | null;
  dumped_database: string | null;
  archive_created_at: string | null;
  last_migration: number;
  app_last_migration: number;
  business_name: string | null;
  backup_counts: CountItem[];
  current_counts: CountItem[];
}

export interface BackupInspection {
  file_name: string;
  size_bytes: number;
  encrypted: boolean;
  needs_password: boolean;
  details: ArchiveDetails | null;
}

export interface RestoreOutcome {
  file_name: string;
  pre_restore_file: string | null;
  last_migration: number;
  warnings: string[];
  restarting: boolean;
}
