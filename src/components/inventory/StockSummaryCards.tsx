import React from 'react';
import type { InventoryValueDto } from '../../types';
import { formatMoney } from '../../utils/format';
import './StockSummaryCards.css';

interface StockSummaryCardsProps {
  value: InventoryValueDto | null;
  lowCount: number;
  outCount: number;
}

export const StockSummaryCards: React.FC<StockSummaryCardsProps> = ({
  value,
  lowCount,
  outCount,
}) => {
  const dash = '—';

  return (
    <div className="stock-summary">
      <div className="stock-card stock-card--primary">
        <span className="stock-card-label">Valor del inventario</span>
        <span className="stock-card-value">
          {value ? formatMoney(value.value_at_cost) : dash}
        </span>
        <span className="stock-card-sub">a costo</span>
        {value && value.variants_without_cost > 0 && (
          <span className="stock-card-warn">
            ⚠ {value.variants_without_cost} variante
            {value.variants_without_cost !== 1 ? 's' : ''} sin costo (no incluida
            {value.variants_without_cost !== 1 ? 's' : ''})
          </span>
        )}
      </div>

      <div className="stock-card">
        <span className="stock-card-label">Valor potencial de venta</span>
        <span className="stock-card-value">
          {value ? formatMoney(value.potential_sale_value) : dash}
        </span>
        <span className="stock-card-sub">
          {value ? `${value.variants_valued} variantes valoradas` : ' '}
        </span>
        {value && value.negative_stock_variants > 0 && (
          <span className="stock-card-warn">
            {value.negative_stock_variants} con stock negativo (aportan $0)
          </span>
        )}
      </div>

      <div className="stock-card">
        <span className="stock-card-label">Stock bajo</span>
        <span className="stock-card-value">{lowCount}</span>
        <span className="stock-card-sub">variantes</span>
      </div>

      <div className="stock-card">
        <span className="stock-card-label">Agotadas</span>
        <span className="stock-card-value">{outCount}</span>
        <span className="stock-card-sub">variantes</span>
      </div>
    </div>
  );
};
