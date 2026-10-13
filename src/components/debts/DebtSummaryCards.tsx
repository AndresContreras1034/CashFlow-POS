import React from 'react';
import type { DebtOptions, DebtSummary } from '../../types/debts';
import { DUE_GROUP_LABELS, type CategoryRow, type DueGroup } from '../../utils/debts';
import { formatMoney } from '../../utils/format';
import './DebtSummaryCards.css';

interface DebtSummaryCardsProps {
  summary: DebtSummary | null;
  error: string | null;
  activeGroup: DueGroup;
  /** Filtra el listado por vencidas / próximas a vencer. Volver a pulsar lo quita. */
  onSelectGroup: (group: DueGroup) => void;
  onRetry: () => void;
}

const plural = (n: number, one: string, many: string): string => `${n} ${n === 1 ? one : many}`;

export const DebtSummaryCards: React.FC<DebtSummaryCardsProps> = ({
  summary,
  error,
  activeGroup,
  onSelectGroup,
  onRetry,
}) => {
  const dash = '—';
  const toggle = (group: Exclude<DueGroup, ''>) => onSelectGroup(activeGroup === group ? '' : group);

  return (
    <section className="debts-summary" aria-label="Resumen de obligaciones">
      <div className="debts-card debts-card--primary">
        <span className="debts-card-label">Total pendiente</span>
        <span className="debts-card-value tabular">
          {summary ? formatMoney(summary.pending_total) : dash}
        </span>
        <span className="debts-card-sub">
          {summary ? `En ${plural(summary.active_count, 'obligación activa', 'obligaciones activas')}` : ' '}
        </span>
      </div>

      <button
        type="button"
        className={`debts-card debts-card--button ${summary && summary.overdue_count > 0 ? 'debts-card--danger' : ''}`}
        aria-pressed={activeGroup === 'overdue'}
        onClick={() => toggle('overdue')}
      >
        <span className="debts-card-label">Vencido</span>
        <span className="debts-card-value tabular">
          {summary ? formatMoney(summary.overdue_total) : dash}
        </span>
        <span className="debts-card-sub">
          {summary ? plural(summary.overdue_count, 'vencida', 'vencidas') : ' '}
          {activeGroup === 'overdue' ? ' · filtrando' : ''}
        </span>
      </button>

      <button
        type="button"
        className={`debts-card debts-card--button ${summary && summary.due_soon_count > 0 ? 'debts-card--warning' : ''}`}
        aria-pressed={activeGroup === 'soon'}
        onClick={() => toggle('soon')}
      >
        <span className="debts-card-label">{DUE_GROUP_LABELS.soon}</span>
        <span className="debts-card-value tabular">
          {summary ? formatMoney(summary.due_soon_total) : dash}
        </span>
        <span className="debts-card-sub">
          {summary ? plural(summary.due_soon_count, 'obligación', 'obligaciones') : ' '}
          {activeGroup === 'soon' ? ' · filtrando' : ''}
        </span>
      </button>

      <div className="debts-card">
        <span className="debts-card-label">Obligaciones activas</span>
        <span className="debts-card-value tabular">{summary ? summary.active_count : dash}</span>
        <span className="debts-card-sub">con saldo por pagar</span>
      </div>

      {error && (
        <p className="debts-summary-error" role="alert">
          No se pudo cargar el resumen: {error}{' '}
          <button type="button" className="debts-link" onClick={onRetry}>Reintentar</button>
        </p>
      )}
    </section>
  );
};

interface DebtCategoryStatsProps {
  rows: CategoryRow[] | null;
  options: DebtOptions;
}

/** Indicadores por categoría (activas, saldo, vencido, pagadas y anuladas). */
export const DebtCategoryStats: React.FC<DebtCategoryStatsProps> = ({ rows, options }) => (
  <details className="debts-categories">
    <summary>Indicadores por categoría</summary>
    {rows === null && <p className="debts-muted">Cargando…</p>}
    {rows !== null && rows.length === 0 && (
      <p className="debts-muted">No hay obligaciones para estos filtros.</p>
    )}
    {rows !== null && rows.length > 0 && (
      <div className="debts-table-wrap">
        <table className="debts-table">
          <caption className="debts-sr-only">Totales por categoría</caption>
          <thead>
            <tr>
              <th scope="col">Categoría</th>
              <th scope="col" className="debts-num">Activas</th>
              <th scope="col" className="debts-num">Saldo pendiente</th>
              <th scope="col" className="debts-num">Vencido</th>
              <th scope="col" className="debts-num">Pagadas</th>
              <th scope="col" className="debts-num">Anuladas</th>
            </tr>
          </thead>
          <tbody>
            {rows.map(row => (
              <tr key={row.category}>
                <th scope="row">
                  {options.categories.find(c => c.value === row.category)?.label ?? row.category}
                </th>
                <td className="debts-num tabular">{row.activeCount}</td>
                <td className="debts-num tabular">{formatMoney(row.pendingTotal)}</td>
                <td className="debts-num tabular">{formatMoney(row.overdueTotal)}</td>
                <td className="debts-num tabular">{row.paidCount}</td>
                <td className="debts-num tabular">{row.voidedCount}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    )}
  </details>
);