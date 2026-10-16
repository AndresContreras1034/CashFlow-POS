import { describe, expect, it } from 'vitest';
import {
  getPreviousYearReportRange,
  getReportRange,
  getComparisonReportRange,
  todayInTimezone,
  validateReportRange,
} from './reportRanges';

describe('report date ranges', () => {
  it('returns a local calendar date for valid zones and null for invalid zones', () => {
    expect(todayInTimezone('UTC')).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    expect(todayInTimezone('Invalid/Timezone')).toBeNull();
  });

  it('returns a single local day for the today preset', () => {
    expect(getReportRange('last_30_days', '2026-10-15')).toEqual({
      date_from: '2026-09-16',
      date_to: '2026-10-15',
    });
  });

  it('supports year-to-date and previous-year presets', () => {
    expect(getReportRange('year_to_date', '2026-10-15')).toEqual({
      date_from: '2026-01-01',
      date_to: '2026-10-15',
    });
    expect(getReportRange('previous_year', '2026-10-15')).toEqual({
      date_from: '2025-01-01',
      date_to: '2025-12-31',
    });
  });

  it('handles leap-year February', () => {
    expect(getReportRange('last_12_months', '2028-02-29')).toEqual({
      date_from: '2027-03-01',
      date_to: '2028-02-29',
    });
  });

  it('builds same-period-last-year comparison and clamps leap day to February 28', () => {
    expect(getPreviousYearReportRange({
      date_from: '2028-02-29',
      date_to: '2028-03-10',
    })).toEqual({
      date_from: '2027-02-28',
      date_to: '2027-03-10',
    });
    expect(getComparisonReportRange({
      date_from: '2026-01-01',
      date_to: '2028-01-03',
    }, false)).toBeNull();
  });

  it('supports 7- and 90-day presets and the 732-day maximum', () => {
    expect(getReportRange('last_7_days', '2026-10-15')).toEqual({
      date_from: '2026-10-09',
      date_to: '2026-10-15',
    });
    expect(getReportRange('last_90_days', '2026-10-15')).toEqual({
      date_from: '2026-07-18',
      date_to: '2026-10-15',
    });
    expect(validateReportRange('2024-10-14', '2026-10-15')).toBeNull();
    expect(validateReportRange('2024-10-13', '2026-10-15')).toBe(
      'El rango no puede superar 732 días.',
    );
  });

  it('validates range presence, date order and the backend maximum', () => {
    expect(validateReportRange('', '2026-10-11')).toBe('Selecciona las dos fechas del período.');
    expect(validateReportRange('2026-10-12', '2026-10-11'))
      .toBe('La fecha Desde no puede ser posterior a Hasta.');
    expect(validateReportRange('2026-02-30', '2026-03-01')).toBe('Ingresa fechas válidas.');
    expect(validateReportRange('2026-01-01', '2028-01-03'))
      .toBe('El rango no puede superar 732 días.');
    expect(validateReportRange('2026-01-01', '2028-01-02')).toBeNull();
  });
});
