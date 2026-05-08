export interface MetricVisualColumn {
  key?: string
  label?: string
  sourceColumnIndex?: number
}

export interface MetricVisualCell {
  text?: string
  title?: string
  className?: string
}

export interface MetricVisualRow {
  cells: MetricVisualCell[]
}

export type MetricVisualRole = 'higher_better' | 'lower_better' | 'neutral' | 'status'
export type MetricVisualTone = 'good' | 'risk' | 'warn' | 'neutral' | 'accent'
export type MetricVisualKind = 'bar' | 'badge'

export interface MetricColumnVisualRange {
  key: string
  role: Exclude<MetricVisualRole, 'status'>
  min: number
  max: number
}

export interface MetricCellVisual {
  kind: MetricVisualKind
  tone: MetricVisualTone
  fillPct: number
  label: string
}

const HIGHER_BETTER_KEYS = new Set([
  'pnl',
  'ret_pct',
  'sharpe',
  'trades',
  'score',
  'robust',
  'psr',
  'dsr',
  'cross',
  'maximize_value',
  'active_mean',
  'active_sortino',
  'robustness_margin',
  'omega',
  'calmar',
  'sortino',
])

const LOWER_BETTER_KEYS = new Set([
  'max_dd',
  'violation',
  'minimize_value',
  'cscv_rank',
  'drawdown_chronicity',
  'active_downside_deviation',
  'downside_share',
  'pbo',
])

const NEUTRAL_KEYS = new Set([
  'exposure',
  'long_exposure',
  'short_exposure',
  'directional_bias',
])

const STATUS_KEYS = new Set(['gate', 'robust_pareto', 'cross_pareto'])

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

export function metricColumnVisualRole(key: unknown): MetricVisualRole | null {
  const normalized = normalizedKey(key)
  if (!normalized || normalized === 'rank' || normalized === 'fast' || normalized === 'slow') return null
  if (STATUS_KEYS.has(normalized)) return 'status'
  if (LOWER_BETTER_KEYS.has(normalized)) return 'lower_better'
  if (HIGHER_BETTER_KEYS.has(normalized)) return 'higher_better'
  if (NEUTRAL_KEYS.has(normalized)) return 'neutral'
  return null
}

export function parseMetricCellNumber(cell: MetricVisualCell | null | undefined): number | null {
  const raw = String(cell?.title || cell?.text || '').trim()
  if (!raw || raw === '-' || raw === '—') return null
  const parenthesized = /^\(.*\)$/u.test(raw)
  const cleaned = raw
    .replace(/[,$%]/g, '')
    .replace(/[()]/g, '')
    .replace(/\s+/g, '')
  const value = Number(cleaned)
  if (!Number.isFinite(value)) return null
  return parenthesized && value > 0 ? -value : value
}

export function buildMetricColumnVisualRanges(
  columns: readonly MetricVisualColumn[],
  rows: readonly MetricVisualRow[],
): Record<string, MetricColumnVisualRange> {
  const ranges: Record<string, MetricColumnVisualRange> = {}
  const eligible: Array<{
    key: string
    role: Exclude<MetricVisualRole, 'status'>
    sourceIndex: number
    min: number
    max: number
    seen: boolean
  }> = []
  columns.forEach((column, index) => {
    const key = columnKey(column, index)
    const role = metricColumnVisualRole(key)
    if (!role || role === 'status') return
    eligible.push({
      key,
      role,
      sourceIndex: sourceIndexFor(column, index),
      min: Number.POSITIVE_INFINITY,
      max: Number.NEGATIVE_INFINITY,
      seen: false,
    })
  })

  for (const row of rows) {
    for (const column of eligible) {
      const value = parseMetricCellNumber(row.cells[column.sourceIndex])
      if (value == null) continue
      column.seen = true
      if (value < column.min) column.min = value
      if (value > column.max) column.max = value
    }
  }

  for (const column of eligible) {
    if (!column.seen) continue
    ranges[column.key] = {
      key: column.key,
      role: column.role,
      min: column.min,
      max: column.max,
    }
  }
  return ranges
}

function clampPct(value: number): number {
  if (!Number.isFinite(value)) return 0
  return Math.max(0, Math.min(100, Math.round(value)))
}

function statusTone(text: string): MetricVisualTone {
  const normalized = text.toLowerCase()
  if (!normalized || normalized === '-' || normalized === '—') return 'neutral'
  if (normalized.includes('fail') || normalized.includes('reject')) return 'risk'
  if (normalized.includes('*') || normalized.includes('warn')) return 'warn'
  if (normalized.includes('pass') || normalized.includes('robust') || normalized.includes('cross')) return 'good'
  return 'accent'
}

function valueTone(role: Exclude<MetricVisualRole, 'status'>, value: number, range: MetricColumnVisualRange): MetricVisualTone {
  if (role === 'neutral') return 'neutral'
  if (range.max === range.min) return 'accent'
  const midpoint = range.min + (range.max - range.min) / 2
  if (role === 'higher_better') return value >= midpoint ? 'good' : 'warn'
  return value <= midpoint ? 'good' : 'risk'
}

export function resolveMetricCellVisual(
  columnKeyRaw: unknown,
  cell: MetricVisualCell | null | undefined,
  ranges: Record<string, MetricColumnVisualRange>,
): MetricCellVisual | null {
  const key = normalizedKey(columnKeyRaw)
  const role = metricColumnVisualRole(key)
  const text = String(cell?.text || '').trim()
  if (!role || !text) return null
  if (role === 'status') {
    return {
      kind: 'badge',
      tone: statusTone(text),
      fillPct: 0,
      label: `${key} ${text}`,
    }
  }

  const value = parseMetricCellNumber(cell)
  const range = ranges[key]
  if (value == null || !range) return null
  const span = range.max - range.min
  const normalized = span === 0 ? 1 : (value - range.min) / span
  const fillPct = role === 'lower_better' ? (1 - normalized) * 100 : normalized * 100
  return {
    kind: 'bar',
    tone: valueTone(role, value, range),
    fillPct: clampPct(fillPct),
    label: `${key} ${text}`,
  }
}
