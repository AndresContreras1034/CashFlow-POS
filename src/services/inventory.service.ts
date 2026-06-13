import { invoke } from '@tauri-apps/api/core';
import type {
  Category, CreateCategoryDto, UpdateCategoryDto,
  Product, ProductWithCategory, CreateProductDto, UpdateProductDto,
  ProductVariant, VariantWithProduct, CreateVariantDto, UpdateVariantDto,
  StockEntryDto, StockOutDto, StockAdjustmentDto,
  ProductFilterDto, KardexFilterDto,
  PaginatedResponse, MovementWithDetails, LowStockItemDto,
} from '../types';

// Categorías
export const listCategories = (): Promise<Category[]> =>
  invoke('list_categories');

export const getCategory = (id: number): Promise<Category> =>
  invoke('get_category', { id });

export const createCategory = (dto: CreateCategoryDto): Promise<Category> =>
  invoke('create_category', { dto });

export const updateCategory = (id: number, dto: UpdateCategoryDto): Promise<Category> =>
  invoke('update_category', { id, dto });

// Productos
export const listProducts = (filter: ProductFilterDto): Promise<PaginatedResponse<ProductWithCategory>> =>
  invoke('list_products', { filter });

export const getProduct = (id: number): Promise<ProductWithCategory> =>
  invoke('get_product', { id });

export const createProduct = (dto: CreateProductDto): Promise<Product> =>
  invoke('create_product', { dto });

export const updateProduct = (id: number, dto: UpdateProductDto): Promise<Product> =>
  invoke('update_product', { id, dto });

export const deactivateProduct = (id: number): Promise<Product> =>
  invoke('deactivate_product', { id });

// Variantes
export const listVariants = (productId: number): Promise<ProductVariant[]> =>
  invoke('list_variants', { product_id: productId });

export const getVariant = (id: number): Promise<ProductVariant> =>
  invoke('get_variant', { id });

export const findByBarcode = (barcode: string): Promise<VariantWithProduct> =>
  invoke('find_by_barcode', { barcode });

export const searchVariants = (query: string): Promise<VariantWithProduct[]> =>
  invoke('search_variants', { query });

export const createVariant = (dto: CreateVariantDto): Promise<ProductVariant> =>
  invoke('create_variant', { dto });

export const updateVariant = (id: number, dto: UpdateVariantDto): Promise<ProductVariant> =>
  invoke('update_variant', { id, dto });

// Movimientos de stock
export const registerStockEntry = (dto: StockEntryDto): Promise<ProductVariant> =>
  invoke('register_stock_entry', { dto });

export const registerInitialStock = (dto: StockEntryDto): Promise<ProductVariant> =>
  invoke('register_initial_stock', { dto });

export const registerManualEntry = (dto: StockEntryDto): Promise<ProductVariant> =>
  invoke('register_manual_entry', { dto });

export const registerManualOut = (dto: StockOutDto): Promise<ProductVariant> =>
  invoke('register_manual_out', { dto });

export const adjustStock = (dto: StockAdjustmentDto): Promise<ProductVariant> =>
  invoke('adjust_stock', { dto });

// Kardex
export const getKardex = (filter: KardexFilterDto): Promise<PaginatedResponse<MovementWithDetails>> =>
  invoke('get_kardex', { filter });

// Alertas
export const getLowStock = (): Promise<LowStockItemDto[]> =>
  invoke('get_low_stock');