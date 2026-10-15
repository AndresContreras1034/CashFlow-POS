export type PurchaseListStatus = 'draft' | 'ordered' | 'received' | 'cancelled';
export type PurchaseItemSource = 'recommended' | 'manual';

export interface PurchaseList {
  id: number;
  title: string;
  notes: string | null;
  status: PurchaseListStatus;
  created_by: string;
  created_at: string;
  updated_at: string;
  ordered_at: string | null;
  received_at: string | null;
  cancelled_at: string | null;
}

export interface PurchaseListSummary {
  id: number;
  title: string;
  status: PurchaseListStatus;
  created_at: string;
  updated_at: string;
  item_count: number;
  supplier_count: number;
  estimated_cost: number;
  items_without_cost: number;
}

export interface PurchaseSupplier {
  id: number;
  name: string;
  phone: string | null;
  whatsapp: string | null;
  email: string | null;
}

export interface PurchaseItem {
  item_id: number;
  variant_id: number;
  product_id: number;
  product_name: string;
  brand: string | null;
  attributes: Record<string, string>;
  sku: string | null;
  barcode: string | null;
  category_name: string;
  stock: number;
  stock_min: number;
  stock_status: 'out_of_stock' | 'low' | 'ok';
  variant_cost: number;
  variant_price: number;
  quantity: number;
  /** cantidad × costo de la variante; null si la variante no tiene costo. */
  line_cost: number | null;
  source: PurchaseItemSource;
  suggested_qty: number | null;
  note: string | null;
  supplier: PurchaseSupplier | null;
  supplier_sku: string | null;
}

export interface PurchaseListDetail {
  list: PurchaseList;
  items: PurchaseItem[];
}

export interface AddItemsResult {
  requested: number;
  added: number;
  already_in_list: number;
}

export interface CreatePurchaseListDto {
  title: string;
  notes?: string | null;
  created_by?: string | null;
}

export interface UpdatePurchaseListDto {
  title: string;
  notes?: string | null;
  updated_by?: string | null;
}

export interface SetListStatusDto {
  status: 'draft' | 'ordered' | 'cancelled';
  updated_by?: string | null;
}

export interface AddItemsDto {
  variant_ids: number[];
  updated_by?: string | null;
}

export interface AddRecommendedDto {
  supplier_id?: number;
  variant_ids?: number[];
  only_urgent?: boolean;
  window_days?: number;
  cover_days?: number;
  updated_by?: string | null;
}

export interface UpdateItemDto {
  quantity: number;
  supplier_id: number | null;
  note: string | null;
  updated_by?: string | null;
}

export interface ReceiveLineDto {
  item_id: number;
  /** 0 = no llegó. */
  quantity: number;
  /** Costo unitario real; null = desconocido (el costo actual no cambia). */
  unit_cost: number | null;
}

export interface ReceiveListDto {
  lines: ReceiveLineDto[];
  updated_by?: string | null;
}

export interface CostChange {
  variant_id: number;
  product_name: string;
  from: number;
  to: number;
}

export interface ReceiveResult {
  list: PurchaseList;
  received_lines: number;
  skipped_lines: number;
  units: number;
  total_cost: number;
  cost_changes: CostChange[];
}

export interface PurchaseListFilterDto {
  scope?: 'open' | 'all';
  search?: string;
  page?: number;
  page_size?: number;
}

export interface PurchasableVariant {
  variant_id: number;
  product_id: number;
  product_name: string;
  brand: string | null;
  attributes: Record<string, string>;
  sku: string | null;
  barcode: string | null;
  category_name: string;
  stock: number;
  stock_min: number;
  variant_cost: number;
  in_list: boolean;
}

export interface PurchasablePage {
  data: PurchasableVariant[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface PurchasableIds {
  ids: number[];
  truncated: boolean;
}
