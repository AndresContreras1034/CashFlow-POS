import React, { useRef, useState } from 'react';
import { createCustomer, updateCustomer } from '../../services/customers.service';
import {
  DOCUMENT_TYPES,
  type Customer,
  type DocumentType,
} from '../../types/customers';
import { errorMessage } from '../../utils/audit';
import { actorField } from '../../utils/preferences';
import './CustomerForm.css';

interface CustomerFormProps {
  initial: Customer | null;
  presetName?: string;
  onSaved: (customer: Customer) => void;
  onCancel: () => void;
}

interface Values {
  name: string;
  documentType: DocumentType | '';
  documentNumber: string;
  phone: string;
  email: string;
  address: string;
  notes: string;
}

type Errors = Partial<Record<keyof Values, string>>;

const blankToNull = (text: string): string | null => {
  const trimmed = text.trim();
  return trimmed === '' ? null : trimmed;
};

const validate = (values: Values): Errors => {
  const errors: Errors = {};
  const name = values.name.trim();
  if (!name) errors.name = 'El nombre es obligatorio';
  else if (name.length > 150) errors.name = 'Máximo 150 caracteres';

  const hasNumber = values.documentNumber.trim() !== '';
  if (values.documentType && !hasNumber) {
    errors.documentNumber = 'Indica el número o quita el tipo';
  }
  if (!values.documentType && hasNumber) {
    errors.documentType = 'Elige el tipo de documento';
  }
  if (values.documentNumber.trim().length > 30) {
    errors.documentNumber = 'Máximo 30 caracteres';
  }

  const email = values.email.trim();
  if (
    email.length > 150 ||
    (email &&
      (!email.includes('@') ||
        /\s/.test(email) ||
        email.startsWith('@') ||
        email.endsWith('@')))
  ) {
    errors.email = email.length > 150 ? 'Máximo 150 caracteres' : 'El email no es válido';
  }
  if (values.phone.trim().length > 50) errors.phone = 'Máximo 50 caracteres';
  if (values.address.trim().length > 2000) errors.address = 'Máximo 2000 caracteres';
  if (values.notes.trim().length > 2000) errors.notes = 'Máximo 2000 caracteres';
  return errors;
};

export const CustomerForm: React.FC<CustomerFormProps> = ({
  initial,
  presetName,
  onSaved,
  onCancel,
}) => {
  const editing = initial !== null;
  const [values, setValues] = useState<Values>({
    name: initial?.name ?? presetName ?? '',
    documentType: initial?.document_type ?? '',
    documentNumber: initial?.document_number ?? '',
    phone: initial?.phone ?? '',
    email: initial?.email ?? '',
    address: initial?.address ?? '',
    notes: initial?.notes ?? '',
  });
  const [errors, setErrors] = useState<Errors>({});
  const [serverError, setServerError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const submittingRef = useRef(false);

  const set = <K extends keyof Values>(key: K, value: Values[K]) => {
    setValues(current => ({ ...current, [key]: value }));
    setErrors(current => (current[key] ? { ...current, [key]: undefined } : current));
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (submittingRef.current) return;

    const found = validate(values);
    setErrors(found);
    if (Object.keys(found).length > 0) return;

    const fields = {
      name: values.name.trim(),
      document_type: values.documentType === '' ? null : values.documentType,
      document_number: blankToNull(values.documentNumber),
      phone: blankToNull(values.phone),
      email: blankToNull(values.email),
      address: blankToNull(values.address),
      notes: blankToNull(values.notes),
    };

    submittingRef.current = true;
    setSubmitting(true);
    setServerError(null);
    try {
      const saved = editing
        ? await updateCustomer(initial.id, { ...fields, ...actorField('updated_by') })
        : await createCustomer({ ...fields, ...actorField('created_by') });
      onSaved(saved);
    } catch (cause) {
      setServerError(errorMessage(cause));
      submittingRef.current = false;
      setSubmitting(false);
    }
  };

  const invalid = (key: keyof Values) => (errors[key] ? true : undefined);

  return (
    <form className="customer-form" onSubmit={handleSubmit} noValidate>
      {serverError && <div className="customer-form-error" role="alert">{serverError}</div>}

      <div className="customer-form-grid">
        <div className="customer-field customer-wide">
          <label htmlFor="customer-name">Nombre *</label>
          <input
            id="customer-name"
            className="customers-input"
            type="text"
            maxLength={151}
            autoFocus
            value={values.name}
            aria-invalid={invalid('name')}
            onChange={event => set('name', event.target.value)}
          />
          {errors.name && <span className="customer-field-error">{errors.name}</span>}
        </div>

        <div className="customer-field">
          <label htmlFor="customer-doc-type">Tipo de documento</label>
          <select
            id="customer-doc-type"
            className="customers-input"
            value={values.documentType}
            aria-invalid={invalid('documentType')}
            onChange={event => set('documentType', event.target.value as DocumentType | '')}
          >
            <option value="">Sin documento</option>
            {DOCUMENT_TYPES.map(type => <option key={type} value={type}>{type}</option>)}
          </select>
          {errors.documentType && (
            <span className="customer-field-error">{errors.documentType}</span>
          )}
        </div>

        <div className="customer-field">
          <label htmlFor="customer-doc-number">Número de documento</label>
          <input
            id="customer-doc-number"
            className="customers-input"
            type="text"
            maxLength={31}
            value={values.documentNumber}
            aria-invalid={invalid('documentNumber')}
            onChange={event => set('documentNumber', event.target.value)}
          />
          {errors.documentNumber && (
            <span className="customer-field-error">{errors.documentNumber}</span>
          )}
        </div>

        <div className="customer-field">
          <label htmlFor="customer-phone">Teléfono</label>
          <input
            id="customer-phone"
            className="customers-input"
            type="tel"
            maxLength={51}
            value={values.phone}
            aria-invalid={invalid('phone')}
            onChange={event => set('phone', event.target.value)}
          />
          {errors.phone && <span className="customer-field-error">{errors.phone}</span>}
        </div>

        <div className="customer-field">
          <label htmlFor="customer-email">Email</label>
          <input
            id="customer-email"
            className="customers-input"
            type="email"
            maxLength={151}
            value={values.email}
            aria-invalid={invalid('email')}
            onChange={event => set('email', event.target.value)}
          />
          {errors.email && <span className="customer-field-error">{errors.email}</span>}
        </div>

        <div className="customer-field customer-wide">
          <label htmlFor="customer-address">Dirección</label>
          <input
            id="customer-address"
            className="customers-input"
            type="text"
            maxLength={2001}
            value={values.address}
            aria-invalid={invalid('address')}
            onChange={event => set('address', event.target.value)}
          />
          {errors.address && <span className="customer-field-error">{errors.address}</span>}
        </div>

        <div className="customer-field customer-wide">
          <label htmlFor="customer-notes">Notas</label>
          <textarea
            id="customer-notes"
            className="customers-input customers-textarea"
            rows={3}
            maxLength={2001}
            value={values.notes}
            aria-invalid={invalid('notes')}
            onChange={event => set('notes', event.target.value)}
          />
          {errors.notes && <span className="customer-field-error">{errors.notes}</span>}
        </div>
      </div>

      <div className="customer-form-actions">
        <button type="button" className="customers-btn" onClick={onCancel} disabled={submitting}>
          Cancelar
        </button>
        <button
          type="submit"
          className="customers-btn customers-btn-primary"
          disabled={submitting}
        >
          {submitting ? 'Guardando…' : editing ? 'Guardar cambios' : 'Registrar cliente'}
        </button>
      </div>
    </form>
  );
};
