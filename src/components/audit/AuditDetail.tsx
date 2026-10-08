import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Modal } from '../ui/Modal';
import {
  getAuditEvent,
  listAuditEventsByCorrelation,
} from '../../services/audit.service';
import type { AuditEvent } from '../../types';
import {
  actionLabel,
  buildChangeRows,
  buildMetadataRows,
  CATEGORY_LABELS,
  errorMessage,
  formatActor,
  formatAuditDate,
  formatEntity,
  moduleLabel,
  OUTCOME_LABELS,
} from '../../utils/audit';
import { formatMoney } from '../../utils/format';

const money = (minor: number): string => formatMoney(minor);

const MAX_CORRELATION = 1000;

interface AuditDetailProps {
  eventId: number;
  timeZone: string;
  onClose: () => void;
}

const FieldRow: React.FC<{ label: string; children: React.ReactNode }> = ({ label, children }) => (
  <>
    <dt>{label}</dt>
    <dd>{children}</dd>
  </>
);

export const AuditDetail: React.FC<AuditDetailProps> = ({ eventId, timeZone, onClose }) => {
  const [currentId, setCurrentId] = useState(eventId);
  const [event, setEvent] = useState<AuditEvent | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const eventRequest = useRef(0);

  const [corrOpen, setCorrOpen] = useState(false);
  const [corr, setCorr] = useState<{ id: string; events: AuditEvent[] } | null>(null);
  const [corrLoading, setCorrLoading] = useState(false);
  const [corrError, setCorrError] = useState<string | null>(null);
  const corrRequest = useRef(0);

  const [copyState, setCopyState] = useState<'idle' | 'ok' | 'fail'>('idle');
  const copyTimer = useRef<number | undefined>(undefined);

  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    rootRef.current?.focus();
    return () => {
      if (previous && previous.isConnected) previous.focus();
    };
  }, []);

  useEffect(
    () => () => {
      eventRequest.current += 1;
      corrRequest.current += 1;
      window.clearTimeout(copyTimer.current);
    },
    [],
  );

  useEffect(() => {
    const requestId = ++eventRequest.current;
    setLoading(true);
    setError(null);
    getAuditEvent(currentId)
      .then(response => {
        if (requestId !== eventRequest.current) return;
        setEvent(response);
        setLoading(false);
      })
      .catch(cause => {
        if (requestId !== eventRequest.current) return;
        setEvent(null);
        setError(errorMessage(cause));
        setLoading(false);
      });
    return () => {
      eventRequest.current += 1;
    };
  }, [currentId, reloadToken]);

  useEffect(() => {
    if (event && corr && corr.id !== event.correlation_id) {
      setCorr(null);
      setCorrOpen(false);
    }
  }, [event, corr]);

  const loadCorrelation = useCallback((correlationId: string) => {
    const requestId = ++corrRequest.current;
    setCorrLoading(true);
    setCorrError(null);
    listAuditEventsByCorrelation(correlationId)
      .then(events => {
        if (requestId !== corrRequest.current) return;
        setCorr({ id: correlationId, events });
        setCorrLoading(false);
      })
      .catch(cause => {
        if (requestId !== corrRequest.current) return;
        setCorrError(errorMessage(cause));
        setCorrLoading(false);
      });
  }, []);

  const toggleCorrelation = () => {
    if (corrOpen) {
      setCorrOpen(false);
      return;
    }
    const correlationId = event?.correlation_id;
    if (!correlationId) return;
    setCorrOpen(true);
    if (corr?.id !== correlationId) loadCorrelation(correlationId);
  };

  const copyCorrelation = async (value: string) => {
    window.clearTimeout(copyTimer.current);
    try {
      await navigator.clipboard.writeText(value);
      setCopyState('ok');
    } catch {
      setCopyState('fail');
    }
    copyTimer.current = window.setTimeout(() => setCopyState('idle'), 2500);
  };

  const changeRows = event ? buildChangeRows(event, money) : [];
  const metadataRows = event ? buildMetadataRows(event, money) : [];

  const corrEvents = corr?.events ?? [];
  const corrIndex = corrEvents.findIndex(item => item.id === currentId);
  const previousInOperation = corrIndex > 0 ? corrEvents[corrIndex - 1] : null;
  const nextInOperation =
    corrIndex >= 0 && corrIndex < corrEvents.length - 1 ? corrEvents[corrIndex + 1] : null;

  return (
    <Modal title={`Evento #${currentId}`} onClose={onClose} width={760}>
      <div className="audit-detail" ref={rootRef} tabIndex={-1}>
        {loading && <p className="audit-status" role="status">Cargando…</p>}

        {error && (
          <div className="audit-error-block" role="alert">
            <p>{error}</p>
            <button
              type="button"
              className="btn btn-primary"
              onClick={() => setReloadToken(token => token + 1)}
            >
              Reintentar
            </button>
          </div>
        )}

        {event?.id === currentId && (
          <div className={loading ? 'audit-results--loading' : undefined}>
            {event.error_message && (
              <div className="audit-detail-error" role="alert">
                <strong>Mensaje de error</strong>
                <p>{event.error_message}</p>
              </div>
            )}

            <dl className="audit-dl">
              <FieldRow label="Fecha">
                {formatAuditDate(event.created_at, timeZone)}
                <span className="audit-muted"> · UTC: {event.created_at}</span>
              </FieldRow>
              <FieldRow label="Módulo">{moduleLabel(event.module)}</FieldRow>
              <FieldRow label="Categoría">
                {CATEGORY_LABELS[event.category] ?? event.category}
              </FieldRow>
              <FieldRow label="Acción">
                {actionLabel(event.module, event.action)}
                <span className="audit-muted"> · {event.action}</span>
              </FieldRow>
              <FieldRow label="Resultado">
                <span className={`audit-badge audit-badge--${event.outcome}`}>
                  {OUTCOME_LABELS[event.outcome] ?? event.outcome}
                </span>
              </FieldRow>
              <FieldRow label="Actor">{formatActor(event.actor)}</FieldRow>
              <FieldRow label="Entidad">
                {formatEntity(event)}
                {event.entity_type && (
                  <span className="audit-muted"> · {event.entity_type}</span>
                )}
              </FieldRow>
              <FieldRow label="Resumen">{event.summary}</FieldRow>
              <FieldRow label="Operación">
                {event.correlation_id ? (
                  <span className="audit-corr-id">
                    <code>{event.correlation_id}</code>
                    <button
                      type="button"
                      className="btn btn-ghost"
                      onClick={() => void copyCorrelation(event.correlation_id as string)}
                    >
                      Copiar
                    </button>
                    <span className="audit-muted" role="status" aria-live="polite">
                      {copyState === 'ok' && 'Copiado'}
                      {copyState === 'fail' && 'No se pudo copiar; selecciona el texto'}
                    </span>
                  </span>
                ) : (
                  '—'
                )}
              </FieldRow>
            </dl>

            {changeRows.length > 0 && (
              <section className="audit-detail-section">
                <h3>Cambios</h3>
                <div className="audit-table-wrap">
                  <table className="audit-table">
                    <thead>
                      <tr>
                        <th scope="col">Campo</th>
                        <th scope="col">Antes</th>
                        <th scope="col">Después</th>
                      </tr>
                    </thead>
                    <tbody>
                      {changeRows.map(row => (
                        <tr key={row.key}>
                          <th scope="row">{row.label}</th>
                          <td>{row.from}</td>
                          <td>{row.to}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </section>
            )}

            {metadataRows.length > 0 && (
              <section className="audit-detail-section">
                <h3>Detalles</h3>
                <div className="audit-table-wrap">
                  <table className="audit-table">
                    <tbody>
                      {metadataRows.map(row => (
                        <tr key={row.key}>
                          <th scope="row">{row.label}</th>
                          <td>{row.value}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </section>
            )}

            <details className="audit-detail-section">
              <summary>Ver JSON completo</summary>
              <pre className="audit-json">{JSON.stringify(event, null, 2)}</pre>
            </details>

            {event.correlation_id && (
              <div className="audit-filters-actions">
                <button
                  type="button"
                  className="btn btn-primary"
                  aria-expanded={corrOpen}
                  onClick={toggleCorrelation}
                >
                  {corrOpen ? 'Ocultar operación completa' : 'Ver operación completa'}
                </button>
              </div>
            )}
          </div>
        )}

        {corrOpen && (
          <section className="audit-detail-section" aria-label="Operación completa">
            <h3>Operación completa</h3>

            {corrLoading && <p className="audit-status" role="status">Cargando operación…</p>}

            {corrError && (
              <div className="audit-error-block" role="alert">
                <p>{corrError}</p>
                {event?.correlation_id && (
                  <button
                    type="button"
                    className="btn btn-primary"
                    onClick={() => loadCorrelation(event.correlation_id as string)}
                  >
                    Reintentar
                  </button>
                )}
              </div>
            )}

            {corr && !corrLoading && (
              <>
                <p className="audit-status">
                  {corrEvents.length} evento{corrEvents.length !== 1 ? 's' : ''}, del más antiguo al más reciente
                  {corrEvents.length >= MAX_CORRELATION && ` (se muestran como máximo ${MAX_CORRELATION})`}
                </p>
                <div className="audit-filters-actions">
                  <button
                    type="button"
                    className="btn btn-ghost"
                    disabled={!previousInOperation}
                    onClick={() => previousInOperation && setCurrentId(previousInOperation.id)}
                  >
                    ← Evento anterior
                  </button>
                  <button
                    type="button"
                    className="btn btn-ghost"
                    disabled={!nextInOperation}
                    onClick={() => nextInOperation && setCurrentId(nextInOperation.id)}
                  >
                    Evento siguiente →
                  </button>
                </div>
                <ol className="audit-corr-list">
                  {corrEvents.map(item => (
                    <li key={item.id}>
                      <button
                        type="button"
                        className={`audit-corr-item ${item.id === currentId ? 'audit-corr-item--current' : ''}`}
                        aria-current={item.id === currentId ? 'true' : undefined}
                        onClick={() => setCurrentId(item.id)}
                      >
                        <span className="audit-nowrap">
                          {formatAuditDate(item.created_at, timeZone)}
                        </span>
                        <span>{moduleLabel(item.module)} · {actionLabel(item.module, item.action)}</span>
                        <span className={`audit-badge audit-badge--${item.outcome}`}>
                          {OUTCOME_LABELS[item.outcome] ?? item.outcome}
                        </span>
                        <span className="audit-corr-summary">{item.summary}</span>
                      </button>
                    </li>
                  ))}
                </ol>
              </>
            )}
          </section>
        )}
      </div>
    </Modal>
  );
};