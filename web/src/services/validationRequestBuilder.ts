interface ValidationRowLike {
  params?: unknown
}

interface ValidationCrossDatasetInput {
  csv_path?: unknown
  label?: unknown
}

interface ValidationBuildInputs {
  currentRunReq?: unknown
  currentRunRes?: unknown
  topRows?: ValidationRowLike[]
  topRowsFiltered?: ValidationRowLike[]
  useFiltered?: boolean
  candidates?: unknown
  seed?: unknown
  minPsr?: unknown
  minDsr?: unknown
  wfEnable?: unknown
  wfWindow?: unknown
  wfStep?: unknown
  perturbEnable?: unknown
  perturbIters?: unknown
  perturbMin?: unknown
  perturbMax?: unknown
  mcEnable?: unknown
  mcSims?: unknown
  cscvEnable?: unknown
  cscvSlices?: unknown
  crossEnable?: unknown
  crossMax?: unknown
  crossDatasets?: ValidationCrossDatasetInput[]
}

interface ValidationCandidateLike {
  fast_ma_type: string
  slow_ma_type: string
  fast_len: number
  slow_len: number
  fast_ma_params?: unknown
  slow_ma_params?: unknown
  strategy?: unknown
  halftrend_filter?: unknown
  halftrend?: unknown
  macz?: unknown
  ott?: unknown
  signal_mode?: string
}

export interface ValidationRequestPayload {
  data_id: string
  ma_source: string
  fast_ma_params: unknown
  slow_ma_params: unknown
  candidates: ValidationCandidateLike[]
  walk_forward: {
    window_bars: number
    step_bars: number
  } | null
  perturb: {
    iterations: number
    pct_min: number
    pct_max: number
    flip_bool_prob: number
  } | null
  trade_bootstrap: {
    simulations: number
  } | null
  cross_ticker: {
    max_datasets: number
    datasets: Array<{ csv_path: string; label: string }>
  } | null
  cscv_pbo: {
    slices: number
    max_combinations: number
    threshold: number
  } | null
  seed: number
  max_candidates: number
  min_psr: number | null
  min_dsr: number | null
}

function num(value: unknown, fallback: number): number {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : fallback
}

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizeStrategyKindRaw(kindRaw: unknown): string {
  const kind = normalizeText(kindRaw || 'DoubleMa') || 'DoubleMa'
  if (kind === 'CompositeCpu') return 'CompositeCpu'
  if (kind === 'CompositeMo') return 'CompositeMo'
  if (kind === 'CompositeTrend') return 'CompositeTrend'
  if (kind === 'RegimeHybrid') return 'RegimeHybrid'
  if (kind === 'BreakoutMomentum') return 'BreakoutMomentum'
  return 'DoubleMa'
}

function kindFromReq(req: Record<string, unknown>): string {
  return normalizeStrategyKindRaw(req.strategy_kind ?? req.strategyKind ?? 'DoubleMa')
}

function isDoubleLikeKind(kind: string): boolean {
  const nextKind = normalizeText(kind)
  return nextKind === 'DoubleMa'
    || nextKind === 'CompositeTrend'
    || nextKind === 'RegimeHybrid'
    || nextKind === 'BreakoutMomentum'
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') return null
  return value as Record<string, unknown>
}

function parseProbabilityGate(value: unknown): number | null {
  const raw = normalizeText(value)
  if (!raw) return null
  const parsed = Number(raw)
  if (!Number.isFinite(parsed)) return null
  return Math.max(0, Math.min(1, parsed))
}

function normalizeCrossDatasets(
  value: unknown,
  maxDatasets: number,
): Array<{ csv_path: string; label: string }> {
  if (maxDatasets <= 0 || !Array.isArray(value)) return []
  const out: Array<{ csv_path: string; label: string }> = []
  for (const entry of value) {
    const row = asRecord(entry)
    if (!row) continue
    const csv_path = normalizeText(row.csv_path)
    if (!csv_path) continue
    out.push({
      csv_path,
      label: normalizeText(row.label),
    })
    if (out.length >= maxDatasets) break
  }
  return out
}

export function buildValidationRequest(inputs: ValidationBuildInputs): ValidationRequestPayload {
  const currentRunReq = asRecord(inputs.currentRunReq)
  if (!currentRunReq || !inputs.currentRunRes) throw new Error('Run an optimization first.')

  const kind = kindFromReq(currentRunReq)
  const dataId = normalizeText(currentRunReq.data_id)
  if (!dataId) throw new Error('Missing data id (run an optimization first).')

  const maxCandidates = Math.max(1, Math.floor(num(inputs.candidates, 32)))
  const seed = Math.max(0, Math.floor(num(inputs.seed, 1)))
  const min_psr = parseProbabilityGate(inputs.minPsr)
  const min_dsr = parseProbabilityGate(inputs.minDsr)

  const rowsFiltered = Array.isArray(inputs.topRowsFiltered) ? inputs.topRowsFiltered : []
  const rowsAll = Array.isArray(inputs.topRows) ? inputs.topRows : []
  const useFiltered = inputs.useFiltered !== false
  const baseRows = (useFiltered && rowsFiltered.length) ? rowsFiltered : rowsAll
  if (!baseRows.length) throw new Error('No Top rows available. Go to the Top tab and run an optimization.')

  const candidatesRaw = baseRows.slice(0, maxCandidates)
  const fastMaTypeFallback = Array.isArray(currentRunReq.fast_ma_types)
    ? normalizeText(currentRunReq.fast_ma_types[0])
    : ''
  const slowMaTypeFallback = Array.isArray(currentRunReq.slow_ma_types)
    ? normalizeText(currentRunReq.slow_ma_types[0])
    : ''

  const candidates = candidatesRaw
    .map((row): ValidationCandidateLike | null => {
      const rowRecord = asRecord(row)
      const p = asRecord(rowRecord?.params) || {}

      const fast_ma_type = normalizeText(p.fast_ma_type || fastMaTypeFallback)
      const slow_ma_type = normalizeText(p.slow_ma_type || slowMaTypeFallback)
      const fast_len = Math.max(0, Math.floor(Number(p.fast_len || 0)))
      const slow_len = Math.max(0, Math.floor(Number(p.slow_len || 0)))

      if (isDoubleLikeKind(kind)) {
        const fast_ma_params = asRecord(p.fast_ma_params)
        const slow_ma_params = asRecord(p.slow_ma_params)
        const strategy = currentRunReq.strategy || currentRunReq.base_strategy || {}
        const halftrend_filter = currentRunReq.halftrend_filter || currentRunReq.halftrendFilter || null
        return {
          fast_ma_type,
          slow_ma_type,
          fast_len,
          slow_len,
          fast_ma_params: fast_ma_params || undefined,
          slow_ma_params: slow_ma_params || undefined,
          strategy,
          halftrend_filter: halftrend_filter || undefined,
        }
      }

      const halftrend = kind === 'CompositeMo'
        ? (asRecord(p.halftrend) || null)
        : (asRecord(currentRunReq.halftrend) || null)
      const signal_mode = kind === 'CompositeMo'
        ? normalizeText(p.signal_mode || p.signalMode || currentRunReq.signal_mode || currentRunReq.signalMode || 'hard_gate')
        : normalizeText(currentRunReq.signal_mode || currentRunReq.signalMode || 'hard_gate')

      return {
        fast_ma_type,
        slow_ma_type,
        fast_len,
        slow_len,
        halftrend: halftrend || undefined,
        macz: kind === 'CompositeMo' ? (p.macz || {}) : (currentRunReq.macz || {}),
        ott: kind === 'CompositeMo' ? (p.ott || {}) : (currentRunReq.ott || {}),
        signal_mode,
        strategy: kind === 'CompositeMo'
          ? (p.strategy || currentRunReq.strategy || {})
          : (currentRunReq.strategy || {}),
      }
    })
    .filter((candidate): candidate is ValidationCandidateLike => {
      if (!candidate) return false
      if (!candidate.fast_ma_type || !candidate.slow_ma_type) return false
      if (candidate.fast_len <= 0 || candidate.slow_len <= 0) return false
      if (isDoubleLikeKind(kind)) return true
      return !!candidate.halftrend
    })

  if (!candidates.length) throw new Error('No valid candidates found (try lowering filters or increasing Candidates).')

  const walk_forward = inputs.wfEnable
    ? {
      window_bars: Math.max(64, Math.floor(num(inputs.wfWindow, 20000))),
      step_bars: Math.max(1, Math.floor(num(inputs.wfStep, 20000))),
    }
    : null

  const perturb = inputs.perturbEnable
    ? {
      iterations: Math.max(0, Math.floor(num(inputs.perturbIters, 8))),
      pct_min: Math.max(0, num(inputs.perturbMin, 0.015)),
      pct_max: Math.max(0, num(inputs.perturbMax, 0.06)),
      flip_bool_prob: 0.02,
    }
    : null

  const trade_bootstrap = inputs.mcEnable
    ? { simulations: Math.max(0, Math.floor(num(inputs.mcSims, 500))) }
    : null

  const cscv_pbo = inputs.cscvEnable
    ? (() => {
      let slices = Math.max(4, Math.floor(num(inputs.cscvSlices, 16)))
      if ((slices & 1) === 1) slices = Math.max(4, slices - 1)
      return { slices, max_combinations: 50000, threshold: 0.0 }
    })()
    : null

  const cross_ticker = inputs.crossEnable
    ? (() => {
      const max_datasets = Math.max(0, Math.floor(num(inputs.crossMax, 6)))
      if (max_datasets <= 0) return null
      const datasets = normalizeCrossDatasets(inputs.crossDatasets, max_datasets)
      if (!datasets.length) return null
      return {
        max_datasets,
        datasets,
      }
    })()
    : null

  return {
    data_id: dataId,
    ma_source: normalizeText(currentRunReq.ma_source || 'close') || 'close',
    fast_ma_params: currentRunReq.fast_ma_params || null,
    slow_ma_params: currentRunReq.slow_ma_params || null,
    candidates,
    walk_forward,
    perturb,
    trade_bootstrap,
    cross_ticker,
    cscv_pbo,
    seed,
    max_candidates: maxCandidates,
    min_psr,
    min_dsr,
  }
}
