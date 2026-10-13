import React, { useCallback, useEffect, useRef, useState } from 'react';
import { getCustomer, setCustomerActive } from '../../services/customers.service';
import { listSales } from '../../services/sales.service';
import type { Sale, SaleStatus } from '../../types';
import type { CustomerOverview } from '../../types/customers';
import { errorMessage } from '../../utils/audit';
import { formatDate, formatMoney } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import './CustomerDetail.css';

const STATUS_LABEL: Record<SaleStatus, string> = {
  completed: 'Completada',
  cancelled: 'Cancelada',
  refunded: 'Reembolsada',
};

const HISTORY_PAGE_SIZE = 10;

interface CustomerDetailProps {
  customerId: number;
  reloadKey: number;
  onEdit: (customer: CustomerOverview) => void;
  onChanged: () => void;
  onNotify: (message: string, type?: 'success' | 'error') => void;
}

export const CustomerDetail: React.FC<CustomerDetailProps> = ({
  customerId,
  reloadKey,
  onEdit,
  onChanged,
  onNotify,
}) => {
  const [customer, setCustomer] = useState<CustomerOverview | null>(null);
  const [loadingCustomer, setLoadingCustomer] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [sales, setSales] = useState<Sale[]>([]);
  const [salesTotal, setSalesTotal] = useState(0);
  const [salesPage, setSalesPage] = useState(1);
  const [salesError, setSalesError] = useState<string | null>(null);
  const [salesLoading, setSalesLoading] = useState(true);
  const [confirmingToggle, setConfirmingToggle] = useState(false);
  const [toggling, setToggling] = useState(false);
  const customerRequest = useRef(0);
  const salesRequest = useRef(0);

  const loadCustomer = useCallback(() => {
    const requestId = ++customerRequest.current;
    setLoadingCustomer(true);
    setError(null);
    getCustomer(customerId)
      .then(nextCustomer => {
        if (requestId !== customerRequest.current) return;
        setCustomer(nextCustomer);
        setLoadingCustomer(false);
      })
      .catch(cause => {
        if (requestId !== customerRequest.current) return;
        setError(errorMessage(cause));
        setLoadingCustomer(false);
      });
  }, [customerId]);

  useEffect(() => {
    loadCustomer();
    return () => {
      customerRequest.current += 1;
    };
  }, [loadCustomer, reloadKey]);

  useEffect(() => {
    const requestId = ++salesRequest.current;
    setSalesLoading(true);
    setSalesError(null);
    listSales({ customer_id: customerId, page: salesPage, page_size: HISTORY_PAGE_SIZE })
      .then(response => {
        if (requestId !== salesRequest.current) return;
        setSales(response.data);
        setSalesTotal(response.total);
        setSalesLoading(false);
      })
      .catch(cause => {
        if (requestId !== salesRequest.current) return;
        setSalesError(errorMessage(cause));
        setSalesLoading(false);
      });
    return () => {
      salesRequest.current += 1;
    };
  }, [customerId, salesPage]);

  const toggleActive = async () => {
    if (!customer || toggling) return;
    setToggling(true);
    try {
      await setCustomerActive(customer.id, {
        is_active: !customer.is_active,
        ...actorField('updated_by'),
      });
      setConfirmingToggle(false);
      loadCustomer();
      onChanged();
      onNotify(customer.is_active ? 'Cliente desactivado' : 'Cliente reactivado');
    } catch (cause) {
      onNotify(errorMessage(cause), 'error');
    } finally {
      setToggling(false);
    }
  };

  if (error) return <div className="customer-detail-error" role="alert">{error}</div>;
  if (loadingCustomer || !customer) {
    return <p className="customer-detail-muted">Cargando cliente…</p>;
  }

  const totalPages = Math.max(1, Math.ceil(salesTotal / HISTORY_PAGE_SIZE));
  const document =
    customer.document_type && customer.document_number
      ? `${customer.document_type} ${customer.document_number}`
      : '—';

  return (
    <div className="customer-detail">
      <div className="customer-detail-head">
        <h3>{customer.name}</h3>
        <span className={customer.is_active ? 'customers-badge customers-badge-ok' : 'customers-badge'}>
          {customer.is_active ? 'Activo' : 'Inactivo'}
        </span>
      </div>

      <div className="customer-stats">
        <div>
          <span>Compras</span>
          <strong>{customer.sales_count}</strong>
        </div>
        <div>
          <span>Total comprado</span>
          <strong>{formatMoney(customer.total_spent)}</strong>
        </div>
        <div>
          <span>Última compra</span>
          <strong>
            {customer.last_purchase_at ? formatDate(customer.last_purchase_at) : '—'}
          </strong>
        </div>
      </div>

      <dl className="customer-dl">
        <dt>Documento</dt><dd>{document}</dd>
        <dt>Teléfono</dt><dd>{customer.phone ?? '—'}</dd>
        <dt>Email</dt><dd>{customer.email ?? '—'}</dd>
        <dt>Dirección</dt><dd>{customer.address ?? '—'}</dd>
        <dt>Notas</dt><dd>{customer.notes ?? '—'}</dd>
        <dt>Registrado</dt>
        <dd>{formatDate(customer.created_at)} por {customer.created_by}</dd>
      </dl>

      <div className="customer-detail-actions">
        <button type="button" className="customers-btn" onClick={() => onEdit(customer)}>
          Editar
        </button>
        {!confirmingToggle ? (
          <button
            type="button"
            className={customer.is_active ? 'customers-btn customers-btn-danger' : 'customers-btn'}
            onClick={() => setConfirmingToggle(true)}
          >
            {customer.is_active ? 'Desactivar' : 'Reactivar'}
          </button>
        ) : (
          <>
            <span className="customer-detail-muted">
              {customer.is_active
                ? 'No podrá asignarse a nuevas ventas. Su historial se conserva.'
                : '¿Reactivar este cliente?'}
            </span>
            <button
              type="button"
              className="customers-btn customers-btn-primary"
              onClick={() => void toggleActive()}
              disabled={toggling}
            >
              {toggling ? 'Procesando…' : 'Confirmar'}
            </button>
            <button
              type="button"
              className="customers-btn"
              onClick={() => setConfirmingToggle(false)}
              disabled={toggling}
            >
              Cancelar
            </button>
          </>
        )}
      </div>

      <section className="customer-history">
        <h4>Historial de compras ({salesTotal})</h4>
        {salesError && <div className="customer-detail-error" role="alert">{salesError}</div>}
        {salesLoading && <p className="customer-detail-muted">Cargando compras…</p>}
        {!salesLoading && !salesError && sales.length === 0 && (
          <p className="customer-detail-muted">Este cliente aún no tiene compras.</p>
        )}
        {sales.length > 0 && (
          <>
            <div className="customer-history-wrap">
              <table className="customer-history-table">
                <thead>
                  <tr>
                    <th>#</th>
                    <th>Fecha</th>
                    <th>Estado</th>
                    <th className="customers-num">Total</th>
                  </tr>
                </thead>
                <tbody>
                  {sales.map(sale => (
                    <tr key={sale.id}>
                      <td>#{sale.id}</td>
                      <td>{formatDate(sale.created_at)}</td>
                      <td>{STATUS_LABEL[sale.status]}</td>
                      <td className="customers-num">{formatMoney(sale.total)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {totalPages > 1 && (
              <div className="customer-history-pager">
                <button
                  type="button"
                  className="customers-btn"
                  disabled={salesPage <= 1}
                  onClick={() => setSalesPage(page => page - 1)}
                >
                  ← Anterior
                </button>
                <span>Página {salesPage} de {totalPages}</span>
                <button
                  type="button"
                  className="customers-btn"
                  disabled={salesPage >= totalPages}
                  onClick={() => setSalesPage(page => page + 1)}
                >
                  Siguiente →
                </button>
              </div>
            )}
          </>
        )}
      </section>
    </div>
  );
};
