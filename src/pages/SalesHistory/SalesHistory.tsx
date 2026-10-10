import { useEffect, useState, useCallback, useRef } from 'react';
import type { Sale, SaleDetail, SaleFilterDto, SaleStatus } from '../../types';
import { listSales, getSale } from '../../services/sales.service';
import { printSaleTicket } from '../../services/billing.service';
import { formatMoney } from '../../utils/format';
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
  const [page, setPage] = useState(1);
  const pageSize = 20;

  const [status, setStatus] = useState<string>('');
  const [dateFrom, setDateFrom] = useState('');
  const [dateTo, setDateTo] = useState('');
  const [applied, setApplied] = useState({ status: '', dateFrom: '', dateTo: '' });
  const reqRef = useRef(0);

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [selected, setSelected] = useState<SaleDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  const [printing, setPrinting] = useState(false);
  const [printError, setPrintError] = useState<string | null>(null);
  const [printSuccess, setPrintSuccess] = useState(false);

  const load = useCallback(async () => {
    const id = ++reqRef.current;
    setLoading(true);
    setError(null);
    try {
      const filter: SaleFilterDto = {
        status: applied.status || null,
        date_from: applied.dateFrom || null,
        date_to: applied.dateTo || null,
        page,
        page_size: pageSize,
      };
      const res = await listSales(filter);
      if (id !== reqRef.current) return;
      setSales(res.data);
      setTotal(res.total);
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
    if (dateFrom && dateTo && dateFrom > dateTo) {
      reqRef.current += 1;
      setLoading(false);
      setError('La fecha inicial no puede ser posterior a la final');
      return;
    }
    setError(null);
    setApplied({ status, dateFrom, dateTo });
    setPage(1);
  }

  function clearFilters() {
    setStatus('');
    setDateFrom('');
    setDateTo('');
    setApplied({ status: '', dateFrom: '', dateTo: '' });
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
      await printSaleTicket(selected.id);
      setPrintSuccess(true);
    } catch (e) {
      setPrintError(String(e));
    } finally {
      setPrinting(false);
    }
  }

  const totalPages = Math.max(1, Math.ceil(total / pageSize));

  return (
    <div className="sales-history-page">
      <h1>Historial de ventas</h1>

      {error && <div className="form-error">{error}</div>}

      <form className="sales-history-filters" onSubmit={applyFilters}>
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
        <div className="sales-history-filter-actions">
          <button type="submit" className="btn btn-primary">Filtrar</button>
          <button type="button" className="btn" onClick={clearFilters}>Limpiar</button>
        </div>
      </form>

      <div className="sales-history-card">
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
                      {printing ? 'Imprimiendo…' : '🖨️ Imprimir'}
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
                        <td>#{item.variant_id}</td>
                        <td>{item.quantity}</td>
                        <td>{formatMoney(item.unit_price)}</td>
                        <td>{formatMoney(item.discount)}</td>
                        <td>{formatMoney(item.subtotal)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>

                <div className="sales-history-totals">
                  <div><span>Subtotal</span><span>{formatMoney(selected.subtotal)}</span></div>
                  <div><span>Descuento</span><span>{formatMoney(selected.discount)}</span></div>
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