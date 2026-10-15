interface RgbColor {
  red: number
  green: number
  blue: number
}

const parseColor = (value: string): RgbColor | null => {
  const hex = value.match(/^#([\da-f]{3}|[\da-f]{6})$/i)?.[1]
  if (hex) {
    const normalized = hex.length === 3
      ? [...hex].map((channel) => channel + channel).join('')
      : hex
    const channels = normalized.match(/.{2}/g)
    if (!channels) return null
    const [red, green, blue] = channels.map((channel) => Number.parseInt(channel, 16))
    return { red, green, blue }
  }

  const channels = value.match(/^rgba?\(([^)]+)\)$/i)?.[1]
  if (!channels) return null
  const [red, green, blue] = channels.split(',').slice(0, 3).map(Number)
  if (![red, green, blue].every(Number.isFinite)) return null
  return { red, green, blue }
}

const luminance = ({ red, green, blue }: RgbColor): number => {
  const linearize = (channel: number) => {
    const normalized = channel / 255
    return normalized <= 0.04045
      ? normalized / 12.92
      : ((normalized + 0.055) / 1.055) ** 2.4
  }
  return (
    0.2126 * linearize(red) +
    0.7152 * linearize(green) +
    0.0722 * linearize(blue)
  )
}

const contrastRatio = (foreground: string, background: string): number => {
  const fg = parseColor(foreground)
  const bg = parseColor(background)
  if (!fg || !bg) return 0
  const values = [luminance(fg), luminance(bg)].sort((a, b) => b - a)
  return (values[0] + 0.05) / (values[1] + 0.05)
}

const mixWith = (color: RgbColor, target: RgbColor, amount: number): RgbColor => ({
  red: Math.round(color.red + (target.red - color.red) * amount),
  green: Math.round(color.green + (target.green - color.green) * amount),
  blue: Math.round(color.blue + (target.blue - color.blue) * amount),
})

const toCssColor = ({ red, green, blue }: RgbColor): string =>
  `rgb(${red}, ${green}, ${blue})`

const ensureActionContrast = (
  colors: [string, string],
  foreground: string,
): [string, string] => {
  const parsed = colors.map(parseColor)
  const fg = parseColor(foreground)
  if (!fg || parsed.some((color) => !color)) return colors

  const backgrounds = parsed as [RgbColor, RgbColor]
  if (colors.every((color) => contrastRatio(color, foreground) >= 4.5)) return colors

  const target = luminance(fg) > 0.5
    ? { red: 0, green: 0, blue: 0 }
    : { red: 255, green: 255, blue: 255 }
  let low = 0
  let high = 1

  for (let attempt = 0; attempt < 16; attempt++) {
    const amount = (low + high) / 2
    const meetsContrast = backgrounds.every((color) =>
      contrastRatio(toCssColor(mixWith(color, target, amount)), foreground) >= 4.5,
    )
    if (meetsContrast) high = amount
    else low = amount
  }

  return backgrounds.map((color) => toCssColor(mixWith(color, target, high))) as [
    string,
    string,
  ]
}

const readToken = (
  styles: CSSStyleDeclaration,
  name: string,
  fallback: string,
): string => styles.getPropertyValue(name).trim() || fallback

/** Elige texto legible sobre el color de acento del tema activo. */
export function syncTourTheme(): string {
  const root = document.documentElement
  const styles = getComputedStyle(root)
  const accent = readToken(styles, '--accent', '#334155')
  const accentHover = readToken(styles, '--accent-hover', accent)
  const primaryText = readToken(styles, '--text-primary', '#1f2933')
  const surface = readToken(styles, '--bg-surface', '#ffffff')

  const foreground = [primaryText, surface, '#ffffff', '#000000'].reduce(
    (best, candidate) => {
      const candidateContrast = Math.min(
        contrastRatio(accent, candidate),
        contrastRatio(accentHover, candidate),
      )
      return candidateContrast > best.contrast
        ? { color: candidate, contrast: candidateContrast }
        : best
    },
    { color: surface, contrast: 0 },
  ).color
  const [action, actionHover] = ensureActionContrast([accent, accentHover], foreground)

  root.style.setProperty('--tour-accent-fg', foreground)
  root.style.setProperty('--tour-action', action)
  root.style.setProperty('--tour-action-hover', actionHover)
  return readToken(styles, '--overlay-bg', 'rgba(0, 0, 0, 0.6)')
}
