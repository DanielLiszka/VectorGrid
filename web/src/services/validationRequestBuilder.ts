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
  runDatasetMeta?: unknown
  runCsvPath?: unknown
  topRows?: ValidationRowLike[]
  topRowsFiltered?: ValidationRowLike[]
  useFiltered?: boolean
  candidates?: unknown
  seed?: unknown
  minPsr?: unknown
  minDsr?: unknown
  minCscvOosRankFrac?: unknown
  minCscvOosMetric?: unknown
  maxPbo?: unknown
  spaEnable?: unknown
  spaSlices?: unknown
  spaBootstrapSamples?: unknown
  spaBlockLength?: unknown
  reoptWfEnable?: unknown
  reoptWfWindow?: unknown
  reoptWfStep?: unknown
  reoptWfTopN?: unknown
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
  asr?: unknown
  vcc?: unknown
  vemr?: unknown
  rocb?: unknown
  mkcs?: unknown
  strategy?: unknown
  halftrend_filter?: unknown
  halftrend?: unknown
  macz?: unknown
  ott?: unknown
  signal_mode?: string
}

export interface ValidationRequestPayload {
  data_id: string
  backend: {
    backend: 'Auto' | 'CpuOnly' | 'GpuOnly' | 'GpuHybridKernel' | 'GpuKernel'
    device_id?: number
  }
  ma_source: string
  fast_ma_params: unknown
  slow_ma_params: unknown
  candidates: ValidationCandidateLike[]
  walk_forward: {
    window_bars: number
    step_bars: number
  } | null
  reopt_walk_forward: {
    window_bars: number
    step_bars: number
    top_n_candidates: number
  } | null
  spa: {
    slices: number
    bootstrap_samples: number
    block_length: number
    benchmark: 'buy_hold'
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
  min_cscv_oos_rank_frac: number | null
  min_cscv_oos_metric: number | null
  max_pbo: number | null
  validation_context: {
    strategy_kind: string
    csv_path: string | null
    timeframe: string | null
    label: string | null
  }
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
  if (kind === 'Pit') return 'Pit'
  if (kind === 'Set') return 'Set'
  if (kind === 'Mts') return 'Mts'
  if (kind === 'Orc') return 'Orc'
  if (kind === 'Marrt') return 'Marrt'
  if (kind === 'Vfct') return 'Vfct'
  if (kind === 'Rocb') return 'Rocb'
  if (kind === 'Asr') return 'Asr'
  if (kind === 'Vcc') return 'Vcc'
  if (kind === 'Vemr') return 'Vemr'
  if (kind === 'Mkcs') return 'Mkcs'
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
  return nextKind === 'DoubleMa' || nextKind === 'Pit' || nextKind === 'Set' || nextKind === 'Mts' || nextKind === 'Orc' || nextKind === 'Marrt' || nextKind === 'Vfct' || nextKind === 'CompositeTrend' || nextKind === 'RegimeHybrid' || nextKind === 'BreakoutMomentum'
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

function parseFiniteNumber(value: unknown): number | null {
  const raw = normalizeText(value)
  if (!raw) return null
  const parsed = Number(raw)
  return Number.isFinite(parsed) ? parsed : null
}

function normalizeCrossDatasets(value: unknown, maxDatasets: number): Array<{ csv_path: string; label: string }> {
  if (maxDatasets <= 0 || !Array.isArray(value)) return []
  const out: Array<{ csv_path: string; label: string }> = []
  for (const entry of value) {
    const row = asRecord(entry)
    if (!row) continue
    const csv_path = normalizeText(row.csv_path)
    if (!csv_path) continue
    out.push({
      csv_path,
      label: normalizeText(row.label)
    })
    if (out.length >= maxDatasets) break
  }
  return out
}

function firstNonEmptyText(...values: unknown[]): string {
  for (const value of values) {
    const next = normalizeText(value)
    if (next) return next
  }
  return ''
}

function normalizeValidationBackend(value: unknown): ValidationRequestPayload['backend'] {
  const row = asRecord(value)
  const backend = normalizeText(row?.backend ?? value)
  const deviceId = Math.max(0, Math.floor(num(row?.device_id, 0)))
  if (backend === 'Auto') return { backend: 'Auto', device_id: deviceId }
  if (backend === 'GpuOnly') return { backend: 'GpuOnly', device_id: deviceId }
  if (backend === 'GpuHybridKernel') return { backend: 'GpuHybridKernel', device_id: deviceId }
  if (backend === 'GpuKernel') return { backend: 'GpuKernel', device_id: deviceId }
  return { backend: 'CpuOnly' }
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
  const min_cscv_oos_rank_frac = parseProbabilityGate(inputs.minCscvOosRankFrac)
  const min_cscv_oos_metric = parseFiniteNumber(inputs.minCscvOosMetric)
  const max_pbo = parseProbabilityGate(inputs.maxPbo)
  const datasetMeta = asRecord(inputs.runDatasetMeta)
  const validation_context = {
    strategy_kind: kind,
    csv_path: firstNonEmptyText(datasetMeta?.csv_path, inputs.runCsvPath) || null,
    timeframe: firstNonEmptyText(datasetMeta?.timeframe, datasetMeta?.derived_timeframe, datasetMeta?.base_timeframe) || null,
    label: firstNonEmptyText(datasetMeta?.symbol, datasetMeta?.label, inputs.runCsvPath) || null,
  }

  const rowsFiltered = Array.isArray(inputs.topRowsFiltered) ? inputs.topRowsFiltered : []
  const rowsAll = Array.isArray(inputs.topRows) ? inputs.topRows : []
  const useFiltered = inputs.useFiltered !== false
  const baseRows = useFiltered && rowsFiltered.length ? rowsFiltered : rowsAll
  if (!baseRows.length) throw new Error('No Top rows available. Go to the Top tab and run an optimization.')

  const candidatesRaw = baseRows.slice(0, maxCandidates)
  const fastMaTypeFallback = Array.isArray(currentRunReq.fast_ma_types) ? normalizeText(currentRunReq.fast_ma_types[0]) : ''
  const slowMaTypeFallback = Array.isArray(currentRunReq.slow_ma_types) ? normalizeText(currentRunReq.slow_ma_types[0]) : ''

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
          halftrend_filter: halftrend_filter || undefined
        }
      }

      if (kind === 'Asr') {
        const strategy = currentRunReq.strategy || currentRunReq.base_strategy || {}
        const asr = asRecord(p.asr) || asRecord(currentRunReq.asr) || {}
        return {
          fast_ma_type,
          slow_ma_type,
          fast_len,
          slow_len,
          fast_ma_params: asRecord(p.fast_ma_params) || undefined,
          slow_ma_params: asRecord(p.slow_ma_params) || undefined,
          asr,
          strategy
        }
      }

      if (kind === 'Vcc') {
        const strategy = currentRunReq.strategy || currentRunReq.base_strategy || {}
        const vcc = asRecord(p.vcc) || asRecord(currentRunReq.vcc) || {}
        return {
          fast_ma_type,
          slow_ma_type,
          fast_len,
          slow_len,
          fast_ma_params: asRecord(p.fast_ma_params) || undefined,
          slow_ma_params: asRecord(p.slow_ma_params) || undefined,
          vcc,
          strategy
        }
      }

      if (kind === 'Vemr') {
        const strategy = currentRunReq.strategy || currentRunReq.base_strategy || {}
        const vemr = asRecord(p.vemr) || asRecord(currentRunReq.vemr) || {}
        return {
          fast_ma_type,
          slow_ma_type,
          fast_len,
          slow_len,
          fast_ma_params: asRecord(p.fast_ma_params) || undefined,
          slow_ma_params: asRecord(p.slow_ma_params) || undefined,
          vemr,
          strategy
        }
      }

      if (kind === 'Rocb') {
        const strategy = currentRunReq.strategy || currentRunReq.base_strategy || {}
        const rocb = asRecord(p.rocb) || asRecord(currentRunReq.rocb) || {}
        return {
          fast_ma_type,
          slow_ma_type,
          fast_len,
          slow_len,
          fast_ma_params: asRecord(p.fast_ma_params) || undefined,
          slow_ma_params: asRecord(p.slow_ma_params) || undefined,
          rocb,
          strategy
        }
      }

      if (kind === 'Mkcs') {
        const strategy = currentRunReq.strategy || currentRunReq.base_strategy || {}
        const mkcs = asRecord(p.mkcs) || asRecord(currentRunReq.mkcs) || {}
        return {
          fast_ma_type,
          slow_ma_type,
          fast_len,
          slow_len,
          fast_ma_params: asRecord(p.fast_ma_params) || undefined,
          slow_ma_params: asRecord(p.slow_ma_params) || undefined,
          mkcs,
          strategy
        }
      }

      const halftrend = kind === 'CompositeMo' ? asRecord(p.halftrend) || null : asRecord(currentRunReq.halftrend) || null
      const signal_mode = kind === 'CompositeMo' ? normalizeText(p.signal_mode || p.signalMode || currentRunReq.signal_mode || currentRunReq.signalMode || 'hard_gate') : normalizeText(currentRunReq.signal_mode || currentRunReq.signalMode || 'hard_gate')

      return {
        fast_ma_type,
        slow_ma_type,
        fast_len,
        slow_len,
        halftrend: halftrend || undefined,
        macz: kind === 'CompositeMo' ? p.macz || {} : currentRunReq.macz || {},
        ott: kind === 'CompositeMo' ? p.ott || {} : currentRunReq.ott || {},
        signal_mode,
        strategy: kind === 'CompositeMo' ? p.strategy || currentRunReq.strategy || {} : currentRunReq.strategy || {}
      }
    })
    .filter((candidate): candidate is ValidationCandidateLike => {
      if (!candidate) return false
      if (!candidate.fast_ma_type || !candidate.slow_ma_type) return false
      if (candidate.fast_len <= 0 || candidate.slow_len <= 0) return false
      if (isDoubleLikeKind(kind)) return true
      if (kind === 'Asr') return true
      if (kind === 'Vcc') return true
      if (kind === 'Vemr') return true
      if (kind === 'Rocb') return true
      if (kind === 'Mkcs') return true
      return !!candidate.halftrend
    })

  if (!candidates.length) throw new Error('No valid candidates found (try lowering filters or increasing Candidates).')

  const walk_forward = inputs.wfEnable
    ? {
        window_bars: Math.max(64, Math.floor(num(inputs.wfWindow, 20000))),
        step_bars: Math.max(1, Math.floor(num(inputs.wfStep, 20000)))
      }
    : null

  const reopt_walk_forward = inputs.reoptWfEnable
    ? {
        window_bars: Math.max(64, Math.floor(num(inputs.reoptWfWindow, 20000))),
        step_bars: Math.max(1, Math.floor(num(inputs.reoptWfStep, 20000))),
        top_n_candidates: Math.max(1, Math.floor(num(inputs.reoptWfTopN, 8))),
      }
    : null

  const spa = inputs.spaEnable
    ? {
        slices: Math.max(4, Math.floor(num(inputs.spaSlices, 16))),
        bootstrap_samples: Math.max(100, Math.floor(num(inputs.spaBootstrapSamples, 500))),
        block_length: Math.max(1, Math.floor(num(inputs.spaBlockLength, 4))),
        benchmark: 'buy_hold' as const
      }
    : null

  const perturb = inputs.perturbEnable
    ? {
        iterations: Math.max(0, Math.floor(num(inputs.perturbIters, 8))),
        pct_min: Math.max(0, num(inputs.perturbMin, 0.015)),
        pct_max: Math.max(0, num(inputs.perturbMax, 0.06)),
        flip_bool_prob: 0.02
      }
    : null

  const trade_bootstrap = inputs.mcEnable ? { simulations: Math.max(0, Math.floor(num(inputs.mcSims, 500))) } : null

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
          datasets
        }
      })()
    : null

  return {
    data_id: dataId,
    backend: normalizeValidationBackend(currentRunReq.backend),
    ma_source: normalizeText(currentRunReq.ma_source || 'close') || 'close',
    fast_ma_params: currentRunReq.fast_ma_params || null,
    slow_ma_params: currentRunReq.slow_ma_params || null,
    candidates,
    walk_forward,
    reopt_walk_forward,
    spa,
    perturb,
    trade_bootstrap,
    cross_ticker,
    cscv_pbo,
    seed,
    max_candidates: maxCandidates,
    min_psr,
    min_dsr,
    min_cscv_oos_rank_frac,
    min_cscv_oos_metric,
    max_pbo,
    validation_context
  }
}
