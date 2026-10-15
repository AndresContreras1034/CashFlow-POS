export type CsvCell = string | number | null;

function isNumericText(value: string): boolean {
  return /^[+-]?\d+(?:[.,]\d+)?$/.test(value.trim());
}

function isInternationalPhone(value: string): boolean {
  if (!/^\+\d(?:[\d ()-]*\d)?$/.test(value.trim())) return false;
  const digits = value.replace(/\D/g, '');
  return digits.length >= 7 && digits.length <= 15;
}

export function toCsv(headers: string[], rows: CsvCell[][]): string {
  const escapeCell = (value: CsvCell): string => {
    if (value === null) return '';
    if (typeof value === 'number') return String(value).replace('.', ',');
    const dangerousFormula = /^[ \t\r\n]*[=+\-@]/.test(value);
    const text = dangerousFormula && !isNumericText(value) && !isInternationalPhone(value)
      ? `'${value}`
      : value;
    const escaped = text.replace(/"/g, '""');
    return /[;"\t\r\n]/.test(escaped) ? `"${escaped}"` : escaped;
  };

  return `\uFEFF${[headers, ...rows]
    .map(row => row.map(escapeCell).join(';'))
    .join('\r\n')}`;
}

export function formatCsvAmount(minorUnits: number, decimals: number): string {
  const precision = Math.min(Math.max(Math.trunc(decimals), 0), 2);
  const scale = 10 ** precision;
  const minorUnitsPerDigit = 100 / scale;
  const absolute = Math.abs(minorUnits);
  const rounded = Math.floor((absolute + minorUnitsPerDigit / 2) / minorUnitsPerDigit);
  const whole = Math.floor(rounded / scale);
  const fraction = rounded % scale;
  const sign = minorUnits < 0 && rounded > 0 ? '-' : '';

  if (precision === 0) return `${sign}${whole}`;
  return `${sign}${whole},${String(fraction).padStart(precision, '0')}`;
}
