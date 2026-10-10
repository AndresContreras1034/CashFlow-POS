import { describe, expect, it } from 'vitest';
import { parseSaleNumber, presetRange } from './salesFilters';

describe('sales filter utilities', () => {
  it('uses local calendar days for presets', () => {
    const now = new Date(2025, 0, 2, 23, 45);
    expect(presetRange('today', now)).toEqual({ from: '2025-01-02', to: '2025-01-02' });
    expect(presetRange('yesterday', now)).toEqual({ from: '2025-01-01', to: '2025-01-01' });
    expect(presetRange('week', now)).toEqual({ from: '2024-12-27', to: '2025-01-02' });
  });

  it.each([['12', 12], ['#12', 12], ['T-000042', 42], ['x', null], ['0', null]])(
    'parses %s',
    (value, expected) => expect(parseSaleNumber(value)).toBe(expected)
  );
});
