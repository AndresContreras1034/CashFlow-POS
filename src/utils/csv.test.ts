import { describe, expect, it } from 'vitest';
import { formatCsvAmount, neutralizeCell, toCsv } from './csv';

describe('neutralizeCell', () => {
  it.each([
    ['-1234,56', '-1234,56'],
    ['1234,56', '1234,56'],
    ['=1+1', "'=1+1"],
    ['@SUM(A1)', "'@SUM(A1)"],
    ['\t=1+1', "'\t=1+1"],
    ['\r=1+1', "'\r=1+1"],
    ['  =1+1', "'  =1+1"],
    ['-cmd', "'-cmd"],
    ['+573001234567', '+573001234567'],
    ['Perfume Rosa', 'Perfume Rosa'],
  ])('neutralizes %j', (input, expected) => {
    expect(neutralizeCell(input)).toBe(expected);
  });
});

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

  it('neutralizes tabs and carriage returns without altering numeric amounts or phone numbers', () => {
    const csv = toCsv(
      ['Monto', 'Teléfono', 'Tab', 'Retorno'],
      [['-1234,56', '+573001234567', '\t=1+1', '\r@SUM(A1:A2)']],
    );
    expect(csv).toBe(
      "\uFEFFMonto;Teléfono;Tab;Retorno\r\n-1234,56;+573001234567;\"'\t=1+1\";\"'\r@SUM(A1:A2)\"",
    );
  });
});

describe('formatCsvAmount', () => {
  it('formats minor units with zero currency decimals', () => {
    expect(formatCsvAmount(123_456, 0)).toBe('1235');
  });

  it('formats minor units with two currency decimals and a comma separator', () => {
    expect(formatCsvAmount(123_456, 2)).toBe('1234,56');
  });

  it('preserves the sign of a negative amount', () => {
    expect(formatCsvAmount(-123_456, 2)).toBe('-1234,56');
  });
});
