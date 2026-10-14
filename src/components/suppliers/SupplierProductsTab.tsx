import React, { useCallback, useEffect, useRef, useState } from 'react';
import { searchVariants } from '../../services/inventory.service';
import {
  listSupplierVariants,
  unlinkSupplierVariant,
} from '../../services/suppliers.service';
import type { PaginatedResponse, VariantWithProduct } from '../../types';
import type { SupplierVariantView } from '../../types/suppliers';
import { errorMessage } from '../../utils/audit';
import { formatAttributes, formatDate, formatMoney } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import { useDebounce } from '../../hooks/useDebounce';
import { StockBadge } from '../inventory/StockBadge';
import { Modal } from '../ui/Modal';
import { SupplierLinkForm } from './SupplierLinkForm';
import './SuppliersBase.css';
import './SupplierProductsTab.css';

const PAGE_SIZE = 10;

interface SupplierProductsTabProps {
  supplierId: number;
  isActive: boolean;
  onChanged: () => void;
  onNotify: (message: string, type?: 'success' | 'error') => void;
}

type DialogState =
  | { kind: 'none' }
  | { kind: 'search' }
  | { kind: 'link'; variant: VariantWithProduct }
  | { kind: 'edit'; view: SupplierVariantView };

export const SupplierProductsTab: React.FC<SupplierProductsTabProps> = ({
  supplierId,
  isActive,
  onChanged,
  onNotify,
}) => {
  const [searchInput, setSearchInput] = useState('');
  const search = useDebounce(searchInput.trim(), 400);
  const [onlyPreferred, setOnlyPreferred] = useState(false);
  const [page, setPage] = useState(1);

  const [result, setResult] = useState<PaginatedResponse<SupplierVariantView> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const requestRef = useRef(0);

  const [dialog, setDialog] = useState<DialogState>({ kind: 'none' });
  const [unlinkingId, setUnlinkingId] = useState<number | null>(null);
  const [confirmUnlinkId, setConfirmUnlinkId] = useState<number | null>(null);

  // Búsqueda de variantes para enlazar
  const [variantSearchInput, setVariantSearchInput] = useState('');
  const debouncedVariantSearch = useDebounce(variantSearchInput.trim(), 300);
  const [variantResults, setVariantResults] = useState<VariantWithProduct[]>([]);
  const [searchingVariants, setSearchingVariants] = useState(false);
  const searchRequestRef = useRef(0);

  const refresh = useCallback(() => setReloadToken(t => t + 1), []);

  // Carga de variantes enlazadas
  useEffect(() => {
    const requestId = ++requestRef.current;
    setLoading(true);
    setError(null);

    listSupplierVariants(supplierId, {
      search: search || undefined,
      only_preferred: onlyPreferred ? true : undefined,
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
        setError(errorMessage(cause));
        setLoading(false);
      });

    return () => {
      requestRef.current += 1;
    };
  }, [supplierId, search, onlyPreferred, page, reloadToken]);

  // Búsqueda en vivo de variantes para el modal "Enlazar producto"
  useEffect(() => {
    if (debouncedVariantSearch.length < 2) {
      setVariantResults([]);
      setSearchingVariants(false);
      return;
    }

    const requestId = ++searchRequestRef.current;
    setSearchingVariants(true);

    searchVariants(debouncedVariantSearch)
      .then(items => {
        if (requestId !== searchRequestRef.current) return;
        setVariantResults(items);
        setSearchingVariants(false);
      })
      .catch(() => {
        if (requestId !== searchRequestRef.current) return;
        setVariantResults([]);
        setSearchingVariants(false);
      });

    return () => {
      searchRequestRef.current += 1;
    };
  }, [debouncedVariantSearch]);

  const handleUnlink = async (linkId: number) => {
    if (!isActive) return;
    setUnlinkingId(linkId);
    try {
      await unlinkSupplierVariant(linkId, { ...actorField('updated_by') });
      setConfirmUnlinkId(null);
      onNotify('Enlace eliminado con éxito', 'success');
      refresh();
      onChanged();
    } catch (cause) {
      onNotify(errorMessage(cause), 'error');
    } finally {
      setUnlinkingId(null);
    }
  };

  const links = result?.data ?? [];
  const totalPages = Math.max(result?.total_pages ?? 0, 1);

  return (
    <div className="supplier-products-tab">
      {!isActive && (
        <div className="supplier-tab-notice" role="note">
          Reactiva el proveedor para modificar sus enlaces.
        </div>
      )}

      <div className="supplier-products-toolbar">
        <div className="supplier-products-filters">
          <input
            type="text"
            className="suppliers-input supplier-products-search"
            placeholder="Buscar por producto, SKU, categoría…"
            value={searchInput}
            onChange={e => setSearchInput(e.target.value)}
          />
          <label className="supplier-products-preferred-filter">
            <input
              type="checkbox"
              checked={onlyPreferred}
              onChange={e => setOnlyPreferred(e.target.checked)}
            />
            <span>Solo preferidos</span>
          </label>
        </div>

        <button
          type="button"
          className="suppliers-btn suppliers-btn-primary"
          disabled={!isActive}
          onClick={() => {
            setVariantSearchInput('');
            setVariantResults([]);
            setDialog({ kind: 'search' });
          }}
        >
          + Enlazar producto
        </button>
      </div>

      {error && (
        <div className="supplier-tab-error" role="alert">
          <span>No se pudieron cargar los productos: {error}</span>
          <button
            type="button"
            className="suppliers-btn suppliers-btn-primary"
            onClick={refresh}
          >
            Reintentar
          </button>
        </div>
      )}

      {!error && !loading && links.length === 0 && (
        <p className="supplier-tab-muted">
          {search !== '' || onlyPreferred
            ? 'Ningún producto coincide con los filtros.'
            : 'Este proveedor aún no tiene productos enlazados.'}
        </p>
      )}

      {links.length > 0 && (
        <div className="supplier-products-table-wrap">
          <table className="supplier-products-table">
            <thead>
              <tr>
                <th>Producto</th>
                <th>Categoría</th>
                <th>Stock</th>
                <th>Costo</th>
                <th>SKU Prov.</th>
                <th>Preferido</th>
                <th>Último pedido</th>
                <th>Acciones</th>
              </tr>
            </thead>
            <tbody>
              {links.map(link => {
                const attrs = formatAttributes(link.attributes);
                const isConfirming = confirmUnlinkId === link.id;

                let costLabel = 'Sin costo';
                if (link.last_cost !== null) {
                  costLabel = formatMoney(link.last_cost);
                } else if (link.variant_cost > 0) {
                  costLabel = `${formatMoney(link.variant_cost)} (costo de variante)`;
                }

                return (
                  <tr key={link.id}>
                    <td>
                      <strong>{link.product_name}</strong>
                      {attrs && <span className="supplier-products-sub">{attrs}</span>}
                      {(link.sku || link.barcode) && (
                        <span className="supplier-products-sub">
                          {[link.sku ? `SKU: ${link.sku}` : null, link.barcode]
                            .filter(Boolean)
                            .join(' · ')}
                        </span>
                      )}
                    </td>
                    <td>{link.category_name}</td>
                    <td>
                      <StockBadge stock={link.stock} stockMin={link.stock_min} />
                    </td>
                    <td>{costLabel}</td>
                    <td>{link.supplier_sku ?? '—'}</td>
                    <td>
                      {link.is_preferred ? (
                        <span className="suppliers-badge suppliers-badge-preferred">
                          ★ Preferido
                        </span>
                      ) : (
                        '—'
                      )}
                    </td>
                    <td>{link.last_purchase_at ? formatDate(link.last_purchase_at) : '—'}</td>
                    <td>
                      {!isConfirming ? (
                        <div className="supplier-products-actions">
                          <button
                            type="button"
                            className="suppliers-btn"
                            disabled={!isActive}
                            onClick={() => setDialog({ kind: 'edit', view: link })}
                          >
                            Editar
                          </button>
                          <button
                            type="button"
                            className="suppliers-btn suppliers-btn-danger"
                            disabled={!isActive}
                            onClick={() => setConfirmUnlinkId(link.id)}
                          >
                            Quitar
                          </button>
                        </div>
                      ) : (
                        <div className="supplier-unlink-confirm">
                          <span>
                            Se quita el enlace con este proveedor. El producto y su stock
                            no cambian.
                          </span>
                          <button
                            type="button"
                            className="suppliers-btn"
                            disabled={unlinkingId === link.id}
                            onClick={() => setConfirmUnlinkId(null)}
                          >
                            Cancelar
                          </button>
                          <button
                            type="button"
                            className="suppliers-btn suppliers-btn-danger"
                            disabled={unlinkingId === link.id}
                            onClick={() => void handleUnlink(link.id)}
                          >
                            {unlinkingId === link.id ? 'Quitando…' : 'Confirmar quitar'}
                          </button>
                        </div>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {totalPages > 1 && (
        <nav className="suppliers-pagination" aria-label="Paginación de productos enlazados">
          <button
            type="button"
            className="suppliers-btn"
            disabled={loading || page <= 1}
            onClick={() => setPage(p => p - 1)}
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
            onClick={() => setPage(p => p + 1)}
          >
            Siguiente →
          </button>
        </nav>
      )}

      {/* Modal buscador de variantes */}
      {dialog.kind === 'search' && (
        <Modal
          title="Buscar producto para enlazar"
          onClose={() => setDialog({ kind: 'none' })}
          width={640}
        >
          <div className="variant-search-wrap">
            <input
              type="text"
              className="suppliers-input"
              autoFocus
              placeholder="Escribe al menos 2 caracteres (nombre, SKU o código)…"
              value={variantSearchInput}
              onChange={e => setVariantSearchInput(e.target.value)}
            />

            {searchingVariants && (
              <p className="supplier-tab-muted">Buscando productos…</p>
            )}

            {!searchingVariants &&
              variantSearchInput.trim().length >= 2 &&
              variantResults.length === 0 && (
                <p className="supplier-tab-muted">
                  No se encontraron productos coincidentes.
                </p>
              )}

            {variantResults.length > 0 && (
              <div className="variant-search-results" role="listbox">
                {variantResults.map(v => {
                  const attrs = formatAttributes(v.attributes);
                  return (
                    <button
                      key={v.id}
                      type="button"
                      className="variant-search-item"
                      onClick={() => setDialog({ kind: 'link', variant: v })}
                    >
                      <div className="variant-search-item-info">
                        <span className="variant-search-item-title">
                          {v.product_name}
                        </span>
                        <span className="variant-search-item-meta">
                          {attrs ? `${attrs} · ` : ''}
                          Categoría: {v.category_name}
                          {v.sku ? ` · SKU: ${v.sku}` : ''}
                        </span>
                      </div>
                      <StockBadge stock={v.stock} stockMin={v.stock_min} />
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </Modal>
      )}

      {/* Modal formulario de nuevo enlace */}
      {dialog.kind === 'link' && (
        <Modal
          title="Enlazar producto a proveedor"
          onClose={() => setDialog({ kind: 'none' })}
          width={620}
        >
          <SupplierLinkForm
            supplierId={supplierId}
            variant={{
              id: dialog.variant.id,
              product_name: dialog.variant.product_name,
              attributes: dialog.variant.attributes,
              cost: dialog.variant.cost,
            }}
            initial={null}
            onSaved={() => {
              setDialog({ kind: 'none' });
              onNotify('Producto enlazado con éxito', 'success');
              refresh();
              onChanged();
            }}
            onCancel={() => setDialog({ kind: 'search' })}
          />
        </Modal>
      )}

      {/* Modal formulario de edición de enlace */}
      {dialog.kind === 'edit' && (
        <Modal
          title="Editar enlace de producto"
          onClose={() => setDialog({ kind: 'none' })}
          width={620}
        >
          <SupplierLinkForm
            supplierId={supplierId}
            variant={{
              id: dialog.view.variant_id,
              product_name: dialog.view.product_name,
              attributes: dialog.view.attributes,
              cost: dialog.view.variant_cost,
            }}
            initial={dialog.view}
            onSaved={() => {
              setDialog({ kind: 'none' });
              onNotify('Enlace actualizado con éxito', 'success');
              refresh();
              onChanged();
            }}
            onCancel={() => setDialog({ kind: 'none' })}
          />
        </Modal>
      )}
    </div>
  );
};
