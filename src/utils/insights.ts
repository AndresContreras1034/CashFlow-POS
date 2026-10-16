export function resolveProductAliases(
  text: string,
  productNames: Readonly<Record<string, string>> | null | undefined,
): string {
  if (!productNames) return text;
  return text.replace(/Producto \d+/g, alias => productNames[alias] ?? alias);
}
