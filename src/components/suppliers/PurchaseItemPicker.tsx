import { useEffect, useMemo, useRef, useState } from 'react';
import { useDebounce } from '../../hooks/useDebounce';
import { listCategories } from '../../services/inventory.service';
import {
  addPurchaseItems,
  listPurchasableVariantIds,
  searchPurchasableVariants,
} from '../../services/purchases.service';
import type { Category } from '../../types';
import type { LinkableFilterDto, LinkableVariant } from '../../types/suppliers';
import { errorMessage } from '../../utils/audit';
import { formatAttributes, formatMoney } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import {
  addAll,
  addIds,
  allSelected,
  emptySelection,
  removeAll,
  removeOne,
  searchBlockedReason,
  selectedIds,
  toggleSelection,
  variantLabel,
  type Selection,
} from '../../utils/linkPicker';
import { addedMessage, toPickable } from '../../utils/purchaseList';
import { StockBadge } from '../inventory/StockBadge';
import { Modal } from '../ui/Modal';
import './SuppliersBase.css';
import './PurchaseItemPicker.css';

const PAGE_SIZE = 50;
const MAX_ITEMS = 500;

interface PurchaseItemPickerProps {
  listId: number;
  onClose: () => void;
  onSaved: () => void;
  onNotify: (message: string, type?: 'success' | 'error') => void;
}

export const PurchaseItemPicker: React.FC<PurchaseItemPickerProps> = ({
  listId,
  onClose,
  onSaved,
  onNotify,
}) => {
  const [searchInput, setSearchInput] = useState('');
  const search = useDebounce(searchInput.trim(), 300);
  const [categoryId, setCategoryId] = useState<number | undefined>();
  const [page, setPage] = useState(1);
  const [categories, setCategories] = useState<Category[]>([]);
  const [categoriesError, setCategoriesError] = useState<string | null>(null);
  const [result, setResult] = useState<{
    data: LinkableVariant[];
    total: number;
    total_pages: number;
  } | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selection, setSelection] = useState<Selection>(emptySelection);
  const [selectingAll, setSelectingAll] = useState(false);
  const [saving, setSaving] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);
  const [confirmClose, setConfirmClose] = useState(false);
  const [selectAllNotice, setSelectAllNotice] = useState<string | null>(null);
  const [searchReload, setSearchReload] = useState(0);
  const requestRef = useRef(0);
  const submittingRef = useRef(false);

  const blockedReason = searchBlockedReason({ search: searchInput, category_id: categoryId });
  const filter = useMemo<LinkableFilterDto>(
    () => ({
      search: search || undefined,
      category_id: categoryId,
      page,
      page_size: PAGE_SIZE,
    }),
    [search, categoryId, page],
  );

  const loadCategories = () => {
    setCategoriesError(null);
    void listCategories()
      .then(items => setCategories(items.filter(category => category.is_active)))
      .catch(cause => setCategoriesError(errorMessage(cause)));
  };

  useEffect(() => {
    loadCategories();
  }, []);

  useEffect(() => {
    const requestId = ++requestRef.current;
    if (blockedReason) {
      setResult(null);
      setLoading(false);
      setError(null);
      return () => {
        requestRef.current += 1;
      };
    }
    setLoading(true);
    setError(null);
    void searchPurchasableVariants(listId, filter)
      .then(response => {
        if (requestId !== requestRef.current) return;
        setResult({ ...response, data: response.data.map(toPickable) });
        setLoading(false);
      })
      .catch(cause => {
        if (requestId !== requestRef.current) return;
        setError(errorMessage(cause));
        setResult(null);
        setLoading(false);
      });
    return () => {
      requestRef.current += 1;
    };
  }, [listId, filter, blockedReason, searchReload]);

  const items = result?.data ?? [];
  const totalPages = Math.max(result?.total_pages ?? 0, 1);
  const selectionTooLarge = selection.size > MAX_ITEMS;

  const close = () => {
    if (saving) return;
    if (selection.size > 0) setConfirmClose(true);
    else onClose();
  };

  const handleSelectAllResults = async () => {
    if (!result || selectingAll || blockedReason) return;
    setSelectingAll(true);
    setServerError(null);
    const { page: _page, page_size: _pageSize, ...allFilter } = filter;
    try {
      const response = await listPurchasableVariantIds(listId, allFilter);
      setSelection(current => addIds(current, response.ids));
      setSelectAllNotice(response.truncated
        ? `Se seleccionaron hasta ${MAX_ITEMS} productos.`
        : null);
    } catch (cause) {
      setServerError(errorMessage(cause));
    } finally {
      setSelectingAll(false);
    }
  };

  const handleSave = async () => {
    if (submittingRef.current || selection.size === 0 || selectionTooLarge) return;
    submittingRef.current = true;
    setSaving(true);
    setServerError(null);
    try {
      const response = await addPurchaseItems(listId, {
        variant_ids: selectedIds(selection),
        ...actorField('updated_by'),
      });
      onNotify(addedMessage(response));
      onSaved();
    } catch (cause) {
      setServerError(errorMessage(cause));
      submittingRef.current = false;
      setSaving(false);
    }
  };

  return (
    <Modal title="Agregar productos a la lista" onClose={close} width={900}>
      <div className="purchase-picker">
        <div className="purchase-picker-controls">
          <input
            type="search"
            className="suppliers-input"
            autoFocus
            placeholder="Buscar producto, marca, SKU o código…"
            value={searchInput}
            onChange={event => {
              setSearchInput(event.target.value);
              setPage(1);
            }}
            aria-label="Buscar productos"
          />
          <select
            className="suppliers-input"
            value={categoryId ?? ''}
            onChange={event => {
              setCategoryId(event.target.value ? Number(event.target.value) : undefined);
              setPage(1);
            }}
            aria-label="Filtrar por categoría"
          >
            <option value="">Todas las categorías</option>
            {categories.map(category => (
              <option key={category.id} value={category.id}>{category.name}</option>
            ))}
          </select>
        </div>

        {categoriesError && (
          <div className="purchase-picker-error" role="alert">
            No se pudieron cargar las categorías: {categoriesError}
            <button type="button" className="suppliers-btn" onClick={loadCategories}>Reintentar</button>
          </div>
        )}
        {serverError && <div className="purchase-picker-error" role="alert">{serverError}</div>}
        {selectAllNotice && (
          <p className="purchase-picker-notice" role="status">{selectAllNotice}</p>
        )}
        {selectionTooLarge && (
          <p className="purchase-picker-notice" role="status">
            Puedes agregar hasta {MAX_ITEMS} productos por vez. Quita algunos de la selección.
          </p>
        )}

        {blockedReason ? (
          <p className="purchase-picker-muted" role="status">{blockedReason}</p>
        ) : (
          <>
            <div className="purchase-picker-toolbar">
              <span>{loading ? 'Buscando productos…' : `${result?.total ?? 0} resultados`}</span>
              <button
                type="button"
                className="suppliers-btn"
                disabled={!result || result.total === 0 || selectingAll || loading}
                onClick={() => void handleSelectAllResults()}
              >
                {selectingAll ? 'Seleccionando…' : `Seleccionar todos (${result?.total ?? 0})`}
              </button>
            </div>
            {error && (
              <div className="purchase-picker-error" role="alert">
                No se pudieron buscar productos: {error}
                <button
                  type="button"
                  className="suppliers-btn"
                  onClick={() => setSearchReload(value => value + 1)}
                >
                  Reintentar
                </button>
              </div>
            )}
            {!error && !loading && items.length === 0 && (
              <p className="purchase-picker-muted">No se encontraron productos.</p>
            )}
            {items.length > 0 && (
              <div className="purchase-picker-table-wrap">
                <table className="purchase-picker-table">
                  <caption className="purchase-picker-sr-only">Productos para agregar a la lista</caption>
                  <thead>
                    <tr>
                      <th>
                        <label className="purchase-picker-page-select">
                          <input
                            type="checkbox"
                            checked={allSelected(selection, items)}
                            onChange={() => setSelection(current =>
                              allSelected(current, items) ? removeAll(current, items) : addAll(current, items))}
                            aria-label="Seleccionar esta página"
                          />
                          <span>Seleccionar esta página</span>
                        </label>
                      </th>
                      <th>Producto</th>
                      <th>Categoría</th>
                      <th>Stock</th>
                      <th>Costo (inventario)</th>
                      <th>En la lista</th>
                    </tr>
                  </thead>
                  <tbody>
                    {items.map(item => (
                      <PickerRow
                        key={item.variant_id}
                        item={item}
                        checked={selection.has(item.variant_id)}
                        onToggle={() => setSelection(current =>
                          toggleSelection(current, item.variant_id, variantLabel(item)))}
                      />
                    ))}
                  </tbody>
                </table>
              </div>
            )}
            {result && totalPages > 1 && (
              <nav className="purchase-picker-pagination" aria-label="Paginación de productos">
                <button
                  type="button"
                  className="suppliers-btn"
                  disabled={loading || page <= 1}
                  onClick={() => setPage(value => value - 1)}
                >← Anterior</button>
                <span>Página {page} de {totalPages}</span>
                <button
                  type="button"
                  className="suppliers-btn"
                  disabled={loading || page >= totalPages}
                  onClick={() => setPage(value => value + 1)}
                >Siguiente →</button>
              </nav>
            )}
          </>
        )}

        <div className="purchase-picker-selection">
          <div className="purchase-picker-selection-head">
            <strong>{selection.size} seleccionados</strong>
            {selection.size > 0 && (
              <button
                type="button"
                className="suppliers-btn"
                disabled={saving}
                onClick={() => setSelection(emptySelection())}
              >Vaciar</button>
            )}
          </div>
          {selection.size > 0 && (
            <div className="purchase-picker-chips" aria-label="Productos seleccionados">
              {[...selection].slice(0, 8).map(([id, label]) => (
                <span className="purchase-picker-chip" key={id}>
                  {label}
                  <button
                    type="button"
                    aria-label={`Quitar ${label} de la selección`}
                    disabled={saving}
                    onClick={() => setSelection(current => removeOne(current, id))}
                  >×</button>
                </span>
              ))}
              {selection.size > 8 && <span className="purchase-picker-chip">+{selection.size - 8} más</span>}
            </div>
          )}
          {confirmClose && (
            <div className="purchase-picker-confirm" role="alert">
              Hay productos seleccionados que no se han guardado.
              <button type="button" className="suppliers-btn" onClick={() => setConfirmClose(false)}>
                Seguir editando
              </button>
              <button type="button" className="suppliers-btn suppliers-btn-danger" onClick={onClose}>
                Cerrar sin guardar
              </button>
            </div>
          )}
          <div className="purchase-picker-actions">
            <button type="button" className="suppliers-btn" onClick={close} disabled={saving}>Cancelar</button>
            <button
              type="button"
              className="suppliers-btn suppliers-btn-primary"
              disabled={selection.size === 0 || saving || selectionTooLarge}
              onClick={() => void handleSave()}
            >
              {saving ? 'Guardando…' : `Agregar ${selection.size} productos`}
            </button>
          </div>
        </div>
      </div>
    </Modal>
  );
};

const PickerRow: React.FC<{
  item: LinkableVariant;
  checked: boolean;
  onToggle: () => void;
}> = ({ item, checked, onToggle }) => {
  const attributes = formatAttributes(item.attributes);
  const codes = [
    item.brand ? `Marca: ${item.brand}` : null,
    item.sku ? `SKU: ${item.sku}` : null,
    item.barcode ? `Código: ${item.barcode}` : null,
  ].filter(Boolean).join(' · ');
  return (
    <tr>
      <td>
        <input
          type="checkbox"
          checked={checked}
          disabled={item.linked_here}
          aria-label={`Seleccionar ${variantLabel(item)}`}
          onChange={onToggle}
        />
      </td>
      <td>
        <strong>{item.product_name}</strong>
        {attributes && <span className="purchase-picker-sub">{attributes}</span>}
        {codes && <span className="purchase-picker-sub">{codes}</span>}
      </td>
      <td>{item.category_name}</td>
      <td><StockBadge stock={item.stock} stockMin={item.stock_min} /></td>
      <td>{item.variant_cost > 0 ? formatMoney(item.variant_cost) : 'Sin costo'}</td>
      <td>{item.linked_here ? <span className="suppliers-badge">Ya en la lista</span> : '—'}</td>
    </tr>
  );
};
