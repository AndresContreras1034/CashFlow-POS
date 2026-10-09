import { invoke } from '@tauri-apps/api/core';
import type { DeveloperEvent, DeveloperStatus, HealthSnapshot } from '../types/developer';

export async function setDeveloperMode(enabled: boolean): Promise<void> {
  return invoke('set_developer_mode', { enabled });
}

export async function getDeveloperStatus(): Promise<DeveloperStatus> {
  return invoke('get_developer_status');
}

export async function getDeveloperEvents(): Promise<DeveloperEvent[]> {
  return invoke('get_developer_events');
}

export async function clearDeveloperEvents(): Promise<void> {
  return invoke('clear_developer_events');
}

export async function getHealth(): Promise<HealthSnapshot> {
  return invoke('get_health');
}