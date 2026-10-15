import type {
  BulkLinkResult,
  LinkableFilterDto,
  LinkableVariant,
} from '../types/suppliers';
import { formatAttributes } from './format';

export type Selection = ReadonlyMap<number, string>;

export const emptySelection = (): Selection => new Map();

export const variantLabel = (
  item: Pick<LinkableVariant, 'product_name' | 'attributes'>,
): string => {
  const attributes = formatAttributes(item.attributes);
  return attributes ? `${item.product_name} (${attributes})` : item.product_name;
};

export const selectableItems = (items: readonly LinkableVariant[]): LinkableVariant[] =>
  items.filter(item => !item.linked_here);

export const toggleSelection = (selection: Selection, id: number, label: string): Selection => {
  const next = new Map(selection);
  if (next.has(id)) next.delete(id);
  else next.set(id, label);
  return next;
};

export const addAll = (selection: Selection, items: readonly LinkableVariant[]): Selection => {
  const next = new Map(selection);
  for (const item of selectableItems(items)) {
    if (!next.has(item.variant_id)) next.set(item.variant_id, variantLabel(item));
  }
  return next;
};

export const addIds = (selection: Selection, ids: readonly number[]): Selection => {
  const next = new Map(selection);
  for (const id of ids) {
    if (!next.has(id)) next.set(id, `Producto #${id}`);
  }
  return next;
};

export const removeAll = (selection: Selection, items: readonly LinkableVariant[]): Selection => {
  const next = new Map(selection);
  for (const item of items) next.delete(item.variant_id);
  return next;
};

export const removeOne = (selection: Selection, id: number): Selection => {
  const next = new Map(selection);
  next.delete(id);
  return next;
};

export const allSelected = (selection: Selection, items: readonly LinkableVariant[]): boolean => {
  const selectable = selectableItems(items);
  return selectable.length > 0 && selectable.every(item => selection.has(item.variant_id));
};

export const selectedIds = (selection: Selection): number[] => [...selection.keys()];

export const searchBlockedReason = (
  filter: Pick<LinkableFilterDto, 'search' | 'category_id' | 'only_unassigned'>,
): string | null => {
  const search = filter.search?.trim() ?? '';
  if (search.length === 1) return 'Escribe al menos 2 letras';
  if (search === '' && filter.category_id === undefined && !filter.only_unassigned) {
    return 'Escribe al menos 2 letras o elige una categoría';
  }
  return null;
};

const noun = (count: number, singular: string, plural: string): string =>
  `${count} ${count === 1 ? singular : plural}`;

export const bulkLinkMessage = (result: BulkLinkResult): string => {
  if (result.linked === 0) {
    return result.already_linked === 1
      ? 'Ese producto ya estaba enlazado'
      : `Esos ${result.already_linked} productos ya estaban enlazados`;
  }
  const linked = noun(result.linked, 'producto enlazado', 'productos enlazados');
  return result.already_linked > 0
    ? `${linked} · ${result.already_linked} ya estaban enlazados`
    : linked;
};
