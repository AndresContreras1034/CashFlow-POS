import { describe, expect, it } from 'vitest';
import { resolveProductAliases } from './insights';

describe('resolveProductAliases', () => {
  const productNames = {
    'Producto 1': 'Café molido (250 g)',
    'Producto 10': 'Taza de cerámica (SKU TAZ-10)',
  };

  it('resuelve aliases en orden numérico sin confundir los prefijos', () => {
    expect(resolveProductAliases(
      'Producto 1 y Producto 10; repetir Producto 1.',
      productNames,
    )).toBe('Café molido (250 g) y Taza de cerámica (SKU TAZ-10); repetir Café molido (250 g).');
  });

  it('conserva los aliases que no tengan nombre asociado', () => {
    expect(resolveProductAliases('Producto 2', productNames)).toBe('Producto 2');
    expect(resolveProductAliases('Producto 1', undefined)).toBe('Producto 1');
  });
});
