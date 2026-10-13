export type DebtCategory =
  | 'maintenance'
  | 'business_operation'
  | 'fixed_services'
  | 'other_obligations';
export type DebtNature = 'operating' | 'financial_loan' | 'tax' | 'other';
export type DebtPaymentMethod = 'cash' | 'card' | 'transfer' | 'check' | 'other';
export type DebtStatus = 'voided' | 'paid' | 'overdue' | 'partially_paid' | 'pending';
export type DebtUrgency = 'none' | 'ok' | 'alert' | 'due_today' | 'overdue';

export interface LabeledOption<T extends string = string> {
  value: T;
  label: string;
}

export interface CategoryOption extends LabeledOption<DebtCategory> {
  description: string;
  subcategories: string[];
}

export interface DebtOptions {
  categories: CategoryOption[];
  natures: LabeledOption<DebtNature>[];
  payment_methods: LabeledOption<DebtPaymentMethod>[];
  statuses: LabeledOption<DebtStatus>[];
}

export interface DebtOverview {
  id: number;
  creditor_name: string;
  category: DebtCategory;
  subcategory: string | null;
  nature: DebtNature;
  concept: string;
  original_amount: number;
  issued_on: string;
  due_on: string | null;
  document_ref: string | null;
  notes: string | null;
  voided_at: string | null;
  voided_by: string | null;
  void_reason: string | null;
  created_by: string;
  created_at: string;
  updated_at: string;
  paid_amount: number;
  balance: number;
  payment_count: number;
  last_paid_on: string | null;
  today: string;
  days_until_due: number | null;
  status: DebtStatus;
  urgency: DebtUrgency;
}

export interface DebtPayment {
  id: number;
  debt_id: number;
  amount: number;
  paid_on: string;
  method: DebtPaymentMethod;
  reference: string | null;
  notes: string | null;
  idempotency_key: string;
  voided_at: string | null;
  voided_by: string | null;
  void_reason: string | null;
  created_by: string;
  created_at: string;
}

export interface DebtDetail extends DebtOverview {
  payments: DebtPayment[];
}

export interface DebtSummary {
  pending_total: number;
  overdue_total: number;
  due_soon_total: number;
  active_count: number;
  overdue_count: number;
  due_soon_count: number;
}

export interface DebtCategoryStat {
  category: DebtCategory;
  status: DebtStatus;
  count: number;
  original_total: number;
  paid_total: number;
  balance_total: number;
}

export type DebtSortBy = 'amount' | 'issued_on' | 'created_at' | 'due_on' | 'balance';
export type DebtSortDir = 'asc' | 'desc';

export interface DebtFilterDto {
  search?: string;
  category?: DebtCategory;
  nature?: DebtNature;
  status?: DebtStatus;
  urgencies?: DebtUrgency[];
  issued_from?: string;
  issued_to?: string;
  due_from?: string;
  due_to?: string;
  paid_from?: string;
  paid_to?: string;
  sort_by?: DebtSortBy;
  sort_dir?: DebtSortDir;
  page?: number;
  page_size?: number;
}

export interface DueDebtsFilterDto {
  page?: number;
  page_size?: number;
}

export interface DebtFields {
  creditor_name: string;
  category: DebtCategory;
  subcategory: string | null;
  nature: DebtNature;
  concept: string;
  original_amount: number;
  issued_on: string;
  due_on: string | null;
  document_ref: string | null;
  notes: string | null;
}

export interface PaymentFields {
  amount: number;
  paid_on: string;
  method: DebtPaymentMethod;
  reference: string | null;
  notes: string | null;
}

export interface CreateDebtDto extends DebtFields {
  idempotency_key: string;
  created_by?: string | null;
}

export interface UpdateDebtDto extends DebtFields {
  updated_by?: string | null;
}

export interface RegisterPaymentDto extends PaymentFields {
  idempotency_key: string;
  created_by?: string | null;
}

export interface VoidDebtDto {
  reason: string;
  voided_by?: string | null;
}

export interface VoidPaymentDto {
  reason: string;
  voided_by?: string | null;
}