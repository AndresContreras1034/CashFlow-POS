import React from 'react';
import type { DebtOptions, DebtOverview } from '../../types/debts';
import { canPay, formatDay, labelOf } from '../../utils/debts';
import { formatMoney } from '../../utils/format';
import { DueBadge, StatusBadge } from './DebtBadges';
import './DebtList.css';

interface DebtListProps {
  debts: DebtOverview[];
  options: DebtOptions;
  onOpen: (debt: DebtOverview) => void;
  onPay: (debt: DebtOverview) => void;
}

export const DebtList: React.FC<DebtListProps> = ({ debts, options, onOpen, onPay }) => (
  <div className="debts-table-wrap">
    <table className="debts-table">
      <caption className="debts-sr-only">Obligaciones por pagar</caption>
      <thead>
        <tr>
          <th scope="col">Acreedor y concepto</th>
          <th scope="col">Categoría</th>
          <th scope="col">Emisión</th>
          <th scope="col">Vencimiento</th>
          <th scope="col" className="debts-num">Importe</th>
          <th scope="col" className="debts-num">Saldo</th>
          <th scope="col">Estado</th>
          <th scope="col"><span className="debts-sr-only">Acciones</span></th>
        </tr>
      </thead>
      <tbody>
        {debts.map(debt => {
          const category = options.categories.find(item => item.value === debt.category);
          return (
            <tr key={debt.id} className={debt.status === 'voided' ? 'debts-row--voided' : undefined}>
              <td className="debts-main">
                <button type="button" className="debts-link" onClick={() => onOpen(debt)}>
                  {debt.creditor_name}
                </button>
                <span className="debts-sub">{debt.concept}</span>
                {debt.document_ref && <span className="debts-sub">Ref. {debt.document_ref}</span>}
              </td>
              <td>
                {category?.label ?? debt.category}
                {debt.subcategory && <span className="debts-sub">{debt.subcategory}</span>}
              </td>
              <td className="debts-nowrap">{formatDay(debt.issued_on)}</td>
              <td>
                <span className="debts-nowrap">{formatDay(debt.due_on)}</span>
                <DueBadge debt={debt} />
              </td>
              <td className="debts-num tabular">{formatMoney(debt.original_amount)}</td>
              <td className="debts-num tabular">{formatMoney(debt.balance)}</td>
              <td>
                <StatusBadge status={debt.status} label={labelOf(options.statuses, debt.status)} />
                {debt.status === 'overdue' && debt.payment_count > 0 && (
                  <span className="debts-chip">con abonos</span>
                )}
              </td>
              <td className="debts-actions">
                {canPay(debt) && (
                  <button type="button" className="btn btn-ghost" onClick={() => onPay(debt)}>
                    Abonar
                  </button>
                )}
                <button type="button" className="btn btn-ghost" onClick={() => onOpen(debt)}>
                  Ver
                </button>
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  </div>
);