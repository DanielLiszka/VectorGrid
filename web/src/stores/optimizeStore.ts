import { defineStore } from "pinia";
import { useWorkflowStore } from "./workflowStore";
import { resolveRuntimePinia } from "./runtimePinia";
import { mlSignalPrimaryActionForStep } from "../utils/core/workflowModel";

let optimizeServicePromise:
  | Promise<typeof import("../services/optimizeService")>
  | null = null;

function loadOptimizeService(): Promise<typeof import("../services/optimizeService")> {
  if (!optimizeServicePromise) {
    optimizeServicePromise = import("../services/optimizeService");
  }
  return optimizeServicePromise;
}

export interface OptimizeOption {
  value: string;
  label: string;
  disabled: boolean;
}

const STRATEGY_OPTIONS_DEFAULT: OptimizeOption[] = [
  { value: "DoubleMa", label: "Double MA", disabled: false },
  { value: "Pit", label: "Pressure Impulse Trend (PIT)", disabled: false },
  { value: "Set", label: "Slope + Efficiency Trend (SET)", disabled: false },
  { value: "Mts", label: "Momentum Triple Stack (MTS)", disabled: false },
  {
    value: "Orc",
    label: "Oscillator Reversion Confluence (ORC)",
    disabled: false,
  },
  { value: "Marrt", label: "MA Ribbon Regime Trend (MARRT)", disabled: false },
  {
    value: "Vfct",
    label: "Volume Flow Confirmation Trend (VFCT)",
    disabled: false,
  },
  {
    value: "Rocb",
    label: "Risk Overlay Channel Breakout (ROCB)",
    disabled: false,
  },
  {
    value: "Vcet",
    label: "VIDYA Chandelier Exit Trend (VCET)",
    disabled: false,
  },
  {
    value: "Kqsr",
    label: "KDJ QQE Stochastic Reversion (KQSR)",
    disabled: false,
  },
  {
    value: "Carm",
    label: "Coppock AO Relative Momentum (CARM)",
    disabled: false,
  },
  {
    value: "Kvdf",
    label: "Keltner VWAP Directional Flow (KVDF)",
    disabled: false,
  },
  {
    value: "Mkcs",
    label: "Market Kurtosis Compression Setup (MKCS)",
    disabled: false,
  },
  {
    value: "Cdmc",
    label: "Cycle Decycler MAMA Composite (CDMC)",
    disabled: false,
  },
  { value: "Asr", label: "Adaptive Squeeze Regime (ASR)", disabled: false },
  {
    value: "Vcc",
    label: "Volatility Compression Confirmation (VCC)",
    disabled: false,
  },
  {
    value: "Vemr",
    label: "Volatility Expansion Mean Reversion (VEMR)",
    disabled: false,
  },
  { value: "CompositeCpu", label: "Composite CPU", disabled: false },
  { value: "CompositeTrend", label: "Composite Trend", disabled: false },
  { value: "RegimeHybrid", label: "Regime Hybrid", disabled: false },
  { value: "BreakoutMomentum", label: "Breakout Momentum", disabled: false },
  { value: "CompositeMo", label: "Composite MO", disabled: false },
  { value: "DynamicDoubleMa", label: "Dynamic Double MA", disabled: false },
];

const BACKEND_OPTIONS_DEFAULT: OptimizeOption[] = [
  { value: "Auto", label: "Auto (recommended)", disabled: false },
  {
    value: "CpuOnly",
    label: "CPU only (indicators + backtest on CPU)",
    disabled: false,
  },
  {
    value: "GpuKernel",
    label: "GPU unified (kernel + staged fallback)",
    disabled: false,
  },
];

type MaParamMap = Record<string, number>;
type MaParamSweepMap = Record<string, [number, number, number]>;
interface MaParamStateInput {
  params?: unknown;
  sweeps?: unknown;
}

function normalizedOptions(values: unknown): OptimizeOption[] {
  if (!Array.isArray(values)) return [];
  const entries: OptimizeOption[] = [];
  for (const item of values) {
    if (!item || typeof item !== "object") continue;
    const typed = item as Partial<OptimizeOption>;
    const value = String(typed.value || "").trim();
    const label = String(typed.label || typed.value || "").trim();
    if (!value && !label) continue;
    entries.push({
      value: value || label,
      label: label || value,
      disabled: Boolean(typed.disabled),
    });
  }
  return entries;
}

function normalizeStrategyOptions(values: unknown): OptimizeOption[] {
  const entries = normalizedOptions(values);
  const out: OptimizeOption[] = [];
  const seen = new Set<string>();
  for (const option of entries) {
    const value = String(option.value || "").trim();
    if (!value || value === "MlSignal" || seen.has(value)) continue;
    seen.add(value);
    out.push(option);
  }
  for (const fallback of STRATEGY_OPTIONS_DEFAULT) {
    if (seen.has(fallback.value)) continue;
    out.push({ ...fallback });
  }
  return out.length > 0
    ? out
    : STRATEGY_OPTIONS_DEFAULT.map((option) => ({ ...option }));
}

function normalizeBackendOptions(values: unknown): OptimizeOption[] {
  const entries = normalizedOptions(values);
  const out: OptimizeOption[] = [];
  const seen = new Set<string>();
  for (const option of entries) {
    const value = normalizeBackendValue(option.value);
    if (!value || seen.has(value)) continue;
    seen.add(value);
    const fallbackLabel = BACKEND_OPTIONS_DEFAULT.find(
      (row) => row.value === value,
    )?.label;
    out.push({
      value,
      label: String(fallbackLabel || option.label || value).trim() || value,
      disabled: Boolean(option.disabled),
    });
  }
  for (const fallback of BACKEND_OPTIONS_DEFAULT) {
    if (seen.has(fallback.value)) continue;
    out.push({ ...fallback });
  }
  return out.length > 0
    ? out
    : BACKEND_OPTIONS_DEFAULT.map((option) => ({ ...option }));
}

function normalizeBackendValue(value: unknown): string {
  const backend = String(value || "").trim();
  if (backend === "Auto") return "Auto";
  if (backend === "CpuOnly") return "CpuOnly";
  if (backend === "GpuKernel") return "GpuKernel";
  // Legacy backends are normalized to unified GPU backend.
  if (backend === "GpuOnly" || backend === "GpuHybridKernel")
    return "GpuKernel";
  return "CpuOnly";
}

function normalizeMaParams(
  value: unknown,
  fallback: MaParamMap | null = null,
): MaParamMap | null {
  if (!value || typeof value !== "object" || Array.isArray(value))
    return fallback;
  const out: MaParamMap = {};
  for (const [key, raw] of Object.entries(value as Record<string, unknown>)) {
    const name = String(key || "").trim();
    if (!name) continue;
    const parsed = Number(raw);
    if (!Number.isFinite(parsed)) continue;
    out[name] = parsed;
  }
  return Object.keys(out).length > 0 ? out : null;
}

function normalizeMaParamSweeps(
  value: unknown,
  fallback: MaParamSweepMap | null = null,
): MaParamSweepMap | null {
  if (!value || typeof value !== "object" || Array.isArray(value))
    return fallback;
  const out: MaParamSweepMap = {};
  for (const [key, raw] of Object.entries(value as Record<string, unknown>)) {
    const name = String(key || "").trim();
    if (!name) continue;
    const row = raw as unknown;
    let start = Number.NaN;
    let end = Number.NaN;
    let step = Number.NaN;
    if (Array.isArray(row)) {
      start = Number(row[0]);
      end = Number(row[1]);
      step = Number(row[2]);
    } else if (row && typeof row === "object") {
      const typed = row as Record<string, unknown>;
      start = Number(typed.start);
      end = Number(typed.end);
      step = Number(typed.step);
    }
    if (
      !Number.isFinite(start) ||
      !Number.isFinite(end) ||
      !Number.isFinite(step) ||
      step <= 0
    )
      continue;
    out[name] = [start, end, step];
  }
  return Object.keys(out).length > 0 ? out : null;
}

function applyVueFieldUpdate(
  state: OptimizeState,
  field: string,
  value: string,
): boolean {
  const key = String(field || "").trim();
  const next = String(value || "").trim();
  if (!key) return false;
  switch (key) {
    case "strategyKind":
      state.strategyKind = next;
      return true;
    case "presetProfile":
      state.presetProfile = next || "Balanced";
      return true;
    case "backend":
      state.backend = next;
      return true;
    case "objective":
      state.objective = next;
      return true;
    case "mode":
      state.mode = next;
      return true;
    case "fastStart":
      state.fastStart = next;
      return true;
    case "fastEnd":
      state.fastEnd = next;
      return true;
    case "fastStep":
      state.fastStep = next;
      return true;
    case "slowStart":
      state.slowStart = next;
      return true;
    case "slowEnd":
      state.slowEnd = next;
      return true;
    case "slowStep":
      state.slowStep = next;
      return true;
    case "fastMaType":
      state.fastMaType = next;
      return true;
    case "slowMaType":
      state.slowMaType = next;
      return true;
    case "mlSignalDatasetExportPath":
      state.mlSignalDatasetExportPath = next;
      return true;
    case "mlSignalScoreInputPath":
      state.mlSignalScoreInputPath = next;
      return true;
    case "mlSignalScoreCachePath":
      state.mlSignalScoreCachePath = next;
      return true;
    case "mlSignalModelBundleId":
      state.mlSignalModelBundleId = next;
      return true;
    case "mlSignalModelStatus":
      state.mlSignalModelStatus = next;
      return true;
    case "mlSignalDatasetMeta":
      state.mlSignalDatasetMeta = next;
      return true;
    case "mlSignalScoreMeta":
      state.mlSignalScoreMeta = next;
      return true;
    case "mlSignalValidationMeta":
      state.mlSignalValidationMeta = next;
      return true;
    case "mlSignalSweepMeta":
      state.mlSignalSweepMeta = next;
      return true;
    case "mlSignalMaSource":
      state.mlSignalMaSource = next || "close";
      return true;
    case "mlSignalHorizonBars":
      state.mlSignalHorizonBars = next;
      return true;
    case "mlSignalSplitTrainBars":
      state.mlSignalSplitTrainBars = next;
      return true;
    case "mlSignalSplitCalibrationBars":
      state.mlSignalSplitCalibrationBars = next;
      return true;
    case "mlSignalSplitTestBars":
      state.mlSignalSplitTestBars = next;
      return true;
    case "mlSignalSplitGapBars":
      state.mlSignalSplitGapBars = next;
      return true;
    case "mlSignalSplitStepBars":
      state.mlSignalSplitStepBars = next;
      return true;
    case "mlSignalHoldoutModulus":
      state.mlSignalHoldoutModulus = next;
      return true;
    case "mlSignalHoldoutResidue":
      state.mlSignalHoldoutResidue = next;
      return true;
    case "mlSignalLongThreshold":
      state.mlSignalLongThreshold = next;
      return true;
    case "mlSignalShortThreshold":
      state.mlSignalShortThreshold = next;
      return true;
    case "mlSignalDelta":
      state.mlSignalDelta = next;
      return true;
    case "mlSignalMaxHoldBars":
      state.mlSignalMaxHoldBars = next;
      return true;
    case "mlSignalMinPsr":
      state.mlSignalMinPsr = next;
      return true;
    case "mlSignalMinDsr":
      state.mlSignalMinDsr = next;
      return true;
    case "mlSignalSweepLongStart":
      state.mlSignalSweepLongStart = next;
      return true;
    case "mlSignalSweepLongEnd":
      state.mlSignalSweepLongEnd = next;
      return true;
    case "mlSignalSweepLongStep":
      state.mlSignalSweepLongStep = next;
      return true;
    case "mlSignalSweepShortStart":
      state.mlSignalSweepShortStart = next;
      return true;
    case "mlSignalSweepShortEnd":
      state.mlSignalSweepShortEnd = next;
      return true;
    case "mlSignalSweepShortStep":
      state.mlSignalSweepShortStep = next;
      return true;
    case "mlSignalSweepDeltaStart":
      state.mlSignalSweepDeltaStart = next;
      return true;
    case "mlSignalSweepDeltaEnd":
      state.mlSignalSweepDeltaEnd = next;
      return true;
    case "mlSignalSweepDeltaStep":
      state.mlSignalSweepDeltaStep = next;
      return true;
    case "mlSignalSweepHoldStart":
      state.mlSignalSweepHoldStart = next;
      return true;
    case "mlSignalSweepHoldEnd":
      state.mlSignalSweepHoldEnd = next;
      return true;
    case "mlSignalSweepHoldStep":
      state.mlSignalSweepHoldStep = next;
      return true;
    case "mlSignalSweepTopK":
      state.mlSignalSweepTopK = next;
      return true;
    default:
      return false;
  }
}

function syncOptimizeParityFieldsToAdvancedStore(
  field: string,
  value: string,
): void {
  const pinia = resolveRuntimePinia();
  const advancedState = pinia?.state.value?.advanced as
    | Record<string, unknown>
    | undefined;
  if (!advancedState) return;

  const now = Date.now();
  if (field === "objective") {
    const next = String(value || "").trim() || "Sharpe";
    if (String(advancedState.objective ?? "").trim() === next) return;
    advancedState.objective = next;
    advancedState.source = "vue";
    advancedState.updatedAtUnixMs = now;
    return;
  }

  if (field === "mode") {
    const next = String(value || "").trim() || "Grid";
    if (String(advancedState.mode ?? "").trim() === next) return;
    advancedState.mode = next;
    advancedState.source = "vue";
    advancedState.updatedAtUnixMs = now;
  }
}

function applyVueCheckboxFieldUpdate(
  state: OptimizeState,
  field: string,
  checked: boolean,
): boolean {
  const key = String(field || "").trim();
  switch (key) {
    case "mlSignalExitOnFlat":
      state.mlSignalExitOnFlat = checked;
      return true;
    case "mlSignalExitOnOpposite":
      state.mlSignalExitOnOpposite = checked;
      return true;
    default:
      return false;
  }
}

interface OptimizeState {
  activeControlsTab: string;
  strategyKind: string;
  strategyHint: string;
  strategyOptions: OptimizeOption[];
  presetProfile: string;
  presetOptions: OptimizeOption[];
  backend: string;
  backendOptions: OptimizeOption[];
  objective: string;
  mode: string;
  fastStart: string;
  fastEnd: string;
  fastStep: string;
  slowStart: string;
  slowEnd: string;
  slowStep: string;
  fastMaType: string;
  fastMaTypeOptions: OptimizeOption[];
  slowMaType: string;
  slowMaTypeOptions: OptimizeOption[];
  fastMaParams: MaParamMap | null;
  slowMaParams: MaParamMap | null;
  fastMaParamSweeps: MaParamSweepMap | null;
  slowMaParamSweeps: MaParamSweepMap | null;
  pairsPreview: string;
  gpuStatus: string;
  gpuDeviceMeta: string;
  gpuParityMeta: string;
  progressMeta: string;
  runPlanMeta: string;
  runReadinessHint: string;
  runReadinessBadges: string[];
  dynamicStrategyIrJson: string;
  dynamicStrategyBuilderStateJson: string;
  dynamicStrategyPreviewMeta: string;
  dynamicStrategyPreviewReasons: string;
  dynamicStrategyPreviewPrettyJson: string;
  mlSignalDatasetExportPath: string;
  mlSignalScoreInputPath: string;
  mlSignalScoreCachePath: string;
  mlSignalModelBundleId: string;
  mlSignalModelStatus: string;
  mlSignalDatasetMeta: string;
  mlSignalScoreMeta: string;
  mlSignalValidationMeta: string;
  mlSignalSweepMeta: string;
  mlSignalMaSource: string;
  mlSignalHorizonBars: string;
  mlSignalSplitTrainBars: string;
  mlSignalSplitCalibrationBars: string;
  mlSignalSplitTestBars: string;
  mlSignalSplitGapBars: string;
  mlSignalSplitStepBars: string;
  mlSignalHoldoutModulus: string;
  mlSignalHoldoutResidue: string;
  mlSignalLongThreshold: string;
  mlSignalShortThreshold: string;
  mlSignalDelta: string;
  mlSignalMaxHoldBars: string;
  mlSignalMinPsr: string;
  mlSignalMinDsr: string;
  mlSignalSweepLongStart: string;
  mlSignalSweepLongEnd: string;
  mlSignalSweepLongStep: string;
  mlSignalSweepShortStart: string;
  mlSignalSweepShortEnd: string;
  mlSignalSweepShortStep: string;
  mlSignalSweepDeltaStart: string;
  mlSignalSweepDeltaEnd: string;
  mlSignalSweepDeltaStep: string;
  mlSignalSweepHoldStart: string;
  mlSignalSweepHoldEnd: string;
  mlSignalSweepHoldStep: string;
  mlSignalSweepTopK: string;
  mlSignalExitOnFlat: boolean;
  mlSignalExitOnOpposite: boolean;
  statusLine: string;
  runDisabled: boolean;
  canCancel: boolean;
  validateDisabled: boolean;
  canApplyPreset: boolean;
  canPresetKernel: boolean;
  canRefreshGpuStatus: boolean;
  canParityCheck: boolean;
  source: "vue" | "unknown";
  updatedAtUnixMs: number;
}

export const useOptimizeStore = defineStore("optimize", {
  state: (): OptimizeState => ({
    activeControlsTab: "data",
    strategyKind: "DoubleMa",
    strategyHint: "Double MA crossover strategy.",
    strategyOptions: STRATEGY_OPTIONS_DEFAULT.map((option) => ({ ...option })),
    presetProfile: "Balanced",
    presetOptions: [
      { value: "Quick", label: "Quick", disabled: false },
      { value: "Balanced", label: "Balanced", disabled: false },
      { value: "Deep", label: "Deep", disabled: false },
    ],
    backend: "Auto",
    backendOptions: BACKEND_OPTIONS_DEFAULT.map((option) => ({ ...option })),
    objective: "Sharpe",
    mode: "Grid",
    fastStart: "5",
    fastEnd: "20",
    fastStep: "1",
    slowStart: "20",
    slowEnd: "80",
    slowStep: "1",
    fastMaType: "sma",
    fastMaTypeOptions: [
      { value: "sma", label: "sma", disabled: false },
      { value: "alma", label: "alma", disabled: false },
    ],
    slowMaType: "alma",
    slowMaTypeOptions: [
      { value: "sma", label: "sma", disabled: false },
      { value: "alma", label: "alma", disabled: false },
    ],
    fastMaParams: null,
    slowMaParams: null,
    fastMaParamSweeps: null,
    slowMaParamSweeps: null,
    pairsPreview: "Pairs: —",
    gpuStatus: "GPU: —",
    gpuDeviceMeta: "CUDA device selection is automatic.",
    gpuParityMeta: "Parity: —",
    progressMeta: "",
    runPlanMeta: "Plan: —",
    runReadinessHint: "Readiness: Load data first.",
    runReadinessBadges: [],
    dynamicStrategyIrJson: "",
    dynamicStrategyBuilderStateJson: "",
    dynamicStrategyPreviewMeta: "Dynamic strategy preview: —",
    dynamicStrategyPreviewReasons: "Reasons: —",
    dynamicStrategyPreviewPrettyJson: "",
    mlSignalDatasetExportPath: "",
    mlSignalScoreInputPath: "",
    mlSignalScoreCachePath: "",
    mlSignalModelBundleId: "",
    mlSignalModelStatus: "Model bundle: —",
    mlSignalDatasetMeta: "Dataset export: —",
    mlSignalScoreMeta: "Score cache: —",
    mlSignalValidationMeta: "ML validation: —",
    mlSignalSweepMeta: "ML policy sweep: —",
    mlSignalMaSource: "close",
    mlSignalHorizonBars: "10",
    mlSignalSplitTrainBars: "",
    mlSignalSplitCalibrationBars: "",
    mlSignalSplitTestBars: "",
    mlSignalSplitGapBars: "10",
    mlSignalSplitStepBars: "63",
    mlSignalHoldoutModulus: "",
    mlSignalHoldoutResidue: "0",
    mlSignalLongThreshold: "0.55",
    mlSignalShortThreshold: "0.55",
    mlSignalDelta: "0.05",
    mlSignalMaxHoldBars: "10",
    mlSignalMinPsr: "",
    mlSignalMinDsr: "",
    mlSignalSweepLongStart: "0.50",
    mlSignalSweepLongEnd: "0.70",
    mlSignalSweepLongStep: "0.05",
    mlSignalSweepShortStart: "0.50",
    mlSignalSweepShortEnd: "0.70",
    mlSignalSweepShortStep: "0.05",
    mlSignalSweepDeltaStart: "0.00",
    mlSignalSweepDeltaEnd: "0.10",
    mlSignalSweepDeltaStep: "0.05",
    mlSignalSweepHoldStart: "5",
    mlSignalSweepHoldEnd: "15",
    mlSignalSweepHoldStep: "5",
    mlSignalSweepTopK: "10",
    mlSignalExitOnFlat: false,
    mlSignalExitOnOpposite: true,
    statusLine: "Ready.",
    runDisabled: false,
    canCancel: false,
    validateDisabled: false,
    canApplyPreset: true,
    canPresetKernel: true,
    canRefreshGpuStatus: true,
    canParityCheck: true,
    source: "unknown",
    updatedAtUnixMs: 0,
  }),

  actions: {
    applyLegacySnapshot(snapshot: Partial<OptimizeState>): void {
      this.activeControlsTab =
        String(
          snapshot.activeControlsTab ?? this.activeControlsTab ?? "data",
        ).trim() || "data";
      this.strategyKind = String(
        snapshot.strategyKind ?? this.strategyKind ?? "",
      ).trim();
      this.strategyHint =
        String(snapshot.strategyHint ?? this.strategyHint ?? "—").trim() || "—";
      this.strategyOptions = normalizeStrategyOptions(
        snapshot.strategyOptions ?? this.strategyOptions ?? [],
      );
      this.presetProfile =
        String(
          snapshot.presetProfile ?? this.presetProfile ?? "Balanced",
        ).trim() || "Balanced";
      this.presetOptions = normalizedOptions(
        snapshot.presetOptions ?? this.presetOptions ?? [],
      );
      this.backend = normalizeBackendValue(
        snapshot.backend ?? this.backend ?? "",
      );
      this.backendOptions = normalizeBackendOptions(
        snapshot.backendOptions ?? this.backendOptions ?? [],
      );
      this.objective = String(
        snapshot.objective ?? this.objective ?? "",
      ).trim();
      this.mode = String(snapshot.mode ?? this.mode ?? "").trim();
      this.fastStart = String(
        snapshot.fastStart ?? this.fastStart ?? "",
      ).trim();
      this.fastEnd = String(snapshot.fastEnd ?? this.fastEnd ?? "").trim();
      this.fastStep = String(snapshot.fastStep ?? this.fastStep ?? "").trim();
      this.slowStart = String(
        snapshot.slowStart ?? this.slowStart ?? "",
      ).trim();
      this.slowEnd = String(snapshot.slowEnd ?? this.slowEnd ?? "").trim();
      this.slowStep = String(snapshot.slowStep ?? this.slowStep ?? "").trim();
      this.fastMaType = String(
        snapshot.fastMaType ?? this.fastMaType ?? "",
      ).trim();
      this.fastMaTypeOptions = normalizedOptions(
        snapshot.fastMaTypeOptions ?? this.fastMaTypeOptions ?? [],
      );
      this.slowMaType = String(
        snapshot.slowMaType ?? this.slowMaType ?? "",
      ).trim();
      this.slowMaTypeOptions = normalizedOptions(
        snapshot.slowMaTypeOptions ?? this.slowMaTypeOptions ?? [],
      );
      this.fastMaParams = normalizeMaParams(
        snapshot.fastMaParams,
        this.fastMaParams,
      );
      this.slowMaParams = normalizeMaParams(
        snapshot.slowMaParams,
        this.slowMaParams,
      );
      this.fastMaParamSweeps = normalizeMaParamSweeps(
        snapshot.fastMaParamSweeps,
        this.fastMaParamSweeps,
      );
      this.slowMaParamSweeps = normalizeMaParamSweeps(
        snapshot.slowMaParamSweeps,
        this.slowMaParamSweeps,
      );
      this.pairsPreview =
        String(
          snapshot.pairsPreview ?? this.pairsPreview ?? "Pairs: —",
        ).trim() || "Pairs: —";
      this.gpuStatus =
        String(snapshot.gpuStatus ?? this.gpuStatus ?? "GPU: —").trim() ||
        "GPU: —";
      this.gpuDeviceMeta =
        String(
          snapshot.gpuDeviceMeta ??
            this.gpuDeviceMeta ??
            "CUDA device selection is automatic.",
        ).trim() || "CUDA device selection is automatic.";
      this.gpuParityMeta =
        String(
          snapshot.gpuParityMeta ?? this.gpuParityMeta ?? "Parity: —",
        ).trim() || "Parity: —";
      this.progressMeta = String(
        snapshot.progressMeta ?? this.progressMeta ?? "",
      ).trim();
      this.runPlanMeta =
        String(snapshot.runPlanMeta ?? this.runPlanMeta ?? "Plan: —").trim() ||
        "Plan: —";
      this.runReadinessHint =
        String(
          snapshot.runReadinessHint ?? this.runReadinessHint ?? "Readiness: —",
        ).trim() || "Readiness: —";
      this.runReadinessBadges = Array.isArray(snapshot.runReadinessBadges)
        ? snapshot.runReadinessBadges
            .map((badge) => String(badge || "").trim())
            .filter((badge) => badge.length > 0)
        : this.runReadinessBadges;
      this.dynamicStrategyIrJson = String(
        snapshot.dynamicStrategyIrJson ?? this.dynamicStrategyIrJson ?? "",
      );
      this.dynamicStrategyBuilderStateJson = String(
        snapshot.dynamicStrategyBuilderStateJson ??
          this.dynamicStrategyBuilderStateJson ??
          "",
      );
      this.dynamicStrategyPreviewMeta =
        String(
          snapshot.dynamicStrategyPreviewMeta ??
            this.dynamicStrategyPreviewMeta ??
            "Dynamic strategy preview: —",
        ).trim() || "Dynamic strategy preview: —";
      this.dynamicStrategyPreviewReasons =
        String(
          snapshot.dynamicStrategyPreviewReasons ??
            this.dynamicStrategyPreviewReasons ??
            "Reasons: —",
        ).trim() || "Reasons: —";
      this.dynamicStrategyPreviewPrettyJson = String(
        snapshot.dynamicStrategyPreviewPrettyJson ??
          this.dynamicStrategyPreviewPrettyJson ??
          "",
      );
      this.mlSignalDatasetExportPath = String(
        snapshot.mlSignalDatasetExportPath ??
          this.mlSignalDatasetExportPath ??
          "",
      );
      this.mlSignalScoreInputPath = String(
        snapshot.mlSignalScoreInputPath ?? this.mlSignalScoreInputPath ?? "",
      );
      this.mlSignalScoreCachePath = String(
        snapshot.mlSignalScoreCachePath ?? this.mlSignalScoreCachePath ?? "",
      );
      this.mlSignalModelBundleId = String(
        snapshot.mlSignalModelBundleId ?? this.mlSignalModelBundleId ?? "",
      );
      this.mlSignalModelStatus =
        String(
          snapshot.mlSignalModelStatus ??
            this.mlSignalModelStatus ??
            "Model bundle: —",
        ).trim() || "Model bundle: —";
      this.mlSignalDatasetMeta =
        String(
          snapshot.mlSignalDatasetMeta ??
            this.mlSignalDatasetMeta ??
            "Dataset export: —",
        ).trim() || "Dataset export: —";
      this.mlSignalScoreMeta =
        String(
          snapshot.mlSignalScoreMeta ??
            this.mlSignalScoreMeta ??
            "Score cache: —",
        ).trim() || "Score cache: —";
      this.mlSignalValidationMeta =
        String(
          snapshot.mlSignalValidationMeta ??
            this.mlSignalValidationMeta ??
            "ML validation: —",
        ).trim() || "ML validation: —";
      this.mlSignalSweepMeta =
        String(
          snapshot.mlSignalSweepMeta ??
            this.mlSignalSweepMeta ??
            "ML policy sweep: —",
        ).trim() || "ML policy sweep: —";
      this.mlSignalMaSource =
        String(
          snapshot.mlSignalMaSource ?? this.mlSignalMaSource ?? "close",
        ).trim() || "close";
      this.mlSignalHorizonBars =
        String(
          snapshot.mlSignalHorizonBars ?? this.mlSignalHorizonBars ?? "10",
        ).trim() || "10";
      this.mlSignalSplitTrainBars = String(
        snapshot.mlSignalSplitTrainBars ?? this.mlSignalSplitTrainBars ?? "",
      ).trim();
      this.mlSignalSplitCalibrationBars = String(
        snapshot.mlSignalSplitCalibrationBars ??
          this.mlSignalSplitCalibrationBars ??
          "",
      ).trim();
      this.mlSignalSplitTestBars = String(
        snapshot.mlSignalSplitTestBars ?? this.mlSignalSplitTestBars ?? "",
      ).trim();
      this.mlSignalSplitGapBars =
        String(
          snapshot.mlSignalSplitGapBars ?? this.mlSignalSplitGapBars ?? "10",
        ).trim() || "10";
      this.mlSignalSplitStepBars =
        String(
          snapshot.mlSignalSplitStepBars ?? this.mlSignalSplitStepBars ?? "63",
        ).trim() || "63";
      this.mlSignalHoldoutModulus = String(
        snapshot.mlSignalHoldoutModulus ?? this.mlSignalHoldoutModulus ?? "",
      ).trim();
      this.mlSignalHoldoutResidue =
        String(
          snapshot.mlSignalHoldoutResidue ?? this.mlSignalHoldoutResidue ?? "0",
        ).trim() || "0";
      this.mlSignalLongThreshold =
        String(
          snapshot.mlSignalLongThreshold ??
            this.mlSignalLongThreshold ??
            "0.55",
        ).trim() || "0.55";
      this.mlSignalShortThreshold =
        String(
          snapshot.mlSignalShortThreshold ??
            this.mlSignalShortThreshold ??
            "0.55",
        ).trim() || "0.55";
      this.mlSignalDelta =
        String(snapshot.mlSignalDelta ?? this.mlSignalDelta ?? "0.05").trim() ||
        "0.05";
      this.mlSignalMaxHoldBars =
        String(
          snapshot.mlSignalMaxHoldBars ?? this.mlSignalMaxHoldBars ?? "10",
        ).trim() || "10";
      this.mlSignalMinPsr = String(
        snapshot.mlSignalMinPsr ?? this.mlSignalMinPsr ?? "",
      ).trim();
      this.mlSignalMinDsr = String(
        snapshot.mlSignalMinDsr ?? this.mlSignalMinDsr ?? "",
      ).trim();
      this.mlSignalSweepLongStart =
        String(
          snapshot.mlSignalSweepLongStart ??
            this.mlSignalSweepLongStart ??
            "0.50",
        ).trim() || "0.50";
      this.mlSignalSweepLongEnd =
        String(
          snapshot.mlSignalSweepLongEnd ?? this.mlSignalSweepLongEnd ?? "0.70",
        ).trim() || "0.70";
      this.mlSignalSweepLongStep =
        String(
          snapshot.mlSignalSweepLongStep ??
            this.mlSignalSweepLongStep ??
            "0.05",
        ).trim() || "0.05";
      this.mlSignalSweepShortStart =
        String(
          snapshot.mlSignalSweepShortStart ??
            this.mlSignalSweepShortStart ??
            "0.50",
        ).trim() || "0.50";
      this.mlSignalSweepShortEnd =
        String(
          snapshot.mlSignalSweepShortEnd ??
            this.mlSignalSweepShortEnd ??
            "0.70",
        ).trim() || "0.70";
      this.mlSignalSweepShortStep =
        String(
          snapshot.mlSignalSweepShortStep ??
            this.mlSignalSweepShortStep ??
            "0.05",
        ).trim() || "0.05";
      this.mlSignalSweepDeltaStart =
        String(
          snapshot.mlSignalSweepDeltaStart ??
            this.mlSignalSweepDeltaStart ??
            "0.00",
        ).trim() || "0.00";
      this.mlSignalSweepDeltaEnd =
        String(
          snapshot.mlSignalSweepDeltaEnd ??
            this.mlSignalSweepDeltaEnd ??
            "0.10",
        ).trim() || "0.10";
      this.mlSignalSweepDeltaStep =
        String(
          snapshot.mlSignalSweepDeltaStep ??
            this.mlSignalSweepDeltaStep ??
            "0.05",
        ).trim() || "0.05";
      this.mlSignalSweepHoldStart =
        String(
          snapshot.mlSignalSweepHoldStart ?? this.mlSignalSweepHoldStart ?? "5",
        ).trim() || "5";
      this.mlSignalSweepHoldEnd =
        String(
          snapshot.mlSignalSweepHoldEnd ?? this.mlSignalSweepHoldEnd ?? "15",
        ).trim() || "15";
      this.mlSignalSweepHoldStep =
        String(
          snapshot.mlSignalSweepHoldStep ?? this.mlSignalSweepHoldStep ?? "5",
        ).trim() || "5";
      this.mlSignalSweepTopK =
        String(
          snapshot.mlSignalSweepTopK ?? this.mlSignalSweepTopK ?? "10",
        ).trim() || "10";
      this.mlSignalExitOnFlat = Boolean(
        snapshot.mlSignalExitOnFlat ?? this.mlSignalExitOnFlat,
      );
      this.mlSignalExitOnOpposite = Boolean(
        snapshot.mlSignalExitOnOpposite ?? this.mlSignalExitOnOpposite,
      );
      this.statusLine =
        String(snapshot.statusLine ?? this.statusLine ?? "Ready.").trim() ||
        "Ready.";
      this.runDisabled = Boolean(snapshot.runDisabled ?? this.runDisabled);
      this.canCancel = Boolean(snapshot.canCancel ?? this.canCancel);
      this.validateDisabled = Boolean(
        snapshot.validateDisabled ?? this.validateDisabled,
      );
      this.canApplyPreset = Boolean(
        snapshot.canApplyPreset ?? this.canApplyPreset,
      );
      this.canPresetKernel = Boolean(
        snapshot.canPresetKernel ?? this.canPresetKernel,
      );
      this.canRefreshGpuStatus = Boolean(
        snapshot.canRefreshGpuStatus ?? this.canRefreshGpuStatus,
      );
      this.canParityCheck = Boolean(
        snapshot.canParityCheck ?? this.canParityCheck,
      );
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();
      syncOptimizeParityFieldsToAdvancedStore("objective", this.objective);
      syncOptimizeParityFieldsToAdvancedStore("mode", this.mode);
    },

    setInputField(field: string, value: string): void {
      const key = String(field || "").trim();
      const next = String(value || "");
      if (applyVueFieldUpdate(this, key, next)) {
        this.source = "vue";
        this.updatedAtUnixMs = Date.now();
      }
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || "").trim();
      const next = String(value || "");
      if (applyVueFieldUpdate(this, key, next)) {
        this.source = "vue";
        this.updatedAtUnixMs = Date.now();
        if (key === "objective" || key === "mode") {
          syncOptimizeParityFieldsToAdvancedStore(key, next);
        }
      }
    },

    setCheckboxField(field: string, checked: boolean): void {
      const key = String(field || "").trim();
      const next = !!checked;
      if (applyVueCheckboxFieldUpdate(this, key, next)) {
        this.source = "vue";
        this.updatedAtUnixMs = Date.now();
      }
    },

    setDynamicStrategyIrJson(value: string): void {
      this.dynamicStrategyIrJson = String(value ?? "");
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();
    },

    setDynamicStrategyBuilderStateJson(value: string): void {
      this.dynamicStrategyBuilderStateJson = String(value ?? "");
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();
    },

    syncMaParamState(state: {
      fast?: MaParamStateInput | null;
      slow?: MaParamStateInput | null;
    }): void {
      const fastState = state?.fast ?? null;
      const slowState = state?.slow ?? null;
      this.fastMaParams = normalizeMaParams(fastState?.params, null);
      this.slowMaParams = normalizeMaParams(slowState?.params, null);
      this.fastMaParamSweeps = normalizeMaParamSweeps(fastState?.sweeps, null);
      this.slowMaParamSweeps = normalizeMaParamSweeps(slowState?.sweeps, null);
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();
    },

    applyPreset(): void {
      void loadOptimizeService().then((service) => service.applyPreset());
    },

    presetKernel(): void {
      void loadOptimizeService().then((service) => service.presetKernel());
    },

    refreshGpuStatus(quiet = false): void {
      void loadOptimizeService()
        .then((service) => service.refreshGpuStatus())
        .then((summary) => {
          this.gpuStatus =
            String(summary?.text || this.gpuStatus || "GPU: —").trim() ||
            "GPU: —";
          this.gpuDeviceMeta =
            String(
              summary?.deviceMeta ||
                this.gpuDeviceMeta ||
                "CUDA device selection is automatic.",
            ).trim() || "CUDA device selection is automatic.";
          if (!quiet) {
            this.statusLine = "GPU status refreshed.";
          }
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        })
        .catch((error: unknown) => {
          if (!quiet) {
            this.statusLine =
              `GPU status refresh failed: ${String(error || "")}`.trim();
          }
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    runGpuParityCheck(): void {
      void loadOptimizeService().then((service) => service.runGpuParityCheck());
    },

    previewDynamicStrategyIr(): void {
      void loadOptimizeService().then((service) =>
        service.previewDynamicStrategyIr(),
      );
    },

    runOptimization(): void {
      const pinia = resolveRuntimePinia();
      if (pinia && String(useWorkflowStore(pinia).mode || "").trim() === "ml-signal") {
        const workflowStep = String(useWorkflowStore(pinia).workflowStep || "").trim();
        const action = mlSignalPrimaryActionForStep(workflowStep);
        void import("../services/mlSignalService").then(
          ({
            exportMlSignalDataset,
            generateMlSignalScoreCache,
            runMlSignalPolicySweep,
            runMlSignalValidation,
          }) => {
            switch (action.kind) {
              case "dataset_export":
                return exportMlSignalDataset();
              case "score_cache":
                return generateMlSignalScoreCache();
              case "validation":
                return runMlSignalValidation();
              case "policy_sweep":
                return runMlSignalPolicySweep();
              case "artifacts": {
                this.statusLine =
                  "Current ML artifacts are inspect-only from the Artifacts step.";
                this.source = "vue";
                this.updatedAtUnixMs = Date.now();
                return Promise.resolve();
              }
            }
          },
        );
        return;
      }
      void loadOptimizeService().then((service) => service.runOptimization());
    },

    cancelOptimization(): void {
      this.statusLine = "Cancel requested...";
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();
      void loadOptimizeService()
        .then((service) => service.cancelOptimization())
        .catch((error: unknown) => {
          this.statusLine = `Cancel failed: ${String(error || "")}`.trim();
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    runValidation(): void {
      const pinia = resolveRuntimePinia();
      if (pinia && String(useWorkflowStore(pinia).mode || "").trim() === "ml-signal") {
        void import("../services/mlSignalService").then(
          ({ runMlSignalValidation }) => runMlSignalValidation(),
        );
        return;
      }
      void loadOptimizeService().then((service) => service.runValidation());
    },
  },
});
