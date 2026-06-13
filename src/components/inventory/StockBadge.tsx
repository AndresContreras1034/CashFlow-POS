import React from 'react';
import type { StockStatus } from '../../types';
import './StockBadge.css';

interface StockBadgeProps { stock: number; stockMin: number; }

const getStatus = (stock: number, stockMin: number): StockStatus => {
  if (stock <= 0) return 'out_of_stock';
  if (stock <= stockMin) return 'low';
  return 'ok';
};

const CONFIG: Record<StockStatus, { label: string; className: string }> = {
  ok:           { label: 'OK',      className: 'badge--ok'  },
  low:          { label: 'Bajo',    className: 'badge--low' },
  out_of_stock: { label: 'Agotado', className: 'badge--out' },
};

export const StockBadge: React.FC<StockBadgeProps> = ({ stock, stockMin }) => {
  const config = CONFIG[getStatus(stock, stockMin)];
  return (
    <span className={`stock-badge ${config.className}`}>
      <span className="stock-badge-dot" />
      {stock} — {config.label}
    </span>
  );
};