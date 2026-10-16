import { describe, expect, it } from 'vitest';
import {
  agingLabel,
  debtCategoryLabel,
  debtUrgencyLabel,
  fillHeatmap,
  formatBps,
  formatDaysOfCover,
  formatDaysSinceSale,
  formatSalesBucketLabel,
  inventoryDaysReasonLabel,
  formatDueDays,
  formatTurnover,
  formatVariantAttributes,
  stockStatusLabel,
} from './reportFormat';

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

  it('formats variant attributes defensively and reuses inventory labels', () => {
    expect(formatVariantAttributes({ volumen: '100ml' })).toBe('Volumen: 100ml');
    expect(formatVariantAttributes({ talla: 4, color: 'Azul' })).toBe('Talla: 4 · Color: Azul');
    expect(formatVariantAttributes({
      talla: 4,
      color: 'Azul',
      visible: true,
      empty: '  ',
      missing: null,
    })).toBe('Talla: 4 · Color: Azul · Visible: true');
    expect(formatVariantAttributes(null)).toBe('');
    expect(formatVariantAttributes({})).toBe('');
    expect(formatVariantAttributes('100ml')).toBe('');
  });

  it('formats days since last sale including never sold', () => {
    expect(formatDaysSinceSale(null)).toBe('Nunca vendido');
    expect(formatDaysSinceSale(1)).toBe('1 día');
    expect(formatDaysSinceSale(12)).toBe('12 días');
  });

  it('formats inventory turnover ratios in times', () => {
    expect(formatTurnover(15_000)).toBe('1,50 veces');
    expect(formatTurnover(0)).toBe('0,00 veces');
    expect(formatTurnover(null)).toBe('—');
  });

  it('labels monthly chart buckets and marks partial months', () => {
    expect(formatSalesBucketLabel('2026-10-01', true, true)).toBe('oct 2026 (parcial)');
    expect(formatSalesBucketLabel('2026-10-01', false, true)).toBe('oct 2026');
    expect(formatSalesBucketLabel('2026-10-15', false, false)).toBe('10-15');
  });

  it('formats stock coverage and status labels', () => {
    expect(formatDaysOfCover(null)).toBe('Sin ventas');
    expect(formatDaysOfCover(1)).toBe('1 día');
    expect(formatDaysOfCover(12)).toBe('12 días');
    expect(stockStatusLabel('out_of_stock')).toBe('Agotado');
    expect(stockStatusLabel('low')).toBe('Bajo');
  });

  it('explains why inventory days are not available', () => {
    expect(inventoryDaysReasonLabel('range_too_short')).toBe(
      'El rango debe cubrir al menos 28 días',
    );
    expect(inventoryDaysReasonLabel('no_cogs')).toBe('No hay costo de ventas en el período');
    expect(inventoryDaysReasonLabel('low_cost_coverage')).toBe(
      'Menos del 80 % de las unidades vendidas tiene costo',
    );
    expect(inventoryDaysReasonLabel('implausible_result')).toBe(
      'El resultado supera el máximo plausible de 3.650 días',
    );
  });

  it('formats debt due days and labels with the debt catalog wording', () => {
    expect(formatDueDays(-1)).toBe('hace 1 día');
    expect(formatDueDays(-12)).toBe('hace 12 días');
    expect(formatDueDays(0)).toBe('hoy');
    expect(formatDueDays(1)).toBe('en 1 día');
    expect(formatDueDays(12)).toBe('en 12 días');

    expect(agingLabel('overdue_1_30')).toBe('Vencidas 1–30 días');
    expect(agingLabel('no_due_date')).toBe('Sin fecha de vencimiento');
    expect(debtUrgencyLabel('overdue')).toBe('Vencida');
    expect(debtUrgencyLabel('due_today')).toBe('Hoy');
    expect(debtUrgencyLabel('alert')).toBe('Alerta');
    expect(debtUrgencyLabel('ok')).toBe('Ok');
    expect(debtCategoryLabel('maintenance')).toBe('Mantenimiento');
    expect(debtCategoryLabel('business_operation')).toBe('Operación del negocio');
    expect(debtCategoryLabel('fixed_services')).toBe('Servicios y gastos fijos');
    expect(debtCategoryLabel('other_obligations')).toBe('Otras obligaciones');
  });
});
