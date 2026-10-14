import { describe, expect, it } from 'vitest';
import type { RestockItem, SupplierContact } from '../types/suppliers';
import {
  buildRestockOrderMessage,
  callNumber,
  contactActions,
  groupNeedsAttention,
  isAllowedContactUrl,
  isColombianMobile,
  splitItems,
  urgencySummary,
  whatsappTarget,
} from './callPanel';

const contact = (patch: Partial<Pick<SupplierContact, 'phone' | 'whatsapp' | 'email'>> = {}) => ({
  phone: null,
  whatsapp: null,
  email: null,
  ...patch,
});

describe('isColombianMobile', () => {
  it('reconoce móviles con y sin código de país', () => {
    expect(isColombianMobile('301 224 9460')).toBe(true);
    expect(isColombianMobile('+57 301 224 9460')).toBe(true);
    expect(isColombianMobile('573012249460')).toBe(true);
  });

  it('rechaza fijos, números cortos y vacíos', () => {
    expect(isColombianMobile('6012345678')).toBe(false);
    expect(isColombianMobile('3012')).toBe(false);
    expect(isColombianMobile(null)).toBe(false);
  });
});

describe('callNumber / whatsappTarget', () => {
  it('llama al teléfono; si es inválido, al WhatsApp', () => {
    expect(callNumber(contact({ phone: '6012345678' }))).toBe('6012345678');
    expect(callNumber(contact({ phone: 'abc', whatsapp: '3012249460' }))).toBe('3012249460');
    expect(callNumber(contact())).toBeNull();
  });

  it('usa el teléfono para WhatsApp solo si es un móvil colombiano', () => {
    expect(whatsappTarget(contact({ phone: '3012249460' }))).toBe('3012249460');
    expect(whatsappTarget(contact({ phone: '6012345678' }))).toBeNull();
    expect(whatsappTarget(contact({ phone: '6012345678', whatsapp: '3012249460' }))).toBe('3012249460');
  });
});

describe('contactActions', () => {
  it('construye las tres acciones con un contacto completo', () => {
    const [call, wa, mail] = contactActions(
      contact({ phone: '3012249460', email: 'v@n.com' }),
      'Hola',
      'Mi Tienda',
    );
    expect(call).toMatchObject({ channel: 'call', url: 'tel:3012249460', disabledReason: null });
    expect(wa.url).toBe('https://wa.me/573012249460?text=Hola');
    expect(mail.url).toBe('mailto:v@n.com?subject=Pedido%20de%20Mi%20Tienda&body=Hola');
  });

  it('usa WhatsApp para llamar cuando no hay teléfono', () => {
    const [call, wa] = contactActions(contact({ whatsapp: '+57 301 224 9460' }), '', 'T');
    expect(call.url).toBe('tel:+573012249460');
    expect(wa.url).toBe('https://wa.me/573012249460');
  });

  it('deshabilita cada canal con su motivo', () => {
    const [call, wa, mail] = contactActions(contact({ phone: '6012345678' }), 'Hola', 'T');
    expect(call.url).toBe('tel:6012345678');
    expect(wa).toMatchObject({ url: null, disabledReason: 'Sin WhatsApp ni móvil registrado' });
    expect(mail).toMatchObject({ url: null, disabledReason: 'Sin email válido registrado' });

    const none = contactActions(contact(), 'Hola', 'T');
    expect(none.every(action => action.url === null && action.disabledReason !== null)).toBe(true);
  });

  it('todas las URLs generadas pasan el filtro de seguridad', () => {
    const actions = contactActions(
      contact({ phone: '+57 301 224 9460', email: 'ventas@norte.com' }),
      'Pedido: 5 un. & más\nGracias',
      'Mi Tienda ñ',
    );
    for (const action of actions) {
      expect(action.url).not.toBeNull();
      expect(isAllowedContactUrl(action.url ?? ''), action.channel).toBe(true);
    }
  });
});

describe('isAllowedContactUrl', () => {
  it('acepta tel, mailto y wa.me', () => {
    expect(isAllowedContactUrl('tel:3012249460')).toBe(true);
    expect(isAllowedContactUrl('tel:+573012249460')).toBe(true);
    expect(isAllowedContactUrl('mailto:a@b.com')).toBe(true);
    expect(isAllowedContactUrl('https://wa.me/573012249460?text=Hola%20mundo')).toBe(true);
  });

  it('rechaza todo lo demás', () => {
    for (const bad of [
      'javascript:alert(1)',
      'file:///C:/Windows/System32',
      'https://evil.com/?x=wa.me/57301',
      'https://wa.me/12',
      'http://wa.me/573012249460',
      'tel:abc',
      'tel:3012249460;rm',
      'mailto:a b@c.com',
      '',
    ]) {
      expect(isAllowedContactUrl(bad), bad).toBe(false);
    }
  });
});

describe('urgencySummary / groupNeedsAttention', () => {
  it('describe agotados, críticos y bajos con su plural', () => {
    expect(urgencySummary({ item_count: 6, out_count: 2, critical_count: 1 })).toBe(
      '2 agotados · 1 crítico · 3 con stock bajo',
    );
    expect(urgencySummary({ item_count: 1, out_count: 1, critical_count: 0 })).toBe('1 agotado');
    expect(urgencySummary({ item_count: 0, out_count: 0, critical_count: 0 })).toBe('Nada que reponer');
  });

  it('un grupo requiere atención si tiene agotados o críticos', () => {
    expect(groupNeedsAttention({ out_count: 0, critical_count: 1 })).toBe(true);
    expect(groupNeedsAttention({ out_count: 0, critical_count: 0 })).toBe(false);
  });
});

describe('splitItems / buildRestockOrderMessage', () => {
  const items = Array.from({ length: 7 }, (_, i) => ({ variant_id: i }) as RestockItem);
  const messageItem: RestockItem = {
    variant_id: 1,
    product_id: 1,
    product_name: 'Arroz',
    brand: null,
    attributes: { peso: '1 kg' },
    sku: null,
    barcode: null,
    category_name: 'Granos',
    stock: 0,
    stock_min: 1,
    stock_status: 'out_of_stock',
    units_sold_window: 0,
    days_of_cover: null,
    urgency: 'out_of_stock',
    suggested_qty: 4,
    unit_cost: null,
    line_cost: null,
    supplier_sku: null,
    alternative_suppliers: 0,
  };

  it('recorta y cuenta los ocultos', () => {
    expect(splitItems(items, 5)).toMatchObject({ hiddenCount: 2 });
    expect(splitItems(items, 5).visible).toHaveLength(5);
  });

  it('muestra todo si está expandido o cabe', () => {
    expect(splitItems(items, 5, true)).toEqual({ visible: items, hiddenCount: 0 });
    expect(splitItems(items.slice(0, 3), 5)).toEqual({ visible: items.slice(0, 3), hiddenCount: 0 });
  });

  it('construye una lista de pedido para WhatsApp, correo o copiar', () => {
    expect(buildRestockOrderMessage('Proveedor', [messageItem], 'Mi tienda')).toBe(
      'Hola Proveedor,\n\nQuisiera solicitar:\n- Arroz (peso: 1 kg): 4 unidades\n\nGracias,\nMi tienda',
    );
  });
});
