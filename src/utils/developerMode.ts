import { setDeveloperMode } from '../services/developer.service';

const DEVELOPER_MODE_KEY = 'cashflow.developerMode';

/** Preferencia local del equipo. Nunca lanza: apagado por defecto. */
export function readDeveloperMode(): boolean {
  try {
    return localStorage.getItem(DEVELOPER_MODE_KEY) === '1';
  } catch {
    return false;
  }
}

function writeDeveloperMode(enabled: boolean): void {
  localStorage.setItem(DEVELOPER_MODE_KEY, enabled ? '1' : '0');
}

/** Al arrancar el frontend: lee la preferencia y la envía al backend. */
export async function syncDeveloperMode(): Promise<void> {
  try {
    await setDeveloperMode(readDeveloperMode());
  } catch (error) {
    console.warn('No se pudo sincronizar el modo desarrollador:', error);
  }
}

/** Desde Ajustes. Primero el backend: si falla, la preferencia no cambia. */
export async function applyDeveloperMode(enabled: boolean): Promise<void> {
  await setDeveloperMode(enabled);
  writeDeveloperMode(enabled);
}