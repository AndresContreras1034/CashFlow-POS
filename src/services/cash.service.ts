import { invoke } from '@tauri-apps/api/core';
import type {
  CashSession,
  CashSessionWithTotals,
  CashMovement,
  OpenSessionDto,
  CloseSessionDto,
  CreateMovementDto,
  CashSessionFilterDto,
  CashMovementFilterDto,
  PaginatedResponse,
} from '../types';

// OJO: las claves de nivel superior del objeto de argumentos van en camelCase
// (Tauri las traduce a snake_case del lado de Rust). El bug ya documentado
// en ProductDetail fue justo por mandar snake_case aquí.

export async function openCashSession(dto: OpenSessionDto): Promise<CashSession> {
  return invoke('open_cash_session', { dto });
}

export async function closeCashSession(
  id: number,
  dto: CloseSessionDto
): Promise<CashSession> {
  return invoke('close_cash_session', { id, dto });
}

export async function getCurrentCashSession(): Promise<CashSessionWithTotals | null> {
  return invoke('get_current_cash_session');
}

export async function getCashSession(id: number): Promise<CashSession> {
  return invoke('get_cash_session', { id });
}

export async function listCashSessions(
  filter: CashSessionFilterDto = {}
): Promise<PaginatedResponse<CashSession>> {
  return invoke('list_cash_sessions', { filter });
}

export async function registerCashMovement(
  sessionId: number,
  dto: CreateMovementDto
): Promise<CashMovement> {
  return invoke('register_cash_movement', { sessionId, dto });
}

export async function listCashMovements(
  filter: CashMovementFilterDto = {}
): Promise<PaginatedResponse<CashMovement>> {
  return invoke('list_cash_movements', { filter });
}