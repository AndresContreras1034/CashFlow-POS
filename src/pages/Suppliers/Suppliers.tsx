import { type KeyboardEvent, useCallback, useEffect, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { SupplierDetail } from '../../components/suppliers/SupplierDetail';
import { SupplierForm } from '../../components/suppliers/SupplierForm';
import { SupplierCallPanel } from '../../components/suppliers/SupplierCallPanel';
import {
  SuppliersSummaryCards,
  type RestockSummaryData,
} from '../../components/suppliers/SuppliersSummaryCards';
import { Modal } from '../../components/ui/Modal';
import { ToastContainer, type ToastData, type ToastType } from '../../components/ui/Toast';
import { useDebounce } from '../../hooks/useDebounce';
import { useSettings } from '../../context/AppContext';
import {
  getRestockOverview,
  listSuppliers,
} from '../../services/suppliers.service';
import type { PaginatedResponse } from '../../types';
import type {
  SupplierOverview,
  SupplierPriority,
} from '../../types/suppliers';
import type { ContactAction } from '../../utils/callPanel';
import { openExternal } from '../../utils/openExternal';
import { errorMessage } from '../../utils/audit';
import {
  formatSupplierConditions,
  priorityBadgeClass,
  priorityLabel,
} from '../../utils/suppliers';
import './Suppliers.css';

const PAGE_SIZE = 20;
type ActiveFilter = 'active' | 'inactive' | 'all';
type SupplierView = 'directory' | 'call';

type Dialog =
  | { kind: 'none' }
  | { kind: 'create' }
  | {
      kind: 'detail';
      id: number;
      initialTab?: 'data' | 'categories' | 'products' | 'restock';
      fromCallPanel?: boolean;
    }
  | { kind: 'edit'; supplier: SupplierOverview };

export default function Suppliers() {
  const [searchParams, setSearchParams] = useSearchParams();
  const view: SupplierView = searchParams.get('view') === 'call' ? 'call' : 'directory';
  const { settings } = useSettings();
  const [searchInput, setSearchInput] = useState('');
  const search = useDebounce(searchInput.trim(), 400);
  const [activeFilter, setActiveFilter] = useState<ActiveFilter>('active');
  const [priorityFilter, setPriorityFilter] = useState<SupplierPriority | ''>('');
  const [page, setPage] = useState(1);
  const [result, setResult] = useState<PaginatedResponse<SupplierOverview> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const requestRef = useRef(0);

  const [activeCount, setActiveCount] = useState<number | null>(null);
  const [activeError, setActiveError] = useState<string | null>(null);
  const [restockSummary, setRestockSummary] = useState<RestockSummaryData | null>(null);
  const [restockError, setRestockError] = useState<string | null>(null);

  const [dialog, setDialog] = useState<Dialog>({ kind: 'none' });
  const [detailReload, setDetailReload] = useState(0);
  const [toasts, setToasts] = useState<ToastData[]>([]);
  const [initialOnlyUrgent, setInitialOnlyUrgent] = useState(false);
  const [callPanelReload, setCallPanelReload] = useState(0);

  const refresh = useCallback(() => setReloadToken(token => token + 1), []);
  const closeDialog = useCallback(() => {
    if (dialog.kind === 'detail' && dialog.fromCallPanel) {
      setCallPanelReload(token => token + 1);
    }
    setDialog({ kind: 'none' });
  }, [dialog]);

  const changeView = useCallback((nextView: SupplierView) => {
    setSearchParams(current => {
      const next = new URLSearchParams(current);
      if (nextView === 'directory') next.delete('view');
      else next.set('view', nextView);
      return next;
    });
  }, [setSearchParams]);

  const openCallPanel = useCallback((onlyUrgent: boolean) => {
    setInitialOnlyUrgent(onlyUrgent);
    changeView('call');
  }, [changeView]);

  const handleViewTabKeyDown = useCallback((event: KeyboardEvent<HTMLDivElement>) => {
    let nextView: SupplierView | null = null;
    if (event.key === 'ArrowRight' || event.key === 'ArrowLeft') {
      nextView = view === 'directory' ? 'call' : 'directory';
    } else if (event.key === 'Home') {
      nextView = 'directory';
    } else if (event.key === 'End') {
      nextView = 'call';
    }
    if (!nextView) return;
    event.preventDefault();
    changeView(nextView);
    document.getElementById(`suppliers-view-${nextView}`)?.focus();
  }, [changeView, view]);

  const dismissToast = useCallback(
    (id: string) => setToasts(current => current.filter(toast => toast.id !== id)),
    [],
  );

  const notify = useCallback((message: string, type: ToastType = 'success') => {
    const id = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    setToasts(current => [...current, { id, message, type }]);
  }, []);

  const handleAction = useCallback(async (action: ContactAction) => {
    if (action.url === null) return;
    const result = await openExternal(action.url);
    if (!result.ok) notify(`No se pudo abrir ${action.label}: ${result.error}`, 'error');
  }, [notify]);

  const handleCopy = useCallback(async (text: string, what: string) => {
    try {
      if (!navigator.clipboard) throw new Error('El portapapeles no está disponible');
      await navigator.clipboard.writeText(text);
      notify(`${what} copiado`);
    } catch {
      notify('No se pudo copiar', 'error');
    }
  }, [notify]);

  const loadActiveCount = useCallback(async () => {
    setActiveError(null);
    try {
      const activeRes = await listSuppliers({ is_active: true, page_size: 1 });
      setActiveCount(activeRes.total);
    } catch (cause) {
      setActiveError(errorMessage(cause));
    }
  }, []);

  const loadRestockSummary = useCallback(async () => {
    setRestockError(null);
    try {
      const restockRes = await getRestockOverview({});
      setRestockSummary({
        suppliersToContact: restockRes.summary.suppliers_to_contact,
        urgentCount:
          restockRes.summary.out_of_stock_count + restockRes.summary.critical_count,
        unassignedCount: restockRes.summary.unassigned_count,
      });
    } catch (cause) {
      setRestockError(errorMessage(cause));
    }
  }, []);

  const loadSummary = useCallback(() => {
    void loadActiveCount();
    void loadRestockSummary();
  }, [loadActiveCount, loadRestockSummary]);

  // Cargar resumen de tarjetas independientemente
  useEffect(() => {
    loadSummary();
  }, [loadSummary, reloadToken]);

  // Reset de página al cambiar filtros
  useEffect(() => {
    setPage(1);
  }, [search, activeFilter, priorityFilter]);

  // Carga de la lista de proveedores con protección contra respuestas obsoletas
  useEffect(() => {
    if (view !== 'directory') {
      setLoading(false);
      return;
    }
    const requestId = ++requestRef.current;
    setLoading(true);
    setError(null);

    listSuppliers({
      search: search || undefined,
      is_active: activeFilter === 'all' ? undefined : activeFilter === 'active',
      priority: priorityFilter || undefined,
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
  }, [search, activeFilter, priorityFilter, page, reloadToken, view]);

  const suppliers = result?.data ?? [];
  const totalPages = Math.max(result?.total_pages ?? 0, 1);
  const filtersActive =
    search !== '' || activeFilter !== 'active' || priorityFilter !== '';

  let statusText = '';
  if (loading) statusText = 'Cargando…';
  else if (error) statusText = 'No se pudieron cargar los proveedores';
  else if (result) {
    statusText = `${result.total.toLocaleString('es-CO')} proveedor${
      result.total !== 1 ? 'es' : ''
    }`;
  }

  const handleClearFilters = () => {
    setSearchInput('');
    setActiveFilter('active');
    setPriorityFilter('');
  };

  return (
    <div className="suppliers-page">
      <header className="suppliers-header">
        <div>
          <h1 className="suppliers-title">Proveedores</h1>
          <p className="suppliers-subtitle">
            Directorio de proveedores y reposición de stock
          </p>
        </div>
        <button
          type="button"
          className="suppliers-btn suppliers-btn-primary"
          onClick={() => setDialog({ kind: 'create' })}
        >
          Nuevo proveedor
        </button>
      </header>

      <SuppliersSummaryCards
        activeCount={activeCount}
        activeError={activeError}
        onRetryActive={loadActiveCount}
        restockSummary={restockSummary}
        restockError={restockError}
        onRetryRestock={loadRestockSummary}
        onOpenCallPanel={openCallPanel}
      />

      <div
        className="suppliers-view-tabs"
        role="tablist"
        aria-label="Vista de proveedores"
        onKeyDown={handleViewTabKeyDown}
      >
        <button
          type="button"
          role="tab"
          id="suppliers-view-directory"
          aria-selected={view === 'directory'}
          aria-controls="suppliers-view-panel"
          tabIndex={view === 'directory' ? 0 : -1}
          className="suppliers-view-tab"
          onClick={() => changeView('directory')}
        >
          Directorio
        </button>
        <button
          type="button"
          role="tab"
          id="suppliers-view-call"
          aria-selected={view === 'call'}
          aria-controls="suppliers-view-panel"
          tabIndex={view === 'call' ? 0 : -1}
          className="suppliers-view-tab"
          onClick={() => changeView('call')}
        >
          Para llamar
        </button>
      </div>

      {view === 'call' ? (
        <div
          id="suppliers-view-panel"
          role="tabpanel"
          aria-labelledby="suppliers-view-call"
        >
          <SupplierCallPanel
            initialOnlyUrgent={initialOnlyUrgent}
            reloadToken={callPanelReload}
            businessName={settings.business_name}
            onAction={handleAction}
            onCopy={handleCopy}
            onOpenSupplier={id => setDialog({
              kind: 'detail',
              id,
              initialTab: 'restock',
              fromCallPanel: true,
            })}
          />
        </div>
      ) : (
      <div id="suppliers-view-panel" role="tabpanel" aria-labelledby="suppliers-view-directory">
      <section className="suppliers-filters" aria-label="Filtros de proveedores">
        <div className="suppliers-field suppliers-field-grow">
          <label htmlFor="suppliers-search">Buscar</label>
          <input
            id="suppliers-search"
            className="suppliers-input"
            type="text"
            maxLength={100}
            placeholder="Nombre, contacto, documento, teléfono o email…"
            value={searchInput}
            onChange={event => setSearchInput(event.target.value)}
          />
        </div>

        <div className="suppliers-field">
          <label htmlFor="suppliers-active">Estado</label>
          <select
            id="suppliers-active"
            className="suppliers-input"
            value={activeFilter}
            onChange={event => setActiveFilter(event.target.value as ActiveFilter)}
          >
            <option value="active">Activos</option>
            <option value="inactive">Inactivos</option>
            <option value="all">Todos</option>
          </select>
        </div>

        <div className="suppliers-field">
          <label htmlFor="suppliers-priority">Prioridad</label>
          <select
            id="suppliers-priority"
            className="suppliers-input"
            value={priorityFilter}
            onChange={event =>
              setPriorityFilter(event.target.value as SupplierPriority | '')
            }
          >
            <option value="">Todas</option>
            <option value="normal">Normal</option>
            <option value="preferred">Preferido</option>
            <option value="urgent">Urgente</option>
          </select>
        </div>

        <button
          type="button"
          className="suppliers-btn"
          disabled={!filtersActive}
          onClick={handleClearFilters}
        >
          Limpiar
        </button>
      </section>

      <p className="suppliers-status" role="status" aria-live="polite">
        {statusText}
      </p>

      {error && (
        <div className="suppliers-error" role="alert">
          <p>{error}</p>
          <button
            type="button"
            className="suppliers-btn suppliers-btn-primary"
            onClick={refresh}
          >
            Reintentar
          </button>
        </div>
      )}

      {!error && !loading && suppliers.length === 0 && (
        <div className="suppliers-empty">
          {filtersActive ? (
            'Ningún proveedor coincide con los filtros.'
          ) : (
            <>
              <p>Aún no hay proveedores registrados.</p>
              <button
                type="button"
                className="suppliers-btn suppliers-btn-primary"
                onClick={() => setDialog({ kind: 'create' })}
              >
                Registrar el primer proveedor
              </button>
            </>
          )}
        </div>
      )}

      {suppliers.length > 0 && (
        <div
          className={
            loading
              ? 'suppliers-results suppliers-results--loading'
              : 'suppliers-results'
          }
        >
          <div className="suppliers-table-wrap">
            <table className="suppliers-table">
              <thead>
                <tr>
                  <th>Proveedor</th>
                  <th>Contacto</th>
                  <th>Prioridad</th>
                  <th>Productos</th>
                  <th>Stock</th>
                  <th>Condiciones</th>
                  <th>Estado</th>
                </tr>
              </thead>
              <tbody>
                {suppliers.map(supplier => {
                  const subContactParts: string[] = [];
                  if (supplier.email) subContactParts.push(supplier.email);
                  if (supplier.city) subContactParts.push(supplier.city);
                  const subContact = subContactParts.join(' · ');

                  const hasStockAlerts =
                    supplier.out_of_stock_count > 0 || supplier.low_stock_count > 0;

                  return (
                    <tr
                      key={supplier.id}
                      className={
                        supplier.is_active
                          ? 'suppliers-row'
                          : 'suppliers-row suppliers-row--inactive'
                      }
                      onClick={() => setDialog({ kind: 'detail', id: supplier.id })}
                    >
                      <td>
                        <button
                          type="button"
                          className="suppliers-link"
                          onClick={event => {
                            event.stopPropagation();
                            setDialog({ kind: 'detail', id: supplier.id });
                          }}
                        >
                          {supplier.name}
                        </button>
                        {(supplier.contact_name ||
                          (supplier.document_type && supplier.document_number)) && (
                          <span className="suppliers-sub">
                            {[
                              supplier.contact_name,
                              supplier.document_type && supplier.document_number
                                ? `${supplier.document_type} ${supplier.document_number}`
                                : null,
                            ]
                              .filter(Boolean)
                              .join(' · ')}
                          </span>
                        )}
                      </td>

                      <td>
                        {supplier.phone ?? '—'}
                        {subContact && (
                          <span className="suppliers-sub">{subContact}</span>
                        )}
                      </td>

                      <td>
                        <span className={priorityBadgeClass(supplier.priority)}>
                          {priorityLabel(supplier.priority)}
                        </span>
                      </td>

                      <td>
                        {supplier.variant_count} var · {supplier.category_count} cat
                      </td>

                      <td>
                        {hasStockAlerts ? (
                          <div className="suppliers-stock-chips">
                            {supplier.out_of_stock_count > 0 && (
                              <span className="suppliers-badge suppliers-badge-out">
                                {supplier.out_of_stock_count}{' '}
                                {supplier.out_of_stock_count === 1
                                  ? 'agotado'
                                  : 'agotados'}
                              </span>
                            )}
                            {supplier.low_stock_count > 0 && (
                              <span className="suppliers-badge suppliers-badge-low">
                                {supplier.low_stock_count}{' '}
                                {supplier.low_stock_count === 1 ? 'bajo' : 'bajos'}
                              </span>
                            )}
                          </div>
                        ) : (
                          '—'
                        )}
                      </td>

                      <td>
                        {formatSupplierConditions(
                          supplier.lead_time_days,
                          supplier.payment_terms_days,
                        )}
                      </td>

                      <td>
                        <span
                          className={
                            supplier.is_active
                              ? 'suppliers-badge suppliers-badge-ok'
                              : 'suppliers-badge'
                          }
                        >
                          {supplier.is_active ? 'Activo' : 'Inactivo'}
                        </span>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <nav className="suppliers-pagination" aria-label="Paginación">
        <button
          type="button"
          className="suppliers-btn"
          disabled={loading || page <= 1}
          onClick={() => setPage(current => current - 1)}
        >
          ← Anterior
        </button>
        <span>
          Página {page} de {totalPages}
        </span>
        <button
          type="button"
          className="suppliers-btn"
          disabled={loading || !result || page >= totalPages}
          onClick={() => setPage(current => current + 1)}
        >
          Siguiente →
        </button>
      </nav>
      </div>
      )}

      {dialog.kind === 'create' && (
        <Modal title="Nuevo proveedor" onClose={closeDialog} width={720}>
          <SupplierForm
            initial={null}
            onCancel={closeDialog}
            onSaved={saved => {
              closeDialog();
              refresh();
              void loadSummary();
              notify(`Proveedor «${saved.name}» registrado`);
            }}
          />
        </Modal>
      )}

      {dialog.kind === 'edit' && (
        <Modal
          title={`Editar proveedor #${dialog.supplier.id}`}
          onClose={closeDialog}
          width={720}
        >
          <SupplierForm
            initial={dialog.supplier}
            onCancel={() => setDialog({ kind: 'detail', id: dialog.supplier.id })}
            onSaved={saved => {
              refresh();
              void loadSummary();
              setDetailReload(token => token + 1);
              notify(`Proveedor «${saved.name}» actualizado`);
              setDialog({ kind: 'detail', id: saved.id });
            }}
          />
        </Modal>
      )}

      {dialog.kind === 'detail' && (
        <Modal
          title={`Proveedor #${dialog.id}`}
          onClose={closeDialog}
          width={960}
        >
          <SupplierDetail
            key={dialog.id}
            supplierId={dialog.id}
            reloadKey={detailReload}
            initialTab={dialog.initialTab}
            businessName={settings.business_name}
            onEdit={supplier => setDialog({ kind: 'edit', supplier })}
            onChanged={() => {
              refresh();
              void loadSummary();
              if (dialog.fromCallPanel) setCallPanelReload(token => token + 1);
            }}
            onNotify={notify}
            onAction={action => void handleAction(action)}
            onCopy={(text, what) => void handleCopy(text, what)}
          />
        </Modal>
      )}

      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
}
