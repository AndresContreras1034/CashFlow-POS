import type {
  DebtCategory,
  DebtCategoryStat,
  DebtFields,
  DebtFilterDto,
  DebtNature,
  DebtOverview,
  DebtPaymentMethod,
  DebtSortBy,
  DebtStatus,
  DebtUrgency,
  LabeledOption,
  PaymentFields,
} from '../types/debts';
import { minorToInput, parseMoneyInput } from './format';

// ============================================================
// Vencimiento: texto y tono
//
// El backend calcula `status`, `urgency` y `days_until_due` con la fecha
// de hoy en la zona horaria de Ajustes. Aquí solo se traducen a texto y
// tono visual: no se calcula ninguna fecha ni se escribe ningún umbral.
// ============================================================

export type DueTone = 'ok' | 'alert' | 'today' | 'overdue' | 'paid' | 'voided' | 'none';

export interface DueIndicator {
  tone: DueTone;
  text: string;
}

const days = (n: number): string => `${n} ${n === 1 ? 'día' : 'días'}`;

/** Mensaje y tono del vencimiento. Estado financiero y urgencia son independientes. */
export const dueIndicator = (
  debt: Pick<DebtOverview, 'status' | 'urgency' | 'days_until_due'>,
): DueIndicator => {
  if (debt.status === 'voided') return { tone: 'voided', text: 'Anulada' };
  if (debt.status === 'paid') return { tone: 'paid', text: 'Pagada' };

  const remaining = debt.days_until_due;
  if (debt.urgency === 'none' || remaining === null) {
    return { tone: 'none', text: 'Sin vencimiento' };
  }

  switch (debt.urgency) {
    case 'ok':
      return { tone: 'ok', text: `Vence en ${days(remaining)}` };
    case 'alert':
      return {
        tone: 'alert',
        text: remaining === 1 ? 'Vence mañana' : `Vence en ${days(remaining)}`,
      };
    case 'due_today':
      return { tone: 'today', text: 'Vence hoy' };
    case 'overdue':
      return { tone: 'overdue', text: `Vencida hace ${days(Math.abs(remaining))}` };
  }
};

/** Etiqueta de una opción del catálogo del backend (`get_debt_options`). */
export const labelOf = <T extends string>(
  options: readonly LabeledOption<T>[],
  value: T,
): string => options.find(option => option.value === value)?.label ?? value;

// ============================================================
// Claves de idempotencia y fechas
// ============================================================

/** UUID v4 para `idempotency_key`. Usa `crypto.randomUUID` si existe. */
export const newIdempotencyKey = (): string => {
  const c = globalThis.crypto;
  if (c && typeof c.randomUUID === 'function') return c.randomUUID();

  const bytes = new Uint8Array(16);
  if (c && typeof c.getRandomValues === 'function') {
    c.getRandomValues(bytes);
  } else {
    for (let i = 0; i < 16; i += 1) bytes[i] = Math.floor(Math.random() * 256);
  }
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = Array.from(bytes, b => b.toString(16).padStart(2, '0')).join('');
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
};

const DATE_ONLY = /^\d{4}-\d{2}-\d{2}$/;

export const isRealDate = (value: string): boolean => {
  if (!DATE_ONLY.test(value)) return false;
  const parsed = new Date(`${value}T00:00:00Z`);
  return !Number.isNaN(parsed.getTime()) && parsed.toISOString().slice(0, 10) === value;
};

/**
 * «Hoy» como 'YYYY-MM-DD' en la zona de Ajustes. Solo sirve para el atributo
 * `max` de los campos de fecha; el backend valida con su propio «hoy».
 */
export const todayInTimeZone = (timeZone: string, now: Date = new Date()): string => {
  try {
    return new Intl.DateTimeFormat('en-CA', {
      timeZone,
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
    }).format(now);
  } catch {
    const local = new Date(now.getTime() - now.getTimezoneOffset() * 60_000);
    return local.toISOString().slice(0, 10);
  }
};

// ============================================================
// Límites (espejo de los del backend; el backend es la autoridad)
// ============================================================

export const LIMITS = {
  creditor: 150,
  concept: 200,
  subcategory: 60,
  reference: 100,
  notes: 2000,
  reason: 500,
  search: 100,
} as const;

// ============================================================
// Filtros: estado de UI <-> DTO <-> URL
// ============================================================

/** Grupos de urgencia del filtro de vencimiento. Solo nombres, sin umbrales. */
export type DueGroup = '' | 'ok' | 'soon' | 'overdue' | 'attention';

const DUE_GROUPS: Record<Exclude<DueGroup, ''>, DebtUrgency[]> = {
  ok: ['ok'],
  soon: ['alert', 'due_today'],
  overdue: ['overdue'],
  attention: ['overdue', 'due_today', 'alert'],
};

export const DUE_GROUP_LABELS: Record<Exclude<DueGroup, ''>, string> = {
  attention: 'Requieren atención',
  soon: 'Próximas a vencer',
  overdue: 'Vencidas',
  ok: 'Con plazo',
};

export const DEBT_SORT_OPTIONS: readonly { value: DebtSortBy; label: string }[] = [
  { value: 'created_at', label: 'Fecha de registro' },
  { value: 'due_on', label: 'Vencimiento' },
  { value: 'issued_on', label: 'Fecha de emisión' },
  { value: 'amount', label: 'Importe' },
  { value: 'balance', label: 'Saldo' },
];

export const DEFAULT_DEBT_PAGE_SIZE = 20;
export const MAX_DEBT_PAGE_SIZE = 100;

export interface DebtFilterState {
  search: string;
  category: string;
  nature: string;
  status: string;
  due: DueGroup;
  issuedFrom: string;
  issuedTo: string;
  dueFrom: string;
  dueTo: string;
  paidFrom: string;
  paidTo: string;
  sortBy: DebtSortBy;
  sortDir: 'asc' | 'desc';
  page: number;
}

export const EMPTY_DEBT_FILTER: DebtFilterState = {
  search: '',
  category: '',
  nature: '',
  status: '',
  due: '',
  issuedFrom: '',
  issuedTo: '',
  dueFrom: '',
  dueTo: '',
  paidFrom: '',
  paidTo: '',
  sortBy: 'created_at',
  sortDir: 'desc',
  page: 1,
};

const CATEGORIES: readonly DebtCategory[] = [
  'maintenance', 'business_operation', 'fixed_services', 'other_obligations',
];
const NATURES: readonly DebtNature[] = ['operating', 'financial_loan', 'tax', 'other'];
const STATUSES = ['voided', 'paid', 'overdue', 'partially_paid', 'pending'] as const;
const SORTS: readonly DebtSortBy[] = DEBT_SORT_OPTIONS.map(option => option.value);

const oneOf = <T extends string>(allowed: readonly T[], value: string): T | undefined =>
  (allowed as readonly string[]).includes(value) ? (value as T) : undefined;

const clean = (value: string): string | undefined => {
  const trimmed = value.trim();
  return trimmed === '' ? undefined : trimmed;
};

const normalizePage = (page: number): number =>
  Number.isFinite(page) && page >= 1 ? Math.floor(page) : 1;

const dueGroup = (value: string): DueGroup =>
  (Object.keys(DUE_GROUPS) as Exclude<DueGroup, ''>[]).find(group => group === value) ?? '';

/** Estado de UI -> filtro del backend (snake_case, sin claves vacías). */
export const buildDebtFilter = (
  state: DebtFilterState,
  pageSize: number = DEFAULT_DEBT_PAGE_SIZE,
): DebtFilterDto => {
  const filter: DebtFilterDto = {
    sort_by: oneOf(SORTS, state.sortBy) ?? 'created_at',
    sort_dir: state.sortDir === 'asc' ? 'asc' : 'desc',
    page: normalizePage(state.page),
    page_size: Math.min(
      Math.max(Math.floor(pageSize) || DEFAULT_DEBT_PAGE_SIZE, 1),
      MAX_DEBT_PAGE_SIZE,
    ),
  };

  const search = clean(state.search);
  const category = oneOf(CATEGORIES, state.category);
  const nature = oneOf(NATURES, state.nature);
  const status = oneOf(STATUSES, state.status);
  const group = dueGroup(state.due);

  if (search) filter.search = search;
  if (category) filter.category = category;
  if (nature) filter.nature = nature;
  if (status) filter.status = status;
  if (group) filter.urgencies = [...DUE_GROUPS[group as Exclude<DueGroup, ''>]];

  const dates: [keyof DebtFilterDto, string][] = [
    ['issued_from', state.issuedFrom], ['issued_to', state.issuedTo],
    ['due_from', state.dueFrom], ['due_to', state.dueTo],
    ['paid_from', state.paidFrom], ['paid_to', state.paidTo],
  ];
  for (const [key, value] of dates) {
    const date = clean(value);
    if (date) (filter as Record<string, unknown>)[key] = date;
  }
  return filter;
};

/** El resumen y las estadísticas usan el mismo filtro, sin orden ni paginación. */
export const toSummaryFilter = (filter: DebtFilterDto): DebtFilterDto => {
  const { sort_by: _sortBy, sort_dir: _sortDir, page: _page, page_size: _pageSize, ...rest } = filter;
  return rest;
};

/** True si hay algún filtro activo (sin contar orden ni página). */
export const hasActiveDebtFilters = (state: DebtFilterState): boolean =>
  [
    state.search, state.category, state.nature, state.status, state.due,
    state.issuedFrom, state.issuedTo, state.dueFrom, state.dueTo,
    state.paidFrom, state.paidTo,
  ].some(value => value.trim() !== '');

/** Mensaje de error del primer rango inválido, o null. */
export const validateDebtFilterRanges = (state: DebtFilterState): string | null => {
  const ranges: [string, string, string][] = [
    ['emisión', state.issuedFrom, state.issuedTo],
    ['vencimiento', state.dueFrom, state.dueTo],
    ['pago', state.paidFrom, state.paidTo],
  ];
  for (const [label, from, to] of ranges) {
    const f = from.trim();
    const t = to.trim();
    if ((f && !isRealDate(f)) || (t && !isRealDate(t))) {
      return `Rango de ${label}: la fecha no es válida`;
    }
    if (f && t && f > t) {
      return `Rango de ${label}: la fecha inicial no puede ser posterior a la final`;
    }
  }
  return null;
};

const URL_DATE_KEYS: [keyof DebtFilterState, string][] = [
  ['issuedFrom', 'issued_from'], ['issuedTo', 'issued_to'],
  ['dueFrom', 'due_from'], ['dueTo', 'due_to'],
  ['paidFrom', 'paid_from'], ['paidTo', 'paid_to'],
];

/** Estado -> query string. Omite los valores por defecto. */
export const debtFilterToSearchParams = (state: DebtFilterState): URLSearchParams => {
  const params = new URLSearchParams();
  const put = (key: string, value: string) => {
    const trimmed = value.trim();
    if (trimmed) params.set(key, trimmed);
  };
  put('q', state.search);
  put('category', state.category);
  put('nature', state.nature);
  put('status', state.status);
  put('due', state.due);
  for (const [stateKey, urlKey] of URL_DATE_KEYS) put(urlKey, String(state[stateKey]));
  if (state.sortBy !== 'created_at') params.set('sort', state.sortBy);
  if (state.sortDir === 'asc') params.set('dir', 'asc');
  const page = normalizePage(state.page);
  if (page > 1) params.set('page', String(page));
  return params;
};

/** Query string -> estado. Ignora los valores inválidos. */
export const searchParamsToDebtFilter = (params: URLSearchParams): DebtFilterState => {
  const get = (key: string) => params.get(key)?.trim() ?? '';
  const date = (key: string) => (isRealDate(get(key)) ? get(key) : '');
  const page = Number(params.get('page'));
  return {
    search: get('q'),
    category: oneOf(CATEGORIES, get('category')) ?? '',
    nature: oneOf(NATURES, get('nature')) ?? '',
    status: oneOf(STATUSES, get('status')) ?? '',
    due: dueGroup(get('due')),
    issuedFrom: date('issued_from'),
    issuedTo: date('issued_to'),
    dueFrom: date('due_from'),
    dueTo: date('due_to'),
    paidFrom: date('paid_from'),
    paidTo: date('paid_to'),
    sortBy: oneOf(SORTS, get('sort')) ?? 'created_at',
    sortDir: get('dir') === 'asc' ? 'asc' : 'desc',
    page: Number.isInteger(page) && page >= 1 ? page : 1,
  };
};

// ============================================================
// Formulario de deuda
// ============================================================

export interface DebtFormValues {
  creditorName: string;
  category: DebtCategory | '';
  subcategory: string;
  nature: DebtNature | '';
  concept: string;
  amount: string;
  issuedOn: string;
  dueOn: string;
  documentRef: string;
  notes: string;
}

export const emptyDebtForm = (today: string): DebtFormValues => ({
  creditorName: '',
  category: '',
  subcategory: '',
  nature: '',
  concept: '',
  amount: '',
  issuedOn: today,
  dueOn: '',
  documentRef: '',
  notes: '',
});

export const debtToFormValues = (debt: DebtOverview, decimals: number): DebtFormValues => ({
  creditorName: debt.creditor_name,
  category: debt.category,
  subcategory: debt.subcategory ?? '',
  nature: debt.nature,
  concept: debt.concept,
  amount: minorToInput(debt.original_amount, decimals),
  issuedOn: debt.issued_on,
  dueOn: debt.due_on ?? '',
  documentRef: debt.document_ref ?? '',
  notes: debt.notes ?? '',
});

export type DebtFormErrors = Partial<Record<keyof DebtFormValues, string>>;

const textLimit = (value: string, max: number, label: string): string | undefined =>
  value.trim().length > max ? `${label} no puede superar ${max} caracteres` : undefined;

/**
 * Valida el formulario (comodidad de uso: el backend vuelve a validar todo)
 * y construye los campos del DTO. `today` es 'YYYY-MM-DD'.
 */
export const validateDebtForm = (
  values: DebtFormValues,
  context: { decimals: number; today: string },
): { errors: DebtFormErrors; fields: DebtFields | null } => {
  const errors: DebtFormErrors = {};

  const creditor = values.creditorName.trim();
  if (!creditor) errors.creditorName = 'El acreedor es obligatorio';
  else errors.creditorName = textLimit(creditor, LIMITS.creditor, 'El acreedor');

  const concept = values.concept.trim();
  if (!concept) errors.concept = 'El concepto es obligatorio';
  else errors.concept = textLimit(concept, LIMITS.concept, 'El concepto');

  if (!values.category) errors.category = 'Elige una categoría';
  if (!values.nature) errors.nature = 'Elige la naturaleza';

  errors.subcategory = textLimit(values.subcategory, LIMITS.subcategory, 'La subcategoría');
  errors.documentRef = textLimit(values.documentRef, LIMITS.reference, 'La referencia');
  errors.notes = textLimit(values.notes, LIMITS.notes, 'Las observaciones');

  const amount = parseMoneyInput(values.amount, context.decimals);
  if (amount === null || amount <= 0) {
    errors.amount = context.decimals > 0
      ? `Ingresa un importe mayor a 0 (máx. ${context.decimals} decimales)`
      : 'Ingresa un importe entero mayor a 0';
  }

  const issued = values.issuedOn.trim();
  const due = values.dueOn.trim();
  if (!isRealDate(issued)) errors.issuedOn = 'Ingresa una fecha de emisión válida';
  else if (issued > context.today) errors.issuedOn = 'La emisión no puede ser futura';
  if (due) {
    if (!isRealDate(due)) errors.dueOn = 'La fecha de vencimiento no es válida';
    else if (isRealDate(issued) && due < issued) {
      errors.dueOn = 'El vencimiento no puede ser anterior a la emisión';
    }
  }

  for (const key of Object.keys(errors) as (keyof DebtFormValues)[]) {
    if (errors[key] === undefined) delete errors[key];
  }
  if (Object.keys(errors).length > 0 || amount === null) return { errors, fields: null };

  return {
    errors,
    fields: {
      creditor_name: creditor,
      category: values.category as DebtCategory,
      subcategory: values.subcategory.trim() || null,
      nature: values.nature as DebtNature,
      concept,
      original_amount: amount,
      issued_on: issued,
      due_on: due || null,
      document_ref: values.documentRef.trim() || null,
      notes: values.notes.trim() || null,
    },
  };
};

// ============================================================
// Formulario de pago
// ============================================================

export interface PaymentFormValues {
  amount: string;
  paidOn: string;
  method: DebtPaymentMethod | '';
  reference: string;
  notes: string;
}

export const emptyPaymentForm = (today: string): PaymentFormValues => ({
  amount: '',
  paidOn: today,
  method: 'cash',
  reference: '',
  notes: '',
});

export type PaymentFormErrors = Partial<Record<keyof PaymentFormValues, string>>;

export const validatePaymentForm = (
  values: PaymentFormValues,
  context: { decimals: number; today: string; issuedOn: string; balance: number },
): { errors: PaymentFormErrors; fields: PaymentFields | null } => {
  const errors: PaymentFormErrors = {};

  const amount = parseMoneyInput(values.amount, context.decimals);
  if (amount === null || amount <= 0) {
    errors.amount = context.decimals > 0
      ? `Ingresa un importe mayor a 0 (máx. ${context.decimals} decimales)`
      : 'Ingresa un importe entero mayor a 0';
  } else if (amount > context.balance) {
    errors.amount = 'El pago supera el saldo pendiente';
  }

  const paidOn = values.paidOn.trim();
  if (!isRealDate(paidOn)) errors.paidOn = 'Ingresa una fecha de pago válida';
  else if (paidOn > context.today) errors.paidOn = 'La fecha de pago no puede ser futura';
  else if (paidOn < context.issuedOn) {
    errors.paidOn = 'La fecha de pago no puede ser anterior a la emisión de la deuda';
  }

  if (!values.method) errors.method = 'Elige el método de pago';
  errors.reference = textLimit(values.reference, LIMITS.reference, 'La referencia');
  errors.notes = textLimit(values.notes, LIMITS.notes, 'Las observaciones');

  for (const key of Object.keys(errors) as (keyof PaymentFormValues)[]) {
    if (errors[key] === undefined) delete errors[key];
  }
  if (Object.keys(errors).length > 0 || amount === null) return { errors, fields: null };

  return {
    errors,
    fields: {
      amount,
      paid_on: paidOn,
      method: values.method as DebtPaymentMethod,
      reference: values.reference.trim() || null,
      notes: values.notes.trim() || null,
    },
  };
};

/** Texto de un motivo de anulación: recortado, o null si está vacío o es muy largo. */
export const validateReason = (reason: string): string | null => {
  const text = reason.trim();
  if (!text) return 'El motivo es obligatorio';
  if (text.length > LIMITS.reason) return `El motivo no puede superar ${LIMITS.reason} caracteres`;
  return null;
};

// ============================================================
// Fechas sin hora
// ============================================================

/**
 * 'YYYY-MM-DD' -> 'DD/MM/YYYY' SIN pasar por `Date`. `formatDateShort` no
 * sirve para fechas sin hora: `new Date('2026-10-09')` es medianoche UTC y en
 * Bogotá (UTC-5) se mostraría como el 08.
 */
export const formatDay = (value: string | null | undefined): string => {
  if (!value) return '—';
  const match = /^(\d{4})-(\d{2})-(\d{2})/.exec(value);
  return match ? `${match[3]}/${match[2]}/${match[1]}` : value;
};

// ============================================================
// Reglas de presentación de acciones
// ============================================================

/** Estados con saldo por pagar (los mismos que suma el resumen del backend). */
export const ACTIVE_STATUSES: readonly DebtStatus[] = ['pending', 'partially_paid', 'overdue'];

export const isActiveDebt = (debt: Pick<DebtOverview, 'status'>): boolean =>
  ACTIVE_STATUSES.includes(debt.status);

/** Solo se abona a una deuda activa. El backend vuelve a validarlo. */
export const canPay = isActiveDebt;

/** Anular exige que no queden pagos vigentes (hay que reversarlos antes). */
export const canVoidDebt = (debt: Pick<DebtOverview, 'status' | 'payment_count'>): boolean =>
  debt.status !== 'voided' && debt.payment_count === 0;

// ============================================================
// Panel: filtro y estadísticas por categoría
// ============================================================

/**
 * Filtro del panel de totales: el del listado SIN estado ni urgencia. Así las
 * tarjetas siguen mostrando «vencido» o «próximo a vencer» aunque el listado
 * ya esté filtrado por uno de ellos.
 */
export const toPanelFilter = (filter: DebtFilterDto): DebtFilterDto => {
  const { status: _status, urgencies: _urgencies, ...rest } = toSummaryFilter(filter);
  return rest;
};

export interface CategoryRow {
  category: DebtCategory;
  activeCount: number;
  /** Saldo por pagar de las activas (incluye vencidas). */
  pendingTotal: number;
  overdueTotal: number;
  paidCount: number;
  voidedCount: number;
}

/** (categoría, estado) -> una fila por categoría, en el orden del catálogo. */
export const pivotCategoryStats = (stats: readonly DebtCategoryStat[]): CategoryRow[] =>
  CATEGORIES.map(category => {
    const rows = stats.filter(row => row.category === category);
    const active = rows.filter(row => ACTIVE_STATUSES.includes(row.status));
    return {
      category,
      activeCount: active.reduce((sum, row) => sum + row.count, 0),
      pendingTotal: active.reduce((sum, row) => sum + row.balance_total, 0),
      overdueTotal: rows
        .filter(row => row.status === 'overdue')
        .reduce((sum, row) => sum + row.balance_total, 0),
      paidCount: rows.filter(row => row.status === 'paid').reduce((sum, row) => sum + row.count, 0),
      voidedCount: rows
        .filter(row => row.status === 'voided')
        .reduce((sum, row) => sum + row.count, 0),
      total: rows.reduce((sum, row) => sum + row.count, 0),
    };
  })
    .filter(row => row.total > 0)
    .map(({ total: _total, ...row }) => row);