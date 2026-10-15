import { useEffect } from 'react'
import { useLocation } from 'react-router-dom'
import { getTourByRoute } from './tours'
import { useTour } from './useTour'
import { syncTourTheme } from './tourTheme'
import './tour.css'

/** Botón flotante "?" que lanza el tutorial de la pantalla actual. */
export function TourHelpButton() {
  const { pathname } = useLocation()
  const { startTour } = useTour()
  const tour = getTourByRoute(pathname)

  // Mantiene los colores del botón alineados con el tema activo
  useEffect(() => {
    syncTourTheme()
    const observer = new MutationObserver(() => syncTourTheme())
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme'],
    })
    return () => observer.disconnect()
  }, [pathname])

  // Autocuración: si quedó la marca de "tour activo" sin tour, se limpia
  useEffect(() => {
    if (!document.body.classList.contains('driver-active')) {
      document.body.removeAttribute('data-tour-active')
    }
  }, [pathname])

  if (!tour) return null

  return (
    <button
      type="button"
      className="tour-help-btn"
      title={`Tutorial: ${tour.label}`}
      aria-label={`Ver tutorial de ${tour.label}`}
      onClick={() => void startTour(tour.id)}
    >
      <span className="tour-help-btn__q">?</span>
      <span className="tour-help-btn__label">Tutorial</span>
    </button>
  )
}

export default TourHelpButton