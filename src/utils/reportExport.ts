import type {
  AgingBucket,
  CategorySales,
  DashboardKpis,
  InventoryTurnover,
  InventoryValuation,
  PayablesSummary,
  StaleProduct,
  StockAlert,
  TopCreditor,
  TopProduct,
  UpcomingDue,
} from '../types/reports';
import {
  agingLabel,
  debtCategoryLabel,
  debtUrgencyLabel,
  formatDaysSinceSale,
  formatVariantAttributes,
} from './reportFormat';
import { formatCsvAmount, type CsvCell } from './csv';

export interface ReportCsv {
  filename: string;
  headers: string[];
  rows: CsvCell[][];
}

export interface ExportRange {
  date_from: string;
  date_to: string;
  timezone: string;
  generatedAt: Date;
}

export interface DashboardExportData {
  dashboard: DashboardKpis;
}

export interface ProductsExportData {
  topProducts: TopProduct[];
  categories: CategorySales[];
  staleProducts: StaleProduct[];
}

export interface InventoryExportData {
  valuation: InventoryValuation;
  turnover: InventoryTurnover;
  alerts: StockAlert[];
}

export interface PayablesExportData {
  summary: PayablesSummary;
  aging: AgingBucket[];
  creditors: TopCreditor[];
  dues: UpcomingDue[];
}

function generatedLocalDate(range: ExportRange): string {
  const date = range.generatedAt;
  const parts = new Intl.DateTimeFormat('en-CA', {
    timeZone: range.timezone,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hourCycle: 'h23',
  }).formatToParts(date);
  const value = (type: Intl.DateTimeFormatPartTypes) =>
    parts.find(part => part.type === type)?.value ?? '';
  return `${value('year')}-${value('month')}-${value('day')} ${value('hour')}:${value('minute')}:${value('second')}`;
}

function baseRows(range: ExportRange): CsvCell[][] {
  return [
    ['Rango', `${range.date_from} — ${range.date_to}`],
    ['Generado', generatedLocalDate(range)],
    [],
  ];
}

function amount(value: number, decimals: number): string {
  return formatCsvAmount(value, decimals);
}

function attributes(value: unknown): string {
  return formatVariantAttributes(value);
}

export function buildSummaryCsv(
  data: DashboardExportData,
  range: ExportRange,
  decimals: number,
  comparePrevious = false,
): ReportCsv {
  const current = data.dashboard.current;
  const previous = comparePrevious ? data.dashboard.previous : null;
  const rows: CsvCell[][] = [
    ...baseRows(range),
    ['Indicadores del período'],
    ['Indicador', 'Período actual', ...(previous ? ['Período previo'] : [])],
    ['Ventas', amount(current.gross_sales, decimals), ...(previous ? [amount(previous.gross_sales, decimals)] : [])],
    ['Utilidad', amount(current.gross_profit, decimals), ...(previous ? [amount(previous.gross_profit, decimals)] : [])],
    ['Margen (bps)', current.margin_bps, ...(previous ? [previous.margin_bps] : [])],
    ['Tickets', current.tickets, ...(previous ? [previous.tickets] : [])],
    ['Ticket promedio', amount(current.avg_ticket, decimals), ...(previous ? [amount(previous.avg_ticket, decimals)] : [])],
    ['Unidades', current.units_sold, ...(previous ? [previous.units_sold] : [])],
    ['Descuentos', amount(current.discount_total, decimals), ...(previous ? [amount(previous.discount_total, decimals)] : [])],
    ['Impuestos', amount(current.tax_total, decimals), ...(previous ? [amount(previous.tax_total, decimals)] : [])],
    ['Costo de ventas', amount(current.cogs, decimals), ...(previous ? [amount(previous.cogs, decimals)] : [])],
    ['Cortesías', current.courtesy_count, ...(previous ? [previous.courtesy_count] : [])],
    ['Valor de cortesías', amount(current.courtesy_value, decimals), ...(previous ? [amount(previous.courtesy_value, decimals)] : [])],
    ['Ventas anuladas', current.cancelled_count, ...(previous ? [previous.cancelled_count] : [])],
    ['Ventas reembolsadas', current.refunded_count, ...(previous ? [previous.refunded_count] : [])],
  ];
  return {
    filename: `reporte-resumen-${range.date_from}_${range.date_to}.csv`,
    headers: ['Indicador', 'Valor', ...(previous ? ['Período previo'] : [])],
    rows,
  };
}

export function buildProductsCsv(
  data: ProductsExportData,
  range: ExportRange,
  decimals: number,
): ReportCsv {
  const rows: CsvCell[][] = [
    ...baseRows(range),
    ['Top productos'],
    ['#', 'Producto', 'Variante', 'Unidades', 'Ingreso neto', 'Ingreso bruto', 'Utilidad', 'Margen (bps)', 'Participación (bps)', 'Unidades sin costo'],
    ...data.topProducts.map((row, index) => [
      index + 1, row.product_name, attributes(row.attributes), row.units,
      amount(row.revenue, decimals), amount(row.gross_revenue, decimals),
      amount(row.profit, decimals), row.margin_bps, row.share_bps, row.uncosted_units,
    ]),
    [],
    ['Ventas por categoría'],
    ['Categoría', 'Unidades', 'Ingreso neto', 'Ingreso bruto', 'Utilidad', 'Margen (bps)', 'Participación (bps)', 'Unidades sin costo'],
    ...data.categories.map(row => [
      row.category_name, row.units, amount(row.revenue, decimals),
      amount(row.gross_revenue, decimals), amount(row.profit, decimals),
      row.margin_bps, row.share_bps, row.uncosted_units,
    ]),
    [],
    ['Productos sin movimiento'],
    ['Producto', 'Variante', 'Stock', 'Valor a costo', 'Días sin venta', 'Costo faltante'],
    ...data.staleProducts.map(row => [
      row.product_name, attributes(row.attributes), row.stock,
      amount(row.stock_value_at_cost, decimals), formatDaysSinceSale(row.days_since_last_sale),
      row.cost_missing ? 'Sí' : 'No',
    ]),
  ];
  return {
    filename: `reporte-productos-${range.date_from}_${range.date_to}.csv`,
    headers: ['Campo', 'Valor'],
    rows,
  };
}

export function buildInventoryCsv(
  data: InventoryExportData,
  range: ExportRange,
  decimals: number,
): ReportCsv {
  const rows: CsvCell[][] = [
    ...baseRows(range),
    ['Valorización por categoría · foto actual'],
    ['Categoría', 'Unidades', 'Valor a costo', 'Valor potencial de venta', 'Participación (bps)', 'Variantes sin costo'],
    ...data.valuation.by_category.map(row => [
      row.category_name, row.units, amount(row.value_at_cost, decimals),
      amount(row.potential_sale_value, decimals), row.share_bps, row.variants_without_cost,
    ]),
    ['Total de valorización', data.valuation.units_in_stock, amount(data.valuation.value_at_cost, decimals), amount(data.valuation.potential_sale_value, decimals)],
    ['Margen potencial (bps)', data.valuation.potential_margin_bps],
    ['Variantes sin costo', data.valuation.variants_without_cost],
    ['Variantes con stock negativo', data.valuation.negative_stock_variants],
    [],
    ['Rotación'],
    ['COGS del período', amount(data.turnover.cogs, decimals)],
    ['Inventario actual a costo', amount(data.turnover.inventory_value_at_cost, decimals)],
    ['Rotación (bps)', data.turnover.turnover_bps],
    ['Días de inventario', data.turnover.days_of_inventory],
    ['Motivo de días no disponibles', data.turnover.days_of_inventory_reason],
    ['Días del rango', data.turnover.range_days],
    ['Ítems sin costo', data.turnover.uncosted_items],
    [],
    ['Alertas de stock'],
    ['Producto', 'Variante', 'Categoría', 'Estado', 'Stock', 'Stock mínimo', 'Unidades vendidas', 'Días de cobertura', 'Reposición sugerida'],
    ...data.alerts.map(row => [
      row.product_name, attributes(row.attributes), row.category_name,
      row.status === 'out_of_stock' ? 'Agotado' : 'Bajo', row.stock, row.stock_min,
      row.units_sold, row.days_of_cover, row.suggested_reorder,
    ]),
  ];
  return {
    filename: `reporte-inventario-${range.date_from}_${range.date_to}.csv`,
    headers: ['Campo', 'Valor'],
    rows,
  };
}

export function buildPayablesCsv(
  data: PayablesExportData,
  range: ExportRange,
  decimals: number,
): ReportCsv {
  const rows: CsvCell[][] = [
    ...baseRows(range),
    ['Resumen'],
    ['Indicador', 'Valor'],
    ['Saldo por pagar', amount(data.summary.pending_total, decimals)],
    ['Vencido', amount(data.summary.overdue_total, decimals)],
    ['Por vencer en 7 días', amount(data.summary.due_soon_total, decimals)],
    ['Deudas activas', data.summary.active_count],
    ['Deudas vencidas', data.summary.overdue_count],
    ['Deudas por vencer', data.summary.due_soon_count],
    ['Pagado en el rango', amount(data.summary.paid_in_range, decimals)],
    ['Cantidad de pagos', data.summary.payments_in_range_count],
    ['Emitido en el rango', amount(data.summary.issued_in_range, decimals)],
    [],
    ['Antigüedad del vencimiento'],
    ['Antigüedad', 'Cantidad', 'Saldo'],
    ...data.aging.map(row => [agingLabel(row.label), row.count, amount(row.balance_total, decimals)]),
    [],
    ['Por categoría · foto actual'],
    ['Categoría', 'Deudas activas', 'Saldo', 'Vencido', 'Participación (bps)'],
    ...data.summary.by_category.map(row => [
      debtCategoryLabel(row.category), row.count, amount(row.balance_total, decimals),
      amount(row.overdue_total, decimals), row.share_bps,
    ]),
    [],
    ['Principales acreedores'],
    ['Acreedor', 'Deudas activas', 'Saldo', 'Vencido', 'Próximo vencimiento'],
    ...data.creditors.map(row => [
      row.creditor_name, row.active_count, amount(row.balance_total, decimals),
      amount(row.overdue_total, decimals), row.next_due_on,
    ]),
    [],
    ['Próximos vencimientos'],
    ['Vencimiento', 'Acreedor', 'Concepto', 'Categoría', 'Saldo', 'Urgencia', 'Días'],
    ...data.dues.map(row => [
      row.due_on, row.creditor_name, row.concept, debtCategoryLabel(row.category),
      amount(row.balance, decimals), debtUrgencyLabel(row.urgency), row.days_until_due,
    ]),
  ];
  return {
    filename: `reporte-obligaciones-${range.date_from}_${range.date_to}.csv`,
    headers: ['Campo', 'Valor'],
    rows,
  };
}
