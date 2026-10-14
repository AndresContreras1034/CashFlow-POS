import React, { useCallback, useEffect, useRef, useState } from 'react';
import { getSupplierRestock } from '../../services/suppliers.service';
import type { RestockGroup } from '../../types/suppliers';
import { errorMessage } from '../../utils/audit';
import { formatAttributes, formatMoney } from '../../utils/format';
import { RESTOCK_URGENCY_LABELS } from '../../utils/suppliers';
import { formatCover, restockUrgencyBadgeClass } from '../../utils/supplierTabs';
import { StockBadge } from '../inventory/StockBadge';
import './SuppliersBase.css';
import './SupplierRestockTab.css';

interface SupplierRestockTabProps {
  supplierId: number;
  isActive: boolean;
}

export const SupplierRestockTab: React.FC<SupplierRestockTabProps> = ({
  supplierId,
  isActive,
}) => {
  const [windowDays, setWindowDays] = useState(30);
  const [coverDays, setCoverDays] = useState(14);
  const [group, setGroup] = useState<RestockGroup | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const requestRef = useRef(0);

  const loadRestock = useCallback(() => {
    const requestId = ++requestRef.current;
    setLoading(true);
    setError(null);

    getSupplierRestock(supplierId, windowDays, coverDays)
      .then(res => {
        if (requestId !== requestRef.current) return;
        setGroup(res);
        setLoading(false);
      })
      .catch(cause => {
        if (requestId !== requestRef.current) return;
        setError(errorMessage(cause));
        setLoading(false);
      });
  }, [supplierId, windowDays, coverDays]);

  useEffect(() => {
    loadRestock();
    return () => {
      requestRef.current += 1;
    };
  }, [loadRestock]);

  if (loading) {
    return <p className="supplier-tab-muted">Calculando necesidades de reposición…</p>;
  }

  if (error) {
    return (
      <div className="supplier-tab-error" role="alert">
        <span>No se pudo cargar la reposición: {error}</span>
        <button
          type="button"
          className="suppliers-btn suppliers-btn-primary"
          onClick={loadRestock}
        >
          Reintentar
        </button>
      </div>
    );
  }

  const items = group?.items ?? [];
  const itemCount = group?.item_count ?? 0;
  const outCount = group?.out_count ?? 0;
  const criticalCount = group?.critical_count ?? 0;
  const suggestedUnits = group?.suggested_units ?? 0;
  const estimatedCost = group?.estimated_cost ?? 0;
  const itemsWithoutCost = group?.items_without_cost ?? 0;

  return (
    <div className="supplier-restock-tab">
      <div className="supplier-restock-controls">
        <label className="supplier-restock-param">
          <span>Ventas de los últimos:</span>
          <select
            className="suppliers-input"
            value={windowDays}
            onChange={e => setWindowDays(Number(e.target.value))}
          >
            <option value={30}>30 días</option>
            <option value={60}>60 días</option>
            <option value={90}>90 días</option>
          </select>
        </label>

        <label className="supplier-restock-param">
          <span>Cubrir:</span>
          <select
            className="suppliers-input"
            value={coverDays}
            onChange={e => setCoverDays(Number(e.target.value))}
          >
            <option value={7}>7 días</option>
            <option value={14}>14 días</option>
            <option value={30}>30 días</option>
          </select>
        </label>
      </div>

      {itemCount === 0 ? (
        <div className="supplier-restock-empty">
          <p>Nada que reponer de este proveedor ahora.</p>
          {!isActive && (
            <p>El proveedor está inactivo y no participa en reposición.</p>
          )}
          <p>Enlaza productos en la pestaña Productos para habilitar reposición.</p>
        </div>
      ) : (
        <>
          <div className="supplier-restock-summary-bar">
            <strong>
              {itemCount} producto{itemCount !== 1 ? 's' : ''}
            </strong>
            <span>·</span>
            <span>
              {outCount} agotado{outCount !== 1 ? 's' : ''}
            </span>
            <span>·</span>
            <span>
              {criticalCount} crítico{criticalCount !== 1 ? 's' : ''}
            </span>
            <span>·</span>
            <span>
              {suggestedUnits.toLocaleString('es-CO')} unidad
              {suggestedUnits !== 1 ? 'es' : ''} sugerida{suggestedUnits !== 1 ? 's' : ''}
            </span>
            <span>·</span>
            <span>Costo estimado: {formatMoney(estimatedCost)}</span>
            {itemsWithoutCost > 0 && (
              <>
                <span>·</span>
                <span className="suppliers-sub">
                  {itemsWithoutCost} sin costo
                </span>
              </>
            )}
          </div>

          <div className="supplier-restock-table-wrap">
            <table className="supplier-restock-table">
              <thead>
                <tr>
                  <th>Producto</th>
                  <th>Stock</th>
                  <th className="suppliers-num">Vendidas</th>
                  <th>Cobertura</th>
                  <th>Urgencia</th>
                  <th className="suppliers-num">Sugerida</th>
                  <th className="suppliers-num">Costo unitario</th>
                  <th className="suppliers-num">Costo línea</th>
                </tr>
              </thead>
              <tbody>
                {items.map(it => {
                  const attrs = formatAttributes(it.attributes as Record<string, string>);
                  const urgencyLabel = RESTOCK_URGENCY_LABELS[it.urgency];
                  const urgencyClass = restockUrgencyBadgeClass(it.urgency);

                  return (
                    <tr key={it.variant_id}>
                      <td>
                        <strong>{it.product_name}</strong>
                        {attrs && <span className="suppliers-sub">{attrs}</span>}
                        {it.category_name && (
                          <span className="suppliers-sub">{it.category_name}</span>
                        )}
                      </td>
                      <td>
                        <StockBadge stock={it.stock} stockMin={it.stock_min} />
                      </td>
                      <td className="suppliers-num">{it.units_sold_window}</td>
                      <td>{formatCover(it.days_of_cover !== null ? Number(it.days_of_cover) : null)}</td>
                      <td>
                        <span className={urgencyClass}>{urgencyLabel}</span>
                      </td>
                      <td className="suppliers-num">
                        <strong>{it.suggested_qty}</strong>
                      </td>
                      <td className="suppliers-num">
                        {it.unit_cost !== null ? formatMoney(it.unit_cost) : 'Sin costo'}
                      </td>
                      <td className="suppliers-num">
                        {it.line_cost !== null ? formatMoney(it.line_cost) : '—'}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </>
      )}
    </div>
  );
};
