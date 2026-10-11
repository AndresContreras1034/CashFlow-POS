import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';
import type { TimeSeriesPoint } from '../../types/reports';
import { formatMoney } from '../../utils/format';
import './SalesChart.css';

interface SalesChartProps {
  data: TimeSeriesPoint[];
  previousData: TimeSeriesPoint[];
  comparePrevious: boolean;
  currency: string;
  decimals: number;
}

const LABELS: Record<string, string> = {
  gross_sales: 'Ventas',
  gross_profit: 'Utilidad',
  previous_sales: 'Ventas período anterior',
  previous_profit: 'Utilidad período anterior',
};

export function SalesChart({
  data,
  previousData,
  comparePrevious,
  currency,
  decimals,
}: SalesChartProps) {
  if (data.length === 0) {
    return <p className="reports-empty">Sin datos de ventas para graficar.</p>;
  }

  const chartData = data.map((point, index) => ({
    bucket: point.bucket,
    gross_sales: point.gross_sales,
    gross_profit: point.gross_profit,
    previous_sales: comparePrevious ? previousData[index]?.gross_sales ?? null : null,
    previous_profit: comparePrevious ? previousData[index]?.gross_profit ?? null : null,
  }));

  return (
    <div className="reports-chart">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={chartData} margin={{ top: 8, right: 14, bottom: 2, left: 4 }}>
          <CartesianGrid stroke="var(--border-subtle)" strokeDasharray="3 3" />
          <XAxis
            dataKey="bucket"
            tickFormatter={value => String(value).slice(5)}
            tickLine={false}
            axisLine={{ stroke: 'var(--border)' }}
            minTickGap={24}
          />
          <YAxis
            tickFormatter={value =>
              new Intl.NumberFormat('es-CO', {
                notation: 'compact',
                maximumFractionDigits: 1,
              }).format(Number(value) / 100)
            }
            tickLine={false}
            axisLine={false}
            width={48}
          />
          <Tooltip
            labelFormatter={label => String(label)}
            formatter={(value, name) => [
              formatMoney(Number(value ?? 0), currency, decimals),
              LABELS[String(name)] ?? String(name),
            ]}
            contentStyle={{
              borderColor: 'var(--border)',
              borderRadius: 'var(--radius)',
              background: 'var(--bg-surface)',
              color: 'var(--text-primary)',
            }}
          />
          <Legend formatter={value => LABELS[String(value)] ?? String(value)} />
          <Line
            type="monotone"
            dataKey="gross_sales"
            name="gross_sales"
            stroke="var(--accent)"
            strokeWidth={2.5}
            dot={false}
            activeDot={{ r: 4 }}
          />
          <Line
            type="monotone"
            dataKey="gross_profit"
            name="gross_profit"
            stroke="var(--success)"
            strokeWidth={2.5}
            dot={false}
            activeDot={{ r: 4 }}
          />
          {comparePrevious && (
            <>
              <Line
                type="monotone"
                dataKey="previous_sales"
                name="previous_sales"
                stroke="var(--accent)"
                strokeWidth={1.5}
                strokeDasharray="5 5"
                dot={false}
                connectNulls
              />
              <Line
                type="monotone"
                dataKey="previous_profit"
                name="previous_profit"
                stroke="var(--success)"
                strokeWidth={1.5}
                strokeDasharray="5 5"
                dot={false}
                connectNulls
              />
            </>
          )}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}