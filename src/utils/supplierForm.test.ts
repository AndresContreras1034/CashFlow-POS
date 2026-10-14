import { describe, expect, it } from 'vitest';
import {
  emptySupplierForm,
  isValidEmail,
  isValidPhone,
  supplierToFormValues,
  validateSupplierForm,
} from './supplierForm';

describe('validateSupplierForm', () => {
  it('exige nombre y lo recorta', () => {
    expect(validateSupplierForm(emptySupplierForm()).errors.name).toBe('El nombre es obligatorio');
    const { fields } = validateSupplierForm({ ...emptySupplierForm(), name: '  Norte  ' });
    expect(fields?.name).toBe('Norte');
  });

  it('convierte los opcionales vacíos en null y la prioridad por defecto es normal', () => {
    const { fields } = validateSupplierForm({ ...emptySupplierForm(), name: 'Distribuidora Norte' });
    expect(fields).toMatchObject({
      contact_name: null,
      document_type: null,
      document_number: null,
      phone: null,
      whatsapp: null,
      email: null,
      payment_terms_days: null,
      lead_time_days: null,
      priority: 'normal',
    });
  });

  it('el tipo y el número de documento van juntos', () => {
    expect(
      validateSupplierForm({ ...emptySupplierForm(), name: 'X', documentType: 'NIT' }).errors
        .documentNumber,
    ).toBeDefined();
    expect(
      validateSupplierForm({ ...emptySupplierForm(), name: 'X', documentNumber: '900' }).errors
        .documentType,
    ).toBeDefined();

    const { fields } = validateSupplierForm({
      ...emptySupplierForm(),
      name: 'X',
      documentType: 'NIT',
      documentNumber: ' 900.123.456-7 ',
    });
    expect(fields).toMatchObject({ document_type: 'NIT', document_number: '900.123.456-7' });
  });

  it('valida los días como enteros de 0 a 365', () => {
    expect(
      validateSupplierForm({
        ...emptySupplierForm(),
        name: 'X',
        paymentTermsDays: '30',
        leadTimeDays: '0',
      }).fields,
    ).toMatchObject({ payment_terms_days: 30, lead_time_days: 0 });

    for (const bad of ['-1', '1.5', 'abc', '366']) {
      expect(
        validateSupplierForm({ ...emptySupplierForm(), name: 'X', leadTimeDays: bad }).errors
          .leadTimeDays,
        bad,
      ).toBeDefined();
      expect(
        validateSupplierForm({ ...emptySupplierForm(), name: 'X', paymentTermsDays: bad }).errors
          .paymentTermsDays,
        bad,
      ).toBeDefined();
    }
  });

  it('valida teléfono y WhatsApp', () => {
    expect(
      validateSupplierForm({
        ...emptySupplierForm(),
        name: 'X',
        phone: 'abc',
        whatsapp: '123',
      }).errors,
    ).toMatchObject({ phone: expect.any(String), whatsapp: expect.any(String) });
    expect(
      validateSupplierForm({
        ...emptySupplierForm(),
        name: 'X',
        phone: '+57 301 224 9460',
      }).fields?.phone,
    ).toBe('+57 301 224 9460');
  });

  it('valida el email y los límites de longitud', () => {
    expect(
      validateSupplierForm({ ...emptySupplierForm(), name: 'X', email: 'sin-arroba' }).errors.email,
    ).toBeDefined();
    expect(
      validateSupplierForm({ ...emptySupplierForm(), name: 'X', notes: 'x'.repeat(2001) }).errors
        .notes,
    ).toBeDefined();
    expect(
      validateSupplierForm({ ...emptySupplierForm(), name: 'X', city: 'x'.repeat(101) }).errors
        .city,
    ).toBeDefined();
  });

  it('no devuelve campos si hay errores', () => {
    expect(validateSupplierForm(emptySupplierForm()).fields).toBeNull();
  });
});

describe('isValidPhone / isValidEmail', () => {
  it('aceptan vacío (campos opcionales)', () => {
    expect(isValidPhone('')).toBe(true);
    expect(isValidEmail('')).toBe(true);
  });

  it('rechazan valores inválidos', () => {
    expect(isValidPhone('abc')).toBe(false);
    expect(isValidEmail('sin-arroba')).toBe(false);
  });
});

describe('supplierToFormValues', () => {
  it('mapea un proveedor y conserva los valores nulos', () => {
    const supplier: import('../types/suppliers').SupplierOverview = {
      id: 1,
      name: 'Distribuidora Norte',
      contact_name: 'Ana',
      document_type: 'NIT',
      document_number: '900',
      phone: null,
      whatsapp: null,
      email: null,
      address: null,
      city: 'Bogotá',
      payment_terms_days: 30,
      lead_time_days: 3,
      visit_days: 'Lunes',
      priority: 'preferred',
      notes: null,
      is_active: true,
      created_by: 'system',
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-02T00:00:00Z',
      category_count: 0,
      variant_count: 0,
      low_stock_count: 0,
      out_of_stock_count: 0,
    };

    expect(supplierToFormValues(supplier)).toMatchObject({
      name: 'Distribuidora Norte',
      contactName: 'Ana',
      documentType: 'NIT',
      city: 'Bogotá',
      paymentTermsDays: '30',
      leadTimeDays: '3',
      priority: 'preferred',
    });
  });
});
