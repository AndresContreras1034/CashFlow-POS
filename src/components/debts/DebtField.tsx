import React from 'react';

interface DebtFieldProps {
  id: string;
  label: string;
  error?: string;
  hint?: string;
  wide?: boolean;
  children: React.ReactNode;
}

/** Campo de formulario con etiqueta, ayuda y error. Usa las clases globales `form-field`. */
export const DebtField: React.FC<DebtFieldProps> = ({ id, label, error, hint, wide, children }) => (
  <div className={wide ? 'form-field debts-wide' : 'form-field'}>
    <label htmlFor={id}>{label}</label>
    {children}
    {hint && !error && <span className="debts-hint" id={`${id}-hint`}>{hint}</span>}
    {error && (
      <span className="debts-field-error" id={`${id}-error`} role="alert">
        {error}
      </span>
    )}
  </div>
);