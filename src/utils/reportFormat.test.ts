import { describe, expect, it } from 'vitest';
import { fillHeatmap, formatBps } from './reportFormat';

describe('report formatting helpers', () => {
  it('formats basis-point values with a sign and Spanish decimal separator', () => {
    expect(formatBps(1_380)).toBe('+13,8 %');
    expect(formatBps(-250)).toBe('-2,5 %');
    expect(formatBps(0)).toBe('0,0 %');
    expect(formatBps(null)).toBe('—');
  });

  it('fills missing weekday-hour pairs in a Monday-first 7 by 24 matrix', () => {
    const cells = fillHeatmap([
      { weekday: 0, hour: 9, tickets: 3, gross_sales: 25_000 },
      { weekday: 6, hour: 23, tickets: 1, gross_sales: 8_000 },
    ]);

    expect(cells).toHaveLength(7);
    expect(cells.every(row => row.length === 24)).toBe(true);
    expect(cells[0][9]).toEqual({
      weekday: 0,
      hour: 9,
      tickets: 3,
      gross_sales: 25_000,
    });
    expect(cells[0][0]).toEqual({
      weekday: 0,
      hour: 0,
      tickets: 0,
      gross_sales: 0,
    });
    expect(cells[6][23].gross_sales).toBe(8_000);
  });
});
