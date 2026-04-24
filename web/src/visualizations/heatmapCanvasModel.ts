import {
  fastForHeatmapCol,
  heatmapIndex,
  labelDerivedPlateau,
  slowForHeatmapLogicalRow,
  type HeatmapMode,
  type HeatmapModel,
  type HeatmapPalette,
  type HeatmapPlateauCellDiagnostics,
  type HeatmapRange,
} from './heatmapModel'

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

export interface HeatmapCanvasRequestModel {
  fastMaType: string
  slowMaType: string
  fastMaParams: unknown
  slowMaParams: unknown
  scoreLabel: string
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
}

export type HeatmapCellParams = Record<string, unknown>

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

function prepareLogicalCanvasContext(
  canvas: HTMLCanvasElement,
): { ctx: CanvasRenderingContext2D; width: number; height: number } | null {
  const ctx = canvasContext2d(canvas)
  if (!ctx) return null
  const { width, height, ratio } = logicalCanvasSize(canvas)
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0)
  return { ctx, width, height }
}

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, value))
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
  if (abs < zeroThreshold) return '0'
  if (abs >= 1_000_000) {
    return next.toLocaleString(undefined, {
      maximumFractionDigits: Math.min(2, precision),
      minimumFractionDigits: 0,
    })
  }
  return trimFixed(next.toFixed(precision))
}

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const normalized = hex.replace('#', '').trim()
  const chunk =
    normalized.length === 3
      ? normalized
          .split('')
          .map((value) => `${value}${value}`)
          .join('')
      : normalized.padStart(6, '0').slice(0, 6)
  const numValue = Number.parseInt(chunk, 16)
  return {
    r: (numValue >> 16) & 0xff,
    g: (numValue >> 8) & 0xff,
    b: numValue & 0xff,
  }
}

function rgbToCss(rgb: { r: number; g: number; b: number }, alpha = 1): string {
  return `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${Math.max(0, Math.min(1, alpha))})`
}

export function sampleHeatmapColor(
  value: number,
  minValue: number,
  maxValue: number,
  palette: HeatmapPalette,
): string {
  if (palette.kind === 'sequential') {
    const low = hexToRgb(palette.low)
    const high = hexToRgb(palette.high)
    const denom = Math.max(1.0e-12, maxValue - minValue)
    const ratio = clamp01((value - minValue) / denom)
    return rgbToCss({
      r: Math.round(low.r + (high.r - low.r) * ratio),
      g: Math.round(low.g + (high.g - low.g) * ratio),
      b: Math.round(low.b + (high.b - low.b) * ratio),
    })
  }

  const anchor = Math.max(1.0e-12, Math.max(Math.abs(minValue), Math.abs(maxValue)))
  const zero = hexToRgb(palette.zero)
  if (value <= 0) {
    const neg = hexToRgb(palette.negative)
    const ratio = clamp01(Math.abs(value) / anchor)
    return rgbToCss({
      r: Math.round(zero.r + (neg.r - zero.r) * ratio),
      g: Math.round(zero.g + (neg.g - zero.g) * ratio),
      b: Math.round(zero.b + (neg.b - zero.b) * ratio),
    })
  }

  const pos = hexToRgb(palette.positive)
  const ratio = clamp01(value / anchor)
  return rgbToCss({
    r: Math.round(zero.r + (pos.r - zero.r) * ratio),
    g: Math.round(zero.g + (pos.g - zero.g) * ratio),
    b: Math.round(zero.b + (pos.b - zero.b) * ratio),
  })
}

export function paintHeatmapLegend(
  targets: HeatmapLegendTargets,
  range: HeatmapRange,
  palette: HeatmapPalette,
): void {
  const legend = targets.canvas
  if (!legend) return
  const ctx = canvasContext2d(legend)
  if (!ctx) return
  const width = legend.width
  const height = legend.height
  const gradient = ctx.createLinearGradient(0, 0, width, 0)
  if (palette.kind === 'sequential') {
    gradient.addColorStop(0, palette.low)
    gradient.addColorStop(1, palette.high)
  } else {
    gradient.addColorStop(0, palette.negative)
    gradient.addColorStop(0.5, palette.zero)
    gradient.addColorStop(1, palette.positive)
  }
  ctx.clearRect(0, 0, width, height)
  ctx.fillStyle = gradient
  ctx.fillRect(0, 0, width, height)
  if (targets.minLabel) targets.minLabel.textContent = formatHeatmapDisplay(range.min, 4)
  if (targets.midLabel) targets.midLabel.textContent = formatHeatmapDisplay((range.min + range.max) / 2, 4)
  if (targets.maxLabel) targets.maxLabel.textContent = formatHeatmapDisplay(range.max, 4)
}

export function clearHeatmapCanvas(canvas: HTMLCanvasElement | null): void {
  const ctx = canvasContext2d(canvas)
  if (!ctx || !canvas) return
  ctx.setTransform(1, 0, 0, 1, 0, 0)
  ctx.clearRect(0, 0, canvas.width, canvas.height)
}

export function paintHeatmapBase(
  renderModel: HeatmapCanvasRenderModel,
  canvas: HTMLCanvasElement,
): void {
  const { model, displayValues, range, palette } = renderModel
  const prepared = prepareLogicalCanvasContext(canvas)
  if (!prepared) return
  const { ctx, width, height } = prepared
  const cellW = width / Math.max(1, model.binsFast)
  const cellH = height / Math.max(1, model.binsSlow)

  ctx.clearRect(0, 0, width, height)
  for (let logicalRow = 0; logicalRow < model.binsSlow; logicalRow += 1) {
    const drawRow = model.binsSlow - 1 - logicalRow
    for (let col = 0; col < model.binsFast; col += 1) {
      const value = displayValues[heatmapIndex(model, logicalRow, col)]
      const x = col * cellW
      const y = drawRow * cellH
      if (typeof value !== 'number' || !Number.isFinite(value)) {
        ctx.fillStyle = 'rgba(82, 96, 118, 0.12)'
        ctx.fillRect(x, y, cellW, cellH)
        continue
      }
      ctx.fillStyle = sampleHeatmapColor(value, range.min, range.max, palette)
      ctx.fillRect(x, y, cellW, cellH)
    }
  }
}

export function paintHeatmapOverlay(
  renderModel: HeatmapCanvasRenderModel,
  overlay: HTMLCanvasElement,
  hoverCell: HeatmapHoverCell | null,
): void {
  const { model, markers } = renderModel
  const prepared = prepareLogicalCanvasContext(overlay)
  if (!prepared) return
  const { ctx: overlayCtx, width, height } = prepared
  const cellW = width / Math.max(1, model.binsFast)
  const cellH = height / Math.max(1, model.binsSlow)

  overlayCtx.clearRect(0, 0, width, height)
  for (const marker of markers) {
    const cx = (marker.col + 0.5) * cellW
    const cy = (marker.drawRow + 0.5) * cellH
    overlayCtx.fillStyle = 'rgba(248, 250, 252, 0.95)'
    overlayCtx.beginPath()
    overlayCtx.arc(cx, cy, Math.max(3, Math.min(cellW, cellH) * 0.18), 0, Math.PI * 2)
    overlayCtx.fill()
    overlayCtx.strokeStyle = 'rgba(15, 23, 42, 0.95)'
    overlayCtx.lineWidth = 1
    overlayCtx.stroke()
    overlayCtx.fillStyle = 'rgba(15, 23, 42, 0.95)'
    overlayCtx.font = '11px ui-monospace, monospace'
    overlayCtx.fillText(marker.label, cx + 5, cy - 5)
  }

  if (!hoverCell) return
  const hoverX = hoverCell.col * cellW
  const hoverY = hoverCell.row * cellH
  overlayCtx.strokeStyle = 'rgba(248, 250, 252, 0.95)'
  overlayCtx.lineWidth = 1.5
  overlayCtx.strokeRect(hoverX + 0.5, hoverY + 0.5, Math.max(0, cellW - 1), Math.max(0, cellH - 1))
  overlayCtx.beginPath()
  overlayCtx.moveTo(hoverX + cellW / 2, 0)
  overlayCtx.lineTo(hoverX + cellW / 2, height)
  overlayCtx.moveTo(0, hoverY + cellH / 2)
  overlayCtx.lineTo(width, hoverY + cellH / 2)
  overlayCtx.strokeStyle = 'rgba(248, 250, 252, 0.28)'
  overlayCtx.stroke()
}

export function resolveHeatmapCellFromPoint(
  overlay: HTMLCanvasElement,
  model: HeatmapModel,
  clientX: number,
  clientY: number,
): HeatmapResolvedCell | null {
  const rect = overlay.getBoundingClientRect()
  if (!rect.width || !rect.height) return null
  const col = Math.max(
    0,
    Math.min(model.binsFast - 1, Math.floor(((clientX - rect.left) / rect.width) * model.binsFast)),
  )
  const drawRow = Math.max(
    0,
    Math.min(model.binsSlow - 1, Math.floor(((clientY - rect.top) / rect.height) * model.binsSlow)),
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

export function buildHeatmapCellParams(
  renderModel: HeatmapCanvasRenderModel,
  cell: HeatmapResolvedCell,
): HeatmapCellParams | null {
  const value = renderModel.model.values[cell.idx]
  if (typeof value !== 'number' || !Number.isFinite(value)) return null
  const fast = fastForHeatmapCol(renderModel.model, cell.col)
  const slowBase = slowForHeatmapLogicalRow(renderModel.model, cell.logicalRow)
  const slow = Math.max(slowBase, fast + 1)
  return {
    fast_len: fast,
    fast_ma_type: renderModel.request.fastMaType || '?',
    slow_len: slow,
    slow_ma_type: renderModel.request.slowMaType || '?',
    fast_ma_params: renderModel.request.fastMaParams ?? null,
    slow_ma_params: renderModel.request.slowMaParams ?? null,
  }
}

export function heatmapTooltipText(
  renderModel: HeatmapCanvasRenderModel,
  cell: HeatmapResolvedCell,
): string | null {
  const objectiveValue = renderModel.model.values[cell.idx]
  if (typeof objectiveValue !== 'number' || !Number.isFinite(objectiveValue)) return null
  const displayValue = renderModel.displayValues[cell.idx]
  const fast = fastForHeatmapCol(renderModel.model, cell.col)
  const slow = slowForHeatmapLogicalRow(renderModel.model, cell.logicalRow)
  const fastLabel = renderModel.request.fastMaType.toUpperCase() || 'FAST'
  const slowLabel = renderModel.request.slowMaType.toUpperCase() || 'SLOW'
  const objectiveDisplay = formatHeatmapDisplay(objectiveValue, 4)

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
    return `${fastLabel} ${fast} · ${slowLabel} ${slow} · Plateau ${plateauLabel} ${formatHeatmapDisplay(plateauValue, 4)} · supportive ${supportDisplay}${sampleDisplay}${meanDisplay}${agreementDisplay} · ${renderModel.request.scoreLabel} ${objectiveDisplay}`
  }

  const displayMetric =
    typeof displayValue === 'number' && Number.isFinite(displayValue)
      ? formatHeatmapDisplay(displayValue, 4)
      : objectiveDisplay
  return `${fastLabel} ${fast} · ${slowLabel} ${slow} · ${renderModel.displayLabel} ${displayMetric}`
}

export function downloadHeatmapPng(
  base: HTMLCanvasElement | null,
  overlay: HTMLCanvasElement | null,
  fileName: string,
): boolean {
  if (!base || !overlay || base.width <= 0 || base.height <= 0 || typeof document === 'undefined') {
    return false
  }
  const tmp = document.createElement('canvas')
  tmp.width = base.width
  tmp.height = base.height
  const ctx = canvasContext2d(tmp)
  if (!ctx) return false
  ctx.drawImage(base, 0, 0)
  ctx.drawImage(overlay, 0, 0)
  const a = document.createElement('a')
  a.href = tmp.toDataURL('image/png')
  a.download = fileName || 'heatmap.png'
  a.style.display = 'none'
  document.body?.appendChild(a)
  a.click()
  a.remove()
  return true
}
