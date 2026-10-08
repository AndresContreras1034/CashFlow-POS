const OPERATOR_KEY = 'cashflow.operator';

export function getOperator(): string {
  return localStorage.getItem(OPERATOR_KEY)?.trim() || 'system';
}

export function setOperator(operator: string): void {
  localStorage.setItem(OPERATOR_KEY, operator.trim());
}

/** Operador guardado, o null si no hay ninguno. Nunca lanza: el actor no debe romper una operación. */
export function getOperatorOrNull(): string | null {
  try {
    const value = localStorage.getItem(OPERATOR_KEY)?.trim();
    return value ? value : null;
  } catch {
    return null;
  }
}

/** Fragmento opcional para dto o argumentos de invoke. */
export function actorField<K extends string>(key: K): { [P in K]?: string } {
  const actor = getOperatorOrNull();
  return actor ? ({ [key]: actor } as { [P in K]?: string }) : {};
}
