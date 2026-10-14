import React, { useCallback, useEffect, useRef, useState } from 'react';
import { listCategories } from '../../services/inventory.service';
import { getRestockOverview } from '../../services/suppliers.service';
import type { Category } from '../../types';
import type { RestockGroup, RestockOverview } from '../../types/suppliers';
import { errorMessage } from '../../utils/audit';
import { formatMoney } from '../../utils/format';
import { useDebounce } from '../../hooks/useDebounce';
import { SupplierCallCard } from './SupplierCallCard';
import type { ContactAction } from '../../utils/callPanel';
import './SuppliersBase.css';
import './SupplierCallPanel.css';

interface SupplierCallPanelProps {
  initialOnlyUrgent: boolean;
  reloadToken: number;
  businessName: string;
  onAction: (action: ContactAction, group: RestockGroup) => void;
  onCopy: (text: string, what: string) => void;
  onOpenSupplier: (supplierId: number) => void;
}

export const SupplierCallPanel: React.FC<SupplierCallPanelProps> = ({
  initialOnlyUrgent,
  reloadToken,
  businessName,
  onAction,
  onCopy,
  onOpenSupplier,
}) => {
  const [searchInput, setSearchInput] = useState('');
  const search = useDebounce(searchInput.trim(), 400);
  const [categoryId, setCategoryId] = useState('');
  const [onlyUrgent, setOnlyUrgent] = useState(initialOnlyUrgent);
  const [windowDays, setWindowDays] = useState(30);
  const [coverDays, setCoverDays] = useState(14);
  const [categories, setCategories] = useState<Category[]>([]);
  const [categoryError, setCategoryError] = useState<string | null>(null);
  const [overview, setOverview] = useState<RestockOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const requestRef = useRef(0);

  useEffect(() => {
    setOnlyUrgent(initialOnlyUrgent);
  }, [initialOnlyUrgent]);

  const loadCategories = useCallback(async () => {
    setCategoryError(null);
    try {
      setCategories((await listCategories()).filter(category => category.is_active));
    } catch (cause) {
      setCategoryError(errorMessage(cause));
    }
  }, []);

  useEffect(() => {
    void loadCategories();
  }, [loadCategories]);

  const loadOverview = useCallback(() => {
    const requestId = ++requestRef.current;
    setLoading(true);
    setError(null);
    getRestockOverview({
      only_urgent: onlyUrgent,
      category_id: categoryId ? Number(categoryId) : undefined,
      search: search || undefined,
      window_days: windowDays,
      cover_days: coverDays,
    })
      .then(result => {
        if (requestId !== requestRef.current) return;
        setOverview(result);
        setLoading(false);
      })
      .catch(cause => {
        if (requestId !== requestRef.current) return;
        setOverview(null);
        setError(errorMessage(cause));
        setLoading(false);
      });
  }, [categoryId, coverDays, onlyUrgent, search, windowDays]);

  useEffect(() => {
    loadOverview();
    return () => {
      requestRef.current += 1;
    };
  }, [loadOverview, reloadToken]);

  const hasFilters =
    search !== '' ||
    categoryId !== '' ||
    onlyUrgent ||
    windowDays !== 30 ||
    coverDays !== 14;
  const summary = overview?.summary;
  const groups = overview?.groups ?? [];

  return (
    <section className="supplier-call-panel" aria-label="Proveedores por contactar">
      <div className="supplier-call-filters">
        <div className="suppliers-field supplier-call-search">
          <label htmlFor="supplier-call-search">Buscar</label>
          <input
            id="supplier-call-search"
            className="suppliers-input"
            type="search"
            maxLength={100}
            value={searchInput}
            onChange={event => setSearchInput(event.target.value)}
            placeholder="Producto, variante o SKU…"
          />
        </div>

        <div className="suppliers-field">
          <label htmlFor="supplier-call-category">Categoría</label>
          <select
            id="supplier-call-category"
            className="suppliers-input"
            value={categoryId}
            onChange={event => setCategoryId(event.target.value)}
          >
            <option value="">Todas</option>
            {categories.map(category => (
              <option key={category.id} value={category.id}>{category.name}</option>
            ))}
          </select>
        </div>

        <label className="supplier-call-checkbox">
          <input
            type="checkbox"
            checked={onlyUrgent}
            onChange={event => setOnlyUrgent(event.target.checked)}
          />
          <span>Solo urgentes (agotados y críticos)</span>
        </label>

        <div className="suppliers-field">
          <label htmlFor="supplier-call-window">Ventas de los últimos</label>
          <select
            id="supplier-call-window"
            className="suppliers-input"
            value={windowDays}
            onChange={event => setWindowDays(Number(event.target.value))}
          >
            <option value={30}>30 días</option>
            <option value={60}>60 días</option>
            <option value={90}>90 días</option>
          </select>
        </div>

        <div className="suppliers-field">
          <label htmlFor="supplier-call-cover">Cubrir</label>
          <select
            id="supplier-call-cover"
            className="suppliers-input"
            value={coverDays}
            onChange={event => setCoverDays(Number(event.target.value))}
          >
            <option value={7}>7 días</option>
            <option value={14}>14 días</option>
            <option value={30}>30 días</option>
          </select>
        </div>

        <button type="button" className="suppliers-btn" onClick={loadOverview} disabled={loading}>
          Actualizar
        </button>
      </div>

      {categoryError && (
        <div className="supplier-call-error" role="alert">
          <span>No se pudieron cargar las categorías: {categoryError}</span>
          <button type="button" className="suppliers-btn" onClick={() => void loadCategories()}>
            Reintentar
          </button>
        </div>
      )}

      {loading ? (
        <p className="supplier-call-status" role="status">Cargando necesidades de reposición…</p>
      ) : error ? (
        <div className="supplier-call-error" role="alert">
          <span>No se pudo cargar el panel: {error}</span>
          <button type="button" className="suppliers-btn suppliers-btn-primary" onClick={loadOverview}>
            Reintentar
          </button>
        </div>
      ) : (
        <>
          {summary && (
            <p className="supplier-call-status" role="status" aria-live="polite">
              {summary.suppliers_to_contact} proveedores por contactar · {summary.out_of_stock_count}{' '}
              agotados · {summary.critical_count} críticos · {summary.unassigned_count} sin proveedor ·
              {' '}pedido estimado {formatMoney(summary.estimated_cost)}
              {summary.items_without_cost > 0 && ` · ${summary.items_without_cost} líneas sin costo`}
            </p>
          )}

          {groups.length === 0 ? (
            <div className="supplier-call-empty">
              <p>
                {hasFilters
                  ? 'Ningún resultado con estos filtros.'
                  : 'No hay productos por reponer. Todo en orden.'}
              </p>
              {hasFilters && (
                <button
                  type="button"
                  className="suppliers-btn"
                  onClick={() => {
                    setSearchInput('');
                    setCategoryId('');
                    setOnlyUrgent(false);
                    setWindowDays(30);
                    setCoverDays(14);
                  }}
                >
                  Limpiar
                </button>
              )}
            </div>
          ) : (
            <div className="supplier-call-groups">
              {groups.map((group, index) => (
                <SupplierCallCard
                  key={group.supplier?.id ?? `unassigned-${index}`}
                  group={group}
                  businessName={businessName}
                  onAction={onAction}
                  onCopy={onCopy}
                  onOpenSupplier={onOpenSupplier}
                />
              ))}
            </div>
          )}
        </>
      )}
    </section>
  );
};
