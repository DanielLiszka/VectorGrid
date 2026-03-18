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
  mcBlockLen?: unknown
  mcMode?: unknown
  cscvEnable?: unknown
  cscvSlices?: unknown
  crossEnable?: unknown
  crossMax?: unknown
  crossMode?: unknown
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
  kqsr?: unknown
  carm?: unknown
  kvdf?: unknown
  cdmc?: unknown
  vcet?: unknown
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
    stationary_mean_block_len?: number
  } | null
  cross_ticker: {
    max_datasets: number
    mode: 'lightweight' | 'preserved_finalists'
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

export interface ValidationCapabilitySummary {
  routeSummary: string
  crossSummary: string
  moduleSummary: string
  categorySummary: string
  skippedSummary: string
  warningSummary: string
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
  if (kind === 'Vcet') return 'Vcet'
  if (kind === 'Asr') return 'Asr'
  if (kind === 'Vcc') return 'Vcc'
  if (kind === 'Vemr') return 'Vemr'
  if (kind === 'Mkcs') return 'Mkcs'
  if (kind === 'Kqsr') return 'Kqsr'
  if (kind === 'Carm') return 'Carm'
  if (kind === 'Kvdf') return 'Kvdf'
  if (kind === 'Cdmc') return 'Cdmc'
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

function normalizeCrossTickerMode(value: unknown, strategyKind: string): 'lightweight' | 'preserved_finalists' {
  const raw = normalizeText(value).toLowerCase()
  if (raw === 'lightweight') return 'lightweight'
  if (raw === 'preserved_finalists' || raw === 'preserved-finalists') return 'preserved_finalists'
  return isDoubleLikeKind(strategyKind) ? 'preserved_finalists' : 'lightweight'
}

function firstNonEmptyText(...values: unknown[]): string {
  for (const value of values) {
    const next = normalizeText(value)
    if (next) return next
  }
  return ''
}

function normalizeValidationBackend(
  value: unknown,
  strategyKind: string,
): ValidationRequestPayload['backend'] {
  const row = asRecord(value)
  const backend = normalizeText(row?.backend ?? value)
  const deviceId = Math.max(0, Math.floor(num(row?.device_id, 0)))
  if (!isDoubleLikeKind(strategyKind) && backend !== 'CpuOnly') {
    return { backend: 'Auto', device_id: deviceId }
  }
  if (backend === 'Auto') return { backend: 'Auto', device_id: deviceId }
  if (backend === 'GpuOnly') return { backend: 'GpuOnly', device_id: deviceId }
  if (backend === 'GpuHybridKernel') return { backend: 'GpuHybridKernel', device_id: deviceId }
  if (backend === 'GpuKernel') return { backend: 'GpuKernel', device_id: deviceId }
  return { backend: 'CpuOnly' }
}

function titleCaseBackend(backend: ValidationRequestPayload['backend']['backend']): string {
  switch (backend) {
    case 'Auto':
      return 'Auto'
    case 'CpuOnly':
      return 'CpuOnly'
    case 'GpuOnly':
      return 'GpuOnly'
    case 'GpuHybridKernel':
      return 'GpuHybridKernel'
    case 'GpuKernel':
      return 'GpuKernel'
    default:
      return String(backend || '').trim() || 'Unknown'
  }
}

function describeEnabledModules(req: ValidationRequestPayload): string[] {
  const enabled: string[] = ['base reruns', 'hard gates']
  if (req.walk_forward) enabled.push('walk-forward')
  if (req.cscv_pbo) enabled.push('CSCV/PBO')
  if (req.spa) enabled.push('SPA')
  if (req.reopt_walk_forward) enabled.push('reopt WF')
  return enabled
}

type CompositeProfileKey =
  | 'asr'
  | 'vcc'
  | 'vemr'
  | 'rocb'
  | 'vcet'
  | 'mkcs'
  | 'kqsr'
  | 'carm'
  | 'kvdf'
  | 'cdmc'
  | 'base_composite'
  | 'unknown'

function detectCompositeProfile(req: ValidationRequestPayload): CompositeProfileKey {
  const cand = req.candidates[0]
  if (!cand) return 'unknown'
  if (cand.asr) return 'asr'
  if (cand.vcc) return 'vcc'
  if (cand.vemr) return 'vemr'
  if (cand.rocb) return 'rocb'
  if (cand.vcet) return 'vcet'
  if (cand.mkcs) return 'mkcs'
  if (cand.kqsr) return 'kqsr'
  if (cand.carm) return 'carm'
  if (cand.kvdf) return 'kvdf'
  if (cand.cdmc) return 'cdmc'
  return 'base_composite'
}

function compositeProfileLabel(profile: CompositeProfileKey): string {
  switch (profile) {
    case 'base_composite':
      return 'base composite'
    case 'unknown':
      return 'unknown composite'
    default:
      return profile.toUpperCase()
  }
}

function compositeProfileGpuRouteNote(profile: CompositeProfileKey): string {
  switch (profile) {
    case 'base_composite':
      return 'Plain CompositeCpu GPU validation currently requires uniform HalfTrend parameters across finalists; mixed finalist sets can fall back to CPU.'
    case 'asr':
      return 'ASR GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.'
    case 'vcc':
      return 'VCC GPU validation currently falls back on the measured 50k/8 advanced-stack workload because the requested momentum rows are not fp32-stable.'
    case 'vemr':
      return 'VEMR GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.'
    case 'rocb':
      return 'ROCB GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.'
    case 'vcet':
      return 'VCET GPU validation now supports the fixed-candidate GPU route for the main profile. Walk-forward currently uses the generic validation path, and perturb/trade bootstrap remain auxiliary CPU work.'
    case 'mkcs':
      return 'MKCS GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.'
    case 'kqsr':
      return 'KQSR GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.'
    case 'kvdf':
      return 'KVDF GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.'
    case 'carm':
      return 'CARM GPU validation currently uses CPU-reference signal prep with GPU metric evaluation. Current advanced-stack benches show CPU faster than GPU on the measured 50k/8 and 100k/32 workloads.'
    case 'cdmc':
      return 'CDMC GPU validation currently uses CPU-reference signal prep with GPU metric evaluation. Fresh advanced-stack benches show GPU faster than CPU on the measured 50k/8 and 100k/32 workloads, and the remaining small walk-forward / CSCV / SPA metric drift is currently accepted because candidate ordering and verdict semantics still match on the fixture.'
    default:
      return ''
  }
}

function compositeProfileAutoCpuPreferenceNote(profile: CompositeProfileKey): string {
  switch (profile) {
    case 'asr':
      return 'Auto composite validation currently prefers CPU for ASR because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.'
    case 'vcc':
      return 'Auto composite validation currently prefers CPU for VCC because the measured 50k/8 advanced-stack GPU route falls back on the current fp32-stability check.'
    case 'vemr':
      return 'Auto composite validation currently prefers CPU for VEMR because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.'
    case 'rocb':
      return 'Auto composite validation currently prefers CPU for ROCB because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.'
    case 'mkcs':
      return 'Auto composite validation currently prefers CPU for MKCS because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.'
    case 'kqsr':
      return 'Auto composite validation currently prefers CPU for KQSR because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.'
    case 'kvdf':
      return 'Auto composite validation currently prefers CPU for KVDF because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.'
    case 'carm':
      return 'Auto composite validation currently prefers CPU for CARM because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 and 100k/32 workloads.'
    default:
      return ''
  }
}

function compositeGpuMainRouteUnsupportedModules(req: ValidationRequestPayload): string[] {
  const profile = detectCompositeProfile(req)
  const supportsVectorizedModules = profile !== 'unknown'
  const unsupported: string[] = []
  if (req.walk_forward && !supportsVectorizedModules) unsupported.push('walk_forward')
  if (req.reopt_walk_forward && !supportsVectorizedModules) unsupported.push('reopt_walk_forward')
  if (req.spa && !supportsVectorizedModules) unsupported.push('spa')
  if (req.trade_bootstrap && !supportsVectorizedModules) unsupported.push('trade_bootstrap')
  if (req.cscv_pbo && !supportsVectorizedModules) unsupported.push('cscv_pbo')
  return unsupported
}

function normalizeBootstrapMode(raw: unknown): 'compare' | 'iid_only' {
  return String(raw || '').trim().toLowerCase() === 'iid_only' ? 'iid_only' : 'compare'
}

function describeTradeBootstrapConfig(
  tradeBootstrap: ValidationRequestPayload['trade_bootstrap'],
): string {
  if (!tradeBootstrap) return 'not run'
  const statBlock = tradeBootstrap.stationary_mean_block_len
  if (statBlock === 0) return 'trade bootstrap iid-only'
  return statBlock != null
    ? `trade bootstrap compare (iid + stationary, stat block=${statBlock})`
    : 'trade bootstrap compare (iid + stationary, stat block=auto)'
}

type ValidationModuleCategory =
  | 'GPU-native'
  | 'GPU main + CPU tail'
  | 'CPU-only'
  | 'lightweight rerun only'
  | 'not supported'

function enabledExecutionCategoryModules(req: ValidationRequestPayload): string[] {
  const modules = ['base reruns', 'hard gates']
  if (req.walk_forward) modules.push('walk_forward')
  if (req.reopt_walk_forward) modules.push('reopt_walk_forward')
  if (req.cscv_pbo) modules.push('cscv_pbo')
  if (req.spa) modules.push('spa')
  if (req.perturb) modules.push('perturb')
  if (req.trade_bootstrap) modules.push('trade_bootstrap')
  return modules
}

function formatExecutionCategorySummary(groups: Array<[ValidationModuleCategory, string[]]>): string {
  const parts = groups
    .filter(([, labels]) => labels.length > 0)
    .map(([category, labels]) => `${category}: ${labels.join(' + ')}`)
  return parts.join(' | ') || 'none'
}

function buildValidationCategorySummaryForMode(
  req: ValidationRequestPayload,
  mode: 'double_gpu' | 'composite_gpu' | 'cpu',
): string {
  const cpuOnly = enabledExecutionCategoryModules(req)
  if (mode === 'cpu') return formatExecutionCategorySummary([['CPU-only', cpuOnly]])

  if (mode === 'double_gpu') {
    const gpuNative: string[] = ['base reruns', 'hard gates']
    if (req.walk_forward) gpuNative.push('walk_forward')
    if (req.reopt_walk_forward) gpuNative.push('reopt_walk_forward')
    if (req.cscv_pbo) gpuNative.push('cscv_pbo')
    if (req.spa) gpuNative.push('spa')
    const gpuMainCpuTail: string[] = []
    if (req.perturb) gpuMainCpuTail.push('perturb')
    if (req.trade_bootstrap) gpuMainCpuTail.push('trade_bootstrap')
    return formatExecutionCategorySummary([
      ['GPU-native', gpuNative],
      ['GPU main + CPU tail', gpuMainCpuTail],
    ])
  }

  return formatExecutionCategorySummary([['GPU-native', cpuOnly]])
}

export function buildValidationCapabilitySummary(req: ValidationRequestPayload): ValidationCapabilitySummary {
  const strategyKind = normalizeStrategyKindRaw(req?.validation_context?.strategy_kind)
  const isDoubleLike = isDoubleLikeKind(strategyKind)
  const backend = req.backend?.backend || 'CpuOnly'
  const crossMode = normalizeText(req.cross_ticker?.mode)
  const crossDatasets = Array.isArray(req.cross_ticker?.datasets) ? req.cross_ticker.datasets.length : 0
  const enabledMain = describeEnabledModules(req)
  const auxCpu: string[] = []
  if (req.perturb) auxCpu.push('perturb')
  if (req.trade_bootstrap) {
    auxCpu.push(describeTradeBootstrapConfig(req.trade_bootstrap))
  }

  let routeSummary = `${titleCaseBackend(backend)} requested`
  let moduleSummary = ''
  let categorySummary = 'none'
  const skippedNotes: string[] = []
  const warningNotes: string[] = []

  if (isDoubleLike) {
    if (backend === 'CpuOnly') {
      routeSummary = 'CpuOnly requested | CPU validation route expected'
      moduleSummary = `CPU route: ${enabledMain.join(' + ')}${auxCpu.length > 0 ? ` + ${auxCpu.join(' + ')}` : ''}`
      categorySummary = buildValidationCategorySummaryForMode(req, 'cpu')
    } else if (backend === 'GpuOnly') {
      routeSummary = 'GpuOnly requested | expected CpuOnly fallback for fixed-candidate validation'
      moduleSummary = `Expected CPU fallback route: ${enabledMain.join(' + ')}${auxCpu.length > 0 ? ` + ${auxCpu.join(' + ')}` : ''}`
      categorySummary = buildValidationCategorySummaryForMode(req, 'cpu')
      warningNotes.push('GpuOnly is not supported for fixed-candidate Double-MA validation; use GpuKernel, GpuHybridKernel, or Auto.')
    } else if (backend === 'GpuHybridKernel') {
      routeSummary = 'GpuHybridKernel requested | expected GPU main route (GpuKernel-compatible)'
      moduleSummary = `GPU main route: ${enabledMain.join(' + ')}${auxCpu.length > 0 ? ` | auxiliary CPU tails: ${auxCpu.join(' + ')}` : ''}`
      categorySummary = buildValidationCategorySummaryForMode(req, 'double_gpu')
    } else if (backend === 'GpuKernel') {
      routeSummary = 'GpuKernel requested | expected GPU main route'
      moduleSummary = `GPU main route: ${enabledMain.join(' + ')}${auxCpu.length > 0 ? ` | auxiliary CPU tails: ${auxCpu.join(' + ')}` : ''}`
      categorySummary = buildValidationCategorySummaryForMode(req, 'double_gpu')
    } else {
      routeSummary = 'Auto requested | expected GPU main route when available'
      moduleSummary = `Expected GPU main route: ${enabledMain.join(' + ')}${auxCpu.length > 0 ? ` | auxiliary CPU tails: ${auxCpu.join(' + ')}` : ''}`
      categorySummary = buildValidationCategorySummaryForMode(req, 'double_gpu')
    }

    if (auxCpu.length > 0 && backend !== 'CpuOnly' && backend !== 'GpuOnly') {
      warningNotes.push(`Main GPU route is preserved; ${auxCpu.join(' + ')} currently run as auxiliary CPU work.`)
    }
  } else {
    const compositeProfileKey = detectCompositeProfile(req)
    const compositeProfile = compositeProfileLabel(compositeProfileKey)
    const compositeGpuRouteNote = compositeProfileGpuRouteNote(compositeProfileKey)
    const compositeAutoCpuPreferenceNote = backend === 'Auto'
      ? compositeProfileAutoCpuPreferenceNote(compositeProfileKey)
      : ''
    const unsupported = compositeGpuMainRouteUnsupportedModules(req)
    const compositeProfileFallbackReason = strategyKind === 'CompositeMo'
      ? 'CompositeMo GPU fixed-candidate validation currently supports plain CompositeCpu HardGate candidates only.'
      : ''
    if (backend === 'CpuOnly') {
      routeSummary = `CpuOnly requested | CPU composite route expected (${compositeProfile})`
      moduleSummary = `CPU route (${compositeProfile}): ${enabledMain.join(' + ')}${auxCpu.length > 0 ? ` + ${auxCpu.join(' + ')}` : ''}`
      categorySummary = buildValidationCategorySummaryForMode(req, 'cpu')
    } else if (compositeAutoCpuPreferenceNote) {
      routeSummary = `Auto requested | expected CPU-preferred composite route (${compositeProfile})`
      moduleSummary = `Expected CPU route (${compositeProfile}): ${enabledMain.join(' + ')}${auxCpu.length > 0 ? ` + ${auxCpu.join(' + ')}` : ''}`
      categorySummary = buildValidationCategorySummaryForMode(req, 'cpu')
      warningNotes.push(compositeAutoCpuPreferenceNote)
    } else if (unsupported.length > 0 || compositeProfileFallbackReason) {
      routeSummary = `${titleCaseBackend(backend)} requested | expected CpuOnly fallback for composite validation`
      moduleSummary = `Expected CPU fallback route (${compositeProfile}): ${enabledMain.join(' + ')}${auxCpu.length > 0 ? ` + ${auxCpu.join(' + ')}` : ''}`
      categorySummary = buildValidationCategorySummaryForMode(req, 'cpu')
      if (compositeProfileFallbackReason) warningNotes.push(compositeProfileFallbackReason)
      if (unsupported.length > 0) {
        warningNotes.push(`Composite GPU fixed-candidate validation does not support: ${unsupported.join(', ')}.`)
        skippedNotes.push(`GPU main route unavailable for: ${unsupported.join(', ')}`)
      }
    } else {
      routeSummary = `${titleCaseBackend(backend)} requested | expected GPU composite route (${compositeProfile})`
      moduleSummary = `Expected GPU composite route (${compositeProfile}): ${enabledMain.join(' + ')}${auxCpu.length > 0 ? ` + ${auxCpu.join(' + ')}` : ''}`
      categorySummary = buildValidationCategorySummaryForMode(req, 'composite_gpu')
      if (compositeGpuRouteNote) warningNotes.push(compositeGpuRouteNote)
      if (auxCpu.length > 0) {
        warningNotes.push(`${auxCpu.join(' + ')} may not share the same execution route as the main validation pass; confirm post-run route notes.`)
      }
    }
    if (req.cross_ticker) {
      warningNotes.push(
        crossMode && crossMode !== 'lightweight'
          ? `Composite fixed-candidate main-route validation does not support inline cross_ticker; composite cross reruns remain lightweight and omit walk_forward, reopt_walk_forward, spa, perturb, trade_bootstrap, cscv_pbo, and hard gates; requested ${crossMode} currently uses lightweight behavior.`
          : 'Composite fixed-candidate main-route validation does not support inline cross_ticker; composite cross reruns are lightweight and omit walk_forward, reopt_walk_forward, spa, perturb, trade_bootstrap, cscv_pbo, and hard gates.',
      )
      skippedNotes.push('Cross reruns: composite main route does not support inline cross_ticker; lightweight composite reruns omit walk_forward + reopt_walk_forward + SPA + perturb + trade bootstrap + CSCV + hard gates')
    }
  }

  let crossSummary = 'not run'
  if (crossDatasets > 0) {
    const base = `${crossDatasets} dataset${crossDatasets === 1 ? '' : 's'} | ${crossMode || 'default'}`
    if (isDoubleLike && crossMode === 'preserved_finalists') {
      crossSummary = `${base} | finalists only`
      warningNotes.push('Preserved-finalists cross reruns keep CSCV/PBO + hard gates; auxiliary cross reruns omit walk-forward, SPA, reopt WF, perturb, and trade bootstrap.')
      skippedNotes.push('Cross reruns: preserved-finalists omit walk_forward + reopt_walk_forward + SPA + perturb + trade bootstrap')
    } else if (isDoubleLike && crossMode === 'lightweight') {
      crossSummary = `${base} | lightweight reruns`
    } else if (!isDoubleLike) {
      crossSummary = crossMode && crossMode !== 'lightweight'
        ? `${base} | effective lightweight reruns`
        : `${base} | lightweight reruns`
    } else {
      crossSummary = base
    }
  }

  return {
    routeSummary,
    crossSummary,
    moduleSummary: moduleSummary || '-',
    categorySummary,
    skippedSummary: skippedNotes.join(' | ') || 'none',
    warningSummary: warningNotes.join(' | ') || 'none',
  }
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

      if (kind === 'Vcet') {
        const strategy = currentRunReq.strategy || currentRunReq.base_strategy || {}
        const vcet = asRecord(p.vcet) || asRecord(currentRunReq.vcet) || {}
        return {
          fast_ma_type,
          slow_ma_type,
          fast_len,
          slow_len,
          fast_ma_params: asRecord(p.fast_ma_params) || undefined,
          slow_ma_params: asRecord(p.slow_ma_params) || undefined,
          vcet,
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

      if (kind === 'Kqsr') {
        const strategy = currentRunReq.strategy || currentRunReq.base_strategy || {}
        const kqsr = asRecord(p.kqsr) || asRecord(currentRunReq.kqsr) || {}
        return {
          fast_ma_type,
          slow_ma_type,
          fast_len,
          slow_len,
          fast_ma_params: asRecord(p.fast_ma_params) || undefined,
          slow_ma_params: asRecord(p.slow_ma_params) || undefined,
          kqsr,
          strategy
        }
      }

      if (kind === 'Carm') {
        const strategy = currentRunReq.strategy || currentRunReq.base_strategy || {}
        const carm = asRecord(p.carm) || asRecord(currentRunReq.carm) || {}
        return {
          fast_ma_type,
          slow_ma_type,
          fast_len,
          slow_len,
          fast_ma_params: asRecord(p.fast_ma_params) || undefined,
          slow_ma_params: asRecord(p.slow_ma_params) || undefined,
          carm,
          strategy
        }
      }

      if (kind === 'Kvdf') {
        const strategy = currentRunReq.strategy || currentRunReq.base_strategy || {}
        const kvdf = asRecord(p.kvdf) || asRecord(currentRunReq.kvdf) || {}
        return {
          fast_ma_type,
          slow_ma_type,
          fast_len,
          slow_len,
          fast_ma_params: asRecord(p.fast_ma_params) || undefined,
          slow_ma_params: asRecord(p.slow_ma_params) || undefined,
          kvdf,
          strategy
        }
      }

      if (kind === 'Cdmc') {
        const strategy = currentRunReq.strategy || currentRunReq.base_strategy || {}
        const cdmc = asRecord(p.cdmc) || asRecord(currentRunReq.cdmc) || {}
        return {
          fast_ma_type,
          slow_ma_type,
          fast_len,
          slow_len,
          fast_ma_params: asRecord(p.fast_ma_params) || undefined,
          slow_ma_params: asRecord(p.slow_ma_params) || undefined,
          cdmc,
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
      if (kind === 'Vcet') return true
      if (kind === 'Mkcs') return true
      if (kind === 'Kqsr') return true
      if (kind === 'Carm') return true
      if (kind === 'Kvdf') return true
      if (kind === 'Cdmc') return true
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

  const trade_bootstrap = inputs.mcEnable
    ? (() => {
        const simulations = Math.max(0, Math.floor(num(inputs.mcSims, 500)))
        const mode = normalizeBootstrapMode(inputs.mcMode)
        const stationary_mean_block_len = Math.max(0, Math.floor(num(inputs.mcBlockLen, 0)))
        if (mode === 'iid_only') return { simulations, stationary_mean_block_len: 0 }
        return stationary_mean_block_len > 0
          ? { simulations, stationary_mean_block_len }
          : { simulations }
      })()
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
        const mode = normalizeCrossTickerMode(inputs.crossMode, kind)
        return {
          max_datasets,
          mode,
          datasets
        }
      })()
    : null

  return {
    data_id: dataId,
    backend: normalizeValidationBackend(currentRunReq.backend, kind),
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
