import React, { useEffect, useRef, useState } from 'react';
import { useDebounce } from '../../hooks/useDebounce';
import {
  AUDIT_CATEGORIES,
  AUDIT_MODULES,
  AUDIT_OUTCOMES,
  CATEGORY_LABELS,
  MODULE_LABELS,
  OUTCOME_LABELS,
  type AuditFilterState,
} from '../../utils/audit';

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
    <div className="audit-field">
      <label htmlFor={id}>{label}</label>
      <input
        id={id}
        className="audit-input"
        type="text"
        value={local}
        placeholder={placeholder}
        onChange={event => setLocal(event.target.value)}
      />
    </div>
  );
};

interface AuditFiltersProps {
  filter: AuditFilterState;
  dateError: string | null;
  hasFilters: boolean;
  onChange: (patch: Partial<AuditFilterState>, replace?: boolean) => void;
  onReset: () => void;
}

export const AuditFilters: React.FC<AuditFiltersProps> = ({
  filter,
  dateError,
  hasFilters,
  onChange,
  onReset,
}) => (
  <section className="audit-filters" aria-label="Filtros de auditoría">
    <div className="audit-filters-grid">
      <DebouncedText
        id="audit-search"
        label="Buscar"
        value={filter.search}
        placeholder="Resumen, acción, id o error…"
        onCommit={search => onChange({ search })}
      />

      <div className="audit-field">
        <label htmlFor="audit-module">Módulo</label>
        <select
          id="audit-module"
          className="audit-input"
          value={filter.module}
          onChange={event => onChange({ module: event.target.value })}
        >
          <option value="">Todos</option>
          {AUDIT_MODULES.map(module => (
            <option key={module} value={module}>{MODULE_LABELS[module]}</option>
          ))}
        </select>
      </div>

      <div className="audit-field">
        <label htmlFor="audit-category">Categoría</label>
        <select
          id="audit-category"
          className="audit-input"
          value={filter.category}
          onChange={event => onChange({ category: event.target.value })}
        >
          <option value="">Todas</option>
          {AUDIT_CATEGORIES.map(category => (
            <option key={category} value={category}>{CATEGORY_LABELS[category]}</option>
          ))}
        </select>
      </div>

      <div className="audit-field">
        <label htmlFor="audit-outcome">Resultado</label>
        <select
          id="audit-outcome"
          className="audit-input"
          value={filter.outcome}
          onChange={event => onChange({ outcome: event.target.value })}
        >
          <option value="">Todos</option>
          {AUDIT_OUTCOMES.map(outcome => (
            <option key={outcome} value={outcome}>{OUTCOME_LABELS[outcome]}</option>
          ))}
        </select>
      </div>

      <DebouncedText
        id="audit-action"
        label="Acción"
        value={filter.action}
        placeholder="Ej: open, stock_adjust"
        onCommit={action => onChange({ action })}
      />

      <DebouncedText
        id="audit-entity-type"
        label="Tipo de entidad"
        value={filter.entityType}
        placeholder="Ej: variant, sale"
        onCommit={entityType => onChange({ entityType })}
      />

      <DebouncedText
        id="audit-entity-id"
        label="Id de entidad"
        value={filter.entityId}
        placeholder="Ej: 12"
        onCommit={entityId => onChange({ entityId })}
      />

      <div className="audit-field">
        <label htmlFor="audit-date-from">Desde</label>
        <input
          id="audit-date-from"
          className="audit-input"
          type="date"
          value={filter.dateFrom}
          aria-invalid={dateError ? true : undefined}
          onChange={event => onChange({ dateFrom: event.target.value })}
        />
      </div>

      <div className="audit-field">
        <label htmlFor="audit-date-to">Hasta</label>
        <input
          id="audit-date-to"
          className="audit-input"
          type="date"
          value={filter.dateTo}
          aria-invalid={dateError ? true : undefined}
          onChange={event => onChange({ dateTo: event.target.value })}
        />
      </div>

      <div className="audit-field">
        <label htmlFor="audit-sort">Orden</label>
        <select
          id="audit-sort"
          className="audit-input"
          value={filter.sortDir}
          onChange={event =>
            onChange({ sortDir: event.target.value === 'asc' ? 'asc' : 'desc' })
          }
        >
          <option value="desc">Más recientes primero</option>
          <option value="asc">Más antiguos primero</option>
        </select>
      </div>
    </div>

    {dateError && <p className="audit-error-text" role="alert">{dateError}</p>}

    <div className="audit-filters-actions">
      <button
        type="button"
        className={`btn ${filter.outcome === 'failure' ? 'btn-primary' : 'btn-ghost'}`}
        aria-pressed={filter.outcome === 'failure'}
        onClick={() => onChange({ outcome: filter.outcome === 'failure' ? '' : 'failure' })}
      >
        Solo errores
      </button>
      <button type="button" className="btn btn-ghost" onClick={onReset} disabled={!hasFilters}>
        Limpiar filtros
      </button>
    </div>
  </section>
);