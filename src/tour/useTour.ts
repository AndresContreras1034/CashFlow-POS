import { useCallback, useRef } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import { driver, type DriveStep, type Driver } from 'driver.js'
import 'driver.js/dist/driver.css'
import './tour.css'
import { LISTED_TOURS, getTour, type TourDef } from './tours'
import { markSeen } from './tourStorage'

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

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

function runSteps(tour: TourDef, options: RunOptions = {}): Promise<boolean> {
  const sourceSteps = tour.steps
  const steps: DriveStep[] = sourceSteps.map((step) => ({
    element: step.element,
    popover: {
      title: step.title,
      description: step.description,
      side: step.side,
      align: 'start',
    },
  }))

  return new Promise((resolve) => {
    let completed = false

    const prepare = async (index: number) => {
      const step = sourceSteps[index]
      if (!step?.click) return
      document.querySelector<HTMLElement>(step.click)?.click()
      await sleep(250)
      if (step.element) await waitForElement(step.element, 3000)
    }

    const move = async (direction: 1 | -1) => {
      const target = (instance.getActiveIndex() ?? 0) + direction
      if (target >= 0 && target < sourceSteps.length) await prepare(target)
      if (direction === 1) instance.moveNext()
      else instance.movePrevious()
    }

    const instance: Driver = driver({
      steps,
      showProgress: true,
      allowClose: true,
      allowKeyboardControl: false,
      overlayOpacity: 0.65,
      stagePadding: 6,
      nextBtnText: 'Siguiente',
      prevBtnText: 'Anterior',
      doneBtnText: options.doneText ?? 'Finalizar',
      progressText: `${options.progressPrefix ?? ''}{{current}} de {{total}}`,
      popoverClass: 'cashflow-tour',
      onNextClick: () => void move(1),
      onPrevClick: () => void move(-1),
      onDestroyStarted: () => {
        completed = !instance.hasNextStep()
        instance.destroy()
      },
      onDestroyed: () => {
        document.body.removeAttribute('data-tour-active')
        resolve(completed)
      },
    })

    document.body.setAttribute('data-tour-active', 'true')
    void prepare(0).then(() => instance.drive())
  })
}

export function useTour() {
  const navigate = useNavigate()
  const { pathname } = useLocation()
  const pathRef = useRef(pathname)
  pathRef.current = pathname

  const prepareTour = useCallback(
    async (tour: TourDef) => {
      if (!tour.standalone && pathRef.current !== tour.route) {
        navigate(tour.route)
        await sleep(350)
      }
      if (tour.waitFor) await waitForElement(tour.waitFor)
      await sleep(500)
    },
    [navigate],
  )

  const startTour = useCallback(
    async (id: string) => {
      const tour = getTour(id)
      if (!tour) return
      await prepareTour(tour)
      if (await runSteps(tour)) markSeen(id)
    },
    [prepareTour],
  )

  const startAll = useCallback(async () => {
    for (let index = 0; index < LISTED_TOURS.length; index++) {
      const tour = LISTED_TOURS[index]
      await prepareTour(tour)
      const isLast = index === LISTED_TOURS.length - 1
      const completed = await runSteps(tour, {
        progressPrefix: `${tour.label} (${index + 1}/${LISTED_TOURS.length}) · `,
        doneText: isLast ? 'Finalizar' : 'Siguiente módulo',
      })
      if (!completed) return
      markSeen(tour.id)
    }
  }, [prepareTour])

  return { startTour, startAll }
}
