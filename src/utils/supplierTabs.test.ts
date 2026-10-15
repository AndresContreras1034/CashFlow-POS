import { describe, expect, it } from 'vitest';
import {
  emptyLinkForm,
  formatCover,
  linkToFormValues,
  restockUrgencyBadgeClass,
  sameIdSet,
  toggleId,
  validateLinkForm,
} from './supplierTabs';

describe('validateLinkForm', () => {
  it('acepta el formulario vacío: todo es opcional', () => {
    const { fields, errors } = validateLinkForm(emptyLinkForm());
    expect(errors).toEqual({});
    expect(fields).toEqual({
      supplier_sku: null,
      is_preferred: false,
      notes: null,
    });
  });

  it('recorta textos y valida longitudes', () => {
    const ok = validateLinkForm({ ...emptyLinkForm(), supplierSku: '  P-1  ', notes: ' x ' });
    expect(ok.fields).toMatchObject({ supplier_sku: 'P-1', notes: 'x' });
    expect(validateLinkForm({ ...emptyLinkForm(), supplierSku: 's'.repeat(101) }).errors.supplierSku).toBeDefined();
    expect(validateLinkForm({ ...emptyLinkForm(), notes: 'n'.repeat(2001) }).errors.notes).toBeDefined();
  });
});

describe('linkToFormValues', () => {
  it('hace ida y vuelta sin perder datos', () => {
    const link = { supplier_sku: 'P-9', is_preferred: true, notes: 'Pedir lunes' };
    const { fields } = validateLinkForm(linkToFormValues(link));
    expect(fields).toEqual({ supplier_sku: 'P-9', is_preferred: true, notes: 'Pedir lunes' });
  });

  it('muestra vacío cuando no hay SKU', () => {
    const values = linkToFormValues({ supplier_sku: null, is_preferred: false, notes: null });
    expect(values).toEqual(emptyLinkForm());
  });
});

describe('selección de categorías', () => {
  it('toggleId agrega y quita sin mutar', () => {
    const original = [1, 2];
    expect(toggleId(original, 3)).toEqual([1, 2, 3]);
    expect(toggleId(original, 1)).toEqual([2]);
    expect(original).toEqual([1, 2]);
  });

  it('sameIdSet ignora orden y repetidos', () => {
    expect(sameIdSet([1, 2, 3], [3, 2, 1])).toBe(true);
    expect(sameIdSet([1, 1, 2], [2, 1])).toBe(true);
    expect(sameIdSet([1, 2], [1, 3])).toBe(false);
    expect(sameIdSet([], [])).toBe(true);
    expect(sameIdSet([1], [])).toBe(false);
  });
});

describe('presentación de reposición', () => {
  it('asigna clase de badge por urgencia', () => {
    expect(restockUrgencyBadgeClass('out_of_stock')).toContain('suppliers-badge-out');
    expect(restockUrgencyBadgeClass('critical')).toContain('suppliers-badge-urgent');
    expect(restockUrgencyBadgeClass('low')).toContain('suppliers-badge-low');
  });

  it('formatea la cobertura', () => {
    expect(formatCover(null)).toBe('Sin ventas');
    expect(formatCover(0)).toBe('Menos de 1 día');
    expect(formatCover(1)).toBe('1 día');
    expect(formatCover(12)).toBe('12 días');
  });
});
