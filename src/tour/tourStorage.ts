const KEY = 'cashflow_tours_seen'

function read(): Record<string, boolean> {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? '{}')
  } catch {
    return {}
  }
}

export function isSeen(id: string): boolean {
  return !!read()[id]
}

export function markSeen(id: string): void {
  try {
    localStorage.setItem(KEY, JSON.stringify({ ...read(), [id]: true }))
  } catch {
    /* localStorage no disponible */
  }
}

export function resetSeen(): void {
  try {
    localStorage.removeItem(KEY)
  } catch {
    /* noop */
  }
}