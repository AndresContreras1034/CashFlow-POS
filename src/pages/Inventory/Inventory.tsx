import React, { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { Modal } from '../../components/ui/Modal';
import { ToastContainer, ToastData } from '../../components/ui/Toast';
import { StockBadge } from '../../components/inventory/StockBadge';
import { ProductForm } from '../../components/inventory/ProductForm';
import {
  Category,
  ProductWithCategory,
  CreateProductDto,
  UpdateProductDto,
  LowStockItemDto,
} from '../../types';
import { formatMoney, formatAttributes } from '../../utils/format';
import {
  listCategories,
  listProducts,
  listVariants,
  getLowStock,
  createProduct,
  updateProduct,
} from '../../services/inventory.service';
import './inventory.css';

type VariantStats = { count: number; total: number; min: number };

export const Inventory: React.FC = () => {
  const navigate = useNavigate();

  const [products, setProducts] = useState<ProductWithCategory[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [variantStats, setVariantStats] = useState<Record<number, VariantStats>>({});
  const [lowStock, setLowStock] = useState<LowStockItemDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [toasts, setToasts] = useState<ToastData[]>([]);

  const [search, setSearch] = useState('');
  const [filterCat, setFilterCat] = useState<number | ''>('');
  const [filterActive, setFilterActive] = useState<'all' | 'active' | 'inactive'>('active');

  const [showCreate, setShowCreate] = useState(false);
  const [editProduct, setEditProduct] = useState<ProductWithCategory | null>(null);
  const [saving, setSaving] = useState(false);

  const addToast = useCallback((message: string, type: ToastData['type']) => {
    setToasts(t => [...t, { id: Math.random().toString(36).slice(2), message, type }]);
  }, []);

  const dismissToast = useCallback((id: string) => {
    setToasts(t => t.filter(x => x.id !== id));
  }, []);

  const refreshVariantStats = useCallback(async (productList: ProductWithCategory[]) => {
    try {
      const entries = await Promise.all(productList.map(async product => {
        const variants = await listVariants(product.id);
        const total = variants.reduce((sum, variant) => sum + variant.stock, 0);
        const min = variants.length ? Math.min(...variants.map(v => v.stock_min)) : 0;
        return [product.id, { count: variants.length, total, min }] as const;
      }));
      setVariantStats(Object.fromEntries(entries));
    } catch (error) {
      addToast('No se pudo obtener el inventario de variantes', 'error');
    }
  }, [addToast]);

  useEffect(() => {
    const load = async () => {
      try {
        setLoading(true);
        const [categoriesRes, productsRes, lowStockRes] = await Promise.all([
          listCategories(),
          listProducts({}),
          getLowStock(),
        ]);

        setCategories(categoriesRes);
        setProducts(productsRes.data);
        setLowStock(lowStockRes);
        await refreshVariantStats(productsRes.data);
      } catch (error) {
        addToast('Error cargando datos del inventario', 'error');
      } finally {
        setLoading(false);
      }
    };

    load();
  }, [refreshVariantStats, addToast]);

  const handleCreate = async (dto: CreateProductDto | UpdateProductDto) => {
    setSaving(true);
    try {
      const result = await createProduct(dto as CreateProductDto);
      const categoryName = categories.find(c => c.id === result.category_id)?.name ?? '';
      const newProduct: ProductWithCategory = { ...result, category_name: categoryName };
      setProducts(prev => [newProduct, ...prev]);
      await refreshVariantStats([newProduct, ...products]);
      setShowCreate(false);
      addToast('Producto creado correctamente', 'success');
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
      const result = await updateProduct(editProduct.id, dto as UpdateProductDto);
      const categoryName = categories.find(c => c.id === result.category_id)?.name ?? editProduct.category_name;
      const updatedProduct: ProductWithCategory = { ...editProduct, ...result, category_name: categoryName };
      setProducts(prev => prev.map(p => p.id === updatedProduct.id ? updatedProduct : p));
      setEditProduct(null);
      addToast('Producto actualizado', 'success');
    } catch (error) {
      addToast('No se pudo actualizar el producto', 'error');
    } finally {
      setSaving(false);
    }
  };

  const filtered = products.filter(p => {
    const matchSearch = !search || p.name.toLowerCase().includes(search.toLowerCase()) || (p.brand ?? '').toLowerCase().includes(search.toLowerCase());
    const matchCat = filterCat === '' || p.category_id === filterCat;
    const matchActive = filterActive === 'all' || (filterActive === 'active' ? p.is_active : !p.is_active);
    return matchSearch && matchCat && matchActive;
  });

  const totalVariantCount = Object.values(variantStats).reduce((sum, stats) => sum + stats.count, 0);

  if (loading) {
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
          <p className="inv-subtitle">{products.filter(p => p.is_active).length} productos activos · {totalVariantCount} variantes</p>
        </div>
        <button className="btn btn-primary" onClick={() => setShowCreate(true)}>
          + Nuevo producto
        </button>
      </div>

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
            placeholder="Buscar por nombre o marca…"
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

      {filtered.length === 0 ? (
        <div className="inv-empty">
          <span style={{ fontSize: 36, opacity: 0.25 }}>📦</span>
          <p>No se encontraron productos</p>
          {(search || filterCat !== '') && (
            <button className="btn btn-ghost" onClick={() => { setSearch(''); setFilterCat(''); }}>
              Limpiar filtros
            </button>
          )}
        </div>
      ) : (
        <div className="inv-table-wrap">
          <table className="inv-table">
            <thead>
              <tr>
                <th>Producto</th>
                <th>Categoría</th>
                <th>Variantes</th>
                <th className="col-right">Stock total</th>
                <th className="col-right">Precio desde</th>
                <th>Estado</th>
                <th /></tr>
            </thead>
            <tbody>
              {filtered.map(product => {
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
                    <td className="col-right tabular">
                      {stats && stats.count > 0 ? formatMoney(Math.min(...Array(stats.count).fill(stats.total))) : '—'}
                    </td>
                    <td>
                      <span className={`status-dot ${product.is_active ? 'status-dot--active' : 'status-dot--inactive'}`}>
                        {product.is_active ? 'Activo' : 'Inactivo'}
                      </span>
                    </td>
                    <td className="cell-actions" onClick={e => e.stopPropagation()}>
                      <button
                        className="action-btn"
                        title="Editar"
                        onClick={() => setEditProduct(product)}
                      >✎</button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
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

      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
};
