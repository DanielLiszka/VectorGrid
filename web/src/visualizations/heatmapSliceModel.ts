import {
  heatmapColForFast,
  heatmapIndex,
  heatmapLogicalRowForSlow,
  heatmapPalette,
  type HeatmapModel,
  type HeatmapPalette,
  type HeatmapRange,
} from './heatmapModel'

export interface HeatmapSliceSourceRow {
  params: Record<string, unknown>
  metrics: Record<string, unknown>
  score: number
}

export interface HeatmapSliceCell {
  params: Record<string, unknown>
  metrics: Record<string, unknown>
}

export interface Heatmap3dSlice {
  key: string
  label: string
  shortLabel: string
  values: Array<number | null>
  sampleCounts: Array<number | null>
  cells: Array<HeatmapSliceCell | null>
  finiteCells: number
  bestScore: number
}

export interface HeatmapSlice3dModel {
  axisLabel: string
  scoreLabel: string
  slices: Heatmap3dSlice[]
  range: HeatmapRange
  palette: HeatmapPalette
  sourceRowCount: number
  retainedSliceRowCount: number
  unavailableReason: string
}

function finiteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value)
}

function num(value: unknown, fallback = Number.NaN): number {
  const next = Number(value)
  return Number.isFinite(next) ? next : fallback
}

function cleanText(value: unknown, fallback = ''): string {
  const text = String(value ?? '').trim()
  return text ? text : fallback
}

function stableStringify(value: unknown): string {
  if (value == null || typeof value !== 'object') return JSON.stringify(value)
  if (Array.isArray(value)) return `[${value.map(stableStringify).join(',')}]`
  const record = value as Record<string, unknown>
  return `{${Object.keys(record).sort().map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`).join(',')}}`
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {}
}

function paramsAreEmpty(value: Record<string, unknown>): boolean {
  return Object.keys(value).length === 0
}

function formatParamValue(value: unknown): string {
  const next = Number(value)
  if (Number.isFinite(next)) {
    return next.toFixed(4).replace(/(\.\d*?[1-9])0+$/u, '$1').replace(/\.0+$/u, '')
  }
  return cleanText(value, '-')
}

function formatParamSet(params: Record<string, unknown>): string {
  const entries = Object.entries(params)
    .filter(([, value]) => value != null && cleanText(value).length > 0)
    .sort(([left], [right]) => left.localeCompare(right))
  if (!entries.length) return 'default'
  return entries.map(([key, value]) => `${key}=${formatParamValue(value)}`).join(', ')
}

function sideLabel(maType: unknown, params: Record<string, unknown>): string {
  const type = cleanText(maType, '?').toUpperCase()
  return paramsAreEmpty(params) ? type : `${type} ${formatParamSet(params)}`
}

function sliceKeyForParams(params: Record<string, unknown>): string {
  return [
    cleanText(params.fast_ma_type, '?').toLowerCase(),
    stableStringify(asRecord(params.fast_ma_params ?? params.fastMaParams)),
    cleanText(params.slow_ma_type, '?').toLowerCase(),
    stableStringify(asRecord(params.slow_ma_params ?? params.slowMaParams)),
  ].join('|')
}

function sliceLabels(params: Record<string, unknown>): { label: string; shortLabel: string } {
  const fastParams = asRecord(params.fast_ma_params ?? params.fastMaParams)
  const slowParams = asRecord(params.slow_ma_params ?? params.slowMaParams)
  const fastType = cleanText(params.fast_ma_type, '?')
  const slowType = cleanText(params.slow_ma_type, '?')
  return {
    label: `${sideLabel(fastType, fastParams)} / ${sideLabel(slowType, slowParams)}`,
    shortLabel: `${cleanText(fastType, '?').toUpperCase()}/${cleanText(slowType, '?').toUpperCase()}`,
  }
}

function emptySlice(key: string, params: Record<string, unknown>, cellCount: number): Heatmap3dSlice {
  const labels = sliceLabels(params)
  return {
    key,
    label: labels.label,
    shortLabel: labels.shortLabel,
    values: new Array<number | null>(cellCount).fill(null),
    sampleCounts: new Array<number | null>(cellCount).fill(null),
    cells: new Array<HeatmapSliceCell | null>(cellCount).fill(null),
    finiteCells: 0,
    bestScore: Number.NEGATIVE_INFINITY,
  }
}

export function buildHeatmapSlice3dModel(options: {
  model: HeatmapModel
  rows: HeatmapSliceSourceRow[]
  scoreLabel: string
}): HeatmapSlice3dModel | null {
  const { model, rows, scoreLabel } = options
  const cellCount = model.binsFast * model.binsSlow
  if (cellCount <= 0 || !rows.length) return null

  const slicesByKey = new Map<string, Heatmap3dSlice>()
  let retainedSliceRowCount = 0

  for (const row of rows) {
    const score = row.score
    if (!finiteNumber(score)) continue
    const fast = Math.max(0, Math.floor(num(row.params.fast_len, 0)))
    const slow = Math.max(0, Math.floor(num(row.params.slow_len, 0)))
    if (fast <= 0 || slow <= 0) continue

    const key = sliceKeyForParams(row.params)
    let slice = slicesByKey.get(key)
    if (!slice) {
      slice = emptySlice(key, row.params, cellCount)
      slicesByKey.set(key, slice)
    }

    const col = heatmapColForFast(model, fast)
    const logicalRow = heatmapLogicalRowForSlow(model, slow)
    const idx = heatmapIndex(model, logicalRow, col)
    const count = Math.max(0, Math.floor(num(slice.sampleCounts[idx], 0))) + 1
    slice.sampleCounts[idx] = count
    const current = slice.values[idx]
    if (!finiteNumber(current) || score > current) {
      if (!finiteNumber(current)) slice.finiteCells += 1
      slice.values[idx] = score
      slice.cells[idx] = {
        params: row.params,
        metrics: row.metrics,
      }
      if (score > slice.bestScore) slice.bestScore = score
    }
    retainedSliceRowCount += 1
  }

  const slices = [...slicesByKey.values()]
    .filter((slice) => slice.finiteCells > 0)
    .sort((left, right) => right.bestScore - left.bestScore || left.label.localeCompare(right.label))

  if (slices.length < 2) return null

  let maxAbs = 1.0e-12
  for (const slice of slices) {
    for (const value of slice.values) {
      if (!finiteNumber(value)) continue
      const magnitude = Math.abs(value)
      if (magnitude > maxAbs) maxAbs = magnitude
    }
  }
  return {
    axisLabel: 'MA type / parameter slice',
    scoreLabel: cleanText(scoreLabel, 'Score'),
    slices,
    range: { min: -maxAbs, max: maxAbs, label: 'zero-centered' },
    palette: heatmapPalette('objective'),
    sourceRowCount: rows.length,
    retainedSliceRowCount,
    unavailableReason: '',
  }
}
