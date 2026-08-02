import {
  heatmapPalette,
  type HeatmapPalette,
} from './heatmapTheme'

export {
  heatmapPalette,
  type HeatmapPalette,
} from './heatmapTheme'

export type HeatmapMode = 'objective' | 'plateau'

export interface HeatmapModel {
  binsFast: number
  binsSlow: number
  fastMin: number
  fastMax: number
  slowMin: number
  slowMax: number
  values: Array<number | null>
  meanValues: Array<number | null> | null
  sampleCounts: Array<number | null> | null
}

export interface HeatmapRange {
  min: number
  max: number
  label: string
}

export interface HeatmapPlateauCellDiagnostics {
  plateauScore: number
  plateauLabel: string
  supportiveNeighbors: number
  availableNeighbors: number
  sampleCount: number | null
  meanValue: number | null
  binAgreement: number | null
}

export interface DerivedPlateauSurface {
  values: Array<number | null>
  diagnostics: Array<HeatmapPlateauCellDiagnostics | null>
  multiSampleCellCount: number
}

export interface HeatmapDisplayModel {
  values: Array<number | null>
  range: HeatmapRange
  palette: HeatmapPalette
  label: string
  legendNote: string
  plateauDiagnostics: Array<HeatmapPlateauCellDiagnostics | null> | null
  multiSampleCellCount: number
}

const HEATMAP_NEIGHBOR_OFFSETS_CARDINAL: ReadonlyArray<readonly [number, number]> = [
  [0, -1],
  [0, 1],
  [-1, 0],
  [1, 0],
]

const HEATMAP_NEIGHBOR_OFFSETS_ALL: ReadonlyArray<readonly [number, number]> = [
  ...HEATMAP_NEIGHBOR_OFFSETS_CARDINAL,
  [-1, -1],
  [-1, 1],
  [1, -1],
  [1, 1],
]
const HEATMAP_CELL_ORDER = 'slow_fast_row_major_v1'
const MAX_HEATMAP_AXIS_BINS = 512
const MAX_HEATMAP_CELLS = MAX_HEATMAP_AXIS_BINS * MAX_HEATMAP_AXIS_BINS
const MAX_HEATMAP_PERIOD = 65_535

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' ? (value as Record<string, unknown>) : {}
}

function nonNegativeInteger(value: unknown): number | null {
  const next = Number(value)
  return Number.isInteger(next) && next >= 0 ? next : null
}

export function readHeatmapModel(result: Record<string, unknown>): HeatmapModel | null {
  const heatmap = asRecord(result.heatmap)
  if (!Object.keys(heatmap).length) return null
  const binsFast = nonNegativeInteger(heatmap.bins_fast)
  const binsSlow = nonNegativeInteger(heatmap.bins_slow)
  const fastMin = nonNegativeInteger(heatmap.fast_min)
  const fastMax = nonNegativeInteger(heatmap.fast_max)
  const slowMin = nonNegativeInteger(heatmap.slow_min)
  const slowMax = nonNegativeInteger(heatmap.slow_max)
  if (
    binsFast == null ||
    binsSlow == null ||
    fastMin == null ||
    fastMax == null ||
    slowMin == null ||
    slowMax == null ||
    binsFast <= 0 ||
    binsSlow <= 0 ||
    binsFast > MAX_HEATMAP_AXIS_BINS ||
    binsSlow > MAX_HEATMAP_AXIS_BINS ||
    fastMin > fastMax ||
    slowMin > slowMax ||
    fastMax > MAX_HEATMAP_PERIOD ||
    slowMax > MAX_HEATMAP_PERIOD
  ) {
    return null
  }
  const expectedLen = binsFast * binsSlow
  if (
    !Number.isSafeInteger(expectedLen) ||
    expectedLen <= 0 ||
    expectedLen > MAX_HEATMAP_CELLS
  ) {
    return null
  }
  if (
    heatmap.cell_order != null &&
    String(heatmap.cell_order).trim() !== HEATMAP_CELL_ORDER
  ) {
    return null
  }
  const rawValues = Array.isArray(heatmap.values) ? heatmap.values : []
  if (rawValues.length !== expectedLen) return null
  if (
    heatmap.mean_values != null &&
    (!Array.isArray(heatmap.mean_values) ||
      heatmap.mean_values.length !== expectedLen)
  ) {
    return null
  }
  if (
    heatmap.sample_counts != null &&
    (!Array.isArray(heatmap.sample_counts) ||
      heatmap.sample_counts.length !== expectedLen)
  ) {
    return null
  }

  const readHeatmapVector = (
    raw: unknown,
    mapValue: (value: unknown) => number | null,
  ): Array<number | null> | null => {
    if (!Array.isArray(raw) || raw.length !== expectedLen) return null
    return raw.map(mapValue)
  }

  const meanValues = readHeatmapVector(heatmap.mean_values, (value) => {
    if (value == null) return null
    const next = Number(value)
    return Number.isFinite(next) ? next : null
  })
  const sampleCounts = readHeatmapVector(heatmap.sample_counts, (value) => {
    if (value == null) return null
    const next = Number(value)
    if (!Number.isInteger(next) || next < 0) return null
    return next
  })

  return {
    binsFast,
    binsSlow,
    fastMin,
    fastMax,
    slowMin,
    slowMax,
    values: rawValues.map((value) => {
      if (value == null) return null
      const next = Number(value)
      return Number.isFinite(next) ? next : null
    }),
    meanValues,
    sampleCounts,
  }
}

export function finiteValues(values: Array<number | null>): number[] {
  const out: number[] = []
  for (const value of values) {
    if (typeof value === 'number' && Number.isFinite(value)) out.push(value)
  }
  return out
}

export function finiteHeatmapValues(model: HeatmapModel): number[] {
  return finiteValues(model.values)
}

export function heatmapModeFromValue(value: unknown): HeatmapMode {
  return String(value ?? '').trim().toLowerCase() === 'plateau' ? 'plateau' : 'objective'
}

export function heatmapDisplayLabel(mode: HeatmapMode, objectiveLabel: string): string {
  return mode === 'plateau' ? 'Derived plateau' : objectiveLabel
}

export function heatmapLegendNote(mode: HeatmapMode): string {
  return mode === 'plateau'
    ? 'Sequential perceptual stability scale · fragile → resilient'
    : 'Zero-centered perceptual scale · negative ↔ neutral 0 ↔ positive'
}

export function heatmapQuantile(sorted: number[], p: number): number {
  if (!sorted.length) return 0
  const clamped = Math.max(0, Math.min(1, p))
  const pos = (sorted.length - 1) * clamped
  const base = Math.floor(pos)
  const frac = pos - base
  const left = sorted[base] ?? sorted[0] ?? 0
  const right = sorted[Math.min(sorted.length - 1, base + 1)] ?? left
  return left + (right - left) * frac
}

export function resolveHeatmapRange(values: number[]): HeatmapRange {
  if (!values.length) return { min: -1, max: 1, label: 'zero-centered' }
  let maxAbs = 1e-12
  for (const value of values) {
    const magnitude = Math.abs(value)
    if (Number.isNaN(magnitude) || magnitude > maxAbs) {
      maxAbs = magnitude
    }
  }
  return { min: -maxAbs, max: maxAbs, label: 'zero-centered' }
}

export function resolveNullableHeatmapRange(values: Array<number | null>): HeatmapRange {
  let maxAbs = 1e-12
  let count = 0
  for (const value of values) {
    if (typeof value !== 'number' || !Number.isFinite(value)) continue
    count += 1
    const magnitude = Math.abs(value)
    if (magnitude > maxAbs) maxAbs = magnitude
  }
  if (!count) return { min: -1, max: 1, label: 'zero-centered' }
  return { min: -maxAbs, max: maxAbs, label: 'zero-centered' }
}

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, value))
}

export function heatmapHigherIsBetter(_objective: string, _scoreMode = false): boolean {
  // Optimization heatmaps contain the backend's normalized ranking score, not
  // the raw objective metric. Lower-is-better metrics such as volatility and
  // max drawdown are negated before aggregation, so every exported heatmap
  // surface follows the same higher-is-better contract.
  return true
}

export function heatmapIndex(model: HeatmapModel, logicalRow: number, col: number): number {
  return logicalRow * model.binsFast + col
}

export function heatmapValueAt(
  model: HeatmapModel,
  logicalRow: number,
  col: number,
): number | null {
  if (
    logicalRow < 0 ||
    logicalRow >= model.binsSlow ||
    col < 0 ||
    col >= model.binsFast
  ) {
    return null
  }
  const value = model.values[heatmapIndex(model, logicalRow, col)]
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

export function heatmapMeanValueAt(
  model: HeatmapModel,
  logicalRow: number,
  col: number,
): number | null {
  if (
    logicalRow < 0 ||
    logicalRow >= model.binsSlow ||
    col < 0 ||
    col >= model.binsFast ||
    !model.meanValues
  ) {
    return null
  }
  const value = model.meanValues[heatmapIndex(model, logicalRow, col)]
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

export function heatmapSampleCountAt(
  model: HeatmapModel,
  logicalRow: number,
  col: number,
): number | null {
  if (
    logicalRow < 0 ||
    logicalRow >= model.binsSlow ||
    col < 0 ||
    col >= model.binsFast ||
    !model.sampleCounts
  ) {
    return null
  }
  const value = model.sampleCounts[heatmapIndex(model, logicalRow, col)]
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

export function heatmapNeighborOffsets(includeDiagonals = false): Array<[number, number]> {
  const offsets = includeDiagonals
    ? HEATMAP_NEIGHBOR_OFFSETS_ALL
    : HEATMAP_NEIGHBOR_OFFSETS_CARDINAL
  return offsets.map(([rowOffset, colOffset]) => [rowOffset, colOffset])
}

export function heatmapNeighborValues(
  model: HeatmapModel,
  logicalRow: number,
  col: number,
  includeDiagonals = false,
): number[] {
  const offsets = includeDiagonals
    ? HEATMAP_NEIGHBOR_OFFSETS_ALL
    : HEATMAP_NEIGHBOR_OFFSETS_CARDINAL
  const out: number[] = []
  for (const [rowOffset, colOffset] of offsets) {
    const value = heatmapValueAt(model, logicalRow + rowOffset, col + colOffset)
    if (typeof value === 'number') out.push(value)
  }
  return out
}

export function labelDerivedPlateau(
  score: number,
  supportiveNeighbors: number,
  availableNeighbors: number,
): string {
  const supportRatio =
    availableNeighbors > 0 ? supportiveNeighbors / availableNeighbors : 0
  if (availableNeighbors >= 3 && supportRatio >= 0.7 && score >= 0.78) return 'Strong'
  if (supportRatio >= 0.45 && score >= 0.58) return 'Moderate'
  if (score >= 0.38) return 'Narrow'
  return 'Fragile'
}

export function buildDerivedPlateauSurface(
  model: HeatmapModel,
  options: { higherIsBetter: boolean },
): DerivedPlateauSurface {
  const out = Array<number | null>(model.values.length).fill(null)
  const diagnostics = Array<HeatmapPlateauCellDiagnostics | null>(
    model.values.length,
  ).fill(null)
  let surfaceMin = Number.POSITIVE_INFINITY
  let surfaceMax = Number.NEGATIVE_INFINITY
  let finiteCount = 0
  for (const value of model.values) {
    if (typeof value !== 'number' || !Number.isFinite(value)) continue
    finiteCount += 1
    if (value < surfaceMin) surfaceMin = value
    if (value > surfaceMax) surfaceMax = value
  }
  if (!finiteCount) {
    return { values: out, diagnostics, multiSampleCellCount: 0 }
  }
  const { higherIsBetter } = options
  const meritDenom = Math.max(1.0e-9, surfaceMax - surfaceMin)
  let multiSampleCellCount = 0
  const neighbors: number[] = []
  const localValues: number[] = []

  for (let logicalRow = 0; logicalRow < model.binsSlow; logicalRow += 1) {
    for (let col = 0; col < model.binsFast; col += 1) {
      const base = heatmapValueAt(model, logicalRow, col)
      if (base == null) continue
      const idx = heatmapIndex(model, logicalRow, col)
      neighbors.length = 0
      localValues.length = 0
      localValues.push(base)
      for (const [rowOffset, colOffset] of HEATMAP_NEIGHBOR_OFFSETS_ALL) {
        const value = heatmapValueAt(model, logicalRow + rowOffset, col + colOffset)
        if (value == null) continue
        neighbors.push(value)
        localValues.push(value)
      }
      if (!neighbors.length) {
        out[idx] = 0
        diagnostics[idx] = {
          plateauScore: 0,
          plateauLabel: 'Fragile',
          supportiveNeighbors: 0,
          availableNeighbors: 0,
          sampleCount: heatmapSampleCountAt(model, logicalRow, col),
          meanValue: heatmapMeanValueAt(model, logicalRow, col),
          binAgreement: null,
        }
        continue
      }

      localValues.sort((a, b) => a - b)
      const localMedian = heatmapQuantile(localValues, 0.5)
      const localMin = localValues[0] ?? base
      const localMax = localValues[localValues.length - 1] ?? base
      const localSpan = Math.abs(localMax - localMin)
      const scoreScale = Math.max(localSpan, meritDenom * 0.1, 1.0e-9)
      const supportTolerance = Math.max(localSpan * 0.75, meritDenom * 0.06, 1.0e-9)
      const merit =
        meritDenom <= 1.0e-9
          ? 1
          : higherIsBetter
            ? clamp01((base - surfaceMin) / meritDenom)
            : clamp01((surfaceMax - base) / meritDenom)
      const adverseMedianGap = higherIsBetter
        ? Math.max(0, base - localMedian)
        : Math.max(0, localMedian - base)
      const peakDelta = higherIsBetter
        ? Math.max(0, localMax - base)
        : Math.max(0, base - localMin)
      const medianStability = 1 / (1 + adverseMedianGap / scoreScale)
      const peakStability = 1 / (1 + peakDelta / scoreScale)
      const scoreStability = 0.6 * medianStability + 0.4 * peakStability
      const availableNeighbors = neighbors.length
      let supportiveNeighbors = 0
      let supportWeightSum = 0
      let roughnessSum = 0
      for (const value of neighbors) {
        const adverseDelta = higherIsBetter
          ? Math.max(0, base - value)
          : Math.max(0, value - base)
        const weight = 1 / (1 + adverseDelta / supportTolerance)
        supportWeightSum += weight
        if (weight >= 0.5) supportiveNeighbors += 1
        const delta = value - base
        roughnessSum += delta * delta
      }
      const supportStrength =
        supportWeightSum / Math.max(1, neighbors.length)
      const roughnessRmsDelta = Math.sqrt(roughnessSum / neighbors.length)
      const roughnessStability = 1 / (1 + roughnessRmsDelta / scoreScale)

      let curvaturePenaltySum = 0
      let curvaturePenaltyCount = 0
      const left = heatmapValueAt(model, logicalRow, col - 1)
      const right = heatmapValueAt(model, logicalRow, col + 1)
      if (left != null && right != null) {
        curvaturePenaltySum += Math.abs(base - (left + right) / 2)
        curvaturePenaltyCount += 1
      }
      const up = heatmapValueAt(model, logicalRow - 1, col)
      const down = heatmapValueAt(model, logicalRow + 1, col)
      if (up != null && down != null) {
        curvaturePenaltySum += Math.abs(base - (up + down) / 2)
        curvaturePenaltyCount += 1
      }
      const curvaturePenalty = curvaturePenaltyCount
        ? curvaturePenaltySum / curvaturePenaltyCount
        : scoreScale
      const curvatureStability = 1 / (1 + curvaturePenalty / scoreScale)
      const sampleCount = heatmapSampleCountAt(model, logicalRow, col)
      const meanValue = heatmapMeanValueAt(model, logicalRow, col)
      if ((sampleCount ?? 0) > 1) multiSampleCellCount += 1
      const binAgreement =
        sampleCount != null &&
        sampleCount > 1 &&
        meanValue != null &&
        Number.isFinite(meanValue)
          ? 1 / (1 + Math.abs(base - meanValue) / scoreScale)
          : null

      const plateauScore = clamp01(
        0.30 * merit +
          0.16 * supportStrength +
          0.18 * scoreStability +
          0.14 * roughnessStability +
          0.12 * curvatureStability +
          0.10 * (binAgreement ?? 1),
      )
      out[idx] = plateauScore
      diagnostics[idx] = {
        plateauScore,
        plateauLabel: labelDerivedPlateau(
          plateauScore,
          supportiveNeighbors,
          availableNeighbors,
        ),
        supportiveNeighbors,
        availableNeighbors,
        sampleCount,
        meanValue,
        binAgreement,
      }
    }
  }

  return { values: out, diagnostics, multiSampleCellCount }
}

export function resolveHeatmapDisplayModel(options: {
  mode: HeatmapMode
  objectiveLabel: string
  model: HeatmapModel
  higherIsBetter: boolean
}): HeatmapDisplayModel {
  const { mode, objectiveLabel, model, higherIsBetter } = options
  if (mode === 'plateau') {
    const surface = buildDerivedPlateauSurface(model, { higherIsBetter })
    return {
      values: surface.values,
      range: { min: 0, max: 1, label: 'derived plateau score' },
      palette: heatmapPalette(mode),
      label: heatmapDisplayLabel(mode, objectiveLabel),
      legendNote: heatmapLegendNote(mode),
      plateauDiagnostics: surface.diagnostics,
      multiSampleCellCount: surface.multiSampleCellCount,
    }
  }

  return {
    values: model.values,
    range: resolveNullableHeatmapRange(model.values),
    palette: heatmapPalette(mode),
    label: heatmapDisplayLabel(mode, objectiveLabel),
    legendNote: heatmapLegendNote(mode),
    plateauDiagnostics: null,
    multiSampleCellCount: 0,
  }
}

export function fastForHeatmapCol(model: HeatmapModel, col: number): number {
  if (model.binsFast <= 1) return model.fastMin
  const boundedCol = Math.max(0, Math.min(model.binsFast - 1, Math.floor(col)))
  const span = Math.max(0, model.fastMax - model.fastMin)
  const offset = Math.ceil(
    (boundedCol * span) / Math.max(1, model.binsFast - 1),
  )
  return Math.min(model.fastMax, model.fastMin + offset)
}

export function slowForHeatmapLogicalRow(
  model: HeatmapModel,
  logicalRow: number,
): number {
  if (model.binsSlow <= 1) return model.slowMin
  const boundedRow = Math.max(
    0,
    Math.min(model.binsSlow - 1, Math.floor(logicalRow)),
  )
  const span = Math.max(0, model.slowMax - model.slowMin)
  const offset = Math.ceil(
    (boundedRow * span) / Math.max(1, model.binsSlow - 1),
  )
  return Math.min(model.slowMax, model.slowMin + offset)
}

export function heatmapColForFast(model: HeatmapModel, fast: number): number {
  if (model.binsFast <= 1 || model.fastMax <= model.fastMin) return 0
  const period = Number.isFinite(fast) ? Math.floor(fast) : model.fastMin
  const numerator = Math.max(0, period - model.fastMin) * (model.binsFast - 1)
  const denominator = model.fastMax - model.fastMin
  return Math.max(
    0,
    Math.min(model.binsFast - 1, Math.floor(numerator / denominator)),
  )
}

export function heatmapLogicalRowForSlow(model: HeatmapModel, slow: number): number {
  if (model.binsSlow <= 1 || model.slowMax <= model.slowMin) return 0
  const period = Number.isFinite(slow) ? Math.floor(slow) : model.slowMin
  const numerator = Math.max(0, period - model.slowMin) * (model.binsSlow - 1)
  const denominator = model.slowMax - model.slowMin
  return Math.max(
    0,
    Math.min(model.binsSlow - 1, Math.floor(numerator / denominator)),
  )
}
