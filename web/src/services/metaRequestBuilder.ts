function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizeBoolean(value: unknown): boolean {
  if (typeof value === 'boolean') return value
  const text = normalizeText(value).toLowerCase()
  return text === 'true' || text === '1' || text === 'yes' || text === 'on'
}

function num(value: unknown, fallback: number): number {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : fallback
}

function supportsHoldout(strategyKind: string): boolean {
  return strategyKind === 'CompositeCpu' || strategyKind === 'CompositeMo' || strategyKind === 'DoubleMa' || strategyKind === 'Pit' || strategyKind === 'Set' || strategyKind === 'Mts' || strategyKind === 'Orc' || strategyKind === 'Marrt' || strategyKind === 'Vfct' || strategyKind === 'Rocb' || strategyKind === 'Asr' || strategyKind === 'Vcc' || strategyKind === 'Vemr' || strategyKind === 'CompositeTrend' || strategyKind === 'RegimeHybrid' || strategyKind === 'BreakoutMomentum'
}

function normalizeBootstrapMode(raw: unknown): 'compare' | 'iid_only' {
  return normalizeText(raw).toLowerCase() === 'iid_only' ? 'iid_only' : 'compare'
}

export interface MetaSelectionBuildInputs {
  strategyKind?: unknown
  timeframe?: unknown
  requiredTag?: unknown
  minSamples?: unknown
  topK?: unknown
  holdoutTag?: unknown
  evalHoldout?: unknown
  holdoutMode?: unknown
  holdoutEvalTopN?: unknown
  holdoutExploreN?: unknown
  holdoutRankBy?: unknown
  holdoutReps?: unknown
  groupingMode?: unknown
  clusterIntBucket?: unknown
  clusterFloatBucket?: unknown
  saveHoldoutRows?: unknown
  valSeed?: unknown
  valWfWindow?: unknown
  valWfStep?: unknown
  valPerturbEnable?: unknown
  valPerturbIters?: unknown
  valPerturbMin?: unknown
  valPerturbMax?: unknown
  valMcEnable?: unknown
  valMcSims?: unknown
  valMcBlockLen?: unknown
  valMcMode?: unknown
}

export function buildMetaSelectionRequest(inputs: MetaSelectionBuildInputs): Record<string, unknown> {
  const strategyKind = normalizeText(inputs.strategyKind || 'CompositeMo') || 'CompositeMo'
  const holdoutSupported = supportsHoldout(strategyKind)
  const timeframeRaw = normalizeText(inputs.timeframe)
  const requiredTagRaw = normalizeText(inputs.requiredTag)
  const minSamples = Math.max(1, Math.floor(num(inputs.minSamples, 30)))
  const topK = Math.max(1, Math.floor(num(inputs.topK, 25)))
  const holdoutTagRaw = normalizeText(inputs.holdoutTag)
  const evalHoldout = holdoutSupported && normalizeBoolean(inputs.evalHoldout)
  const holdoutModeRaw = normalizeText(inputs.holdoutMode || 'light') || 'light'
  const holdoutMode = evalHoldout ? holdoutModeRaw : 'light'
  const holdoutEvalTopN = Math.max(0, Math.floor(num(inputs.holdoutEvalTopN, 0)))
  const holdoutExploreN = Math.max(0, Math.floor(num(inputs.holdoutExploreN, 0)))
  const holdoutRankBy = normalizeText(inputs.holdoutRankBy || 'auto') || 'auto'
  const repsRaw = Math.floor(num(inputs.holdoutReps, 2))
  const clusterRepresentatives = Math.min(3, Math.max(1, Number.isFinite(repsRaw) ? repsRaw : 2))
  const groupingMode = normalizeText(inputs.groupingMode || 'clustered') || 'clustered'
  const intBucket = Math.max(1, Math.floor(num(inputs.clusterIntBucket, 5)))
  const floatBucketRaw = Number(inputs.clusterFloatBucket)
  const floatBucket = Number.isFinite(floatBucketRaw) ? Math.max(0, floatBucketRaw) : 0.25

  const holdoutValidation =
    evalHoldout && holdoutMode === 'heavy'
      ? (() => {
          const seed = Math.max(0, Math.floor(num(inputs.valSeed, 1)))
          const walk_forward = {
            window_bars: Math.max(64, Math.floor(num(inputs.valWfWindow, 20000))),
            step_bars: Math.max(1, Math.floor(num(inputs.valWfStep, 20000)))
          }
          const perturb = normalizeBoolean(inputs.valPerturbEnable)
            ? {
                iterations: Math.max(0, Math.floor(num(inputs.valPerturbIters, 8))),
                pct_min: Math.max(0, num(inputs.valPerturbMin, 0.015)),
                pct_max: Math.max(0, num(inputs.valPerturbMax, 0.06)),
                flip_bool_prob: 0.02
              }
            : null
          const trade_bootstrap = normalizeBoolean(inputs.valMcEnable)
            ? (() => {
                const simulations = Math.max(0, Math.floor(num(inputs.valMcSims, 500)))
                const mode = normalizeBootstrapMode(inputs.valMcMode)
                const statBlock = Math.max(0, Math.floor(num(inputs.valMcBlockLen, 0)))
                return mode === 'iid_only'
                  ? { simulations, stationary_mean_block_len: 0 }
                  : statBlock > 0
                    ? { simulations, stationary_mean_block_len: statBlock }
                    : { simulations }
              })()
            : null
          const validation_engine = 'iterative'
          const iterative = {
            stop_loss_pct: null,
            take_profit_pct: null,
            intrabar_rule: 'pessimistic',
            slippage_vol_mult: 0.15
          }
          return {
            seed,
            walk_forward,
            perturb,
            trade_bootstrap,
            validation_engine,
            iterative
          }
        })()
      : null

  return {
    strategy_kind: strategyKind,
    timeframe: timeframeRaw ? timeframeRaw : null,
    required_tag: requiredTagRaw ? requiredTagRaw : null,
    holdout_tag: evalHoldout && holdoutTagRaw ? holdoutTagRaw : null,
    eval_holdout: evalHoldout,
    holdout_mode: holdoutMode,
    holdout_validation: holdoutValidation,
    save_holdout_rows: evalHoldout && normalizeBoolean(inputs.saveHoldoutRows),
    holdout_eval_top_n: evalHoldout && holdoutEvalTopN > 0 ? holdoutEvalTopN : null,
    holdout_eval_explore_n: evalHoldout ? holdoutExploreN : 0,
    holdout_eval_rank_by: evalHoldout ? holdoutRankBy : null,
    grouping_mode: groupingMode,
    cluster_config: { int_bucket: intBucket, float_bucket: floatBucket },
    cluster_representatives: clusterRepresentatives,
    min_samples: minSamples,
    top_k: topK,
    strict_shortlist: true,
    allow_shortlist_fallback: false
  }
}
