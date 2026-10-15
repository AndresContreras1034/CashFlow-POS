import React, { useRef, useState } from 'react';
import {
  linkSupplierVariant,
  updateSupplierVariant,
} from '../../services/suppliers.service';
import type { SupplierVariantLink } from '../../types/suppliers';
import { errorMessage } from '../../utils/audit';
import { formatAttributes } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import {
  emptyLinkForm,
  LINK_LIMITS,
  linkToFormValues,
  validateLinkForm,
  type LinkFormErrors,
  type LinkFormValues,
} from '../../utils/supplierTabs';
import './SuppliersBase.css';
import './SupplierLinkForm.css';

interface VariantInfo {
  id: number;
  product_name: string;
  attributes: Record<string, string>;
}

interface SupplierLinkFormProps {
  supplierId: number;
  variant: VariantInfo;
  initial: SupplierVariantLink | null;
  onSaved: () => void;
  onCancel: () => void;
}

export const SupplierLinkForm: React.FC<SupplierLinkFormProps> = ({
  supplierId,
  variant,
  initial,
  onSaved,
  onCancel,
}) => {
  const editing = initial !== null;

  const [values, setValues] = useState<LinkFormValues>(() =>
    initial ? linkToFormValues(initial) : emptyLinkForm(),
  );
  const [errors, setErrors] = useState<LinkFormErrors>({});
  const [serverError, setServerError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const submittingRef = useRef(false);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (submittingRef.current) return;

    const { errors: found, fields } = validateLinkForm(values);
    setErrors(found);
    if (!fields) return;

    submittingRef.current = true;
    setSubmitting(true);
    setServerError(null);

    try {
      if (editing) {
        await updateSupplierVariant(initial.id, {
          ...fields,
          ...actorField('updated_by'),
        });
      } else {
        await linkSupplierVariant(supplierId, {
          variant_id: variant.id,
          ...fields,
          ...actorField('created_by'),
        });
      }
      onSaved();
    } catch (cause) {
      setServerError(errorMessage(cause));
      submittingRef.current = false;
      setSubmitting(false);
    }
  };

  const attrsText = formatAttributes(variant.attributes);

  return (
    <form className="supplier-link-form" onSubmit={handleSubmit} noValidate>
      {serverError && (
        <div className="supplier-link-form-error" role="alert">
          {serverError}
        </div>
      )}

      <div className="supplier-link-variant-info">
        <span className="supplier-link-variant-name">{variant.product_name}</span>
        {attrsText && (
          <span className="supplier-link-variant-attrs">{attrsText}</span>
        )}
      </div>

      <div className="supplier-link-form-grid">
        <div className="supplier-link-field">
          <label htmlFor="link-supplier-sku">SKU del proveedor</label>
          <input
            id="link-supplier-sku"
            className="suppliers-input"
            type="text"
            maxLength={LINK_LIMITS.supplierSku + 1}
            value={values.supplierSku}
            aria-invalid={errors.supplierSku ? true : undefined}
            onChange={e => {
              const next = e.target.value;
              setValues(curr => ({ ...curr, supplierSku: next }));
              setErrors(curr => (curr.supplierSku ? { ...curr, supplierSku: undefined } : curr));
            }}
            placeholder="Ej. PROV-SKU-01"
          />
          {errors.supplierSku && (
            <span className="supplier-link-field-error">{errors.supplierSku}</span>
          )}
        </div>

        <div className="supplier-link-field supplier-link-field-full">
          <label className="supplier-link-checkbox-label">
            <input
              type="checkbox"
              checked={values.isPreferred}
              onChange={e =>
                setValues(curr => ({ ...curr, isPreferred: e.target.checked }))
              }
            />
            <span>Proveedor preferido para esta variante</span>
          </label>
          <span className="supplier-link-field-hint">
            Si otro proveedor era el preferido de esta variante, dejará de serlo.
          </span>
        </div>

        <div className="supplier-link-field supplier-link-field-full">
          <label htmlFor="link-notes">Notas</label>
          <textarea
            id="link-notes"
            className="suppliers-input suppliers-textarea"
            rows={2}
            maxLength={LINK_LIMITS.notes + 1}
            value={values.notes}
            aria-invalid={errors.notes ? true : undefined}
            onChange={e => {
              const next = e.target.value;
              setValues(curr => ({ ...curr, notes: next }));
              setErrors(curr => (curr.notes ? { ...curr, notes: undefined } : curr));
            }}
            placeholder="Observaciones de compra para este enlace…"
          />
          {errors.notes && (
            <span className="supplier-link-field-error">{errors.notes}</span>
          )}
        </div>
      </div>

      <div className="supplier-link-form-actions">
        <button
          type="button"
          className="suppliers-btn"
          onClick={onCancel}
          disabled={submitting}
        >
          Cancelar
        </button>
        <button
          type="submit"
          className="suppliers-btn suppliers-btn-primary"
          disabled={submitting}
        >
          {submitting
            ? 'Guardando…'
            : editing
            ? 'Guardar enlace'
            : 'Enlazar producto'}
        </button>
      </div>
    </form>
  );
};
