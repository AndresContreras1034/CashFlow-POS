import type { PaymentMethod } from './index';

export type Granularity = 'day' | 'week' | 'month';

export interface ReportFilter {
  date_from: string;
  date_to: string;
  granularity?: Granularity | null;
  compare_previous?: boolean;
  category_id?: number | null;
  limit?: number | null;
}

export interface DateRange {
  from: string;
  to: string;
}

export interface Delta {
  current: number;
  previous: number | null;
  change_bps: number | null;
}

export interface SalesKpis {
  tickets: number;
  gross_sales: number;
  discount_total: number;
  tax_total: number;
  net_sales: number;
  avg_ticket: number;
  cogs: number;
  gross_profit: number;
  margin_bps: number;
  units_sold: number;
  items_without_cost: number;
  cancelled_count: number;
  refunded_count: number;
  courtesy_count: number;
  courtesy_cost: number;
  uncosted_tickets: number;
  profit_base_net_sales: number;
  courtesy_value: number;
}

export interface DashboardKpis {
  range: DateRange;
  previous_range: DateRange | null;
  current: SalesKpis;
  previous: SalesKpis | null;
  gross_sales_delta: Delta;
  gross_profit_delta: Delta;
  tickets_delta: Delta;
  avg_ticket_delta: Delta;
}

export interface TimeSeriesPoint {
  bucket: string;
  tickets: number;
  gross_sales: number;
  gross_profit: number;
}

export interface PaymentMethodStat {
  method: PaymentMethod;
  tickets: number;
  amount: number;
  share_bps: number;
}

export interface HeatmapCell {
  weekday: number;
  hour: number;
  tickets: number;
  gross_sales: number;
}

export interface TopProduct {
  product_id: number;
  variant_id: number;
  product_name: string;
  attributes: unknown;
  units: number;
  revenue: number;
  profit: number;
  share_bps: number;
}

export interface CategorySales {
  category_id: number;
  category_name: string;
  units: number;
  revenue: number;
  profit: number;
  share_bps: number;
}

export interface StaleProduct {
  variant_id: number;
  product_name: string;
  attributes: unknown;
  stock: number;
  stock_value_at_cost: number;
  days_since_last_sale: number | null;
}
