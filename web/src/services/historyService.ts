import { invokeCommand } from "../tauri/invoke";
import { HISTORY_KEY, SETTINGS_KEY } from "../utils/core/storageKeys";
import type {
  ArtifactLineageSummary,
  RouteProvenanceSummary,
} from "../utils/core/runArtifactReview";
import {
  parseArtifactLineage,
  parseRouteProvenance,
} from "../utils/core/runArtifactReview";
import { workflowStepForMlSignalStage } from "../utils/core/workflowModel";
import { readStorageJson, writeStorageJson } from "../utils/storage";
import {
  applyWorkflowArtifactsPayload,
  useWorkflowArtifactsStore,
} from "../stores/workflowArtifactsStore";
import { resolveRuntimePinia } from "../stores/runtimePinia";
import { measureArtifactHistoryPerf } from "./artifactHistoryPerf";
import { openPathWithRevealFallback } from "../utils/openPath";
import { useAdvancedStore } from "../stores/advancedStore";
import { useAnalysisStore } from "../stores/analysisStore";
import { useDataStore } from "../stores/dataStore";
import { useExecutionStore } from "../stores/executionStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useStrategyStore } from "../stores/strategyStore";
import { useValidationStore } from "../stores/validationStore";
import { useWorkflowStore } from "../stores/workflowStore";

let dataServicePromise: Promise<typeof import("./dataService")> | null = null;
let optimizeServicePromise: Promise<typeof import("./optimizeService")> | null =
  null;
let analysisServicePromise: Promise<typeof import("./analysisService")> | null =
  null;

function loadDataService(): Promise<typeof import("./dataService")> {
  if (!dataServicePromise) dataServicePromise = import("./dataService");
  return dataServicePromise;
}

function loadOptimizeService(): Promise<typeof import("./optimizeService")> {
  if (!optimizeServicePromise)
    optimizeServicePromise = import("./optimizeService");
  return optimizeServicePromise;
}

function loadAnalysisService(): Promise<typeof import("./analysisService")> {
  if (!analysisServicePromise)
    analysisServicePromise = import("./analysisService");
  return analysisServicePromise;
}

function normalizePath(path: string): string {
  return String(path || "").trim();
}

function normalizeText(value: unknown): string {
  return String(value || "").trim();
}

function normalizeStringList(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  const out: string[] = [];
  for (const item of value) {
    const next = normalizeText(item);
    if (!next) continue;
    out.push(next);
  }
  return out;
}

function parseTagsInput(value: string): string[] {
  return normalizeText(value)
    .split(",")
    .map((token) => normalizeText(token))
    .filter((token) => token.length > 0);
}

function parseBooleanLike(value: unknown): boolean | null {
  if (typeof value === "boolean") return value;
  const raw = normalizeText(value).toLowerCase();
  if (!raw) return null;
  if (raw === "1" || raw === "true" || raw === "yes" || raw === "on")
    return true;
  if (raw === "0" || raw === "false" || raw === "no" || raw === "off")
    return false;
  return null;
}

function normalizeOptionalNumber(value: unknown): number | null {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

function isMlSignalStrategyKind(value: unknown): boolean {
  return normalizeText(value) === "MlSignal";
}

function hasMlSignalDatasetExportShape(
  req: Record<string, unknown> | null | undefined,
): boolean {
  return (
    normalizeText(req?.ma_source ?? req?.maSource).length > 0 ||
    Number.isFinite(Number(req?.horizon_bars ?? req?.horizonBars))
  );
}

function hasMlSignalValidationShape(
  validationReq: Record<string, unknown> | null | undefined,
): boolean {
  return (
    !!validationReq &&
    Array.isArray(validationReq.candidates) &&
    validationReq.candidates.length > 0
  );
}

function hasMlSignalSweepShape(
  validationReq: Record<string, unknown> | null | undefined,
): boolean {
  if (!validationReq || Array.isArray(validationReq.candidates)) return false;
  return (
    validationReq.t_long != null ||
    validationReq.t_short != null ||
    validationReq.delta != null ||
    validationReq.max_hold_bars != null
  );
}

function inferMlSignalWorkflowStep(
  req: Record<string, unknown> | null | undefined,
  validationReq?: Record<string, unknown> | null,
  mlSignalStage?: unknown,
): "data" | "optimize" | "validate" | "analyze" | "finalize" {
  const normalizedStage = normalizeText(mlSignalStage);
  if (normalizedStage) return workflowStepForMlSignalStage(normalizedStage);
  if (hasMlSignalDatasetExportShape(req)) return "data";
  if (hasMlSignalValidationShape(validationReq)) return "validate";
  if (hasMlSignalSweepShape(validationReq)) return "analyze";
  return "optimize";
}

function selectShellStateForRunRequest(
  req: Record<string, unknown> | null | undefined,
  validationReq?: Record<string, unknown> | null,
  mlSignalStage?: unknown,
): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  const workflowStore = useWorkflowStore(pinia);
  if (isMlSignalStrategyKind(req?.strategy_kind ?? req?.strategyKind)) {
    workflowStore.selectMode("ml-signal");
    workflowStore.selectWorkflowStep(
      inferMlSignalWorkflowStep(req, validationReq, mlSignalStage),
    );
    return;
  }
  workflowStore.selectMode("conventional");
}

function updateAnalysisStatus(message: string): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  const analysisStore = useAnalysisStore(pinia);
  analysisStore.statusLine = normalizeText(message) || "Ready.";
  analysisStore.source = "vue";
  analysisStore.updatedAtUnixMs = Date.now();
}

function downloadText(filename: string, body: string, mime: string): boolean {
  if (typeof document === "undefined") return false;
  if (typeof URL === "undefined" || typeof URL.createObjectURL !== "function")
    return false;
  const blob = new Blob([body], { type: mime });
  const href = URL.createObjectURL(blob);
  try {
    const anchor = document.createElement("a");
    anchor.href = href;
    anchor.download = filename;
    anchor.style.display = "none";
    document.body?.appendChild(anchor);
    anchor.click();
    anchor.remove();
    return true;
  } finally {
    URL.revokeObjectURL(href);
  }
}

function parsePresetPayloadText(text: string): Record<string, unknown> {
  const parsed = JSON.parse(String(text || ""));
  const payload =
    parsed && typeof parsed === "object" && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : null;
  const preset =
    payload &&
    payload.preset &&
    typeof payload.preset === "object" &&
    !Array.isArray(payload.preset)
      ? (payload.preset as Record<string, unknown>)
      : payload;
  if (!preset || typeof preset !== "object" || Array.isArray(preset)) {
    throw new Error("Preset payload is not an object.");
  }
  return preset;
}

function readPresetTextWithInputFallback(): Promise<string> {
  return new Promise((resolve, reject) => {
    if (typeof document === "undefined") {
      reject(new Error("Preset import is unavailable in this environment."));
      return;
    }
    for (const node of document.querySelectorAll(
      "input[data-vta-preset-import='1']",
    )) {
      try {
        node.remove();
      } catch {}
    }
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json,application/json";
    input.style.display = "none";
    input.dataset.vtaPresetImport = "1";
    const cleanup = () => {
      try {
        input.remove();
      } catch {}
    };
    input.addEventListener(
      "change",
      async () => {
        try {
          const file = input.files?.[0];
          if (!file || typeof file.text !== "function") {
            reject(new Error("Preset import canceled."));
            return;
          }
          resolve(await file.text());
        } catch (error) {
          reject(error);
        } finally {
          cleanup();
        }
      },
      { once: true },
    );
    document.body?.appendChild(input);
    input.click();
  });
}

async function pickPresetText(): Promise<string> {
  const showOpenFilePicker = (
    globalThis as {
      showOpenFilePicker?: (
        options?: unknown,
      ) => Promise<
        Array<{ getFile?: () => Promise<{ text?: () => Promise<string> }> }>
      >;
    }
  ).showOpenFilePicker;
  if (typeof showOpenFilePicker === "function") {
    try {
      const handles = await showOpenFilePicker({
        multiple: false,
        excludeAcceptAllOption: false,
        types: [
          {
            description: "Preset JSON",
            accept: { "application/json": [".json"] },
          },
        ],
      });
      const handle = Array.isArray(handles) ? handles[0] : null;
      if (!handle || typeof handle.getFile !== "function") {
        throw new Error("Preset import canceled.");
      }
      const file = await handle.getFile();
      if (!file || typeof file.text !== "function") {
        throw new Error("Unable to read selected preset file.");
      }
      return await file.text();
    } catch (error) {
      const name = normalizeText((error as { name?: unknown } | null)?.name);
      if (name === "AbortError") throw new Error("Preset import canceled.");
    }
  }
  return readPresetTextWithInputFallback();
}

interface PresetStoreAdapter {
  setInputField?: (field: string, value: string) => void;
  setSelectField?: (field: string, value: string) => void;
  setCheckboxField?: (field: string, checked: boolean) => void;
}

function applyPresetToVueStores(preset: Record<string, unknown>): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  const stores: PresetStoreAdapter[] = [
    useDataStore(pinia),
    useOptimizeStore(pinia),
    useAdvancedStore(pinia),
    useStrategyStore(pinia),
    useValidationStore(pinia),
    useAnalysisStore(pinia),
    useSettingsStore(pinia),
    useExecutionStore(pinia),
  ];

  for (const [field, value] of Object.entries(preset)) {
    const key = normalizeText(field);
    if (!key) continue;
    const next = value == null ? "" : String(value);
    const bool = parseBooleanLike(value);
    for (const store of stores) {
      if (typeof store.setInputField === "function")
        store.setInputField(key, next);
      if (typeof store.setSelectField === "function")
        store.setSelectField(key, next);
      if (bool != null && typeof store.setCheckboxField === "function")
        store.setCheckboxField(key, bool);
    }
  }
}

interface RunArtifactSummaryResponse {
  ts?: unknown;
  path?: unknown;
  bytes?: unknown;
  strategy_kind?: unknown;
  config_fingerprint?: unknown;
  config_fingerprint_version?: unknown;
  has_validation?: unknown;
  report_xlsx_path?: unknown;
  csv_path?: unknown;
  timeframe?: unknown;
  base_timeframe?: unknown;
  derived_timeframe?: unknown;
  resample_applied?: unknown;
  source_content_fingerprint?: unknown;
  route_provenance?: unknown;
  lineage?: unknown;
  tags?: unknown;
}

export interface HistorySavedRunSummary {
  ts: string;
  path: string;
  bytes: number;
  strategyKind: string;
  configFingerprint?: string;
  configFingerprintVersion?: number | null;
  hasValidation: boolean;
  reportXlsxPath: string;
  csvPath: string;
  timeframe?: string;
  baseTimeframe?: string;
  derivedTimeframe?: string;
  resampleApplied?: boolean | null;
  sourceContentFingerprint?: string;
  routeProvenance?: RouteProvenanceSummary | null;
  lineage?: ArtifactLineageSummary | null;
  tags: string[];
}

export interface HistorySavedRunsRefreshSummary {
  runs: HistorySavedRunSummary[];
}

export interface HistorySavedRunDetail {
  strategyKind: string;
  configFingerprint?: string;
  configFingerprintVersion?: number | null;
  timeframe: string;
  csvPath: string;
  reportPath: string;
  hasValidation: boolean;
  datasetMeta?: Record<string, unknown> | null;
  routeProvenance?: RouteProvenanceSummary | null;
  lineage?: ArtifactLineageSummary | null;
  optimizationMetricSurface?: Record<string, unknown> | null;
  runReq?: Record<string, unknown> | null;
  runRes?: Record<string, unknown> | null;
  metricSurfaceSummary?: string;
  metricSurfaceDetail?: string;
  validationAdmissionSummary?: string;
  validationAdmissionDetail?: string;
  tags: string[];
  mlBundleId?: string;
  mlScoreCachePath?: string;
  mlScoreManifestPath?: string;
  mlFeatureSchemaPath?: string;
  mlFeatureSchemaVersion?: string;
  mlPolicySummary?: string;
  mlPolicySweepSummary?: string;
  mlRunVerdictSummary?: string;
  mlTopCandidatesSummary?: string;
  paramsText: string;
}

interface RunArtifactHydrationResponse {
  currentRun?: unknown;
  currentValidation?: unknown;
}

interface RunArtifactDetailResponse extends HistorySavedRunDetail {
  validationReq?: Record<string, unknown> | null;
  mlSignalStage?: string;
}

interface SessionHistoryEntry {
  ts?: unknown;
  backend?: unknown;
  pairs?: unknown;
  req?: unknown;
  res?: unknown;
}

interface NormalizedSessionHistoryEntry {
  ts: string;
  backend: string;
  pairs: number;
  req: unknown;
  res: unknown;
}

export interface HistorySessionSummary {
  ts: string;
  backend: string;
  pairs: number;
}

function normalizeSessionHistoryEntries(
  raw: unknown,
): NormalizedSessionHistoryEntry[] {
  if (!Array.isArray(raw)) return [];
  const out: NormalizedSessionHistoryEntry[] = [];
  for (const entry of raw) {
    const row = (
      entry && typeof entry === "object" ? entry : {}
    ) as SessionHistoryEntry;
    const ts = normalizeText(row.ts);
    if (!ts) continue;
    const pairs = Number(row.pairs);
    out.push({
      ts,
      backend: normalizeText(row.backend),
      pairs: Number.isFinite(pairs) && pairs > 0 ? Math.floor(pairs) : 0,
      req: row.req ?? null,
      res: row.res ?? null,
    });
  }
  return out;
}

function readSessionHistoryEntries(): NormalizedSessionHistoryEntry[] {
  return normalizeSessionHistoryEntries(readStorageJson<unknown>(HISTORY_KEY));
}

export function readSessionHistory(): HistorySessionSummary[] {
  return readSessionHistoryEntries().map((entry) => ({
    ts: entry.ts,
    backend: entry.backend,
    pairs: entry.pairs,
  }));
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object"
    ? (value as Record<string, unknown>)
    : null;
}

function asNumber(value: unknown): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return Number.NaN;
  return parsed;
}

function buildValidationAdmissionSummary(
  validationReq: Record<string, unknown> | null,
): { summary: string; detail: string } {
  if (!validationReq) return { summary: "", detail: "" };
  const admissionPolicy = normalizeText(
    validationReq.admission_policy ?? validationReq.admissionPolicy,
  );
  if (!admissionPolicy) return { summary: "", detail: "" };
  const provenance = asRecord(
    validationReq.admission_provenance ?? validationReq.admissionProvenance,
  );
  const requested = normalizeText(
    provenance?.requested_policy ?? provenance?.requestedPolicy,
  );
  const sourceKind = normalizeText(
    provenance?.source_kind ?? provenance?.sourceKind,
  );
  const postCap = asNumber(
    provenance?.post_cap_count ?? provenance?.postCapCount,
  );
  const postDedupe = asNumber(
    provenance?.post_dedupe_count ?? provenance?.postDedupeCount,
  );
  const heads = Array.isArray(provenance?.heads)
    ? (provenance?.heads as unknown[])
        .map((entry) => asRecord(entry))
        .map((entry) => normalizeText(entry?.objective ?? entry?.head))
        .filter(
          (value, index, values) =>
            value.length > 0 && values.indexOf(value) === index,
        )
    : [];

  const summaryParts = [admissionPolicy];
  if (
    Number.isFinite(postCap) &&
    postCap > 0 &&
    Number.isFinite(postDedupe) &&
    postDedupe > 0
  ) {
    summaryParts.push(`kept ${Math.floor(postCap)}/${Math.floor(postDedupe)}`);
  }

  const detailParts: string[] = [];
  if (requested && requested !== admissionPolicy) detailParts.push(`req=${requested}`);
  if (sourceKind) detailParts.push(`src=${sourceKind}`);
  if (heads.length > 0) detailParts.push(`heads=${heads.slice(0, 4).join("/")}`);

  return {
    summary: summaryParts.join(" • "),
    detail: detailParts.join(" • "),
  };
}

function buildMetricSurfaceSummary(
  metricSurface: Record<string, unknown> | null,
): { summary: string; detail: string } {
  if (!metricSurface) return { summary: "", detail: "" };
  const surface = normalizeText(metricSurface.surface).toLowerCase();
  const fields = Array.isArray(metricSurface.fields)
    ? (metricSurface.fields as unknown[])
        .map((entry) => normalizeText(entry))
        .filter((entry) => entry.length > 0)
    : [];
  const summary =
    surface === "trade22"
      ? `Trade22 • ${fields.length || 22} metrics`
      : surface === "core17"
        ? `Core17 • ${fields.length || 17} metrics`
        : surface === "legacy"
          ? "Legacy / route-specific"
          : "";
  if (!summary) return { summary: "", detail: "" };
  if (!fields.length) return { summary, detail: "" };
  const preview = fields.slice(0, 6).join(", ");
  const suffix =
    fields.length > 6 ? ` +${fields.length - 6} more` : "";
  return {
    summary,
    detail: `fields=${preview}${suffix}`,
  };
}

function firstBooleanFromList(value: unknown): boolean | null {
  if (!Array.isArray(value) || value.length === 0) return null;
  return parseBooleanLike(value[0]);
}

function applyMlStrategyConfigToStores(
  optimizeStore: ReturnType<typeof useOptimizeStore>,
  settingsStore: ReturnType<typeof useSettingsStore> | null,
  executionStore: ReturnType<typeof useExecutionStore> | null,
  strategyStore: ReturnType<typeof useStrategyStore>,
  config: Record<string, unknown> | null,
): void {
  if (!config) return;
  const longOnly = parseBooleanLike(config.long_only);
  const allowFlip = parseBooleanLike(config.allow_flip);
  const tradeOnNext = parseBooleanLike(
    config.trade_on_next_bar ?? config.tradeOnNextBar,
  );
  const costPreset = normalizeText(config.cost_preset ?? config.costPreset);
  const feeBps = asNumber(config.fee_bps ?? config.feeBps);
  const spreadBps = asNumber(config.spread_bps ?? config.spreadBps);
  const slippageBps = asNumber(config.slippage_bps ?? config.slippageBps);
  const stopLossPct = asNumber(config.stop_loss_pct ?? config.stopLossPct);
  const takeProfitPct = asNumber(
    config.take_profit_pct ?? config.takeProfitPct,
  );
  const cooldownBars = asNumber(config.cooldown_bars ?? config.cooldownBars);
  const persistenceBars = asNumber(
    config.signal_persistence_bars ?? config.signalPersistenceBars,
  );
  const epsRel = asNumber(config.eps_rel ?? config.epsRel);
  if (costPreset) strategyStore.setSelectField("costPreset", costPreset);
  if (Number.isFinite(feeBps)) strategyStore.setInputField("feeBps", String(feeBps));
  if (Number.isFinite(spreadBps)) {
    strategyStore.setInputField("spreadBps", String(spreadBps));
  }
  if (Number.isFinite(slippageBps)) {
    strategyStore.setInputField("slippageBps", String(slippageBps));
  }
  if (Number.isFinite(stopLossPct)) {
    strategyStore.setInputField("stopLossPct", String(stopLossPct));
  }
  if (Number.isFinite(takeProfitPct)) {
    strategyStore.setInputField("takeProfitPct", String(takeProfitPct));
  }
  if (Number.isFinite(cooldownBars)) {
    strategyStore.setInputField("cooldownBars", String(Math.floor(cooldownBars)));
  }
  if (Number.isFinite(persistenceBars)) {
    strategyStore.setInputField(
      "signalPersistenceBars",
      String(Math.max(1, Math.floor(persistenceBars))),
    );
  }
  if (Number.isFinite(epsRel)) strategyStore.setInputField("epsRel", String(epsRel));
  if (longOnly != null) strategyStore.setCheckboxField("longOnly", longOnly);
  if (allowFlip != null) strategyStore.setCheckboxField("allowFlip", allowFlip);
  if (tradeOnNext != null) {
    strategyStore.setCheckboxField("tradeOnNext", tradeOnNext);
  }
  if (settingsStore) {
    settingsStore.source = "vue";
    settingsStore.updatedAtUnixMs = Date.now();
  }
  if (executionStore) {
    executionStore.source = "vue";
    executionStore.updatedAtUnixMs = Date.now();
  }
  optimizeStore.source = "vue";
  optimizeStore.updatedAtUnixMs = Date.now();
}

function updateStoresFromRunRequest(
  req: Record<string, unknown>,
  csvPath: string,
  timeframe: string,
  validationReq: Record<string, unknown> | null = null,
): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;

  const optimizeStore = useOptimizeStore(pinia);
  const advancedStore = useAdvancedStore(pinia);
  const dataStore = useDataStore(pinia);
  const strategyStore = useStrategyStore(pinia);
  const settingsStore = useSettingsStore(pinia);
  const executionStore = useExecutionStore(pinia);
  const validationStore = useValidationStore(pinia);

  const strategyKind = normalizeText(req.strategy_kind ?? req.strategyKind);
  const objective = normalizeText(req.objective);
  const mode = normalizeText(req.mode);
  const maSource = normalizeText(req.ma_source ?? req.maSource);
  const scoreMode = normalizeText(req.score_mode ?? req.scoreMode);
  const hurdleTau = normalizeText(req.hurdle_tau ?? req.hurdleTau);
  const metricContract =
    normalizeText(req.metric_contract ?? req.metricContract) || "auto";
  const includeTradeMetrics =
    parseBooleanLike(req.include_trade_metrics ?? req.includeTradeMetrics) ??
    false;
  const extraShortlistObjectives = normalizeStringList(
    req.extra_shortlist_objectives ?? req.extraShortlistObjectives,
  ).join(", ");
  const shortlistConfidenceMode = normalizeText(
    req.shortlist_confidence_mode ?? req.shortlistConfidenceMode,
  );
  const shortlistUnionMode = normalizeText(
    req.shortlist_union_mode ?? req.shortlistUnionMode,
  );
  const shortlistSliceMode = normalizeText(
    req.shortlist_slice_mode ?? req.shortlistSliceMode,
  );
  const shortlistSliceBenchmark = normalizeText(
    req.shortlist_slice_benchmark ?? req.shortlistSliceBenchmark,
  );
  const shortlistNeighborhoodRadiusValue = asNumber(
    req.shortlist_neighborhood_radius ?? req.shortlistNeighborhoodRadius,
  );
  const shortlistNeighborhoodStepValue = asNumber(
    req.shortlist_neighborhood_step ?? req.shortlistNeighborhoodStep,
  );
  const topKValue = asNumber(req.top_k ?? req.topK);
  const includeAll =
    parseBooleanLike(req.include_all ?? req.includeAll) ?? false;
  const heatmapBinsValue = asNumber(req.heatmap_bins ?? req.heatmapBins);
  const exportAllCsvPath = normalizeText(
    req.export_all_csv_path ?? req.exportAllCsvPath,
  );
  const backendRaw = asRecord(req.backend);
  const backend = normalizeText(backendRaw?.backend ?? req.backend);
  const fastRange = asRecord(req.fast_range);
  const slowRange = asRecord(req.slow_range);
  const fastTypes = normalizeStringList(req.fast_ma_types);
  const slowTypes = normalizeStringList(req.slow_ma_types);

  const mlSignalStrategy = strategyKind === "MlSignal";
  if (strategyKind && !mlSignalStrategy) {
    optimizeStore.applyLegacySnapshot({
      strategyKind,
    });
  }
  if (mlSignalStrategy) {
    const outputPath = normalizeText(req.output_path ?? req.outputPath);
    const inputPath = normalizeText(req.input_path ?? req.inputPath);
    const scoreCachePath = normalizeText(
      req.score_cache_path ?? req.scoreCachePath,
    );
    const modelBundleId = normalizeText(
      req.model_bundle_id ?? req.modelBundleId,
    );
    const splitPlan = asRecord(req.split_plan ?? req.splitPlan);
    const tickerHoldout = asRecord(req.ticker_holdout ?? req.tickerHoldout);
    const validationStrategy = asRecord(
      validationReq?.base_strategy ??
        (Array.isArray(validationReq?.candidates)
          ? asRecord(
              (validationReq?.candidates as Array<unknown>)[0] &&
                typeof (validationReq?.candidates as Array<unknown>)[0] ===
                  "object"
                ? (
                    validationReq?.candidates as Array<Record<string, unknown>>
                  )[0]?.strategy
                : null,
            )
          : null),
    );
    const hasDatasetExportShape =
      normalizeText(req.ma_source ?? req.maSource).length > 0 ||
      Number.isFinite(Number(req.horizon_bars ?? req.horizonBars));
    const maSourceValue = normalizeText(req.ma_source ?? req.maSource);
    const horizonBars = asNumber(req.horizon_bars ?? req.horizonBars);
    if (inputPath) optimizeStore.mlSignalScoreInputPath = inputPath;
    if (hasDatasetExportShape) {
      if (outputPath) optimizeStore.mlSignalDatasetExportPath = outputPath;
    } else if (outputPath) {
      optimizeStore.mlSignalScoreCachePath = outputPath;
    }
    if (scoreCachePath) optimizeStore.mlSignalScoreCachePath = scoreCachePath;
    if (modelBundleId) optimizeStore.mlSignalModelBundleId = modelBundleId;
    if (maSourceValue) optimizeStore.mlSignalMaSource = maSourceValue;
    if (Number.isFinite(horizonBars) && horizonBars > 0) {
      optimizeStore.mlSignalHorizonBars = String(Math.floor(horizonBars));
    }
    if (splitPlan) {
      const trainBars = asNumber(splitPlan.train_bars);
      const calibrationBars = asNumber(splitPlan.calibration_bars);
      const testBars = asNumber(splitPlan.test_bars);
      const gapBars = asNumber(splitPlan.gap_bars);
      const stepBars = asNumber(splitPlan.step_bars);
      optimizeStore.mlSignalSplitTrainBars = Number.isFinite(trainBars)
        ? String(Math.floor(trainBars))
        : "";
      optimizeStore.mlSignalSplitCalibrationBars = Number.isFinite(
        calibrationBars,
      )
        ? String(Math.floor(calibrationBars))
        : "";
      optimizeStore.mlSignalSplitTestBars = Number.isFinite(testBars)
        ? String(Math.floor(testBars))
        : "";
      if (Number.isFinite(gapBars))
        optimizeStore.mlSignalSplitGapBars = String(Math.floor(gapBars));
      if (Number.isFinite(stepBars) && stepBars > 0)
        optimizeStore.mlSignalSplitStepBars = String(Math.floor(stepBars));
    }
    if (tickerHoldout) {
      const modulus = asNumber(tickerHoldout.modulus);
      const residue = asNumber(tickerHoldout.residue);
      optimizeStore.mlSignalHoldoutModulus = Number.isFinite(modulus)
        ? String(Math.floor(modulus))
        : "";
      if (Number.isFinite(residue))
        optimizeStore.mlSignalHoldoutResidue = String(Math.floor(residue));
    }
    const firstCandidate =
      Array.isArray(validationReq?.candidates) &&
      validationReq?.candidates[0] &&
      typeof validationReq.candidates[0] === "object"
        ? (validationReq.candidates[0] as Record<string, unknown>)
        : null;
    if (firstCandidate) {
      const tLong = asNumber(firstCandidate.t_long);
      const tShort = asNumber(firstCandidate.t_short);
      const delta = asNumber(firstCandidate.delta);
      const maxHoldBars = asNumber(firstCandidate.max_hold_bars);
      const exitOnFlat = parseBooleanLike(firstCandidate.exit_on_flat);
      const exitOnOpposite = parseBooleanLike(firstCandidate.exit_on_opposite);
      if (Number.isFinite(tLong)) optimizeStore.mlSignalLongThreshold = String(tLong);
      if (Number.isFinite(tShort))
        optimizeStore.mlSignalShortThreshold = String(tShort);
      if (Number.isFinite(delta)) optimizeStore.mlSignalDelta = String(delta);
      if (Number.isFinite(maxHoldBars)) {
        optimizeStore.mlSignalMaxHoldBars = String(Math.floor(maxHoldBars));
      }
      if (exitOnFlat != null) optimizeStore.mlSignalExitOnFlat = exitOnFlat;
      if (exitOnOpposite != null) {
        optimizeStore.mlSignalExitOnOpposite = exitOnOpposite;
      }
    } else if (validationReq) {
      const tLong = asRecord(validationReq.t_long);
      const tShort = asRecord(validationReq.t_short);
      const delta = asRecord(validationReq.delta);
      const hold = asRecord(validationReq.max_hold_bars);
      const topK = asNumber(validationReq.top_k);
      const exitOnFlat = firstBooleanFromList(validationReq.exit_on_flat_values);
      const exitOnOpposite = firstBooleanFromList(
        validationReq.exit_on_opposite_values,
      );
      if (tLong) {
        optimizeStore.mlSignalSweepLongStart = normalizeText(tLong.start);
        optimizeStore.mlSignalSweepLongEnd = normalizeText(tLong.end);
        optimizeStore.mlSignalSweepLongStep = normalizeText(tLong.step);
      }
      if (tShort) {
        optimizeStore.mlSignalSweepShortStart = normalizeText(tShort.start);
        optimizeStore.mlSignalSweepShortEnd = normalizeText(tShort.end);
        optimizeStore.mlSignalSweepShortStep = normalizeText(tShort.step);
      }
      if (delta) {
        optimizeStore.mlSignalSweepDeltaStart = normalizeText(delta.start);
        optimizeStore.mlSignalSweepDeltaEnd = normalizeText(delta.end);
        optimizeStore.mlSignalSweepDeltaStep = normalizeText(delta.step);
      }
      if (hold) {
        optimizeStore.mlSignalSweepHoldStart = normalizeText(hold.start);
        optimizeStore.mlSignalSweepHoldEnd = normalizeText(hold.end);
        optimizeStore.mlSignalSweepHoldStep = normalizeText(hold.step);
      }
      if (Number.isFinite(topK) && topK > 0) {
        optimizeStore.mlSignalSweepTopK = String(Math.floor(topK));
      }
      if (exitOnFlat != null) optimizeStore.mlSignalExitOnFlat = exitOnFlat;
      if (exitOnOpposite != null) {
        optimizeStore.mlSignalExitOnOpposite = exitOnOpposite;
      }
    }
    const minPsr = asNumber(validationReq?.min_psr ?? validationReq?.minPsr);
    const minDsr = asNumber(validationReq?.min_dsr ?? validationReq?.minDsr);
    if (Number.isFinite(minPsr)) optimizeStore.mlSignalMinPsr = String(minPsr);
    if (Number.isFinite(minDsr)) optimizeStore.mlSignalMinDsr = String(minDsr);
    applyMlStrategyConfigToStores(
      optimizeStore,
      settingsStore,
      executionStore,
      strategyStore,
      validationStrategy,
    );
    if (csvPath) dataStore.csvPath = csvPath;
    dataStore.dataTimeframe = timeframe;
    optimizeStore.statusLine = "Applied saved ML signal run settings to form.";
    advancedStore.statusLine = "Applied saved ML signal settings.";
    dataStore.statusLine = "Saved ML signal run context loaded.";
    optimizeStore.source = "vue";
    optimizeStore.updatedAtUnixMs = Date.now();
    advancedStore.source = "vue";
    advancedStore.updatedAtUnixMs = Date.now();
    dataStore.source = "vue";
    dataStore.updatedAtUnixMs = Date.now();
    return;
  }
  advancedStore.applyLegacySnapshot({
    objective,
    mode,
    maSource: maSource || advancedStore.maSource || "close",
    scoreMode,
    hurdleTau,
    metricContract,
    includeTradeMetrics,
    extraShortlistObjectives,
    shortlistConfidenceMode,
    shortlistUnionMode,
    shortlistSliceMode,
    shortlistSliceBenchmark,
    shortlistNeighborhoodRadius: Number.isFinite(shortlistNeighborhoodRadiusValue)
      ? String(Math.floor(shortlistNeighborhoodRadiusValue))
      : "",
    shortlistNeighborhoodStep: Number.isFinite(shortlistNeighborhoodStepValue)
      ? String(Math.floor(shortlistNeighborhoodStepValue))
      : "",
    topK:
      Number.isFinite(topKValue) && topKValue >= 0
        ? String(Math.floor(topKValue))
        : "20",
    includeAll: includeAll ? "true" : "false",
    heatmapBins:
      Number.isFinite(heatmapBinsValue) && heatmapBinsValue >= 0
        ? String(Math.floor(heatmapBinsValue))
        : "128",
    exportAllCsvPath,
  } as never);

  const fastStart = asNumber(fastRange?.start);
  const fastEnd = asNumber(fastRange?.end);
  const fastStep = asNumber(fastRange?.step);
  const slowStart = asNumber(slowRange?.start);
  const slowEnd = asNumber(slowRange?.end);
  const slowStep = asNumber(slowRange?.step);
  const fastMaType = fastTypes[0];
  const slowMaType = slowTypes[0];
  optimizeStore.applyLegacySnapshot({
    backend,
    fastStart: Number.isFinite(fastStart)
      ? String(Math.floor(fastStart))
      : optimizeStore.fastStart,
    fastEnd: Number.isFinite(fastEnd)
      ? String(Math.floor(fastEnd))
      : optimizeStore.fastEnd,
    fastStep:
      Number.isFinite(fastStep) && fastStep > 0
        ? String(Math.floor(fastStep))
        : optimizeStore.fastStep,
    slowStart: Number.isFinite(slowStart)
      ? String(Math.floor(slowStart))
      : optimizeStore.slowStart,
    slowEnd: Number.isFinite(slowEnd)
      ? String(Math.floor(slowEnd))
      : optimizeStore.slowEnd,
    slowStep:
      Number.isFinite(slowStep) && slowStep > 0
        ? String(Math.floor(slowStep))
        : optimizeStore.slowStep,
    fastMaType: fastMaType || optimizeStore.fastMaType,
    slowMaType: slowMaType || optimizeStore.slowMaType,
  });

  if (csvPath) dataStore.csvPath = csvPath;
  dataStore.dataTimeframe = timeframe;

  if (validationReq) {
    const maxCandidates = asNumber(
      validationReq.max_candidates ?? validationReq.maxCandidates,
    );
    const seed = asNumber(validationReq.seed);
    const minPsr = asNumber(validationReq.min_psr ?? validationReq.minPsr);
    const minDsr = asNumber(validationReq.min_dsr ?? validationReq.minDsr);
    const admissionPolicy = normalizeText(
      validationReq.admission_policy ?? validationReq.admissionPolicy,
    );

    if (Number.isFinite(maxCandidates) && maxCandidates > 0) {
      validationStore.candidates = String(Math.floor(maxCandidates));
    }
    if (Number.isFinite(seed) && seed >= 0) {
      validationStore.seed = String(Math.floor(seed));
    }
    if (Number.isFinite(minPsr)) validationStore.minPsr = String(minPsr);
    if (Number.isFinite(minDsr)) validationStore.minDsr = String(minDsr);
    if (admissionPolicy) validationStore.admissionPolicy = admissionPolicy;
    validationStore.source = "vue";
    validationStore.updatedAtUnixMs = Date.now();
  }

  optimizeStore.statusLine = "Applied saved run settings to form.";
  advancedStore.statusLine = "Applied saved run settings.";
  dataStore.statusLine = "Saved run context loaded.";

  optimizeStore.source = "vue";
  optimizeStore.updatedAtUnixMs = Date.now();
  advancedStore.source = "vue";
  advancedStore.updatedAtUnixMs = Date.now();
  dataStore.source = "vue";
  dataStore.updatedAtUnixMs = Date.now();
}

async function loadRunArtifactHydration(
  path: string,
): Promise<{ path: string; payload: RunArtifactHydrationResponse }> {
  const nextPath = normalizePath(path);
  if (!nextPath) throw new Error("Select a saved run first.");
  return measureArtifactHistoryPerf(
    "historyService.loadSavedRunHydrationMs",
    async () => {
      const raw = (await invokeCommand("load_run_artifact_hydration", {
        path: nextPath,
      })) as RunArtifactHydrationResponse;
      return {
        path: nextPath,
        payload:
          raw && typeof raw === "object"
            ? raw
            : ({} as RunArtifactHydrationResponse),
      };
    },
    {
      meta: { path: nextPath },
      onSuccessMeta: (result) => ({
        hasRun: !!asRecord(result.payload.currentRun),
        hasValidation: !!asRecord(result.payload.currentValidation),
      }),
    },
  );
}

async function loadRunArtifactDetailRecord(
  path: string,
): Promise<{ path: string; detail: RunArtifactDetailResponse }> {
  const nextPath = normalizePath(path);
  if (!nextPath) throw new Error("Missing run artifact path.");
  return measureArtifactHistoryPerf(
    "historyService.loadSavedRunDetailMs",
    async () => {
      const raw = (await invokeCommand("load_run_artifact_detail", {
        path: nextPath,
      })) as RunArtifactDetailResponse;
      return {
        path: nextPath,
        detail:
          raw && typeof raw === "object"
            ? raw
            : ({} as RunArtifactDetailResponse),
      };
    },
    {
      meta: { path: nextPath },
      onSuccessMeta: (result) => ({
        hasValidation: !!result.detail.hasValidation,
        strategyKind: normalizeText(result.detail.strategyKind),
      }),
    },
  );
}

export async function applySession(ts: string): Promise<void> {
  const selectedTs = normalizeText(ts);
  if (!selectedTs) throw new Error("Select a run from history first.");

  const entry =
    readSessionHistoryEntries().find((row) => row.ts === selectedTs) || null;
  if (!entry?.req || !entry?.res)
    throw new Error("Selected history run is missing req/res.");

  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ts: entry.ts,
        artifact_path: "",
        report_xlsx_path: "",
        csv_path: "",
        tags: [],
        req: entry.req,
        res: entry.res,
      },
      currentValidation: null,
    },
    "vue",
  );
  selectShellStateForRunRequest(entry.req as Record<string, unknown> | null);
  if (
    isMlSignalStrategyKind(
      (entry.req as Record<string, unknown> | null)?.strategy_kind,
    )
  ) {
    updateAnalysisStatus(
      "ML signal run loaded. Analysis views remain focused on optimizer artifacts.",
    );
    return;
  }
  const { refreshAnalysisFromWorkflowRun } = await loadAnalysisService();
  refreshAnalysisFromWorkflowRun();
}

export async function exportPreset(): Promise<void> {
  const ts = new Date().toISOString().replace(/:/g, "-");
  const rawPreset = readStorageJson<Record<string, unknown>>(SETTINGS_KEY);
  const preset =
    rawPreset && typeof rawPreset === "object" && !Array.isArray(rawPreset)
      ? rawPreset
      : {};
  const payload = {
    version: 1,
    kind: "vectorgrid_preset",
    ts,
    preset,
  };
  const downloaded = downloadText(
    `vectorgrid_preset_${ts}.json`,
    JSON.stringify(payload, null, 2),
    "application/json;charset=utf-8",
  );
  if (!downloaded)
    throw new Error("Preset export is unavailable in this environment.");
}

export async function importPreset(): Promise<void> {
  const text = await pickPresetText();
  const preset = parsePresetPayloadText(text);
  writeStorageJson(SETTINGS_KEY, preset);
  applyPresetToVueStores(preset);
}

export async function clearSession(): Promise<void> {
  writeStorageJson(HISTORY_KEY, []);
  applyWorkflowArtifactsPayload(
    {
      currentRun: null,
      currentValidation: null,
    },
    "vue",
  );
}

export async function saveSavedRunTags(
  path: string,
  tags: string,
): Promise<void> {
  const nextPath = normalizePath(path);
  if (!nextPath) return;
  await invokeCommand("update_artifact_tags", {
    path: nextPath,
    tags: parseTagsInput(tags),
  });
}

export async function refreshSavedRuns(): Promise<HistorySavedRunsRefreshSummary> {
  return measureArtifactHistoryPerf(
    "historyService.refreshSavedRunsMs",
    async () => {
      const rows = (await invokeCommand("list_run_artifacts")) as unknown;
      const list = Array.isArray(rows) ? rows : [];
      const runs: HistorySavedRunSummary[] = [];

      for (const entry of list) {
        const row = (
          entry && typeof entry === "object" ? entry : {}
        ) as RunArtifactSummaryResponse;
        const path = normalizeText(row.path);
        if (!path) continue;

        const bytes = Number(row.bytes);
        runs.push({
          ts: normalizeText(row.ts),
          path,
          bytes: Number.isFinite(bytes) && bytes > 0 ? Math.floor(bytes) : 0,
          strategyKind: normalizeText(row.strategy_kind),
          configFingerprint: normalizeText(row.config_fingerprint),
          configFingerprintVersion: normalizeOptionalNumber(
            row.config_fingerprint_version,
          ),
          hasValidation: !!row.has_validation,
          reportXlsxPath: normalizeText(row.report_xlsx_path),
          csvPath: normalizeText(row.csv_path),
          timeframe: normalizeText(row.timeframe),
          baseTimeframe: normalizeText(row.base_timeframe),
          derivedTimeframe: normalizeText(row.derived_timeframe),
          resampleApplied:
            row.resample_applied == null
              ? null
              : parseBooleanLike(row.resample_applied),
          sourceContentFingerprint: normalizeText(row.source_content_fingerprint),
          routeProvenance: parseRouteProvenance(row.route_provenance),
          lineage: parseArtifactLineage(row.lineage),
          tags: normalizeStringList(row.tags),
        });
      }

      return { runs };
    },
    {
      onSuccessMeta: (summary) => ({
        runCount: summary.runs.length,
        validationCount: summary.runs.filter((run) => run.hasValidation).length,
      }),
    },
  );
}

export async function openRunsDir(): Promise<void> {
  await invokeCommand("open_runs_dir");
}

export async function loadSavedRun(path: string): Promise<void> {
  const loaded = await loadRunArtifactHydration(path);
  const payload =
    loaded.payload && typeof loaded.payload === "object"
      ? loaded.payload
      : ({} as RunArtifactHydrationResponse);
  applyWorkflowArtifactsPayload(payload, "vue");

  const pinia = resolveRuntimePinia();
  const currentRun = pinia ? useWorkflowArtifactsStore(pinia).currentRun : null;
  const runReq =
    currentRun && currentRun.req && typeof currentRun.req === "object"
      ? (currentRun.req as Record<string, unknown>)
      : null;
  selectShellStateForRunRequest(
    runReq,
    payload.currentValidation && typeof payload.currentValidation === "object"
      ? asRecord(
          (payload.currentValidation as Record<string, unknown>).req,
        )
      : null,
    currentRun?.mlSignal?.stage,
  );
  if (
    isMlSignalStrategyKind(
      (runReq as Record<string, unknown> | null)?.strategy_kind,
    )
  ) {
    updateAnalysisStatus(
      "ML signal artifact loaded. Use workflow/history details or rerun validation to inspect results.",
    );
    return;
  }
  const { refreshAnalysisFromWorkflowRun } = await loadAnalysisService();
  refreshAnalysisFromWorkflowRun();
}

export async function applySavedRun(path: string): Promise<void> {
  const loaded = await loadRunArtifactDetailRecord(path);
  const detail = loaded.detail;
  const req = asRecord(detail.runReq);
  const validationReq = asRecord(detail.validationReq);
  if (!req) throw new Error("Saved run artifact is missing run.req.");
  const csvPath = normalizeText(detail.csvPath);
  const timeframe = normalizeText(detail.timeframe);
  selectShellStateForRunRequest(req, validationReq, detail.mlSignalStage);
  updateStoresFromRunRequest(req, csvPath, timeframe, validationReq);
}

export async function rerunSavedRun(path: string): Promise<void> {
  const loaded = await loadRunArtifactDetailRecord(path);
  const detail = loaded.detail;
  const req = asRecord(detail.runReq);
  const validationReq = asRecord(detail.validationReq);
  if (!req) throw new Error("Saved run artifact is missing run.req.");

  const csvPath = normalizeText(detail.csvPath);
  if (!csvPath)
    throw new Error(
      "Saved run is missing data.csv_path. Load a dataset first, then rerun.",
    );

  const { applyTimeframe: applyDataTimeframe, loadCsv: loadDataCsv } =
    await loadDataService();
  const base = await loadDataCsv(csvPath);
  let currentDataId = normalizeText(base.dataId);
  const timeframe = normalizeText(detail.timeframe);
  if (timeframe) {
    const baseTf = normalizeText(base.meta.timeframe);
    if (!baseTf || baseTf !== timeframe) {
      const resampled = await applyDataTimeframe(timeframe, currentDataId);
      currentDataId = normalizeText(resampled.dataId) || currentDataId;
    }
  }

  updateStoresFromRunRequest(req, csvPath, timeframe, validationReq);

  const request: Record<string, unknown> = { ...req };
  if (currentDataId) {
    request.data_id = currentDataId;
    request.dataId = currentDataId;
  }
  selectShellStateForRunRequest(req, validationReq, detail.mlSignalStage);
  if (isMlSignalStrategyKind(req.strategy_kind ?? req.strategyKind)) {
    const {
      exportMlSignalDataset,
      generateMlSignalScoreCache,
      runMlSignalPolicySweep,
      runMlSignalValidation,
    } = await import("./mlSignalService");
    const pinia = resolveRuntimePinia();
    const isDatasetExportStage =
      normalizeText(detail.mlSignalStage) === "dataset_export" ||
      (!!normalizeText(request.ma_source ?? request.maSource) &&
        !normalizeText(request.input_path ?? request.inputPath));
    if (pinia) {
      const optimizeStore = useOptimizeStore(pinia);
      if (normalizeText(request.input_path ?? request.inputPath)) {
        optimizeStore.mlSignalScoreInputPath = normalizeText(
          request.input_path ?? request.inputPath,
        );
      }
      if (normalizeText(request.output_path ?? request.outputPath)) {
        if (isDatasetExportStage) {
          optimizeStore.mlSignalDatasetExportPath = normalizeText(
            request.output_path ?? request.outputPath,
          );
        } else {
          optimizeStore.mlSignalScoreCachePath = normalizeText(
            request.output_path ?? request.outputPath,
          );
        }
      }
      if (normalizeText(request.model_bundle_id ?? request.modelBundleId)) {
        optimizeStore.mlSignalModelBundleId = normalizeText(
          request.model_bundle_id ?? request.modelBundleId,
        );
      }
    }
    const hasSweepReq =
      !!validationReq &&
      !Array.isArray(validationReq.candidates) &&
      (validationReq.t_long != null ||
        validationReq.t_short != null ||
        validationReq.delta != null ||
        validationReq.max_hold_bars != null);
    const hasValidationReq =
      !!validationReq && Array.isArray(validationReq.candidates);

    if (isDatasetExportStage) {
      await exportMlSignalDataset();
      return;
    }

    await generateMlSignalScoreCache();
    if (hasValidationReq) {
      await runMlSignalValidation();
      return;
    }
    if (hasSweepReq) {
      await runMlSignalPolicySweep();
      return;
    }
    return;
  }
  const { runOptimization } = await loadOptimizeService();
  await runOptimization({ request, switchTab: true });
}

export async function openSavedRunReport(path: string): Promise<void> {
  const detail = await loadSavedRunDetail(path);
  const reportPath = normalizePath(detail.reportPath);
  if (!reportPath)
    throw new Error(
      "No report path for this saved run yet (run validation + export XLSX).",
    );
  await openPathWithRevealFallback(reportPath);
}

export async function revealSavedRun(path: string): Promise<void> {
  const nextPath = normalizePath(path);
  if (!nextPath) return;
  await invokeCommand("reveal_path", { path: nextPath });
}

export async function deleteSavedRun(path: string): Promise<void> {
  const nextPath = normalizePath(path);
  if (!nextPath) return;
  await invokeCommand("delete_run_artifact", { path: nextPath });
}

export async function loadSavedRunDetail(
  path: string,
): Promise<HistorySavedRunDetail> {
  const loaded = await loadRunArtifactDetailRecord(path);
  const detail = loaded.detail;
  const validationReq = asRecord(detail.validationReq);
  const admission = buildValidationAdmissionSummary(validationReq);
  const metricSurface = buildMetricSurfaceSummary(
    asRecord(
      (detail as unknown as Record<string, unknown>).optimizationMetricSurface ??
        (detail as unknown as Record<string, unknown>).optimization_metric_surface,
    ),
  );
  return {
    strategyKind: normalizeText(detail.strategyKind),
    configFingerprint: normalizeText(
      detail.configFingerprint ??
        (detail as unknown as Record<string, unknown>).config_fingerprint,
    ),
    configFingerprintVersion: normalizeOptionalNumber(
      detail.configFingerprintVersion ??
        (detail as unknown as Record<string, unknown>).config_fingerprint_version,
    ),
    timeframe: normalizeText(detail.timeframe),
    csvPath: normalizeText(detail.csvPath),
    reportPath: normalizeText(detail.reportPath),
    hasValidation: !!detail.hasValidation,
    datasetMeta:
      detail.datasetMeta && typeof detail.datasetMeta === "object"
        ? (detail.datasetMeta as Record<string, unknown>)
        : null,
    routeProvenance: parseRouteProvenance(
      detail.routeProvenance ??
        (detail as unknown as Record<string, unknown>).route_provenance,
    ),
    lineage: parseArtifactLineage(detail.lineage),
    optimizationMetricSurface: asRecord(
      detail.optimizationMetricSurface ??
        (detail as unknown as Record<string, unknown>).optimization_metric_surface,
    ),
    runReq: asRecord(detail.runReq),
    runRes: asRecord(detail.runRes),
    ...(metricSurface.summary
      ? { metricSurfaceSummary: metricSurface.summary }
      : {}),
    ...(metricSurface.detail ? { metricSurfaceDetail: metricSurface.detail } : {}),
    ...(admission.summary
      ? { validationAdmissionSummary: admission.summary }
      : {}),
    ...(admission.detail
      ? { validationAdmissionDetail: admission.detail }
      : {}),
    tags: normalizeStringList(detail.tags),
    mlBundleId: normalizeText(detail.mlBundleId),
    mlScoreCachePath: normalizeText(detail.mlScoreCachePath),
    mlScoreManifestPath: normalizeText(detail.mlScoreManifestPath),
    mlFeatureSchemaPath: normalizeText(detail.mlFeatureSchemaPath),
    mlFeatureSchemaVersion: normalizeText(detail.mlFeatureSchemaVersion),
    mlPolicySummary: normalizeText(detail.mlPolicySummary),
    mlPolicySweepSummary: normalizeText(detail.mlPolicySweepSummary),
    mlRunVerdictSummary: normalizeText(detail.mlRunVerdictSummary),
    mlTopCandidatesSummary: normalizeText(detail.mlTopCandidatesSummary),
    paramsText: normalizeText(detail.paramsText),
  };
}
