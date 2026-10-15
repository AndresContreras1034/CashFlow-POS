import { useState } from 'react'
import { LISTED_TOURS } from './tours'
import { useTour } from './useTour'
import { isSeen, resetSeen } from './tourStorage'
import './tour.css'

/** Panel para Ajustes → Tutoriales. */
export function TourPanel() {
  const { startTour, startAll } = useTour()
  const [, force] = useState(0)

  return (
    <section className="settings-section tour-panel">
      <h2 className="settings-section-title">Tutoriales</h2>
      <p className="settings-hint">
        Aprende a usar cada módulo paso a paso. Los tutoriales solo explican la pantalla: no
        crean ni modifican datos. También puedes abrir el de la pantalla actual con el botón “?”
        de la esquina.
      </p>

      <div className="tour-panel__actions">
        <button type="button" className="btn btn-primary" onClick={() => void startAll()}>
          Ver tutorial completo
        </button>
        <button
          type="button"
          className="btn btn-ghost"
          onClick={() => {
            resetSeen()
            force((n) => n + 1)
          }}
        >
          Marcar todos como no vistos
        </button>
      </div>

      <div className="tour-panel__list">
        {LISTED_TOURS.map((t) => (
          <div key={t.id} className="tour-panel__item">
            <div>
              <strong>{t.label}</strong>
              {isSeen(t.id) && <span className="tour-panel__seen">✓ visto</span>}
              <small>{t.description}</small>
            </div>
            <button type="button" className="btn btn-ghost" onClick={() => void startTour(t.id)}>
              Iniciar
            </button>
          </div>
        ))}
      </div>
    </section>
  )
}

export default TourPanel