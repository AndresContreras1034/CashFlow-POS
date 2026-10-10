import type { CreateSalePaymentDto } from '../types';

export interface CartLine {
  variant_id: number;
  product_name: string;
  attributes: Record<string, string>;
  unit_price: number;
  quantity: number;
  discount: number;
  stock: number;
}

export interface CheckoutInput {
  lines: CartLine[];
  payments: CreateSalePaymentDto[];
  isCourtesyFlag: boolean;
  cashReceived: number | null;
}

export interface CheckoutTotals {
  subtotal: number;
  courtesyMode: boolean;
  total: number;
  paymentsTotal: number;
  remaining: number;
  cashNet: number;
  cashShortage: boolean;
  cashChange: number | null;
  oversold: boolean;
  hasInvalidDiscount: boolean;
}

export function lineGross(line: CartLine): number {
  return line.unit_price * line.quantity;
}

export function lineNet(line: CartLine): number {
  return lineGross(line) - line.discount;
}

export function lineDiscountInvalid(line: CartLine): boolean {
  return line.discount < 0 || line.discount > lineGross(line);
}

export function cartSubtotal(lines: CartLine[]): number {
  return lines.reduce((sum, line) => sum + lineNet(line), 0);
}

export function cartHasInvalidDiscount(lines: CartLine[]): boolean {
  return lines.some(lineDiscountInvalid);
}

export function isCourtesyMode(
  lineCount: number,
  isCourtesyFlag: boolean,
  subtotal: number
): boolean {
  return lineCount > 0 && (isCourtesyFlag || subtotal === 0);
}

export function paymentsTotal(payments: CreateSalePaymentDto[]): number {
  return payments.reduce((sum, payment) => sum + (payment.amount || 0), 0);
}

export function cashNet(payments: CreateSalePaymentDto[]): number {
  return payments.reduce(
    (sum, payment) => sum + (payment.method === 'cash' ? payment.amount : 0),
    0
  );
}

export function computeCheckout(input: CheckoutInput): CheckoutTotals {
  const subtotal = cartSubtotal(input.lines);
  const courtesyMode = isCourtesyMode(input.lines.length, input.isCourtesyFlag, subtotal);
  const total = courtesyMode ? 0 : subtotal;
  const totalPayments = paymentsTotal(input.payments);
  const receivedCash = cashNet(input.payments);
  const cashShortage = input.cashReceived !== null && input.cashReceived < receivedCash;

  return {
    subtotal,
    courtesyMode,
    total,
    paymentsTotal: totalPayments,
    remaining: courtesyMode ? 0 : total - totalPayments,
    cashNet: receivedCash,
    cashShortage,
    cashChange:
      input.cashReceived !== null && !cashShortage
        ? input.cashReceived - receivedCash
        : null,
    oversold: input.lines.some((line) => line.quantity > line.stock),
    hasInvalidDiscount: cartHasInvalidDiscount(input.lines),
  };
}

export function retryFingerprint(payload: unknown): string {
  return JSON.stringify(payload);
}
