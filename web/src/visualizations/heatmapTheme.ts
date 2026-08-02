import {
  resolveLightweightChartTheme,
  type LightweightChartTheme,
} from './chartTheme'

export type HeatmapPaletteScheme = 'objective' | 'plateau'

interface HeatmapPaletteMetadata {
  scheme?: HeatmapPaletteScheme
  theme?: LightweightChartTheme
  interpolation?: 'oklab'
}

export type HeatmapPalette =
  | (HeatmapPaletteMetadata & {
      kind: 'diverging'
      negative: string
      zero: string
      positive: string
    })
  | (HeatmapPaletteMetadata & {
      kind: 'sequential'
      low: string
      high: string
      stops?: readonly string[]
    })

export interface HeatmapLegendStop {
  offset: number
  color: string
}

export interface HeatmapLegendRange {
  min: number
  max: number
}

export interface HeatmapLegendRangeAnchors {
  min: number
  middle: number
  max: number
  middleOffset: number
  zeroOffset: number | null
}

export interface HeatmapPaletteSemanticLabels {
  low: string
  middle: string
  high: string
}

export interface HeatmapVisualizationTokens {
  unavailableFill: string
  chipFill: string
  chipBorder: string
  chipText: string
  markerFill: string
  markerStroke: string
  markerHalo: string
  selectionOuter: string
  selectionInner: string
  selectionGuide: string
  surfaceUnavailable: string
  surfacePartial: string
  surfaceFloor: string
  surfaceGrid: string
  axisFast: string
  axisSlow: string
  selectionLocator: string
}

interface RgbColor {
  r: number
  g: number
  b: number
}

interface OklabColor {
  l: number
  a: number
  b: number
}

const LIGHT_OBJECTIVE: HeatmapPalette = {
  kind: 'diverging',
  scheme: 'objective',
  theme: 'light',
  interpolation: 'oklab',
  negative: '#B6405B',
  zero: '#F4F2EC',
  positive: '#006A73',
}

const DARK_OBJECTIVE: HeatmapPalette = {
  kind: 'diverging',
  scheme: 'objective',
  theme: 'dark',
  interpolation: 'oklab',
  negative: '#FF8A73',
  zero: '#26313A',
  positive: '#50C4C8',
}

const LIGHT_PLATEAU: HeatmapPalette = {
  kind: 'sequential',
  scheme: 'plateau',
  theme: 'light',
  interpolation: 'oklab',
  low: '#2D245F',
  high: '#F4D35E',
  stops: ['#2D245F', '#2F659A', '#159486', '#7DBD67', '#F4D35E'],
}

const DARK_PLATEAU: HeatmapPalette = {
  kind: 'sequential',
  scheme: 'plateau',
  theme: 'dark',
  interpolation: 'oklab',
  low: '#4C4B9D',
  high: '#F5D76E',
  stops: ['#4C4B9D', '#3F79B5', '#35A59A', '#8AC76C', '#F5D76E'],
}

const OKLAB_CACHE = new Map<string, OklabColor>()

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, value))
}

function parseHexColor(value: string): RgbColor {
  const normalized = String(value || '').replace('#', '').trim()
  const expanded =
    normalized.length === 3
      ? normalized
          .split('')
          .map((part) => `${part}${part}`)
          .join('')
      : normalized.padStart(6, '0').slice(0, 6)
  const parsed = Number.parseInt(expanded, 16)
  return {
    r: (parsed >> 16) & 0xff,
    g: (parsed >> 8) & 0xff,
    b: parsed & 0xff,
  }
}

function srgbChannelToLinear(channel: number): number {
  const normalized = clamp01(channel / 255)
  return normalized <= 0.04045
    ? normalized / 12.92
    : ((normalized + 0.055) / 1.055) ** 2.4
}

function linearChannelToSrgb(channel: number): number {
  const normalized =
    channel <= 0.0031308
      ? channel * 12.92
      : 1.055 * Math.max(0, channel) ** (1 / 2.4) - 0.055
  return Math.round(clamp01(normalized) * 255)
}

function rgbToOklab(rgb: RgbColor): OklabColor {
  const r = srgbChannelToLinear(rgb.r)
  const g = srgbChannelToLinear(rgb.g)
  const b = srgbChannelToLinear(rgb.b)

  const l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b
  const m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b
  const s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b
  const lRoot = Math.cbrt(l)
  const mRoot = Math.cbrt(m)
  const sRoot = Math.cbrt(s)

  return {
    l: 0.2104542553 * lRoot + 0.793617785 * mRoot - 0.0040720468 * sRoot,
    a: 1.9779984951 * lRoot - 2.428592205 * mRoot + 0.4505937099 * sRoot,
    b: 0.0259040371 * lRoot + 0.7827717662 * mRoot - 0.808675766 * sRoot,
  }
}

function oklabToRgb(color: OklabColor): RgbColor {
  const lRoot = color.l + 0.3963377774 * color.a + 0.2158037573 * color.b
  const mRoot = color.l - 0.1055613458 * color.a - 0.0638541728 * color.b
  const sRoot = color.l - 0.0894841775 * color.a - 1.291485548 * color.b
  const l = lRoot ** 3
  const m = mRoot ** 3
  const s = sRoot ** 3

  return {
    r: linearChannelToSrgb(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
    g: linearChannelToSrgb(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
    b: linearChannelToSrgb(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s),
  }
}

function cachedOklab(value: string): OklabColor {
  const key = String(value || '').trim().toUpperCase()
  const cached = OKLAB_CACHE.get(key)
  if (cached) return cached
  const color = rgbToOklab(parseHexColor(key))
  OKLAB_CACHE.set(key, color)
  return color
}

function rgbCss(rgb: RgbColor): string {
  return `rgb(${rgb.r}, ${rgb.g}, ${rgb.b})`
}

export function interpolateHeatmapColorOklab(
  from: string,
  to: string,
  amount: number,
): string {
  const ratio = clamp01(Number.isFinite(amount) ? amount : 0)
  if (ratio <= 0) return rgbCss(parseHexColor(from))
  if (ratio >= 1) return rgbCss(parseHexColor(to))
  const left = cachedOklab(from)
  const right = cachedOklab(to)
  return rgbCss(
    oklabToRgb({
      l: left.l + (right.l - left.l) * ratio,
      a: left.a + (right.a - left.a) * ratio,
      b: left.b + (right.b - left.b) * ratio,
    }),
  )
}

function sampleSequentialPalette(
  palette: Extract<HeatmapPalette, { kind: 'sequential' }>,
  ratio: number,
): string {
  const stops = palette.stops?.length ? palette.stops : [palette.low, palette.high]
  if (stops.length === 1) {
    const only = stops[0] ?? palette.low
    return interpolateHeatmapColorOklab(only, only, 0)
  }
  const position = clamp01(ratio) * (stops.length - 1)
  const leftIndex = Math.min(stops.length - 2, Math.floor(position))
  return interpolateHeatmapColorOklab(
    stops[leftIndex] ?? palette.low,
    stops[leftIndex + 1] ?? palette.high,
    position - leftIndex,
  )
}

export function heatmapPalette(
  mode: HeatmapPaletteScheme,
  theme: LightweightChartTheme = resolveLightweightChartTheme(),
): HeatmapPalette {
  if (mode === 'plateau') {
    return theme === 'dark' ? { ...DARK_PLATEAU } : { ...LIGHT_PLATEAU }
  }
  return theme === 'dark' ? { ...DARK_OBJECTIVE } : { ...LIGHT_OBJECTIVE }
}

export function resolveHeatmapPaletteTheme(
  palette: HeatmapPalette,
  theme: LightweightChartTheme = resolveLightweightChartTheme(),
): HeatmapPalette {
  return palette.scheme ? heatmapPalette(palette.scheme, theme) : palette
}

export function sampleHeatmapColor(
  value: number,
  minValue: number,
  maxValue: number,
  palette: HeatmapPalette,
  theme: LightweightChartTheme = resolveLightweightChartTheme(),
): string {
  const resolved = resolveHeatmapPaletteTheme(palette, theme)
  if (resolved.kind === 'sequential') {
    const denominator = Math.max(1.0e-12, maxValue - minValue)
    return sampleSequentialPalette(resolved, (value - minValue) / denominator)
  }

  const anchor = Math.max(1.0e-12, Math.abs(minValue), Math.abs(maxValue))
  if (value < 0) {
    return interpolateHeatmapColorOklab(
      resolved.zero,
      resolved.negative,
      Math.abs(value) / anchor,
    )
  }
  if (value > 0) {
    return interpolateHeatmapColorOklab(
      resolved.zero,
      resolved.positive,
      value / anchor,
    )
  }
  return interpolateHeatmapColorOklab(resolved.zero, resolved.zero, 0)
}

export function heatmapLegendStops(
  palette: HeatmapPalette,
  theme: LightweightChartTheme = resolveLightweightChartTheme(),
  sampleCount = 24,
  range?: HeatmapLegendRange,
): HeatmapLegendStop[] {
  const resolved = resolveHeatmapPaletteTheme(palette, theme)
  const intervals = Math.max(4, Math.floor(sampleCount))
  const anchors = range ? heatmapLegendRangeAnchors(range, resolved) : null
  const stops: HeatmapLegendStop[] = []
  for (let index = 0; index <= intervals; index += 1) {
    const offset = index / intervals
    const color = anchors
      ? sampleHeatmapColor(
          anchors.min + (anchors.max - anchors.min) * offset,
          anchors.min,
          anchors.max,
          resolved,
          theme,
        )
      : resolved.kind === 'sequential'
        ? sampleSequentialPalette(resolved, offset)
        : offset <= 0.5
          ? interpolateHeatmapColorOklab(
              resolved.negative,
              resolved.zero,
              offset * 2,
            )
          : interpolateHeatmapColorOklab(
              resolved.zero,
              resolved.positive,
              (offset - 0.5) * 2,
            )
    stops.push({ offset, color })
  }
  return stops
}

function normalizedHeatmapLegendRange(
  range: HeatmapLegendRange,
): { min: number; max: number } {
  const first = Number(range.min)
  const second = Number(range.max)
  if (!Number.isFinite(first) && !Number.isFinite(second)) return { min: -1, max: 1 }
  if (!Number.isFinite(first)) return { min: second, max: second }
  if (!Number.isFinite(second)) return { min: first, max: first }
  return first <= second
    ? { min: first, max: second }
    : { min: second, max: first }
}

export function heatmapLegendZeroOffset(
  range: HeatmapLegendRange,
): number | null {
  const { min, max } = normalizedHeatmapLegendRange(range)
  if (min > 0 || max < 0) return null
  if (min === max) return min === 0 ? 0.5 : null
  return clamp01((0 - min) / (max - min))
}

export function heatmapLegendRangeAnchors(
  range: HeatmapLegendRange,
  palette: HeatmapPalette,
): HeatmapLegendRangeAnchors {
  const { min, max } = normalizedHeatmapLegendRange(range)
  const zeroOffset = palette.kind === 'diverging'
    ? heatmapLegendZeroOffset({ min, max })
    : null
  const zeroIsInterior = zeroOffset != null && zeroOffset > 0 && zeroOffset < 1
  return {
    min,
    middle: zeroIsInterior ? 0 : min + (max - min) / 2,
    max,
    middleOffset: zeroIsInterior ? zeroOffset : 0.5,
    zeroOffset,
  }
}

export function heatmapPaletteSemanticLabels(
  palette: HeatmapPalette,
  range?: HeatmapLegendRange,
): HeatmapPaletteSemanticLabels {
  if (palette.kind !== 'diverging') {
    return { low: 'Fragile', middle: 'Developing', high: 'Resilient' }
  }
  if (!range) return { low: 'Negative', middle: 'Neutral · 0', high: 'Positive' }

  const anchors = heatmapLegendRangeAnchors(range, palette)
  if (anchors.min >= 0) {
    return {
      low: anchors.min === 0 ? 'Neutral · 0' : 'Lower positive',
      middle: 'Positive',
      high: 'Higher positive',
    }
  }
  if (anchors.max <= 0) {
    return {
      low: 'More negative',
      middle: 'Negative',
      high: anchors.max === 0 ? 'Neutral · 0' : 'Nearer zero',
    }
  }
  return { low: 'Negative', middle: 'Neutral · 0', high: 'Positive' }
}

export function heatmapVisualizationTokens(
  theme: LightweightChartTheme = resolveLightweightChartTheme(),
): Readonly<HeatmapVisualizationTokens> {
  if (theme === 'dark') {
    return {
      unavailableFill: 'rgba(148, 163, 184, 0.13)',
      chipFill: 'rgba(2, 6, 23, 0.88)',
      chipBorder: 'rgba(248, 250, 252, 0.48)',
      chipText: 'rgba(248, 250, 252, 0.98)',
      markerFill: '#F8FAFC',
      markerStroke: '#020617',
      markerHalo: 'rgba(248, 250, 252, 0.55)',
      selectionOuter: 'rgba(248, 250, 252, 0.98)',
      selectionInner: 'rgba(2, 6, 23, 0.96)',
      selectionGuide: 'rgba(248, 250, 252, 0.34)',
      surfaceUnavailable: '#536171',
      surfacePartial: '#64748B',
      surfaceFloor: '#334155',
      surfaceGrid: '#CBD5E1',
      axisFast: '#67E8F9',
      axisSlow: '#C4B5FD',
      selectionLocator: '#FDE047',
    }
  }
  return {
    unavailableFill: 'rgba(71, 85, 105, 0.16)',
    chipFill: 'rgba(255, 253, 250, 0.92)',
    chipBorder: 'rgba(15, 23, 42, 0.48)',
    chipText: 'rgba(15, 23, 42, 0.98)',
    markerFill: '#FFFFFF',
    markerStroke: '#0F172A',
    markerHalo: 'rgba(15, 23, 42, 0.44)',
    selectionOuter: 'rgba(255, 255, 255, 0.98)',
    selectionInner: 'rgba(15, 23, 42, 0.96)',
    selectionGuide: 'rgba(15, 23, 42, 0.32)',
    surfaceUnavailable: '#9AA9B8',
    surfacePartial: '#B7C4D3',
    surfaceFloor: '#64748B',
    surfaceGrid: '#475569',
    axisFast: '#007A78',
    axisSlow: '#7C3AED',
    selectionLocator: '#CA8A04',
  }
}

export function relativeLuminance(color: string): number {
  const rgb = parseHexColor(color)
  return (
    0.2126 * srgbChannelToLinear(rgb.r) +
    0.7152 * srgbChannelToLinear(rgb.g) +
    0.0722 * srgbChannelToLinear(rgb.b)
  )
}

export function heatmapContrastRatio(left: string, right: string): number {
  const leftLuminance = relativeLuminance(left)
  const rightLuminance = relativeLuminance(right)
  return (
    (Math.max(leftLuminance, rightLuminance) + 0.05) /
    (Math.min(leftLuminance, rightLuminance) + 0.05)
  )
}

export function observeHeatmapTheme(
  onChange: (theme: LightweightChartTheme) => void,
  root: HTMLElement | null =
    typeof document !== 'undefined' ? document.documentElement : null,
): () => void {
  if (!root || typeof MutationObserver === 'undefined') return () => undefined
  let current = resolveLightweightChartTheme(root)
  let observer: MutationObserver | null = null
  try {
    observer = new MutationObserver(() => {
      const next = resolveLightweightChartTheme(root)
      if (next === current) return
      current = next
      onChange(next)
    })
    observer.observe(root, { attributes: true, attributeFilter: ['data-theme'] })
  } catch {
    try {
      observer?.disconnect()
    } catch {
      // Theme observation is progressive enhancement; setup failures are nonfatal.
    }
    return () => undefined
  }
  return () => {
    try {
      observer?.disconnect()
    } catch {
      // A failing observer teardown must not block component disposal.
    }
  }
}
