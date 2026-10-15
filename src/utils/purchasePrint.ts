import type { PurchaseItem, PurchaseList, PurchaseSupplier } from '../types/purchases';
import { formatDay } from './debts';
import { formatAttributes } from './format';
import {
  PURCHASE_STATUS_LABELS,
  groupBySupplier,
  purchaseTotals,
  type PurchaseTotals,
} from './purchaseList';

export interface PrintLine {
  key: number;
  product: string;
  detail: string | null;
  supplierSku: string | null;
  note: string | null;
  quantity: number;
  unitCost: number | null;
  lineCost: number | null;
}

export interface PrintGroup {
  key: number | null;
  name: string;
  contact: string | null;
  lines: PrintLine[];
  itemCount: number;
  units: number;
  estimatedCost: number;
  itemsWithoutCost: number;
}

export interface PrintModel {
  businessName: string;
  title: string;
  statusLabel: string;
  day: string;
  notes: string | null;
  groups: PrintGroup[];
  totals: PurchaseTotals;
}

export const supplierContactLine = (supplier: PurchaseSupplier | null): string | null => {
  if (!supplier) return null;
  const parts: string[] = [];
  if (supplier.phone && supplier.phone === supplier.whatsapp) {
    parts.push(`Tel./WhatsApp ${supplier.phone}`);
  } else {
    if (supplier.phone) parts.push(`Tel. ${supplier.phone}`);
    if (supplier.whatsapp) parts.push(`WhatsApp ${supplier.whatsapp}`);
  }
  if (supplier.email) parts.push(supplier.email);
  return parts.length > 0 ? parts.join(' · ') : null;
};

const lineDetail = (item: PurchaseItem): string | null => {
  const text = [item.brand, formatAttributes(item.attributes)].filter(Boolean).join(' · ');
  return text === '' ? null : text;
};

/** `day` is `YYYY-MM-DD` (for example, from `todayInTimeZone`). Amounts come from the backend. */
export const buildPrintModel = (
  list: Pick<PurchaseList, 'title' | 'notes' | 'status'>,
  items: readonly PurchaseItem[],
  options: { businessName: string; day: string },
): PrintModel => ({
  businessName: options.businessName,
  title: list.title,
  statusLabel: PURCHASE_STATUS_LABELS[list.status],
  day: formatDay(options.day),
  notes: list.notes,
  totals: purchaseTotals(items),
  groups: groupBySupplier(items).map(group => ({
    key: group.key,
    name: group.supplier?.name ?? 'Sin proveedor',
    contact: supplierContactLine(group.supplier),
    itemCount: group.itemCount,
    units: group.units,
    estimatedCost: group.estimatedCost,
    itemsWithoutCost: group.itemsWithoutCost,
    lines: group.items.map(item => ({
      key: item.item_id,
      product: item.product_name,
      detail: lineDetail(item),
      supplierSku: item.supplier_sku,
      note: item.note,
      quantity: item.quantity,
      unitCost: item.variant_cost > 0 ? item.variant_cost : null,
      lineCost: item.line_cost,
    })),
  })),
});

const noun = (n: number, one: string, many: string): string => `${n} ${n === 1 ? one : many}`;

export const describeTotals = (
  totals: PurchaseTotals,
  formatMoney: (minor: number) => string,
): string => {
  const parts = [
    noun(totals.itemCount, 'producto', 'productos'),
    noun(totals.supplierCount, 'proveedor', 'proveedores'),
    noun(totals.units, 'unidad', 'unidades'),
    `Costo estimado ${formatMoney(totals.estimatedCost)}`,
  ];
  if (totals.itemsWithoutCost > 0) parts.push(`${totals.itemsWithoutCost} sin costo`);
  return parts.join(' · ');
};

/** Sanitizes the suggested PDF filename for Windows. */
export const printDocumentTitle = (title: string, day: string): string => {
  const safe = title
    // eslint-disable-next-line no-control-regex
    .replace(/[\x00-\x1f<>:"/\\|?*]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, 100)
    .trim();
  return `${safe === '' ? 'Lista de compras' : safe} - ${day}`;
};
