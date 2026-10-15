import { useEffect, useState } from 'react'
import { LISTED_TOURS } from './tours'
import { useTour } from './useTour'
import { isSeen, resetSeen } from './tourStorage'
import { syncTourTheme } from './tourTheme'
import './tour.css'

const TOTAL_STEPS = LISTED_TOURS.reduce((n, t) => n + t.steps.length, 0)

/** Panel de Ajustes → Tutoriales. */
export function TourPanel() {
  const { startTour, startAll } = useTour()
  const [, force] = useState(0)

  useEffect(() => {
    syncTourTheme()
  }, [])

  const seenCount = LISTED_TOURS.filter((t) => isSeen(t.id)).length
  const percent = Math.round((seenCount / LISTED_TOURS.length) * 100)

  return (
    <section className="settings-section tour-panel">
      <div className="tour-panel__hero">
        <div className="tour-panel__hero-text">
          <h3>Aprende CashFlow POS paso a paso</h3>
          <p>
            Recorre cada módulo con explicaciones sobre la pantalla real. Los tutoriales solo
            muestran y explican: no crean ni modifican datos. Desde cualquier pantalla puedes abrir
            el suyo con el botón “?”.
          </p>
        </div>
        <div className="tour-panel__hero-actions">
          <button type="button" className="tour-panel__hero-btn" onClick={() => void startAll()}>
            ▶ Ver tutorial completo
          </button>
          <button
            type="button"
            className="tour-panel__reset"
            onClick={() => {
              resetSeen()
              force((n) => n + 1)
            }}
          >
            Marcar todos como no vistos
          </button>
        </div>
      </div>

      <div className="tour-panel__progress">
        <span>
          {seenCount} de {LISTED_TOURS.length} módulos vistos · {TOTAL_STEPS} pasos en total
        </span>
        <div className="tour-panel__progress-bar">
          <div className="tour-panel__progress-fill" style={{ width: `${percent}%` }} />
        </div>
      </div>

      <div className="tour-panel__grid">
        {LISTED_TOURS.map((t) => (
          <div key={t.id} className="tour-card">
            <div className="tour-card__top">
              <span className="tour-card__icon">{t.icon}</span>
              <div>
                <div className="tour-card__title">{t.label}</div>
                <div className="tour-card__meta">{t.steps.length} pasos</div>
              </div>
              {isSeen(t.id) && <span className="tour-card__seen">✓ Visto</span>}
            </div>
            <p className="tour-card__desc">{t.description}</p>
            <button type="button" className="tour-card__btn" onClick={() => void startTour(t.id)}>
              Iniciar tutorial
            </button>
          </div>
        ))}
      </div>
    </section>
  )
}

export default TourPanel