import { defineStore } from "pinia";
import type {
  AnalyzeOptionsExpressionFeasibilitySummary,
  AnalyzeOptionsExpressionValidationFeasibilitySummary,
  OptionsDatasetManifestSummary,
  OptionsExpressionArtifactRecord,
  OptionsExpressionArtifactSummary,
  OptionsExpressionDatasetProvenanceSummary,
  OptionsExpressionEventPathInspectionSummary,
  OptionsExpressionTupleInspectionSummary,
  ValidateOptionsExpressionArtifactSummary,
  PolygonOptionsDatasetDownloadSummary,
  RunOptionsExpressionSummary,
} from "../services/optionsExpressionService";
import {
  analyzeOptionsExpressionFeasibility,
  analyzeOptionsExpressionValidationFeasibility,
  downloadPolygonOptionsDataset,
  exportOptionsExpressionCsv,
  exportOptionsExpressionEventPathCsv,
  exportOptionsExpressionInspectionCsv,
  importOptionsDataset,
  inspectOptionsExpressionEventPath,
  inspectOptionsExpressionTuple,
  listOptionsDatasets,
  listOptionsExpressionArtifacts,
  loadOptionsExpressionArtifact,
  pickOptionsCsvPath,
  revealOptionsPath,
  runOptionsExpression,
  validateOptionsExpressionArtifact,
} from "../services/optionsExpressionService";
import {
  createOptionsValidationFormValues,
  optionsValidationErrorMessages,
  toOptionsValidationCommandRequest,
  validateOptionsValidationFormValues,
  type OptionsValidationFormValues,
} from "../utils/optionsValidationForm";

export const OPTIONS_MONEYNESS_BUCKET_OPTIONS = [
  "atm",
  "otm_1",
  "otm_2",
  "itm_1",
] as const;
export const OPTIONS_EXIT_TEMPLATE_OPTIONS = [
  "fixed_hold",
  "fixed_hold_or_target_stop",
  "end_of_day_or_target_stop",
  "base_exit_or_target_stop",
] as const;
export const OPTIONS_FILL_MODEL_OPTIONS = [
  "aggregate_research",
  "quote_conservative",
  "quote_mid",
] as const;
export const OPTIONS_STRUCTURE_FAMILY_OPTIONS = [
  "long_single",
  "debit_vertical",
  "long_pair",
] as const;
export const OPTIONS_BACKEND_PREFERENCE_OPTIONS = [
  "auto",
  "cpu_reference",
  "gpu_compiled_tensor",
] as const;
export const OPTIONS_OBJECTIVE_PROFILE_OPTIONS = ["balanced"] as const;
export const OPTIONS_SPLIT_ASSIGNMENT_MODE_OPTIONS = [
  "event_chronological",
  "session_chronological",
] as const;
export const OPTIONS_VALIDATION_DATASET_POLICY_OPTIONS = [
  "require_exact_replay",
  "allow_same_root_replacement",
  "allow_explicit_cross_lane_comparison",
] as const;
export const OPTIONS_VALIDATION_PLAN_MODE_OPTIONS = [
  "original_embedded_splits",
  "session_chronological_replay",
  "date_range_holdout",
  "anchored_walk_forward",
  "rolling_session_walk_forward",
] as const;
export const OPTIONS_IMPORT_PRICE_SEMANTICS_OPTIONS = [
  "minute_aggregate",
  "top_of_book_quote",
  "derived_mid_snapshot",
] as const;
export const OPTIONS_REALISM_LANE_OPTIONS = [
  "aggregate",
  "derived_mid",
  "quote_aware",
] as const;
export const OPTIONS_SUPPORTED_PARENT_STRATEGIES = [
  "DoubleMa",
  "CompositeTrend",
  "RegimeHybrid",
  "BreakoutMomentum",
] as const;

function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
}

function isSupportedParentStrategyKind(value: unknown): boolean {
  const normalized = normalizeText(value);
  return (OPTIONS_SUPPORTED_PARENT_STRATEGIES as readonly string[]).includes(
    normalized,
  );
}

export function isQuoteAwareOptionsPriceSemantics(value: unknown): boolean {
  return normalizeText(value) === "top_of_book_quote";
}

export function priceSemanticsForRealismLane(value: unknown): string {
  switch (normalizeText(value)) {
    case "derived_mid":
      return "derived_mid_snapshot";
    case "quote_aware":
      return "top_of_book_quote";
    case "aggregate":
    default:
      return "minute_aggregate";
  }
}

export function realismLaneForPriceSemantics(value: unknown): string {
  switch (normalizeText(value)) {
    case "derived_mid_snapshot":
      return "derived_mid";
    case "top_of_book_quote":
      return "quote_aware";
    case "minute_aggregate":
    default:
      return "aggregate";
  }
}

export function isFillModelCompatibleWithPriceSemantics(
  fillModel: unknown,
  priceSemantics: unknown,
): boolean {
  const normalizedFill = normalizeText(fillModel);
  const normalizedSemantics = normalizeText(priceSemantics);
  if (!normalizedFill) return false;
  switch (normalizedSemantics) {
    case "top_of_book_quote":
      return (OPTIONS_FILL_MODEL_OPTIONS as readonly string[]).includes(
        normalizedFill,
      );
    case "minute_aggregate":
    case "derived_mid_snapshot":
    default:
      return normalizedFill === "aggregate_research";
  }
}

export function recommendedFillModelForPriceSemantics(
  priceSemantics: unknown,
): string {
  return isQuoteAwareOptionsPriceSemantics(priceSemantics)
    ? "quote_conservative"
    : "aggregate_research";
}

export function fillModelGuidanceForPriceSemantics(priceSemantics: unknown): {
  laneLabel: string;
  detail: string;
} {
  if (isQuoteAwareOptionsPriceSemantics(priceSemantics)) {
    return {
      laneLabel: "Quote-aware lane",
      detail:
        "Top-of-book quote datasets allow quote_conservative and quote_mid. quote_conservative is the realistic default; quote_mid is research-only; aggregate_research falls back to the row close.",
    };
  }
  const normalizedSemantics =
    normalizeText(priceSemantics) || "minute_aggregate";
  return {
    laneLabel:
      normalizedSemantics === "derived_mid_snapshot"
        ? "Derived-mid lane"
        : "Aggregate lane",
    detail: `${normalizedSemantics} datasets only support aggregate_research in the backend. Quote fill models are reserved for top_of_book_quote datasets.`,
  };
}

function coerceFillModelForPriceSemantics(
  fillModel: unknown,
  priceSemantics: unknown,
): string {
  const normalized = pickSupported(
    normalizeText(fillModel),
    OPTIONS_FILL_MODEL_OPTIONS,
    recommendedFillModelForPriceSemantics(priceSemantics),
  );
  return isFillModelCompatibleWithPriceSemantics(normalized, priceSemantics)
    ? normalized
    : recommendedFillModelForPriceSemantics(priceSemantics);
}

function selectPreferredDatasetIdForRealismLane(
  datasets: OptionsDatasetManifestSummary[],
  currentSelectedId: string,
  realismLane: string,
  parentRootHint: string,
): string {
  const targetSemantics = priceSemanticsForRealismLane(realismLane);
  const matching = datasets.filter(
    (entry) => normalizeText(entry.priceSemantics) === targetSemantics,
  );
  if (matching.length === 0) return "";
  if (matching.some((entry) => entry.optionsDatasetId === currentSelectedId)) {
    return currentSelectedId;
  }
  const rootHint = normalizeText(parentRootHint).toUpperCase();
  const preferred = matching.find(
    (entry) => normalizeText(entry.rootSymbol).toUpperCase() === rootHint,
  );
  return preferred?.optionsDatasetId || matching[0]?.optionsDatasetId || "";
}

function normalizeNumber(value: unknown, fallback = 0): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function normalizeOptionalNumber(value: unknown): number | null {
  const raw = normalizeText(value);
  if (!raw) return null;
  const parsed = Number(raw);
  return Number.isFinite(parsed) ? parsed : null;
}

function normalizeOptionalInteger(value: unknown): number | null {
  const parsed = normalizeOptionalNumber(value);
  return parsed == null ? null : Math.max(0, Math.floor(parsed));
}

function parseOptionalFiniteInput(value: unknown): {
  raw: string;
  value: number | null;
  invalid: boolean;
} {
  const raw = normalizeText(value);
  if (!raw) return { raw: "", value: null, invalid: false };
  const parsed = Number(raw);
  return {
    raw,
    value: Number.isFinite(parsed) ? parsed : null,
    invalid: !Number.isFinite(parsed),
  };
}

function buildValidatedOptionsFilters(inputs: {
  maxEntryLagMinutesInput: string;
  maxPremiumInput: string;
  minPremiumInput: string;
  minVolumeInput: string;
  minOpenInterestInput: string;
}): {
  filters: {
    max_entry_lag_minutes: number | null;
    max_premium: number | null;
    min_premium: number | null;
    min_volume: number | null;
    min_open_interest: number | null;
  };
  error: string;
} {
  const maxEntryLag = parseOptionalFiniteInput(inputs.maxEntryLagMinutesInput);
  const maxPremium = parseOptionalFiniteInput(inputs.maxPremiumInput);
  const minPremium = parseOptionalFiniteInput(inputs.minPremiumInput);
  const minVolume = parseOptionalFiniteInput(inputs.minVolumeInput);
  const minOpenInterest = parseOptionalFiniteInput(inputs.minOpenInterestInput);

  const emptyFilters = {
    max_entry_lag_minutes: null,
    max_premium: null,
    min_premium: null,
    min_volume: null,
    min_open_interest: null,
  };
  if (
    maxEntryLag.invalid ||
    (maxEntryLag.value != null &&
      (!Number.isInteger(maxEntryLag.value) || maxEntryLag.value < 0))
  ) {
    return {
      filters: emptyFilters,
      error: "Max entry lag minutes must be a non-negative whole number.",
    };
  }
  if (
    maxPremium.invalid ||
    (maxPremium.value != null && maxPremium.value < 0)
  ) {
    return {
      filters: emptyFilters,
      error: "Max premium must be a non-negative number.",
    };
  }
  if (
    minPremium.invalid ||
    (minPremium.value != null && minPremium.value < 0)
  ) {
    return {
      filters: emptyFilters,
      error: "Min premium must be a non-negative number.",
    };
  }
  if (minVolume.invalid || (minVolume.value != null && minVolume.value < 0)) {
    return {
      filters: emptyFilters,
      error: "Min volume must be a non-negative number.",
    };
  }
  if (
    minOpenInterest.invalid ||
    (minOpenInterest.value != null && minOpenInterest.value < 0)
  ) {
    return {
      filters: emptyFilters,
      error: "Min open interest must be a non-negative number.",
    };
  }
  if (
    minPremium.value != null &&
    maxPremium.value != null &&
    minPremium.value > maxPremium.value
  ) {
    return {
      filters: emptyFilters,
      error: "Min premium must be less than or equal to max premium.",
    };
  }

  return {
    filters: {
      max_entry_lag_minutes:
        maxEntryLag.value == null ? null : Math.floor(maxEntryLag.value),
      max_premium: maxPremium.value,
      min_premium: minPremium.value,
      min_volume: minVolume.value,
      min_open_interest: minOpenInterest.value,
    },
    error: "",
  };
}

function fileStem(value: string): string {
  const trimmed = normalizeText(value);
  if (!trimmed) return "";
  const parts = trimmed.split(/[\\/]+/g);
  const leaf = parts[parts.length - 1] || "";
  return leaf.replace(/\.[^.]+$/, "").trim();
}

function deriveRootHint(datasetLabel: string): string {
  const stem = fileStem(datasetLabel);
  if (!stem) return "";
  const normalized = stem.match(/[A-Za-z0-9]+/g)?.join("") || "";
  return normalized.toUpperCase();
}

function parseIntegerList(value: string, fallback: number[]): number[] {
  const tokens = normalizeText(value)
    .split(",")
    .map((entry) => normalizeText(entry))
    .filter((entry) => entry.length > 0);
  const out = tokens
    .map((entry) => Math.floor(normalizeNumber(entry, Number.NaN)))
    .filter((entry) => Number.isFinite(entry) && entry >= 0);
  return out.length > 0
    ? Array.from(new Set(out)).sort((a, b) => a - b)
    : fallback;
}

function parseFloatList(value: string, fallback: number[]): number[] {
  const tokens = normalizeText(value)
    .split(",")
    .map((entry) => normalizeText(entry))
    .filter((entry) => entry.length > 0);
  const out = tokens
    .map((entry) => normalizeNumber(entry, Number.NaN))
    .filter((entry) => Number.isFinite(entry) && entry >= 0);
  return out.length > 0
    ? Array.from(new Set(out)).sort((a, b) => a - b)
    : fallback;
}

function pickSupported<T extends string>(
  value: string,
  allowed: readonly T[],
  fallback: T,
): T {
  const normalized = normalizeText(value);
  return (allowed.includes(normalized as T) ? normalized : fallback) as T;
}

function optionsValidationValuesFromState(state: {
  latestRun: RunOptionsExpressionSummary | null;
  activeArtifact: OptionsExpressionArtifactRecord | null;
  selectedArtifactPath: string;
  selectedDatasetId: string;
  validationDatasetPolicy: string;
  validationPlanMode: string;
  validationStartDateInput: string;
  validationEndDateInput: string;
  validationTrainSessionCountInput: string;
  validationSessionCountInput: string;
  validationStepSessionCountInput: string;
  validationTupleIndexesInput: string;
  validationIncludeSealedFinalists: boolean;
  validationIncludeRobustLeader: boolean;
  validationIncludeHoldoutLeader: boolean;
  minValidTradesInput: string;
}): OptionsValidationFormValues {
  const activeArtifactPath =
    normalizeText(state.latestRun?.artifactPath) ||
    normalizeText(state.activeArtifact?.artifactPath) ||
    normalizeText(state.selectedArtifactPath);
  return createOptionsValidationFormValues({
    artifactPath: activeArtifactPath,
    activeArtifactOptionsDatasetId: state.activeArtifact?.optionsDatasetId || "",
    selectedDatasetId: state.selectedDatasetId,
    datasetPolicy: pickSupported(
      state.validationDatasetPolicy,
      OPTIONS_VALIDATION_DATASET_POLICY_OPTIONS,
      "require_exact_replay",
    ),
    validationPlanMode: pickSupported(
      state.validationPlanMode,
      OPTIONS_VALIDATION_PLAN_MODE_OPTIONS,
      "original_embedded_splits",
    ),
    startDate: state.validationStartDateInput,
    endDate: state.validationEndDateInput,
    trainSessionCountInput: state.validationTrainSessionCountInput,
    validationSessionCountInput: state.validationSessionCountInput,
    stepSessionCountInput: state.validationStepSessionCountInput,
    tupleIndexesInput: state.validationTupleIndexesInput,
    includeSealedFinalists: state.validationIncludeSealedFinalists,
    includeRobustLeader: state.validationIncludeRobustLeader,
    includeHoldoutLeader: state.validationIncludeHoldoutLeader,
    minValidTradesInput: state.minValidTradesInput,
  });
}

export const useOptionsExpressionStore = defineStore("optionsExpression", {
  state: () => ({
    parentArtifactPath: "",
    parentArtifactId: "",
    parentStrategyKind: "",
    parentDatasetLabel: "",
    parentRootHint: "",
    importRootSymbol: "",
    realismLane: "aggregate",
    importPriceSemantics: "minute_aggregate",
    downloadDatasetIdInput: "",
    downloadFromDateInput: "",
    downloadToDateInput: "",
    downloadContractType: "both",
    downloadExpirationFromDateInput: "",
    downloadExpirationToDateInput: "",
    downloadMaxDteDaysInput: "",
    downloadStrikeMinInput: "",
    downloadStrikeMaxInput: "",
    downloadMaxContractsInput: "30",
    datasets: [] as OptionsDatasetManifestSummary[],
    artifacts: [] as OptionsExpressionArtifactSummary[],
    latestDatasetDownload: null as PolygonOptionsDatasetDownloadSummary | null,
    selectedDatasetId: "",
    selectedArtifactPath: "",
    activeArtifact: null as OptionsExpressionArtifactRecord | null,
    latestRun: null as RunOptionsExpressionSummary | null,
    latestFeasibility:
      null as AnalyzeOptionsExpressionFeasibilitySummary | null,
    latestValidationFeasibility:
      null as AnalyzeOptionsExpressionValidationFeasibilitySummary | null,
    latestValidation: null as ValidateOptionsExpressionArtifactSummary | null,
    activeTupleInspection:
      null as OptionsExpressionTupleInspectionSummary | null,
    activeEventPathInspection:
      null as OptionsExpressionEventPathInspectionSummary | null,
    delayMinutesInput: "0,1,5",
    dteBucketsInput: "1,3,5",
    holdMinutesInput: "5,15,30,60",
    targetReturnInput: "0.25,0.5,1",
    stopReturnInput: "0.25,0.5",
    selectedMoneynessBuckets: [...OPTIONS_MONEYNESS_BUCKET_OPTIONS] as string[],
    exitTemplate: "fixed_hold_or_target_stop",
    fillModel: "aggregate_research",
    structureFamily: "long_single",
    backendPreference: "auto",
    objectiveProfile: "balanced",
    splitAssignmentMode: "event_chronological",
    validationDatasetPolicy: "require_exact_replay",
    validationPlanMode: "original_embedded_splits",
    validationStartDateInput: "",
    validationEndDateInput: "",
    validationTrainSessionCountInput: "5",
    validationSessionCountInput: "1",
    validationStepSessionCountInput: "1",
    validationIncludeSealedFinalists: true,
    validationIncludeRobustLeader: true,
    validationIncludeHoldoutLeader: true,
    validationTupleIndexesInput: "",
    topKInput: "25",
    sealedFinalistCountInput: "5",
    minValidTradesInput: "",
    splitRatioSearchInput: "0.6",
    splitRatioHoldoutInput: "0.2",
    maxEntryLagMinutesInput: "5",
    maxPremiumInput: "",
    minPremiumInput: "",
    minVolumeInput: "",
    minOpenInterestInput: "",
    datasetsBusy: false,
    artifactsBusy: false,
    runBusy: false,
    loadBusy: false,
    importBusy: false,
    downloadBusy: false,
    inspectionBusy: false,
    eventPathBusy: false,
    feasibilityBusy: false,
    validationFeasibilityBusy: false,
    validationBusy: false,
    statusLine:
      "Select a saved supported run to configure an options expression follow-up.",
  }),

  getters: {
    supportedParent: (state) =>
      isSupportedParentStrategyKind(state.parentStrategyKind),

    realismLanePriceSemantics(state): string {
      return priceSemanticsForRealismLane(state.realismLane);
    },

    realismLaneDatasets(state): OptionsDatasetManifestSummary[] {
      const targetSemantics = priceSemanticsForRealismLane(state.realismLane);
      return state.datasets.filter(
        (entry) => normalizeText(entry.priceSemantics) === targetSemantics,
      );
    },

    filteredArtifacts(state): OptionsExpressionArtifactSummary[] {
      const parentId = normalizeText(state.parentArtifactId);
      if (!parentId) return [];
      return state.artifacts.filter(
        (entry) => normalizeText(entry.parentArtifactId) === parentId,
      );
    },

    selectedDataset(state): OptionsDatasetManifestSummary | null {
      return (
        state.datasets.find(
          (entry) => entry.optionsDatasetId === state.selectedDatasetId,
        ) || null
      );
    },

    activeArtifactPath(state): string {
      return (
        normalizeText(state.latestRun?.artifactPath) ||
        normalizeText(state.activeArtifact?.artifactPath) ||
        normalizeText(state.selectedArtifactPath)
      );
    },

    activeValidationArtifactPath(state): string {
      return normalizeText(state.latestValidation?.artifactPath);
    },

    validationAcceptance(state) {
      return state.latestValidation?.result.acceptanceSummary || null;
    },

    validationFormErrorMessages(state): string[] {
      return optionsValidationErrorMessages(optionsValidationValuesFromState(state));
    },

    validationFormValid(state): boolean {
      return !validateOptionsValidationFormValues(
        optionsValidationValuesFromState(state),
      );
    },

    activeArtifactTimestamp(state): string {
      return (
        normalizeText(state.latestRun?.artifactTs) ||
        normalizeText(state.activeArtifact?.ts)
      );
    },

    activeDiagnostics(state) {
      return (
        state.latestRun?.diagnostics ||
        state.activeArtifact?.diagnostics ||
        null
      );
    },

    activeRequest(state) {
      return state.latestRun?.request || state.activeArtifact?.request || null;
    },

    activeOptionsDatasetHash(state): string {
      return (
        normalizeText(state.latestRun?.optionsDatasetHash) ||
        normalizeText(state.activeArtifact?.optionsDatasetHash)
      );
    },

    activeStaging(state) {
      return state.latestRun?.staging || state.activeArtifact?.staging || null;
    },

    activeRankedResults(state) {
      if (state.latestRun) return state.latestRun.rankedResults;
      return state.activeArtifact?.selectionMetrics || [];
    },

    activeBackendUsed(state): string {
      return (
        normalizeText(state.latestRun?.backendUsed) ||
        normalizeText(state.activeArtifact?.backendUsed)
      );
    },

    activeBackendFallbackReason(state): string {
      return (
        normalizeText(state.latestRun?.backendFallbackReason) ||
        normalizeText(state.activeArtifact?.backendFallbackReason)
      );
    },

    activePriceSemantics(state): string {
      return (
        normalizeText(state.latestRun?.priceSemantics) ||
        normalizeText(state.activeArtifact?.priceSemantics)
      );
    },

    activeDatasetProvenance(
      state,
    ): OptionsExpressionDatasetProvenanceSummary | null {
      return (
        state.latestRun?.datasetProvenance ||
        state.activeArtifact?.datasetProvenance ||
        null
      );
    },
  },

  actions: {
    ensureCompatibleFillModel(notify = false): void {
      const semantics = normalizeText(this.selectedDataset?.priceSemantics);
      if (!semantics) return;
      const nextFillModel = coerceFillModelForPriceSemantics(
        this.fillModel,
        semantics,
      );
      if (nextFillModel === this.fillModel) return;
      this.fillModel = nextFillModel;
      if (notify) {
        const guidance = fillModelGuidanceForPriceSemantics(semantics);
        this.statusLine = `${guidance.laneLabel}: fill model reset to ${nextFillModel} for ${semantics}.`;
      }
    },

    setRealismLane(nextLane: string, notify = false): void {
      const normalizedLane = pickSupported(
        nextLane,
        OPTIONS_REALISM_LANE_OPTIONS,
        "aggregate",
      );
      this.realismLane = normalizedLane;
      this.importPriceSemantics = priceSemanticsForRealismLane(normalizedLane);
      this.selectedDatasetId = selectPreferredDatasetIdForRealismLane(
        this.datasets,
        this.selectedDatasetId,
        this.realismLane,
        this.parentRootHint,
      );
      const nextFillModel = coerceFillModelForPriceSemantics(
        this.fillModel,
        this.importPriceSemantics,
      );
      if (nextFillModel !== this.fillModel) {
        this.fillModel = nextFillModel;
      }
      if (notify) {
        const guidance = fillModelGuidanceForPriceSemantics(
          this.importPriceSemantics,
        );
        this.statusLine = `${guidance.laneLabel}: workflow lane set to ${normalizedLane}.`;
      }
    },

    attachParent(
      parentArtifactPath: string,
      parentStrategyKind: string,
      parentDatasetLabel: string,
    ): void {
      const nextPath = normalizeText(parentArtifactPath);
      const nextStrategy = normalizeText(parentStrategyKind);
      const nextDatasetLabel = normalizeText(parentDatasetLabel);
      const nextParentId = fileStem(nextPath);
      const nextRootHint = deriveRootHint(nextDatasetLabel);
      const changed =
        nextPath !== this.parentArtifactPath ||
        nextStrategy !== this.parentStrategyKind ||
        nextDatasetLabel !== this.parentDatasetLabel;

      this.parentArtifactPath = nextPath;
      this.parentArtifactId = nextParentId;
      this.parentStrategyKind = nextStrategy;
      this.parentDatasetLabel = nextDatasetLabel;
      this.parentRootHint = nextRootHint;
      if (changed) {
        this.importRootSymbol = nextRootHint;
        this.realismLane = "aggregate";
        this.importPriceSemantics = "minute_aggregate";
        this.downloadDatasetIdInput = "";
        this.downloadFromDateInput = "";
        this.downloadToDateInput = "";
        this.downloadContractType = "both";
        this.downloadExpirationFromDateInput = "";
        this.downloadExpirationToDateInput = "";
        this.downloadMaxDteDaysInput = "";
        this.downloadStrikeMinInput = "";
        this.downloadStrikeMaxInput = "";
        this.downloadMaxContractsInput = "30";
        this.selectedArtifactPath = "";
        this.activeArtifact = null;
        this.latestRun = null;
        this.latestFeasibility = null;
        this.latestValidationFeasibility = null;
        this.latestValidation = null;
        this.latestDatasetDownload = null;
        this.activeTupleInspection = null;
        this.activeEventPathInspection = null;
      }
      if (!nextPath) {
        this.statusLine =
          "Select a saved run to configure an options expression follow-up.";
        return;
      }
      if (!isSupportedParentStrategyKind(nextStrategy)) {
        this.statusLine =
          "Options expression follow-up currently supports saved DoubleMa, CompositeTrend, RegimeHybrid, and BreakoutMomentum runs only.";
        return;
      }
      this.statusLine =
        "Ready to import a Polygon.io options dataset or reuse an existing one.";
    },

    async bootstrap(): Promise<void> {
      if (!this.supportedParent || !this.parentArtifactPath) return;
      await Promise.all([this.refreshDatasets(), this.refreshArtifacts()]);
    },

    async refreshDatasets(): Promise<void> {
      this.datasetsBusy = true;
      try {
        const datasets = await listOptionsDatasets();
        this.datasets = [...datasets].sort((left, right) => {
          const byRoot = normalizeText(left.rootSymbol).localeCompare(
            normalizeText(right.rootSymbol),
          );
          if (byRoot !== 0) return byRoot;
          return normalizeText(left.optionsDatasetId).localeCompare(
            normalizeText(right.optionsDatasetId),
          );
        });
        const selectedExists = this.datasets.some(
          (entry) => entry.optionsDatasetId === this.selectedDatasetId,
        );
        const artifactDatasetId = normalizeText(
          this.activeArtifact?.optionsDatasetId,
        );
        const artifactDatasetExists =
          artifactDatasetId.length > 0 &&
          this.datasets.some(
            (entry) => entry.optionsDatasetId === artifactDatasetId,
          );
        const keepReplacementDataset =
          artifactDatasetId.length > 0 &&
          this.selectedDatasetId.length > 0 &&
          this.selectedDatasetId !== artifactDatasetId &&
          selectedExists;
        const nextSelectedDatasetId =
          artifactDatasetId.length > 0
            ? keepReplacementDataset
              ? this.selectedDatasetId
              : artifactDatasetExists
                ? artifactDatasetId
                : ""
            : selectedExists
              ? selectPreferredDatasetIdForRealismLane(
                  this.datasets,
                  this.selectedDatasetId,
                  this.realismLane,
                  this.parentRootHint,
                )
              : selectPreferredDatasetIdForRealismLane(
                  this.datasets,
                  "",
                  this.realismLane,
                  this.parentRootHint,
                );
        this.selectedDatasetId = nextSelectedDatasetId;
        this.ensureCompatibleFillModel();
        const laneMatchCount = this.realismLaneDatasets.length;
        this.statusLine =
          artifactDatasetId.length > 0 && !artifactDatasetExists
            ? `Saved artifact dataset ${artifactDatasetId} is not currently imported. Select a replacement dataset intentionally before rerunning or previewing feasibility.`
            : this.datasets.length
              ? laneMatchCount > 0
                ? `Loaded ${this.datasets.length} options dataset${this.datasets.length === 1 ? "" : "s"}; ${laneMatchCount} match the current ${this.realismLane.replace("_", "-")} lane.`
                : `Loaded ${this.datasets.length} options dataset${this.datasets.length === 1 ? "" : "s"}, but none match the current ${this.realismLane.replace("_", "-")} lane yet.`
              : "No imported options datasets yet. Import a normalized CSV to continue.";
      } catch (error) {
        this.statusLine =
          String(error || "").trim() || "Failed to refresh options datasets.";
      } finally {
        this.datasetsBusy = false;
      }
    },

    async importDatasetFromPicker(): Promise<void> {
      this.importBusy = true;
      try {
        const csvPath = await pickOptionsCsvPath();
        if (!csvPath) {
          this.statusLine = "Options dataset import canceled.";
          return;
        }
        const importPriceSemantics = pickSupported(
          this.importPriceSemantics,
          OPTIONS_IMPORT_PRICE_SEMANTICS_OPTIONS,
          "minute_aggregate",
        );
        const imported = await importOptionsDataset({
          csv_path: csvPath,
          root_symbol: normalizeText(this.importRootSymbol) || undefined,
          provider_name: "polygon.io",
          provider_source_surface: importPriceSemantics,
          contract_reference_source: "polygon_contract_reference",
          price_semantics: importPriceSemantics,
          aggregate_price_field:
            importPriceSemantics === "minute_aggregate" ? "close" : undefined,
        });
        await this.refreshDatasets();
        this.selectedDatasetId = imported.optionsDatasetId;
        this.ensureCompatibleFillModel();
        this.latestDatasetDownload = null;
        this.statusLine = `Imported options dataset ${imported.optionsDatasetId}.`;
      } catch (error) {
        this.statusLine =
          String(error || "").trim() || "Failed to import options dataset.";
      } finally {
        this.importBusy = false;
      }
    },

    async downloadDatasetFromPolygon(): Promise<void> {
      if (this.realismLane !== "aggregate") {
        this.statusLine =
          "Direct Polygon.io download in this panel is aggregate-lane only. Switch the realism lane to Aggregate to use provider download.";
        return;
      }
      const rootSymbol = normalizeText(
        this.importRootSymbol || this.parentRootHint,
      ).toUpperCase();
      const from = normalizeText(this.downloadFromDateInput);
      const to = normalizeText(this.downloadToDateInput);
      if (!rootSymbol) {
        this.statusLine =
          "Enter a Polygon.io root symbol before downloading options data.";
        return;
      }
      if (!from || !to) {
        this.statusLine =
          "Enter both from/to dates for the Polygon.io options download.";
        return;
      }

      const request = {
        rootSymbol,
        from,
        to,
      } as {
        rootSymbol: string;
        from: string;
        to: string;
        optionsDatasetId?: string;
        contractType?: "both" | "call" | "put";
        expirationDateFrom?: string;
        expirationDateTo?: string;
        maxDteDays?: number;
        strikeMin?: number;
        strikeMax?: number;
        maxContracts?: number;
      };
      const optionsDatasetId = normalizeText(this.downloadDatasetIdInput);
      if (optionsDatasetId) request.optionsDatasetId = optionsDatasetId;
      const contractType = pickSupported(
        this.downloadContractType,
        ["both", "call", "put"] as const,
        "both",
      );
      if (contractType) request.contractType = contractType;
      const expirationDateFrom = normalizeText(
        this.downloadExpirationFromDateInput,
      );
      if (expirationDateFrom) request.expirationDateFrom = expirationDateFrom;
      const expirationDateTo = normalizeText(
        this.downloadExpirationToDateInput,
      );
      if (expirationDateTo) request.expirationDateTo = expirationDateTo;
      const maxDteDays = normalizeOptionalInteger(this.downloadMaxDteDaysInput);
      if (maxDteDays != null) request.maxDteDays = maxDteDays;
      const strikeMin = normalizeOptionalNumber(this.downloadStrikeMinInput);
      if (strikeMin != null) request.strikeMin = strikeMin;
      const strikeMax = normalizeOptionalNumber(this.downloadStrikeMaxInput);
      if (strikeMax != null) request.strikeMax = strikeMax;
      const maxContracts = normalizeOptionalInteger(
        this.downloadMaxContractsInput,
      );
      if (maxContracts != null) request.maxContracts = maxContracts;

      this.downloadBusy = true;
      try {
        const downloaded: PolygonOptionsDatasetDownloadSummary =
          await downloadPolygonOptionsDataset(request);
        await this.refreshDatasets();
        this.selectedDatasetId = downloaded.manifest.optionsDatasetId;
        this.ensureCompatibleFillModel();
        this.latestDatasetDownload = downloaded;
        this.statusLine =
          `Downloaded Polygon.io options dataset ${downloaded.manifest.optionsDatasetId} ` +
          `(${downloaded.contractsWithRows}/${downloaded.contractsSelected} contracts with rows, ` +
          `${downloaded.downloadedOptionRows} option rows).`;
      } catch (error) {
        this.statusLine =
          String(error || "").trim() ||
          "Failed to download Polygon.io options dataset.";
      } finally {
        this.downloadBusy = false;
      }
    },

    async revealSelectedDatasetPath(): Promise<void> {
      const datasetPath = normalizeText(this.selectedDataset?.datasetPath);
      if (!datasetPath) {
        this.statusLine =
          "No dataset path available for the selected options dataset.";
        return;
      }
      try {
        await revealOptionsPath(datasetPath);
        this.statusLine = `Revealed dataset path ${datasetPath}.`;
      } catch (error) {
        this.statusLine =
          String(error || "").trim() ||
          "Failed to reveal the selected options dataset path.";
      }
    },

    async revealLatestDownloadUnderlyingPath(): Promise<void> {
      const csvPath = normalizeText(
        this.latestDatasetDownload?.underlyingCsvPath,
      );
      if (!csvPath) {
        this.statusLine =
          "No underlying CSV path is available for the latest options download.";
        return;
      }
      try {
        await revealOptionsPath(csvPath);
        this.statusLine = `Revealed underlying CSV path ${csvPath}.`;
      } catch (error) {
        this.statusLine =
          String(error || "").trim() ||
          "Failed to reveal the underlying CSV path.";
      }
    },

    async refreshArtifacts(): Promise<void> {
      this.artifactsBusy = true;
      try {
        const artifacts = await listOptionsExpressionArtifacts();
        this.artifacts = [...artifacts].sort((left, right) =>
          normalizeText(right.ts).localeCompare(normalizeText(left.ts)),
        );
        const filtered = this.filteredArtifacts;
        const selectedExists = filtered.some(
          (entry) => entry.path === this.selectedArtifactPath,
        );
        if (!selectedExists)
          this.selectedArtifactPath = filtered[0]?.path || "";
        this.statusLine = filtered.length
          ? `Loaded ${filtered.length} saved options expression artifact${filtered.length === 1 ? "" : "s"} for this run.`
          : this.statusLine;
      } catch (error) {
        this.statusLine =
          String(error || "").trim() ||
          "Failed to refresh saved options expression artifacts.";
      } finally {
        this.artifactsBusy = false;
      }
    },

    async runCurrentExpression(): Promise<void> {
      if (!this.parentArtifactPath) {
        this.statusLine =
          "Select a saved run before starting options expression optimization.";
        return;
      }
      if (!this.supportedParent) {
        this.statusLine =
          "Only saved DoubleMa, CompositeTrend, RegimeHybrid, and BreakoutMomentum runs are currently supported.";
        return;
      }
      if (!this.selectedDatasetId) {
        this.statusLine = "Import or select an options dataset first.";
        return;
      }

      const splitRatioSearch = normalizeNumber(this.splitRatioSearchInput, 0.6);
      const splitRatioHoldout = normalizeNumber(
        this.splitRatioHoldoutInput,
        0.2,
      );
      const topK = Math.max(1, Math.floor(normalizeNumber(this.topKInput, 25)));
      const sealedFinalistCount = Math.min(
        topK,
        Math.max(
          1,
          Math.floor(normalizeNumber(this.sealedFinalistCountInput, 5)),
        ),
      );
      if (
        !(
          splitRatioSearch > 0 &&
          splitRatioHoldout >= 0 &&
          splitRatioSearch + splitRatioHoldout < 1
        )
      ) {
        this.statusLine =
          "Split ratios must satisfy search > 0, holdout >= 0, and search + holdout < 1.";
        return;
      }

      const datasetPriceSemantics =
        normalizeText(this.selectedDataset?.priceSemantics) ||
        "minute_aggregate";
      const fillModel = coerceFillModelForPriceSemantics(
        this.fillModel,
        datasetPriceSemantics,
      );
      this.fillModel = fillModel;
      const validatedFilters = buildValidatedOptionsFilters({
        maxEntryLagMinutesInput: this.maxEntryLagMinutesInput,
        maxPremiumInput: this.maxPremiumInput,
        minPremiumInput: this.minPremiumInput,
        minVolumeInput: this.minVolumeInput,
        minOpenInterestInput: this.minOpenInterestInput,
      });
      if (validatedFilters.error) {
        this.statusLine = validatedFilters.error;
        return;
      }
      const request = {
        parent_artifact_path: this.parentArtifactPath,
        options_dataset_id: this.selectedDatasetId,
        delay_minutes: parseIntegerList(this.delayMinutesInput, [0, 1, 5]),
        dte_buckets: parseIntegerList(this.dteBucketsInput, [1, 3, 5]),
        moneyness_buckets:
          this.selectedMoneynessBuckets.length > 0
            ? [...this.selectedMoneynessBuckets]
            : [...OPTIONS_MONEYNESS_BUCKET_OPTIONS],
        hold_minutes: parseIntegerList(this.holdMinutesInput, [5, 15, 30, 60]),
        target_return_buckets: parseFloatList(
          this.targetReturnInput,
          [0.25, 0.5, 1],
        ),
        stop_return_buckets: parseFloatList(this.stopReturnInput, [0.25, 0.5]),
        exit_template: pickSupported(
          this.exitTemplate,
          OPTIONS_EXIT_TEMPLATE_OPTIONS,
          "fixed_hold_or_target_stop",
        ),
        fill_model: fillModel,
        structure_family: pickSupported(
          this.structureFamily,
          OPTIONS_STRUCTURE_FAMILY_OPTIONS,
          "long_single",
        ),
        backend_preference: pickSupported(
          this.backendPreference,
          OPTIONS_BACKEND_PREFERENCE_OPTIONS,
          "auto",
        ),
        objective_config: {
          profile: pickSupported(
            this.objectiveProfile,
            OPTIONS_OBJECTIVE_PROFILE_OPTIONS,
            "balanced",
          ),
        },
        split_assignment_mode: pickSupported(
          this.splitAssignmentMode,
          OPTIONS_SPLIT_ASSIGNMENT_MODE_OPTIONS,
          "event_chronological",
        ),
        filters: validatedFilters.filters,
        top_k: topK,
        min_valid_trades: normalizeOptionalInteger(this.minValidTradesInput),
        split_ratio_search: splitRatioSearch,
        split_ratio_holdout: splitRatioHoldout,
        sealed_finalist_count: sealedFinalistCount,
      };

      this.runBusy = true;
      try {
        const result = await runOptionsExpression(request);
        this.latestRun = result;
        this.activeArtifact = null;
        this.activeTupleInspection = null;
        this.activeEventPathInspection = null;
        this.selectedArtifactPath = result.artifactPath;
        await this.refreshArtifacts();
        this.statusLine = `Options expression run completed with ${result.tupleCount} ranked tuple${result.tupleCount === 1 ? "" : "s"}.`;
      } catch (error) {
        this.statusLine =
          String(error || "").trim() || "Options expression run failed.";
      } finally {
        this.runBusy = false;
      }
    },

    async previewCurrentFeasibility(): Promise<void> {
      if (!this.parentArtifactPath) {
        this.statusLine =
          "Select a saved run before previewing options-expression feasibility.";
        return;
      }
      if (!this.supportedParent) {
        this.statusLine =
          "Only saved DoubleMa, CompositeTrend, RegimeHybrid, and BreakoutMomentum runs are currently supported.";
        return;
      }
      if (!this.selectedDatasetId) {
        this.statusLine = "Import or select an options dataset first.";
        return;
      }

      const splitRatioSearch = normalizeNumber(this.splitRatioSearchInput, 0.6);
      const splitRatioHoldout = normalizeNumber(
        this.splitRatioHoldoutInput,
        0.2,
      );
      const topK = Math.max(1, Math.floor(normalizeNumber(this.topKInput, 25)));
      const sealedFinalistCount = Math.min(
        topK,
        Math.max(
          1,
          Math.floor(normalizeNumber(this.sealedFinalistCountInput, 5)),
        ),
      );
      if (
        !(
          splitRatioSearch > 0 &&
          splitRatioHoldout >= 0 &&
          splitRatioSearch + splitRatioHoldout < 1
        )
      ) {
        this.statusLine =
          "Split ratios must satisfy search > 0, holdout >= 0, and search + holdout < 1.";
        return;
      }

      this.feasibilityBusy = true;
      try {
        const datasetPriceSemantics =
          normalizeText(this.selectedDataset?.priceSemantics) ||
          "minute_aggregate";
        const fillModel = coerceFillModelForPriceSemantics(
          this.fillModel,
          datasetPriceSemantics,
        );
        this.fillModel = fillModel;
        const validatedFilters = buildValidatedOptionsFilters({
          maxEntryLagMinutesInput: this.maxEntryLagMinutesInput,
          maxPremiumInput: this.maxPremiumInput,
          minPremiumInput: this.minPremiumInput,
          minVolumeInput: this.minVolumeInput,
          minOpenInterestInput: this.minOpenInterestInput,
        });
        if (validatedFilters.error) {
          this.statusLine = validatedFilters.error;
          return;
        }
        const preview = await analyzeOptionsExpressionFeasibility({
          parent_artifact_path: this.parentArtifactPath,
          options_dataset_id: this.selectedDatasetId,
          delay_minutes: parseIntegerList(this.delayMinutesInput, [0, 1, 5]),
          dte_buckets: parseIntegerList(this.dteBucketsInput, [1, 3, 5]),
          moneyness_buckets:
            this.selectedMoneynessBuckets.length > 0
              ? [...this.selectedMoneynessBuckets]
              : [...OPTIONS_MONEYNESS_BUCKET_OPTIONS],
          hold_minutes: parseIntegerList(
            this.holdMinutesInput,
            [5, 15, 30, 60],
          ),
          target_return_buckets: parseFloatList(
            this.targetReturnInput,
            [0.25, 0.5, 1],
          ),
          stop_return_buckets: parseFloatList(
            this.stopReturnInput,
            [0.25, 0.5],
          ),
          exit_template: pickSupported(
            this.exitTemplate,
            OPTIONS_EXIT_TEMPLATE_OPTIONS,
            "fixed_hold_or_target_stop",
          ),
          fill_model: fillModel,
          structure_family: pickSupported(
            this.structureFamily,
            OPTIONS_STRUCTURE_FAMILY_OPTIONS,
            "long_single",
          ),
          backend_preference: pickSupported(
            this.backendPreference,
            OPTIONS_BACKEND_PREFERENCE_OPTIONS,
            "auto",
          ),
          objective_config: {
            profile: pickSupported(
              this.objectiveProfile,
              OPTIONS_OBJECTIVE_PROFILE_OPTIONS,
              "balanced",
            ),
          },
          split_assignment_mode: pickSupported(
            this.splitAssignmentMode,
            OPTIONS_SPLIT_ASSIGNMENT_MODE_OPTIONS,
            "event_chronological",
          ),
          filters: validatedFilters.filters,
          top_k: topK,
          min_valid_trades: normalizeOptionalInteger(this.minValidTradesInput),
          split_ratio_search: splitRatioSearch,
          split_ratio_holdout: splitRatioHoldout,
          sealed_finalist_count: sealedFinalistCount,
        });
        this.latestFeasibility = preview;
        this.statusLine = `Feasibility preview compiled ${preview.compiledTupleCount} tuple${preview.compiledTupleCount === 1 ? "" : "s"} with ${(preview.candidateDensity * 100).toFixed(1)}% candidate density.`;
      } catch (error) {
        this.statusLine =
          String(error || "").trim() ||
          "Failed to preview options-expression feasibility.";
      } finally {
        this.feasibilityBusy = false;
      }
    },

    async loadArtifact(path?: string): Promise<void> {
      const nextPath = normalizeText(path || this.selectedArtifactPath);
      if (!nextPath) {
        this.statusLine = "Select a saved options expression artifact first.";
        return;
      }
      this.loadBusy = true;
      try {
        const artifact = await loadOptionsExpressionArtifact(nextPath);
        this.selectedArtifactPath = nextPath;
        this.activeArtifact = artifact;
        this.realismLane = realismLaneForPriceSemantics(
          artifact.priceSemantics,
        );
        this.importPriceSemantics = priceSemanticsForRealismLane(
          this.realismLane,
        );
        this.selectedDatasetId = this.datasets.some(
          (entry) => entry.optionsDatasetId === artifact.optionsDatasetId,
        )
          ? artifact.optionsDatasetId
          : "";
        this.fillModel = coerceFillModelForPriceSemantics(
          normalizeText(artifact.request?.fillModel) || this.fillModel,
          this.importPriceSemantics,
        );
        this.objectiveProfile = pickSupported(
          artifact.request?.objectiveConfig?.profile ?? "",
          OPTIONS_OBJECTIVE_PROFILE_OPTIONS,
          "balanced",
        );
        this.latestRun = null;
        this.latestFeasibility = null;
        this.activeTupleInspection = null;
        this.activeEventPathInspection = null;
        this.latestValidation = null;
        this.latestValidationFeasibility = null;
        this.statusLine = this.selectedDatasetId
          ? `Loaded saved options expression artifact ${fileStem(nextPath) || nextPath}; pinned replay to dataset ${artifact.optionsDatasetId}.`
          : `Loaded saved options expression artifact ${fileStem(nextPath) || nextPath}, but dataset ${artifact.optionsDatasetId} is not currently imported. Select a replacement dataset intentionally before rerunning or previewing feasibility.`;
      } catch (error) {
        this.statusLine =
          String(error || "").trim() ||
          "Failed to load saved options expression artifact.";
      } finally {
        this.loadBusy = false;
      }
    },

    buildValidationRequest(): Record<string, unknown> {
      const values = optionsValidationValuesFromState(this);
      const errors = validateOptionsValidationFormValues(values);
      const firstError = Object.values(errors?.fields ?? {})[0];
      if (firstError) throw new Error(firstError);
      return toOptionsValidationCommandRequest(values) as unknown as Record<
        string,
        unknown
      >;
    },

    async previewValidationFeasibility(): Promise<void> {
      this.validationFeasibilityBusy = true;
      try {
        const preview = await analyzeOptionsExpressionValidationFeasibility(
          this.buildValidationRequest(),
        );
        this.latestValidationFeasibility = preview;
        this.statusLine = `Validation preview selected ${preview.selectedTupleCount} tuple${preview.selectedTupleCount === 1 ? "" : "s"} for ${preview.datasetPolicy}.`;
      } catch (error) {
        this.statusLine =
          String(error || "").trim() ||
          "Failed to preview options-expression validation.";
      } finally {
        this.validationFeasibilityBusy = false;
      }
    },

    async validateActiveArtifact(): Promise<void> {
      this.validationBusy = true;
      try {
        const validation = await validateOptionsExpressionArtifact(
          this.buildValidationRequest(),
        );
        this.latestValidation = validation;
        this.latestValidationFeasibility = null;
        const summary = validation.result.acceptanceSummary;
        this.statusLine = `Validation ${summary.verdict.replace(/_/g, " ")}: ${summary.acceptedTupleCount}/${summary.validatedTupleCount} tuple${summary.validatedTupleCount === 1 ? "" : "s"} accepted.`;
      } catch (error) {
        this.statusLine =
          String(error || "").trim() ||
          "Failed to validate options expression artifact.";
      } finally {
        this.validationBusy = false;
      }
    },

    async revealActiveArtifact(): Promise<void> {
      const nextPath = this.activeArtifactPath;
      if (!nextPath) {
        this.statusLine = "No options expression artifact is active yet.";
        return;
      }
      try {
        await revealOptionsPath(nextPath);
        this.statusLine = `Revealed ${nextPath}.`;
      } catch (error) {
        this.statusLine =
          String(error || "").trim() ||
          "Failed to reveal options expression artifact path.";
      }
    },

    async revealLatestValidationArtifact(): Promise<void> {
      const nextPath = this.activeValidationArtifactPath;
      if (!nextPath) {
        this.statusLine = "No options validation artifact is active yet.";
        return;
      }
      try {
        await revealOptionsPath(nextPath);
        this.statusLine = `Revealed ${nextPath}.`;
      } catch (error) {
        this.statusLine =
          String(error || "").trim() ||
          "Failed to reveal options validation artifact path.";
      }
    },

    async exportActiveCsv(): Promise<void> {
      const tuples = this.activeRankedResults;
      if (!tuples.length) {
        this.statusLine = "No ranked tuples are available to export yet.";
        return;
      }

      try {
        const exported = await exportOptionsExpressionCsv({
          filenameStem: normalizeText(this.parentArtifactId)
            ? `${this.parentArtifactId}_options_expression`
            : `vectorgrid_options_expression_${normalizeText(this.activeDiagnostics?.rootSymbol || this.parentRootHint).toLowerCase() || "results"}`,
          rootSymbol:
            normalizeText(this.activeDiagnostics?.rootSymbol) ||
            normalizeText(this.parentRootHint),
          backendUsed: this.activeBackendUsed,
          priceSemantics: this.activePriceSemantics,
          tuples,
        });

        if (exported.mode === "saved" && exported.path) {
          this.statusLine = `${exported.revealed ? "Saved and revealed" : "Saved"} ${tuples.length} ranked tuple${tuples.length === 1 ? "" : "s"} to ${exported.path}.`;
          return;
        }
        if (exported.mode === "downloaded") {
          this.statusLine = `Exported ${tuples.length} ranked tuple${tuples.length === 1 ? "" : "s"} as a file download.`;
          return;
        }
        if (exported.mode === "cancelled") {
          this.statusLine = "CSV export cancelled.";
          return;
        }
        this.statusLine =
          "CSV export is unavailable in the current environment.";
      } catch (error) {
        this.statusLine =
          String(error || "").trim() ||
          "Failed to export ranked tuples to CSV.";
      }
    },

    async exportActiveInspectionCsv(): Promise<void> {
      const inspection = this.activeTupleInspection;
      const rows = inspection?.rows ?? [];
      if (!inspection || !rows.length) {
        this.statusLine =
          "Load tuple detail before exporting inspection rows.";
        return;
      }

      const artifactStem = normalizeText(this.activeArtifactPath)
        ? normalizeText(this.activeArtifactPath)
            .split(/[\\/]/)
            .pop()
            ?.replace(/\.[^.]+$/, "") || "vectorgrid_options_expression"
        : "vectorgrid_options_expression";

      try {
        const exported = await exportOptionsExpressionInspectionCsv({
          filenameStem: `${artifactStem}_tuple_${inspection.tupleIndex + 1}_inspection`,
          artifactPath: this.activeArtifactPath,
          tupleIndex: inspection.tupleIndex,
          rows,
        });

        if (exported.mode === "saved" && exported.path) {
          this.statusLine = `${exported.revealed ? "Saved and revealed" : "Saved"} ${rows.length} inspection row${rows.length === 1 ? "" : "s"} to ${exported.path}.`;
          return;
        }
        if (exported.mode === "downloaded") {
          this.statusLine = `Exported ${rows.length} inspection row${rows.length === 1 ? "" : "s"} as a file download.`;
          return;
        }
        if (exported.mode === "cancelled") {
          this.statusLine = "CSV export cancelled.";
          return;
        }
        this.statusLine =
          "CSV export is unavailable in the current environment.";
      } catch (error) {
        this.statusLine =
          String(error || "").trim() ||
          "Failed to export tuple inspection rows.";
      }
    },

    async exportActiveEventPathCsv(): Promise<void> {
      const inspection = this.activeEventPathInspection;
      const points = inspection?.points ?? [];
      if (!inspection || !points.length) {
        this.statusLine =
          "Load an executed event path before exporting replay points.";
        return;
      }

      const artifactStem = normalizeText(this.activeArtifactPath)
        ? normalizeText(this.activeArtifactPath)
            .split(/[\\/]/)
            .pop()
            ?.replace(/\.[^.]+$/, "") || "vectorgrid_options_expression"
        : "vectorgrid_options_expression";

      try {
        const exported = await exportOptionsExpressionEventPathCsv({
          filenameStem: `${artifactStem}_tuple_${inspection.tupleIndex + 1}_${normalizeText(inspection.eventId) || `event_${inspection.eventIndex + 1}`}_path`,
          artifactPath: this.activeArtifactPath,
          tupleIndex: inspection.tupleIndex,
          eventId: inspection.eventId,
          points,
        });

        if (exported.mode === "saved" && exported.path) {
          this.statusLine = `${exported.revealed ? "Saved and revealed" : "Saved"} ${points.length} replay point${points.length === 1 ? "" : "s"} for event ${inspection.eventId} to ${exported.path}.`;
          return;
        }
        if (exported.mode === "downloaded") {
          this.statusLine = `Exported ${points.length} replay point${points.length === 1 ? "" : "s"} for event ${inspection.eventId} as a file download.`;
          return;
        }
        if (exported.mode === "cancelled") {
          this.statusLine = "CSV export cancelled.";
          return;
        }
        this.statusLine =
          "CSV export is unavailable in the current environment.";
      } catch (error) {
        this.statusLine =
          String(error || "").trim() ||
          "Failed to export event path replay points.";
      }
    },

    clearTupleInspection(): void {
      this.activeTupleInspection = null;
      this.activeEventPathInspection = null;
    },

    clearEventPathInspection(): void {
      this.activeEventPathInspection = null;
    },

    async loadTupleInspection(tupleIndex: number): Promise<void> {
      const artifactPath = this.activeArtifactPath;
      if (!artifactPath) {
        this.statusLine =
          "Run or load an options expression artifact before inspecting tuple events.";
        return;
      }
      if (!this.activeRankedResults[tupleIndex]) {
        this.statusLine =
          "Select a valid ranked tuple before loading detail events.";
        return;
      }
      this.inspectionBusy = true;
      this.activeEventPathInspection = null;
      try {
        const inspection = await inspectOptionsExpressionTuple({
          artifactPath,
          tupleIndex,
        });
        this.activeTupleInspection = inspection;
        this.statusLine = `Loaded ${inspection.rows.length} event row${inspection.rows.length === 1 ? "" : "s"} for tuple #${tupleIndex + 1}.`;
      } catch (error) {
        this.statusLine =
          String(error || "").trim() || "Failed to load tuple inspection rows.";
      } finally {
        this.inspectionBusy = false;
      }
    },

    async loadEventPathInspection(
      tupleIndex: number,
      eventIndex: number,
    ): Promise<void> {
      const artifactPath = this.activeArtifactPath;
      if (!artifactPath) {
        this.statusLine =
          "Run or load an options expression artifact before inspecting event paths.";
        return;
      }
      this.eventPathBusy = true;
      try {
        const inspection = await inspectOptionsExpressionEventPath({
          artifactPath,
          tupleIndex,
          eventIndex,
        });
        this.activeEventPathInspection = inspection;
        this.statusLine = `Loaded ${inspection.pointCount} path point${inspection.pointCount === 1 ? "" : "s"} for event ${inspection.eventId}.`;
      } catch (error) {
        this.statusLine =
          String(error || "").trim() || "Failed to load event-path inspection.";
      } finally {
        this.eventPathBusy = false;
      }
    },
  },
});
