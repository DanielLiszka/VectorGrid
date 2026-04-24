export interface ParetoAxisOption {
  value: string
  label: string
  disabled: boolean
}

export interface ParetoMoValues {
  maximize_value?: unknown
  minimize_value?: unknown
  violation?: unknown
}

export interface ParetoSourceRow {
  rank: number
  metrics: Record<string, unknown>
  mo?: ParetoMoValues | null
}

export interface ParetoRange {
  min: number
  max: number
}

export interface ParetoPoint<T> {
  index: number
  row: T
  rowId: string
  rank: number
  x: number
  y: number
  frontier: boolean
  summary: string
}

export interface ParetoRenderPoint {
  index: number
  rowId: string
  rank: number
  x: number
  y: number
  frontier: boolean
  summary: string
}

export interface ParetoViewModel<T> {
  xKey: string
  yKey: string
  points: ParetoPoint<T>[]
  frontier: ParetoPoint<T>[]
  xRange: ParetoRange
  yRange: ParetoRange
  meta: string
  truthNote: string
  representativeSummary: string
  confidenceNote: string
  status: 'ready' | 'unavailable'
}

export interface ParetoRenderModel {
  xKey: string
  yKey: string
  points: ParetoRenderPoint[]
  frontier: ParetoRenderPoint[]
  xRange: ParetoRange
  yRange: ParetoRange
  meta: string
  truthNote: string
  representativeSummary: string
  confidenceNote: string
  status: 'ready' | 'unavailable'
}

export interface BuildParetoViewModelOptions<T> {
  rows: T[]
  xKey: string
  yKey: string
  axisValueFor: (row: T, key: string) => number
  rowIdFor?: (row: T, index: number) => string
  rankFor?: (row: T, index: number) => number
  selectedRowId?: string | null
  selectedRank?: number | null
  hasValidationResult?: boolean
  limit?: number
  formatValue?: (value: number, digits?: number) => string
}

const BASE_AXIS_OPTIONS: ParetoAxisOption[] = [
  { value: 'pnl', label: 'PnL', disabled: false },
  { value: 'sharpe', label: 'Sharpe', disabled: false },
  { value: 'max_dd', label: 'Max DD', disabled: false },
  { value: 'trades', label: 'Trades', disabled: false },
  { value: 'exposure', label: 'Exposure', disabled: false },
  { value: 'score', label: 'Score', disabled: false },
]

const MO_AXIS_OPTIONS: ParetoAxisOption[] = [
  { value: 'maximize_value', label: 'Max objective', disabled: false },
  { value: 'minimize_value', label: 'Min objective', disabled: false },
  { value: 'violation', label: 'Violation', disabled: false },
]

export const PARETO_TRUTH_NOTE =
  'Pareto points are non-dominated under the selected axes. They visualize tradeoffs, not validation approval.'

export const PARETO_UNAVAILABLE_TRUTH_NOTE =
  'Pareto points show non-dominated tradeoffs under the selected axes, not proof of robustness.'

function text(value: unknown): string {
  return String(value ?? '').trim()
}

function finiteNumber(value: unknown, fallback = Number.NaN): number {
  const next = Number(value)
  return Number.isFinite(next) ? next : fallback
}

function trimFixed(src: string): string {
  return src.replace(/(\.\d*?[1-9])0+$/u, '$1').replace(/\.0+$/u, '')
}

export function formatParetoDisplay(value: unknown, digits = 4): string {
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

export function buildParetoAxisOptions(hasMultiObjectiveValues: boolean): ParetoAxisOption[] {
  return hasMultiObjectiveValues ? [...BASE_AXIS_OPTIONS, ...MO_AXIS_OPTIONS] : [...BASE_AXIS_OPTIONS]
}

export function paretoAxisValue(
  row: ParetoSourceRow,
  keyRaw: unknown,
  scoreValue = Number.NaN,
): number {
  const key = text(keyRaw || 'pnl') || 'pnl'
  if (key === 'maximize_value') return finiteNumber(row.mo?.maximize_value)
  if (key === 'minimize_value') return finiteNumber(row.mo?.minimize_value)
  if (key === 'violation') return finiteNumber(row.mo?.violation)
  if (key === 'score') return finiteNumber(scoreValue)
  return finiteNumber((row.metrics || {})[key])
}

export function paddedParetoRange(values: number[], padRatio = 0.08): ParetoRange {
  if (!values.length) return { min: 0, max: 1 }
  let min = Math.min(...values)
  let max = Math.max(...values)
  if (min === max) {
    min -= 1
    max += 1
  }
  const pad = (max - min) * Math.max(0, padRatio)
  return {
    min: min - pad,
    max: max + pad,
  }
}

export function buildParetoConfidenceNote(hasValidationResult = false): string {
  if (!hasValidationResult) {
    return 'Validation has not run. Frontier membership alone does not approve a candidate.'
  }
  return 'Validation adds admission evidence, but robust_pareto and cross_ticker_pareto are still filters rather than approval by themselves.'
}

export function buildParetoRepresentativeSummary<T>(
  points: ParetoPoint<T>[],
  selectedRowId?: string | null,
  selectedRank?: number | null,
): string {
  if (!points.length) return '—'
  const xExtreme = points.reduce((best, point) => (point.x < best.x ? point : best))
  const yExtreme = points.reduce((best, point) => (point.y > best.y ? point : best))
  const parts = [`x-extreme #${xExtreme.rank}`, `y-extreme #${yExtreme.rank}`]
  const selectedId = text(selectedRowId)
  if (selectedId) {
    const selectedPoint = points.find((point) => point.rowId === selectedId) || null
    const rank = selectedPoint?.rank ?? selectedRank
    if (rank != null) {
      parts.push(selectedPoint ? `selected #${rank}` : `selected off-frontier #${rank}`)
    }
  }
  return parts.join(' • ')
}

export function buildParetoViewModel<T>({
  rows,
  xKey,
  yKey,
  axisValueFor,
  rowIdFor,
  rankFor,
  selectedRowId = null,
  selectedRank = null,
  hasValidationResult = false,
  limit = 512,
  formatValue = formatParetoDisplay,
}: BuildParetoViewModelOptions<T>): ParetoViewModel<T> {
  const normalizedXKey = text(xKey || 'max_dd') || 'max_dd'
  const normalizedYKey = text(yKey || 'pnl') || 'pnl'
  const cappedRows = rows.slice(0, Math.max(0, Math.floor(limit)))
  const points = cappedRows
    .map((row, index): ParetoPoint<T> | null => {
      const x = finiteNumber(axisValueFor(row, normalizedXKey))
      const y = finiteNumber(axisValueFor(row, normalizedYKey))
      if (!Number.isFinite(x) || !Number.isFinite(y)) return null
      const rank = rankFor?.(row, index) ?? index + 1
      const rowId = rowIdFor?.(row, index) ?? String(rank)
      return {
        index,
        row,
        rowId,
        rank,
        x,
        y,
        frontier: true,
        summary: `#${rank} • ${normalizedXKey}=${formatValue(x, 3)} • ${normalizedYKey}=${formatValue(y, 3)}`,
      }
    })
    .filter((point): point is ParetoPoint<T> => point != null)

  const confidenceNote = buildParetoConfidenceNote(hasValidationResult)
  if (!points.length) {
    return {
      xKey: normalizedXKey,
      yKey: normalizedYKey,
      points,
      frontier: [],
      xRange: { min: 0, max: 1 },
      yRange: { min: 0, max: 1 },
      meta: 'Pareto scatter unavailable for this run.',
      truthNote: PARETO_UNAVAILABLE_TRUTH_NOTE,
      representativeSummary: '—',
      confidenceNote,
      status: 'unavailable',
    }
  }

  const xRange = paddedParetoRange(points.map((point) => point.x))
  const yRange = paddedParetoRange(points.map((point) => point.y))
  return {
    xKey: normalizedXKey,
    yKey: normalizedYKey,
    points,
    frontier: [...points].sort((left, right) => left.x - right.x),
    xRange,
    yRange,
    meta: `${points.length} points • x=${normalizedXKey} (${formatValue(xRange.min, 3)} to ${formatValue(xRange.max, 3)}) • y=${normalizedYKey} (${formatValue(yRange.min, 3)} to ${formatValue(yRange.max, 3)})`,
    truthNote: PARETO_TRUTH_NOTE,
    representativeSummary: buildParetoRepresentativeSummary(points, selectedRowId, selectedRank),
    confidenceNote,
    status: 'ready',
  }
}

export function toParetoRenderModel<T>(model: ParetoViewModel<T>): ParetoRenderModel {
  const toPoint = (point: ParetoPoint<T>): ParetoRenderPoint => ({
    index: point.index,
    rowId: point.rowId,
    rank: point.rank,
    x: point.x,
    y: point.y,
    frontier: point.frontier,
    summary: point.summary,
  })

  return {
    xKey: model.xKey,
    yKey: model.yKey,
    points: model.points.map(toPoint),
    frontier: model.frontier.map(toPoint),
    xRange: { ...model.xRange },
    yRange: { ...model.yRange },
    meta: model.meta,
    truthNote: model.truthNote,
    representativeSummary: model.representativeSummary,
    confidenceNote: model.confidenceNote,
    status: model.status,
  }
}
