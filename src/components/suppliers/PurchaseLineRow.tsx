import { useEffect, useRef, useState } from 'react';
import type { PurchaseItem } from '../../types/purchases';
import { errorMessage } from '../../utils/audit';
import { calcMargin, formatAttributes, formatMargin, formatMoney } from '../../utils/format';
import {
  MAX_QUANTITY,
  PURCHASE_SOURCE_LABELS,
  resolveQuantityCommit,
  type SupplierOption,
} from '../../utils/purchaseList';
import { StockBadge } from '../inventory/StockBadge';

export interface ItemPatch {
  quantity: number;
  supplier_id: number | null;
  note: string | null;
}

interface PurchaseLineRowProps {
  item: PurchaseItem;
  editable: boolean;
  supplierOptions: readonly SupplierOption[];
  onSave: (item: PurchaseItem, patch: ItemPatch) => Promise<void>;
  onRemove: (item: PurchaseItem) => Promise<void>;
}

const MAX_NOTE = 500;

export const PurchaseLineRow: React.FC<PurchaseLineRowProps> = ({
  item,
  editable,
  supplierOptions,
  onSave,
  onRemove,
}) => {
  const [quantity, setQuantity] = useState(String(item.quantity));
  const [note, setNote] = useState(item.note ?? '');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const busyRef = useRef(false);

  useEffect(() => {
    setQuantity(String(item.quantity));
    setNote(item.note ?? '');
  }, [item.quantity, item.note]);

  const run = async (action: () => Promise<void>) => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (cause) {
      setError(errorMessage(cause));
      setQuantity(String(item.quantity));
      setNote(item.note ?? '');
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  };

  const current = (): ItemPatch => ({
    quantity: item.quantity,
    supplier_id: item.supplier?.id ?? null,
    note: item.note,
  });

  const commitQuantity = () => {
    const result = resolveQuantityCommit(quantity, item.quantity);
    if (result.kind === 'invalid') {
      setError(`La cantidad debe ser un entero de 1 a ${MAX_QUANTITY}`);
      return;
    }
    setError(null);
    if (result.kind === 'unchanged') {
      setQuantity(String(item.quantity));
      return;
    }
    void run(() => onSave(item, { ...current(), quantity: result.value }));
  };

  const commitNote = () => {
    const next = note.trim() === '' ? null : note.trim();
    if (next === item.note) return;
    if (next !== null && next.length > MAX_NOTE) {
      setError(`La nota no puede superar ${MAX_NOTE} caracteres`);
      return;
    }
    void run(() => onSave(item, { ...current(), note: next }));
  };

  const changeSupplier = (value: string) => {
    const supplierId = value === '' ? null : Number(value);
    if (supplierId === (item.supplier?.id ?? null)) return;
    void run(() => onSave(item, { ...current(), supplier_id: supplierId }));
  };

  const attributes = formatAttributes(item.attributes);
  const margin = formatMargin(calcMargin(item.variant_price, item.variant_cost));

  return (
    <tr className="purchase-line">
      <td className="purchase-line-product">
        <span className="purchase-line-name">{item.product_name}</span>
        {attributes && <span className="purchase-line-sub">{attributes}</span>}
        <span className={item.source === 'recommended'
          ? 'suppliers-badge suppliers-badge-low'
          : 'suppliers-badge suppliers-badge-preferred'}>
          {PURCHASE_SOURCE_LABELS[item.source]}
        </span>
        {editable ? (
          <input
            className="suppliers-input purchase-line-note"
            type="text"
            placeholder="Nota (opcional)"
            aria-label={`Nota de ${item.product_name}`}
            maxLength={MAX_NOTE + 1}
            value={note}
            disabled={busy}
            onChange={event => setNote(event.target.value)}
            onBlur={commitNote}
          />
        ) : (
          item.note && <span className="purchase-line-sub">{item.note}</span>
        )}
        {error && <span className="purchase-line-error" role="alert">{error}</span>}
      </td>
      <td><StockBadge stock={item.stock} stockMin={item.stock_min} /></td>
      <td>
        {editable ? (
          <select
            className="suppliers-input"
            aria-label={`Proveedor de ${item.product_name}`}
            value={item.supplier?.id ?? ''}
            disabled={busy}
            onChange={event => changeSupplier(event.target.value)}
          >
            <option value="">Sin proveedor</option>
            {supplierOptions.map(option => (
              <option key={option.id} value={option.id}>
                {option.name}{option.inactive ? ' (desactivado)' : ''}
              </option>
            ))}
          </select>
        ) : item.supplier?.name ?? 'Sin proveedor'}
      </td>
      <td className="suppliers-num">
        {editable ? (
          <input
            className="suppliers-input purchase-line-qty"
            type="number"
            min={1}
            max={MAX_QUANTITY}
            step={1}
            inputMode="numeric"
            aria-label={`Cantidad de ${item.product_name}`}
            aria-invalid={error ? true : undefined}
            value={quantity}
            disabled={busy}
            onChange={event => setQuantity(event.target.value)}
            onBlur={commitQuantity}
            onKeyDown={event => {
              if (event.key === 'Enter') event.currentTarget.blur();
            }}
          />
        ) : item.quantity}
        {item.suggested_qty !== null && item.suggested_qty !== item.quantity && (
          <span className="purchase-line-sub">Sugerido: {item.suggested_qty}</span>
        )}
      </td>
      <td className="suppliers-num tabular">
        {item.variant_cost > 0 ? formatMoney(item.variant_cost) : 'Sin costo'}
        <span className="purchase-line-sub">Margen {margin}</span>
      </td>
      <td className="suppliers-num tabular">
        {item.line_cost === null ? '—' : formatMoney(item.line_cost)}
      </td>
      <td className="purchase-line-actions">
        {editable && (confirmRemove ? (
          <>
            <button
              type="button"
              className="suppliers-btn suppliers-btn-danger"
              disabled={busy}
              onClick={() => void run(() => onRemove(item))}
            >Quitar</button>
            <button type="button" className="suppliers-btn" onClick={() => setConfirmRemove(false)}>
              No
            </button>
          </>
        ) : (
          <button
            type="button"
            className="suppliers-btn"
            aria-label={`Quitar ${item.product_name} de la lista`}
            disabled={busy}
            onClick={() => setConfirmRemove(true)}
          >×</button>
        ))}
      </td>
    </tr>
  );
};
