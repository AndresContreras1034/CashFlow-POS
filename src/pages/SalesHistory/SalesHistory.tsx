import { useEffect, useState, useCallback, useRef } from 'react';
import type { Sale, SaleDetail, SaleFilterDto, SaleStatus, VoidSaleDto } from '../../types';
import { listSales, getSale, voidSale } from '../../services/sales.service';
import { printSaleTicket } from '../../services/billing.service';
import { formatMoney } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import { parseSaleNumber, presetRange, type SaleDatePreset } from '../../utils/salesFilters';
import './SalesHistory.css';

const statusLabel: Record<SaleStatus, string> = {
  completed: 'Completada',
  cancelled: 'Cancelada',
  refunded: 'Reembolsada',
};

const paymentMethodLabel: Record<string, string> = {
  cash: 'Efectivo',
  card: 'Tarjeta',
  transfer: 'Transferencia',
};

export default function SalesHistory() {
  const [sales, setSales] = useState<Sale[]>([]);
  const [total, setTotal] = useState(0);
  const [totalAmount, setTotalAmount] = useState(0);
  const [page, setPage] = useState(1);
  const pageSize = 20;

  const [status, setStatus] = useState<string>('');
  const [dateFrom, setDateFrom] = useState('');
  const [dateTo, setDateTo] = useState('');
  const [saleNumber, setSaleNumber] = useState('');
  const [paymentMethod, setPaymentMethod] = useState('');
  const [cashSessionId, setCashSessionId] = useState('');
  const [applied, setApplied] = useState({
    status: '', dateFrom: '', dateTo: '', saleId: null as number | null,
    paymentMethod: '', cashSessionId: null as number | null,
  });
  const reqRef = useRef(0);

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [selected, setSelected] = useState<SaleDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  const [printing, setPrinting] = useState(false);
  const [printError, setPrintError] = useState<string | null>(null);
  const [printSuccess, setPrintSuccess] = useState(false);
  const [voidFormOpen, setVoidFormOpen] = useState(false);
  const [voidTarget, setVoidTarget] = useState<VoidSaleDto['target_status']>('cancelled');
  const [voidReason, setVoidReason] = useState('');
  const [voiding, setVoiding] = useState(false);

  const load = useCallback(async () => {
    const id = ++reqRef.current;
    setLoading(true);
    setError(null);
    try {
      const filter: SaleFilterDto = {
        status: applied.status || null,
        date_from: applied.dateFrom || null,
        date_to: applied.dateTo || null,
        sale_id: applied.saleId,
        payment_method: applied.paymentMethod || null,
        cash_session_id: applied.cashSessionId,
        page,
        page_size: pageSize,
      };
      const res = await listSales(filter);
      if (id !== reqRef.current) return;
      setSales(res.data);
      setTotal(res.total);
      setTotalAmount(res.total_amount);
    } catch (e) {
      if (id !== reqRef.current) return;
      setError(String(e));
    } finally {
      if (id === reqRef.current) setLoading(false);
    }
  }, [applied, page]);

  useEffect(() => {
    load();
  }, [load]);

  function applyFilters(e: React.FormEvent) {
    e.preventDefault();
    const saleId = saleNumber.trim() ? parseSaleNumber(saleNumber) : null;
    const sessionId = cashSessionId.trim() ? Number(cashSessionId) : null;
    if (saleNumber.trim() && saleId === null) {
      setError('Ingresa un número de venta válido');
      return;
    }
    if (cashSessionId.trim() && (!Number.isSafeInteger(sessionId) || sessionId! <= 0)) {
      setError('Ingresa un número de turno válido');
      return;
    }
    if (dateFrom && dateTo && dateFrom > dateTo) {
      reqRef.current += 1;
      setLoading(false);
      setError('La fecha inicial no puede ser posterior a la final');
      return;
    }
    setError(null);
    setApplied({ status, dateFrom, dateTo, saleId, paymentMethod, cashSessionId: sessionId });
    setPage(1);
  }

  function clearFilters() {
    setStatus('');
    setDateFrom('');
    setDateTo('');
    setSaleNumber('');
    setPaymentMethod('');
    setCashSessionId('');
    setApplied({ status: '', dateFrom: '', dateTo: '', saleId: null, paymentMethod: '', cashSessionId: null });
    setPage(1);
  }

  function applyPreset(preset: SaleDatePreset) {
    const range = presetRange(preset, new Date());
    setDateFrom(range.from);
    setDateTo(range.to);
    setApplied((current) => ({ ...current, dateFrom: range.from, dateTo: range.to }));
    setPage(1);
  }

  async function openDetail(id: number) {
    setDetailLoading(true);
    setError(null);
    setPrintError(null);
    setPrintSuccess(false);
    try {
      const detail = await getSale(id);
      setSelected(detail);
    } catch (e) {
      setError(String(e));
    } finally {
      setDetailLoading(false);
    }
  }

  function closeDetail() {
    setSelected(null);
    setPrintError(null);
    setPrintSuccess(false);
  }

  async function handlePrint() {
    if (!selected) return;
    setPrinting(true);
    setPrintError(null);
    setPrintSuccess(false);
    try {
      await printSaleTicket(selected.id, true);
      setPrintSuccess(true);
    } catch (e) {
      setPrintError(String(e));
    } finally {
      setPrinting(false);
    }
  }

  async function handleVoid() {
    if (!selected || voiding || !voidReason.trim()) return;
    setVoiding(true);
    setError(null);
    try {
      const updated = await voidSale(selected.id, {
        target_status: voidTarget,
        reason: voidReason.trim(),
        ...actorField('performed_by'),
      });
      setSelected(updated);
      setVoidFormOpen(false);
      setVoidReason('');
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setVoiding(false);
    }
  }

  const cashToReturn = selected?.payments
    .filter((payment) => payment.method === 'cash')
    .reduce((sum, payment) => sum + payment.amount, 0) ?? 0;
  const totalPages = Math.max(1, Math.ceil(total / pageSize));

  return (
    <div className="sales-history-page">
      <h1>Historial de ventas</h1>

      {error && <div className="form-error">{error}</div>}

      <form className="sales-history-filters" onSubmit={applyFilters}>
        <div className="sales-history-presets">
          <button type="button" className="btn" onClick={() => applyPreset('today')}>Hoy</button>
          <button type="button" className="btn" onClick={() => applyPreset('yesterday')}>Ayer</button>
          <button type="button" className="btn" onClick={() => applyPreset('week')}>7 días</button>
        </div>
        <div className="form-field">
          <label>Estado</label>
          <select className="form-input" value={status} onChange={(e) => setStatus(e.target.value)}>
            <option value="">Todos</option>
            <option value="completed">Completada</option>
            <option value="cancelled">Cancelada</option>
            <option value="refunded">Reembolsada</option>
          </select>
        </div>
        <div className="form-field">
          <label>Desde</label>
          <input
            className="form-input"
            type="date"
            value={dateFrom}
            onChange={(e) => setDateFrom(e.target.value)}
          />
        </div>
        <div className="form-field">
          <label>Hasta</label>
          <input
            className="form-input"
            type="date"
            value={dateTo}
            onChange={(e) => setDateTo(e.target.value)}
          />
        </div>
        <div className="form-field">
          <label>N° venta</label>
          <input className="form-input" value={saleNumber} onChange={(e) => setSaleNumber(e.target.value)} placeholder="#12 o T-000012" />
        </div>
        <div className="form-field">
          <label>Método de pago</label>
          <select className="form-input" value={paymentMethod} onChange={(e) => setPaymentMethod(e.target.value)}>
            <option value="">Todos</option>
            <option value="cash">Efectivo</option>
            <option value="card">Tarjeta</option>
            <option value="transfer">Transferencia</option>
          </select>
        </div>
        <div className="form-field">
          <label>Turno #</label>
          <input className="form-input" type="number" min={1} value={cashSessionId} onChange={(e) => setCashSessionId(e.target.value)} />
        </div>
        <div className="sales-history-filter-actions">
          <button type="submit" className="btn btn-primary">Filtrar</button>
          <button type="button" className="btn" onClick={clearFilters}>Limpiar</button>
        </div>
      </form>

      <div className="sales-history-card">
        <p className="sales-history-meta">
          {total} ventas · Total completadas: {formatMoney(totalAmount)}
        </p>
        {loading ? (
          <p className="sales-history-meta">Cargando...</p>
        ) : sales.length === 0 ? (
          <p className="sales-history-meta">No hay ventas para estos filtros.</p>
        ) : (
          <>
            <table className="sales-history-table">
              <thead>
                <tr>
                  <th>#</th>
                  <th>Fecha</th>
                  <th>Estado</th>
                  <th>Total</th>
                  <th>Vendedor</th>
                </tr>
              </thead>
              <tbody>
                {sales.map((s) => (
                  <tr key={s.id} onClick={() => openDetail(s.id)} className="sales-history-row">
                    <td>#{s.id}</td>
                    <td>{new Date(s.created_at).toLocaleString('es-CO')}</td>
                    <td>
                      <span className={`sales-history-status status-${s.status}`}>
                        {statusLabel[s.status]}
                      </span>
                    </td>
                    <td>
                      {formatMoney(s.total)}
                      {s.courtesy_reason && (
                        <span className="sales-history-courtesy">Cortesía</span>
                      )}
                    </td>
                    <td>{s.created_by}</td>
                  </tr>
                ))}
              </tbody>
            </table>

            <div className="sales-history-pagination">
              <button
                className="btn"
                disabled={page <= 1}
                onClick={() => setPage((p) => Math.max(1, p - 1))}
              >
                Anterior
              </button>
              <span>Página {page} de {totalPages}</span>
              <button
                className="btn"
                disabled={page >= totalPages}
                onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
              >
                Siguiente
              </button>
            </div>
          </>
        )}
      </div>

      {(selected || detailLoading) && (
        <div className="sales-history-modal-backdrop" onClick={closeDetail}>
          <div className="sales-history-modal" onClick={(e) => e.stopPropagation()}>
            {detailLoading || !selected ? (
              <p>Cargando detalle...</p>
            ) : (
              <>
                <div className="sales-history-modal-header">
                  <h2>Venta #{selected.id}</h2>
                  <div className="sales-history-modal-actions">
                    <button
                      className="btn btn-primary"
                      onClick={handlePrint}
                      disabled={printing}
                    >
                      {printing ? 'Imprimiendo…' : 'Reimprimir (COPIA)'}
                    </button>
                    <button className="btn" onClick={closeDetail}>✕</button>
                  </div>
                </div>

                {printError && <div className="form-error">{printError}</div>}
                {printSuccess && (
                  <div className="sales-history-print-success">Ticket enviado a la impresora.</div>
                )}

                <p className="sales-history-meta">
                  {new Date(selected.created_at).toLocaleString('es-CO')} — {statusLabel[selected.status]}
                  {' · '}Vendedor: {selected.created_by}
                </p>
                {selected.status === 'completed' ? (
                  <div className="sales-history-void-actions">
                    {!voidFormOpen ? (
                      <button
                        className="btn btn-danger"
                        type="button"
                        onClick={() => {
                          setVoidTarget('cancelled');
                          setVoidReason('');
                          setVoidFormOpen(true);
                        }}
                      >
                        Anular / devolver
                      </button>
                    ) : (
                      <div className="sales-history-void-form">
                        <div className="form-field">
                          <label htmlFor="sale-void-target">Operación</label>
                          <select
                            id="sale-void-target"
                            className="form-input"
                            value={voidTarget}
                            onChange={(event) =>
                              setVoidTarget(event.target.value as VoidSaleDto['target_status'])
                            }
                            disabled={voiding}
                          >
                            <option value="cancelled">Anulación</option>
                            <option value="refunded">Devolución</option>
                          </select>
                        </div>
                        <div className="form-field">
                          <label htmlFor="sale-void-reason">Motivo</label>
                          <textarea
                            id="sale-void-reason"
                            className="form-input"
                            maxLength={200}
                            value={voidReason}
                            onChange={(event) => setVoidReason(event.target.value)}
                            disabled={voiding}
                          />
                        </div>
                        <p className="sales-history-void-notice">
                          {cashToReturn > 0
                            ? `Se repondrá el stock y se devolverán ${formatMoney(cashToReturn)} en efectivo del turno actual.`
                            : 'Se repondrá el stock. Esta venta no tiene efectivo que devolver.'}
                        </p>
                        <div className="sales-history-modal-actions">
                          <button
                            className="btn btn-danger"
                            type="button"
                            onClick={() => void handleVoid()}
                            disabled={voiding || !voidReason.trim()}
                          >
                            {voiding ? 'Procesando...' : 'Confirmar'}
                          </button>
                          <button
                            className="btn"
                            type="button"
                            onClick={() => setVoidFormOpen(false)}
                            disabled={voiding}
                          >
                            Cancelar
                          </button>
                        </div>
                      </div>
                    )}
                  </div>
                ) : (
                  <div className="sales-history-void-details">
                    <strong>{statusLabel[selected.status]}</strong>
                    <span>Motivo: {selected.void_reason ?? '—'}</span>
                    <span>
                      Fecha: {selected.voided_at
                        ? new Date(selected.voided_at).toLocaleString('es-CO')
                        : '—'}
                    </span>
                    <span>Operador: {selected.voided_by ?? '—'}</span>
                  </div>
                )}
                {selected.courtesy_reason && (
                  <div className="sales-history-courtesy-reason">
                    <strong>Venta de cortesía</strong>
                    <p>Motivo: {selected.courtesy_reason}</p>
                  </div>
                )}

                <table className="sales-history-table">
                  <thead>
                    <tr>
                      <th>Variante</th>
                      <th>Cant.</th>
                      <th>Precio unit.</th>
                      <th>Desc.</th>
                      <th>Subtotal</th>
                    </tr>
                  </thead>
                  <tbody>
                    {selected.items.map((item) => (
                      <tr key={item.id}>
                        <td>
                          {selected.item_names.find((name) => name.item_id === item.id)?.product_name ??
                            `#${item.variant_id}`}
                          {(() => {
                            const name = selected.item_names.find((entry) => entry.item_id === item.id);
                            const attrs = name?.attributes ?? {};
                            const text = Object.entries(attrs)
                              .map(([key, value]) => `${key}: ${String(value)}`)
                              .join(', ');
                            return text ? <div className="sales-history-meta">{text}</div> : null;
                          })()}
                        </td>
                        <td>{item.quantity}</td>
                        <td>{formatMoney(item.unit_price)}</td>
                        <td>{formatMoney(item.discount)}</td>
                        <td>{formatMoney(item.subtotal)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>

                <div className="sales-history-totals">
                  <div><span>Subtotal (bruto)</span><span>{formatMoney(selected.subtotal)}</span></div>
                  <div><span>Descuento total</span><span>{formatMoney(selected.discount)}</span></div>
                  {selected.cash_received !== null && (
                    <>
                      <div>
                        <span>Efectivo recibido</span>
                        <span>{formatMoney(selected.cash_received)}</span>
                      </div>
                      <div>
                        <span>Cambio</span>
                        <span>
                          {selected.change_given === null
                            ? '—'
                            : formatMoney(selected.change_given)}
                        </span>
                      </div>
                    </>
                  )}
                  <div className="sales-history-total-final">
                    <span>Total</span><span>{formatMoney(selected.total)}</span>
                  </div>
                </div>

                <h3>Pagos</h3>
                <ul className="sales-history-payments">
                  {selected.payments.length === 0 ? (
                    <li>Sin pagos</li>
                  ) : (
                    selected.payments.map((p) => (
                      <li key={p.id}>
                        {paymentMethodLabel[p.method] ?? p.method} — {formatMoney(p.amount)}
                      </li>
                    ))
                  )}
                </ul>

                {selected.notes && (
                  <p className="sales-history-notes">Notas: {selected.notes}</p>
                )}
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}