import React from 'react';
import type { DebtOverview, DebtStatus } from '../../types/debts';
import { dueIndicator } from '../../utils/debts';
import './DebtBadges.css';

/**
 * Vencimiento: texto y color vienen de `status`, `urgency` y `days_until_due`,
 * calculados por el backend con la fecha de hoy en la zona de Ajustes.
 * Nada de fechas ni umbrales se calcula aquí.
 */
export const DueBadge: React.FC<{
  debt: Pick<DebtOverview, 'status' | 'urgency' | 'days_until_due'>;
}> = ({ debt }) => {
  const indicator = dueIndicator(debt);
  return <span className={`debts-due debts-due--${indicator.tone}`}>{indicator.text}</span>;
};

export const StatusBadge: React.FC<{ status: DebtStatus; label: string }> = ({ status, label }) => (
  <span className={`debts-status debts-status--${status}`}>{label}</span>
);