import type { RestockGroup, SupplierOverview } from '../types/suppliers';
import { formatCsvAmount, type CsvCell } from './csv';
import { formatAttributes } from './format';
import { RESTOCK_URGENCY_LABELS, priorityLabel } from './suppliers';
import { formatCover } from './supplierTabs';

export interface CsvTable {
  headers: string[];
  rows: CsvCell[][];
}

const text = (value: string | null | undefined): string => value ?? '';

export const SUPPLIERS_CSV_HEADERS = [
  'Proveedor',
  'Contacto',
  'Tipo de documento',
  'Documento',
  'Teléfono',
  'WhatsApp',
  'Email',
  'Ciudad',
  'Dirección',
  'Plazo de pago (días)',
  'Entrega (días)',
  'Días de visita',
  'Prioridad',
  'Estado',
  'Categorías',
  'Productos enlazados',
  'Agotados',
  'Stock bajo',
  'Notas',
];

export const buildSuppliersCsv = (suppliers: readonly SupplierOverview[]): CsvTable => ({
  headers: SUPPLIERS_CSV_HEADERS,
  rows: suppliers.map(supplier => [
    supplier.name,
    text(supplier.contact_name),
    text(supplier.document_type),
    text(supplier.document_number),
    text(supplier.phone),
    text(supplier.whatsapp),
    text(supplier.email),
    text(supplier.city),
    text(supplier.address),
    supplier.payment_terms_days,
    supplier.lead_time_days,
    text(supplier.visit_days),
    priorityLabel(supplier.priority),
    supplier.is_active ? 'Activo' : 'Inactivo',
    supplier.category_count,
    supplier.variant_count,
    supplier.out_of_stock_count,
    supplier.low_stock_count,
    text(supplier.notes),
  ]),
});

export const RESTOCK_CSV_HEADERS = [
  'Proveedor',
  'Teléfono',
  'WhatsApp',
  'Email',
  'Producto',
  'Atributos',
  'SKU',
  'Código de barras',
  'SKU del proveedor',
  'Stock',
  'Stock mínimo',
  'Vendidas en la ventana',
  'Cobertura',
  'Urgencia',
  'Cantidad sugerida',
  'Costo unitario',
  'Costo de la línea',
];

const money = (minor: number | null, decimals: number): CsvCell =>
  minor === null ? null : formatCsvAmount(minor, decimals);

export const buildRestockCsv = (
  groups: readonly RestockGroup[],
  decimals: number,
): CsvTable => ({
  headers: RESTOCK_CSV_HEADERS,
  rows: groups.flatMap(group =>
    group.items.map(item => [
      group.supplier?.name ?? 'Sin proveedor',
      text(group.supplier?.phone),
      text(group.supplier?.whatsapp),
      text(group.supplier?.email),
      item.product_name,
      formatAttributes(item.attributes),
      text(item.sku),
      text(item.barcode),
      text(item.supplier_sku),
      item.stock,
      item.stock_min,
      item.units_sold_window,
      formatCover(item.days_of_cover),
      RESTOCK_URGENCY_LABELS[item.urgency],
      item.suggested_qty,
      money(item.unit_cost, decimals),
      money(item.line_cost, decimals),
    ]),
  ),
});

export type SupplierExportKind = 'directorio' | 'reposicion';

export const supplierExportFileName = (kind: SupplierExportKind, day: string): string =>
  `proveedores_${kind}_${day}.csv`;
