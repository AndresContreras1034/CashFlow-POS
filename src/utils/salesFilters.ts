export type SaleDatePreset = 'today' | 'yesterday' | 'week';

function localDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

export function presetRange(
  preset: SaleDatePreset,
  now: Date
): { from: string; to: string } {
  const to = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const from = new Date(to);
  if (preset === 'yesterday') {
    from.setDate(from.getDate() - 1);
    to.setDate(to.getDate() - 1);
  } else if (preset === 'week') {
    from.setDate(from.getDate() - 6);
  }
  return { from: localDate(from), to: localDate(to) };
}

export function parseSaleNumber(text: string): number | null {
  const match = /^(?:#|T-)?(\d+)$/i.exec(text.trim());
  if (!match) return null;
  const parsed = Number(match[1]);
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : null;
}
