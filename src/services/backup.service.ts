import { invoke } from '@tauri-apps/api/core';
import type {
  BackupDelivery,
  BackupEnvironment,
  BackupRecipient,
  BackupRecord,
  BackupSecretsStatus,
  BackupSettings,
  BackupStatus,
  EmailTestResult,
  UpdateBackupSettingsDto,
} from '../types/backup';

export const getBackupStatus = () => invoke<BackupStatus>('get_backup_status');
export const getBackupSettings = () => invoke<BackupSettings>('get_backup_settings');

export const updateBackupSettings = (dto: UpdateBackupSettingsDto) =>
  invoke<BackupSettings>('update_backup_settings', { dto });

export const checkBackupEnvironment = () =>
  invoke<BackupEnvironment>('check_backup_environment');

export const runBackupNow = (actor: string | null) =>
  invoke<BackupRecord>('run_backup_now', { actor });

export const listBackupHistory = (limit = 20) =>
  invoke<BackupRecord[]>('list_backup_history', { limit });

export const isBackupRunning = () => invoke<boolean>('is_backup_running');

export const getBackupSecretsStatus = () =>
  invoke<BackupSecretsStatus>('get_backup_secrets_status');

export const setBackupEncryptionPassword = (password: string | null) =>
  invoke<BackupSecretsStatus>('set_backup_encryption_password', { password });

export const setBackupSmtpPassword = (password: string | null) =>
  invoke<BackupSecretsStatus>('set_backup_smtp_password', { password });

export const listBackupRecipients = () =>
  invoke<BackupRecipient[]>('list_backup_recipients');

export const addBackupRecipient = (dto: {
  email: string;
  name: string | null;
  send_attachment: boolean;
}) => invoke<BackupRecipient>('add_backup_recipient', { dto });

export const updateBackupRecipient = (
  id: number,
  dto: { name: string | null; active: boolean; send_attachment: boolean },
) => invoke<BackupRecipient>('update_backup_recipient', { id, dto });

export const deleteBackupRecipient = (id: number) =>
  invoke<void>('delete_backup_recipient', { id });

export const sendBackupTestEmail = () =>
  invoke<EmailTestResult[]>('send_backup_test_email');

export const listBackupDeliveries = (limit = 30) =>
  invoke<BackupDelivery[]>('list_backup_deliveries', { limit });
