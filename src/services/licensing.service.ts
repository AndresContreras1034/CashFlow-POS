import { invoke } from '@tauri-apps/api/core';
import type { LicenseStatusDto } from '../types';

export const getLicenseStatus = (): Promise<LicenseStatusDto> =>
  invoke('get_license_status');

export const activateLicense = (filePath: string): Promise<void> =>
  invoke('activate_license', { filePath });
