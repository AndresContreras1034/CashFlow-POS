const OPERATOR_KEY = 'cashflow.operator';

export function getOperator(): string {
  return localStorage.getItem(OPERATOR_KEY)?.trim() || 'system';
}

export function setOperator(operator: string): void {
  localStorage.setItem(OPERATOR_KEY, operator.trim());
}
