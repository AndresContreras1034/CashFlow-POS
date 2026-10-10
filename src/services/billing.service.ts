import { invoke } from '@tauri-apps/api/core';

export async function printSaleTicket(saleId: number, copy = false): Promise<void> {
  return invoke('print_sale_ticket', { saleId, copy });
}

export async function printCashReport(sessionId: number, copy = false): Promise<void> {
  return invoke('print_cash_report', { sessionId, copy });
}