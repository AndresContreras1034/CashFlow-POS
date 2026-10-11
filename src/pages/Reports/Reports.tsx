import { lazy, Suspense, useState } from 'react';
import type { Granularity, ReportFilter } from '../../types/reports';
import { useSettings } from '../../context/AppContext';
import {
  getReportRange,
  type ReportRangePreset,
} from '../../utils/reportRanges';
import './Reports.css';
import '../../components/reports/ReportsComponents.css';

const DashboardTab = lazy(() =>
  import('../../components/reports/DashboardTab').then(module => ({
    default: module.DashboardTab,
  })),
);
const ProductsTab = lazy(() =>
  import('../../components/reports/ProductsTab').then(module => ({
    default: module.ProductsTab,
  })),
);
const InventoryTab = lazy(() =>
  import('../../components/reports/InventoryTab').then(module => ({
    default: module.InventoryTab,
  })),
);

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
  const [activeTab, setActiveTab] = useState<'summary' | 'products' | 'inventory'>('summary');
  const [categoryId, setCategoryId] = useState<number | null>(null);
  const [rankBy, setRankBy] = useState<'units' | 'revenue' | 'profit'>('revenue');
  const [productLimit, setProductLimit] = useState(10);
  const [inventoryCategoryId, setInventoryCategoryId] = useState<number | null>(null);
  const [alertLimit, setAlertLimit] = useState(25);

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

      <div className="reports-tabs" role="tablist" aria-label="Secciones de reportes">
        <button
          type="button"
          role="tab"
          id="reports-tab-summary"
          aria-selected={activeTab === 'summary'}
          aria-controls="reports-panel-summary"
          className={activeTab === 'summary' ? 'reports-tab--active' : ''}
          onClick={() => setActiveTab('summary')}
        >
          Resumen
        </button>
        <button
          type="button"
          role="tab"
          id="reports-tab-products"
          aria-selected={activeTab === 'products'}
          aria-controls="reports-panel-products"
          className={activeTab === 'products' ? 'reports-tab--active' : ''}
          onClick={() => setActiveTab('products')}
        >
          Productos
        </button>
        <button
          type="button"
          role="tab"
          id="reports-tab-inventory"
          aria-selected={activeTab === 'inventory'}
          aria-controls="reports-panel-inventory"
          className={activeTab === 'inventory' ? 'reports-tab--active' : ''}
          onClick={() => setActiveTab('inventory')}
        >
          Inventario
        </button>
      </div>

      {activeTab === 'summary' ? (
        <div
          id="reports-panel-summary"
          role="tabpanel"
          aria-labelledby="reports-tab-summary"
          className="reports-tab-panel"
        >
          <Suspense fallback={<p className="reports-empty">Cargando resumen…</p>}>
            <DashboardTab filter={filter} />
          </Suspense>
        </div>
      ) : activeTab === 'products' ? (
        <div
          id="reports-panel-products"
          role="tabpanel"
          aria-labelledby="reports-tab-products"
          className="reports-tab-panel"
        >
          <Suspense fallback={<p className="reports-empty">Cargando productos…</p>}>
            <ProductsTab
              filter={filter}
              categoryId={categoryId}
              onCategoryIdChange={setCategoryId}
              rankBy={rankBy}
              onRankByChange={setRankBy}
              productLimit={productLimit}
              onProductLimitChange={setProductLimit}
            />
          </Suspense>
        </div>
      ) : (
        <div
          id="reports-panel-inventory"
          role="tabpanel"
          aria-labelledby="reports-tab-inventory"
          className="reports-tab-panel"
        >
          <Suspense fallback={<p className="reports-empty">Cargando inventario…</p>}>
            <InventoryTab
              filter={filter}
              categoryId={inventoryCategoryId}
              onCategoryIdChange={setInventoryCategoryId}
              alertLimit={alertLimit}
              onAlertLimitChange={setAlertLimit}
            />
          </Suspense>
        </div>
      )}
    </div>
  );
}
