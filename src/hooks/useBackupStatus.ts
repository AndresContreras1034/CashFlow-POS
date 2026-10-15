import { useCallback, useEffect, useState } from 'react';
import { getBackupStatus } from '../services/backup.service';
import type { BackupStatus } from '../types/backup';

/** Se dispara tras guardar ajustes o hacer un backup para refrescar badge y banner. */
export const BACKUP_CHANGED_EVENT = 'backup:changed';
export const notifyBackupChanged = () => window.dispatchEvent(new Event(BACKUP_CHANGED_EVENT));

let polling = true;

/** Evita consultas mientras la base se cambia por una restauración. */
export const setBackupPolling = (enabled: boolean) => {
  polling = enabled;
};

export function useBackupStatus(intervalMs = 30_000) {
  const [status, setStatus] = useState<BackupStatus | null>(null);

  const refresh = useCallback(async () => {
    if (!polling) return;
    try {
      setStatus(await getBackupStatus());
    } catch (error) {
      console.error('No se pudo consultar el estado del backup:', error);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const id = window.setInterval(() => void refresh(), intervalMs);
    window.addEventListener(BACKUP_CHANGED_EVENT, refresh);
    return () => {
      window.clearInterval(id);
      window.removeEventListener(BACKUP_CHANGED_EVENT, refresh);
    };
  }, [intervalMs, refresh]);

  return { status, refresh };
}
