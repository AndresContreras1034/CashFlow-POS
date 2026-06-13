import React, { useState, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Modal } from '../../components/ui/Modal';
import { ToastContainer, ToastData } from '../../components/ui/Toast';
import { StockBadge } from '../../components/inventory/StockBadge';
import { KardexTable } from '../../components/inventory/KardexTable';
import { VariantForm } from '../../components/inventory/VariantForm.tsx';
import {
  ProductWithCategory,
  ProductVariant,
  MovementWithDetails,
  CreateVariantDto,
  UpdateVariantDto,
  StockEntryDto,
  StockOutDto,
  StockAdjustmentDto,
} from '../../types';
import { formatMoney, formatAttributes } from '../../utils/format';
import './ProductDetail.css';

// ─── Datos de ejemplo ─────────────────────────────────────────────────────────

const MOCK_PRODUCTS: ProductWithCategory[] = [
  { id: 1, category_id: 1, category_name: 'Cuidado capilar', name: 'Shampoo Anticaída', brand: "L'Oréal", description: 'Fórmula enriquecida con biotina y queratina para fortalecer el cabello desde la raíz.', is_active: true, created_at: '2026-01-10T10:00:00Z', updated_at: '2026-06-01T08:00:00Z' },
  { id: 2, category_id: 1, category_name: 'Cuidado capilar', name: 'Acondicionador Hidratante', brand: 'Pantene', description: '', is_active: true, created_at: '2026-01-12T10:00:00Z', updated_at: '2026-05-20T08:00:00Z' },
  { id: 3, category_id: 2, category_name: 'Cuidado facial', name: 'Crema Hidratante SPF 30', brand: 'Neutrogena', description: 'Protección solar diaria con hidratación duradera.', is_active: true, created_at: '2026-02-01T10:00:00Z', updated_at: '2026-06-05T08:00:00Z' },
  { id: 4, category_id: 3, category_name: 'Maquillaje', name: 'Base Fluida HD', brand: 'Maybelline', description: '', is_active: true, created_at: '2026-02-15T10:00:00Z', updated_at: '2026-06-10T08:00:00Z' },
];

const INITIAL_VARIANTS: Record<number, ProductVariant[]> = {
  1: [
    { id: 1, product_id: 1, attributes: { Tamaño: '400 ml' }, sku: 'SHA-001', barcode: '7501234000011', price: 2800000, cost: 1500000, stock: 24, stock_min: 5, allow_negative: false, is_active: true, created_at: '2026-01-10T10:00:00Z', updated_at: '' },
    { id: 2, product_id: 1, attributes: { Tamaño: '800 ml' }, sku: 'SHA-002', barcode: '7501234000012', price: 4500000, cost: 2400000, stock: 3,  stock_min: 5, allow_negative: false, is_active: true, created_at: '2026-01-10T10:00:00Z', updated_at: '' },
  ],
  2: [
    { id: 3, product_id: 2, attributes: { Tamaño: '400 ml' }, sku: 'ACO-001', barcode: '7501234000021', price: 2600000, cost: 1300000, stock: 0,  stock_min: 5, allow_negative: false, is_active: true, created_at: '2026-01-12T10:00:00Z', updated_at: '' },
  ],
  3: [
    { id: 4, product_id: 3, attributes: {}, sku: 'CRE-001', price: 3900000, cost: 2000000, stock: 12, stock_min: 3, allow_negative: false, is_active: true, created_at: '2026-02-01T10:00:00Z', updated_at: '' },
  ],
  4: [
    { id: 5, product_id: 4, attributes: { Tono: 'Beige' },  sku: 'BAS-001', price: 5200000, cost: 2800000, stock: 8, stock_min: 4, allow_negative: false, is_active: true, created_at: '2026-02-15T10:00:00Z', updated_at: '' },
    { id: 6, product_id: 4, attributes: { Tono: 'Marfil' }, sku: 'BAS-002', price: 5200000, cost: 2800000, stock: 2, stock_min: 4, allow_negative: false, is_active: true, created_at: '2026-02-15T10:00:00Z', updated_at: '' },
  ],
};

const MOCK_MOVEMENTS: MovementWithDetails[] = [
  { id: 1,  variant_id: 1, movement_type: 'initial_stock', quantity: 30,  stock_before: 0,  stock_after: 30, unit_cost: 1500000, product_name: 'Shampoo Anticaída', attributes: { Tamaño: '400 ml' }, created_by: 'admin', created_at: '2026-01-10T10:00:00Z' },
  { id: 2,  variant_id: 1, movement_type: 'sale',          quantity: -3,  stock_before: 30, stock_after: 27, unit_cost: 0,       product_name: 'Shampoo Anticaída', attributes: { Tamaño: '400 ml' }, created_by: 'cajero1', created_at: '2026-02-14T15:30:00Z' },
  { id: 3,  variant_id: 1, movement_type: 'sale',          quantity: -2,  stock_before: 27, stock_after: 25, unit_cost: 0,       product_name: 'Shampoo Anticaída', attributes: { Tamaño: '400 ml' }, created_by: 'cajero1', created_at: '2026-03-05T11:20:00Z' },
  { id: 4,  variant_id: 1, movement_type: 'purchase',      quantity: 20,  stock_before: 25, stock_after: 45, unit_cost: 1500000, product_name: 'Shampoo Anticaída', attributes: { Tamaño: '400 ml' }, created_by: 'admin', created_at: '2026-04-01T09:00:00Z' },
  { id: 5,  variant_id: 1, movement_type: 'sale',          quantity: -5,  stock_before: 45, stock_after: 40, unit_cost: 0,       product_name: 'Shampoo Anticaída', attributes: { Tamaño: '400 ml' }, notes: 'Venta mayorista', created_by: 'cajero2', created_at: '2026-04-20T14:00:00Z' },
  { id: 6,  variant_id: 1, movement_type: 'adjustment',    quantity: -16, stock_before: 40, stock_after: 24, unit_cost: 0,       product_name: 'Shampoo Anticaída', attributes: { Tamaño: '400 ml' }, notes: 'Conteo físico', created_by: 'admin', created_at: '2026-06-01T08:00:00Z' },
  { id: 7,  variant_id: 2, movement_type: 'initial_stock', quantity: 10,  stock_before: 0,  stock_after: 10, unit_cost: 2400000, product_name: 'Shampoo Anticaída', attributes: { Tamaño: '800 ml' }, created_by: 'admin', created_at: '2026-01-10T10:05:00Z' },
  { id: 8,  variant_id: 2, movement_type: 'sale',          quantity: -7,  stock_before: 10, stock_after: 3,  unit_cost: 0,       product_name: 'Shampoo Anticaída', attributes: { Tamaño: '800 ml' }, created_by: 'cajero1', created_at: '2026-05-15T16:00:00Z' },
];

// ─── Helpers ──────────────────────────────────────────────────────────────────

function toastId() { return Math.random().toString(36).slice(2); }

// ─── Stock modal ──────────────────────────────────────────────────────────────

type StockModal = { mode: 'entry' | 'out' | 'adjustment'; variantId: number };

const StockMoveForm: React.FC<{
  mode: 'entry' | 'out' | 'adjustment';
  variant: ProductVariant;
  onSubmit: (data: StockEntryDto | StockOutDto | StockAdjustmentDto) => Promise<void>;
  onCancel: () => void;
  loading?: boolean;
}> = ({ mode, variant, onSubmit, onCancel, loading = false }) => {
  const [qty, setQty]     = useState('');
  const [notes, setNotes] = useState('');
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const n = Number(qty);
    if (!qty || isNaN(n) || n <= 0) { setError('Ingresa una cantidad válida mayor a 0'); return; }
    if (mode === 'out' && !variant.allow_negative && n > variant.stock) {
      setError(`Stock insuficiente (disponible: ${variant.stock})`);
      return;
    }
    const base = { variant_id: variant.id, notes: notes.trim() || undefined, created_by: 'admin' };
    if (mode === 'entry')      await onSubmit({ ...base, quantity: n } as StockEntryDto);
    else if (mode === 'out')   await onSubmit({ ...base, quantity: n } as StockOutDto);
    else                       await onSubmit({ ...base, actual_stock: n } as StockAdjustmentDto);
  };

  const labels = {
    entry:      { title: 'Cantidad a ingresar', placeholder: '0' },
    out:        { title: 'Cantidad a retirar',  placeholder: '0' },
    adjustment: { title: 'Stock real (conteo)', placeholder: String(variant.stock) },
  };

  return (
    <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: 16 }} noValidate>
      <div className="stock-form-info">
        <span className="stock-form-product">
          {variant.attributes && Object.keys(variant.attributes).length > 0
            ? formatAttributes(variant.attributes)
            : 'Variante única'}
        </span>
        <span className="stock-form-current">
          Stock actual: <strong>{variant.stock}</strong>
        </span>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        <label className="form-label">{labels[mode].title} <span className="required">*</span></label>
        <input
          className={`form-input${error ? ' form-input--error' : ''}`}
          type="number"
          min="1"
          value={qty}
          onChange={e => { setQty(e.target.value); setError(''); }}
          placeholder={labels[mode].placeholder}
          autoFocus
        />
        {error && <span className="form-error">{error}</span>}
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        <label className="form-label">Notas</label>
        <input
          className="form-input"
          type="text"
          value={notes}
          onChange={e => setNotes(e.target.value)}
          placeholder="Opcional"
        />
      </div>

      <div className="form-actions">
        <button type="button" className="btn btn-ghost" onClick={onCancel} disabled={loading}>Cancelar</button>
        <button type="submit" className="btn btn-primary" disabled={loading}>
          {loading ? 'Guardando…' : mode === 'entry' ? 'Registrar entrada' : mode === 'out' ? 'Registrar salida' : 'Ajustar stock'}
        </button>
      </div>
    </form>
  );
};

// ─── Componente principal ─────────────────────────────────────────────────────

export const ProductDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const productId = Number(id);
  const [product] = useState<ProductWithCategory | undefined>(
    MOCK_PRODUCTS.find(p => p.id === productId)
  );

  const [variants, setVariants] = useState<ProductVariant[]>(
    INITIAL_VARIANTS[productId] ?? []
  );
  const [movements, setMovements] = useState<MovementWithDetails[]>(
    MOCK_MOVEMENTS.filter(m => variants.map(v => v.id).includes(m.variant_id))
  );

  const [toasts, setToasts]             = useState<ToastData[]>([]);
  const [activeTab, setActiveTab]       = useState<'variants' | 'kardex'>('variants');
  const [showCreateVar, setShowCreateVar] = useState(false);
  const [editVariant, setEditVariant]   = useState<ProductVariant | null>(null);
  const [stockModal, setStockModal]     = useState<StockModal | null>(null);
  const [saving, setSaving]             = useState(false);

  const addToast = useCallback((message: string, type: ToastData['type']) => {
    setToasts(t => [...t, { id: toastId(), message, type }]);
  }, []);
  const dismissToast = useCallback((id: string) => {
    setToasts(t => t.filter(x => x.id !== id));
  }, []);

  if (!product) {
    return (
      <div className="detail-not-found">
        <p>Producto no encontrado</p>
        <button className="btn btn-ghost" onClick={() => navigate('/inventory')}>← Volver</button>
      </div>
    );
  }

  // Crear variante
  const handleCreateVariant = async (dto: CreateVariantDto | UpdateVariantDto) => {
    setSaving(true);
    await new Promise(r => setTimeout(r, 600));
    const newVar: ProductVariant = {
      id:             Math.max(0, ...variants.map(v => v.id)) + 1,
      product_id:     productId,
      attributes:     (dto as CreateVariantDto).attributes ?? {},
      sku:            (dto as CreateVariantDto).sku,
      barcode:        (dto as CreateVariantDto).barcode,
      price:          (dto as CreateVariantDto).price,
      cost:           (dto as CreateVariantDto).cost ?? 0,
      stock:          (dto as CreateVariantDto).stock ?? 0,
      stock_min:      (dto as CreateVariantDto).stock_min ?? 5,
      allow_negative: (dto as CreateVariantDto).allow_negative ?? false,
      is_active:      true,
      created_at:     new Date().toISOString(),
      updated_at:     new Date().toISOString(),
    };
    setVariants(prev => [...prev, newVar]);
    setSaving(false);
    setShowCreateVar(false);
    addToast('Variante creada', 'success');
  };

  // Editar variante
  const handleEditVariant = async (dto: CreateVariantDto | UpdateVariantDto) => {
    if (!editVariant) return;
    setSaving(true);
    await new Promise(r => setTimeout(r, 600));
    setVariants(prev => prev.map(v =>
      v.id === editVariant.id ? { ...v, ...dto, updated_at: new Date().toISOString() } : v
    ));
    setSaving(false);
    setEditVariant(null);
    addToast('Variante actualizada', 'success');
  };

  // Movimiento de stock
  const handleStockMove = async (data: StockEntryDto | StockOutDto | StockAdjustmentDto) => {
    if (!stockModal) return;
    setSaving(true);
    await new Promise(r => setTimeout(r, 600));

    const variant = variants.find(v => v.id === stockModal.variantId);
    if (!variant) { setSaving(false); return; }

    let qty = 0;
    let newStock = variant.stock;
    let movType: MovementWithDetails['movement_type'] = 'manual_in';

    if (stockModal.mode === 'entry') {
      qty = (data as StockEntryDto).quantity;
      newStock = variant.stock + qty;
      movType = 'manual_in';
    } else if (stockModal.mode === 'out') {
      qty = (data as StockOutDto).quantity;
      newStock = variant.stock - qty;
      movType = 'manual_out';
    } else {
      newStock = (data as StockAdjustmentDto).actual_stock;
      qty = newStock - variant.stock;
      movType = 'adjustment';
    }

    setVariants(prev => prev.map(v =>
      v.id === stockModal.variantId ? { ...v, stock: newStock, updated_at: new Date().toISOString() } : v
    ));

    const newMov: MovementWithDetails = {
      id:            Math.max(0, ...movements.map(m => m.id)) + 1,
      variant_id:    stockModal.variantId,
      movement_type: movType,
      quantity:      qty,
      stock_before:  variant.stock,
      stock_after:   newStock,
      unit_cost:     0,
      notes:         data.notes,
      created_by:    data.created_by ?? 'admin',
      created_at:    new Date().toISOString(),
      product_name:  product.name,
      attributes:    variant.attributes,
      sku:           variant.sku,
    };
    setMovements(prev => [newMov, ...prev]);

    setSaving(false);
    setStockModal(null);
    addToast(
      stockModal.mode === 'entry' ? 'Entrada registrada' :
      stockModal.mode === 'out'   ? 'Salida registrada'  : 'Stock ajustado',
      'success'
    );
  };

  const stockModalVariant = stockModal ? variants.find(v => v.id === stockModal.variantId) : null;
  const stockModalTitle = stockModal
    ? { entry: 'Registrar entrada', out: 'Registrar salida', adjustment: 'Ajustar stock' }[stockModal.mode]
    : '';

  const totalStock = variants.reduce((s, v) => s + v.stock, 0);
  const activeVariants = variants.filter(v => v.is_active).length;

  return (
    <div className="detail-page">
      {/* ── Breadcrumb ── */}
      <button className="detail-back" onClick={() => navigate('/inventory')}>
        ← Inventario
      </button>

      {/* ── Hero del producto ── */}
      <div className="detail-hero">
        <div className="detail-hero-left">
          <div className="detail-avatar">
            {product.image_url
              ? <img src={product.image_url} alt={product.name} />
              : <span>📦</span>
            }
          </div>
          <div className="detail-hero-info">
            <div className="detail-hero-meta">
              <span className="detail-category">{product.category_name}</span>
              <span className={`status-dot ${product.is_active ? 'status-dot--active' : 'status-dot--inactive'}`}>
                {product.is_active ? 'Activo' : 'Inactivo'}
              </span>
            </div>
            <h1 className="detail-title">{product.name}</h1>
            {product.brand && <p className="detail-brand">{product.brand}</p>}
            {product.description && <p className="detail-description">{product.description}</p>}
          </div>
        </div>

        <div className="detail-stats">
          <div className="stat-card">
            <span className="stat-label">Stock total</span>
            <span className="stat-value tabular">{totalStock}</span>
          </div>
          <div className="stat-card">
            <span className="stat-label">Variantes activas</span>
            <span className="stat-value tabular">{activeVariants}</span>
          </div>
          <div className="stat-card">
            <span className="stat-label">Precio desde</span>
            <span className="stat-value tabular">
              {variants.length ? formatMoney(Math.min(...variants.map(v => v.price))) : '—'}
            </span>
          </div>
          <div className="stat-card">
            <span className="stat-label">Movimientos</span>
            <span className="stat-value tabular">{movements.length}</span>
          </div>
        </div>
      </div>

      {/* ── Tabs ── */}
      <div className="detail-tabs">
        <button
          className={`detail-tab ${activeTab === 'variants' ? 'detail-tab--active' : ''}`}
          onClick={() => setActiveTab('variants')}
        >
          Variantes <span className="tab-count">{variants.length}</span>
        </button>
        <button
          className={`detail-tab ${activeTab === 'kardex' ? 'detail-tab--active' : ''}`}
          onClick={() => setActiveTab('kardex')}
        >
          Kardex <span className="tab-count">{movements.length}</span>
        </button>
      </div>

      {/* ── Contenido: Variantes ── */}
      {activeTab === 'variants' && (
        <div className="detail-section">
          <div className="section-header">
            <h2 className="section-title">Variantes</h2>
            <button className="btn btn-primary" onClick={() => setShowCreateVar(true)}>
              + Nueva variante
            </button>
          </div>

          {variants.length === 0 ? (
            <div className="section-empty">
              <span style={{ fontSize: 32, opacity: 0.25 }}>📦</span>
              <p>Sin variantes. Agrega la primera.</p>
            </div>
          ) : (
            <div className="variants-grid">
              {variants.map(v => (
                <div key={v.id} className={`variant-card ${!v.is_active ? 'variant-card--inactive' : ''}`}>
                  <div className="variant-card-top">
                    <div className="variant-attrs">
                      {Object.keys(v.attributes).length > 0
                        ? Object.entries(v.attributes).map(([k, val]) => (
                            <span key={k} className="attr-tag"><span className="attr-key">{k}</span>{val}</span>
                          ))
                        : <span className="attr-tag">Variante única</span>
                      }
                    </div>
                    <StockBadge stock={v.stock} stockMin={v.stock_min} />
                  </div>

                  <div className="variant-prices">
                    <div className="price-row">
                      <span className="price-label">Precio</span>
                      <span className="price-value tabular">{formatMoney(v.price)}</span>
                    </div>
                    {v.cost > 0 && (
                      <div className="price-row">
                        <span className="price-label">Costo</span>
                        <span className="price-value price-cost tabular">{formatMoney(v.cost)}</span>
                      </div>
                    )}
                    {v.cost > 0 && (
                      <div className="price-row">
                        <span className="price-label">Margen</span>
                        <span className="price-value price-margin tabular">
                          {(((v.price - v.cost) / v.price) * 100).toFixed(1)}%
                        </span>
                      </div>
                    )}
                  </div>

                  {(v.sku || v.barcode) && (
                    <div className="variant-codes">
                      {v.sku     && <span className="code-chip">SKU: {v.sku}</span>}
                      {v.barcode && <span className="code-chip">EAN: {v.barcode}</span>}
                    </div>
                  )}

                  <div className="variant-actions">
                    <button
                      className="vaction-btn vaction-btn--in"
                      onClick={() => setStockModal({ mode: 'entry', variantId: v.id })}
                    >↑ Entrada</button>
                    <button
                      className="vaction-btn vaction-btn--out"
                      onClick={() => setStockModal({ mode: 'out', variantId: v.id })}
                    >↓ Salida</button>
                    <button
                      className="vaction-btn vaction-btn--adj"
                      onClick={() => setStockModal({ mode: 'adjustment', variantId: v.id })}
                    >⊕ Ajuste</button>
                    <button
                      className="vaction-btn vaction-btn--edit"
                      onClick={() => setEditVariant(v)}
                    >✎</button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* ── Contenido: Kardex ── */}
      {activeTab === 'kardex' && (
        <div className="detail-section">
          <div className="section-header">
            <h2 className="section-title">Kardex de movimientos</h2>
          </div>
          <KardexTable movements={movements} />
        </div>
      )}

      {/* ── Modales ── */}
      {showCreateVar && (
        <Modal title="Nueva variante" onClose={() => setShowCreateVar(false)} width={600}>
          <VariantForm
            productId={productId}
            onSubmit={handleCreateVariant}
            onCancel={() => setShowCreateVar(false)}
            loading={saving}
          />
        </Modal>
      )}

      {editVariant && (
        <Modal title="Editar variante" onClose={() => setEditVariant(null)} width={600}>
          <VariantForm
            productId={productId}
            variant={editVariant}
            onSubmit={handleEditVariant}
            onCancel={() => setEditVariant(null)}
            loading={saving}
          />
        </Modal>
      )}

      {stockModal && stockModalVariant && (
        <Modal title={stockModalTitle} onClose={() => setStockModal(null)} width={440}>
          <StockMoveForm
            mode={stockModal.mode}
            variant={stockModalVariant}
            onSubmit={handleStockMove}
            onCancel={() => setStockModal(null)}
            loading={saving}
          />
        </Modal>
      )}

      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
};