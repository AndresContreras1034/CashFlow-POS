import React from 'react';
import type { AuditEvent } from '../../types';
import {
  actionLabel,
  formatActor,
  formatAuditDate,
  formatEntity,
  moduleLabel,
  OUTCOME_LABELS,
} from '../../utils/audit';

interface AuditTableProps {
  events: AuditEvent[];
  timeZone: string;
  /** Si se pasa, cada resumen es seleccionable. */
  onSelect?: (event: AuditEvent) => void;
}

export const AuditTable: React.FC<AuditTableProps> = ({ events, timeZone, onSelect }) => (
  <div className="audit-table-wrap">
    <table className="audit-table">
      <caption className="audit-sr-only">Eventos de auditoría</caption>
      <thead>
        <tr>
          <th scope="col">Fecha</th>
          <th scope="col">Módulo</th>
          <th scope="col">Acción</th>
          <th scope="col">Entidad</th>
          <th scope="col">Resumen</th>
          <th scope="col">Actor</th>
          <th scope="col">Resultado</th>
        </tr>
      </thead>
      <tbody>
        {events.map(event => (
          <tr
            key={event.id}
            className={event.outcome === 'failure' ? 'audit-row--failure' : undefined}
          >
            <td className="audit-nowrap">{formatAuditDate(event.created_at, timeZone)}</td>
            <td>{moduleLabel(event.module)}</td>
            <td>{actionLabel(event.module, event.action)}</td>
            <td>{formatEntity(event)}</td>
            <td className="audit-summary">
              {onSelect ? (
                <button type="button" className="audit-link" onClick={() => onSelect(event)}>
                  {event.summary}
                </button>
              ) : (
                event.summary
              )}
            </td>
            <td>{formatActor(event.actor)}</td>
            <td>
              <span className={`audit-badge audit-badge--${event.outcome}`}>
                {OUTCOME_LABELS[event.outcome]}
              </span>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  </div>
);