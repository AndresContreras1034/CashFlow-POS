import type { CartLine } from './cart';
import { cartSubtotal, isCourtesyMode } from './cart';

export const HELD_SALES_KEY = 'pos.heldCarts.v1';
export const MAX_HELD_SALES = 5;

export type StorageLike = Pick<Storage, 'getItem' | 'setItem'>;

export interface HeldSale {
  id: string;
  session_id: number;
  saved_at: string;
  is_courtesy: boolean;
  courtesy_reason: string;
  lines: CartLine[];
}

export interface HeldResult {
  ok: boolean;
  error?: string;
  list?: HeldSale[];
  entry?: HeldSale;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value);
}

function isCartLine(value: unknown): value is CartLine {
  if (!isRecord(value)) return false;
  if (!isInteger(value.variant_id) || value.variant_id < 1) return false;
  if (typeof value.product_name !== 'string') return false;
  if (!isRecord(value.attributes)) return false;
  if (!Object.values(value.attributes).every((attribute) => typeof attribute === 'string')) {
    return false;
  }
  return (
    isInteger(value.quantity) &&
    value.quantity >= 1 &&
    isInteger(value.unit_price) &&
    value.unit_price >= 0 &&
    isInteger(value.discount) &&
    value.discount >= 0 &&
    isInteger(value.stock)
  );
}

function isHeldSale(value: unknown): value is HeldSale {
  if (!isRecord(value)) return false;
  const savedAt = value.saved_at;
  return (
    typeof value.id === 'string' &&
    value.id.length > 0 &&
    isInteger(value.session_id) &&
    value.session_id > 0 &&
    typeof savedAt === 'string' &&
    /^\d{4}-\d{2}-\d{2}T/.test(savedAt) &&
    Number.isFinite(Date.parse(savedAt)) &&
    typeof value.is_courtesy === 'boolean' &&
    typeof value.courtesy_reason === 'string' &&
    Array.isArray(value.lines) &&
    value.lines.length > 0 &&
    value.lines.every(isCartLine)
  );
}

export function loadHeld(storage: StorageLike): HeldSale[] {
  try {
    const raw = storage.getItem(HELD_SALES_KEY);
    if (raw === null) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(isHeldSale).slice(0, MAX_HELD_SALES);
  } catch {
    return [];
  }
}

export function saveHeld(
  storage: StorageLike,
  list: HeldSale[]
): { ok: boolean; error?: string } {
  try {
    storage.setItem(HELD_SALES_KEY, JSON.stringify(list.slice(0, MAX_HELD_SALES)));
    return { ok: true };
  } catch (error) {
    return {
      ok: false,
      error: `No se pudieron guardar las ventas en espera: ${String(error)}`,
    };
  }
}

export function addHeld(
  storage: StorageLike,
  entry: Omit<HeldSale, 'id' | 'saved_at'>
): HeldResult {
  const list = loadHeld(storage);
  if (list.length >= MAX_HELD_SALES) {
    return { ok: false, error: 'Máximo 5 ventas en espera' };
  }
  if (entry.lines.length === 0) {
    return { ok: false, error: 'No se puede guardar un carrito vacío' };
  }

  const heldEntry: HeldSale = {
    ...entry,
    id: crypto.randomUUID(),
    saved_at: new Date().toISOString(),
  };
  const next = [...list, heldEntry];
  const result = saveHeld(storage, next);
  return result.ok
    ? { ok: true, list: next, entry: heldEntry }
    : { ok: false, error: result.error };
}

export function removeHeld(storage: StorageLike, id: string): HeldResult {
  const next = loadHeld(storage).filter((entry) => entry.id !== id);
  const result = saveHeld(storage, next);
  return result.ok
    ? { ok: true, list: next }
    : { ok: false, error: result.error };
}

export function pruneHeld(storage: StorageLike, currentSessionId: number | null): HeldSale[] {
  const list = loadHeld(storage);
  const next =
    currentSessionId === null
      ? []
      : list.filter((entry) => entry.session_id === currentSessionId);
  if (next.length !== list.length) {
    const result = saveHeld(storage, next);
    if (!result.ok) throw new Error(result.error);
  }
  return next;
}

export function heldTotal(entry: HeldSale): number {
  const subtotal = cartSubtotal(entry.lines);
  return isCourtesyMode(entry.lines.length, entry.is_courtesy, subtotal) ? 0 : subtotal;
}
