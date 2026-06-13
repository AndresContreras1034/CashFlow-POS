import React, { useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { Modal } from '../../components/ui/Modal';
import { ToastContainer, ToastData } from '../../components/ui/Toast';
import { StockBadge } from '../../components/inventory/StockBadge';
import { ProductForm } from '../../components/inventory/ProductForm';
import {
  Category,
  ProductWithCategory,
  VariantWithProduct,
  CreateProductDto,
  UpdateProductDto,
  LowStockItemDto,
} from '../../types';
import { formatMoney, formatAttributes } from '../../utils/format';
import './inventory.css';

// ─── Datos de ejemplo ────────────────────────────────────────────────────────

const MOCK_CATEGORIES: Category[] = [
  { id: 1, name: 'Cuidado capilar', is_active: true, created_at: '', updated_at: '' },
  { id: 2, name: 'Cuidado facial', is_active: true, created_at: '', updated_at: '' },
  { id: 3, name: 'Maquillaje',      is_active: true, created_at: '', updated_at: '' },
  { id: 4, name: 'Fragancias',      is_active: true, created_at: '', updated_at: '' },
];

const MOCK_PRODUCTS: ProductWithCategory[] = [
  { id: 1, category_id: 1, category_name: 'Cuidado capilar', name: 'Shampoo Anticaída', brand: "L'Oréal", description: 'Fórmula con biotina', is_active: true, created_at: '2026-01-10T10:00:00Z', updated_at: '2026-06-01T08:00:00Z' },
  { id: 2, category_id: 1, category_name: 'Cuidado capilar', name: 'Acondicionador Hidratante', brand: 'Pantene', description: '', is_active: true, created_at: '2026-01-12T10:00:00Z', updated_at: '2026-05-20T08:00:00Z' },
  { id: 3, category_id: 2, category_name: 'Cuidado facial', name: 'Crema Hidratante SPF 30', brand: 'Neutrogena', description: 'Protección solar diaria', is_active: true, created_at: '2026-02-01T10:00:00Z', updated_at: '2026-06-05T08:00:00Z' },
  { id: 4, category_id: 3, category_name: 'Maquillaje', name: 'Base Fluida HD', brand: 'Maybelline', description: '', is_active: true, created_at: '2026-02-15T10:00:00Z', updated_at: '2026-06-10T08:00:00Z' },
  { id: 5, category_id: 4, category_name: 'Fragancias', name: 'Eau de Parfum Rosé', brand: 'Carolina Herrera', description: '100 ml', is_active: false, created_at: '2026-03-01T10:00:00Z', updated_at: '2026-06-01T08:00:00Z' },
];

const MOCK_VARIANTS: VariantWithProduct[] = [
  { id: 1,  product_id: 1, product_name: 'Shampoo Anticaída',      brand: "L'Oréal",   category_id: 1, category_name: 'Cuidado capilar', attributes: { Tamaño: '400 ml' }, sku: 'SHA-001', price: 2800000, cost: 1500000, stock: 24, stock_min: 5,  allow_negative: false, is_active: true,  created_at: '2026-01-10T10:00:00Z', updated_at: '' },
  { id: 2,  product_id: 1, product_name: 'Shampoo Anticaída',      brand: "L'Oréal",   category_id: 1, category_name: 'Cuidado capilar', attributes: { Tamaño: '800 ml' }, sku: 'SHA-002', price: 4500000, cost: 2400000, stock: 3,  stock_min: 5,  allow_negative: false, is_active: true,  created_at: '2026-01-10T10:00:00Z', updated_at: '' },
  { id: 3,  product_id: 2, product_name: 'Acondicionador Hidratante', brand: 'Pantene', category_id: 1, category_name: 'Cuidado capilar', attributes: { Tamaño: '400 ml' }, sku: 'ACO-001', price: 2600000, cost: 1300000, stock: 0,  stock_min: 5,  allow_negative: false, is_active: true,  created_at: '2026-01-12T10:00:00Z', updated_at: '' },
  { id: 4,  product_id: 3, product_name: 'Crema Hidratante SPF 30', brand: 'Neutrogena',category_id: 2, category_name: 'Cuidado facial', attributes: {},               sku: 'CRE-001', price: 3900000, cost: 2000000, stock: 12, stock_min: 3,  allow_negative: false, is_active: true,  created_at: '2026-02-01T10:00:00Z', updated_at: '' },
  { id: 5,  product_id: 4, product_name: 'Base Fluida HD',          brand: 'Maybelline',category_id: 3, category_name: 'Maquillaje',      attributes: { Tono: 'Beige' },  sku: 'BAS-001', price: 5200000, cost: 2800000, stock: 8,  stock_min: 4,  allow_negative: false, is_active: true,  created_at: '2026-02-15T10:00:00Z', updated_at: '' },
  { id: 6,  product_id: 4, product_name: 'Base Fluida HD',          brand: 'Maybelline',category_id: 3, category_name: 'Maquillaje',      attributes: { Tono: 'Marfil' }, sku: 'BAS-002', price: 5200000, cost: 2800000, stock: 2,  stock_min: 4,  allow_negative: false, is_active: true,  created_at: '2026-02-15T10:00:00Z', updated_at: '' },
];

// ─── Helpers ──────────────────────────────────────────────────────────────────

function toastId() { return Math.random().toString(36).slice(2); }

function variantsForProduct(productId: number) {
  return MOCK_VARIANTS.filter(v => v.product_id === productId);
}

function productStockSummary(productId: number): { total: number; min: number } {
  const variants = variantsForProduct(productId);
  return {
    total: variants.reduce((s, v) => s + v.stock, 0),
    min:   Math.min(...variants.map(v => v.stock_min), 9999),
  };
}

// ─── Componente principal ─────────────────────────────────────────────────────

export const Inventory: React.FC = () => {
  const navigate = useNavigate();

  const [products, setProducts]       = useState<ProductWithCategory[]>(MOCK_PRODUCTS);
  const [categories]                  = useState<Category[]>(MOCK_CATEGORIES);
  const [toasts, setToasts]           = useState<ToastData[]>([]);

  // Filtros
  const [search, setSearch]           = useState('');
  const [filterCat, setFilterCat]     = useState<number | ''>('');
  const [filterActive, setFilterActive] = useState<'all' | 'active' | 'inactive'>('active');

  // Modales
  const [showCreate, setShowCreate]   = useState(false);
  const [editProduct, setEditProduct] = useState<ProductWithCategory | null>(null);
  const [saving, setSaving]           = useState(false);

  // Panel de stock bajo
  const lowStock: LowStockItemDto[] = MOCK_VARIANTS
    .filter(v => v.stock <= v.stock_min)
    .map(v => ({
      variant_id:   v.id,
      product_name: v.product_name,
      attributes:   v.attributes,
      barcode:      v.sku,
      stock:        v.stock,
      stock_min:    v.stock_min,
      stock_status: v.stock <= 0 ? 'out_of_stock' : 'low',
    }));

  // Filtrado
  const filtered = products.filter(p => {
    const matchSearch = !search ||
      p.name.toLowerCase().includes(search.toLowerCase()) ||
      (p.brand ?? '').toLowerCase().includes(search.toLowerCase());
    const matchCat    = filterCat === '' || p.category_id === filterCat;
    const matchActive = filterActive === 'all' ||
      (filterActive === 'active' ? p.is_active : !p.is_active);
    return matchSearch && matchCat && matchActive;
  });

  // Toast helpers
  const addToast = useCallback((message: string, type: ToastData['type']) => {
    setToasts(t => [...t, { id: toastId(), message, type }]);
  }, []);
  const dismissToast = useCallback((id: string) => {
    setToasts(t => t.filter(x => x.id !== id));
  }, []);

  // Crear producto
  const handleCreate = async (dto: CreateProductDto | UpdateProductDto) => {
    setSaving(true);
    await new Promise(r => setTimeout(r, 600));
    const newProduct: ProductWithCategory = {
      id:            products.length + 10,
      category_id:   (dto as CreateProductDto).category_id,
      category_name: categories.find(c => c.id === (dto as CreateProductDto).category_id)?.name ?? '',
      name:          (dto as CreateProductDto).name,
      description:   dto.description,
      brand:         dto.brand,
      image_url:     dto.image_url,
      is_active:     true,
      created_at:    new Date().toISOString(),
      updated_at:    new Date().toISOString(),
    };
    setProducts(prev => [newProduct, ...prev]);
    setSaving(false);
    setShowCreate(false);
    addToast('Producto creado correctamente', 'success');
  };

  // Editar producto
  const handleEdit = async (dto: CreateProductDto | UpdateProductDto) => {
    if (!editProduct) return;
    setSaving(true);
    await new Promise(r => setTimeout(r, 600));
    setProducts(prev => prev.map(p =>
      p.id === editProduct.id
        ? { ...p, ...dto, category_name: categories.find(c => c.id === (dto as UpdateProductDto).category_id)?.name ?? p.category_name, updated_at: new Date().toISOString() }
        : p
    ));
    setSaving(false);
    setEditProduct(null);
    addToast('Producto actualizado', 'success');
  };

  return (
    <div className="inv-page">
      {/* ── Cabecera ── */}
      <div className="inv-header">
        <div>
          <h1 className="inv-title">Inventario</h1>
          <p className="inv-subtitle">{products.filter(p => p.is_active).length} productos activos · {MOCK_VARIANTS.length} variantes</p>
        </div>
        <button className="btn btn-primary" onClick={() => setShowCreate(true)}>
          + Nuevo producto
        </button>
      </div>

      {/* ── Alerta stock bajo ── */}
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

      {/* ── Filtros ── */}
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

      {/* ── Tabla ── */}
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
                <th />
              </tr>
            </thead>
            <tbody>
              {filtered.map(product => {
                const variants    = variantsForProduct(product.id);
                const { total }   = productStockSummary(product.id);
                const minStockMin = variants.length ? Math.min(...variants.map(v => v.stock_min)) : 5;
                const minPrice    = variants.length ? Math.min(...variants.map(v => v.price)) : 0;

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
                      <span className="variant-count">{variants.length}</span>
                    </td>
                    <td className="col-right">
                      {variants.length > 0
                        ? <StockBadge stock={total} stockMin={minStockMin * variants.length} />
                        : <span style={{ color: 'var(--text-muted)' }}>—</span>
                      }
                    </td>
                    <td className="col-right tabular">
                      {minPrice > 0 ? formatMoney(minPrice) : '—'}
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

      {/* ── Modal crear ── */}
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

      {/* ── Modal editar ── */}
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