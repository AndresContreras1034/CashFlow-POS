import type { PaymentMethodStat } from '../../types/reports';
import { formatMoney } from '../../utils/format';
import { formatBps } from '../../utils/reportFormat';

const METHOD_LABELS: Record<PaymentMethodStat['method'], string> = {
  cash: 'Efectivo',
  card: 'Tarjeta',
  transfer: 'Transferencia',
};

interface PaymentMethodChartProps {
  data: PaymentMethodStat[];
  currency: string;
  decimals: number;
}

export function PaymentMethodChart({
  data,
  currency,
  decimals,
}: PaymentMethodChartProps) {
  if (data.length === 0) {
    return <p className="reports-empty">Sin pagos en este rango.</p>;
  }

  return (
    <div className="reports-payment-list">
      {data.map(item => (
        <div className="reports-payment-row" key={item.method}>
          <div className="reports-payment-heading">
            <span>{METHOD_LABELS[item.method]}</span>
            <span>{formatBps(item.share_bps)}</span>
          </div>
          <div
            className="reports-payment-track"
            role="img"
            aria-label={`${METHOD_LABELS[item.method]}: ${formatBps(item.share_bps)}`}
          >
            <span style={{ width: `${Math.min(100, Math.max(0, item.share_bps / 100))}%` }} />
          </div>
          <div className="reports-payment-meta">
            <strong>{formatMoney(item.amount, currency, decimals)}</strong>
            <span>{item.tickets} tickets</span>
          </div>
        </div>
      ))}
    </div>
  );
}
