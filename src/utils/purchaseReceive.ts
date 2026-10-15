import type { PurchaseItem, ReceiveLineDto, ReceiveResult } from '../types/purchases';
import { minorToInput, parseMoneyInput } from './format';
import { MAX_QUANTITY } from './purchaseList';

export interface ReceiveRow {
  itemId: number;
  quantity: string;
  unitCost: string;
}

export interface ReceiveRowErrors {
  quantity?: string;
  unitCost?: string;
}

export const initialReceiveRows = (
  items: readonly PurchaseItem[],
  decimals: number,
): ReceiveRow[] => items.map(item => ({
  itemId: item.item_id,
  quantity: String(item.quantity),
  unitCost: item.variant_cost > 0 ? minorToInput(item.variant_cost, decimals) : '',
}));

export interface ReceiveValidation {
  errors: Record<number, ReceiveRowErrors>;
  formError: string | null;
  lines: ReceiveLineDto[] | null;
  receivedLines: number;
  skippedLines: number;
  units: number;
  estimatedTotal: number;
}

export const parseReceivedQuantity = (raw: string): number | null => {
  const text = raw.trim();
  if (!/^\d+$/.test(text)) return null;
  const value = Number(text);
  return value <= MAX_QUANTITY ? value : null;
};

export const validateReceive = (
  rows: readonly ReceiveRow[],
  decimals: number,
): ReceiveValidation => {
  const errors: Record<number, ReceiveRowErrors> = {};
  const lines: ReceiveLineDto[] = [];
  let receivedLines = 0;
  let skippedLines = 0;
  let units = 0;
  let estimatedTotal = 0;

  for (const row of rows) {
    const quantity = parseReceivedQuantity(row.quantity);
    if (quantity === null) {
      errors[row.itemId] = {
        quantity: `Ingresa un entero de 0 a ${MAX_QUANTITY} (0 = no llegó)`,
      };
      continue;
    }
    if (quantity === 0) {
      skippedLines += 1;
      lines.push({ item_id: row.itemId, quantity: 0, unit_cost: null });
      continue;
    }

    let unitCost: number | null = null;
    const rawCost = row.unitCost.trim();
    if (rawCost !== '') {
      const parsed = parseMoneyInput(rawCost, decimals);
      if (parsed === null || parsed <= 0) {
        errors[row.itemId] = {
          unitCost: decimals > 0
            ? `Costo mayor a 0 (máx. ${decimals} decimales) o vacío`
            : 'Costo entero mayor a 0 o vacío',
        };
        continue;
      }
      unitCost = parsed;
      estimatedTotal += quantity * parsed;
    }

    receivedLines += 1;
    units += quantity;
    lines.push({ item_id: row.itemId, quantity, unit_cost: unitCost });
  }

  const hasErrors = Object.keys(errors).length > 0;
  const formError = !hasErrors && receivedLines === 0
    ? 'Indica la cantidad recibida de al menos un producto'
    : null;

  return {
    errors,
    formError,
    lines: hasErrors || formError ? null : lines,
    receivedLines,
    skippedLines,
    units,
    estimatedTotal,
  };
};

const noun = (n: number, one: string, many: string): string => `${n} ${n === 1 ? one : many}`;

export const receiveMessage = (
  result: Pick<ReceiveResult, 'received_lines' | 'skipped_lines' | 'units' | 'cost_changes'>,
): string => {
  const parts = [
    `Compra recibida: ${noun(result.received_lines, 'producto', 'productos')}`,
    noun(result.units, 'unidad', 'unidades'),
  ];
  if (result.cost_changes.length > 0) {
    parts.push(noun(result.cost_changes.length, 'costo actualizado', 'costos actualizados'));
  }
  if (result.skipped_lines > 0) {
    parts.push(noun(result.skipped_lines, 'línea sin recibir', 'líneas sin recibir'));
  }
  return parts.join(' · ');
};
