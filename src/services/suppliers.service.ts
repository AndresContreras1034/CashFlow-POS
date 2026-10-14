import { invoke } from '@tauri-apps/api/core';
import type { PaginatedResponse } from '../types';
import type {
  CreateSupplierDto,
  LinkSupplierVariantDto,
  SetSupplierCategoriesDto,
  SetSupplierActiveDto,
  Supplier,
  SupplierCategory,
  SupplierFilterDto,
  SupplierOverview,
  SupplierVariantFilterDto,
  SupplierVariantLink,
  SupplierVariantView,
  UnlinkSupplierVariantDto,
  UpdateSupplierDto,
  UpdateSupplierVariantDto,
  VariantSupplier,
} from '../types/suppliers';

export const listSuppliers = (
  filter: SupplierFilterDto = {},
): Promise<PaginatedResponse<SupplierOverview>> => invoke('list_suppliers', { filter });

export const getSupplier = (id: number): Promise<SupplierOverview> =>
  invoke('get_supplier', { id });

export const createSupplier = (dto: CreateSupplierDto): Promise<Supplier> =>
  invoke('create_supplier', { dto });

export const updateSupplier = (id: number, dto: UpdateSupplierDto): Promise<Supplier> =>
  invoke('update_supplier', { id, dto });

export const setSupplierActive = (
  id: number,
  dto: SetSupplierActiveDto,
): Promise<Supplier> => invoke('set_supplier_active', { id, dto });

export const setSupplierCategories = (
  supplierId: number,
  categoryIds: number[],
  updatedBy?: string | null,
): Promise<void> => {
  const dto: SetSupplierCategoriesDto = { category_ids: categoryIds, updated_by: updatedBy };
  return invoke('set_supplier_categories', { supplierId, dto });
};

export const listSupplierCategories = (supplierId: number): Promise<SupplierCategory[]> =>
  invoke('list_supplier_categories', { supplierId });

export const linkSupplierVariant = (
  supplierId: number,
  dto: LinkSupplierVariantDto,
): Promise<SupplierVariantLink> => invoke('link_supplier_variant', { supplierId, dto });

export const updateSupplierVariant = (
  linkId: number,
  dto: UpdateSupplierVariantDto,
): Promise<SupplierVariantLink> => invoke('update_supplier_variant', { linkId, dto });

export const unlinkSupplierVariant = (
  linkId: number,
  dto: UnlinkSupplierVariantDto,
): Promise<void> => invoke('unlink_supplier_variant', { linkId, dto });

export const listSupplierVariants = (
  supplierId: number,
  filter: SupplierVariantFilterDto = {},
): Promise<PaginatedResponse<SupplierVariantView>> =>
  invoke('list_supplier_variants', { supplierId, filter });

export const listVariantSuppliers = (variantId: number): Promise<VariantSupplier[]> =>
  invoke('list_variant_suppliers', { variantId });
