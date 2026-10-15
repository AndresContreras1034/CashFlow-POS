import { describe, expect, it } from 'vitest';
import type { PurchaseItem } from '../types/purchases';
import {
  initialReceiveRows,
  parseReceivedQuantity,
  receiveMessage,
  validateReceive,
} from './purchaseReceive';

const item = (id: number, patch: Partial<PurchaseItem> = {}): PurchaseItem => ({
  item_id: id,
  variant_id: id,
  product_id: id,
  product_name: `Producto ${id}`,
  brand: null,
  attributes: {},
  sku: null,
  barcode: null,
  category_name: 'Cat',
  stock: 0,
  stock_min: 0,
  stock_status: 'out_of_stock',
  variant_cost: 150_000,
  variant_price: 0,
  quantity: 4,
  line_cost: null,
  source: 'manual',
  suggested_qty: null,
  note: null,
  supplier: null,
  supplier_sku: null,
  ...patch,
});

describe('initialReceiveRows', () => {
  it('parte de la cantidad pedida y del costo del inventario', () => {
    expect(initialReceiveRows([item(1), item(2, { variant_cost: 0 })], 0)).toEqual([
      { itemId: 1, quantity: '4', unitCost: '1500' },
      { itemId: 2, quantity: '4', unitCost: '' },
    ]);
  });
});

describe('parseReceivedQuantity', () => {
  it('acepta de 0 a 100000', () => {
    expect(parseReceivedQuantity('0')).toBe(0);
    expect(parseReceivedQuantity(' 12 ')).toBe(12);
    expect(parseReceivedQuantity('100000')).toBe(100_000);
  });
  it('rechaza negativos, decimales, texto y excesos', () => {
    for (const bad of ['-1', '1.5', 'abc', '', '100001']) {
      expect(parseReceivedQuantity(bad), bad).toBeNull();
    }
  });
});

describe('validateReceive', () => {
  it('arma líneas: cantidad 0 no lleva costo y el costo vacío se envía como null', () => {
    const result = validateReceive([
      { itemId: 1, quantity: '8', unitCost: '1500' },
      { itemId: 2, quantity: '0', unitCost: 'basura' },
      { itemId: 3, quantity: '2', unitCost: '' },
    ], 0);
    expect(result.formError).toBeNull();
    expect(result.lines).toEqual([
      { item_id: 1, quantity: 8, unit_cost: 150_000 },
      { item_id: 2, quantity: 0, unit_cost: null },
      { item_id: 3, quantity: 2, unit_cost: null },
    ]);
    expect(result).toMatchObject({
      receivedLines: 2,
      skippedLines: 1,
      units: 10,
      estimatedTotal: 1_200_000,
    });
  });

  it('reporta el error en la fila correcta y no devuelve líneas', () => {
    const result = validateReceive([
      { itemId: 1, quantity: 'x', unitCost: '' },
      { itemId: 2, quantity: '3', unitCost: '0' },
      { itemId: 3, quantity: '3', unitCost: '1500' },
    ], 0);
    expect(result.lines).toBeNull();
    expect(result.errors[1].quantity).toBeDefined();
    expect(result.errors[2].unitCost).toBeDefined();
    expect(result.errors[3]).toBeUndefined();
  });

  it('rechaza decimales que la moneda no admite', () => {
    expect(validateReceive([{ itemId: 1, quantity: '1', unitCost: '15.5' }], 0).errors[1].unitCost)
      .toBeDefined();
    expect(validateReceive([{ itemId: 1, quantity: '1', unitCost: '15.5' }], 2).lines).toEqual([
      { item_id: 1, quantity: 1, unit_cost: 1_550 },
    ]);
  });

  it('exige que al menos un producto haya llegado', () => {
    const result = validateReceive([
      { itemId: 1, quantity: '0', unitCost: '' },
      { itemId: 2, quantity: '0', unitCost: '' },
    ], 0);
    expect(result.formError).toBe('Indica la cantidad recibida de al menos un producto');
    expect(result.lines).toBeNull();
  });
});

describe('receiveMessage', () => {
  it('resume lo recibido con sus plurales', () => {
    expect(receiveMessage({
      received_lines: 1,
      skipped_lines: 0,
      units: 1,
      cost_changes: [],
    })).toBe('Compra recibida: 1 producto · 1 unidad');
    expect(receiveMessage({
      received_lines: 3,
      skipped_lines: 2,
      units: 12,
      cost_changes: [
        { variant_id: 1, product_name: 'A', from: 1, to: 2 },
        { variant_id: 2, product_name: 'B', from: 1, to: 2 },
      ],
    })).toBe(
      'Compra recibida: 3 productos · 12 unidades · 2 costos actualizados · 2 líneas sin recibir',
    );
  });
});
