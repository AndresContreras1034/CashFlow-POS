import { invoke } from '@tauri-apps/api/core';
import type {
  Stocktake,
  CreateStocktakeDto,
  StocktakeLineFilterDto,
  StocktakeCountLineDto,
  StocktakeReviewDto,
  StocktakeApplyResultDto,
  PaginatedResponse,
} from '../types';

export const startStocktake = (dto: CreateStocktakeDto): Promise<Stocktake> =>
  invoke('start_stocktake', { dto });

export const getCurrentStocktake = (): Promise<Stocktake | null> =>
  invoke('get_current_stocktake');

export const listStocktakes = (): Promise<Stocktake[]> =>
  invoke('list_stocktakes');

export const listStocktakeLines = (
  stocktakeId: number,
  filter: StocktakeLineFilterDto
): Promise<PaginatedResponse<StocktakeCountLineDto>> =>
  invoke('list_stocktake_lines', { stocktakeId, filter });

export const findStocktakeLineByCode = (
  stocktakeId: number,
  code: string
): Promise<StocktakeCountLineDto[]> =>
  invoke('find_stocktake_line_by_code', { stocktakeId, code });

export const setStocktakeCount = (
  stocktakeId: number,
  variantId: number,
  countedStock: number | null
): Promise<void> =>
  invoke('set_stocktake_count', { stocktakeId, variantId, countedStock });

export const getStocktakeReview = (stocktakeId: number): Promise<StocktakeReviewDto> =>
  invoke('get_stocktake_review', { stocktakeId });

export const applyStocktake = (
  stocktakeId: number,
  createdBy?: string
): Promise<StocktakeApplyResultDto> =>
  invoke('apply_stocktake', { stocktakeId, createdBy: createdBy ?? null });

export const cancelStocktake = (stocktakeId: number): Promise<void> =>
  invoke('cancel_stocktake', { stocktakeId });
