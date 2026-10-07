export const APP_THEMES = [
  { id: 'slate', name: 'Pizarra', color: '#334155', dark: false },
  { id: 'violet', name: 'Violeta', color: '#6D4FC2', dark: false },
  { id: 'breeze', name: 'Brisa', color: '#0F766E', dark: false },
  { id: 'parchment', name: 'Pergamino', color: '#8A4B2A', dark: false },
  { id: 'coral', name: 'Coral', color: '#C2410C', dark: false },
  { id: 'ocean', name: 'Océano', color: '#1D5FB8', dark: false },
  { id: 'forest', name: 'Bosque', color: '#3F6B2A', dark: false },
  { id: 'midnight', name: 'Medianoche', color: '#3B6FD4', dark: true },
  { id: 'graphite', name: 'Grafito', color: '#3A7D75', dark: true },
  { id: 'plum', name: 'Ciruela', color: '#7B5FD0', dark: true },
  { id: 'rose', name: 'Rosa', color: '#BE185D', dark: false },
  { id: 'lagoon', name: 'Laguna', color: '#087E8B', dark: false },
  { id: 'honey', name: 'Miel', color: '#A16207', dark: false },
  { id: 'lilac', name: 'Lila', color: '#7546A8', dark: false },
  { id: 'mint', name: 'Menta', color: '#16845B', dark: false },
  { id: 'cherry', name: 'Cereza', color: '#F05A78', dark: true },
  { id: 'cyberpunk', name: 'Cyberpunk', color: '#F000B8', dark: true },
  { id: 'neon', name: 'Neón', color: '#B7F500', dark: true },
  { id: 'deepsea', name: 'Mar profundo', color: '#22B8CF', dark: true },
  { id: 'sunset', name: 'Atardecer', color: '#FF7547', dark: true },
] as const;

export type AppTheme = (typeof APP_THEMES)[number]['id'];

const THEME_STORAGE_KEY = 'cashflow-pos-theme';

const isAppTheme = (value: string): value is AppTheme =>
  APP_THEMES.some(theme => theme.id === value);

export const applyTheme = (theme: AppTheme): void => {
  if (theme === 'slate') {
    document.documentElement.removeAttribute('data-theme');
  } else {
    document.documentElement.dataset.theme = theme;
  }
  localStorage.setItem(THEME_STORAGE_KEY, theme);
};

export const loadSavedTheme = (): AppTheme => {
  const savedTheme = localStorage.getItem(THEME_STORAGE_KEY);
  if (savedTheme && isAppTheme(savedTheme)) return savedTheme;
  return 'slate';
};
