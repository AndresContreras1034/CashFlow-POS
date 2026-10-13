import { invoke } from '@tauri-apps/api/core';
import type { PaginatedResponse } from '../types';
import type {
  CreateCustomerDto,
  Customer,
  CustomerFilterDto,
  CustomerOverview,
  SetCustomerActiveDto,
  UpdateCustomerDto,
} from '../types/customers';

export const listCustomers = (
  filter: CustomerFilterDto = {},
): Promise<PaginatedResponse<CustomerOverview>> => invoke('list_customers', { filter });

export const getCustomer = (id: number): Promise<CustomerOverview> =>
  invoke('get_customer', { id });

export const getCustomerNames = (
  ids: number[],
): Promise<{ id: number; name: string }[]> => invoke('get_customer_names', { ids });

export const createCustomer = (dto: CreateCustomerDto): Promise<Customer> =>
  invoke('create_customer', { dto });

export const updateCustomer = (id: number, dto: UpdateCustomerDto): Promise<Customer> =>
  invoke('update_customer', { id, dto });

export const setCustomerActive = (
  id: number,
  dto: SetCustomerActiveDto,
): Promise<Customer> => invoke('set_customer_active', { id, dto });
