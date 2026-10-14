import type { RestockGroup, RestockItem, SupplierContact } from '../types/suppliers';
import { mailtoUrl, normalizeWhatsappNumber, whatsappUrl } from './suppliers';

export type ContactChannel = 'call' | 'whatsapp' | 'email';

export interface ContactAction {
  channel: ContactChannel;
  label: string;
  url: string | null;
  disabledReason: string | null;
}

type Contactable = Pick<SupplierContact, 'phone' | 'whatsapp' | 'email'>;

const safeTelUrl = (value: string | null): string | null => {
  if (!value) return null;
  const trimmed = value.trim();
  if (!/^[+\d\s().-]+$/.test(trimmed)) return null;
  const compact = trimmed.replace(/[^\d+]/g, '');
  return /^\+?\d{7,15}$/.test(compact) ? `tel:${compact}` : null;
};

export const isColombianMobile = (raw: string | null | undefined): boolean => {
  if (!raw || !/^[+\d\s().-]+$/.test(raw.trim())) return false;
  const digits = raw.replace(/\D/g, '');
  return /^3\d{9}$/.test(digits) || /^573\d{9}$/.test(digits);
};

export const callNumber = (contact: Contactable): string | null => {
  if (safeTelUrl(contact.phone) !== null) return contact.phone;
  if (safeTelUrl(contact.whatsapp) !== null) return contact.whatsapp;
  return null;
};

export const whatsappTarget = (contact: Contactable): string | null => {
  const whatsappDigits = normalizeWhatsappNumber(contact.whatsapp ?? '');
  if (/^\d{7,15}$/.test(whatsappDigits)) return contact.whatsapp;
  if (isColombianMobile(contact.phone)) return contact.phone;
  return null;
};

export const contactActions = (
  contact: Contactable,
  message: string,
  businessName: string,
): ContactAction[] => {
  const number = callNumber(contact);
  const target = whatsappTarget(contact);
  const email = mailtoUrl(
    contact.email ?? '',
    `Pedido de ${businessName}`,
    message,
  );

  return [
    {
      channel: 'call',
      label: 'Llamar',
      url: safeTelUrl(number),
      disabledReason: number === null ? 'Sin teléfono registrado' : null,
    },
    {
      channel: 'whatsapp',
      label: 'WhatsApp',
      url: target === null ? null : whatsappUrl(target, message),
      disabledReason: target === null ? 'Sin WhatsApp ni móvil registrado' : null,
    },
    {
      channel: 'email',
      label: 'Correo',
      url: email || null,
      disabledReason: email ? null : 'Sin email válido registrado',
    },
  ];
};

export const isAllowedContactUrl = (url: string): boolean =>
  /^tel:\+?\d{7,15}$/.test(url) ||
  /^mailto:[^\s?]+(\?[^\s]*)?$/.test(url) ||
  /^https:\/\/wa\.me\/\d{7,15}(\?text=[^\s]*)?$/.test(url);

export const buildRestockOrderMessage = (
  supplierName: string | null,
  items: RestockItem[],
  businessName: string,
): string => {
  const greeting = supplierName ? `Hola ${supplierName},` : 'Hola,';
  const lines = items.map(item => {
    const attributes = Object.entries(item.attributes)
      .map(([key, value]) => `${key}: ${String(value)}`)
      .join(', ');
    const variant = attributes ? ` (${attributes})` : '';
    return `- ${item.product_name}${variant}: ${item.suggested_qty} unidades`;
  });

  return `${greeting}\n\nQuisiera solicitar:\n${lines.join('\n')}\n\nGracias,\n${businessName}`;
};

const plural = (n: number, one: string, many: string): string =>
  `${n} ${n === 1 ? one : many}`;

export const urgencySummary = (
  group: Pick<RestockGroup, 'item_count' | 'out_count' | 'critical_count'>,
): string => {
  const low = Math.max(group.item_count - group.out_count - group.critical_count, 0);
  const parts: string[] = [];
  if (group.out_count > 0) parts.push(plural(group.out_count, 'agotado', 'agotados'));
  if (group.critical_count > 0) parts.push(plural(group.critical_count, 'crítico', 'críticos'));
  if (low > 0) parts.push(`${low} con stock bajo`);
  return parts.length > 0 ? parts.join(' · ') : 'Nada que reponer';
};

export const groupNeedsAttention = (
  group: Pick<RestockGroup, 'out_count' | 'critical_count'>,
): boolean => group.out_count + group.critical_count > 0;

export const splitItems = (
  items: RestockItem[],
  limit = 5,
  expanded = false,
): { visible: RestockItem[]; hiddenCount: number } => {
  if (expanded || items.length <= limit) return { visible: items, hiddenCount: 0 };
  return { visible: items.slice(0, limit), hiddenCount: items.length - limit };
};
