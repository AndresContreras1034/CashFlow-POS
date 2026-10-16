import type { PaymentMethod } from './index';

export type Granularity = 'day' | 'week' | 'month';
export type RankBy = 'units' | 'revenue' | 'profit';

export interface ReportFilter {
  date_from: string;
  date_to: string;
  granularity?: Granularity | null;
  compare_previous?: boolean;
  compare_year_over_year?: boolean;
  category_id?: number | null;
  limit?: number | null;
  rank_by?: RankBy | null;
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
  partial: boolean;
  tickets: number;
  gross_sales: number;
  gross_profit: number;
}

export interface PaymentMethodStat {
  method: PaymentMethod;
  /** Non-additive: a mixed-payment sale is counted once for each method it uses. */
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
  gross_revenue: number;
  revenue: number;
  profit: number;
  margin_bps: number | null;
  uncosted_units: number;
  share_bps: number;
}

export interface CategorySales {
  category_id: number;
  category_name: string;
  units: number;
  gross_revenue: number;
  revenue: number;
  profit: number;
  margin_bps: number | null;
  uncosted_units: number;
  share_bps: number;
}

export interface StaleProduct {
  variant_id: number;
  product_name: string;
  attributes: unknown;
  stock: number;
  stock_value_at_cost: number;
  cost_missing: boolean;
  days_since_last_sale: number | null;
  total_value_at_cost: number;
  total_rows: number;
  total_without_cost: number;
}

export interface CategoryValuation {
  category_id: number;
  category_name: string;
  units: number;
  value_at_cost: number;
  potential_sale_value: number;
  share_bps: number;
  variants_without_cost: number;
}

export interface InventoryValuation {
  value_at_cost: number;
  potential_sale_value: number;
  potential_margin_bps: number | null;
  variants_valued: number;
  variants_without_cost: number;
  negative_stock_variants: number;
  units_in_stock: number;
  by_category: CategoryValuation[];
}

export interface InventoryTurnover {
  cogs: number;
  inventory_value_at_cost: number;
  turnover_bps: number | null;
  days_of_inventory: number | null;
  days_of_inventory_reason: InventoryDaysReason | null;
  range_days: number;
  uncosted_items: number;
}

export type InventoryDaysReason =
  | 'range_too_short'
  | 'no_cogs'
  | 'low_cost_coverage'
  | 'implausible_result';

export type StockStatus = 'low' | 'out_of_stock';

export interface StockAlert {
  variant_id: number;
  product_name: string;
  attributes: unknown;
  category_name: string;
  stock: number;
  stock_min: number;
  status: StockStatus;
  units_sold: number;
  days_of_cover: number | null;
  suggested_reorder: number;
}

export interface PayablesSummary {
  pending_total: number;
  overdue_total: number;
  due_soon_total: number;
  active_count: number;
  overdue_count: number;
  due_soon_count: number;
  paid_in_range: number;
  payments_in_range_count: number;
  issued_in_range: number;
  by_category: PayablesByCategory[];
  by_urgency: UrgencyBucket[];
}

export type DebtCategory =
  | 'maintenance'
  | 'business_operation'
  | 'fixed_services'
  | 'other_obligations';

export type DebtUrgency = 'none' | 'ok' | 'alert' | 'due_today' | 'overdue';

export interface PayablesByCategory {
  category: DebtCategory;
  count: number;
  balance_total: number;
  overdue_total: number;
  share_bps: number;
}

export interface UrgencyBucket {
  urgency: DebtUrgency;
  count: number;
  balance_total: number;
}

export type AgingLabel =
  | 'not_due'
  | 'overdue_1_30'
  | 'overdue_31_60'
  | 'overdue_61_90'
  | 'overdue_90_plus'
  | 'no_due_date';

export interface AgingBucket {
  label: AgingLabel;
  count: number;
  balance_total: number;
}

export interface TopCreditor {
  creditor_name: string;
  active_count: number;
  balance_total: number;
  overdue_total: number;
  next_due_on: string | null;
}

export interface UpcomingDue {
  debt_id: number;
  creditor_name: string;
  concept: string;
  category: DebtCategory;
  due_on: string;
  balance: number;
  days_until_due: number;
  urgency: DebtUrgency;
}
