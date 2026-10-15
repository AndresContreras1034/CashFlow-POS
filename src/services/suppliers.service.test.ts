import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

import * as service from './suppliers.service';

describe('contrato con los comandos de Tauri', () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue('respuesta');
  });

  const dto = { updated_by: 'Ana' };

  const cases: [string, () => Promise<unknown>, string, unknown][] = [
    ['listSuppliers', () => service.listSuppliers({ page: 2 }), 'list_suppliers', { filter: { page: 2 } }],
    ['listSuppliers sin filtro', () => service.listSuppliers(), 'list_suppliers', { filter: {} }],
    ['getSupplier', () => service.getSupplier(7), 'get_supplier', { id: 7 }],
    [
      'getSupplierPayables',
      () => service.getSupplierPayables(7),
      'get_supplier_payables',
      { supplierId: 7 },
    ],
    ['createSupplier', () => service.createSupplier({} as never), 'create_supplier', { dto: {} }],
    ['updateSupplier', () => service.updateSupplier(3, {} as never), 'update_supplier', { id: 3, dto: {} }],
    ['setSupplierActive', () => service.setSupplierActive(3, { is_active: false }), 'set_supplier_active', { id: 3, dto: { is_active: false } }],
    [
      'setSupplierCategories',
      () => service.setSupplierCategories(4, [1, 2], 'Ana'),
      'set_supplier_categories',
      { supplierId: 4, dto: { category_ids: [1, 2], updated_by: 'Ana' } },
    ],
    ['listSupplierCategories', () => service.listSupplierCategories(4), 'list_supplier_categories', { supplierId: 4 }],
    [
      'linkSupplierVariant',
      () => service.linkSupplierVariant(4, { variant_id: 9, is_preferred: true }),
      'link_supplier_variant',
      { supplierId: 4, dto: { variant_id: 9, is_preferred: true } },
    ],
    [
      'updateSupplierVariant',
      () => service.updateSupplierVariant(11, { is_preferred: false, ...dto }),
      'update_supplier_variant',
      { linkId: 11, dto: { is_preferred: false, updated_by: 'Ana' } },
    ],
    [
      'unlinkSupplierVariant',
      () => service.unlinkSupplierVariant(11, dto),
      'unlink_supplier_variant',
      { linkId: 11, dto },
    ],
    [
      'listSupplierVariants',
      () => service.listSupplierVariants(4, { search: 'aceite' }),
      'list_supplier_variants',
      { supplierId: 4, filter: { search: 'aceite' } },
    ],
    [
      'listSupplierVariants sin filtro',
      () => service.listSupplierVariants(4),
      'list_supplier_variants',
      { supplierId: 4, filter: {} },
    ],
    [
      'linkSupplierVariants',
      () => service.linkSupplierVariants(4, { variant_ids: [9, 10], updated_by: 'Ana' }),
      'link_supplier_variants',
      { supplierId: 4, dto: { variant_ids: [9, 10], updated_by: 'Ana' } },
    ],
    [
      'searchLinkableVariants',
      () => service.searchLinkableVariants(4, { search: 'aceite', only_unassigned: true }),
      'search_linkable_variants',
      { supplierId: 4, filter: { search: 'aceite', only_unassigned: true } },
    ],
    [
      'listLinkableVariantIds',
      () => service.listLinkableVariantIds(4, { category_id: 7 }),
      'list_linkable_variant_ids',
      { supplierId: 4, filter: { category_id: 7 } },
    ],
    ['listVariantSuppliers', () => service.listVariantSuppliers(9), 'list_variant_suppliers', { variantId: 9 }],
    ['getRestockOverview', () => service.getRestockOverview({ supplier_id: 4 }), 'get_restock_overview', { filter: { supplier_id: 4 } }],
    [
      'getSupplierRestock',
      () => service.getSupplierRestock(4, 30, 14),
      'get_supplier_restock',
      { supplierId: 4, windowDays: 30, coverDays: 14 },
    ],
  ];

  it.each(cases)('%s', async (_name, call, command, args) => {
    await call();
    expect(invoke).toHaveBeenCalledWith(command, args);
  });
});
