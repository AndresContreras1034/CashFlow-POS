import React, { useEffect, useRef, useState } from 'react';
import { Modal } from '../ui/Modal';
import { CustomerForm } from './CustomerForm';
import { useDebounce } from '../../hooks/useDebounce';
import { listCustomers } from '../../services/customers.service';
import type { CustomerOverview } from '../../types/customers';
import { errorMessage } from '../../utils/audit';
import './CustomerPicker.css';

export interface CustomerRef {
  id: number;
  name: string;
  document?: string | null;
}

interface CustomerPickerProps {
  value: CustomerRef | null;
  onChange: (customer: CustomerRef | null) => void;
  disabled?: boolean;
}

const toRef = (customer: {
  id: number;
  name: string;
  document_type: string | null;
  document_number: string | null;
}): CustomerRef => ({
  id: customer.id,
  name: customer.name,
  document:
    customer.document_type && customer.document_number
      ? `${customer.document_type} ${customer.document_number}`
      : null,
});

export const CustomerPicker: React.FC<CustomerPickerProps> = ({
  value,
  onChange,
  disabled,
}) => {
  const [query, setQuery] = useState('');
  const debounced = useDebounce(query.trim(), 300);
  const [open, setOpen] = useState(false);
  const [results, setResults] = useState<CustomerOverview[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const requestRef = useRef(0);

  useEffect(() => {
    if (!open || value) return;
    const requestId = ++requestRef.current;
    setError(null);
    setLoading(true);
    listCustomers({ search: debounced || undefined, is_active: true, page_size: 8 })
      .then(response => {
        if (requestId === requestRef.current) setResults(response.data);
      })
      .catch(cause => {
        if (requestId === requestRef.current) setError(errorMessage(cause));
      })
      .finally(() => {
        if (requestId === requestRef.current) setLoading(false);
      });
    return () => {
      requestRef.current += 1;
    };
  }, [debounced, open, value]);

  const select = (customer: CustomerRef) => {
    onChange(customer);
    setQuery('');
    setOpen(false);
  };

  if (value) {
    return (
      <div className="customer-picker-chosen">
        <div>
          <strong>{value.name}</strong>
          {value.document && <span className="customer-picker-doc">{value.document}</span>}
        </div>
        <button
          type="button"
          className="btn"
          disabled={disabled}
          onClick={() => onChange(null)}
        >
          Quitar
        </button>
      </div>
    );
  }

  return (
    <div className="customer-picker">
      <input
        className="form-input"
        type="text"
        disabled={disabled}
        placeholder="Consumidor final · busca por nombre, documento o teléfono…"
        value={query}
        aria-expanded={open}
        aria-autocomplete="list"
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
        onChange={event => {
          setQuery(event.target.value);
          setOpen(true);
        }}
      />
      {open && (
        <ul className="customer-picker-list" onMouseDown={event => event.preventDefault()}>
          {loading && <li className="customer-picker-empty">Buscando…</li>}
          {error && <li className="customer-picker-empty" role="alert">{error}</li>}
          {!loading && !error && results.length === 0 && (
            <li className="customer-picker-empty">Sin resultados</li>
          )}
          {!error && results.map(customer => (
            <li
              key={customer.id}
              className="customer-picker-item"
              onClick={() => select(toRef(customer))}
            >
              <span>{customer.name}</span>
              <span className="customer-picker-doc">
                {customer.document_type && customer.document_number
                  ? `${customer.document_type} ${customer.document_number}`
                  : customer.phone ?? ''}
              </span>
            </li>
          ))}
          {!error && (
            <li
              className="customer-picker-item customer-picker-new"
              onClick={() => {
                setOpen(false);
                setCreating(true);
              }}
            >
              + Crear cliente{query.trim() ? ` «${query.trim()}»` : ''}
            </li>
          )}
        </ul>
      )}

      {creating && (
        <Modal title="Nuevo cliente" onClose={() => setCreating(false)} width={640}>
          <CustomerForm
            initial={null}
            presetName={query.trim()}
            onCancel={() => setCreating(false)}
            onSaved={saved => {
              setCreating(false);
              select(toRef(saved));
            }}
          />
        </Modal>
      )}
    </div>
  );
};
