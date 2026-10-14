import { beforeEach, describe, expect, it, vi } from 'vitest';
import { openUrl } from '@tauri-apps/plugin-opener';
import { openExternal } from './openExternal';

vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));

const mockedOpenUrl = vi.mocked(openUrl);

describe('openExternal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('abre una URL de contacto permitida', async () => {
    mockedOpenUrl.mockResolvedValue();

    await expect(openExternal('tel:3012249460')).resolves.toEqual({ ok: true });
    expect(mockedOpenUrl).toHaveBeenCalledOnce();
    expect(mockedOpenUrl).toHaveBeenCalledWith('tel:3012249460');
  });

  it('rechaza URL no permitida sin llamar al opener', async () => {
    await expect(openExternal('https://evil.example')).resolves.toMatchObject({ ok: false });
    expect(mockedOpenUrl).not.toHaveBeenCalled();
  });

  it('devuelve el error cuando openUrl falla', async () => {
    mockedOpenUrl.mockRejectedValue(new Error('No hay aplicación registrada'));

    await expect(openExternal('mailto:a@b.com')).resolves.toEqual({
      ok: false,
      error: 'No hay aplicación registrada',
    });
  });
});
