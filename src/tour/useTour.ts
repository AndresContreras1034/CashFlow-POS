import { useCallback, useRef } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import { driver, type DriveStep, type Driver } from 'driver.js'
import 'driver.js/dist/driver.css'
import './tour.css'
import { LISTED_TOURS, getTour, type TourDef } from './tours'
import { markSeen } from './tourStorage'
import { syncTourTheme } from './tourTheme'

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

/* ---------- Control de concurrencia (anti doble clic / clics repetidos) ---------- */
type Phase = 'idle' | 'preparing' | 'running'
let phase: Phase = 'idle'
let phaseSince = 0
let currentDriver: Driver | null = null

/** Intenta tomar el control. Si quedó algo colgado, lo libera. */
function acquire(): boolean {
  if (phase !== 'idle') {
    const driverAlive = document.body.classList.contains('driver-active')
    const stuck =
      (phase === 'running' && !driverAlive) ||
      (phase === 'preparing' && Date.now() - phaseSince > 8000)
    if (!stuck) return false
    try {
      currentDriver?.destroy()
    } catch {
      /* ya estaba destruido */
    }
    currentDriver = null
  }
  phase = 'preparing'
  phaseSince = Date.now()
  return true
}

function release() {
  phase = 'idle'
  currentDriver = null
  document.body.removeAttribute('data-tour-active')
}

async function waitForElement(selector: string, timeout = 2500): Promise<boolean> {
  const start = Date.now()
  while (Date.now() - start < timeout) {
    if (document.querySelector(selector)) return true
    await sleep(100)
  }
  return false
}

interface RunOptions {
  /** Texto de contexto, p. ej. "Módulo 3 de 11" en el tour completo. */
  context?: string
  doneText?: string
}

/** Quita lo que inyectamos en el popover del paso anterior. */
function clearInjected(wrapper: HTMLElement) {
  wrapper.querySelectorAll('.tour-head, .tour-bar, .tour-hero').forEach((n) => n.remove())
}

/** Ejecuta un tour. Devuelve true si el usuario llegó al último paso. */
function runSteps(tour: TourDef, opts: RunOptions = {}): Promise<boolean> {
  const src = tour.steps
  const overlayColor = syncTourTheme()

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
    let moving = false

    // Algunos pasos necesitan pulsar algo antes (p. ej. cambiar de pestaña)
    const prepare = async (index: number) => {
      const step = src[index]
      if (!step?.click) return
      document.querySelector<HTMLElement>(step.click)?.click()
      await sleep(250)
      if (step.element) await waitForElement(step.element, 3000)
    }

    // Un solo movimiento a la vez: los clics repetidos se ignoran
    const move = async (dir: 1 | -1) => {
      if (moving) return
      moving = true
      try {
        const target = (d.getActiveIndex() ?? 0) + dir
        if (target >= 0 && target < src.length) await prepare(target)
        if (dir === 1) d.moveNext()
        else d.movePrevious()
      } finally {
        moving = false
      }
    }

    const d: Driver = driver({
      steps,
      showProgress: false, // usamos nuestra propia barra
      allowClose: true,
      // El escáner de código de barras escribe como teclado: no queremos que
      // las teclas controlen el tour por accidente.
      allowKeyboardControl: false,
      overlayColor,
      overlayOpacity: 1,
      stagePadding: 8,
      stageRadius: 10,
      smoothScroll: true,
      nextBtnText: 'Siguiente →',
      prevBtnText: '← Anterior',
      doneBtnText: opts.doneText ?? '✓ Finalizar',
      popoverClass: 'cashflow-tour',
      onNextClick: () => void move(1),
      onPrevClick: () => void move(-1),
      onPopoverRender: (popover, { state }) => {
        const w = popover.wrapper
        clearInjected(w)

        const index = d.getActiveIndex() ?? 0
        const total = src.length
        const isIntro = state.activeElement?.id === 'driver-dummy-element'
        w.classList.toggle('is-intro', isIntro)

        // Barra de progreso
        const bar = document.createElement('div')
        bar.className = 'tour-bar'
        const fill = document.createElement('div')
        fill.className = 'tour-bar__fill'
        fill.style.width = `${((index + 1) / total) * 100}%`
        bar.appendChild(fill)
        w.insertBefore(bar, w.firstChild)

        // Cabecera: ícono + módulo + paso
        const head = document.createElement('div')
        head.className = 'tour-head'
        const chip = document.createElement('span')
        chip.className = 'tour-chip'
        chip.textContent = `${tour.icon}  ${tour.label}`
        const stepText = document.createElement('span')
        stepText.className = 'tour-step'
        stepText.textContent = `${opts.context ? opts.context + ' · ' : ''}Paso ${index + 1} de ${total}`
        head.append(chip, stepText)
        w.insertBefore(head, bar.nextSibling)

        // Ícono grande en pasos de introducción (sin elemento resaltado)
        if (isIntro) {
          const hero = document.createElement('div')
          hero.className = 'tour-hero'
          hero.textContent = tour.icon
          w.insertBefore(hero, head.nextSibling)
        }
      },
      onDestroyStarted: () => {
        completed = !d.hasNextStep()
        d.destroy()
      },
      onDestroyed: () => {
        document.body.removeAttribute('data-tour-active')
        resolve(completed)
      },
    })

    currentDriver = d
    document.body.setAttribute('data-tour-active', 'true')
    prepare(0)
      .then(() => {
        phase = 'running'
        d.drive()
      })
      .catch(() => resolve(false))
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
      if (!tour || !acquire()) return
      try {
        await prepare(tour)
        const done = await runSteps(tour)
        if (done) markSeen(id)
      } catch (error) {
        console.error('Error en el tutorial:', error)
      } finally {
        release()
      }
    },
    [prepare],
  )

  /** Todos los módulos encadenados; si el usuario cierra, se detiene. */
  const startAll = useCallback(async () => {
    if (!acquire()) return
    try {
      for (let i = 0; i < LISTED_TOURS.length; i++) {
        const tour = LISTED_TOURS[i]
        phase = 'preparing'
        phaseSince = Date.now()
        await prepare(tour)
        const isLast = i === LISTED_TOURS.length - 1
        const done = await runSteps(tour, {
          context: `Módulo ${i + 1} de ${LISTED_TOURS.length}`,
          doneText: isLast ? '✓ Finalizar' : 'Siguiente módulo →',
        })
        if (!done) return
        markSeen(tour.id)
      }
    } catch (error) {
      console.error('Error en el tutorial:', error)
    } finally {
      release()
    }
  }, [prepare])

  return { startTour, startAll }
}