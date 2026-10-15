import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  PurchaseItem,
  PurchaseListDetail,
  PurchaseListSummary,
} from '../../types/purchases';
import { buildPrintModel, printDocumentTitle, type PrintModel } from '../../utils/purchasePrint';
import type { RestockOverview, SupplierOverview } from '../../types/suppliers';
import {
  addRecommendedToList,
  createPurchaseList,
  getPurchaseList,
  listPurchaseLists,
  removePurchaseItem,
  setPurchaseListStatus,
  updatePurchaseItem,
  updatePurchaseList,
} from '../../services/purchases.service';
import { getRestockOverview, listSuppliers } from '../../services/suppliers.service';
import { errorMessage } from '../../utils/audit';
import { formatDate, formatMoney } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import { buildRestockOrderMessage, contactActions } from '../../utils/callPanel';
import type { ContactAction } from '../../utils/callPanel';
import {
  addedMessage,
  buildSupplierOptions,
  defaultListTitle,
  groupBySupplier,
  isEditable,
  messageItems,
  pendingRecommended,
  purchaseStatusBadgeClass,
  purchaseTotals,
  replaceItem,
  statusActions,
  PURCHASE_STATUS_LABELS,
} from '../../utils/purchaseList';
import { todayInTimeZone } from '../../utils/debts';
import { PurchaseItemPicker } from './PurchaseItemPicker';
import { PurchasePrintJob } from './PurchasePrintJob';
import { PurchaseLineRow, type ItemPatch } from './PurchaseLineRow';
import './SuppliersBase.css';
import './PurchasePanel.css';

interface PurchasePanelProps {
  listId: number | null;
  onSelectList: (id: number | null) => void;
  timezone: string;
  businessName: string;
  onAction: (action: ContactAction) => void;
  onCopy: (text: string, what: string) => void;
  onNotify: (message: string, type?: 'success' | 'error' | 'warning') => void;
}

interface PurchasePrintJobState {
  id: number;
  model: PrintModel;
  documentTitle: string;
}

export const PurchasePanel: React.FC<PurchasePanelProps> = ({
  listId,
  onSelectList,
  timezone,
  businessName,
  onAction,
  onCopy,
  onNotify,
}) => {
  const [scope, setScope] = useState<'open' | 'all'>('open');
  const [summaries, setSummaries] = useState<PurchaseListSummary[]>([]);
  const [summaryLoading, setSummaryLoading] = useState(true);
  const [summaryError, setSummaryError] = useState<string | null>(null);
  const [summaryReload, setSummaryReload] = useState(0);
  const [detail, setDetail] = useState<PurchaseListDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [detailReload, setDetailReload] = useState(0);
  const [overview, setOverview] = useState<RestockOverview | null>(null);
  const [overviewError, setOverviewError] = useState<string | null>(null);
  const [overviewReload, setOverviewReload] = useState(0);
  const [recommendedActionError, setRecommendedActionError] = useState<string | null>(null);
  const [supplierOptionsSource, setSupplierOptionsSource] = useState<SupplierOverview[] | null>(null);
  const [suppliersError, setSuppliersError] = useState<string | null>(null);
  const [suppliersReload, setSuppliersReload] = useState(0);
  const [supplierTotal, setSupplierTotal] = useState<number | null>(null);
  const [creating, setCreating] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [createError, setCreateError] = useState<string | null>(null);
  const [savingTitle, setSavingTitle] = useState(false);
  const [editingTitle, setEditingTitle] = useState(false);
  const [titleDraft, setTitleDraft] = useState('');
  const [headerError, setHeaderError] = useState<string | null>(null);
  const [pendingStatus, setPendingStatus] = useState<('draft' | 'ordered' | 'cancelled') | null>(null);
  const [statusBusy, setStatusBusy] = useState(false);
  const [printJob, setPrintJob] = useState<PurchasePrintJobState | null>(null);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [working, setWorking] = useState(false);
  const initialSelectionDone = useRef(false);
  const summaryRequestRef = useRef(0);
  const detailRequestRef = useRef(0);
  const overviewRequestRef = useRef(0);
  const operationRef = useRef(false);
  const cancelTitleEditRef = useRef(false);

  const reloadSummaries = useCallback(() => setSummaryReload(token => token + 1), []);

  useEffect(() => {
    const requestId = ++summaryRequestRef.current;
    setSummaryLoading(true);
    setSummaryError(null);
    void (async () => {
      const firstPage = await listPurchaseLists({ scope, page: 1, page_size: 100 });
      const all = [...firstPage.data];
      for (let page = 2; page <= firstPage.total_pages; page += 1) {
        if (requestId !== summaryRequestRef.current) return;
        const nextPage = await listPurchaseLists({ scope, page, page_size: 100 });
        all.push(...nextPage.data);
      }
      return all;
    })()
      .then(response => {
        if (requestId !== summaryRequestRef.current) return;
        setSummaries(response ?? []);
        setSummaryLoading(false);
        if (!initialSelectionDone.current) {
          initialSelectionDone.current = true;
          if (listId === null) {
            const firstOpen = (response ?? []).find(summary => summary.status === 'draft' || summary.status === 'ordered');
            if (firstOpen) onSelectList(firstOpen.id);
          }
        }
      })
      .catch(cause => {
        if (requestId !== summaryRequestRef.current) return;
        setSummaryError(errorMessage(cause));
        setSummaries([]);
        setSummaryLoading(false);
      });
    return () => {
      summaryRequestRef.current += 1;
    };
  }, [scope, summaryReload, listId, onSelectList]);

  useEffect(() => {
    const requestId = ++detailRequestRef.current;
    setDetail(null);
    setHeaderError(null);
    setPendingStatus(null);
    if (listId === null) {
      setDetailLoading(false);
      setDetailError(null);
      return () => {
        detailRequestRef.current += 1;
      };
    }
    setDetailLoading(true);
    setDetailError(null);
    void getPurchaseList(listId)
      .then(response => {
        if (requestId !== detailRequestRef.current) return;
        setDetail(response);
        setDetailLoading(false);
      })
      .catch(cause => {
        if (requestId !== detailRequestRef.current) return;
        setDetailError(errorMessage(cause));
        setDetailLoading(false);
      });
    return () => {
      detailRequestRef.current += 1;
    };
  }, [listId, detailReload]);

  useEffect(() => {
    if (!detail || !isEditable(detail.list.status)) {
      setOverview(null);
      setOverviewError(null);
      return;
    }
    const requestId = ++overviewRequestRef.current;
    setOverviewError(null);
    void getRestockOverview({})
      .then(response => {
        if (requestId !== overviewRequestRef.current) return;
        setOverview(response);
      })
      .catch(cause => {
        if (requestId !== overviewRequestRef.current) return;
        setOverview(null);
        setOverviewError(errorMessage(cause));
      });
    return () => {
      overviewRequestRef.current += 1;
    };
  }, [detail?.list.id, detail?.list.status, overviewReload]);

  useEffect(() => {
    setSuppliersError(null);
    void listSuppliers({ is_active: true, page_size: 100 })
      .then(response => {
        setSupplierOptionsSource(response.data);
        setSupplierTotal(response.total);
      })
      .catch(cause => {
        setSupplierOptionsSource(null);
        setSuppliersError(errorMessage(cause));
      });
  }, [suppliersReload]);

  const inList = useMemo(
    () => new Set((detail?.items ?? []).map(item => item.variant_id)),
    [detail?.items],
  );
  const recommended = useMemo(
    () => overview ? pendingRecommended(overview, inList) : null,
    [overview, inList],
  );
  const groups = useMemo(() => groupBySupplier(detail?.items ?? []), [detail?.items]);
  const totals = useMemo(() => purchaseTotals(detail?.items ?? []), [detail?.items]);
  const editable = detail ? isEditable(detail.list.status) : false;

  const refreshDetail = useCallback(() => setDetailReload(token => token + 1), []);

  const handleCreate = async () => {
    const title = newTitle.trim();
    if (!title) {
      setCreateError('El título es obligatorio');
      return;
    }
    if (title.length > 150) {
      setCreateError('El título no puede superar 150 caracteres');
      return;
    }
    setWorking(true);
    setCreateError(null);
    try {
      const created = await createPurchaseList({
        title,
        ...actorField('created_by'),
      });
      setCreating(false);
      setNewTitle('');
      onSelectList(created.id);
      reloadSummaries();
      onNotify(`Lista «${created.title}» creada`);
    } catch (cause) {
      setCreateError(errorMessage(cause));
    } finally {
      setWorking(false);
    }
  };

  const handleSaveTitle = async () => {
    if (cancelTitleEditRef.current) {
      cancelTitleEditRef.current = false;
      return;
    }
    if (!detail || savingTitle) return;
    const title = titleDraft.trim();
    if (!title || title.length > 150) {
      setHeaderError(!title ? 'El título es obligatorio' : 'El título no puede superar 150 caracteres');
      return;
    }
    if (title === detail.list.title) {
      setEditingTitle(false);
      return;
    }
    setSavingTitle(true);
    setHeaderError(null);
    try {
      const list = await updatePurchaseList(detail.list.id, {
        title,
        notes: detail.list.notes,
        ...actorField('updated_by'),
      });
      setDetail(current => current ? { ...current, list } : current);
      setEditingTitle(false);
      reloadSummaries();
    } catch (cause) {
      setHeaderError(errorMessage(cause));
    } finally {
      setSavingTitle(false);
    }
  };

  const handleStatusChange = async () => {
    if (!detail || !pendingStatus || statusBusy) return;
    setStatusBusy(true);
    setHeaderError(null);
    try {
      const list = await setPurchaseListStatus(detail.list.id, {
        status: pendingStatus,
        ...actorField('updated_by'),
      });
      setDetail(current => current ? { ...current, list } : current);
      setPendingStatus(null);
      reloadSummaries();
    } catch (cause) {
      setHeaderError(errorMessage(cause));
    } finally {
      setStatusBusy(false);
    }
  };

  const withOperation = async (action: () => Promise<void>) => {
    if (operationRef.current) return;
    operationRef.current = true;
    setWorking(true);
    try {
      await action();
    } finally {
      operationRef.current = false;
      setWorking(false);
    }
  };

  const handleSaveItem = async (item: PurchaseItem, patch: ItemPatch) => {
    if (!detail) return;
    const updated = await updatePurchaseItem(item.item_id, {
      ...patch,
      ...actorField('updated_by'),
    });
    setDetail(current => current
      ? { ...current, items: replaceItem(current.items, updated) }
      : current);
    reloadSummaries();
  };

  const handleRemoveItem = async (item: PurchaseItem) => {
    if (!detail) return;
    await removePurchaseItem(item.item_id);
    setDetail(current => current
      ? { ...current, items: current.items.filter(line => line.item_id !== item.item_id) }
      : current);
    reloadSummaries();
  };

  const addRecommended = async (variantIds: number[]) => {
    if (!detail || variantIds.length === 0) return;
    setRecommendedActionError(null);
    try {
      await withOperation(async () => {
        const result = await addRecommendedToList(detail.list.id, {
          variant_ids: variantIds,
          ...actorField('updated_by'),
        });
        onNotify(addedMessage(result));
        refreshDetail();
        reloadSummaries();
        setOverviewReload(token => token + 1);
      });
    } catch (cause) {
      setRecommendedActionError(errorMessage(cause));
    }
  };

  const openCreate = () => {
    setNewTitle(defaultListTitle(todayInTimeZone(timezone)));
    setCreateError(null);
    setCreating(true);
    setSidebarOpen(true);
  };

  const handlePrint = () => {
    if (!detail || detail.items.length === 0) return;
    const day = todayInTimeZone(timezone);
    setPrintJob({
      id: Date.now(),
      model: buildPrintModel(detail.list, detail.items, { businessName, day }),
      documentTitle: printDocumentTitle(detail.list.title, day),
    });
  };

  return (
    <section className="purchase-panel" aria-label="Listas de compras">
      <aside className={`purchase-sidebar${sidebarOpen ? ' purchase-sidebar--open' : ''}`}>
        <div className="purchase-sidebar-header">
          <h2>Mis listas</h2>
          <button
            type="button"
            className="purchase-sidebar-toggle"
            aria-expanded={sidebarOpen}
            aria-controls="purchase-sidebar-content"
            onClick={() => setSidebarOpen(open => !open)}
          >{sidebarOpen ? 'Ocultar' : 'Mostrar'}</button>
        </div>
        <div id="purchase-sidebar-content" className="purchase-sidebar-content">
          <button type="button" className="suppliers-btn suppliers-btn-primary purchase-new-button" onClick={openCreate}>
            Nueva lista
          </button>
          {creating && (
            <form
              className="purchase-create-form"
              onSubmit={event => {
                event.preventDefault();
                void handleCreate();
              }}
            >
              <label htmlFor="purchase-new-title">Título</label>
              <input
                id="purchase-new-title"
                className="suppliers-input"
                maxLength={151}
                value={newTitle}
                disabled={working}
                onChange={event => setNewTitle(event.target.value)}
                autoFocus
              />
              {createError && <p className="purchase-inline-error" role="alert">{createError}</p>}
              <div className="purchase-create-actions">
                <button type="button" className="suppliers-btn" onClick={() => setCreating(false)} disabled={working}>
                  Cancelar
                </button>
                <button type="submit" className="suppliers-btn suppliers-btn-primary" disabled={working}>
                  Crear
                </button>
              </div>
            </form>
          )}
          <label className="purchase-history-toggle">
            <input
              type="checkbox"
              checked={scope === 'all'}
              onChange={event => setScope(event.target.checked ? 'all' : 'open')}
            />
            <span>Mostrar historial</span>
          </label>
          {summaryLoading ? (
            <p className="purchase-muted" role="status">Cargando listas…</p>
          ) : summaryError ? (
            <div className="purchase-error" role="alert">
              <span>No se pudieron cargar las listas: {summaryError}</span>
              <button type="button" className="suppliers-btn" onClick={reloadSummaries}>Reintentar</button>
            </div>
          ) : summaries.length === 0 ? (
            <p className="purchase-muted">Aún no tienes listas. Crea la primera para armar tu próxima compra.</p>
          ) : (
            <nav className="purchase-list-nav" aria-label="Listas guardadas">
              {summaries.map(summary => (
                <button
                  type="button"
                  key={summary.id}
                  className="purchase-list-entry"
                  aria-current={summary.id === listId ? 'true' : undefined}
                  onClick={() => {
                    onSelectList(summary.id);
                    setSidebarOpen(false);
                  }}
                >
                  <span className="purchase-list-entry-title">{summary.title}</span>
                  <span className={purchaseStatusBadgeClass(summary.status)}>
                    {PURCHASE_STATUS_LABELS[summary.status]}
                  </span>
                  <span className="purchase-list-entry-meta">
                    {summary.item_count} {summary.item_count === 1 ? 'producto' : 'productos'} · {formatMoney(summary.estimated_cost)}
                  </span>
                  <time className="purchase-list-entry-date" dateTime={summary.updated_at}>
                    {formatDate(summary.updated_at)}
                  </time>
                </button>
              ))}
            </nav>
          )}
        </div>
      </aside>

      <div className="purchase-detail">
        {detailLoading ? (
          <p className="purchase-muted" role="status">Cargando lista…</p>
        ) : detailError ? (
          <div className="purchase-error" role="alert">
            <span>No se pudo cargar la lista: {detailError}</span>
            <button type="button" className="suppliers-btn suppliers-btn-primary" onClick={refreshDetail}>Reintentar</button>
          </div>
        ) : !detail ? (
          <div className="purchase-empty">
            <p>Aún no tienes listas. Crea la primera para armar tu próxima compra.</p>
            <button type="button" className="suppliers-btn suppliers-btn-primary" onClick={openCreate}>
              Nueva lista
            </button>
          </div>
        ) : (
          <>
            <header className="purchase-detail-header">
              <div className="purchase-title-row">
                {editingTitle ? (
                  <input
                    className="suppliers-input purchase-title-input"
                    aria-label="Título de la lista"
                    maxLength={151}
                    value={titleDraft}
                    disabled={savingTitle}
                    onChange={event => setTitleDraft(event.target.value)}
                    onBlur={() => void handleSaveTitle()}
                    onKeyDown={event => {
                      if (event.key === 'Enter') event.currentTarget.blur();
                      if (event.key === 'Escape') {
                        cancelTitleEditRef.current = true;
                        setTitleDraft(detail.list.title);
                        setEditingTitle(false);
                      }
                    }}
                    autoFocus
                  />
                ) : (
                  <>
                    <h2>{detail.list.title}</h2>
                    {(detail.list.status === 'draft' || detail.list.status === 'ordered') && (
                      <button
                        type="button"
                        className="purchase-icon-button"
                        aria-label="Editar título"
                        onClick={() => {
                          setTitleDraft(detail.list.title);
                          cancelTitleEditRef.current = false;
                          setEditingTitle(true);
                        }}
                      >✎</button>
                    )}
                  </>
                )}
                <span className={purchaseStatusBadgeClass(detail.list.status)}>
                  {PURCHASE_STATUS_LABELS[detail.list.status]}
                </span>
              </div>
              <div className="purchase-status-actions">
                <button
                  type="button"
                  className="suppliers-btn"
                  disabled={detail.items.length === 0}
                  title={detail.items.length === 0
                    ? 'La lista está vacía'
                    : 'Abre el diálogo de impresión: elige “Guardar como PDF”'}
                  onClick={handlePrint}
                >Imprimir / Guardar PDF</button>
                {statusActions(detail.list.status).map(action => (
                  <button
                    key={action.target}
                    type="button"
                    className={`suppliers-btn${action.tone === 'primary' ? ' suppliers-btn-primary' : ''}${action.tone === 'danger' ? ' suppliers-btn-danger' : ''}`}
                    disabled={statusBusy}
                    onClick={() => setPendingStatus(action.target)}
                  >{action.label}</button>
                ))}
              </div>
              {headerError && <p className="purchase-inline-error" role="alert">{headerError}</p>}
              {pendingStatus && (
                <div className="purchase-status-confirm" role="alert">
                  <span>{statusActions(detail.list.status).find(action => action.target === pendingStatus)?.confirm}</span>
                  <button type="button" className="suppliers-btn" disabled={statusBusy} onClick={() => setPendingStatus(null)}>
                    No, volver
                  </button>
                  <button
                    type="button"
                    className="suppliers-btn suppliers-btn-primary"
                    disabled={statusBusy}
                    onClick={() => void handleStatusChange()}
                  >{statusBusy ? 'Guardando…' : 'Confirmar'}</button>
                </div>
              )}
            </header>

            <div className="purchase-totals" aria-label="Totales de la lista">
              <span>{totals.itemCount} {totals.itemCount === 1 ? 'producto' : 'productos'}</span>
              <span>{totals.supplierCount} {totals.supplierCount === 1 ? 'proveedor' : 'proveedores'}</span>
              <span>{totals.units} {totals.units === 1 ? 'unidad' : 'unidades'}</span>
              <strong>Costo estimado {formatMoney(totals.estimatedCost)}</strong>
              {totals.itemsWithoutCost > 0 && (
                <span>{totals.itemsWithoutCost} sin costo</span>
              )}
            </div>

            {editable && (
              <section className="purchase-recommended" aria-label="Productos recomendados">
                <div className="purchase-recommended-copy">
                  <strong>Recomendado</strong>
                  {overviewError ? (
                    <span role="alert">No se pudo cargar la reposición: {overviewError}</span>
                  ) : recommended === null ? (
                    <span role="status">Cargando necesidades de reposición…</span>
                  ) : recommended.total > 0 ? (
                    <span>
                      Hay {recommended.total} {recommended.total === 1 ? 'producto' : 'productos'} por reponer que no están en esta lista ({recommended.urgent} urgentes)
                    </span>
                  ) : (
                    <span>{overview?.groups.some(group => group.items.length > 0)
                      ? 'Todo lo que necesita reposición ya está en esta lista.'
                      : 'Nada por reponer ahora.'}</span>
                  )}
                </div>
                {overviewError ? (
                  <button type="button" className="suppliers-btn" onClick={() => setOverviewReload(token => token + 1)}>
                    Reintentar
                  </button>
                ) : recommended && recommended.total > 0 && overview && (
                  <div className="purchase-recommended-actions">
                    <button
                      type="button"
                      className="suppliers-btn suppliers-btn-primary"
                      disabled={working}
                      onClick={() => void addRecommended(overview.groups.flatMap(group =>
                        group.items.filter(item => !inList.has(item.variant_id)).map(item => item.variant_id)))}
                    >Agregar todo lo recomendado</button>
                    {recommended.bySupplier.map(group => {
                      const variants = overview.groups.find(candidate =>
                        (candidate.supplier?.id ?? null) === group.supplierId)?.items
                        .filter(item => !inList.has(item.variant_id))
                        .map(item => item.variant_id) ?? [];
                      return (
                        <button
                          type="button"
                          key={group.supplierId ?? 'none'}
                          className="suppliers-btn"
                          disabled={working}
                          onClick={() => void addRecommended(variants)}
                        >{group.name} ({group.count})</button>
                      );
                    })}
                  </div>
                )}
                {recommendedActionError && (
                  <p className="purchase-inline-error" role="alert">
                    No se pudieron agregar los recomendados: {recommendedActionError}
                  </p>
                )}
              </section>
            )}

            <div className="purchase-lines-toolbar">
              {editable && (
                <button
                  type="button"
                  className="suppliers-btn suppliers-btn-primary"
                  onClick={() => setPickerOpen(true)}
                >Agregar productos</button>
              )}
              {suppliersError && (
                <span className="purchase-subtle-error" role="alert">
                  No se pudieron cargar los proveedores: {suppliersError}
                  <button type="button" className="suppliers-btn" onClick={() => setSuppliersReload(value => value + 1)}>
                    Reintentar
                  </button>
                </span>
              )}
              {supplierTotal !== null && supplierTotal > 100 && (
                <span className="purchase-muted" role="status">
                  Se muestran los primeros 100 proveedores activos.
                </span>
              )}
            </div>

            {detail.items.length === 0 ? (
              <p className="purchase-empty-lines">La lista está vacía. Agrega lo recomendado o busca productos.</p>
            ) : (
              <div className="purchase-groups">
                {groups.map(group => {
                  const orderMessage = buildRestockOrderMessage(
                    group.supplier?.name ?? null,
                    messageItems(group),
                    businessName,
                  );
                  const actions = group.supplier
                    ? contactActions(group.supplier, orderMessage, businessName)
                    : [];
                  return (
                    <section
                      key={group.key ?? 'none'}
                      className="purchase-group"
                      aria-label={`Productos de ${group.supplier?.name ?? 'Sin proveedor'}`}
                    >
                      <header className="purchase-group-header">
                        <div>
                          <h3>{group.supplier?.name ?? 'Sin proveedor'}</h3>
                          <span>
                            {group.itemCount} {group.itemCount === 1 ? 'producto' : 'productos'} · {group.units} unidades · {formatMoney(group.estimatedCost)}
                          </span>
                        </div>
                        <div className="purchase-contact-actions">
                          {actions.map(action => (
                            <button
                              type="button"
                              className="suppliers-btn"
                              key={action.channel}
                              disabled={action.url === null}
                              title={action.disabledReason ?? undefined}
                              onClick={() => onAction(action)}
                            >{action.label}</button>
                          ))}
                          <button
                            type="button"
                            className="suppliers-btn"
                            onClick={() => onCopy(orderMessage, group.supplier ? 'Pedido' : 'Lista')}
                          >{group.supplier ? 'Copiar pedido' : 'Copiar lista'}</button>
                        </div>
                      </header>
                      <div className="purchase-table-wrap">
                        <table className="suppliers-table purchase-lines-table">
                          <caption className="purchase-sr-only">
                            Productos de {group.supplier?.name ?? 'Sin proveedor'}
                          </caption>
                          <thead>
                            <tr>
                              <th>Producto</th>
                              <th>Stock</th>
                              <th>Proveedor</th>
                              <th className="suppliers-num">Cantidad</th>
                              <th className="suppliers-num">Costo (inventario)</th>
                              <th className="suppliers-num">Total</th>
                              <th>Acciones</th>
                            </tr>
                          </thead>
                          <tbody>
                            {group.items.map(item => (
                              <PurchaseLineRow
                                key={item.item_id}
                                item={item}
                                editable={editable}
                                supplierOptions={buildSupplierOptions(item.supplier, supplierOptionsSource ?? [])}
                                onSave={handleSaveItem}
                                onRemove={handleRemoveItem}
                              />
                            ))}
                          </tbody>
                        </table>
                      </div>
                    </section>
                  );
                })}
              </div>
            )}
          </>
        )}
      </div>
      {pickerOpen && detail && (
        <PurchaseItemPicker
          listId={detail.list.id}
          onClose={() => setPickerOpen(false)}
          onSaved={() => {
            setPickerOpen(false);
            refreshDetail();
            reloadSummaries();
          }}
          onNotify={onNotify}
        />
      )}
      {printJob && (
        <PurchasePrintJob
          key={printJob.id}
          model={printJob.model}
          documentTitle={printJob.documentTitle}
          onDone={() => setPrintJob(null)}
        />
      )}
    </section>
  );
};
