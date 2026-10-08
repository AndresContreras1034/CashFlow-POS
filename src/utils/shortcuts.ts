export const NAVIGATION_SHORTCUTS = [
  { path: '/inventory', label: 'Inventario' },
  { path: '/stocktake', label: 'Toma de inventario' },
  { path: '/sales', label: 'Ventas' },
  { path: '/sales-history', label: 'Historial ventas' },
  { path: '/cash', label: 'Caja' },
  { path: '/customers', label: 'Clientes' },
  { path: '/debts', label: 'Deudas' },
  { path: '/suppliers', label: 'Proveedores' },
  { path: '/reports', label: 'Reportes' },
  { path: '/audit', label: 'Auditoría' },
  { path: '/settings', label: 'Ajustes' },
] as const;

export type NavigationPath = (typeof NAVIGATION_SHORTCUTS)[number]['path'];
export type NavigationShortcuts = Partial<Record<NavigationPath, string>>;

const STORAGE_KEY = 'cashflow-pos-navigation-shortcuts';
const UPDATED_EVENT = 'cashflow-pos-shortcuts-updated';

export const readNavigationShortcuts = (): NavigationShortcuts => {
  const saved = localStorage.getItem(STORAGE_KEY);
  if (!saved) return {};

  const parsed: unknown = JSON.parse(saved);
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('La configuración de atajos guardada no es válida.');
  }

  const shortcuts: NavigationShortcuts = {};
  for (const item of NAVIGATION_SHORTCUTS) {
    const key = (parsed as Record<string, unknown>)[item.path];
    if (typeof key === 'string' && isSupportedShortcut(key)) {
      shortcuts[item.path] = key;
    }
  }
  return shortcuts;
};

export const saveNavigationShortcuts = (shortcuts: NavigationShortcuts): void => {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(shortcuts));
  window.dispatchEvent(new Event(UPDATED_EVENT));
};

export const onNavigationShortcutsChanged = (callback: () => void): (() => void) => {
  window.addEventListener(UPDATED_EVENT, callback);
  return () => window.removeEventListener(UPDATED_EVENT, callback);
};

export const isSupportedShortcut = (key: string): boolean =>
  /^[A-Z0-9]$/.test(key) || /^F([1-9]|1[0-2])$/.test(key);

export const formatShortcutKey = (key: string | undefined): string =>
  key ? (key.length === 1 ? key.toUpperCase() : key) : 'Sin asignar';
