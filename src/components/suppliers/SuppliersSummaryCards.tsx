import React from 'react';
import './SuppliersSummaryCards.css';

export interface RestockSummaryData {
  suppliersToContact: number;
  urgentCount: number;
  unassignedCount: number;
}

interface SuppliersSummaryCardsProps {
  activeCount: number | null;
  activeError: string | null;
  onRetryActive: () => void;
  restockSummary: RestockSummaryData | null;
  restockError: string | null;
  onRetryRestock: () => void;
}

export const SuppliersSummaryCards: React.FC<SuppliersSummaryCardsProps> = ({
  activeCount,
  activeError,
  onRetryActive,
  restockSummary,
  restockError,
  onRetryRestock,
}) => {
  const dash = '—';

  return (
    <section className="suppliers-summary" aria-label="Resumen de proveedores">
      <div className="suppliers-card suppliers-card--primary">
        <span className="suppliers-card-label">Proveedores activos</span>
        <span className="suppliers-card-value">
          {activeCount !== null ? activeCount.toLocaleString('es-CO') : dash}
        </span>
        <span className="suppliers-card-sub">en el catálogo</span>
      </div>

      <div className="suppliers-card">
        <span className="suppliers-card-label">Por contactar</span>
        <span className="suppliers-card-value">
          {restockSummary !== null
            ? restockSummary.suppliersToContact.toLocaleString('es-CO')
            : dash}
        </span>
        <span className="suppliers-card-sub">con productos por reponer</span>
      </div>

      <div
        className={`suppliers-card ${
          restockSummary && restockSummary.urgentCount > 0
            ? 'suppliers-card--urgent'
            : ''
        }`}
      >
        <span className="suppliers-card-label">Urgentes</span>
        <span className="suppliers-card-value">
          {restockSummary !== null
            ? restockSummary.urgentCount.toLocaleString('es-CO')
            : dash}
        </span>
        <span className="suppliers-card-sub">agotados + críticos</span>
      </div>

      <div
        className={`suppliers-card ${
          restockSummary && restockSummary.unassignedCount > 0
            ? 'suppliers-card--warning'
            : ''
        }`}
      >
        <span className="suppliers-card-label">Sin proveedor</span>
        <span className="suppliers-card-value">
          {restockSummary !== null
            ? restockSummary.unassignedCount.toLocaleString('es-CO')
            : dash}
        </span>
        <span className="suppliers-card-sub">productos sin a quién pedirle</span>
      </div>

      {activeError && (
        <div className="suppliers-summary-error" role="alert">
          <span>No se pudo cargar el conteo de proveedores activos: {activeError}</span>
          <button
            type="button"
            className="suppliers-btn suppliers-btn-primary"
            onClick={onRetryActive}
          >
            Reintentar
          </button>
        </div>
      )}

      {restockError && (
        <div className="suppliers-summary-error" role="alert">
          <span>No se pudo cargar el resumen de reposición: {restockError}</span>
          <button
            type="button"
            className="suppliers-btn suppliers-btn-primary"
            onClick={onRetryRestock}
          >
            Reintentar
          </button>
        </div>
      )}
    </section>
  );
};
