export type SupplierPriority = 'normal' | 'preferred' | 'urgent';

export const SUPPLIER_PRIORITIES: { value: SupplierPriority; label: string }[] = [
  { value: 'normal', label: 'Normal' },
  { value: 'preferred', label: 'Preferido' },
  { value: 'urgent', label: 'Urgente' },
];

export type SupplierDocumentType = 'NIT' | 'CC' | 'CE' | 'OTRO';

export const SUPPLIER_DOCUMENT_TYPES: SupplierDocumentType[] = ['NIT', 'CC', 'CE', 'OTRO'];

export interface Supplier {
  id: number;
  name: string;
  contact_name: string | null;
  document_type: SupplierDocumentType | null;
  document_number: string | null;
  phone: string | null;
  whatsapp: string | null;
  email: string | null;
  address: string | null;
  city: string | null;
  /** Plazo de pago acordado en días (0 = de contado). */
  payment_terms_days: number | null;
  /** Días que tarda en entregar un pedido. */
  lead_time_days: number | null;
  /** Texto libre, p. ej. «Lunes y jueves». */
  visit_days: string | null;
  priority: SupplierPriority;
  notes: string | null;
  is_active: boolean;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface SupplierOverview extends Supplier {
  category_count: number;
  variant_count: number;
  low_stock_count: number;
  out_of_stock_count: number;
}

export interface SupplierCategory {
  category_id: number;
  category_name: string;
}

export interface SupplierVariantLink {
  id: number;
  supplier_id: number;
  variant_id: number;
  supplier_sku: string | null;
  last_cost: number | null;
  is_preferred: boolean;
  last_purchase_at: string | null;
  notes: string | null;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface SupplierVariantView extends SupplierVariantLink {
  product_name: string;
  attributes: Record<string, string>;
  sku: string | null;
  barcode: string | null;
  category_id: number;
  category_name: string;
  stock: number;
  stock_min: number;
  stock_status: 'ok' | 'low' | 'out_of_stock';
  variant_cost: number;
}

export interface VariantSupplier {
  id: number;
  supplier_id: number;
  supplier_name: string;
  phone: string | null;
  whatsapp: string | null;
  supplier_sku: string | null;
  last_cost: number | null;
  last_purchase_at: string | null;
  is_preferred: boolean;
}

export interface SupplierFields {
  name: string;
  contact_name: string | null;
  document_type: SupplierDocumentType | null;
  document_number: string | null;
  phone: string | null;
  whatsapp: string | null;
  email: string | null;
  address: string | null;
  city: string | null;
  payment_terms_days: number | null;
  lead_time_days: number | null;
  visit_days: string | null;
  priority: SupplierPriority;
  notes: string | null;
}

export interface CreateSupplierDto extends SupplierFields {
  created_by?: string | null;
}

export interface UpdateSupplierDto extends SupplierFields {
  updated_by?: string | null;
}

export interface SetSupplierActiveDto {
  is_active: boolean;
  updated_by?: string | null;
}

export interface SupplierFilterDto {
  search?: string;
  is_active?: boolean;
  priority?: SupplierPriority;
  page?: number;
  page_size?: number;
}

export interface SetSupplierCategoriesDto {
  category_ids: number[];
  updated_by?: string | null;
}

export interface LinkSupplierVariantDto {
  variant_id: number;
  supplier_sku?: string | null;
  last_cost?: number | null;
  is_preferred: boolean;
  notes?: string | null;
  created_by?: string | null;
}

export interface UpdateSupplierVariantDto {
  supplier_sku?: string | null;
  last_cost?: number | null;
  is_preferred: boolean;
  notes?: string | null;
  updated_by?: string | null;
}

export interface UnlinkSupplierVariantDto {
  updated_by?: string | null;
}

export interface SupplierVariantFilterDto {
  search?: string;
  only_preferred?: boolean;
  category_id?: number;
  include_inactive?: boolean;
  page?: number;
  page_size?: number;
}
