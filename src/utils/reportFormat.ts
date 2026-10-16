import type {
  AgingLabel,
  DebtCategory,
  DebtUrgency,
  HeatmapCell,
  InventoryDaysReason,
  StockStatus,
} from '../types/reports';
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
    Object.entries(attributes)
      .filter(([, value]) =>
        value !== null && value !== undefined && String(value).trim() !== '')
      .map(([key, value]) => [key, String(value)]),
  );
  return formatAttributes(stringAttributes);
}

export function formatDaysSinceSale(days: number | null): string {
  if (days === null) return 'Nunca vendido';
  return `${new Intl.NumberFormat('es-CO').format(days)} ${days === 1 ? 'día' : 'días'}`;
}

export function formatTurnover(bps: number | null): string {
  if (bps === null) return '—';
  const times = new Intl.NumberFormat('es-CO', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(bps / 10_000);
  return `${times} veces`;
}

export function formatSalesBucketLabel(bucket: string, partial: boolean, monthly: boolean): string {
  if (!monthly) return bucket.slice(5);
  const month = new Intl.DateTimeFormat('es-CO', {
    month: 'short',
    timeZone: 'UTC',
  }).format(new Date(`${bucket}T00:00:00Z`)).replace('.', '');
  return `${month} ${bucket.slice(0, 4)}${partial ? ' (parcial)' : ''}`;
}

export function formatDaysOfCover(days: number | null): string {
  if (days === null) return 'Sin ventas';
  return `${new Intl.NumberFormat('es-CO').format(days)} ${days === 1 ? 'día' : 'días'}`;
}

export function inventoryDaysReasonLabel(reason: InventoryDaysReason): string {
  const labels: Record<InventoryDaysReason, string> = {
    range_too_short: 'El rango debe cubrir al menos 28 días',
    no_cogs: 'No hay costo de ventas en el período',
    low_cost_coverage: 'Menos del 80 % de las unidades vendidas tiene costo',
    implausible_result: 'El resultado supera el máximo plausible de 3.650 días',
  };
  return labels[reason];
}

export function stockStatusLabel(status: StockStatus): string {
  return status === 'out_of_stock' ? 'Agotado' : 'Bajo';
}

export function formatDueDays(days: number): string {
  if (days < 0) return `hace ${new Intl.NumberFormat('es-CO').format(Math.abs(days))} ${Math.abs(days) === 1 ? 'día' : 'días'}`;
  if (days === 0) return 'hoy';
  return `en ${new Intl.NumberFormat('es-CO').format(days)} ${days === 1 ? 'día' : 'días'}`;
}

export function agingLabel(label: AgingLabel): string {
  const labels: Record<AgingLabel, string> = {
    not_due: 'No vencidas',
    overdue_1_30: 'Vencidas 1–30 días',
    overdue_31_60: 'Vencidas 31–60 días',
    overdue_61_90: 'Vencidas 61–90 días',
    overdue_90_plus: 'Vencidas más de 90 días',
    no_due_date: 'Sin fecha de vencimiento',
  };
  return labels[label];
}

export function debtUrgencyLabel(urgency: DebtUrgency): string {
  const labels: Record<DebtUrgency, string> = {
    none: '—',
    ok: 'Ok',
    alert: 'Alerta',
    due_today: 'Hoy',
    overdue: 'Vencida',
  };
  return labels[urgency];
}

export function debtCategoryLabel(category: DebtCategory): string {
  const labels: Record<DebtCategory, string> = {
    maintenance: 'Mantenimiento',
    business_operation: 'Operación del negocio',
    fixed_services: 'Servicios y gastos fijos',
    other_obligations: 'Otras obligaciones',
  };
  return labels[category];
}
