/** Moneda por defecto hasta que Ajustes la provea vía contexto (Bloque 4). */
export const MONEY_DEFAULTS = { currency: 'COP', decimals: 0 } as const;

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
export const formatAttributes = (attrs: Record<string, string>): string => {
  return Object.entries(attrs)
    .map(([k, v]) => `${capitalize(k)}: ${v}`)
    .join(' · ');
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