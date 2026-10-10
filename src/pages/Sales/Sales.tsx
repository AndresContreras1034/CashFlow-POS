import { useState, useRef, useCallback, useEffect } from 'react';
import type { CreateSalePaymentDto, PaymentMethod, VariantWithProduct } from '../../types';
import { findByBarcode, searchVariants, getVariant } from '../../services/inventory.service';
import { createSale } from '../../services/sales.service';
import { getCurrentCashSession } from '../../services/cash.service';
import { formatMoney, parseMoneyInput, minorToInput, moneyStep } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import './Sales.css';

interface CartLine {
  variant_id: number;
  product_name: string;
  attributes: Record<string, string>;
  unit_price: number;
  quantity: number;
  discount: number;
  stock: number; // referencia visual, la validación real es del backend
}

function attributesLabel(attrs: Record<string, string>) {
  const entries = Object.entries(attrs || {});
  if (entries.length === 0) return '';
  return entries.map(([k, v]) => `${k}: ${v}`).join(', ');
}

export default function Sales() {
  const [scanValue, setScanValue] = useState('');
  const [searchResults, setSearchResults] = useState<VariantWithProduct[]>([]);
  const [cart, setCart] = useState<CartLine[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [cashOpen, setCashOpen] = useState<boolean | null>(null);
  const [lastSaleId, setLastSaleId] = useState<number | null>(null);
  const [isCourtesy, setIsCourtesy] = useState(false);
  const [courtesyReason, setCourtesyReason] = useState('');
  const [cashReceivedInput, setCashReceivedInput] = useState('');

  const [payments, setPayments] = useState<CreateSalePaymentDto[]>([
    { method: 'cash', amount: 0 },
  ]);

  const scanInputRef = useRef<HTMLInputElement>(null);
  const attemptRef = useRef<{ fingerprint: string; key: string } | null>(null);
  const submittingRef = useRef(false);

  const refreshCash = useCallback(async () => {
    try {
      setCashOpen((await getCurrentCashSession()) !== null);
    } catch {
      setCashOpen(null);
    }
  }, []);

  useEffect(() => {
    void refreshCash();
  }, [refreshCash]);

  const subtotal = cart.reduce(
    (sum, line) => sum + line.unit_price * line.quantity - line.discount,
    0
  );
  const courtesyMode = cart.length > 0 && (isCourtesy || subtotal === 0);
  const total = courtesyMode ? 0 : subtotal;
  const paymentsTotal = payments.reduce((sum, p) => sum + (p.amount || 0), 0);
  const remaining = courtesyMode ? 0 : total - paymentsTotal;
  const cashNet = payments.reduce(
    (sum, payment) => sum + (payment.method === 'cash' ? payment.amount : 0),
    0
  );
  const cashReceived =
    cashReceivedInput.trim() === '' ? null : parseMoneyInput(cashReceivedInput);
  const cashReceivedInvalid = cashReceivedInput.trim() !== '' && cashReceived === null;
  const cashShortage = cashReceived !== null && cashReceived < cashNet;
  const cashChange =
    cashReceived !== null && !cashShortage ? cashReceived - cashNet : null;
  const cashBlocked = cashOpen === false;

  useEffect(() => {
    if (cashNet === 0 || courtesyMode) setCashReceivedInput('');
  }, [cashNet, courtesyMode]);

  function addToCart(v: VariantWithProduct) {
    setNotice(null);
    setCart((prev) => {
      const existing = prev.find((l) => l.variant_id === v.id);
      if (existing) {
        return prev.map((l) =>
          l.variant_id === v.id ? { ...l, quantity: l.quantity + 1 } : l
        );
      }
      return [
        ...prev,
        {
          variant_id: v.id,
          product_name: v.product_name,
          attributes: v.attributes,
          unit_price: v.price,
          quantity: 1,
          discount: 0,
          stock: v.stock,
        },
      ];
    });
    setSearchResults([]);
    setScanValue('');
    scanInputRef.current?.focus();
  }

  async function handleScanSubmit(e: React.FormEvent) {
    e.preventDefault();
    setNotice(null);
    if (!scanValue.trim()) return;
    setError(null);

    try {
      // Primero intenta como código de barras exacto
      const variant = await findByBarcode(scanValue.trim());
      addToCart(variant);
    } catch {
      // Si no hay match exacto, cae a búsqueda por texto
      try {
        const results = await searchVariants(scanValue.trim());
        if (results.length === 0) {
          setNotice(`No se encontró ningún producto para «${scanValue.trim()}»`);
        } else if (results.length === 1) {
          addToCart(results[0]);
        } else {
          setSearchResults(results);
        }
      } catch (e2) {
        setError(String(e2));
      }
    }
  }

  function updateQuantity(variantId: number, quantity: number) {
    setCart((prev) =>
      prev.map((l) =>
        l.variant_id === variantId ? { ...l, quantity: Math.max(1, quantity) } : l
      )
    );
  }

  function updateDiscount(variantId: number, pesos: string) {
    const cents = pesos.trim() === '' ? 0 : parseMoneyInput(pesos);
    if (cents === null) return;
    setCart((prev) =>
      prev.map((l) => (l.variant_id === variantId ? { ...l, discount: cents } : l))
    );
  }

  function removeLine(variantId: number) {
    setCart((prev) => prev.filter((l) => l.variant_id !== variantId));
  }

  function updatePaymentMethod(index: number, method: PaymentMethod) {
    setPayments((prev) => prev.map((p, i) => (i === index ? { ...p, method } : p)));
  }

  function updatePaymentAmount(index: number, pesos: string) {
    const cents = pesos.trim() === '' ? 0 : parseMoneyInput(pesos);
    if (cents === null) return;
    setPayments((prev) => prev.map((p, i) => (i === index ? { ...p, amount: cents } : p)));
  }

  function addPaymentLine() {
    setPayments((prev) => [...prev, { method: 'card', amount: 0 }]);
  }

  function removePaymentLine(index: number) {
    setPayments((prev) => prev.filter((_, i) => i !== index));
  }

  async function detectPriceChanges(): Promise<string[]> {
    const changes: string[] = [];
    for (const line of cart) {
      const fresh = await getVariant(line.variant_id);
      if (fresh.price !== line.unit_price) {
        changes.push(
          `${line.product_name}: ${formatMoney(line.unit_price)} → ${formatMoney(fresh.price)}`
        );
        setCart((prev) =>
          prev.map((l) =>
            l.variant_id === line.variant_id ? { ...l, unit_price: fresh.price } : l
          )
        );
      }
    }
    return changes;
  }

  function fillRemainingOnFirstPayment() {
    setPayments((prev) => {
      if (prev.length === 0) return prev;
      const [first, ...rest] = prev;
      return [{ ...first, amount: total }, ...rest];
    });
  }

  const resetSale = useCallback(() => {
    attemptRef.current = null;
    setCart([]);
    setPayments([{ method: 'cash', amount: 0 }]);
    setCashReceivedInput('');
    setLastSaleId(null);
    setIsCourtesy(false);
    setCourtesyReason('');
    setError(null);
    void refreshCash();
  }, [refreshCash]);

  async function handleConfirmSale() {
    if (submittingRef.current) return;
    setError(null);

    if (cart.length === 0) {
      setError('Agrega al menos un producto al carrito');
      return;
    }
    submittingRef.current = true;
    setConfirming(true);
    try {
      const priceChanges = await detectPriceChanges();
      if (priceChanges.length > 0) {
        setError(
          `Los precios cambiaron: ${priceChanges.join('; ')}. Revisa el total y los pagos.`
        );
        return;
      }
      if (!courtesyMode && remaining !== 0) {
        setError(
          remaining > 0
            ? `Faltan ${formatMoney(remaining)} por cubrir en los pagos`
            : `Los pagos exceden el total en ${formatMoney(-remaining)}`
        );
        return;
      }
      if (!courtesyMode && cashShortage) {
        setError(`El efectivo recibido es menor al efectivo por cubrir (${formatMoney(cashNet)})`);
        return;
      }
      if (!courtesyMode && cashReceivedInvalid) {
        setError('Ingresa un valor válido para el efectivo recibido');
        return;
      }
      const trimmedCourtesyReason = courtesyReason.trim();
      if (courtesyMode && !trimmedCourtesyReason) {
        setError('Escribe el motivo de la venta de cortesía');
        return;
      }

      const payload = {
        items: cart.map((l) => ({
          variant_id: l.variant_id,
          quantity: l.quantity,
          discount: l.discount || null,
        })),
        discount: courtesyMode ? subtotal : null,
        payments: courtesyMode ? [] : payments.filter((p) => p.amount > 0),
        cash_received: courtesyMode ? null : cashReceived,
        courtesy_reason: courtesyMode ? trimmedCourtesyReason : null,
        ...actorField('created_by'),
      };
      const fingerprint = JSON.stringify(payload);
      if (attemptRef.current?.fingerprint !== fingerprint) {
        attemptRef.current = { fingerprint, key: crypto.randomUUID() };
      }

      const attempt = attemptRef.current;
      const sale = await createSale({ ...payload, idempotency_key: attempt.key });
      attemptRef.current = null;
      setLastSaleId(sale.id);
      setCart([]);
      setPayments([{ method: 'cash', amount: 0 }]);
      setCashReceivedInput('');
      setIsCourtesy(false);
      setCourtesyReason('');
    } catch (e) {
      const message = String(e);
      setError(message);
      if (message.toLowerCase().includes('turno de caja')) {
        void refreshCash();
      }
    } finally {
      submittingRef.current = false;
      setConfirming(false);
    }
  }

  return (
    <div className="sales-page">
      <h1>Ventas</h1>

      {error && <div className="form-error">{error}</div>}
      {notice && <div className="sales-meta">{notice}</div>}

      {lastSaleId && (
        <div className="sales-success">
          Venta #{lastSaleId} registrada correctamente.
          <button className="btn" onClick={resetSale}>
            Nueva venta
          </button>
        </div>
      )}

      <div className="sales-card">
        <form onSubmit={handleScanSubmit} className="sales-scan-form">
          <input
            ref={scanInputRef}
            className="form-input"
            type="text"
            autoFocus
            placeholder="Escanea un código de barras o escribe para buscar..."
            value={scanValue}
            onChange={(e) => setScanValue(e.target.value)}
          />
          <button type="submit" className="btn btn-primary">
            Agregar
          </button>
        </form>

        {searchResults.length > 0 && (
          <ul className="sales-search-results">
            {searchResults.map((v) => (
              <li key={v.id} onClick={() => addToCart(v)}>
                <span>{v.product_name} {attributesLabel(v.attributes)}</span>
                <span>{formatMoney(v.price)} · stock: {v.stock}</span>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="sales-card">
        <h2>Carrito</h2>
        {cart.length === 0 ? (
          <p className="sales-meta">Sin productos todavía.</p>
        ) : (
          <table className="sales-table">
            <thead>
              <tr>
                <th>Producto</th>
                <th>Precio unit.</th>
                <th>Cant.</th>
                <th>Descuento</th>
                <th>Subtotal</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {cart.map((line) => (
                <tr key={line.variant_id}>
                  <td>
                    {line.product_name}
                    {attributesLabel(line.attributes) && (
                      <div className="sales-line-attrs">{attributesLabel(line.attributes)}</div>
                    )}
                  </td>
                  <td>{formatMoney(line.unit_price)}</td>
                  <td>
                    <input
                      className="form-input sales-qty-input"
                      type="number"
                      min={1}
                      value={line.quantity}
                      onChange={(e) =>
                        updateQuantity(line.variant_id, parseInt(e.target.value || '1', 10))
                      }
                    />
                  </td>
                  <td>
                    <input
                      className="form-input sales-qty-input"
                      type="number"
                      min={0}
                      step={moneyStep()}
                      value={minorToInput(line.discount)}
                      onChange={(e) => updateDiscount(line.variant_id, e.target.value)}
                    />
                  </td>
                  <td>{formatMoney(line.unit_price * line.quantity - line.discount)}</td>
                  <td>
                    <button
                      className="btn btn-danger sales-remove-btn"
                      onClick={() => removeLine(line.variant_id)}
                    >
                      ✕
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {cart.length > 0 && (
        <div className="sales-card">
          <h2>Pago</h2>
          <label className="sales-courtesy-toggle">
            <input
              type="checkbox"
              checked={courtesyMode}
              disabled={subtotal === 0}
              onChange={(e) => {
                const checked = e.target.checked;
                setIsCourtesy(checked);
                setPayments(checked ? [] : [{ method: 'cash', amount: 0 }]);
              }}
            />
            Venta de cortesía (total $0)
          </label>
          {courtesyMode && (
            <div className="form-field">
              <label htmlFor="courtesy-reason">Motivo de la cortesía</label>
              <textarea
                id="courtesy-reason"
                className="form-input sales-courtesy-reason"
                maxLength={200}
                value={courtesyReason}
                onChange={(e) => setCourtesyReason(e.target.value)}
              />
              <span className="sales-meta">{courtesyReason.length}/200</span>
            </div>
          )}
          <div className="sales-total-row">
            <span>Total a pagar</span>
            <span className="sales-total-value">{formatMoney(total)}</span>
          </div>

          {!courtesyMode && (
            <>
              {payments.map((p, i) => (
                <div className="sales-payment-row" key={i}>
                  <select
                    className="form-input"
                    value={p.method}
                    onChange={(e) => updatePaymentMethod(i, e.target.value as PaymentMethod)}
                  >
                    <option value="cash">Efectivo</option>
                    <option value="card">Tarjeta</option>
                    <option value="transfer">Transferencia</option>
                  </select>
                  <input
                    className="form-input"
                    type="number"
                    min={0}
                    step={moneyStep()}
                    placeholder="0"
                    value={p.amount ? minorToInput(p.amount) : ''}
                    onChange={(e) => updatePaymentAmount(i, e.target.value)}
                  />
                  {payments.length > 1 && (
                    <button className="btn btn-danger" onClick={() => removePaymentLine(i)}>
                      ✕
                    </button>
                  )}
                </div>
              ))}

              <div className="sales-payment-actions">
                <button className="btn" onClick={addPaymentLine}>
                  + Agregar forma de pago
                </button>
                <button className="btn" onClick={fillRemainingOnFirstPayment}>
                  Llenar con el total
                </button>
              </div>
            </>
          )}
          {!courtesyMode && cashNet > 0 && (
            <div className="sales-cash-tender">
              <div className="sales-cash-input-row">
                <label htmlFor="cash-received">Recibido en efectivo</label>
                <input
                  id="cash-received"
                  className="form-input"
                  type="number"
                  min={0}
                  step={moneyStep()}
                  value={cashReceivedInput}
                  onChange={(e) => setCashReceivedInput(e.target.value)}
                />
              </div>
              <div className="sales-cash-actions">
                <button
                  className="btn"
                  type="button"
                  onClick={() => setCashReceivedInput(minorToInput(cashNet))}
                >
                  Exacto
                </button>
                {[2_000, 5_000, 10_000, 20_000, 50_000, 100_000]
                  .map((pesos) => pesos * 100)
                  .filter((amount) => amount >= cashNet)
                  .map((amount) => (
                    <button
                      className="btn"
                      type="button"
                      key={amount}
                      onClick={() => setCashReceivedInput(minorToInput(amount))}
                    >
                      {formatMoney(amount)}
                    </button>
                  ))}
              </div>
              {cashShortage && (
                <div className="sales-cash-shortage">
                  Faltan {formatMoney(cashNet - (cashReceived ?? 0))} de efectivo.
                </div>
              )}
              {cashReceivedInvalid && (
                <div className="sales-cash-shortage">
                  Ingresa un valor válido para el efectivo recibido.
                </div>
              )}
              {cashChange !== null && (
                <div className="sales-cash-change">Vuelto: {formatMoney(cashChange)}</div>
              )}
            </div>
          )}

          <div
            className={
              'sales-remaining ' + (remaining === 0 ? 'sales-remaining-ok' : 'sales-remaining-pending')
            }
          >
            {courtesyMode
              ? 'Sin pago: venta de cortesía'
              : remaining === 0
              ? 'Pagos completos'
              : remaining > 0
              ? `Falta ${formatMoney(remaining)}`
              : `Sobran ${formatMoney(-remaining)}`}
          </div>

          {cashBlocked && (
            <div className="form-error">
              No hay un turno de caja abierto: no se pueden registrar ventas.
              <button className="btn" onClick={() => void refreshCash()}>
                Reintentar
              </button>
            </div>
          )}

          <button
            className="btn btn-primary sales-confirm-btn"
            disabled={
              confirming ||
              remaining !== 0 ||
              cashShortage ||
              cashReceivedInvalid ||
              cashBlocked ||
              (courtesyMode && !courtesyReason.trim())
            }
            onClick={handleConfirmSale}
          >
            {confirming ? 'Registrando...' : 'Confirmar venta'}
          </button>
        </div>
      )}
    </div>
  );
}