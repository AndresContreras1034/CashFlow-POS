import { useEffect, useState } from 'react';
import type { CommercialContext } from '../../types/insights';
import type { StoredAdvice } from '../../types/insights';
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

const money = (cents: number) =>
  new Intl.NumberFormat('es-CO', {
    style: 'currency',
    currency: 'COP',
    maximumFractionDigits: 0,
  }).format(cents / 100);

const pct = (bps: number) => `${(bps / 100).toFixed(1)} %`;

function Insufficient() {
  return <p className="insights-muted">Datos insuficientes</p>;
}

export function InsightsCard({
  ctx,
  aiEnabled,
}: {
  ctx: CommercialContext | null | undefined;
  aiEnabled: boolean;
}) {
  const [storedAdvice, setStoredAdvice] = useState<StoredAdvice | null>(null);
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
        if (active) setStoredAdvice(null);
      });
    return () => {
      active = false;
    };
  }, []);

  if (!ctx) return null;

  const { pace, calendar, inventory, promo_candidates: promos } = ctx;
  const risks = inventory.coverage.filter((c) => c.risk);

  const generateAdvice = async () => {
    setGenerating(true);
    setGenerationError(null);
    try {
      setStoredAdvice(await generateAiAdvice('dashboard'));
      setFeedback(null);
    } catch {
      setGenerationError('No se pudo generar el consejo. Se conserva el último guardado.');
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

  return (
    <section className="insights-card" aria-label="Contexto comercial">
      <h3>Contexto comercial</h3>

      <div className="insights-block">
        <h4>Ritmo de ventas</h4>
        {pace.status === 'ok' && pace.vs_baseline_bps !== null ? (
          <p>
            Últimos {pace.days} días: {pace.vs_baseline_bps >= 0 ? '+' : ''}
            {pct(pace.vs_baseline_bps)} frente a tu línea base.
          </p>
        ) : (
          <Insufficient />
        )}
      </div>

      <div className="insights-block">
        <h4>Próximos eventos</h4>
        {calendar.length === 0 ? (
          <p className="insights-muted">Sin eventos próximos.</p>
        ) : (
          <ul>
            {calendar.map((e) => (
              <li key={`${e.name}-${e.in_days}`}>
                {e.name}: en {e.in_days} días
                {e.last_year.status === 'insufficient' && (
                  <span className="insights-muted"> (sin historial del año pasado)</span>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="insights-block">
        <h4>Riesgo de faltante</h4>
        {ctx.data_sufficiency.history_days < 28 ? (
          <Insufficient />
        ) : risks.length === 0 ? (
          <p className="insights-muted">Ningún producto estrella en riesgo.</p>
        ) : (
          <ul>
            {risks.map((r) => (
              <li key={r.alias}>
                {r.alias}: alcanza para {r.days_cover ?? '?'} días
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="insights-block">
        <h4>Inventario parado y combos</h4>
        <p>
          {inventory.stale.rows} productos parados, {money(inventory.stale.total_value_cents)}{' '}
          inmovilizados.
        </p>
        {promos.length === 0 ? (
          <Insufficient />
        ) : (
          promos.map((p) => (
            <div key={p.alias}>
              <strong>{p.alias}</strong>
              <ul>
                {p.scenarios.map((s) => (
                  <li key={s.kind}>
                    {KIND_LABEL[s.kind] ?? s.kind}: utilidad {money(s.profit * 100)}
                    {s.allowed ? '' : ' (no permitido)'}
                    {` · punto de equilibrio: ${s.breakeven_x100 === null ? 'no disponible' : `${(s.breakeven_x100 / 100).toFixed(1)}x`}`}
                  </li>
                ))}
              </ul>
            </div>
          ))
        )}
      </div>

      {ctx.data_sufficiency.uncosted_pct_bps > 2000 && (
        <p className="insights-muted">
          {pct(ctx.data_sufficiency.uncosted_pct_bps)} de las ventas no tiene costo registrado.
        </p>
      )}

      <div className="insights-block">
        <h4>Consejos de IA</h4>
        {aiEnabled && (
          <button type="button" className="btn btn-primary" disabled={generating} onClick={generateAdvice}>
            {generating ? 'Generando…' : 'Generar consejos de hoy'}
          </button>
        )}
        {storedAdvice && <p className="insights-muted">Consejo guardado: {storedAdvice.local_date}</p>}
        {generationError && <p className="insights-muted" role="status">{generationError}</p>}
        {storedAdvice?.content.insufficient_data ? (
          <p className="insights-muted">Datos insuficientes para aconsejar</p>
        ) : storedAdvice?.content.advice.map((item, index) => {
          const breakEven = item.promo
            ? ctx.promo_candidates
                .find(candidate => candidate.alias === item.promo?.alias)
                ?.scenarios.find(scenario => scenario.kind === item.promo?.kind)?.breakeven_x100
            : undefined;
          return (
            <article key={`${storedAdvice.id}-${index}`}>
              <strong>{item.observation}</strong>
              <p>{item.context}</p>
              <p>{item.proposal}</p>
              {item.inventory_action && <p>{item.inventory_action}</p>}
              {item.promo && (
                <p>
                  {item.promo.alias}: {KIND_LABEL[item.promo.kind] ?? item.promo.kind}
                  {` · punto de equilibrio: ${breakEven == null ? 'no disponible' : `${(breakEven / 100).toFixed(1)}x`}`}
                </p>
              )}
            </article>
          );
        })}
        {storedAdvice && !storedAdvice.content.insufficient_data && storedAdvice.content.advice.length === 0 && (
          <p className="insights-muted">No hay consejos verificables para mostrar.</p>
        )}
        {storedAdvice && storedAdvice.content.advice.length > 0 && (
          <div className="insights-ai-feedback">
            <button
              type="button"
              className="btn btn-ghost"
              disabled={feedback !== null}
              onClick={() => void submitFeedback('useful')}
            >
              Útil
            </button>
            <button
              type="button"
              className="btn btn-ghost"
              disabled={feedback !== null}
              onClick={() => void submitFeedback('not_applicable')}
            >
              No aplica
            </button>
          </div>
        )}
      </div>
    </section>
  );
}

export default InsightsCard;