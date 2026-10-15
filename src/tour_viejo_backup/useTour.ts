import { useCallback, useRef } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import { driver, type DriveStep, type Driver } from 'driver.js'
import 'driver.js/dist/driver.css'
import './tour.css'
import { LISTED_TOURS, getTour, type TourDef } from './tours'
import { markSeen } from './tourStorage'

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

async function waitForElement(selector: string, timeout = 2500): Promise<boolean> {
  const start = Date.now()
  while (Date.now() - start < timeout) {
    if (document.querySelector(selector)) return true
    await sleep(100)
  }
  return false
}

interface RunOptions {
  progressPrefix?: string
  doneText?: string
}

/** Ejecuta un tour. Devuelve true si el usuario llegó al último paso. */
function runSteps(tour: TourDef, opts: RunOptions = {}): Promise<boolean> {
  const src = tour.steps

  // Los pasos cuyo elemento no existe se muestran centrados (comportamiento de driver.js).
  const steps: DriveStep[] = src.map((s) => ({
    element: s.element,
    popover: {
      title: s.title,
      description: s.description,
      side: s.side,
      align: 'start',
    },
  }))

  return new Promise((resolve) => {
    let completed = false

    // Algunos pasos necesitan pulsar algo antes (p. ej. cambiar de pestaña)
    const prepare = async (index: number) => {
      const step = src[index]
      if (!step?.click) return
      document.querySelector<HTMLElement>(step.click)?.click()
      await sleep(250)
      if (step.element) await waitForElement(step.element, 3000)
    }

    const move = async (dir: 1 | -1) => {
      const target = (d.getActiveIndex() ?? 0) + dir
      if (target >= 0 && target < src.length) await prepare(target)
      if (dir === 1) d.moveNext()
      else d.movePrevious()
    }

    const d: Driver = driver({
      steps,
      showProgress: true,
      allowClose: true,
      // El escáner de código de barras escribe como teclado: no queremos que
      // las teclas controlen el tour por accidente.
      allowKeyboardControl: false,
      overlayOpacity: 0.65,
      stagePadding: 6,
      nextBtnText: 'Siguiente',
      prevBtnText: 'Anterior',
      doneBtnText: opts.doneText ?? 'Finalizar',
      progressText: `${opts.progressPrefix ?? ''}{{current}} de {{total}}`,
      popoverClass: 'cashflow-tour',
      onNextClick: () => void move(1),
      onPrevClick: () => void move(-1),
      onDestroyStarted: () => {
        completed = !d.hasNextStep()
        d.destroy()
      },
      onDestroyed: () => {
        document.body.removeAttribute('data-tour-active')
        resolve(completed)
      },
    })

    document.body.setAttribute('data-tour-active', 'true')
    void prepare(0).then(() => d.drive())
  })
}

export function useTour() {
  const navigate = useNavigate()
  const { pathname } = useLocation()

  // Ref para leer siempre la ruta actual, incluso dentro del tour completo
  const pathRef = useRef(pathname)
  pathRef.current = pathname

  const prepare = useCallback(
    async (tour: TourDef) => {
      if (!tour.standalone && pathRef.current !== tour.route) {
        navigate(tour.route)
        await sleep(350)
      }
      if (tour.waitFor) await waitForElement(tour.waitFor)
      await sleep(500) // deja terminar la carga de datos
    },
    [navigate],
  )

  /** Tour de un solo módulo. */
  const startTour = useCallback(
    async (id: string) => {
      const tour = getTour(id)
      if (!tour) return
      await prepare(tour)
      const done = await runSteps(tour)
      if (done) markSeen(id)
    },
    [prepare],
  )

  /** Todos los módulos encadenados; si el usuario cierra, se detiene. */
  const startAll = useCallback(async () => {
    for (let i = 0; i < LISTED_TOURS.length; i++) {
      const tour = LISTED_TOURS[i]
      await prepare(tour)
      const isLast = i === LISTED_TOURS.length - 1
      const done = await runSteps(tour, {
        progressPrefix: `${tour.label} (${i + 1}/${LISTED_TOURS.length}) · `,
        doneText: isLast ? 'Finalizar' : 'Siguiente módulo',
      })
      if (!done) return
      markSeen(tour.id)
    }
  }, [prepare])

  return { startTour, startAll }
}