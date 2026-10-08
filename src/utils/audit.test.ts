import { describe, expect, it } from 'vitest';
import type { AuditEvent } from '../types';
import {
  actionLabel,
  buildAuditFilter,
  buildChangeRows,
  buildMetadataRows,
  EMPTY_FILTER,
  filterToSearchParams,
  formatActor,
  formatAuditDate,
  formatAuditValue,
  formatEntity,
  hasActiveFilters,
  isMoneyField,
  searchParamsToFilter,
  validateDateRange,
  type AuditFilterState,
} from './audit';

const money = (minor: number) => `$${minor / 100}`;
const state = (patch: Partial<AuditFilterState> = {}): AuditFilterState => ({
  ...EMPTY_FILTER,
  ...patch,
});

describe('buildAuditFilter', () => {
  it('con estado vacío solo envía orden y paginación', () => {
    expect(buildAuditFilter(EMPTY_FILTER)).toStrictEqual({
      sort_dir: 'desc',
      page: 1,
      page_size: 50,
    });
  });

  it('usa snake_case, recorta espacios y omite vacíos', () => {
    const filter = buildAuditFilter(
      state({
        module: 'cash',
        outcome: 'failure',
        action: '  open ',
        entityType: 'cash_session',
        entityId: ' 7 ',
        dateFrom: '2026-06-01',
        dateTo: '2026-06-30',
        search: '  caja ',
        sortDir: 'asc',
        page: 3,
      }),
    );
    expect(filter).toStrictEqual({
      sort_dir: 'asc',
      page: 3,
      page_size: 50,
      module: 'cash',
      outcome: 'failure',
      action: 'open',
      entity_type: 'cash_session',
      entity_id: '7',
      date_from: '2026-06-01',
      date_to: '2026-06-30',
      search: 'caja',
    });
  });

  it('descarta módulo/categoría/resultado fuera del catálogo', () => {
    const filter = buildAuditFilter(state({ module: 'hack', category: 'x', outcome: 'y' }));
    expect(filter).not.toHaveProperty('module');
    expect(filter).not.toHaveProperty('category');
    expect(filter).not.toHaveProperty('outcome');
  });

  it('normaliza página inválida y limita page_size a 200', () => {
    expect(buildAuditFilter(state({ page: 0 })).page).toBe(1);
    expect(buildAuditFilter(state({ page: NaN })).page).toBe(1);
    expect(buildAuditFilter(state({ page: 2.7 })).page).toBe(2);
    expect(buildAuditFilter(EMPTY_FILTER, 999).page_size).toBe(200);
    expect(buildAuditFilter(EMPTY_FILTER, 0).page_size).toBe(50);
  });
});

describe('hasActiveFilters', () => {
  it('ignora orden y página', () => {
    expect(hasActiveFilters(state({ sortDir: 'asc', page: 4 }))).toBe(false);
    expect(hasActiveFilters(state({ search: ' ' }))).toBe(false);
    expect(hasActiveFilters(state({ outcome: 'failure' }))).toBe(true);
  });
});

describe('validateDateRange', () => {
  it('acepta rangos válidos y vacíos', () => {
    expect(validateDateRange('', '')).toBeNull();
    expect(validateDateRange('2026-06-01', '')).toBeNull();
    expect(validateDateRange('2026-06-01', '2026-06-01')).toBeNull();
  });

  it('rechaza inicio posterior al fin con el texto del backend', () => {
    expect(validateDateRange('2026-06-10', '2026-06-01')).toBe(
      'La fecha inicial no puede ser posterior a la final',
    );
  });

  it('rechaza fechas inexistentes o con mal formato', () => {
    expect(validateDateRange('2026-02-30', '')).toBe('La fecha inicial no es válida');
    expect(validateDateRange('', '12/06/2026')).toBe('La fecha final no es válida');
  });
});

describe('URL <-> filtro', () => {
  it('ida y vuelta conserva el estado', () => {
    const original = state({
      module: 'inventory',
      category: 'business',
      outcome: 'success',
      action: 'stock_adjust',
      entityType: 'variant',
      entityId: '12',
      dateFrom: '2026-06-01',
      dateTo: '2026-06-30',
      search: 'camisa roja',
      sortDir: 'asc',
      page: 2,
    });
    const params = filterToSearchParams(original);
    expect(searchParamsToFilter(new URLSearchParams(params.toString()))).toStrictEqual(original);
  });

  it('omite valores por defecto en la URL', () => {
    expect(filterToSearchParams(EMPTY_FILTER).toString()).toBe('');
  });

  it('ignora parámetros inválidos', () => {
    const parsed = searchParamsToFilter(
      new URLSearchParams('module=nope&from=2026-13-40&page=-2&sort=up'),
    );
    expect(parsed).toStrictEqual(EMPTY_FILTER);
  });
});

describe('etiquetas del catálogo', () => {
  it('traduce acciones conocidas y humaniza las desconocidas', () => {
    expect(actionLabel('cash', 'open')).toBe('Apertura de caja');
    expect(actionLabel('inventory', 'stock_adjust')).toBe('Ajuste de stock');
    expect(actionLabel('reports', 'export_all')).toBe('Export all');
  });

  it('formatActor y formatEntity', () => {
    expect(formatActor(null)).toBe('—');
    expect(formatActor('  ')).toBe('—');
    expect(formatActor('Ana')).toBe('Ana');
    expect(formatEntity({ entity_type: null, entity_id: null })).toBe('—');
    expect(formatEntity({ entity_type: 'variant', entity_id: '12' })).toBe('Variante #12');
    expect(formatEntity({ entity_type: 'raro', entity_id: null })).toBe('Raro');
  });
});

describe('formatAuditDate', () => {
  it('convierte UTC a la zona indicada', () => {
    // 01:30 UTC del 12 jun = 20:30 del 11 jun en Bogotá (UTC-5)
    const text = formatAuditDate('2026-06-12T01:30:00Z', 'America/Bogota');
    expect(text).toMatch(/11/);
    expect(text).toMatch(/2026/);
    expect(text).toMatch(/8:30/);
  });

  it('devuelve el original si la fecha o la zona son inválidas', () => {
    expect(formatAuditDate('no-es-fecha', 'America/Bogota')).toBe('no-es-fecha');
    expect(formatAuditDate('2026-06-12T01:30:00Z', 'Zona/Falsa')).toBe('2026-06-12T01:30:00Z');
  });
});

describe('isMoneyField', () => {
  it('depende del módulo', () => {
    expect(isMoneyField('cash', 'difference')).toBe(true);
    expect(isMoneyField('inventory', 'difference')).toBe(false);
    expect(isMoneyField('inventory', 'price')).toBe(true);
    expect(isMoneyField('settings', 'tax_rate_bps')).toBe(false);
    expect(isMoneyField('system', 'amount')).toBe(false);
  });
});

describe('formatAuditValue', () => {
  it('formatea tipos básicos', () => {
    expect(formatAuditValue('inventory', 'x', null, money)).toBe('—');
    expect(formatAuditValue('inventory', 'allow_negative', true, money)).toBe('Sí');
    expect(formatAuditValue('inventory', 'stock', 5, money)).toBe('5');
    expect(formatAuditValue('inventory', 'price', 280000, money)).toBe('$2800');
    expect(formatAuditValue('inventory', 'notes', '', money)).toBe('(vacío)');
  });

  it('traduce estados y deja pasar textos largos', () => {
    expect(formatAuditValue('cash', 'status', 'closed', money)).toBe('Cerrado');
    expect(formatAuditValue('settings', 'ticket_footer', '[texto de 120 caracteres]', money)).toBe(
      '[texto de 120 caracteres]',
    );
  });

  it('formatea pagos de ventas con dinero', () => {
    const payments = [
      { method: 'cash', amount: 10000 },
      { method: 'card', amount: 5000 },
    ];
    expect(formatAuditValue('sales', 'payments', payments, money)).toBe(
      'Efectivo: $100 · Tarjeta: $50',
    );
  });

  it('cae a JSON para objetos desconocidos', () => {
    expect(formatAuditValue('system', 'raro', { a: 1 }, money)).toBe('{"a":1}');
  });
});

const baseEvent: AuditEvent = {
  id: 1,
  created_at: '2026-06-12T01:30:00Z',
  correlation_id: null,
  category: 'business',
  module: 'inventory',
  action: 'update',
  outcome: 'success',
  actor: null,
  entity_type: 'variant',
  entity_id: '12',
  summary: 'x',
  changes: null,
  metadata: null,
  error_message: null,
};

describe('buildChangeRows', () => {
  it('devuelve [] sin changes', () => {
    expect(buildChangeRows(baseEvent, money)).toStrictEqual([]);
  });

  it('formatea antes/después con dinero en price y no en stock', () => {
    const rows = buildChangeRows(
      {
        ...baseEvent,
        changes: {
          price: { from: 100000, to: 120000 },
          stock: { from: 3, to: 8 },
        },
      },
      money,
    );
    expect(rows).toStrictEqual([
      { key: 'price', label: 'Precio', from: '$1000', to: '$1200' },
      { key: 'stock', label: 'Stock', from: '3', to: '8' },
    ]);
  });

  it('tolera entradas sin forma {from,to}', () => {
    const rows = buildChangeRows(
      { ...baseEvent, changes: { rareza: 'x' } as never },
      money,
    );
    expect(rows[0]).toStrictEqual({ key: 'rareza', label: 'Rareza', from: '—', to: 'x' });
  });
});

describe('buildMetadataRows', () => {
  it('etiqueta claves conocidas y cae a genérico en las demás', () => {
    const rows = buildMetadataRows(
      {
        module: 'inventory',
        metadata: { movement_type: 'manual_out', unit_cost: 5000, difference: -2, campo_nuevo: 'ok' },
      },
      money,
    );
    expect(rows).toStrictEqual([
      { key: 'movement_type', label: 'Tipo de movimiento', value: 'Egreso manual' },
      { key: 'unit_cost', label: 'Costo unitario', value: '$50' },
      { key: 'difference', label: 'Diferencia', value: '-2' },
      { key: 'campo_nuevo', label: 'Campo nuevo', value: 'ok' },
    ]);
  });

  it('devuelve [] sin metadata', () => {
    expect(buildMetadataRows(baseEvent, money)).toStrictEqual([]);
  });
});