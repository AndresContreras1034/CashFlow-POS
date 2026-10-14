import type { SupplierPriority } from '../types/suppliers';

export const normalizeWhatsappNumber = (value: string): string => value.replace(/\D/g, '');

export const telUrl = (value: string): string => {
  const trimmed = value.trim();
  return trimmed ? `tel:${trimmed}` : '';
};

export const whatsappUrl = (value: string): string => {
  const digits = normalizeWhatsappNumber(value);
  return digits ? `https://wa.me/${digits}` : '';
};

export const mailtoUrl = (value: string): string => {
  const trimmed = value.trim();
  return trimmed ? `mailto:${trimmed}` : '';
};

export const RESTOCK_URGENCY_LABELS = {
  out_of_stock: 'Agotado',
  critical: 'Crítico',
  low: 'Bajo',
} as const;

export const priorityLabel = (priority: SupplierPriority): string => {
  const labels: Record<SupplierPriority, string> = {
    normal: 'Normal',
    preferred: 'Preferido',
    urgent: 'Urgente',
  };
  return labels[priority] ?? priority;
};

export const priorityBadgeClass = (priority: SupplierPriority): string => {
  switch (priority) {
    case 'preferred':
      return 'suppliers-badge suppliers-badge-preferred';
    case 'urgent':
      return 'suppliers-badge suppliers-badge-urgent';
    case 'normal':
    default:
      return 'suppliers-badge';
  }
};

export const formatSupplierConditions = (
  leadTimeDays: number | null | undefined,
  paymentTermsDays: number | null | undefined,
): string => {
  const parts: string[] = [];
  if (leadTimeDays !== null && leadTimeDays !== undefined) {
    parts.push(`Entrega ${leadTimeDays} d`);
  }
  if (paymentTermsDays !== null && paymentTermsDays !== undefined) {
    parts.push(`Pago ${paymentTermsDays} d`);
  }
  return parts.length > 0 ? parts.join(' · ') : '—';
};

export const buildRestockMessage = ({
  productName,
  variantName,
  supplierName,
  urgency,
  suggestedQty,
}: {
  productName: string;
  variantName?: string;
  supplierName?: string;
  urgency: keyof typeof RESTOCK_URGENCY_LABELS;
  suggestedQty: number;
}): string => {
  const variantText = variantName ? ` (${variantName})` : '';
  const supplierText = supplierName ? ` a ${supplierName}` : '';
  const urgencyText = RESTOCK_URGENCY_LABELS[urgency];
  return `Solicitar ${productName}${variantText}${supplierText}: ${urgencyText} · ${suggestedQty} unidades.`;
};
