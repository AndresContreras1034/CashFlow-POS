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
  notes?: string;
  created_by?: string;
}

export interface StockAdjustmentDto {
  variant_id: number;
  actual_stock: number;
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