import { useState, useRef, useCallback, useEffect } from 'react';
import type { CreateSalePaymentDto, PaymentMethod, VariantWithProduct } from '../../types';
import { findByBarcode, searchVariants, getVariant } from '../../services/inventory.service';
import { createSale } from '../../services/sales.service';
import { printSaleTicket } from '../../services/billing.service';
import { getCurrentCashSession } from '../../services/cash.service';
import { getCustomer } from '../../services/customers.service';
import { CustomerPicker, type CustomerRef } from '../../components/customers/CustomerPicker';
import { formatMoney, parseMoneyInput, minorToInput, moneyStep } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import {
  computeCheckout,
  retryFingerprint,
  type CartLine,
} from '../../utils/cart';
import {
  addHeld,
  heldTotal,
  pruneHeld,
  removeHeld,
  type HeldSale,
} from '../../utils/heldSales';
import './Sales.css';

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
  const [cashSessionId, setCashSessionId] = useState<number | null>(null);
  const [heldSales, setHeldSales] = useState<HeldSale[]>([]);
  const [customer, setCustomer] = useState<CustomerRef | null>(null);
  const [heldReviewPending, setHeldReviewPending] = useState(false);
  const [heldReviewAcknowledged, setHeldReviewAcknowledged] = useState(false);
  const [heldReviewChanges, setHeldReviewChanges] = useState<string[]>([]);
  const [heldLineIssues, setHeldLineIssues] = useState<Record<number, string>>({});
  const [lastSaleId, setLastSaleId] = useState<number | null>(null);
  const [isCourtesy, setIsCourtesy] = useState(false);
  const [courtesyReason, setCourtesyReason] = useState('');
  const [cashReceivedInput, setCashReceivedInput] = useState('');
  const [autoPrint, setAutoPrint] = useState(() => {
    try {
      return localStorage.getItem('pos.autoPrint') === 'true';
    } catch {
      return false;
    }
  });
  const [printError, setPrintError] = useState<string | null>(null);
  const [clearPending, setClearPending] = useState(false);

  const [payments, setPayments] = useState<CreateSalePaymentDto[]>([
    { method: 'cash', amount: 0 },
  ]);

  const scanInputRef = useRef<HTMLInputElement>(null);
  const cashInputRef = useRef<HTMLInputElement>(null);
  const confirmButtonRef = useRef<HTMLButtonElement>(null);
  const clearTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const attemptRef = useRef<{ fingerprint: string; key: string } | null>(null);
  const submittingRef = useRef(false);
  const resumingHeldRef = useRef(false);

  const refreshCash = useCallback(async () => {
    try {
      const current = await getCurrentCashSession();
      const currentSessionId = current?.id ?? null;
      setCashOpen(current !== null);
      setCashSessionId(currentSessionId);
      try {
        const pruned = pruneHeld(window.localStorage, currentSessionId);
        setHeldSales(pruned);
      } catch (storageError) {
        setNotice(String(storageError));
      }
    } catch {
      setCashOpen(null);
      setCashSessionId(null);
    }
  }, []);

  useEffect(() => {
    void refreshCash();
  }, [refreshCash]);

  const cashReceived =
    cashReceivedInput.trim() === '' ? null : parseMoneyInput(cashReceivedInput);
  const cashReceivedInvalid = cashReceivedInput.trim() !== '' && cashReceived === null;
  const {
    subtotal,
    courtesyMode,
    total,
    remaining,
    cashNet,
    cashShortage,
    cashChange,
    oversold,
    hasInvalidDiscount,
  } = computeCheckout({ lines: cart, payments, isCourtesyFlag: isCourtesy, cashReceived });
  const cashBlocked = cashOpen === false;
  const heldIssuesRemain = cart.some((line) => heldLineIssues[line.variant_id] !== undefined);

  useEffect(() => {
    if (cashNet === 0 || courtesyMode) setCashReceivedInput('');
  }, [cashNet, courtesyMode]);

  function addToCart(v: VariantWithProduct) {
    if (resumingHeldRef.current) {
      setNotice('Espera a que termine la validación de la venta en espera.');
      return;
    }
    setNotice(null);
    setHeldLineIssues((prev) => {
      if (prev[v.id] === undefined) return prev;
      const next = { ...prev };
      delete next[v.id];
      return next;
    });
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
    if (heldLineIssues[variantId] !== undefined) setError(null);
    setCart((prev) => prev.filter((l) => l.variant_id !== variantId));
    setHeldLineIssues((prev) => {
      if (prev[variantId] === undefined) return prev;
      const next = { ...prev };
      delete next[variantId];
      return next;
    });
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

  const clearCurrentCart = useCallback(() => {
    attemptRef.current = null;
    setCart([]);
    setCustomer(null);
    setPayments([{ method: 'cash', amount: 0 }]);
    setCashReceivedInput('');
    setIsCourtesy(false);
    setCourtesyReason('');
    setHeldReviewPending(false);
    setHeldReviewAcknowledged(false);
    setHeldReviewChanges([]);
    setHeldLineIssues({});
  }, []);

  function saveCurrentCart() {
    if (cashSessionId === null || cashOpen !== true) {
      setNotice('Se requiere un turno de caja abierto para guardar en espera.');
      return;
    }
    try {
      const result = addHeld(window.localStorage, {
        session_id: cashSessionId,
        is_courtesy: isCourtesy,
        courtesy_reason: courtesyReason,
        customer,
        lines: cart,
      });
      if (!result.ok || !result.list) {
        setNotice(result.error ?? 'No se pudo guardar la venta en espera.');
        return;
      }
      setHeldSales(result.list);
      clearCurrentCart();
      setNotice('Carrito guardado en espera. No se registró la venta.');
      scanInputRef.current?.focus();
    } catch (storageError) {
      setNotice(`No se pudo guardar la venta en espera: ${String(storageError)}`);
    }
  }

  function discardHeldSale(id: string) {
    try {
      const result = removeHeld(window.localStorage, id);
      if (!result.ok || !result.list) {
        setNotice(result.error ?? 'No se pudo descartar la venta en espera.');
        return;
      }
      setHeldSales(result.list);
      setNotice('Venta en espera descartada.');
    } catch (storageError) {
      setNotice(`No se pudo descartar la venta en espera: ${String(storageError)}`);
    }
  }

  async function resumeHeldSale(entry: HeldSale) {
    if (resumingHeldRef.current) return;
    if (cart.length > 0) {
      setNotice('Vacía el carrito antes de retomar una venta en espera');
      return;
    }

    resumingHeldRef.current = true;
    try {
      const refreshedLines: CartLine[] = [];
      const changes: string[] = [];
      const issues: Record<number, string> = {};
      let refreshedCustomer: CustomerRef | null = entry.customer ?? null;
      if (entry.customer) {
        try {
          const activeCustomer = await getCustomer(entry.customer.id);
          if (!activeCustomer.is_active) {
            throw new Error('El cliente guardado está desactivado');
          }
          refreshedCustomer = {
            id: activeCustomer.id,
            name: activeCustomer.name,
            document:
              activeCustomer.document_type && activeCustomer.document_number
                ? `${activeCustomer.document_type} ${activeCustomer.document_number}`
                : null,
          };
          if (refreshedCustomer.name !== entry.customer.name) {
            changes.push(`Cliente: ${entry.customer.name} → ${refreshedCustomer.name}`);
          }
        } catch (customerError) {
          refreshedCustomer = null;
          changes.push(`Cliente sin asignar: no se pudo revalidar (${String(customerError)}).`);
        }
      }
      for (const line of entry.lines) {
        try {
          const fresh = await getVariant(line.variant_id);
          refreshedLines.push({
            ...line,
            attributes: fresh.attributes,
            unit_price: fresh.price,
            stock: fresh.stock,
          });
          if (fresh.price !== line.unit_price) {
            changes.push(
              `${line.product_name}: ${formatMoney(line.unit_price)} -> ${formatMoney(fresh.price)}`
            );
          }
          if (fresh.stock !== line.stock) {
            changes.push(`${line.product_name}: stock ${line.stock} -> ${fresh.stock}`);
          }
          if (!fresh.is_active) {
            issues[line.variant_id] =
              `${line.product_name}: la variante está inactiva. Elimínala o revísala antes de confirmar.`;
          }
        } catch (lookupError) {
          refreshedLines.push(line);
          issues[line.variant_id] =
            `${line.product_name}: no se pudo validar la variante (${String(lookupError)}). Elimínala o revísala antes de confirmar.`;
        }
      }

      setCart(refreshedLines);
      setCustomer(refreshedCustomer);
      setPayments([{ method: 'cash', amount: 0 }]);
      setCashReceivedInput('');
      setIsCourtesy(entry.is_courtesy);
      setCourtesyReason(entry.courtesy_reason);
      setHeldLineIssues(issues);
      setHeldReviewPending(true);
      setHeldReviewAcknowledged(false);
      setHeldReviewChanges(
        changes.length > 0
          ? changes
          : ['Precios sin cambios; revisa disponibilidad, cantidades y descuentos antes de confirmar.']
      );
      setError(
        Object.keys(issues).length > 0
          ? 'Hay productos que requieren revisión antes de confirmar.'
          : null
      );
      setNotice('Venta en espera cargada. Revisa los cambios y vuelve a armar los pagos.');

      try {
        const result = removeHeld(window.localStorage, entry.id);
        if (result.ok && result.list) {
          setHeldSales(result.list);
        } else {
          setNotice(
            `${result.error ?? 'No se pudo quitar la venta en espera del almacenamiento.'} El carrito ya está cargado.`
          );
        }
      } catch (storageError) {
        setNotice(
          `No se pudo quitar la venta en espera del almacenamiento: ${String(storageError)}. El carrito ya está cargado.`
        );
      }
    } finally {
      resumingHeldRef.current = false;
    }
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
    clearCurrentCart();
    setLastSaleId(null);
    setError(null);
    setPrintError(null);
    setClearPending(false);
    if (clearTimerRef.current) clearTimeout(clearTimerRef.current);
    void refreshCash();
  }, [refreshCash, clearCurrentCart]);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === 'F2') {
        event.preventDefault();
        scanInputRef.current?.focus();
      } else if (event.key === 'F4' && cashNet > 0 && !courtesyMode) {
        event.preventDefault();
        cashInputRef.current?.focus();
      } else if (event.key === 'F9' && confirmButtonRef.current && !confirmButtonRef.current.disabled) {
        event.preventDefault();
        confirmButtonRef.current.click();
      } else if (event.key === 'Escape') {
        setSearchResults([]);
        setNotice(null);
      }
    }
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [cashNet, courtesyMode]);

  useEffect(
    () => () => {
      if (clearTimerRef.current) clearTimeout(clearTimerRef.current);
    },
    []
  );

  function requestClearCart() {
    if (!clearPending) {
      setClearPending(true);
      clearTimerRef.current = setTimeout(() => setClearPending(false), 3000);
      return;
    }
    clearCurrentCart();
    setNotice(null);
    setClearPending(false);
    if (clearTimerRef.current) clearTimeout(clearTimerRef.current);
  }

  async function handleConfirmSale() {
    if (submittingRef.current) return;
    setError(null);

    if (hasInvalidDiscount) {
      setError('El descuento de una línea no puede superar su importe');
      return;
    }
    if (heldReviewPending && !heldReviewAcknowledged) {
      setError('Revisa la venta retomada antes de confirmar');
      return;
    }
    if (heldIssuesRemain) {
      setError('Revisa o elimina los productos marcados antes de confirmar');
      return;
    }
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
        ...(cashReceived === null || courtesyMode ? {} : { cash_received: cashReceived }),
        courtesy_reason: courtesyMode ? trimmedCourtesyReason : null,
        customer_id: customer?.id ?? null,
        ...actorField('created_by'),
      };
      const fingerprint = retryFingerprint(payload);
      if (attemptRef.current?.fingerprint !== fingerprint) {
        attemptRef.current = { fingerprint, key: crypto.randomUUID() };
      }

      const attempt = attemptRef.current;
      const sale = await createSale({ ...payload, idempotency_key: attempt.key });
      attemptRef.current = null;
      setLastSaleId(sale.id);
      setPrintError(null);
      clearCurrentCart();
      if (autoPrint) {
        try {
          await printSaleTicket(sale.id);
        } catch (e) {
          setPrintError(`Venta #${sale.id} registrada, pero no se pudo imprimir: ${String(e)}`);
        }
      }
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
      <p className="sales-shortcuts">Atajos: F2 escáner · F4 efectivo · F9 confirmar · Esc cerrar aviso</p>

      {error && <div className="form-error">{error}</div>}
      {printError && !lastSaleId && <div className="form-error">{printError}</div>}
      {notice && <div className="sales-meta">{notice}</div>}
      {heldReviewPending && (
        <div className="sales-held-review">
          <strong>Resumen de revalidación</strong>
          <ul>
            {heldReviewChanges.map((change) => <li key={change}>{change}</li>)}
          </ul>
          <label>
            <input
              type="checkbox"
              checked={heldReviewAcknowledged}
              onChange={(event) => {
                setHeldReviewAcknowledged(event.target.checked);
                if (event.target.checked) setError(null);
              }}
            />
            Revisé los cambios, productos, cantidades y descuentos.
          </label>
        </div>
      )}

      {lastSaleId && (
        <div className="sales-success">
          Venta #{lastSaleId} registrada correctamente.
          {printError && <div className="sales-print-error">{printError}</div>}
          <button
            className="btn"
            onClick={async () => {
              try {
                setPrintError(null);
                await printSaleTicket(lastSaleId);
              } catch (e) {
                setPrintError(`Venta #${lastSaleId} registrada, pero no se pudo imprimir: ${String(e)}`);
              }
            }}
          >
            Imprimir ticket
          </button>
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

      <div className="sales-card sales-held-section">
        <h2>Ventas en espera ({heldSales.length}/5) - no están registradas ni reservan stock</h2>
        {heldSales.length === 0 ? (
          <p className="sales-meta">No hay ventas en espera para este turno.</p>
        ) : (
          <ul className="sales-held-list">
            {heldSales.map((entry) => (
              <li className="sales-held-item" key={entry.id}>
                <div className="sales-held-info">
                  <strong>
                    {new Date(entry.saved_at).toLocaleTimeString('es-CO', {
                      hour: '2-digit',
                      minute: '2-digit',
                    })}
                  </strong>
                  <span>{entry.lines.length} líneas</span>
                  <span>{formatMoney(heldTotal(entry))}</span>
                </div>
                <div className="sales-held-actions">
                  <button
                    className="btn"
                    type="button"
                    onClick={() => void resumeHeldSale(entry)}
                  >
                    Retomar
                  </button>
                  <button
                    className="btn btn-danger"
                    type="button"
                    onClick={() => discardHeldSale(entry.id)}
                  >
                    Descartar
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="sales-card">
        <div className="sales-cart-heading">
          <h2>Carrito</h2>
          {cart.length > 0 && (
            <div className="sales-held-actions">
              <button
                className="btn"
                type="button"
                onClick={saveCurrentCart}
                disabled={cashSessionId === null || cashOpen !== true}
                title={
                  cashSessionId === null || cashOpen !== true
                    ? 'Abre un turno de caja para guardar en espera'
                    : undefined
                }
              >
                Guardar en espera
              </button>
              <button className="btn" type="button" onClick={requestClearCart}>
                {clearPending ? '¿Seguro?' : 'Vaciar carrito'}
              </button>
            </div>
          )}
        </div>
        <div className="form-field">
          <label>Cliente (opcional)</label>
          <CustomerPicker value={customer} onChange={setCustomer} disabled={confirming} />
        </div>
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
                  <td className={line.quantity > line.stock ? 'sales-stock-warn' : undefined}>
                    {line.product_name}
                    {attributesLabel(line.attributes) && (
                      <div className="sales-line-attrs">{attributesLabel(line.attributes)}</div>
                    )}
                    <div className="sales-line-attrs">Stock: {line.stock}</div>
                    {heldLineIssues[line.variant_id] && (
                      <div className="sales-held-line-issue">
                        {heldLineIssues[line.variant_id]}
                        <button
                          type="button"
                          className="btn btn-danger"
                          onClick={() => removeLine(line.variant_id)}
                        >
                          Eliminar producto
                        </button>
                      </div>
                    )}
                  </td>
                  <td>{formatMoney(line.unit_price)}</td>
                  <td>
                    <div className="sales-quantity-controls">
                      <button type="button" className="btn" aria-label="Disminuir cantidad"
                        onClick={() => updateQuantity(line.variant_id, line.quantity - 1)}>−</button>
                      <input
                        className="form-input sales-qty-input"
                        type="number"
                        min={1}
                        value={line.quantity}
                        onChange={(e) =>
                          updateQuantity(line.variant_id, parseInt(e.target.value || '1', 10))
                        }
                      />
                      <button type="button" className="btn" aria-label="Aumentar cantidad"
                        onClick={() => updateQuantity(line.variant_id, line.quantity + 1)}>+</button>
                    </div>
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
          <label className="sales-auto-print">
            <input
              type="checkbox"
              checked={autoPrint}
              onChange={(event) => {
                setAutoPrint(event.target.checked);
                try {
                  localStorage.setItem('pos.autoPrint', String(event.target.checked));
                } catch (e) {
                  setPrintError(`No se pudo guardar la preferencia de impresión: ${String(e)}`);
                }
              }}
            />
            Imprimir al confirmar
          </label>
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
                  ref={cashInputRef}
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
          {oversold && (
            <div className="sales-stock-warning">
              Hay productos que superan el stock mostrado; el servidor confirmará.
            </div>
          )}
          {hasInvalidDiscount && (
            <div className="form-error">
              El descuento de una línea no puede superar su importe
            </div>
          )}

          <button
            ref={confirmButtonRef}
            className="btn btn-primary sales-confirm-btn"
            disabled={
              confirming ||
              remaining !== 0 ||
              cashShortage ||
              cashReceivedInvalid ||
              hasInvalidDiscount ||
              heldIssuesRemain ||
              (heldReviewPending && !heldReviewAcknowledged) ||
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