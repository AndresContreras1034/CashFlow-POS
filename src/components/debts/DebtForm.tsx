import React, { useRef, useState } from 'react';
import { createDebt, updateDebt } from '../../services/debts.service';
import type { DebtDetail, DebtOptions, DebtOverview } from '../../types/debts';
import { errorMessage } from '../../utils/audit';
import {
  LIMITS,
  debtToFormValues,
  emptyDebtForm,
  newIdempotencyKey,
  validateDebtForm,
  type DebtFormErrors,
  type DebtFormValues,
} from '../../utils/debts';
import { moneyStep } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import { DebtField } from './DebtField';
import './DebtForm.css';

interface DebtFormProps {
  /** Crear una deuda nueva o editar `initial`. */
  initial: DebtOverview | null;
  options: DebtOptions;
  /** 'YYYY-MM-DD' en la zona de Ajustes: solo para el atributo `max` de las fechas. */
  today: string;
  decimals: number;
  onSaved: (detail: DebtDetail) => void;
  onCancel: () => void;
}

export const DebtForm: React.FC<DebtFormProps> = ({
  initial,
  options,
  today,
  decimals,
  onSaved,
  onCancel,
}) => {
  const editing = initial !== null;
  const [values, setValues] = useState<DebtFormValues>(() =>
    initial ? debtToFormValues(initial, decimals) : emptyDebtForm(today),
  );
  const [errors, setErrors] = useState<DebtFormErrors>({});
  const [serverError, setServerError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const submittingRef = useRef(false);
  // Una clave por apertura del formulario: reenviar no crea una segunda deuda.
  const idempotencyKey = useRef(newIdempotencyKey());

  const amountLocked = editing && initial.payment_count > 0;
  const subcategories =
    options.categories.find(category => category.value === values.category)?.subcategories ?? [];

  const set = <K extends keyof DebtFormValues>(key: K, value: DebtFormValues[K]) => {
    setValues(current => ({ ...current, [key]: value }));
    setErrors(current => (current[key] ? { ...current, [key]: undefined } : current));
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (submittingRef.current) return;

    const { errors: found, fields } = validateDebtForm(values, { decimals, today });
    setErrors(found);
    if (!fields) {
      setServerError(null);
      return;
    }

    submittingRef.current = true;
    setSubmitting(true);
    setServerError(null);
    try {
      const saved = editing
        ? await updateDebt(initial.id, { ...fields, ...actorField('updated_by') })
        : await createDebt({
            ...fields,
            idempotency_key: idempotencyKey.current,
            ...actorField('created_by'),
          });
      onSaved(saved);
    } catch (cause) {
      setServerError(errorMessage(cause));
      submittingRef.current = false;
      setSubmitting(false);
    }
  };

  const invalid = (key: keyof DebtFormValues) => (errors[key] ? true : undefined);

  return (
    <form className="debts-form" onSubmit={handleSubmit} noValidate>
      {serverError && <div className="form-error" role="alert">{serverError}</div>}

      <div className="debts-form-grid">
        <DebtField id="debt-creditor" label="Acreedor *" error={errors.creditorName}>
          <input
            id="debt-creditor"
            className="form-input"
            type="text"
            value={values.creditorName}
            maxLength={LIMITS.creditor + 20}
            aria-invalid={invalid('creditorName')}
            onChange={event => set('creditorName', event.target.value)}
          />
        </DebtField>

        <DebtField id="debt-concept" label="Concepto *" error={errors.concept}>
          <input
            id="debt-concept"
            className="form-input"
            type="text"
            value={values.concept}
            maxLength={LIMITS.concept + 20}
            aria-invalid={invalid('concept')}
            onChange={event => set('concept', event.target.value)}
          />
        </DebtField>

        <DebtField id="debt-category" label="Categoría *" error={errors.category}>
          <select
            id="debt-category"
            className="form-input"
            value={values.category}
            aria-invalid={invalid('category')}
            onChange={event => set('category', event.target.value as DebtFormValues['category'])}
          >
            <option value="">Elige una categoría</option>
            {options.categories.map(category => (
              <option key={category.value} value={category.value}>{category.label}</option>
            ))}
          </select>
        </DebtField>

        <DebtField
          id="debt-subcategory"
          label="Subcategoría"
          error={errors.subcategory}
          hint="Opcional. Puedes elegir una sugerida o escribir otra."
        >
          <input
            id="debt-subcategory"
            className="form-input"
            type="text"
            list="debt-subcategory-options"
            value={values.subcategory}
            maxLength={LIMITS.subcategory + 20}
            aria-invalid={invalid('subcategory')}
            onChange={event => set('subcategory', event.target.value)}
          />
          <datalist id="debt-subcategory-options">
            {subcategories.map(name => <option key={name} value={name} />)}
          </datalist>
        </DebtField>

        <DebtField id="debt-nature" label="Naturaleza *" error={errors.nature}>
          <select
            id="debt-nature"
            className="form-input"
            value={values.nature}
            aria-invalid={invalid('nature')}
            onChange={event => set('nature', event.target.value as DebtFormValues['nature'])}
          >
            <option value="">Elige la naturaleza</option>
            {options.natures.map(nature => (
              <option key={nature.value} value={nature.value}>{nature.label}</option>
            ))}
          </select>
        </DebtField>

        <DebtField
          id="debt-amount"
          label="Importe original *"
          error={errors.amount}
          hint={amountLocked ? 'No se puede cambiar: la deuda tiene pagos vigentes.' : undefined}
        >
          <input
            id="debt-amount"
            className="form-input"
            type="number"
            min="0"
            step={moneyStep(decimals)}
            value={values.amount}
            disabled={amountLocked}
            aria-invalid={invalid('amount')}
            onChange={event => set('amount', event.target.value)}
          />
        </DebtField>

        <DebtField id="debt-issued" label="Fecha de emisión *" error={errors.issuedOn}>
          <input
            id="debt-issued"
            className="form-input"
            type="date"
            max={today}
            value={values.issuedOn}
            aria-invalid={invalid('issuedOn')}
            onChange={event => set('issuedOn', event.target.value)}
          />
        </DebtField>

        <DebtField
          id="debt-due"
          label="Fecha de vencimiento"
          error={errors.dueOn}
          hint="Opcional. Con ella se calculan los días restantes."
        >
          <input
            id="debt-due"
            className="form-input"
            type="date"
            min={values.issuedOn || undefined}
            value={values.dueOn}
            aria-invalid={invalid('dueOn')}
            onChange={event => set('dueOn', event.target.value)}
          />
        </DebtField>

        <DebtField
          id="debt-ref"
          label="Factura, contrato o referencia"
          error={errors.documentRef}
          wide
        >
          <input
            id="debt-ref"
            className="form-input"
            type="text"
            value={values.documentRef}
            maxLength={LIMITS.reference + 20}
            aria-invalid={invalid('documentRef')}
            onChange={event => set('documentRef', event.target.value)}
          />
        </DebtField>

        <DebtField id="debt-notes" label="Observaciones" error={errors.notes} wide>
          <textarea
            id="debt-notes"
            className="form-input debts-textarea"
            rows={3}
            value={values.notes}
            aria-invalid={invalid('notes')}
            onChange={event => set('notes', event.target.value)}
          />
        </DebtField>
      </div>

      <div className="debts-form-actions">
        <button type="button" className="btn btn-ghost" onClick={onCancel} disabled={submitting}>
          Cancelar
        </button>
        <button type="submit" className="btn btn-primary" disabled={submitting}>
          {submitting ? 'Guardando…' : editing ? 'Guardar cambios' : 'Registrar deuda'}
        </button>
      </div>
    </form>
  );
};