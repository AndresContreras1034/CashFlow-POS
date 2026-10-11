import { invoke } from '@tauri-apps/api/core';
import type {
  DashboardKpis,
  HeatmapCell,
  InventoryTurnover,
  InventoryValuation,
  AgingBucket,
  PaymentMethodStat,
  PayablesSummary,
  TopCreditor,
  UpcomingDue,
  ReportFilter,
  CategorySales,
  StaleProduct,
  StockAlert,
  TopProduct,
  TimeSeriesPoint,
} from '../types/reports';

export function getDashboardKpis(filter: ReportFilter): Promise<DashboardKpis> {
  return invoke('get_dashboard_kpis', { filter });
}

export function getSalesSeries(filter: ReportFilter): Promise<TimeSeriesPoint[]> {
  return invoke('get_sales_series', { filter });
}

export function getSalesByPaymentMethod(
  filter: ReportFilter,
): Promise<PaymentMethodStat[]> {
  return invoke('get_sales_by_payment_method', { filter });
}

export function getSalesHeatmap(filter: ReportFilter): Promise<HeatmapCell[]> {
  return invoke('get_sales_heatmap', { filter });
}

export function getTopProducts(filter: ReportFilter): Promise<TopProduct[]> {
  return invoke('get_top_products', { filter });
}

export function getSalesByCategory(filter: ReportFilter): Promise<CategorySales[]> {
  return invoke('get_sales_by_category', { filter });
}

export function getStaleProducts(filter: ReportFilter): Promise<StaleProduct[]> {
  return invoke('get_stale_products', { filter });
}

export function getInventoryValuation(filter: ReportFilter): Promise<InventoryValuation> {
  return invoke('get_inventory_valuation', { filter });
}

export function getInventoryTurnover(filter: ReportFilter): Promise<InventoryTurnover> {
  return invoke('get_inventory_turnover', { filter });
}

export function getStockAlerts(filter: ReportFilter): Promise<StockAlert[]> {
  return invoke('get_stock_alerts', { filter });
}

export function getPayablesSummary(filter: ReportFilter): Promise<PayablesSummary> {
  return invoke('get_payables_summary', { filter });
}

export function getPayablesAging(filter: ReportFilter): Promise<AgingBucket[]> {
  return invoke('get_payables_aging', { filter });
}

export function getTopCreditors(filter: ReportFilter): Promise<TopCreditor[]> {
  return invoke('get_top_creditors', { filter });
}

export function getUpcomingDues(filter: ReportFilter): Promise<UpcomingDue[]> {
  return invoke('get_upcoming_dues', { filter });
}

export function saveReportCsv(filePath: string, contents: string): Promise<void> {
  return invoke('save_report_csv', { filePath, contents });
}
