import React, { useEffect, useRef, useState } from 'react';
import { useDebounce } from '../../hooks/useDebounce';
import type { DebtOptions } from '../../types/debts';
import {
  DEBT_SORT_OPTIONS,
  DUE_GROUP_LABELS,
  LIMITS,
  type DebtFilterState,
  type DueGroup,
} from '../../utils/debts';
import './DebtFilters.css';

interface DebouncedTextProps {
  id: string;
  label: string;
  value: string;
  placeholder?: string;
  onCommit: (value: string) => void;
}

/** Input de texto que confirma el valor tras 400 ms sin teclear. */
const DebouncedText: React.FC<DebouncedTextProps> = ({ id, label, value, placeholder, onCommit }) => {
  const [local, setLocal] = useState(value);
  const lastCommitted = useRef(value);
  const debounced = useDebounce(local, 400);

  useEffect(() => {
    if (value !== lastCommitted.current) {
      lastCommitted.current = value;
      setLocal(value);
    }
  }, [value]);

  useEffect(() => {
    const next = debounced.trim();
    if (next !== lastCommitted.current) {
      lastCommitted.current = next;
      onCommit(next);
    }
  }, [debounced, onCommit]);

  return (
    <div className="debts-field">
      <label htmlFor={id}>{label}</label>
      <input
        id={id}
        className="debts-input"
        type="text"
        value={local}
        placeholder={placeholder}
        maxLength={LIMITS.search}
        onChange={event => setLocal(event.target.value)}
      />
    </div>
  );
};

interface DebtFiltersProps {
  filter: DebtFilterState;
  options: DebtOptions;
  rangeError: string | null;
  hasFilters: boolean;
  onChange: (patch: Partial<DebtFilterState>, replace?: boolean) => void;
  onReset: () => void;
}

const DATE_FIELDS: { id: string; label: string; key: keyof DebtFilterState }[] = [
  { id: 'debts-issued-from', label: 'Emisión desde', key: 'issuedFrom' },
  { id: 'debts-issued-to', label: 'Emisión hasta', key: 'issuedTo' },
  { id: 'debts-due-from', label: 'Vencimiento desde', key: 'dueFrom' },
  { id: 'debts-due-to', label: 'Vencimiento hasta', key: 'dueTo' },
  { id: 'debts-paid-from', label: 'Pago desde', key: 'paidFrom' },
  { id: 'debts-paid-to', label: 'Pago hasta', key: 'paidTo' },
];

export const DebtFilters: React.FC<DebtFiltersProps> = ({
  filter,
  options,
  rangeError,
  hasFilters,
  onChange,
  onReset,
}) => {
  const hasDates = DATE_FIELDS.some(field => String(filter[field.key]) !== '');
  const [datesOpen, setDatesOpen] = useState(hasDates);

  return (
    <section className="debts-filters" aria-label="Filtros de deudas">
      <div className="debts-filters-grid">
        <DebouncedText
          id="debts-search"
          label="Buscar"
          value={filter.search}
          placeholder="Acreedor, concepto o referencia…"
          onCommit={search => onChange({ search })}
        />

        <div className="debts-field">
          <label htmlFor="debts-category">Categoría</label>
          <select
            id="debts-category"
            className="debts-input"
            value={filter.category}
            onChange={event => onChange({ category: event.target.value })}
          >
            <option value="">Todas</option>
            {options.categories.map(category => (
              <option key={category.value} value={category.value}>{category.label}</option>
            ))}
          </select>
        </div>

        <div className="debts-field">
          <label htmlFor="debts-nature">Naturaleza</label>
          <select
            id="debts-nature"
            className="debts-input"
            value={filter.nature}
            onChange={event => onChange({ nature: event.target.value })}
          >
            <option value="">Todas</option>
            {options.natures.map(nature => (
              <option key={nature.value} value={nature.value}>{nature.label}</option>
            ))}
          </select>
        </div>

        <div className="debts-field">
          <label htmlFor="debts-status">Estado</label>
          <select
            id="debts-status"
            className="debts-input"
            value={filter.status}
            onChange={event => onChange({ status: event.target.value })}
          >
            <option value="">Todos</option>
            {options.statuses.map(status => (
              <option key={status.value} value={status.value}>{status.label}</option>
            ))}
          </select>
        </div>

        <div className="debts-field">
          <label htmlFor="debts-due">Vencimiento</label>
          <select
            id="debts-due"
            className="debts-input"
            value={filter.due}
            onChange={event => onChange({ due: event.target.value as DueGroup })}
          >
            <option value="">Todos</option>
            {(Object.keys(DUE_GROUP_LABELS) as Exclude<DueGroup, ''>[]).map(group => (
              <option key={group} value={group}>{DUE_GROUP_LABELS[group]}</option>
            ))}
          </select>
        </div>

        <div className="debts-field">
          <label htmlFor="debts-sort">Ordenar por</label>
          <select
            id="debts-sort"
            className="debts-input"
            value={filter.sortBy}
            onChange={event =>
              onChange({ sortBy: event.target.value as DebtFilterState['sortBy'] })
            }
          >
            {DEBT_SORT_OPTIONS.map(option => (
              <option key={option.value} value={option.value}>{option.label}</option>
            ))}
          </select>
        </div>

        <div className="debts-field">
          <label htmlFor="debts-sort-dir">Sentido</label>
          <select
            id="debts-sort-dir"
            className="debts-input"
            value={filter.sortDir}
            onChange={event => onChange({ sortDir: event.target.value === 'asc' ? 'asc' : 'desc' })}
          >
            <option value="desc">Descendente</option>
            <option value="asc">Ascendente</option>
          </select>
        </div>
      </div>

      <details
        className="debts-dates"
        open={datesOpen}
        onToggle={event => setDatesOpen((event.target as HTMLDetailsElement).open)}
      >
        <summary>Filtrar por fechas (emisión, vencimiento y pago)</summary>
        <div className="debts-filters-grid">
          {DATE_FIELDS.map(field => (
            <div className="debts-field" key={field.id}>
              <label htmlFor={field.id}>{field.label}</label>
              <input
                id={field.id}
                className="debts-input"
                type="date"
                value={String(filter[field.key])}
                aria-invalid={rangeError ? true : undefined}
                onChange={event => onChange({ [field.key]: event.target.value })}
              />
            </div>
          ))}
        </div>
      </details>

      {rangeError && <p className="debts-error-text" role="alert">{rangeError}</p>}

      <div className="debts-filters-actions">
        <button
          type="button"
          className={`btn ${filter.due === 'attention' ? 'btn-primary' : 'btn-ghost'}`}
          aria-pressed={filter.due === 'attention'}
          onClick={() => onChange({ due: filter.due === 'attention' ? '' : 'attention' })}
        >
          {DUE_GROUP_LABELS.attention}
        </button>
        <button type="button" className="btn btn-ghost" onClick={onReset} disabled={!hasFilters}>
          Limpiar filtros
        </button>
      </div>
    </section>
  );
};