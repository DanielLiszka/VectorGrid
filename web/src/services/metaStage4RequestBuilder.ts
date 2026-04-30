function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
}

function normalizeBoolean(value: unknown): boolean {
  if (typeof value === "boolean") return value;
  const text = normalizeText(value).toLowerCase();
  return text === "true" || text === "1" || text === "yes" || text === "on";
}

function num(value: unknown, fallback: number): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function probability(value: unknown, fallback: number): number {
  const text = normalizeText(value);
  if (!text) return fallback;
  const cleaned = text.endsWith("%") ? text.slice(0, -1).trim() : text;
  const parsed = Number(cleaned);
  if (!Number.isFinite(parsed)) return fallback;
  const normalized = parsed > 1 && parsed <= 100 ? parsed / 100 : parsed;
  return clamp(normalized, 0, 1);
}

export interface MetaStage4BuildInputs {
  strategyKind?: unknown;
  timeframe?: unknown;
  sourceMetaPath?: unknown;
  topK?: unknown;
  minMlConfidence?: unknown;
  metaWeight?: unknown;
  mlWeight?: unknown;
  uncertaintyWeight?: unknown;
  acceptConfidence?: unknown;
  reviewConfidence?: unknown;
  maxAcceptUncertainty?: unknown;
  autoTrainModel?: unknown;
  requireHealthyModel?: unknown;
  fallbackToMetaIfEmpty?: unknown;
  trainingHistoryPolicy?: unknown;
}

export interface MetaStage5BuildInputs extends MetaStage4BuildInputs {}

export function buildMetaStage5Request(
  inputs: MetaStage5BuildInputs,
): Record<string, unknown> {
  const strategyKind =
    normalizeText(inputs.strategyKind || "CompositeMo") || "CompositeMo";
  const timeframeRaw = normalizeText(inputs.timeframe);
  const sourceMetaPathRaw = normalizeText(inputs.sourceMetaPath);
  const autoTrainModel = normalizeBoolean(inputs.autoTrainModel);
  const trainingHistoryRaw = normalizeText(
    inputs.trainingHistoryPolicy,
  ).toLowerCase();
  const trainingHistoryPolicy =
    trainingHistoryRaw === "chronological_before_source"
      ? "chronological_before_source"
      : "source_excluded";

  const minMlConfidence = probability(inputs.minMlConfidence, 0.55);
  const acceptConfidence = probability(inputs.acceptConfidence, 0.7);
  const reviewConfidence = Math.min(
    probability(inputs.reviewConfidence, 0.55),
    acceptConfidence,
  );
  const maxAcceptUncertainty = probability(inputs.maxAcceptUncertainty, 0.45);

  return {
    strategy_kind: strategyKind,
    timeframe: timeframeRaw ? timeframeRaw : null,
    source_meta_path: sourceMetaPathRaw ? sourceMetaPathRaw : null,
    top_k: Math.max(1, Math.floor(num(inputs.topK, 10))),
    min_ml_confidence: minMlConfidence,
    meta_weight: Math.max(0, num(inputs.metaWeight, 0.6)),
    ml_weight: Math.max(0, num(inputs.mlWeight, 0.35)),
    uncertainty_weight: Math.max(0, num(inputs.uncertaintyWeight, 0.05)),
    accept_confidence: acceptConfidence,
    review_confidence: reviewConfidence,
    max_accept_uncertainty: maxAcceptUncertainty,
    auto_train_model: autoTrainModel,
    auto_retrain_stale: autoTrainModel,
    require_healthy_model: normalizeBoolean(inputs.requireHealthyModel),
    fallback_to_meta_if_empty: normalizeBoolean(inputs.fallbackToMetaIfEmpty),
    training_history_policy: trainingHistoryPolicy,
  };
}

export function buildMetaStage4Request(
  inputs: MetaStage4BuildInputs,
): Record<string, unknown> {
  return buildMetaStage5Request(inputs);
}
