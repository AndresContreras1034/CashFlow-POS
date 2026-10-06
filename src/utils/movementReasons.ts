import type { MovementReason } from '../types';

export const REASON_LABELS: Record<MovementReason, string> = {
  damaged: 'Dañado',
  lost: 'Perdido',
  expired: 'Vencido',
  theft: 'Robo',
  internal_use: 'Uso interno',
  count_correction: 'Corrección de conteo',
  other: 'Otro',
};

export const ALL_REASONS: MovementReason[] = [
  'damaged',
  'lost',
  'expired',
  'theft',
  'internal_use',
  'count_correction',
  'other',
];

export const INCREASE_REASONS: MovementReason[] = ['count_correction', 'other'];

export const formatReason = (reason: MovementReason | null | undefined): string =>
  reason ? REASON_LABELS[reason] : '—';
