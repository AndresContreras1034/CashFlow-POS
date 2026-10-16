import { describe, expect, it } from 'vitest';
import {
  getPreviousReportRange,
  getReportRange,
  todayInTimezone,
  validateReportRange,
} from './reportRanges';

describe('report date ranges', () => {
  it('returns a local calendar date for valid zones and null for invalid zones', () => {
    expect(todayInTimezone('UTC')).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    expect(todayInTimezone('Invalid/Timezone')).toBeNull();
  });

  it('returns a single local day for the today preset', () => {
    expect(getReportRange('today', '2026-10-10')).toEqual({
      date_from: '2026-10-10',
      date_to: '2026-10-10',
    });
  });

  it('keeps presets correct across a year boundary', () => {
    expect(getReportRange('yesterday', '2026-01-01')).toEqual({
      date_from: '2025-12-31',
      date_to: '2025-12-31',
    });
    expect(getReportRange('last_7_days', '2026-01-01')).toEqual({
      date_from: '2025-12-26',
      date_to: '2026-01-01',
    });
    expect(getPreviousReportRange({
      date_from: '2026-01-01',
      date_to: '2026-01-07',
    })).toEqual({
      date_from: '2025-12-25',
      date_to: '2025-12-31',
    });
  });

  it('handles leap-year February', () => {
    expect(getReportRange('yesterday', '2024-03-01')).toEqual({
      date_from: '2024-02-29',
      date_to: '2024-02-29',
    });
    expect(getReportRange('this_month', '2024-02-12')).toEqual({
      date_from: '2024-02-01',
      date_to: '2024-02-29',
    });
    expect(getReportRange('last_30_days', '2024-02-29')).toEqual({
      date_from: '2024-01-31',
      date_to: '2024-02-29',
    });
  });

  it('uses December as the previous month in January', () => {
    expect(getReportRange('previous_month', '2026-01-10')).toEqual({
      date_from: '2025-12-01',
      date_to: '2025-12-31',
    });
  });

  it('validates range presence, date order and the backend maximum', () => {
    expect(validateReportRange('', '2026-10-11')).toBe('Selecciona las dos fechas del período.');
    expect(validateReportRange('2026-10-12', '2026-10-11'))
      .toBe('La fecha Desde no puede ser posterior a Hasta.');
    expect(validateReportRange('2026-02-30', '2026-03-01')).toBe('Ingresa fechas válidas.');
    expect(validateReportRange('2026-01-01', '2027-01-02'))
      .toBe('El rango no puede superar 366 días.');
    expect(validateReportRange('2026-01-01', '2027-01-01')).toBeNull();
  });
});
