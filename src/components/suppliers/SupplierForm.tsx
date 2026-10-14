import React, { useRef, useState } from 'react';
import { createSupplier, updateSupplier } from '../../services/suppliers.service';
import {
  SUPPLIER_DOCUMENT_TYPES,
  SUPPLIER_PRIORITIES,
  type Supplier,
} from '../../types/suppliers';
import { errorMessage } from '../../utils/audit';
import { actorField } from '../../utils/preferences';
import {
  SUPPLIER_LIMITS,
  emptySupplierForm,
  supplierToFormValues,
  validateSupplierForm,
  type SupplierFormErrors,
  type SupplierFormValues,
} from '../../utils/supplierForm';
import './SuppliersBase.css';
import './SupplierForm.css';

interface SupplierFormProps {
  initial: Supplier | null;
  onSaved: (supplier: Supplier) => void;
  onCancel: () => void;
}

interface FieldProps {
  id: string;
  label: string;
  error?: string;
  hint?: string;
  wide?: boolean;
  children: React.ReactNode;
}

const Field: React.FC<FieldProps> = ({ id, label, error, hint, wide, children }) => (
  <div className={wide ? 'supplier-field supplier-wide' : 'supplier-field'}>
    <label htmlFor={id}>{label}</label>
    {children}
    {hint && !error && <span className="supplier-field-hint">{hint}</span>}
    {error && <span className="supplier-field-error">{error}</span>}
  </div>
);

type ChangeEvent = React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>;

export const SupplierForm: React.FC<SupplierFormProps> = ({ initial, onSaved, onCancel }) => {
  const editing = initial !== null;
  const [values, setValues] = useState<SupplierFormValues>(() =>
    initial ? supplierToFormValues(initial) : emptySupplierForm(),
  );
  const [errors, setErrors] = useState<SupplierFormErrors>({});
  const [serverError, setServerError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const submittingRef = useRef(false);

  const bind = (key: keyof SupplierFormValues) => ({
    value: values[key],
    'aria-invalid': errors[key] ? true : undefined,
    onChange: (event: ChangeEvent) => {
      const next = event.target.value;
      setValues(current => ({ ...current, [key]: next }));
      setErrors(current => (current[key] ? { ...current, [key]: undefined } : current));
    },
  });

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (submittingRef.current) return;

    const { errors: found, fields } = validateSupplierForm(values);
    setErrors(found);
    if (!fields) return;

    submittingRef.current = true;
    setSubmitting(true);
    setServerError(null);
    try {
      const saved = editing
        ? await updateSupplier(initial.id, { ...fields, ...actorField('updated_by') })
        : await createSupplier({ ...fields, ...actorField('created_by') });
      onSaved(saved);
    } catch (cause) {
      setServerError(errorMessage(cause));
      submittingRef.current = false;
      setSubmitting(false);
    }
  };

  return (
    <form className="supplier-form" onSubmit={handleSubmit} noValidate>
      {serverError && <div className="supplier-form-error" role="alert">{serverError}</div>}

      <div className="supplier-form-grid">
        <Field id="supplier-name" label="Nombre *" error={errors.name} wide>
          <input
            id="supplier-name"
            className="suppliers-input"
            type="text"
            maxLength={SUPPLIER_LIMITS.name + 1}
            autoFocus
            {...bind('name')}
          />
        </Field>

        <Field id="supplier-contact" label="Persona de contacto" error={errors.contactName}>
          <input
            id="supplier-contact"
            className="suppliers-input"
            type="text"
            maxLength={SUPPLIER_LIMITS.contact + 1}
            {...bind('contactName')}
          />
        </Field>

        <Field id="supplier-priority" label="Prioridad">
          <select id="supplier-priority" className="suppliers-input" {...bind('priority')}>
            {SUPPLIER_PRIORITIES.map(option => (
              <option key={option.value} value={option.value}>{option.label}</option>
            ))}
          </select>
        </Field>

        <Field id="supplier-doc-type" label="Tipo de documento" error={errors.documentType}>
          <select id="supplier-doc-type" className="suppliers-input" {...bind('documentType')}>
            <option value="">Sin documento</option>
            {SUPPLIER_DOCUMENT_TYPES.map(type => (
              <option key={type} value={type}>{type}</option>
            ))}
          </select>
        </Field>

        <Field id="supplier-doc-number" label="Número de documento" error={errors.documentNumber}>
          <input
            id="supplier-doc-number"
            className="suppliers-input"
            type="text"
            maxLength={SUPPLIER_LIMITS.documentNumber + 1}
            {...bind('documentNumber')}
          />
        </Field>

        <div className="supplier-form-section">Contacto</div>

        <Field id="supplier-phone" label="Teléfono" error={errors.phone}>
          <input
            id="supplier-phone"
            className="suppliers-input"
            type="tel"
            maxLength={SUPPLIER_LIMITS.phone + 1}
            {...bind('phone')}
          />
        </Field>

        <Field
          id="supplier-whatsapp"
          label="WhatsApp"
          error={errors.whatsapp}
          hint="Un móvil colombiano de 10 dígitos se completa con +57 al abrir el chat."
        >
          <input
            id="supplier-whatsapp"
            className="suppliers-input"
            type="tel"
            maxLength={SUPPLIER_LIMITS.phone + 1}
            {...bind('whatsapp')}
          />
        </Field>

        <Field id="supplier-email" label="Email" error={errors.email}>
          <input
            id="supplier-email"
            className="suppliers-input"
            type="email"
            maxLength={SUPPLIER_LIMITS.email + 1}
            {...bind('email')}
          />
        </Field>

        <Field id="supplier-city" label="Ciudad" error={errors.city}>
          <input
            id="supplier-city"
            className="suppliers-input"
            type="text"
            maxLength={SUPPLIER_LIMITS.city + 1}
            {...bind('city')}
          />
        </Field>

        <Field id="supplier-address" label="Dirección" error={errors.address} wide>
          <input
            id="supplier-address"
            className="suppliers-input"
            type="text"
            maxLength={SUPPLIER_LIMITS.address + 1}
            {...bind('address')}
          />
        </Field>

        <div className="supplier-form-section">Condiciones</div>

        <Field
          id="supplier-payment"
          label="Plazo de pago (días)"
          error={errors.paymentTermsDays}
          hint="0 = de contado"
        >
          <input
            id="supplier-payment"
            className="suppliers-input"
            type="number"
            min={0}
            max={SUPPLIER_LIMITS.maxDays}
            step={1}
            {...bind('paymentTermsDays')}
          />
        </Field>

        <Field
          id="supplier-lead"
          label="Tiempo de entrega (días)"
          error={errors.leadTimeDays}
          hint="Se usa para calcular qué tan urgente es reponer."
        >
          <input
            id="supplier-lead"
            className="suppliers-input"
            type="number"
            min={0}
            max={SUPPLIER_LIMITS.maxDays}
            step={1}
            {...bind('leadTimeDays')}
          />
        </Field>

        <Field
          id="supplier-visit"
          label="Días de visita o pedido"
          error={errors.visitDays}
          hint="Ej.: Lunes y jueves"
          wide
        >
          <input
            id="supplier-visit"
            className="suppliers-input"
            type="text"
            maxLength={SUPPLIER_LIMITS.visitDays + 1}
            {...bind('visitDays')}
          />
        </Field>

        <Field id="supplier-notes" label="Notas" error={errors.notes} wide>
          <textarea
            id="supplier-notes"
            className="suppliers-input suppliers-textarea"
            rows={3}
            maxLength={SUPPLIER_LIMITS.notes + 1}
            {...bind('notes')}
          />
        </Field>
      </div>

      <div className="supplier-form-actions">
        <button type="button" className="suppliers-btn" onClick={onCancel} disabled={submitting}>
          Cancelar
        </button>
        <button
          type="submit"
          className="suppliers-btn suppliers-btn-primary"
          disabled={submitting}
        >
          {submitting ? 'Guardando…' : editing ? 'Guardar cambios' : 'Registrar proveedor'}
        </button>
      </div>
    </form>
  );
};
