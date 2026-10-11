import { useEffect, useState, type ReactNode } from 'react';
import {
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';
import type { Category } from '../../types';
import type {
  InventoryTurnover,
  InventoryValuation,
  ReportFilter,
  StockAlert,
} from '../../types/reports';
import { useSettings } from '../../context/AppContext';
import {
  getInventoryTurnover,
  getInventoryValuation,
  getStockAlerts,
} from '../../services/reports.service';
import { listCategories } from '../../services/inventory.service';
import { formatMoney } from '../../utils/format';
import {
  formatBps,
  formatDaysOfCover,
  formatTurnover,
  formatVariantAttributes,
  stockStatusLabel,
} from '../../utils/reportFormat';
import './InventoryTab.css';

interface InventoryTabProps {
  filter: ReportFilter;
  categoryId: number | null;
  onCategoryIdChange: (categoryId: number | null) => void;
  alertLimit: number;
  onAlertLimitChange: (limit: number) => void;
}

interface SectionState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
}

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
      <div className="reports-loading reports-inventory-status" role="status">
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

function ProductName({ name, attributes }: { name: string; attributes: unknown }) {
  const variant = formatVariantAttributes(attributes);
  return (
    <span className="reports-product-name">
      <strong>{name}</strong>
      {variant && <small>{variant}</small>}
    </span>
  );
}

export function InventoryTab({
  filter,
  categoryId,
  onCategoryIdChange,
  alertLimit,
  onAlertLimitChange,
}: InventoryTabProps) {
  const { settings } = useSettings();
  const [categories, setCategories] = useState<Category[]>([]);
  const [categoriesLoading, setCategoriesLoading] = useState(true);
  const [categoriesError, setCategoriesError] = useState<string | null>(null);
  const [categoryRetry, setCategoryRetry] = useState(0);
  const [valuationRetry, setValuationRetry] = useState(0);
  const [turnoverRetry, setTurnoverRetry] = useState(0);
  const [alertsRetry, setAlertsRetry] = useState(0);
  const [valuation, setValuation] = useState<SectionState<InventoryValuation>>(
    initialSection<InventoryValuation>(),
  );
  const [turnover, setTurnover] = useState<SectionState<InventoryTurnover>>(
    initialSection<InventoryTurnover>(),
  );
  const [alerts, setAlerts] = useState<SectionState<StockAlert[]>>(
    initialSection<StockAlert[]>(),
  );

  useEffect(() => {
    let active = true;
    setCategoriesLoading(true);
    setCategoriesError(null);
    listCategories()
      .then(result => {
        if (active) setCategories(result);
      })
      .catch(error => {
        if (active) setCategoriesError(messageFromError(error));
      })
      .finally(() => {
        if (active) setCategoriesLoading(false);
      });
    return () => {
      active = false;
    };
  }, [categoryRetry]);

  useEffect(() => {
    let active = true;
    setValuation({ data: null, loading: true, error: null });
    getInventoryValuation({
      date_from: filter.date_from,
      date_to: filter.date_to,
      category_id: categoryId,
    })
      .then(data => {
        if (active) setValuation({ data, loading: false, error: null });
      })
      .catch(error => {
        if (active) setValuation({ data: null, loading: false, error: messageFromError(error) });
      });
    return () => {
      active = false;
    };
  }, [filter.date_from, filter.date_to, categoryId, valuationRetry]);

  useEffect(() => {
    let active = true;
    setTurnover({ data: null, loading: true, error: null });
    getInventoryTurnover({
      date_from: filter.date_from,
      date_to: filter.date_to,
      category_id: categoryId,
    })
      .then(data => {
        if (active) setTurnover({ data, loading: false, error: null });
      })
      .catch(error => {
        if (active) setTurnover({ data: null, loading: false, error: messageFromError(error) });
      });
    return () => {
      active = false;
    };
  }, [filter.date_from, filter.date_to, categoryId, turnoverRetry]);

  useEffect(() => {
    let active = true;
    setAlerts({ data: null, loading: true, error: null });
    getStockAlerts({
      date_from: filter.date_from,
      date_to: filter.date_to,
      category_id: categoryId,
      limit: alertLimit,
    })
      .then(data => {
        if (active) setAlerts({ data, loading: false, error: null });
      })
      .catch(error => {
        if (active) setAlerts({ data: null, loading: false, error: messageFromError(error) });
      });
    return () => {
      active = false;
    };
  }, [filter.date_from, filter.date_to, categoryId, alertLimit, alertsRetry]);

  const money = (amount: number) =>
    formatMoney(amount, settings.currency, settings.currency_decimals);
  const number = (value: number) => new Intl.NumberFormat('es-CO').format(value);

  return (
    <div className="reports-inventory">
      <section className="reports-panel reports-products-filters" aria-label="Filtros de inventario">
        <label>
          <span>Categoría</span>
          <select
            value={categoryId ?? ''}
            disabled={categoriesLoading}
            onChange={event =>
              onCategoryIdChange(event.target.value ? Number(event.target.value) : null)
            }
          >
            <option value="">Todas</option>
            {categories.map(category => (
              <option value={category.id} key={category.id}>{category.name}</option>
            ))}
          </select>
        </label>
        <label>
          <span>Cantidad de alertas</span>
          <select
            value={alertLimit}
            onChange={event => onAlertLimitChange(Number(event.target.value))}
          >
            <option value={25}>25</option>
            <option value={50}>50</option>
            <option value={100}>100</option>
          </select>
        </label>
      </section>

      {categoriesError && (
        <div className="reports-error" role="alert">
          <span>No se pudieron cargar las categorías: {categoriesError}</span>
          <button type="button" onClick={() => setCategoryRetry(value => value + 1)}>
            Reintentar
          </button>
        </div>
      )}

      <section className="reports-panel">
        <div className="reports-panel-heading">
          <div>
            <h2>Valorización</h2>
            <p>Foto actual del inventario: no depende de las fechas</p>
          </div>
        </div>
        <SectionStatus
          state={valuation}
          retry={() => setValuationRetry(value => value + 1)}
          emptyText="No hay datos de inventario para mostrar."
        >
          {value => (
            <>
              <div className="reports-inventory-kpis">
                <div className="reports-inventory-kpi">
                  <span>Valor a costo</span>
                  <strong>{money(value.value_at_cost)}</strong>
                </div>
                <div className="reports-inventory-kpi">
                  <span>Valor potencial de venta</span>
                  <strong>{money(value.potential_sale_value)}</strong>
                </div>
                <div className="reports-inventory-kpi">
                  <span>Margen potencial</span>
                  <strong>{formatBps(value.potential_margin_bps)}</strong>
                </div>
                <div className="reports-inventory-kpi">
                  <span>Unidades en stock</span>
                  <strong>{number(value.units_in_stock)}</strong>
                </div>
              </div>
              {value.variants_without_cost > 0 && (
                <p className="reports-inventory-warning" role="status">
                  {number(value.variants_without_cost)} variantes sin costo: el valor está subestimado
                </p>
              )}
              {value.negative_stock_variants > 0 && (
                <p className="reports-inventory-warning" role="status">
                  {number(value.negative_stock_variants)} variantes con stock negativo, fuera de los totales
                </p>
              )}
            </>
          )}
        </SectionStatus>
      </section>

      <section className="reports-panel">
        <div className="reports-panel-heading">
          <div>
            <h2>Valor por categoría</h2>
            <p>Valorización actual a costo</p>
          </div>
        </div>
        <SectionStatus
          state={valuation}
          retry={() => setValuationRetry(value => value + 1)}
          emptyText="No hay categorías con inventario."
          isEmpty={value => value.by_category.length === 0}
        >
          {value => {
            const rows = value.by_category;
            return (
              <>
                <div
                  className="reports-inventory-chart"
                  style={{ height: Math.max(190, rows.length * 42) }}
                >
                  <ResponsiveContainer width="100%" height="100%">
                    <BarChart
                      data={rows}
                      layout="vertical"
                      margin={{ top: 4, right: 20, bottom: 4, left: 8 }}
                    >
                      <CartesianGrid stroke="var(--border-subtle)" strokeDasharray="3 3" horizontal={false} />
                      <XAxis
                        type="number"
                        tickFormatter={tick => new Intl.NumberFormat('es-CO', {
                          notation: 'compact',
                          maximumFractionDigits: 1,
                        }).format(Number(tick) / 100)}
                        tickLine={false}
                        axisLine={{ stroke: 'var(--border)' }}
                      />
                      <YAxis
                        type="category"
                        dataKey="category_name"
                        width={132}
                        tickLine={false}
                        axisLine={false}
                        tick={{ fill: 'var(--text-secondary)', fontSize: 11 }}
                      />
                      <Tooltip
                        formatter={tick => money(typeof tick === 'number' ? tick : Number(tick ?? 0))}
                        contentStyle={{
                          borderColor: 'var(--border)',
                          borderRadius: 'var(--radius)',
                          background: 'var(--bg-surface)',
                          color: 'var(--text-primary)',
                        }}
                      />
                      <Bar
                        dataKey="value_at_cost"
                        name="Valor a costo"
                        fill="var(--accent)"
                        radius={[0, 4, 4, 0]}
                      />
                    </BarChart>
                  </ResponsiveContainer>
                </div>
                <div className="reports-table-scroll">
                  <table className="reports-data-table">
                    <thead>
                      <tr>
                        <th>Categoría</th>
                        <th>Unidades</th>
                        <th>Valor a costo</th>
                        <th>Valor de venta</th>
                        <th>Participación</th>
                      </tr>
                    </thead>
                    <tbody>
                      {rows.map(category => (
                        <tr key={category.category_id}>
                          <td>
                            {category.category_name}
                            {category.variants_without_cost > 0 && (
                              <span className="reports-badge reports-badge--warning">Sin costo</span>
                            )}
                          </td>
                          <td>{number(category.units)}</td>
                          <td>{money(category.value_at_cost)}</td>
                          <td>{money(category.potential_sale_value)}</td>
                          <td>{formatBps(category.share_bps)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </>
            );
          }}
        </SectionStatus>
      </section>

      <section className="reports-panel">
        <div className="reports-panel-heading">
          <div>
            <h2>Rotación</h2>
            <p>Se calcula con el inventario actual, no con el promedio del período; no es comparable entre rangos de distinta duración</p>
          </div>
        </div>
        <SectionStatus
          state={turnover}
          retry={() => setTurnoverRetry(value => value + 1)}
          emptyText="No hay datos de rotación para mostrar."
        >
          {value => (
            <>
              <div className="reports-inventory-kpis reports-inventory-kpis--turnover">
                <div className="reports-inventory-kpi reports-inventory-kpi--primary">
                  <span>Días de inventario</span>
                  <strong>
                    {value.days_of_inventory === null
                      ? '—'
                      : `${number(value.days_of_inventory)} días`}
                  </strong>
                </div>
                <div className="reports-inventory-kpi">
                  <span>Rotación</span>
                  <strong>{formatTurnover(value.turnover_bps)}</strong>
                </div>
                <div className="reports-inventory-kpi">
                  <span>COGS del período</span>
                  <strong>{money(value.cogs)}</strong>
                </div>
                <div className="reports-inventory-kpi">
                  <span>Inventario actual a costo</span>
                  <strong>{money(value.inventory_value_at_cost)}</strong>
                </div>
              </div>
              {value.uncosted_items > 0 && (
                <p className="reports-inventory-warning" role="status">
                  {number(value.uncosted_items)} ítems vendidos sin costo no entran en el COGS
                </p>
              )}
            </>
          )}
        </SectionStatus>
      </section>

      <section className="reports-panel">
        <div className="reports-panel-heading">
          <div>
            <h2>Alertas de stock</h2>
            <p>Demanda calculada entre {filter.date_from} y {filter.date_to}</p>
          </div>
        </div>
        <SectionStatus
          state={alerts}
          retry={() => setAlertsRetry(value => value + 1)}
          emptyText="No hay alertas de stock en esta categoría."
          isEmpty={rows => rows.length === 0}
        >
          {rows => (
            <div className="reports-table-scroll">
              <table className="reports-data-table">
                <thead>
                  <tr>
                    <th>Producto / variante</th>
                    <th>Categoría</th>
                    <th>Estado</th>
                    <th>Stock / mínimo</th>
                    <th>Unidades vendidas</th>
                    <th>Días de cobertura</th>
                    <th>Reposición sugerida</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map(alert => (
                    <tr key={alert.variant_id}>
                      <td><ProductName name={alert.product_name} attributes={alert.attributes} /></td>
                      <td>{alert.category_name}</td>
                      <td>
                        <span className={`reports-stock-status reports-stock-status--${alert.status}`}>
                          {stockStatusLabel(alert.status)}
                        </span>
                      </td>
                      <td>{number(alert.stock)} / {number(alert.stock_min)}</td>
                      <td>{number(alert.units_sold)}</td>
                      <td>{formatDaysOfCover(alert.days_of_cover)}</td>
                      <td>{number(alert.suggested_reorder)}</td>
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
