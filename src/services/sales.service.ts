import { invoke } from '@tauri-apps/api/core';
import type {
  SaleDetail,
  CreateSaleDto,
  SaleFilterDto,
  SaleListResponse,
  VoidSaleDto,
} from '../types';

export async function createSale(dto: CreateSaleDto): Promise<SaleDetail> {
  return invoke('create_sale', { dto });
}

export async function getSale(id: number): Promise<SaleDetail> {
  return invoke('get_sale', { id });
}

export async function listSales(
  filter: SaleFilterDto = {}
): Promise<SaleListResponse> {
  return invoke('list_sales', { filter });
}

export async function voidSale(id: number, dto: VoidSaleDto): Promise<SaleDetail> {
  return invoke('void_sale', { id, dto });
}