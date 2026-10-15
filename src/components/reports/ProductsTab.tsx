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
  CategorySales,
  RankBy,
  ReportFilter,
  StaleProduct,
  TopProduct,
} from '../../types/reports';
import { useSettings } from '../../context/AppContext';
import {
  getSalesByCategory,
  getStaleProducts,
  getTopProducts,
} from '../../services/reports.service';
import { listCategories } from '../../services/inventory.service';
import { formatMoney } from '../../utils/format';
import { formatBps, formatDaysSinceSale, formatVariantAttributes } from '../../utils/reportFormat';
import { buildProductsCsv } from '../../utils/reportExport';
import { ReportCsvButton } from './ReportCsvButton';
import './ProductsTab.css';

interface ProductsTabProps {
  filter: ReportFilter;
  categoryId: number | null;
  onCategoryIdChange: (categoryId: number | null) => void;
  rankBy: RankBy;
  onRankByChange: (rankBy: RankBy) => void;
  productLimit: number;
  onProductLimitChange: (limit: number) => void;
}

interface SectionState<T> {
  rows: T[] | null;
  loading: boolean;
  error: string | null;
}

function initialSection<T>(): SectionState<T> {
  return { rows: null, loading: true, error: null };
}

function messageFromError(error: unknown): string {
  return error instanceof Error
    ? error.message
    : typeof error === 'string'
      ? error
      : 'Error desconocido';
}

function sectionFromResult<T>(
  result: PromiseSettledResult<T[]>,
): SectionState<T> {
  return result.status === 'fulfilled'
    ? { rows: result.value, loading: false, error: null }
    : { rows: null, loading: false, error: messageFromError(result.reason) };
}

function SectionStatus<T>({
  state,
  retry,
  emptyText,
  children,
}: {
  state: SectionState<T>;
  retry: () => void;
  emptyText: string;
  children: (rows: T[]) => ReactNode;
}) {
  if (state.loading) {
    return (
      <div className="reports-loading reports-products-status" role="status">
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
  if (!state.rows?.length) {
    return <p className="reports-empty">{emptyText}</p>;
  }
  return <>{children(state.rows)}</>;
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

function Margin({ value }: { value: number | null }) {
  return value === null
    ? <span className="reports-margin reports-margin--empty">—</span>
    : <span className={`reports-margin ${value < 0 ? 'reports-margin--negative' : 'reports-margin--positive'}`}>
        {formatBps(value)}
      </span>;
}

export function ProductsTab({
  filter,
  categoryId,
  onCategoryIdChange,
  rankBy,
  onRankByChange,
  productLimit,
  onProductLimitChange,
}: ProductsTabProps) {
  const { settings } = useSettings();
  const [categories, setCategories] = useState<Category[]>([]);
  const [categoriesLoading, setCategoriesLoading] = useState(true);
  const [categoriesError, setCategoriesError] = useState<string | null>(null);
  const [categoryRetry, setCategoryRetry] = useState(0);
  const [retry, setRetry] = useState(0);
  const [topProducts, setTopProducts] = useState<SectionState<TopProduct>>(
    initialSection<TopProduct>(),
  );
  const [categorySales, setCategorySales] = useState<SectionState<CategorySales>>(
    initialSection<CategorySales>(),
  );
  const [staleProducts, setStaleProducts] = useState<SectionState<StaleProduct>>(
    initialSection<StaleProduct>(),
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
    setTopProducts({ rows: null, loading: true, error: null });
    setCategorySales({ rows: null, loading: true, error: null });
    setStaleProducts({ rows: null, loading: true, error: null });

    const shared: ReportFilter = {
      date_from: filter.date_from,
      date_to: filter.date_to,
    };
    const topFilter: ReportFilter = {
      ...shared,
      category_id: categoryId,
      rank_by: rankBy,
      limit: productLimit,
    };
    const staleFilter: ReportFilter = { ...shared, category_id: categoryId, limit: productLimit };

    Promise.allSettled([
      getTopProducts(topFilter),
      getSalesByCategory(shared),
      getStaleProducts(staleFilter),
    ]).then(([topResult, categoriesResult, staleResult]) => {
      if (!active) return;
      setTopProducts(sectionFromResult(topResult));
      setCategorySales(sectionFromResult(categoriesResult));
      setStaleProducts(sectionFromResult(staleResult));
    });

    return () => {
      active = false;
    };
  }, [filter.date_from, filter.date_to, categoryId, rankBy, productLimit, retry]);

  const money = (amount: number) =>
    formatMoney(amount, settings.currency, settings.currency_decimals);
  const retrySections = () => setRetry(value => value + 1);

  return (
    <div className="reports-products">
      <ReportCsvButton
        disabled={
          topProducts.loading || categorySales.loading || staleProducts.loading
          || topProducts.rows === null || categorySales.rows === null || staleProducts.rows === null
          || !(topProducts.rows.length + categorySales.rows.length + staleProducts.rows.length)
        }
        buildReport={() => (
          topProducts.rows !== null && categorySales.rows !== null && staleProducts.rows !== null
            ? buildProductsCsv(
                {
                  topProducts: topProducts.rows,
                  categories: categorySales.rows,
                  staleProducts: staleProducts.rows,
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
      <section className="reports-panel reports-products-filters" aria-label="Filtros de productos">
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
          <span>Ordenar por</span>
          <select
            value={rankBy}
            onChange={event => onRankByChange(event.target.value as RankBy)}
          >
            <option value="units">Unidades</option>
            <option value="revenue">Ingreso</option>
            <option value="profit">Utilidad</option>
          </select>
        </label>
        <label>
          <span>Cantidad</span>
          <select
            value={productLimit}
            onChange={event => onProductLimitChange(Number(event.target.value))}
          >
            <option value={10}>10</option>
            <option value={25}>25</option>
            <option value={50}>50</option>
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
            <h2>Top productos</h2>
            <p>
              {categoryId
                ? 'Participación dentro de la categoría seleccionada'
                : 'Participación sobre el total del período'}
            </p>
          </div>
        </div>
        <SectionStatus
          state={topProducts}
          retry={retrySections}
          emptyText="No hay productos vendidos en el período seleccionado."
        >
          {rows => (
            <div className="reports-table-scroll">
              <table className="reports-data-table">
                <thead>
                  <tr>
                    <th>#</th>
                    <th>Producto / variante</th>
                    <th>Unidades</th>
                    <th>Ingreso neto</th>
                    <th>Utilidad</th>
                    <th>Margen</th>
                    <th>Participación</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((product, index) => (
                    <tr key={product.variant_id}>
                      <td>{index + 1}</td>
                      <td>
                        <ProductName name={product.product_name} attributes={product.attributes} />
                        {product.uncosted_units > 0 && (
                          <span
                            className="reports-badge reports-badge--warning"
                            title="Esas unidades no entran en utilidad"
                          >
                            {product.uncosted_units} uds sin costo
                          </span>
                        )}
                      </td>
                      <td>{new Intl.NumberFormat('es-CO').format(product.units)}</td>
                      <td title={`Bruto con IVA: ${money(product.gross_revenue)}`}>
                        {money(product.revenue)}
                      </td>
                      <td>{money(product.profit)}</td>
                      <td><Margin value={product.margin_bps} /></td>
                      <td>
                        <div className="reports-share">
                          <span>{formatBps(product.share_bps)}</span>
                          <span
                            className="reports-share-track"
                            role="img"
                            aria-label={`Participación ${formatBps(product.share_bps)}`}
                          >
                            <span style={{ width: `${Math.max(0, Math.min(100, product.share_bps / 100))}%` }} />
                          </span>
                        </div>
                      </td>
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
            <h2>Ventas por categoría</h2>
            <p>Ingreso neto · ignora el filtro de categoría</p>
          </div>
        </div>
        <SectionStatus
          state={categorySales}
          retry={retrySections}
          emptyText="No hay ventas por categoría en el período seleccionado."
        >
          {rows => (
            <>
              <div className="reports-category-chart" style={{ height: Math.max(190, rows.length * 42) }}>
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart
                    data={rows}
                    layout="vertical"
                    margin={{ top: 4, right: 20, bottom: 4, left: 8 }}
                  >
                    <CartesianGrid stroke="var(--border-subtle)" strokeDasharray="3 3" horizontal={false} />
                    <XAxis
                      type="number"
                      tickFormatter={value => new Intl.NumberFormat('es-CO', {
                        notation: 'compact',
                        maximumFractionDigits: 1,
                      }).format(Number(value) / 100)}
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
                      formatter={value => money(typeof value === 'number' ? value : Number(value ?? 0))}
                      contentStyle={{
                        borderColor: 'var(--border)',
                        borderRadius: 'var(--radius)',
                        background: 'var(--bg-surface)',
                        color: 'var(--text-primary)',
                      }}
                    />
                    <Bar dataKey="revenue" name="Ingreso neto" fill="var(--accent)" radius={[0, 4, 4, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              </div>
              <div className="reports-table-scroll">
                <table className="reports-data-table">
                  <thead>
                    <tr>
                      <th>Categoría</th>
                      <th>Unidades</th>
                      <th>Ingreso neto</th>
                      <th>Utilidad</th>
                      <th>Margen</th>
                      <th>Participación</th>
                    </tr>
                  </thead>
                  <tbody>
                    {rows.map(category => (
                      <tr key={category.category_id}>
                        <td>{category.category_name}</td>
                        <td>{new Intl.NumberFormat('es-CO').format(category.units)}</td>
                        <td title={`Bruto con IVA: ${money(category.gross_revenue)}`}>
                          {money(category.revenue)}
                        </td>
                        <td>{money(category.profit)}</td>
                        <td><Margin value={category.margin_bps} /></td>
                        <td>{formatBps(category.share_bps)}</td>
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
            <h2>Productos sin movimiento</h2>
            <p>Sin ventas entre {filter.date_from} y {filter.date_to}</p>
          </div>
        </div>
        <SectionStatus
          state={staleProducts}
          retry={retrySections}
          emptyText="No hay productos con stock sin movimiento en este período."
        >
          {rows => {
            const totalValue = BigInt(rows[0].total_value_at_cost);
            const missingCostCount = rows[0].total_without_cost;
            return (
              <>
                <div className="reports-stale-summary">
                  <div>
                    <span>Valor inmovilizado total a costo</span>
                    <strong>{money(Number(totalValue))}</strong>
                  </div>
                  <div>
                    <span>Variantes sin costo (total)</span>
                    <strong>{missingCostCount}</strong>
                  </div>
                  <p className="reports-stale-warning" role="status">
                    Mostrando {rows.length} de {rows[0].total_rows} variantes sin movimiento.
                  </p>
                  {missingCostCount > 0 && (
                    <p className="reports-stale-warning" role="status">
                      El valor está subestimado porque hay productos sin costo registrado.
                    </p>
                  )}
                </div>
                <div className="reports-table-scroll">
                  <table className="reports-data-table">
                    <thead>
                      <tr>
                        <th>Producto / variante</th>
                        <th>Stock</th>
                        <th>Valor a costo</th>
                        <th>Días sin venta</th>
                      </tr>
                    </thead>
                    <tbody>
                      {rows.map(product => (
                        <tr key={product.variant_id}>
                          <td>
                            <ProductName name={product.product_name} attributes={product.attributes} />
                            {product.cost_missing && (
                              <span
                                className="reports-badge reports-badge--warning"
                                title="El valor está subestimado"
                              >
                                Sin costo
                              </span>
                            )}
                          </td>
                          <td>{new Intl.NumberFormat('es-CO').format(product.stock)}</td>
                          <td>{money(product.stock_value_at_cost)}</td>
                          <td>{formatDaysSinceSale(product.days_since_last_sale)}</td>
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
    </div>
  );
}
