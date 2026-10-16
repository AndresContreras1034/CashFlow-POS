import React, { useState } from 'react';

interface Props {
  id: string;
  label: string;
  isSet: boolean;
  canRemove?: boolean;
  hint?: React.ReactNode;
  placeholder?: string;
  /** Devuelve true si se guardó bien (entonces se limpia el campo). */
  onSave: (value: string | null) => Promise<boolean>;
}

export const SecretField: React.FC<Props> = ({
  id,
  label,
  isSet,
  canRemove = true,
  hint,
  placeholder,
  onSave,
}) => {
  const [value, setValue] = useState('');
  const [busy, setBusy] = useState(false);

  const run = async (next: string | null) => {
    setBusy(true);
    try {
      if (await onSave(next)) setValue('');
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="form-field">
      <label className="form-label" htmlFor={id}>
        {label}{' '}
        <span className="settings-hint">
          {isSet ? '— guardada en este equipo ✓' : '— sin configurar'}
        </span>
      </label>
      <div style={{ display: 'flex', gap: 8 }}>
        <input
          id={id}
          className="form-input"
          type="password"
          autoComplete="new-password"
          value={value}
          placeholder={placeholder ?? (isSet ? 'Escribe una nueva para reemplazarla' : '')}
          onChange={event => setValue(event.target.value)}
          onKeyDown={event => {
            if (event.key === 'Enter' && value) {
              event.preventDefault();
              void run(value);
            }
          }}
        />
        <button
          type="button"
          className="btn btn-primary"
          disabled={busy || !value}
          onClick={() => void run(value)}
        >
          Guardar
        </button>
        {isSet && canRemove && (
          <button
            type="button"
            className="btn btn-ghost"
            disabled={busy}
            onClick={() => {
              if (window.confirm('¿Quitar esta contraseña guardada?')) void run(null);
            }}
          >
            Quitar
          </button>
        )}
      </div>
      {hint && <span className="settings-hint">{hint}</span>}
    </div>
  );
};
