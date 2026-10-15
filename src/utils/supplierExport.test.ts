import { describe, expect, it } from 'vitest';
import type { RestockGroup, RestockItem, SupplierOverview } from '../types/suppliers';
import { toCsv } from './csv';
import {
  RESTOCK_CSV_HEADERS,
  SUPPLIERS_CSV_HEADERS,
  buildRestockCsv,
  buildSuppliersCsv,
  supplierExportFileName,
} from './supplierExport';
import { formatAttributes } from './format';

const supplier = (patch = {}): SupplierOverview =>
  ({
    id: 1,
    name: 'Norte',
    contact_name: 'Ana',
    document_type: 'NIT',
    document_number: '900',
    phone: '3012249460',
    whatsapp: null,
    email: null,
    address: null,
    city: 'Bogotá',
    payment_terms_days: 30,
    lead_time_days: null,
    visit_days: null,
    priority: 'urgent',
    notes: null,
    is_active: true,
    created_by: 'system',
    created_at: '2026-10-13T00:00:00Z',
    updated_at: '2026-10-13T00:00:00Z',
    category_count: 2,
    variant_count: 5,
    low_stock_count: 1,
    out_of_stock_count: 1,
    ...patch,
  }) as SupplierOverview;

const item = (patch = {}): RestockItem =>
  ({
    variant_id: 1,
    product_id: 1,
    product_name: 'Aretes',
    brand: null,
    attributes: { color: 'Rojo' },
    sku: 'A1',
    barcode: null,
    category_name: 'Bisutería',
    stock: 1,
    stock_min: 3,
    stock_status: 'low',
    units_sold_window: 10,
    days_of_cover: 3,
    urgency: 'critical',
    suggested_qty: 5,
    unit_cost: 200_000,
    line_cost: 1_000_000,
    supplier_sku: null,
    alternative_suppliers: 0,
    ...patch,
  }) as RestockItem;

const group = (supplierName: string | null, items: RestockItem[]): RestockGroup => ({
  supplier:
    supplierName === null
      ? null
      : {
          id: 7,
          name: supplierName,
          contact_name: null,
          phone: '3012249460',
          whatsapp: null,
          email: 'v@n.com',
          priority: 'normal',
          lead_time_days: null,
          payment_terms_days: null,
          visit_days: null,
        },
  items,
  item_count: items.length,
  out_count: 0,
  critical_count: 0,
  suggested_units: 0,
  estimated_cost: 0,
  items_without_cost: 0,
  top_urgency: null,
});

describe('buildSuppliersCsv', () => {
  it('genera una fila por proveedor con tantas celdas como encabezados', () => {
    const table = buildSuppliersCsv([supplier(), supplier({ id: 2, is_active: false })]);
    expect(table.rows).toHaveLength(2);
    for (const row of table.rows) expect(row).toHaveLength(SUPPLIERS_CSV_HEADERS.length);
  });

  it('traduce prioridad y estado, y deja los días ausentes como vacío', () => {
    const [row] = buildSuppliersCsv([supplier({ is_active: false })]).rows;
    expect(row).toContain('Urgente');
    expect(row).toContain('Inactivo');
    expect(row[SUPPLIERS_CSV_HEADERS.indexOf('Entrega (días)')]).toBeNull();
    expect(row[SUPPLIERS_CSV_HEADERS.indexOf('Plazo de pago (días)')]).toBe(30);
  });
});

describe('buildRestockCsv', () => {
  it('genera una fila por producto, respetando el orden de los grupos', () => {
    const table = buildRestockCsv(
      [
        group('Norte', [item(), item({ variant_id: 2, product_name: 'Collar' })]),
        group(null, [item({ variant_id: 3, product_name: 'Anillo', unit_cost: null, line_cost: null })]),
      ],
      0,
    );
    expect(table.rows).toHaveLength(3);
    for (const row of table.rows) expect(row).toHaveLength(RESTOCK_CSV_HEADERS.length);
    expect(table.rows.map(row => row[0])).toEqual(['Norte', 'Norte', 'Sin proveedor']);
  });

  it('formatea importes con los decimales de la moneda y deja vacío si no hay costo', () => {
    const costIndex = RESTOCK_CSV_HEADERS.indexOf('Costo unitario');
    const lineIndex = RESTOCK_CSV_HEADERS.indexOf('Costo de la línea');

    const [withCost] = buildRestockCsv([group('Norte', [item()])], 0).rows;
    expect(withCost[costIndex]).toBe('2000');
    expect(withCost[lineIndex]).toBe('10000');

    const [twoDecimals] = buildRestockCsv([group('Norte', [item()])], 2).rows;
    expect(twoDecimals[costIndex]).toBe('2000,00');

    const [noCost] = buildRestockCsv(
      [group('Norte', [item({ unit_cost: null, line_cost: null })])],
      0,
    ).rows;
    expect(noCost[costIndex]).toBeNull();
    expect(noCost[lineIndex]).toBeNull();
  });

  it('incluye urgencia y cobertura legibles', () => {
    const [row] = buildRestockCsv([group('Norte', [item()])], 0).rows;
    expect(row).toContain('Crítico');
    expect(row).toContain('3 días');
    expect(row).toContain('Color: Rojo');
  });

  it('sin grupos devuelve solo encabezados', () => {
    expect(buildRestockCsv([], 0).rows).toEqual([]);
  });
});

describe('CSV final', () => {
  it('se serializa con BOM, separador ; y los encabezados', () => {
    const table = buildRestockCsv([group('Norte', [item()])], 0);
    const csv = toCsv(table.headers, table.rows);
    expect(csv.startsWith('\uFEFFProveedor;Teléfono;')).toBe(true);
    expect(csv.split('\r\n')).toHaveLength(2);
  });
});

describe('formatAttributes', () => {
  it('maneja valores nulos, vacíos y mixtos sin romper la exportación', () => {
    expect(formatAttributes(null)).toBe('');
    expect(formatAttributes({ color: 'Rojo', size: null, qty: 3, notes: undefined })).toBe(
      'Color: Rojo · Qty: 3',
    );
    expect(formatAttributes({ details: { code: 'A1', weight: 2 } })).toBe(
      'Details: {"code":"A1","weight":2}',
    );
  });
});

describe('supplierExportFileName', () => {
  it('compone el nombre con tipo y fecha', () => {
    expect(supplierExportFileName('directorio', '2026-10-14')).toBe(
      'proveedores_directorio_2026-10-14.csv',
    );
    expect(supplierExportFileName('reposicion', '2026-10-14')).toBe(
      'proveedores_reposicion_2026-10-14.csv',
    );
  });
});
