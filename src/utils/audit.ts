import type {
  AuditCategory,
  AuditEvent,
  AuditFilterDto,
  AuditModule,
  AuditOutcome,
} from '../types';
import { REASON_LABELS } from './movementReasons';

// ============================================================
// Errores
// ============================================================

export const errorMessage = (error: unknown): string =>
  typeof error === 'string'
    ? error
    : error instanceof Error
      ? error.message
      : 'Error desconocido';

// ============================================================
// Catálogo de etiquetas
// ============================================================

export const AUDIT_MODULES: readonly AuditModule[] = [
  'sales', 'inventory', 'cash', 'stocktake', 'settings',
  'import', 'licensing', 'billing', 'system',
];
export const AUDIT_CATEGORIES: readonly AuditCategory[] = ['business', 'error'];
export const AUDIT_OUTCOMES: readonly AuditOutcome[] = ['success', 'failure'];

export const MODULE_LABELS: Record<AuditModule, string> = {
  sales: 'Ventas',
  inventory: 'Inventario',
  cash: 'Caja',
  stocktake: 'Toma de inventario',
  settings: 'Ajustes',
  import: 'Importación',
  licensing: 'Licencia',
  billing: 'Facturación',
  system: 'Sistema',
};

export const CATEGORY_LABELS: Record<AuditCategory, string> = {
  business: 'Negocio',
  error: 'Error técnico',
};

export const OUTCOME_LABELS: Record<AuditOutcome, string> = {
  success: 'Exitoso',
  failure: 'Fallido',
};

const ACTION_LABELS: Record<string, string> = {
  'cash.open': 'Apertura de caja',
  'cash.close': 'Cierre de caja',
  'cash.create': 'Movimiento de caja',
  'sales.create': 'Venta registrada',
  'inventory.create': 'Creación',
  'inventory.update': 'Actualización',
  'inventory.stock_in': 'Entrada de stock',
  'inventory.stock_out': 'Salida de stock',
  'inventory.stock_adjust': 'Ajuste de stock',
  'stocktake.start': 'Inicio de toma',
  'stocktake.apply': 'Toma aplicada',
  'stocktake.cancel': 'Toma cancelada',
  'settings.update': 'Ajustes actualizados',
  'licensing.activate': 'Licencia activada',
  'import.execute': 'Importación ejecutada',
};

const ENTITY_TYPE_LABELS: Record<string, string> = {
  cash_session: 'Turno de caja',
  cash_movement: 'Movimiento de caja',
  sale: 'Venta',
  category: 'Categoría',
  product: 'Producto',
  variant: 'Variante',
  stocktake: 'Toma de inventario',
  app_settings: 'Ajustes',
  license: 'Licencia',
  inventory_import: 'Importación de inventario',
};

const FIELD_LABELS: Record<string, string> = {
  // genéricos
  status: 'Estado', name: 'Nombre', description: 'Descripción', notes: 'Notas',
  is_active: 'Activo', created_at: 'Creado', updated_at: 'Actualizado',
  // caja
  opening_amount: 'Monto de apertura', opening_notes: 'Notas de apertura',
  expected_amount: 'Monto esperado', counted_amount: 'Monto contado',
  difference: 'Diferencia', closing_notes: 'Notas de cierre',
  movement_type: 'Tipo de movimiento', amount: 'Monto', session_id: 'Turno',
  // ventas
  subtotal: 'Subtotal', discount: 'Descuento', tax: 'Impuesto', total: 'Total',
  item_count: 'Líneas', units: 'Unidades', payments: 'Pagos',
  customer_id: 'Cliente', cash_session_id: 'Turno de caja',
  // inventario
  category_id: 'Categoría', product_id: 'Producto', brand: 'Marca',
  image_url: 'Imagen', attributes: 'Atributos', sku: 'SKU', barcode: 'Código de barras',
  price: 'Precio', cost: 'Costo', stock: 'Stock', stock_min: 'Stock mínimo',
  allow_negative: 'Permite stock negativo', quantity: 'Cantidad',
  unit_cost: 'Costo unitario', reason: 'Motivo',
  // stocktake
  line_count: 'Variantes', adjusted: 'Ajustadas', unchanged: 'Sin diferencia',
  uncounted: 'Sin contar', surplus_units: 'Unidades sobrantes',
  shortage_units: 'Unidades faltantes', total_lines: 'Total de variantes',
  counted_lines: 'Variantes contadas', category_name: 'Nombre de categoría',
  // ajustes
  business_name: 'Nombre del negocio', tax_id: 'NIT / RUT', address: 'Dirección',
  phone: 'Teléfono', email: 'Correo', currency: 'Moneda',
  currency_decimals: 'Decimales', tax_rate_bps: 'Tasa de impuesto (pb)',
  tax_name: 'Nombre del impuesto', timezone: 'Zona horaria',
  ticket_header: 'Encabezado del ticket', ticket_footer: 'Pie del ticket',
  low_stock_default_threshold: 'Umbral de stock bajo', logo_url: 'Logo',
  // licencia
  license_id: 'Licencia', licensee: 'Titular', kind: 'Modalidad',
  issued_at: 'Emitida', expires_at: 'Vence', replaced_license_id: 'Licencia reemplazada',
  // importación
  file_name: 'Archivo', total_rows: 'Filas', new_categories: 'Categorías nuevas',
  new_products: 'Productos nuevos', new_variants: 'Variantes nuevas',
  skipped: 'Omitidas', errors: 'Errores',
  // error
  error_kind: 'Tipo de error',
};

const VALUE_LABELS: Record<string, Record<string, string>> = {
  status: {
    open: 'Abierto', closed: 'Cerrado', counting: 'En conteo',
    applied: 'Aplicada', cancelled: 'Cancelada',
  },
  movement_type: {
    manual_in: 'Ingreso manual', manual_out: 'Egreso manual',
    sale_in: 'Venta (ingreso)', sale_out: 'Venta (devolución)',
    purchase: 'Compra', sale: 'Venta', sale_return: 'Devolución',
    adjustment: 'Ajuste', initial_stock: 'Stock inicial',
  },
  method: { cash: 'Efectivo', card: 'Tarjeta', transfer: 'Transferencia' },
  reason: { ...REASON_LABELS },
  kind: { purchase: 'Compra perpetua', rental: 'Alquiler' },
  error_kind: { database: 'Base de datos', internal: 'Interno' },
};

const humanize = (key: string): string => {
  const text = key.replace(/_/g, ' ').trim();
  return text.charAt(0).toUpperCase() + text.slice(1);
};

export const moduleLabel = (module: string): string =>
  MODULE_LABELS[module as AuditModule] ?? humanize(module);

export const actionLabel = (module: string, action: string): string =>
  ACTION_LABELS[`${module}.${action}`] ?? humanize(action);

export const entityTypeLabel = (entityType: string): string =>
  ENTITY_TYPE_LABELS[entityType] ?? humanize(entityType);

export const fieldLabel = (key: string): string => FIELD_LABELS[key] ?? humanize(key);

export const formatActor = (actor: string | null): string =>
  actor && actor.trim() ? actor : '—';

/** "Variante #12" / "Ajustes #1" / "—" */
export const formatEntity = (event: Pick<AuditEvent, 'entity_type' | 'entity_id'>): string => {
  if (!event.entity_type) return '—';
  const label = entityTypeLabel(event.entity_type);
  return event.entity_id ? `${label} #${event.entity_id}` : label;
};

// ============================================================
// Fechas
// ============================================================

/** ISO UTC -> fecha localizada en la zona indicada. Si algo falla, devuelve el original. */
export const formatAuditDate = (iso: string, timeZone: string): string => {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  try {
    return new Intl.DateTimeFormat('es-CO', {
      day: '2-digit',
      month: 'short',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      timeZone,
    }).format(date);
  } catch {
    return iso;
  }
};

const DATE_ONLY = /^\d{4}-\d{2}-\d{2}$/;

const isRealDate = (value: string): boolean => {
  if (!DATE_ONLY.test(value)) return false;
  const parsed = new Date(`${value}T00:00:00Z`);
  return !Number.isNaN(parsed.getTime()) && parsed.toISOString().slice(0, 10) === value;
};

/** Mensaje de error o null. Mismo texto que el backend para el rango invertido. */
export const validateDateRange = (from: string, to: string): string | null => {
  const f = from.trim();
  const t = to.trim();
  if (f && !isRealDate(f)) return 'La fecha inicial no es válida';
  if (t && !isRealDate(t)) return 'La fecha final no es válida';
  if (f && t && f > t) return 'La fecha inicial no puede ser posterior a la final';
  return null;
};

// ============================================================
// Filtros: estado de UI <-> DTO <-> URL
// ============================================================

export const DEFAULT_PAGE_SIZE = 50;
export const MAX_PAGE_SIZE = 200;

export interface AuditFilterState {
  module: string;
  category: string;
  outcome: string;
  action: string;
  entityType: string;
  entityId: string;
  dateFrom: string;
  dateTo: string;
  search: string;
  sortDir: 'asc' | 'desc';
  page: number;
}

export const EMPTY_FILTER: AuditFilterState = {
  module: '',
  category: '',
  outcome: '',
  action: '',
  entityType: '',
  entityId: '',
  dateFrom: '',
  dateTo: '',
  search: '',
  sortDir: 'desc',
  page: 1,
};

const clean = (value: string): string | undefined => {
  const trimmed = value.trim();
  return trimmed === '' ? undefined : trimmed;
};

const oneOf = <T extends string>(allowed: readonly T[], value: string): T | undefined =>
  (allowed as readonly string[]).includes(value) ? (value as T) : undefined;

const normalizePage = (page: number): number =>
  Number.isFinite(page) && page >= 1 ? Math.floor(page) : 1;

/** Estado de UI -> filter del backend (snake_case, sin claves vacías). */
export const buildAuditFilter = (
  state: AuditFilterState,
  pageSize: number = DEFAULT_PAGE_SIZE,
): AuditFilterDto => {
  const filter: AuditFilterDto = {
    sort_dir: state.sortDir === 'asc' ? 'asc' : 'desc',
    page: normalizePage(state.page),
    page_size: Math.min(Math.max(Math.floor(pageSize) || DEFAULT_PAGE_SIZE, 1), MAX_PAGE_SIZE),
  };

  const module = oneOf(AUDIT_MODULES, state.module);
  const category = oneOf(AUDIT_CATEGORIES, state.category);
  const outcome = oneOf(AUDIT_OUTCOMES, state.outcome);
  const action = clean(state.action);
  const entityType = clean(state.entityType);
  const entityId = clean(state.entityId);
  const dateFrom = clean(state.dateFrom);
  const dateTo = clean(state.dateTo);
  const search = clean(state.search);

  if (module) filter.module = module;
  if (category) filter.category = category;
  if (outcome) filter.outcome = outcome;
  if (action) filter.action = action;
  if (entityType) filter.entity_type = entityType;
  if (entityId) filter.entity_id = entityId;
  if (dateFrom) filter.date_from = dateFrom;
  if (dateTo) filter.date_to = dateTo;
  if (search) filter.search = search;
  return filter;
};

/** True si hay algún filtro activo (sin contar orden ni página). */
export const hasActiveFilters = (state: AuditFilterState): boolean =>
  [
    state.module, state.category, state.outcome, state.action,
    state.entityType, state.entityId, state.dateFrom, state.dateTo, state.search,
  ].some(value => value.trim() !== '');

/** Estado -> query string. Omite valores por defecto. */
export const filterToSearchParams = (state: AuditFilterState): URLSearchParams => {
  const params = new URLSearchParams();
  const put = (key: string, value: string) => {
    const trimmed = value.trim();
    if (trimmed) params.set(key, trimmed);
  };
  put('module', state.module);
  put('category', state.category);
  put('outcome', state.outcome);
  put('action', state.action);
  put('entity_type', state.entityType);
  put('entity_id', state.entityId);
  put('from', state.dateFrom);
  put('to', state.dateTo);
  put('q', state.search);
  if (state.sortDir === 'asc') params.set('sort', 'asc');
  const page = normalizePage(state.page);
  if (page > 1) params.set('page', String(page));
  return params;
};

/** Query string -> estado. Ignora valores inválidos. */
export const searchParamsToFilter = (params: URLSearchParams): AuditFilterState => {
  const get = (key: string) => params.get(key)?.trim() ?? '';
  const page = Number(params.get('page'));
  const from = get('from');
  const to = get('to');
  return {
    module: oneOf(AUDIT_MODULES, get('module')) ?? '',
    category: oneOf(AUDIT_CATEGORIES, get('category')) ?? '',
    outcome: oneOf(AUDIT_OUTCOMES, get('outcome')) ?? '',
    action: get('action'),
    entityType: get('entity_type'),
    entityId: get('entity_id'),
    dateFrom: isRealDate(from) ? from : '',
    dateTo: isRealDate(to) ? to : '',
    search: get('q'),
    sortDir: get('sort') === 'asc' ? 'asc' : 'desc',
    page: Number.isInteger(page) && page >= 1 ? page : 1,
  };
};

// ============================================================
// Changes y metadata
// ============================================================

export type MoneyFormatter = (minor: number) => string;

const MONEY_FIELDS: Record<string, readonly string[]> = {
  cash: ['opening_amount', 'expected_amount', 'counted_amount', 'difference', 'amount'],
  sales: ['subtotal', 'discount', 'tax', 'total', 'amount'],
  inventory: ['price', 'cost', 'unit_cost'],
};

/** Si el campo es un importe en centavos según el módulo. */
export const isMoneyField = (module: string, key: string): boolean =>
  MONEY_FIELDS[module]?.includes(key) ?? false;

const isPlainObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

/** Valor genérico legible. Los textos largos ("[texto de N caracteres]") pasan tal cual. */
export const formatAuditValue = (
  module: string,
  key: string,
  value: unknown,
  formatMoney: MoneyFormatter,
): string => {
  if (value === null || value === undefined) return '—';
  if (typeof value === 'boolean') return value ? 'Sí' : 'No';
  if (typeof value === 'number') {
    return isMoneyField(module, key) ? formatMoney(value) : String(value);
  }
  if (typeof value === 'string') {
    if (value === '') return '(vacío)';
    return VALUE_LABELS[key]?.[value] ?? value;
  }
  if (Array.isArray(value) && key === 'payments') {
    return formatPayments(module, value, formatMoney);
  }
  if (isPlainObject(value) && key === 'attributes') {
    const entries = Object.entries(value);
    return entries.length === 0
      ? '—'
      : entries.map(([k, v]) => `${humanize(k)}: ${String(v)}`).join(' · ');
  }
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
};

const formatPayments = (module: string, payments: unknown[], formatMoney: MoneyFormatter): string => {
  const parts = payments.map(item => {
    if (!isPlainObject(item)) return String(item);
    const method = typeof item.method === 'string'
      ? (VALUE_LABELS.method[item.method] ?? item.method)
      : 'Pago';
    const amount = typeof item.amount === 'number'
      ? (isMoneyField(module, 'amount') ? formatMoney(item.amount) : String(item.amount))
      : '—';
    return `${method}: ${amount}`;
  });
  return parts.length > 0 ? parts.join(' · ') : '—';
};

export interface ChangeRow {
  key: string;
  label: string;
  from: string;
  to: string;
}

export interface MetadataRow {
  key: string;
  label: string;
  value: string;
}

/** `changes` -> filas antes/después. Tolera entradas mal formadas. */
export const buildChangeRows = (
  event: Pick<AuditEvent, 'module' | 'changes'>,
  formatMoney: MoneyFormatter,
): ChangeRow[] => {
  if (!event.changes) return [];
  return Object.entries(event.changes).map(([key, change]) => {
    const hasShape = isPlainObject(change) && ('from' in change || 'to' in change);
    const from = hasShape ? change.from : undefined;
    const to = hasShape ? change.to : change;
    return {
      key,
      label: fieldLabel(key),
      from: formatAuditValue(event.module, key, from, formatMoney),
      to: formatAuditValue(event.module, key, to, formatMoney),
    };
  });
};

/** `metadata` -> filas etiquetadas. Claves desconocidas usan el renderizador genérico. */
export const buildMetadataRows = (
  event: Pick<AuditEvent, 'module' | 'metadata'>,
  formatMoney: MoneyFormatter,
): MetadataRow[] => {
  if (!event.metadata) return [];
  return Object.entries(event.metadata).map(([key, value]) => ({
    key,
    label: fieldLabel(key),
    value: formatAuditValue(event.module, key, value, formatMoney),
  }));
};