import { openUrl } from '@tauri-apps/plugin-opener';
import { errorMessage } from './audit';
import { isAllowedContactUrl } from './callPanel';

export type OpenExternalResult = { ok: true } | { ok: false; error: string };

export const openExternal = async (url: string): Promise<OpenExternalResult> => {
  if (!isAllowedContactUrl(url)) {
    return { ok: false, error: 'URL de contacto no permitida' };
  }

  try {
    await openUrl(url);
    return { ok: true };
  } catch (cause) {
    return { ok: false, error: errorMessage(cause) };
  }
};
