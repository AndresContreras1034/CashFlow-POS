import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  getDebt,
  listDebtHistory,
  voidDebt,
  voidDebtPayment,
} from '../../services/debts.service';
import type { AuditEvent } from '../../types';
import type {
  DebtDetail as DebtDetailData,
  DebtOptions,
  DebtOverview,
  DebtPayment,
} from '../../types/debts';
import {
  actionLabel,
  buildChangeRows,
  entityTypeLabel,
  errorMessage,
  formatActor,
  formatAuditDate,
} from '../../utils/audit';
import { canPay, canVoidDebt, formatDay, labelOf, validateReason } from '../../utils/debts';
import { formatMoney } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import type { ToastType } from '../ui/Toast';
import { DueBadge, StatusBadge } from './DebtBadges';
import { PaymentForm } from './PaymentForm';
import './DebtDetail.css';

const money = (minor: number): string => formatMoney(minor);

type Panel =
  | { kind: 'none' }
  | { kind: 'pay' }
  | { kind: 'void-debt' }
  | { kind: 'void-payment'; payment: DebtPayment };

interface ReasonPanelProps {
  title: string;
  confirmLabel: string;
  onConfirm: (reason: string) => Promise<void>;
  onCancel: () => void;
}

/** Pide el motivo de una anulación. Es obligatorio y queda en el historial. */
const ReasonPanel: React.FC<ReasonPanelProps> = ({ title, confirmLabel, onConfirm, onCancel }) => {
  const [reason, setReason] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const submittingRef = useRef(false);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (submittingRef.current) return;
    const problem = validateReason(reason);
    if (problem) {
      setError(problem);
      return;
    }
    submittingRef.current = true;
    setSubmitting(true);
    setError(null);
    try {
      await onConfirm(reason.trim());
    } catch (cause) {
      setError(errorMessage(cause));
      submittingRef.current = false;
      setSubmitting(false);
    }
  };

  return (
    <form className="debts-reason" onSubmit={submit} noValidate>
      <h3>{title}</h3>
      <div className="form-field">
        <label htmlFor="debt-void-reason">Motivo *</label>
        <textarea
          id="debt-void-reason"
          className="form-input debts-textarea"
          rows={2}
          value={reason}
          aria-invalid={error ? true : undefined}
          onChange={event => {
            setReason(event.target.value);
            setError(null);
          }}
        />
        {error && <span className="debts-field-error" role="alert">{error}</span>}
      </div>
      <div className="debts-form-actions">
        <button type="button" className="btn btn-ghost" onClick={onCancel} disabled={submitting}>
          Cancelar
        </button>
        <button type="submit" className="btn btn-danger" disabled={submitting}>
          {submitting ? 'Anulando…' : confirmLabel}
        </button>
      </div>
    </form>
  );
};

interface DebtDetailProps {
  debtId: number;
  options: DebtOptions;
  today: string;
  decimals: number;
  timeZone: string;
  /** Abre directamente el formulario de pago (acción «Abonar» del listado). */
  startWithPayment?: boolean;
  onEdit: (debt: DebtOverview) => void;
  /** La deuda cambió: el listado y los totales deben recargarse. */
  onChanged: () => void;
  onNotify: (message: string, type?: ToastType) => void;
}

const Row: React.FC<{ label: string; children: React.ReactNode }> = ({ label, children }) => (
  <>
    <dt>{label}</dt>
    <dd>{children}</dd>
  </>
);

export const DebtDetail: React.FC<DebtDetailProps> = ({
  debtId,
  options,
  today,
  decimals,
  timeZone,
  startWithPayment,
  onEdit,
  onChanged,
  onNotify,
}) => {
  const [detail, setDetail] = useState<DebtDetailData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const [panel, setPanel] = useState<Panel>(startWithPayment ? { kind: 'pay' } : { kind: 'none' });
  const requestRef = useRef(0);
  const rootRef = useRef<HTMLDivElement>(null);

  const [historyOpen, setHistoryOpen] = useState(false);
  const [history, setHistory] = useState<AuditEvent[] | null>(null);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [historyToken, setHistoryToken] = useState(0);
  const historyRequest = useRef(0);

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    rootRef.current?.focus();
    return () => {
      if (previous && previous.isConnected) previous.focus();
    };
  }, []);

  useEffect(() => {
    const requestId = ++requestRef.current;
    setLoading(true);
    setError(null);
    getDebt(debtId)
      .then(response => {
        if (requestId !== requestRef.current) return;
        setDetail(response);
        setLoading(false);
      })
      .catch(cause => {
        if (requestId !== requestRef.current) return;
        setDetail(null);
        setError(errorMessage(cause));
        setLoading(false);
      });
    return () => {
      requestRef.current += 1;
    };
  }, [debtId, reloadToken]);

  useEffect(() => {
    if (!historyOpen) return;
    const requestId = ++historyRequest.current;
    setHistoryError(null);
    listDebtHistory(debtId)
      .then(events => {
        if (requestId === historyRequest.current) setHistory(events);
      })
      .catch(cause => {
        if (requestId === historyRequest.current) setHistoryError(errorMessage(cause));
      });
    return () => {
      historyRequest.current += 1;
    };
  }, [debtId, historyOpen, historyToken]);

  /** Una operación terminó bien: se reemplaza el estado con lo que devolvió el backend. */
  const applyResult = useCallback(
    (result: DebtDetailData, message: string) => {
      setDetail(result);
      setPanel({ kind: 'none' });
      setHistoryToken(token => token + 1);
      onChanged();
      onNotify(message);
    },
    [onChanged, onNotify],
  );

  const statusLabel = (debt: DebtOverview) => labelOf(options.statuses, debt.status);

  const renderPayment = (debt: DebtDetailData, payment: DebtPayment) => {
    const voided = payment.voided_at !== null;
    return (
      <tr key={payment.id} className={voided ? 'debts-row--voided' : undefined}>
        <td className="debts-nowrap">{formatDay(payment.paid_on)}</td>
        <td className="debts-num tabular">{formatMoney(payment.amount)}</td>
        <td>{labelOf(options.payment_methods, payment.method)}</td>
        <td>{payment.reference ?? '—'}</td>
        <td>
          {formatActor(payment.created_by)}
          <span className="debts-sub">{formatAuditDate(payment.created_at, timeZone)}</span>
          {payment.notes && <span className="debts-sub">{payment.notes}</span>}
        </td>
        <td>
          {voided ? (
            <>
              <span className="debts-status debts-status--voided">Anulado</span>
              <span className="debts-sub">
                {payment.void_reason} · {formatActor(payment.voided_by)}
              </span>
            </>
          ) : (
            <span className="debts-status debts-status--paid">Vigente</span>
          )}
        </td>
        <td className="debts-actions">
          {!voided && debt.status !== 'voided' && (
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => setPanel({ kind: 'void-payment', payment })}
            >
              Anular
            </button>
          )}
        </td>
      </tr>
    );
  };

  return (
    <div className="debts-detail" ref={rootRef} tabIndex={-1}>
      {loading && !detail && <p className="debts-muted" role="status">Cargando…</p>}

      {error && (
        <div className="debts-error-block" role="alert">
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

      {detail && (
        <>
          <div className="debts-detail-head">
            <StatusBadge status={detail.status} label={statusLabel(detail)} />
            <DueBadge debt={detail} />
          </div>

          <div className="debts-amounts">
            <div>
              <span className="debts-card-label">Importe original</span>
              <strong className="tabular">{formatMoney(detail.original_amount)}</strong>
            </div>
            <div>
              <span className="debts-card-label">Pagado</span>
              <strong className="tabular">{formatMoney(detail.paid_amount)}</strong>
            </div>
            <div>
              <span className="debts-card-label">Saldo</span>
              <strong className="tabular">{formatMoney(detail.balance)}</strong>
            </div>
          </div>

          <dl className="debts-dl">
            <Row label="Acreedor">{detail.creditor_name}</Row>
            <Row label="Concepto">{detail.concept}</Row>
            <Row label="Categoría">
              {options.categories.find(item => item.value === detail.category)?.label ?? detail.category}
              {detail.subcategory && ` · ${detail.subcategory}`}
            </Row>
            <Row label="Naturaleza">{labelOf(options.natures, detail.nature)}</Row>
            <Row label="Referencia">{detail.document_ref ?? '—'}</Row>
            <Row label="Emisión">{formatDay(detail.issued_on)}</Row>
            <Row label="Vencimiento">{formatDay(detail.due_on)}</Row>
            <Row label="Registrada">
              {formatAuditDate(detail.created_at, timeZone)} · {formatActor(detail.created_by)}
            </Row>
            {detail.notes && <Row label="Observaciones">{detail.notes}</Row>}
            {detail.voided_at && (
              <Row label="Anulada">
                {formatAuditDate(detail.voided_at, timeZone)} · {formatActor(detail.voided_by)}
                <span className="debts-sub">Motivo: {detail.void_reason}</span>
              </Row>
            )}
          </dl>

          {panel.kind === 'none' && detail.status !== 'voided' && (
            <div className="debts-detail-actions">
              {canPay(detail) && (
                <button type="button" className="btn btn-primary" onClick={() => setPanel({ kind: 'pay' })}>
                  Registrar pago
                </button>
              )}
              <button type="button" className="btn btn-ghost" onClick={() => onEdit(detail)}>
                Editar
              </button>
              <button
                type="button"
                className="btn btn-danger"
                disabled={!canVoidDebt(detail)}
                title={
                  canVoidDebt(detail)
                    ? undefined
                    : 'Para anular la deuda, anula primero cada pago vigente'
                }
                onClick={() => setPanel({ kind: 'void-debt' })}
              >
                Anular deuda
              </button>
              {!canVoidDebt(detail) && (
                <span className="debts-hint">Anula primero los pagos vigentes para poder anular la deuda.</span>
              )}
            </div>
          )}

          {panel.kind === 'pay' && canPay(detail) && (
            <section className="debts-panel" aria-label="Registrar pago">
              <h3>Registrar pago</h3>
              <PaymentForm
                debt={detail}
                options={options}
                today={today}
                decimals={decimals}
                onSaved={saved => applyResult(saved, 'Pago registrado')}
                onCancel={() => setPanel({ kind: 'none' })}
              />
            </section>
          )}

          {panel.kind === 'void-debt' && (
            <section className="debts-panel" aria-label="Anular deuda">
              <ReasonPanel
                title="Anular deuda"
                confirmLabel="Anular deuda"
                onCancel={() => setPanel({ kind: 'none' })}
                onConfirm={async reason => {
                  const result = await voidDebt(detail.id, { reason, ...actorField('voided_by') });
                  applyResult(result, 'Deuda anulada');
                }}
              />
            </section>
          )}

          {panel.kind === 'void-payment' && (
            <section className="debts-panel" aria-label="Anular pago">
              <ReasonPanel
                title={`Anular el pago de ${formatMoney(panel.payment.amount)} (${formatDay(panel.payment.paid_on)})`}
                confirmLabel="Anular pago"
                onCancel={() => setPanel({ kind: 'none' })}
                onConfirm={async reason => {
                  const result = await voidDebtPayment(panel.payment.id, {
                    reason,
                    ...actorField('voided_by'),
                  });
                  applyResult(result, 'Pago anulado');
                }}
              />
            </section>
          )}

          <section className="debts-detail-section">
            <h3>Pagos</h3>
            {detail.payments.length === 0 ? (
              <p className="debts-muted">Esta deuda aún no tiene pagos.</p>
            ) : (
              <div className="debts-table-wrap">
                <table className="debts-table">
                  <thead>
                    <tr>
                      <th scope="col">Fecha</th>
                      <th scope="col" className="debts-num">Importe</th>
                      <th scope="col">Método</th>
                      <th scope="col">Referencia</th>
                      <th scope="col">Registrado</th>
                      <th scope="col">Estado</th>
                      <th scope="col"><span className="debts-sr-only">Acciones</span></th>
                    </tr>
                  </thead>
                  <tbody>{detail.payments.map(payment => renderPayment(detail, payment))}</tbody>
                </table>
              </div>
            )}
          </section>

          <details
            className="debts-detail-section"
            open={historyOpen}
            onToggle={event => setHistoryOpen((event.target as HTMLDetailsElement).open)}
          >
            <summary>Historial de modificaciones y anulaciones</summary>
            {historyOpen && history === null && !historyError && (
              <p className="debts-muted" role="status">Cargando historial…</p>
            )}
            {historyError && (
              <div className="debts-error-block" role="alert">
                <p>{historyError}</p>
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => setHistoryToken(token => token + 1)}
                >
                  Reintentar
                </button>
              </div>
            )}
            {history !== null && history.length === 0 && (
              <p className="debts-muted">Sin eventos registrados.</p>
            )}
            {history !== null && history.length > 0 && (
              <ol className="debts-history">
                {history.map(event => {
                  const rows = buildChangeRows(event, money);
                  const reason = event.metadata?.reason;
                  return (
                    <li key={event.id}>
                      <div className="debts-history-head">
                        <strong>
                          {event.entity_type ? `${entityTypeLabel(event.entity_type)} · ` : ''}
                          {actionLabel('debts', event.action)}
                        </strong>
                        <span className="debts-sub">
                          {formatAuditDate(event.created_at, timeZone)} · {formatActor(event.actor)}
                        </span>
                      </div>
                      <p>{event.summary}</p>
                      {typeof reason === 'string' && reason !== '' && (
                        <p className="debts-sub">Motivo: {reason}</p>
                      )}
                      {rows.length > 0 && (
                        <ul className="debts-changes">
                          {rows.map(row => (
                            <li key={row.key}>
                              <span>{row.label}:</span> {row.from} → {row.to}
                            </li>
                          ))}
                        </ul>
                      )}
                    </li>
                  );
                })}
              </ol>
            )}
          </details>
        </>
      )}
    </div>
  );
};