import { invoke } from '@tauri-apps/api/core';
import type {
  Sale,
  SaleDetail,
  CreateSaleDto,
  SaleFilterDto,
  PaginatedResponse,
} from '../types';

export async function createSale(dto: CreateSaleDto): Promise<SaleDetail> {
  return invoke('create_sale', { dto });
}

export async function getSale(id: number): Promise<SaleDetail> {
  return invoke('get_sale', { id });
}

export async function listSales(
  filter: SaleFilterDto = {}
): Promise<PaginatedResponse<Sale>> {
  return invoke('list_sales', { filter });
}