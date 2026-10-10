import { beforeEach, describe, expect, it } from 'vitest';
import type { CartLine } from './cart';
import {
  addHeld,
  HELD_SALES_KEY,
  loadHeld,
  MAX_HELD_SALES,
  pruneHeld,
  removeHeld,
  saveHeld,
  type HeldSale,
  type StorageLike,
} from './heldSales';

class MemoryStorage implements StorageLike {
  private values = new Map<string, string>();
  failWrites = false;

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    if (this.failWrites) throw new Error('storage blocked');
    this.values.set(key, value);
  }
}

const storage = new MemoryStorage();

function line(overrides: Partial<CartLine> = {}): CartLine {
  return {
    variant_id: 10,
    product_name: 'Producto',
    attributes: { color: 'Azul' },
    unit_price: 12_000,
    quantity: 2,
    discount: 1_000,
    stock: 8,
    ...overrides,
  };
}

function held(overrides: Partial<HeldSale> = {}): HeldSale {
  return {
    id: crypto.randomUUID(),
    session_id: 5,
    saved_at: '2026-10-10T10:00:00.000Z',
    is_courtesy: false,
    courtesy_reason: '',
    lines: [line()],
    ...overrides,
  };
}

function entry(sessionId = 5) {
  return {
    session_id: sessionId,
    is_courtesy: false,
    courtesy_reason: '',
    lines: [line()],
  };
}

beforeEach(() => {
  storage.failWrites = false;
  storage.setItem(HELD_SALES_KEY, '[]');
});

describe('held sales storage', () => {
  it('returns empty list for invalid JSON, invalid roots, or a missing key', () => {
    expect(loadHeld(storage)).toEqual([]);
    storage.setItem(HELD_SALES_KEY, '{bad json');
    expect(loadHeld(storage)).toEqual([]);
    storage.setItem(HELD_SALES_KEY, JSON.stringify({ data: [] }));
    expect(loadHeld(storage)).toEqual([]);

    const missing = new MemoryStorage();
    expect(loadHeld(missing)).toEqual([]);
  });

  it('discards malformed entries while keeping valid entries', () => {
    const valid = held();
    const invalidId = { ...held(), id: 42 };
    const invalidDiscountLine = held({
      lines: [line({ quantity: 0 })],
    });
    const invalidAttributes = {
      ...held(),
      lines: [{ ...line(), attributes: { size: 2 } }],
    };
    storage.setItem(
      HELD_SALES_KEY,
      JSON.stringify([invalidId, invalidDiscountLine, invalidAttributes, valid])
    );

    expect(loadHeld(storage)).toEqual([valid]);
  });

  it('keeps at most five entries and rejects a sixth with a clear message', () => {
    let result;
    for (let index = 0; index < MAX_HELD_SALES; index += 1) {
      result = addHeld(storage, entry());
      expect(result.ok).toBe(true);
    }
    const sixth = addHeld(storage, entry());
    expect(sixth).toMatchObject({ ok: false, error: 'Máximo 5 ventas en espera' });
    expect(loadHeld(storage)).toHaveLength(MAX_HELD_SALES);
  });

  it('rejects an empty cart', () => {
    expect(addHeld(storage, { ...entry(), lines: [] })).toMatchObject({
      ok: false,
      error: 'No se puede guardar un carrito vacío',
    });
  });

  it('returns a storage error instead of throwing when setItem fails', () => {
    storage.failWrites = true;
    expect(saveHeld(storage, [held()])).toMatchObject({
      ok: false,
      error: expect.stringContaining('No se pudieron guardar'),
    });
    expect(addHeld(storage, entry()).ok).toBe(false);
  });

  it('prunes entries from other sessions and all entries when no session is open', () => {
    const entries = [held({ session_id: 5 }), held({ session_id: 6 })];
    saveHeld(storage, entries);
    expect(pruneHeld(storage, 5)).toEqual([entries[0]]);
    expect(loadHeld(storage)).toEqual([entries[0]]);
    expect(pruneHeld(storage, null)).toEqual([]);
    expect(loadHeld(storage)).toEqual([]);
  });

  it('round-trips a saved cart through addHeld and loadHeld', () => {
    const result = addHeld(storage, {
      session_id: 8,
      is_courtesy: true,
      courtesy_reason: 'Apoyo',
      lines: [line()],
    });

    expect(result.ok).toBe(true);
    expect(result.entry?.id).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
    );
    expect(result.entry?.saved_at).toBeTruthy();
    expect(loadHeld(storage)).toEqual([result.entry]);
  });

  it('removes a selected held sale', () => {
    const existing = held();
    saveHeld(storage, [existing]);
    expect(removeHeld(storage, existing.id)).toMatchObject({ ok: true, list: [] });
    expect(loadHeld(storage)).toEqual([]);
  });
});
