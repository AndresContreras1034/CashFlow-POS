import { useState, useRef, useCallback } from 'react';
import type { CreateSalePaymentDto, PaymentMethod } from '../../types';
import { findByBarcode, searchVariants } from '../../services/inventory.service';
import { createSale } from '../../services/sales.service';
import './Sales.css';

interface VariantResult {
  id: number;
  product_name: string;
  attributes: Record<string, string>;
  barcode: string | null;
  sku: string;
  price: number;
  stock: number;
}

interface CartLine {
  variant_id: number;
  product_name: string;
  attributes: Record<string, string>;
  unit_price: number;
  quantity: number;
  discount: number;
  stock: number; // referencia visual, la validación real es del backend
}

const formatMoney = (cents: number) =>
  (cents / 100).toLocaleString('es-CO', { style: 'currency', currency: 'COP' });

function attributesLabel(attrs: Record<string, string>) {
  const entries = Object.entries(attrs || {});
  if (entries.length === 0) return '';
  return entries.map(([k, v]) => `${k}: ${v}`).join(', ');
}

export default function Sales() {
  const [scanValue, setScanValue] = useState('');
  const [searchResults, setSearchResults] = useState<VariantResult[]>([]);
  const [cart, setCart] = useState<CartLine[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [lastSaleId, setLastSaleId] = useState<number | null>(null);

  const [payments, setPayments] = useState<CreateSalePaymentDto[]>([
    { method: 'cash', amount: 0 },
  ]);

  const scanInputRef = useRef<HTMLInputElement>(null);

  const subtotal = cart.reduce(
    (sum, line) => sum + line.unit_price * line.quantity - line.discount,
    0
  );
  const paymentsTotal = payments.reduce((sum, p) => sum + (p.amount || 0), 0);
  const remaining = subtotal - paymentsTotal;

  function addToCart(v: VariantResult) {
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
        if (results.length === 1) {
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

  function updateDiscount(variantId: number, discountPesos: string) {
    const cents = Math.round(parseFloat(discountPesos || '0') * 100);
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
    const cents = Math.round(parseFloat(pesos || '0') * 100);
    setPayments((prev) => prev.map((p, i) => (i === index ? { ...p, amount: cents } : p)));
  }

  function addPaymentLine() {
    setPayments((prev) => [...prev, { method: 'card', amount: 0 }]);
  }

  function removePaymentLine(index: number) {
    setPayments((prev) => prev.filter((_, i) => i !== index));
  }

  function fillRemainingOnFirstPayment() {
    setPayments((prev) => {
      if (prev.length === 0) return prev;
      const [first, ...rest] = prev;
      return [{ ...first, amount: subtotal }, ...rest];
    });
  }

  const resetSale = useCallback(() => {
    setCart([]);
    setPayments([{ method: 'cash', amount: 0 }]);
    setLastSaleId(null);
    setError(null);
  }, []);

  async function handleConfirmSale() {
    setError(null);

    if (cart.length === 0) {
      setError('Agrega al menos un producto al carrito');
      return;
    }
    if (remaining !== 0) {
      setError(
        remaining > 0
          ? `Faltan ${formatMoney(remaining)} por cubrir en los pagos`
          : `Los pagos exceden el total en ${formatMoney(-remaining)}`
      );
      return;
    }

    setConfirming(true);
    try {
      const sale = await createSale({
        items: cart.map((l) => ({
          variant_id: l.variant_id,
          quantity: l.quantity,
          discount: l.discount || null,
        })),
        payments: payments.filter((p) => p.amount > 0),
      });
      setLastSaleId(sale.id);
      setCart([]);
      setPayments([{ method: 'cash', amount: 0 }]);
    } catch (e) {
      setError(String(e));
    } finally {
      setConfirming(false);
    }
  }

  return (
    <div className="sales-page">
      <h1>Ventas</h1>

      {error && <div className="form-error">{error}</div>}

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
                      step="0.01"
                      value={line.discount / 100}
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
          <div className="sales-total-row">
            <span>Total a pagar</span>
            <span className="sales-total-value">{formatMoney(subtotal)}</span>
          </div>

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
                step="0.01"
                placeholder="0.00"
                value={p.amount / 100 || ''}
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

          <div
            className={
              'sales-remaining ' + (remaining === 0 ? 'sales-remaining-ok' : 'sales-remaining-pending')
            }
          >
            {remaining === 0
              ? 'Pagos completos'
              : remaining > 0
              ? `Falta ${formatMoney(remaining)}`
              : `Sobran ${formatMoney(-remaining)}`}
          </div>

          <button
            className="btn btn-primary sales-confirm-btn"
            disabled={confirming || remaining !== 0}
            onClick={handleConfirmSale}
          >
            {confirming ? 'Registrando...' : 'Confirmar venta'}
          </button>
        </div>
      )}
    </div>
  );
}