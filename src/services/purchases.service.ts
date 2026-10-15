import { invoke } from '@tauri-apps/api/core';
import type { PaginatedResponse } from '../types';
import type { LinkableFilterDto } from '../types/suppliers';
import type {
  AddItemsDto,
  AddItemsResult,
  AddRecommendedDto,
  CreatePurchaseListDto,
  PurchasableIds,
  PurchasablePage,
  PurchaseItem,
  PurchaseList,
  PurchaseListDetail,
  PurchaseListFilterDto,
  PurchaseListSummary,
  SetListStatusDto,
  UpdateItemDto,
  UpdatePurchaseListDto,
} from '../types/purchases';

export const listPurchaseLists = (
  filter: PurchaseListFilterDto = {},
): Promise<PaginatedResponse<PurchaseListSummary>> =>
  invoke('list_purchase_lists', { filter });

export const getPurchaseList = (id: number): Promise<PurchaseListDetail> =>
  invoke('get_purchase_list', { id });

export const createPurchaseList = (dto: CreatePurchaseListDto): Promise<PurchaseList> =>
  invoke('create_purchase_list', { dto });

export const updatePurchaseList = (
  id: number,
  dto: UpdatePurchaseListDto,
): Promise<PurchaseList> => invoke('update_purchase_list', { id, dto });

export const setPurchaseListStatus = (
  id: number,
  dto: SetListStatusDto,
): Promise<PurchaseList> => invoke('set_purchase_list_status', { id, dto });

export const addPurchaseItems = (listId: number, dto: AddItemsDto): Promise<AddItemsResult> =>
  invoke('add_purchase_items', { listId, dto });

export const addRecommendedToList = (
  listId: number,
  dto: AddRecommendedDto,
): Promise<AddItemsResult> => invoke('add_recommended_to_list', { listId, dto });

export const updatePurchaseItem = (
  itemId: number,
  dto: UpdateItemDto,
): Promise<PurchaseItem> => invoke('update_purchase_item', { itemId, dto });

export const removePurchaseItem = (itemId: number): Promise<void> =>
  invoke('remove_purchase_item', { itemId });

export const searchPurchasableVariants = (
  listId: number,
  filter: LinkableFilterDto,
): Promise<PurchasablePage> => invoke('search_purchasable_variants', { listId, filter });

export const listPurchasableVariantIds = (
  listId: number,
  filter: LinkableFilterDto,
): Promise<PurchasableIds> => invoke('list_purchasable_variant_ids', { listId, filter });
