<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  shallowRef,
  watch,
} from "vue";
import { storeToRefs } from "pinia";
import { useShellMount } from "../composables/useShellMount";
import { useOptimizeStore, type OptimizeOption } from "../stores/optimizeStore";
import { useStrategyStore } from "../stores/strategyStore";
import { useWorkflowStore } from "../stores/workflowStore";
import { useQueuedInputWriter } from "../composables/useQueuedInputWriter";
import {
  listMovingAverageCatalog,
  type MaParamDescriptor,
  type MaParamKind,
  type MovingAverageCatalogEntry,
} from "../services/movingAverageCatalogService";
import {
  listDynamicIndicatorCatalog,
  type DynamicIndicatorCatalogEntry,
  type DynamicIndicatorParamDescriptor,
  type DynamicIndicatorParamValue,
} from "../services/dynamicIndicatorCatalogService";
import {
  listGpuSpeedupMap,
  type GpuSpeedupEntry,
} from "../services/gpuSpeedupsService";
import { tauriRuntimeAvailable } from "../tauri/invoke";
import { describeStrategyOptimizationCapabilities } from "../utils/strategyOptimizationCapabilities";
import { formatUiTimeFromUnixMs } from "../utils/uiTime";
import {
  builderEligibleDynamicIndicators,
  createDefaultDynamicDoubleMaBuilderState,
  formatDynamicDoubleMaBuilderIrJson,
  preferredFloatingOutputId,
  defaultIndicatorParams,
  setDynamicDoubleMaBuilderLongOnly,
  type DynamicBuilderCondition,
  type DynamicBuilderConditionOp,
  type DynamicBuilderGroup,
  type DynamicBuilderOperand,
  type DynamicDoubleMaBuilderState,
} from "../strategies/dynamicDoubleMaBuilder";
import {
  isCompositeProfileKind,
  normalizeCompositeRequestedBackend,
  resolveCompositeProfileRoute,
} from "../strategies/compositeProfileGpuPolicy";

type MaSide = "fast" | "slow";
interface MaParamUiState {
  value: string;
  sweepEnabled: boolean;
  sweepStart: string;
  sweepEnd: string;
  sweepStep: string;
}
type MaParamUiStateByKey = Record<string, MaParamUiState>;
type MaParamUiStateByMa = Record<string, MaParamUiStateByKey>;
interface RenderedMaParam {
  schema: MaParamDescriptor;
  state: MaParamUiState;
}
interface BuilderOption {
  value: string;
  label: string;
}

const optimizeStore = useOptimizeStore();
const strategyStore = useStrategyStore();
const workflowStore = useWorkflowStore();
const {
  strategyKind,
  strategyHint,
  strategyOptions,
  presetProfile,
  presetOptions,
  backend,
  backendOptions,
  objective,
  mode,
  fastStart,
  fastEnd,
  fastStep,
  slowStart,
  slowEnd,
  slowStep,
  fastMaType,
  fastMaTypeOptions,
  slowMaType,
  slowMaTypeOptions,
  pairsPreview,
  gpuStatus,
  gpuDeviceMeta,
  gpuParityMeta,
  progressMeta,
  dynamicStrategyIrJson,
  dynamicStrategyBuilderStateJson,
  dynamicStrategyPreviewMeta,
  dynamicStrategyPreviewReasons,
  dynamicStrategyPreviewPrettyJson,
  statusLine,
  runDisabled,
  validateDisabled,
  canApplyPreset,
  canPresetKernel,
  canRefreshGpuStatus,
  canParityCheck,
  updatedAtUnixMs,
} = storeToRefs(optimizeStore);
const { controlsTab } = storeToRefs(workflowStore);

const { mountReady } = useShellMount("#optVueShellMount", {
  paneSelector: '[data-vue-panel="opt"]',
  paneDatasetKey: "vueOpt",
});
let panelDisposed = false;

const strategyDraft = ref("");
const presetDraft = ref("");
const backendDraft = ref("");
const objectiveDraft = ref("Sharpe");
const modeDraft = ref("Grid");
const dynamicStrategyIrJsonDraft = ref("");
const fastStartDraft = ref("");
const fastEndDraft = ref("");
const fastStepDraft = ref("");
const slowStartDraft = ref("");
const slowEndDraft = ref("");
const slowStepDraft = ref("");
const fastTypeDraft = ref("");
const slowTypeDraft = ref("");
const maParamCatalogLoaded = ref(false);
const maParamSchemasByMaId = shallowRef<Record<string, MaParamDescriptor[]>>(
  {},
);
const maCatalogEntries = shallowRef<MovingAverageCatalogEntry[]>([]);
const dynamicCatalogEntries = shallowRef<DynamicIndicatorCatalogEntry[]>([]);
const dynamicBuilderState = ref<DynamicDoubleMaBuilderState>(
  createDefaultDynamicDoubleMaBuilderState([]),
);
const dynamicBuilderSeeded = ref(false);
const maSpeedupById = shallowRef<Record<string, GpuSpeedupEntry>>({});
const maSpeedupLoaded = ref(false);
const maParamUiStateBySide = ref<{
  fast: MaParamUiStateByMa;
  slow: MaParamUiStateByMa;
}>({ fast: {}, slow: {} });
const maParamTouchedBySide = ref<{
  fast: Record<string, boolean>;
  slow: Record<string, boolean>;
}>({ fast: {}, slow: {} });

const visible = computed(
  () => String(controlsTab.value || "").trim() === "opt",
);
const hasTauriRuntime = computed(() => tauriRuntimeAvailable());
const strategyCapabilities = computed(() =>
  describeStrategyOptimizationCapabilities(
    strategyDraft.value || strategyKind.value,
    {
      requestedBackend: backendDraft.value || backend.value,
      trendUseAdx: strategyStore.trendUseAdx,
      breakoutUseAdx: strategyStore.breakoutUseAdx,
    },
  ),
);
const compositeRoutePreview = computed(() => {
  const kind = String(strategyDraft.value || strategyKind.value || "").trim();
  if (!isCompositeProfileKind(kind)) return null;
  return resolveCompositeProfileRoute(
    kind,
    normalizeCompositeRequestedBackend(backendDraft.value || backend.value),
    {
      trendUseAdx: strategyStore.trendUseAdx,
      breakoutUseAdx: strategyStore.breakoutUseAdx,
    },
  );
});
const isDynamicDoubleMaKind = computed(
  () =>
    String(strategyDraft.value || strategyKind.value || "").trim() ===
    "DynamicDoubleMa",
);
const strategyHintText = computed(() =>
  isDynamicDoubleMaKind.value
    ? "Constrained builder + JSON preview for dynamic Double-MA strategy_ir payloads."
    : String(strategyHint.value || "").trim() || "—",
);
const selectedFastMaType = computed(() =>
  String(fastTypeDraft.value || fastMaType.value || "").trim(),
);
const selectedSlowMaType = computed(() =>
  String(slowTypeDraft.value || slowMaType.value || "").trim(),
);
const fastMaParamSchema = computed(
  () => maParamSchemasByMaId.value[selectedFastMaType.value] || [],
);
const slowMaParamSchema = computed(
  () => maParamSchemasByMaId.value[selectedSlowMaType.value] || [],
);
const fastRenderedMaParams = computed<RenderedMaParam[]>(() =>
  fastMaParamSchema.value.map((schema) => ({
    schema,
    state: resolveMaParamUiState("fast", selectedFastMaType.value, schema),
  })),
);
const slowRenderedMaParams = computed<RenderedMaParam[]>(() =>
  slowMaParamSchema.value.map((schema) => ({
    schema,
    state: resolveMaParamUiState("slow", selectedSlowMaType.value, schema),
  })),
);
const dynamicBuilderEligibleEntries = computed(() =>
  builderEligibleDynamicIndicators(dynamicCatalogEntries.value),
);
const dynamicBuilderIndicatorOperandOptions = computed<BuilderOption[]>(() => {
  const options: BuilderOption[] = [
    { value: "fast_ma", label: "Fast MA" },
    { value: "slow_ma", label: "Slow MA" },
  ];
  for (const slot of dynamicBuilderState.value.indicators) {
    const indicatorLabel =
      dynamicCatalogEntries.value.find((entry) => entry.id === slot.indicatorId)
        ?.label ||
      slot.indicatorId ||
      slot.id;
    options.push({ value: slot.id, label: `${slot.id} • ${indicatorLabel}` });
  }
  options.push({ value: "close", label: "Close" });
  options.push({ value: "ma_source", label: "MA source" });
  options.push({ value: "__constant__", label: "Constant…" });
  return options;
});
const dynamicBuilderConditionOptions = computed<BuilderOption[]>(() =>
  dynamicBuilderState.value.conditions.map((condition) => ({
    value: condition.id,
    label: condition.id,
  })),
);
const dynamicBuilderGroupOptions = computed<BuilderOption[]>(() =>
  dynamicBuilderState.value.groups.map((group) => ({
    value: group.id,
    label: group.id,
  })),
);
const dynamicBuilderPrettyJson = computed(() => {
  try {
    return formatDynamicDoubleMaBuilderIrJson({
      fastMaType: selectedFastMaType.value,
      slowMaType: selectedSlowMaType.value,
      fastMaParams: optimizeStore.fastMaParams,
      slowMaParams: optimizeStore.slowMaParams,
      builder: dynamicBuilderState.value,
    });
  } catch {
    return "";
  }
});
const dynamicBuilderError = computed(() => {
  try {
    formatDynamicDoubleMaBuilderIrJson({
      fastMaType: selectedFastMaType.value,
      slowMaType: selectedSlowMaType.value,
      fastMaParams: optimizeStore.fastMaParams,
      slowMaParams: optimizeStore.slowMaParams,
      builder: dynamicBuilderState.value,
    });
    return "";
  } catch (error) {
    return String(error || "").trim();
  }
});
const dynamicPreviewAvailable = computed(() => hasTauriRuntime.value);
const dynamicPreviewDisabledReason = computed(() => {
  if (!dynamicPreviewAvailable.value) {
    return "Dynamic preview commands require the desktop shell. Draft the builder here, then preview from the Tauri app.";
  }
  return "";
});

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value));

function resolvedOptions(
  options: OptimizeOption[],
  fallbackValue: string,
  fallbackLabel: string,
  fallbackOptions: OptimizeOption[] = [],
): OptimizeOption[] {
  if (options.length > 0) return options;
  if (fallbackOptions.length > 0) return fallbackOptions;
  const value = String(fallbackValue || "").trim();
  if (!value) return [];
  return [
    {
      value,
      label: String(fallbackLabel || value).trim() || value,
      disabled: false,
    },
  ];
}

type BackendKind = "Auto" | "CpuOnly" | "GpuKernel";

function normalizeBackendKind(value: string): BackendKind {
  const next = String(value || "").trim();
  if (next === "CpuOnly") return "CpuOnly";
  if (next === "GpuOnly") return "GpuKernel";
  if (next === "GpuHybridKernel") return "GpuKernel";
  if (next === "GpuKernel") return "GpuKernel";
  return "Auto";
}

function backendLabel(backendKind: BackendKind): string {
  if (backendKind === "CpuOnly") return "CPU";
  if (backendKind === "GpuKernel") return "GPU unified";
  return "Auto";
}

function backendOptionDisplayLabel(option: OptimizeOption): string {
  const value = String(option.value || "").trim();
  if (value === "Auto") return "Auto (recommended)";
  if (value === "CpuOnly") return "CPU only (indicators + backtest on CPU)";
  if (
    value === "GpuOnly" ||
    value === "GpuHybridKernel" ||
    value === "GpuKernel"
  ) {
    return "GPU unified (kernel + staged fallback)";
  }
  return String(option.label || option.value || "").trim() || value;
}

function routeBackendLabel(
  backendKind: "CpuOnly" | "GpuKernel" | "GpuHybridKernel",
): string {
  if (backendKind === "CpuOnly") return "CPU";
  if (backendKind === "GpuHybridKernel") return "GPU hybrid";
  return "GPU kernel";
}

const backendExecutionHint = computed(() => {
  if (!strategyCapabilities.value.usesGlobalBackend) {
    return strategyCapabilities.value.executionSummary;
  }
  const routePreview = compositeRoutePreview.value;
  if (routePreview) {
    if (routePreview.targetBackend === "CpuOnly") {
      return "Composite route preview: current inputs resolve to the CPU route. Heatmap, Top-K, include-all, and explicit metric-contract overrides remain available; export-all and exact-kernel shortlist controls do not.";
    }
    if (routePreview.targetBackend === "GpuHybridKernel") {
      return "Composite route preview: current inputs resolve to the hybrid GPU route. Heatmap, Top-K, include-all, and explicit metric-contract overrides remain available; export-all and exact-kernel shortlist controls stay disabled on hybrid execution.";
    }
    return "Composite route preview: current inputs resolve to the GPU kernel route. Export-all and exact-kernel shortlist controls stay available on this route.";
  }
  const backendKind = selectedBackend.value;
  if (backendKind === "CpuOnly") {
    return "All indicator calculations and backtest metrics are executed on CPU (fp64 path).";
  }
  if (backendKind === "GpuKernel") {
    return "Unified route: kernel-capable indicators stay on GPU; non-kernel indicators are staged, and fallback paths are handled automatically.";
  }
  return "Auto selects the best route for strategy constraints, MA capability, and gpu_speedups.csv policy.";
});

const showModeControl = computed(
  () => strategyCapabilities.value.usesGlobalMode,
);
const showBackendControl = computed(
  () => strategyCapabilities.value.usesGlobalBackend,
);
const showSearchSpaceControls = computed(() => true);
const executionSectionTitle = computed(() =>
  showBackendControl.value ? "Backend and GPU route" : "Execution route",
);
const showGpuPresetActions = computed(() => showBackendControl.value);

function supportsBackend(
  entry: MovingAverageCatalogEntry,
  backendKind: BackendKind,
): boolean {
  if (!entry.periodBased || !entry.singleOutput) return false;
  if (backendKind === "CpuOnly") return entry.supportsCpu;
  if (backendKind === "GpuKernel") {
    // Unified GPU mode can route via kernel, CUDA-sweep fallback, or CPU staging.
    return entry.supportsCudaKernel || entry.supportsCuda || entry.supportsCpu;
  }
  return entry.supportsCpu || entry.supportsCuda || entry.supportsCudaKernel;
}

function maOptionLabel(
  entry: MovingAverageCatalogEntry,
  backendKind: BackendKind,
  speedup: GpuSpeedupEntry | null,
): string {
  const base = String(entry.label || entry.id).trim() || entry.id;
  if (!entry.periodBased || !entry.singleOutput) {
    return `${base} (not period/single-output)`;
  }
  if (backendKind !== "GpuKernel") return base;

  const supportsGpuPath = entry.supportsCuda;
  const backendSupported = supportsBackend(entry, backendKind);

  let perfText = "";
  if (supportsGpuPath) {
    if (speedup) {
      perfText = `${speedup.speedup.toFixed(2)}x GPU`;
    } else if (maSpeedupLoaded.value) {
      perfText = "warning: GPU slower vs CPU";
    } else {
      perfText = "GPU speed: loading...";
    }
  }

  const supportText = backendSupported
    ? ""
    : `not supported on ${backendLabel(backendKind)}`;
  if (perfText && supportText) {
    return `${base} (${perfText}; ${supportText})`;
  }
  if (perfText) return `${base} (${perfText})`;
  if (supportText) {
    return `${base} (${supportText})`;
  }
  return base;
}

function buildBackendAwareMaOptions(
  catalog: MovingAverageCatalogEntry[],
  backendKind: BackendKind,
  speedupById: Record<string, GpuSpeedupEntry>,
): OptimizeOption[] {
  const entries = [...catalog];
  entries.sort((a, b) => {
    const left = String(a.label || a.id).toLowerCase();
    const right = String(b.label || b.id).toLowerCase();
    return left.localeCompare(right);
  });
  return entries.map((entry) => {
    const id = String(entry.id || "").trim();
    const speedup = id ? speedupById[id] || null : null;
    return {
      value: id,
      label: maOptionLabel(entry, backendKind, speedup),
      disabled: !supportsBackend(entry, backendKind),
    };
  });
}

function selectedMaHint(
  maType: string,
  backendKind: BackendKind,
  catalog: MovingAverageCatalogEntry[],
  speedupById: Record<string, GpuSpeedupEntry>,
): string {
  if (backendKind !== "GpuKernel") return "";
  const id = String(maType || "").trim();
  if (!id) return "";
  const entry = catalog.find((row) => row.id === id);
  if (!entry) return "";
  const notes: string[] = [];
  if (!supportsBackend(entry, backendKind)) {
    notes.push(`Selected MA is not supported on ${backendLabel(backendKind)}.`);
  }
  if (entry.supportsCpu && !entry.supportsCudaKernel) {
    notes.push("This MA will use CPU staging in unified GPU mode.");
  }
  if (entry.supportsCuda) {
    const speedup = speedupById[id];
    if (speedup) {
      notes.push(
        `GPU speedup on this system: ${speedup.speedup.toFixed(2)}x vs CPU.`,
      );
    } else if (maSpeedupLoaded.value) {
      notes.push("Warning: this MA is slower on GPU vs CPU on this system.");
    }
  }
  return notes.join(" ");
}

function optionLabelFor(
  options: OptimizeOption[],
  value: string,
  fallback: string,
): string {
  const target = String(value || "").trim();
  if (!target) return String(fallback || "").trim();
  const match = options.find(
    (option) => String(option.value || "").trim() === target,
  );
  const raw = String(match?.label || fallback || target).trim();
  return raw || target;
}

function compactSummaryLabel(label: string): string {
  return String(label || "")
    .replace(/\s+\([^)]*\)$/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

const selectedBackend = computed<BackendKind>(() =>
  normalizeBackendKind(backendDraft.value || backend.value || "Auto"),
);
const backendAwareMaOptions = computed(() =>
  buildBackendAwareMaOptions(
    maCatalogEntries.value,
    selectedBackend.value,
    maSpeedupById.value,
  ),
);
const fastMaBackendHint = computed(() =>
  selectedMaHint(
    selectedFastMaType.value,
    selectedBackend.value,
    maCatalogEntries.value,
    maSpeedupById.value,
  ),
);
const slowMaBackendHint = computed(() =>
  selectedMaHint(
    selectedSlowMaType.value,
    selectedBackend.value,
    maCatalogEntries.value,
    maSpeedupById.value,
  ),
);
const dynamicCatalogMeta = computed(() => {
  const total = dynamicCatalogEntries.value.length;
  if (total <= 0) {
    return hasTauriRuntime.value
      ? "Dynamic indicator catalog: unavailable."
      : "Dynamic indicator catalog requires the desktop shell. The builder still works for fast/slow MA logic and manual JSON editing.";
  }
  const gpuPreferred = dynamicCatalogEntries.value.filter(
    (entry) => entry.prefersGpuBySpeedup,
  ).length;
  return `Dynamic indicator catalog: ${total} eligible indicators (${gpuPreferred} GPU-preferred by gpu_speedups.csv).`;
});

const strategyOptionItems = computed(() =>
  resolvedOptions(
    strategyOptions.value,
    strategyKind.value,
    strategyKind.value || "Strategy",
  ),
);
const presetOptionItems = computed(() =>
  resolvedOptions(
    presetOptions.value,
    presetProfile.value,
    presetProfile.value || "Preset",
  ),
);
const backendOptionItems = computed(() =>
  resolvedOptions(
    backendOptions.value,
    backend.value,
    backend.value || "Backend",
  ).map((option) => ({
    ...option,
    label: backendOptionDisplayLabel(option),
  })),
);
const fastTypeOptionItems = computed(() =>
  resolvedOptions(
    backendAwareMaOptions.value,
    fastMaType.value,
    fastMaType.value || "MA type",
    fastMaTypeOptions.value,
  ),
);
const slowTypeOptionItems = computed(() =>
  resolvedOptions(
    backendAwareMaOptions.value,
    slowMaType.value,
    slowMaType.value || "MA type",
    slowMaTypeOptions.value,
  ),
);
const optimizeSummaryChips = computed(() => {
  const chips: string[] = [];
  const strategyValue = String(
    strategyDraft.value || strategyKind.value || "",
  ).trim();
  const presetValue = String(
    presetDraft.value || presetProfile.value || "",
  ).trim();
  const objectiveName = String(
    objectiveDraft.value || objective.value || "",
  ).trim();
  const modeName = String(modeDraft.value || mode.value || "").trim();
  const strategyLabel = compactSummaryLabel(
    optionLabelFor(strategyOptionItems.value, strategyValue, strategyValue),
  );
  const presetLabel = compactSummaryLabel(
    optionLabelFor(presetOptionItems.value, presetValue, presetValue),
  );
  const backendName = compactSummaryLabel(
    compositeRoutePreview.value
      ? routeBackendLabel(compositeRoutePreview.value.targetBackend)
      : backendLabel(selectedBackend.value),
  );
  const pairs = String(pairsPreview.value || "").trim();
  if (strategyLabel) chips.push(`Strategy: ${strategyLabel}`);
  if (presetLabel) chips.push(`Preset: ${presetLabel}`);
  if (objectiveName) chips.push(`Objective: ${objectiveName}`);
  if (modeName) chips.push(`Mode: ${modeName}`);
  if (backendName) chips.push(`Route: ${backendName}`);
  if (pairs) chips.push(pairs);
  return chips;
});

watch(
  () =>
    [
      String(strategyKind.value || ""),
      String(presetProfile.value || ""),
      String(backend.value || ""),
      String(objective.value || "Sharpe"),
      String(mode.value || "Grid"),
      String(dynamicStrategyIrJson.value || ""),
      String(fastStart.value || ""),
      String(fastEnd.value || ""),
      String(fastStep.value || ""),
      String(slowStart.value || ""),
      String(slowEnd.value || ""),
      String(slowStep.value || ""),
      String(fastMaType.value || ""),
      String(slowMaType.value || ""),
    ] as const,
  ([
    nextStrategy,
    nextPreset,
    nextBackend,
    nextObjective,
    nextMode,
    nextDynamicStrategyIrJson,
    nextFastStart,
    nextFastEnd,
    nextFastStep,
    nextSlowStart,
    nextSlowEnd,
    nextSlowStep,
    nextFastMaType,
    nextSlowMaType,
  ]) => {
    if (strategyDraft.value !== nextStrategy)
      strategyDraft.value = nextStrategy;
    if (presetDraft.value !== nextPreset) presetDraft.value = nextPreset;
    if (backendDraft.value !== nextBackend) backendDraft.value = nextBackend;
    if (objectiveDraft.value !== nextObjective)
      objectiveDraft.value = nextObjective;
    if (modeDraft.value !== nextMode) modeDraft.value = nextMode;
    if (dynamicStrategyIrJsonDraft.value !== nextDynamicStrategyIrJson)
      dynamicStrategyIrJsonDraft.value = nextDynamicStrategyIrJson;
    if (fastStartDraft.value !== nextFastStart)
      fastStartDraft.value = nextFastStart;
    if (fastEndDraft.value !== nextFastEnd) fastEndDraft.value = nextFastEnd;
    if (fastStepDraft.value !== nextFastStep)
      fastStepDraft.value = nextFastStep;
    if (slowStartDraft.value !== nextSlowStart)
      slowStartDraft.value = nextSlowStart;
    if (slowEndDraft.value !== nextSlowEnd) slowEndDraft.value = nextSlowEnd;
    if (slowStepDraft.value !== nextSlowStep)
      slowStepDraft.value = nextSlowStep;
    if (fastTypeDraft.value !== nextFastMaType)
      fastTypeDraft.value = nextFastMaType;
    if (slowTypeDraft.value !== nextSlowMaType)
      slowTypeDraft.value = nextSlowMaType;
  },
  { immediate: true },
);

function setInputValue(id: string, nextValue: string): void {
  optimizeStore.setInputField(id, nextValue);
}

const { queueInputValue, flushQueuedInputValuesNow } =
  useQueuedInputWriter(setInputValue);

function setSelectValue(id: string, nextValue: string): void {
  optimizeStore.setSelectField(id, nextValue);
}

function runOptimizationAction(): void {
  flushQueuedInputValuesNow();
  optimizeStore.runOptimization();
}

function runValidationAction(): void {
  flushQueuedInputValuesNow();
  optimizeStore.runValidation();
}

function previewDynamicStrategyIrAction(): void {
  if (!dynamicPreviewAvailable.value) {
    optimizeStore.dynamicStrategyPreviewMeta =
      "Dynamic strategy preview: unavailable in browser-only mode";
    optimizeStore.dynamicStrategyPreviewReasons = `Reasons: ${dynamicPreviewDisabledReason.value}`;
    optimizeStore.statusLine = dynamicPreviewDisabledReason.value;
    return;
  }
  optimizeStore.setDynamicStrategyIrJson(dynamicStrategyIrJsonDraft.value);
  optimizeStore.previewDynamicStrategyIr();
}

function nextDynamicBuilderId(prefix: string, existingIds: string[]): string {
  const taken = new Set(existingIds);
  for (let index = 1; index <= 999; index += 1) {
    const candidate = `${prefix}_${index}`;
    if (!taken.has(candidate)) return candidate;
  }
  return `${prefix}_${Date.now()}`;
}

function tryRestoreDynamicBuilderState(
  raw: string,
  eligibleEntries: DynamicIndicatorCatalogEntry[],
): DynamicDoubleMaBuilderState | null {
  const text = String(raw || "").trim();
  if (!text) return null;
  try {
    const parsed = JSON.parse(text) as Record<string, unknown>;
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed))
      return null;
    const restored = createDefaultDynamicDoubleMaBuilderState(eligibleEntries);
    if (typeof parsed.longOnly === "boolean")
      restored.longOnly = parsed.longOnly;
    if (typeof parsed.allowFlip === "boolean")
      restored.allowFlip = parsed.allowFlip;
    if (Array.isArray(parsed.indicators)) {
      restored.indicators = parsed.indicators
        .map((entry) => {
          if (!entry || typeof entry !== "object" || Array.isArray(entry))
            return null;
          const typed = entry as Record<string, unknown>;
          const paramsRaw =
            typed.params &&
            typeof typed.params === "object" &&
            !Array.isArray(typed.params)
              ? (typed.params as Record<string, unknown>)
              : {};
          const params: Record<string, DynamicIndicatorParamValue> = {};
          for (const [key, rawValue] of Object.entries(paramsRaw)) {
            if (
              !rawValue ||
              typeof rawValue !== "object" ||
              Array.isArray(rawValue)
            )
              continue;
            const valueRow = rawValue as Partial<DynamicIndicatorParamValue>;
            const kind = String(valueRow.kind || "").trim();
            if (kind === "bool")
              params[key] = { kind: "bool", value: Boolean(valueRow.value) };
            else if (kind === "enum_string")
              params[key] = {
                kind: "enum_string",
                value: String(valueRow.value || ""),
              };
            else if (kind === "int")
              params[key] = {
                kind: "int",
                value: Math.round(Number(valueRow.value || 0)),
              };
            else if (kind === "float")
              params[key] = {
                kind: "float",
                value: Number(valueRow.value || 0),
              };
          }
          return {
            id: String(typed.id || "").trim(),
            indicatorId: String(typed.indicatorId || "").trim(),
            outputId: String(typed.outputId || "").trim(),
            params,
          };
        })
        .filter(
          (entry): entry is DynamicDoubleMaBuilderState["indicators"][number] =>
            Boolean(entry?.id),
        );
    }
    if (Array.isArray(parsed.conditions)) {
      restored.conditions = parsed.conditions
        .map((entry) => {
          if (!entry || typeof entry !== "object" || Array.isArray(entry))
            return null;
          const typed = entry as Record<string, unknown>;
          const lhs =
            typed.lhs &&
            typeof typed.lhs === "object" &&
            !Array.isArray(typed.lhs)
              ? (typed.lhs as Record<string, unknown>)
              : {};
          const rhs =
            typed.rhs &&
            typeof typed.rhs === "object" &&
            !Array.isArray(typed.rhs)
              ? (typed.rhs as Record<string, unknown>)
              : {};
          return {
            id: String(typed.id || "").trim(),
            op: String(typed.op || "gt")
              .trim()
              .toLowerCase() as DynamicBuilderConditionOp,
            lhs: {
              kind: String(lhs.kind || "indicator")
                .trim()
                .toLowerCase() as DynamicBuilderOperand["kind"],
              value: String(lhs.value || "fast_ma"),
            },
            rhs: {
              kind: String(rhs.kind || "indicator")
                .trim()
                .toLowerCase() as DynamicBuilderOperand["kind"],
              value: String(rhs.value || "slow_ma"),
            },
          };
        })
        .filter((entry): entry is DynamicBuilderCondition =>
          Boolean(entry?.id),
        );
    }
    if (Array.isArray(parsed.groups)) {
      restored.groups = parsed.groups
        .map((entry) => {
          if (!entry || typeof entry !== "object" || Array.isArray(entry))
            return null;
          const typed = entry as Record<string, unknown>;
          return {
            id: String(typed.id || "").trim(),
            mode:
              String(typed.mode || "all")
                .trim()
                .toLowerCase() === "any"
                ? "any"
                : "all",
            conditionIds: Array.isArray(typed.conditionIds)
              ? typed.conditionIds
                  .map((id) => String(id || "").trim())
                  .filter(Boolean)
              : [],
          };
        })
        .filter((entry): entry is DynamicBuilderGroup => Boolean(entry?.id));
    }
    const signals =
      parsed.signals &&
      typeof parsed.signals === "object" &&
      !Array.isArray(parsed.signals)
        ? (parsed.signals as Record<string, unknown>)
        : {};
    restored.signals = {
      longEntryGroup: String(signals.longEntryGroup || ""),
      longExitGroup: String(signals.longExitGroup || ""),
      shortEntryGroup: String(signals.shortEntryGroup || ""),
      shortExitGroup: String(signals.shortExitGroup || ""),
    };
    return restored;
  } catch {
    return null;
  }
}

function ensureDynamicBuilderSeeded(force = false): void {
  if (!isDynamicDoubleMaKind.value) return;
  if (dynamicBuilderSeeded.value && !force) return;
  dynamicBuilderState.value =
    tryRestoreDynamicBuilderState(
      dynamicStrategyBuilderStateJson.value,
      dynamicBuilderEligibleEntries.value,
    ) ||
    createDefaultDynamicDoubleMaBuilderState(
      dynamicBuilderEligibleEntries.value,
    );
  dynamicBuilderSeeded.value = true;
}

function updateDynamicBuilderLongOnly(nextLongOnly: boolean): void {
  setDynamicDoubleMaBuilderLongOnly(dynamicBuilderState.value, nextLongOnly);
}

function updateDynamicBuilderAllowFlip(nextAllowFlip: boolean): void {
  dynamicBuilderState.value.allowFlip = dynamicBuilderState.value.longOnly
    ? false
    : nextAllowFlip;
}

function selectDynamicCatalogEntry(
  indicatorId: string,
): DynamicIndicatorCatalogEntry | null {
  const id = String(indicatorId || "")
    .trim()
    .toLowerCase();
  return dynamicCatalogEntries.value.find((entry) => entry.id === id) || null;
}

function dynamicBuilderOutputOptions(slotId: string): BuilderOption[] {
  const slot = dynamicBuilderState.value.indicators.find(
    (entry) => entry.id === slotId,
  );
  const entry = selectDynamicCatalogEntry(slot?.indicatorId || "");
  if (!entry) return [];
  return entry.outputs
    .filter(
      (output) => output.valueType === "f64" || output.valueType === "f32",
    )
    .map((output) => ({
      value: output.id,
      label: `${output.label} (${output.valueType})`,
    }));
}

function dynamicBuilderParamDescriptors(
  slotId: string,
): DynamicIndicatorParamDescriptor[] {
  const slot = dynamicBuilderState.value.indicators.find(
    (entry) => entry.id === slotId,
  );
  const entry = selectDynamicCatalogEntry(slot?.indicatorId || "");
  return entry?.params || [];
}

function dynamicBuilderParamValue(
  slotId: string,
  descriptor: DynamicIndicatorParamDescriptor,
): string | number | boolean {
  const slot = dynamicBuilderState.value.indicators.find(
    (entry) => entry.id === slotId,
  );
  const current = slot?.params?.[descriptor.key];
  if (current) return current.value;
  if (descriptor.defaultValue) return descriptor.defaultValue.value;
  if (descriptor.kind === "bool") return false;
  if (descriptor.kind === "enum_string") return descriptor.enumValues[0] || "";
  return descriptor.min ?? descriptor.max ?? "";
}

function updateDynamicBuilderParam(
  slotId: string,
  descriptor: DynamicIndicatorParamDescriptor,
  rawValue: string | number | boolean,
): void {
  const slot = dynamicBuilderState.value.indicators.find(
    (entry) => entry.id === slotId,
  );
  if (!slot) return;
  let next: DynamicIndicatorParamValue | null = null;
  if (descriptor.kind === "bool") {
    next = { kind: "bool", value: Boolean(rawValue) };
  } else if (descriptor.kind === "enum_string") {
    const value = String(rawValue || "").trim();
    if (!value) return;
    next = { kind: "enum_string", value };
  } else {
    const parsed = Number(rawValue);
    if (!Number.isFinite(parsed)) return;
    next =
      descriptor.kind === "int"
        ? { kind: "int", value: Math.round(parsed) }
        : { kind: "float", value: parsed };
  }
  slot.params = {
    ...slot.params,
    [descriptor.key]: next,
  };
}

function updateDynamicBuilderIndicator(
  slotId: string,
  indicatorId: string,
): void {
  const slot = dynamicBuilderState.value.indicators.find(
    (entry) => entry.id === slotId,
  );
  if (!slot) return;
  const nextEntry = selectDynamicCatalogEntry(indicatorId);
  slot.indicatorId = String(indicatorId || "")
    .trim()
    .toLowerCase();
  slot.outputId = preferredFloatingOutputId(nextEntry);
  slot.params = defaultIndicatorParams(nextEntry);
}

function addDynamicBuilderIndicator(): void {
  const existingIds = dynamicBuilderState.value.indicators.map(
    (entry) => entry.id,
  );
  const nextId = nextDynamicBuilderId("indicator", existingIds);
  const usedIndicators = new Set(
    dynamicBuilderState.value.indicators.map((entry) => entry.indicatorId),
  );
  const nextEntry =
    dynamicBuilderEligibleEntries.value.find(
      (entry) => !usedIndicators.has(entry.id),
    ) ||
    dynamicBuilderEligibleEntries.value[0] ||
    null;
  dynamicBuilderState.value.indicators.push({
    id: nextId,
    indicatorId: String(nextEntry?.id || ""),
    outputId: preferredFloatingOutputId(nextEntry),
    params: defaultIndicatorParams(nextEntry),
  });
}

function removeDynamicBuilderIndicator(slotId: string): void {
  dynamicBuilderState.value.indicators =
    dynamicBuilderState.value.indicators.filter((entry) => entry.id !== slotId);
  for (const condition of dynamicBuilderState.value.conditions) {
    if (condition.lhs.kind === "indicator" && condition.lhs.value === slotId) {
      condition.lhs.value = "fast_ma";
    }
    if (condition.rhs.kind === "indicator" && condition.rhs.value === slotId) {
      condition.rhs.value = "slow_ma";
    }
  }
}

function updateDynamicConditionOperand(
  conditionId: string,
  side: "lhs" | "rhs",
  operandKind: DynamicBuilderOperand["kind"],
  value: string,
): void {
  const condition = dynamicBuilderState.value.conditions.find(
    (entry) => entry.id === conditionId,
  );
  if (!condition) return;
  condition[side] = { kind: operandKind, value };
}

function dynamicOperandSelectValue(operand: DynamicBuilderOperand): string {
  return operand.kind === "constant" ? "__constant__" : operand.value;
}

function updateDynamicConditionOperandSelection(
  conditionId: string,
  side: "lhs" | "rhs",
  rawValue: string,
): void {
  const normalized = String(rawValue || "").trim();
  if (normalized === "__constant__") {
    updateDynamicConditionOperand(conditionId, side, "constant", "0");
    return;
  }
  updateDynamicConditionOperand(
    conditionId,
    side,
    dynamicOperandKindForValue(normalized),
    normalized || (side === "lhs" ? "fast_ma" : "slow_ma"),
  );
}

function updateDynamicConditionConstantValue(
  conditionId: string,
  side: "lhs" | "rhs",
  value: string,
): void {
  updateDynamicConditionOperand(conditionId, side, "constant", value);
}

function dynamicOperandKindForValue(
  value: string,
): DynamicBuilderOperand["kind"] {
  const normalized = String(value || "").trim();
  if (normalized === "__constant__") return "constant";
  if (normalized === "close") return "close";
  if (normalized === "ma_source") return "ma_source";
  return "indicator";
}

function updateDynamicConditionOp(conditionId: string, op: string): void {
  const condition = dynamicBuilderState.value.conditions.find(
    (entry) => entry.id === conditionId,
  );
  if (!condition) return;
  condition.op = String(op || "gt")
    .trim()
    .toLowerCase() as DynamicBuilderConditionOp;
}

function updateDynamicGroupMode(groupId: string, mode: string): void {
  const group = dynamicBuilderState.value.groups.find(
    (entry) => entry.id === groupId,
  );
  if (!group) return;
  group.mode =
    String(mode || "all")
      .trim()
      .toLowerCase() === "any"
      ? "any"
      : "all";
}

function addDynamicCondition(): void {
  const nextId = nextDynamicBuilderId(
    "condition",
    dynamicBuilderState.value.conditions.map((entry) => entry.id),
  );
  const condition: DynamicBuilderCondition = {
    id: nextId,
    op: "gt",
    lhs: { kind: "indicator", value: "fast_ma" },
    rhs: { kind: "indicator", value: "slow_ma" },
  };
  dynamicBuilderState.value.conditions.push(condition);
}

function removeDynamicCondition(conditionId: string): void {
  dynamicBuilderState.value.conditions =
    dynamicBuilderState.value.conditions.filter(
      (entry) => entry.id !== conditionId,
    );
  for (const group of dynamicBuilderState.value.groups) {
    group.conditionIds = group.conditionIds.filter(
      (entry) => entry !== conditionId,
    );
  }
}

function addDynamicGroup(): void {
  const nextId = nextDynamicBuilderId(
    "group",
    dynamicBuilderState.value.groups.map((entry) => entry.id),
  );
  const firstConditionId = dynamicBuilderState.value.conditions[0]?.id;
  const group: DynamicBuilderGroup = {
    id: nextId,
    mode: "all",
    conditionIds: firstConditionId ? [firstConditionId] : [],
  };
  dynamicBuilderState.value.groups.push(group);
}

function removeDynamicGroup(groupId: string): void {
  dynamicBuilderState.value.groups = dynamicBuilderState.value.groups.filter(
    (entry) => entry.id !== groupId,
  );
  if (dynamicBuilderState.value.signals.longEntryGroup === groupId)
    dynamicBuilderState.value.signals.longEntryGroup = "";
  if (dynamicBuilderState.value.signals.longExitGroup === groupId)
    dynamicBuilderState.value.signals.longExitGroup = "";
  if (dynamicBuilderState.value.signals.shortEntryGroup === groupId)
    dynamicBuilderState.value.signals.shortEntryGroup = "";
  if (dynamicBuilderState.value.signals.shortExitGroup === groupId)
    dynamicBuilderState.value.signals.shortExitGroup = "";
}

function toggleDynamicGroupCondition(
  groupId: string,
  conditionId: string,
  enabled: boolean,
): void {
  const group = dynamicBuilderState.value.groups.find(
    (entry) => entry.id === groupId,
  );
  if (!group) return;
  if (enabled) {
    if (!group.conditionIds.includes(conditionId))
      group.conditionIds.push(conditionId);
    return;
  }
  group.conditionIds = group.conditionIds.filter(
    (entry) => entry !== conditionId,
  );
}

function previewDynamicStrategyFromBuilderAction(): void {
  try {
    const prettyJson = formatDynamicDoubleMaBuilderIrJson({
      fastMaType: selectedFastMaType.value,
      slowMaType: selectedSlowMaType.value,
      fastMaParams: optimizeStore.fastMaParams,
      slowMaParams: optimizeStore.slowMaParams,
      builder: dynamicBuilderState.value,
    });
    dynamicStrategyIrJsonDraft.value = prettyJson;
    optimizeStore.setDynamicStrategyIrJson(prettyJson);
    if (!dynamicPreviewAvailable.value) {
      optimizeStore.dynamicStrategyPreviewMeta =
        "Dynamic strategy preview: unavailable in browser-only mode";
      optimizeStore.dynamicStrategyPreviewReasons = `Reasons: ${dynamicPreviewDisabledReason.value}`;
      optimizeStore.statusLine = dynamicPreviewDisabledReason.value;
      return;
    }
    optimizeStore.previewDynamicStrategyIr();
  } catch (error) {
    optimizeStore.dynamicStrategyPreviewMeta =
      "Dynamic strategy preview: failed";
    optimizeStore.dynamicStrategyPreviewReasons = `Reasons: ${String(error || "").trim()}`;
    optimizeStore.statusLine = `Dynamic strategy preview failed: ${String(error || "").trim()}`;
  }
}

function normalizeFinite(value: unknown): number | null {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function serializeNumber(value: number): string {
  return Number.isFinite(value) ? String(value) : "";
}

function defaultSweepStep(schema: MaParamDescriptor): number {
  if (schema.step != null && Number.isFinite(schema.step) && schema.step > 0)
    return schema.step;
  return schema.kind === "int" ? 1 : 0.1;
}

function resolveInputStep(step: number | null, kind: MaParamKind): string {
  if (step != null && Number.isFinite(step) && step > 0) return String(step);
  return kind === "int" ? "1" : "any";
}

function createMaParamUiState(
  schema: MaParamDescriptor,
  seededValue: unknown,
  seededSweep: unknown,
): MaParamUiState {
  const defaultValue = normalizeFinite(seededValue) ?? schema.defaultValue;
  const sweepRow = Array.isArray(seededSweep) ? seededSweep : null;
  const sweepStart = sweepRow ? normalizeFinite(sweepRow[0]) : null;
  const sweepEnd = sweepRow ? normalizeFinite(sweepRow[1]) : null;
  const sweepStep = sweepRow ? normalizeFinite(sweepRow[2]) : null;
  const sweepEnabled =
    sweepStart != null &&
    sweepEnd != null &&
    sweepStep != null &&
    sweepStep > 0;

  const resolvedSweepStep = sweepEnabled
    ? (sweepStep as number)
    : defaultSweepStep(schema);

  return {
    value: serializeNumber(defaultValue),
    sweepEnabled,
    sweepStart: serializeNumber(
      sweepEnabled ? (sweepStart as number) : defaultValue,
    ),
    sweepEnd: serializeNumber(
      sweepEnabled ? (sweepEnd as number) : defaultValue,
    ),
    sweepStep: serializeNumber(resolvedSweepStep),
  };
}

function resolveSideTypeState(side: MaSide, maId: string): MaParamUiStateByKey {
  if (!maId) return {};
  const sideState = maParamUiStateBySide.value[side];
  const row = sideState[maId];
  return row && typeof row === "object" ? row : {};
}

function resolveMaParamUiState(
  side: MaSide,
  maId: string,
  schema: MaParamDescriptor,
): MaParamUiState {
  const state = resolveSideTypeState(side, maId)[schema.key];
  return state || createMaParamUiState(schema, schema.defaultValue, null);
}

function ensureSideTypeSchemaState(
  side: MaSide,
  maId: string,
  schema: MaParamDescriptor[],
  seedParams: unknown,
  seedSweeps: unknown,
  overwrite = false,
): void {
  if (!maId || schema.length === 0) return;
  const sideState = maParamUiStateBySide.value[side];
  const paramsRow =
    seedParams && typeof seedParams === "object"
      ? (seedParams as Record<string, unknown>)
      : {};
  const sweepsRow =
    seedSweeps && typeof seedSweeps === "object"
      ? (seedSweeps as Record<string, unknown>)
      : {};
  let typeState = sideState[maId];
  if (!typeState || typeof typeState !== "object") {
    typeState = {};
    sideState[maId] = typeState;
  }

  const allowed = new Set<string>();
  for (const item of schema) {
    allowed.add(item.key);
    if (!overwrite && typeState[item.key]) continue;
    typeState[item.key] = createMaParamUiState(
      item,
      paramsRow[item.key],
      sweepsRow[item.key],
    );
  }

  for (const key of Object.keys(typeState)) {
    if (!allowed.has(key)) delete typeState[key];
  }
}

function buildSideMaParamsAndSweeps(side: MaSide): {
  params: Record<string, number> | null;
  sweeps: Record<string, [number, number, number]> | null;
} {
  const maId =
    side === "fast" ? selectedFastMaType.value : selectedSlowMaType.value;
  const schema =
    side === "fast" ? fastMaParamSchema.value : slowMaParamSchema.value;
  if (!maId || schema.length === 0) return { params: null, sweeps: null };

  const stateByKey = resolveSideTypeState(side, maId);
  const seededParams =
    side === "fast" ? optimizeStore.fastMaParams : optimizeStore.slowMaParams;
  const seededSweeps =
    side === "fast"
      ? optimizeStore.fastMaParamSweeps
      : optimizeStore.slowMaParamSweeps;
  const paramsRow =
    seededParams && typeof seededParams === "object"
      ? (seededParams as Record<string, unknown>)
      : {};
  const sweepsRow =
    seededSweeps && typeof seededSweeps === "object"
      ? (seededSweeps as Record<string, unknown>)
      : {};
  const params: Record<string, number> = {};
  const sweeps: Record<string, [number, number, number]> = {};
  for (const item of schema) {
    const state =
      stateByKey[item.key] ||
      createMaParamUiState(item, paramsRow[item.key], sweepsRow[item.key]);
    const rawValue = normalizeFinite(state.value);
    params[item.key] = rawValue ?? item.defaultValue;
    if (!state.sweepEnabled) continue;
    const start = normalizeFinite(state.sweepStart);
    const end = normalizeFinite(state.sweepEnd);
    const step = normalizeFinite(state.sweepStep);
    if (start == null || end == null || step == null || step <= 0) continue;
    sweeps[item.key] = [start, end, step];
  }

  return {
    params: Object.keys(params).length > 0 ? params : null,
    sweeps: Object.keys(sweeps).length > 0 ? sweeps : null,
  };
}

function syncMaParamStateFromVue(): void {
  optimizeStore.syncMaParamState({
    fast: buildSideMaParamsAndSweeps("fast"),
    slow: buildSideMaParamsAndSweeps("slow"),
  });
}

function markSelectedTypeTouched(side: MaSide): void {
  const maId =
    side === "fast" ? selectedFastMaType.value : selectedSlowMaType.value;
  if (!maId) return;
  maParamTouchedBySide.value[side][maId] = true;
}

function updateMaParamValue(side: MaSide, key: string, raw: string): void {
  const maId =
    side === "fast" ? selectedFastMaType.value : selectedSlowMaType.value;
  const schema =
    side === "fast" ? fastMaParamSchema.value : slowMaParamSchema.value;
  const item = schema.find((row) => row.key === key);
  if (!maId || !item) return;
  ensureSideTypeSchemaState(
    side,
    maId,
    schema,
    side === "fast" ? optimizeStore.fastMaParams : optimizeStore.slowMaParams,
    side === "fast"
      ? optimizeStore.fastMaParamSweeps
      : optimizeStore.slowMaParamSweeps,
    false,
  );
  const stateByKey = resolveSideTypeState(side, maId);
  const state = stateByKey[key];
  if (!state) return;
  state.value = raw;
  markSelectedTypeTouched(side);
  syncMaParamStateFromVue();
}

function updateMaParamSweepEnabled(
  side: MaSide,
  key: string,
  enabled: boolean,
): void {
  const maId =
    side === "fast" ? selectedFastMaType.value : selectedSlowMaType.value;
  const schema =
    side === "fast" ? fastMaParamSchema.value : slowMaParamSchema.value;
  const item = schema.find((row) => row.key === key);
  if (!maId || !item) return;
  const stateByKey = resolveSideTypeState(side, maId);
  const state = stateByKey[key];
  if (!state) return;
  state.sweepEnabled = enabled;
  if (enabled) {
    if (!String(state.sweepStart || "").trim())
      state.sweepStart = String(state.value || item.defaultValue);
    if (!String(state.sweepEnd || "").trim())
      state.sweepEnd = String(state.value || item.defaultValue);
    if (!String(state.sweepStep || "").trim())
      state.sweepStep = serializeNumber(defaultSweepStep(item));
  }
  markSelectedTypeTouched(side);
  syncMaParamStateFromVue();
}

function updateMaParamSweepField(
  side: MaSide,
  key: string,
  field: "sweepStart" | "sweepEnd" | "sweepStep",
  raw: string,
): void {
  const maId =
    side === "fast" ? selectedFastMaType.value : selectedSlowMaType.value;
  if (!maId) return;
  const stateByKey = resolveSideTypeState(side, maId);
  const state = stateByKey[key];
  if (!state) return;
  state[field] = raw;
  markSelectedTypeTouched(side);
  syncMaParamStateFromVue();
}

function maParamInputId(side: MaSide, key: string): string {
  const maId =
    side === "fast" ? selectedFastMaType.value : selectedSlowMaType.value;
  return `vue_${side}_${maId || "ma"}_param_${key}`;
}

function waitForNextPaint(): Promise<void> {
  return new Promise((resolve) => {
    if (typeof requestAnimationFrame !== "function") {
      setTimeout(() => resolve(), 0);
      return;
    }
    requestAnimationFrame(() => resolve());
  });
}

async function loadMaParamCatalog(): Promise<void> {
  try {
    const catalog = await listMovingAverageCatalog();
    if (panelDisposed) return;
    const nextMap: Record<string, MaParamDescriptor[]> = {};
    for (const item of catalog) {
      nextMap[item.id] = item.params;
    }
    maParamSchemasByMaId.value = nextMap;
    maCatalogEntries.value = catalog;
  } catch {
    if (panelDisposed) return;
    maParamSchemasByMaId.value = {};
    maCatalogEntries.value = [];
  } finally {
    if (!panelDisposed) maParamCatalogLoaded.value = true;
  }
}

async function loadMaSpeedups(): Promise<void> {
  try {
    const speedupMap = await listGpuSpeedupMap();
    if (panelDisposed) return;
    maSpeedupById.value = speedupMap;
  } catch {
    if (panelDisposed) return;
    maSpeedupById.value = {};
  } finally {
    if (!panelDisposed) maSpeedupLoaded.value = true;
  }
}

async function loadDynamicCatalogMeta(): Promise<void> {
  if (!hasTauriRuntime.value) {
    dynamicCatalogEntries.value = [];
    if (isDynamicDoubleMaKind.value) ensureDynamicBuilderSeeded(true);
    return;
  }
  try {
    const catalog = await listDynamicIndicatorCatalog();
    if (panelDisposed) return;
    dynamicCatalogEntries.value = catalog;
    if (isDynamicDoubleMaKind.value) ensureDynamicBuilderSeeded(true);
  } catch {
    if (panelDisposed) return;
    dynamicCatalogEntries.value = [];
  }
}

async function hydrateCatalogMeta(): Promise<void> {
  await Promise.all([
    loadMaParamCatalog(),
    loadMaSpeedups(),
    loadDynamicCatalogMeta(),
  ]);
  if (panelDisposed) return;
  syncMaParamStateFromVue();
}

watch(
  [maParamCatalogLoaded, selectedFastMaType, fastMaParamSchema],
  () => {
    if (!maParamCatalogLoaded.value) return;
    ensureSideTypeSchemaState(
      "fast",
      selectedFastMaType.value,
      fastMaParamSchema.value,
      optimizeStore.fastMaParams,
      optimizeStore.fastMaParamSweeps,
    );
    syncMaParamStateFromVue();
  },
  { immediate: true },
);

watch(
  [maParamCatalogLoaded, selectedSlowMaType, slowMaParamSchema],
  () => {
    if (!maParamCatalogLoaded.value) return;
    ensureSideTypeSchemaState(
      "slow",
      selectedSlowMaType.value,
      slowMaParamSchema.value,
      optimizeStore.slowMaParams,
      optimizeStore.slowMaParamSweeps,
    );
    syncMaParamStateFromVue();
  },
  { immediate: true },
);

watch(
  [() => optimizeStore.fastMaParams, () => optimizeStore.fastMaParamSweeps],
  () => {
    if (!maParamCatalogLoaded.value) return;
    const maId = selectedFastMaType.value;
    if (!maId || maParamTouchedBySide.value.fast[maId]) return;
    ensureSideTypeSchemaState(
      "fast",
      maId,
      fastMaParamSchema.value,
      optimizeStore.fastMaParams,
      optimizeStore.fastMaParamSweeps,
      true,
    );
  },
  { deep: true, immediate: true },
);

watch(
  [() => optimizeStore.slowMaParams, () => optimizeStore.slowMaParamSweeps],
  () => {
    if (!maParamCatalogLoaded.value) return;
    const maId = selectedSlowMaType.value;
    if (!maId || maParamTouchedBySide.value.slow[maId]) return;
    ensureSideTypeSchemaState(
      "slow",
      maId,
      slowMaParamSchema.value,
      optimizeStore.slowMaParams,
      optimizeStore.slowMaParamSweeps,
      true,
    );
  },
  { deep: true, immediate: true },
);

watch(
  [isDynamicDoubleMaKind, dynamicBuilderEligibleEntries],
  ([nextIsDynamic]) => {
    if (!nextIsDynamic) return;
    ensureDynamicBuilderSeeded();
  },
  { deep: true, immediate: true },
);

watch(
  dynamicBuilderState,
  (nextState) => {
    if (!isDynamicDoubleMaKind.value) return;
    optimizeStore.setDynamicStrategyBuilderStateJson(JSON.stringify(nextState));
  },
  { deep: true },
);

onMounted(async () => {
  await nextTick();
  if (hasTauriRuntime.value) {
    await waitForNextPaint();
    void hydrateCatalogMeta();
    optimizeStore.refreshGpuStatus(true);
    return;
  }
  await hydrateCatalogMeta();
  optimizeStore.refreshGpuStatus(true);
});

onBeforeUnmount(() => {
  flushQueuedInputValuesNow();
  panelDisposed = true;
});
</script>

<template>
  <Teleport v-if="mountReady" to="#optVueShellMount">
    <section class="optPanelVueRoot optVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="headCopy">
          <h2 class="title" style="margin: 0;">Optimize</h2>
          <div class="subtitle">
            Choose the strategy shape, execution route, and sweep ranges for the
            next run.
          </div>
        </div>
        <div class="meta mono">{{ stamp }}</div>
      </header>
      <div class="summaryChips" aria-label="Optimization summary">
        <span
          v-for="chip in optimizeSummaryChips"
          :key="`opt_summary_${chip}`"
          class="summaryChip mono"
          >{{ chip }}</span
        >
      </div>

      <section class="pane paneProminent">
        <div class="paneHeader">
          <div>
            <div class="sectionEyebrow">Setup</div>
            <h3>Strategy and objective</h3>
          </div>
        </div>
        <div class="gridTwo">
          <div>
            <label for="vueStrategyKind">Strategy</label>
            <select
              id="vueStrategyKind"
              :value="strategyDraft"
              @change="
                strategyDraft = String(
                  ($event.target as HTMLSelectElement).value || '',
                );
                setSelectValue('strategyKind', strategyDraft);
              "
            >
              <option
                v-for="option in strategyOptionItems"
                :key="`strategy_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
            <div class="muted">{{ strategyHintText }}</div>
          </div>

          <div>
            <label for="vuePresetProfile">Preset</label>
            <div class="inputWithAction">
              <select
                id="vuePresetProfile"
                :value="presetDraft"
                @change="
                  presetDraft = String(
                    ($event.target as HTMLSelectElement).value || '',
                  );
                  setSelectValue('presetProfile', presetDraft);
                "
              >
                <option
                  v-for="option in presetOptionItems"
                  :key="`preset_${option.value}_${option.label}`"
                  :value="option.value"
                  :disabled="option.disabled"
                >
                  {{ option.label }}
                </option>
              </select>
              <button
                type="button"
                :disabled="!canApplyPreset"
                @click="optimizeStore.applyPreset()"
              >
                Apply preset
              </button>
            </div>
          </div>
        </div>

        <div class="gridTwo">
          <div>
            <label for="vueObjective">Objective</label>
            <select
              id="vueObjective"
              :value="objectiveDraft"
              @change="
                objectiveDraft = String(
                  ($event.target as HTMLSelectElement).value || 'Sharpe',
                );
                setSelectValue('objective', objectiveDraft);
              "
            >
              <option value="Sharpe">Sharpe</option>
              <option value="Sortino">Sortino</option>
              <option value="Calmar">Calmar</option>
              <option value="Omega">Omega</option>
              <option value="Pnl">PnL</option>
              <option value="MaxDrawdown">Min drawdown</option>
            </select>
          </div>
          <div>
            <template v-if="showModeControl">
              <label for="vueMode">Mode</label>
              <select
                id="vueMode"
                :value="modeDraft"
                @change="
                  modeDraft = String(
                    ($event.target as HTMLSelectElement).value || 'Grid',
                  );
                  setSelectValue('mode', modeDraft);
                "
              >
                <option value="Grid">Grid</option>
              </select>
            </template>
            <template v-else>
              <label>Mode</label>
              <div class="muted mono">
                {{ strategyCapabilities.modeSummary }}
              </div>
            </template>
          </div>
        </div>
      </section>

      <section class="pane">
          <div class="paneHeader">
            <div>
              <div class="sectionEyebrow">Execution</div>
              <h3>{{ executionSectionTitle }}</h3>
            </div>
          </div>
          <div class="gridTwo">
            <div>
              <template v-if="showBackendControl">
                <label for="vueBackend">Backend</label>
                <select
                  id="vueBackend"
                  :value="backendDraft"
                  @change="
                    backendDraft = String(
                      ($event.target as HTMLSelectElement).value || '',
                    );
                    setSelectValue('backend', backendDraft);
                  "
                >
                  <option
                    v-for="option in backendOptionItems"
                    :key="`backend_${option.value}_${option.label}`"
                    :value="option.value"
                    :disabled="option.disabled"
                  >
                    {{ option.label }}
                  </option>
                </select>
              </template>
              <template v-else>
                <label>Backend</label>
                <div class="muted mono">
                  {{ strategyCapabilities.executionSummary }}
                </div>
              </template>
              <div class="muted mono">{{ backendExecutionHint }}</div>
            </div>
            <div>
              <label>CUDA device</label>
              <div class="muted mono">{{ gpuDeviceMeta }}</div>
            </div>
          </div>
          <div v-if="showGpuPresetActions" class="actions">
            <button
              type="button"
              :disabled="!canPresetKernel"
              @click="optimizeStore.presetKernel()"
            >
              Preset GPU kernel
            </button>
            <button
              type="button"
              :disabled="!canRefreshGpuStatus"
              @click="optimizeStore.refreshGpuStatus()"
            >
              Refresh GPU status
            </button>
            <button
              type="button"
              :disabled="!canParityCheck"
              @click="optimizeStore.runGpuParityCheck()"
            >
              Parity check
            </button>
          </div>
          <div v-else class="actions">
            <button
              type="button"
              :disabled="!canRefreshGpuStatus"
              @click="optimizeStore.refreshGpuStatus()"
            >
              Refresh GPU status
            </button>
          </div>
          <div class="muted mono">{{ gpuStatus }}</div>
          <div class="muted mono">{{ gpuParityMeta }}</div>
          <div class="muted mono">{{ dynamicCatalogMeta }}</div>
        </section>

        <section v-if="isDynamicDoubleMaKind" class="pane">
          <div class="paneHeader">
            <div>
              <div class="sectionEyebrow">Dynamic IR</div>
              <h3>Constrained Dynamic Double-MA builder</h3>
            </div>
          </div>
          <div class="muted">
            Build a constrained <code>strategy_ir</code> for
            <code>profile=DoubleMa</code>, then preview the resolved route
            before running. The builder supports indicator, price, and
            numeric-constant operands; use the advanced JSON box for lower-level
            edge cases.
          </div>
          <div v-if="!dynamicPreviewAvailable" class="builderNotice muted">
            Desktop-only preview commands are unavailable in this plain browser
            session. You can still assemble the builder state and review the
            generated JSON here.
          </div>
          <div class="gridTwo">
            <label class="inlineToggle">
              <input
                type="checkbox"
                :checked="dynamicBuilderState.longOnly"
                @change="
                  updateDynamicBuilderLongOnly(
                    !!($event.target as HTMLInputElement).checked,
                  )
                "
              />
              Long only
            </label>
            <label class="inlineToggle">
              <input
                type="checkbox"
                :checked="dynamicBuilderState.allowFlip"
                :disabled="dynamicBuilderState.longOnly"
                @change="
                  updateDynamicBuilderAllowFlip(
                    !!($event.target as HTMLInputElement).checked,
                  )
                "
              />
              Allow flip
            </label>
          </div>
          <div v-if="dynamicBuilderState.longOnly" class="muted mono">
            Long-only mode keeps the builder focused on long entry and exit
            logic. Disable it to restore the default short-side conditions and
            groups.
          </div>
          <div class="gridTwo">
            <div>
              <label for="vueDynamicStopLossPct">Stop loss %</label>
              <input
                id="vueDynamicStopLossPct"
                type="number"
                min="0"
                step="0.001"
                :value="dynamicBuilderState.stopLossPct"
                @input="
                  dynamicBuilderState.stopLossPct = String(
                    ($event.target as HTMLInputElement).value || '',
                  )
                "
              />
            </div>
            <div>
              <label for="vueDynamicTakeProfitPct">Take profit %</label>
              <input
                id="vueDynamicTakeProfitPct"
                type="number"
                min="0"
                step="0.001"
                :value="dynamicBuilderState.takeProfitPct"
                @input="
                  dynamicBuilderState.takeProfitPct = String(
                    ($event.target as HTMLInputElement).value || '',
                  )
                "
              />
            </div>
          </div>
          <div class="dynamicBuilderSplit">
            <div class="rangeCard">
              <div class="rangeTitle">Extra indicators</div>
              <div
                v-if="dynamicBuilderEligibleEntries.length === 0"
                class="muted mono"
              >
                No builder-eligible floating indicators are available in the
                catalog.
              </div>
              <div
                v-for="slot in dynamicBuilderState.indicators"
                :key="`dyn_builder_indicator_${slot.id}`"
                class="maParamBlock"
              >
                <div class="rowBetween">
                  <div class="muted mono">{{ slot.id }}</div>
                  <button
                    type="button"
                    @click="removeDynamicBuilderIndicator(slot.id)"
                  >
                    Remove
                  </button>
                </div>
                <label :for="`dyn_indicator_${slot.id}`">Indicator</label>
                <select
                  :id="`dyn_indicator_${slot.id}`"
                  :value="slot.indicatorId"
                  @change="
                    updateDynamicBuilderIndicator(
                      slot.id,
                      String(($event.target as HTMLSelectElement).value || ''),
                    )
                  "
                >
                  <option
                    v-for="entry in dynamicBuilderEligibleEntries"
                    :key="`dyn_indicator_option_${slot.id}_${entry.id}`"
                    :value="entry.id"
                  >
                    {{ entry.label }}
                  </option>
                </select>
                <label :for="`dyn_indicator_output_${slot.id}`">Output</label>
                <select
                  :id="`dyn_indicator_output_${slot.id}`"
                  :value="slot.outputId"
                  @change="
                    slot.outputId = String(
                      ($event.target as HTMLSelectElement).value || '',
                    )
                  "
                >
                  <option
                    v-for="output in dynamicBuilderOutputOptions(slot.id)"
                    :key="`dyn_indicator_output_option_${slot.id}_${output.value}`"
                    :value="output.value"
                  >
                    {{ output.label }}
                  </option>
                </select>
                <template
                  v-if="dynamicBuilderParamDescriptors(slot.id).length > 0"
                >
                  <div
                    v-for="descriptor in dynamicBuilderParamDescriptors(
                      slot.id,
                    )"
                    :key="`dyn_indicator_param_${slot.id}_${descriptor.key}`"
                    class="maParamBlock"
                  >
                    <template v-if="descriptor.kind === 'bool'">
                      <label
                        class="inlineToggle"
                        :for="`dyn_indicator_param_${slot.id}_${descriptor.key}`"
                      >
                        <input
                          :id="`dyn_indicator_param_${slot.id}_${descriptor.key}`"
                          type="checkbox"
                          :checked="
                            Boolean(
                              dynamicBuilderParamValue(slot.id, descriptor),
                            )
                          "
                          @change="
                            updateDynamicBuilderParam(
                              slot.id,
                              descriptor,
                              !!($event.target as HTMLInputElement).checked,
                            )
                          "
                        />
                        {{ descriptor.label }}
                      </label>
                    </template>
                    <template v-else-if="descriptor.kind === 'enum_string'">
                      <label
                        :for="`dyn_indicator_param_${slot.id}_${descriptor.key}`"
                        >{{ descriptor.label }}</label
                      >
                      <select
                        :id="`dyn_indicator_param_${slot.id}_${descriptor.key}`"
                        :value="
                          String(
                            dynamicBuilderParamValue(slot.id, descriptor) || '',
                          )
                        "
                        @change="
                          updateDynamicBuilderParam(
                            slot.id,
                            descriptor,
                            String(
                              ($event.target as HTMLSelectElement).value || '',
                            ),
                          )
                        "
                      >
                        <option
                          v-for="value in descriptor.enumValues"
                          :key="`dyn_indicator_param_option_${slot.id}_${descriptor.key}_${value}`"
                          :value="value"
                        >
                          {{ value }}
                        </option>
                      </select>
                    </template>
                    <template v-else>
                      <label
                        :for="`dyn_indicator_param_${slot.id}_${descriptor.key}`"
                        >{{ descriptor.label }}</label
                      >
                      <input
                        :id="`dyn_indicator_param_${slot.id}_${descriptor.key}`"
                        type="number"
                        :min="descriptor.min ?? undefined"
                        :max="descriptor.max ?? undefined"
                        :step="
                          descriptor.step ??
                          (descriptor.kind === 'int' ? 1 : 'any')
                        "
                        :value="dynamicBuilderParamValue(slot.id, descriptor)"
                        @change="
                          updateDynamicBuilderParam(
                            slot.id,
                            descriptor,
                            String(
                              ($event.target as HTMLInputElement).value || '',
                            ),
                          )
                        "
                      />
                    </template>
                    <div v-if="descriptor.notes" class="muted mono">
                      {{ descriptor.notes }}
                    </div>
                  </div>
                </template>
                <div v-else class="muted mono">Uses catalog defaults only.</div>
              </div>
              <div class="actions">
                <button
                  type="button"
                  :disabled="dynamicBuilderEligibleEntries.length === 0"
                  @click="addDynamicBuilderIndicator"
                >
                  Add indicator
                </button>
              </div>
            </div>
            <div class="rangeCard">
              <div class="rangeTitle">Conditions</div>
              <div
                v-for="condition in dynamicBuilderState.conditions"
                :key="`dyn_builder_condition_${condition.id}`"
                class="maParamBlock"
              >
                <div class="rowBetween">
                  <div class="muted mono">{{ condition.id }}</div>
                  <button
                    type="button"
                    @click="removeDynamicCondition(condition.id)"
                  >
                    Remove
                  </button>
                </div>
                <label :for="`dyn_condition_op_${condition.id}`"
                  >Operation</label
                >
                <select
                  :id="`dyn_condition_op_${condition.id}`"
                  :value="condition.op"
                  @change="
                    updateDynamicConditionOp(
                      condition.id,
                      String(
                        ($event.target as HTMLSelectElement).value || 'gt',
                      ),
                    )
                  "
                >
                  <option value="gt">Greater than</option>
                  <option value="ge">Greater or equal</option>
                  <option value="lt">Less than</option>
                  <option value="le">Less or equal</option>
                  <option value="cross_up">Cross up</option>
                  <option value="cross_down">Cross down</option>
                </select>
                <label :for="`dyn_condition_lhs_${condition.id}`"
                  >Left operand</label
                >
                <select
                  :id="`dyn_condition_lhs_${condition.id}`"
                  :value="dynamicOperandSelectValue(condition.lhs)"
                  @change="
                    updateDynamicConditionOperandSelection(
                      condition.id,
                      'lhs',
                      String(
                        ($event.target as HTMLSelectElement).value || 'fast_ma',
                      ),
                    )
                  "
                >
                  <option
                    v-for="option in dynamicBuilderIndicatorOperandOptions"
                    :key="`dyn_condition_lhs_option_${condition.id}_${option.value}`"
                    :value="option.value"
                  >
                    {{ option.label }}
                  </option>
                </select>
                <input
                  v-if="condition.lhs.kind === 'constant'"
                  :id="`dyn_condition_lhs_constant_${condition.id}`"
                  type="number"
                  step="any"
                  :value="condition.lhs.value"
                  @input="
                    updateDynamicConditionConstantValue(
                      condition.id,
                      'lhs',
                      String(($event.target as HTMLInputElement).value || ''),
                    )
                  "
                />
                <label :for="`dyn_condition_rhs_${condition.id}`"
                  >Right operand</label
                >
                <select
                  :id="`dyn_condition_rhs_${condition.id}`"
                  :value="dynamicOperandSelectValue(condition.rhs)"
                  @change="
                    updateDynamicConditionOperandSelection(
                      condition.id,
                      'rhs',
                      String(
                        ($event.target as HTMLSelectElement).value || 'slow_ma',
                      ),
                    )
                  "
                >
                  <option
                    v-for="option in dynamicBuilderIndicatorOperandOptions"
                    :key="`dyn_condition_rhs_option_${condition.id}_${option.value}`"
                    :value="option.value"
                  >
                    {{ option.label }}
                  </option>
                </select>
                <input
                  v-if="condition.rhs.kind === 'constant'"
                  :id="`dyn_condition_rhs_constant_${condition.id}`"
                  type="number"
                  step="any"
                  :value="condition.rhs.value"
                  @input="
                    updateDynamicConditionConstantValue(
                      condition.id,
                      'rhs',
                      String(($event.target as HTMLInputElement).value || ''),
                    )
                  "
                />
              </div>
              <div class="actions">
                <button type="button" @click="addDynamicCondition">
                  Add condition
                </button>
              </div>
              <div class="muted mono">
                Builder operands support indicators, close, MA source, and
                numeric constants.
              </div>
            </div>
          </div>
          <div class="dynamicBuilderSplit">
            <div class="rangeCard">
              <div class="rangeTitle">Groups</div>
              <div
                v-for="group in dynamicBuilderState.groups"
                :key="`dyn_builder_group_${group.id}`"
                class="maParamBlock"
              >
                <div class="rowBetween">
                  <div class="muted mono">{{ group.id }}</div>
                  <button type="button" @click="removeDynamicGroup(group.id)">
                    Remove
                  </button>
                </div>
                <label :for="`dyn_group_mode_${group.id}`">Mode</label>
                <select
                  :id="`dyn_group_mode_${group.id}`"
                  :value="group.mode"
                  @change="
                    updateDynamicGroupMode(
                      group.id,
                      String(
                        ($event.target as HTMLSelectElement).value || 'all',
                      ),
                    )
                  "
                >
                  <option value="all">All</option>
                  <option value="any">Any</option>
                </select>
                <div class="muted mono">Conditions</div>
                <label
                  v-for="condition in dynamicBuilderConditionOptions"
                  :key="`dyn_group_condition_${group.id}_${condition.value}`"
                  class="inlineToggle"
                >
                  <input
                    type="checkbox"
                    :checked="group.conditionIds.includes(condition.value)"
                    @change="
                      toggleDynamicGroupCondition(
                        group.id,
                        condition.value,
                        !!($event.target as HTMLInputElement).checked,
                      )
                    "
                  />
                  {{ condition.label }}
                </label>
              </div>
              <div class="actions">
                <button type="button" @click="addDynamicGroup">
                  Add group
                </button>
              </div>
            </div>
            <div class="rangeCard">
              <div class="rangeTitle">Signal bindings</div>
              <label for="dyn_long_entry_group">Long entry</label>
              <select
                id="dyn_long_entry_group"
                :value="dynamicBuilderState.signals.longEntryGroup"
                @change="
                  dynamicBuilderState.signals.longEntryGroup = String(
                    ($event.target as HTMLSelectElement).value || '',
                  )
                "
              >
                <option value="">—</option>
                <option
                  v-for="group in dynamicBuilderGroupOptions"
                  :key="`dyn_signal_long_entry_${group.value}`"
                  :value="group.value"
                >
                  {{ group.label }}
                </option>
              </select>
              <label for="dyn_long_exit_group">Long exit</label>
              <select
                id="dyn_long_exit_group"
                :value="dynamicBuilderState.signals.longExitGroup"
                @change="
                  dynamicBuilderState.signals.longExitGroup = String(
                    ($event.target as HTMLSelectElement).value || '',
                  )
                "
              >
                <option value="">—</option>
                <option
                  v-for="group in dynamicBuilderGroupOptions"
                  :key="`dyn_signal_long_exit_${group.value}`"
                  :value="group.value"
                >
                  {{ group.label }}
                </option>
              </select>
              <template v-if="!dynamicBuilderState.longOnly">
                <label for="dyn_short_entry_group">Short entry</label>
                <select
                  id="dyn_short_entry_group"
                  :value="dynamicBuilderState.signals.shortEntryGroup"
                  @change="
                    dynamicBuilderState.signals.shortEntryGroup = String(
                      ($event.target as HTMLSelectElement).value || '',
                    )
                  "
                >
                  <option value="">—</option>
                  <option
                    v-for="group in dynamicBuilderGroupOptions"
                    :key="`dyn_signal_short_entry_${group.value}`"
                    :value="group.value"
                  >
                    {{ group.label }}
                  </option>
                </select>
                <label for="dyn_short_exit_group">Short exit</label>
                <select
                  id="dyn_short_exit_group"
                  :value="dynamicBuilderState.signals.shortExitGroup"
                  @change="
                    dynamicBuilderState.signals.shortExitGroup = String(
                      ($event.target as HTMLSelectElement).value || '',
                    )
                  "
                >
                  <option value="">—</option>
                  <option
                    v-for="group in dynamicBuilderGroupOptions"
                    :key="`dyn_signal_short_exit_${group.value}`"
                    :value="group.value"
                  >
                    {{ group.label }}
                  </option>
                </select>
              </template>
            </div>
          </div>
          <div class="actions">
            <button
              type="button"
              :disabled="!!dynamicBuilderError || !dynamicPreviewAvailable"
              :title="dynamicBuilderError || dynamicPreviewDisabledReason"
              @click="previewDynamicStrategyFromBuilderAction"
            >
              Preview Builder IR
            </button>
          </div>
          <div class="muted mono">
            {{ dynamicBuilderError || "Builder JSON ready." }}
          </div>
          <pre v-if="dynamicBuilderPrettyJson" class="jsonPreview mono">{{
            dynamicBuilderPrettyJson
          }}</pre>
        </section>

        <section v-if="isDynamicDoubleMaKind" class="pane">
          <div class="paneHeader">
            <div>
              <div class="sectionEyebrow">Advanced</div>
              <h3>Manual JSON override</h3>
            </div>
          </div>
          <div class="muted">
            Optional advanced path. Paste or edit raw
            <code>strategy_ir</code> JSON directly, then preview that exact
            payload.
          </div>
          <label for="vueDynamicStrategyIrJson">Strategy IR JSON</label>
          <textarea
            id="vueDynamicStrategyIrJson"
            rows="12"
            spellcheck="false"
            :value="dynamicStrategyIrJsonDraft"
            @input="
              dynamicStrategyIrJsonDraft = String(
                ($event.target as HTMLTextAreaElement).value || '',
              );
              optimizeStore.setDynamicStrategyIrJson(
                dynamicStrategyIrJsonDraft,
              );
            "
          ></textarea>
          <div class="actions">
            <button
              type="button"
              :disabled="!dynamicPreviewAvailable"
              :title="dynamicPreviewDisabledReason"
              @click="previewDynamicStrategyIrAction"
            >
              Preview JSON
            </button>
          </div>
          <div class="muted mono">{{ dynamicStrategyPreviewMeta }}</div>
          <div class="muted mono">{{ dynamicStrategyPreviewReasons }}</div>
          <pre
            v-if="dynamicStrategyPreviewPrettyJson"
            class="jsonPreview mono"
            >{{ dynamicStrategyPreviewPrettyJson }}</pre
          >
        </section>

        <section v-if="showSearchSpaceControls" class="pane">
          <div class="paneHeader">
            <div>
              <div class="sectionEyebrow">Search Space</div>
              <h3>Fast and slow sweep ranges</h3>
            </div>
            <div class="meta mono">{{ pairsPreview }}</div>
          </div>
          <div class="split">
            <div class="rangeCard">
              <div class="rangeTitle">Fast side</div>
              <div class="rowThree">
                <div>
                  <label for="vueFastStart">Start</label>
                  <input
                    id="vueFastStart"
                    type="number"
                    min="1"
                    step="1"
                    :value="fastStartDraft"
                    @input="
                      fastStartDraft = String(
                        ($event.target as HTMLInputElement).value || '',
                      );
                      queueInputValue('fastStart', fastStartDraft);
                    "
                  />
                </div>
                <div>
                  <label for="vueFastEnd">End</label>
                  <input
                    id="vueFastEnd"
                    type="number"
                    min="1"
                    step="1"
                    :value="fastEndDraft"
                    @input="
                      fastEndDraft = String(
                        ($event.target as HTMLInputElement).value || '',
                      );
                      queueInputValue('fastEnd', fastEndDraft);
                    "
                  />
                </div>
                <div>
                  <label for="vueFastStep">Step</label>
                  <input
                    id="vueFastStep"
                    type="number"
                    min="0"
                    step="1"
                    :value="fastStepDraft"
                    @input="
                      fastStepDraft = String(
                        ($event.target as HTMLInputElement).value || '',
                      );
                      queueInputValue('fastStep', fastStepDraft);
                    "
                  />
                </div>
              </div>
              <label for="vueFastType">Fast MA type</label>
              <select
                id="vueFastType"
                :value="fastTypeDraft"
                @change="
                  fastTypeDraft = String(
                    ($event.target as HTMLSelectElement).value || '',
                  );
                  setSelectValue('fastMaType', fastTypeDraft);
                "
              >
                <option
                  v-for="option in fastTypeOptionItems"
                  :key="`fasttype_${option.value}_${option.label}`"
                  :value="option.value"
                  :disabled="option.disabled"
                >
                  {{ option.label }}
                </option>
              </select>
              <div v-if="fastMaBackendHint" class="muted mono">
                {{ fastMaBackendHint }}
              </div>
            </div>

            <div class="rangeCard">
              <div class="rangeTitle">Slow side</div>
              <div class="rowThree">
                <div>
                  <label for="vueSlowStart">Start</label>
                  <input
                    id="vueSlowStart"
                    type="number"
                    min="1"
                    step="1"
                    :value="slowStartDraft"
                    @input="
                      slowStartDraft = String(
                        ($event.target as HTMLInputElement).value || '',
                      );
                      queueInputValue('slowStart', slowStartDraft);
                    "
                  />
                </div>
                <div>
                  <label for="vueSlowEnd">End</label>
                  <input
                    id="vueSlowEnd"
                    type="number"
                    min="1"
                    step="1"
                    :value="slowEndDraft"
                    @input="
                      slowEndDraft = String(
                        ($event.target as HTMLInputElement).value || '',
                      );
                      queueInputValue('slowEnd', slowEndDraft);
                    "
                  />
                </div>
                <div>
                  <label for="vueSlowStep">Step</label>
                  <input
                    id="vueSlowStep"
                    type="number"
                    min="0"
                    step="1"
                    :value="slowStepDraft"
                    @input="
                      slowStepDraft = String(
                        ($event.target as HTMLInputElement).value || '',
                      );
                      queueInputValue('slowStep', slowStepDraft);
                    "
                  />
                </div>
              </div>
              <label for="vueSlowType">Slow MA type</label>
              <select
                id="vueSlowType"
                :value="slowTypeDraft"
                @change="
                  slowTypeDraft = String(
                    ($event.target as HTMLSelectElement).value || '',
                  );
                  setSelectValue('slowMaType', slowTypeDraft);
                "
              >
                <option
                  v-for="option in slowTypeOptionItems"
                  :key="`slowtype_${option.value}_${option.label}`"
                  :value="option.value"
                  :disabled="option.disabled"
                >
                  {{ option.label }}
                </option>
              </select>
              <div v-if="slowMaBackendHint" class="muted mono">
                {{ slowMaBackendHint }}
              </div>
            </div>
          </div>
        </section>

        <section class="pane">
          <div class="paneHeader">
            <div>
              <div class="sectionEyebrow">Actions</div>
              <h3>Run from this panel</h3>
            </div>
          </div>
          <div class="actions">
            <button
              type="button"
              class="primary"
              :disabled="runDisabled"
              @click="runOptimizationAction"
            >
              Run optimization
            </button>
            <button
              type="button"
              :disabled="validateDisabled"
              @click="runValidationAction"
            >
              Run validation
            </button>
          </div>
          <div class="muted mono">{{ progressMeta || "—" }}</div>
          <div class="muted mono" role="status" aria-live="polite">{{ statusLine }}</div>
        </section>

        <section class="pane">
          <div class="paneHeader">
            <div>
              <div class="sectionEyebrow">Advanced</div>
              <h3>MA parameters</h3>
            </div>
          </div>
          <div class="split">
            <div class="maParamSide">
              <div class="muted maParamTitle">Fast MA parameters</div>
              <div v-if="fastRenderedMaParams.length === 0" class="muted mono">
                No extra params for the selected MA.
              </div>
              <div
                v-for="entry in fastRenderedMaParams"
                :key="`fast_ma_param_${entry.schema.key}`"
                class="maParamBlock"
              >
                <label :for="maParamInputId('fast', entry.schema.key)">{{
                  entry.schema.label
                }}</label>
                <input
                  :id="maParamInputId('fast', entry.schema.key)"
                  type="number"
                  :min="entry.schema.min ?? undefined"
                  :max="entry.schema.max ?? undefined"
                  :step="resolveInputStep(entry.schema.step, entry.schema.kind)"
                  :value="entry.state.value"
                  @input="
                    updateMaParamValue(
                      'fast',
                      entry.schema.key,
                      String(($event.target as HTMLInputElement).value || ''),
                    )
                  "
                />
                <label class="maParamSweepToggle">
                  <input
                    type="checkbox"
                    :checked="entry.state.sweepEnabled"
                    @change="
                      updateMaParamSweepEnabled(
                        'fast',
                        entry.schema.key,
                        !!($event.target as HTMLInputElement).checked,
                      )
                    "
                  />
                  Optimize (sweep)
                </label>
                <div
                  class="rowThree maParamSweepRow"
                  v-show="entry.state.sweepEnabled"
                >
                  <div>
                    <label
                      :for="`${maParamInputId('fast', entry.schema.key)}_sweep_start`"
                      >Start</label
                    >
                    <input
                      :id="`${maParamInputId('fast', entry.schema.key)}_sweep_start`"
                      type="number"
                      :min="entry.schema.min ?? undefined"
                      :max="entry.schema.max ?? undefined"
                      :step="
                        resolveInputStep(entry.schema.step, entry.schema.kind)
                      "
                      :value="entry.state.sweepStart"
                      @input="
                        updateMaParamSweepField(
                          'fast',
                          entry.schema.key,
                          'sweepStart',
                          String(
                            ($event.target as HTMLInputElement).value || '',
                          ),
                        )
                      "
                    />
                  </div>
                  <div>
                    <label
                      :for="`${maParamInputId('fast', entry.schema.key)}_sweep_end`"
                      >End</label
                    >
                    <input
                      :id="`${maParamInputId('fast', entry.schema.key)}_sweep_end`"
                      type="number"
                      :min="entry.schema.min ?? undefined"
                      :max="entry.schema.max ?? undefined"
                      :step="
                        resolveInputStep(entry.schema.step, entry.schema.kind)
                      "
                      :value="entry.state.sweepEnd"
                      @input="
                        updateMaParamSweepField(
                          'fast',
                          entry.schema.key,
                          'sweepEnd',
                          String(
                            ($event.target as HTMLInputElement).value || '',
                          ),
                        )
                      "
                    />
                  </div>
                  <div>
                    <label
                      :for="`${maParamInputId('fast', entry.schema.key)}_sweep_step`"
                      >Step</label
                    >
                    <input
                      :id="`${maParamInputId('fast', entry.schema.key)}_sweep_step`"
                      type="number"
                      min="0"
                      :step="
                        resolveInputStep(entry.schema.step, entry.schema.kind)
                      "
                      :value="entry.state.sweepStep"
                      @input="
                        updateMaParamSweepField(
                          'fast',
                          entry.schema.key,
                          'sweepStep',
                          String(
                            ($event.target as HTMLInputElement).value || '',
                          ),
                        )
                      "
                    />
                  </div>
                </div>
                <div v-if="entry.schema.notes" class="muted maParamNote">
                  {{ entry.schema.notes }}
                </div>
              </div>
            </div>
            <div class="maParamSide">
              <div class="muted maParamTitle">Slow MA parameters</div>
              <div v-if="slowRenderedMaParams.length === 0" class="muted mono">
                No extra params for the selected MA.
              </div>
              <div
                v-for="entry in slowRenderedMaParams"
                :key="`slow_ma_param_${entry.schema.key}`"
                class="maParamBlock"
              >
                <label :for="maParamInputId('slow', entry.schema.key)">{{
                  entry.schema.label
                }}</label>
                <input
                  :id="maParamInputId('slow', entry.schema.key)"
                  type="number"
                  :min="entry.schema.min ?? undefined"
                  :max="entry.schema.max ?? undefined"
                  :step="resolveInputStep(entry.schema.step, entry.schema.kind)"
                  :value="entry.state.value"
                  @input="
                    updateMaParamValue(
                      'slow',
                      entry.schema.key,
                      String(($event.target as HTMLInputElement).value || ''),
                    )
                  "
                />
                <label class="maParamSweepToggle">
                  <input
                    type="checkbox"
                    :checked="entry.state.sweepEnabled"
                    @change="
                      updateMaParamSweepEnabled(
                        'slow',
                        entry.schema.key,
                        !!($event.target as HTMLInputElement).checked,
                      )
                    "
                  />
                  Optimize (sweep)
                </label>
                <div
                  class="rowThree maParamSweepRow"
                  v-show="entry.state.sweepEnabled"
                >
                  <div>
                    <label
                      :for="`${maParamInputId('slow', entry.schema.key)}_sweep_start`"
                      >Start</label
                    >
                    <input
                      :id="`${maParamInputId('slow', entry.schema.key)}_sweep_start`"
                      type="number"
                      :min="entry.schema.min ?? undefined"
                      :max="entry.schema.max ?? undefined"
                      :step="
                        resolveInputStep(entry.schema.step, entry.schema.kind)
                      "
                      :value="entry.state.sweepStart"
                      @input="
                        updateMaParamSweepField(
                          'slow',
                          entry.schema.key,
                          'sweepStart',
                          String(
                            ($event.target as HTMLInputElement).value || '',
                          ),
                        )
                      "
                    />
                  </div>
                  <div>
                    <label
                      :for="`${maParamInputId('slow', entry.schema.key)}_sweep_end`"
                      >End</label
                    >
                    <input
                      :id="`${maParamInputId('slow', entry.schema.key)}_sweep_end`"
                      type="number"
                      :min="entry.schema.min ?? undefined"
                      :max="entry.schema.max ?? undefined"
                      :step="
                        resolveInputStep(entry.schema.step, entry.schema.kind)
                      "
                      :value="entry.state.sweepEnd"
                      @input="
                        updateMaParamSweepField(
                          'slow',
                          entry.schema.key,
                          'sweepEnd',
                          String(
                            ($event.target as HTMLInputElement).value || '',
                          ),
                        )
                      "
                    />
                  </div>
                  <div>
                    <label
                      :for="`${maParamInputId('slow', entry.schema.key)}_sweep_step`"
                      >Step</label
                    >
                    <input
                      :id="`${maParamInputId('slow', entry.schema.key)}_sweep_step`"
                      type="number"
                      min="0"
                      :step="
                        resolveInputStep(entry.schema.step, entry.schema.kind)
                      "
                      :value="entry.state.sweepStep"
                      @input="
                        updateMaParamSweepField(
                          'slow',
                          entry.schema.key,
                          'sweepStep',
                          String(
                            ($event.target as HTMLInputElement).value || '',
                          ),
                        )
                      "
                    />
                  </div>
                </div>
                <div v-if="entry.schema.notes" class="muted maParamNote">
                  {{ entry.schema.notes }}
                </div>
              </div>
            </div>
          </div>
        </section>
    </section>
  </Teleport>
</template>

<style scoped>
.optPanelVueRoot {
  display: grid;
  gap: 0.85rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.headCopy {
  display: grid;
  gap: 0.18rem;
}

.title {
  font-size: 1rem;
  font-weight: 650;
}

.subtitle {
  font-size: 0.82rem;
  line-height: 1.35;
  color: var(--muted);
}

.meta {
  font-size: 0.78rem;
  color: var(--muted);
}

.summaryChips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.42rem;
}

.summaryChip {
  border: 1px solid var(--border);
  border-radius: 999px;
  background: color-mix(in oklab, var(--panel) 86%, var(--surface));
  padding: 0.22rem 0.52rem;
  font-size: 0.72rem;
  color: var(--muted);
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.paneProminent {
  background: linear-gradient(
    135deg,
    color-mix(in oklab, var(--accent-soft) 60%, var(--panel-footer)),
    color-mix(in oklab, var(--panel-footer) 92%, var(--surface))
  );
}

.paneHeader {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
  margin-bottom: 0.65rem;
}

.sectionEyebrow {
  font-size: 0.69rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--muted);
}

.pane h3 {
  margin: 0.12rem 0 0;
  font-size: 0.92rem;
}

.gridTwo {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
  gap: 0.7rem;
}

.gridTwo + .gridTwo {
  margin-top: 0.7rem;
}

.split {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
  gap: 0.8rem;
}

.dynamicBuilderSplit {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 0.8rem;
}

.rangeCard {
  border: 1px solid color-mix(in oklab, var(--border) 84%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface) 80%, transparent);
  padding: 0.7rem;
}

.rangeTitle {
  margin-bottom: 0.55rem;
  font-size: 0.78rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.rowThree {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
  gap: 0.55rem;
}

.inputWithAction {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.45rem;
}

.actions {
  margin-top: 0.6rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.actions .primary {
  border-color: color-mix(in oklab, var(--accent) 70%, var(--border));
}

.maParamSide {
  min-width: 0;
}

.maParamTitle {
  margin-bottom: 0.35rem;
}

.maParamBlock {
  margin-top: 0.6rem;
}

.rowBetween {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.5rem;
}

.inlineToggle {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
}

.maParamSweepToggle {
  margin-top: 0.35rem;
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
}

.maParamSweepRow {
  margin-top: 0.35rem;
}

.maParamNote {
  margin-top: 0.25rem;
  font-size: 0.78rem;
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
  margin-top: 0.4rem;
}

.builderNotice {
  padding: 0.55rem 0.65rem;
  border: 1px dashed color-mix(in oklab, var(--accent) 28%, var(--border));
  border-radius: 10px;
  background: color-mix(in oklab, var(--surface) 84%, var(--panel-footer));
}

.jsonPreview {
  margin-top: 0.6rem;
  padding: 0.7rem;
  border: 1px solid color-mix(in oklab, var(--border) 84%, transparent);
  border-radius: 10px;
  background: color-mix(in oklab, var(--surface) 86%, transparent);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 18rem;
  overflow: auto;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 980px) {
  .gridTwo,
  .split,
  .dynamicBuilderSplit,
  .rowThree,
  .inputWithAction {
    grid-template-columns: 1fr;
  }

  .paneHeader {
    flex-direction: column;
  }
}
</style>

<style>
[data-vue-panel="opt"][data-vue-opt="1"] > :not(#optVueShellMount) {
  display: none !important;
}
</style>
