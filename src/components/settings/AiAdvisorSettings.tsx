import { useEffect, useState } from 'react';
import { SecretField } from '../backup/SecretField';
import { getAiSettings, updateAiSettings } from '../../services/commercialCalendar.service';
import type { AiSettings } from '../../types/insights';

export function AiAdvisorSettings() {
  const [settings, setSettings] = useState<AiSettings | null>(null);
  const [capDraft, setCapDraft] = useState('60000');
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    getAiSettings()
      .then(value => {
        if (!active) return;
        setSettings(value);
        setCapDraft(String(value.daily_token_cap));
      })
      .catch(cause => {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);

  const save = async (
    enabled: boolean,
    dailyTokenCap: number,
    apiKey: string | null = null,
  ): Promise<boolean> => {
    setBusy(true);
    setError(null);
    try {
      const updated = await updateAiSettings({ enabled, dailyTokenCap, apiKey });
      setSettings(updated);
      setCapDraft(String(updated.daily_token_cap));
      return true;
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      return false;
    } finally {
      setBusy(false);
    }
  };

  const saveCap = () => {
    if (!settings) return;
    const cap = Number(capDraft);
    if (!Number.isInteger(cap) || cap < 1_000 || cap > 1_000_000) {
      setError('El tope debe estar entre 1.000 y 1.000.000 tokens.');
      return;
    }
    void save(settings.enabled, cap);
  };

  const saveKey = (value: string | null) => {
    if (!settings || value === null) return Promise.resolve(false);
    return save(settings.enabled, settings.daily_token_cap, value);
  };

  if (loading) return <p role="status">Cargando ajustes del asesor…</p>;
  if (!settings) {
    return <p className="reports-error" role="alert">{error ?? 'No se pudieron cargar los ajustes.'}</p>;
  }

  return (
    <section className="settings-section">
      <h2 className="settings-section-title">Asesor de IA (experimental)</h2>
      <div className="settings-switch-list">
        <label className="settings-switch">
          <span>Activar asesor de IA</span>
          <input
            type="checkbox"
            checked={settings.enabled}
            disabled={busy}
            onChange={event => void save(event.target.checked, settings.daily_token_cap)}
          />
          <span className="settings-switch-track" aria-hidden="true" />
        </label>
      </div>

      <div className="form-field">
        <label className="form-label" htmlFor="ai-daily-token-cap">Tope diario de tokens</label>
        <div className="settings-inline-field">
          <input
            id="ai-daily-token-cap"
            className="form-input"
            type="number"
            min={1_000}
            max={1_000_000}
            step={1_000}
            value={capDraft}
            onChange={event => setCapDraft(event.target.value)}
          />
          <button type="button" className="btn btn-primary" disabled={busy} onClick={saveCap}>
            Guardar tope
          </button>
        </div>
        <span className="settings-hint">
          Usados hoy: {settings.tokens_today.toLocaleString('es-CO')} / {settings.daily_token_cap.toLocaleString('es-CO')}
        </span>
      </div>

      <SecretField
        id="deepseek-api-key"
        label="Clave API de DeepSeek"
        isSet={settings.has_key}
        canRemove={false}
        hint="La clave se guarda en el almacén de credenciales de este equipo."
        onSave={saveKey}
      />

      <p className="settings-notice">
        Se envían a DeepSeek cifras agregadas y alias de productos; nunca nombres de clientes, acreedores ni productos.
      </p>
      {error && <p className="reports-error" role="alert">{error}</p>}
    </section>
  );
}