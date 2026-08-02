import {
  fastForHeatmapCol,
  heatmapIndex,
  slowForHeatmapLogicalRow,
} from './heatmapModel'
import {
  type HeatmapCanvasRenderModel,
  type HeatmapResolvedCell,
} from './heatmapCanvasModel'
import type { LightweightChartTheme } from './chartTheme'
import { sampleHeatmapColor } from './heatmapTheme'
import type { Heatmap3dSlice } from './heatmapSliceModel'

export const HEATMAP_SURFACE_3D_MAX_BINS = 96

export interface HeatmapSurface3dSample {
  colStart: number
  colEnd: number
  logicalRowStart: number
  logicalRowEnd: number
  representativeCell: HeatmapResolvedCell
  fast: number
  slow: number
  displayValue: number
  finiteCellCount: number
  blockCellCount: number
  normalizedValue: number
  color: string
  params: Record<string, unknown> | null
  metrics: Record<string, unknown> | null
}

export interface HeatmapSurface3dUnavailableBlock {
  colStart: number
  colEnd: number
  logicalRowStart: number
  logicalRowEnd: number
}

export interface HeatmapSurface3dModel {
  samples: HeatmapSurface3dSample[]
  unavailableBlocks: HeatmapSurface3dUnavailableBlock[]
  totalCells: number
  unavailableCells: number
  finiteCells: number
  colStride: number
  rowStride: number
  sampledBinsFast: number
  sampledBinsSlow: number
}

export interface HeatmapSurface3dBuildOptions {
  maxBins?: number
  slice?: Heatmap3dSlice | null
  theme?: LightweightChartTheme
}

export function resolveHeatmapSurface3dKeyboardIndex(
  samples: readonly HeatmapSurface3dSample[],
  currentIndex: number,
  key: string,
): number | null {
  if (!samples.length) return null
  if (key === 'Home') return 0
  if (key === 'End') return samples.length - 1
  if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(key)) return null

  const index = Math.min(samples.length - 1, Math.max(0, currentIndex))
  const current = samples[index]
  if (!current) return null
  const horizontal = key === 'ArrowLeft' || key === 'ArrowRight'
  const forward = key === 'ArrowRight' || key === 'ArrowDown'
  let bestIndex = index
  let bestScore = Infinity

  for (let candidateIndex = 0; candidateIndex < samples.length; candidateIndex += 1) {
    const candidate = samples[candidateIndex]
    if (!candidate || candidateIndex === index) continue
    const primaryDelta = horizontal
      ? candidate.colStart - current.colStart
      : candidate.logicalRowStart - current.logicalRowStart
    if ((forward && primaryDelta <= 0) || (!forward && primaryDelta >= 0)) continue
    const secondaryDelta = horizontal
      ? Math.abs(candidate.logicalRowStart - current.logicalRowStart)
      : Math.abs(candidate.colStart - current.colStart)
    const score = Math.abs(primaryDelta) + secondaryDelta * 10_000
    if (score < bestScore) {
      bestScore = score
      bestIndex = candidateIndex
    }
  }
  return bestIndex
}

function finiteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value)
}

function normalizedHeatmapValue(value: number, minValue: number, maxValue: number): number {
  const denominator = Math.max(1.0e-12, maxValue - minValue)
  return Math.max(0, Math.min(1, (value - minValue) / denominator))
}

export function buildHeatmapSurface3dModel(
  renderModel: HeatmapCanvasRenderModel,
  options: HeatmapSurface3dBuildOptions = {},
): HeatmapSurface3dModel {
  const { model } = renderModel
  const displayValues = options.slice?.values ?? renderModel.displayValues
  const range = options.slice ? (renderModel.slice3d?.range ?? renderModel.range) : renderModel.range
  const palette = options.slice ? (renderModel.slice3d?.palette ?? renderModel.palette) : renderModel.palette
  const maxBins = options.maxBins ?? HEATMAP_SURFACE_3D_MAX_BINS
  const binsFast = Math.max(0, model.binsFast)
  const binsSlow = Math.max(0, model.binsSlow)
  const totalCells = binsFast * binsSlow
  const colStride = Math.max(1, Math.ceil(binsFast / Math.max(1, maxBins)))
  const rowStride = Math.max(1, Math.ceil(binsSlow / Math.max(1, maxBins)))
  const samples: HeatmapSurface3dSample[] = []
  const unavailableBlocks: HeatmapSurface3dUnavailableBlock[] = []
  let finiteCells = 0

  for (let logicalRowStart = 0; logicalRowStart < binsSlow; logicalRowStart += rowStride) {
    const logicalRowEnd = Math.min(binsSlow, logicalRowStart + rowStride)
    for (let colStart = 0; colStart < binsFast; colStart += colStride) {
      const colEnd = Math.min(binsFast, colStart + colStride)
      const blockCellCount =
        (logicalRowEnd - logicalRowStart) * (colEnd - colStart)
      let blockCount = 0
      let blockSum = 0
      for (let logicalRow = logicalRowStart; logicalRow < logicalRowEnd; logicalRow += 1) {
        for (let col = colStart; col < colEnd; col += 1) {
          const idx = heatmapIndex(model, logicalRow, col)
          const value = displayValues[idx]
          if (!finiteNumber(value)) continue
          blockCount += 1
          blockSum += value
        }
      }
      finiteCells += blockCount
      if (!blockCount) {
        unavailableBlocks.push({ colStart, colEnd, logicalRowStart, logicalRowEnd })
        continue
      }

      const displayValue = blockSum / blockCount
      let representativeCol = colStart
      let representativeLogicalRow = logicalRowStart
      let representativeIdx = -1
      let representativeDistance = Infinity
      for (let logicalRow = logicalRowStart; logicalRow < logicalRowEnd; logicalRow += 1) {
        for (let col = colStart; col < colEnd; col += 1) {
          const idx = heatmapIndex(model, logicalRow, col)
          const value = displayValues[idx]
          if (!finiteNumber(value)) continue
          const distance = Math.abs(value - displayValue)
          if (representativeIdx >= 0 && distance >= representativeDistance) continue
          representativeCol = col
          representativeLogicalRow = logicalRow
          representativeIdx = idx
          representativeDistance = distance
        }
      }
      const representativeCell: HeatmapResolvedCell = {
        col: representativeCol,
        logicalRow: representativeLogicalRow,
        drawRow: model.binsSlow - 1 - representativeLogicalRow,
        idx: representativeIdx,
      }

      samples.push({
        colStart,
        colEnd,
        logicalRowStart,
        logicalRowEnd,
        representativeCell,
        fast: fastForHeatmapCol(model, representativeCol),
        slow: slowForHeatmapLogicalRow(model, representativeLogicalRow),
        displayValue,
        finiteCellCount: blockCount,
        blockCellCount,
        normalizedValue: normalizedHeatmapValue(displayValue, range.min, range.max),
        color: sampleHeatmapColor(
          displayValue,
          range.min,
          range.max,
          palette,
          options.theme,
        ),
        params: options.slice?.cells[representativeIdx]?.params ?? null,
        metrics: options.slice?.cells[representativeIdx]?.metrics ?? null,
      })
    }
  }

  return {
    samples,
    unavailableBlocks,
    totalCells,
    unavailableCells: Math.max(0, totalCells - finiteCells),
    finiteCells,
    colStride,
    rowStride,
    sampledBinsFast: Math.ceil(binsFast / colStride),
    sampledBinsSlow: Math.ceil(binsSlow / rowStride),
  }
}
