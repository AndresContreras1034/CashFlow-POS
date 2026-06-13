import React, { useState, useEffect, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Modal } from '../../components/ui/Modal';
import { ToastContainer, ToastData } from '../../components/ui/Toast';
import { StockBadge } from '../../components/inventory/StockBadge';
import { KardexTable } from '../../components/inventory/KardexTable';
import { VariantForm } from '../../components/inventory/VariantForm';
import { StockMoveForm } from '../../components/inventory/StockMoveForm';
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
import { formatMoney } from '../../utils/format';
import {
  getProduct,
  listVariants,
  getKardex,
  createVariant,
  updateVariant,
  registerManualEntry,
  registerManualOut,
  adjustStock,
} from '../../services/inventory.service';
import './ProductDetail.css';

export const ProductDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const productId = Number(id);
  const [product, setProduct] = useState<ProductWithCategory | null>(null);
  const [variants, setVariants] = useState<ProductVariant[]>([]);
  const [movements, setMovements] = useState<MovementWithDetails[]>([]);
  const [toasts, setToasts] = useState<ToastData[]>([]);
  const [activeTab, setActiveTab] = useState<'variants' | 'kardex'>('variants');
  const [showCreateVar, setShowCreateVar] = useState(false);
  const [editVariant, setEditVariant] = useState<ProductVariant | null>(null);
  const [stockModal, setStockModal] = useState<{ mode: 'entry' | 'out' | 'adjustment'; variantId: number } | null>(null);
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(true);
  const [notFound, setNotFound] = useState(false);

  const addToast = useCallback((message: string, type: ToastData['type']) => {
    setToasts(t => [...t, { id: Math.random().toString(36).slice(2), message, type }]);
  }, []);

  const dismissToast = useCallback((id: string) => {
    setToasts(t => t.filter(x => x.id !== id));
  }, []);

  useEffect(() => {
    const load = async () => {
      if (Number.isNaN(productId)) {
        setNotFound(true);
        setLoading(false);
        return;
      }

      try {
        setLoading(true);
        const [productRes, variantsRes, kardexRes] = await Promise.all([
          getProduct(productId),
          listVariants(productId),
          getKardex({ product_id: productId }),
        ]);

        setProduct(productRes);
        setVariants(variantsRes);
        setMovements(kardexRes.data);
      } catch (error) {
        setNotFound(true);
        addToast('No se pudo cargar el producto', 'error');
      } finally {
        setLoading(false);
      }
    };

    load();
  }, [productId, addToast]);

  const refreshMovements = useCallback(async () => {
    try {
      const kardexRes = await getKardex({ product_id: productId });
      setMovements(kardexRes.data);
    } catch (error) {
      addToast('No se pudo actualizar el kardex', 'error');
    }
  }, [productId, addToast]);

  const handleCreateVariant = async (dto: CreateVariantDto | UpdateVariantDto) => {
    if (Number.isNaN(productId)) return;
    setSaving(true);
    try {
      const newVar = await createVariant({ ...(dto as CreateVariantDto), product_id: productId });
      setVariants(prev => [newVar, ...prev]);
      addToast('Variante creada', 'success');
    } catch (error) {
      addToast('No se pudo crear la variante', 'error');
    } finally {
      setSaving(false);
      setShowCreateVar(false);
    }
  };

  const handleEditVariant = async (dto: CreateVariantDto | UpdateVariantDto) => {
    if (!editVariant) return;
    setSaving(true);
    try {
      const updated = await updateVariant(editVariant.id, dto as UpdateVariantDto);
      setVariants(prev => prev.map(v => v.id === updated.id ? updated : v));
      setEditVariant(null);
      addToast('Variante actualizada', 'success');
    } catch (error) {
      addToast('No se pudo actualizar la variante', 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleStockMove = async (data: StockEntryDto | StockOutDto | StockAdjustmentDto) => {
    if (!stockModal) return;
    setSaving(true);
    try {
      let updatedVariant: ProductVariant;
      if (stockModal.mode === 'entry') {
        updatedVariant = await registerManualEntry(data as StockEntryDto);
      } else if (stockModal.mode === 'out') {
        updatedVariant = await registerManualOut(data as StockOutDto);
      } else {
        updatedVariant = await adjustStock(data as StockAdjustmentDto);
      }

      setVariants(prev => prev.map(v => v.id === updatedVariant.id ? updatedVariant : v));
      await refreshMovements();
      addToast(
        stockModal.mode === 'entry' ? 'Entrada registrada' :
        stockModal.mode === 'out' ? 'Salida registrada' : 'Stock ajustado',
        'success'
      );
    } catch (error) {
      addToast('No se pudo registrar el movimiento', 'error');
    } finally {
      setSaving(false);
      setStockModal(null);
    }
  };

  if (loading) {
    return (
      <div className="detail-page">
        <div className="detail-loading">Cargando detalles del producto...</div>
      </div>
    );
  }

  if (notFound || !product) {
    return (
      <div className="detail-not-found">
        <p>Producto no encontrado</p>
        <button className="btn btn-ghost" onClick={() => navigate('/inventory')}>← Volver</button>
      </div>
    );
  }

  const stockModalVariant = stockModal ? variants.find(v => v.id === stockModal.variantId) : null;
  const stockModalTitle = stockModal
    ? { entry: 'Registrar entrada', out: 'Registrar salida', adjustment: 'Ajustar stock' }[stockModal.mode]
    : '';

  const totalStock = variants.reduce((s, v) => s + v.stock, 0);
  const activeVariants = variants.filter(v => v.is_active).length;

  return (
    <div className="detail-page">
      <button className="detail-back" onClick={() => navigate('/inventory')}>
        ← Inventario
      </button>

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
                        <span className="price-value price-margin tabular">{(((v.price - v.cost) / v.price) * 100).toFixed(1)}%</span>
                      </div>
                    )}
                  </div>

                  {(v.sku || v.barcode) && (
                    <div className="variant-codes">
                      {v.sku && <span className="code-chip">SKU: {v.sku}</span>}
                      {v.barcode && <span className="code-chip">EAN: {v.barcode}</span>}
                    </div>
                  )}

                  <div className="variant-actions">
                    <button className="vaction-btn vaction-btn--in" onClick={() => setStockModal({ mode: 'entry', variantId: v.id })}>↑ Entrada</button>
                    <button className="vaction-btn vaction-btn--out" onClick={() => setStockModal({ mode: 'out', variantId: v.id })}>↓ Salida</button>
                    <button className="vaction-btn vaction-btn--adj" onClick={() => setStockModal({ mode: 'adjustment', variantId: v.id })}>⊕ Ajuste</button>
                    <button className="vaction-btn vaction-btn--edit" onClick={() => setEditVariant(v)}>✎</button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {activeTab === 'kardex' && (
        <div className="detail-section">
          <div className="section-header">
            <h2 className="section-title">Kardex de movimientos</h2>
          </div>
          <KardexTable movements={movements} />
        </div>
      )}

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
