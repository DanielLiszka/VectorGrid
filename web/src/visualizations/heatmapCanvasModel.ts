import {
  fastForHeatmapCol,
  heatmapIndex,
  heatmapPalette,
  labelDerivedPlateau,
  resolveNullableHeatmapRange,
  slowForHeatmapLogicalRow,
  type HeatmapMode,
  type HeatmapModel,
  type HeatmapPalette,
  type HeatmapPlateauCellDiagnostics,
  type HeatmapRange,
} from './heatmapModel'
import type {
  Heatmap3dSlice,
  HeatmapAggregateWinnerCell,
  HeatmapSlice3dModel,
  HeatmapSliceCell,
} from './heatmapSliceModel'
import type { LightweightChartTheme } from './chartTheme'
import {
  heatmapLegendRangeAnchors,
  heatmapLegendStops,
  heatmapVisualizationTokens,
  resolveHeatmapPaletteTheme,
  sampleHeatmapColor,
} from './heatmapTheme'

export { sampleHeatmapColor } from './heatmapTheme'

export interface HeatmapMarker {
  col: number
  drawRow: number
  label: string
}

export interface HeatmapResolvedCell {
  col: number
  drawRow: number
  logicalRow: number
  idx: number
}

export interface HeatmapHoverCell {
  col: number
  row: number
}

export interface HeatmapCanvasViewport {
  scale: number
  offsetX: number
  offsetY: number
}

export interface HeatmapCanvasRequestModel {
  fastAxisLabel: string
  slowAxisLabel: string
  fastAxisDescription: string
  slowAxisDescription: string
  supports3dSlices: boolean
  threeDimensionalUnavailableReason: string
  fastMaType: string
  slowMaType: string
  fastMaParams: unknown
  slowMaParams: unknown
  scoreLabel: string
}

export type HeatmapSliceCanvasMode = 'slice' | 'delta'

export type HeatmapFitMode = 'full' | 'valid'

export interface HeatmapCanvasSliceSelection {
  mode: HeatmapSliceCanvasMode
  axisLabel: string
  scoreLabel: string
  selectedSliceKey: string
  selectedSliceLabel: string
  baselineSliceKey: string | null
  baselineSliceLabel: string | null
  selectedValues: Array<number | null>
  baselineValues: Array<number | null> | null
  selectedSampleCounts: Array<number | null>
  selectedCells: Array<HeatmapSliceCell | null>
}

export interface HeatmapCanvasRenderModel {
  signature: string
  meta: string
  statusLine: string
  model: HeatmapModel
  displayMode: HeatmapMode
  displayValues: Array<number | null>
  displayLabel: string
  displayLegendNote: string
  request: HeatmapCanvasRequestModel
  markers: HeatmapMarker[]
  range: HeatmapRange
  palette: HeatmapPalette
  plateauDiagnostics: Array<HeatmapPlateauCellDiagnostics | null> | null
  slice3d: HeatmapSlice3dModel | null
  sliceSelection?: HeatmapCanvasSliceSelection | null
  exactAggregateWinnerCells?: Array<HeatmapAggregateWinnerCell | null> | null
  aggregateSelectionRequiresExactParams?: boolean
  aggregateSelectionUnavailableReason?: string
}

export type HeatmapCellParams = Record<string, unknown>

export type HeatmapCellSelectionKind =
  | 'exact-slice'
  | 'exact-aggregate'
  | 'representative'
  | 'unavailable'

export interface HeatmapCellSelectionResolution {
  kind: HeatmapCellSelectionKind
  params: HeatmapCellParams | null
  reason: string
}

export const HEATMAP_MIN_VIEWPORT_SCALE = 1
export const HEATMAP_MAX_VIEWPORT_SCALE = 12
export const HEATMAP_MAX_CANVAS_PIXEL_RATIO = 3
export const HEATMAP_MAX_CANVAS_PIXELS = 2048 * 2048
export const HEATMAP_MAX_CANVAS_DIMENSION = 4096

export const DEFAULT_HEATMAP_VIEWPORT: HeatmapCanvasViewport = {
  scale: 1,
  offsetX: 0,
  offsetY: 0,
}

export interface HeatmapCanvasBackingStore {
  width: number
  height: number
  pixelRatio: number
}

export function resolveHeatmapCanvasBackingStore(options: {
  cssWidth: number
  cssHeight: number
  devicePixelRatio: number
  maxPixels?: number
  maxDimension?: number
}): HeatmapCanvasBackingStore {
  const cssWidthRaw = Number(options.cssWidth)
  const cssHeightRaw = Number(options.cssHeight)
  const cssWidth = Number.isFinite(cssWidthRaw) && cssWidthRaw > 0
    ? Math.max(1, Math.round(cssWidthRaw))
    : 1
  const cssHeight = Number.isFinite(cssHeightRaw) && cssHeightRaw > 0
    ? Math.max(1, Math.round(cssHeightRaw))
    : 1
  const requestedPixelRatioRaw = Number(options.devicePixelRatio)
  const requestedPixelRatio =
    Number.isFinite(requestedPixelRatioRaw) && requestedPixelRatioRaw > 0
      ? Math.min(HEATMAP_MAX_CANVAS_PIXEL_RATIO, requestedPixelRatioRaw)
      : 1
  const maxPixelsRaw = Number(options.maxPixels ?? HEATMAP_MAX_CANVAS_PIXELS)
  const maxPixels = Number.isFinite(maxPixelsRaw) && maxPixelsRaw > 0
    ? Math.max(1, Math.floor(maxPixelsRaw))
    : HEATMAP_MAX_CANVAS_PIXELS
  const maxDimensionRaw = Number(options.maxDimension ?? HEATMAP_MAX_CANVAS_DIMENSION)
  const maxDimension = Number.isFinite(maxDimensionRaw) && maxDimensionRaw > 0
    ? Math.max(1, Math.floor(maxDimensionRaw))
    : HEATMAP_MAX_CANVAS_DIMENSION
  const pixelBudgetRatio = Math.sqrt(maxPixels / (cssWidth * cssHeight))
  const dimensionBudgetRatio = Math.min(
    maxDimension / cssWidth,
    maxDimension / cssHeight,
  )
  const rawBoundedPixelRatio = Math.min(
    requestedPixelRatio,
    pixelBudgetRatio,
    dimensionBudgetRatio,
  )
  const boundedPixelRatio = rawBoundedPixelRatio > 0
    ? rawBoundedPixelRatio
    : 1 / Math.max(cssWidth, cssHeight)
  // Quantizing down keeps repeated resizes stable and guarantees that rounding
  // cannot push the backing store above its pixel budget.
  const pixelRatio = boundedPixelRatio >= 0.01
    ? Math.max(0.01, Math.floor(boundedPixelRatio * 100) / 100)
    : boundedPixelRatio
  return {
    width: Math.max(1, Math.floor(cssWidth * pixelRatio)),
    height: Math.max(1, Math.floor(cssHeight * pixelRatio)),
    pixelRatio,
  }
}

export interface HeatmapCanvasSummaryItem {
  key: string
  label: string
  value: string
  detail?: string
  tone?: 'neutral' | 'success' | 'warning' | 'danger' | 'accent'
  title?: string
}

export interface HeatmapFiniteBounds {
  minCol: number
  maxCol: number
  minDrawRow: number
  maxDrawRow: number
  finiteCells: number
  totalCells: number
}

export interface HeatmapLegendTargets {
  canvas: HTMLCanvasElement | null
  minLabel: HTMLElement | null
  midLabel: HTMLElement | null
  maxLabel: HTMLElement | null
}

function canvasContext2d(canvas: HTMLCanvasElement | null): CanvasRenderingContext2D | null {
  try {
    return canvas?.getContext('2d') ?? null
  } catch {
    return null
  }
}

function canvasPixelRatio(canvas: HTMLCanvasElement): number {
  const ratio = Number(canvas.dataset.pixelRatio || 1)
  return Number.isFinite(ratio) && ratio > 0 ? ratio : 1
}

function logicalCanvasSize(canvas: HTMLCanvasElement): { width: number; height: number; ratio: number } {
  const ratio = canvasPixelRatio(canvas)
  return {
    width: canvas.width / ratio,
    height: canvas.height / ratio,
    ratio,
  }
}

function clampViewportScale(scale: unknown): number {
  const next = Number(scale)
  if (!Number.isFinite(next)) return HEATMAP_MIN_VIEWPORT_SCALE
  return Math.max(HEATMAP_MIN_VIEWPORT_SCALE, Math.min(HEATMAP_MAX_VIEWPORT_SCALE, next))
}

export function clampHeatmapViewport(
  viewport: HeatmapCanvasViewport | null | undefined,
  width: number,
  height: number,
): HeatmapCanvasViewport {
  const scale = clampViewportScale(viewport?.scale)
  if (scale <= HEATMAP_MIN_VIEWPORT_SCALE) return { ...DEFAULT_HEATMAP_VIEWPORT }
  const logicalWidth = Math.max(1, Number.isFinite(width) ? width : 1)
  const logicalHeight = Math.max(1, Number.isFinite(height) ? height : 1)
  const minOffsetX = logicalWidth - logicalWidth * scale
  const minOffsetY = logicalHeight - logicalHeight * scale
  const offsetXRaw = Number(viewport?.offsetX)
  const offsetYRaw = Number(viewport?.offsetY)
  return {
    scale,
    offsetX: Math.min(0, Math.max(minOffsetX, Number.isFinite(offsetXRaw) ? offsetXRaw : 0)),
    offsetY: Math.min(0, Math.max(minOffsetY, Number.isFinite(offsetYRaw) ? offsetYRaw : 0)),
  }
}

export function heatmapViewportIsZoomed(
  viewport: HeatmapCanvasViewport | null | undefined,
): boolean {
  return clampViewportScale(viewport?.scale) > HEATMAP_MIN_VIEWPORT_SCALE + 1.0e-6
}

function normalizedHeatmapViewport(
  viewport: HeatmapCanvasViewport | null | undefined,
): HeatmapCanvasViewport {
  return {
    scale: clampViewportScale(viewport?.scale),
    offsetX: Number.isFinite(Number(viewport?.offsetX)) ? Number(viewport?.offsetX) : 0,
    offsetY: Number.isFinite(Number(viewport?.offsetY)) ? Number(viewport?.offsetY) : 0,
  }
}

function applyHeatmapViewport(
  ctx: CanvasRenderingContext2D,
  viewport: HeatmapCanvasViewport | null | undefined,
): HeatmapCanvasViewport {
  const view = normalizedHeatmapViewport(viewport)
  ctx.translate(view.offsetX, view.offsetY)
  ctx.scale(view.scale, view.scale)
  return view
}

function screenXForHeatmapCol(
  col: number,
  width: number,
  binsFast: number,
  view: HeatmapCanvasViewport,
): number {
  const cellW = width / Math.max(1, binsFast)
  return (col + 0.5) * cellW * view.scale + view.offsetX
}

function screenYForHeatmapDrawRow(
  drawRow: number,
  height: number,
  binsSlow: number,
  view: HeatmapCanvasViewport,
): number {
  const cellH = height / Math.max(1, binsSlow)
  return (drawRow + 0.5) * cellH * view.scale + view.offsetY
}

function visibleIndexSamples(maxIndex: number, count = 5): number[] {
  if (maxIndex < 0) return []
  if (maxIndex === 0) return [0]
  const samples = new Set<number>()
  const slots = Math.max(2, Math.floor(count))
  for (let i = 0; i < slots; i += 1) {
    samples.add(Math.round((i / (slots - 1)) * maxIndex))
  }
  return Array.from(samples).sort((a, b) => a - b)
}

export function visibleHeatmapIndexSamples(options: {
  bins: number
  logicalSize: number
  scale: number
  offset: number
  count?: number
}): number[] {
  const bins = Math.floor(Number(options.bins))
  const logicalSize = Number(options.logicalSize)
  const scale = Number(options.scale)
  const offsetRaw = Number(options.offset)
  if (
    !Number.isFinite(bins) ||
    bins <= 0 ||
    !Number.isFinite(logicalSize) ||
    logicalSize <= 0 ||
    !Number.isFinite(scale) ||
    scale <= 0
  ) return []

  const offset = Number.isFinite(offsetRaw) ? offsetRaw : 0
  const screenCellSize = (logicalSize / bins) * scale
  const firstVisibleRaw = Math.floor(-offset / screenCellSize)
  const lastVisibleRaw = Math.ceil((logicalSize - offset) / screenCellSize) - 1
  if (lastVisibleRaw < 0 || firstVisibleRaw >= bins || lastVisibleRaw < firstVisibleRaw) return []

  const firstVisible = Math.max(0, firstVisibleRaw)
  const lastVisible = Math.min(bins - 1, lastVisibleRaw)
  return visibleIndexSamples(lastVisible - firstVisible, options.count)
    .map((sample) => firstVisible + sample)
}

function textChipWidth(textValue: string, maxWidth = 92): number {
  return Math.max(28, Math.min(maxWidth, textValue.length * 6.4 + 10))
}

function paintTextChip(
  ctx: CanvasRenderingContext2D,
  textValue: string,
  xRaw: number,
  yRaw: number,
  options: {
    align?: 'left' | 'center' | 'right'
    fill?: string
    stroke?: string
    text?: string
    maxWidth?: number
  } = {},
): void {
  const width = textChipWidth(textValue, options.maxWidth)
  const height = 16
  const align = options.align ?? 'center'
  const x =
    align === 'left'
      ? xRaw
      : align === 'right'
        ? xRaw - width
        : xRaw - width / 2
  const y = yRaw - height / 2
  ctx.fillStyle = options.fill ?? 'rgba(2, 6, 23, 0.72)'
  ctx.fillRect(x, y, width, height)
  ctx.strokeStyle = options.stroke ?? 'rgba(226, 232, 240, 0.28)'
  ctx.lineWidth = 1
  ctx.strokeRect(x + 0.5, y + 0.5, Math.max(0, width - 1), Math.max(0, height - 1))
  ctx.fillStyle = options.text ?? 'rgba(248, 250, 252, 0.95)'
  ctx.font = '11px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace'
  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  ctx.fillText(textValue, x + width / 2, y + height / 2 + 0.5, Math.max(1, width - 8))
}

function heatmapAxisLabel(
  request: HeatmapCanvasRequestModel,
  axis: 'fast' | 'slow',
): string {
  const configured = cleanText(
    axis === 'fast' ? request.fastAxisLabel : request.slowAxisLabel,
    '',
  )
  if (configured) return configured
  const maType = cleanText(
    axis === 'fast' ? request.fastMaType : request.slowMaType,
    '',
  ).toUpperCase()
  const prefix = axis === 'fast' ? 'Fast' : 'Slow'
  return maType && maType !== '?' ? `${prefix} ${maType} length` : `${prefix} length`
}

function paintHeatmapAxisGuides(
  ctx: CanvasRenderingContext2D,
  renderModel: HeatmapCanvasRenderModel,
  width: number,
  height: number,
  view: HeatmapCanvasViewport,
  theme?: LightweightChartTheme,
): void {
  const { model, request } = renderModel
  if (width < 280 || height < 280 || model.binsFast <= 0 || model.binsSlow <= 0) return
  const tokens = heatmapVisualizationTokens(theme)

  const fastLabel = heatmapAxisLabel(request, 'fast')
  const slowLabel = heatmapAxisLabel(request, 'slow')
  const fastSamples = visibleHeatmapIndexSamples({
    bins: model.binsFast,
    logicalSize: width,
    scale: view.scale,
    offset: view.offsetX,
    count: 5,
  })
  const slowDrawSamples = visibleHeatmapIndexSamples({
    bins: model.binsSlow,
    logicalSize: height,
    scale: view.scale,
    offset: view.offsetY,
    count: 5,
  })

  ctx.save()
  for (const col of fastSamples) {
    const x = Math.max(
      32,
      Math.min(width - 28, screenXForHeatmapCol(col, width, model.binsFast, view)),
    )
    const fast = fastForHeatmapCol(model, col)
    paintTextChip(ctx, String(fast), x, height - 12, {
      fill: tokens.chipFill,
      stroke: tokens.chipBorder,
      text: tokens.chipText,
    })
  }
  for (const drawRow of slowDrawSamples) {
    const y = Math.max(
      42,
      Math.min(height - 34, screenYForHeatmapDrawRow(drawRow, height, model.binsSlow, view)),
    )
    const logicalRow = model.binsSlow - 1 - drawRow
    const slow = slowForHeatmapLogicalRow(model, logicalRow)
    paintTextChip(ctx, String(slow), 8, y, {
      align: 'left',
      fill: tokens.chipFill,
      stroke: tokens.chipBorder,
      text: tokens.chipText,
    })
  }
  const axisTitleWidth = Math.max(92, Math.min(220, width - 20))
  paintTextChip(ctx, fastLabel, width - 10, height - 34, {
    align: 'right',
    fill: tokens.chipFill,
    stroke: tokens.axisFast,
    text: tokens.chipText,
    maxWidth: axisTitleWidth,
  })
  paintTextChip(ctx, slowLabel, 8, 18, {
    align: 'left',
    fill: tokens.chipFill,
    stroke: tokens.axisSlow,
    text: tokens.chipText,
    maxWidth: axisTitleWidth,
  })
  ctx.restore()
}

function prepareLogicalCanvasContext(
  canvas: HTMLCanvasElement,
): { ctx: CanvasRenderingContext2D; width: number; height: number } | null {
  const ctx = canvasContext2d(canvas)
  if (!ctx) return null
  const { width, height, ratio } = logicalCanvasSize(canvas)
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0)
  return { ctx, width, height }
}

function finiteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value)
}

function cleanText(value: unknown, fallback = ''): string {
  const text = String(value ?? '').trim()
  return text ? text : fallback
}

function trimFixed(src: string): string {
  return src.replace(/(\.\d*?[1-9])0+$/u, '$1').replace(/\.0+$/u, '')
}

export function formatHeatmapDisplay(value: unknown, digits = 4): string {
  const next = Number(value)
  if (!Number.isFinite(next)) return '-'
  const abs = Math.abs(next)
  if (abs === 0) return '0'
  const precision = Math.max(0, Math.floor(digits))
  const zeroThreshold = 1 / 10 ** Math.max(1, precision)
  if (abs < zeroThreshold) {
    return next
      .toExponential(Math.max(0, precision - 1))
      .replace(/\.?0+e/u, 'e')
      .replace(/e\+/u, 'e')
  }
  if (abs >= 1_000_000) {
    return next.toLocaleString(undefined, {
      maximumFractionDigits: Math.min(2, precision),
      minimumFractionDigits: 0,
    })
  }
  return trimFixed(next.toFixed(precision))
}

function heatmapIntegerBinBounds(options: {
  min: number
  max: number
  bins: number
  index: number
}): { min: number; max: number } {
  const min = Math.floor(options.min)
  const max = Math.max(min, Math.floor(options.max))
  const bins = Math.max(1, Math.floor(options.bins))
  const index = Math.max(0, Math.min(bins - 1, Math.floor(options.index)))
  if (bins <= 1 || max <= min) return { min, max }
  const denominator = bins - 1
  const span = max - min
  const lower = min + Math.ceil((index * span) / denominator)
  const upper =
    index >= bins - 1
      ? max
      : min + Math.ceil(((index + 1) * span) / denominator) - 1
  return {
    min: Math.max(min, Math.min(max, lower)),
    max: Math.max(min, Math.min(max, upper)),
  }
}

function heatmapBinLabel(bounds: { min: number; max: number }): string {
  return bounds.min === bounds.max ? String(bounds.min) : `${bounds.min}–${bounds.max}`
}

function heatmapCellBinLabels(
  model: HeatmapModel,
  cell: Pick<HeatmapResolvedCell, 'col' | 'logicalRow'>,
): { fast: string; slow: string } {
  return {
    fast: heatmapBinLabel(
      heatmapIntegerBinBounds({
        min: model.fastMin,
        max: model.fastMax,
        bins: model.binsFast,
        index: cell.col,
      }),
    ),
    slow: heatmapBinLabel(
      heatmapIntegerBinBounds({
        min: model.slowMin,
        max: model.slowMax,
        bins: model.binsSlow,
        index: cell.logicalRow,
      }),
    ),
  }
}

export function paintHeatmapLegend(
  targets: HeatmapLegendTargets,
  range: HeatmapRange,
  palette: HeatmapPalette,
  theme?: LightweightChartTheme,
): void {
  const resolvedPalette = resolveHeatmapPaletteTheme(palette, theme)
  const anchors = heatmapLegendRangeAnchors(range, resolvedPalette)
  if (targets.minLabel) targets.minLabel.textContent = formatHeatmapDisplay(anchors.min, 4)
  if (targets.midLabel) {
    targets.midLabel.textContent = formatHeatmapDisplay(anchors.middle, 4)
    targets.midLabel.style.left = `${(anchors.middleOffset * 100).toFixed(4).replace(/0+$/u, '').replace(/\.$/u, '')}%`
    targets.midLabel.dataset.legendAnchor =
      resolvedPalette.kind === 'diverging' && anchors.middle === 0
        ? 'zero'
        : 'midpoint'
  }
  if (targets.maxLabel) targets.maxLabel.textContent = formatHeatmapDisplay(anchors.max, 4)

  const legend = targets.canvas
  if (!legend) return
  const ctx = canvasContext2d(legend)
  if (!ctx) return
  const width = legend.width
  const height = legend.height
  const gradient = ctx.createLinearGradient(0, 0, width, 0)
  for (const stop of heatmapLegendStops(palette, theme, 24, range)) {
    gradient.addColorStop(stop.offset, stop.color)
  }
  ctx.clearRect(0, 0, width, height)
  ctx.fillStyle = gradient
  ctx.fillRect(0, 0, width, height)
  const tokens = heatmapVisualizationTokens(theme)
  ctx.strokeStyle = tokens.chipBorder
  ctx.lineWidth = 1
  ctx.strokeRect(0.5, 0.5, Math.max(0, width - 1), Math.max(0, height - 1))
  if (resolvedPalette.kind === 'diverging') {
    const zeroOffset = anchors.zeroOffset
    if (zeroOffset != null) {
      const zeroX = Math.max(0.5, Math.min(width - 0.5, zeroOffset * width))
      ctx.strokeStyle = tokens.selectionOuter
      ctx.lineWidth = 3
      ctx.beginPath()
      ctx.moveTo(zeroX, 1)
      ctx.lineTo(zeroX, Math.max(1, height - 1))
      ctx.stroke()
      ctx.strokeStyle = tokens.selectionInner
      ctx.lineWidth = 1
      ctx.stroke()
    }
  }
}

export function clearHeatmapCanvas(canvas: HTMLCanvasElement | null): void {
  const ctx = canvasContext2d(canvas)
  if (!ctx || !canvas) return
  ctx.setTransform(1, 0, 0, 1, 0, 0)
  ctx.clearRect(0, 0, canvas.width, canvas.height)
}

function bestMarkerForValues(
  model: HeatmapModel,
  values: Array<number | null>,
  label: string,
  preferMin = false,
): HeatmapMarker | null {
  let bestIdx = -1
  let bestValue = preferMin ? Number.POSITIVE_INFINITY : Number.NEGATIVE_INFINITY
  for (let idx = 0; idx < values.length; idx += 1) {
    const value = values[idx]
    if (!finiteNumber(value)) continue
    if ((preferMin && value < bestValue) || (!preferMin && value > bestValue)) {
      bestValue = value
      bestIdx = idx
    }
  }
  if (bestIdx < 0) return null
  const logicalRow = Math.floor(bestIdx / Math.max(1, model.binsFast))
  const col = bestIdx % Math.max(1, model.binsFast)
  return {
    col,
    drawRow: model.binsSlow - 1 - logicalRow,
    label,
  }
}

function sliceByKey(
  slices: Heatmap3dSlice[],
  key: string,
): Heatmap3dSlice | null {
  return slices.find((slice) => slice.key === key) ?? null
}

function firstDifferentSlice(
  slices: Heatmap3dSlice[],
  selectedKey: string,
): Heatmap3dSlice | null {
  return slices.find((slice) => slice.key !== selectedKey) ?? null
}

export function buildHeatmapSliceCanvasRenderModel(
  renderModel: HeatmapCanvasRenderModel,
  options: {
    mode: HeatmapSliceCanvasMode
    selectedSliceKey?: string
    baselineSliceKey?: string
  },
): HeatmapCanvasRenderModel | null {
  const sliceModel = renderModel.slice3d
  const slices = sliceModel?.slices ?? []
  if (!sliceModel || slices.length < 2) return null

  const selected = sliceByKey(slices, cleanText(options.selectedSliceKey)) ?? slices[0] ?? null
  if (!selected) return null
  const baseline =
    options.mode === 'delta'
      ? (sliceByKey(slices, cleanText(options.baselineSliceKey)) ?? firstDifferentSlice(slices, selected.key))
      : null
  if (options.mode === 'delta' && (!baseline || baseline.key === selected.key)) return null

  const displayValues =
    options.mode === 'delta' && baseline
      ? selected.values.map((value, idx) => (
          finiteNumber(value) && finiteNumber(baseline.values[idx])
            ? value - (baseline.values[idx] as number)
            : null
        ))
      : selected.values.slice()
  let finiteCount = 0
  for (const value of displayValues) {
    if (finiteNumber(value)) finiteCount += 1
  }
  if (!finiteCount) return null

  const modeLabel = options.mode === 'delta' ? 'Delta vs baseline' : 'Selected slice'
  const scoreLabel = cleanText(sliceModel.scoreLabel, renderModel.request.scoreLabel || 'Score')
  const markers = [
    bestMarkerForValues(renderModel.model, selected.values, 'Best'),
    options.mode === 'delta' ? bestMarkerForValues(renderModel.model, displayValues, '+Δ') : null,
    options.mode === 'delta' ? bestMarkerForValues(renderModel.model, displayValues, '-Δ', true) : null,
  ].filter((marker): marker is HeatmapMarker => Boolean(marker))
  const deltaRange = options.mode === 'delta' ? resolveNullableHeatmapRange(displayValues) : sliceModel.range

  return {
    ...renderModel,
    signature: `${renderModel.signature}:slice2d:${options.mode}:${selected.key}:${baseline?.key ?? ''}`,
    meta: options.mode === 'delta' && baseline
      ? `${scoreLabel} delta heatmap: ${selected.label} minus ${baseline.label}.`
      : `${scoreLabel} slice heatmap: ${selected.label}.`,
    displayMode: 'objective',
    displayValues,
    displayLabel: options.mode === 'delta' ? `${scoreLabel} Δ` : scoreLabel,
    displayLegendNote:
      options.mode === 'delta'
        ? 'Delta vs baseline · neutral 0 means no change'
        : modeLabel,
    markers,
    range: deltaRange,
    palette: options.mode === 'delta' ? heatmapPalette('objective') : sliceModel.palette,
    plateauDiagnostics: null,
    sliceSelection: {
      mode: options.mode,
      axisLabel: sliceModel.axisLabel,
      scoreLabel,
      selectedSliceKey: selected.key,
      selectedSliceLabel: selected.label,
      baselineSliceKey: baseline?.key ?? null,
      baselineSliceLabel: baseline?.label ?? null,
      selectedValues: selected.values,
      baselineValues: baseline?.values ?? null,
      selectedSampleCounts: selected.sampleCounts,
      selectedCells: selected.cells,
    },
  }
}

export function finiteHeatmapDisplayBounds(
  renderModel: HeatmapCanvasRenderModel,
): HeatmapFiniteBounds | null {
  const { model, displayValues } = renderModel
  const totalCells = Math.max(0, model.binsFast * model.binsSlow)
  if (!totalCells) return null
  let minCol = Number.POSITIVE_INFINITY
  let maxCol = Number.NEGATIVE_INFINITY
  let minDrawRow = Number.POSITIVE_INFINITY
  let maxDrawRow = Number.NEGATIVE_INFINITY
  let finiteCells = 0
  for (let logicalRow = 0; logicalRow < model.binsSlow; logicalRow += 1) {
    const drawRow = model.binsSlow - 1 - logicalRow
    for (let col = 0; col < model.binsFast; col += 1) {
      const value = displayValues[heatmapIndex(model, logicalRow, col)]
      if (!finiteNumber(value)) continue
      finiteCells += 1
      minCol = Math.min(minCol, col)
      maxCol = Math.max(maxCol, col)
      minDrawRow = Math.min(minDrawRow, drawRow)
      maxDrawRow = Math.max(maxDrawRow, drawRow)
    }
  }
  if (!finiteCells) return null
  return { minCol, maxCol, minDrawRow, maxDrawRow, finiteCells, totalCells }
}

export function fitHeatmapViewportToFiniteDomain(
  renderModel: HeatmapCanvasRenderModel,
  width: number,
  height: number,
  paddingCells = 0.75,
): HeatmapCanvasViewport {
  const bounds = finiteHeatmapDisplayBounds(renderModel)
  if (!bounds) return { ...DEFAULT_HEATMAP_VIEWPORT }
  const { model } = renderModel
  const logicalWidth = Math.max(1, Number.isFinite(width) ? width : 1)
  const logicalHeight = Math.max(1, Number.isFinite(height) ? height : 1)
  const cellW = logicalWidth / Math.max(1, model.binsFast)
  const cellH = logicalHeight / Math.max(1, model.binsSlow)
  const x0 = Math.max(0, (bounds.minCol - paddingCells) * cellW)
  const x1 = Math.min(logicalWidth, (bounds.maxCol + 1 + paddingCells) * cellW)
  const y0 = Math.max(0, (bounds.minDrawRow - paddingCells) * cellH)
  const y1 = Math.min(logicalHeight, (bounds.maxDrawRow + 1 + paddingCells) * cellH)
  const domainWidth = Math.max(cellW, x1 - x0)
  const domainHeight = Math.max(cellH, y1 - y0)
  const scale = Math.min(
    HEATMAP_MAX_VIEWPORT_SCALE,
    Math.max(HEATMAP_MIN_VIEWPORT_SCALE, Math.min(logicalWidth / domainWidth, logicalHeight / domainHeight) * 0.96),
  )
  if (scale <= HEATMAP_MIN_VIEWPORT_SCALE + 0.02) return { ...DEFAULT_HEATMAP_VIEWPORT }
  return clampHeatmapViewport({
    scale,
    offsetX: (logicalWidth - domainWidth * scale) / 2 - x0 * scale,
    offsetY: (logicalHeight - domainHeight * scale) / 2 - y0 * scale,
  }, logicalWidth, logicalHeight)
}

export function buildHeatmapCanvasSummaryItems(
  renderModel: HeatmapCanvasRenderModel,
): HeatmapCanvasSummaryItem[] {
  const bounds = finiteHeatmapDisplayBounds(renderModel)
  const { model, displayValues } = renderModel
  if (!bounds) {
    return [{ key: 'coverage', label: 'Coverage', value: '0%', detail: 'No populated cells', tone: 'warning' }]
  }
  const best = bestMarkerForValues(model, displayValues, 'best')
  const bestValue = best ? displayValues[heatmapIndex(model, model.binsSlow - 1 - best.drawRow, best.col)] : null
  const coverage = bounds.totalCells > 0 ? Math.round((bounds.finiteCells / bounds.totalCells) * 100) : 0

  const colSums = Array<number>(model.binsFast).fill(0)
  const colCounts = Array<number>(model.binsFast).fill(0)
  const rowSums = Array<number>(model.binsSlow).fill(0)
  const rowCounts = Array<number>(model.binsSlow).fill(0)
  let finiteMax = Number.NEGATIVE_INFINITY
  let finiteDisplayCount = 0
  for (let logicalRow = 0; logicalRow < model.binsSlow; logicalRow += 1) {
    for (let col = 0; col < model.binsFast; col += 1) {
      const value = displayValues[heatmapIndex(model, logicalRow, col)]
      if (!finiteNumber(value)) continue
      finiteDisplayCount += 1
      if (value > finiteMax) finiteMax = value
      colSums[col] = (colSums[col] ?? 0) + value
      colCounts[col] = (colCounts[col] ?? 0) + 1
      rowSums[logicalRow] = (rowSums[logicalRow] ?? 0) + value
      rowCounts[logicalRow] = (rowCounts[logicalRow] ?? 0) + 1
    }
  }
  const colMeans = colSums.map((sum, col) =>
    colCounts[col] ? sum / colCounts[col] : null,
  )
  const rowMeans = rowSums.map((sum, logicalRow) =>
    rowCounts[logicalRow] ? sum / rowCounts[logicalRow] : null,
  )
  let bestColIdx = -1
  let bestColValue = Number.NEGATIVE_INFINITY
  for (let col = 0; col < colMeans.length; col += 1) {
    const value = colMeans[col]
    if (!finiteNumber(value) || value <= bestColValue) continue
    bestColValue = value
    bestColIdx = col
  }
  const bestRowIdx = rowMeans.reduce<number>((bestIdx, value, idx) => {
    if (!finiteNumber(value)) return bestIdx
    const current = bestIdx >= 0 ? rowMeans[bestIdx] : null
    return !finiteNumber(current) || value > current ? idx : bestIdx
  }, -1)

  const bestFast =
    bestColIdx >= 0 ? fastForHeatmapCol(model, bestColIdx) : null
  const bestSlow =
    bestRowIdx >= 0 ? slowForHeatmapLogicalRow(model, bestRowIdx) : null
  const slice = renderModel.sliceSelection
  const deltaTone =
    slice?.mode === 'delta' && finiteDisplayCount
      ? (finiteMax > 0 ? 'success' : 'danger')
      : 'accent'

  const items: HeatmapCanvasSummaryItem[] = []
  if (slice) {
    items.push({
      key: 'slice',
      label: slice.mode === 'delta' ? 'Delta' : 'Slice',
      value: slice.mode === 'delta' ? 'Compare' : 'Selected',
      detail: slice.mode === 'delta' && slice.baselineSliceLabel
        ? `${slice.selectedSliceLabel} vs ${slice.baselineSliceLabel}`
        : slice.selectedSliceLabel,
      tone: slice.mode === 'delta' ? deltaTone : 'accent',
    })
  }
  items.push(
    {
      key: 'coverage',
      label: 'Coverage',
      value: `${coverage}%`,
      detail: `${bounds.finiteCells}/${bounds.totalCells} populated bins`,
      tone: coverage < 50 ? 'warning' : 'neutral',
    },
    {
      key: 'best',
      label:
        slice?.mode === 'delta'
          ? 'Best Δ bin'
          : slice
            ? 'Best slice bin'
            : 'Best bin',
      value: formatHeatmapDisplay(bestValue, 4),
      detail: best
        ? (() => {
            const labels = heatmapCellBinLabels(model, {
              col: best.col,
              logicalRow: model.binsSlow - 1 - best.drawRow,
            })
            const semantics = slice ? 'retained-row maximum' : 'aggregate maximum'
            return `${heatmapAxisLabel(renderModel.request, 'fast')} ${labels.fast} · ${heatmapAxisLabel(renderModel.request, 'slow')} ${labels.slow} · ${semantics}`
          })()
        : '—',
      tone: slice?.mode === 'delta' && finiteNumber(bestValue) && bestValue > 0 ? 'success' : 'neutral',
    },
    {
      key: 'fast-band',
      label: 'Best fast band',
      value: bestFast != null ? String(bestFast) : '—',
      detail: bestColIdx >= 0 ? `mean ${formatHeatmapDisplay(colMeans[bestColIdx], 4)}` : 'No populated fast band',
    },
    {
      key: 'slow-band',
      label: 'Best slow band',
      value: bestSlow != null ? String(bestSlow) : '—',
      detail: bestRowIdx >= 0 ? `mean ${formatHeatmapDisplay(rowMeans[bestRowIdx], 4)}` : 'No populated slow band',
    },
  )
  return items
}

export function paintHeatmapBase(
  renderModel: HeatmapCanvasRenderModel,
  canvas: HTMLCanvasElement,
  viewport?: HeatmapCanvasViewport | null,
  theme?: LightweightChartTheme,
): void {
  const { model, displayValues, range, palette } = renderModel
  const tokens = heatmapVisualizationTokens(theme)
  const prepared = prepareLogicalCanvasContext(canvas)
  if (!prepared) return
  const { ctx, width, height } = prepared
  const cellW = width / Math.max(1, model.binsFast)
  const cellH = height / Math.max(1, model.binsSlow)

  ctx.clearRect(0, 0, width, height)
  ctx.save()
  applyHeatmapViewport(ctx, viewport)
  for (let logicalRow = 0; logicalRow < model.binsSlow; logicalRow += 1) {
    const drawRow = model.binsSlow - 1 - logicalRow
    for (let col = 0; col < model.binsFast; col += 1) {
      const value = displayValues[heatmapIndex(model, logicalRow, col)]
      const x = col * cellW
      const y = drawRow * cellH
      if (typeof value !== 'number' || !Number.isFinite(value)) {
        ctx.fillStyle = tokens.unavailableFill
        ctx.fillRect(x, y, cellW, cellH)
        continue
      }
      ctx.fillStyle = sampleHeatmapColor(value, range.min, range.max, palette, theme)
      ctx.fillRect(x, y, cellW, cellH)
    }
  }
  ctx.restore()
}

export function paintHeatmapOverlay(
  renderModel: HeatmapCanvasRenderModel,
  overlay: HTMLCanvasElement,
  hoverCell: HeatmapHoverCell | null,
  viewport?: HeatmapCanvasViewport | null,
  theme?: LightweightChartTheme,
): void {
  const { model, markers } = renderModel
  const tokens = heatmapVisualizationTokens(theme)
  const prepared = prepareLogicalCanvasContext(overlay)
  if (!prepared) return
  const { ctx: overlayCtx, width, height } = prepared
  const cellW = width / Math.max(1, model.binsFast)
  const cellH = height / Math.max(1, model.binsSlow)

  overlayCtx.clearRect(0, 0, width, height)
  overlayCtx.save()
  const view = applyHeatmapViewport(overlayCtx, viewport)
  for (const marker of markers) {
    const cx = (marker.col + 0.5) * cellW
    const cy = (marker.drawRow + 0.5) * cellH
    const radius = Math.max(4, Math.min(cellW, cellH) * 0.2)
    overlayCtx.fillStyle = tokens.markerFill
    overlayCtx.beginPath()
    overlayCtx.arc(cx, cy, radius, 0, Math.PI * 2)
    overlayCtx.fill()
    overlayCtx.strokeStyle = tokens.markerStroke
    overlayCtx.lineWidth = Math.max(1, 1.4 / view.scale)
    overlayCtx.stroke()
    overlayCtx.strokeStyle = tokens.markerHalo
    overlayCtx.lineWidth = Math.max(1, 2.2 / view.scale)
    overlayCtx.strokeRect(cx - cellW * 0.5 + 1 / view.scale, cy - cellH * 0.5 + 1 / view.scale, Math.max(0, cellW - 2 / view.scale), Math.max(0, cellH - 2 / view.scale))
  }

  if (hoverCell) {
    const hoverX = hoverCell.col * cellW
    const hoverY = hoverCell.row * cellH
    overlayCtx.strokeStyle = tokens.selectionOuter
    overlayCtx.lineWidth = 3.5 / view.scale
    overlayCtx.strokeRect(hoverX + 0.5, hoverY + 0.5, Math.max(0, cellW - 1), Math.max(0, cellH - 1))
    overlayCtx.strokeStyle = tokens.selectionInner
    overlayCtx.lineWidth = 1.4 / view.scale
    overlayCtx.strokeRect(hoverX + 0.5, hoverY + 0.5, Math.max(0, cellW - 1), Math.max(0, cellH - 1))
    overlayCtx.beginPath()
    overlayCtx.moveTo(hoverX + cellW / 2, 0)
    overlayCtx.lineTo(hoverX + cellW / 2, height)
    overlayCtx.moveTo(0, hoverY + cellH / 2)
    overlayCtx.lineTo(width, hoverY + cellH / 2)
    overlayCtx.strokeStyle = tokens.selectionGuide
    overlayCtx.stroke()
  }
  overlayCtx.restore()
  paintHeatmapAxisGuides(
    overlayCtx,
    renderModel,
    width,
    height,
    normalizedHeatmapViewport(viewport),
    theme,
  )
  for (const marker of markers) {
    const screenX = screenXForHeatmapCol(marker.col, width, model.binsFast, view)
    const screenY = screenYForHeatmapDrawRow(marker.drawRow, height, model.binsSlow, view)
    if (screenX < -16 || screenX > width + 16 || screenY < -16 || screenY > height + 16) continue
    const labelWidth = textChipWidth(marker.label)
    const canPlaceRight = screenX + 8 + labelWidth <= width - 4
    const labelX = canPlaceRight ? Math.max(4, screenX + 8) : Math.min(width - 4, screenX - 8)
    const labelY = Math.max(14, Math.min(height - 16, screenY < 44 ? screenY + 24 : screenY - 14))
    paintTextChip(overlayCtx, marker.label, labelX, labelY, {
      align: canPlaceRight ? 'left' : 'right',
      fill: tokens.chipFill,
      stroke: tokens.chipBorder,
      text: tokens.chipText,
    })
  }
}

export function resolveHeatmapCellFromPoint(
  overlay: HTMLCanvasElement,
  model: HeatmapModel,
  clientX: number,
  clientY: number,
  viewport?: HeatmapCanvasViewport | null,
): HeatmapResolvedCell | null {
  const rect = overlay.getBoundingClientRect()
  if (!rect.width || !rect.height) return null
  const { width, height } = logicalCanvasSize(overlay)
  const view = normalizedHeatmapViewport(viewport)
  const logicalX = (((clientX - rect.left) / rect.width) * width - view.offsetX) / view.scale
  const logicalY = (((clientY - rect.top) / rect.height) * height - view.offsetY) / view.scale
  if (logicalX < 0 || logicalY < 0 || logicalX > width || logicalY > height) return null
  const col = Math.max(
    0,
    Math.min(model.binsFast - 1, Math.floor((logicalX / width) * model.binsFast)),
  )
  const drawRow = Math.max(
    0,
    Math.min(model.binsSlow - 1, Math.floor((logicalY / height) * model.binsSlow)),
  )
  return resolveHeatmapCellAt(model, col, drawRow)
}

export function resolveHeatmapCellAt(
  model: HeatmapModel,
  colRaw: unknown,
  drawRowRaw: unknown,
): HeatmapResolvedCell | null {
  const col = Math.floor(Number(colRaw))
  const drawRow = Math.floor(Number(drawRowRaw))
  if (
    !Number.isFinite(col) ||
    !Number.isFinite(drawRow) ||
    col < 0 ||
    drawRow < 0 ||
    col >= model.binsFast ||
    drawRow >= model.binsSlow
  ) {
    return null
  }
  const logicalRow = model.binsSlow - 1 - drawRow
  return {
    col,
    drawRow,
    logicalRow,
    idx: heatmapIndex(model, logicalRow, col),
  }
}

function populatedCellValue(
  renderModel: HeatmapCanvasRenderModel,
  cell: HeatmapResolvedCell,
): number | null {
  const value = renderModel.sliceSelection
    ? renderModel.displayValues[cell.idx]
    : renderModel.model.values[cell.idx]
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

export function resolveHeatmapCellSelection(
  renderModel: HeatmapCanvasRenderModel,
  cell: HeatmapResolvedCell,
): HeatmapCellSelectionResolution {
  const value = populatedCellValue(renderModel, cell)
  if (value == null) {
    return {
      kind: 'unavailable',
      params: null,
      reason: 'Candidate detail is unavailable because this heatmap cell has no finite score.',
    }
  }

  const exactParams = renderModel.sliceSelection?.selectedCells[cell.idx]?.params
  if (exactParams && Object.keys(exactParams).length > 0) {
    return {
      kind: 'exact-slice',
      params: exactParams,
      reason: 'Candidate detail uses the exact retained winner for the selected slice.',
    }
  }

  if (renderModel.sliceSelection) {
    return {
      kind: 'unavailable',
      params: null,
      reason:
        'Candidate detail is unavailable because the selected slice has no retained candidate for this cell.',
    }
  }

  const exactAggregateWinner = renderModel.exactAggregateWinnerCells?.[cell.idx] ?? null
  const objectiveValue = renderModel.model.values[cell.idx]
  if (
    exactAggregateWinner &&
    Object.keys(exactAggregateWinner.params).length > 0 &&
    typeof objectiveValue === 'number' &&
    Number.isFinite(objectiveValue) &&
    exactAggregateWinner.score === objectiveValue
  ) {
    return {
      kind: 'exact-aggregate',
      params: exactAggregateWinner.params,
      reason: 'Candidate detail uses the exact retained best-in-bin winner.',
    }
  }

  if (renderModel.aggregateSelectionRequiresExactParams) {
    return {
      kind: 'unavailable',
      params: null,
      reason:
        renderModel.aggregateSelectionUnavailableReason ||
        'Candidate detail is unavailable because the exact winning MA variant was not retained for this aggregate cell. Use Top Results, or rerun with Return all results.',
    }
  }

  const fast = fastForHeatmapCol(renderModel.model, cell.col)
  const slowBase = slowForHeatmapLogicalRow(renderModel.model, cell.logicalRow)
  const slow = Math.max(slowBase, fast + 1)
  return {
    kind: 'representative',
    params: {
      fast_len: fast,
      fast_ma_type: renderModel.request.fastMaType || '?',
      slow_len: slow,
      slow_ma_type: renderModel.request.slowMaType || '?',
      fast_ma_params: renderModel.request.fastMaParams ?? null,
      slow_ma_params: renderModel.request.slowMaParams ?? null,
    },
    reason:
      'Candidate detail uses a representative parameter pair from the grid and re-evaluates it; this is not a retained aggregate winner.',
  }
}

export function buildHeatmapCellParams(
  renderModel: HeatmapCanvasRenderModel,
  cell: HeatmapResolvedCell,
): HeatmapCellParams | null {
  return resolveHeatmapCellSelection(renderModel, cell).params
}

function heatmapSelectionDetailText(
  renderModel: HeatmapCanvasRenderModel,
  cell: HeatmapResolvedCell,
): string {
  return `Candidate detail: ${resolveHeatmapCellSelection(renderModel, cell).reason}`
}

export function heatmapSelectionLegendText(
  renderModel: HeatmapCanvasRenderModel,
): string {
  if (renderModel.sliceSelection) {
    return 'Populated cells open the exact retained winner from the selected slice.'
  }

  let finiteCells = 0
  let exactCells = 0
  for (let index = 0; index < renderModel.model.values.length; index += 1) {
    const value = renderModel.model.values[index]
    if (typeof value !== 'number' || !Number.isFinite(value)) continue
    finiteCells += 1
    const winner = renderModel.exactAggregateWinnerCells?.[index] ?? null
    if (
      winner &&
      Object.keys(winner.params).length > 0 &&
      winner.score === value
    ) {
      exactCells += 1
    }
  }

  if (finiteCells > 0 && exactCells === finiteCells) {
    return 'Populated cells open the exact retained best-in-bin winner.'
  }
  if (renderModel.aggregateSelectionRequiresExactParams) {
    const unavailable = Math.max(0, finiteCells - exactCells)
    const coverage =
      exactCells > 0
        ? `${exactCells} retained winner${exactCells === 1 ? '' : 's'} can open; ${unavailable} unresolved cell${unavailable === 1 ? '' : 's'} cannot. `
        : ''
    return `${coverage}${renderModel.aggregateSelectionUnavailableReason || 'Exact winning MA variants were not retained for these aggregate cells. Use Top Results, or rerun with Return all results.'}`
  }
  if (exactCells > 0) {
    return `${exactCells} populated cell${exactCells === 1 ? '' : 's'} open an exact retained winner; remaining cells use a representative parameter pair from the grid and re-evaluate it.`
  }
  return 'Populated cells use a representative parameter pair from the grid and re-evaluate it; Top Results keeps exact ranked candidates.'
}

export function heatmapTooltipText(
  renderModel: HeatmapCanvasRenderModel,
  cell: HeatmapResolvedCell,
): string | null {
  const slice = renderModel.sliceSelection ?? null
  const objectiveValue = renderModel.model.values[cell.idx]
  if (!slice && (typeof objectiveValue !== 'number' || !Number.isFinite(objectiveValue))) return null
  const displayValue = renderModel.displayValues[cell.idx]
  if (slice && !finiteNumber(displayValue)) return null
  const binLabels = heatmapCellBinLabels(renderModel.model, cell)
  const fastLabel = heatmapAxisLabel(renderModel.request, 'fast')
  const slowLabel = heatmapAxisLabel(renderModel.request, 'slow')
  const objectiveDisplay = formatHeatmapDisplay(objectiveValue, 4)

  if (slice) {
    const selectedValue = slice.selectedValues[cell.idx]
    const baselineValue = slice.baselineValues?.[cell.idx] ?? null
    const sampleCount = slice.selectedSampleCounts[cell.idx]
    const sampleDisplay = finiteNumber(sampleCount)
      ? ` · samples ${Math.max(0, Math.floor(sampleCount))}`
      : ''
    if (slice.mode === 'delta') {
      return `${slice.selectedSliceLabel} vs ${slice.baselineSliceLabel ?? 'baseline'}\n${fastLabel} bin ${binLabels.fast} · ${slowLabel} bin ${binLabels.slow} · best-in-bin Δ ${formatHeatmapDisplay(displayValue, 4)} · selected ${formatHeatmapDisplay(selectedValue, 4)} · baseline ${formatHeatmapDisplay(baselineValue, 4)}${sampleDisplay}\n${heatmapSelectionDetailText(renderModel, cell)}`
    }
    return `${slice.selectedSliceLabel}\n${fastLabel} bin ${binLabels.fast} · ${slowLabel} bin ${binLabels.slow} · best retained ${slice.scoreLabel} ${formatHeatmapDisplay(selectedValue, 4)}${sampleDisplay}\n${heatmapSelectionDetailText(renderModel, cell)}`
  }

  if (renderModel.displayMode === 'plateau') {
    const plateauDetails = renderModel.plateauDiagnostics?.[cell.idx] ?? null
    const plateauValue =
      typeof displayValue === 'number' && Number.isFinite(displayValue)
        ? displayValue
        : plateauDetails?.plateauScore ?? 0
    const plateauLabel =
      plateauDetails?.plateauLabel ??
      labelDerivedPlateau(
        plateauValue,
        plateauDetails?.supportiveNeighbors ?? 0,
        plateauDetails?.availableNeighbors ?? 0,
      )
    const supportDisplay = `${plateauDetails?.supportiveNeighbors ?? 0}/${plateauDetails?.availableNeighbors ?? 0}`
    const meanDisplay =
      plateauDetails?.meanValue != null && Number.isFinite(plateauDetails.meanValue)
        ? ` · mean ${formatHeatmapDisplay(plateauDetails.meanValue, 4)}`
        : ''
    const sampleDisplay =
      plateauDetails?.sampleCount != null && Number.isFinite(plateauDetails.sampleCount)
        ? ` · samples ${Math.max(0, Math.floor(plateauDetails.sampleCount))}`
        : ''
    const agreementDisplay =
      plateauDetails?.binAgreement != null &&
      Number.isFinite(plateauDetails.binAgreement) &&
      (plateauDetails.sampleCount ?? 0) > 1
        ? ` · bin ${formatHeatmapDisplay(plateauDetails.binAgreement, 3)}`
        : ''
    return `${fastLabel} bin ${binLabels.fast} · ${slowLabel} bin ${binLabels.slow} · Plateau ${plateauLabel} ${formatHeatmapDisplay(plateauValue, 4)} · supportive ${supportDisplay}${sampleDisplay}${meanDisplay}${agreementDisplay} · best-in-bin ${renderModel.request.scoreLabel} ${objectiveDisplay}\n${heatmapSelectionDetailText(renderModel, cell)}`
  }

  const displayMetric =
    typeof displayValue === 'number' && Number.isFinite(displayValue)
      ? formatHeatmapDisplay(displayValue, 4)
      : objectiveDisplay
  const meanValue = renderModel.model.meanValues?.[cell.idx]
  const sampleCount = renderModel.model.sampleCounts?.[cell.idx]
  const meanDisplay = finiteNumber(meanValue)
    ? ` · mean ${formatHeatmapDisplay(meanValue, 4)}`
    : ''
  const sampleDisplay = finiteNumber(sampleCount)
    ? ` · samples ${Math.max(0, Math.floor(sampleCount))}`
    : ''
  return `${fastLabel} bin ${binLabels.fast} · ${slowLabel} bin ${binLabels.slow} · best-in-bin ${renderModel.displayLabel} ${displayMetric}${meanDisplay}${sampleDisplay}\n${heatmapSelectionDetailText(renderModel, cell)}`
}

export function downloadHeatmapPng(
  base: HTMLCanvasElement | null,
  overlay: HTMLCanvasElement | null,
  fileName: string,
  restoreBase: () => void,
): boolean {
  if (
    !base ||
    !overlay ||
    base.width <= 0 ||
    base.height <= 0 ||
    overlay.width !== base.width ||
    overlay.height !== base.height ||
    typeof document === 'undefined'
  ) {
    return false
  }
  const ctx = canvasContext2d(base)
  if (!ctx) return false
  let href = ''
  ctx.save()
  try {
    ctx.setTransform(1, 0, 0, 1, 0, 0)
    ctx.drawImage(overlay, 0, 0, base.width, base.height)
    href = base.toDataURL('image/png')
  } catch {
    return false
  } finally {
    ctx.restore()
    restoreBase()
  }
  if (!href) return false
  const a = document.createElement('a')
  a.href = href
  a.download = fileName || 'heatmap.png'
  a.style.display = 'none'
  document.body?.appendChild(a)
  a.click()
  a.remove()
  return true
}
