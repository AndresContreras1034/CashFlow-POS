import { invoke } from '@tauri-apps/api/core';
import type { AuditEvent, PaginatedResponse } from '../types';
import type {
  CreateDebtDto,
  DebtCategoryStat,
  DebtDetail,
  DebtFilterDto,
  DebtOptions,
  DebtOverview,
  DebtSummary,
  DueDebtsFilterDto,
  RegisterPaymentDto,
  UpdateDebtDto,
  VoidDebtDto,
  VoidPaymentDto,
} from '../types/debts';

// Las claves de primer nivel van en camelCase (Tauri las traduce a snake_case
// del lado de Rust); los campos dentro de `dto` y `filter` conservan
// snake_case. Los errores llegan como string, ya en español.

// ---------------- Lecturas ----------------

export const getDebtOptions = (): Promise<DebtOptions> => invoke('get_debt_options');

export const listDebts = (
  filter: DebtFilterDto = {},
): Promise<PaginatedResponse<DebtOverview>> => invoke('list_debts', { filter });

export const getDebt = (id: number): Promise<DebtDetail> => invoke('get_debt', { id });

export const getDebtSummary = (filter: DebtFilterDto = {}): Promise<DebtSummary> =>
  invoke('get_debt_summary', { filter });

export const getDebtCategoryStats = (
  filter: DebtFilterDto = {},
): Promise<DebtCategoryStat[]> => invoke('get_debt_category_stats', { filter });

/** Vencidas, que vencen hoy y próximas a vencer; la más antigua primero. */
export const listDueDebts = (
  filter: DueDebtsFilterDto = {},
): Promise<PaginatedResponse<DebtOverview>> => invoke('list_due_debts', { filter });

/** Eventos de la deuda y de sus pagos, del más reciente al más antiguo. */
export const listDebtHistory = (debtId: number): Promise<AuditEvent[]> =>
  invoke('list_debt_history', { debtId });

// ---------------- Escrituras (devuelven la deuda actualizada) ----------------

export const createDebt = (dto: CreateDebtDto): Promise<DebtDetail> =>
  invoke('create_debt', { dto });

export const updateDebt = (id: number, dto: UpdateDebtDto): Promise<DebtDetail> =>
  invoke('update_debt', { id, dto });

export const voidDebt = (id: number, dto: VoidDebtDto): Promise<DebtDetail> =>
  invoke('void_debt', { id, dto });

export const registerDebtPayment = (
  debtId: number,
  dto: RegisterPaymentDto,
): Promise<DebtDetail> => invoke('register_debt_payment', { debtId, dto });

export const voidDebtPayment = (
  paymentId: number,
  dto: VoidPaymentDto,
): Promise<DebtDetail> => invoke('void_debt_payment', { paymentId, dto });
