import { resolveRuntimePinia } from "../stores/runtimePinia";
import { markRaw } from "vue";
import { listenEvent } from "../tauri/events";
import { invokeCommand } from "../tauri/invoke";
import { buildOptimizationRequestFromVueStores } from "./optimizeService";
import { useAnalysisStore } from "../stores/analysisStore";
import { useCommandActionStore } from "../stores/commandActionStore";
import { useDataStore } from "../stores/dataStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useWorkflowStore } from "../stores/workflowStore";
import { LAST_BATCH_SUMMARY_KEY } from "../utils/core/storageKeys";
import {
  defaultBatchTag,
  dedupTags,
  splitHoldoutPaths,
} from "../utils/core/artifactIdentity";
import { openPathWithRevealFallback } from "../utils/openPath";
import {
  readStorageString,
  removeStorageKey,
  writeStorageString,
} from "../utils/storage";
import { formatUiCount, formatUtcDateFromUnixMs } from "../utils/uiFormat";
import { buildMetaBatchRequest } from "./metaBatchRequestBuilder";
import { buildMetaMlScope } from "./metaMlScopeBuilder";
import { buildMetaSelectionRequest } from "./metaRequestBuilder";
import { buildMetaStage5Request } from "./metaStage4RequestBuilder";
import { storeErrorText } from "../utils/storeErrorText";
import {
  buildCampaignStageCompletion,
  buildMetaStageCompletion,
  buildPolicyEvaluationStageCompletion,
  buildStage5MlConfidenceStageCompletion,
  serializeStageCompletionsForArtifact,
} from "../utils/core/stageCompletion";

export interface MetaRunOptions {
  request?: unknown;
  switchTab?: boolean;
}

export interface MetaBatchRunOptions {
  request?: unknown;
}

export interface MetaStage4RunOptions {
  request?: unknown;
}

export interface MetaStage5RunOptions extends MetaStage4RunOptions {}

export interface MetaMlPolicyEvaluationRunOptions {
  request?: unknown;
}

export interface MetaMlActionOptions {
  scope?: unknown;
}

interface VueStoreFieldBinding {
  key: string;
}

interface MetaTableColumn {
  key?: string;
  label: string;
  alignLeft: boolean;
}

interface MetaTableCell {
  text: string;
  className: string;
  title: string;
}

interface MetaTableRow {
  cells: MetaTableCell[];
}

interface BatchDatasetEntry {
  path: string;
  label: string;
  meta: string;
  market: string;
  timeframe: string;
  adjusted: boolean | null;
  source: string;
  source_size_bytes: number | null;
  source_mtime_utc_ms: number | null;
  source_content_fingerprint: string;
}

interface BatchSummaryEnvelope {
  path: string;
  summary: Record<string, unknown>;
}

type AnalysisStore = ReturnType<typeof useAnalysisStore>;

const META_TABLE_COLUMNS: MetaTableColumn[] = markRaw([
  { key: "rank", label: "#", alignLeft: false },
  { key: "samples", label: "Samples", alignLeft: false },
  { key: "pass_rate", label: "Pass", alignLeft: false },
  { key: "meta_score", label: "Meta", alignLeft: false },
  { key: "ml_confidence", label: "ML", alignLeft: false },
  { key: "robust_score_p10", label: "Robust p10", alignLeft: false },
  { key: "robust_score_median", label: "Robust med", alignLeft: false },
  { key: "max_dd_p90", label: "DD p90", alignLeft: false },
  { key: "psr", label: "PSR", alignLeft: false },
  { key: "dsr", label: "DSR", alignLeft: false },
  { key: "holdout_pass_rate", label: "H pass", alignLeft: false },
  { key: "holdout_score_p10", label: "H p10", alignLeft: false },
  { key: "holdout_score_median", label: "H med", alignLeft: false },
  { key: "holdout_max_dd_p90", label: "H DD p90", alignLeft: false },
  { key: "holdout_coverage", label: "H coverage", alignLeft: false },
  { key: "fast", label: "Fast", alignLeft: true },
  { key: "slow", label: "Slow", alignLeft: true },
  { key: "params", label: "Params", alignLeft: true },
]) as MetaTableColumn[];

const ANALYSIS_TEXT_FIELD_BINDINGS: Record<string, VueStoreFieldBinding> = {
  metaStrategyKind: { key: "metaStrategyKind" },
  metaTimeframe: { key: "metaTimeframe" },
  metaRequiredTag: { key: "metaRequiredTag" },
  metaMinSamples: { key: "metaMinSamples" },
  metaTopK: { key: "metaTopK" },
  metaHoldoutTag: { key: "metaHoldoutTag" },
  metaHoldoutMode: { key: "metaHoldoutMode" },
  metaHoldoutEvalTopN: { key: "metaHoldoutEvalTopN" },
  metaHoldoutExploreN: { key: "metaHoldoutExploreN" },
  metaHoldoutEvalRankBy: { key: "metaHoldoutEvalRankBy" },
  metaHoldoutReps: { key: "metaHoldoutReps" },
  metaGroupingMode: { key: "metaGroupingMode" },
  metaClusterIntBucket: { key: "metaClusterIntBucket" },
  metaClusterFloatBucket: { key: "metaClusterFloatBucket" },
  batchHoldoutPct: { key: "batchHoldoutPct" },
  batchHoldoutTag: { key: "batchHoldoutTag" },
  batchTags: { key: "batchTags" },
  metaStage4TopK: { key: "metaStage4TopK" },
  metaStage4MinConf: { key: "metaStage4MinConf" },
  metaStage4MetaWeight: { key: "metaStage4MetaWeight" },
  metaStage4MlWeight: { key: "metaStage4MlWeight" },
  metaStage4UncertaintyWeight: { key: "metaStage4UncertaintyWeight" },
  metaStage4AcceptConf: { key: "metaStage4AcceptConf" },
  metaStage4ReviewConf: { key: "metaStage4ReviewConf" },
  metaStage4MaxAcceptUncertainty: { key: "metaStage4MaxAcceptUncertainty" },
  metaStage4TrainingHistoryPolicy: { key: "metaStage4TrainingHistoryPolicy" },
  policyEvalSavedSearch: { key: "policyEvalSavedSearch" },
  policyEvalSavedSelect: { key: "policyEvalSavedSelected" },
  policyEvalSavedTags: { key: "policyEvalSavedTagsDraft" },
};

const ANALYSIS_CHECKBOX_FIELD_BINDINGS: Record<string, VueStoreFieldBinding> = {
  metaEvalHoldout: { key: "metaEvalHoldout" },
  metaSaveHoldoutRows: { key: "metaSaveHoldoutRows" },
  batchAutoMeta: { key: "batchAutoMeta" },
  batchAutoStage5: { key: "batchAutoStage5" },
  batchSkipDone: { key: "batchSkipDone" },
  metaStage4AutoTrain: { key: "metaStage4AutoTrain" },
  metaStage4RequireHealthy: { key: "metaStage4RequireHealthy" },
  metaStage4FallbackToMeta: { key: "metaStage4FallbackToMeta" },
};

const VALIDATION_TEXT_FIELD_BINDINGS: Record<string, VueStoreFieldBinding> = {
  valSeed: { key: "seed" },
  valWfWindow: { key: "wfWindow" },
  valWfStep: { key: "wfStep" },
  valPerturbIters: { key: "perturbIters" },
  valPerturbMin: { key: "perturbMin" },
  valPerturbMax: { key: "perturbMax" },
  valMcSims: { key: "mcSims" },
  valMcBlockLen: { key: "mcBlockLen" },
  valMcMode: { key: "mcMode" },
};

const VALIDATION_CHECKBOX_FIELD_BINDINGS: Record<string, VueStoreFieldBinding> =
  {
    valPerturbEnable: { key: "perturbEnable" },
    valMcEnable: { key: "mcEnable" },
  };

let LAST_META_RUN: Record<string, unknown> | null = null;
let LAST_META_STAGE4_RUN: Record<string, unknown> | null = null;
let LAST_META_ML_STATUS: Record<string, unknown> | null = null;
let LAST_META_ML_POLICY_EVAL: Record<string, unknown> | null = null;
let SAVED_META: Record<string, unknown>[] = [];
let SAVED_STAGE4: Record<string, unknown>[] = [];
let SAVED_POLICY_EVAL: Record<string, unknown>[] = [];
let BATCH_RUNNING = false;
let BATCH_DATASETS_CACHE: BatchDatasetEntry[] = [];
let BATCH_SELECTED_PATHS = new Set<string>();
let BATCH_SELECTED_PATHS_READY = false;
let LAST_BATCH_SUMMARY: BatchSummaryEnvelope | null = null;

function normalizePath(value: unknown): string {
  return String(value || "").trim();
}

function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
}

function normalizeRequestPayload(
  value: unknown,
): Record<string, unknown> | null {
  if (!value || typeof value !== "object") return null;
  return value as Record<string, unknown>;
}

function normalizeBoolean(value: unknown): boolean {
  if (typeof value === "boolean") return value;
  const text = normalizeText(value).toLowerCase();
  return text === "true" || text === "1" || text === "yes" || text === "on";
}

function normalizeOptionalBoolean(value: unknown): boolean | null {
  if (typeof value === "boolean") return value;
  const text = normalizeText(value).toLowerCase();
  if (!text) return null;
  if (text === "true" || text === "1" || text === "yes" || text === "on")
    return true;
  if (text === "false" || text === "0" || text === "no" || text === "off")
    return false;
  return null;
}

function canOpenPersistedReport(
  pathValue: unknown,
  existsValue: unknown,
): boolean {
  const reportPath = normalizePath(pathValue);
  if (!reportPath) return false;
  const reportExists = normalizeOptionalBoolean(existsValue);
  return reportExists == null ? true : reportExists;
}

function normalizeFiniteNumber(value: unknown, fallback = 0): number {
  const num = Number(value);
  return Number.isFinite(num) ? num : fallback;
}

function normalizeMaybeFiniteNumber(value: unknown): number | null {
  const num = Number(value);
  return Number.isFinite(num) ? num : null;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  return value as Record<string, unknown>;
}

function asRecordArray(value: unknown): Record<string, unknown>[] {
  if (!Array.isArray(value)) return [];
  const rows: Record<string, unknown>[] = [];
  for (const entry of value) {
    const row = asRecord(entry);
    if (!row) continue;
    rows.push(row);
  }
  return rows;
}

function fmtFixed(value: unknown, digits: number): string {
  const num = Number(value);
  return Number.isFinite(num) ? num.toFixed(digits) : "-";
}

function fmtPct(value: unknown, digits: number): string {
  const num = Number(value);
  return Number.isFinite(num) ? `${(num * 100).toFixed(digits)}%` : "-";
}

function fileBasename(path: string): string {
  const normalized = normalizePath(path).replace(/\\/g, "/");
  if (!normalized) return "";
  const index = normalized.lastIndexOf("/");
  return index >= 0 ? normalized.slice(index + 1) : normalized;
}

function pretty(value: unknown): string {
  try {
    return JSON.stringify(value ?? null, null, 2);
  } catch {
    return String(value ?? "");
  }
}

const META_ARTIFACT_KINDS = Object.freeze({
  selection: "vectorgrid_meta_selection",
  stage5: "vectorgrid_meta_stage5_ml_confidence",
  mlPolicyEvaluation: "vectorgrid_ml_confidence_policy_evaluation",
});

const BATCH_SUMMARY_ARTIFACT_KINDS = new Set([
  "vectorbt_batch",
  "vectorgrid_batch",
  "vectorbt_batch_summary",
  "vectorgrid_batch_summary",
  "vectorbt_campaign_summary",
  "vectorgrid_campaign_summary",
]);

function responseStage5Path(value: unknown): string {
  const record = asRecord(value);
  return normalizePath(record?.stage5_json_path ?? record?.stage4_json_path);
}

function responseStage5Bytes(value: unknown): number {
  const record = asRecord(value);
  return Math.max(
    0,
    Math.floor(
      Number(record?.stage5_json_bytes ?? record?.stage4_json_bytes ?? 0),
    ),
  );
}

function parseTagsInput(value: string): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  for (const part of normalizeText(value).split(",")) {
    const token = normalizeText(part);
    if (!token) continue;
    const key = token.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(token);
  }
  return out;
}

function formatParamsCompact(params: Record<string, unknown>): string {
  const parts: string[] = [];
  for (const [key, value] of Object.entries(params)) {
    if (value == null) continue;
    if (typeof value === "number" && Number.isFinite(value)) {
      parts.push(
        `${key}=${Number.isInteger(value) ? value : value.toFixed(4)}`,
      );
      continue;
    }
    if (typeof value === "boolean") {
      parts.push(`${key}=${value ? "1" : "0"}`);
      continue;
    }
    const text = normalizeText(value);
    if (!text) continue;
    parts.push(`${key}=${text}`);
  }
  return parts.join(" ");
}

function readStoreRecord(
  storeKey: "analysis" | "validation",
): Record<string, unknown> | null {
  const pinia = resolveRuntimePinia();
  if (!pinia) return null;
  const root = pinia.state.value as Record<string, unknown>;
  const store = root[storeKey];
  if (!store || typeof store !== "object") return null;
  return store as Record<string, unknown>;
}

function readPiniaStoreRecord(
  storeKey: string,
): Record<string, unknown> | null {
  const pinia = resolveRuntimePinia();
  if (!pinia) return null;
  const root = pinia.state.value as Record<string, unknown>;
  const store = root[storeKey];
  if (!store || typeof store !== "object") return null;
  return store as Record<string, unknown>;
}

function getAnalysisStore(): AnalysisStore | null {
  const pinia = resolveRuntimePinia();
  if (!pinia) return null;
  return useAnalysisStore(pinia);
}

function getDataStore() {
  const pinia = resolveRuntimePinia();
  if (!pinia) return null;
  return useDataStore(pinia);
}

function getOptimizeStore() {
  const pinia = resolveRuntimePinia();
  if (!pinia) return null;
  return useOptimizeStore(pinia);
}

function patchAnalysisStore(patch: Record<string, unknown>): void {
  const store = getAnalysisStore();
  if (!store) return;
  const target = store as unknown as Record<string, unknown>;
  for (const [key, value] of Object.entries(patch)) {
    target[key] = value;
  }
  store.source = "vue";
  store.updatedAtUnixMs = Date.now();
}

function setStatusLine(message: string): void {
  patchAnalysisStore({
    statusLine: normalizeText(message) || "Ready.",
  });
}

function setErrorStatus(error: unknown): void {
  const message = storeErrorText(error) || "Operation failed.";
  patchAnalysisStore({ statusLine: message });
}

function selectMetaResultsTab(): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  const workflowStore = useWorkflowStore(pinia);
  workflowStore.selectResultsTab("meta");
}

function readVueTextFieldValue(id: string): string | null {
  const key = normalizeText(id);
  if (!key) return null;

  const analysisBinding = ANALYSIS_TEXT_FIELD_BINDINGS[key];
  if (analysisBinding) {
    const store = readStoreRecord("analysis");
    if (
      store &&
      Object.prototype.hasOwnProperty.call(store, analysisBinding.key)
    ) {
      return normalizeText(store[analysisBinding.key]);
    }
  }

  const validationBinding = VALIDATION_TEXT_FIELD_BINDINGS[key];
  if (validationBinding) {
    const store = readStoreRecord("validation");
    if (
      store &&
      Object.prototype.hasOwnProperty.call(store, validationBinding.key)
    ) {
      return normalizeText(store[validationBinding.key]);
    }
  }

  return null;
}

function readVueCheckboxFieldValue(id: string): boolean | null {
  const key = normalizeText(id);
  if (!key) return null;

  const analysisBinding = ANALYSIS_CHECKBOX_FIELD_BINDINGS[key];
  if (analysisBinding) {
    const store = readStoreRecord("analysis");
    if (
      store &&
      Object.prototype.hasOwnProperty.call(store, analysisBinding.key)
    ) {
      return normalizeBoolean(store[analysisBinding.key]);
    }
  }

  const validationBinding = VALIDATION_CHECKBOX_FIELD_BINDINGS[key];
  if (validationBinding) {
    const store = readStoreRecord("validation");
    if (
      store &&
      Object.prototype.hasOwnProperty.call(store, validationBinding.key)
    ) {
      return normalizeBoolean(store[validationBinding.key]);
    }
  }

  return null;
}

function readVueBatchSelectedPaths(): string[] | null {
  const store = readStoreRecord("analysis");
  if (!store) return null;
  const rows = store.batchDatasets;
  if (!Array.isArray(rows)) return null;

  const selected: string[] = [];
  for (const entry of rows) {
    if (!entry || typeof entry !== "object") continue;
    const row = entry as Record<string, unknown>;
    if (!normalizeBoolean(row.checked)) continue;
    const path = normalizeText(row.path);
    if (!path) continue;
    selected.push(path);
  }
  return selected;
}

function readStoreTextInputValue(id: string): string {
  const key = normalizeText(id);
  if (!key) return "";
  const vueValue = readVueTextFieldValue(key);
  return vueValue != null ? vueValue : "";
}

function readStoreCheckboxValue(id: string): boolean {
  const key = normalizeText(id);
  if (!key) return false;
  const vueValue = readVueCheckboxFieldValue(key);
  return vueValue != null ? vueValue : false;
}

function tryBuildAutoMetaRequest(): Record<string, unknown> | null {


  return buildMetaSelectionRequest({
    strategyKind: readStoreTextInputValue("metaStrategyKind"),
    timeframe: readStoreTextInputValue("metaTimeframe"),
    requiredTag: readStoreTextInputValue("metaRequiredTag"),
    minSamples: readStoreTextInputValue("metaMinSamples"),
    topK: readStoreTextInputValue("metaTopK"),
    holdoutTag: readStoreTextInputValue("metaHoldoutTag"),
    evalHoldout: readStoreCheckboxValue("metaEvalHoldout"),
    holdoutMode: readStoreTextInputValue("metaHoldoutMode"),
    holdoutEvalTopN: readStoreTextInputValue("metaHoldoutEvalTopN"),
    holdoutExploreN: readStoreTextInputValue("metaHoldoutExploreN"),
    holdoutRankBy: readStoreTextInputValue("metaHoldoutEvalRankBy"),
    holdoutReps: readStoreTextInputValue("metaHoldoutReps"),
    groupingMode: readStoreTextInputValue("metaGroupingMode"),
    clusterIntBucket: readStoreTextInputValue("metaClusterIntBucket"),
    clusterFloatBucket: readStoreTextInputValue("metaClusterFloatBucket"),
    saveHoldoutRows: readStoreCheckboxValue("metaSaveHoldoutRows"),
    valSeed: readStoreTextInputValue("valSeed"),
    valWfWindow: readStoreTextInputValue("valWfWindow"),
    valWfStep: readStoreTextInputValue("valWfStep"),
    valPerturbEnable: readStoreCheckboxValue("valPerturbEnable"),
    valPerturbIters: readStoreTextInputValue("valPerturbIters"),
    valPerturbMin: readStoreTextInputValue("valPerturbMin"),
    valPerturbMax: readStoreTextInputValue("valPerturbMax"),
    valMcEnable: readStoreCheckboxValue("valMcEnable"),
    valMcSims: readStoreTextInputValue("valMcSims"),
  });
}

function readStoreBatchSelectedPaths(): string[] {
  return readBatchSelectedPaths();
}

function normalizeBatchPreloadPolicy(
  value: unknown,
): "disabled" | "host_only" | "host_plus_doublema_vram" {
  const raw = normalizeText(value).toLowerCase();
  if (raw === "disabled" || raw === "host_plus_doublema_vram") return raw;
  return "host_only";
}

function normalizeBatchExecutionMode(
  value: unknown,
): "full" | "optimize_only" | "validate_only" {
  const raw = normalizeText(value).toLowerCase();
  if (raw === "optimize_only" || raw === "validate_only") return raw;
  return "full";
}

function normalizeValidationAdmissionPolicy(
  value: unknown,
): "legacy_topk" | "scalar_union" | "pareto_frontier" | "auto" {
  const raw = normalizeText(value)
    .toLowerCase()
    .replace(/[\s-]+/g, "_");
  if (raw === "scalar_union") return "scalar_union";
  if (raw === "pareto_frontier") return "pareto_frontier";
  if (raw === "auto") return "auto";
  return "legacy_topk";
}

function tryBuildAutoMetaBatchRequest(): Record<string, unknown> | null {
  return buildMetaBatchRequest({
    selectedPaths: readStoreBatchSelectedPaths(),
    holdoutPct: readStoreTextInputValue("batchHoldoutPct"),
    holdoutTag: readStoreTextInputValue("batchHoldoutTag"),
    executionMode: readStoreTextInputValue("batchExecutionMode"),
    preloadPolicy: readStoreTextInputValue("batchPreloadPolicy"),
    autoMeta: readStoreCheckboxValue("batchAutoMeta"),
    autoStage5: readStoreCheckboxValue("batchAutoStage5"),
    skipDone: readStoreCheckboxValue("batchSkipDone"),
    tags: readStoreTextInputValue("batchTags"),
  });
}

function tryBuildAutoMetaStage4Request(): Record<string, unknown> | null {
  const scope = currentMetaScope();
  return buildMetaStage5Request({
    strategyKind: scope.strategy_kind,
    timeframe: scope.timeframe,
    sourceMetaPath: currentMetaSourcePath(),
    topK: readStoreTextInputValue("metaStage4TopK"),
    minMlConfidence: readStoreTextInputValue("metaStage4MinConf"),
    metaWeight: readStoreTextInputValue("metaStage4MetaWeight"),
    mlWeight: readStoreTextInputValue("metaStage4MlWeight"),
    uncertaintyWeight: readStoreTextInputValue("metaStage4UncertaintyWeight"),
    acceptConfidence: readStoreTextInputValue("metaStage4AcceptConf"),
    reviewConfidence: readStoreTextInputValue("metaStage4ReviewConf"),
    maxAcceptUncertainty: readStoreTextInputValue(
      "metaStage4MaxAcceptUncertainty",
    ),
    autoTrainModel: readStoreCheckboxValue("metaStage4AutoTrain"),
    requireHealthyModel: readStoreCheckboxValue("metaStage4RequireHealthy"),
    fallbackToMetaIfEmpty: readStoreCheckboxValue("metaStage4FallbackToMeta"),
    trainingHistoryPolicy: readStoreTextInputValue(
      "metaStage4TrainingHistoryPolicy",
    ),
  });
}

function tryBuildAutoMetaMlPolicyEvaluationRequest(): Record<
  string,
  unknown
> | null {
  const scope = currentMetaScope();
  const timeframe = normalizeText(scope.timeframe);
  if (!timeframe) return null;

  const normalizedStage5 = buildMetaStage5Request({
    strategyKind: scope.strategy_kind,
    timeframe,
    topK: readStoreTextInputValue("metaStage4TopK"),
    minMlConfidence: readStoreTextInputValue("metaStage4MinConf"),
    metaWeight: readStoreTextInputValue("metaStage4MetaWeight"),
    mlWeight: readStoreTextInputValue("metaStage4MlWeight"),
    uncertaintyWeight: readStoreTextInputValue("metaStage4UncertaintyWeight"),
    requireHealthyModel: readStoreCheckboxValue("metaStage4RequireHealthy"),
    fallbackToMetaIfEmpty: readStoreCheckboxValue("metaStage4FallbackToMeta"),
  });

  return {
    strategy_kind: normalizeText(scope.strategy_kind),
    timeframe,
    top_k: normalizedStage5.top_k,
    min_ml_confidence: normalizedStage5.min_ml_confidence,
    meta_weight: normalizedStage5.meta_weight,
    ml_weight: normalizedStage5.ml_weight,
    uncertainty_weight: normalizedStage5.uncertainty_weight,
    require_healthy_model: normalizedStage5.require_healthy_model,
    fallback_to_meta_if_empty: normalizedStage5.fallback_to_meta_if_empty,
  };
}

function tryBuildAutoMetaMlScope(): Record<string, unknown> | null {
  return buildMetaMlScope({
    strategyKind: readStoreTextInputValue("metaStrategyKind"),
    timeframe: readStoreTextInputValue("metaTimeframe"),
  });
}

function currentMetaScope(): { strategy_kind: string; timeframe: string } {
  const store = getAnalysisStore();
  const result = asRecord(LAST_META_RUN?.result);


  const strategy =
    normalizeText(
      result?.strategy_kind ?? store?.metaStrategyKind ?? "CompositeMo",
    ) || "CompositeMo";
  const timeframe = normalizeText(result?.timeframe ?? store?.metaTimeframe);
  return {
    strategy_kind: strategy,
    timeframe,
  };
}

function currentMetaSourcePath(): string {
  const activeMetaPath = normalizePath(LAST_META_RUN?.meta_json_path);
  if (activeMetaPath) return activeMetaPath;
  const selectedSavedPath = normalizePath(selectedMetaSavedEntry()?.path);
  return selectedSavedPath;
}

function normalizeMlScope(scopeValue: unknown): {
  strategy_kind: string;
  timeframe: string;
} {
  const payload = normalizeRequestPayload(scopeValue);
  if (payload) {
    const strategyKind =
      normalizeText(payload.strategy_kind ?? payload.strategyKind) ||
      currentMetaScope().strategy_kind;
    const timeframe =
      normalizeText(payload.timeframe) || currentMetaScope().timeframe;
    return { strategy_kind: strategyKind, timeframe };
  }

  let autoScope: Record<string, unknown> | null = null;
  try {
    autoScope = tryBuildAutoMetaMlScope();
  } catch {
    autoScope = null;
  }

  if (autoScope) {
    const strategyKind =
      normalizeText(autoScope.strategy_kind ?? autoScope.strategyKind) ||
      currentMetaScope().strategy_kind;
    const timeframe =
      normalizeText(autoScope.timeframe) || currentMetaScope().timeframe;
    return { strategy_kind: strategyKind, timeframe };
  }

  return currentMetaScope();
}

function buildMetaRows(groups: Record<string, unknown>[]): MetaTableRow[] {
  const rows: MetaTableRow[] = [];
  for (let index = 0; index < groups.length; index += 1) {
    const group = groups[index] ?? {};
    const params = asRecord(group.params) ?? {};
    const holdout = asRecord(group.holdout);
    const holdoutCoverage = asRecord(group.holdout_coverage);
    const evidenceFlags = Array.isArray(group.evidence_flags)
      ? group.evidence_flags
          .map((item) => normalizeText(item))
          .filter((item) => item.length > 0)
      : [];
    const evidenceTitle = evidenceFlags.length
      ? `Evidence flags: ${evidenceFlags.join(", ")}`
      : "";

    const fast =
      `${normalizeText(params.fast_ma_type).toUpperCase()} ${Math.max(0, Math.floor(Number(params.fast_len || 0)))}`.trim();
    const slow =
      `${normalizeText(params.slow_ma_type).toUpperCase()} ${Math.max(0, Math.floor(Number(params.slow_len || 0)))}`.trim();

    const finalScore = Number(group.final_score);
    const rankScore = Number.isFinite(finalScore)
      ? finalScore
      : Number(group.meta_score || 0);

    const cells = [
      String(index + 1),
      String(Math.max(0, Math.floor(Number(group.samples || 0)))),
      fmtPct(group.pass_rate, 1),
      fmtFixed(rankScore, 6),
      Number.isFinite(Number(group.ml_confidence))
        ? fmtPct(group.ml_confidence, 1)
        : "-",
      fmtFixed(group.robust_score_p10, 6),
      fmtFixed(group.robust_score_median, 6),
      fmtFixed(group.max_dd_p90, 6),
      fmtFixed(group.psr_median, 4),
      fmtFixed(group.dsr_median, 4),
      holdout ? fmtPct(holdout.pass_rate, 1) : "-",
      holdout ? fmtFixed(holdout.score_p10, 6) : "-",
      holdout ? fmtFixed(holdout.score_median, 6) : "-",
      holdout ? fmtFixed(holdout.max_dd_p90, 6) : "-",
      holdoutCoverage
        ? normalizeBoolean(holdoutCoverage.evaluated)
          ? `${Math.max(0, Math.floor(Number(holdoutCoverage.dataset_rows || 0)))}`
          : normalizeText(holdoutCoverage.missing_reason) || "not evaluated"
        : "-",
      fast,
      slow,
      formatParamsCompact(params) || pretty(params),
    ];

    rows.push({
      cells: cells.map((text, cellIndex) => ({
        text,
        className: cellIndex >= 15 ? "left" : "",
        title:
          cellIndex === 3 || cellIndex === cells.length - 1
            ? evidenceTitle
            : "",
      })),
    });
  }
  return rows;
}

function renderMetaSelection(resp: Record<string, unknown> | null): void {
  const previousMetaPath = normalizePath(LAST_META_RUN?.meta_json_path);
  LAST_META_RUN = resp;

  const result = asRecord(resp?.result);
  const groups = asRecordArray(result?.groups);
  const holdout = asRecord(result?.holdout);
  const holdoutCoverage = asRecord(result?.holdout_coverage);
  const cohortManifest = asRecord(resp?.cohort_manifest);
  const cohortDisjointness = asRecord(cohortManifest?.disjointness);
  const holdoutDatasets = Math.max(
    0,
    Math.floor(Number(holdout?.datasets || 0)),
  );
  const holdoutSkipped = Math.max(0, Math.floor(Number(holdout?.skipped || 0)));
  const warnings = Array.isArray(result?.warnings) ? result.warnings : [];
  const groupsWithEvidenceFlags = groups.filter(
    (group) =>
      Array.isArray(group.evidence_flags) && group.evidence_flags.length > 0,
  ).length;
  const reportPath = normalizePath(resp?.report_xlsx_path);
  const canOpenReport = canOpenPersistedReport(
    resp?.report_xlsx_path,
    resp?.report_xlsx_exists,
  );
  const metaPath = normalizePath(resp?.meta_json_path);
  const pinia = resolveRuntimePinia();
  if (pinia) {
    useCommandActionStore(pinia).applyLatestMetaReportState(
      canOpenReport ? reportPath : "",
      canOpenReport ? true : reportPath ? false : null,
    );
  }
  if (typeof document !== "undefined") {
    if (canOpenReport) {
      document.documentElement.dataset.latestMetaReportPath = reportPath;
      document.documentElement.dataset.latestMetaReportExists = "1";
    } else {
      delete document.documentElement.dataset.latestMetaReportPath;
      if (reportPath) {
        document.documentElement.dataset.latestMetaReportExists = "0";
      } else {
        delete document.documentElement.dataset.latestMetaReportExists;
      }
    }
  }

  const warnText = warnings.length ? ` · warn=${warnings.length}` : "";
  const evidenceText = groupsWithEvidenceFlags
    ? ` · weak=${groupsWithEvidenceFlags}`
    : "";
  const holdoutText = holdoutDatasets
    ? ` · holdout=${holdoutDatasets}${holdoutSkipped ? ` (skipped ${holdoutSkipped})` : ""}`
    : "";
  const holdoutCoverageText = holdoutCoverage
    ? ` · holdout_coverage=${Math.max(0, Math.floor(Number(holdoutCoverage.groups_evaluated || 0)))}/${Math.max(0, Math.floor(Number(holdoutCoverage.groups_total || 0)))} ${normalizeText(holdoutCoverage.coverage_mode) || "unknown"}`
    : "";
  const cohortText = cohortDisjointness
    ? ` · cohort_disjoint=${normalizeBoolean(cohortDisjointness.disjoint) ? "yes" : "no"}`
    : "";
  const rankMode = groups.some((group) =>
    Number.isFinite(Number(group.final_score)),
  )
    ? " · rank=holdout-adjusted"
    : "";

  const metaMeta = result
    ? `groups=${groups.length} · artifacts=${Math.max(0, Math.floor(Number(result.artifacts_used || 0)))}/${Math.max(0, Math.floor(Number(result.artifacts_considered || 0)))} · candidates=${Math.max(0, Math.floor(Number(result.candidates_considered || 0)))} · runtime=${Math.max(0, Math.floor(Number(result.runtime_ms || 0)))}ms${holdoutText}${holdoutCoverageText}${cohortText}${rankMode}${evidenceText}${warnText}`
    : "—";

  const previewLines: string[] = [];
  if (result) {
    previewLines.push(
      `groups=${groups.length} artifacts=${Math.max(0, Math.floor(Number(result.artifacts_used || 0)))}/${Math.max(0, Math.floor(Number(result.artifacts_considered || 0)))} candidates=${Math.max(0, Math.floor(Number(result.candidates_considered || 0)))} runtime=${Math.max(0, Math.floor(Number(result.runtime_ms || 0)))}ms`,
    );
    if (holdoutDatasets) {
      previewLines.push(
        `holdout datasets=${holdoutDatasets}${holdoutSkipped ? ` skipped=${holdoutSkipped}` : ""}`,
      );
    }
    if (holdoutCoverage) {
      previewLines.push(
        `holdout coverage=${Math.max(0, Math.floor(Number(holdoutCoverage.groups_evaluated || 0)))}/${Math.max(0, Math.floor(Number(holdoutCoverage.groups_total || 0)))} mode=${normalizeText(holdoutCoverage.coverage_mode) || "unknown"} policy=${normalizeText(holdoutCoverage.selection_policy) || "unknown"}`,
      );
    }
    if (cohortDisjointness) {
      previewLines.push(
        `cohort train=${Math.max(0, Math.floor(Number(cohortDisjointness.train_artifacts || 0)))} holdout=${Math.max(0, Math.floor(Number(cohortDisjointness.holdout_artifacts || 0)))} disjoint=${normalizeBoolean(cohortDisjointness.disjoint) ? "yes" : "no"} overlap=${Math.max(0, Math.floor(Number(cohortDisjointness.overlapping_dataset_keys || 0)))}`,
      );
    }
    if (groupsWithEvidenceFlags > 0) {
      previewLines.push(
        `evidence warning: ${groupsWithEvidenceFlags} group(s) have thin or noisy evidence`,
      );
      const flagged = groups
        .map((group, index) => {
          const evidenceFlags = Array.isArray(group.evidence_flags)
            ? group.evidence_flags
                .map((item) => normalizeText(item))
                .filter((item) => item.length > 0)
            : [];
          if (!evidenceFlags.length) return "";
          const params = asRecord(group.params) ?? {};
          const fast =
            `${normalizeText(params.fast_ma_type).toUpperCase()} ${Math.max(0, Math.floor(Number(params.fast_len || 0)))}`.trim();
          const slow =
            `${normalizeText(params.slow_ma_type).toUpperCase()} ${Math.max(0, Math.floor(Number(params.slow_len || 0)))}`.trim();
          return `#${index + 1} ${fast} vs ${slow}: ${evidenceFlags.join(", ")}`;
        })
        .filter((line) => line.length > 0);
      previewLines.push(...flagged.slice(0, 8));
    }
    if (warnings.length > 0) {
      previewLines.push(
        `warnings=${warnings.length}: ${warnings
          .map((item) => normalizeText(item))
          .filter((item) => item.length > 0)
          .join(" | ")}`,
      );
    }
    previewLines.push("");
  }

  const latestGroups = String(groups.length);
  const latestArtifacts = result
    ? `${Math.max(0, Math.floor(Number(result.artifacts_used || 0)))}/${Math.max(0, Math.floor(Number(result.artifacts_considered || 0)))}`
    : "—";
  const latestHoldout = holdoutDatasets
    ? `${holdoutDatasets}${holdoutSkipped ? ` (skip ${holdoutSkipped})` : ""}`
    : result
      ? "0"
      : "—";
  const runtimeMs = Math.max(0, Math.floor(Number(result?.runtime_ms || 0)));
  const latestRuntime = result
    ? runtimeMs >= 1000
      ? `${(runtimeMs / 1000).toFixed(2)}s`
      : `${runtimeMs}ms`
    : "—";

  patchAnalysisStore({
    metaMeta,
    metaOutputPreview: result
      ? `${previewLines.join("\n")}${pretty(result)}`
      : "-",
    metaTableColumns: META_TABLE_COLUMNS,
    metaTableRows: markRaw(buildMetaRows(groups)),
    canOpenMetaReport: canOpenReport,
    canRevealMetaFiles: !!(reportPath || metaPath),
    aggregateSummaryLatestGroups: latestGroups,
    aggregateSummaryLatestArtifacts: latestArtifacts,
    aggregateSummaryLatestHoldout: latestHoldout,
    aggregateSummaryLatestRuntime: latestRuntime,
    statusLine: canOpenReport
      ? `Meta complete. Saved report: ${reportPath}`
      : result
        ? "Meta complete."
        : "Ready.",
  });

  if (!resp || previousMetaPath !== metaPath) {
    LAST_META_STAGE4_RUN = null;
    LAST_META_ML_POLICY_EVAL = null;
    patchAnalysisStore({
      metaStage4Status: "—",
      metaStage4OutputPreview: "-",
      metaPolicyEvalStatus: "—",
      metaPolicyEvalOutputPreview: "-",
      canRevealMetaStage4: false,
    });
  }

  updateMetaMlButtons();
  updateMetaStage4Buttons();
}

function formatStage5EvidencePolicyMeta(result: Record<string, unknown>): string {
  const summary = asRecord(result.evidence_policy_summary);
  const metrics = asRecordArray(summary?.metrics);
  if (!metrics.length) return "";
  const confidence = metrics.find(
    (metric) => normalizeText(metric.metric) === "ml_confidence",
  );
  const decision = metrics.find(
    (metric) => normalizeText(metric.metric) === "decision_bucket",
  );
  const parts: string[] = [];
  if (confidence) {
    parts.push("ml_confidence=model_estimated_holdout_success_probability");
  }
  if (decision) {
    parts.push("decision_bucket=review_label_not_deployability_proof");
  }
  return parts.length > 0 ? `evidence=${parts.join(" ")}` : "";
}

function formatEvidenceTier(value: unknown): string {
  const raw = normalizeText(value);
  if (!raw) return "";
  return raw.split("_").join(" ");
}

function renderMetaStage4(resp: Record<string, unknown> | null): void {
  LAST_META_STAGE4_RUN = resp;
  const result = asRecord(resp?.result);
  const groups = asRecordArray(result?.selected_groups);
  const decisionCounts = asRecord(result?.decision_counts) ?? {};
  const path = responseStage5Path(resp);
  const sourceMetaPath = normalizePath(result?.source_meta_json_path);
  const trainingManifest = asRecord(result?.training_manifest);
  const evidenceTier = formatEvidenceTier(result?.model_evidence_tier);
  const warningCount = Math.max(
    0,
    Math.floor(Number(result?.warning_count ?? 0)),
  );
  const warnings = Array.isArray(result?.warnings)
    ? result.warnings
        .map((item) => normalizeText(item))
        .filter((item) => item.length > 0)
    : [];

  if (!result || groups.length === 0) {
    patchAnalysisStore({
      metaStage4Status: "—",
      metaStage4OutputPreview: "-",
    });
    updateMetaStage4Buttons();
    return;
  }

  const lines: string[] = [];
  lines.push(
    `scope=${normalizeText(result.strategy_kind)}/${normalizeText(result.timeframe)} selected=${Math.max(0, Math.floor(Number(result.groups_selected || 0)))} scored=${Math.max(0, Math.floor(Number(result.groups_scored || 0)))}/${Math.max(0, Math.floor(Number(result.groups_considered || 0)))} min_ml=${fmtFixed(result.min_ml_confidence, 3)}`,
  );
  lines.push(
    `source_meta=${sourceMetaPath ? fileBasename(sourceMetaPath) : "—"} model=${normalizeText(result.model_path) ? fileBasename(normalizeText(result.model_path)) : "—"} state=${normalizeText(result.model_state) || "unknown"} healthy=${normalizeBoolean(result.model_healthy) ? "yes" : "no"} trained_now=${normalizeBoolean(result.model_trained_now) ? "yes" : "no"}`,
  );
  if (evidenceTier) {
    lines.push(
      `model_evidence_tier=${evidenceTier} training_artifacts=${Math.max(0, Math.floor(Number(result.training_artifact_count || trainingManifest?.training_artifact_count || 0)))} source_sha256=${normalizeText(result.source_artifact_sha256) || "—"}`,
    );
  }
  lines.push(
    "execution=run_meta_stage5_ml_confidence artifact=json replay=stage5_saved",
  );
  lines.push(
    `weights(meta/ml/unc)=${fmtFixed(result.meta_weight, 3)}/${fmtFixed(result.ml_weight, 3)}/${fmtFixed(result.uncertainty_weight, 3)} buckets(accept/review/reject)=${Math.max(0, Math.floor(Number(decisionCounts.accept || 0)))}/${Math.max(0, Math.floor(Number(decisionCounts.review || 0)))}/${Math.max(0, Math.floor(Number(decisionCounts.reject || 0)))}`,
  );
  lines.push(
    `policy accept>=${fmtFixed(result.accept_confidence, 3)} review>=${fmtFixed(result.review_confidence, 3)} max_accept_unc=${fmtFixed(result.max_accept_uncertainty, 3)}`,
  );
  lines.push(
    "selection=weighted rank -> min_ml filter -> optional meta fallback -> decision buckets",
  );
  const evidencePolicyMeta = formatStage5EvidencePolicyMeta(result);
  if (evidencePolicyMeta) {
    lines.push(evidencePolicyMeta);
  }
  lines.push(
    "auto_train excludes the active source meta artifact from history to avoid train/score leakage",
  );
  if (trainingManifest) {
    lines.push(
      `training_history=${normalizeText(trainingManifest.policy) || "source_excluded"} artifacts=${Math.max(0, Math.floor(Number(trainingManifest.training_artifact_count || 0)))} excluded_source=${Math.max(0, Math.floor(Number(trainingManifest.excluded_source_count || 0)))} excluded_future=${Math.max(0, Math.floor(Number(trainingManifest.excluded_future_or_equal_count || 0)))} excluded_missing_ts=${Math.max(0, Math.floor(Number(trainingManifest.excluded_missing_ts_count || 0)))}`,
    );
  }
  const fallbackReason = normalizeText(result.fallback_reason);
  if (fallbackReason) {
    lines.push(`fallback=${fallbackReason}`);
  }

  for (let index = 0; index < groups.length; index += 1) {
    const row = groups[index] ?? {};
    const params = asRecord(row.params) ?? {};
    const rank = Math.max(
      1,
      Math.floor(Number(row.stage5_rank ?? row.stage4_rank ?? index + 1)),
    );
    const bucket = normalizeText(row.decision_bucket).toLowerCase() || "reject";
    const confText = Number.isFinite(Number(row.ml_confidence))
      ? `${fmtPct(row.ml_confidence, 1)}`
      : "-";
    const uncText = Number.isFinite(Number(row.ml_uncertainty))
      ? fmtFixed(row.ml_uncertainty, 3)
      : "-";
    const fast =
      `${normalizeText(params.fast_ma_type).toUpperCase()} ${Math.max(0, Math.floor(Number(params.fast_len || 0)))}`.trim();
    const slow =
      `${normalizeText(params.slow_ma_type).toUpperCase()} ${Math.max(0, Math.floor(Number(params.slow_len || 0)))}`.trim();
    const finalRank = Number(row.final_rank_score);
    const rankScore = Number.isFinite(finalRank)
      ? finalRank
      : Number.isFinite(Number(row.final_score))
        ? Number(row.final_score)
        : Number(row.meta_score || 0);
    lines.push(
      `#${rank} ${bucket} ml=${confText} unc=${uncText} final=${fmtFixed(rankScore, 6)} ${fast} vs ${slow}`.trim(),
    );
  }

  if (warnings.length > 0) {
    lines.push(
      `warnings=${warnings.length}: ${warnings.join(" | ")}`,
    );
  }
  const effectiveWarningCount = Math.max(warningCount, warnings.length);
  const warningStatus =
    effectiveWarningCount > 0 ? ` · warn=${effectiveWarningCount}` : "";

  patchAnalysisStore({
    metaStage4Status: `Stage 5 complete: selected=${Math.max(0, Math.floor(Number(result.groups_selected || 0)))} scored=${Math.max(0, Math.floor(Number(result.groups_scored || 0)))}/${Math.max(0, Math.floor(Number(result.groups_considered || 0)))} accept/review/reject=${Math.max(0, Math.floor(Number(decisionCounts.accept || 0)))}/${Math.max(0, Math.floor(Number(decisionCounts.review || 0)))}/${Math.max(0, Math.floor(Number(decisionCounts.reject || 0)))}${evidenceTier ? ` · evidence=${evidenceTier}` : ""}${warningStatus} · file=${path ? fileBasename(path) : "—"}`,
    metaStage4OutputPreview: lines.join("\n"),
  });

  updateMetaStage4Buttons();
}

function renderMlPolicyEvaluation(resp: Record<string, unknown> | null): void {
  LAST_META_ML_POLICY_EVAL = resp;
  const result = asRecord(resp?.result);
  const path = normalizePath(resp?.eval_json_path);

  if (!result) {
    patchAnalysisStore({
      metaPolicyEvalStatus: "—",
      metaPolicyEvalOutputPreview: "-",
    });
    updateMetaStage4Buttons();
    return;
  }

  const metaOnly = asRecord(result.meta_only) ?? {};
  const mlOnly = asRecord(result.ml_only) ?? {};
  const stage5Blended = asRecord(result.stage5_blended) ?? {};
  const warnings = Array.isArray(result.warnings) ? result.warnings : [];
  const bestPolicy = normalizeText(result.best_policy) || "—";
  const deltaVsMeta = Number(result.stage5_vs_meta_precision_delta);
  const deltaVsMl = Number(result.stage5_vs_ml_precision_delta);

  const lines = [
    `scope=${normalizeText(result.strategy_kind)}/${normalizeText(result.timeframe)} evaluated=${Math.max(0, Math.floor(Number(result.artifacts_evaluated || 0)))}/${Math.max(0, Math.floor(Number(result.artifacts_considered || 0)))} top_k=${Math.max(0, Math.floor(Number(result.top_k || 0)))}`,
    "contract=offline scoped history over saved Meta artifacts • request reuses current Stage 5 thresholds/weights • artifact=json replay only",
    "arms=meta_only / ml_only / stage5_blended",
    `skipped(no_history/train_fail/unhealthy/no_labels)=${Math.max(0, Math.floor(Number(result.artifacts_skipped_no_history || 0)))}/${Math.max(0, Math.floor(Number(result.artifacts_skipped_training_failure || 0)))}/${Math.max(0, Math.floor(Number(result.artifacts_skipped_unhealthy_model || 0)))}/${Math.max(0, Math.floor(Number(result.artifacts_skipped_no_labels || 0)))}`,
    `meta_only: precision@k=${fmtPct(metaOnly.precision_at_k, 1)} positives=${Math.max(0, Math.floor(Number(metaOnly.positives_total || 0)))}/${Math.max(0, Math.floor(Number(metaOnly.selected_total || 0)))}`,
    `ml_only: precision@k=${fmtPct(mlOnly.precision_at_k, 1)} positives=${Math.max(0, Math.floor(Number(mlOnly.positives_total || 0)))}/${Math.max(0, Math.floor(Number(mlOnly.selected_total || 0)))}`,
    `stage5_blended: precision@k=${fmtPct(stage5Blended.precision_at_k, 1)} positives=${Math.max(0, Math.floor(Number(stage5Blended.positives_total || 0)))}/${Math.max(0, Math.floor(Number(stage5Blended.selected_total || 0)))}`,
    `best_policy=${bestPolicy} delta_vs_meta=${Number.isFinite(deltaVsMeta) ? fmtPct(deltaVsMeta, 1) : "-"} delta_vs_ml=${Number.isFinite(deltaVsMl) ? fmtPct(deltaVsMl, 1) : "-"}`,
  ];

  if (warnings.length > 0) {
    lines.push(
      `warnings=${warnings.length}: ${warnings
        .map((item) => normalizeText(item))
        .filter((item) => item.length > 0)
        .join(" | ")}`,
    );
  }

  patchAnalysisStore({
    metaPolicyEvalStatus: `Policy evaluation complete: evaluated=${Math.max(0, Math.floor(Number(result.artifacts_evaluated || 0)))}/${Math.max(0, Math.floor(Number(result.artifacts_considered || 0)))} · best=${bestPolicy} · stage5_precision=${fmtPct(stage5Blended.precision_at_k, 1)} · file=${path ? fileBasename(path) : "—"}`,
    metaPolicyEvalOutputPreview: lines.join("\n"),
  });

  updateMetaStage4Buttons();
}

async function maybeRunMetaMlAutopilot(): Promise<void> {
  const store = getAnalysisStore();
  if (!store?.metaMlAutopilot) return;

  const scope = currentMetaScope();
  if (!normalizeText(scope.timeframe)) return;

  await mlRefresh({ scope });

  const status = asRecord(LAST_META_ML_STATUS) ?? {};
  const modelExists = normalizeBoolean(status.exists);
  const modelHealthy = normalizeBoolean(status.model_healthy);
  const modelStale = normalizeBoolean(status.stale);
  const retrainRecommended = normalizeBoolean(status.retrain_recommended);

  if (!modelExists || !modelHealthy || modelStale || retrainRecommended) {
    await mlTrain({ scope });
    return;
  }

  const hasMetaGroups =
    asRecordArray(asRecord(LAST_META_RUN?.result)?.groups).length > 0;
  if (store.metaUseMlConfidence && hasMetaGroups) {
    await mlApply({ scope });
    return;
  }

  setStatusLine(
    "Meta analysis complete. ML autopilot verified the current model.",
  );
}

function formatMlStatus(
  status: Record<string, unknown>,
  scope: { strategy_kind: string; timeframe: string },
): string {
  const exists = normalizeBoolean(status.exists);
  const errorText = normalizeText(status.error);
  const availableMeta = Math.max(
    0,
    Math.floor(Number(status.available_meta_artifacts || 0)),
  );
  const availableRows = Math.max(
    0,
    Math.floor(Number(status.available_labeled_rows || 0)),
  );
  const newest = normalizeText(status.newest_meta_ts_available);
  const stale = normalizeBoolean(status.stale);
  const retrain = normalizeBoolean(status.retrain_recommended);
  const retrainReason = normalizeText(status.retrain_reason);
  const modelState = normalizeText(status.state) || "missing";
  const legacyModel = normalizeBoolean(status.legacy_model);
  const modelHealthyFlag = status.model_healthy;

  if (!exists) {
    const metaScanError = normalizeText(status.meta_scan_error);
    return `No ML model saved for ${scope.strategy_kind} / ${scope.timeframe}. · state=${modelState} · available: meta=${availableMeta}, rows=${availableRows}${newest ? ` · newest=${newest}` : ""}${metaScanError ? ` · meta_scan_error=${metaScanError}` : ""}`;
  }

  if (errorText) {
    const metaScanError = normalizeText(status.meta_scan_error);
    return `ML model exists but failed to load: ${errorText} · state=${modelState} · available: meta=${availableMeta}, rows=${availableRows}${newest ? ` · newest=${newest}` : ""}${metaScanError ? ` · meta_scan_error=${metaScanError}` : ""}`;
  }

  const modelSummary = asRecord(status.model_summary);
  const label = asRecord(modelSummary?.label);
  const modelMeta = asRecord(modelSummary?.meta);
  const modelPath = normalizePath(status.model_path);
  const metaScanError = normalizeText(status.meta_scan_error);

  const healthy =
    modelHealthyFlag === false || modelMeta?.healthy === false
      ? "UNHEALTHY"
      : modelHealthyFlag === true || modelMeta?.healthy === true
        ? "healthy"
        : "unknown";
  const target = normalizeText(modelMeta?.target);
  const evidenceTier = formatEvidenceTier(modelMeta?.evidence_tier);
  const modelKind = normalizeText(modelSummary?.model_kind);
  const trainedRows = Number(modelMeta?.trained_rows);
  const auc = Number(modelMeta?.cv_auc);
  const brierRaw = Number(modelMeta?.cv_brier_calibrated);
  const brier = Number.isFinite(brierRaw)
    ? brierRaw
    : Number(modelMeta?.cv_brier);
  const baselineBrier = Number(modelMeta?.baseline_brier);

  const parts = [
    `Model: ${fileBasename(modelPath) || modelPath || "model.json"}`,
    `bytes=${Math.max(0, Math.floor(Number(status.bytes || 0)))}`,
    `state=${modelState}`,
    healthy,
  ];

  if (modelKind) parts.push(`model_kind=${modelKind}`);
  if (target) parts.push(`target=${target}`);
  if (evidenceTier) parts.push(`evidence=${evidenceTier}`);
  if (legacyModel) parts.push("legacy");
  if (Number.isFinite(trainedRows))
    parts.push(`trained_rows=${Math.max(0, Math.floor(trainedRows))}`);
  if (Number.isFinite(auc)) parts.push(`cv_auc=${auc.toFixed(3)}`);
  if (Number.isFinite(brier)) parts.push(`cv_brier=${brier.toFixed(4)}`);
  if (Number.isFinite(baselineBrier))
    parts.push(`baseline=${baselineBrier.toFixed(4)}`);

  const passRate = Number(label?.min_holdout_pass_rate);
  const scoreP10 = Number(label?.min_holdout_score_p10);
  const minDatasets = Number(label?.min_holdout_datasets);
  const ddP90 = Number(label?.max_holdout_max_dd_p90);
  if (
    Number.isFinite(passRate) ||
    Number.isFinite(scoreP10) ||
    Number.isFinite(minDatasets) ||
    Number.isFinite(ddP90)
  ) {
    const ddSuffix = Number.isFinite(ddP90)
      ? ` dd_p90<=${ddP90.toFixed(3)}`
      : "";
    parts.push(
      `label: pass>=${Number.isFinite(passRate) ? passRate.toFixed(2) : "0.00"}, p10>=${Number.isFinite(scoreP10) ? scoreP10.toFixed(3) : "0.000"}, datasets>=${Number.isFinite(minDatasets) ? Math.max(0, Math.floor(minDatasets)) : 0}${ddSuffix}`,
    );
  }

  parts.push(`available: meta=${availableMeta}, rows=${availableRows}`);
  if (newest) parts.push(`newest=${newest}`);
  parts.push(stale ? "stale" : "fresh");
  if (retrain) parts.push("retrain recommended");
  if (retrainReason) parts.push(retrainReason);
  if (metaScanError) parts.push(`meta_scan_error=${metaScanError}`);

  return parts.join(" · ");
}

function updateMetaMlButtons(): void {
  const scope = currentMetaScope();
  const hasTf = !!normalizeText(scope.timeframe);
  const hasMeta = !!asRecord(LAST_META_RUN?.result);
  const hasModel =
    !!normalizeBoolean(LAST_META_ML_STATUS?.exists) &&
    !normalizeText(LAST_META_ML_STATUS?.error);

  patchAnalysisStore({
    canMetaMlRefresh: hasTf,
    canMetaMlTrain: hasTf,
    canMetaMlApply: hasTf && hasMeta && hasModel,
    canMetaMlDelete: hasTf && hasModel,
  });
}

function updateMetaStage4Buttons(): void {
  const hasTf = !!normalizeText(currentMetaScope().timeframe);
  const result = asRecord(LAST_META_RUN?.result);
  const groups = asRecordArray(result?.groups);
  const stage4Path = responseStage5Path(LAST_META_STAGE4_RUN);
  patchAnalysisStore({
    canRunMetaStage4: hasTf && groups.length > 0,
    canRevealMetaStage4: !!stage4Path,
    canRunMetaPolicyEval: hasTf,
  });
}

function filterMetaSaved(
  items: Record<string, unknown>[],
  query: string,
): Record<string, unknown>[] {
  const q = normalizeText(query).toLowerCase();
  if (!q) return items;
  const terms = q.split(/\s+/).filter((token) => token.length > 0);
  if (terms.length === 0) return items;

  return items.filter((item) => {
    const tags = Array.isArray(item.tags)
      ? item.tags.map((tag) => normalizeText(tag).toLowerCase()).join(" ")
      : "";
    const haystack = [
      normalizeText(item.ts).toLowerCase(),
      normalizeText(item.strategy_kind).toLowerCase(),
      normalizeText(item.timeframe).toLowerCase(),
      normalizeText(item.required_tag).toLowerCase(),
      tags,
    ].join(" ");
    return terms.every((term) => haystack.includes(term));
  });
}

function filterStage4Saved(
  items: Record<string, unknown>[],
  query: string,
): Record<string, unknown>[] {
  const q = normalizeText(query).toLowerCase();
  if (!q) return items;
  const terms = q.split(/\s+/).filter((token) => token.length > 0);
  if (terms.length === 0) return items;

  return items.filter((item) => {
    const tags = Array.isArray(item.tags)
      ? item.tags.map((tag) => normalizeText(tag).toLowerCase()).join(" ")
      : "";
    const haystack = [
      normalizeText(item.ts).toLowerCase(),
      normalizeText(item.strategy_kind).toLowerCase(),
      normalizeText(item.timeframe).toLowerCase(),
      normalizeText(item.source_meta_json_path).toLowerCase(),
      normalizeText(item.model_evidence_tier).toLowerCase(),
      tags,
    ].join(" ");
    return terms.every((term) => haystack.includes(term));
  });
}

function filterPolicyEvalSaved(
  items: Record<string, unknown>[],
  query: string,
): Record<string, unknown>[] {
  const q = normalizeText(query).toLowerCase();
  if (!q) return items;
  const terms = q.split(/\s+/).filter((token) => token.length > 0);
  if (terms.length === 0) return items;

  return items.filter((item) => {
    const tags = Array.isArray(item.tags)
      ? item.tags.map((tag) => normalizeText(tag).toLowerCase()).join(" ")
      : "";
    const haystack = [
      normalizeText(item.ts).toLowerCase(),
      normalizeText(item.strategy_kind).toLowerCase(),
      normalizeText(item.timeframe).toLowerCase(),
      normalizeText(item.best_policy).toLowerCase(),
      tags,
    ].join(" ");
    return terms.every((term) => haystack.includes(term));
  });
}

function updateMetaSavedState(preferredPath = ""): void {
  const store = getAnalysisStore();
  if (!store) return;

  const filtered = filterMetaSaved(SAVED_META, store.metaSavedSearch);
  let selectedPath = normalizePath(preferredPath || store.metaSavedSelected);
  if (
    selectedPath &&
    !filtered.some((entry) => normalizePath(entry.path) === selectedPath)
  ) {
    selectedPath = "";
  }

  const selectedEntry =
    SAVED_META.find((entry) => normalizePath(entry.path) === selectedPath) ??
    null;
  const selectedTags =
    selectedEntry && Array.isArray(selectedEntry.tags)
      ? selectedEntry.tags
          .map((tag) => normalizeText(tag))
          .filter((tag) => tag.length > 0)
          .join(", ")
      : "";

  const metaSavedMeta = selectedEntry
    ? [
        `ts=${normalizeText(selectedEntry.ts)}`,
        `strategy=${normalizeText(selectedEntry.strategy_kind) || "—"}`,
        normalizeText(selectedEntry.timeframe)
          ? `tf=${normalizeText(selectedEntry.timeframe)}`
          : "",
        normalizeText(selectedEntry.required_tag)
          ? `tag=${normalizeText(selectedEntry.required_tag)}`
          : "",
        `groups=${Math.max(0, Math.floor(Number(selectedEntry.groups || 0)))}`,
        `artifacts=${Math.max(0, Math.floor(Number(selectedEntry.artifacts_used || 0)))}`,
        Number(selectedEntry.holdout_datasets || 0) > 0
          ? `holdout=${Math.max(0, Math.floor(Number(selectedEntry.holdout_datasets || 0)))}`
          : "",
        Array.isArray(selectedEntry.tags) && selectedEntry.tags.length > 0
          ? `tags=${selectedEntry.tags
              .map((tag) => normalizeText(tag))
              .filter((tag) => tag.length > 0)
              .join(",")}`
          : "",
        selectedEntry.bytes != null
          ? `bytes=${Math.max(0, Math.floor(Number(selectedEntry.bytes || 0)))}`
          : "",
      ]
        .filter((part) => part.length > 0)
        .join(" • ")
    : `saved meta: ${filtered.length}/${SAVED_META.length}`;

  const canOpenSavedReport =
    !!selectedEntry &&
    canOpenPersistedReport(
      selectedEntry?.report_xlsx_path,
      selectedEntry?.report_xlsx_exists ?? selectedEntry?.reportXlsxExists,
    );

  const options = [
    {
      value: "",
      label:
        filtered.length > 0 ? "Select a saved meta run..." : "No meta runs yet",
      disabled: false,
    },
    ...filtered.map((entry) => {
      const ts = normalizeText(entry.ts) || "unknown";
      const kind = normalizeText(entry.strategy_kind);
      const tf = normalizeText(entry.timeframe);
      const groups = Math.max(0, Math.floor(Number(entry.groups || 0)));
      const artifacts = Math.max(
        0,
        Math.floor(Number(entry.artifacts_used || 0)),
      );
      const holdout = Math.max(
        0,
        Math.floor(Number(entry.holdout_datasets || 0)),
      );
      const tagsCount = Array.isArray(entry.tags) ? entry.tags.length : 0;
      const tfHint = tf ? ` | ${tf}` : "";
      const holdoutHint = holdout ? ` | holdout=${holdout}` : "";
      const tagsHint = tagsCount ? ` | tags=${tagsCount}` : "";
      return {
        value: normalizePath(entry.path),
        label: `${ts} | ${kind}${tfHint} | groups=${groups} | arts=${artifacts}${holdoutHint}${tagsHint}`,
        disabled: false,
      };
    }),
  ];

  let compareSelected = normalizePath(store.metaCompareSelected);
  if (
    compareSelected &&
    !SAVED_META.some((entry) => normalizePath(entry.path) === compareSelected)
  ) {
    compareSelected = "";
  }
  const compareOptions = [
    {
      value: "",
      label:
        SAVED_META.length > 0
          ? "Compare against saved meta..."
          : "No meta runs yet",
      disabled: false,
    },
    ...SAVED_META.map((entry) => {
      const ts = normalizeText(entry.ts) || "unknown";
      const kind = normalizeText(entry.strategy_kind);
      const tf = normalizeText(entry.timeframe);
      const groups = Math.max(0, Math.floor(Number(entry.groups || 0)));
      const artifacts = Math.max(
        0,
        Math.floor(Number(entry.artifacts_used || 0)),
      );
      const tfHint = tf ? ` | ${tf}` : "";
      const artifactsHint = artifacts ? ` | arts=${artifacts}` : "";
      return {
        value: normalizePath(entry.path),
        label: `${ts} | ${kind}${tfHint} | groups=${groups}${artifactsHint}`,
        disabled: false,
      };
    }),
  ];

  patchAnalysisStore({
    metaSavedOptions: markRaw(options),
    metaSavedSelected: selectedPath,
    metaSavedTagsDraft: selectedTags,
    metaSavedMeta,
    canLoadMetaSaved: !!selectedEntry,
    canOpenMetaSavedReport: canOpenSavedReport,
    canRevealMetaSaved: !!selectedEntry,
    canDeleteMetaSaved: !!selectedEntry,
    canSaveMetaSavedTags: !!selectedEntry,
    canRefreshMetaSaved: true,
    metaCompareOptions: markRaw(compareOptions),
    metaCompareSelected: compareSelected,
    aggregateSummarySavedMeta: String(SAVED_META.length),
  });
}

function updateStage4SavedState(preferredPath = ""): void {
  const store = getAnalysisStore();
  if (!store) return;

  const filtered = filterStage4Saved(SAVED_STAGE4, store.stage4SavedSearch);
  let selectedPath = normalizePath(preferredPath || store.stage4SavedSelected);
  if (
    selectedPath &&
    !filtered.some((entry) => normalizePath(entry.path) === selectedPath)
  ) {
    selectedPath = "";
  }

  const selectedEntry =
    SAVED_STAGE4.find((entry) => normalizePath(entry.path) === selectedPath) ??
    null;
  const selectedTags =
    selectedEntry && Array.isArray(selectedEntry.tags)
      ? selectedEntry.tags
          .map((tag) => normalizeText(tag))
          .filter((tag) => tag.length > 0)
          .join(", ")
      : "";

  const stage4SavedMeta = selectedEntry
    ? [
        `ts=${normalizeText(selectedEntry.ts)}`,
        normalizeText(selectedEntry.kind)
          ? `kind=${normalizeText(selectedEntry.kind)}`
          : "",
        `strategy=${normalizeText(selectedEntry.strategy_kind) || "—"}`,
        normalizeText(selectedEntry.timeframe)
          ? `tf=${normalizeText(selectedEntry.timeframe)}`
          : "",
        `selected=${Math.max(0, Math.floor(Number(selectedEntry.groups_selected || 0)))}/${Math.max(0, Math.floor(Number(selectedEntry.groups_scored || 0)))}/${Math.max(0, Math.floor(Number(selectedEntry.groups_considered || 0)))}`,
        Number.isFinite(Number(selectedEntry.min_ml_confidence))
          ? `min_ml=${fmtFixed(selectedEntry.min_ml_confidence, 3)}`
          : "",
        Number.isFinite(Number(selectedEntry.top_k))
          ? `top_k=${Math.max(0, Math.floor(Number(selectedEntry.top_k || 0)))}`
          : "",
        `model=${normalizeBoolean(selectedEntry.model_healthy) ? "healthy" : "unhealthy"}`,
        normalizeText(selectedEntry.model_evidence_tier)
          ? `evidence=${formatEvidenceTier(selectedEntry.model_evidence_tier)}`
          : "",
        Number.isFinite(Number(selectedEntry.training_artifact_count))
          ? `training_artifacts=${Math.max(0, Math.floor(Number(selectedEntry.training_artifact_count || 0)))}`
          : "",
        Number.isFinite(Number(selectedEntry.warning_count))
          ? `warn=${Math.max(0, Math.floor(Number(selectedEntry.warning_count || 0)))}`
          : "",
        normalizeText(selectedEntry.source_meta_json_path)
          ? `source=${fileBasename(normalizeText(selectedEntry.source_meta_json_path))}`
          : "",
        "replay=json",
        Array.isArray(selectedEntry.tags) && selectedEntry.tags.length > 0
          ? `tags=${selectedEntry.tags
              .map((tag) => normalizeText(tag))
              .filter((tag) => tag.length > 0)
              .join(",")}`
          : "",
        selectedEntry.bytes != null
          ? `bytes=${Math.max(0, Math.floor(Number(selectedEntry.bytes || 0)))}`
          : "",
      ]
        .filter((part) => part.length > 0)
        .join(" • ")
    : `saved Stage 5: ${filtered.length}/${SAVED_STAGE4.length}`;

  const options = [
    {
      value: "",
      label:
        filtered.length > 0
          ? "Select a saved Stage 5 run..."
          : "No Stage 5 runs yet",
      disabled: false,
    },
    ...filtered.map((entry) => {
      const ts = normalizeText(entry.ts) || "unknown";
      const kind = normalizeText(entry.strategy_kind);
      const tf = normalizeText(entry.timeframe);
      const selected = Math.max(
        0,
        Math.floor(Number(entry.groups_selected || 0)),
      );
      const scored = Math.max(0, Math.floor(Number(entry.groups_scored || 0)));
      const considered = Math.max(
        0,
        Math.floor(Number(entry.groups_considered || 0)),
      );
      const minConf = Number(entry.min_ml_confidence);
      const evidenceTier = formatEvidenceTier(entry.model_evidence_tier);
      const tagsCount = Array.isArray(entry.tags) ? entry.tags.length : 0;
      const tfHint = tf ? ` | ${tf}` : "";
      const confHint = Number.isFinite(minConf)
        ? ` | min_ml=${minConf.toFixed(2)}`
        : "";
      const evidenceHint = evidenceTier ? ` | evidence=${evidenceTier}` : "";
      const tagsHint = tagsCount ? ` | tags=${tagsCount}` : "";
      return {
        value: normalizePath(entry.path),
        label: `${ts} | ${kind}${tfHint} | selected=${selected}/${scored}/${considered}${confHint}${evidenceHint}${tagsHint}`,
        disabled: false,
      };
    }),
  ];

  patchAnalysisStore({
    stage4SavedOptions: markRaw(options),
    stage4SavedSelected: selectedPath,
    stage4SavedTagsDraft: selectedTags,
    stage4SavedMeta,
    canRefreshStage4Saved: true,
    canLoadStage4Saved: !!selectedEntry,
    canRevealStage4Saved: !!selectedEntry,
    canDeleteStage4Saved: !!selectedEntry,
    canSaveStage4SavedTags: !!selectedEntry,
  });
}

function updatePolicyEvalSavedState(preferredPath = ""): void {
  const store = getAnalysisStore();
  if (!store) return;

  const filtered = filterPolicyEvalSaved(
    SAVED_POLICY_EVAL,
    store.policyEvalSavedSearch,
  );
  let selectedPath = normalizePath(
    preferredPath || store.policyEvalSavedSelected,
  );
  if (
    selectedPath &&
    !filtered.some((entry) => normalizePath(entry.path) === selectedPath)
  ) {
    selectedPath = "";
  }

  const selectedEntry =
    SAVED_POLICY_EVAL.find(
      (entry) => normalizePath(entry.path) === selectedPath,
    ) ?? null;
  const selectedTags =
    selectedEntry && Array.isArray(selectedEntry.tags)
      ? selectedEntry.tags
          .map((tag) => normalizeText(tag))
          .filter((tag) => tag.length > 0)
          .join(", ")
      : "";

  const policyEvalSavedMeta = selectedEntry
    ? [
        `ts=${normalizeText(selectedEntry.ts)}`,
        `strategy=${normalizeText(selectedEntry.strategy_kind) || "—"}`,
        normalizeText(selectedEntry.timeframe)
          ? `tf=${normalizeText(selectedEntry.timeframe)}`
          : "",
        `evaluated=${Math.max(0, Math.floor(Number(selectedEntry.artifacts_evaluated || 0)))}/${Math.max(0, Math.floor(Number(selectedEntry.artifacts_considered || 0)))}`,
        normalizeText(selectedEntry.best_policy)
          ? `best=${normalizeText(selectedEntry.best_policy)}`
          : "",
        `stage5_p@k=${fmtPct(selectedEntry.stage5_precision_at_k, 1)}`,
        Number.isFinite(Number(selectedEntry.stage5_vs_meta_precision_delta))
          ? `vs_meta=${fmtPct(selectedEntry.stage5_vs_meta_precision_delta, 1)}`
          : "",
        Number.isFinite(Number(selectedEntry.stage5_vs_ml_precision_delta))
          ? `vs_ml=${fmtPct(selectedEntry.stage5_vs_ml_precision_delta, 1)}`
          : "",
        "arms=meta/ml/stage5",
        "replay=json",
        Array.isArray(selectedEntry.tags) && selectedEntry.tags.length > 0
          ? `tags=${selectedEntry.tags
              .map((tag) => normalizeText(tag))
              .filter((tag) => tag.length > 0)
              .join(",")}`
          : "",
        selectedEntry.bytes != null
          ? `bytes=${Math.max(0, Math.floor(Number(selectedEntry.bytes || 0)))}`
          : "",
      ]
        .filter((part) => part.length > 0)
        .join(" • ")
    : `saved policy eval: ${filtered.length}/${SAVED_POLICY_EVAL.length}`;

  const options = [
    {
      value: "",
      label:
        filtered.length > 0
          ? "Select a saved policy evaluation..."
          : "No policy evaluations yet",
      disabled: false,
    },
    ...filtered.map((entry) => {
      const ts = normalizeText(entry.ts) || "unknown";
      const kind = normalizeText(entry.strategy_kind);
      const tf = normalizeText(entry.timeframe);
      const best = normalizeText(entry.best_policy) || "—";
      const evaluated = Math.max(
        0,
        Math.floor(Number(entry.artifacts_evaluated || 0)),
      );
      const considered = Math.max(
        0,
        Math.floor(Number(entry.artifacts_considered || 0)),
      );
      const tfHint = tf ? ` | ${tf}` : "";
      return {
        value: normalizePath(entry.path),
        label: `${ts} | ${kind}${tfHint} | best=${best} | eval=${evaluated}/${considered} | p@k=${fmtPct(entry.stage5_precision_at_k, 1)}`,
        disabled: false,
      };
    }),
  ];

  patchAnalysisStore({
    policyEvalSavedOptions: markRaw(options),
    policyEvalSavedSelected: selectedPath,
    policyEvalSavedTagsDraft: selectedTags,
    policyEvalSavedMeta,
    canRefreshPolicyEvalSaved: true,
    canLoadPolicyEvalSaved: !!selectedEntry,
    canRevealPolicyEvalSaved: !!selectedEntry,
    canDeletePolicyEvalSaved: !!selectedEntry,
    canSavePolicyEvalSavedTags: !!selectedEntry,
  });
}

async function refreshArtifactLibraryInternal(
  options: {
    preferredMetaPath?: string;
    preferredStage5Path?: string;
    preferredPolicyEvalPath?: string;
  } = {},
): Promise<void> {
  const rows = (await invokeCommand("list_artifact_library")) as unknown;
  const library = asRecord(rows) ?? {};
  SAVED_META = asRecordArray(library.meta);
  SAVED_STAGE4 = asRecordArray(library.stage5 ?? library.stage_5);
  SAVED_POLICY_EVAL = asRecordArray(
    library.policyEvals ?? library.policy_evals,
  );
  updateMetaSavedState(options.preferredMetaPath ?? "");
  updateStage4SavedState(options.preferredStage5Path ?? "");
  updatePolicyEvalSavedState(options.preferredPolicyEvalPath ?? "");
}

async function refreshSavedInternal(preferredPath = ""): Promise<void> {
  await refreshArtifactLibraryInternal({ preferredMetaPath: preferredPath });
}

async function refreshStage4SavedInternal(preferredPath = ""): Promise<void> {
  await refreshArtifactLibraryInternal({ preferredStage5Path: preferredPath });
}

async function refreshPolicyEvalSavedInternal(
  preferredPath = "",
): Promise<void> {
  await refreshArtifactLibraryInternal({
    preferredPolicyEvalPath: preferredPath,
  });
}

function sortSavedEntries(
  entries: Record<string, unknown>[],
): Record<string, unknown>[] {
  return [...entries].sort((lhs, rhs) =>
    normalizeText(rhs.ts).localeCompare(normalizeText(lhs.ts)),
  );
}

function upsertSavedEntry(
  entries: Record<string, unknown>[],
  nextEntry: Record<string, unknown>,
): Record<string, unknown>[] {
  const nextPath = normalizePath(nextEntry.path);
  if (!nextPath) return entries;
  const filtered = entries.filter(
    (entry) => normalizePath(entry.path) !== nextPath,
  );
  return sortSavedEntries([nextEntry, ...filtered]);
}

function buildMetaSavedEntryFromResponse(
  response: Record<string, unknown>,
): Record<string, unknown> {
  const result = asRecord(response.result) ?? {};
  const path = normalizePath(response.meta_json_path);
  const kind = META_ARTIFACT_KINDS.selection;
  return {
    ts: normalizeText(response.ts) || new Date().toISOString(),
    path,
    bytes: Math.max(0, Math.floor(Number(response.meta_json_bytes || 0))),
    kind,
    strategy_kind: normalizeText(result.strategy_kind),
    timeframe: normalizeText(result.timeframe),
    groups: asRecordArray(result.groups).length,
    report_xlsx_path: normalizePath(response.report_xlsx_path),
    report_xlsx_exists: normalizeOptionalBoolean(response.report_xlsx_exists),
    stage_completions: serializeStageCompletionsForArtifact([
      buildMetaStageCompletion({
        ts: response.ts,
        artifactKind: kind,
        artifactPath: path,
        reportXlsxPath: response.report_xlsx_path,
        res: result,
        backfilled: true,
      }),
    ]),
    tags: [],
  };
}

function buildStage5SavedEntryFromResponse(
  response: Record<string, unknown>,
): Record<string, unknown> {
  const result = asRecord(response.result) ?? {};
  const path = responseStage5Path(response);
  const kind = META_ARTIFACT_KINDS.stage5;
  return {
    ts: normalizeText(response.ts) || new Date().toISOString(),
    path,
    bytes: responseStage5Bytes(response),
    kind,
    strategy_kind: normalizeText(result.strategy_kind),
    timeframe: normalizeText(result.timeframe),
    groups_selected: Math.max(
      0,
      Math.floor(Number(result.groups_selected || 0)),
    ),
    groups_scored: Math.max(0, Math.floor(Number(result.groups_scored || 0))),
    groups_considered: Math.max(
      0,
      Math.floor(Number(result.groups_considered || 0)),
    ),
    min_ml_confidence: Number(result.min_ml_confidence),
    top_k: Math.max(0, Math.floor(Number(result.top_k || 0))),
    model_healthy: normalizeBoolean(result.model_healthy),
    model_evidence_tier: normalizeText(result.model_evidence_tier),
    training_artifact_count: Math.max(
      0,
      Math.floor(Number(result.training_artifact_count || 0)),
    ),
    source_artifact_sha256: normalizeText(result.source_artifact_sha256),
    warning_count: Math.max(0, Math.floor(Number(result.warning_count || 0))),
    source_meta_json_path: normalizePath(result.source_meta_json_path),
    stage_completions: serializeStageCompletionsForArtifact([
      buildStage5MlConfidenceStageCompletion({
        ts: response.ts,
        artifactKind: kind,
        artifactPath: path,
        res: result,
        backfilled: true,
      }),
    ]),
    tags: [],
  };
}

function buildPolicyEvalSavedEntryFromResponse(
  response: Record<string, unknown>,
): Record<string, unknown> {
  const result = asRecord(response.result) ?? {};
  const stage5Blended = asRecord(result.stage5_blended) ?? {};
  const path = normalizePath(response.eval_json_path);
  const kind = META_ARTIFACT_KINDS.mlPolicyEvaluation;
  return {
    ts: normalizeText(response.ts) || new Date().toISOString(),
    path,
    bytes: Math.max(0, Math.floor(Number(response.eval_json_bytes || 0))),
    kind,
    strategy_kind: normalizeText(result.strategy_kind),
    timeframe: normalizeText(result.timeframe),
    artifacts_evaluated: Math.max(
      0,
      Math.floor(Number(result.artifacts_evaluated || 0)),
    ),
    artifacts_considered: Math.max(
      0,
      Math.floor(Number(result.artifacts_considered || 0)),
    ),
    best_policy: normalizeText(result.best_policy),
    stage5_precision_at_k: Number(stage5Blended.precision_at_k),
    stage5_vs_meta_precision_delta: Number(
      result.stage5_vs_meta_precision_delta,
    ),
    stage5_vs_ml_precision_delta: Number(result.stage5_vs_ml_precision_delta),
    stage_completions: serializeStageCompletionsForArtifact([
      buildPolicyEvaluationStageCompletion({
        ts: response.ts,
        artifactKind: kind,
        artifactPath: path,
        res: result,
        backfilled: true,
      }),
    ]),
    tags: [],
  };
}

function scheduleDeferredTask(task: () => Promise<void>): void {
  void task().catch((error) => {
    console.warn("Deferred meta refresh failed", error);
  });
}

function applyMlStatusPayload(
  statusRaw: unknown,
  scope: { strategy_kind: string; timeframe: string },
): void {
  LAST_META_ML_STATUS = asRecord(statusRaw) ?? {};
  patchAnalysisStore({
    metaMlStatus: formatMlStatus(LAST_META_ML_STATUS, scope),
  });
  updateMetaMlButtons();
}

function toMlFeatureGroup(
  group: Record<string, unknown>,
): Record<string, unknown> {
  return {
    samples: Math.max(0, Math.floor(normalizeFiniteNumber(group.samples))),
    pass_rate: normalizeFiniteNumber(group.pass_rate),
    robust_score_p10: normalizeFiniteNumber(group.robust_score_p10),
    robust_score_median: normalizeFiniteNumber(group.robust_score_median),
    max_dd_p90: normalizeFiniteNumber(group.max_dd_p90),
    cross_score_median: normalizeMaybeFiniteNumber(group.cross_score_median),
    meta_score: normalizeFiniteNumber(group.meta_score),
  };
}

function didStage5ModelStateChange(
  result: Record<string, unknown> | null,
): boolean {
  if (!result) return false;
  if (normalizeBoolean(result.model_trained_now)) return true;

  const previousStatus = asRecord(LAST_META_ML_STATUS);
  const nextModelPath = normalizePath(result.model_path);
  const nextState = normalizeText(result.model_state);
  const nextHealthy = result.model_healthy;

  if (!previousStatus) {
    return !!nextModelPath || !!nextState || typeof nextHealthy === "boolean";
  }

  const previousModelPath = normalizePath(previousStatus.model_path);
  const previousState = normalizeText(previousStatus.state);
  const previousHealthy = previousStatus.model_healthy;

  return (
    previousModelPath !== nextModelPath ||
    previousState !== nextState ||
    previousHealthy !== nextHealthy
  );
}

function selectedMetaSavedEntry(): Record<string, unknown> | null {
  const store = getAnalysisStore();
  if (!store) return null;
  const path = normalizePath(store.metaSavedSelected);
  if (!path) return null;
  return SAVED_META.find((entry) => normalizePath(entry.path) === path) ?? null;
}

function selectedStage4SavedEntry(): Record<string, unknown> | null {
  const store = getAnalysisStore();
  if (!store) return null;
  const path = normalizePath(store.stage4SavedSelected);
  if (!path) return null;
  return (
    SAVED_STAGE4.find((entry) => normalizePath(entry.path) === path) ?? null
  );
}

function selectedPolicyEvalSavedEntry(): Record<string, unknown> | null {
  const store = getAnalysisStore();
  if (!store) return null;
  const path = normalizePath(store.policyEvalSavedSelected);
  if (!path) return null;
  return (
    SAVED_POLICY_EVAL.find((entry) => normalizePath(entry.path) === path) ??
    null
  );
}

function formatYmdUtc(value: unknown): string {
  return formatUtcDateFromUnixMs(value);
}

function normalizeMaybeInteger(value: unknown): number | null {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return null;
  return Math.floor(parsed);
}

function normalizeProbability(value: unknown): number | null {
  const text = normalizeText(value);
  if (!text) return null;
  const parsed = Number(text);
  if (!Number.isFinite(parsed)) return null;
  return Math.max(0, Math.min(1, parsed));
}

function isCancellationError(error: unknown): boolean {
  const message = normalizeText(error).toLowerCase();
  return message.includes("cancelled") || message.includes("canceled");
}

function normalizeStrategyKind(kindRaw: unknown): string {
  const kind = normalizeText(kindRaw || "DoubleMa") || "DoubleMa";
  if (kind === "DoubleMa") return "DoubleMa";
  if (kind === "DynamicDoubleMa") return "DynamicDoubleMa";
  if (kind === "MlSignal") return "MlSignal";
  if (kind === "Pit") return "Pit";
  if (kind === "Set") return "Set";
  if (kind === "Mts") return "Mts";
  if (kind === "Orc") return "Orc";
  if (kind === "Marrt") return "Marrt";
  if (kind === "Vfct") return "Vfct";
  if (kind === "Rocb") return "Rocb";
  if (kind === "Asr") return "Asr";
  if (kind === "Vcc") return "Vcc";
  if (kind === "Vemr") return "Vemr";
  if (kind === "Vcet") return "Vcet";
  if (kind === "Mkcs") return "Mkcs";
  if (kind === "Kqsr") return "Kqsr";
  if (kind === "Carm") return "Carm";
  if (kind === "Kvdf") return "Kvdf";
  if (kind === "Cdmc") return "Cdmc";
  if (kind === "CompositeCpu") return "CompositeCpu";
  if (kind === "CompositeMo") return "CompositeMo";
  if (kind === "CompositeTrend") return "CompositeTrend";
  if (kind === "RegimeHybrid") return "RegimeHybrid";
  if (kind === "BreakoutMomentum") return "BreakoutMomentum";
  return kind;
}

function isSupportedResearchCampaignStrategy(kind: string): boolean {
  return (
    kind === "DoubleMa" ||
    kind === "Pit" ||
    kind === "Set" ||
    kind === "Mts" ||
    kind === "Orc" ||
    kind === "Marrt" ||
    kind === "Vfct" ||
    kind === "Rocb" ||
    kind === "Asr" ||
    kind === "Vcc" ||
    kind === "Vemr" ||
    kind === "Vcet" ||
    kind === "Mkcs" ||
    kind === "Kqsr" ||
    kind === "Carm" ||
    kind === "Kvdf" ||
    kind === "Cdmc" ||
    kind === "CompositeCpu" ||
    kind === "CompositeMo" ||
    kind === "CompositeTrend" ||
    kind === "RegimeHybrid" ||
    kind === "BreakoutMomentum"
  );
}

function readBatchSelectedPaths(): string[] {
  if (BATCH_SELECTED_PATHS_READY) return Array.from(BATCH_SELECTED_PATHS);
  const paths = readVueBatchSelectedPaths();
  if (!paths) return [];
  BATCH_SELECTED_PATHS = new Set(
    paths.map((path) => normalizePath(path)).filter((path) => path.length > 0),
  );
  BATCH_SELECTED_PATHS_READY = true;
  return Array.from(BATCH_SELECTED_PATHS);
}

function setBatchSelectedPaths(paths: unknown): void {
  const next = Array.isArray(paths)
    ? paths
        .map((entry) => normalizePath(entry))
        .filter((entry) => entry.length > 0)
    : [];
  BATCH_SELECTED_PATHS = new Set(next);
  BATCH_SELECTED_PATHS_READY = true;
}

function normalizeMarketLabel(market: string, adjusted: boolean): string {
  if (!market) return "market";
  if (market !== "stocks") return market;
  return adjusted ? "stocks (adj)" : "stocks (raw)";
}

function normalizeRowsHint(value: unknown): string {
  return formatUiCount(value);
}

function batchSummaryItems(
  summary: Record<string, unknown>,
): Record<string, unknown>[] {
  const items = asRecordArray(summary.items);
  if (items.length > 0) return items;
  const results = asRecord(summary.results);
  return asRecordArray(results?.failed);
}

function batchSummaryFailedPaths(summary: Record<string, unknown>): string[] {
  const items = batchSummaryItems(summary);
  const failed = items
    .filter(
      (item) =>
        normalizeText(item.status).toLowerCase() === "failed" ||
        normalizeText(item.error).length > 0,
    )
    .map((item) => normalizePath(item.csv_path ?? item.path))
    .filter((path) => path.length > 0);
  if (failed.length > 0) return failed;

  const results = asRecord(summary.results);
  return asRecordArray(results?.failed)
    .map((item) => normalizePath(item.csv_path ?? item.path))
    .filter((path) => path.length > 0);
}

function batchSummarySelectedPaths(summary: Record<string, unknown>): string[] {
  const request = asRecord(summary.request);
  const datasets = Array.isArray(request?.datasets)
    ? (request?.datasets as unknown[])
    : [];
  const fromRequest = datasets
    .map((item) => normalizePath(asRecord(item)?.csv_path))
    .filter((path) => path.length > 0);
  if (fromRequest.length > 0) return fromRequest;
  return Array.isArray(summary.selected_paths)
    ? summary.selected_paths
        .map((item) => normalizePath(item))
        .filter((path) => path.length > 0)
    : [];
}

function batchSummaryPrimaryTag(summary: Record<string, unknown>): string {
  const tags = Array.isArray(summary.tags) ? dedupTags(summary.tags) : [];
  if (tags.length > 0) return normalizeText(tags[0]);
  return normalizeText(summary.batch_tag);
}

function batchSummaryRequest(
  summary: Record<string, unknown>,
): Record<string, unknown> | null {
  return asRecord(summary.request);
}

function batchSummaryResumeState(summary: Record<string, unknown>): string {
  return normalizeText(summary.resume_state).toLowerCase();
}

function batchSummaryResumeBlockedReason(
  summary: Record<string, unknown>,
): string {
  const resumeState = batchSummaryResumeState(summary);
  if (resumeState === "blocked_unsafe") {
    return "Current summary is blocked from resume because its campaign evidence is unsafe. Restart the campaign instead.";
  }
  if (resumeState === "blocked_stale") {
    return "Current summary is blocked from resume because its campaign evidence is stale. Restart the campaign instead.";
  }
  return "";
}

function batchCanResumeIncomplete(summary: Record<string, unknown>): boolean {
  if (!batchSummaryRequest(summary)) return false;
  if (batchSummaryResumeBlockedReason(summary)) return false;
  const state = normalizeText(summary.state).toLowerCase();
  if (state === "running" || state === "cancelled" || state === "failed")
    return true;
  if (
    state === "completed_with_failures" ||
    state === "completed_with_warnings"
  )
    return true;
  if (!state) {
    const counts = asRecord(summary.counts);
    return (
      normalizeBoolean(counts?.cancelled) ||
      batchSummaryFailedPaths(summary).length > 0
    );
  }
  return false;
}

function batchFailedCount(): number {
  const summary = LAST_BATCH_SUMMARY?.summary;
  if (!summary) return 0;
  const counts = asRecord(summary.counts);
  const failed = Number(counts?.failed);
  if (Number.isFinite(failed) && failed > 0)
    return Math.max(0, Math.floor(failed));
  return batchSummaryFailedPaths(summary).length;
}

function batchRunDisabledReason(): string {
  if (BATCH_RUNNING) return "campaign already running";

  const optimizeStore = getOptimizeStore();
  const strategyKind = normalizeStrategyKind(optimizeStore?.strategyKind);
  if (!strategyKind) return "choose a strategy in Optimize first";
  if (strategyKind === "MlSignal")
    return "MlSignal uses the dedicated ML Signal workflow";
  if (strategyKind === "DynamicDoubleMa") {
    return "DynamicDoubleMa campaigns are not supported yet";
  }
  if (!isSupportedResearchCampaignStrategy(strategyKind)) {
    return `strategy '${strategyKind}' is not supported for research campaigns`;
  }

  const selectedPaths = readBatchSelectedPaths();
  if (selectedPaths.length === 0) {
    return BATCH_DATASETS_CACHE.length === 0
      ? "refresh datasets and select at least 1 dataset"
      : "select at least 1 dataset";
  }
  if (BATCH_DATASETS_CACHE.length === 0) return "";

  const byPath = new Map<string, BatchDatasetEntry>();
  for (const entry of BATCH_DATASETS_CACHE) byPath.set(entry.path, entry);

  const dataStore = getDataStore();
  const timeframeOverride = normalizeText(dataStore?.dataTimeframe);
  if (timeframeOverride) return "";

  const selectedTimeframes = new Set<string>();
  for (const path of selectedPaths) {
    const timeframe = normalizeText(byPath.get(path)?.timeframe);
    if (timeframe) selectedTimeframes.add(timeframe);
  }
  if (selectedTimeframes.size > 1) {
    const found = Array.from(selectedTimeframes).sort().join(", ");
    return `selected datasets must share a timeframe unless Data → Timeframe forces resampling (${found})`;
  }
  return "";
}

function syncBatchButtons(): void {
  const hasSummaryPath = !!normalizePath(LAST_BATCH_SUMMARY?.path);
  const disabledReason = batchRunDisabledReason();
  patchAnalysisStore({
    canRunBatch: !disabledReason,
    canBatchStop: BATCH_RUNNING,
    canBatchRefresh: !BATCH_RUNNING,
    canBatchSelectAll: !BATCH_RUNNING,
    canBatchSelectNone: !BATCH_RUNNING,
    canBatchRetryFailed: !BATCH_RUNNING && batchFailedCount() > 0,
    canBatchResumeIncomplete:
      !BATCH_RUNNING &&
      !!LAST_BATCH_SUMMARY?.summary &&
      batchCanResumeIncomplete(LAST_BATCH_SUMMARY.summary),
    canBatchRevealSummary: !BATCH_RUNNING && hasSummaryPath,
  });
}

function updateBatchMeta(extra = ""): void {
  const total = Array.isArray(BATCH_DATASETS_CACHE)
    ? BATCH_DATASETS_CACHE.length
    : 0;
  const selected = readBatchSelectedPaths().length;
  const parts = [`selected ${selected}/${total}`];
  const executionMode = normalizeBatchExecutionMode(
    readStoreTextInputValue("batchExecutionMode"),
  );
  parts.push(`mode=${executionMode}`);
  const note = normalizeText(extra);
  if (note) parts.push(note);
  const disabledReason = batchRunDisabledReason();
  if (disabledReason && !BATCH_RUNNING) parts.push(`blocked=${disabledReason}`);
  const summaryPath = normalizePath(LAST_BATCH_SUMMARY?.path);
  if (summaryPath) parts.push(`summary=${fileBasename(summaryPath)}`);
  patchAnalysisStore({ batchMeta: parts.join(" • ") });
}

function filterBatchDatasets(
  items: BatchDatasetEntry[],
  query: string,
): BatchDatasetEntry[] {
  const q = normalizeText(query).toLowerCase();
  if (!q) return items;
  const terms = q.split(/\s+/g).filter((term) => term.length > 0);
  if (terms.length === 0) return items;
  return items.filter((item) => {
    const haystack =
      `${item.market} ${item.label} ${item.path} tf=${item.timeframe}`.toLowerCase();
    return terms.every((term) => haystack.includes(term));
  });
}

function renderBatchDatasetList(extra = ""): void {
  const store = getAnalysisStore();
  if (!store) return;
  const filtered = filterBatchDatasets(
    BATCH_DATASETS_CACHE,
    normalizeText(store.batchDatasetSearch),
  );

  const maxRender = filtered.length > 1000 ? 500 : filtered.length;
  const visible = filtered.slice(0, maxRender);
  const selected = new Set(readBatchSelectedPaths());
  const rows = visible.map((entry) => ({
    path: entry.path,
    label: entry.label,
    meta: entry.meta,
    checked: selected.has(entry.path),
    disabled: BATCH_RUNNING,
  }));
  patchAnalysisStore({
    batchDatasets: markRaw(rows),
    batchSelectedCount: selected.size,
    batchTotalCount: BATCH_DATASETS_CACHE.length,
  });
  updateBatchMeta(extra);
  syncBatchButtons();
}

function normalizeBatchDatasetEntry(
  value: unknown,
  fallback: Partial<BatchDatasetEntry> = {},
): BatchDatasetEntry | null {
  const row = asRecord(value);
  if (!row) return null;
  const path = normalizePath(row.csv_path ?? row.path ?? fallback.path);
  if (!path) return null;
  const market = normalizeText(row.market ?? fallback.market).toLowerCase();
  const adjusted =
    market === "stocks"
      ? normalizeBoolean(row.adjusted ?? fallback.adjusted)
      : null;
  const timeframe = normalizeText(row.timeframe ?? fallback.timeframe);
  const label = normalizeText(
    (row.display_symbol ??
      row.polygon_ticker ??
      row.label ??
      fallback.label ??
      fileBasename(path)) ||
      path,
  );
  const source =
    normalizeText(row.source ?? fallback.source) ||
    (market === "local" ? "repo_data" : "polygon");
  const sourceSize = normalizeMaybeInteger(
    row.source_size_bytes ?? fallback.source_size_bytes,
  );
  const sourceMtime = normalizeMaybeInteger(
    row.source_mtime_utc_ms ?? fallback.source_mtime_utc_ms,
  );
  const sourceFp = normalizeText(
    row.source_content_fingerprint ?? fallback.source_content_fingerprint,
  );
  const marketLabel = normalizeMarketLabel(market, adjusted === true);
  const minTs = formatYmdUtc(row.min_ts);
  const maxTs = formatYmdUtc(row.max_ts);
  const updated = formatYmdUtc(row.last_updated_utc_ms);
  const rowsHint = normalizeRowsHint(row.row_count);

  return {
    path,
    label: label || path,
    meta: `${marketLabel} • tf=${timeframe || "?"} • ${minTs} → ${maxTs} • rows=${rowsHint} • updated=${updated}`,
    market,
    timeframe,
    adjusted,
    source,
    source_size_bytes: sourceSize,
    source_mtime_utc_ms: sourceMtime,
    source_content_fingerprint: sourceFp,
  };
}

async function refreshBatchDatasetsInternal(): Promise<void> {
  const byPath = new Map<string, BatchDatasetEntry>();
  let polygonError = "";
  let localError = "";
  let localDir = "";
  let localTruncated = false;

  try {
    const raw = (await invokeCommand("polygon_get_cache_info")) as unknown;
    const record = asRecord(raw);
    const datasets = Array.isArray(record?.datasets)
      ? (record?.datasets as unknown[])
      : [];
    for (const entry of datasets) {
      const row = normalizeBatchDatasetEntry(entry, { source: "polygon" });
      if (!row) continue;
      byPath.set(row.path, row);
    }
  } catch (error) {
    polygonError = normalizeText(error);
  }

  try {
    const raw = (await invokeCommand("list_repo_data_csvs", {
      maxFiles: 20000,
      recursive: true,
      includeIdentity: true,
    })) as unknown;
    const record = asRecord(raw);
    localDir = normalizeText(record?.dir);
    localTruncated = normalizeBoolean(record?.truncated);
    const entries = Array.isArray(record?.csv_entries)
      ? (record?.csv_entries as unknown[])
      : Array.isArray(record?.csv_paths)
        ? (record?.csv_paths as unknown[])
        : [];
    for (const entry of entries) {
      const row = normalizeBatchDatasetEntry(entry, {
        market: "local",
        source: "repo_data",
      });
      if (!row) continue;
      if (byPath.has(row.path)) continue;
      byPath.set(row.path, row);
    }
  } catch (error) {
    localError = normalizeText(error);
  }

  BATCH_DATASETS_CACHE = Array.from(byPath.values());
  BATCH_DATASETS_CACHE.sort((left, right) =>
    `${left.market}${left.label}`.localeCompare(
      `${right.market}${right.label}`,
    ),
  );
  const validPaths = new Set(BATCH_DATASETS_CACHE.map((entry) => entry.path));
  setBatchSelectedPaths(
    readBatchSelectedPaths().filter((path) => validPaths.has(path)),
  );

  const extras: string[] = [];
  if (localDir) extras.push(`local=${fileBasename(localDir) || "data"}`);
  if (localTruncated) extras.push("local truncated");
  if (polygonError) extras.push("polygon error");
  if (localError) extras.push("local data error");
  renderBatchDatasetList(extras.join(" • "));
}

async function ensureBatchDatasetsLoaded(): Promise<void> {
  if (Array.isArray(BATCH_DATASETS_CACHE) && BATCH_DATASETS_CACHE.length > 0)
    return;
  await refreshBatchDatasetsInternal();
}

function setBatchSummaryState(
  summary: Record<string, unknown> | null,
  path: string,
): void {
  const summaryPath = normalizePath(path);
  const rawPayload = asRecord(summary);
  const existingCompletions = Array.isArray(rawPayload?.stage_completions)
    ? rawPayload?.stage_completions
    : [];
  const payload = rawPayload
    ? {
        ...rawPayload,
        stage_completions: existingCompletions.length
          ? existingCompletions
          : serializeStageCompletionsForArtifact([
              buildCampaignStageCompletion({
                ts: rawPayload.ts,
                artifactPath: summaryPath,
                res: { ...rawPayload, summary_path: summaryPath },
                backfilled: true,
              }),
            ]),
      }
    : null;
  LAST_BATCH_SUMMARY = payload ? { path: summaryPath, summary: payload } : null;

  patchAnalysisStore({
    batchOutputPreview: payload
      ? pretty({ path: summaryPath || null, summary: payload })
      : "-",
  });
  if (summaryPath) writeStorageString(LAST_BATCH_SUMMARY_KEY, summaryPath);
  else removeStorageKey(LAST_BATCH_SUMMARY_KEY);
  updateBatchMeta("");
  syncBatchButtons();
}

async function loadLastBatchSummaryFromDisk(): Promise<void> {
  if (LAST_BATCH_SUMMARY) return;
  const path = normalizePath(readStorageString(LAST_BATCH_SUMMARY_KEY, ""));
  if (!path) return;
  try {
    const artifact = (await invokeCommand("load_artifact", {
      path,
    })) as unknown;
    const row = asRecord(artifact);
    if (!row || !BATCH_SUMMARY_ARTIFACT_KINDS.has(normalizeText(row.kind)))
      return;
    setBatchSummaryState(row, path);
  } catch {

  }
}

function buildValidationFingerprintReq(
  runReq: Record<string, unknown>,
): Record<string, unknown> {
  const validation = readPiniaStoreRecord("validation");
  const strategyKind = normalizeStrategyKind(
    runReq.strategy_kind ?? runReq.strategyKind,
  );
  const candidates = Math.max(
    1,
    Math.floor(Number(validation?.candidates || 32)),
  );
  const seed = Math.max(0, Math.floor(Number(validation?.seed || 1)));
  const admissionPolicy = normalizeValidationAdmissionPolicy(
    validation?.admissionPolicy,
  );
  const minPsr = normalizeProbability(validation?.minPsr);
  const minDsr = normalizeProbability(validation?.minDsr);

  const wfEnable = normalizeBoolean(validation?.wfEnable);
  const wfWindow = Math.max(
    64,
    Math.floor(Number(validation?.wfWindow || 20000)),
  );
  const wfStep = Math.max(1, Math.floor(Number(validation?.wfStep || 20000)));

  const perturbEnable = normalizeBoolean(validation?.perturbEnable);
  const perturbIters = Math.max(
    0,
    Math.floor(Number(validation?.perturbIters || 8)),
  );
  const perturbMin = Math.max(0, Number(validation?.perturbMin || 0.015));
  const perturbMax = Math.max(0, Number(validation?.perturbMax || 0.06));

  const mcEnable = normalizeBoolean(validation?.mcEnable);
  const mcSims = Math.max(0, Math.floor(Number(validation?.mcSims || 500)));
  const mcBlockLen = Math.max(
    0,
    Math.floor(Number(validation?.mcBlockLen || 0)),
  );
  const mcMode =
    normalizeText(validation?.mcMode).toLowerCase() === "iid_only"
      ? "iid_only"
      : "compare";

  const cscvEnable = normalizeBoolean(validation?.cscvEnable);
  let cscvSlices = Math.max(
    4,
    Math.floor(Number(validation?.cscvSlices || 16)),
  );
  if ((cscvSlices & 1) === 1) cscvSlices = Math.max(4, cscvSlices - 1);

  const crossEnable = normalizeBoolean(validation?.crossEnable);
  const crossMax = Math.max(0, Math.floor(Number(validation?.crossMax || 6)));
  const crossModeRaw = normalizeText(validation?.crossMode).toLowerCase();
  const crossMode =
    crossModeRaw === "lightweight" ||
    crossModeRaw === "preserved_finalists" ||
    crossModeRaw === "full_stack"
      ? crossModeRaw
      : [
            "DoubleMa",
            "Pit",
            "Set",
            "Mts",
            "Orc",
            "Marrt",
            "Vfct",
            "CompositeTrend",
            "RegimeHybrid",
            "BreakoutMomentum",
          ].includes(strategyKind)
        ? "preserved_finalists"
        : "lightweight";
  const crossRows = Array.isArray(validation?.crossDatasets)
    ? (validation?.crossDatasets as unknown[])
    : [];
  const crossDatasets = crossRows
    .map((entry) => asRecord(entry))
    .filter((entry) => !!entry && normalizeBoolean(entry.checked))
    .map((entry) => ({
      csv_path: normalizePath(entry?.csv_path ?? entry?.path),
      label: normalizeText(entry?.label),
    }))
    .filter((entry) => entry.csv_path.length > 0)
    .slice(0, crossMax);

  return {
    data_id:
      normalizeText(runReq.data_id || runReq.dataId || "__cfg__") || "__cfg__",
    ma_source:
      normalizeText(runReq.ma_source || runReq.maSource || "close") || "close",
    candidates: [],
    walk_forward: wfEnable
      ? { window_bars: wfWindow, step_bars: wfStep }
      : null,
    perturb: perturbEnable
      ? {
          iterations: perturbIters,
          pct_min: perturbMin,
          pct_max: perturbMax,
          flip_bool_prob: 0.02,
        }
      : null,
    trade_bootstrap: mcEnable
      ? mcMode === "iid_only"
        ? { simulations: mcSims, stationary_mean_block_len: 0 }
        : mcBlockLen > 0
          ? { simulations: mcSims, stationary_mean_block_len: mcBlockLen }
          : { simulations: mcSims }
      : null,
    cross_ticker:
      crossEnable && crossMax > 0 && crossDatasets.length > 0
        ? { max_datasets: crossMax, mode: crossMode, datasets: crossDatasets }
        : null,
    cscv_pbo: cscvEnable
      ? {
          slices: cscvSlices,
          max_combinations: 50000,
          threshold: 0,
        }
      : null,
    seed,
    max_candidates: candidates,
    admission_policy: admissionPolicy,
    min_psr: minPsr,
    min_dsr: minDsr,
  };
}

function buildValidationCampaignConfig(
  runReq: Record<string, unknown>,
): Record<string, unknown> {
  const validation = readPiniaStoreRecord("validation");
  const base = buildValidationFingerprintReq(runReq);
  const reoptWfEnable = normalizeBoolean(validation?.reoptWfEnable);
  const reoptWfWindow = Math.max(
    64,
    Math.floor(Number(validation?.reoptWfWindow || 20000)),
  );
  const reoptWfStep = Math.max(
    1,
    Math.floor(Number(validation?.reoptWfStep || 20000)),
  );
  const reoptWfTopN = Math.max(
    1,
    Math.floor(Number(validation?.reoptWfTopN || 8)),
  );
  const spaEnable = normalizeBoolean(validation?.spaEnable);
  const spaSlices = Math.max(
    4,
    Math.floor(Number(validation?.spaSlices || 16)),
  );
  const spaBootstrapSamples = Math.max(
    100,
    Math.floor(Number(validation?.spaBootstrapSamples || 500)),
  );
  const spaBlockLength = Math.max(
    1,
    Math.floor(Number(validation?.spaBlockLength || 4)),
  );
  const minCscvOosRankFrac = normalizeProbability(validation?.minCscvRank);
  const minCscvOosMetricRaw = Number(validation?.minCscvMetric);
  const minCscvOosMetric = Number.isFinite(minCscvOosMetricRaw)
    ? minCscvOosMetricRaw
    : null;
  const maxPbo = normalizeProbability(validation?.maxPbo);
  return {
    ...base,
    reopt_walk_forward: reoptWfEnable
      ? {
          window_bars: reoptWfWindow,
          step_bars: reoptWfStep,
          top_n_candidates: reoptWfTopN,
        }
      : null,
    spa: spaEnable
      ? {
          slices: spaSlices,
          bootstrap_samples: spaBootstrapSamples,
          block_length: spaBlockLength,
          benchmark: "buy_hold",
        }
      : null,
    min_cscv_oos_rank_frac: minCscvOosRankFrac,
    min_cscv_oos_metric: minCscvOosMetric,
    max_pbo: maxPbo,
  };
}

async function refreshSavedRunCount(): Promise<void> {
  try {
    const rows = (await invokeCommand("list_run_artifacts")) as unknown;
    patchAnalysisStore({
      aggregateSummarySavedRuns: String(Array.isArray(rows) ? rows.length : 0),
    });
  } catch {

  }
}

export function refreshBatchDatasetView(): void {
  renderBatchDatasetList("");
}

export async function runBatch(
  options: MetaBatchRunOptions = {},
): Promise<void> {
  let offCampaignProgress: (() => void) | null = null;
  try {
    if (BATCH_RUNNING) return;

    let req = normalizeRequestPayload(options.request);
    if (!req) {
      try {
        req = tryBuildAutoMetaBatchRequest();
      } catch {
        req = null;
      }
    }

    const optimizeStore = getOptimizeStore();
    const strategyKind = normalizeStrategyKind(
      req?.strategy_kind ?? optimizeStore?.strategyKind,
    );

    if (strategyKind === "MlSignal") {
      setStatusLine(
        "Research campaigns do not support MlSignal. Use the dedicated ML Signal workflow instead.",
      );
      return;
    }
    if (strategyKind === "DynamicDoubleMa") {
      setStatusLine(
        "Research campaigns do not support DynamicDoubleMa yet because validation does not preserve its dynamic strategy IR.",
      );
      return;
    }
    if (!isSupportedResearchCampaignStrategy(strategyKind)) {
      setStatusLine(
        `Research campaigns do not support strategy '${strategyKind}' yet.`,
      );
      return;
    }

    await ensureBatchDatasetsLoaded();

    const selectedPaths = Array.isArray(req?.selected_paths)
      ? (req!.selected_paths as unknown[])
      : Array.isArray(req?.datasets)
        ? (req!.datasets as unknown[])
            .map((entry) => normalizePath(asRecord(entry)?.csv_path))
            .filter((entry) => entry.length > 0)
        : readBatchSelectedPaths();
    const normalizedSelected = selectedPaths
      .map((entry) => normalizePath(entry))
      .filter((entry) => entry.length > 0);
    setBatchSelectedPaths(normalizedSelected);
    renderBatchDatasetList("");

    if (normalizedSelected.length === 0) {
      setStatusLine("Select at least 1 dataset in Meta → Batch runs first.");
      return;
    }

    const holdoutPctRaw = Math.floor(
      Number(
        (req?.holdout_pct ?? readStoreTextInputValue("batchHoldoutPct")) || 0,
      ),
    );
    const holdoutPct = Number.isFinite(holdoutPctRaw)
      ? Math.max(0, Math.min(50, holdoutPctRaw))
      : 0;
    const holdoutTag =
      normalizeText(
        req?.holdout_tag ?? readStoreTextInputValue("batchHoldoutTag"),
      ) || "holdout";
    const executionMode = normalizeBatchExecutionMode(
      req?.execution_mode ?? readStoreTextInputValue("batchExecutionMode"),
    );
    const preloadPolicy = normalizeBatchPreloadPolicy(
      req?.preload_policy ?? readStoreTextInputValue("batchPreloadPolicy"),
    );
    const autoMeta =
      executionMode !== "optimize_only" &&
      normalizeBoolean(
        req?.auto_meta ?? readStoreCheckboxValue("batchAutoMeta"),
      );
    const autoStage5 =
      autoMeta &&
      normalizeBoolean(
        req?.auto_stage5 ?? readStoreCheckboxValue("batchAutoStage5"),
      );
    const skipDone = normalizeBoolean(
      req?.skip_done ??
        req?.skip_existing ??
        readStoreCheckboxValue("batchSkipDone"),
    );
    let baseTags = dedupTags(
      Array.isArray(req?.tags)
        ? req?.tags
        : parseTagsInput(readStoreTextInputValue("batchTags")),
    );
    if (baseTags.length === 0) baseTags = [defaultBatchTag()];
    const primaryTag = normalizeText(baseTags[0]);

    const analysisStore = getAnalysisStore();
    if (analysisStore) {
      analysisStore.batchTags = baseTags.join(", ");
      analysisStore.batchHoldoutPct = String(holdoutPct);
      analysisStore.batchHoldoutTag = holdoutTag;
      analysisStore.batchExecutionMode = executionMode;
      analysisStore.batchPreloadPolicy = preloadPolicy;
      analysisStore.batchAutoMeta = autoMeta;
      analysisStore.batchAutoStage5 = autoStage5;
      analysisStore.batchSkipDone = skipDone;
      if (!normalizeText(analysisStore.metaRequiredTag))
        analysisStore.metaRequiredTag = primaryTag;
      analysisStore.updatedAtUnixMs = Date.now();
    }

    const byPath = new Map<string, BatchDatasetEntry>();
    for (const entry of BATCH_DATASETS_CACHE) {
      byPath.set(entry.path, entry);
    }

    const dataStore = getDataStore();
    const tfOverride = normalizeText(
      req?.timeframe_override ?? dataStore?.dataTimeframe,
    );
    const hasTfOverride = tfOverride.length > 0;
    const selectedTimeframes = new Set<string>();
    for (const selectedPath of normalizedSelected) {
      const entry = byPath.get(selectedPath);
      const tf = normalizeText(entry?.timeframe);
      if (tf) selectedTimeframes.add(tf);
    }

    if (!hasTfOverride && selectedTimeframes.size > 1) {
      const found = Array.from(selectedTimeframes).sort().join(", ");
      setStatusLine(
        `Batch requires a single timeframe across selected datasets (found: ${found}). Filter by timeframe or set Data → Timeframe to force resampling.`,
      );
      return;
    }

    let expectedTimeframe = hasTfOverride
      ? tfOverride
      : selectedTimeframes.size === 1
        ? Array.from(selectedTimeframes)[0]
        : "";

    BATCH_RUNNING = true;
    syncBatchButtons();
    setBatchSummaryState(null, "");
    const n = normalizedSelected.length;
    const split = splitHoldoutPaths(
      normalizedSelected,
      holdoutPct,
      primaryTag || "0",
    );
    const holdoutSet = new Set(
      Array.isArray(split.holdout) ? split.holdout : [],
    );
    const batchTimeframes = new Set<string>();
    if (expectedTimeframe) batchTimeframes.add(expectedTimeframe);
    updateBatchMeta(`running campaign 0/${n}`);
    setStatusLine(`Running campaign across ${n} dataset${n === 1 ? "" : "s"}…`);
    try {
      offCampaignProgress = await listenEvent(
        "research_campaign_progress",
        (payload) => {
          const row = asRecord(payload);
          const summaryPath = normalizePath(
            row?.summaryJsonPath ?? row?.summary_json_path,
          );
          if (summaryPath)
            writeStorageString(LAST_BATCH_SUMMARY_KEY, summaryPath);
        },
      );
    } catch {
      offCampaignProgress = null;
    }

    const requestDatasetsByPath = new Map<string, Record<string, unknown>>();
    if (Array.isArray(req?.datasets)) {
      for (const raw of req.datasets as unknown[]) {
        const row = asRecord(raw);
        const path = normalizePath(row?.csv_path);
        if (row && path) requestDatasetsByPath.set(path, row);
      }
    }

    const datasets = normalizedSelected
      .map((path) => {
        const entry = byPath.get(path);
        const requestDataset = requestDatasetsByPath.get(path);
        return {
          csv_path: path,
          label:
            normalizeText(
              requestDataset?.label ??
                entry?.label ??
                fileBasename(path) ??
                path,
            ) || null,
          timeframe:
            normalizeText(requestDataset?.timeframe ?? entry?.timeframe) ||
            null,
          source_size_bytes: normalizeMaybeInteger(
            entry?.source_size_bytes ?? requestDataset?.source_size_bytes,
          ),
          source_mtime_utc_ms: normalizeMaybeInteger(
            entry?.source_mtime_utc_ms ?? requestDataset?.source_mtime_utc_ms,
          ),
          source_content_fingerprint:
            normalizeText(entry?.source_content_fingerprint) ||
            normalizeText(requestDataset?.source_content_fingerprint) ||
            null,
        };
      })
      .filter((entry) => normalizePath(entry.csv_path).length > 0);

    const optimizationTemplate =
      asRecord(req?.optimization_template) ??
      (await buildOptimizationRequestFromVueStores());
    const validationConfig =
      asRecord(req?.validation_config) ??
      buildValidationCampaignConfig(optimizationTemplate);

    if (analysisStore) {
      analysisStore.metaStrategyKind = strategyKind;
      if (expectedTimeframe && !normalizeText(analysisStore.metaTimeframe)) {
        analysisStore.metaTimeframe = expectedTimeframe;
      }
      analysisStore.metaRequiredTag =
        primaryTag || analysisStore.metaRequiredTag;
      analysisStore.metaHoldoutTag = holdoutTag;
      analysisStore.metaEvalHoldout = holdoutSet.size > 0;
    }

    let metaRequest: Record<string, unknown> | null = asRecord(
      req?.meta_request,
    );
    let stage5Request: Record<string, unknown> | null = asRecord(
      req?.stage5_request,
    );
    if (autoMeta && !metaRequest) {
      try {
        metaRequest = tryBuildAutoMetaRequest();
      } catch {
        metaRequest = null;
      }
    }
    if (autoMeta && autoStage5 && !stage5Request) {
      try {
        stage5Request = tryBuildAutoMetaStage4Request();
      } catch {
        stage5Request = null;
      }
    }

    const campaignReq = {
      strategy_kind: strategyKind,
      datasets,
      timeframe_override: expectedTimeframe || null,
      tags: baseTags,
      holdout_pct: holdoutPct,
      holdout_tag: holdoutTag || null,
      skip_existing: skipDone,
      execution_mode: executionMode,
      auto_meta: autoMeta,
      auto_stage5: autoStage5,
      optimization_template: optimizationTemplate,
      validation_config: validationConfig,
      meta_request: metaRequest,
      stage5_request: stage5Request,
      preload_policy: preloadPolicy,
    };

    const response = asRecord(
      (await invokeCommand("run_research_campaign", {
        req: campaignReq,
      })) as unknown,
    );
    const summary = asRecord(response?.summary);
    const summaryPath = normalizePath(response?.summary_json_path);
    if (!summary) {
      setStatusLine("Campaign completed, but no summary payload was returned.");
      return;
    }

    const counts = asRecord(summary.counts);
    const ok = Math.max(0, Math.floor(Number(counts?.validated || 0)));
    const skipped = Math.max(0, Math.floor(Number(counts?.skipped || 0)));
    const failed = Math.max(0, Math.floor(Number(counts?.failed || 0)));
    const cancelled = normalizeBoolean(counts?.cancelled);
    const msg = cancelled
      ? `Campaign cancelled: validated=${ok}/${n}${skipped ? ` • skipped=${skipped}` : ""}${failed ? ` • failed=${failed}` : ""}`
      : `Campaign done: validated=${ok}/${n}${skipped ? ` • skipped=${skipped}` : ""}${failed ? ` • failed=${failed}` : ""}`;

    setBatchSummaryState(summary, summaryPath);
    await refreshSavedRunCount();
    setStatusLine(msg);
    updateBatchMeta(msg);

    const metaResponse = asRecord(response?.meta);
    if (metaResponse) {
      renderMetaSelection(metaResponse);
      SAVED_META = upsertSavedEntry(
        SAVED_META,
        buildMetaSavedEntryFromResponse(metaResponse),
      );
      updateMetaSavedState(normalizePath(metaResponse.meta_json_path));
      scheduleDeferredTask(() =>
        refreshSavedInternal(normalizePath(metaResponse.meta_json_path)),
      );
      selectMetaResultsTab();
    }

    const stage5Response = asRecord(response?.stage5);
    if (stage5Response) {
      renderMetaStage4(stage5Response);
      SAVED_STAGE4 = upsertSavedEntry(
        SAVED_STAGE4,
        buildStage5SavedEntryFromResponse(stage5Response),
      );
      updateStage4SavedState(responseStage5Path(stage5Response));
      scheduleDeferredTask(() =>
        refreshStage4SavedInternal(responseStage5Path(stage5Response)),
      );
      selectMetaResultsTab();
    }
  } catch (error) {
    if (isCancellationError(error)) {
      setStatusLine("Campaign cancelled.");
      updateBatchMeta("campaign cancelled");
    } else {
      setErrorStatus(error);
    }
  } finally {
    if (offCampaignProgress) offCampaignProgress();
    BATCH_RUNNING = false;
    syncBatchButtons();
    renderBatchDatasetList("");
  }
}

export async function batchStop(): Promise<void> {
  try {
    if (!BATCH_RUNNING) return;
    await invokeCommand("cancel_current_run");
    updateBatchMeta("cancelling campaign");
    setStatusLine("Campaign cancellation requested.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function batchRefresh(): Promise<void> {
  try {
    if (BATCH_RUNNING) return;
    await refreshBatchDatasetsInternal();
    await loadLastBatchSummaryFromDisk();
    setStatusLine("Refreshed batch dataset list.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function batchSelectAll(): Promise<void> {
  try {
    if (BATCH_RUNNING) return;
    await ensureBatchDatasetsLoaded();
    const query = normalizeText(getAnalysisStore()?.batchDatasetSearch);
    const filtered = filterBatchDatasets(BATCH_DATASETS_CACHE, query);
    const selected = new Set(readBatchSelectedPaths());
    for (const entry of filtered) {
      selected.add(entry.path);
    }
    setBatchSelectedPaths(Array.from(selected));
    renderBatchDatasetList("");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function batchSelectNone(): Promise<void> {
  try {
    if (BATCH_RUNNING) return;
    setBatchSelectedPaths([]);
    renderBatchDatasetList("");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function batchRetryFailed(): Promise<void> {
  try {
    if (BATCH_RUNNING) return;
    await loadLastBatchSummaryFromDisk();
    const summary = LAST_BATCH_SUMMARY?.summary;
    if (!summary) {
      setStatusLine("No batch summary loaded.");
      return;
    }
    if (batchFailedCount() <= 0) {
      setStatusLine("No failures to retry.");
      return;
    }

    const tags = Array.isArray(summary.tags) ? dedupTags(summary.tags) : [];
    const request = batchSummaryRequest(summary);
    const selectedPaths = batchSummarySelectedPaths(summary);
    const retryPaths =
      selectedPaths.length > 0
        ? selectedPaths
        : batchSummaryFailedPaths(summary);
    const analysisStore = getAnalysisStore();
    if (analysisStore) {
      analysisStore.batchTags = tags.join(", ");
      analysisStore.batchHoldoutPct = String(
        Math.max(0, Math.floor(Number(summary.holdout_pct || 0))),
      );
      analysisStore.batchHoldoutTag =
        normalizeText(summary.holdout_tag) || "holdout";
      analysisStore.batchSkipDone = true;
      analysisStore.updatedAtUnixMs = Date.now();
    }

    setBatchSelectedPaths(retryPaths);
    renderBatchDatasetList(
      `retrying failed (tag=${batchSummaryPrimaryTag(summary) || "batch"})`,
    );
    await runBatch({
      request: request
        ? {
            ...request,
            selected_paths: retryPaths,
            skip_existing: true,
          }
        : undefined,
    });
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function batchResumeIncomplete(): Promise<void> {
  try {
    if (BATCH_RUNNING) return;
    await loadLastBatchSummaryFromDisk();
    const summary = LAST_BATCH_SUMMARY?.summary;
    if (!summary) {
      setStatusLine("No batch summary loaded.");
      return;
    }
    const request = batchSummaryRequest(summary);
    if (!request) {
      setStatusLine(
        "Current summary does not contain a resumable campaign request.",
      );
      return;
    }
    const blockedReason = batchSummaryResumeBlockedReason(summary);
    if (blockedReason) {
      setStatusLine(blockedReason);
      return;
    }

    const tags = Array.isArray(summary.tags) ? dedupTags(summary.tags) : [];
    const selectedPaths = batchSummarySelectedPaths(summary);
    const analysisStore = getAnalysisStore();
    if (analysisStore) {
      analysisStore.batchTags = tags.join(", ");
      analysisStore.batchHoldoutPct = String(
        Math.max(0, Math.floor(Number(summary.holdout_pct || 0))),
      );
      analysisStore.batchHoldoutTag =
        normalizeText(summary.holdout_tag) || "holdout";
      analysisStore.batchSkipDone = true;
      analysisStore.updatedAtUnixMs = Date.now();
    }

    setBatchSelectedPaths(selectedPaths);
    renderBatchDatasetList(
      `resuming incomplete (tag=${batchSummaryPrimaryTag(summary) || "batch"})`,
    );
    await runBatch({
      request: {
        ...request,
        skip_existing: true,
      },
    });
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function batchRevealSummary(): Promise<void> {
  try {
    if (BATCH_RUNNING) return;
    await loadLastBatchSummaryFromDisk();
    const path = normalizePath(LAST_BATCH_SUMMARY?.path);
    if (!path) {
      setStatusLine("No batch summary saved yet.");
      return;
    }
    await invokeCommand("reveal_path", { path });
    setStatusLine("Revealed batch summary.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function saveSavedTags(): Promise<void> {
  try {
    const entry = selectedMetaSavedEntry();
    const path = normalizePath(entry?.path);
    if (!path) {
      setStatusLine("Select a saved meta run first.");
      return;
    }

    const store = getAnalysisStore();
    const tags = parseTagsInput(normalizeText(store?.metaSavedTagsDraft));
    await invokeCommand("update_artifact_tags", { path, tags });
    await refreshSavedInternal(path);
    setStatusLine("Saved meta tags.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function refreshSaved(): Promise<void> {
  try {
    const store = getAnalysisStore();
    await refreshSavedInternal(normalizePath(store?.metaSavedSelected));
    setStatusLine("Refreshed saved meta runs.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function loadSaved(): Promise<void> {
  try {
    const entry = selectedMetaSavedEntry();
    const path = normalizePath(entry?.path);
    if (!path) {
      setStatusLine("Select a saved meta run first.");
      return;
    }

    const responseRaw = (await invokeCommand("load_meta_artifact_replay", {
      path,
    })) as unknown;
    const response = asRecord(responseRaw);
    if (!response) {
      setStatusLine("Saved meta artifact payload was empty.");
      return;
    }
    renderMetaSelection(response);
    await refreshSavedInternal(path);
    selectMetaResultsTab();
    setStatusLine(`Loaded meta run ${normalizeText(response.ts)}`);
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function openSavedReport(): Promise<void> {
  try {
    const entry = selectedMetaSavedEntry();
    const reportPath = normalizePath(entry?.report_xlsx_path);
    if (
      entry &&
      !canOpenPersistedReport(
        entry.report_xlsx_path,
        entry.report_xlsx_exists ?? entry.reportXlsxExists,
      )
    ) {
      setStatusLine("Saved meta report file no longer exists.");
      return;
    }
    if (!reportPath) {
      setStatusLine("No report path for this meta run.");
      return;
    }

    const opened = await openPathWithRevealFallback(reportPath);
    setStatusLine(
      opened.openedWith === "reveal_path"
        ? "Revealed report."
        : "Opened report.",
    );
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function revealSaved(): Promise<void> {
  try {
    const entry = selectedMetaSavedEntry();
    const path = normalizePath(entry?.path);
    if (!path) {
      setStatusLine("Select a saved meta run first.");
      return;
    }
    await invokeCommand("reveal_path", { path });
    setStatusLine("Revealed meta run.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function deleteSaved(): Promise<void> {
  try {
    const entry = selectedMetaSavedEntry();
    const path = normalizePath(entry?.path);
    if (!path) {
      setStatusLine("Select a saved meta run first.");
      return;
    }

    await invokeCommand("delete_artifact", { path });
    if (normalizePath(LAST_META_RUN?.meta_json_path) === path) {
      LAST_META_RUN = null;
      renderMetaSelection(null);
    }
    await refreshSavedInternal("");
    setStatusLine("Deleted meta run.");
  } catch (error) {
    setErrorStatus(error);
  }
}


export async function saveStage5SavedTags(): Promise<void> {
  try {
    const entry = selectedStage4SavedEntry();
    const path = normalizePath(entry?.path);
    if (!path) {
      setStatusLine("Select a saved Stage 5 run first.");
      return;
    }

    const store = getAnalysisStore();
    const tags = parseTagsInput(normalizeText(store?.stage4SavedTagsDraft));
    await invokeCommand("update_artifact_tags", { path, tags });
    await refreshStage4SavedInternal(path);
    setStatusLine("Saved Stage 5 tags.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function saveStage4SavedTags(): Promise<void> {
  await saveStage5SavedTags();
}

export async function refreshStage5Saved(): Promise<void> {
  try {
    const store = getAnalysisStore();
    await refreshStage4SavedInternal(normalizePath(store?.stage4SavedSelected));
    setStatusLine("Refreshed saved Stage 5 runs.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function refreshStage4Saved(): Promise<void> {
  await refreshStage5Saved();
}

export async function loadStage5Saved(): Promise<void> {
  try {
    const entry = selectedStage4SavedEntry();
    const path = normalizePath(entry?.path);
    if (!path) {
      setStatusLine("Select a saved Stage 5 run first.");
      return;
    }

    const responseRaw = (await invokeCommand("load_stage5_artifact_replay", {
      path,
    })) as unknown;
    const response = asRecord(responseRaw);
    if (!response) {
      setStatusLine("Saved Stage 5 artifact payload was empty.");
      return;
    }
    renderMetaStage4(response);
    await refreshStage4SavedInternal(path);
    selectMetaResultsTab();
    setStatusLine(`Loaded Stage 5 run ${normalizeText(response.ts)}`);
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function loadStage4Saved(): Promise<void> {
  await loadStage5Saved();
}

export async function revealStage5Saved(): Promise<void> {
  try {
    const entry = selectedStage4SavedEntry();
    const path = normalizePath(entry?.path);
    if (!path) {
      setStatusLine("Select a saved Stage 5 run first.");
      return;
    }
    await invokeCommand("reveal_path", { path });
    setStatusLine("Revealed Stage 5 run.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function revealStage4Saved(): Promise<void> {
  await revealStage5Saved();
}

export async function deleteStage5Saved(): Promise<void> {
  try {
    const entry = selectedStage4SavedEntry();
    const path = normalizePath(entry?.path);
    if (!path) {
      setStatusLine("Select a saved Stage 5 run first.");
      return;
    }

    await invokeCommand("delete_artifact", { path });
    if (normalizePath(LAST_META_STAGE4_RUN?.stage4_json_path) === path) {
      LAST_META_STAGE4_RUN = null;
      renderMetaStage4(null);
    }
    await refreshStage4SavedInternal("");
    setStatusLine("Deleted Stage 5 run.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function deleteStage4Saved(): Promise<void> {
  await deleteStage5Saved();
}

export async function savePolicyEvalSavedTags(): Promise<void> {
  try {
    const entry = selectedPolicyEvalSavedEntry();
    const path = normalizePath(entry?.path);
    if (!path) {
      setStatusLine("Select a saved policy evaluation first.");
      return;
    }

    const store = getAnalysisStore();
    const tags = parseTagsInput(normalizeText(store?.policyEvalSavedTagsDraft));
    await invokeCommand("update_artifact_tags", { path, tags });
    await refreshPolicyEvalSavedInternal(path);
    setStatusLine("Saved policy evaluation tags.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function refreshPolicyEvalSaved(): Promise<void> {
  try {
    const store = getAnalysisStore();
    await refreshPolicyEvalSavedInternal(
      normalizePath(store?.policyEvalSavedSelected),
    );
    setStatusLine("Refreshed saved policy evaluations.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function loadPolicyEvalSaved(): Promise<void> {
  try {
    const entry = selectedPolicyEvalSavedEntry();
    const path = normalizePath(entry?.path);
    if (!path) {
      setStatusLine("Select a saved policy evaluation first.");
      return;
    }

    const responseRaw = (await invokeCommand(
      "load_ml_policy_eval_artifact_replay",
      {
        path,
      },
    )) as unknown;
    const response = asRecord(responseRaw);
    if (!response) {
      setStatusLine("Saved policy evaluation payload was empty.");
      return;
    }
    renderMlPolicyEvaluation(response);
    await refreshPolicyEvalSavedInternal(path);
    selectMetaResultsTab();
    setStatusLine(`Loaded policy evaluation ${normalizeText(response.ts)}`);
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function revealPolicyEvalSaved(): Promise<void> {
  try {
    const entry = selectedPolicyEvalSavedEntry();
    const path = normalizePath(entry?.path);
    if (!path) {
      setStatusLine("Select a saved policy evaluation first.");
      return;
    }
    await invokeCommand("reveal_path", { path });
    setStatusLine("Revealed policy evaluation.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function deletePolicyEvalSaved(): Promise<void> {
  try {
    const entry = selectedPolicyEvalSavedEntry();
    const path = normalizePath(entry?.path);
    if (!path) {
      setStatusLine("Select a saved policy evaluation first.");
      return;
    }

    await invokeCommand("delete_artifact", { path });
    if (normalizePath(LAST_META_ML_POLICY_EVAL?.eval_json_path) === path) {
      LAST_META_ML_POLICY_EVAL = null;
      renderMlPolicyEvaluation(null);
    }
    await refreshPolicyEvalSavedInternal("");
    setStatusLine("Deleted policy evaluation.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function mlRefresh(
  options: MetaMlActionOptions = {},
): Promise<void> {
  try {
    const scope = normalizeMlScope(options.scope);
    if (!normalizeText(scope.timeframe)) {
      LAST_META_ML_STATUS = null;
      patchAnalysisStore({
        metaMlStatus:
          "Timeframe required (set Meta timeframe filter or run meta once).",
      });
      updateMetaMlButtons();
      return;
    }

    const statusRaw = (await invokeCommand("ml_confidence_model_status", {
      scope,
    })) as unknown;
    applyMlStatusPayload(statusRaw, scope);
    setStatusLine("Refreshed ML confidence status.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function mlTrain(
  options: MetaMlActionOptions = {},
): Promise<void> {
  try {
    const scope = normalizeMlScope(options.scope);
    if (!normalizeText(scope.timeframe)) {
      setStatusLine("Timeframe required for ML model training.");
      return;
    }

    const responseRaw = (await invokeCommand("train_ml_confidence_model", {
      scope,
      cfg: null,
    })) as unknown;
    const response = asRecord(responseRaw);
    if (!response) {
      setStatusLine("ML model training returned an invalid response.");
      return;
    }
    applyMlStatusPayload(response.status, scope);
    const store = getAnalysisStore();
    if (
      store?.metaUseMlConfidence &&
      asRecordArray(asRecord(LAST_META_RUN?.result)?.groups).length > 0
    ) {
      await mlApply({ scope });
      return;
    }
    setStatusLine("ML model trained.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function mlApply(
  options: MetaMlActionOptions = {},
): Promise<void> {
  try {
    const scope = normalizeMlScope(options.scope);
    if (!normalizeText(scope.timeframe)) {
      setStatusLine("Timeframe required for ML apply.");
      return;
    }

    const run = asRecord(LAST_META_RUN);
    const result = asRecord(run?.result);
    const groups = asRecordArray(result?.groups);
    if (groups.length === 0) {
      setStatusLine("Run or load a meta analysis first.");
      return;
    }

    const raw = (await invokeCommand("apply_ml_confidence_model", {
      scope,
      groups: groups.map((group) => toMlFeatureGroup(group)),
    })) as unknown;
    const payload = asRecord(raw) ?? {};
    const confidences = Array.isArray(payload.confidences)
      ? payload.confidences
      : [];

    for (let i = 0; i < groups.length; i += 1) {
      const group = groups[i];
      if (!group) continue;
      const conf = Number(confidences[i]);
      group.ml_confidence = Number.isFinite(conf) ? conf : null;
    }

    const modelHealthy = LAST_META_ML_STATUS?.model_healthy;
    const useMlRank = !!getAnalysisStore()?.metaUseMlConfidence;
    if (useMlRank && modelHealthy !== false) {
      groups.sort((a, b) => {
        const lhs = Number(a.ml_confidence);
        const rhs = Number(b.ml_confidence);
        const left = Number.isFinite(lhs) ? lhs : -1;
        const right = Number.isFinite(rhs) ? rhs : -1;
        return right - left;
      });
    }

    if (result) {
      result.groups = groups;
      (LAST_META_RUN as Record<string, unknown>).result = result;
    }

    renderMetaSelection(LAST_META_RUN);

    const warnings = Array.isArray(payload.warnings)
      ? payload.warnings
          .map((warning) => normalizeText(warning))
          .filter((warning) => warning.length > 0)
      : [];
    if (warnings.length > 0) {
      setStatusLine(`Applied ML confidence (warnings: ${warnings.length}).`);
    } else {
      setStatusLine("Applied ML confidence.");
    }
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function mlDelete(
  options: MetaMlActionOptions = {},
): Promise<void> {
  try {
    const scope = normalizeMlScope(options.scope);
    if (!normalizeText(scope.timeframe)) {
      setStatusLine("Timeframe required for ML delete.");
      return;
    }

    await invokeCommand("delete_ml_confidence_model", { scope });
    LAST_META_ML_STATUS = null;
    patchAnalysisStore({ metaMlStatus: "Deleted model." });
    updateMetaMlButtons();
    setStatusLine("Deleted ML confidence model.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function runStage5(
  options: MetaStage5RunOptions = {},
): Promise<void> {
  try {
    let req = normalizeRequestPayload(options.request);
    if (!req) {
      try {
        req = tryBuildAutoMetaStage4Request();
      } catch {
        req = null;
      }
    }

    if (!req) {
      setStatusLine("Missing Stage 5 request payload.");
      return;
    }

    const payload = (await invokeCommand("run_meta_stage5_ml_confidence", {
      req,
    })) as unknown;
    const response = asRecord(payload);
    if (!response) {
      setStatusLine("Stage 5 returned an invalid response.");
      return;
    }

    renderMetaStage4(response);
    SAVED_STAGE4 = upsertSavedEntry(
      SAVED_STAGE4,
      buildStage5SavedEntryFromResponse(response),
    );
    updateStage4SavedState(responseStage5Path(response));
    scheduleDeferredTask(() =>
      refreshStage4SavedInternal(responseStage5Path(response)),
    );
    const result = asRecord(response.result);
    if (didStage5ModelStateChange(result)) {
      scheduleDeferredTask(() =>
        mlRefresh({
          scope: {
            strategy_kind: normalizeText(result?.strategy_kind),
            timeframe: normalizeText(result?.timeframe),
          },
        }),
      );
    }
    selectMetaResultsTab();
    setStatusLine(
      responseStage5Path(response)
        ? `Stage 5 complete. Saved: ${responseStage5Path(response)}`
        : "Stage 5 complete.",
    );
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function runStage4(
  options: MetaStage4RunOptions = {},
): Promise<void> {
  await runStage5(options);
}

export async function revealStage5(): Promise<void> {
  try {
    const path = responseStage5Path(LAST_META_STAGE4_RUN);
    if (!path) {
      setStatusLine("No Stage 5 artifact saved yet.");
      return;
    }
    await invokeCommand("reveal_path", { path });
    setStatusLine("Revealed Stage 5 artifact.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function revealStage4(): Promise<void> {
  await revealStage5();
}

export async function runMlPolicyEvaluation(
  options: MetaMlPolicyEvaluationRunOptions = {},
): Promise<void> {
  try {
    let req = normalizeRequestPayload(options.request);
    if (!req) {
      req = tryBuildAutoMetaMlPolicyEvaluationRequest();
    }

    if (!req) {
      setStatusLine("Missing ML policy evaluation request payload.");
      return;
    }

    const payload = (await invokeCommand("evaluate_ml_confidence_policy", {
      req,
    })) as unknown;
    const response = asRecord(payload);
    if (!response) {
      setStatusLine("ML policy evaluation returned an invalid response.");
      return;
    }

    renderMlPolicyEvaluation(response);
    SAVED_POLICY_EVAL = upsertSavedEntry(
      SAVED_POLICY_EVAL,
      buildPolicyEvalSavedEntryFromResponse(response),
    );
    updatePolicyEvalSavedState(normalizePath(response.eval_json_path));
    scheduleDeferredTask(() =>
      refreshPolicyEvalSavedInternal(normalizePath(response.eval_json_path)),
    );
    setStatusLine(
      normalizePath(response.eval_json_path)
        ? `ML policy evaluation complete. Saved: ${normalizePath(response.eval_json_path)}`
        : "ML policy evaluation complete.",
    );
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function runMeta(options: MetaRunOptions = {}): Promise<void> {
  try {
    let req = normalizeRequestPayload(options.request);
    if (!req) {
      try {
        req = tryBuildAutoMetaRequest();
      } catch {
        req = null;
      }
    }

    if (!req) {
      setStatusLine("Missing meta request payload.");
      return;
    }

    const payload = (await invokeCommand("run_meta_selection", {
      req,
    })) as unknown;
    const response = asRecord(payload);
    if (!response) {
      setStatusLine("Meta run returned an invalid response.");
      return;
    }

    renderMetaSelection(response);
    SAVED_META = upsertSavedEntry(
      SAVED_META,
      buildMetaSavedEntryFromResponse(response),
    );
    updateMetaSavedState(normalizePath(response.meta_json_path));
    scheduleDeferredTask(() =>
      refreshSavedInternal(normalizePath(response.meta_json_path)),
    );
    await maybeRunMetaMlAutopilot();
    if (options.switchTab !== false) {
      selectMetaResultsTab();
    }
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function openMetaReport(): Promise<void> {
  try {
    const reportPath = normalizePath(LAST_META_RUN?.report_xlsx_path);
    if (
      LAST_META_RUN &&
      !canOpenPersistedReport(
        LAST_META_RUN.report_xlsx_path,
        LAST_META_RUN.report_xlsx_exists ?? LAST_META_RUN.reportXlsxExists,
      )
    ) {
      setStatusLine("Meta report file no longer exists on disk.");
      return;
    }
    if (!reportPath) {
      setStatusLine("No meta report path yet. Run meta analysis first.");
      return;
    }

    const opened = await openPathWithRevealFallback(reportPath);
    setStatusLine(
      opened.openedWith === "reveal_path"
        ? "Revealed meta report."
        : "Opened meta report.",
    );
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function revealMetaFiles(): Promise<void> {
  try {
    const reportPath = normalizePath(LAST_META_RUN?.report_xlsx_path);
    const metaPath = normalizePath(LAST_META_RUN?.meta_json_path);
    const path = canOpenPersistedReport(
      LAST_META_RUN?.report_xlsx_path,
      LAST_META_RUN?.report_xlsx_exists ?? LAST_META_RUN?.reportXlsxExists,
    )
      ? reportPath
      : metaPath;
    if (!path) {
      setStatusLine("No meta files yet. Run meta analysis first.");
      return;
    }

    await invokeCommand("reveal_path", { path });
    setStatusLine("Revealed meta files.");
  } catch (error) {
    setErrorStatus(error);
  }
}

export async function toggleBatchDataset(
  path: string,
  checked: boolean,
): Promise<void> {
  try {
    const normalizedPath = normalizePath(path);
    if (!normalizedPath) return;
    const selected = new Set(readBatchSelectedPaths());
    if (checked) selected.add(normalizedPath);
    else selected.delete(normalizedPath);
    setBatchSelectedPaths(Array.from(selected));
    renderBatchDatasetList("");
  } catch (error) {
    setErrorStatus(error);
  }
}
