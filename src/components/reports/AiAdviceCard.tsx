import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import type { AiSettings, CommercialContext, StoredAdvice } from '../../types/insights';
import {
  generateAiAdvice,
  getAiAdvice,
  setAiFeedback,
} from '../../services/commercialCalendar.service';

const KIND_LABEL: Record<string, string> = {
  two_for_one: '2x1',
  second_unit_off: '2.ª unidad con descuento',
  direct_discount: 'Descuento directo',
  bundle: 'Combo',
};

export function AiAdviceCard({
  ctx,
  aiSettings,
  aiSettingsLoading,
  aiSettingsError,
}: {
  ctx: CommercialContext | null;
  aiSettings: AiSettings | null;
  aiSettingsLoading: boolean;
  aiSettingsError: boolean;
}) {
  const navigate = useNavigate();
  const [storedAdvice, setStoredAdvice] = useState<StoredAdvice | null>(null);
  const [adviceLoading, setAdviceLoading] = useState(true);
  const [adviceLoadError, setAdviceLoadError] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [generationError, setGenerationError] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<'useful' | 'not_applicable' | null>(null);

  useEffect(() => {
    let active = true;
    getAiAdvice('dashboard')
      .then(value => {
        if (active) setStoredAdvice(value);
      })
      .catch(() => {
        if (active) setAdviceLoadError(true);
      })
      .finally(() => {
        if (active) setAdviceLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);

  const generateAdvice = async () => {
    setGenerating(true);
    setGenerationError(null);
    try {
      setStoredAdvice(await generateAiAdvice('dashboard'));
      setFeedback(null);
    } catch (cause) {
      const reason = typeof cause === 'string'
        ? cause
        : cause instanceof Error
          ? cause.message
          : 'Error inesperado';
      setGenerationError(`No se generó el consejo: ${reason}`);
    } finally {
      setGenerating(false);
    }
  };

  const submitFeedback = async (value: 'useful' | 'not_applicable') => {
    if (!storedAdvice) return;
    try {
      await setAiFeedback(storedAdvice.id, value);
      setFeedback(value);
    } catch {
      setGenerationError('No se pudo guardar la valoración.');
    }
  };

  const breakEvenFor = (alias: string, kind: string) =>
    ctx?.promo_candidates
      .find(candidate => candidate.alias === alias)
      ?.scenarios.find(scenario => scenario.kind === kind)?.breakeven_x100;

  return (
    <section className="ai-advice-card" aria-label="Recomendaciones de IA">
      <header className="ai-advice-header">
        <div>
          <p className="ai-advice-eyebrow">ASESOR COMERCIAL</p>
          <h3>Recomendaciones de IA</h3>
        </div>
        {storedAdvice && <time dateTime={storedAdvice.local_date}>{storedAdvice.local_date}</time>}
      </header>

      {aiSettingsLoading && <p className="ai-advice-muted">Revisando configuración…</p>}
      {!aiSettingsLoading && aiSettingsError && (
        <p className="ai-advice-muted" role="status">No se pudieron leer los ajustes del asesor. Revisa Ajustes.</p>
      )}
      {!aiSettingsLoading && aiSettings && !aiSettings.enabled && (
        <div className="ai-advice-empty">
          <p>El asesor está apagado. Actívalo para generar recomendaciones.</p>
          <button type="button" className="btn btn-secondary" onClick={() => navigate('/settings', { state: { section: 'ai' } })}>
            Abrir ajustes
          </button>
        </div>
      )}
      {!aiSettingsLoading && aiSettings?.enabled && !aiSettings.has_key && (
        <div className="ai-advice-empty">
          <p>Falta guardar la clave de DeepSeek.</p>
          <button type="button" className="btn btn-secondary" onClick={() => navigate('/settings', { state: { section: 'ai' } })}>
            Configurar clave
          </button>
        </div>
      )}

      {aiSettings?.enabled && aiSettings.has_key && (
        <button type="button" className="btn btn-primary ai-advice-generate" disabled={generating} onClick={generateAdvice}>
          {generating ? 'Generando recomendación…' : 'Generar recomendación de hoy'}
        </button>
      )}

      {adviceLoading && <p className="ai-advice-muted">Buscando el último consejo guardado…</p>}
      {!adviceLoading && adviceLoadError && (
        <p className="ai-advice-muted" role="status">No se pudo cargar el último consejo guardado.</p>
      )}
      {generationError && <p className="ai-advice-error" role="alert">{generationError}</p>}
      {storedAdvice?.content.insufficient_data && (
        <p className="ai-advice-muted">Datos insuficientes para aconsejar.</p>
      )}
      {storedAdvice && !storedAdvice.content.insufficient_data && storedAdvice.content.advice.length === 0 && (
        <p className="ai-advice-muted">No hay recomendaciones verificables para mostrar.</p>
      )}
      {storedAdvice?.content.advice.map((item, index) => {
        const breakEven = item.promo ? breakEvenFor(item.promo.alias, item.promo.kind) : null;
        return (
          <article className="ai-advice-item" key={`${storedAdvice.id}-${index}`}>
            <h4>{item.observation}</h4>
            <p>{item.context}</p>
            <p>{item.proposal}</p>
            {item.inventory_action && <p>{item.inventory_action}</p>}
            {item.promo && (
              <p className="ai-advice-promo">
                {item.promo.alias} · {KIND_LABEL[item.promo.kind] ?? item.promo.kind}
                {` · punto de equilibrio: ${breakEven == null ? 'no disponible' : `${(breakEven / 100).toFixed(1)}x`}`}
              </p>
            )}
          </article>
        );
      })}

      {!adviceLoading && !adviceLoadError && !storedAdvice && aiSettings?.enabled && aiSettings.has_key && (
        <p className="ai-advice-muted">Todavía no hay recomendaciones guardadas. Genera la primera cuando quieras.</p>
      )}
      {storedAdvice && storedAdvice.content.advice.length > 0 && (
        <div className="ai-advice-feedback">
          <button type="button" className="btn btn-ghost" disabled={feedback !== null} onClick={() => void submitFeedback('useful')}>
            Útil
          </button>
          <button type="button" className="btn btn-ghost" disabled={feedback !== null} onClick={() => void submitFeedback('not_applicable')}>
            No aplica
          </button>
          {feedback && <span className="ai-advice-muted">Gracias por valorar el consejo.</span>}
        </div>
      )}
    </section>
  );
}