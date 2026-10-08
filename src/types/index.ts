// ============================================================
// CATEGORÍAS
// ============================================================

export interface Category {
  id: number;
  name: string;
  description?: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateCategoryDto {
  name: string;
  description?: string;
}

export interface UpdateCategoryDto {
  name?: string;
  description?: string;
  is_active?: boolean;
}

// ============================================================
// PRODUCTOS
// ============================================================

export interface Product {
  id: number;
  category_id: number;
  name: string;
  description?: string;
  brand?: string;
  image_url?: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface ProductWithCategory extends Product {
  category_name: string;
}

export interface CreateProductDto {
  category_id: number;
  name: string;
  description?: string;
  brand?: string;
  image_url?: string;
}

export interface UpdateProductDto {
  category_id?: number;
  name?: string;
  description?: string;
  brand?: string;
  image_url?: string;
  is_active?: boolean;
}

// ============================================================
// VARIANTES
// ============================================================

export type StockStatus = 'ok' | 'low' | 'out_of_stock';

export interface ProductVariant {
  id: number;
  product_id: number;
  attributes: Record<string, string>;
  sku?: string;
  barcode?: string;
  price: number; // centavos
  cost: number;  // centavos
  stock: number;
  stock_min: number;
  allow_negative: boolean;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface VariantWithProduct extends ProductVariant {
  product_name: string;
  brand?: string;
  image_url?: string;
  category_id: number;
  category_name: string;
}

export interface CreateVariantDto {
  product_id: number;
  attributes: Record<string, string>;
  sku?: string;
  barcode?: string;
  price: number;
  cost?: number;
  stock?: number;
  stock_min?: number;
  allow_negative?: boolean;
}

export interface UpdateVariantDto {
  attributes?: Record<string, string>;
  sku?: string;
  barcode?: string;
  price?: number;
  cost?: number;
  stock_min?: number;
  allow_negative?: boolean;
  is_active?: boolean;
}

// ============================================================
// MOVIMIENTOS DE INVENTARIO (KARDEX)
// ============================================================

export type MovementType =
  | 'purchase'
  | 'sale'
  | 'sale_return'
  | 'manual_in'
  | 'manual_out'
  | 'adjustment'
  | 'initial_stock';

export type MovementReason =
  | 'damaged'
  | 'lost'
  | 'expired'
  | 'theft'
  | 'internal_use'
  | 'count_correction'
  | 'other';

export interface InventoryMovement {
  id: number;
  variant_id: number;
  movement_type: MovementType;
  quantity: number;
  stock_before: number;
  stock_after: number;
  unit_cost: number;
  sale_id?: number;
  purchase_id?: number;
  reason: MovementReason | null;
  notes?: string;
  created_by: string;
  created_at: string;
}

export interface MovementWithDetails extends InventoryMovement {
  product_name: string;
  attributes: Record<string, string>;
  sku?: string;
  barcode?: string;
}

// ============================================================
// DTOs DE STOCK
// ============================================================

export interface StockEntryDto {
  variant_id: number;
  quantity: number;
  unit_cost?: number;
  notes?: string;
  created_by?: string;
}

export interface StockOutDto {
  variant_id: number;
  quantity: number;
  reason?: MovementReason;
  notes?: string;
  created_by?: string;
}

export interface StockAdjustmentDto {
  variant_id: number;
  actual_stock: number;
  reason?: MovementReason;
  notes?: string;
  created_by?: string;
}

// ============================================================
// FILTROS
// ============================================================

export interface ProductFilterDto {
  category_id?: number;
  search?: string;
  is_active?: boolean;
  variant_is_active?: boolean;
  stock_status?: 'ok' | 'low' | 'out_of_stock';
  sort_by?: 'name' | 'brand' | 'category' | 'created_at';
  sort_dir?: 'asc' | 'desc';
  page?: number;
  page_size?: number;
}

export interface KardexFilterDto {
  variant_id?: number;
  product_id?: number;
  movement_type?: string;
  date_from?: string;
  date_to?: string;
  page?: number;
  page_size?: number;
}

// ============================================================
// RESPUESTAS
// ============================================================

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface LowStockItemDto {
  variant_id: number;
  product_name: string;
  attributes: Record<string, string>;
  barcode?: string;
  stock: number;
  stock_min: number;
  stock_status: string;
}

export interface InventoryValueDto {
  value_at_cost: number;
  potential_sale_value: number;
  variants_without_cost: number;
  negative_stock_variants: number;
  variants_valued: number;
}

export interface ProductStockStatsDto {
  product_id: number;
  variant_count: number;
  total_stock: number;
  min_stock_min: number;
}

export interface ExportSummaryDto {
  active_rows: number;
  inactive_rows: number;
  low_stock_rows: number;
}

export type LicenseKind = 'purchase' | 'rental';
export type LicenseStatusKind = 'unlicensed' | 'active' | 'expired' | 'invalid';

export interface LicenseStatusDto {
  installation_id: string;
  status: LicenseStatusKind;
  license_id: string | null;
  licensee: string | null;
  kind: LicenseKind | null;
  expires_at: string | null;
  source_code_access: boolean;
  message: string;
}

// ============================================================
// AJUSTES
// ============================================================

export interface AppSettings {
  id: number;
  business_name: string;
  tax_id: string | null;
  address: string | null;
  phone: string | null;
  email: string | null;
  currency: string;
  currency_decimals: number;
  tax_rate_bps: number;
  tax_name: string;
  timezone: string;
  ticket_header: string | null;
  ticket_footer: string | null;
  show_logo: boolean;
  show_tax_id: boolean;
  show_address: boolean;
  show_phone: boolean;
  show_cashier: boolean;
  show_tax_breakdown: boolean;
  show_discounts: boolean;
  show_payment_method: boolean;
  low_stock_default_threshold: number;
  logo_url: string | null;
  created_at: string;
  updated_at: string;
}

export interface UpdateSettingsDto {
  business_name?: string;
  // Nullable fields always replace current values; null clears the field.
  tax_id: string | null;
  address: string | null;
  phone: string | null;
  email: string | null;
  currency?: string;
  currency_decimals?: number;
  tax_rate_bps?: number;
  tax_name?: string;
  timezone?: string;
  ticket_header: string | null;
  ticket_footer: string | null;
  show_logo?: boolean;
  show_tax_id?: boolean;
  show_address?: boolean;
  show_phone?: boolean;
  show_cashier?: boolean;
  show_tax_breakdown?: boolean;
  show_discounts?: boolean;
  show_payment_method?: boolean;
  low_stock_default_threshold?: number;
  logo_url: string | null;
}
// ============================================================
// CAJA
// ============================================================

export type CashMovementType = 'manual_in' | 'manual_out' | 'sale_in' | 'sale_out';

export interface CashSession {
  id: number;
  opening_amount: number;
  expected_amount: number | null;
  counted_amount: number | null;
  difference: number | null;
  status: 'open' | 'closed';
  opened_by: string;
  closed_by: string | null;
  opening_notes: string | null;
  closing_notes: string | null;
  opened_at: string;
  closed_at: string | null;
}

export interface CashSessionWithTotals {
  id: number;
  opening_amount: number;
  status: 'open' | 'closed';
  opened_by: string;
  opening_notes: string | null;
  opened_at: string;
  total_in: number;
  total_out: number;
  current_balance: number;
}

export interface CashMovement {
  id: number;
  session_id: number;
  movement_type: CashMovementType;
  amount: number;
  sale_id: number | null;
  notes: string | null;
  created_by: string;
  created_at: string;
}

export interface OpenSessionDto {
  opening_amount: number;
  opening_notes?: string | null;
  opened_by?: string | null;
}

export interface CloseSessionDto {
  counted_amount: number;
  closing_notes?: string | null;
  closed_by?: string | null;
}

export type ManualCashMovementType = 'manual_in' | 'manual_out';

export interface CreateMovementDto {
  movement_type: ManualCashMovementType;
  amount: number;
  notes?: string | null;
  created_by?: string | null;
  sale_id?: number | null;
}

export interface CashSessionFilterDto {
  status?: string | null;
  date_from?: string | null;
  date_to?: string | null;
  page?: number;
  page_size?: number;
}

export interface CashMovementFilterDto {
  session_id?: number | null;
  movement_type?: string | null;
  page?: number;
  page_size?: number;
}

// ============================================================
// VENTAS
// ============================================================

export type PaymentMethod = 'cash' | 'card' | 'transfer';
export type SaleStatus = 'completed' | 'cancelled' | 'refunded';

export interface Sale {
  id: number;
  customer_id: number | null;
  subtotal: number;
  tax: number;
  discount: number;
  total: number;
  status: SaleStatus;
  notes: string | null;
  created_by: string;
  created_at: string;
}

export interface SaleItem {
  id: number;
  sale_id: number;
  variant_id: number;
  quantity: number;
  unit_price: number;
  discount: number;
  tax: number;
  subtotal: number;
}

export interface SalePayment {
  id: number;
  sale_id: number;
  method: PaymentMethod;
  amount: number;
}

export interface SaleDetail extends Sale {
  items: SaleItem[];
  payments: SalePayment[];
}

export interface CreateSaleItemDto {
  variant_id: number;
  quantity: number;
  discount?: number | null;
}

export interface CreateSalePaymentDto {
  method: PaymentMethod;
  amount: number;
}

export interface CreateSaleDto {
  customer_id?: number | null;
  items: CreateSaleItemDto[];
  payments: CreateSalePaymentDto[];
  discount?: number | null;
  notes?: string | null;
  created_by?: string | null;
}

export interface SaleFilterDto {
  status?: string | null;
  customer_id?: number | null;
  date_from?: string | null;
  date_to?: string | null;
  page?: number;
  page_size?: number;
}
export interface ImportRowResult {
  row_number: number;
  product_name: string;
  sku?: string;
  status: 'new_product' | 'existing_product' | 'skipped_duplicate_sku' | 'error';
  message?: string;
}

export interface ImportSummaryDto {
  new_categories: number;
  new_products: number;
  new_variants: number;
  skipped: number;
  errors: number;
  rows: ImportRowResult[];
  total_rows: number;
  rows_truncated: boolean;
  can_execute: boolean;
}

export type StocktakeStatus = 'counting' | 'applied' | 'cancelled';

export interface Stocktake {
  id: number;
  category_id: number | null;
  category_name: string | null;
  status: StocktakeStatus;
  notes: string | null;
  created_by: string;
  created_at: string;
  applied_at: string | null;
  cancelled_at: string | null;
  total_lines: number;
  counted_lines: number;
}

export interface CreateStocktakeDto {
  category_id?: number | null;
  notes?: string | null;
  created_by?: string | null;
}

export interface StocktakeLineFilterDto {
  search?: string | null;
  only_pending?: boolean;
  page?: number;
  page_size?: number;
}

/** Fila para el conteo ciego: no contiene el stock del sistema. */
export interface StocktakeCountLineDto {
  variant_id: number;
  product_name: string;
  attributes: Record<string, string>;
  sku: string | null;
  barcode: string | null;
  counted_stock: number | null;
}

export interface StocktakeReviewLineDto {
  variant_id: number;
  product_name: string;
  attributes: Record<string, string>;
  sku: string | null;
  barcode: string | null;
  expected_stock: number;
  current_stock: number;
  counted_stock: number;
  difference: number;
  moved_during_count: boolean;
}

export interface StocktakeSummaryDto {
  uncounted: number;
  matching: number;
  with_difference: number;
  surplus_units: number;
  shortage_units: number;
  moved_during_count: number;
}

// ============================================================
// AUDITORÍA
// ============================================================

export type AuditCategory = 'business' | 'error';
export type AuditOutcome = 'success' | 'failure';
export type AuditModule =
  | 'sales'
  | 'inventory'
  | 'cash'
  | 'stocktake'
  | 'settings'
  | 'import'
  | 'licensing'
  | 'billing'
  | 'system';

/** Un campo que cambió: solo aparecen los campos modificados. */
export interface AuditChange {
  from: unknown;
  to: unknown;
}

export interface AuditEvent {
  id: number;
  /** ISO-8601 UTC */
  created_at: string;
  correlation_id: string | null;
  category: AuditCategory;
  module: AuditModule;
  action: string;
  outcome: AuditOutcome;
  actor: string | null;
  entity_type: string | null;
  entity_id: string | null;
  summary: string;
  changes: Record<string, AuditChange> | null;
  metadata: Record<string, unknown> | null;
  error_message: string | null;
}

export interface AuditFilterDto {
  module?: AuditModule;
  category?: AuditCategory;
  outcome?: AuditOutcome;
  action?: string;
  entity_type?: string;
  entity_id?: string;
  correlation_id?: string;
  search?: string;
  /** 'YYYY-MM-DD', inclusivo */
  date_from?: string;
  /** 'YYYY-MM-DD', inclusivo */
  date_to?: string;
  sort_dir?: 'asc' | 'desc';
  page?: number;
  page_size?: number;
}

export interface StocktakeReviewDto {
  stocktake: Stocktake;
  summary: StocktakeSummaryDto;
  lines: StocktakeReviewLineDto[];
}

export interface StocktakeApplyResultDto {
  adjusted: number;
  unchanged: number;
  uncounted: number;
}