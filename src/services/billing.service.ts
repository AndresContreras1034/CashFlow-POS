import { invoke } from '@tauri-apps/api/core';

export async function printSaleTicket(saleId: number): Promise<void> {
  return invoke('print_sale_ticket', { saleId });
}