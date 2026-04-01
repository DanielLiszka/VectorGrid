import { invokeCommand } from "../tauri/invoke";
import { HISTORY_KEY, SETTINGS_KEY } from "../utils/core/storageKeys";
import { readStorageJson, writeStorageJson } from "../utils/storage";
import { applyWorkflowArtifactsPayload } from "../stores/workflowArtifactsStore";
import { resolveRuntimePinia } from "../stores/runtimePinia";
import { useAdvancedStore } from "../stores/advancedStore";
import { useAnalysisStore } from "../stores/analysisStore";
import { useDataStore } from "../stores/dataStore";
import { useExecutionStore } from "../stores/executionStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useStrategyStore } from "../stores/strategyStore";
import { useValidationStore } from "../stores/validationStore";
import {
  applyTimeframe as applyDataTimeframe,
  loadCsv as loadDataCsv,
} from "./dataService";
import { runOptimization } from "./optimizeService";
import { refreshAnalysisFromWorkflowRun } from "./analysisService";

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

function isMlSignalStrategyKind(value: unknown): boolean {
  return normalizeText(value) === "MlSignal";
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
  has_validation?: unknown;
  report_xlsx_path?: unknown;
  csv_path?: unknown;
  tags?: unknown;
}

export interface HistorySavedRunSummary {
  ts: string;
  path: string;
  bytes: number;
  strategyKind: string;
  hasValidation: boolean;
  reportXlsxPath: string;
  csvPath: string;
  tags: string[];
}

export interface HistorySavedRunsRefreshSummary {
  runs: HistorySavedRunSummary[];
}

export interface HistorySavedRunDetail {
  strategyKind: string;
  timeframe: string;
  csvPath: string;
  reportPath: string;
  hasValidation: boolean;
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

interface RunArtifactResponse {
  ts?: unknown;
  strategy_kind?: unknown;
  artifact_path?: unknown;
  artifactPath?: unknown;
  report_xlsx_path?: unknown;
  reportXlsxPath?: unknown;
  csv_path?: unknown;
  csvPath?: unknown;
  tags?: unknown;
  dataset_meta?: unknown;
  data?: unknown;
  ml_signal?: unknown;
  run?: unknown;
  validation?: unknown;
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

function formatRangeLabel(value: unknown): string {
  const range =
    value && typeof value === "object"
      ? (value as Record<string, unknown>)
      : null;
  if (!range) return "-";
  const start = Number(range.start);
  const end = Number(range.end);
  const step = Number(range.step);
  if (!Number.isFinite(start) || !Number.isFinite(end)) return "-";
  const a = Math.floor(start);
  const b = Math.floor(end);
  if (Number.isFinite(step) && step > 0)
    return `${a}-${b} x${Math.floor(step)}`;
  return `${a}-${b}`;
}

function formatMlPolicySummary(value: unknown): string {
  const row =
    value && typeof value === "object"
      ? (value as Record<string, unknown>)
      : null;
  if (!row) return "";
  const parts: string[] = [];
  if (row.t_long != null) parts.push(`t_long=${normalizeText(row.t_long)}`);
  if (row.t_short != null) parts.push(`t_short=${normalizeText(row.t_short)}`);
  if (row.delta != null) parts.push(`delta=${normalizeText(row.delta)}`);
  if (row.max_hold_bars != null)
    parts.push(`hold=${normalizeText(row.max_hold_bars)}`);
  return parts.join(" | ");
}

function formatMlPolicySweepSummary(value: unknown): string {
  const row =
    value && typeof value === "object"
      ? (value as Record<string, unknown>)
      : null;
  if (!row) return "";
  const parts: string[] = [];
  if (row.sweep_size != null)
    parts.push(`size=${normalizeText(row.sweep_size)}`);
  if (row.top_k != null) parts.push(`top_k=${normalizeText(row.top_k)}`);
  if (row.best_label != null)
    parts.push(`best=${normalizeText(row.best_label)}`);
  if (row.best_robust_score != null)
    parts.push(`score=${normalizeText(row.best_robust_score)}`);
  return parts.join(" | ");
}

function readMlValidationResult(
  value: unknown,
): Record<string, unknown> | null {
  const row = asRecord(value);
  if (!row) return null;
  return asRecord(row.validation) ?? row;
}

function readMlCandidateRows(value: unknown): Record<string, unknown>[] {
  const result = readMlValidationResult(value);
  if (!result || !Array.isArray(result.candidates)) return [];
  return result.candidates.filter(
    (entry): entry is Record<string, unknown> =>
      !!entry && typeof entry === "object",
  );
}

function formatMlRunVerdictSummary(value: unknown): string {
  const result = readMlValidationResult(value);
  const verdict = asRecord(result?.run_verdict);
  if (!verdict) return "";
  const parts: string[] = [];
  if (verdict.approved_candidate_count != null) {
    parts.push(`approved=${normalizeText(verdict.approved_candidate_count)}`);
  }
  if (verdict.candidates_survived_hard_gates != null) {
    parts.push(
      `survived=${normalizeText(verdict.candidates_survived_hard_gates)}`,
    );
  }
  if (verdict.candidates_considered != null) {
    parts.push(`considered=${normalizeText(verdict.candidates_considered)}`);
  }
  return parts.join(" | ");
}

function formatMlTopCandidatesSummary(value: unknown): string {
  const candidates = readMlCandidateRows(value).slice(0, 3);
  if (!candidates.length) return "";
  return candidates
    .map((candidate, index) => {
      const params = asRecord(candidate.params);
      const label = normalizeText(params?.label) || `candidate_${index + 1}`;
      const robust =
        candidate.robust_score != null
          ? `robust=${normalizeText(candidate.robust_score)}`
          : "";
      const psr =
        candidate.psr != null ? `psr=${normalizeText(candidate.psr)}` : "";
      const dsr =
        candidate.dsr != null ? `dsr=${normalizeText(candidate.dsr)}` : "";
      const pieces = [label, robust, psr, dsr].filter((part) => part);
      return pieces.join(" | ");
    })
    .join("\n");
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
  if (longOnly != null) strategyStore.longOnly = longOnly;
  if (allowFlip != null) strategyStore.allowFlip = allowFlip;
  if (tradeOnNext != null) strategyStore.tradeOnNext = tradeOnNext;
  if (costPreset) strategyStore.costPreset = costPreset;
  if (Number.isFinite(feeBps)) strategyStore.feeBps = String(feeBps);
  if (Number.isFinite(spreadBps)) strategyStore.spreadBps = String(spreadBps);
  if (Number.isFinite(slippageBps))
    strategyStore.slippageBps = String(slippageBps);
  if (Number.isFinite(stopLossPct)) {
    strategyStore.stopLossPct = String(stopLossPct);
  }
  if (Number.isFinite(takeProfitPct)) {
    strategyStore.takeProfitPct = String(takeProfitPct);
  }
  if (Number.isFinite(cooldownBars)) {
    strategyStore.cooldownBars = String(Math.floor(cooldownBars));
  }
  if (Number.isFinite(persistenceBars)) {
    strategyStore.signalPersistenceBars = String(
      Math.max(1, Math.floor(persistenceBars)),
    );
  }
  if (Number.isFinite(epsRel)) strategyStore.epsRel = String(epsRel);
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

  const strategyKind = normalizeText(req.strategy_kind ?? req.strategyKind);
  const objective = normalizeText(req.objective);
  const mode = normalizeText(req.mode);
  const maSource = normalizeText(req.ma_source ?? req.maSource);
  const backendRaw = asRecord(req.backend);
  const backend = normalizeText(backendRaw?.backend ?? req.backend);
  const fastRange = asRecord(req.fast_range);
  const slowRange = asRecord(req.slow_range);
  const fastTypes = normalizeStringList(req.fast_ma_types);
  const slowTypes = normalizeStringList(req.slow_ma_types);

  if (strategyKind) optimizeStore.strategyKind = strategyKind;
  if (strategyKind === "MlSignal") {
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
  if (objective) {
    optimizeStore.objective = objective;
    advancedStore.objective = objective;
  }
  if (mode) {
    optimizeStore.mode = mode;
    advancedStore.mode = mode;
  }
  if (backend) optimizeStore.backend = backend;
  if (maSource) advancedStore.maSource = maSource;

  const fastStart = asNumber(fastRange?.start);
  const fastEnd = asNumber(fastRange?.end);
  const fastStep = asNumber(fastRange?.step);
  const slowStart = asNumber(slowRange?.start);
  const slowEnd = asNumber(slowRange?.end);
  const slowStep = asNumber(slowRange?.step);

  if (Number.isFinite(fastStart))
    optimizeStore.fastStart = String(Math.floor(fastStart));
  if (Number.isFinite(fastEnd))
    optimizeStore.fastEnd = String(Math.floor(fastEnd));
  if (Number.isFinite(fastStep) && fastStep > 0)
    optimizeStore.fastStep = String(Math.floor(fastStep));
  if (Number.isFinite(slowStart))
    optimizeStore.slowStart = String(Math.floor(slowStart));
  if (Number.isFinite(slowEnd))
    optimizeStore.slowEnd = String(Math.floor(slowEnd));
  if (Number.isFinite(slowStep) && slowStep > 0)
    optimizeStore.slowStep = String(Math.floor(slowStep));
  const fastMaType = fastTypes[0];
  const slowMaType = slowTypes[0];
  if (fastMaType) optimizeStore.fastMaType = fastMaType;
  if (slowMaType) optimizeStore.slowMaType = slowMaType;

  if (csvPath) dataStore.csvPath = csvPath;
  dataStore.dataTimeframe = timeframe;

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

async function loadRunArtifact(
  path: string,
): Promise<{ path: string; record: RunArtifactResponse }> {
  const nextPath = normalizePath(path);
  if (!nextPath) throw new Error("Select a saved run first.");
  const raw = (await invokeCommand("load_run_artifact", {
    path: nextPath,
  })) as RunArtifactResponse;
  return {
    path: nextPath,
    record: raw && typeof raw === "object" ? raw : ({} as RunArtifactResponse),
  };
}

function buildReqSnapshot(
  req: Record<string, unknown> | null,
  validationReq: Record<string, unknown> | null = null,
): string {
  if (!req) return "Request unavailable.";
  if (isMlSignalStrategyKind(req.strategy_kind ?? req.strategyKind)) {
    const lines: string[] = [];
    lines.push("strategy: MlSignal");
    const dataId = normalizeText(req.data_id ?? req.dataId);
    const inputPath = normalizeText(req.input_path ?? req.inputPath);
    const outputPath = normalizeText(req.output_path ?? req.outputPath);
    const scoreCachePath = normalizeText(
      req.score_cache_path ?? req.scoreCachePath,
    );
    const modelBundleId = normalizeText(
      req.model_bundle_id ?? req.modelBundleId,
    );
    if (dataId) lines.push(`data id: ${dataId}`);
    if (inputPath) lines.push(`score input: ${inputPath}`);
    if (outputPath) lines.push(`dataset/score output: ${outputPath}`);
    if (scoreCachePath) lines.push(`score cache: ${scoreCachePath}`);
    if (modelBundleId) lines.push(`model bundle: ${modelBundleId}`);
    const sweepAxis = (value: unknown): string => {
      const row =
        value && typeof value === "object"
          ? (value as Record<string, unknown>)
          : null;
      if (!row) return "";
      const start = normalizeText(row.start);
      const end = normalizeText(row.end);
      const step = normalizeText(row.step);
      if (!start && !end && !step) return "";
      return [start, end, step].filter((part) => part).join("/");
    };
    const candidates = Array.isArray(validationReq?.candidates)
      ? (validationReq?.candidates as Array<Record<string, unknown>>)
      : [];
    const firstCandidate = candidates[0];
    if (firstCandidate) {
      lines.push(
        `policy t_long: ${normalizeText(firstCandidate.t_long) || "-"}`,
      );
      lines.push(
        `policy t_short: ${normalizeText(firstCandidate.t_short) || "-"}`,
      );
      lines.push(`policy delta: ${normalizeText(firstCandidate.delta) || "-"}`);
      lines.push(
        `policy max_hold_bars: ${normalizeText(firstCandidate.max_hold_bars) || "-"}`,
      );
    } else if (validationReq) {
      const tLongAxis = sweepAxis(validationReq.t_long);
      const tShortAxis = sweepAxis(validationReq.t_short);
      const deltaAxis = sweepAxis(validationReq.delta);
      const holdAxis = sweepAxis(validationReq.max_hold_bars);
      const topK = normalizeText(validationReq.top_k);
      if (tLongAxis) lines.push(`sweep t_long: ${tLongAxis}`);
      if (tShortAxis) lines.push(`sweep t_short: ${tShortAxis}`);
      if (deltaAxis) lines.push(`sweep delta: ${deltaAxis}`);
      if (holdAxis) lines.push(`sweep hold: ${holdAxis}`);
      if (topK) lines.push(`sweep top_k: ${topK}`);
    }
    return lines.join("\n");
  }
  const backend =
    normalizeText(
      (req.backend as { backend?: unknown } | undefined)?.backend,
    ) || "CpuOnly";
  const objectives =
    req.objectives && typeof req.objectives === "object"
      ? (req.objectives as Record<string, unknown>)
      : null;
  const objective = normalizeText(req.objective ?? objectives?.maximize) || "-";
  const maSource = normalizeText(req.ma_source) || "close";
  const fastTypes = normalizeStringList(req.fast_ma_types).join(", ") || "-";
  const slowTypes = normalizeStringList(req.slow_ma_types).join(", ") || "-";
  const startingEquity = Number(req.starting_equity);

  const lines: string[] = [];
  lines.push(`strategy: ${normalizeText(req.strategy_kind) || "-"}`);
  lines.push(`backend: ${backend}`);
  lines.push(`objective: ${objective}`);
  lines.push(`ma source: ${maSource}`);
  lines.push(`fast range: ${formatRangeLabel(req.fast_range)}`);
  lines.push(`slow range: ${formatRangeLabel(req.slow_range)}`);
  lines.push(`fast ma: ${fastTypes}`);
  lines.push(`slow ma: ${slowTypes}`);
  if (Number.isFinite(startingEquity) && startingEquity > 0) {
    lines.push(`starting equity: ${startingEquity}`);
  }

  return lines.join("\n");
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
      hasValidation: !!row.has_validation,
      reportXlsxPath: normalizeText(row.report_xlsx_path),
      csvPath: normalizeText(row.csv_path),
      tags: normalizeStringList(row.tags),
    });
  }

  return { runs };
}

export async function openRunsDir(): Promise<void> {
  await invokeCommand("open_runs_dir");
}

export async function loadSavedRun(path: string): Promise<void> {
  const loaded = await loadRunArtifact(path);
  const nextPath = loaded.path;
  const record = loaded.record;
  const runRecord =
    record.run && typeof record.run === "object"
      ? (record.run as Record<string, unknown>)
      : null;
  const validationRecord =
    record.validation && typeof record.validation === "object"
      ? (record.validation as Record<string, unknown>)
      : null;
  const runReq = runRecord?.req ?? null;
  const runRes = runRecord?.res ?? null;
  if (!runReq || !runRes)
    throw new Error("Saved run artifact is missing run.req/run.res.");

  const ts = normalizeText(record.ts) || new Date().toISOString();
  const csvPath =
    normalizeText(record.csv_path ?? record.csvPath) ||
    normalizeText((record.data as { csv_path?: unknown } | null)?.csv_path);
  const reportXlsxPath = normalizeText(
    record.report_xlsx_path ?? record.reportXlsxPath,
  );
  const datasetMeta =
    record.dataset_meta && typeof record.dataset_meta === "object"
      ? (record.dataset_meta as Record<string, unknown>)
      : null;
  const mlSignal =
    record.ml_signal && typeof record.ml_signal === "object"
      ? (record.ml_signal as Record<string, unknown>)
      : null;
  const tags = normalizeStringList(record.tags);
  const validationReq = validationRecord?.req ?? null;
  const validationRes = validationRecord?.res ?? null;

  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ts,
        artifact_path: nextPath,
        report_xlsx_path: reportXlsxPath,
        csv_path: csvPath,
        dataset_meta: datasetMeta,
        ml_signal: mlSignal,
        tags,
        req: runReq,
        res: runRes,
      },
      currentValidation: validationRes
        ? {
            ts,
            req: validationReq,
            res: validationRes,
          }
        : null,
    },
    "vue",
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
  refreshAnalysisFromWorkflowRun();
}

export async function applySavedRun(path: string): Promise<void> {
  const loaded = await loadRunArtifact(path);
  const record = loaded.record;
  const runRecord = asRecord(record.run);
  const req = asRecord(runRecord?.req);
  const validationReq = asRecord(asRecord(record.validation)?.req);
  if (!req) throw new Error("Saved run artifact is missing run.req.");

  const csvPath =
    normalizeText(record.csv_path ?? record.csvPath) ||
    normalizeText(asRecord(record.data)?.csv_path);
  const timeframe = normalizeText(asRecord(record.dataset_meta)?.timeframe);
  updateStoresFromRunRequest(req, csvPath, timeframe, validationReq);
}

export async function rerunSavedRun(path: string): Promise<void> {
  const loaded = await loadRunArtifact(path);
  const record = loaded.record;
  const runRecord = asRecord(record.run);
  const req = asRecord(runRecord?.req);
  const validationReq = asRecord(asRecord(record.validation)?.req);
  const mlSignal = asRecord(record.ml_signal);
  if (!req) throw new Error("Saved run artifact is missing run.req.");

  const csvPath =
    normalizeText(record.csv_path ?? record.csvPath) ||
    normalizeText(asRecord(record.data)?.csv_path);
  if (!csvPath)
    throw new Error(
      "Saved run is missing data.csv_path. Load a dataset first, then rerun.",
    );

  const base = await loadDataCsv(csvPath);
  let currentDataId = normalizeText(base.dataId);
  const timeframe = normalizeText(asRecord(record.dataset_meta)?.timeframe);
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
  if (isMlSignalStrategyKind(req.strategy_kind ?? req.strategyKind)) {
    const {
      exportMlSignalDataset,
      generateMlSignalScoreCache,
      runMlSignalPolicySweep,
      runMlSignalValidation,
    } = await import("./mlSignalService");
    const pinia = resolveRuntimePinia();
    const isDatasetExportStage =
      normalizeText(mlSignal?.stage) === "dataset_export" ||
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
  await runOptimization({ request, switchTab: true });
}

export async function openSavedRunReport(path: string): Promise<void> {
  const detail = await loadSavedRunDetail(path);
  const reportPath = normalizePath(detail.reportPath);
  if (!reportPath)
    throw new Error(
      "No report path for this saved run yet (run validation + export XLSX).",
    );
  try {
    await invokeCommand("open_path", { path: reportPath });
  } catch {
    await invokeCommand("reveal_path", { path: reportPath });
  }
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
  const nextPath = normalizePath(path);
  if (!nextPath) throw new Error("Missing run artifact path.");

  const raw = (await invokeCommand("load_run_artifact", {
    path: nextPath,
  })) as RunArtifactResponse;
  const record =
    raw && typeof raw === "object" ? raw : ({} as RunArtifactResponse);

  const runRecord =
    record.run && typeof record.run === "object"
      ? (record.run as Record<string, unknown>)
      : null;
  const req =
    runRecord?.req && typeof runRecord.req === "object"
      ? (runRecord.req as Record<string, unknown>)
      : null;
  const datasetMeta =
    record.dataset_meta && typeof record.dataset_meta === "object"
      ? (record.dataset_meta as Record<string, unknown>)
      : null;
  const data =
    record.data && typeof record.data === "object"
      ? (record.data as Record<string, unknown>)
      : null;
  const validation =
    record.validation && typeof record.validation === "object"
      ? (record.validation as Record<string, unknown>)
      : null;
  const mlSignal =
    record.ml_signal && typeof record.ml_signal === "object"
      ? (record.ml_signal as Record<string, unknown>)
      : null;
  const validationReq =
    validation?.req && typeof validation.req === "object"
      ? (validation.req as Record<string, unknown>)
      : null;

  return {
    strategyKind: normalizeText(req?.strategy_kind ?? record.strategy_kind),
    timeframe: normalizeText(datasetMeta?.timeframe),
    csvPath: normalizeText(data?.csv_path),
    reportPath: normalizeText(record.report_xlsx_path),
    hasValidation: !!(validation && validation.res),
    tags: normalizeStringList(record.tags),
    mlBundleId: normalizeText(mlSignal?.bundle_id),
    mlScoreCachePath: normalizeText(mlSignal?.score_cache_path),
    mlScoreManifestPath: normalizeText(mlSignal?.score_manifest_path),
    mlFeatureSchemaPath: normalizeText(mlSignal?.feature_schema_path),
    mlFeatureSchemaVersion: normalizeText(mlSignal?.feature_schema_version),
    mlPolicySummary: formatMlPolicySummary(mlSignal?.policy),
    mlPolicySweepSummary: formatMlPolicySweepSummary(mlSignal?.policy_sweep),
    mlRunVerdictSummary: formatMlRunVerdictSummary(validation?.res),
    mlTopCandidatesSummary: formatMlTopCandidatesSummary(validation?.res),
    paramsText: buildReqSnapshot(req, validationReq),
  };
}
