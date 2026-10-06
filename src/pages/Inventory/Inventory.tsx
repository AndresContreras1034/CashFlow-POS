import React, { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { save } from '@tauri-apps/plugin-dialog';
import { Modal } from '../../components/ui/Modal';
import { ToastContainer, ToastData } from '../../components/ui/Toast';
import { StockBadge } from '../../components/inventory/StockBadge';
import { ProductForm } from '../../components/inventory/ProductForm';
import { ImportInventoryModal } from '../../components/inventory/ImportInventoryModal';
import {
  Category,
  ProductWithCategory,
  CreateProductDto,
  UpdateProductDto,
  LowStockItemDto,
  InventoryValueDto,
  ProductFilterDto,
} from '../../types';
import { formatAttributes } from '../../utils/format';
import { StockSummaryCards } from '../../components/inventory/StockSummaryCards';
import {
  listCategories,
  listProducts,
  getLowStock,
  getInventoryValue,
  getProductStockStats,
  exportInventory,
  createProduct,
  updateProduct,
} from '../../services/inventory.service';
import './inventory.css';

type VariantStats = { count: number; total: number; min: number };

const DEFAULT_PAGE_SIZE = 20;
type StockFilter = '' | 'ok' | 'low' | 'out_of_stock';
type SortOption =
  | 'name:asc' | 'name:desc'
  | 'brand:asc'
  | 'category:asc'
  | 'created_at:desc' | 'created_at:asc';

const SORT_OPTIONS: { value: SortOption; label: string }[] = [
  { value: 'name:asc', label: 'Nombre (A–Z)' },
  { value: 'name:desc', label: 'Nombre (Z–A)' },
  { value: 'brand:asc', label: 'Marca (A–Z)' },
  { value: 'category:asc', label: 'Categoría (A–Z)' },
  { value: 'created_at:desc', label: 'Más recientes' },
  { value: 'created_at:asc', label: 'Más antiguos' },
];

const STOCK_FILTER_LABELS: Record<StockFilter, string> = {
  '': 'Todo el stock',
  ok: 'Stock normal',
  low: 'Stock bajo',
  out_of_stock: 'Agotados',
};

export const Inventory: React.FC = () => {
  const navigate = useNavigate();

  const [products, setProducts] = useState<ProductWithCategory[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [variantStats, setVariantStats] = useState<Record<number, VariantStats>>({});
  const [lowStock, setLowStock] = useState<LowStockItemDto[]>([]);
  const [inventoryValue, setInventoryValue] = useState<InventoryValueDto | null>(null);
  const [loading, setLoading] = useState(true);
  const [toasts, setToasts] = useState<ToastData[]>([]);

  const [search, setSearch] = useState('');
  const [debouncedSearch, setDebouncedSearch] = useState('');
  const [filterCat, setFilterCat] = useState<number | ''>('');
  const [filterActive, setFilterActive] = useState<'all' | 'active' | 'inactive'>('active');
  const [filterVariantActive, setFilterVariantActive] = useState<'all' | 'active' | 'inactive'>('all');
  const [filterStock, setFilterStock] = useState<StockFilter>('');
  const [sort, setSort] = useState<SortOption>('name:asc');

  // Antes esto se filtraba en React sobre solo los primeros 20 productos.
  // Ahora se manda al backend y PostgreSQL filtra sobre TODO el inventario.
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(DEFAULT_PAGE_SIZE);
  const [total, setTotal] = useState(0);
  const [totalPages, setTotalPages] = useState(0);

  const [showCreate, setShowCreate] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [editProduct, setEditProduct] = useState<ProductWithCategory | null>(null);
  const [saving, setSaving] = useState(false);
  const [exporting, setExporting] = useState(false);

  const addToast = useCallback((message: string, type: ToastData['type']) => {
    setToasts(t => [...t, { id: Math.random().toString(36).slice(2), message, type }]);
  }, []);

  const dismissToast = useCallback((id: string) => {
    setToasts(t => t.filter(x => x.id !== id));
  }, []);

  // Debounce simple: espera 300ms de silencio antes de buscar en el backend.
  useEffect(() => {
    const timer = window.setTimeout(() => setDebouncedSearch(search.trim()), 300);
    return () => window.clearTimeout(timer);
  }, [search]);

  // Si cambia cualquier filtro, volvemos a página 1.
  useEffect(() => {
    setPage(1);
  }, [debouncedSearch, filterCat, filterActive, filterVariantActive, filterStock, sort, pageSize]);

  const refreshVariantStats = useCallback(async (productList: ProductWithCategory[]) => {
    if (productList.length === 0) {
      setVariantStats({});
      return;
    }
    try {
      const stats = await getProductStockStats(productList.map(product => product.id));
      const byProduct: Record<number, VariantStats> = {};
      for (const stat of stats) {
        byProduct[stat.product_id] = {
          count: stat.variant_count,
          total: stat.total_stock,
          min: stat.min_stock_min,
        };
      }
      for (const product of productList) {
        byProduct[product.id] ??= { count: 0, total: 0, min: 0 };
      }
      setVariantStats(byProduct);
    } catch (error) {
      addToast('No se pudo obtener el inventario de variantes', 'error');
    }
  }, [addToast]);

  const loadProducts = useCallback(async () => {
    try {
      setLoading(true);
      const [sortBy, sortDir] = sort.split(':') as [
        NonNullable<ProductFilterDto['sort_by']>,
        NonNullable<ProductFilterDto['sort_dir']>,
      ];
      const res = await listProducts({
        search: debouncedSearch || undefined,
        category_id: filterCat === '' ? undefined : filterCat,
        is_active: filterActive === 'all' ? undefined : filterActive === 'active',
        variant_is_active: filterVariantActive === 'all' ? undefined : filterVariantActive === 'active',
        stock_status: filterStock || undefined,
        sort_by: sortBy,
        sort_dir: sortDir,
        page,
        page_size: pageSize,
      });
      setProducts(res.data);
      setTotal(res.total);
      setTotalPages(res.total_pages);
      await refreshVariantStats(res.data);
    } catch (error) {
      addToast('Error cargando los productos del inventario', 'error');
    } finally {
      setLoading(false);
    }
  }, [debouncedSearch, filterCat, filterActive, filterVariantActive, filterStock, sort, page, pageSize, refreshVariantStats, addToast]);

  const loadCategories = useCallback(async () => {
    try {
      setCategories(await listCategories());
    } catch (error) {
      addToast('No se pudieron cargar las categorías', 'error');
    }
  }, [addToast]);

  const loadLowStock = useCallback(async () => {
    try {
      setLowStock(await getLowStock());
    } catch (error) {
      addToast('No se pudo cargar el stock bajo', 'error');
    }
  }, [addToast]);

  const loadInventoryValue = useCallback(async () => {
    try {
      setInventoryValue(await getInventoryValue());
    } catch (error) {
      addToast('No se pudo calcular el valor del inventario', 'error');
    }
  }, [addToast]);

  // Para recargar todo después de importar un Excel, por ejemplo.
  const loadAll = useCallback(async () => {
    setPage(1);
    await Promise.all([loadCategories(), loadLowStock(), loadInventoryValue(), loadProducts()]);
  }, [loadCategories, loadLowStock, loadInventoryValue, loadProducts]);

  useEffect(() => {
    loadCategories();
    loadLowStock();
    loadInventoryValue();
  }, [loadCategories, loadLowStock, loadInventoryValue]);

  useEffect(() => {
    loadProducts();
  }, [loadProducts]);

  const handleCreate = async (dto: CreateProductDto | UpdateProductDto) => {
    setSaving(true);
    try {
      await createProduct(dto as CreateProductDto);
      setShowCreate(false);
      addToast('Producto creado correctamente', 'success');
      await loadProducts(); // recargamos desde el backend para mantener paginación y total correctos
    } catch (error) {
      addToast('No se pudo crear el producto', 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleEdit = async (dto: CreateProductDto | UpdateProductDto) => {
    if (!editProduct) return;
    setSaving(true);
    try {
      await updateProduct(editProduct.id, dto as UpdateProductDto);
      setEditProduct(null);
      addToast('Producto actualizado', 'success');
      await loadProducts();
    } catch (error) {
      addToast('No se pudo actualizar el producto', 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleExport = async () => {
    setExporting(true);
    try {
      const date = new Date();
      const today = [
        date.getFullYear(),
        String(date.getMonth() + 1).padStart(2, '0'),
        String(date.getDate()).padStart(2, '0'),
      ].join('-');
      const path = await save({
        defaultPath: `inventario_${today}.xlsx`,
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
      });
      if (!path) return;

      const result = await exportInventory(path);
      addToast(
        `Exportado: ${result.active_rows} variantes activas, ${result.inactive_rows} inactivas, ${result.low_stock_rows} con stock bajo`,
        'success'
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      addToast(`No se pudo exportar: ${message}`, 'error');
    } finally {
      setExporting(false);
    }
  };

  const clearFilters = () => {
    setSearch('');
    setFilterCat('');
    setFilterActive('active');
    setFilterVariantActive('all');
    setFilterStock('');
    setSort('name:asc');
  };

  const outCount = lowStock.filter(item => item.stock_status === 'out_of_stock').length;
  const lowCount = lowStock.length - outCount;
  const totalVariantCount = Object.values(variantStats).reduce((sum, s) => sum + s.count, 0);
  const firstItem = total === 0 ? 0 : (page - 1) * pageSize + 1;
  const lastItem = Math.min(page * pageSize, total);

  if (loading && products.length === 0) {
    return (
      <div className="inv-page">
        <div className="inv-loading">Cargando inventario...</div>
      </div>
    );
  }

  return (
    <div className="inv-page">
      <div className="inv-header">
        <div>
          <h1 className="inv-title">Inventario</h1>
          <p className="inv-subtitle">
            {total.toLocaleString()} productos
            {filterActive !== 'all' ? ` ${filterActive === 'active' ? 'activos' : 'inactivos'}` : ''}
            {' · '}{totalVariantCount} variantes visibles
          </p>
        </div>
        <div className="inv-header-actions">
          <button className="btn btn-ghost" onClick={() => navigate('/stocktake')}>
            Toma de inventario
          </button>
          <button className="btn btn-ghost" onClick={handleExport} disabled={exporting}>
            {exporting ? 'Exportando…' : 'Exportar a Excel'}
          </button>
          <button className="btn btn-ghost" onClick={() => setShowImport(true)}>
            Importar desde Excel
          </button>
          <button className="btn btn-primary" onClick={() => setShowCreate(true)}>
            + Nuevo producto
          </button>
        </div>
      </div>

      <StockSummaryCards value={inventoryValue} lowCount={lowCount} outCount={outCount} />

      {lowStock.length > 0 && (
        <div className="inv-low-stock-banner">
          <span className="low-stock-icon">⚠</span>
          <span><strong>{lowStock.length}</strong> variante{lowStock.length !== 1 ? 's' : ''} con stock bajo o agotado</span>
          <div className="low-stock-pills">
            {lowStock.slice(0, 4).map(item => (
              <span key={item.variant_id} className={`low-pill ${item.stock_status === 'out_of_stock' ? 'low-pill--out' : 'low-pill--low'}`}>
                {item.product_name}{Object.keys(item.attributes).length > 0 ? ` · ${formatAttributes(item.attributes)}` : ''} ({item.stock})
              </span>
            ))}
            {lowStock.length > 4 && <span className="low-pill low-pill--more">+{lowStock.length - 4} más</span>}
          </div>
        </div>
      )}

      <div className="inv-filters">
        <div className="search-wrap">
          <span className="search-icon">⌕</span>
          <input
            className="search-input"
            placeholder="Buscar por nombre, marca, SKU o código…"
            value={search}
            onChange={e => setSearch(e.target.value)}
          />
          {search && <button className="search-clear" onClick={() => setSearch('')}>×</button>}
        </div>

        <select
          className="filter-select"
          value={filterCat}
          onChange={e => setFilterCat(e.target.value === '' ? '' : Number(e.target.value))}
        >
          <option value="">Todas las categorías</option>
          {categories.map(c => <option key={c.id} value={c.id}>{c.name}</option>)}
        </select>

        <select
          className="filter-select"
          value={filterStock}
          onChange={e => setFilterStock(e.target.value as StockFilter)}
        >
          {(Object.keys(STOCK_FILTER_LABELS) as StockFilter[]).map(key => (
            <option key={key} value={key}>{STOCK_FILTER_LABELS[key]}</option>
          ))}
        </select>

        <select
          className="filter-select"
          aria-label="Estado de variantes"
          value={filterVariantActive}
          onChange={e => setFilterVariantActive(e.target.value as 'all' | 'active' | 'inactive')}
        >
          <option value="all">Todas las variantes</option>
          <option value="active">Variantes activas</option>
          <option value="inactive">Variantes inactivas</option>
        </select>

        <select
          className="filter-select"
          value={sort}
          onChange={e => setSort(e.target.value as SortOption)}
        >
          {SORT_OPTIONS.map(option => (
            <option key={option.value} value={option.value}>{option.label}</option>
          ))}
        </select>

        <div className="filter-tabs">
          {(['all', 'active', 'inactive'] as const).map(opt => (
            <button
              key={opt}
              className={`filter-tab ${filterActive === opt ? 'filter-tab--active' : ''}`}
              onClick={() => setFilterActive(opt)}
            >
              {{ all: 'Todos', active: 'Activos', inactive: 'Inactivos' }[opt]}
            </button>
          ))}
        </div>
      </div>

      {products.length === 0 ? (
        <div className="inv-empty">
          <span style={{ fontSize: 36, opacity: 0.25 }}>📦</span>
          <p>No se encontraron productos</p>
          {(search || filterCat !== '' || filterActive !== 'active' || filterVariantActive !== 'all' || filterStock !== '') && (
            <button className="btn btn-ghost" onClick={clearFilters}>Limpiar filtros</button>
          )}
        </div>
      ) : (
        <>
          <div className="inv-table-wrap">
            <table className="inv-table">
              <thead>
                <tr>
                  <th>Producto</th>
                  <th>Categoría</th>
                  <th>Variantes</th>
                  <th className="col-right">Stock total</th>
                  <th>Estado</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {products.map(product => {
                  const stats = variantStats[product.id];
                  return (
                    <tr
                      key={product.id}
                      className="inv-row"
                      onClick={() => navigate(`/inventory/${product.id}`)}
                      style={{ cursor: 'pointer' }}
                    >
                      <td className="cell-product">
                        <div className="product-avatar">
                          {product.image_url
                            ? <img src={product.image_url} alt={product.name} className="product-img" />
                            : <span className="product-img-placeholder">📦</span>
                          }
                        </div>
                        <div className="product-info">
                          <span className="product-name">{product.name}</span>
                          {product.brand && <span className="product-brand">{product.brand}</span>}
                        </div>
                      </td>
                      <td><span className="cat-chip">{product.category_name}</span></td>
                      <td>
                        <span className="variant-count">{stats ? stats.count : '—'}</span>
                      </td>
                      <td className="col-right">
                        {stats && stats.count > 0
                          ? <StockBadge stock={stats.total} stockMin={stats.min * stats.count} />
                          : <span style={{ color: 'var(--text-muted)' }}>—</span>
                        }
                      </td>
                      <td>
                        <span className={`status-dot ${product.is_active ? 'status-dot--active' : 'status-dot--inactive'}`}>
                          {product.is_active ? 'Activo' : 'Inactivo'}
                        </span>
                      </td>
                      <td className="cell-actions" onClick={e => e.stopPropagation()}>
                        <button className="action-btn" title="Editar" onClick={() => setEditProduct(product)}>✎</button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          <div className="inv-pagination">
            <span className="inv-pagination-info">
              Mostrando <strong>{firstItem.toLocaleString()}</strong>–<strong>{lastItem.toLocaleString()}</strong> de <strong>{total.toLocaleString()}</strong>
            </span>

            <div className="inv-pagination-controls">
              <button
                className="btn btn-ghost"
                onClick={() => setPage(p => Math.max(1, p - 1))}
                disabled={page <= 1 || loading}
              >
                ← Anterior
              </button>
              <span className="inv-pagination-page">
                Página <strong>{page}</strong> de <strong>{Math.max(totalPages, 1)}</strong>
              </span>
              <button
                className="btn btn-ghost"
                onClick={() => setPage(p => (totalPages === 0 || p >= totalPages ? p : p + 1))}
                disabled={loading || totalPages === 0 || page >= totalPages}
              >
                Siguiente →
              </button>
            </div>

            <div className="inv-pagination-size">
              <span>Mostrar</span>
              <select
                className="filter-select"
                value={pageSize}
                onChange={e => setPageSize(Number(e.target.value))}
              >
                <option value={10}>10</option>
                <option value={20}>20</option>
                <option value={50}>50</option>
                <option value={100}>100</option>
              </select>
              <span>por página</span>
            </div>
          </div>
        </>
      )}

      {showCreate && (
        <Modal title="Nuevo producto" onClose={() => setShowCreate(false)}>
          <ProductForm
            categories={categories}
            onSubmit={handleCreate}
            onCancel={() => setShowCreate(false)}
            loading={saving}
          />
        </Modal>
      )}

      {editProduct && (
        <Modal title="Editar producto" onClose={() => setEditProduct(null)}>
          <ProductForm
            categories={categories}
            product={editProduct}
            onSubmit={handleEdit}
            onCancel={() => setEditProduct(null)}
            loading={saving}
          />
        </Modal>
      )}

      {showImport && (
        <ImportInventoryModal
          onClose={() => setShowImport(false)}
          onImported={loadAll}
          addToast={addToast}
        />
      )}

      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
};