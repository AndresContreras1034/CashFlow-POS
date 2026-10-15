import type {
  AddItemsResult,
  PurchaseItem,
  PurchaseItemSource,
  PurchaseListStatus,
  PurchaseSupplier,
  PurchasableVariant,
} from '../types/purchases';
import type { LinkableVariant, RestockOverview, Supplier } from '../types/suppliers';
import { formatDay } from './debts';

export const PURCHASE_STATUS_LABELS: Record<PurchaseListStatus, string> = {
  draft: 'Borrador',
  ordered: 'Pedida',
  received: 'Recibida',
  cancelled: 'Cancelada',
};

export const purchaseStatusBadgeClass = (status: PurchaseListStatus): string => {
  switch (status) {
    case 'draft':
      return 'suppliers-badge';
    case 'ordered':
      return 'suppliers-badge suppliers-badge-preferred';
    case 'received':
      return 'suppliers-badge suppliers-badge-ok';
    case 'cancelled':
      return 'suppliers-badge suppliers-badge-out';
  }
};

export const PURCHASE_SOURCE_LABELS: Record<PurchaseItemSource, string> = {
  recommended: 'Recomendado',
  manual: 'Adicional',
};

export interface StatusAction {
  target: 'ordered' | 'draft' | 'cancelled';
  label: string;
  confirm: string;
  tone: 'primary' | 'default' | 'danger';
}

const CANCEL: StatusAction = {
  target: 'cancelled',
  label: 'Cancelar lista',
  confirm: 'La lista se conserva en el historial, pero ya no se podrá usar.',
  tone: 'danger',
};

export const statusActions = (status: PurchaseListStatus): StatusAction[] => {
  if (status === 'draft') {
    return [
      {
        target: 'ordered',
        label: 'Marcar como pedida',
        confirm: 'La lista quedará bloqueada para editar. Podrás reabrirla si algo cambia.',
        tone: 'primary',
      },
      CANCEL,
    ];
  }
  if (status === 'ordered') {
    return [
      {
        target: 'draft',
        label: 'Reabrir para editar',
        confirm: 'La lista volverá a borrador.',
        tone: 'default',
      },
      CANCEL,
    ];
  }
  return [];
};

export const isEditable = (status: PurchaseListStatus): boolean => status === 'draft';

export const MAX_QUANTITY = 100_000;

export const parseQuantity = (raw: string): number | null => {
  const text = raw.trim();
  if (!/^\d+$/.test(text)) return null;
  const value = Number(text);
  return value >= 1 && value <= MAX_QUANTITY ? value : null;
};

export const defaultListTitle = (day: string): string => `Compras del ${formatDay(day)}`;

export interface PurchaseGroup {
  key: number | null;
  supplier: PurchaseSupplier | null;
  items: PurchaseItem[];
  itemCount: number;
  units: number;
  estimatedCost: number;
  itemsWithoutCost: number;
}

const byName = (a: PurchaseItem, b: PurchaseItem): number =>
  a.product_name.localeCompare(b.product_name, 'es') || a.variant_id - b.variant_id;

export const groupBySupplier = (items: readonly PurchaseItem[]): PurchaseGroup[] => {
  const buckets = new Map<number | null, PurchaseItem[]>();
  for (const item of items) {
    const key = item.supplier?.id ?? null;
    const bucket = buckets.get(key);
    if (bucket) bucket.push(item);
    else buckets.set(key, [item]);
  }

  const groups = [...buckets.entries()].map(([key, list]): PurchaseGroup => {
    const sorted = [...list].sort(byName);
    return {
      key,
      supplier: sorted[0].supplier,
      items: sorted,
      itemCount: sorted.length,
      units: sorted.reduce((sum, item) => sum + item.quantity, 0),
      estimatedCost: sorted.reduce((sum, item) => sum + (item.line_cost ?? 0), 0),
      itemsWithoutCost: sorted.filter(item => item.line_cost === null).length,
    };
  });

  return groups.sort((a, b) => {
    if (a.key === null) return b.key === null ? 0 : 1;
    if (b.key === null) return -1;
    return (
      (a.supplier?.name ?? '').localeCompare(b.supplier?.name ?? '', 'es') || a.key - b.key
    );
  });
};

export interface PurchaseTotals {
  itemCount: number;
  supplierCount: number;
  units: number;
  estimatedCost: number;
  itemsWithoutCost: number;
}

export const purchaseTotals = (items: readonly PurchaseItem[]): PurchaseTotals => {
  const suppliers = new Set<number>();
  let units = 0;
  let estimatedCost = 0;
  let itemsWithoutCost = 0;
  for (const item of items) {
    if (item.supplier) suppliers.add(item.supplier.id);
    units += item.quantity;
    if (item.line_cost === null) itemsWithoutCost += 1;
    else estimatedCost += item.line_cost;
  }
  return { itemCount: items.length, supplierCount: suppliers.size, units, estimatedCost, itemsWithoutCost };
};

export const replaceItem = (
  items: readonly PurchaseItem[],
  updated: PurchaseItem,
): PurchaseItem[] => items.map(item => (item.item_id === updated.item_id ? updated : item));

export interface PendingRecommended {
  total: number;
  urgent: number;
  bySupplier: { supplierId: number | null; name: string; count: number; urgent: number }[];
}

export const pendingRecommended = (
  overview: Pick<RestockOverview, 'groups'>,
  inList: ReadonlySet<number>,
): PendingRecommended => {
  const bySupplier: PendingRecommended['bySupplier'] = [];
  let total = 0;
  let urgent = 0;
  for (const group of overview.groups) {
    const pending = group.items.filter(item => !inList.has(item.variant_id));
    if (pending.length === 0) continue;
    const urgentCount = pending.filter(item => item.urgency !== 'low').length;
    bySupplier.push({
      supplierId: group.supplier?.id ?? null,
      name: group.supplier?.name ?? 'Sin proveedor',
      count: pending.length,
      urgent: urgentCount,
    });
    total += pending.length;
    urgent += urgentCount;
  }
  return { total, urgent, bySupplier };
};

export const messageItems = (group: Pick<PurchaseGroup, 'items'>) =>
  group.items.map(item => ({
    product_name: item.product_name,
    attributes: item.attributes,
    suggested_qty: item.quantity,
  }));

export const toPickable = (variant: PurchasableVariant): LinkableVariant => ({
  variant_id: variant.variant_id,
  product_id: variant.product_id,
  product_name: variant.product_name,
  brand: variant.brand,
  attributes: variant.attributes,
  sku: variant.sku,
  barcode: variant.barcode,
  category_name: variant.category_name,
  stock: variant.stock,
  stock_min: variant.stock_min,
  variant_cost: variant.variant_cost,
  linked_here: variant.in_list,
  supplier_count: 0,
});

export type QuantityCommit =
  | { kind: 'invalid' }
  | { kind: 'unchanged' }
  | { kind: 'save'; value: number };

export const resolveQuantityCommit = (raw: string, current: number): QuantityCommit => {
  const parsed = parseQuantity(raw);
  if (parsed === null) return { kind: 'invalid' };
  return parsed === current ? { kind: 'unchanged' } : { kind: 'save', value: parsed };
};

export interface SupplierOption {
  id: number;
  name: string;
  inactive: boolean;
}

export const buildSupplierOptions = (
  current: PurchaseSupplier | null,
  suppliers: readonly Pick<Supplier, 'id' | 'name' | 'is_active'>[],
): SupplierOption[] => {
  const options = suppliers
    .filter(supplier => supplier.is_active)
    .map(supplier => ({ id: supplier.id, name: supplier.name, inactive: false }))
    .sort((a, b) => a.name.localeCompare(b.name, 'es'));
  if (current && !options.some(option => option.id === current.id)) {
    options.unshift({ id: current.id, name: current.name, inactive: true });
  }
  return options;
};

export const addedMessage = (result: AddItemsResult): string => {
  if (result.added === 0) {
    if (result.already_in_list === 0) return 'No había productos para agregar';
    return result.already_in_list === 1
      ? 'Ese producto ya estaba en la lista'
      : `Esos ${result.already_in_list} productos ya estaban en la lista`;
  }
  const added = `${result.added} ${result.added === 1 ? 'producto agregado' : 'productos agregados'}`;
  return result.already_in_list > 0
    ? `${added} · ${result.already_in_list} ya estaban en la lista`
    : added;
};
