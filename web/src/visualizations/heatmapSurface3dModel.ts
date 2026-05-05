import {
  fastForHeatmapCol,
  heatmapIndex,
  slowForHeatmapLogicalRow,
} from './heatmapModel'
import {
  sampleHeatmapColor,
  type HeatmapCanvasRenderModel,
  type HeatmapResolvedCell,
} from './heatmapCanvasModel'
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
  const finiteCells = displayValues.reduce<number>((count, value) => (finiteNumber(value) ? count + 1 : count), 0)
  const colStride = Math.max(1, Math.ceil(binsFast / Math.max(1, maxBins)))
  const rowStride = Math.max(1, Math.ceil(binsSlow / Math.max(1, maxBins)))
  const samples: HeatmapSurface3dSample[] = []
  const unavailableBlocks: HeatmapSurface3dUnavailableBlock[] = []

  for (let logicalRowStart = 0; logicalRowStart < binsSlow; logicalRowStart += rowStride) {
    const logicalRowEnd = Math.min(binsSlow, logicalRowStart + rowStride)
    for (let colStart = 0; colStart < binsFast; colStart += colStride) {
      const colEnd = Math.min(binsFast, colStart + colStride)
      const blockCells: Array<{ col: number; logicalRow: number; idx: number; value: number }> = []
      for (let logicalRow = logicalRowStart; logicalRow < logicalRowEnd; logicalRow += 1) {
        for (let col = colStart; col < colEnd; col += 1) {
          const idx = heatmapIndex(model, logicalRow, col)
          const value = displayValues[idx]
          if (!finiteNumber(value)) continue
          blockCells.push({ col, logicalRow, idx, value })
        }
      }
      if (!blockCells.length) {
        unavailableBlocks.push({ colStart, colEnd, logicalRowStart, logicalRowEnd })
        continue
      }

      const displayValue = blockCells.reduce((sum, cell) => sum + cell.value, 0) / blockCells.length
      const representative =
        blockCells.reduce((best, cell) =>
          Math.abs(cell.value - displayValue) < Math.abs(best.value - displayValue) ? cell : best,
        )
      const representativeCell: HeatmapResolvedCell = {
        col: representative.col,
        logicalRow: representative.logicalRow,
        drawRow: model.binsSlow - 1 - representative.logicalRow,
        idx: representative.idx,
      }

      samples.push({
        colStart,
        colEnd,
        logicalRowStart,
        logicalRowEnd,
        representativeCell,
        fast: fastForHeatmapCol(model, representative.col),
        slow: slowForHeatmapLogicalRow(model, representative.logicalRow),
        displayValue,
        normalizedValue: normalizedHeatmapValue(displayValue, range.min, range.max),
        color: sampleHeatmapColor(displayValue, range.min, range.max, palette),
        params: options.slice?.cells[representative.idx]?.params ?? null,
        metrics: options.slice?.cells[representative.idx]?.metrics ?? null,
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
