import { useEffect, useState, type ReactNode } from 'react';
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';
import type {
  AgingBucket,
  PayablesByCategory,
  PayablesSummary,
  ReportFilter,
  TopCreditor,
  UpcomingDue,
} from '../../types/reports';
import { useSettings } from '../../context/AppContext';
import {
  getPayablesAging,
  getPayablesSummary,
  getTopCreditors,
  getUpcomingDues,
} from '../../services/reports.service';
import { formatMoney } from '../../utils/format';
import {
  agingLabel,
  debtCategoryLabel,
  debtUrgencyLabel,
  formatDueDays,
} from '../../utils/reportFormat';
import { buildPayablesCsv } from '../../utils/reportExport';
import { ReportCsvButton } from './ReportCsvButton';
import './PayablesTab.css';

interface PayablesTabProps {
  filter: ReportFilter;
  creditorLimit: number;
  onCreditorLimitChange: (limit: number) => void;
  dueLimit: number;
  onDueLimitChange: (limit: number) => void;
}

interface SectionState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
}

const AGING_COLORS = [
  'var(--text-muted)',
  'var(--warning)',
  'var(--warning)',
  'var(--danger)',
  'color-mix(in srgb, var(--danger) 78%, var(--text-primary))',
  'var(--text-secondary)',
];

function initialSection<T>(): SectionState<T> {
  return { data: null, loading: true, error: null };
}

function messageFromError(error: unknown): string {
  return error instanceof Error
    ? error.message
    : typeof error === 'string'
      ? error
      : 'Error desconocido';
}

function SectionStatus<T>({
  state,
  retry,
  emptyText,
  isEmpty,
  children,
}: {
  state: SectionState<T>;
  retry: () => void;
  emptyText: string;
  isEmpty?: (data: T) => boolean;
  children: (data: T) => ReactNode;
}) {
  if (state.loading) {
    return (
      <div className="reports-loading reports-payables-status" role="status">
        <span className="reports-spinner" aria-hidden="true" />
        Cargando…
      </div>
    );
  }
  if (state.error) {
    return (
      <div className="reports-error" role="alert">
        <span>{state.error}</span>
        <button type="button" onClick={retry}>Reintentar</button>
      </div>
    );
  }
  if (state.data === null || (isEmpty && isEmpty(state.data))) {
    return <p className="reports-empty">{emptyText}</p>;
  }
  return <>{children(state.data)}</>;
}

function formatDate(value: string): string {
  const date = new Date(`${value}T12:00:00Z`);
  return new Intl.DateTimeFormat('es-CO', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    timeZone: 'UTC',
  }).format(date);
}

export function PayablesTab({
  filter,
  creditorLimit,
  onCreditorLimitChange,
  dueLimit,
  onDueLimitChange,
}: PayablesTabProps) {
  const { settings } = useSettings();
  const [summaryRetry, setSummaryRetry] = useState(0);
  const [categoryRetry, setCategoryRetry] = useState(0);
  const [agingRetry, setAgingRetry] = useState(0);
  const [creditorsRetry, setCreditorsRetry] = useState(0);
  const [duesRetry, setDuesRetry] = useState(0);
  const [summary, setSummary] = useState<SectionState<PayablesSummary>>(
    initialSection<PayablesSummary>(),
  );
  const [categories, setCategories] = useState<SectionState<PayablesByCategory[]>>(
    initialSection<PayablesByCategory[]>(),
  );
  const [aging, setAging] = useState<SectionState<AgingBucket[]>>(
    initialSection<AgingBucket[]>(),
  );
  const [creditors, setCreditors] = useState<SectionState<TopCreditor[]>>(
    initialSection<TopCreditor[]>(),
  );
  const [dues, setDues] = useState<SectionState<UpcomingDue[]>>(
    initialSection<UpcomingDue[]>(),
  );

  useEffect(() => {
    let active = true;
    setSummary({ data: null, loading: true, error: null });
    getPayablesSummary({
      date_from: filter.date_from,
      date_to: filter.date_to,
    })
      .then(data => {
        if (active) setSummary({ data, loading: false, error: null });
      })
      .catch(error => {
        if (active) setSummary({ data: null, loading: false, error: messageFromError(error) });
      });
    return () => {
      active = false;
    };
  }, [filter.date_from, filter.date_to, summaryRetry]);

  useEffect(() => {
    let active = true;
    setCategories({ data: null, loading: true, error: null });
    getPayablesSummary({
      date_from: filter.date_from,
      date_to: filter.date_to,
    })
      .then(data => {
        if (active) setCategories({ data: data.by_category, loading: false, error: null });
      })
      .catch(error => {
        if (active) setCategories({ data: null, loading: false, error: messageFromError(error) });
      });
    return () => {
      active = false;
    };
  }, [filter.date_from, filter.date_to, categoryRetry]);

  useEffect(() => {
    let active = true;
    setAging({ data: null, loading: true, error: null });
    getPayablesAging({
      date_from: filter.date_from,
      date_to: filter.date_to,
    })
      .then(data => {
        if (active) setAging({ data, loading: false, error: null });
      })
      .catch(error => {
        if (active) setAging({ data: null, loading: false, error: messageFromError(error) });
      });
    return () => {
      active = false;
    };
  }, [filter.date_from, filter.date_to, agingRetry]);

  useEffect(() => {
    let active = true;
    setCreditors({ data: null, loading: true, error: null });
    getTopCreditors({
      date_from: filter.date_from,
      date_to: filter.date_to,
      limit: creditorLimit,
    })
      .then(data => {
        if (active) setCreditors({ data, loading: false, error: null });
      })
      .catch(error => {
        if (active) setCreditors({ data: null, loading: false, error: messageFromError(error) });
      });
    return () => {
      active = false;
    };
  }, [filter.date_from, filter.date_to, creditorLimit, creditorsRetry]);

  useEffect(() => {
    let active = true;
    setDues({ data: null, loading: true, error: null });
    getUpcomingDues({
      date_from: filter.date_from,
      date_to: filter.date_to,
      limit: dueLimit,
    })
      .then(data => {
        if (active) setDues({ data, loading: false, error: null });
      })
      .catch(error => {
        if (active) setDues({ data: null, loading: false, error: messageFromError(error) });
      });
    return () => {
      active = false;
    };
  }, [filter.date_from, filter.date_to, dueLimit, duesRetry]);

  const money = (amount: number) =>
    formatMoney(amount, settings.currency, settings.currency_decimals);
  const number = (value: number) => new Intl.NumberFormat('es-CO').format(value);
  const sortedCategories = [...(categories.data ?? [])]
    .sort((left, right) => right.balance_total - left.balance_total);

  return (
    <div className="reports-payables">
      <ReportCsvButton
        disabled={summary.loading || categories.loading || aging.loading || creditors.loading || dues.loading
          || summary.data === null || categories.data === null || aging.data === null
          || creditors.data === null || dues.data === null}
        buildReport={() => (
          summary.data !== null && categories.data !== null && aging.data !== null
            && creditors.data !== null && dues.data !== null
            ? buildPayablesCsv(
                {
                  summary: { ...summary.data, by_category: categories.data },
                  aging: aging.data,
                  creditors: creditors.data,
                  dues: dues.data,
                },
                {
                  date_from: filter.date_from,
                  date_to: filter.date_to,
                  timezone: settings.timezone,
                  generatedAt: new Date(),
                },
                settings.currency_decimals,
              )
            : null
        )}
      />
      <section className="reports-panel reports-payables-filters" aria-label="Filtros de obligaciones">
        <label>
          <span>Cantidad de acreedores</span>
          <select
            value={creditorLimit}
            onChange={event => onCreditorLimitChange(Number(event.target.value))}
          >
            <option value={10}>10</option>
            <option value={25}>25</option>
            <option value={50}>50</option>
          </select>
        </label>
        <label>
          <span>Cantidad de vencimientos</span>
          <select
            value={dueLimit}
            onChange={event => onDueLimitChange(Number(event.target.value))}
          >
            <option value={10}>10</option>
            <option value={25}>25</option>
            <option value={50}>50</option>
          </select>
        </label>
      </section>

      <section className="reports-panel">
        <div className="reports-panel-heading">
          <div>
            <h2>Resumen</h2>
            <p>Saldos: foto actual. Pagos y emisiones: entre {filter.date_from} y {filter.date_to}</p>
          </div>
        </div>
        <SectionStatus
          state={summary}
          retry={() => setSummaryRetry(value => value + 1)}
          emptyText="No hay datos de obligaciones para mostrar."
        >
          {data => (
            <div className="reports-payables-summary">
              <div className="reports-payables-kpi">
                <span>Saldo por pagar</span>
                <strong>{money(data.pending_total)}</strong>
                <small>{number(data.active_count)} deudas activas</small>
              </div>
              <div className="reports-payables-kpi reports-payables-kpi--critical">
                <span>Vencido</span>
                <strong>{money(data.overdue_total)}</strong>
                <small>{number(data.overdue_count)} deudas</small>
              </div>
              <div className="reports-payables-kpi reports-payables-kpi--warning">
                <span>Por vencer en 7 días</span>
                <strong>{money(data.due_soon_total)}</strong>
                <small>{number(data.due_soon_count)} deudas</small>
              </div>
              <div className="reports-payables-kpi">
                <span>Pagado en el rango</span>
                <strong>{money(data.paid_in_range)}</strong>
                <small>{number(data.payments_in_range_count)} pagos</small>
              </div>
              <div className="reports-payables-kpi">
                <span>Emitido en el rango</span>
                <strong>{money(data.issued_in_range)}</strong>
              </div>
            </div>
          )}
        </SectionStatus>
      </section>

      <section className="reports-panel">
        <div className="reports-panel-heading">
          <div>
            <h2>Antigüedad del vencimiento</h2>
            <p>Foto actual de los saldos pendientes</p>
          </div>
        </div>
        <SectionStatus
          state={aging}
          retry={() => setAgingRetry(value => value + 1)}
          emptyText="No hay saldos pendientes para clasificar."
          isEmpty={rows => rows.every(row => row.count === 0)}
        >
          {rows => (
            <>
              <div className="reports-payables-aging-chart">
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart
                    data={rows}
                    layout="vertical"
                    margin={{ top: 4, right: 24, bottom: 4, left: 8 }}
                  >
                    <CartesianGrid stroke="var(--border-subtle)" strokeDasharray="3 3" horizontal={false} />
                    <XAxis
                      type="number"
                      tickFormatter={tick => money(Number(tick))}
                      tickLine={false}
                      axisLine={{ stroke: 'var(--border)' }}
                    />
                    <YAxis
                      type="category"
                      dataKey="label"
                      tickFormatter={label => agingLabel(label as AgingBucket['label'])}
                      width={174}
                      tickLine={false}
                      axisLine={false}
                      tick={{ fill: 'var(--text-secondary)', fontSize: 11 }}
                    />
                    <Tooltip
                      formatter={tick => money(typeof tick === 'number' ? tick : Number(tick ?? 0))}
                      labelFormatter={label => agingLabel(label as AgingBucket['label'])}
                      contentStyle={{
                        borderColor: 'var(--border)',
                        borderRadius: 'var(--radius)',
                        background: 'var(--bg-surface)',
                        color: 'var(--text-primary)',
                      }}
                    />
                    <Bar dataKey="balance_total" name="Saldo" radius={[0, 4, 4, 0]}>
                      {rows.map((row, index) => (
                        <Cell key={row.label} fill={AGING_COLORS[index] ?? 'var(--text-muted)'} />
                      ))}
                    </Bar>
                  </BarChart>
                </ResponsiveContainer>
              </div>
              <div className="reports-table-scroll">
                <table className="reports-data-table">
                  <thead><tr><th>Vencimiento</th><th>Cantidad</th><th>Saldo</th></tr></thead>
                  <tbody>
                    {rows.map(row => (
                      <tr key={row.label}>
                        <td>{agingLabel(row.label)}</td>
                        <td>{number(row.count)}</td>
                        <td>{money(row.balance_total)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </>
          )}
        </SectionStatus>
      </section>

      <section className="reports-panel">
        <div className="reports-panel-heading">
          <div>
            <h2>Por categoría</h2>
            <p>Foto actual de los saldos</p>
          </div>
        </div>
        <SectionStatus
          state={categories}
          retry={() => setCategoryRetry(value => value + 1)}
          emptyText="No hay obligaciones activas por categoría."
          isEmpty={rows => rows.length === 0}
        >
          {() => (
            <div className="reports-table-scroll">
              <table className="reports-data-table">
                <thead>
                  <tr><th>Categoría</th><th>Saldo</th><th>Vencido</th><th>Participación</th></tr>
                </thead>
                <tbody>
                  {sortedCategories.map(row => (
                    <tr key={row.category}>
                      <td>{debtCategoryLabel(row.category)}</td>
                      <td>{money(row.balance_total)}</td>
                      <td>{money(row.overdue_total)}</td>
                      <td>{`${new Intl.NumberFormat('es-CO', { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(row.share_bps / 100)} %`}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </SectionStatus>
      </section>

      <section className="reports-panel">
        <div className="reports-panel-heading">
          <div>
            <h2>Principales acreedores</h2>
            <p>Foto actual · Los nombres se agrupan tal como están escritos</p>
          </div>
        </div>
        <SectionStatus
          state={creditors}
          retry={() => setCreditorsRetry(value => value + 1)}
          emptyText="No hay acreedores con saldo pendiente."
          isEmpty={rows => rows.length === 0}
        >
          {rows => (
            <div className="reports-table-scroll">
              <table className="reports-data-table">
                <thead>
                  <tr>
                    <th>Acreedor</th><th>Deudas activas</th><th>Saldo</th><th>Vencido</th><th>Próximo vencimiento</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map(row => (
                    <tr key={row.creditor_name}>
                      <td>{row.creditor_name}</td>
                      <td>{number(row.active_count)}</td>
                      <td>{money(row.balance_total)}</td>
                      <td>{money(row.overdue_total)}</td>
                      <td>{row.next_due_on ? formatDate(row.next_due_on) : '—'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </SectionStatus>
      </section>

      <section className="reports-panel">
        <div className="reports-panel-heading">
          <div>
            <h2>Próximos vencimientos</h2>
            <p>Demanda calculada entre {filter.date_from} y {filter.date_to}</p>
          </div>
        </div>
        <SectionStatus
          state={dues}
          retry={() => setDuesRetry(value => value + 1)}
          emptyText="No hay vencimientos activos para mostrar."
          isEmpty={rows => rows.length === 0}
        >
          {rows => (
            <div className="reports-table-scroll">
              <table className="reports-data-table">
                <thead>
                  <tr>
                    <th>Vencimiento</th><th>Acreedor</th><th>Concepto</th><th>Categoría</th>
                    <th>Saldo</th><th>Urgencia</th><th>Días</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map(row => (
                    <tr key={row.debt_id}>
                      <td>{formatDate(row.due_on)}</td>
                      <td>{row.creditor_name}</td>
                      <td>{row.concept}</td>
                      <td>{debtCategoryLabel(row.category)}</td>
                      <td>{money(row.balance)}</td>
                      <td>
                        <span className={`reports-debt-urgency reports-debt-urgency--${row.urgency}`}>
                          {debtUrgencyLabel(row.urgency)}
                        </span>
                      </td>
                      <td>{formatDueDays(row.days_until_due)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </SectionStatus>
      </section>
    </div>
  );
}
