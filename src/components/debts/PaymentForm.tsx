import React, { useRef, useState } from 'react';
import { registerDebtPayment } from '../../services/debts.service';
import type { DebtDetail, DebtOptions, DebtOverview } from '../../types/debts';
import { errorMessage } from '../../utils/audit';
import {
  LIMITS,
  emptyPaymentForm,
  formatDay,
  newIdempotencyKey,
  validatePaymentForm,
  type PaymentFormErrors,
  type PaymentFormValues,
} from '../../utils/debts';
import { formatMoney, minorToInput, moneyStep } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import { DebtField } from './DebtField';
import './DebtForm.css';
import './PaymentForm.css';

interface PaymentFormProps {
  debt: Pick<DebtOverview, 'id' | 'creditor_name' | 'concept' | 'balance' | 'issued_on'>;
  options: DebtOptions;
  today: string;
  decimals: number;
  onSaved: (detail: DebtDetail) => void;
  onCancel: () => void;
}

export const PaymentForm: React.FC<PaymentFormProps> = ({
  debt,
  options,
  today,
  decimals,
  onSaved,
  onCancel,
}) => {
  const [values, setValues] = useState<PaymentFormValues>(() => emptyPaymentForm(today));
  const [errors, setErrors] = useState<PaymentFormErrors>({});
  const [serverError, setServerError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const submittingRef = useRef(false);
  // Una clave por apertura: un doble clic o un reintento no registran dos pagos.
  const idempotencyKey = useRef(newIdempotencyKey());

  const set = <K extends keyof PaymentFormValues>(key: K, value: PaymentFormValues[K]) => {
    setValues(current => ({ ...current, [key]: value }));
    setErrors(current => (current[key] ? { ...current, [key]: undefined } : current));
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (submittingRef.current) return;

    const { errors: found, fields } = validatePaymentForm(values, {
      decimals,
      today,
      issuedOn: debt.issued_on,
      balance: debt.balance,
    });
    setErrors(found);
    if (!fields) {
      setServerError(null);
      return;
    }

    submittingRef.current = true;
    setSubmitting(true);
    setServerError(null);
    try {
      const saved = await registerDebtPayment(debt.id, {
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

  const invalid = (key: keyof PaymentFormValues) => (errors[key] ? true : undefined);

  return (
    <form className="debts-form debts-payment" onSubmit={handleSubmit} noValidate>
      <p className="debts-payment-summary">
        <strong>{debt.creditor_name}</strong> · {debt.concept}
        <br />
        Saldo pendiente: <strong className="tabular">{formatMoney(debt.balance)}</strong>
      </p>

      {serverError && <div className="form-error" role="alert">{serverError}</div>}

      <div className="debts-form-grid">
        <DebtField id="pay-amount" label="Importe del pago *" error={errors.amount}>
          <div className="debts-amount-row">
            <input
              id="pay-amount"
              className="form-input"
              type="number"
              min="0"
              step={moneyStep(decimals)}
              value={values.amount}
              aria-invalid={invalid('amount')}
              onChange={event => set('amount', event.target.value)}
            />
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => set('amount', minorToInput(debt.balance, decimals))}
            >
              Saldo total
            </button>
          </div>
        </DebtField>

        <DebtField
          id="pay-date"
          label="Fecha del pago *"
          error={errors.paidOn}
          hint={`Desde la emisión (${formatDay(debt.issued_on)}) hasta hoy.`}
        >
          <input
            id="pay-date"
            className="form-input"
            type="date"
            min={debt.issued_on}
            max={today}
            value={values.paidOn}
            aria-invalid={invalid('paidOn')}
            onChange={event => set('paidOn', event.target.value)}
          />
        </DebtField>

        <DebtField id="pay-method" label="Método de pago *" error={errors.method}>
          <select
            id="pay-method"
            className="form-input"
            value={values.method}
            aria-invalid={invalid('method')}
            onChange={event => set('method', event.target.value as PaymentFormValues['method'])}
          >
            <option value="">Elige un método</option>
            {options.payment_methods.map(method => (
              <option key={method.value} value={method.value}>{method.label}</option>
            ))}
          </select>
        </DebtField>

        <DebtField
          id="pay-reference"
          label="Referencia o comprobante"
          error={errors.reference}
          hint="Opcional. Número de transferencia, recibo, cheque…"
        >
          <input
            id="pay-reference"
            className="form-input"
            type="text"
            value={values.reference}
            maxLength={LIMITS.reference + 20}
            aria-invalid={invalid('reference')}
            onChange={event => set('reference', event.target.value)}
          />
        </DebtField>

        <DebtField id="pay-notes" label="Observaciones" error={errors.notes} wide>
          <textarea
            id="pay-notes"
            className="form-input debts-textarea"
            rows={2}
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
          {submitting ? 'Registrando…' : 'Registrar pago'}
        </button>
      </div>
    </form>
  );
};