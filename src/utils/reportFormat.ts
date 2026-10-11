import type { HeatmapCell } from '../types/reports';
import { formatAttributes } from './format';

export function formatBps(bps: number | null): string {
  if (bps === null) return '—';
  const percent = new Intl.NumberFormat('es-CO', {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(Math.abs(bps) / 100);
  const sign = bps > 0 ? '+' : bps < 0 ? '-' : '';
  return `${sign}${percent} %`;
}

export function fillHeatmap(cells: HeatmapCell[]): HeatmapCell[][] {
  const matrix = Array.from({ length: 7 }, (_, weekday) =>
    Array.from({ length: 24 }, (_, hour): HeatmapCell => ({
      weekday,
      hour,
      tickets: 0,
      gross_sales: 0,
    })),
  );

  for (const cell of cells) {
    if (cell.weekday >= 0 && cell.weekday < 7 && cell.hour >= 0 && cell.hour < 24) {
      matrix[cell.weekday][cell.hour] = cell;
    }
  }

  return matrix;
}

export function formatVariantAttributes(attributes: unknown): string {
  if (
    typeof attributes !== 'object'
    || attributes === null
    || Array.isArray(attributes)
  ) {
    return '';
  }

  const stringAttributes = Object.fromEntries(
    Object.entries(attributes).filter(
      (entry): entry is [string, string] => typeof entry[1] === 'string',
    ),
  );
  return formatAttributes(stringAttributes);
}

export function formatDaysSinceSale(days: number | null): string {
  if (days === null) return 'Nunca vendido';
  return `${new Intl.NumberFormat('es-CO').format(days)} ${days === 1 ? 'día' : 'días'}`;
}
