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

export interface MetaStage4BuildInputs {
  strategyKind?: unknown
  timeframe?: unknown
  sourceMetaPath?: unknown
  topK?: unknown
  minMlConfidence?: unknown
  metaWeight?: unknown
  mlWeight?: unknown
  uncertaintyWeight?: unknown
  acceptConfidence?: unknown
  reviewConfidence?: unknown
  maxAcceptUncertainty?: unknown
  autoTrainModel?: unknown
  requireHealthyModel?: unknown
  fallbackToMetaIfEmpty?: unknown
}

export function buildMetaStage4Request(inputs: MetaStage4BuildInputs): Record<string, unknown> {
  const strategyKind = normalizeText(inputs.strategyKind || 'CompositeMo') || 'CompositeMo'
  const timeframeRaw = normalizeText(inputs.timeframe)
  const sourceMetaPathRaw = normalizeText(inputs.sourceMetaPath)
  const autoTrainModel = normalizeBoolean(inputs.autoTrainModel)

  return {
    strategy_kind: strategyKind,
    timeframe: timeframeRaw ? timeframeRaw : null,
    source_meta_path: sourceMetaPathRaw ? sourceMetaPathRaw : null,
    top_k: Math.max(1, Math.floor(num(inputs.topK, 10))),
    min_ml_confidence: num(inputs.minMlConfidence, 0.55),
    meta_weight: Math.max(0, num(inputs.metaWeight, 0.60)),
    ml_weight: Math.max(0, num(inputs.mlWeight, 0.35)),
    uncertainty_weight: Math.max(0, num(inputs.uncertaintyWeight, 0.05)),
    accept_confidence: num(inputs.acceptConfidence, 0.70),
    review_confidence: num(inputs.reviewConfidence, 0.55),
    max_accept_uncertainty: num(inputs.maxAcceptUncertainty, 0.45),
    auto_train_model: autoTrainModel,
    auto_retrain_stale: autoTrainModel,
    require_healthy_model: normalizeBoolean(inputs.requireHealthyModel),
    fallback_to_meta_if_empty: normalizeBoolean(inputs.fallbackToMetaIfEmpty),
  }
}
