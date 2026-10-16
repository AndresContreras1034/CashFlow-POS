import { useEffect, useRef, useState } from 'react';
import type { PurchaseItem } from '../../types/purchases';
import { errorMessage } from '../../utils/audit';
import { MAX_QUANTITY, resolveQuantityCommit } from '../../utils/purchaseList';

export interface ItemPatch {
  quantity: number;
  supplier_id: number | null;
  note: string | null;
}

interface PurchaseLineRowProps {
  item: PurchaseItem;
  editable: boolean;
  onSave: (item: PurchaseItem, patch: ItemPatch) => Promise<void>;
  onRemove: (item: PurchaseItem) => Promise<void>;
}

export const PurchaseLineRow: React.FC<PurchaseLineRowProps> = ({
  item,
  editable,
  onSave,
  onRemove,
}) => {
  const [quantity, setQuantity] = useState(String(item.quantity));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const busyRef = useRef(false);

  useEffect(() => {
    setQuantity(String(item.quantity));
  }, [item.quantity]);

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

  return (
    <li className="purchase-text-line">
      <span className="purchase-text-product">{item.product_name}</span>
      <span className="purchase-text-times" aria-hidden="true">×</span>
      {editable ? (
        <input
          className="suppliers-input purchase-text-quantity"
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
      ) : (
        <span className="purchase-text-quantity-readonly">{item.quantity}</span>
      )}
      <span className="purchase-text-unit">unid.</span>
      <span className="purchase-text-spacer" />
      {editable && (confirmRemove ? (
        <span className="purchase-text-confirm-remove">
          <button type="button" className="suppliers-btn suppliers-btn-danger" disabled={busy} onClick={() => void run(() => onRemove(item))}>
            Quitar
          </button>
          <button type="button" className="suppliers-btn" disabled={busy} onClick={() => setConfirmRemove(false)}>
            Cancelar
          </button>
        </span>
      ) : (
        <button
          type="button"
          className="purchase-text-remove"
          aria-label={`Quitar ${item.product_name}`}
          disabled={busy}
          onClick={() => setConfirmRemove(true)}
        >
          ×
        </button>
      ))}
      {error && <span className="purchase-line-error" role="alert">{error}</span>}
    </li>
  );
};
