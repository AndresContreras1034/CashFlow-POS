import { invoke } from '@tauri-apps/api/core';
import type {
  DashboardKpis,
  HeatmapCell,
  PaymentMethodStat,
  ReportFilter,
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