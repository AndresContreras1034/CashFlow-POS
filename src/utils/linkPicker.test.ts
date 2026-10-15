import { describe, expect, it } from 'vitest';
import type { LinkableVariant } from '../types/suppliers';
import {
  addAll,
  addIds,
  allSelected,
  bulkLinkMessage,
  emptySelection,
  removeAll,
  removeOne,
  searchBlockedReason,
  selectableItems,
  selectedIds,
  toggleSelection,
  variantLabel,
} from './linkPicker';

const item = (id: number, patch: Partial<LinkableVariant> = {}): LinkableVariant => ({
  variant_id: id,
  product_id: id,
  product_name: `Producto ${id}`,
  brand: null,
  attributes: {},
  sku: null,
  barcode: null,
  category_name: 'Cat',
  stock: 1,
  stock_min: 1,
  variant_cost: 0,
  linked_here: false,
  supplier_count: 0,
  ...patch,
});

describe('variantLabel', () => {
  it('agrega los atributos entre paréntesis', () => {
    expect(variantLabel({ product_name: 'Aretes', attributes: { color: 'Rojo' } })).toBe(
      'Aretes (Color: Rojo)',
    );
    expect(variantLabel({ product_name: 'Collar', attributes: {} })).toBe('Collar');
  });
});

describe('selección', () => {
  it('toggle agrega y quita sin mutar', () => {
    const base = emptySelection();
    const withOne = toggleSelection(base, 1, 'A');
    expect(base.size).toBe(0);
    expect(withOne.get(1)).toBe('A');
    expect(toggleSelection(withOne, 1, 'A').size).toBe(0);
  });

  it('addAll ignora las ya enlazadas y conserva lo seleccionado antes', () => {
    const start = toggleSelection(emptySelection(), 9, 'Previo');
    const next = addAll(start, [item(1), item(2, { linked_here: true }), item(3)]);
    expect(selectedIds(next).sort()).toEqual([1, 3, 9]);
    expect(next.get(9)).toBe('Previo');
  });

  it('addIds usa un texto genérico y no pisa labels existentes', () => {
    const start = toggleSelection(emptySelection(), 1, 'Aretes');
    const next = addIds(start, [1, 2]);
    expect(next.get(1)).toBe('Aretes');
    expect(next.get(2)).toBe('Producto #2');
  });

  it('removeAll y removeOne quitan sin mutar', () => {
    const selection = addAll(emptySelection(), [item(1), item(2), item(3)]);
    expect(selectedIds(removeAll(selection, [item(1), item(2)]))).toEqual([3]);
    expect(selectedIds(removeOne(selection, 3)).sort()).toEqual([1, 2]);
    expect(selection.size).toBe(3);
  });

  it('allSelected considera solo lo seleccionable', () => {
    const items = [item(1), item(2, { linked_here: true })];
    expect(allSelected(emptySelection(), items)).toBe(false);
    expect(allSelected(addAll(emptySelection(), items), items)).toBe(true);
    expect(allSelected(emptySelection(), [item(5, { linked_here: true })])).toBe(false);
    expect(allSelected(emptySelection(), [])).toBe(false);
    expect(selectableItems(items)).toHaveLength(1);
  });
});

describe('searchBlockedReason', () => {
  it('replica las reglas del backend', () => {
    expect(searchBlockedReason({})).toBe('Escribe al menos 2 letras o elige una categoría');
    expect(searchBlockedReason({ search: ' a ' })).toBe('Escribe al menos 2 letras');
    expect(searchBlockedReason({ search: 'sh' })).toBeNull();
    expect(searchBlockedReason({ category_id: 3 })).toBeNull();
    expect(searchBlockedReason({ only_unassigned: true })).toBeNull();
  });
});

describe('bulkLinkMessage', () => {
  it('describe cada resultado con su plural', () => {
    expect(bulkLinkMessage({ requested: 1, linked: 1, already_linked: 0 })).toBe(
      '1 producto enlazado',
    );
    expect(bulkLinkMessage({ requested: 12, linked: 12, already_linked: 0 })).toBe(
      '12 productos enlazados',
    );
    expect(bulkLinkMessage({ requested: 15, linked: 12, already_linked: 3 })).toBe(
      '12 productos enlazados · 3 ya estaban enlazados',
    );
    expect(bulkLinkMessage({ requested: 1, linked: 0, already_linked: 1 })).toBe(
      'Ese producto ya estaba enlazado',
    );
    expect(bulkLinkMessage({ requested: 4, linked: 0, already_linked: 4 })).toBe(
      'Esos 4 productos ya estaban enlazados',
    );
  });
});
