import type { RestockUrgency } from '../types/suppliers';

// ============================================================
// Enlace proveedor ↔ variante
// ============================================================

export const LINK_LIMITS = { supplierSku: 100, notes: 2000 } as const;

export interface LinkFormValues {
  supplierSku: string;
  isPreferred: boolean;
  notes: string;
}

export type LinkFormErrors = Partial<Record<keyof LinkFormValues, string>>;

export interface LinkFields {
  supplier_sku: string | null;
  is_preferred: boolean;
  notes: string | null;
}

export const emptyLinkForm = (): LinkFormValues => ({
  supplierSku: '',
  isPreferred: false,
  notes: '',
});

export const linkToFormValues = (
  link: Pick<LinkFields, 'supplier_sku' | 'is_preferred' | 'notes'>,
): LinkFormValues => ({
  supplierSku: link.supplier_sku ?? '',
  isPreferred: link.is_preferred,
  notes: link.notes ?? '',
});

/**
 * Valida el formulario de enlace (comodidad de uso: el backend vuelve a validar)
 * y construye los campos del DTO. El costo no se usa en este enlace.
 */
export const validateLinkForm = (
  values: LinkFormValues,
): { errors: LinkFormErrors; fields: LinkFields | null } => {
  const errors: LinkFormErrors = {};

  const sku = values.supplierSku.trim();
  if (sku.length > LINK_LIMITS.supplierSku) {
    errors.supplierSku = `El SKU del proveedor no puede superar ${LINK_LIMITS.supplierSku} caracteres`;
  }
  if (values.notes.trim().length > LINK_LIMITS.notes) {
    errors.notes = `Las notas no pueden superar ${LINK_LIMITS.notes} caracteres`;
  }

  if (Object.keys(errors).length > 0) return { errors, fields: null };

  return {
    errors,
    fields: {
      supplier_sku: sku === '' ? null : sku,
      is_preferred: values.isPreferred,
      notes: values.notes.trim() === '' ? null : values.notes.trim(),
    },
  };
};

// ============================================================
// Categorías del proveedor (selección como conjunto)
// ============================================================

export const toggleId = (ids: readonly number[], id: number): number[] =>
  ids.includes(id) ? ids.filter(current => current !== id) : [...ids, id];

/** Igualdad como conjunto: ignora orden y repetidos. */
export const sameIdSet = (a: readonly number[], b: readonly number[]): boolean => {
  const left = new Set(a);
  const right = new Set(b);
  return left.size === right.size && [...left].every(id => right.has(id));
};

// ============================================================
// Reposición: presentación
// ============================================================

export const restockUrgencyBadgeClass = (urgency: RestockUrgency): string => {
  switch (urgency) {
    case 'out_of_stock':
      return 'suppliers-badge suppliers-badge-out';
    case 'critical':
      return 'suppliers-badge suppliers-badge-urgent';
    case 'low':
      return 'suppliers-badge suppliers-badge-low';
  }
};

/** Días de cobertura legibles. `null` = no hubo ventas en la ventana. */
export const formatCover = (days: number | null): string => {
  if (days === null) return 'Sin ventas';
  if (days <= 0) return 'Menos de 1 día';
  return days === 1 ? '1 día' : `${days} días`;
};
