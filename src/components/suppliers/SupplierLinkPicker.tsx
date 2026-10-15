import React, { useEffect, useMemo, useRef, useState } from 'react';
import type { Category } from '../../types';
import {
  listCategories,
} from '../../services/inventory.service';
import {
  linkSupplierVariants,
  listLinkableVariantIds,
  searchLinkableVariants,
} from '../../services/suppliers.service';
import type { LinkableFilterDto, LinkableVariant } from '../../types/suppliers';
import { errorMessage } from '../../utils/audit';
import { formatAttributes, formatMoney } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import {
  addAll,
  addIds,
  allSelected,
  bulkLinkMessage,
  emptySelection,
  removeAll,
  removeOne,
  searchBlockedReason,
  toggleSelection,
  variantLabel,
  type Selection,
} from '../../utils/linkPicker';
import { useDebounce } from '../../hooks/useDebounce';
import { StockBadge } from '../inventory/StockBadge';
import { Modal } from '../ui/Modal';
import './SuppliersBase.css';
import './SupplierLinkPicker.css';

const PAGE_SIZE = 50;
const MAX_BULK_LINKS = 500;

interface SupplierLinkPickerProps {
  supplierId: number;
  onClose: () => void;
  onSaved: () => void;
  onNotify: (message: string, type?: 'success' | 'error') => void;
}

export const SupplierLinkPicker: React.FC<SupplierLinkPickerProps> = ({
  supplierId,
  onClose,
  onSaved,
  onNotify,
}) => {
  const [searchInput, setSearchInput] = useState('');
  const search = useDebounce(searchInput.trim(), 300);
  const [categoryId, setCategoryId] = useState<number | undefined>();
  const [onlyUnassigned, setOnlyUnassigned] = useState(false);
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
  const [selectAllNotice, setSelectAllNotice] = useState<string | null>(null);
  const [serverError, setServerError] = useState<string | null>(null);
  const requestRef = useRef(0);
  const submittingRef = useRef(false);

  const blockedReason = searchBlockedReason({
    search: searchInput,
    category_id: categoryId,
    only_unassigned: onlyUnassigned,
  });
  const filter = useMemo<LinkableFilterDto>(
    () => ({
      search: search || undefined,
      category_id: categoryId,
      only_unassigned: onlyUnassigned || undefined,
      page,
      page_size: PAGE_SIZE,
    }),
    [search, categoryId, onlyUnassigned, page],
  );

  useEffect(() => {
    let active = true;
    listCategories()
      .then(items => {
        if (active) setCategories(items.filter(category => category.is_active));
      })
      .catch(cause => {
        if (active) setCategoriesError(errorMessage(cause));
      });
    return () => {
      active = false;
    };
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
    searchLinkableVariants(supplierId, filter)
      .then(response => {
        if (requestId !== requestRef.current) return;
        setResult(response);
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
  }, [supplierId, filter, blockedReason]);

  const items = result?.data ?? [];
  const totalPages = Math.max(result?.total_pages ?? 0, 1);
  const selectionTooLarge = selection.size > MAX_BULK_LINKS;

  const updateSearch = (value: string) => {
    setSearchInput(value);
    setPage(1);
    setSelectAllNotice(null);
  };

  const updateCategory = (value: string) => {
    setCategoryId(value ? Number(value) : undefined);
    setPage(1);
    setSelectAllNotice(null);
  };

  const handleSelectAllResults = async () => {
    if (!result || selectingAll || blockedReason) return;
    setSelectingAll(true);
    setSelectAllNotice(null);
    setServerError(null);
    const { page: _page, page_size: _pageSize, ...allFilter } = filter;
    try {
      const response = await listLinkableVariantIds(supplierId, allFilter);
      setSelection(current => addIds(current, response.ids));
      if (response.truncated) {
        setSelectAllNotice('Se seleccionaron los primeros 500 resultados.');
      }
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
      const response = await linkSupplierVariants(supplierId, {
        variant_ids: [...selection.keys()],
        ...actorField('updated_by'),
      });
      onNotify(bulkLinkMessage(response), 'success');
      onSaved();
    } catch (cause) {
      setServerError(errorMessage(cause));
      submittingRef.current = false;
      setSaving(false);
    }
  };

  return (
    <Modal title="Enlazar productos" onClose={onClose} width={900}>
      <div className="supplier-picker">
        <div className="supplier-picker-controls">
          <input
            type="search"
            className="suppliers-input"
            autoFocus
            placeholder="Buscar producto, marca, SKU o código…"
            value={searchInput}
            onChange={event => updateSearch(event.target.value)}
          />
          <select
            className="suppliers-input supplier-picker-category"
            value={categoryId ?? ''}
            onChange={event => updateCategory(event.target.value)}
            aria-label="Filtrar por categoría"
          >
            <option value="">Todas las categorías</option>
            {categories.map(category => (
              <option key={category.id} value={category.id}>
                {category.name}
              </option>
            ))}
          </select>
          <label className="supplier-picker-unassigned">
            <input
              type="checkbox"
              checked={onlyUnassigned}
              onChange={event => {
                setOnlyUnassigned(event.target.checked);
                setPage(1);
                setSelectAllNotice(null);
              }}
            />
            <span>Solo productos sin proveedor</span>
          </label>
        </div>

        {categoriesError && (
          <div className="supplier-picker-error" role="alert">
            No se pudieron cargar las categorías: {categoriesError}
          </div>
        )}
        {serverError && (
          <div className="supplier-picker-error" role="alert">
            {serverError}
          </div>
        )}
        {selectAllNotice && (
          <p className="supplier-picker-notice" role="status">
            {selectAllNotice}
          </p>
        )}
        {selectionTooLarge && (
          <p className="supplier-picker-notice" role="status">
            Puedes guardar hasta 500 enlaces por vez. Quita algunos productos de la selección.
          </p>
        )}

        {blockedReason ? (
          <p className="supplier-picker-muted" role="status">
            {blockedReason}
          </p>
        ) : (
          <>
            <div className="supplier-picker-results-toolbar">
              <span>
                {loading
                  ? 'Buscando productos…'
                  : `${result?.total ?? 0} resultados`}
              </span>
              <button
                type="button"
                className="suppliers-btn"
                disabled={!result || result.total === 0 || selectingAll || loading}
                onClick={() => void handleSelectAllResults()}
              >
                {selectingAll
                  ? 'Seleccionando…'
                  : `Seleccionar todos los resultados (${result?.total ?? 0})`}
              </button>
            </div>

            {error && (
              <div className="supplier-picker-error" role="alert">
                No se pudieron buscar productos: {error}
              </div>
            )}

            {!error && !loading && items.length === 0 && (
              <p className="supplier-picker-muted">No se encontraron productos.</p>
            )}

            {items.length > 0 && (
              <div className="supplier-picker-table-wrap">
                <table className="supplier-picker-table">
                  <thead>
                    <tr>
                      <th>
                        <label className="supplier-picker-page-select">
                          <input
                            type="checkbox"
                            checked={allSelected(selection, items)}
                            onChange={() => {
                              setSelection(current =>
                                allSelected(current, items)
                                  ? removeAll(current, items)
                                  : addAll(current, items),
                              );
                            }}
                            aria-label="Seleccionar esta página"
                          />
                          <span>Seleccionar esta página</span>
                        </label>
                      </th>
                      <th>Producto</th>
                      <th>Categoría</th>
                      <th>Stock</th>
                      <th>Costo (inventario)</th>
                      <th>Proveedores</th>
                    </tr>
                  </thead>
                  <tbody>
                    {items.map(item => (
                      <PickerRow
                        key={item.variant_id}
                        item={item}
                        checked={selection.has(item.variant_id)}
                        onToggle={() =>
                          setSelection(current =>
                            toggleSelection(current, item.variant_id, variantLabel(item)),
                          )
                        }
                      />
                    ))}
                  </tbody>
                </table>
              </div>
            )}

            {result && totalPages > 1 && (
              <nav className="supplier-picker-pagination" aria-label="Paginación de productos">
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
                  disabled={loading || page >= totalPages}
                  onClick={() => setPage(current => current + 1)}
                >
                  Siguiente →
                </button>
              </nav>
            )}
          </>
        )}

        <div className="supplier-picker-selection">
          <div className="supplier-picker-selection-head">
            <strong>{selection.size} seleccionados</strong>
            {selection.size > 0 && (
              <button
                type="button"
                className="suppliers-btn"
                disabled={saving}
                onClick={() => setSelection(emptySelection())}
              >
                Vaciar
              </button>
            )}
          </div>
          {selection.size > 0 && (
            <div className="supplier-picker-chips" aria-label="Productos seleccionados">
              {[...selection].slice(0, 8).map(([id, label]) => (
                <span className="supplier-picker-chip" key={id}>
                  {label}
                  <button
                    type="button"
                    aria-label={`Quitar ${label} de la selección`}
                    disabled={saving}
                    onClick={() => setSelection(current => removeOne(current, id))}
                  >
                    ×
                  </button>
                </span>
              ))}
              {selection.size > 8 && (
                <span className="supplier-picker-chip-more">+{selection.size - 8} más</span>
              )}
            </div>
          )}
          <div className="supplier-picker-actions">
            <button type="button" className="suppliers-btn" onClick={onClose} disabled={saving}>
              Cancelar
            </button>
            <button
              type="button"
              className="suppliers-btn suppliers-btn-primary"
              disabled={selection.size === 0 || saving || selectionTooLarge}
              onClick={() => void handleSave()}
            >
              {saving ? 'Guardando…' : `Guardar ${selection.size} enlaces`}
            </button>
          </div>
        </div>
      </div>
    </Modal>
  );
};

interface PickerRowProps {
  item: LinkableVariant;
  checked: boolean;
  onToggle: () => void;
}

const PickerRow: React.FC<PickerRowProps> = ({ item, checked, onToggle }) => {
  const attributes = formatAttributes(item.attributes);
  const cost = item.variant_cost > 0 ? formatMoney(item.variant_cost) : 'Sin costo';
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
        {attributes && <span className="supplier-picker-sub">{attributes}</span>}
        {codes && <span className="supplier-picker-sub">{codes}</span>}
      </td>
      <td>{item.category_name}</td>
      <td><StockBadge stock={item.stock} stockMin={item.stock_min} /></td>
      <td>{cost}</td>
      <td>
        {item.linked_here ? (
          <span className="suppliers-badge">Ya enlazado</span>
        ) : item.supplier_count > 0 ? (
          `${item.supplier_count} proveedores`
        ) : (
          'Sin proveedor'
        )}
      </td>
    </tr>
  );
};
