export interface DevicePixelRatioObserverOptions {
  maxPixelRatio?: number
  matchMedia?: ((query: string) => MediaQueryList) | null
  readPixelRatio?: () => number
}

export function resolveDevicePixelRatio(
  maxPixelRatio = Number.POSITIVE_INFINITY,
  rawPixelRatio: unknown = typeof window !== 'undefined' ? window.devicePixelRatio : 1,
): number {
  const parsedPixelRatio = Number(rawPixelRatio)
  const pixelRatio = Number.isFinite(parsedPixelRatio) && parsedPixelRatio > 0
    ? parsedPixelRatio
    : 1
  const parsedMaximum = Number(maxPixelRatio)
  const maximum = Number.isFinite(parsedMaximum) && parsedMaximum > 0
    ? parsedMaximum
    : Number.POSITIVE_INFINITY
  return Math.max(1, Math.min(maximum, pixelRatio))
}

export function observeDevicePixelRatioChanges(
  onChange: (pixelRatio: number) => void,
  options: DevicePixelRatioObserverOptions = {},
): () => void {
  const maxPixelRatio = options.maxPixelRatio ?? Number.POSITIVE_INFINITY
  const readPixelRatio = options.readPixelRatio ?? (() => (
    typeof window !== 'undefined' ? window.devicePixelRatio : 1
  ))
  const matchMedia = options.matchMedia === undefined
    ? (
        typeof window !== 'undefined' && typeof window.matchMedia === 'function'
          ? window.matchMedia.bind(window)
          : null
      )
    : options.matchMedia
  if (!matchMedia) return () => {}

  let active = true
  let mediaQuery: MediaQueryList | null = null
  let registrationMode: 'modern' | 'legacy' | null = null

  const detach = (): void => {
    const currentQuery = mediaQuery
    const currentRegistrationMode = registrationMode
    mediaQuery = null
    registrationMode = null
    if (!currentQuery || !currentRegistrationMode) return
    try {
      if (currentRegistrationMode === 'legacy') {
        currentQuery.removeListener(handleChange)
      } else {
        currentQuery.removeEventListener('change', handleChange)
      }
    } catch {
      // Some embedded browser implementations expose a partial MediaQueryList.
    }
  }

  const arm = (): void => {
    if (!active) return
    detach()
    const observedPixelRatio = resolveDevicePixelRatio(
      Number.POSITIVE_INFINITY,
      readPixelRatio(),
    )
    try {
      const nextQuery = matchMedia(`(resolution: ${observedPixelRatio}dppx)`)
      try {
        nextQuery.addEventListener('change', handleChange)
        registrationMode = 'modern'
      } catch {
        nextQuery.addListener(handleChange)
        registrationMode = 'legacy'
      }
      mediaQuery = nextQuery
    } catch {
      mediaQuery = null
      registrationMode = null
    }
  }

  function handleChange(): void {
    if (!active) return
    detach()
    const renderedPixelRatio = resolveDevicePixelRatio(maxPixelRatio, readPixelRatio())
    try {
      onChange(renderedPixelRatio)
    } finally {
      arm()
    }
  }

  arm()

  return () => {
    if (!active) return
    active = false
    detach()
  }
}
