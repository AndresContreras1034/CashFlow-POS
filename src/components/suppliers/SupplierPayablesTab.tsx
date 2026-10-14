import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { getDebtOptions } from '../../services/debts.service';
import { getSupplierPayables } from '../../services/suppliers.service';
import type { DebtOptions } from '../../types/debts';
import type { SupplierPayables } from '../../types/suppliers';
import { errorMessage } from '../../utils/audit';
import { formatDay, labelOf } from '../../utils/debts';
import { formatMoney } from '../../utils/format';
import { DueBadge, StatusBadge } from '../debts/DebtBadges';
import './SuppliersBase.css';
import './SupplierPayablesTab.css';

interface SupplierPayablesTabProps {
  supplierId: number;
}

export const SupplierPayablesTab: React.FC<SupplierPayablesTabProps> = ({ supplierId }) => {
  const navigate = useNavigate();
  const [data, setData] = useState<SupplierPayables | null>(null);
  const [options, setOptions] = useState<DebtOptions | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retryKey, setRetryKey] = useState(0);
  const requestRef = useRef(0);

  const load = useCallback(async () => {
    const requestId = ++requestRef.current;
    setLoading(true);
    setError(null);
    try {
      const [payables, debtOptions] = await Promise.all([
        getSupplierPayables(supplierId),
        getDebtOptions(),
      ]);
      if (requestId !== requestRef.current) return;
      setData(payables);
      setOptions(debtOptions);
      setLoading(false);
    } catch (cause) {
      if (requestId !== requestRef.current) return;
      setError(errorMessage(cause));
      setData(null);
      setOptions(null);
      setLoading(false);
    }
  }, [supplierId]);

  useEffect(() => {
    void load();
    return () => {
      requestRef.current += 1;
    };
  }, [load, retryKey]);

  if (loading) {
    return <p className="supplier-payables-status" role="status">Cargando deudas del proveedor…</p>;
  }

  if (error) {
    return (
      <div className="supplier-payables-error" role="alert">
        <span>No se pudieron cargar las deudas: {error}</span>
        <button
          type="button"
          className="suppliers-btn suppliers-btn-primary"
          onClick={() => setRetryKey(value => value + 1)}
        >
          Reintentar
        </button>
      </div>
    );
  }

  if (!data || !options) return null;

  return (
    <section className="supplier-payables-tab" aria-label="Deudas del proveedor">
      <div className="supplier-payables-summary">
        <article className="supplier-payables-metric">
          <span>Saldo pendiente</span>
          <strong>{formatMoney(data.summary.pending_total)}</strong>
        </article>
        <article
          className={`supplier-payables-metric ${
            data.summary.overdue_total > 0 ? 'supplier-payables-metric--overdue' : ''
          }`}
        >
          <span>Vencido</span>
          <strong>{formatMoney(data.summary.overdue_total)}</strong>
        </article>
        <article className="supplier-payables-metric">
          <span>Próximo vencimiento</span>
          <strong>{formatDay(data.summary.next_due_on) || '—'}</strong>
        </article>
        <p className="supplier-payables-count" role="status">
          {data.summary.active_count} deuda{data.summary.active_count === 1 ? '' : 's'} activa
          {data.summary.active_count === 1 ? '' : 's'}
        </p>
      </div>

      <p className="supplier-payables-match">
        Se cruzan por el nombre del acreedor: debe coincidir con «{data.matched_name}» (sin importar
        mayúsculas ni espacios sobrantes).
      </p>

      <div className="supplier-payables-toolbar">
        <button
          type="button"
          className="suppliers-btn"
          onClick={() => navigate(`/debts?q=${encodeURIComponent(data.matched_name)}`)}
        >
          Ver en Deudas
        </button>
      </div>

      {data.debts.length === 0 ? (
        <p className="supplier-payables-empty">No hay deudas activas con este proveedor.</p>
      ) : (
        <>
          {data.debts_truncated && (
            <p className="supplier-payables-truncated">Mostrando las 20 más próximas.</p>
          )}
          <div className="supplier-payables-table-wrap">
            <table className="supplier-payables-table">
              <thead>
                <tr>
                  <th scope="col">Concepto</th>
                  <th scope="col">Vencimiento</th>
                  <th scope="col" className="suppliers-num">Saldo</th>
                  <th scope="col">Estado</th>
                </tr>
              </thead>
              <tbody>
                {data.debts.map(debt => (
                  <tr key={debt.id}>
                    <td>
                      <strong>{debt.concept}</strong>
                      {debt.document_ref && (
                        <span className="supplier-payables-ref">Ref. {debt.document_ref}</span>
                      )}
                    </td>
                    <td>
                      <span className="supplier-payables-due-date">{formatDay(debt.due_on)}</span>
                      <DueBadge debt={debt} />
                    </td>
                    <td className="suppliers-num tabular">{formatMoney(debt.balance)}</td>
                    <td>
                      <StatusBadge
                        status={debt.status}
                        label={labelOf(options.statuses, debt.status)}
                      />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </section>
  );
};
