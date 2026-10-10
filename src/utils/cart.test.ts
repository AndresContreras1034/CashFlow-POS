import { describe, expect, it } from 'vitest';
import type { CartLine } from './cart';
import {
  cartHasInvalidDiscount,
  cartSubtotal,
  computeCheckout,
  lineDiscountInvalid,
  lineGross,
  lineNet,
  retryFingerprint,
} from './cart';

function line(overrides: Partial<CartLine> = {}): CartLine {
  return {
    variant_id: 1,
    product_name: 'Producto',
    attributes: {},
    unit_price: 10_000,
    quantity: 1,
    discount: 0,
    stock: 10,
    ...overrides,
  };
}

describe('cart calculations', () => {
  it.each([
    ['efectivo exacto', 10_000, false, 0],
    ['efectivo con vuelto', 12_000, false, 2_000],
  ])('%s', (_name, received, shortage, change) => {
    const result = computeCheckout({
      lines: [line()],
      payments: [{ method: 'cash', amount: 10_000 }],
      isCourtesyFlag: false,
      cashReceived: received,
    });
    expect(result.cashShortage).toBe(shortage);
    expect(result.cashChange).toBe(change);
  });

  it('detects insufficient cash and null received cash', () => {
    const input = {
      lines: [line()],
      payments: [{ method: 'cash' as const, amount: 10_000 }],
      isCourtesyFlag: false,
    };
    expect(computeCheckout({ ...input, cashReceived: 9_999 })).toMatchObject({
      cashShortage: true,
      cashChange: null,
    });
    expect(computeCheckout({ ...input, cashReceived: null })).toMatchObject({
      cashShortage: false,
      cashChange: null,
    });
  });

  it('counts only cash in cashNet for mixed payments', () => {
    const result = computeCheckout({
      lines: [line()],
      payments: [
        { method: 'cash', amount: 6_000 },
        { method: 'card', amount: 4_000 },
      ],
      isCourtesyFlag: false,
      cashReceived: 8_000,
    });
    expect(result.cashNet).toBe(6_000);
    expect(result.paymentsTotal).toBe(10_000);
    expect(result.cashChange).toBe(2_000);
  });

  it('reports payment amounts that are over or under the total', () => {
    const base = {
      lines: [line()],
      isCourtesyFlag: false,
      cashReceived: null,
    };
    expect(computeCheckout({
      ...base,
      payments: [{ method: 'card', amount: 12_000 }],
    }).remaining).toBe(-2_000);
    expect(computeCheckout({
      ...base,
      payments: [{ method: 'card', amount: 8_000 }],
    }).remaining).toBe(2_000);
  });

  it('treats courtesy by flag and zero total consistently', () => {
    const flagged = computeCheckout({
      lines: [line()],
      payments: [],
      isCourtesyFlag: true,
      cashReceived: null,
    });
    expect(flagged).toMatchObject({ courtesyMode: true, total: 0, remaining: 0 });

    const zeroNet = computeCheckout({
      lines: [line({ discount: 10_000 })],
      payments: [],
      isCourtesyFlag: false,
      cashReceived: null,
    });
    expect(zeroNet).toMatchObject({ courtesyMode: true, total: 0, remaining: 0 });
  });

  it('does not treat an empty cart as courtesy', () => {
    expect(computeCheckout({
      lines: [],
      payments: [],
      isCourtesyFlag: true,
      cashReceived: null,
    })).toMatchObject({ courtesyMode: false, total: 0, remaining: 0 });
  });

  it('rejects discounts over gross but allows an exact gross discount', () => {
    const invalid = line({ discount: 10_001 });
    const exact = line({ discount: 10_000 });
    expect(lineDiscountInvalid(invalid)).toBe(true);
    expect(cartHasInvalidDiscount([invalid])).toBe(true);
    expect(lineDiscountInvalid(exact)).toBe(false);
    expect(cartSubtotal([exact])).toBe(0);
  });

  it('handles large integer amounts without losing precision', () => {
    const large = line({ unit_price: 900_000_000, quantity: 3 });
    expect(lineGross(large)).toBe(2_700_000_000);
    expect(lineNet(large)).toBe(2_700_000_000);
  });

  it('detects quantities over the shown stock', () => {
    expect(computeCheckout({
      lines: [line({ quantity: 3, stock: 2 })],
      payments: [],
      isCourtesyFlag: false,
      cashReceived: null,
    }).oversold).toBe(true);
  });

  it('builds a deterministic retry fingerprint from payload content', () => {
    const payload = { items: [{ variant_id: 1, quantity: 2 }], discount: null };
    expect(retryFingerprint(payload)).toBe(retryFingerprint(payload));
    expect(retryFingerprint(payload)).not.toBe(
      retryFingerprint({ ...payload, discount: 100 })
    );
  });
});
