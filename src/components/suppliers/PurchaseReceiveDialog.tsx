import { useMemo, useRef, useState } from 'react';
import type {
  PurchaseItem,
  PurchaseList,
  ReceiveResult,
} from '../../types/purchases';
import { receivePurchaseList } from '../../services/purchases.service';
import { errorMessage } from '../../utils/audit';
import { formatAttributes, formatMoney, moneyStep } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import { groupBySupplier } from '../../utils/purchaseList';
import {
  initialReceiveRows,
  validateReceive,
  type ReceiveRow,
} from '../../utils/purchaseReceive';
import { Modal } from '../ui/Modal';
import './SuppliersBase.css';
import './PurchaseReceiveDialog.css';

interface PurchaseReceiveDialogProps {
  list: PurchaseList;
  items: readonly PurchaseItem[];
  currencyDecimals: number;
  onClose: () => void;
  onReceived: (result: ReceiveResult) => void;
}

export const PurchaseReceiveDialog: React.FC<PurchaseReceiveDialogProps> = ({
  list,
  items,
  currencyDecimals,
  onClose,
  onReceived,
}) => {
  const [rows, setRows] = useState<ReceiveRow[]>(() => initialReceiveRows(items, currencyDecimals));
  const [confirming, setConfirming] = useState(false);
  const [saving, setSaving] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);
  const submittingRef = useRef(false);
  const validation = useMemo(
    () => validateReceive(rows, currencyDecimals),
    [rows, currencyDecimals],
  );
  const groups = useMemo(() => groupBySupplier(items), [items]);

  const close = () => {
    if (!submittingRef.current) onClose();
  };

  const updateRow = (itemId: number, patch: Partial<ReceiveRow>) => {
    setRows(current => current.map(row => row.itemId === itemId ? { ...row, ...patch } : row));
    setServerError(null);
  };

  const confirm = async () => {
    if (submittingRef.current || validation.lines === null) return;
    submittingRef.current = true;
    setSaving(true);
    setServerError(null);
    try {
      const result = await receivePurchaseList(list.id, {
        lines: validation.lines,
        ...actorField('updated_by'),
      });
      onReceived(result);
    } catch (cause) {
      setServerError(errorMessage(cause));
      submittingRef.current = false;
      setSaving(false);
    }
  };

  const rowMap = new Map(rows.map(row => [row.itemId, row]));

  return (
    <Modal
      title="Registrar compra recibida"
      onClose={close}
      width={900}
    >
      <div className="purchase-receive">
        <p className="purchase-receive-intro">
          Indica lo que realmente llegó. La cantidad parte de lo pedido; pon 0 en lo que no llegó.
          El costo parte del costo actual del inventario: corrígelo con lo que pagaste.
        </p>

        {serverError && <div className="purchase-receive-error" role="alert">{serverError}</div>}

        <div className="purchase-receive-table-wrap">
          {groups.map(group => (
            <section className="purchase-receive-group" key={group.key ?? 'sin-proveedor'}>
              <h3>{group.supplier?.name ?? 'Sin proveedor'}</h3>
              <table className="purchase-receive-table">
                <caption className="purchase-receive-sr-only">
                  Recepción de productos de {group.supplier?.name ?? 'Sin proveedor'}
                </caption>
                <thead>
                  <tr>
                    <th scope="col">Producto</th>
                    <th className="suppliers-num" scope="col">Stock actual</th>
                    <th className="suppliers-num" scope="col">Pedido</th>
                    <th className="suppliers-num" scope="col">Recibido</th>
                    <th className="suppliers-num" scope="col">Costo unitario pagado</th>
                  </tr>
                </thead>
                <tbody>
                  {group.items.map(item => {
                    const row = rowMap.get(item.item_id);
                    const errors = validation.errors[item.item_id];
                    if (!row) return null;
                    return (
                      <tr key={item.item_id}>
                        <td>
                          <span className="purchase-receive-product">{item.product_name}</span>
                          {formatAttributes(item.attributes) && (
                            <span className="purchase-receive-sub">
                              {formatAttributes(item.attributes)}
                            </span>
                          )}
                        </td>
                        <td className="suppliers-num">{item.stock}</td>
                        <td className="suppliers-num">{item.quantity}</td>
                        <td>
                          <input
                            className="suppliers-input purchase-receive-quantity"
                            type="number"
                            min={0}
                            max={100_000}
                            step={1}
                            inputMode="numeric"
                            aria-label={`Cantidad recibida de ${item.product_name}`}
                            aria-invalid={errors?.quantity ? true : undefined}
                            aria-describedby={errors?.quantity
                              ? `purchase-receive-quantity-error-${item.item_id}`
                              : undefined}
                            value={row.quantity}
                            disabled={saving}
                            onChange={event => updateRow(item.item_id, { quantity: event.target.value })}
                          />
                          {errors?.quantity && (
                            <span
                              id={`purchase-receive-quantity-error-${item.item_id}`}
                              className="purchase-receive-field-error"
                              role="alert"
                            >{errors.quantity}</span>
                          )}
                        </td>
                        <td>
                          <input
                            className="suppliers-input purchase-receive-cost"
                            type="number"
                            min={0}
                            step={moneyStep(currencyDecimals)}
                            inputMode="decimal"
                            aria-label={`Costo unitario pagado de ${item.product_name}`}
                            aria-invalid={errors?.unitCost ? true : undefined}
                            aria-describedby={errors?.unitCost
                              ? `purchase-receive-cost-error-${item.item_id}`
                              : undefined}
                            value={row.unitCost}
                            disabled={saving}
                            onChange={event => updateRow(item.item_id, { unitCost: event.target.value })}
                          />
                          {errors?.unitCost && (
                            <span
                              id={`purchase-receive-cost-error-${item.item_id}`}
                              className="purchase-receive-field-error"
                              role="alert"
                            >{errors.unitCost}</span>
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </section>
          ))}
        </div>

        <div className="purchase-receive-summary" aria-live="polite">
          <strong>
            {validation.receivedLines} {validation.receivedLines === 1 ? 'producto' : 'productos'}
            {' · '}{validation.units} {validation.units === 1 ? 'unidad' : 'unidades'}
            {' · '}Total aproximado {formatMoney(validation.estimatedTotal)}
          </strong>
          {validation.formError && (
            <span className="purchase-receive-field-error" role="alert">{validation.formError}</span>
          )}
        </div>

        {confirming && (
          <div className="purchase-receive-confirm" role="status">
            <p>
              Se sumarán {validation.units} {validation.units === 1 ? 'unidad' : 'unidades'} al
              inventario y se actualizará el costo de los productos con costo informado. Esta
              acción no se puede deshacer.
            </p>
            <div className="purchase-receive-actions">
              <button
                type="button"
                className="suppliers-btn"
                disabled={saving}
                onClick={() => setConfirming(false)}
              >Volver</button>
              <button
                type="button"
                className="suppliers-btn suppliers-btn-primary"
                disabled={saving || validation.lines === null}
                onClick={() => void confirm()}
              >{saving ? 'Registrando…' : 'Confirmar recepción'}</button>
            </div>
          </div>
        )}

        <div className="purchase-receive-actions">
          {!confirming ? (
            <>
              <button type="button" className="suppliers-btn" disabled={saving} onClick={close}>
                Cancelar
              </button>
              <button
                type="button"
                className="suppliers-btn suppliers-btn-primary"
                disabled={saving || validation.lines === null}
                onClick={() => setConfirming(true)}
              >Revisar</button>
            </>
          ) : (
            <button type="button" className="suppliers-btn" disabled={saving} onClick={close}>
              Cancelar
            </button>
          )}
        </div>
      </div>
    </Modal>
  );
};
