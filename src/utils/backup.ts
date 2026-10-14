export function formatSize(bytes: number | null | undefined): string {
  if (bytes == null) return '—';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  const seconds = ms / 1000;
  if (seconds < 60) return `${seconds.toFixed(1)} s`;
  return `${Math.floor(seconds / 60)} min ${Math.round(seconds % 60)} s`;
}

export const BACKUP_KIND_LABELS: Record<string, string> = {
  manual: 'Manual',
  scheduled: 'Programado',
  catchup: 'Recuperado al arrancar',
  pre_restore: 'Previo a restauración',
  pre_import: 'Previo a importación',
};

export function errorText(error: unknown, fallback: string): string {
  if (typeof error === 'string') return error;
  if (error instanceof Error) return error.message;
  return fallback;
}

export const DAY_LABELS = ['L', 'M', 'X', 'J', 'V', 'S', 'D'];
export const DAY_NAMES = ['Lunes', 'Martes', 'Miércoles', 'Jueves', 'Viernes', 'Sábado', 'Domingo'];

export const hasDay = (mask: number, index: number) => (mask & (1 << index)) !== 0;
export const toggleDay = (mask: number, index: number) => mask ^ (1 << index);

export function formatAgo(hours: number | null): string {
  if (hours == null) return 'nunca';
  if (hours < 1) return 'hace menos de 1 h';
  if (hours < 24) return `hace ${hours} h`;
  const days = Math.floor(hours / 24);
  return `hace ${days} ${days === 1 ? 'día' : 'días'}`;
}
