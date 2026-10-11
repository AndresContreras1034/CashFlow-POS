export type CsvCell = string | number | null;

export function toCsv(headers: string[], rows: CsvCell[][]): string {
  const escapeCell = (value: CsvCell): string => {
    if (value === null) return '';
    const text = typeof value === 'number'
      ? String(value).replace('.', ',')
      : /^[=+\-@]/.test(value)
        ? `'${value}`
        : value;
    const escaped = text.replace(/"/g, '""');
    return /[;"\r\n]/.test(escaped) ? `"${escaped}"` : escaped;
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
