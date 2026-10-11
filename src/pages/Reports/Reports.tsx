import { lazy, Suspense, useEffect, useState } from 'react';
import type {
  DashboardKpis,
  Granularity,
  HeatmapCell,
  PaymentMethodStat,
  ReportFilter,
  TimeSeriesPoint,
} from '../../types/reports';
import { useSettings } from '../../context/AppContext';
import {
  getDashboardKpis,
  getSalesByPaymentMethod,
  getSalesHeatmap,
  getSalesSeries,
} from '../../services/reports.service';
import { formatMoney } from '../../utils/format';
import {
  getPreviousReportRange,
  getReportRange,
  type ReportDateRange,
  type ReportRangePreset,
} from '../../utils/reportRanges';
import { formatBps } from '../../utils/reportFormat';
import { PaymentMethodChart } from '../../components/reports/PaymentMethodChart';
import { SalesHeatmap } from '../../components/reports/SalesHeatmap';
import './Reports.css';
import '../../components/reports/ReportsComponents.css';

const SalesChart = lazy(() =>
  import('../../components/reports/SalesChart').then(module => ({
    default: module.SalesChart,
  })),
);

interface ReportData {
  dashboard: DashboardKpis;
  series: TimeSeriesPoint[];
  previousSeries: TimeSeriesPoint[];
  paymentMethods: PaymentMethodStat[];
  heatmap: HeatmapCell[];
}

const PRESETS: Array<{ id: ReportRangePreset; label: string }> = [
  { id: 'today', label: 'Hoy' },
  { id: 'yesterday', label: 'Ayer' },
  { id: 'last_7_days', label: 'Últimos 7 días' },
  { id: 'last_30_days', label: 'Últimos 30 días' },
  { id: 'this_month', label: 'Este mes' },
  { id: 'previous_month', label: 'Mes anterior' },
];

function todayInTimezone(timezone: string): string | null {
  try {
    const parts = new Intl.DateTimeFormat('en-CA', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      timeZone: timezone,
    }).formatToParts(new Date());
    const part = (type: string) => parts.find(value => value.type === type)?.value ?? '';
    return `${part('year')}-${part('month')}-${part('day')}`;
  } catch {
    return null;
  }
}

function signedUnits(value: number | null): string {
  if (value === null) return '—';
  const amount = new Intl.NumberFormat('es-CO', { signDisplay: 'always' }).format(value);
  return `${amount} uds.`;
}

function deltaTone(value: number | null): string {
  if (value === null || value === 0) return 'reports-delta--neutral';
  return value > 0 ? 'reports-delta--positive' : 'reports-delta--negative';
}

interface MetricCardProps {
  label: string;
  value: string;
  delta: string;
  tone: string;
  comparison: boolean;
}

function MetricCard({ label, value, delta, tone, comparison }: MetricCardProps) {
  const arrow = !comparison ? '↔' : tone.endsWith('neutral') ? '↔' : tone.endsWith('positive') ? '↑' : '↓';
  return (
    <article className="reports-kpi">
      <span className="reports-kpi-label">{label}</span>
      <strong className="reports-kpi-value">{value}</strong>
      <span className={`reports-delta ${tone}`}>
        <span aria-hidden="true">{arrow}</span>
        {comparison ? delta : 'Sin comparación'}
      </span>
    </article>
  );
}

function rangeFilter(range: ReportDateRange, filter: ReportFilter): ReportFilter {
  return {
    ...filter,
    date_from: range.date_from,
    date_to: range.date_to,
    compare_previous: false,
  };
}

export default function Reports() {
  const { settings } = useSettings();
  const today = todayInTimezone(settings.timezone);
  const [preset, setPreset] = useState<ReportRangePreset | 'custom'>('last_7_days');
  const [filter, setFilter] = useState<ReportFilter>(() => ({
    ...(today
      ? getReportRange('last_7_days', today)
      : { date_from: '', date_to: '' }),
    granularity: 'day',
    compare_previous: false,
  }));
  const [data, setData] = useState<ReportData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    setData(null);
    if (!today) {
      setError('La zona horaria configurada no es válida');
      setLoading(false);
      return () => {
        active = false;
      };
    }

    const previousRange = filter.compare_previous
      ? getPreviousReportRange({
          date_from: filter.date_from,
          date_to: filter.date_to,
        })
      : null;
    const previousFilter = previousRange ? rangeFilter(previousRange, filter) : null;

    Promise.all([
      getDashboardKpis(filter),
      getSalesSeries(filter),
      getSalesByPaymentMethod(filter),
      getSalesHeatmap(filter),
      previousFilter ? getSalesSeries(previousFilter) : Promise.resolve([]),
    ])
      .then(([dashboard, series, paymentMethods, heatmap, previousSeries]) => {
        if (!active) return;
        setData({ dashboard, series, paymentMethods, heatmap, previousSeries });
      })
      .catch(cause => {
        if (active) setError(String(cause));
      })
      .finally(() => {
        if (active) setLoading(false);
      });

    return () => {
      active = false;
    };
  }, [
    filter.date_from,
    filter.date_to,
    filter.granularity,
    filter.compare_previous,
    settings.timezone,
    today,
    retry,
  ]);

  const applyPreset = (nextPreset: ReportRangePreset) => {
    if (!today) return;
    setPreset(nextPreset);
    setFilter(current => ({
      ...current,
      ...getReportRange(nextPreset, today),
    }));
  };

  const updateDate = (field: 'date_from' | 'date_to', value: string) => {
    setPreset('custom');
    setFilter(current => ({ ...current, [field]: value }));
  };

  const current = data?.dashboard.current;
  const previous = data?.dashboard.previous;
  const compared = Boolean(filter.compare_previous && previous);
  const marginChange = previous && current
    ? current.margin_bps - previous.margin_bps
    : null;
  const unitChange = previous && current ? current.units_sold - previous.units_sold : null;
  const money = (amount: number | undefined) =>
    formatMoney(amount ?? 0, settings.currency, settings.currency_decimals);

  const metricValues = [
    {
      label: 'Ventas',
      value: money(current?.gross_sales),
      delta: formatBps(data?.dashboard.gross_sales_delta.change_bps ?? null),
      amount: data?.dashboard.gross_sales_delta.change_bps ?? null,
    },
    {
      label: 'Utilidad',
      value: money(current?.gross_profit),
      delta: formatBps(data?.dashboard.gross_profit_delta.change_bps ?? null),
      amount: data?.dashboard.gross_profit_delta.change_bps ?? null,
    },
    {
      label: 'Margen',
      value: formatBps(current?.margin_bps ?? 0),
      delta: marginChange === null
        ? '—'
        : `${formatBps(marginChange).replace(' %', ' p.p.')}`,
      amount: marginChange,
    },
    {
      label: 'Tickets',
      value: new Intl.NumberFormat('es-CO').format(current?.tickets ?? 0),
      delta: formatBps(data?.dashboard.tickets_delta.change_bps ?? null),
      amount: data?.dashboard.tickets_delta.change_bps ?? null,
    },
    {
      label: 'Ticket promedio',
      value: money(current?.avg_ticket),
      delta: formatBps(data?.dashboard.avg_ticket_delta.change_bps ?? null),
      amount: data?.dashboard.avg_ticket_delta.change_bps ?? null,
    },
    {
      label: 'Unidades',
      value: new Intl.NumberFormat('es-CO').format(current?.units_sold ?? 0),
      delta: signedUnits(unitChange),
      amount: unitChange,
    },
  ];

  return (
    <div className="reports-page">
      <header className="reports-header">
        <div>
          <h1>Reportes</h1>
          <p>Ventas y comportamiento del negocio</p>
        </div>
      </header>

      <section className="reports-panel reports-filters" aria-label="Filtros de reportes">
        <div className="reports-presets" aria-label="Períodos rápidos">
          {PRESETS.map(item => (
            <button
              type="button"
              key={item.id}
              className={`reports-preset ${preset === item.id ? 'reports-preset--active' : ''}`}
              aria-pressed={preset === item.id}
              disabled={!today}
              onClick={() => applyPreset(item.id)}
            >
              {item.label}
            </button>
          ))}
        </div>
        <div className="reports-filter-controls">
          <label>
            <span>Desde</span>
            <input
              type="date"
              value={filter.date_from}
              disabled={!today}
              onChange={event => updateDate('date_from', event.target.value)}
            />
          </label>
          <label>
            <span>Hasta</span>
            <input
              type="date"
              value={filter.date_to}
              disabled={!today}
              onChange={event => updateDate('date_to', event.target.value)}
            />
          </label>
          <label className="reports-select-field">
            <span>Agrupar por</span>
            <select
              value={filter.granularity ?? 'day'}
              disabled={!today}
              onChange={event =>
                setFilter(current => ({
                  ...current,
                  granularity: event.target.value as Granularity,
                }))
              }
            >
              <option value="day">Día</option>
              <option value="week">Semana</option>
              <option value="month">Mes</option>
            </select>
          </label>
          <label className="reports-compare">
            <input
              type="checkbox"
              checked={filter.compare_previous ?? false}
              disabled={!today}
              onChange={event =>
                setFilter(current => ({
                  ...current,
                  compare_previous: event.target.checked,
                }))
              }
            />
            <span>Comparar con período anterior</span>
          </label>
        </div>
      </section>

      {error && (
        <div className="reports-error" role="alert">
          <span>{error}</span>
          <button type="button" onClick={() => setRetry(value => value + 1)}>
            Reintentar
          </button>
        </div>
      )}

      {loading && (
        <div className="reports-loading" role="status" aria-live="polite">
          <span className="reports-spinner" aria-hidden="true" />
          Cargando reportes…
        </div>
      )}

      {!loading && data && current && (
        <>
          <section className="reports-kpis" aria-label="Indicadores de ventas">
            {metricValues.map(metric => {
              const tone = deltaTone(metric.amount);
              return (
                <MetricCard
                  key={metric.label}
                  label={metric.label}
                  value={metric.value}
                  delta={metric.delta}
                  tone={tone}
                  comparison={compared}
                />
              );
            })}
          </section>

          {(current.items_without_cost > 0 || current.uncosted_tickets > 0) && (
            <aside className="reports-warning" role="status">
              <strong>
                {current.uncosted_tickets} tickets sin costo no entran en la utilidad
              </strong>
              <span>{current.items_without_cost} ítems vendidos sin costo registrado.</span>
            </aside>
          )}

          <section className="reports-summary" aria-label="Cortesías y estados">
            <div>
              <span>Cortesías</span>
              <strong>{current.courtesy_count}</strong>
            </div>
            <div>
              <span>Valor regalado</span>
              <strong>{money(current.courtesy_value)}</strong>
            </div>
            <div>
              <span>Costo de cortesías</span>
              <strong>{money(current.courtesy_cost)}</strong>
            </div>
            <div>
              <span>Anuladas</span>
              <strong>{current.cancelled_count}</strong>
            </div>
            <div>
              <span>Reembolsadas</span>
              <strong>{current.refunded_count}</strong>
            </div>
          </section>

          {current.tickets === 0 && (
            <div className="reports-empty-state">No hay ventas en el período seleccionado.</div>
          )}

          <section className="reports-panel">
            <div className="reports-panel-heading">
              <div>
                <h2>Ventas y utilidad</h2>
                <p>
                  {filter.date_from} — {filter.date_to}
                  {filter.granularity ? ` · ${filter.granularity}` : ''}
                </p>
              </div>
            </div>
            <Suspense fallback={<p className="reports-empty">Cargando gráfica…</p>}>
              <SalesChart
                data={data.series}
                previousData={data.previousSeries}
                comparePrevious={filter.compare_previous ?? false}
                currency={settings.currency}
                decimals={settings.currency_decimals}
              />
            </Suspense>
          </section>

          <div className="reports-chart-grid">
            <section className="reports-panel">
              <div className="reports-panel-heading">
                <div>
                  <h2>Ventas por método de pago</h2>
                  <p>Participación sobre el total vendido</p>
                </div>
              </div>
              <PaymentMethodChart
                data={data.paymentMethods}
                currency={settings.currency}
                decimals={settings.currency_decimals}
              />
            </section>
            <section className="reports-panel">
              <div className="reports-panel-heading">
                <div>
                  <h2>Actividad por día y hora</h2>
                  <p>De lunes a domingo, hora local del negocio</p>
                </div>
              </div>
              <SalesHeatmap
                cells={data.heatmap}
                currency={settings.currency}
                decimals={settings.currency_decimals}
              />
            </section>
          </div>
        </>
      )}
    </div>
  );
}