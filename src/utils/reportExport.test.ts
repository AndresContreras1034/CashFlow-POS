import { describe, expect, it } from 'vitest';
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
  buildInventoryCsv,
  buildPayablesCsv,
  buildProductsCsv,
  buildSummaryCsv,
  type ExportRange,
} from './reportExport';

const range: ExportRange = {
  date_from: '2026-10-01',
  date_to: '2026-10-11',
  timezone: 'America/Bogota',
  generatedAt: new Date('2026-10-11T17:25:52.000Z'),
};

const kpis = {
  tickets: 2,
  gross_sales: 120_000,
  discount_total: 0,
  tax_total: 20_000,
  net_sales: 100_000,
  avg_ticket: 50_000,
  cogs: 60_000,
  gross_profit: 40_000,
  margin_bps: 4_000,
  units_sold: 3,
  items_without_cost: 0,
  cancelled_count: 0,
  refunded_count: 0,
  courtesy_count: 0,
  courtesy_cost: 0,
  uncosted_tickets: 0,
  profit_base_net_sales: 100_000,
  courtesy_value: 0,
};

const dashboard: DashboardKpis = {
  range: { from: range.date_from, to: range.date_to },
  previous_range: null,
  current: kpis,
  previous: null,
  gross_sales_delta: { current: 120_000, previous: null, change_bps: null },
  gross_profit_delta: { current: 40_000, previous: null, change_bps: null },
  tickets_delta: { current: 2, previous: null, change_bps: null },
  avg_ticket_delta: { current: 50_000, previous: null, change_bps: null },
};

const topProduct: TopProduct = {
  product_id: 1,
  variant_id: 11,
  product_name: 'Café',
  attributes: { tamaño: '250g' },
  units: 3,
  gross_revenue: 120_000,
  revenue: 100_000,
  profit: 40_000,
  margin_bps: 4_000,
  uncosted_units: 0,
  share_bps: 10_000,
};

const categorySale: CategorySales = {
  category_id: 1,
  category_name: 'Bebidas',
  units: 3,
  gross_revenue: 120_000,
  revenue: 100_000,
  profit: 40_000,
  margin_bps: 4_000,
  uncosted_units: 0,
  share_bps: 10_000,
};

const staleProduct: StaleProduct = {
  variant_id: 11,
  product_name: 'Café',
  attributes: { tamaño: '250g' },
  stock: 4,
  stock_value_at_cost: 60_000,
  cost_missing: false,
  days_since_last_sale: null,
  total_value_at_cost: 60_000,
  total_rows: 1,
  total_without_cost: 0,
};

const valuation: InventoryValuation = {
  value_at_cost: 60_000,
  potential_sale_value: 100_000,
  potential_margin_bps: 4_000,
  variants_valued: 1,
  variants_without_cost: 0,
  negative_stock_variants: 0,
  units_in_stock: 4,
  by_category: [{
    category_id: 1,
    category_name: 'Bebidas',
    units: 4,
    value_at_cost: 60_000,
    potential_sale_value: 100_000,
    share_bps: 10_000,
    variants_without_cost: 0,
  }],
};

const turnover: InventoryTurnover = {
  cogs: 60_000,
  inventory_value_at_cost: 60_000,
  turnover_bps: 10_000,
  days_of_inventory: 11,
  days_of_inventory_reason: null,
  range_days: 11,
  uncosted_items: 0,
};

const stockAlert: StockAlert = {
  variant_id: 11,
  product_name: 'Café',
  attributes: { tamaño: '250g' },
  category_name: 'Bebidas',
  stock: 0,
  stock_min: 2,
  status: 'out_of_stock',
  units_sold: 3,
  days_of_cover: 0,
  suggested_reorder: 3,
};

const payablesSummary: PayablesSummary = {
  pending_total: 100_000,
  overdue_total: 20_000,
  due_soon_total: 30_000,
  active_count: 2,
  overdue_count: 1,
  due_soon_count: 1,
  paid_in_range: 10_000,
  payments_in_range_count: 1,
  issued_in_range: 50_000,
  by_category: [{
    category: 'maintenance',
    count: 1,
    balance_total: 100_000,
    overdue_total: 20_000,
    share_bps: 10_000,
  }],
  by_urgency: [],
};

const aging: AgingBucket[] = [{
  label: 'overdue_1_30',
  count: 1,
  balance_total: 20_000,
}];

const creditor: TopCreditor = {
  creditor_name: 'Proveedor Uno',
  active_count: 1,
  balance_total: 100_000,
  overdue_total: 20_000,
  next_due_on: '2026-10-12',
};

const due: UpcomingDue = {
  debt_id: 1,
  creditor_name: 'Proveedor Uno',
  concept: 'Factura',
  category: 'maintenance',
  due_on: '2026-10-12',
  balance: 100_000,
  days_until_due: 1,
  urgency: 'alert',
};

describe('CSV report builders', () => {
  it('builds summary KPIs for the selected period', () => {
    const result = buildSummaryCsv({ dashboard }, range, 2);
    expect(result.filename).toBe('reporte-resumen-2026-10-01_2026-10-11.csv');
    expect(result.rows.slice(0, 2)).toEqual([
      ['Rango', '2026-10-01 — 2026-10-11'],
      ['Generado', '2026-10-11 12:25:52'],
    ]);
    expect(result.rows).toContainEqual(['Ventas', '1200,00']);
  });

  it('includes previous-period KPIs when comparison data is available', () => {
    const result = buildSummaryCsv({
      dashboard: {
        ...dashboard,
        previous_range: { from: '2026-09-20', to: '2026-09-30' },
        previous: { ...kpis, gross_sales: 90_000 },
      },
    }, range, 0, true);
    expect(result.headers).toEqual(['Indicador', 'Valor', 'Período previo']);
    expect(result.rows).toContainEqual(['Ventas', '1200', '900']);
  });

  it('builds product sections with top products, categories and stale stock', () => {
    const result = buildProductsCsv({
      topProducts: [topProduct],
      categories: [categorySale],
      staleProducts: [staleProduct],
    }, range, 0);
    expect(result.filename).toContain('reporte-productos-');
    expect(result.rows).toContainEqual([
      1, 'Café', 'Tamaño: 250g', 3, '1000', '1200', '400', 4_000, 10_000, 0,
    ]);
    expect(result.rows).toContainEqual(['Bebidas', 3, '1000', '1200', '400', 4_000, 10_000, 0]);
    expect(result.rows).toContainEqual(['Café', 'Tamaño: 250g', 4, '600', 'Nunca vendido', 'No']);
  });

  it('builds inventory valuation, turnover and stock-alert sections', () => {
    const result = buildInventoryCsv({ valuation, turnover, alerts: [stockAlert] }, range, 2);
    expect(result.filename).toContain('reporte-inventario-');
    expect(result.rows).toContainEqual(['Bebidas', 4, '600,00', '1000,00', 10_000, 0]);
    expect(result.rows).toContainEqual(['COGS del período', '600,00']);
    expect(result.rows).toContainEqual([
      'Café', 'Tamaño: 250g', 'Bebidas', 'Agotado', 0, 2, 3, 0, 3,
    ]);
  });

  it('builds payables summary, aging, creditors and due sections', () => {
    const result = buildPayablesCsv({
      summary: payablesSummary,
      aging,
      creditors: [creditor],
      dues: [due],
    }, range, 2);
    expect(result.filename).toContain('reporte-obligaciones-');
    expect(result.rows).toContainEqual(['Saldo por pagar', '1000,00']);
    expect(result.rows).toContainEqual(['Vencidas 1–30 días', 1, '200,00']);
    expect(result.rows).toContainEqual(['Mantenimiento', 1, '1000,00', '200,00', 10_000]);
    expect(result.rows).toContainEqual(['Proveedor Uno', 1, '1000,00', '200,00', '2026-10-12']);
    expect(result.rows).toContainEqual([
      '2026-10-12', 'Proveedor Uno', 'Factura', 'Mantenimiento', '1000,00', 'Alerta', 1,
    ]);
  });
});
