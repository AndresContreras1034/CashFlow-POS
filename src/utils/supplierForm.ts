import type {
  Supplier,
  SupplierDocumentType,
  SupplierFields,
  SupplierPriority,
} from '../types/suppliers';

export const SUPPLIER_LIMITS = {
  name: 150,
  contact: 150,
  documentNumber: 30,
  phone: 50,
  email: 150,
  address: 2000,
  city: 100,
  visitDays: 100,
  notes: 2000,
  maxDays: 365,
} as const;

export interface SupplierFormValues {
  name: string;
  contactName: string;
  documentType: SupplierDocumentType | '';
  documentNumber: string;
  phone: string;
  whatsapp: string;
  email: string;
  address: string;
  city: string;
  paymentTermsDays: string;
  leadTimeDays: string;
  visitDays: string;
  priority: SupplierPriority;
  notes: string;
}

export type SupplierFormErrors = Partial<Record<keyof SupplierFormValues, string>>;

export const emptySupplierForm = (): SupplierFormValues => ({
  name: '',
  contactName: '',
  documentType: '',
  documentNumber: '',
  phone: '',
  whatsapp: '',
  email: '',
  address: '',
  city: '',
  paymentTermsDays: '',
  leadTimeDays: '',
  visitDays: '',
  priority: 'normal',
  notes: '',
});

const dayText = (days: number | null): string => (days === null ? '' : String(days));

export const supplierToFormValues = (supplier: Supplier): SupplierFormValues => ({
  name: supplier.name,
  contactName: supplier.contact_name ?? '',
  documentType: supplier.document_type ?? '',
  documentNumber: supplier.document_number ?? '',
  phone: supplier.phone ?? '',
  whatsapp: supplier.whatsapp ?? '',
  email: supplier.email ?? '',
  address: supplier.address ?? '',
  city: supplier.city ?? '',
  paymentTermsDays: dayText(supplier.payment_terms_days),
  leadTimeDays: dayText(supplier.lead_time_days),
  visitDays: supplier.visit_days ?? '',
  priority: supplier.priority,
  notes: supplier.notes ?? '',
});

const blankToNull = (text: string): string | null => {
  const trimmed = text.trim();
  return trimmed === '' ? null : trimmed;
};

const tooLong = (value: string, max: number, label: string): string | undefined =>
  value.trim().length > max ? `${label} no puede superar ${max} caracteres` : undefined;

export const isValidPhone = (text: string): boolean => {
  const trimmed = text.trim();
  if (trimmed === '') return true;
  if (!/^[\d\s+().-]+$/.test(trimmed)) return false;
  const digits = trimmed.replace(/\D/g, '').length;
  return digits >= 7 && digits <= 15;
};

export const isValidEmail = (text: string): boolean => {
  const email = text.trim();
  if (email === '') return true;
  return (
    email.includes('@') &&
    !/\s/.test(email) &&
    !email.startsWith('@') &&
    !email.endsWith('@')
  );
};

type DaysResult = { ok: true; value: number | null } | { ok: false };

const parseDays = (raw: string): DaysResult => {
  const text = raw.trim();
  if (text === '') return { ok: true, value: null };
  if (!/^\d+$/.test(text)) return { ok: false };
  const value = Number(text);
  return value <= SUPPLIER_LIMITS.maxDays ? { ok: true, value } : { ok: false };
};

const PHONE_MESSAGE = 'Usa solo dígitos, espacios y + - ( ) . (de 7 a 15 dígitos)';
const DAYS_MESSAGE = `Ingresa un número entero de 0 a ${SUPPLIER_LIMITS.maxDays}`;

export const validateSupplierForm = (
  values: SupplierFormValues,
): { errors: SupplierFormErrors; fields: SupplierFields | null } => {
  const errors: SupplierFormErrors = {};
  const put = (key: keyof SupplierFormValues, message: string | undefined) => {
    if (message) errors[key] = message;
  };

  if (values.name.trim() === '') put('name', 'El nombre es obligatorio');
  else put('name', tooLong(values.name, SUPPLIER_LIMITS.name, 'El nombre'));

  put('contactName', tooLong(values.contactName, SUPPLIER_LIMITS.contact, 'El contacto'));

  const hasNumber = values.documentNumber.trim() !== '';
  if (values.documentType !== '' && !hasNumber) {
    put('documentNumber', 'Indica el número o quita el tipo');
  } else if (values.documentType === '' && hasNumber) {
    put('documentType', 'Elige el tipo de documento');
  }
  if (!errors.documentNumber) {
    put(
      'documentNumber',
      tooLong(values.documentNumber, SUPPLIER_LIMITS.documentNumber, 'El documento'),
    );
  }

  if (!isValidPhone(values.phone)) put('phone', PHONE_MESSAGE);
  if (!isValidPhone(values.whatsapp)) put('whatsapp', PHONE_MESSAGE);

  if (values.email.trim().length > SUPPLIER_LIMITS.email) {
    put('email', `El email no puede superar ${SUPPLIER_LIMITS.email} caracteres`);
  } else if (!isValidEmail(values.email)) {
    put('email', 'El email no es válido');
  }

  put('address', tooLong(values.address, SUPPLIER_LIMITS.address, 'La dirección'));
  put('city', tooLong(values.city, SUPPLIER_LIMITS.city, 'La ciudad'));
  put('visitDays', tooLong(values.visitDays, SUPPLIER_LIMITS.visitDays, 'Los días de visita'));
  put('notes', tooLong(values.notes, SUPPLIER_LIMITS.notes, 'Las notas'));

  const payment = parseDays(values.paymentTermsDays);
  const lead = parseDays(values.leadTimeDays);
  if (!payment.ok) put('paymentTermsDays', DAYS_MESSAGE);
  if (!lead.ok) put('leadTimeDays', DAYS_MESSAGE);

  if (Object.keys(errors).length > 0 || !payment.ok || !lead.ok) {
    return { errors, fields: null };
  }

  return {
    errors,
    fields: {
      name: values.name.trim(),
      contact_name: blankToNull(values.contactName),
      document_type: values.documentType === '' ? null : values.documentType,
      document_number: blankToNull(values.documentNumber),
      phone: blankToNull(values.phone),
      whatsapp: blankToNull(values.whatsapp),
      email: blankToNull(values.email),
      address: blankToNull(values.address),
      city: blankToNull(values.city),
      payment_terms_days: payment.value,
      lead_time_days: lead.value,
      visit_days: blankToNull(values.visitDays),
      priority: values.priority,
      notes: blankToNull(values.notes),
    },
  };
};
