import type { CashSessionReport } from '../../types';
import { formatMoney } from '../../utils/format';
import './CashReport.css';

interface CashReportProps {
  report: CashSessionReport;
}

export default function CashReport({ report }: CashReportProps) {
  return (
    <section className="cash-card cash-report">
      <div className="cash-report-header">
        <div>
          <h2>{report.kind === 'X' ? 'Informe X — parcial' : 'Informe Z — cierre de turno'}</h2>
          <p className="cash-meta">
            Turno #{report.session_id} · Generado {report.generated_at_local}
          </p>
        </div>
        {report.kind === 'X' && <span className="cash-report-badge">No cierra el turno</span>}
      </div>

      <p className="cash-meta">
        Abierto por {report.opened_by} · {report.opened_at_local}
        {report.kind === 'Z' && report.closed_by && report.closed_at_local
          ? ` · Cerrado por ${report.closed_by} · ${report.closed_at_local}`
          : ''}
      </p>

      <div className="cash-report-sections">
        <section>
          <h3>Efectivo</h3>
          <dl>
            <div><dt>Apertura</dt><dd>{formatMoney(report.cash.opening_amount)}</dd></div>
            <div><dt>Ventas en efectivo</dt><dd>{formatMoney(report.cash.cash_sales)}</dd></div>
            <div><dt>Devoluciones en efectivo</dt><dd>{formatMoney(report.cash.cash_refunds)}</dd></div>
            <div><dt>Ingresos manuales</dt><dd>{formatMoney(report.cash.manual_in)}</dd></div>
            <div><dt>Egresos manuales</dt><dd>{formatMoney(report.cash.manual_out)}</dd></div>
            <div className="cash-report-total"><dt>Esperado</dt><dd>{formatMoney(report.cash.expected_amount)}</dd></div>
            {report.kind === 'Z' && report.cash.counted_amount !== null && (
              <div><dt>Contado</dt><dd>{formatMoney(report.cash.counted_amount)}</dd></div>
            )}
            {report.kind === 'Z' && report.cash.difference !== null && (
              <div><dt>Diferencia</dt><dd>{formatMoney(report.cash.difference)}</dd></div>
            )}
          </dl>
        </section>

        <section>
          <h3>Ventas completadas</h3>
          <dl>
            <div><dt>Cantidad</dt><dd>{report.sales.count}</dd></div>
            <div><dt>Bruto</dt><dd>{formatMoney(report.sales.gross_total)}</dd></div>
            <div><dt>Descuentos</dt><dd>{formatMoney(report.sales.discount_total)}</dd></div>
            <div><dt>Neto</dt><dd>{formatMoney(report.sales.net_total)}</dd></div>
            <div><dt>Efectivo</dt><dd>{formatMoney(report.sales.by_method.cash)}</dd></div>
            <div><dt>Tarjeta</dt><dd>{formatMoney(report.sales.by_method.card)}</dd></div>
            <div><dt>Transferencia</dt><dd>{formatMoney(report.sales.by_method.transfer)}</dd></div>
            <div>
              <dt>Cortesías (cantidad / bruto)</dt>
              <dd>{report.sales.courtesy_count} / {formatMoney(report.sales.courtesy_gross)}</dd>
            </div>
          </dl>
          <p className="cash-report-note">Tarjeta y transferencia no entran al efectivo.</p>
        </section>

        <section>
          <h3>Anulaciones y devoluciones</h3>
          <dl>
            <div><dt>Anulaciones</dt><dd>{report.voids.cancelled_count}</dd></div>
            <div><dt>Total anulado</dt><dd>{formatMoney(report.voids.cancelled_total)}</dd></div>
            <div><dt>Devoluciones</dt><dd>{report.voids.refunded_count}</dd></div>
            <div><dt>Total devuelto</dt><dd>{formatMoney(report.voids.refunded_total)}</dd></div>
          </dl>
        </section>
      </div>
    </section>
  );
}
