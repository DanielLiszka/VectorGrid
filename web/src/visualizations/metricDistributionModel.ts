import {
  metricColumnVisualRole,
  parseMetricCellNumber,
  type MetricVisualColumn,
  type MetricVisualRow,
} from './metricCellVisualModel'

export interface MetricDistributionOption {
  key: string
  label: string
}

export interface MetricDistributionBin {
  label: string
  count: number
  min: number
  max: number
}

export interface MetricDistributionModel {
  key: string
  label: string
  values: number[]
  min: number
  max: number
  mean: number
  median: number
  bins: MetricDistributionBin[]
}

function normalizedKey(value: unknown): string {
  return String(value || '').trim().toLowerCase()
}

function columnKey(column: MetricVisualColumn, index: number): string {
  return normalizedKey(column.key) || `col_${index}`
}

function sourceIndexFor(column: MetricVisualColumn, index: number): number {
  const source = Number(column.sourceColumnIndex)
  return Number.isInteger(source) && source >= 0 ? source : index
}

function formatNumber(value: number): string {
  if (!Number.isFinite(value)) return '0'
  const abs = Math.abs(value)
  if (abs >= 100) return value.toFixed(0)
  if (abs >= 10) return value.toFixed(1)
  return value.toFixed(3).replace(/\.?0+$/, '')
}

export function metricDistributionOptions(
  columns: readonly MetricVisualColumn[],
  rows: readonly MetricVisualRow[],
): MetricDistributionOption[] {
  const eligible: Array<{
    key: string
    label: string
    sourceIndex: number
    hasValues: boolean
  }> = []
  for (let index = 0; index < columns.length; index += 1) {
    const column = columns[index]
    if (!column) continue
    const key = columnKey(column, index)
    const role = metricColumnVisualRole(key)
    if (!role || role === 'status') continue
    eligible.push({
      key,
      label: String(column.label || key).trim() || key,
      sourceIndex: sourceIndexFor(column, index),
      hasValues: false,
    })
  }
  if (!eligible.length || !rows.length) return []

  let resolvedColumns = 0
  for (const row of rows) {
    for (const column of eligible) {
      if (column.hasValues) continue
      if (parseMetricCellNumber(row.cells[column.sourceIndex]) == null) continue
      column.hasValues = true
      resolvedColumns += 1
    }
    if (resolvedColumns === eligible.length) break
  }

  const options: MetricDistributionOption[] = []
  for (const column of eligible) {
    if (!column.hasValues) continue
    options.push({ key: column.key, label: column.label })
  }
  return options
}

export function chooseMetricDistributionKey(
  options: readonly MetricDistributionOption[],
  preferredKeys: readonly string[],
  currentKey?: string,
): string {
  const optionKeys = new Set(options.map((option) => option.key))
  const normalizedCurrent = normalizedKey(currentKey)
  if (normalizedCurrent && optionKeys.has(normalizedCurrent)) return normalizedCurrent
  for (const key of preferredKeys.map(normalizedKey)) {
    if (optionKeys.has(key)) return key
  }
  return options[0]?.key ?? ''
}

export function buildMetricDistributionModel(options: {
  columns: readonly MetricVisualColumn[]
  rows: readonly MetricVisualRow[]
  metricKey: string
  binCount?: number
}): MetricDistributionModel | null {
  const metricKey = normalizedKey(options.metricKey)
  if (!metricKey) return null
  const columnIndex = options.columns.findIndex((column, index) => columnKey(column, index) === metricKey)
  if (columnIndex < 0) return null
  const column = options.columns[columnIndex]
  if (!column) return null
  const sourceIndex = sourceIndexFor(column, columnIndex)
  const values: number[] = []
  let sum = 0
  for (const row of options.rows) {
    const value = parseMetricCellNumber(row.cells[sourceIndex])
    if (value == null) continue
    values.push(value)
    sum += value
  }
  values.sort((left, right) => left - right)
  if (!values.length) return null

  const min = values[0] ?? 0
  const max = values[values.length - 1] ?? min
  const mean = sum / values.length
  const mid = Math.floor(values.length / 2)
  const median = values.length % 2 === 0
    ? ((values[mid - 1] ?? 0) + (values[mid] ?? 0)) / 2
    : values[mid] ?? 0
  const requestedBins = Math.max(4, Math.min(16, Math.floor(Number(options.binCount || 10))))
  const binCount = min === max ? 1 : requestedBins
  const span = max - min
  const bins = Array.from({ length: binCount }, (_, index) => {
    const binMin = min + (span * index) / binCount
    const binMax = index === binCount - 1 ? max : min + (span * (index + 1)) / binCount
    return {
      label: binCount === 1 ? formatNumber(min) : `${formatNumber(binMin)}-${formatNumber(binMax)}`,
      count: 0,
      min: binMin,
      max: binMax,
    }
  })

  for (const value of values) {
    const index = span === 0 ? 0 : Math.min(binCount - 1, Math.floor(((value - min) / span) * binCount))
    const bin = bins[index]
    if (bin) bin.count += 1
  }

  return {
    key: metricKey,
    label: String(column.label || metricKey).trim() || metricKey,
    values,
    min,
    max,
    mean,
    median,
    bins,
  }
}
