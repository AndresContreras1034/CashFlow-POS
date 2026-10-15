let moneyConfig = { currency: 'COP', decimals: 0 };

/** Update the active money formatting config when application settings load. */
export const setMoneyConfig = (config: { currency: string; decimals: number }): void => {
  moneyConfig = { currency: config.currency, decimals: config.decimals };
};

/** Current currency settings provided by the application context. */
export const MONEY_DEFAULTS = {
  get currency(): string { return moneyConfig.currency; },
  get decimals(): number { return moneyConfig.decimals; },
};

const clampDecimals = (decimals: number): number => Math.min(Math.max(decimals, 0), 2);

/** Centésimas -> importe formateado. Todo importe se guarda en centésimas. */
export const formatMoney = (
  minor: number,
  currency: string = MONEY_DEFAULTS.currency,
  decimals: number = MONEY_DEFAULTS.decimals,
): string => {
  const precision = clampDecimals(decimals);
  return new Intl.NumberFormat('es-CO', {
    style: 'currency',
    currency,
    minimumFractionDigits: precision,
    maximumFractionDigits: precision,
  }).format(minor / 100);
};

/**
 * Texto tecleado por el usuario en unidades principales -> centésimas.
 * Rechaza negativos, texto y precisión superior a la moneda.
 */
export const parseMoneyInput = (
  raw: string,
  decimals: number = MONEY_DEFAULTS.decimals,
): number | null => {
  const text = raw.trim();
  const precision = clampDecimals(decimals);
  const pattern = precision > 0
    ? new RegExp(`^\\d+(\\.\\d{1,${precision}})?$`)
    : /^\d+$/;
  if (!pattern.test(text)) return null;

  const [whole, fraction = ''] = text.split('.');
  const minor = Number(whole) * 100 + Number(fraction.padEnd(2, '0'));
  return Number.isSafeInteger(minor) ? minor : null;
};

/** Centésimas -> texto en unidades principales para un input. */
export const minorToInput = (
  minor: number,
  decimals: number = MONEY_DEFAULTS.decimals,
): string => (minor / 100).toFixed(clampDecimals(decimals));

/** Valor step de un input de dinero. */
export const moneyStep = (decimals: number = MONEY_DEFAULTS.decimals): string =>
  decimals > 0 ? '0.01' : '1';

export type MarginResult =
  | { kind: 'ok'; percent: number }
  | { kind: 'no_cost' }
  | { kind: 'no_price' };

/** Margen sobre precio: (price - cost) / price × 100. Importes en centavos. */
export function calcMargin(price: number, cost: number): MarginResult {
  if (price <= 0) return { kind: 'no_price' };
  if (cost <= 0) return { kind: 'no_cost' };
  const percent = Math.round(((price - cost) / price) * 1000) / 10;
  return { kind: 'ok', percent };
}

export function formatMargin(margin: MarginResult): string {
  switch (margin.kind) {
    case 'ok': return `${margin.percent}%`;
    case 'no_cost': return 'Sin costo';
    case 'no_price': return 'Sin precio';
  }
}

/** Fecha ISO → "12 jun 2026, 8:42 p. m." */
export const formatDate = (iso: string): string => {
  return new Intl.DateTimeFormat('es-CO', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(iso));
};

/** Fecha ISO → "12/06/2026" */
export const formatDateShort = (iso: string): string => {
  return new Intl.DateTimeFormat('es-CO', {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
  }).format(new Date(iso));
};

/** Atributos JSON → "Talla: M · Color: Rojo" */
export const formatAttributes = (attrs: unknown): string => {
  if (attrs == null) return '';

  if (typeof attrs === 'string') return attrs.trim();

  if (Array.isArray(attrs)) {
    return attrs
      .map(value => formatAttributes(value))
      .filter(Boolean)
      .join(' · ');
  }

  if (typeof attrs !== 'object') {
    return String(attrs);
  }

  return Object.entries(attrs as Record<string, unknown>)
    .filter(([, value]) => value !== null && value !== undefined)
    .map(([key, value]) => `${capitalize(key)}: ${formatAttributeValue(value)}`)
    .filter(part => part !== '' && !part.endsWith(': '))
    .join(' · ');
};

const formatAttributeValue = (value: unknown): string => {
  if (value === null || value === undefined) return '';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean' || typeof value === 'bigint') {
    return String(value);
  }
  if (Array.isArray(value)) {
    return value.map(item => formatAttributeValue(item)).filter(Boolean).join(', ');
  }
  if (typeof value === 'object') {
    try {
      return JSON.stringify(value);
    } catch {
      return Object.prototype.toString.call(value);
    }
  }
  return String(value);
};

/** Tipo de movimiento → etiqueta legible */
export const formatMovementType = (type: string): string => {
  const labels: Record<string, string> = {
    purchase:      'Compra',
    sale:          'Venta',
    sale_return:   'Devolución',
    manual_in:     'Entrada manual',
    manual_out:    'Salida manual',
    adjustment:    'Ajuste',
    initial_stock: 'Stock inicial',
  };
  return labels[type] ?? type;
};

const capitalize = (s: string): string =>
  s.charAt(0).toUpperCase() + s.slice(1);