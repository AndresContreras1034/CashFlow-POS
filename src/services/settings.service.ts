import { invoke } from '@tauri-apps/api/core';
import type { AppSettings, UpdateSettingsDto } from '../types';

export const getSettings = (): Promise<AppSettings> =>
  invoke('get_settings');

export const updateSettings = (dto: UpdateSettingsDto): Promise<AppSettings> =>
  invoke('update_settings', { dto });