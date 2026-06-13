import React from 'react';
import type { MovementWithDetails } from '../../types';
import { formatDate, formatMoney, formatAttributes, formatMovementType } from '../../utils/format';
import './KardexTable.css';

interface KardexTableProps { movements: MovementWithDetails[]; }

const movClass = (type: string) =>
  ['purchase','manual_in','initial_stock','sale_return'].includes(type) ? 'mov-in'
  : ['sale','manual_out'].includes(type) ? 'mov-out'
  : 'mov-adj';

const movSign = (type: string) =>
  ['purchase','manual_in','initial_stock','sale_return'].includes(type) ? '+'
  : ['sale','manual_out'].includes(type) ? '−'
  : '±';

export const KardexTable: React.FC<KardexTableProps> = ({ movements }) => {
  if (!movements.length) return (
    <div className="kardex-empty">Sin movimientos en este rango</div>
  );

  return (
    <div className="kardex-table-wrap">
      <table className="kardex-table">
        <thead>
          <tr>
            <th>Fecha</th>
            <th>Tipo</th>
            <th>Producto / Variante</th>
            <th className="col-right">Cant.</th>
            <th className="col-right">Antes</th>
            <th className="col-right">Después</th>
            <th className="col-right">Costo u.</th>
            <th>Notas</th>
            <th>Usuario</th>
          </tr>
        </thead>
        <tbody>
          {movements.map(m => {
            const cls = movClass(m.movement_type);
            return (
              <tr key={m.id} className="kardex-row">
                <td className="cell-date">{formatDate(m.created_at)}</td>
                <td><span className={`movement-tag ${cls}`}>{formatMovementType(m.movement_type)}</span></td>
                <td className="cell-product">
                  <span className="product-name">{m.product_name}</span>
                  {Object.keys(m.attributes).length > 0 && (
                    <span className="product-attrs">{formatAttributes(m.attributes)}</span>
                  )}
                </td>
                <td className={`cell-qty ${cls} tabular`}>{movSign(m.movement_type)}{Math.abs(m.quantity)}</td>
                <td className="cell-stock tabular">{m.stock_before}</td>
                <td className="cell-stock tabular">{m.stock_after}</td>
                <td className="cell-cost tabular">{m.unit_cost > 0 ? formatMoney(m.unit_cost) : '—'}</td>
                <td className="cell-notes">{m.notes ?? '—'}</td>
                <td className="cell-user">{m.created_by}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
};