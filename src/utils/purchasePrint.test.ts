import { describe, expect, it } from 'vitest';
import type { PurchaseItem, PurchaseSupplier } from '../types/purchases';
import {
  buildPrintModel,
  describeTotals,
  printDocumentTitle,
  supplierContactLine,
} from './purchasePrint';

const supplier = (
  id: number,
  name: string,
  patch: Partial<PurchaseSupplier> = {},
): PurchaseSupplier => ({
  id,
  name,
  phone: null,
  whatsapp: null,
  email: null,
  ...patch,
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

const money = (minor: number) => `$${minor / 100}`;

describe('supplierContactLine', () => {
  it('une teléfono, WhatsApp y email', () => {
    expect(
      supplierContactLine(
        supplier(1, 'N', { phone: '6012345678', whatsapp: '3012249460', email: 'v@n.com' }),
      ),
    ).toBe('Tel. 6012345678 · WhatsApp 3012249460 · v@n.com');
  });
  it('no repite el número cuando teléfono y WhatsApp son el mismo', () => {
    expect(supplierContactLine(supplier(1, 'N', { phone: '3012249460', whatsapp: '3012249460' }))).toBe(
      'Tel./WhatsApp 3012249460',
    );
  });
  it('devuelve null sin proveedor o sin datos', () => {
    expect(supplierContactLine(null)).toBeNull();
    expect(supplierContactLine(supplier(1, 'N'))).toBeNull();
  });
});

describe('buildPrintModel', () => {
  const items = [
    item(1, {
      supplier: supplier(7, 'Zeta'),
      brand: "L'Oréal",
      attributes: { color: 'Rojo' },
      quantity: 3,
      line_cost: 3000,
      supplier_sku: 'Z-1',
    }),
    item(2, { supplier: null, quantity: 1, variant_cost: 0, line_cost: null }),
    item(3, { supplier: supplier(5, 'Alfa'), quantity: 2, line_cost: 2000, note: 'urgente' }),
  ];
  const model = buildPrintModel(
    { title: 'Ida a Bogotá', notes: null, status: 'draft' },
    items,
    { businessName: 'Mi Tienda', day: '2026-10-15' },
  );

  it('arma la cabecera con fecha local y estado en español', () => {
    expect(model).toMatchObject({
      businessName: 'Mi Tienda',
      title: 'Ida a Bogotá',
      statusLabel: 'Borrador',
      day: '15/10/2026',
    });
  });

  it('agrupa por proveedor con «Sin proveedor» al final', () => {
    expect(model.groups.map(group => group.name)).toEqual(['Alfa', 'Zeta', 'Sin proveedor']);
  });

  it('describe cada línea con marca, atributos, nota y costo de inventario', () => {
    const zeta = model.groups[1];
    expect(zeta.lines[0]).toMatchObject({
      product: 'Producto 1',
      detail: "L'Oréal · Color: Rojo",
      supplierSku: 'Z-1',
      quantity: 3,
      unitCost: 1000,
      lineCost: 3000,
    });
    expect(model.groups[0].lines[0].note).toBe('urgente');
  });

  it('deja el costo vacío si la variante no tiene costo', () => {
    const noCost = model.groups[2].lines[0];
    expect(noCost).toMatchObject({ detail: null, unitCost: null, lineCost: null });
    expect(model.groups[2].itemsWithoutCost).toBe(1);
  });

  it('calcula los totales de la lista', () => {
    expect(model.totals).toEqual({
      itemCount: 3,
      supplierCount: 2,
      units: 6,
      estimatedCost: 5000,
      itemsWithoutCost: 1,
    });
  });
});

describe('describeTotals', () => {
  it('usa singular y plural y avisa de las líneas sin costo', () => {
    expect(
      describeTotals(
        { itemCount: 1, supplierCount: 1, units: 1, estimatedCost: 10000, itemsWithoutCost: 0 },
        money,
      ),
    ).toBe('1 producto · 1 proveedor · 1 unidad · Costo estimado $100');
    expect(
      describeTotals(
        { itemCount: 3, supplierCount: 2, units: 6, estimatedCost: 5000, itemsWithoutCost: 1 },
        money,
      ),
    ).toBe('3 productos · 2 proveedores · 6 unidades · Costo estimado $50 · 1 sin costo');
  });
});

describe('printDocumentTitle', () => {
  it('compone el nombre sugerido del PDF', () => {
    expect(printDocumentTitle('Ida a Bogotá', '2026-10-15')).toBe('Ida a Bogotá - 2026-10-15');
  });
  it('quita caracteres inválidos en nombres de archivo y espacios repetidos', () => {
    expect(printDocumentTitle('  Compras: ida/vuelta?  <Bogotá> ', '2026-10-15')).toBe(
      'Compras ida vuelta Bogotá - 2026-10-15',
    );
  });
  it('usa un título genérico si queda vacío y limita la longitud', () => {
    expect(printDocumentTitle('???', '2026-10-15')).toBe('Lista de compras - 2026-10-15');
    expect(printDocumentTitle('x'.repeat(300), '2026-10-15').length).toBeLessThanOrEqual(113);
  });
});
