import { describe, expect, it } from 'vitest';
import {
  buildRestockMessage,
  mailtoUrl,
  normalizeWhatsappNumber,
  priorityLabel,
  RESTOCK_URGENCY_LABELS,
  telUrl,
  whatsappUrl,
} from './suppliers';

describe('suppliers helpers', () => {
  it('normaliza números de WhatsApp', () => {
    expect(normalizeWhatsappNumber('+57 (301) 224-9460')).toBe('573012249460');
  });

  it('genera urls para teléfono, WhatsApp y correo', () => {
    expect(telUrl('+57 301 224 9460')).toBe('tel:+57 301 224 9460');
    expect(whatsappUrl('+57 301 224 9460')).toBe('https://wa.me/573012249460');
    expect(mailtoUrl('ventas@norte.com')).toBe('mailto:ventas@norte.com');
  });

  it('construye mensajes de reposición', () => {
    expect(buildRestockMessage({
      productName: 'Aceite',
      variantName: '500 ml',
      supplierName: 'Distribuidora Norte',
      urgency: 'critical',
      suggestedQty: 12,
    })).toBe('Solicitar Aceite (500 ml) a Distribuidora Norte: Crítico · 12 unidades.');
  });

  it('expone etiquetas y prioridad', () => {
    expect(RESTOCK_URGENCY_LABELS.critical).toBe('Crítico');
    expect(priorityLabel('preferred')).toBe('Preferido');
  });
});
