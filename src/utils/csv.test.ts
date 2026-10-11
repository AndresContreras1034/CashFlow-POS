import { describe, expect, it } from 'vitest';
import { formatCsvAmount, toCsv } from './csv';

describe('toCsv', () => {
  it('escapes quotes, separators and line breaks, and renders null as empty', () => {
    expect(toCsv(['A', 'B', 'C', 'D'], [['a"b', 'uno;dos', 'línea 1\nlínea 2', null]]))
      .toBe('\uFEFFA;B;C;D\r\n"a""b";"uno;dos";"línea 1\nlínea 2";');
  });

  it('prepends the UTF-8 BOM and neutralizes formula-leading text', () => {
    const csv = toCsv(['Dato'], [['=1+1']]);
    expect(csv.charCodeAt(0)).toBe(0xFEFF);
    expect(csv).toBe("\uFEFFDato\r\n'=1+1");
  });
});

describe('formatCsvAmount', () => {
  it('formats minor units with zero currency decimals', () => {
    expect(formatCsvAmount(123_456, 0)).toBe('1235');
  });

  it('formats minor units with two currency decimals and a comma separator', () => {
    expect(formatCsvAmount(123_456, 2)).toBe('1234,56');
  });
});
