import { describe, expect, it } from 'vitest';
import type { PurchaseItem } from '../types/purchases';
import type { RestockGroup, RestockItem } from '../types/suppliers';
import {
  addedMessage,
  buildSupplierOptions,
  defaultListTitle,
  groupBySupplier,
  isEditable,
  messageItems,
  parseQuantity,
  pendingRecommended,
  purchaseStatusBadgeClass,
  purchaseTotals,
  replaceItem,
  resolveQuantityCommit,
  statusActions,
  toPickable,
} from './purchaseList';

const supplier = (id: number, name: string) => ({
  id,
  name,
  phone: null,
  whatsapp: null,
  email: null,
});

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
  stock_min: 1,
  stock_status: 'out_of_stock',
  variant_cost: 1000,
  variant_price: 2000,
  quantity: 2,
  line_cost: 2000,
  source: 'manual',
  suggested_qty: null,
  note: null,
  supplier: null,
  supplier_sku: null,
  ...patch,
});

describe('parseQuantity', () => {
  it('acepta enteros de 1 a 100000', () => {
    expect(parseQuantity('1')).toBe(1);
    expect(parseQuantity(' 25 ')).toBe(25);
    expect(parseQuantity('100000')).toBe(100_000);
  });
  it('rechaza cero, negativos, decimales, texto y excesos', () => {
    for (const bad of ['0', '-1', '1.5', 'abc', '', '100001']) {
      expect(parseQuantity(bad), bad).toBeNull();
    }
  });
});

describe('estados', () => {
  it('ofrece las acciones correctas por estado', () => {
    expect(statusActions('draft').map(action => action.target)).toEqual(['ordered', 'cancelled']);
    expect(statusActions('ordered').map(action => action.target)).toEqual(['draft', 'cancelled']);
    expect(statusActions('received')).toEqual([]);
    expect(statusActions('cancelled')).toEqual([]);
  });
  it('solo el borrador se edita', () => {
    expect(isEditable('draft')).toBe(true);
    expect(isEditable('ordered')).toBe(false);
    expect(isEditable('cancelled')).toBe(false);
  });
  it('asigna clase de badge por estado', () => {
    expect(purchaseStatusBadgeClass('ordered')).toContain('suppliers-badge-preferred');
    expect(purchaseStatusBadgeClass('received')).toContain('suppliers-badge-ok');
    expect(purchaseStatusBadgeClass('cancelled')).toContain('suppliers-badge-out');
  });
});

describe('defaultListTitle', () => {
  it('compone el título con la fecha local', () => {
    expect(defaultListTitle('2026-10-14')).toBe('Compras del 14/10/2026');
  });
});

describe('groupBySupplier / purchaseTotals', () => {
  const items = [
    item(1, { supplier: supplier(7, 'Zeta'), quantity: 3, line_cost: 3000 }),
    item(2, { supplier: null, quantity: 1, line_cost: null }),
    item(3, { supplier: supplier(5, 'Alfa'), quantity: 2, line_cost: 2000 }),
    item(4, { supplier: supplier(7, 'Zeta'), quantity: 4, line_cost: null }),
  ];

  it('agrupa por proveedor, por nombre, y deja «Sin proveedor» al final', () => {
    const groups = groupBySupplier(items);
    expect(groups.map(group => group.key)).toEqual([5, 7, null]);
    expect(groups[1]).toMatchObject({
      itemCount: 2,
      units: 7,
      estimatedCost: 3000,
      itemsWithoutCost: 1,
    });
    expect(groups[2]).toMatchObject({ supplier: null, itemsWithoutCost: 1 });
  });

  it('ordena los productos de cada grupo por nombre', () => {
    const groups = groupBySupplier([
      item(10, { product_name: 'Collar', supplier: supplier(1, 'A') }),
      item(11, { product_name: 'Anillo', supplier: supplier(1, 'A') }),
    ]);
    expect(groups[0].items.map(value => value.product_name)).toEqual(['Anillo', 'Collar']);
  });

  it('suma totales sin contar dos veces al mismo proveedor', () => {
    expect(purchaseTotals(items)).toEqual({
      itemCount: 4,
      supplierCount: 2,
      units: 10,
      estimatedCost: 5000,
      itemsWithoutCost: 2,
    });
    expect(purchaseTotals([])).toEqual({
      itemCount: 0,
      supplierCount: 0,
      units: 0,
      estimatedCost: 0,
      itemsWithoutCost: 0,
    });
  });
});

describe('replaceItem', () => {
  it('reemplaza solo la línea indicada sin mutar', () => {
    const original = [item(1), item(2)];
    const next = replaceItem(original, item(2, { quantity: 9 }));
    expect(next[1].quantity).toBe(9);
    expect(next[0]).toBe(original[0]);
    expect(original[1].quantity).toBe(2);
  });
});

describe('pendingRecommended', () => {
  const restock = (variantId: number, urgency: RestockItem['urgency']) =>
    ({ variant_id: variantId, urgency }) as RestockItem;
  const group = (id: number | null, items: RestockItem[]) =>
    ({
      supplier: id === null ? null : { id, name: `Prov ${id}` },
      items,
    }) as unknown as RestockGroup;

  const overview = {
    groups: [
      group(1, [restock(1, 'out_of_stock'), restock(2, 'low')]),
      group(2, [restock(3, 'critical')]),
      group(null, [restock(4, 'low')]),
    ],
  };

  it('cuenta lo que falta por agregar y cuánto es urgente', () => {
    expect(pendingRecommended(overview, new Set())).toEqual({
      total: 4,
      urgent: 2,
      bySupplier: [
        { supplierId: 1, name: 'Prov 1', count: 2, urgent: 1 },
        { supplierId: 2, name: 'Prov 2', count: 1, urgent: 1 },
        { supplierId: null, name: 'Sin proveedor', count: 1, urgent: 0 },
      ],
    });
  });

  it('excluye lo que ya está en la lista y omite grupos vacíos', () => {
    const result = pendingRecommended(overview, new Set([3, 4]));
    expect(result.total).toBe(2);
    expect(result.bySupplier.map(value => value.supplierId)).toEqual([1]);
    expect(pendingRecommended(overview, new Set([1, 2, 3, 4]))).toEqual({
      total: 0,
      urgent: 0,
      bySupplier: [],
    });
  });
});

describe('messageItems', () => {
  it('usa la cantidad de la línea, no la sugerida', () => {
    const [first] = messageItems({
      items: [item(1, { quantity: 6, suggested_qty: 2, attributes: { color: 'Rojo' } })],
    });
    expect(first).toEqual({
      product_name: 'Producto 1',
      attributes: { color: 'Rojo' },
      suggested_qty: 6,
    });
  });
});

describe('resolveQuantityCommit', () => {
  it('distingue inválido, sin cambios y guardar', () => {
    expect(resolveQuantityCommit('abc', 5)).toEqual({ kind: 'invalid' });
    expect(resolveQuantityCommit('0', 5)).toEqual({ kind: 'invalid' });
    expect(resolveQuantityCommit(' 5 ', 5)).toEqual({ kind: 'unchanged' });
    expect(resolveQuantityCommit('8', 5)).toEqual({ kind: 'save', value: 8 });
  });
});

describe('buildSupplierOptions', () => {
  const suppliers = [
    { id: 2, name: 'Zeta', is_active: true },
    { id: 1, name: 'Alfa', is_active: true },
    { id: 3, name: 'Cerrado', is_active: false },
  ];
  it('lista solo los activos, por nombre', () => {
    expect(buildSupplierOptions(null, suppliers).map(option => option.id)).toEqual([1, 2]);
  });
  it('conserva el proveedor actual aunque esté desactivado', () => {
    const options = buildSupplierOptions(supplier(3, 'Cerrado'), suppliers);
    expect(options[0]).toEqual({ id: 3, name: 'Cerrado', inactive: true });
    expect(options.map(option => option.id)).toEqual([3, 1, 2]);
  });
  it('no duplica al actual si ya es activo', () => {
    expect(buildSupplierOptions(supplier(1, 'Alfa'), suppliers).map(option => option.id))
      .toEqual([1, 2]);
  });
});

describe('toPickable', () => {
  it('usa «ya en la lista» como «ya enlazado»', () => {
    const picked = toPickable({
      variant_id: 1, product_id: 1, product_name: 'Aretes', brand: null, attributes: {},
      sku: null, barcode: null, category_name: 'Cat', stock: 1, stock_min: 1,
      variant_cost: 0, in_list: true,
    });
    expect(picked.linked_here).toBe(true);
    expect(picked.supplier_count).toBe(0);
  });
});

describe('addedMessage', () => {
  it('muestra los agregados y los que ya estaban en la lista', () => {
    expect(addedMessage({ requested: 1, added: 1, already_in_list: 0 }))
      .toBe('1 producto agregado');
    expect(addedMessage({ requested: 4, added: 3, already_in_list: 1 }))
      .toBe('3 productos agregados · 1 ya estaban en la lista');
    expect(addedMessage({ requested: 1, added: 0, already_in_list: 1 }))
      .toBe('Ese producto ya estaba en la lista');
    expect(addedMessage({ requested: 2, added: 0, already_in_list: 2 }))
      .toBe('Esos 2 productos ya estaban en la lista');
    expect(addedMessage({ requested: 0, added: 0, already_in_list: 0 }))
      .toBe('No había productos para agregar');
  });
});
