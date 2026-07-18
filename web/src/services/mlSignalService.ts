import { resolveRuntimePinia } from "../stores/runtimePinia";
import { invokeCommand } from "../tauri/invoke";
import {
  applyWorkflowArtifactsPayload,
  type WorkflowRunArtifact,
} from "../stores/workflowArtifactsStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useStrategyStore } from "../stores/strategyStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";
import { computeConfigFingerprint } from "../utils/core/artifactIdentity";
import { buildDatasetMetaForArtifact } from "../utils/core/datasetMeta";
import { storeErrorText } from "../utils/storeErrorText";

interface SaveRunArtifactResponse {
  path?: unknown;
}

interface MlSignalModelBundleSummary {
  bundle_id?: unknown;
  manifest_path?: unknown;
  state?: unknown;
  model_family?: unknown;
  model_format?: unknown;
  issues?: unknown;
}

interface MlSignalModelStatusResponse {
  models_dir?: unknown;
  status?: unknown;
}

interface SweepAxisF64 {
  start: number;
  end: number;
  step: number;
}

interface SweepAxisUsize {
  start: number;
  end: number;
  step: number;
}

interface MlSignalArtifactLike {
  stage?: unknown;
  bundle_id?: unknown;
  bundleId?: unknown;
  source_bundle_manifest_path?: unknown;
  sourceBundleManifestPath?: unknown;
  dataset_export_path?: unknown;
  datasetExportPath?: unknown;
  export_manifest_path?: unknown;
  exportManifestPath?: unknown;
  split_manifest_path?: unknown;
  splitManifestPath?: unknown;
  row_count?: unknown;
  rowCount?: unknown;
  feature_count?: unknown;
  featureCount?: unknown;
  split_count?: unknown;
  splitCount?: unknown;
  score_cache_path?: unknown;
  scoreCachePath?: unknown;
  score_manifest_path?: unknown;
  scoreManifestPath?: unknown;
  feature_schema_path?: unknown;
  featureSchemaPath?: unknown;
  feature_schema_version?: unknown;
  featureSchemaVersion?: unknown;
  feature_schema_hash?: unknown;
  featureSchemaHash?: unknown;
  schema_hash?: unknown;
  schemaHash?: unknown;
  exporter?: unknown;
  exporter_metadata?: unknown;
  exporterMetadata?: unknown;
  exporter_version?: unknown;
  exporterVersion?: unknown;
  provenance?: unknown;
  indicator_config?: unknown;
  indicatorConfig?: unknown;
  warmup_policy?: unknown;
  warmupPolicy?: unknown;
  label_policy?: unknown;
  labelPolicy?: unknown;
  deadband_policy?: unknown;
  deadbandPolicy?: unknown;
  sample_weight_policy?: unknown;
  sampleWeightPolicy?: unknown;
  data_timeframe?: unknown;
  dataTimeframe?: unknown;
  created_at?: unknown;
  createdAt?: unknown;
  source_export_manifest_hash?: unknown;
  sourceExportManifestHash?: unknown;
  source_feature_schema_hash?: unknown;
  sourceFeatureSchemaHash?: unknown;
  training_config?: unknown;
  trainingConfig?: unknown;
  training_config_hash?: unknown;
  trainingConfigHash?: unknown;
  training_windows_summary?: unknown;
  trainingWindowsSummary?: unknown;
  calibration?: unknown;
  calibration_summary?: unknown;
  calibrationSummary?: unknown;
  calibration_state?: unknown;
  calibrationState?: unknown;
  class_balance_policy?: unknown;
  classBalancePolicy?: unknown;
  threshold_policy_default?: unknown;
  thresholdPolicyDefault?: unknown;
  explainability_artifacts?: unknown;
  explainabilityArtifacts?: unknown;
  bundle_parent_id?: unknown;
  bundleParentId?: unknown;
  feature_block_versions?: unknown;
  featureBlockVersions?: unknown;
  indicator_params?: unknown;
  indicatorParams?: unknown;
  label_horizon_bars?: unknown;
  labelHorizonBars?: unknown;
  first_valid_reasoning?: unknown;
  firstValidReasoning?: unknown;
  score_contract?: unknown;
  scoreContract?: unknown;
  score_contract_version?: unknown;
  scoreContractVersion?: unknown;
  score_cache_contract_mode?: unknown;
  scoreCacheContractMode?: unknown;
  score_cache_provenance?: unknown;
  scoreCacheProvenance?: unknown;
  source_bundle_id?: unknown;
  sourceBundleId?: unknown;
  source_bundle_manifest_hash?: unknown;
  sourceBundleManifestHash?: unknown;
  bundle_manifest_sha256?: unknown;
  bundleManifestSha256?: unknown;
  model_long_sha256?: unknown;
  modelLongSha256?: unknown;
  model_short_sha256?: unknown;
  modelShortSha256?: unknown;
  feature_schema_sha256?: unknown;
  featureSchemaSha256?: unknown;
  export_manifest_sha256?: unknown;
  exportManifestSha256?: unknown;
  threshold_policy?: unknown;
  thresholdPolicy?: unknown;
  producer_kind?: unknown;
  producerKind?: unknown;
  native_backend_requested?: unknown;
  nativeBackendRequested?: unknown;
  native_backend_used?: unknown;
  nativeBackendUsed?: unknown;
  native_library_path?: unknown;
  nativeLibraryPath?: unknown;
  native_library_sha256?: unknown;
  nativeLibrarySha256?: unknown;
  native_library_discovery_policy?: unknown;
  nativeLibraryDiscoveryPolicy?: unknown;
  native_gpu_device_id?: unknown;
  nativeGpuDeviceId?: unknown;
  native_prediction_batch_size?: unknown;
  nativePredictionBatchSize?: unknown;
  native_fallback_reason?: unknown;
  nativeFallbackReason?: unknown;
  bundle_status?: unknown;
  bundleStatus?: unknown;
  bundle_status_details?: unknown;
  bundleStatusDetails?: unknown;
  score_space?: unknown;
  scoreSpace?: unknown;
  score_scope?: unknown;
  scoreScope?: unknown;
  score_scope_policy?: unknown;
  scoreScopePolicy?: unknown;
  raw_score_observations?: unknown;
  rawScoreObservations?: unknown;
  effective_score_observations?: unknown;
  effectiveScoreObservations?: unknown;
  matched_rows?: unknown;
  matchedRows?: unknown;
  skipped_rows?: unknown;
  skippedRows?: unknown;
  bars_scored?: unknown;
  barsScored?: unknown;
  coverage_ratio?: unknown;
  coverageRatio?: unknown;
  duplicate_timestamps?: unknown;
  duplicateTimestamps?: unknown;
  deployability?: unknown;
  deployability_state?: unknown;
  deployabilityState?: unknown;
  evidence_policy_summary?: unknown;
  evidencePolicySummary?: unknown;
}

function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
}

function formatMlActionError(prefix: string, error: unknown): string {
  const detail = storeErrorText(error);
  return detail ? `${prefix}: ${detail}` : prefix;
}

function parseNumber(value: unknown, fallback: number): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function parseProbabilityLike(
  value: unknown,
  label: string,
  fallback?: number,
): number {
  const text = normalizeText(value);
  if (!text && fallback != null) return fallback;
  const cleaned = text.endsWith("%") ? text.slice(0, -1).trim() : text;
  const parsed = Number(cleaned);
  if (!Number.isFinite(parsed)) {
    throw new Error(`${label} must be a probability between 0 and 1.`);
  }
  const normalized = parsed > 1 && parsed <= 100 ? parsed / 100 : parsed;
  if (normalized < 0 || normalized > 1) {
    throw new Error(`${label} must be between 0 and 1, or 0 and 100 as a percentage.`);
  }
  return normalized;
}

function parseOptionalPositive(value: unknown): number | null {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) return null;
  return parsed;
}

function parseProbabilityGate(value: unknown, label: string): number | null {
  if (!normalizeText(value)) return null;
  return parseProbabilityLike(value, label);
}

function parseOptionalInteger(value: unknown): number | null {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return null;
  const floored = Math.floor(parsed);
  return floored >= 0 ? floored : null;
}

function isMlNativeScoreSource(value: unknown): boolean {
  return normalizeText(value).toLowerCase() === "native";
}

function normalizeMlScoreCacheContractMode(value: unknown): string {
  const raw = normalizeText(value).toLowerCase().replace(/[-\s]+/g, "_");
  return raw === "release_strict" ? "release_strict" : "research";
}

function formatMlScoreCacheContractMode(value: unknown): string {
  return normalizeMlScoreCacheContractMode(value) === "release_strict"
    ? "release strict"
    : "research";
}

function parseOptionalNumber(value: unknown): number | null {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function readOptionalRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const row = value as Record<string, unknown>;
  return Object.keys(row).length > 0 ? row : null;
}

function hasMeaningfulMetadataValue(value: unknown): boolean {
  if (value == null) return false;
  if (typeof value === "string") return normalizeText(value).length > 0;
  if (typeof value === "number") return Number.isFinite(value);
  if (typeof value === "boolean") return true;
  if (Array.isArray(value)) return value.length > 0;
  if (typeof value === "object") {
    return Object.keys(value as Record<string, unknown>).length > 0;
  }
  return true;
}

function compactMetadataRecord(
  entries: Array<[string, unknown]>,
): Record<string, unknown> | null {
  const out: Record<string, unknown> = {};
  for (const [key, value] of entries) {
    if (!hasMeaningfulMetadataValue(value)) continue;
    out[key] = value;
  }
  return Object.keys(out).length > 0 ? out : null;
}

function firstMeaningfulValue(...values: unknown[]): unknown {
  for (const value of values) {
    if (hasMeaningfulMetadataValue(value)) return value;
  }
  return undefined;
}

function resolveMlArtifactMetadataRecord(
  directCandidates: unknown[],
  fallbackEntries: Array<[string, unknown]>,
): Record<string, unknown> | null {
  for (const candidate of directCandidates) {
    const record = readOptionalRecord(candidate);
    if (record) return record;
  }
  return compactMetadataRecord(fallbackEntries);
}

function readStringList(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((entry) => normalizeText(entry))
    .filter((entry) => entry.length > 0);
}

function basename(path: string): string {
  const normalized = String(path || "").trim();
  if (!normalized) return "";
  return normalized.split(/[\\/]/).pop() || normalized;
}

async function openOrRevealPath(path: string): Promise<{
  path: string;
  openedWith: "open_path" | "reveal_path";
}> {
  const nextPath = normalizeText(path);
  if (!nextPath) throw new Error("Path is required.");
  try {
    await invokeCommand("open_path", { path: nextPath });
    return { path: nextPath, openedWith: "open_path" };
  } catch {
    await invokeCommand("reveal_path", { path: nextPath });
    return { path: nextPath, openedWith: "reveal_path" };
  }
}

function buildOptionalProbabilitySweepAxis(
  label: string,
  startRaw: unknown,
  endRaw: unknown,
  stepRaw: unknown,
): SweepAxisF64 | null {
  const allBlank =
    !normalizeText(startRaw) && !normalizeText(endRaw) && !normalizeText(stepRaw);
  if (allBlank) return null;
  if (!normalizeText(startRaw) || !normalizeText(endRaw) || !normalizeText(stepRaw)) {
    throw new Error(`${label} sweep axis requires start, end, and step.`);
  }
  const start = parseProbabilityLike(startRaw, `${label} start`);
  const end = parseProbabilityLike(endRaw, `${label} end`);
  const step = parseProbabilityLike(stepRaw, `${label} step`);
  if (step <= 0) throw new Error(`${label} sweep step must be greater than 0.`);
  if (end < start) throw new Error(`${label} sweep end must be greater than or equal to start.`);
  return { start, end, step };
}

function buildOptionalSweepAxisUsize(
  label: string,
  startRaw: unknown,
  endRaw: unknown,
  stepRaw: unknown,
): SweepAxisUsize | null {
  const allBlank =
    !normalizeText(startRaw) && !normalizeText(endRaw) && !normalizeText(stepRaw);
  if (allBlank) return null;
  if (!normalizeText(startRaw) || !normalizeText(endRaw) || !normalizeText(stepRaw)) {
    throw new Error(`${label} sweep axis requires start, end, and step.`);
  }
  const start = parseOptionalInteger(startRaw);
  const end = parseOptionalInteger(endRaw);
  const step = parseOptionalInteger(stepRaw);
  if (start == null || end == null || step == null)
    throw new Error(`${label} sweep axis must use whole numbers.`);
  if (step <= 0) throw new Error(`${label} sweep step must be greater than 0.`);
  if (end < start) throw new Error(`${label} sweep end must be greater than or equal to start.`);
  return { start: Math.max(1, start), end: Math.max(1, end), step };
}

function parsePositiveInteger(value: unknown, fallback: number, label: string): number {
  const text = normalizeText(value);
  const parsed = text ? Number(text) : fallback;
  if (!Number.isFinite(parsed)) throw new Error(`${label} must be a whole number.`);
  const floored = Math.floor(parsed);
  if (floored < 1) throw new Error(`${label} must be at least 1.`);
  return floored;
}

function parseRequiredProbabilityWithDefault(
  value: unknown,
  fallback: number,
  label: string,
): number {
  return parseProbabilityLike(value, label, fallback);
}

function readCandidateRows(value: unknown): Array<Record<string, unknown>> {
  if (!value || typeof value !== "object") return [];
  const row = value as Record<string, unknown>;
  if (Array.isArray(row.candidates)) {
    return row.candidates.filter(
      (entry): entry is Record<string, unknown> =>
        !!entry && typeof entry === "object",
    );
  }
  const validation =
    row.validation && typeof row.validation === "object"
      ? (row.validation as Record<string, unknown>)
      : null;
  if (validation && Array.isArray(validation.candidates)) {
    return validation.candidates.filter(
      (entry): entry is Record<string, unknown> =>
        !!entry && typeof entry === "object",
    );
  }
  return [];
}

function resolveMlRunVerdictRecord(
  value: unknown,
): Record<string, unknown> | null {
  if (!value || typeof value !== "object") return null;
  const row = value as Record<string, unknown>;
  if (row.run_verdict && typeof row.run_verdict === "object") {
    return row.run_verdict as Record<string, unknown>;
  }
  const validation =
    row.validation && typeof row.validation === "object"
      ? (row.validation as Record<string, unknown>)
      : null;
  if (validation?.run_verdict && typeof validation.run_verdict === "object") {
    return validation.run_verdict as Record<string, unknown>;
  }
  return null;
}

function resolveMlDependenceSummaryRecord(
  value: unknown,
): Record<string, unknown> | null {
  if (!value || typeof value !== "object") return null;
  const row = value as Record<string, unknown>;
  if (row.dependence_summary && typeof row.dependence_summary === "object") {
    return row.dependence_summary as Record<string, unknown>;
  }
  const validation =
    row.validation && typeof row.validation === "object"
      ? (row.validation as Record<string, unknown>)
      : null;
  if (
    validation?.dependence_summary &&
    typeof validation.dependence_summary === "object"
  ) {
    return validation.dependence_summary as Record<string, unknown>;
  }
  return null;
}

function formatMlScoreScopeAdvisory(value: unknown): string {
  const verdict = resolveMlRunVerdictRecord(value);
  if (!verdict) return "";
  const flags = readStringList(verdict.warning_flags ?? verdict.warningFlags);
  const advisories: string[] = [];
  if (flags.includes("score_scope_calibration_slice")) {
    advisories.push("calibration-only score scope");
  }
  if (flags.includes("score_scope_full_dataset")) {
    advisories.push("full-dataset score scope");
  }
  if (flags.includes("score_scope_custom_partial")) {
    advisories.push("partial score scope");
  }
  if (flags.includes("score_scope_walkforward_overlap_metadata_missing")) {
    advisories.push("walk-forward overlap metadata missing");
  }
  if (flags.includes("score_scope_walkforward_overlap_adjusted")) {
    const dependence = resolveMlDependenceSummaryRecord(value);
    const detailParts: string[] = [];
    const rawObs = parseOptionalInteger(
      dependence?.raw_score_observations ?? dependence?.rawScoreObservations,
    );
    const effectiveObs = parseOptionalInteger(
      dependence?.effective_score_observations_lower_bound ??
        dependence?.effectiveScoreObservationsLowerBound,
    );
    if (
      rawObs != null &&
      effectiveObs != null &&
      rawObs > 0 &&
      effectiveObs < rawObs
    ) {
      detailParts.push(`obs ${effectiveObs}/${rawObs}`);
    }
    const rawWindows = parseOptionalInteger(
      dependence?.raw_walkforward_test_windows ??
        dependence?.rawWalkforwardTestWindows,
    );
    const effectiveWindows = parseOptionalInteger(
      dependence?.effective_walkforward_test_windows_upper_bound ??
        dependence?.effectiveWalkforwardTestWindowsUpperBound,
    );
    if (
      rawWindows != null &&
      effectiveWindows != null &&
      rawWindows > 0 &&
      effectiveWindows < rawWindows
    ) {
      detailParts.push(`windows ${effectiveWindows}/${rawWindows}`);
    }
    advisories.push(
      detailParts.length > 0
        ? `walk-forward overlap adjusted (${detailParts.join(", ")})`
        : "walk-forward overlap adjusted",
    );
  }
  return advisories.join("; ");
}

function formatMlBoundaryAdvisory(value: unknown): string {
  const best = readCandidateRows(value)[0];
  if (
    !best ||
    !best.boundary_diagnostics ||
    typeof best.boundary_diagnostics !== "object"
  ) {
    return "";
  }
  const diagnostics = best.boundary_diagnostics as Record<string, unknown>;
  const parts: string[] = [];
  const nearLong = parseOptionalNumber(
    diagnostics.near_long_threshold_frac ?? diagnostics.nearLongThresholdFrac,
  );
  const nearShort = parseOptionalNumber(
    diagnostics.near_short_threshold_frac ?? diagnostics.nearShortThresholdFrac,
  );
  const nearDelta = parseOptionalNumber(
    diagnostics.near_delta_boundary_frac ?? diagnostics.nearDeltaBoundaryFrac,
  );
  if (nearLong != null) {
    parts.push(`long~${(nearLong * 100).toFixed(1)}%`);
  }
  if (nearShort != null) {
    parts.push(`short~${(nearShort * 100).toFixed(1)}%`);
  }
  if (nearDelta != null) {
    parts.push(`delta~${(nearDelta * 100).toFixed(1)}%`);
  }
  return parts.length > 0 ? `boundary ${parts.join(", ")}` : "";
}

function extractMlPolicy(
  validationReq: Record<string, unknown>,
  validationRes: unknown,
): Record<string, unknown> | null {
  if (
    Array.isArray(validationReq.candidates) &&
    validationReq.candidates.length > 0 &&
    validationReq.candidates[0] &&
    typeof validationReq.candidates[0] === "object"
  ) {
    return validationReq.candidates[0] as Record<string, unknown>;
  }
  const best = readCandidateRows(validationRes)[0];
  if (!best) return null;
  return best.params && typeof best.params === "object"
    ? (best.params as Record<string, unknown>)
    : null;
}

function extractMlPolicySweepSummary(
  validationRes: unknown,
): Record<string, unknown> | null {
  if (!validationRes || typeof validationRes !== "object") return null;
  const row = validationRes as Record<string, unknown>;
  const validation =
    row.validation && typeof row.validation === "object"
      ? (row.validation as Record<string, unknown>)
      : null;
  const sweepSize = parseOptionalInteger(
    firstMeaningfulValue(
      row.full_sweep_size,
      row.fullSweepSize,
      row.sweep_size,
      row.sweepSize,
      validation?.full_sweep_size,
      validation?.fullSweepSize,
      validation?.sweep_size,
      validation?.sweepSize,
    ),
  );
  const topK = parseOptionalInteger(
    firstMeaningfulValue(
      row.retained_top_k,
      row.retainedTopK,
      row.top_k,
      row.topK,
      validation?.retained_top_k,
      validation?.retainedTopK,
      validation?.top_k,
      validation?.topK,
    ),
  );
  const candidates = readCandidateRows(validationRes);
  const best = candidates[0];
  const retainedCandidateCount =
    parseOptionalInteger(
      firstMeaningfulValue(
        row.retained_candidate_count,
        row.retainedCandidateCount,
        validation?.retained_candidate_count,
        validation?.retainedCandidateCount,
      ),
    ) ?? (best ? candidates.length : null);
  const ordering = normalizeText(
    firstMeaningfulValue(
      row.ordering,
      row.candidate_ordering,
      row.candidateOrdering,
      row.ranking_key,
      row.rankingKey,
      validation?.ordering,
      validation?.candidate_ordering,
      validation?.candidateOrdering,
      validation?.ranking_key,
      validation?.rankingKey,
    ),
  );
  const verdictScope = normalizeText(
    firstMeaningfulValue(
      row.verdict_scope,
      row.verdictScope,
      row.verdict_rebuild_scope,
      row.verdictRebuildScope,
      validation?.verdict_scope,
      validation?.verdictScope,
      validation?.verdict_rebuild_scope,
      validation?.verdictRebuildScope,
    ),
  );
  const bestLabel = normalizeText(
    firstMeaningfulValue(
      row.best_label,
      row.bestLabel,
      validation?.best_label,
      validation?.bestLabel,
      best?.params && typeof best.params === "object"
        ? (best.params as Record<string, unknown>).label
        : "",
    ),
  );
  const bestRobustScore = parseOptionalNumber(
    firstMeaningfulValue(
      row.best_robust_score,
      row.bestRobustScore,
      validation?.best_robust_score,
      validation?.bestRobustScore,
      best?.robust_score,
    ),
  );
  if (
    sweepSize == null &&
    topK == null &&
    retainedCandidateCount == null &&
    !ordering &&
    !verdictScope &&
    !bestLabel &&
    bestRobustScore == null
  ) {
    return null;
  }
  return {
    full_sweep_size: sweepSize ?? 0,
    sweep_size: sweepSize ?? 0,
    retained_top_k: topK ?? 0,
    top_k: topK ?? 0,
    retained_candidate_count: retainedCandidateCount ?? 0,
    ordering: ordering || "robust_score_desc",
    verdict_scope: verdictScope || "retained_shortlist",
    best_label: bestLabel,
    best_robust_score: bestRobustScore ?? 0,
  };
}

function formatMlSweepOrderingText(value: unknown): string {
  const normalized = normalizeText(value).toLowerCase();
  if (!normalized) return "robust-score ordering";
  if (
    normalized === "robust_score_desc" ||
    normalized === "robust-score-desc" ||
    normalized === "robust_score" ||
    normalized === "robust-score"
  ) {
    return "robust-score ordering";
  }
  return normalizeText(value).replace(/[_-]+/g, " ");
}

function formatMlVerdictScopeText(value: unknown): string {
  const normalized = normalizeText(value).toLowerCase();
  if (!normalized) return "retained shortlist";
  if (
    normalized === "retained_shortlist" ||
    normalized === "retained-shortlist" ||
    normalized === "retained shortlist" ||
    normalized === "shortlist"
  ) {
    return "retained shortlist";
  }
  return normalizeText(value).replace(/[_-]+/g, " ");
}

function readPiniaState(storeId: string): Record<string, unknown> | null {
  const pinia = resolveRuntimePinia();
  if (!pinia) return null;
  const root = pinia.state.value as Record<string, unknown>;
  const row = root[storeId];
  if (!row || typeof row !== "object") return null;
  return row as Record<string, unknown>;
}

function currentMlArtifactSnapshot(): MlSignalArtifactLike | null {
  const pinia = resolveRuntimePinia();
  if (!pinia) return null;
  const artifacts = useWorkflowArtifactsStore(pinia);
  const artifact = artifacts.currentRun?.mlSignal;
  if (!artifact || typeof artifact !== "object") return null;
  return artifact as MlSignalArtifactLike;
}

function markOptimizeUpdated(): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  const optimize = useOptimizeStore(pinia);
  optimize.source = "vue";
  optimize.updatedAtUnixMs = Date.now();
}

function patchOptimizeState(patch: Record<string, unknown>): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  const optimize = useOptimizeStore(pinia);
  Object.assign(optimize, patch);
  optimize.source = "vue";
  optimize.updatedAtUnixMs = Date.now();
}

function patchCurrentMlSignalArtifact(patch: Record<string, unknown>): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  const artifacts = useWorkflowArtifactsStore(pinia);
  if (!artifacts.currentRun) return;
  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ...artifacts.currentRun,
        ml_signal: {
          ...(artifacts.currentRun.mlSignal ?? {}),
          ...patch,
        },
      },
      currentValidation: artifacts.currentValidation,
    },
    "vue",
  );
}

function setMlBusy(message: string): void {


  patchOptimizeState({
    runDisabled: true,
    validateDisabled: true,
    canCancel: true,
    progressMeta: message,
    statusLine: message,
  });
}

function clearMlBusy(statusLine?: string, progressMeta?: string): void {
  patchOptimizeState({
    runDisabled: false,
    validateDisabled: false,
    canCancel: false,
    ...(statusLine != null ? { statusLine } : {}),
    ...(progressMeta != null ? { progressMeta } : {}),
  });
}

function requireDataContext(): {
  dataId: string;
  csvPath: string;
  timeframe: string;
  datasetLabel: string;
} {
  const data = readPiniaState("data");
  const dataId = normalizeText(data?.dataId);
  if (!dataId || dataId === "—") {
    throw new Error("Load a dataset first.");
  }
  return {
    dataId,
    csvPath: normalizeText(data?.csvPath),
    timeframe: normalizeText(data?.dataTimeframe),
    datasetLabel: normalizeText(data?.datasetLabel),
  };
}

function buildRunDatasetContextState(): {
  currentDataMeta: Record<string, unknown> | null;
  currentBaseDataMeta: Record<string, unknown> | null;
  currentBaseDatasetCtx: Record<string, unknown>;
  currentDatasetCtx: Record<string, unknown>;
} {
  const data = readPiniaState("data");
  const currentDataMeta =
    data?.baseDataMeta && typeof data.baseDataMeta === "object"
      ? (data.baseDataMeta as Record<string, unknown>)
      : null;
  const label = normalizeText(data?.datasetLabel);
  const timeframe = normalizeText(
    data?.dataTimeframe || currentDataMeta?.timeframe,
  );
  const source = normalizeText(data?.datasetContext);
  const symbol = normalizeText(data?.polygonSymbol || label);

  const datasetCtx: Record<string, unknown> = {};
  if (label && label !== "—") datasetCtx.label = label;
  if (symbol && symbol !== "—") datasetCtx.symbol = symbol;
  if (timeframe) {
    datasetCtx.timeframe = timeframe;
    datasetCtx.base_timeframe = timeframe;
    datasetCtx.derived_timeframe = timeframe;
    datasetCtx.resample_applied = false;
  }
  if (source && source !== "—") datasetCtx.source = source;

  return {
    currentDataMeta,
    currentBaseDataMeta: currentDataMeta,
    currentBaseDatasetCtx: datasetCtx,
    currentDatasetCtx: datasetCtx,
  };
}

function currentRoundTripCostBps(): number {
  const pinia = resolveRuntimePinia();
  if (!pinia) return 0;
  const strategy = useStrategyStore(pinia);
  const fee = Math.max(0, parseNumber(strategy.feeBps, 0));
  const spread = Math.max(0, parseNumber(strategy.spreadBps, 0));
  const slippage = Math.max(0, parseNumber(strategy.slippageBps, 0));
  return (fee + slippage + spread * 0.5) * 2;
}

function buildMlStrategyConfig(): Record<string, unknown> {
  const pinia = resolveRuntimePinia();
  if (!pinia) return {};
  const strategy = useStrategyStore(pinia);
  return {
    long_only: Boolean(strategy.longOnly),
    allow_flip: Boolean(strategy.allowFlip),
    trade_on_next_bar: Boolean(strategy.tradeOnNext),
    cost_preset: normalizeText(strategy.costPreset) || undefined,
    fee_bps: parseNumber(strategy.feeBps, 0),
    spread_bps: parseNumber(strategy.spreadBps, 0),
    slippage_bps: parseNumber(strategy.slippageBps, 0),
    stop_loss_pct: parseOptionalPositive(strategy.stopLossPct),
    take_profit_pct: parseOptionalPositive(strategy.takeProfitPct),
    cooldown_bars: Math.max(
      0,
      Math.floor(parseNumber(strategy.cooldownBars, 0)),
    ),
    signal_persistence_bars: Math.max(
      1,
      Math.floor(parseNumber(strategy.signalPersistenceBars, 1)),
    ),
    eps_rel: parseNumber(strategy.epsRel, 0),
  };
}

function buildValidationContext(): Record<string, unknown> {
  const data = requireDataContext();
  const ctx: Record<string, unknown> = {
    strategy_kind: "MlSignal",
  };
  if (data.csvPath) ctx.csv_path = data.csvPath;
  if (data.timeframe) ctx.timeframe = data.timeframe;
  if (data.datasetLabel && data.datasetLabel !== "—")
    ctx.label = data.datasetLabel;
  return ctx;
}

function buildMlSignalArtifactPayload(
  runReq: Record<string, unknown>,
  runRes: unknown,
  validationReq?: Record<string, unknown> | null,
  validationRes?: unknown,
): Record<string, unknown> {
  const pinia = resolveRuntimePinia();
  const priorMlSignal = pinia
    ? (useWorkflowArtifactsStore(pinia).currentRun?.mlSignal as
        | WorkflowRunArtifact["mlSignal"]
        | null)
    : null;
  const runResRecord =
    runRes && typeof runRes === "object"
      ? (runRes as Record<string, unknown>)
      : null;
  const validationResRecord =
    validationRes && typeof validationRes === "object"
      ? (validationRes as Record<string, unknown>)
      : null;
  const validationScoreSummary =
    resolveMlScoreSummaryRecord(validationResRecord);
  const prior = (priorMlSignal ?? {}) as MlSignalArtifactLike;
  const scoreCacheContractMode = normalizeMlScoreCacheContractMode(
    firstMeaningfulValue(
      validationScoreSummary?.score_cache_contract_mode,
      validationScoreSummary?.scoreCacheContractMode,
      validationReq?.score_cache_contract_mode,
      validationReq?.scoreCacheContractMode,
      runReq.score_cache_contract_mode,
      runReq.scoreCacheContractMode,
      prior.score_cache_contract_mode,
      prior.scoreCacheContractMode,
    ),
  );
  const exporter = resolveMlArtifactMetadataRecord(
    [
      validationScoreSummary?.exporter,
      validationScoreSummary?.exporter_metadata,
      runResRecord?.exporter,
      runResRecord?.exporter_metadata,
      prior.exporter,
      prior.exporter_metadata,
      prior.exporterMetadata,
    ],
    [
      [
        "exporter_version",
        firstMeaningfulValue(
          validationScoreSummary?.exporter_version,
          validationScoreSummary?.exporterVersion,
          runResRecord?.exporter_version,
          runResRecord?.exporterVersion,
          prior.exporter_version,
          prior.exporterVersion,
          readOptionalRecord(prior.exporter)?.exporter_version,
        ),
      ],
      [
        "created_at",
        firstMeaningfulValue(
          validationScoreSummary?.created_at,
          validationScoreSummary?.createdAt,
          runResRecord?.created_at,
          runResRecord?.createdAt,
          prior.created_at,
          prior.createdAt,
          readOptionalRecord(prior.exporter)?.created_at,
        ),
      ],
    ],
  );
  const provenance = resolveMlArtifactMetadataRecord(
    [
      validationScoreSummary?.provenance,
      runResRecord?.provenance,
      prior.provenance,
    ],
    [
      [
        "indicator_config",
        firstMeaningfulValue(
          validationScoreSummary?.indicator_config,
          validationScoreSummary?.indicatorConfig,
          runResRecord?.indicator_config,
          runResRecord?.indicatorConfig,
          prior.indicator_config,
          prior.indicatorConfig,
        ),
      ],
      [
        "warmup_policy",
        firstMeaningfulValue(
          validationScoreSummary?.warmup_policy,
          validationScoreSummary?.warmupPolicy,
          runResRecord?.warmup_policy,
          runResRecord?.warmupPolicy,
          prior.warmup_policy,
          prior.warmupPolicy,
        ),
      ],
      [
        "label_policy",
        firstMeaningfulValue(
          validationScoreSummary?.label_policy,
          validationScoreSummary?.labelPolicy,
          runResRecord?.label_policy,
          runResRecord?.labelPolicy,
          prior.label_policy,
          prior.labelPolicy,
        ),
      ],
      [
        "deadband_policy",
        firstMeaningfulValue(
          validationScoreSummary?.deadband_policy,
          validationScoreSummary?.deadbandPolicy,
          runResRecord?.deadband_policy,
          runResRecord?.deadbandPolicy,
          prior.deadband_policy,
          prior.deadbandPolicy,
        ),
      ],
      [
        "sample_weight_policy",
        firstMeaningfulValue(
          validationScoreSummary?.sample_weight_policy,
          validationScoreSummary?.sampleWeightPolicy,
          runResRecord?.sample_weight_policy,
          runResRecord?.sampleWeightPolicy,
          prior.sample_weight_policy,
          prior.sampleWeightPolicy,
        ),
      ],
      [
        "data_timeframe",
        firstMeaningfulValue(
          validationScoreSummary?.data_timeframe,
          validationScoreSummary?.dataTimeframe,
          runResRecord?.data_timeframe,
          runResRecord?.dataTimeframe,
          prior.data_timeframe,
          prior.dataTimeframe,
        ),
      ],
      [
        "source_export_manifest_hash",
        firstMeaningfulValue(
          validationScoreSummary?.source_export_manifest_hash,
          validationScoreSummary?.sourceExportManifestHash,
          runResRecord?.source_export_manifest_hash,
          runResRecord?.sourceExportManifestHash,
          prior.source_export_manifest_hash,
          prior.sourceExportManifestHash,
        ),
      ],
      [
        "source_feature_schema_hash",
        firstMeaningfulValue(
          validationScoreSummary?.source_feature_schema_hash,
          validationScoreSummary?.sourceFeatureSchemaHash,
          runResRecord?.source_feature_schema_hash,
          runResRecord?.sourceFeatureSchemaHash,
          prior.source_feature_schema_hash,
          prior.sourceFeatureSchemaHash,
        ),
      ],
      [
        "training_config",
        firstMeaningfulValue(
          validationScoreSummary?.training_config,
          validationScoreSummary?.trainingConfig,
          runResRecord?.training_config,
          runResRecord?.trainingConfig,
          prior.training_config,
          prior.trainingConfig,
        ),
      ],
      [
        "training_windows_summary",
        firstMeaningfulValue(
          validationScoreSummary?.training_windows_summary,
          validationScoreSummary?.trainingWindowsSummary,
          runResRecord?.training_windows_summary,
          runResRecord?.trainingWindowsSummary,
          prior.training_windows_summary,
          prior.trainingWindowsSummary,
        ),
      ],
      [
        "calibration_summary",
        firstMeaningfulValue(
          validationScoreSummary?.calibration_summary,
          validationScoreSummary?.calibrationSummary,
          runResRecord?.calibration_summary,
          runResRecord?.calibrationSummary,
          prior.calibration_summary,
          prior.calibrationSummary,
        ),
      ],
      [
        "class_balance_policy",
        firstMeaningfulValue(
          validationScoreSummary?.class_balance_policy,
          validationScoreSummary?.classBalancePolicy,
          runResRecord?.class_balance_policy,
          runResRecord?.classBalancePolicy,
          prior.class_balance_policy,
          prior.classBalancePolicy,
        ),
      ],
      [
        "threshold_policy_default",
        firstMeaningfulValue(
          validationScoreSummary?.threshold_policy_default,
          validationScoreSummary?.thresholdPolicyDefault,
          runResRecord?.threshold_policy_default,
          runResRecord?.thresholdPolicyDefault,
          prior.threshold_policy_default,
          prior.thresholdPolicyDefault,
        ),
      ],
      [
        "explainability_artifacts",
        firstMeaningfulValue(
          validationScoreSummary?.explainability_artifacts,
          validationScoreSummary?.explainabilityArtifacts,
          runResRecord?.explainability_artifacts,
          runResRecord?.explainabilityArtifacts,
          prior.explainability_artifacts,
          prior.explainabilityArtifacts,
        ),
      ],
      [
        "bundle_parent_id",
        firstMeaningfulValue(
          validationScoreSummary?.bundle_parent_id,
          validationScoreSummary?.bundleParentId,
          runResRecord?.bundle_parent_id,
          runResRecord?.bundleParentId,
          prior.bundle_parent_id,
          prior.bundleParentId,
        ),
      ],
      [
        "feature_block_versions",
        firstMeaningfulValue(
          validationScoreSummary?.feature_block_versions,
          validationScoreSummary?.featureBlockVersions,
          runResRecord?.feature_block_versions,
          runResRecord?.featureBlockVersions,
          prior.feature_block_versions,
          prior.featureBlockVersions,
        ),
      ],
      [
        "indicator_params",
        firstMeaningfulValue(
          validationScoreSummary?.indicator_params,
          validationScoreSummary?.indicatorParams,
          runResRecord?.indicator_params,
          runResRecord?.indicatorParams,
          prior.indicator_params,
          prior.indicatorParams,
        ),
      ],
      [
        "label_horizon_bars",
        firstMeaningfulValue(
          validationScoreSummary?.label_horizon_bars,
          validationScoreSummary?.labelHorizonBars,
          runResRecord?.label_horizon_bars,
          runResRecord?.labelHorizonBars,
          prior.label_horizon_bars,
          prior.labelHorizonBars,
        ),
      ],
      [
        "first_valid_reasoning",
        firstMeaningfulValue(
          validationScoreSummary?.first_valid_reasoning,
          validationScoreSummary?.firstValidReasoning,
          runResRecord?.first_valid_reasoning,
          runResRecord?.firstValidReasoning,
          prior.first_valid_reasoning,
          prior.firstValidReasoning,
        ),
      ],
    ],
  );
  const calibration = resolveMlArtifactMetadataRecord(
    [
      validationScoreSummary?.calibration,
      runResRecord?.calibration,
      prior.calibration,
    ],
    [
      [
        "calibration_summary",
        firstMeaningfulValue(
          validationScoreSummary?.calibration_summary,
          validationScoreSummary?.calibrationSummary,
          runResRecord?.calibration_summary,
          runResRecord?.calibrationSummary,
          prior.calibration_summary,
          prior.calibrationSummary,
          readOptionalRecord(prior.calibration)?.calibration_summary,
        ),
      ],
      [
        "calibration_state",
        firstMeaningfulValue(
          validationScoreSummary?.calibration_state,
          validationScoreSummary?.calibrationState,
          runResRecord?.calibration_state,
          runResRecord?.calibrationState,
          prior.calibration_state,
          prior.calibrationState,
          readOptionalRecord(prior.calibration)?.calibration_state,
        ),
      ],
    ],
  );
  const scoreContract = resolveMlArtifactMetadataRecord(
    [
      validationScoreSummary?.score_contract,
      runResRecord?.score_contract,
      prior.score_contract,
      prior.scoreContract,
    ],
    [
      [
        "score_contract_version",
        firstMeaningfulValue(
          validationScoreSummary?.score_contract_version,
          validationScoreSummary?.scoreContractVersion,
          runResRecord?.score_contract_version,
          runResRecord?.scoreContractVersion,
          prior.score_contract_version,
          prior.scoreContractVersion,
        ),
      ],
      [
        "native_backend_requested",
        firstMeaningfulValue(
          validationScoreSummary?.native_backend_requested,
          validationScoreSummary?.nativeBackendRequested,
          runResRecord?.native_backend_requested,
          runResRecord?.nativeBackendRequested,
          prior.native_backend_requested,
          prior.nativeBackendRequested,
        ),
      ],
      [
        "native_backend_used",
        firstMeaningfulValue(
          validationScoreSummary?.native_backend_used,
          validationScoreSummary?.nativeBackendUsed,
          runResRecord?.native_backend_used,
          runResRecord?.nativeBackendUsed,
          prior.native_backend_used,
          prior.nativeBackendUsed,
        ),
      ],
      [
        "native_library_path",
        firstMeaningfulValue(
          validationScoreSummary?.native_library_path,
          validationScoreSummary?.nativeLibraryPath,
          runResRecord?.native_library_path,
          runResRecord?.nativeLibraryPath,
          prior.native_library_path,
          prior.nativeLibraryPath,
        ),
      ],
      [
        "native_gpu_device_id",
        firstMeaningfulValue(
          validationScoreSummary?.native_gpu_device_id,
          validationScoreSummary?.nativeGpuDeviceId,
          runResRecord?.native_gpu_device_id,
          runResRecord?.nativeGpuDeviceId,
          prior.native_gpu_device_id,
          prior.nativeGpuDeviceId,
        ),
      ],
      [
        "native_prediction_batch_size",
        firstMeaningfulValue(
          validationScoreSummary?.native_prediction_batch_size,
          validationScoreSummary?.nativePredictionBatchSize,
          runResRecord?.native_prediction_batch_size,
          runResRecord?.nativePredictionBatchSize,
          prior.native_prediction_batch_size,
          prior.nativePredictionBatchSize,
        ),
      ],
      [
        "native_fallback_reason",
        firstMeaningfulValue(
          validationScoreSummary?.native_fallback_reason,
          validationScoreSummary?.nativeFallbackReason,
          runResRecord?.native_fallback_reason,
          runResRecord?.nativeFallbackReason,
          prior.native_fallback_reason,
          prior.nativeFallbackReason,
        ),
      ],
    ],
  );
  const scoreCacheProvenance = resolveMlArtifactMetadataRecord(
    [
      validationScoreSummary?.score_cache_provenance,
      runResRecord?.score_cache_provenance,
      prior.score_cache_provenance,
      prior.scoreCacheProvenance,
    ],
    [
      [
        "source_bundle_id",
        firstMeaningfulValue(
          validationScoreSummary?.source_bundle_id,
          validationScoreSummary?.sourceBundleId,
          runResRecord?.source_bundle_id,
          runResRecord?.sourceBundleId,
          prior.source_bundle_id,
          prior.sourceBundleId,
        ),
      ],
      [
        "source_bundle_manifest_path",
        firstMeaningfulValue(
          validationScoreSummary?.source_bundle_manifest_path,
          validationScoreSummary?.sourceBundleManifestPath,
          runResRecord?.source_bundle_manifest_path,
          runResRecord?.sourceBundleManifestPath,
          prior.source_bundle_manifest_path,
          prior.sourceBundleManifestPath,
        ),
      ],
      [
        "source_bundle_manifest_hash",
        firstMeaningfulValue(
          validationScoreSummary?.source_bundle_manifest_hash,
          validationScoreSummary?.sourceBundleManifestHash,
          runResRecord?.source_bundle_manifest_hash,
          runResRecord?.sourceBundleManifestHash,
          prior.source_bundle_manifest_hash,
          prior.sourceBundleManifestHash,
        ),
      ],
      [
        "bundle_manifest_sha256",
        firstMeaningfulValue(
          validationScoreSummary?.bundle_manifest_sha256,
          validationScoreSummary?.bundleManifestSha256,
          runResRecord?.bundle_manifest_sha256,
          runResRecord?.bundleManifestSha256,
          prior.bundle_manifest_sha256,
          prior.bundleManifestSha256,
        ),
      ],
      [
        "model_long_sha256",
        firstMeaningfulValue(
          validationScoreSummary?.model_long_sha256,
          validationScoreSummary?.modelLongSha256,
          runResRecord?.model_long_sha256,
          runResRecord?.modelLongSha256,
          prior.model_long_sha256,
          prior.modelLongSha256,
        ),
      ],
      [
        "model_short_sha256",
        firstMeaningfulValue(
          validationScoreSummary?.model_short_sha256,
          validationScoreSummary?.modelShortSha256,
          runResRecord?.model_short_sha256,
          runResRecord?.modelShortSha256,
          prior.model_short_sha256,
          prior.modelShortSha256,
        ),
      ],
      [
        "feature_schema_sha256",
        firstMeaningfulValue(
          validationScoreSummary?.feature_schema_sha256,
          validationScoreSummary?.featureSchemaSha256,
          runResRecord?.feature_schema_sha256,
          runResRecord?.featureSchemaSha256,
          prior.feature_schema_sha256,
          prior.featureSchemaSha256,
        ),
      ],
      [
        "export_manifest_sha256",
        firstMeaningfulValue(
          validationScoreSummary?.export_manifest_sha256,
          validationScoreSummary?.exportManifestSha256,
          runResRecord?.export_manifest_sha256,
          runResRecord?.exportManifestSha256,
          prior.export_manifest_sha256,
          prior.exportManifestSha256,
        ),
      ],
      [
        "training_config_hash",
        firstMeaningfulValue(
          validationScoreSummary?.training_config_hash,
          validationScoreSummary?.trainingConfigHash,
          runResRecord?.training_config_hash,
          runResRecord?.trainingConfigHash,
          prior.training_config_hash,
          prior.trainingConfigHash,
        ),
      ],
      [
        "native_library_sha256",
        firstMeaningfulValue(
          validationScoreSummary?.native_library_sha256,
          validationScoreSummary?.nativeLibrarySha256,
          runResRecord?.native_library_sha256,
          runResRecord?.nativeLibrarySha256,
          prior.native_library_sha256,
          prior.nativeLibrarySha256,
        ),
      ],
      [
        "native_library_discovery_policy",
        firstMeaningfulValue(
          validationScoreSummary?.native_library_discovery_policy,
          validationScoreSummary?.nativeLibraryDiscoveryPolicy,
          runResRecord?.native_library_discovery_policy,
          runResRecord?.nativeLibraryDiscoveryPolicy,
          prior.native_library_discovery_policy,
          prior.nativeLibraryDiscoveryPolicy,
        ),
      ],
      [
        "native_prediction_batch_size",
        firstMeaningfulValue(
          validationScoreSummary?.native_prediction_batch_size,
          validationScoreSummary?.nativePredictionBatchSize,
          runResRecord?.native_prediction_batch_size,
          runResRecord?.nativePredictionBatchSize,
          prior.native_prediction_batch_size,
          prior.nativePredictionBatchSize,
        ),
      ],
      [
        "score_scope_policy",
        firstMeaningfulValue(
          validationScoreSummary?.score_scope_policy,
          validationScoreSummary?.scoreScopePolicy,
          runResRecord?.score_scope_policy,
          runResRecord?.scoreScopePolicy,
          prior.score_scope_policy,
          prior.scoreScopePolicy,
        ),
      ],
      [
        "calibration_state",
        firstMeaningfulValue(
          validationScoreSummary?.calibration_state,
          validationScoreSummary?.calibrationState,
          runResRecord?.calibration_state,
          runResRecord?.calibrationState,
          prior.calibration_state,
          prior.calibrationState,
          readOptionalRecord(prior.score_cache_provenance)?.calibration_state,
        ),
      ],
      [
        "threshold_policy",
        firstMeaningfulValue(
          validationScoreSummary?.threshold_policy,
          validationScoreSummary?.thresholdPolicy,
          runResRecord?.threshold_policy,
          runResRecord?.thresholdPolicy,
          prior.threshold_policy,
          prior.thresholdPolicy,
        ),
      ],
      [
        "producer_kind",
        firstMeaningfulValue(
          validationScoreSummary?.producer_kind,
          validationScoreSummary?.producerKind,
          runResRecord?.producer_kind,
          runResRecord?.producerKind,
          prior.producer_kind,
          prior.producerKind,
        ),
      ],
      [
        "score_contract_version",
        firstMeaningfulValue(
          validationScoreSummary?.score_contract_version,
          validationScoreSummary?.scoreContractVersion,
          runResRecord?.score_contract_version,
          runResRecord?.scoreContractVersion,
          prior.score_contract_version,
          prior.scoreContractVersion,
        ),
      ],
      [
        "score_cache_contract_mode",
        firstMeaningfulValue(
          validationScoreSummary?.score_cache_contract_mode,
          validationScoreSummary?.scoreCacheContractMode,
          validationReq?.score_cache_contract_mode,
          validationReq?.scoreCacheContractMode,
          runReq.score_cache_contract_mode,
          runReq.scoreCacheContractMode,
          prior.score_cache_contract_mode,
          prior.scoreCacheContractMode,
          scoreCacheContractMode,
        ),
      ],
    ],
  );
  const bundleStatus = resolveMlArtifactMetadataRecord(
    [
      validationScoreSummary?.bundle_status_details,
      validationScoreSummary?.bundle_status,
      runResRecord?.bundle_status_details,
      runResRecord?.bundle_status,
      prior.bundle_status_details,
      prior.bundle_status,
      prior.bundleStatusDetails,
      prior.bundleStatus,
    ],
    [],
  );
  const scoreContractVersion = normalizeText(
    firstMeaningfulValue(
      validationScoreSummary?.score_contract_version,
      validationScoreSummary?.scoreContractVersion,
      runResRecord?.score_contract_version,
      runResRecord?.scoreContractVersion,
      prior.score_contract_version,
      prior.scoreContractVersion,
      scoreContract?.score_contract_version,
      scoreCacheProvenance?.score_contract_version,
    ),
  );
  const stage =
    validationReq && validationRes
      ? Array.isArray(validationReq.candidates)
        ? "validation"
        : "policy_sweep"
      : normalizeText(runReq.ma_source ?? runReq.maSource) ||
          parseOptionalInteger(runReq.horizon_bars ?? runReq.horizonBars) !=
            null
        ? "dataset_export"
        : "score_cache";
  const rowCount = parseOptionalInteger(
    runResRecord?.row_count ?? prior.row_count ?? prior.rowCount,
  );
  const featureCount = parseOptionalInteger(
    runResRecord?.feature_count ?? prior.feature_count ?? prior.featureCount,
  );
  const splitCount = parseOptionalInteger(
    runResRecord?.split_count ?? prior.split_count ?? prior.splitCount,
  );
  const matchedRows = parseOptionalInteger(
    validationScoreSummary?.matched_rows ??
      runResRecord?.matched_rows ??
      prior.matched_rows ??
      prior.matchedRows,
  );
  const skippedRows = parseOptionalInteger(
    validationScoreSummary?.skipped_rows ??
      runResRecord?.skipped_rows ??
      prior.skipped_rows ??
      prior.skippedRows,
  );
  const barsScored = parseOptionalInteger(
    validationScoreSummary?.bars_scored ??
      runResRecord?.bars_scored ??
      prior.bars_scored ??
      prior.barsScored,
  );
  const coverageRatio = parseOptionalNumber(
    validationScoreSummary?.coverage_ratio ??
      runResRecord?.coverage_ratio ??
      prior.coverage_ratio ??
      prior.coverageRatio,
  );
  const duplicateTimestamps = parseOptionalInteger(
    validationScoreSummary?.duplicate_timestamps ??
      runResRecord?.duplicate_timestamps ??
      prior.duplicate_timestamps ??
      prior.duplicateTimestamps,
  );
  const featureSchemaHash = normalizeText(
    firstMeaningfulValue(
      validationScoreSummary?.feature_schema_hash,
      validationScoreSummary?.featureSchemaHash,
      validationScoreSummary?.schema_hash,
      validationScoreSummary?.schemaHash,
      runResRecord?.feature_schema_hash,
      runResRecord?.featureSchemaHash,
      runResRecord?.schema_hash,
      runResRecord?.schemaHash,
      prior.feature_schema_hash,
      prior.featureSchemaHash,
      prior.schema_hash,
      prior.schemaHash,
    ),
  );
  const scoreCachePath =
    stage === "dataset_export"
      ? normalizeText(prior.score_cache_path ?? prior.scoreCachePath ?? "")
      : normalizeText(validationReq?.score_cache_path) ||
        normalizeText(
          runResRecord?.output_path ??
            runReq.output_path ??
            runReq.outputPath ??
            "",
        ) ||
        normalizeText(prior.score_cache_path ?? prior.scoreCachePath ?? "");
  return {
    stage,
    bundle_id:
      normalizeText(runReq.model_bundle_id) ||
      normalizeText(
        runResRecord?.model_bundle_id ??
          prior.bundle_id ??
          prior.bundleId ??
          "",
      ),
    source_bundle_manifest_path: normalizeText(
      firstMeaningfulValue(
        runResRecord?.source_bundle_manifest_path,
        runResRecord?.sourceBundleManifestPath,
        prior.source_bundle_manifest_path,
        prior.sourceBundleManifestPath,
      ),
    ),
    dataset_export_path:
      stage === "dataset_export"
        ? normalizeText(
            runResRecord?.output_path ??
              runReq.output_path ??
              runReq.outputPath ??
              prior.dataset_export_path ??
              prior.datasetExportPath ??
              "",
          )
        : normalizeText(
            prior.dataset_export_path ?? prior.datasetExportPath ?? "",
          ),
    export_manifest_path: normalizeText(
      runResRecord?.export_manifest_path ??
        prior.export_manifest_path ??
        prior.exportManifestPath ??
        "",
    ),
    split_manifest_path: normalizeText(
      runResRecord?.split_manifest_path ??
        prior.split_manifest_path ??
        prior.splitManifestPath ??
        "",
    ),
    row_count: rowCount,
    feature_count: featureCount,
    split_count: splitCount,
    feature_schema_version: parseNumber(
      runResRecord?.feature_schema_version ??
        prior.feature_schema_version ??
        prior.featureSchemaVersion ??
        1,
      1,
    ),
    feature_schema_hash: featureSchemaHash,
    feature_schema_path: normalizeText(
      runResRecord?.feature_schema_path ??
        prior.feature_schema_path ??
        prior.featureSchemaPath ??
        "",
    ),
    exporter,
    provenance,
    calibration,
    score_contract: scoreContract,
    score_contract_version: scoreContractVersion,
    score_cache_contract_mode: scoreCacheContractMode,
    score_cache_provenance: scoreCacheProvenance,
    bundle_status: bundleStatus,
    score_cache_path: scoreCachePath,
    score_manifest_path: normalizeText(
      runResRecord?.manifest_path ??
        validationScoreSummary?.score_manifest_path ??
        prior.score_manifest_path ??
        prior.scoreManifestPath ??
        "",
    ),
    score_space: normalizeText(
      validationScoreSummary?.score_space ??
        runResRecord?.score_space ??
        prior.score_space ??
        prior.scoreSpace ??
        "",
    ),
    score_scope: normalizeText(
      validationScoreSummary?.score_scope ??
        runResRecord?.score_scope ??
        prior.score_scope ??
        prior.scoreScope ??
        "",
    ),
    score_scope_policy: normalizeText(
      validationScoreSummary?.score_scope_policy ??
        validationScoreSummary?.scoreScopePolicy ??
        runResRecord?.score_scope_policy ??
        runResRecord?.scoreScopePolicy ??
        prior.score_scope_policy ??
        prior.scoreScopePolicy ??
        "",
    ),
    raw_score_observations: parseOptionalInteger(
      validationScoreSummary?.raw_score_observations ??
        validationScoreSummary?.rawScoreObservations ??
        runResRecord?.raw_score_observations ??
        runResRecord?.rawScoreObservations ??
        prior.raw_score_observations ??
        prior.rawScoreObservations,
    ),
    effective_score_observations: parseOptionalInteger(
      validationScoreSummary?.effective_score_observations ??
        validationScoreSummary?.effectiveScoreObservations ??
        runResRecord?.effective_score_observations ??
        runResRecord?.effectiveScoreObservations ??
        prior.effective_score_observations ??
        prior.effectiveScoreObservations,
    ),
    matched_rows: matchedRows,
    skipped_rows: skippedRows,
    bars_scored: barsScored,
    coverage_ratio: coverageRatio,
    duplicate_timestamps: duplicateTimestamps,
    native_backend_requested: normalizeText(
      firstMeaningfulValue(
        validationScoreSummary?.native_backend_requested,
        validationScoreSummary?.nativeBackendRequested,
        runResRecord?.native_backend_requested,
        runResRecord?.nativeBackendRequested,
        prior.native_backend_requested,
        prior.nativeBackendRequested,
      ),
    ),
    native_backend_used: normalizeText(
      firstMeaningfulValue(
        validationScoreSummary?.native_backend_used,
        validationScoreSummary?.nativeBackendUsed,
        runResRecord?.native_backend_used,
        runResRecord?.nativeBackendUsed,
        prior.native_backend_used,
        prior.nativeBackendUsed,
      ),
    ),
    native_library_path: normalizeText(
      firstMeaningfulValue(
        validationScoreSummary?.native_library_path,
        validationScoreSummary?.nativeLibraryPath,
        runResRecord?.native_library_path,
        runResRecord?.nativeLibraryPath,
        prior.native_library_path,
        prior.nativeLibraryPath,
      ),
    ),
    native_library_sha256: normalizeText(
      firstMeaningfulValue(
        validationScoreSummary?.native_library_sha256,
        validationScoreSummary?.nativeLibrarySha256,
        runResRecord?.native_library_sha256,
        runResRecord?.nativeLibrarySha256,
        prior.native_library_sha256,
        prior.nativeLibrarySha256,
      ),
    ),
    native_library_discovery_policy: normalizeText(
      firstMeaningfulValue(
        validationScoreSummary?.native_library_discovery_policy,
        validationScoreSummary?.nativeLibraryDiscoveryPolicy,
        runResRecord?.native_library_discovery_policy,
        runResRecord?.nativeLibraryDiscoveryPolicy,
        prior.native_library_discovery_policy,
        prior.nativeLibraryDiscoveryPolicy,
      ),
    ),
    native_gpu_device_id: parseOptionalInteger(
      firstMeaningfulValue(
        validationScoreSummary?.native_gpu_device_id,
        validationScoreSummary?.nativeGpuDeviceId,
        runResRecord?.native_gpu_device_id,
        runResRecord?.nativeGpuDeviceId,
        prior.native_gpu_device_id,
        prior.nativeGpuDeviceId,
      ),
    ),
    native_prediction_batch_size: parseOptionalInteger(
      firstMeaningfulValue(
        validationScoreSummary?.native_prediction_batch_size,
        validationScoreSummary?.nativePredictionBatchSize,
        runResRecord?.native_prediction_batch_size,
        runResRecord?.nativePredictionBatchSize,
        prior.native_prediction_batch_size,
        prior.nativePredictionBatchSize,
      ),
    ),
    native_fallback_reason: normalizeText(
      firstMeaningfulValue(
        validationScoreSummary?.native_fallback_reason,
        validationScoreSummary?.nativeFallbackReason,
        runResRecord?.native_fallback_reason,
        runResRecord?.nativeFallbackReason,
        prior.native_fallback_reason,
        prior.nativeFallbackReason,
      ),
    ),
    deployability:
      readOptionalRecord(validationResRecord?.deployability) ??
      readOptionalRecord(runResRecord?.deployability) ??
      readOptionalRecord(prior.deployability) ??
      null,
    deployability_state: normalizeText(
      firstMeaningfulValue(
        readOptionalRecord(validationResRecord?.deployability)?.state,
        validationResRecord?.deployability_state,
        validationResRecord?.deployabilityState,
        readOptionalRecord(runResRecord?.deployability)?.state,
        runResRecord?.deployability_state,
        runResRecord?.deployabilityState,
        readOptionalRecord(prior.deployability)?.state,
        prior.deployability_state,
        prior.deployabilityState,
      ),
    ),
    evidence_policy_summary:
      resolveMlEvidencePolicySummaryRecord(validationResRecord) ??
      readOptionalRecord(prior.evidence_policy_summary) ??
      readOptionalRecord(prior.evidencePolicySummary) ??
      null,
    policy:
      validationReq && validationRes
        ? extractMlPolicy(validationReq, validationRes)
        : null,
    policy_sweep:
      validationReq && validationRes
        ? extractMlPolicySweepSummary(validationRes)
        : null,
  };
}

function buildRunArtifactPayload(
  req: Record<string, unknown>,
  res: unknown,
  ts: string,
): {
  artifact: Record<string, unknown>;
  csvPath: string;
} {
  const data = requireDataContext();
  const datasetMeta = buildDatasetMetaForArtifact(
    data.csvPath,
    buildRunDatasetContextState(),
  );
  const artifact: Record<string, unknown> = {
    version: 1,
    kind: "vectorgrid_run_artifact",
    ts,
    config_fingerprint: computeConfigFingerprint(req, null),
    run: { req, res },
    validation: null,
    report_xlsx_path: "",
    data: { csv_path: data.csvPath },
    dataset_meta: datasetMeta,
    ml_signal: buildMlSignalArtifactPayload(req, res),
    tags: [],
  };
  return { artifact, csvPath: data.csvPath };
}

async function persistRunArtifact(
  req: Record<string, unknown>,
  res: unknown,
  ts: string,
): Promise<{ path: string; csvPath: string }> {
  const { artifact, csvPath } = buildRunArtifactPayload(req, res, ts);
  try {
    const saved = (await invokeCommand("save_run_artifact", {
      artifact,
    })) as SaveRunArtifactResponse;
    return {
      path: normalizeText(saved?.path),
      csvPath,
    };
  } catch {
    return { path: "", csvPath };
  }
}

function applyVueRunArtifacts(
  ts: string,
  req: Record<string, unknown>,
  res: unknown,
  artifactPath: string,
  csvPath: string,
): void {
  const datasetMeta = buildDatasetMetaForArtifact(
    csvPath,
    buildRunDatasetContextState(),
  );
  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ts,
        artifact_path: artifactPath,
        report_xlsx_path: "",
        csv_path: csvPath,
        dataset_meta: datasetMeta,
        ml_signal: buildMlSignalArtifactPayload(req, res),
        tags: [],
        req,
        res,
      },
      currentValidation: null,
    },
    "vue",
  );
}

interface PersistedMlValidationArtifact {
  path: string;
  ts: string;
  csvPath: string;
  artifactPersistenceWarning?: string;
}

async function persistValidationArtifact(
  runReq: Record<string, unknown>,
  runRes: unknown,
  validationReq: Record<string, unknown>,
  validationRes: unknown,
): Promise<PersistedMlValidationArtifact> {
  const pinia = resolveRuntimePinia();
  const artifacts = pinia ? useWorkflowArtifactsStore(pinia) : null;
  const currentRun = artifacts?.currentRun as WorkflowRunArtifact | null;
  const data = requireDataContext();
  const ts = normalizeText(currentRun?.ts) || new Date().toISOString();
  const csvPath = normalizeText(currentRun?.csvPath || data.csvPath);
  const datasetMeta = buildDatasetMetaForArtifact(
    csvPath,
    buildRunDatasetContextState(),
  );
  const artifact: Record<string, unknown> = {
    version: 1,
    kind: "vectorgrid_run_artifact",
    ts,
    config_fingerprint: computeConfigFingerprint(runReq, validationReq),
    run: { req: runReq, res: runRes },
    validation: { req: validationReq, res: validationRes },
    report_xlsx_path: "",
    data: { csv_path: csvPath },
    dataset_meta: datasetMeta,
    ml_signal: buildMlSignalArtifactPayload(
      runReq,
      runRes,
      validationReq,
      validationRes,
    ),
    tags: currentRun?.tags ?? [],
  };
  let path = normalizeText(currentRun?.artifactPath);
  let artifactPersistenceWarning = "";
  try {
    const saved = (await invokeCommand("save_run_artifact", {
      artifact,
    })) as SaveRunArtifactResponse;
    path = normalizeText(saved?.path) || path;
  } catch (error) {
    artifactPersistenceWarning =
      normalizeText(error) || "save_run_artifact failed";
  }
  return { path, ts, csvPath, artifactPersistenceWarning };
}

function applyVueValidationArtifacts(
  runReq: Record<string, unknown>,
  runRes: unknown,
  validationReq: Record<string, unknown>,
  validationRes: unknown,
  persisted: PersistedMlValidationArtifact,
): void {
  const pinia = resolveRuntimePinia();
  const artifacts = pinia ? useWorkflowArtifactsStore(pinia) : null;
  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ts: persisted.ts,
        artifact_path: persisted.path,
        report_xlsx_path: "",
        csv_path: persisted.csvPath,
        dataset_meta: buildDatasetMetaForArtifact(
          persisted.csvPath,
          buildRunDatasetContextState(),
        ),
        ml_signal: buildMlSignalArtifactPayload(
          runReq,
          runRes,
          validationReq,
          validationRes,
        ),
        tags: artifacts?.currentRun?.tags ?? [],
        req: runReq,
        res: runRes,
      },
      currentValidation: {
        ts: persisted.ts,
        req: validationReq,
        res: validationRes,
      },
    },
    "vue",
  );
}

function appendMlArtifactPersistenceWarning(
  persisted: PersistedMlValidationArtifact,
  metaFields: Array<"mlSignalValidationMeta" | "mlSignalSweepMeta">,
): void {
  const warning = normalizeText(persisted.artifactPersistenceWarning);
  if (!warning) return;
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  const optimize = useOptimizeStore(pinia);
  const message = `validation artifact not saved: ${warning}`;
  const patch: Record<string, unknown> = {
    statusLine: [normalizeText(optimize.statusLine), message]
      .filter((part) => part)
      .join(" • "),
  };
  for (const field of metaFields) {
    patch[field] = [normalizeText(optimize[field]), message]
      .filter((part) => part)
      .join(" • ");
  }
  patchOptimizeState(patch);
}

function currentRunSnapshot(): {
  req: Record<string, unknown>;
  res: unknown;
} | null {
  const pinia = resolveRuntimePinia();
  if (!pinia) return null;
  const artifacts = useWorkflowArtifactsStore(pinia);
  const run = artifacts.currentRun;
  if (!run || !run.req || typeof run.req !== "object") return null;
  return { req: run.req as Record<string, unknown>, res: run.res };
}

function resolveMlScoreSummaryRecord(
  row: Record<string, unknown> | null,
): Record<string, unknown> | null {
  if (!row) return null;
  const direct =
    row.score_summary && typeof row.score_summary === "object"
      ? (row.score_summary as Record<string, unknown>)
      : null;
  if (direct) return direct;
  const validation =
    row.validation && typeof row.validation === "object"
      ? (row.validation as Record<string, unknown>)
      : null;
  return validation?.score_summary &&
    typeof validation.score_summary === "object"
    ? (validation.score_summary as Record<string, unknown>)
    : row;
}

function formatMlScoreCoverageMeta(
  row: Record<string, unknown> | null,
): string {
  const summary = resolveMlScoreSummaryRecord(row);
  if (!summary) return "";
  const parts: string[] = [];
  const coverageRatio = parseOptionalNumber(
    summary.coverage_ratio ?? summary.coverageRatio,
  );
  const scoreScope = normalizeText(summary.score_scope ?? summary.scoreScope);
  const scoreScopePolicy = normalizeText(
    summary.score_scope_policy ?? summary.scoreScopePolicy,
  );
  const scoreSpace = normalizeText(summary.score_space ?? summary.scoreSpace);
  const scoreCacheContractMode = normalizeText(
    summary.score_cache_contract_mode ?? summary.scoreCacheContractMode,
  );
  const duplicates = parseOptionalInteger(
    summary.duplicate_timestamps ?? summary.duplicateTimestamps,
  );
  if (coverageRatio != null) {
    parts.push(`coverage ${(coverageRatio * 100).toFixed(1)}%`);
  }
  if (scoreScope) parts.push(`scope ${scoreScope}`);
  if (scoreScopePolicy) parts.push(`policy ${scoreScopePolicy}`);
  if (scoreCacheContractMode) {
    parts.push(`contract ${formatMlScoreCacheContractMode(scoreCacheContractMode)}`);
  }
  if (scoreSpace) parts.push(`space ${scoreSpace}`);
  if (duplicates != null) parts.push(`duplicates ${duplicates}`);
  return parts.join(" • ");
}

function formatMlDeployabilityMeta(
  row: Record<string, unknown> | null,
): string {
  if (!row) return "";
  const validation =
    row.validation && typeof row.validation === "object"
      ? (row.validation as Record<string, unknown>)
      : null;
  const deployability =
    readOptionalRecord(row.deployability) ??
    readOptionalRecord(validation?.deployability);
  const state =
    normalizeText(deployability?.state) ||
    normalizeText(row.deployability_state ?? row.deployabilityState);
  if (!state) return "";
  const failed = Array.isArray(deployability?.gates_failed)
    ? deployability.gates_failed.length
    : 0;
  const warnings = Array.isArray(deployability?.warnings)
    ? deployability.warnings.length
    : 0;
  const suffix = [
    failed > 0 ? `failed ${failed}` : "",
    warnings > 0 ? `warnings ${warnings}` : "",
  ]
    .filter((part) => part)
    .join(", ");
  return `deployability ${state}${suffix ? ` (${suffix})` : ""}`;
}

function resolveMlEvidencePolicySummaryRecord(
  value: unknown,
): Record<string, unknown> | null {
  if (!value || typeof value !== "object") return null;
  const row = value as Record<string, unknown>;
  const direct = readOptionalRecord(
    row.evidence_policy_summary ?? row.evidencePolicySummary,
  );
  if (direct) return direct;
  return readOptionalRecord(
    readOptionalRecord(row.validation)?.evidence_policy_summary ??
      readOptionalRecord(row.validation)?.evidencePolicySummary,
  );
}

function formatMlEvidencePolicyMeta(value: unknown): string {
  const summary = resolveMlEvidencePolicySummaryRecord(value);
  const metrics = Array.isArray(summary?.metrics)
    ? summary.metrics
        .map((entry) => readOptionalRecord(entry))
        .filter((entry): entry is Record<string, unknown> => !!entry)
    : [];
  if (!metrics.length) return "";

  const probabilitySampleUnits = metrics
    .filter((metric) => {
      const name = normalizeText(metric.metric).toLowerCase();
      return name === "psr" || name === "dsr";
    })
    .map((metric) => normalizeText(metric.sample_unit))
    .filter(
      (value, index, values) =>
        value.length > 0 && values.indexOf(value) === index,
    );
  const hasDeployabilityLabel = metrics.some(
    (metric) => normalizeText(metric.metric).toLowerCase() === "deployability",
  );
  const parts: string[] = [];
  if (probabilitySampleUnits.length > 0) {
    parts.push(`prob ${probabilitySampleUnits.join("/")}`);
  }
  if (hasDeployabilityLabel) parts.push("deployability metadata");
  return parts.length > 0 ? `evidence ${parts.join(", ")}` : "";
}

function formatMlStageStatus(
  base: string,
  row: Record<string, unknown> | null,
): string {
  const suffix = [formatMlScoreCoverageMeta(row), formatMlDeployabilityMeta(row)]
    .filter((part) => part)
    .join(" • ");
  return suffix ? `${base} • ${suffix}` : base;
}

export async function exportMlSignalDataset(): Promise<unknown> {
  const pinia = resolveRuntimePinia();
  if (!pinia) throw new Error("Pinia is not active.");
  const optimize = useOptimizeStore(pinia);
  const data = requireDataContext();
  const outputPath = normalizeText(optimize.mlSignalDatasetExportPath);
  if (!outputPath) throw new Error("Choose a dataset export CSV path first.");
  const splitTrainBars = parseOptionalInteger(optimize.mlSignalSplitTrainBars);
  const splitCalibrationBars = parseOptionalInteger(
    optimize.mlSignalSplitCalibrationBars,
  );
  const splitTestBars = parseOptionalInteger(optimize.mlSignalSplitTestBars);
  const splitPlan =
    (splitTrainBars ?? 0) > 0 &&
    (splitCalibrationBars ?? 0) > 0 &&
    (splitTestBars ?? 0) > 0
      ? {
          version: 1,
          train_bars: splitTrainBars,
          calibration_bars: splitCalibrationBars,
          test_bars: splitTestBars,
          gap_bars: Math.max(
            0,
            parseOptionalInteger(optimize.mlSignalSplitGapBars) ?? 0,
          ),
          step_bars: Math.max(
            1,
            parseOptionalInteger(optimize.mlSignalSplitStepBars) ?? 1,
          ),
        }
      : null;
  const holdoutModulus = parseOptionalInteger(optimize.mlSignalHoldoutModulus);
  const holdoutResidue = parseOptionalInteger(optimize.mlSignalHoldoutResidue);
  const tickerHoldout =
    (holdoutModulus ?? 0) > 0
      ? {
          modulus: holdoutModulus,
          residue:
            holdoutModulus && holdoutModulus > 0
              ? Math.max(0, holdoutResidue ?? 0) % holdoutModulus
              : 0,
        }
      : null;

  const req = {
    strategy_kind: "MlSignal",
    data_id: data.dataId,
    output_path: outputPath,
    ma_source: normalizeText(optimize.mlSignalMaSource) || "close",
    horizon_bars: Math.max(
      2,
      Math.floor(parseNumber(optimize.mlSignalHorizonBars, 10)),
    ),
    round_trip_cost_bps: currentRoundTripCostBps(),
    split_plan: splitPlan,
    ticker_holdout: tickerHoldout,
  };

  setMlBusy("Exporting ML feature dataset…");
  try {
    const res = await invokeCommand("export_ml_signal_dataset", { req });
    const runTs = new Date().toISOString();
    const resRecord =
      res && typeof res === "object" ? (res as Record<string, unknown>) : null;
    const rowCount = parseNumber(resRecord?.row_count, 0);
    const splitCount = parseOptionalInteger(resRecord?.split_count);
    const splitManifestPath = normalizeText(resRecord?.split_manifest_path);
    const tickerHoldoutEnabled = Boolean(resRecord?.ticker_holdout);
    const metaParts = [
      `Dataset export: ${rowCount.toFixed(0)} rows -> ${outputPath}`,
      splitCount && splitCount > 0 ? `splits ${splitCount}` : "",
      tickerHoldoutEnabled ? "ticker holdout on" : "",
      splitManifestPath ? basename(splitManifestPath) : "",
    ].filter((part) => part);
    const persisted = await persistRunArtifact(req, res, runTs);
    applyVueRunArtifacts(runTs, req, res, persisted.path, persisted.csvPath);
    patchOptimizeState({
      mlSignalDatasetMeta: metaParts.join(" • "),
      progressMeta: `ML dataset export • ${rowCount.toFixed(0)} rows`,
      statusLine: formatMlStageStatus(
        "ML feature dataset exported.",
        resRecord,
      ),
    });
    return res;
  } catch (error) {
    patchOptimizeState({
      statusLine: formatMlActionError("ML dataset export failed", error),
    });
    throw error;
  } finally {
    clearMlBusy();
  }
}

export async function generateMlSignalScoreCache(): Promise<unknown> {
  const pinia = resolveRuntimePinia();
  if (!pinia) throw new Error("Pinia is not active.");
  const optimize = useOptimizeStore(pinia);
  const data = requireDataContext();
  const nativeScoreSource = isMlNativeScoreSource(optimize.mlSignalScoreSource);
  const inputPath = normalizeText(optimize.mlSignalScoreInputPath);
  const outputPath = normalizeText(optimize.mlSignalScoreCachePath);
  if (!nativeScoreSource && !inputPath)
    throw new Error("Choose a model-score CSV input path first.");
  if (nativeScoreSource && !normalizeText(optimize.mlSignalModelBundleId)) {
    throw new Error("Choose a model bundle before using native ML scoring.");
  }
  if (!outputPath) throw new Error("Choose a score-cache output path first.");

  const currentArtifact = currentMlArtifactSnapshot();
  const nativeScoreScope = normalizeText(
    optimize.mlSignalNativeScoreScope || "bundle_default",
  );
  const scoreContract = readOptionalRecord(
    currentArtifact?.scoreContract ?? currentArtifact?.score_contract,
  );
  const req = {
    data_id: data.dataId,
    input_path: nativeScoreSource ? "" : inputPath,
    output_path: outputPath,
    model_bundle_id: normalizeText(optimize.mlSignalModelBundleId) || null,
    feature_schema_path:
      normalizeText(
        currentArtifact?.featureSchemaPath ??
          currentArtifact?.feature_schema_path,
      ) || null,
    feature_schema_hash:
      normalizeText(
        currentArtifact?.featureSchemaHash ??
          currentArtifact?.feature_schema_hash,
      ) || null,
    export_manifest_path:
      normalizeText(
        currentArtifact?.exportManifestPath ??
          currentArtifact?.export_manifest_path,
      ) || null,
    calibration_state:
      normalizeText(
        firstMeaningfulValue(
          currentArtifact?.calibration_state,
          currentArtifact?.calibrationState,
        ),
      ) || null,
    threshold_policy:
      normalizeText(
        firstMeaningfulValue(
          currentArtifact?.threshold_policy,
          currentArtifact?.thresholdPolicy,
        ),
      ) || null,
    source_bundle_manifest_path: null,
    producer_kind:
      normalizeText(
        firstMeaningfulValue(
          currentArtifact?.producer_kind,
          currentArtifact?.producerKind,
        ),
      ) || null,
    score_contract_version: parseOptionalInteger(
      firstMeaningfulValue(
        currentArtifact?.score_contract_version,
        currentArtifact?.scoreContractVersion,
        scoreContract?.score_contract_version,
        scoreContract?.scoreContractVersion,
      ),
    ),
    score_scope:
      nativeScoreSource && nativeScoreScope !== "bundle_default"
        ? nativeScoreScope
        : null,
    allow_non_walkforward_score_scope: nativeScoreSource
      ? Boolean(optimize.mlSignalAllowNonWalkforwardScoreScope)
      : false,
    native_backend_preference: nativeScoreSource
      ? normalizeText(optimize.mlSignalNativeBackend || "native_cpu") ||
        "native_cpu"
      : "score_cache_only",
    native_library_path: null,
    native_gpu_device_id: nativeScoreSource
      ? parseOptionalInteger(optimize.mlSignalNativeGpuDeviceId)
      : null,
    native_prediction_batch_size: nativeScoreSource
      ? parseOptionalInteger(optimize.mlSignalNativePredictionBatchSize)
      : null,
    native_library_discovery_policy: "production",
    strategy_kind: "MlSignal",
  };
  const runTs = new Date().toISOString();

  setMlBusy(
    nativeScoreSource
      ? "Generating ML score cache from native bundle inference…"
      : "Generating ML score cache…",
  );
  try {
    const res = await invokeCommand("generate_ml_signal_score_cache", { req });
    const resRecord =
      res && typeof res === "object" ? (res as Record<string, unknown>) : null;
    const matchedRows = parseNumber(resRecord?.matched_rows, 0);
    const scoreCoverageMeta = formatMlScoreCoverageMeta(resRecord);
    patchOptimizeState({
      mlSignalScoreMeta: [
        nativeScoreSource
          ? `Native score cache: ${matchedRows.toFixed(0)} matched rows -> ${outputPath}`
          : `Score cache: ${matchedRows.toFixed(0)} matched rows -> ${outputPath}`,
        scoreCoverageMeta,
        normalizeText(
          resRecord?.native_backend_used ?? resRecord?.nativeBackendUsed,
        ),
      ]
        .filter((part) => part)
        .join(" • "),
      progressMeta: nativeScoreSource
        ? `ML native score cache • ${matchedRows.toFixed(0)} rows`
        : `ML score cache • ${matchedRows.toFixed(0)} rows`,
      statusLine: formatMlStageStatus(
        nativeScoreSource
          ? "ML native score cache generated."
          : "ML score cache generated.",
        resRecord,
      ),
    });
    const persisted = await persistRunArtifact(req, res, runTs);
    applyVueRunArtifacts(runTs, req, res, persisted.path, persisted.csvPath);
    return res;
  } catch (error) {
    patchOptimizeState({
      statusLine:
        formatMlActionError("ML score cache generation failed", error),
    });
    throw error;
  } finally {
    clearMlBusy();
  }
}

export async function runMlSignalValidation(): Promise<unknown> {
  const pinia = resolveRuntimePinia();
  if (!pinia) throw new Error("Pinia is not active.");
  const optimize = useOptimizeStore(pinia);
  const scoreCachePath = normalizeText(optimize.mlSignalScoreCachePath);
  if (!scoreCachePath)
    throw new Error("Generate or choose a score cache path first.");
  const data = requireDataContext();
  const scoreCacheContractMode = normalizeMlScoreCacheContractMode(
    optimize.mlSignalScoreCacheContractMode,
  );

  let validationReq: Record<string, unknown>;
  try {
    validationReq = {
      data_id: data.dataId,
      score_cache_path: scoreCachePath,
      score_cache_contract_mode: scoreCacheContractMode,
      candidates: [
        {
          t_long: parseRequiredProbabilityWithDefault(
            optimize.mlSignalLongThreshold,
            0.55,
            "Long threshold",
          ),
          t_short: parseRequiredProbabilityWithDefault(
            optimize.mlSignalShortThreshold,
            0.55,
            "Short threshold",
          ),
          delta: parseRequiredProbabilityWithDefault(
            optimize.mlSignalDelta,
            0.05,
            "Delta threshold",
          ),
          max_hold_bars: parsePositiveInteger(
            optimize.mlSignalMaxHoldBars,
            10,
            "Max hold bars",
          ),
          exit_on_flat: Boolean(optimize.mlSignalExitOnFlat),
          exit_on_opposite: Boolean(optimize.mlSignalExitOnOpposite),
          label: "ml_signal_v1",
          strategy: buildMlStrategyConfig(),
        },
      ],
      iterative: {
        stop_loss_pct: parseOptionalPositive(useStrategyStore(pinia).stopLossPct),
        take_profit_pct: parseOptionalPositive(
          useStrategyStore(pinia).takeProfitPct,
        ),
        intrabar_rule: "pessimistic",
        slippage_vol_mult: 0,
      },
      min_psr: parseProbabilityGate(optimize.mlSignalMinPsr, "Min PSR gate"),
      min_dsr: parseProbabilityGate(optimize.mlSignalMinDsr, "Min DSR gate"),
      validation_context: buildValidationContext(),
    };
  } catch (error) {
    patchOptimizeState({
      statusLine: formatMlActionError("ML validation failed", error),
    });
    throw error;
  }

  setMlBusy("Running ML score-cache validation…");
  try {
    const validationRes = await invokeCommand("run_ml_signal_validation", {
      req: validationReq,
    });
    const validationRecord =
      validationRes && typeof validationRes === "object"
        ? (validationRes as Record<string, unknown>)
        : null;
    const candidates = Array.isArray(validationRecord?.candidates)
      ? ((validationRecord as Record<string, unknown>).candidates as unknown[])
      : [];
    const scoreCoverageMeta = formatMlScoreCoverageMeta(validationRecord);
    const deployabilityMeta = formatMlDeployabilityMeta(validationRecord);
    const evidencePolicyMeta = formatMlEvidencePolicyMeta(validationRecord);
    const scopeAdvisory = formatMlScoreScopeAdvisory(validationRecord);
    const boundaryAdvisory = formatMlBoundaryAdvisory(validationRecord);
    patchOptimizeState({
      mlSignalValidationMeta: [
        `ML validation: ${candidates.length} candidate${candidates.length === 1 ? "" : "s"} tested`,
        scoreCoverageMeta,
        deployabilityMeta,
        evidencePolicyMeta,
        scopeAdvisory,
        boundaryAdvisory,
      ]
        .filter((part) => part)
        .join(" • "),
      progressMeta: `ML validation • ${candidates.length} candidate${candidates.length === 1 ? "" : "s"}`,
      statusLine: [
        formatMlStageStatus("ML validation completed.", validationRecord),
        evidencePolicyMeta,
        scopeAdvisory,
        boundaryAdvisory,
      ]
        .filter((part) => part)
        .join(" • "),
    });

    const runSnapshot = currentRunSnapshot();
    const runReq = runSnapshot?.req ?? {
      strategy_kind: "MlSignal",
      data_id: data.dataId,
      score_cache_path: scoreCachePath,
    };
    const runRes = runSnapshot?.res ?? {
      score_cache_path: scoreCachePath,
    };
    const persisted = await persistValidationArtifact(
      runReq,
      runRes,
      validationReq,
      validationRes,
    );
    applyVueValidationArtifacts(
      runReq,
      runRes,
      validationReq,
      validationRes,
      persisted,
    );
    appendMlArtifactPersistenceWarning(persisted, ["mlSignalValidationMeta"]);
    return validationRes;
  } catch (error) {
    patchOptimizeState({
      statusLine: formatMlActionError("ML validation failed", error),
    });
    throw error;
  } finally {
    clearMlBusy();
  }
}

export async function runMlSignalPolicySweep(): Promise<unknown> {
  const pinia = resolveRuntimePinia();
  if (!pinia) throw new Error("Pinia is not active.");
  const optimize = useOptimizeStore(pinia);
  const scoreCachePath = normalizeText(optimize.mlSignalScoreCachePath);
  if (!scoreCachePath)
    throw new Error("Generate or choose a score cache path first.");
  const data = requireDataContext();
  const scoreCacheContractMode = normalizeMlScoreCacheContractMode(
    optimize.mlSignalScoreCacheContractMode,
  );

  let sweepReq: Record<string, unknown>;
  try {
    sweepReq = {
      data_id: data.dataId,
      score_cache_path: scoreCachePath,
      score_cache_contract_mode: scoreCacheContractMode,
      t_long: buildOptionalProbabilitySweepAxis(
        "Long threshold",
        optimize.mlSignalSweepLongStart,
        optimize.mlSignalSweepLongEnd,
        optimize.mlSignalSweepLongStep,
      ),
      t_short: buildOptionalProbabilitySweepAxis(
        "Short threshold",
        optimize.mlSignalSweepShortStart,
        optimize.mlSignalSweepShortEnd,
        optimize.mlSignalSweepShortStep,
      ),
      delta: buildOptionalProbabilitySweepAxis(
        "Delta",
        optimize.mlSignalSweepDeltaStart,
        optimize.mlSignalSweepDeltaEnd,
        optimize.mlSignalSweepDeltaStep,
      ),
      max_hold_bars: buildOptionalSweepAxisUsize(
        "Max hold",
        optimize.mlSignalSweepHoldStart,
        optimize.mlSignalSweepHoldEnd,
        optimize.mlSignalSweepHoldStep,
      ),
      exit_on_flat_values: [Boolean(optimize.mlSignalExitOnFlat)],
      exit_on_opposite_values: [Boolean(optimize.mlSignalExitOnOpposite)],
      base_strategy: buildMlStrategyConfig(),
      iterative: {
        stop_loss_pct: parseOptionalPositive(useStrategyStore(pinia).stopLossPct),
        take_profit_pct: parseOptionalPositive(
          useStrategyStore(pinia).takeProfitPct,
        ),
        intrabar_rule: "pessimistic",
        slippage_vol_mult: 0,
      },
      min_psr: parseProbabilityGate(optimize.mlSignalMinPsr, "Min PSR gate"),
      min_dsr: parseProbabilityGate(optimize.mlSignalMinDsr, "Min DSR gate"),
      validation_context: buildValidationContext(),
      top_k: Math.max(1, Math.floor(parseNumber(optimize.mlSignalSweepTopK, 10))),
    };
  } catch (error) {
    patchOptimizeState({
      statusLine: formatMlActionError("ML policy sweep failed", error),
    });
    throw error;
  }

  setMlBusy("Running ML policy sweep…");
  try {
    const sweepRes = await invokeCommand("run_ml_signal_policy_sweep", {
      req: sweepReq,
    });
    const sweepRow =
      sweepRes && typeof sweepRes === "object"
        ? (sweepRes as Record<string, unknown>)
        : {};
    const sweepSummary = extractMlPolicySweepSummary(sweepRes);
    const sweepSize = Math.max(
      0,
      Math.floor(
        parseNumber(
          firstMeaningfulValue(
            sweepSummary?.full_sweep_size,
            sweepSummary?.sweep_size,
            sweepRow.full_sweep_size,
            sweepRow.fullSweepSize,
            sweepRow.sweep_size,
          ),
          0,
        ),
      ),
    );
    const retainedTopK = Math.max(
      0,
      Math.floor(
        parseNumber(
          firstMeaningfulValue(
            sweepSummary?.retained_top_k,
            sweepSummary?.top_k,
            sweepRow.retained_top_k,
            sweepRow.retainedTopK,
            sweepRow.top_k,
          ),
          0,
        ),
      ),
    );
    const candidates = readCandidateRows(sweepRes);
    const retainedCount = Math.max(
      0,
      Math.floor(
        parseNumber(
          firstMeaningfulValue(
            sweepSummary?.retained_candidate_count,
            sweepRow.retained_candidate_count,
            sweepRow.retainedCandidateCount,
            candidates.length,
          ),
          candidates.length,
        ),
      ),
    );
    const best = candidates[0];
    const bestScore = Number(best?.robust_score ?? Number.NaN);
    const bestLabel = normalizeText(
      best?.params && typeof best.params === "object"
        ? (best.params as Record<string, unknown>).label
        : "",
    );
    const sweepFlow = [
      "full sweep",
      formatMlSweepOrderingText(
        firstMeaningfulValue(
          sweepSummary?.ordering,
          sweepRow.ordering,
          sweepRow.candidate_ordering,
          sweepRow.candidateOrdering,
        ),
      ),
      retainedTopK > 0 ? `retained top_k ${retainedTopK}` : "retained top_k",
    ].join(" -> ");
    const verdictFlow = `verdict rebuilt from ${formatMlVerdictScopeText(
      firstMeaningfulValue(
        sweepSummary?.verdict_scope,
        sweepRow.verdict_scope,
        sweepRow.verdictScope,
      ),
    )}`;
    const scoreCoverageMeta = formatMlScoreCoverageMeta(sweepRow);
    const deployabilityMeta = formatMlDeployabilityMeta(sweepRow);
    const evidencePolicyMeta = formatMlEvidencePolicyMeta(sweepRow);
    const scopeAdvisory = formatMlScoreScopeAdvisory(sweepRow);
    const boundaryAdvisory = formatMlBoundaryAdvisory(sweepRow);
    patchOptimizeState({
      mlSignalSweepMeta: [
        `ML policy sweep: ${sweepSize.toFixed(0)} candidate${sweepSize === 1 ? "" : "s"} evaluated`,
        sweepFlow,
        Number.isFinite(bestScore) ? `best ${bestScore.toFixed(4)}` : "",
        bestLabel || "",
        scoreCoverageMeta,
        deployabilityMeta,
        evidencePolicyMeta,
        scopeAdvisory,
        boundaryAdvisory,
      ]
        .filter((part) => part)
        .join(" • "),
      mlSignalValidationMeta: [
        `ML validation: ${candidates.length} retained from ${sweepSize.toFixed(0)} full-sweep candidate${sweepSize === 1 ? "" : "s"}`,
        verdictFlow,
        retainedCount > 0 ? `${retainedCount} retained` : "",
        scoreCoverageMeta,
        deployabilityMeta,
        evidencePolicyMeta,
        scopeAdvisory,
        boundaryAdvisory,
      ]
        .filter((part) => part)
        .join(" • "),
      progressMeta: `ML policy sweep • ${sweepSize.toFixed(0)} candidate${sweepSize === 1 ? "" : "s"} evaluated`,
      statusLine: [
        formatMlStageStatus("ML policy sweep completed.", sweepRow),
        `${sweepFlow} -> ${verdictFlow}`,
        evidencePolicyMeta,
        scopeAdvisory,
        boundaryAdvisory,
      ]
        .filter((part) => part)
        .join(" • "),
    });

    const runSnapshot = currentRunSnapshot();
    const runReq = runSnapshot?.req ?? {
      strategy_kind: "MlSignal",
      data_id: data.dataId,
      score_cache_path: scoreCachePath,
    };
    const runRes = runSnapshot?.res ?? {
      score_cache_path: scoreCachePath,
    };
    const persisted = await persistValidationArtifact(
      runReq,
      runRes,
      sweepReq,
      sweepRes,
    );
    applyVueValidationArtifacts(runReq, runRes, sweepReq, sweepRes, persisted);
    appendMlArtifactPersistenceWarning(persisted, [
      "mlSignalSweepMeta",
      "mlSignalValidationMeta",
    ]);
    return sweepRes;
  } catch (error) {
    patchOptimizeState({
      statusLine: formatMlActionError("ML policy sweep failed", error),
    });
    throw error;
  } finally {
    clearMlBusy();
  }
}

export async function listMlSignalModelBundles(): Promise<
  Array<{
    bundleId: string;
    manifestPath: string;
    state: string;
    label: string;
  }>
> {
  const raw = (await invokeCommand("list_ml_signal_model_bundles")) as unknown;
  if (!Array.isArray(raw)) return [];
  return raw
    .map((entry) => {
      const row = (
        entry && typeof entry === "object" ? entry : {}
      ) as MlSignalModelBundleSummary;
      const bundleId = normalizeText(row.bundle_id);
      const state = normalizeText(row.state) || "unknown";
      const family = normalizeText(row.model_family);
      const issues = readStringList(row.issues);
      const issueSummary =
        issues.length === 1
          ? "1 issue"
          : issues.length > 1
            ? `${issues.length} issues`
            : "";
      return {
        bundleId,
        manifestPath: normalizeText(row.manifest_path),
        state,
        label: [bundleId, family, state, issueSummary]
          .filter((part) => String(part || "").trim().length > 0)
          .join(" • "),
      };
    })
    .filter((row) => row.bundleId);
}

export async function refreshMlSignalModelStatus(): Promise<unknown> {
  const pinia = resolveRuntimePinia();
  if (!pinia) throw new Error("Pinia is not active.");
  const optimize = useOptimizeStore(pinia);
  const bundleId = normalizeText(optimize.mlSignalModelBundleId);
  if (!bundleId) {
    const nativeScoreSource =
      normalizeText(optimize.mlSignalScoreSource).toLowerCase() === "native";
    patchOptimizeState({
      mlSignalModelStatus: nativeScoreSource
        ? "Model bundle: required for native score-cache generation."
        : "Model bundle: optional for score-cache-backed validation.",
    });
    return null;
  }
  const raw = (await invokeCommand("ml_signal_model_status", {
    bundleId,
  })) as MlSignalModelStatusResponse;
  const status =
    raw && typeof raw.status === "object"
      ? (raw.status as Record<string, unknown>)
      : null;
  const state = normalizeText(status?.state) || "missing";
  const manifestPath = normalizeText(
    status?.manifest_path ?? status?.manifestPath,
  );
  const manifest = readOptionalRecord(status?.manifest);
  const issues = readStringList(status?.issues);
  const modelFamily = normalizeText(manifest?.model_family);
  const modelFormat = normalizeText(manifest?.model_format);
  const scoreContract = readOptionalRecord(
    manifest?.score_contract ?? manifest?.scoreContract,
  );
  const scoreContractVersion = normalizeText(
    scoreContract?.score_contract_version ??
      scoreContract?.scoreContractVersion,
  );
  const calibrationPath = normalizeText(
    manifest?.calibration_path ?? manifest?.calibrationPath,
  );
  const statusParts = [`Model bundle: ${bundleId}`, state];
  if (modelFamily) statusParts.push(modelFamily);
  if (modelFormat) statusParts.push(modelFormat);
  if (scoreContractVersion)
    statusParts.push(`contract v${scoreContractVersion}`);
  if (calibrationPath) statusParts.push("calibration attached");
  if (issues.length === 1 && issues[0]) statusParts.push(issues[0]);
  if (issues.length > 1) statusParts.push(`${issues.length} issues`);
  if (manifestPath) statusParts.push(manifestPath);
  patchOptimizeState({
    mlSignalModelStatus: statusParts.join(" • "),
  });
  if (status) {
    patchCurrentMlSignalArtifact({
      bundle_status: status,
    });
  }
  return raw;
}

export async function openMlSignalArtifactPath(path: string): Promise<unknown> {
  return openOrRevealPath(path);
}

export async function revealMlSignalArtifactPath(
  path: string,
): Promise<unknown> {
  const nextPath = normalizeText(path);
  if (!nextPath) throw new Error("Path is required.");
  await invokeCommand("reveal_path", { path: nextPath });
  return { path: nextPath, openedWith: "reveal_path" };
}

export function setMlSignalPathField(field: string, value: string): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  const optimize = useOptimizeStore(pinia);
  optimize.setInputField(field, value);
  markOptimizeUpdated();
}
