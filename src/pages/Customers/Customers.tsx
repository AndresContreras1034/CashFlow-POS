import { useCallback, useEffect, useRef, useState } from 'react';
import { CustomerDetail } from '../../components/customers/CustomerDetail';
import { CustomerForm } from '../../components/customers/CustomerForm';
import { Modal } from '../../components/ui/Modal';
import { ToastContainer, type ToastData, type ToastType } from '../../components/ui/Toast';
import { useDebounce } from '../../hooks/useDebounce';
import { listCustomers } from '../../services/customers.service';
import type { PaginatedResponse } from '../../types';
import type { CustomerOverview } from '../../types/customers';
import { errorMessage } from '../../utils/audit';
import { formatDate, formatMoney } from '../../utils/format';
import './Customers.css';

const PAGE_SIZE = 20;
type ActiveFilter = 'active' | 'inactive' | 'all';

type Dialog =
  | { kind: 'none' }
  | { kind: 'create' }
  | { kind: 'detail'; id: number }
  | { kind: 'edit'; customer: CustomerOverview };

export default function Customers() {
  const [searchInput, setSearchInput] = useState('');
  const search = useDebounce(searchInput.trim(), 400);
  const [activeFilter, setActiveFilter] = useState<ActiveFilter>('active');
  const [page, setPage] = useState(1);
  const [result, setResult] = useState<PaginatedResponse<CustomerOverview> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const requestRef = useRef(0);
  const [dialog, setDialog] = useState<Dialog>({ kind: 'none' });
  const [detailReload, setDetailReload] = useState(0);
  const [toasts, setToasts] = useState<ToastData[]>([]);

  const refresh = useCallback(() => setReloadToken(token => token + 1), []);
  const closeDialog = useCallback(() => setDialog({ kind: 'none' }), []);
  const dismissToast = useCallback(
    (id: string) => setToasts(current => current.filter(toast => toast.id !== id)),
    [],
  );
  const notify = useCallback((message: string, type: ToastType = 'success') => {
    const id = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    setToasts(current => [...current, { id, message, type }]);
  }, []);

  useEffect(() => {
    setPage(1);
  }, [search, activeFilter]);

  useEffect(() => {
    const requestId = ++requestRef.current;
    setLoading(true);
    setError(null);
    listCustomers({
      search: search || undefined,
      is_active: activeFilter === 'all' ? undefined : activeFilter === 'active',
      page,
      page_size: PAGE_SIZE,
    })
      .then(response => {
        if (requestId !== requestRef.current) return;
        if (response.data.length === 0 && page > 1 && page > response.total_pages) {
          setPage(Math.max(response.total_pages, 1));
          return;
        }
        setResult(response);
        setLoading(false);
      })
      .catch(cause => {
        if (requestId !== requestRef.current) return;
        setResult(null);
        setError(errorMessage(cause));
        setLoading(false);
      });
    return () => {
      requestRef.current += 1;
    };
  }, [search, activeFilter, page, reloadToken]);

  const customers = result?.data ?? [];
  const totalPages = Math.max(result?.total_pages ?? 0, 1);
  const filtersActive = search !== '' || activeFilter !== 'active';

  let status = '';
  if (loading) status = 'Cargando…';
  else if (error) status = 'No se pudieron cargar los clientes';
  else if (result) {
    status = `${result.total.toLocaleString('es-CO')} cliente${result.total !== 1 ? 's' : ''}`;
  }

  return (
    <div className="customers-page">
      <header className="customers-header">
        <div>
          <h1 className="customers-title">Clientes</h1>
          <p className="customers-subtitle">Directorio y compras por cliente</p>
        </div>
        <button
          type="button"
          className="customers-btn customers-btn-primary"
          onClick={() => setDialog({ kind: 'create' })}
        >
          Nuevo cliente
        </button>
      </header>

      <section className="customers-filters" aria-label="Filtros de clientes">
        <div className="customers-field customers-field-grow">
          <label htmlFor="customers-search">Buscar</label>
          <input
            id="customers-search"
            className="customers-input"
            type="text"
            maxLength={100}
            placeholder="Nombre, documento, teléfono o email…"
            value={searchInput}
            onChange={event => setSearchInput(event.target.value)}
          />
        </div>
        <div className="customers-field">
          <label htmlFor="customers-active">Estado</label>
          <select
            id="customers-active"
            className="customers-input"
            value={activeFilter}
            onChange={event => setActiveFilter(event.target.value as ActiveFilter)}
          >
            <option value="active">Activos</option>
            <option value="inactive">Inactivos</option>
            <option value="all">Todos</option>
          </select>
        </div>
        <button
          type="button"
          className="customers-btn"
          disabled={!filtersActive}
          onClick={() => {
            setSearchInput('');
            setActiveFilter('active');
          }}
        >
          Limpiar
        </button>
      </section>

      <p className="customers-status" role="status" aria-live="polite">{status}</p>

      {error && (
        <div className="customers-error" role="alert">
          <p>{error}</p>
          <button type="button" className="customers-btn customers-btn-primary" onClick={refresh}>
            Reintentar
          </button>
        </div>
      )}

      {!error && !loading && customers.length === 0 && (
        <div className="customers-empty">
          {filtersActive ? (
            'Ningún cliente coincide con los filtros.'
          ) : (
            <>
              <p>Aún no hay clientes registrados.</p>
              <button
                type="button"
                className="customers-btn customers-btn-primary"
                onClick={() => setDialog({ kind: 'create' })}
              >
                Registrar el primer cliente
              </button>
            </>
          )}
        </div>
      )}

      {customers.length > 0 && (
        <div className={loading ? 'customers-results customers-results--loading' : 'customers-results'}>
          <div className="customers-table-wrap">
            <table className="customers-table">
              <thead>
                <tr>
                  <th>Cliente</th>
                  <th>Contacto</th>
                  <th className="customers-num">Compras</th>
                  <th className="customers-num">Total comprado</th>
                  <th>Última compra</th>
                  <th>Estado</th>
                </tr>
              </thead>
              <tbody>
                {customers.map(customer => (
                  <tr
                    key={customer.id}
                    className={
                      customer.is_active
                        ? 'customers-row'
                        : 'customers-row customers-row--inactive'
                    }
                    onClick={() => setDialog({ kind: 'detail', id: customer.id })}
                  >
                    <td>
                      <button
                        type="button"
                        className="customers-link"
                        onClick={event => {
                          event.stopPropagation();
                          setDialog({ kind: 'detail', id: customer.id });
                        }}
                      >
                        {customer.name}
                      </button>
                      {customer.document_type && customer.document_number && (
                        <span className="customers-sub">
                          {customer.document_type} {customer.document_number}
                        </span>
                      )}
                    </td>
                    <td>
                      {customer.phone ?? '—'}
                      {customer.email && <span className="customers-sub">{customer.email}</span>}
                    </td>
                    <td className="customers-num">{customer.sales_count}</td>
                    <td className="customers-num">{formatMoney(customer.total_spent)}</td>
                    <td>
                      {customer.last_purchase_at ? formatDate(customer.last_purchase_at) : '—'}
                    </td>
                    <td>
                      <span
                        className={
                          customer.is_active
                            ? 'customers-badge customers-badge-ok'
                            : 'customers-badge'
                        }
                      >
                        {customer.is_active ? 'Activo' : 'Inactivo'}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <nav className="customers-pagination" aria-label="Paginación">
        <button
          type="button"
          className="customers-btn"
          disabled={loading || page <= 1}
          onClick={() => setPage(current => current - 1)}
        >
          ← Anterior
        </button>
        <span>Página {page} de {totalPages}</span>
        <button
          type="button"
          className="customers-btn"
          disabled={loading || !result || page >= totalPages}
          onClick={() => setPage(current => current + 1)}
        >
          Siguiente →
        </button>
      </nav>

      {dialog.kind === 'create' && (
        <Modal title="Nuevo cliente" onClose={closeDialog} width={640}>
          <CustomerForm
            initial={null}
            onCancel={closeDialog}
            onSaved={saved => {
              closeDialog();
              refresh();
              notify(`Cliente «${saved.name}» registrado`);
            }}
          />
        </Modal>
      )}

      {dialog.kind === 'edit' && (
        <Modal title={`Editar cliente #${dialog.customer.id}`} onClose={closeDialog} width={640}>
          <CustomerForm
            initial={dialog.customer}
            onCancel={() => setDialog({ kind: 'detail', id: dialog.customer.id })}
            onSaved={saved => {
              refresh();
              setDetailReload(token => token + 1);
              notify(`Cliente «${saved.name}» actualizado`);
              setDialog({ kind: 'detail', id: saved.id });
            }}
          />
        </Modal>
      )}

      {dialog.kind === 'detail' && (
        <Modal title={`Cliente #${dialog.id}`} onClose={closeDialog} width={820}>
          <CustomerDetail
            key={dialog.id}
            customerId={dialog.id}
            reloadKey={detailReload}
            onEdit={customer => setDialog({ kind: 'edit', customer })}
            onChanged={refresh}
            onNotify={notify}
          />
        </Modal>
      )}

      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
}
