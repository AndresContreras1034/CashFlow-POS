import { useLocation } from 'react-router-dom'
import { getTourByRoute } from './tours'
import { useTour } from './useTour'
import './tour.css'

/** Botón flotante "?" que lanza el tutorial de la pantalla actual. */
export function TourHelpButton() {
  const { pathname } = useLocation()
  const { startTour } = useTour()
  const tour = getTourByRoute(pathname)

  if (!tour) return null

  return (
    <button
      type="button"
      className="tour-help-btn"
      title={`Tutorial: ${tour.label}`}
      aria-label={`Ver tutorial de ${tour.label}`}
      onClick={() => void startTour(tour.id)}
    >
      ?
    </button>
  )
}

export default TourHelpButton