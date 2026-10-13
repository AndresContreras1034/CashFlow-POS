export type DocumentType = 'CC' | 'NIT' | 'CE' | 'TI' | 'PP' | 'OTRO';

export const DOCUMENT_TYPES: DocumentType[] = ['CC', 'NIT', 'CE', 'TI', 'PP', 'OTRO'];

export interface Customer {
  id: number;
  name: string;
  document_type: DocumentType | null;
  document_number: string | null;
  phone: string | null;
  email: string | null;
  address: string | null;
  notes: string | null;
  is_active: boolean;
  created_by: string;
  created_at: string;
  updated_at: string;
}

/** Cliente con estadísticas de compra (solo ventas completadas). */
export interface CustomerOverview extends Customer {
  sales_count: number;
  total_spent: number;
  last_purchase_at: string | null;
}

export interface CustomerFields {
  name: string;
  document_type: DocumentType | null;
  document_number: string | null;
  phone: string | null;
  email: string | null;
  address: string | null;
  notes: string | null;
}

export interface CreateCustomerDto extends CustomerFields {
  created_by?: string | null;
}

export interface UpdateCustomerDto extends CustomerFields {
  updated_by?: string | null;
}

export interface SetCustomerActiveDto {
  is_active: boolean;
  updated_by?: string | null;
}

export interface CustomerFilterDto {
  search?: string;
  is_active?: boolean;
  page?: number;
  page_size?: number;
}
