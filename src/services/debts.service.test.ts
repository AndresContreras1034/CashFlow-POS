import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

import * as service from './debts.service';

describe('contrato con los comandos de Tauri', () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue('respuesta');
  });

  const dto = { reason: 'x' };

  const cases: [string, () => Promise<unknown>, string, unknown][] = [
    ['getDebtOptions', () => service.getDebtOptions(), 'get_debt_options', undefined],
    ['listDebts', () => service.listDebts({ page: 2 }), 'list_debts', { filter: { page: 2 } }],
    ['listDebts sin filtro', () => service.listDebts(), 'list_debts', { filter: {} }],
    ['getDebt', () => service.getDebt(7), 'get_debt', { id: 7 }],
    [
      'getDebtSummary',
      () => service.getDebtSummary({ search: 'a' }),
      'get_debt_summary',
      { filter: { search: 'a' } },
    ],
    [
      'getDebtCategoryStats',
      () => service.getDebtCategoryStats(),
      'get_debt_category_stats',
      { filter: {} },
    ],
    [
      'listDueDebts',
      () => service.listDueDebts({ page_size: 5 }),
      'list_due_debts',
      { filter: { page_size: 5 } },
    ],
    ['listDebtHistory', () => service.listDebtHistory(7), 'list_debt_history', { debtId: 7 }],
    ['voidDebt', () => service.voidDebt(7, dto), 'void_debt', { id: 7, dto }],
    [
      'registerDebtPayment',
      () => service.registerDebtPayment(7, {} as never),
      'register_debt_payment',
      { debtId: 7, dto: {} },
    ],
    [
      'voidDebtPayment',
      () => service.voidDebtPayment(9, dto),
      'void_debt_payment',
      { paymentId: 9, dto },
    ],
    ['createDebt', () => service.createDebt({} as never), 'create_debt', { dto: {} }],
    ['updateDebt', () => service.updateDebt(3, {} as never), 'update_debt', { id: 3, dto: {} }],
  ];

  it.each(cases)('%s', async (_name, call, command, args) => {
    await call();
    if (args === undefined) expect(invoke).toHaveBeenCalledWith(command);
    else expect(invoke).toHaveBeenCalledWith(command, args);
  });

  it('devuelve lo que responde el backend y propaga sus errores como texto', async () => {
    invoke.mockResolvedValueOnce({ id: 1 });
    await expect(service.getDebt(1)).resolves.toEqual({ id: 1 });
    invoke.mockRejectedValueOnce('El pago supera el saldo pendiente de la deuda');
    await expect(service.registerDebtPayment(1, {} as never)).rejects.toBe(
      'El pago supera el saldo pendiente de la deuda',
    );
  });
});
