<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import AppVisualIcon from "./AppVisualIcon.vue";
import type {
  OptionsExpressionCoverageAdequacyVerdict,
  OptionsExpressionArtifactSummary,
  OptionsExpressionDatasetQuoteQualitySummary,
  OptionsExpressionEventInspectionRow,
  OptionsExpressionMetricsSummary,
  OptionsExpressionTupleSummary,
  OptionsExpressionValidationVerdict,
} from "../services/optionsExpressionService";
import {
  fillModelGuidanceForPriceSemantics,
  isFillModelCompatibleWithPriceSemantics,
  recommendedFillModelForPriceSemantics,
  useOptionsExpressionStore,
  OPTIONS_BACKEND_PREFERENCE_OPTIONS,
  OPTIONS_EXIT_TEMPLATE_OPTIONS,
  OPTIONS_FILL_MODEL_OPTIONS,
  OPTIONS_IMPORT_PRICE_SEMANTICS_OPTIONS,
  OPTIONS_MONEYNESS_BUCKET_OPTIONS,
  OPTIONS_SPLIT_ASSIGNMENT_MODE_OPTIONS,
  OPTIONS_STRUCTURE_FAMILY_OPTIONS,
  OPTIONS_SUPPORTED_PARENT_STRATEGIES,
  OPTIONS_REALISM_LANE_OPTIONS,
  OPTIONS_VALIDATION_DATASET_POLICY_OPTIONS,
  OPTIONS_VALIDATION_PLAN_MODE_OPTIONS,
  priceSemanticsForRealismLane,
} from "../stores/optionsExpressionStore";
import {
  emptyOptionsWorkspaceIllustration,
  optionsCurrentResultIcon,
  optionsDatasetImportIcon,
  optionsEventPathReplayIcon,
  optionsFeasibilityPreviewIcon,
  optionsFollowupWorkflowIcon,
  optionsProviderDownloadIcon,
  optionsTupleDetailIcon,
} from "../utils/appSvgAssets";
import {
  buildOptionsFinalistDashboard,
  formatOptionsFinalistNumber,
} from "../utils/optionsFinalistDashboardModel";

const props = withDefaults(
  defineProps<{
    parentArtifactPath: string;
    parentStrategyKind: string;
    parentDatasetLabel: string;
    parentCompatible?: boolean;
  }>(),
  {
    parentCompatible: true,
  },
);

const store = useOptionsExpressionStore();
const {
  activeArtifactPath,
  activeArtifactTimestamp,
  activeBackendUsed,
  activeBackendFallbackReason,
  activeEventPathInspection,
  activePriceSemantics,
  artifacts,
  datasets,
  datasetsBusy,
  eventPathBusy,
  artifactsBusy,
  downloadBusy,
  feasibilityBusy,
  filteredArtifacts,
  importBusy,
  importRootSymbol,
  importPriceSemantics,
  realismLane,
  realismLaneDatasets,
  realismLanePriceSemantics,
  downloadDatasetIdInput,
  downloadFromDateInput,
  downloadToDateInput,
  downloadContractType,
  downloadExpirationFromDateInput,
  downloadExpirationToDateInput,
  downloadMaxDteDaysInput,
  downloadStrikeMinInput,
  downloadStrikeMaxInput,
  downloadMaxContractsInput,
  loadBusy,
  latestFeasibility,
  latestValidation,
  latestValidationFeasibility,
  latestDatasetDownload,
  parentRootHint,
  runBusy,
  selectedArtifactPath,
  selectedDataset,
  selectedDatasetId,
  statusLine,
  supportedParent,
  activeDiagnostics,
  activeDatasetProvenance,
  activeTupleInspection,
  activeOptionsDatasetHash,
  activeRequest,
  activeStaging,
  activeRankedResults,
  inspectionBusy,
  delayMinutesInput,
  dteBucketsInput,
  holdMinutesInput,
  targetReturnInput,
  stopReturnInput,
  selectedMoneynessBuckets,
  exitTemplate,
  fillModel,
  structureFamily,
  backendPreference,
  splitAssignmentMode,
  validationAcceptance,
  validationBusy,
  validationDatasetPolicy,
  validationFormErrorMessages,
  validationFormValid,
  validationFeasibilityBusy,
  validationIncludeHoldoutLeader,
  validationIncludeRobustLeader,
  validationIncludeSealedFinalists,
  validationPlanMode,
  validationStartDateInput,
  validationEndDateInput,
  validationTrainSessionCountInput,
  validationSessionCountInput,
  validationStepSessionCountInput,
  validationTupleIndexesInput,
  topKInput,
  sealedFinalistCountInput,
  minValidTradesInput,
  splitRatioSearchInput,
  splitRatioHoldoutInput,
  maxEntryLagMinutesInput,
  maxPremiumInput,
  minPremiumInput,
  minVolumeInput,
  minOpenInterestInput,
} = storeToRefs(store);

function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
}

function isSupportedParentStrategyKind(value: unknown): boolean {
  return (OPTIONS_SUPPORTED_PARENT_STRATEGIES as readonly string[]).includes(
    normalizeText(value),
  );
}

function formatList(value: Array<number | string> | null | undefined): string {
  return Array.isArray(value) && value.length > 0 ? value.join(", ") : "—";
}

function formatTimestamp(value: number | null | undefined): string {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) return "—";
  try {
    return new Date(parsed).toISOString();
  } catch {
    return "—";
  }
}

function formatOptionalDecimal(
  value: number | null | undefined,
  digits = 4,
): string {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed.toFixed(digits) : "—";
}

function formatPercent(value: number | null | undefined, digits = 1): string {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? `${(parsed * 100).toFixed(digits)}%` : "—";
}

function formatOptionalMinutes(value: number | null | undefined): string {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? `${parsed.toFixed(0)}m` : "—";
}

function formatLatency(
  ms: number | null | undefined,
  us: number | null | undefined,
): string {
  const micros = Number(us);
  if (Number.isFinite(micros) && micros > 0 && micros < 10_000) {
    return `${(micros / 1_000).toFixed(3)} ms`;
  }
  const millis = Number(ms);
  return Number.isFinite(millis) ? `${millis.toFixed(0)} ms` : "—";
}

function formatSplitAssignmentMode(value: string | null | undefined): string {
  switch (normalizeText(value)) {
    case "session_chronological":
      return "session_chronological";
    case "event_chronological":
    default:
      return "event_chronological";
  }
}

function formatValidationDatasetPolicy(
  value: string | null | undefined,
): string {
  switch (normalizeText(value)) {
    case "allow_same_root_replacement":
      return "Same-root replacement";
    case "allow_explicit_cross_lane_comparison":
      return "Cross-lane comparison";
    case "require_exact_replay":
    default:
      return "Exact replay";
  }
}

function formatValidationPlanMode(value: string | null | undefined): string {
  switch (normalizeText(value)) {
    case "session_chronological_replay":
      return "Session replay";
    case "date_range_holdout":
      return "Date-range holdout";
    case "anchored_walk_forward":
      return "Anchored walk-forward";
    case "rolling_session_walk_forward":
      return "Rolling walk-forward";
    case "original_embedded_splits":
    default:
      return "Original splits";
  }
}

function formatValidationPlanDetail(
  plan:
    | {
        mode?: string;
        startTimestampMs?: number | null;
        endTimestampMs?: number | null;
        trainSessionCount?: number | null;
        validationSessionCount?: number | null;
        stepSessionCount?: number | null;
      }
    | null
    | undefined,
): string {
  if (!plan) return "";
  const mode = normalizeText(plan.mode);
  if (mode === "date_range_holdout") {
    return `${formatTimestamp(plan.startTimestampMs)} to ${formatTimestamp(
      plan.endTimestampMs,
    )}`;
  }
  if (
    mode === "anchored_walk_forward" ||
    mode === "rolling_session_walk_forward"
  ) {
    return `train ${plan.trainSessionCount ?? "—"} · validate ${
      plan.validationSessionCount ?? "—"
    } · step ${plan.stepSessionCount ?? "—"}`;
  }
  return "";
}

function formatRealismLaneLabel(value: string | null | undefined): string {
  switch (normalizeText(value)) {
    case "derived_mid":
      return "Derived-mid";
    case "quote_aware":
      return "Quote-aware";
    case "aggregate":
    default:
      return "Aggregate";
  }
}

function formatBytes(value: number | null | undefined): string {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed < 0) return "—";
  if (parsed >= 1024 * 1024)
    return `${(parsed / (1024 * 1024)).toFixed(2)} MiB`;
  if (parsed >= 1024) return `${(parsed / 1024).toFixed(1)} KiB`;
  return `${parsed.toFixed(0)} B`;
}

const VALIDATION_STRESS_DROP_THRESHOLD = 0.03;
const VALIDATION_MONOTONICITY_EPSILON = 1e-9;
const SELECTED_TUPLE_SCOPE_LABEL = "Selected tuple only";
const RETAINED_SCOPE_LABEL = "Within retained top_k only";
const SEALED_SCOPE_LABEL = "Within sealed finalists only";

const optionsFinalistDashboard = computed(() =>
  buildOptionsFinalistDashboard(latestValidation.value),
);

type ValidationTone = "pass" | "pending" | "mixed";
type ValidationMonotonicityState =
  | "monotone"
  | "non_monotone"
  | "insufficient_lanes";

interface ValidationPresentationSummary {
  verdict: OptionsExpressionValidationVerdict;
  label: string;
  tone: ValidationTone;
  detail: string;
}

interface ValidationInterpretationValueRow {
  key: string;
  label: string;
  value: string;
  detail: string;
  scopeLabel: string;
}

interface ValidationInterpretationStateRow {
  key: string;
  label: string;
  statusLabel: string;
  detail: string;
  scopeLabel: string;
}

interface RealismLanePresentationSummary {
  label: string;
  shortLabel: string;
  detail: string;
}

interface WinnerDisagreementPresentationSummary {
  label: string;
  tone: ValidationTone;
  detail: string;
}

interface BackendRoutePresentationSummary {
  label: string;
  detail: string;
}

interface CoverageAdequacyPresentationSummary {
  verdict: OptionsExpressionCoverageAdequacyVerdict;
  label: string;
  tone: ValidationTone;
  detail: string;
}

interface CoverageAdequacyPayload {
  verdict: OptionsExpressionCoverageAdequacyVerdict;
  nonPromotable: boolean;
  reasons?: string[];
  searchOverlapRate: number | null | undefined;
  holdoutOverlapRate: number | null | undefined;
  sealedTestOverlapRate: number | null | undefined;
  searchCandidateCoverageRate: number | null | undefined;
  holdoutCandidateCoverageRate: number | null | undefined;
  sealedTestCandidateCoverageRate: number | null | undefined;
  searchBaselineOutcomeCoverageRate: number | null | undefined;
  holdoutBaselineOutcomeCoverageRate: number | null | undefined;
  sealedTestBaselineOutcomeCoverageRate: number | null | undefined;
}

interface ArtifactMetaOverviewRow {
  key: string;
  label: string;
  value: string;
}

interface ArtifactMetaGroupRow {
  key: string;
  label: string;
  artifactCount: number;
  validatedCount: number;
  fragileCount: number;
  avgRobust: number | null;
  avgSelectionEv: number | null;
  avgValidationEv: number | null;
  avgStressDrop: number | null;
}

interface ArtifactLaneDisagreementRow {
  key: string;
  label: string;
  laneCount: number;
  selectionWinnerLane: string;
  validationWinnerLane: string;
  driftLabel: string;
  validationSpread: number | null;
  laneSummary: string;
}

function pushUniqueRecommendation(target: string[], message: string): void {
  if (!target.includes(message)) {
    target.push(message);
  }
}

function formatBucketInvalidReasons(
  reasons: Array<{ reason: string; count: number }> | null | undefined,
): string {
  if (!Array.isArray(reasons) || reasons.length === 0) return "—";
  return reasons
    .slice(0, 2)
    .map((entry) => `${normalizeText(entry.reason)} (${entry.count})`)
    .join(" | ");
}

function formatMoneynessBucketLabel(value: string | null | undefined): string {
  const normalized = normalizeText(value);
  return normalized.length > 0 ? normalized.split("_").join(" ") : "—";
}

function formatCoverageAdequacyLabel(
  value: OptionsExpressionCoverageAdequacyVerdict,
): string {
  switch (value) {
    case "healthy":
      return "Healthy";
    case "weak_dataset_overlap":
      return "Weak dataset overlap";
    case "search_overlap_missing":
      return "Search overlap missing";
    case "search_coverage_weak":
      return "Search coverage weak";
    case "holdout_coverage_weak":
      return "Holdout coverage weak";
    case "sealed_coverage_weak":
      return "Sealed coverage weak";
    case "contract_eligibility_fragile":
      return "Contract-eligibility fragile";
    case "late_entry_fragile":
      return "Late-entry fragile";
    case "strike_window_fragile":
      return "Strike-window fragile";
    case "mixed_fragile":
      return "Mixed fragile";
    case "unknown":
    default:
      return "Unknown";
  }
}

function coverageAdequacyTone(
  verdict: OptionsExpressionCoverageAdequacyVerdict,
  nonPromotable: boolean,
): ValidationTone {
  if (verdict === "healthy") return "pass";
  if (verdict === "unknown") return "pending";
  return nonPromotable ? "mixed" : "pending";
}

function describeCoverageAdequacy(
  payload: CoverageAdequacyPayload | null | undefined,
): CoverageAdequacyPresentationSummary | null {
  if (!payload) return null;
  const verdict = payload.verdict || "unknown";
  const overlapBits = [
    `search overlap ${formatPercent(payload.searchOverlapRate, 1)}`,
    `holdout ${formatPercent(payload.holdoutOverlapRate, 1)}`,
    `sealed ${formatPercent(payload.sealedTestOverlapRate, 1)}`,
  ];
  const coverageBits = [
    `search cand ${formatPercent(payload.searchCandidateCoverageRate, 1)}`,
    `search out ${formatPercent(payload.searchBaselineOutcomeCoverageRate, 1)}`,
  ];
  let detail = `${overlapBits.join(" · ")} | ${coverageBits.join(" · ")}`;
  if (payload.reasons?.length) {
    detail = payload.reasons.slice(0, 2).join(" ");
  } else {
    switch (verdict) {
      case "healthy":
        detail = `Overlap and search-stage coverage are currently adequate. ${detail}`;
        break;
      case "search_overlap_missing":
        detail = `The imported options dataset does not cover any search events. ${detail}`;
        break;
      case "weak_dataset_overlap":
        detail = `The parent artifact and current options dataset only overlap partially. ${detail}`;
        break;
      case "search_coverage_weak":
        detail = `Search events overlap the dataset, but too few become executable candidates or outcomes. ${detail}`;
        break;
      case "holdout_coverage_weak":
        detail = `Search looks usable, but holdout coverage is too thin to trust the current validation story. ${detail}`;
        break;
      case "sealed_coverage_weak":
        detail = `Sealed coverage is too thin to treat the current finalist stage as robust. ${detail}`;
        break;
      case "contract_eligibility_fragile":
        detail = `Contract-eligibility timing is dominating structural invalidity. ${detail}`;
        break;
      case "late_entry_fragile":
        detail = `Late-entry pressure is dominating structural invalidity. ${detail}`;
        break;
      case "strike_window_fragile":
        detail = `Strike-window misses are dominating structural invalidity. ${detail}`;
        break;
      case "mixed_fragile":
        detail = `This slice remains fragile for mixed overlap/coverage reasons. ${detail}`;
        break;
      case "unknown":
      default:
        detail = `Legacy or incomplete adequacy signal. ${detail}`;
        break;
    }
  }
  return {
    verdict,
    label: `${formatCoverageAdequacyLabel(verdict)}${payload.nonPromotable ? " · non-promotable" : ""}`,
    tone: coverageAdequacyTone(verdict, payload.nonPromotable),
    detail,
  };
}

function strongestTupleValidationBaseMetrics(
  tuple: OptionsExpressionTupleSummary | null | undefined,
): OptionsExpressionMetricsSummary | null {
  return (
    supportedMetrics(tuple?.sealedEvaluationMetrics) ??
    supportedMetrics(tuple?.holdoutMetrics) ??
    null
  );
}

function strongestTupleValidationStressMetrics(
  tuple: OptionsExpressionTupleSummary | null | undefined,
): OptionsExpressionMetricsSummary | null {
  return (
    supportedMetrics(tuple?.sealedExtremeStressMetrics) ??
    supportedMetrics(tuple?.sealedHarsherStressMetrics) ??
    supportedMetrics(tuple?.sealedStressMetrics) ??
    supportedMetrics(tuple?.extremeStressMetrics) ??
    supportedMetrics(tuple?.harsherStressMetrics) ??
    supportedMetrics(tuple?.stressMetrics) ??
    null
  );
}

function strongestArtifactValidationBaseExpectancy(
  artifact: OptionsExpressionArtifactSummary | null | undefined,
): number | null {
  return (
    artifact?.bestSealedExpectancyPerPremium ??
    artifact?.bestHoldoutExpectancyPerPremium ??
    null
  );
}

function strongestArtifactValidationStressExpectancy(
  artifact: OptionsExpressionArtifactSummary | null | undefined,
): number | null {
  return (
    artifact?.bestSealedExtremeStressExpectancyPerPremium ??
    artifact?.bestSealedHarsherStressExpectancyPerPremium ??
    artifact?.bestSealedStressExpectancyPerPremium ??
    artifact?.bestExtremeStressExpectancyPerPremium ??
    artifact?.bestHarsherStressExpectancyPerPremium ??
    artifact?.bestStressExpectancyPerPremium ??
    null
  );
}

function artifactValidationDelta(
  artifact: OptionsExpressionArtifactSummary | null | undefined,
): number | null {
  if (!artifact) return null;
  return metricDeltaAgainstSelection(
    artifact.bestSelectionExpectancyPerPremium,
    strongestArtifactValidationBaseExpectancy(artifact),
  );
}

function artifactValidationSupportRatio(
  artifact: OptionsExpressionArtifactSummary | null | undefined,
): number | null {
  if (!artifact || !artifact.tupleCount) return null;
  return (artifact.validatedTupleCount ?? 0) / artifact.tupleCount;
}

function supportedMetrics(
  metrics: OptionsExpressionMetricsSummary | null | undefined,
): OptionsExpressionMetricsSummary | null {
  if (!metrics) return null;
  return metrics.validTrades > 0 ? metrics : null;
}

function metricsTotalEvents(
  metrics: OptionsExpressionMetricsSummary | null | undefined,
): number {
  if (!metrics) return 0;
  return Math.max(0, metrics.validTrades + metrics.invalidEvents);
}

function metricsSupportRate(
  metrics: OptionsExpressionMetricsSummary | null | undefined,
): number | null {
  const totalEvents = metricsTotalEvents(metrics);
  if (!totalEvents) return null;
  return (metrics?.validTrades ?? 0) / totalEvents;
}

function formatMetricsSupport(
  metrics: OptionsExpressionMetricsSummary | null | undefined,
): string {
  if (!metrics) return "—";
  const totalEvents = metricsTotalEvents(metrics);
  if (!totalEvents) return `${metrics.validTrades} / 0`;
  return `${metrics.validTrades} / ${totalEvents} (${formatPercent(metricsSupportRate(metrics), 1)})`;
}

function formatSignedOptionalDecimal(
  value: number | null | undefined,
  digits = 4,
): string {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return "—";
  return `${parsed >= 0 ? "+" : ""}${parsed.toFixed(digits)}`;
}

function metricsExpectancy(
  metrics: OptionsExpressionMetricsSummary | null | undefined,
  requireSupported = true,
): number | null {
  const resolved = requireSupported ? supportedMetrics(metrics) : metrics;
  const parsed = Number(resolved?.expectancyPerPremium);
  return Number.isFinite(parsed) ? parsed : null;
}

function buildExpectancyTransitionDetail(
  fromLabel: string,
  fromExpectancy: number,
  toLabel: string,
  toExpectancy: number,
): string {
  return `${fromLabel} ${formatOptionalDecimal(fromExpectancy)} -> ${toLabel} ${formatOptionalDecimal(toExpectancy)}`;
}

function buildValidationDriftRow(
  key: string,
  label: string,
  fromLabel: string,
  fromExpectancy: number | null,
  toLabel: string,
  toExpectancy: number | null,
  scopeLabel: string,
): ValidationInterpretationValueRow | null {
  if (fromExpectancy == null || toExpectancy == null) return null;
  const from = Number(fromExpectancy);
  const to = Number(toExpectancy);
  if (!Number.isFinite(from) || !Number.isFinite(to)) return null;
  return {
    key,
    label,
    value: formatSignedOptionalDecimal(to - from),
    detail: buildExpectancyTransitionDetail(fromLabel, from, toLabel, to),
    scopeLabel,
  };
}

function monotonicityStatusLabel(state: ValidationMonotonicityState): string {
  switch (state) {
    case "monotone":
      return "Monotone";
    case "non_monotone":
      return "Non-monotone";
    case "insufficient_lanes":
    default:
      return "Insufficient lanes";
  }
}

function describeValidationMonotonicity(
  points: Array<{ label: string; expectancy: number }>,
): { state: ValidationMonotonicityState; detail: string } {
  if (points.length < 2) {
    const onlyPoint = points[0];
    return {
      state: "insufficient_lanes",
      detail: onlyPoint
        ? `Observed lane: ${onlyPoint.label} ${formatOptionalDecimal(onlyPoint.expectancy)}`
        : "No supported validation lanes are present.",
    };
  }
  for (let index = 1; index < points.length; index += 1) {
    const previous = points[index - 1];
    const current = points[index];
    if (!previous || !current) continue;
    if (
      current.expectancy >
      previous.expectancy + VALIDATION_MONOTONICITY_EPSILON
    ) {
      return {
        state: "non_monotone",
        detail: points
          .map(
            (point) =>
              `${point.label} ${formatOptionalDecimal(point.expectancy)}`,
          )
          .join(" -> "),
      };
    }
  }
  return {
    state: "monotone",
    detail: points
      .map(
        (point) => `${point.label} ${formatOptionalDecimal(point.expectancy)}`,
      )
      .join(" -> "),
  };
}

function buildValidationRegretRow(
  key: string,
  label: string,
  scopeLabel: string,
  selectedExpectancy: number | null,
  candidateExpectancies: Array<number | null>,
): ValidationInterpretationValueRow | null {
  if (selectedExpectancy == null) return null;
  const selected = Number(selectedExpectancy);
  if (!Number.isFinite(selected)) return null;
  let best = Number.NEGATIVE_INFINITY;
  let comparatorCount = 0;
  for (const value of candidateExpectancies) {
    if (value == null) continue;
    const candidate = Number(value);
    if (!Number.isFinite(candidate)) continue;
    comparatorCount += 1;
    if (candidate > best) best = candidate;
  }
  if (comparatorCount < 2) return null;
  return {
    key,
    label,
    value: formatOptionalDecimal(best - selected),
    detail: `Best scope EV ${formatOptionalDecimal(best)} across ${comparatorCount} scope row${comparatorCount === 1 ? "" : "s"}.`,
    scopeLabel,
  };
}

function supportedTupleMetricsByKey(
  tuple: OptionsExpressionTupleSummary | null | undefined,
  key: string,
): OptionsExpressionMetricsSummary | null {
  if (!tuple) return null;
  switch (key) {
    case "holdout":
      return supportedMetrics(tuple.holdoutMetrics);
    case "sealed":
      return supportedMetrics(tuple.sealedEvaluationMetrics);
    case "stress":
      return supportedMetrics(tuple.stressMetrics);
    case "sealed_stress":
      return supportedMetrics(tuple.sealedStressMetrics);
    case "harsher":
      return supportedMetrics(tuple.harsherStressMetrics);
    case "sealed_harsher":
      return supportedMetrics(tuple.sealedHarsherStressMetrics);
    case "extreme":
      return supportedMetrics(tuple.extremeStressMetrics);
    case "sealed_extreme":
      return supportedMetrics(tuple.sealedExtremeStressMetrics);
    default:
      return null;
  }
}

function buildValidationFamilyPoints(
  tuple: OptionsExpressionTupleSummary | null | undefined,
  family: "holdout" | "sealed",
): Array<{ label: string; expectancy: number }> {
  const descriptors =
    family === "sealed"
      ? [
          { key: "sealed", label: "Sealed finalist" },
          { key: "sealed_stress", label: "Sealed stress" },
          { key: "sealed_harsher", label: "Sealed harsher" },
          { key: "sealed_extreme", label: "Sealed extreme" },
        ]
      : [
          { key: "holdout", label: "Holdout" },
          { key: "stress", label: "Stress" },
          { key: "harsher", label: "Harsher" },
          { key: "extreme", label: "Extreme" },
        ];
  return descriptors
    .map((descriptor) => {
      const expectancy = metricsExpectancy(
        supportedTupleMetricsByKey(tuple, descriptor.key),
      );
      if (expectancy == null || !Number.isFinite(Number(expectancy)))
        return null;
      return {
        label: descriptor.label,
        expectancy: Number(expectancy),
      };
    })
    .filter((entry): entry is { label: string; expectancy: number } =>
      Boolean(entry),
    );
}

function formatArtifactSupportCount(
  count: number | null | undefined,
  total: number | null | undefined,
): string {
  const safeCount = Math.max(0, Number(count ?? 0));
  const safeTotal = Math.max(0, Number(total ?? 0));
  if (!safeTotal) return `${safeCount} / 0`;
  return `${safeCount} / ${safeTotal} (${formatPercent(safeCount / safeTotal, 1)})`;
}

function artifactStressFragilityRatio(
  artifact: OptionsExpressionArtifactSummary | null | undefined,
): number | null {
  if (!artifact) return null;
  const denominator = artifact.validatedTupleCount ?? artifact.tupleCount ?? 0;
  if (!denominator) return null;
  return (artifact.stressFragileTupleCount ?? 0) / denominator;
}

function formatMetaGroupKey(
  value: string | null | undefined,
  fallback: string,
): string {
  const normalized = normalizeText(value);
  return normalized || fallback;
}

function averageNumbers(
  values: Array<number | null | undefined>,
): number | null {
  const filtered = values.filter((value): value is number =>
    Number.isFinite(Number(value)),
  );
  if (filtered.length === 0) return null;
  return filtered.reduce((sum, value) => sum + value, 0) / filtered.length;
}

function buildArtifactLaneDisagreementRows(
  entries: OptionsExpressionArtifactSummary[],
): ArtifactLaneDisagreementRow[] {
  const grouped = new Map<string, OptionsExpressionArtifactSummary[]>();
  for (const entry of entries) {
    const key = [
      formatMetaGroupKey(entry.rootSymbol, "Unknown root"),
      formatMetaGroupKey(entry.parentStrategyKind, "Unknown parent family"),
      formatMetaGroupKey(entry.requestStructureFamily, "Unknown structure"),
    ].join(" · ");
    const bucket = grouped.get(key);
    if (bucket) {
      bucket.push(entry);
    } else {
      grouped.set(key, [entry]);
    }
  }

  return [...grouped.entries()]
    .map(([label, rows]) => {
      const bestByLane = new Map<string, OptionsExpressionArtifactSummary>();
      for (const entry of rows) {
        const lane = describeRealismLane(
          entry.priceSemantics,
          entry.requestFillModel,
        ).shortLabel;
        const existing = bestByLane.get(lane);
        const candidateScore =
          strongestArtifactValidationBaseExpectancy(entry) ??
          entry.bestSelectionExpectancyPerPremium;
        const existingScore = existing
          ? (strongestArtifactValidationBaseExpectancy(existing) ??
            existing.bestSelectionExpectancyPerPremium)
          : Number.NEGATIVE_INFINITY;
        if (!existing || candidateScore > existingScore) {
          bestByLane.set(lane, entry);
        }
      }

      if (bestByLane.size < 2) return null;

      const laneRows = [...bestByLane.entries()].map(([lane, artifact]) => ({
        lane,
        selection: artifact.bestSelectionExpectancyPerPremium,
        validation: strongestArtifactValidationBaseExpectancy(artifact),
        artifact,
      }));
      const selectionWinner = laneRows.reduce((best, entry) =>
        entry.selection > best.selection ? entry : best,
      );
      const validationRows = laneRows.filter((entry) =>
        Number.isFinite(Number(entry.validation)),
      );
      const validationWinner =
        validationRows.length > 0
          ? validationRows.reduce((best, entry) =>
              Number(entry.validation) > Number(best.validation) ? entry : best,
            )
          : null;
      let validationMin = Number.POSITIVE_INFINITY;
      let validationMax = Number.NEGATIVE_INFINITY;
      for (const entry of validationRows) {
        const value = Number(entry.validation);
        if (value < validationMin) validationMin = value;
        if (value > validationMax) validationMax = value;
      }
      const validationSpread =
        validationRows.length >= 2 ? validationMax - validationMin : null;
      const driftLabel = validationWinner
        ? validationWinner.lane === selectionWinner.lane
          ? "Aligned"
          : `${selectionWinner.lane} -> ${validationWinner.lane}`
        : "Search only";
      const laneSummary = laneRows
        .map(
          (entry) =>
            `${entry.lane} sel ${formatOptionalDecimal(entry.selection, 3)}${entry.validation != null ? ` / val ${formatOptionalDecimal(entry.validation, 3)}` : ""}`,
        )
        .join(" | ");
      return {
        key: label,
        label,
        laneCount: laneRows.length,
        selectionWinnerLane: selectionWinner.lane,
        validationWinnerLane: validationWinner?.lane ?? "—",
        driftLabel,
        validationSpread,
        laneSummary,
      };
    })
    .filter((entry): entry is ArtifactLaneDisagreementRow => Boolean(entry))
    .sort((left, right) => {
      const leftSpread = left.validationSpread ?? Number.NEGATIVE_INFINITY;
      const rightSpread = right.validationSpread ?? Number.NEGATIVE_INFINITY;
      if (rightSpread !== leftSpread) return rightSpread - leftSpread;
      return left.label.localeCompare(right.label);
    });
}

function artifactValidationLabel(
  artifact: OptionsExpressionArtifactSummary | null | undefined,
): string {
  return describeArtifactValidation(artifact)?.label ?? "Search only";
}

function artifactValidationVerdict(
  artifact: OptionsExpressionArtifactSummary | null | undefined,
): string {
  return resolveArtifactValidationVerdict(artifact);
}

function artifactStressDrop(
  artifact: OptionsExpressionArtifactSummary | null | undefined,
): number | null {
  const base = strongestArtifactValidationBaseExpectancy(artifact);
  const stressed = strongestArtifactValidationStressExpectancy(artifact);
  if (!Number.isFinite(Number(base)) || !Number.isFinite(Number(stressed)))
    return null;
  return Number(base) - Number(stressed);
}

function buildArtifactMetaGroupRows(
  entries: OptionsExpressionArtifactSummary[],
  groupKey: (entry: OptionsExpressionArtifactSummary) => string,
): ArtifactMetaGroupRow[] {
  const grouped = new Map<string, OptionsExpressionArtifactSummary[]>();
  for (const entry of entries) {
    const key = groupKey(entry);
    const bucket = grouped.get(key);
    if (bucket) {
      bucket.push(entry);
    } else {
      grouped.set(key, [entry]);
    }
  }
  return [...grouped.entries()]
    .map(([label, rows]) => ({
      key: label,
      label,
      artifactCount: rows.length,
      validatedCount: rows.filter(
        (entry) => artifactValidationVerdict(entry) !== "search_only",
      ).length,
      fragileCount: rows.filter(
        (entry) => artifactValidationVerdict(entry) === "stress_fragile",
      ).length,
      avgRobust: averageNumbers(rows.map((entry) => entry.bestRobustScore)),
      avgSelectionEv: averageNumbers(
        rows.map((entry) => entry.bestSelectionExpectancyPerPremium),
      ),
      avgValidationEv: averageNumbers(
        rows.map((entry) => strongestArtifactValidationBaseExpectancy(entry)),
      ),
      avgStressDrop: averageNumbers(
        rows.map((entry) => artifactStressDrop(entry)),
      ),
    }))
    .sort((left, right) => {
      if (right.artifactCount !== left.artifactCount)
        return right.artifactCount - left.artifactCount;
      const rightRobust = right.avgRobust ?? Number.NEGATIVE_INFINITY;
      const leftRobust = left.avgRobust ?? Number.NEGATIVE_INFINITY;
      if (rightRobust !== leftRobust) return rightRobust - leftRobust;
      return left.label.localeCompare(right.label);
    });
}

function isStressFragile(
  baseExpectancy: number | null | undefined,
  stressedExpectancy: number | null | undefined,
): boolean {
  const base = Number(baseExpectancy);
  const stressed = Number(stressedExpectancy);
  if (!Number.isFinite(base) || !Number.isFinite(stressed)) return false;
  return stressed < 0 || base - stressed >= VALIDATION_STRESS_DROP_THRESHOLD;
}

function resolveTupleValidationVerdict(
  tuple: OptionsExpressionTupleSummary | null | undefined,
): OptionsExpressionValidationVerdict {
  if (!tuple) return "search_only";
  if (tuple.validationVerdict) return tuple.validationVerdict;
  const hasSealed = Boolean(supportedMetrics(tuple.sealedEvaluationMetrics));
  const hasHoldout = Boolean(supportedMetrics(tuple.holdoutMetrics));
  const fragile = isStressFragile(
    strongestTupleValidationBaseMetrics(tuple)?.expectancyPerPremium,
    strongestTupleValidationStressMetrics(tuple)?.expectancyPerPremium,
  );
  if (hasSealed) return fragile ? "stress_fragile" : "sealed_finalist";
  if (hasHoldout) return fragile ? "stress_fragile" : "holdout_supported";
  return "search_only";
}

function resolveArtifactValidationVerdict(
  artifact: OptionsExpressionArtifactSummary | null | undefined,
): OptionsExpressionValidationVerdict {
  if (!artifact) return "search_only";
  if (artifact.topValidationVerdict) return artifact.topValidationVerdict;
  const hasSealed = (artifact.supportedSealedTupleCount ?? 0) > 0;
  const hasHoldout = (artifact.supportedHoldoutTupleCount ?? 0) > 0;
  const fragile = isStressFragile(
    strongestArtifactValidationBaseExpectancy(artifact),
    strongestArtifactValidationStressExpectancy(artifact),
  );
  if (hasSealed) return fragile ? "stress_fragile" : "sealed_finalist";
  if (hasHoldout) return fragile ? "stress_fragile" : "holdout_supported";
  return "search_only";
}

function formatValidationVerdictLabel(
  verdict: OptionsExpressionValidationVerdict,
): string {
  switch (verdict) {
    case "holdout_supported":
      return "Holdout-supported";
    case "sealed_finalist":
      return "Sealed finalist";
    case "stress_fragile":
      return "Stress-fragile";
    case "search_only":
    default:
      return "Search only";
  }
}

function validationToneForVerdict(
  verdict: OptionsExpressionValidationVerdict,
): ValidationTone {
  switch (verdict) {
    case "sealed_finalist":
      return "pass";
    case "stress_fragile":
      return "mixed";
    case "holdout_supported":
    case "search_only":
    default:
      return "pending";
  }
}

function describeTupleValidation(
  tuple: OptionsExpressionTupleSummary | null | undefined,
): ValidationPresentationSummary | null {
  if (!tuple) return null;
  const verdict = resolveTupleValidationVerdict(tuple);
  const baseExpectancy =
    strongestTupleValidationBaseMetrics(tuple)?.expectancyPerPremium ?? null;
  const stressedExpectancy =
    strongestTupleValidationStressMetrics(tuple)?.expectancyPerPremium ?? null;
  let detail =
    "No supported holdout or sealed trades are present yet; this tuple is still a search-stage candidate.";
  switch (verdict) {
    case "holdout_supported":
      detail =
        tuple.sealedFinalist && !supportedMetrics(tuple.sealedEvaluationMetrics)
          ? "Supported holdout trades are present, but this sealed-budget tuple has not yet produced supported sealed validation."
          : "Supported holdout trades are present, but this tuple has not yet been promoted into supported sealed validation.";
      break;
    case "sealed_finalist":
      detail = tuple.sealedFinalRank
        ? `Promoted into the sealed finalist set (#${tuple.sealedFinalRank}) and still holding up under the current supported stress lanes.`
        : "Promoted into the sealed finalist set and still holding up under the current supported stress lanes.";
      break;
    case "stress_fragile":
      detail =
        Number.isFinite(baseExpectancy) && Number.isFinite(stressedExpectancy)
          ? `Supported validation exists, but EV / premium falls from ${formatOptionalDecimal(baseExpectancy)} to ${formatOptionalDecimal(stressedExpectancy)} under the current stress lanes.`
          : "Supported validation exists, but the tuple degrades materially under the current stress lanes.";
      break;
    case "search_only":
    default:
      break;
  }
  return {
    verdict,
    label: formatValidationVerdictLabel(verdict),
    tone: validationToneForVerdict(verdict),
    detail,
  };
}

function describeArtifactValidation(
  artifact: OptionsExpressionArtifactSummary | null | undefined,
): ValidationPresentationSummary | null {
  if (!artifact) return null;
  const verdict = resolveArtifactValidationVerdict(artifact);
  const baseExpectancy = strongestArtifactValidationBaseExpectancy(artifact);
  const stressedExpectancy =
    strongestArtifactValidationStressExpectancy(artifact);
  let detail = "This artifact currently reads as search-stage only.";
  switch (verdict) {
    case "holdout_supported":
      detail = `Top tuple has supported holdout validation across ${artifact.validatedTupleCount ?? 0} validated tuple${(artifact.validatedTupleCount ?? 0) === 1 ? "" : "s"}, but no supported sealed confirmation yet.`;
      break;
    case "sealed_finalist":
      detail = `Top tuple cleared supported sealed evaluation. ${artifact.sealedEvaluatedTupleCount ?? 0}/${artifact.sealedFinalistBudget ?? 0} tuple${(artifact.sealedEvaluatedTupleCount ?? 0) === 1 ? "" : "s"} were evaluated in the sealed stage.`;
      break;
    case "stress_fragile":
      detail =
        Number.isFinite(baseExpectancy) && Number.isFinite(stressedExpectancy)
          ? `Top tuple has supported validation, but EV / premium falls from ${formatOptionalDecimal(baseExpectancy)} to ${formatOptionalDecimal(stressedExpectancy)} under stress.`
          : `Top tuple has supported validation, but ${artifact.stressFragileTupleCount ?? 0} tuple${(artifact.stressFragileTupleCount ?? 0) === 1 ? "" : "s"} degrade materially under stress.`;
      break;
    case "search_only":
    default:
      break;
  }
  return {
    verdict,
    label: formatValidationVerdictLabel(verdict),
    tone: validationToneForVerdict(verdict),
    detail,
  };
}

function describeRealismLane(
  priceSemantics: string | null | undefined,
  fillModel: string | null | undefined,
): RealismLanePresentationSummary {
  const semantics = normalizeText(priceSemantics) || "minute_aggregate";
  const activeFillModel =
    normalizeText(fillModel) ||
    recommendedFillModelForPriceSemantics(semantics);
  switch (semantics) {
    case "top_of_book_quote":
      return {
        label: "Quote-aware lane",
        shortLabel: "Quote-aware",
        detail: `${activeFillModel} over top_of_book_quote rows. This is the only quote-side execution lane currently surfaced in the options workflow, and new dataset acquisition remains CSV import-first today.`,
      };
    case "derived_mid_snapshot":
      return {
        label: "Derived-mid lane",
        shortLabel: "Derived-mid",
        detail: `${activeFillModel} over derived_mid_snapshot rows. This is a first-class realism lane, but runtime execution remains aggregate-style and non-quote-side; new dataset acquisition remains CSV import-first today.`,
      };
    case "minute_aggregate":
    default:
      return {
        label: "Aggregate lane",
        shortLabel: "Aggregate",
        detail: `${activeFillModel} over minute_aggregate rows. This remains the canonical fast Polygon.io 1m optimization lane.`,
      };
  }
}

function describeBackendRoute(
  requestedBackend: string | null | undefined,
  backendUsed: string | null | undefined,
  fallbackReason: string | null | undefined,
): BackendRoutePresentationSummary {
  const requested = normalizeText(requestedBackend) || "auto";
  const used = normalizeText(backendUsed) || "cpu_reference";
  const fallback = normalizeText(fallbackReason);
  const label = `${requested} -> ${used}`;
  if (fallback) {
    return {
      label,
      detail: `Fallback or promotion override: ${fallback}`,
    };
  }
  if (requested === "auto") {
    return {
      label,
      detail: `Automatic backend policy selected ${used} for this workload.`,
    };
  }
  return {
    label,
    detail: `Explicit backend preference ${requested} executed without fallback.`,
  };
}

function formatTupleIdentity(
  tuple: OptionsExpressionTupleSummary | null | undefined,
): string {
  if (!tuple) return "—";
  return `${tuple.structureFamily || "long_single"} · ${tuple.dteDays}D · ${tuple.moneynessBucket} · ${tuple.delayMinutes}m/${tuple.holdMinutes}m`;
}

function metricDeltaAgainstSelection(
  selectionExpectancy: number | null | undefined,
  candidateExpectancy: number | null | undefined,
): number | null {
  const base = Number(selectionExpectancy);
  const candidate = Number(candidateExpectancy);
  if (!Number.isFinite(base) || !Number.isFinite(candidate)) return null;
  return candidate - base;
}

function strongestTupleStressDrop(
  tuple: OptionsExpressionTupleSummary | null | undefined,
): number | null {
  const base = tuple?.selectionMetrics.expectancyPerPremium;
  const stressed =
    strongestTupleValidationStressMetrics(tuple)?.expectancyPerPremium;
  const delta = metricDeltaAgainstSelection(base, stressed);
  return delta == null ? null : -delta;
}

function bestTupleForMetric(
  rows: OptionsExpressionTupleSummary[],
  metric: (row: OptionsExpressionTupleSummary) => number | null | undefined,
): { row: OptionsExpressionTupleSummary; index: number; value: number } | null {
  let best: {
    row: OptionsExpressionTupleSummary;
    index: number;
    value: number;
  } | null = null;
  rows.forEach((row, index) => {
    const value = Number(metric(row));
    if (!Number.isFinite(value)) return;
    if (!best || value > best.value) {
      best = { row, index, value };
    }
  });
  return best;
}

function describeArtifactProvenanceDiff(
  entry: OptionsExpressionArtifactSummary,
  baseline: OptionsExpressionArtifactSummary | null | undefined,
): string {
  if (!baseline) return "baseline";
  if (normalizeText(entry.path) === normalizeText(baseline.path))
    return "selected baseline";
  const diffs: string[] = [];
  if (
    normalizeText(entry.optionsDatasetId) !==
    normalizeText(baseline.optionsDatasetId)
  ) {
    diffs.push("dataset");
  }
  if (
    normalizeText(entry.priceSemantics) !==
    normalizeText(baseline.priceSemantics)
  ) {
    diffs.push("semantics");
  }
  if (
    normalizeText(entry.requestFillModel) !==
    normalizeText(baseline.requestFillModel)
  ) {
    diffs.push("fill");
  }
  if (
    normalizeText(entry.requestStructureFamily) !==
    normalizeText(baseline.requestStructureFamily)
  ) {
    diffs.push("structure");
  }
  if (
    normalizeText(entry.requestSplitAssignmentMode) !==
    normalizeText(baseline.requestSplitAssignmentMode)
  ) {
    diffs.push("split");
  }
  if (
    normalizeText(entry.requestBackendPreference) !==
      normalizeText(baseline.requestBackendPreference) ||
    normalizeText(entry.backendUsed) !== normalizeText(baseline.backendUsed)
  ) {
    diffs.push("backend");
  }
  if (
    normalizeText(entry.topValidationVerdict) !==
    normalizeText(baseline.topValidationVerdict)
  ) {
    diffs.push("validation");
  }
  return diffs.length > 0 ? diffs.join(" · ") : "same request lane";
}

watch(
  () =>
    [
      props.parentArtifactPath,
      props.parentStrategyKind,
      props.parentDatasetLabel,
      props.parentCompatible,
    ] as const,
  ([path, strategyKind, datasetLabel, parentCompatible]) => {
    store.attachParent(path, strategyKind, datasetLabel);
    if (
      parentCompatible &&
      normalizeText(path) &&
      isSupportedParentStrategyKind(strategyKind)
    ) {
      void store.bootstrap();
    }
  },
  { immediate: true },
);

const hasParentSelection = computed(
  () => normalizeText(props.parentArtifactPath).length > 0,
);

const selectedRealismLaneSemantics = computed(() =>
  priceSemanticsForRealismLane(realismLane.value),
);
const selectedRealismLaneArtifacts = computed(() =>
  filteredArtifacts.value
    .filter(
      (entry) =>
        normalizeText(entry.priceSemantics) ===
        normalizeText(selectedRealismLaneSemantics.value),
    )
    .sort((left, right) =>
      normalizeText(right.ts).localeCompare(normalizeText(left.ts)),
    ),
);
const selectedRealismLaneGuidance = computed(() =>
  fillModelGuidanceForPriceSemantics(selectedRealismLaneSemantics.value),
);
const realismLaneSummary = computed(() => {
  const matching = realismLaneDatasets.value.length;
  const total = datasets.value.length;
  return `${formatRealismLaneLabel(realismLane.value)} lane · ${matching}/${total} saved dataset${total === 1 ? "" : "s"} match`;
});
const realismLaneRouteNote = computed(() => {
  switch (normalizeText(realismLane.value)) {
    case "quote_aware":
      return "Quote-aware realism is a first-class lane with CSV import for new dataset acquisition today. Direct Polygon.io provider download in this panel remains the bounded 1m aggregate path.";
    case "derived_mid":
      return "Derived-mid realism is a first-class lane with CSV import for new dataset acquisition today. Direct Polygon.io provider download in this panel remains the bounded 1m aggregate path.";
    case "aggregate":
    default:
      return "Aggregate realism supports both normalized CSV import and direct bounded Polygon.io 1m provider download.";
  }
});
const realismLaneProductRows = computed(() => {
  const artifactsForLane = selectedRealismLaneArtifacts.value;
  const datasetsForLane = realismLaneDatasets.value;
  const validatedArtifacts = artifactsForLane.filter(
    (entry) => artifactValidationVerdict(entry) !== "search_only",
  ).length;
  const fragileArtifacts = artifactsForLane.filter(
    (entry) => artifactValidationVerdict(entry) === "stress_fragile",
  ).length;
  const avgValidation = averageNumbers(
    artifactsForLane.map((entry) =>
      strongestArtifactValidationBaseExpectancy(entry),
    ),
  );
  const avgStress = averageNumbers(
    artifactsForLane.map((entry) =>
      strongestArtifactValidationStressExpectancy(entry),
    ),
  );
  return [
    {
      key: "lane-datasets",
      label: "Saved datasets",
      value: String(datasetsForLane.length),
    },
    {
      key: "lane-artifacts",
      label: "Saved artifacts",
      value: String(artifactsForLane.length),
    },
    {
      key: "lane-validated",
      label: "Validated artifacts",
      value: artifactsForLane.length
        ? `${validatedArtifacts}/${artifactsForLane.length} (${formatPercent(validatedArtifacts / artifactsForLane.length)})`
        : "0",
    },
    {
      key: "lane-fragile",
      label: "Stress-fragile artifacts",
      value: artifactsForLane.length
        ? `${fragileArtifacts}/${artifactsForLane.length} (${formatPercent(fragileArtifacts / artifactsForLane.length)})`
        : "0",
    },
    {
      key: "lane-validation",
      label: "Avg validation EV",
      value: formatOptionalDecimal(avgValidation, 3),
    },
    {
      key: "lane-stress",
      label: "Avg stressed EV",
      value: formatOptionalDecimal(avgStress, 3),
    },
  ];
});
const realismLaneHistoryRows = computed(() =>
  selectedRealismLaneArtifacts.value.map((entry) => ({
    path: entry.path,
    ts: entry.ts,
    datasetId: entry.optionsDatasetId,
    backend: describeBackendRoute(
      entry.requestBackendPreference,
      entry.backendUsed,
      entry.backendFallbackReason,
    ).label,
    validation: describeArtifactValidation(entry)?.label ?? "Search only",
    selection: entry.bestSelectionExpectancyPerPremium,
    validated: strongestArtifactValidationBaseExpectancy(entry),
    stress: strongestArtifactValidationStressExpectancy(entry),
  })),
);

const datasetSummary = computed(() => {
  const selected = selectedDataset.value;
  if (!selected) return "";
  return [
    selected.rootSymbol || "unknown root",
    `${selected.contractCount} contracts`,
    `${selected.rowCount} rows`,
    selected.priceSemantics || "unknown semantics",
  ].join(" | ");
});

const datasetProvenanceSummary = computed(() => {
  const selected = selectedDataset.value;
  if (!selected) return "";
  const segments = [
    normalizeText(selected.providerName),
    normalizeText(selected.providerSourceSurface),
    normalizeText(selected.quoteResolution),
    normalizeText(selected.aggregatePriceField),
    normalizeText(selected.contractReferenceAsOfUtc),
  ].filter((entry) => entry.length > 0);
  return segments.join(" | ");
});

const selectedDatasetDetailRows = computed(() => {
  const selected = selectedDataset.value;
  if (!selected) return [];
  return [
    {
      key: "coverage",
      label: "Coverage",
      value: `${formatTimestamp(selected.coverageStart)} → ${formatTimestamp(selected.coverageEnd)}`,
    },
    {
      key: "provider",
      label: "Provider",
      value: `${normalizeText(selected.providerName)} · ${normalizeText(selected.providerSourceSurface)}`,
    },
    {
      key: "resolution",
      label: "Resolution",
      value: `${normalizeText(selected.quoteResolution)} · ${normalizeText(selected.priceSemantics)} · ${normalizeText(selected.aggregatePriceField) || "n/a"}`,
    },
    {
      key: "contracts",
      label: "Contracts / Rows",
      value: `${selected.contractCount} / ${selected.rowCount}`,
    },
    {
      key: "calendar",
      label: "Calendar / TZ",
      value: `${normalizeText(selected.calendarId)} · ${normalizeText(selected.timezone)}`,
    },
    {
      key: "contract-ref",
      label: "Contract ref",
      value: `${normalizeText(selected.contractReferenceSource)} · ${normalizeText(selected.contractReferenceAsOfUtc) || "n/a"}`,
    },
    {
      key: "underlier-ref",
      label: "Underlier ref",
      value: normalizeText(selected.underlierReferenceSource) || "—",
    },
    {
      key: "entitlement",
      label: "Entitlement",
      value: normalizeText(selected.providerEntitlementProfile) || "—",
    },
    {
      key: "dataset-path",
      label: "Dataset path",
      value: normalizeText(selected.datasetPath) || "—",
    },
    ...(selected.normalizedRowsSha256
      ? [
          {
            key: "rows-sha256",
            label: "Rows SHA-256",
            value: selected.normalizedRowsSha256,
          },
        ]
      : []),
    ...(selected.manifestSemanticSha256
      ? [
          {
            key: "manifest-sha256",
            label: "Manifest SHA-256",
            value: selected.manifestSemanticSha256,
          },
        ]
      : []),
  ];
});

const latestDownloadAuditRows = computed(() => {
  const latestDownload = latestDatasetDownload.value;
  if (!latestDownload) return [];
  return [
    {
      key: "download-contracts",
      label: "Contracts discovered / selected / rows",
      value: `${latestDownload.contractsDiscovered} / ${latestDownload.contractsSelected} / ${latestDownload.contractsWithRows}`,
    },
    {
      key: "download-rows",
      label: "Rows / skipped",
      value: `${latestDownload.downloadedOptionRows} / ${latestDownload.skippedContracts}`,
    },
    {
      key: "underlier-csv",
      label: "Underlying CSV",
      value: normalizeText(latestDownload.underlyingCsvPath) || "—",
    },
    {
      key: "underlier-cache",
      label: "Underlying source",
      value: latestDownload.usedUnderlyingCacheOnly
        ? "cache-only reuse"
        : "download + cache reuse",
    },
  ];
});

const activeDatasetManifest = computed(() => {
  const requestDatasetId = normalizeText(activeRequest.value?.optionsDatasetId);
  if (requestDatasetId) {
    return (
      datasets.value.find(
        (entry) => normalizeText(entry.optionsDatasetId) === requestDatasetId,
      ) || null
    );
  }
  const selectedId = normalizeText(selectedDataset.value?.optionsDatasetId);
  if (!selectedId) return selectedDataset.value;
  return (
    datasets.value.find(
      (entry) => normalizeText(entry.optionsDatasetId) === selectedId,
    ) || selectedDataset.value
  );
});

const activeDatasetProvenanceSummary = computed(() => {
  const explicitProvenance = activeDatasetProvenance.value;
  if (explicitProvenance) {
    return [
      normalizeText(explicitProvenance.providerName),
      normalizeText(explicitProvenance.providerSourceSurface),
      normalizeText(explicitProvenance.quoteResolution),
      normalizeText(explicitProvenance.aggregatePriceField),
      normalizeText(explicitProvenance.contractReferenceAsOfUtc),
    ]
      .filter((entry) => entry.length > 0)
      .join(" | ");
  }
  const activeDataset = activeDatasetManifest.value;
  const requestDatasetId = normalizeText(activeRequest.value?.optionsDatasetId);
  if (!activeDataset) {
    return requestDatasetId ? "manifest unavailable locally" : "";
  }
  return [
    normalizeText(activeDataset.providerName),
    normalizeText(activeDataset.providerSourceSurface),
    normalizeText(activeDataset.quoteResolution),
    normalizeText(activeDataset.aggregatePriceField),
    normalizeText(activeDataset.contractReferenceAsOfUtc),
  ]
    .filter((entry) => entry.length > 0)
    .join(" | ");
});
const activeDatasetQuoteQuality = computed(
  (): OptionsExpressionDatasetQuoteQualitySummary | null =>
    activeDatasetProvenance.value?.quoteQuality ?? null,
);
const activeDatasetQuoteQualityWarnings = computed(() =>
  activeDatasetQuoteQuality.value?.warnings.length
    ? activeDatasetQuoteQuality.value.warnings.join(" ")
    : "",
);
const selectedDatasetPriceSemantics = computed(
  () =>
    normalizeText(selectedDataset.value?.priceSemantics) ||
    normalizeText(realismLanePriceSemantics.value) ||
    "minute_aggregate",
);
const fillModelGuidance = computed(() =>
  fillModelGuidanceForPriceSemantics(selectedDatasetPriceSemantics.value),
);
const fillModelOptionRows = computed(() =>
  OPTIONS_FILL_MODEL_OPTIONS.map((entry) => {
    const disabled = !isFillModelCompatibleWithPriceSemantics(
      entry,
      selectedDatasetPriceSemantics.value,
    );
    return {
      value: entry,
      disabled,
      label:
        entry ===
        recommendedFillModelForPriceSemantics(
          selectedDatasetPriceSemantics.value,
        )
          ? `${entry} (recommended)`
          : entry,
    };
  }),
);

const rootMismatch = computed(() => {
  const selected = selectedDataset.value;
  if (!selected) return false;
  const selectedRoot = normalizeText(selected.rootSymbol).toUpperCase();
  const parentRoot = normalizeText(parentRootHint.value).toUpperCase();
  return !!selectedRoot && !!parentRoot && selectedRoot !== parentRoot;
});

const activeRows = computed(() => activeRankedResults.value);
const activeMetricAvailability = computed(() => {
  const availability = {
    hasHoldoutMetrics: false,
    hasSealedMetrics: false,
    hasStressMetrics: false,
    hasSealedStressMetrics: false,
    hasHarsherStressMetrics: false,
    hasSealedHarsherStressMetrics: false,
    hasExtremeStressMetrics: false,
    hasSealedExtremeStressMetrics: false,
  };
  for (const row of activeRows.value) {
    availability.hasHoldoutMetrics ||= row.holdoutMetrics != null;
    availability.hasSealedMetrics ||= row.sealedEvaluationMetrics != null;
    availability.hasStressMetrics ||= row.stressMetrics != null;
    availability.hasSealedStressMetrics ||= row.sealedStressMetrics != null;
    availability.hasHarsherStressMetrics ||= row.harsherStressMetrics != null;
    availability.hasSealedHarsherStressMetrics ||=
      row.sealedHarsherStressMetrics != null;
    availability.hasExtremeStressMetrics ||= row.extremeStressMetrics != null;
    availability.hasSealedExtremeStressMetrics ||=
      row.sealedExtremeStressMetrics != null;
    if (
      availability.hasHoldoutMetrics &&
      availability.hasSealedMetrics &&
      availability.hasStressMetrics &&
      availability.hasSealedStressMetrics &&
      availability.hasHarsherStressMetrics &&
      availability.hasSealedHarsherStressMetrics &&
      availability.hasExtremeStressMetrics &&
      availability.hasSealedExtremeStressMetrics
    ) {
      break;
    }
  }
  return availability;
});
const hasHoldoutMetrics = computed(
  () => activeMetricAvailability.value.hasHoldoutMetrics,
);
const hasSealedMetrics = computed(
  () => activeMetricAvailability.value.hasSealedMetrics,
);
const hasStressMetrics = computed(
  () => activeMetricAvailability.value.hasStressMetrics,
);
const hasSealedStressMetrics = computed(
  () => activeMetricAvailability.value.hasSealedStressMetrics,
);
const hasHarsherStressMetrics = computed(
  () => activeMetricAvailability.value.hasHarsherStressMetrics,
);
const hasSealedHarsherStressMetrics = computed(
  () => activeMetricAvailability.value.hasSealedHarsherStressMetrics,
);
const hasExtremeStressMetrics = computed(
  () => activeMetricAvailability.value.hasExtremeStressMetrics,
);
const hasSealedExtremeStressMetrics = computed(
  () => activeMetricAvailability.value.hasSealedExtremeStressMetrics,
);
const selectedTupleIndex = ref(0);
const inspectionSplitFilter = ref<"all" | "search" | "holdout" | "sealed_test">(
  "all",
);
const showAllInspectionRows = ref(false);
const selectedInspectionEventId = ref("");

watch(
  () => selectedDataset.value?.priceSemantics,
  (priceSemantics, previous) => {
    if (!normalizeText(priceSemantics)) return;
    if (normalizeText(priceSemantics) !== normalizeText(previous)) {
      store.ensureCompatibleFillModel(true);
    } else {
      store.ensureCompatibleFillModel();
    }
  },
  { immediate: true },
);

watch(
  activeRows,
  (rows) => {
    if (!rows.length) {
      selectedTupleIndex.value = 0;
      store.clearTupleInspection();
      return;
    }
    if (selectedTupleIndex.value >= rows.length) {
      selectedTupleIndex.value = 0;
    }
  },
  { immediate: true },
);

watch(selectedTupleIndex, () => {
  showAllInspectionRows.value = false;
  inspectionSplitFilter.value = "all";
  selectedInspectionEventId.value = "";
  store.clearTupleInspection();
});

const selectedTuple = computed(
  () => activeRows.value[selectedTupleIndex.value] || null,
);
const selectedInspection = computed(() => {
  const inspection = activeTupleInspection.value;
  if (!inspection) return null;
  return inspection.tupleIndex === selectedTupleIndex.value ? inspection : null;
});
const inspectionRows = computed(() => {
  const inspection = selectedInspection.value;
  if (!inspection) return [];
  const filter = inspectionSplitFilter.value;
  return inspection.rows.filter(
    (row) => filter === "all" || row.split === filter,
  );
});
const inspectionSplitStatusRows = computed(() => {
  const inspection = selectedInspection.value;
  if (!inspection) return [];
  const buckets = new Map<
    string,
    {
      key: string;
      split: string;
      status: string;
      count: number;
      avgReturn: number | null;
      totalPnl: number | null;
    }
  >();
  for (const row of inspection.rows) {
    const key = `${row.split}|${row.status}`;
    const existing = buckets.get(key) || {
      key,
      split: row.split,
      status: row.status,
      count: 0,
      avgReturn: null,
      totalPnl: null,
    };
    existing.count += 1;
    existing.avgReturn =
      row.returnOnPremium == null
        ? existing.avgReturn
        : ((existing.avgReturn ?? 0) * (existing.count - 1) +
            row.returnOnPremium) /
          existing.count;
    existing.totalPnl =
      row.pnlPerContract == null
        ? existing.totalPnl
        : (existing.totalPnl ?? 0) + row.pnlPerContract;
    buckets.set(key, existing);
  }
  return [...buckets.values()].sort((left, right) => {
    const splitSort = normalizeText(left.split).localeCompare(
      normalizeText(right.split),
    );
    if (splitSort !== 0) return splitSort;
    return normalizeText(left.status).localeCompare(
      normalizeText(right.status),
    );
  });
});
const inspectionInvalidReasonRows = computed(() => {
  const inspection = selectedInspection.value;
  if (!inspection) return [];
  const buckets = new Map<
    string,
    { key: string; split: string; reason: string; count: number }
  >();
  for (const row of inspection.rows) {
    if (!row.candidateInvalidReason) continue;
    const key = `${row.split}|${row.candidateInvalidReason}`;
    const existing = buckets.get(key) || {
      key,
      split: row.split,
      reason: row.candidateInvalidReason,
      count: 0,
    };
    existing.count += 1;
    buckets.set(key, existing);
  }
  return [...buckets.values()].sort((left, right) => right.count - left.count);
});
const displayedInspectionRows = computed(() => {
  if (showAllInspectionRows.value) return inspectionRows.value;
  return inspectionRows.value.slice(0, 50);
});
const selectedInspectionRow = computed(() => {
  const rows = inspectionRows.value;
  if (!rows.length) return null;
  const match = rows.find(
    (row) => row.eventId === selectedInspectionEventId.value,
  );
  return match || rows[0] || null;
});
const selectedArtifactSummary = computed(() => {
  const nextPath = normalizeText(selectedArtifactPath.value);
  return (
    filteredArtifacts.value.find(
      (entry) => normalizeText(entry.path) === nextPath,
    ) || null
  );
});
const selectedArtifactValidation = computed(() =>
  describeArtifactValidation(selectedArtifactSummary.value),
);
const selectedArtifactRealismLane = computed(() =>
  describeRealismLane(
    selectedArtifactSummary.value?.priceSemantics,
    selectedArtifactSummary.value?.requestFillModel,
  ),
);
const selectedArtifactBackendRoute = computed(() =>
  describeBackendRoute(
    selectedArtifactSummary.value?.requestBackendPreference,
    selectedArtifactSummary.value?.backendUsed,
    selectedArtifactSummary.value?.backendFallbackReason,
  ),
);
const selectedArtifactValidationFrontierRows = computed(() => {
  const artifact = selectedArtifactSummary.value;
  if (!artifact) return [];
  const selection = artifact.bestSelectionExpectancyPerPremium;
  return [
    {
      key: "selection",
      label: "Selection",
      expectancy: selection,
      delta: 0,
      status: "baseline",
    },
    artifact.bestHoldoutExpectancyPerPremium != null
      ? {
          key: "holdout",
          label: "Holdout",
          expectancy: artifact.bestHoldoutExpectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selection,
              artifact.bestHoldoutExpectancyPerPremium,
            ) ?? 0,
          status: "validation",
        }
      : null,
    artifact.bestSealedExpectancyPerPremium != null
      ? {
          key: "sealed",
          label: "Sealed finalist",
          expectancy: artifact.bestSealedExpectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selection,
              artifact.bestSealedExpectancyPerPremium,
            ) ?? 0,
          status: "validation",
        }
      : null,
    artifact.bestStressExpectancyPerPremium != null
      ? {
          key: "stress",
          label: "Stress",
          expectancy: artifact.bestStressExpectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selection,
              artifact.bestStressExpectancyPerPremium,
            ) ?? 0,
          status: "stress",
        }
      : null,
    artifact.bestSealedStressExpectancyPerPremium != null
      ? {
          key: "sealed-stress",
          label: "Sealed stress",
          expectancy: artifact.bestSealedStressExpectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selection,
              artifact.bestSealedStressExpectancyPerPremium,
            ) ?? 0,
          status: "stress",
        }
      : null,
    artifact.bestHarsherStressExpectancyPerPremium != null
      ? {
          key: "harsher",
          label: "Harsher",
          expectancy: artifact.bestHarsherStressExpectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selection,
              artifact.bestHarsherStressExpectancyPerPremium,
            ) ?? 0,
          status: "stress",
        }
      : null,
    artifact.bestSealedHarsherStressExpectancyPerPremium != null
      ? {
          key: "sealed-harsher",
          label: "Sealed harsher",
          expectancy: artifact.bestSealedHarsherStressExpectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selection,
              artifact.bestSealedHarsherStressExpectancyPerPremium,
            ) ?? 0,
          status: "stress",
        }
      : null,
    artifact.bestExtremeStressExpectancyPerPremium != null
      ? {
          key: "extreme",
          label: "Extreme",
          expectancy: artifact.bestExtremeStressExpectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selection,
              artifact.bestExtremeStressExpectancyPerPremium,
            ) ?? 0,
          status: "stress",
        }
      : null,
    artifact.bestSealedExtremeStressExpectancyPerPremium != null
      ? {
          key: "sealed-extreme",
          label: "Sealed extreme",
          expectancy: artifact.bestSealedExtremeStressExpectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selection,
              artifact.bestSealedExtremeStressExpectancyPerPremium,
            ) ?? 0,
          status: "stress",
        }
      : null,
  ].filter(
    (
      entry,
    ): entry is {
      key: string;
      label: string;
      expectancy: number;
      delta: number;
      status: string;
    } => Boolean(entry),
  );
});
const selectedArtifactDerivedMetrics = computed(() => {
  const artifact = selectedArtifactSummary.value;
  if (!artifact) return [];
  return [
    {
      key: "artifact-validation-delta",
      label: "Validation delta",
      value: formatOptionalDecimal(artifactValidationDelta(artifact), 3),
    },
    {
      key: "artifact-stress-drop",
      label: "Stress drop",
      value: formatOptionalDecimal(artifactStressDrop(artifact), 3),
    },
    {
      key: "artifact-support-ratio",
      label: "Validated ratio",
      value: formatPercent(artifactValidationSupportRatio(artifact), 1),
    },
    {
      key: "artifact-fragility-ratio",
      label: "Fragility ratio",
      value: formatPercent(artifactStressFragilityRatio(artifact), 1),
    },
  ];
});
const selectedArtifactSupportRows = computed(() => {
  const artifact = selectedArtifactSummary.value;
  if (!artifact) return [];
  return [
    {
      key: "artifact-holdout-support",
      label: "Holdout support",
      value: formatArtifactSupportCount(
        artifact.supportedHoldoutTupleCount,
        artifact.tupleCount,
      ),
    },
    {
      key: "artifact-sealed-support",
      label: "Sealed support",
      value: formatArtifactSupportCount(
        artifact.supportedSealedTupleCount,
        artifact.tupleCount,
      ),
    },
    {
      key: "artifact-stress-support",
      label: "Stress support",
      value: formatArtifactSupportCount(
        artifact.supportedStressTupleCount,
        artifact.tupleCount,
      ),
    },
  ];
});
const selectedArtifactLeadershipRows = computed(() => {
  const artifact = selectedArtifactSummary.value;
  if (!artifact || !hasArtifactLeaderSummaryData(artifact)) return [];
  return [
    {
      key: "artifact-leadership-label",
      label: "Leadership",
      value: artifactLeadershipLabel(artifact),
    },
    {
      key: "artifact-leadership-map",
      label: "Retained leaders",
      value: artifactLeadershipMap(artifact),
    },
    {
      key: "artifact-leadership-disagreement",
      label: "Summary disagreement",
      value: artifactLeadershipDisagreement(artifact),
    },
    {
      key: "artifact-robust-leader-finalist",
      label: "Robust leader",
      value: artifactRobustLeaderFinalistDetail(artifact),
    },
  ];
});
const selectedArtifactCoverageAdequacy = computed(() => {
  const artifact = selectedArtifactSummary.value;
  if (!artifact) return null;
  return describeCoverageAdequacy({
    verdict: artifact.coverageAdequacyVerdict,
    nonPromotable: artifact.coverageNonPromotable,
    searchOverlapRate: artifact.searchOverlapRate,
    holdoutOverlapRate: artifact.holdoutOverlapRate,
    sealedTestOverlapRate: artifact.sealedTestOverlapRate,
    searchCandidateCoverageRate: artifact.searchCandidateCoverageRate,
    holdoutCandidateCoverageRate: artifact.holdoutCandidateCoverageRate,
    sealedTestCandidateCoverageRate: artifact.sealedTestCandidateCoverageRate,
    searchBaselineOutcomeCoverageRate:
      artifact.searchBaselineOutcomeCoverageRate,
    holdoutBaselineOutcomeCoverageRate:
      artifact.holdoutBaselineOutcomeCoverageRate,
    sealedTestBaselineOutcomeCoverageRate:
      artifact.sealedTestBaselineOutcomeCoverageRate,
  });
});
const selectedArtifactCoverageRows = computed(() => {
  const artifact = selectedArtifactSummary.value;
  if (!artifact) return [];
  return [
    {
      key: "artifact-adequacy",
      label: "Coverage adequacy",
      value: formatCoverageAdequacyLabel(artifact.coverageAdequacyVerdict),
    },
    {
      key: "artifact-non-promotable",
      label: "Non-promotable",
      value: artifact.coverageNonPromotable ? "yes" : "no",
    },
    {
      key: "artifact-overlap",
      label: "Search / Holdout / Sealed overlap",
      value: `${formatPercent(artifact.searchOverlapRate, 1)} / ${formatPercent(artifact.holdoutOverlapRate, 1)} / ${formatPercent(artifact.sealedTestOverlapRate, 1)}`,
    },
    {
      key: "artifact-candidate-coverage",
      label: "Search / Holdout / Sealed candidates",
      value: `${formatPercent(artifact.searchCandidateCoverageRate, 1)} / ${formatPercent(artifact.holdoutCandidateCoverageRate, 1)} / ${formatPercent(artifact.sealedTestCandidateCoverageRate, 1)}`,
    },
    {
      key: "artifact-outcome-coverage",
      label: "Search / Holdout / Sealed outcomes",
      value: `${formatPercent(artifact.searchBaselineOutcomeCoverageRate, 1)} / ${formatPercent(artifact.holdoutBaselineOutcomeCoverageRate, 1)} / ${formatPercent(artifact.sealedTestBaselineOutcomeCoverageRate, 1)}`,
    },
  ];
});

const artifactComparisonRows = computed(() =>
  [...filteredArtifacts.value].sort((left, right) => {
    const byRobust = (right.bestRobustScore ?? 0) - (left.bestRobustScore ?? 0);
    if (Math.abs(byRobust) > 1e-9) return byRobust;
    return normalizeText(right.ts).localeCompare(normalizeText(left.ts));
  }),
);
const artifactFrontierLeaderRows = computed(() => {
  const rows = filteredArtifacts.value;
  const bestEntryFor = (
    metric: (
      entry: OptionsExpressionArtifactSummary,
    ) => number | null | undefined,
  ): { entry: OptionsExpressionArtifactSummary; value: number } | null => {
    let best: {
      entry: OptionsExpressionArtifactSummary;
      value: number;
    } | null = null;
    for (const entry of rows) {
      const raw = Number(metric(entry));
      if (!Number.isFinite(raw)) continue;
      if (!best || raw > best.value) {
        best = { entry, value: raw };
      }
    }
    return best;
  };
  return [
    {
      key: "robust",
      label: "Best robust artifact",
      metricLabel: "Robust",
      best: bestEntryFor((entry) => entry.bestRobustScore),
    },
    {
      key: "selection",
      label: "Best selection artifact",
      metricLabel: "Sel EV",
      best: bestEntryFor((entry) => entry.bestSelectionExpectancyPerPremium),
    },
    {
      key: "validated",
      label: "Best validated artifact",
      metricLabel: "Val EV",
      best: bestEntryFor((entry) =>
        strongestArtifactValidationBaseExpectancy(entry),
      ),
    },
    {
      key: "stress",
      label: "Best stressed artifact",
      metricLabel: "Stress EV",
      best: bestEntryFor((entry) =>
        strongestArtifactValidationStressExpectancy(entry),
      ),
    },
  ]
    .filter((row) => row.best != null)
    .map((row) => ({
      key: row.key,
      label: row.label,
      metricLabel: row.metricLabel,
      value: row.best!.value,
      entry: row.best!.entry,
      validation:
        describeArtifactValidation(row.best!.entry)?.label ?? "Search only",
      drift: artifactValidationDelta(row.best!.entry),
      stressDrop: artifactStressDrop(row.best!.entry),
      artifactLabel: formatArtifactOptionLabel(row.best!.entry),
    }));
});
const artifactMetaEntries = computed(() =>
  [...artifacts.value].sort((left, right) =>
    normalizeText(right.ts).localeCompare(normalizeText(left.ts)),
  ),
);
const artifactMetaOverviewRows = computed<ArtifactMetaOverviewRow[]>(() => {
  const rows = artifactMetaEntries.value;
  const uniqueRoots = new Set(
    rows.map((entry) => formatMetaGroupKey(entry.rootSymbol, "Unknown root")),
  );
  const uniqueParents = new Set(
    rows.map((entry) =>
      formatMetaGroupKey(entry.parentStrategyKind, "Unknown parent family"),
    ),
  );
  const uniqueStructures = new Set(
    rows.map((entry) =>
      formatMetaGroupKey(entry.requestStructureFamily, "Unknown structure"),
    ),
  );
  const uniqueDatasets = new Set(
    rows.map((entry) =>
      formatMetaGroupKey(entry.optionsDatasetId, "Unknown dataset"),
    ),
  );
  const validatedCount = rows.filter(
    (entry) => artifactValidationVerdict(entry) !== "search_only",
  ).length;
  const fragileCount = rows.filter(
    (entry) => artifactValidationVerdict(entry) === "stress_fragile",
  ).length;
  return [
    { key: "artifacts", label: "Saved artifacts", value: String(rows.length) },
    { key: "roots", label: "Roots", value: String(uniqueRoots.size) },
    {
      key: "parents",
      label: "Parent families",
      value: String(uniqueParents.size),
    },
    {
      key: "structures",
      label: "Structure families",
      value: String(uniqueStructures.size),
    },
    { key: "datasets", label: "Datasets", value: String(uniqueDatasets.size) },
    {
      key: "validated",
      label: "Validated artifacts",
      value:
        rows.length > 0
          ? `${validatedCount}/${rows.length} (${formatPercent(validatedCount / rows.length)})`
          : "0",
    },
    {
      key: "fragile",
      label: "Stress-fragile artifacts",
      value:
        rows.length > 0
          ? `${fragileCount}/${rows.length} (${formatPercent(fragileCount / rows.length)})`
          : "0",
    },
  ];
});
const artifactMetaByRootRows = computed(() =>
  buildArtifactMetaGroupRows(artifactMetaEntries.value, (entry) =>
    formatMetaGroupKey(entry.rootSymbol, "Unknown root"),
  ),
);
const artifactMetaByParentRows = computed(() =>
  buildArtifactMetaGroupRows(artifactMetaEntries.value, (entry) =>
    formatMetaGroupKey(entry.parentStrategyKind, "Unknown parent family"),
  ),
);
const artifactMetaByStructureRows = computed(() =>
  buildArtifactMetaGroupRows(artifactMetaEntries.value, (entry) =>
    formatMetaGroupKey(entry.requestStructureFamily, "Unknown structure"),
  ),
);
const artifactMetaByLaneRows = computed(() =>
  buildArtifactMetaGroupRows(artifactMetaEntries.value, (entry) => {
    const lane = describeRealismLane(
      entry.priceSemantics,
      entry.requestFillModel,
    );
    return `${lane.shortLabel} · ${entry.requestFillModel || "aggregate_research"}`;
  }),
);
const artifactMetaByVerdictRows = computed(() =>
  buildArtifactMetaGroupRows(artifactMetaEntries.value, (entry) =>
    artifactValidationLabel(entry),
  ),
);
const artifactLaneDisagreementRows = computed(() =>
  buildArtifactLaneDisagreementRows(artifactMetaEntries.value),
);
const artifactMetaSections = computed(() => [
  { key: "root", label: "By root", rows: artifactMetaByRootRows.value },
  {
    key: "parent",
    label: "By parent family",
    rows: artifactMetaByParentRows.value,
  },
  {
    key: "structure",
    label: "By structure family",
    rows: artifactMetaByStructureRows.value,
  },
  { key: "lane", label: "By realism lane", rows: artifactMetaByLaneRows.value },
  {
    key: "verdict",
    label: "By top validation verdict",
    rows: artifactMetaByVerdictRows.value,
  },
]);
const resultSortKey = ref<
  | "robust"
  | "selection_ev"
  | "holdout_ev"
  | "stress_ev"
  | "harsher_ev"
  | "extreme_ev"
  | "invalid_rate"
  | "valid_trades"
  | "sealed_rank"
>("robust");
const resultSortDirection = ref<"desc" | "asc">("desc");
const finalistsOnly = ref(false);
const maxInvalidRateFilterInput = ref("");
const minRobustScoreFilterInput = ref("");
const maxStressDropFilterInput = ref("");
const selectedEventPathInspection = computed(() => {
  const inspection = activeEventPathInspection.value;
  const selectedRow = selectedInspectionRow.value;
  if (!inspection || !selectedRow) return null;
  if (inspection.tupleIndex !== selectedTupleIndex.value) return null;
  return inspection.eventIndex === selectedRow.eventIndex ? inspection : null;
});
const selectedTupleValidation = computed(() =>
  describeTupleValidation(selectedTuple.value),
);
const selectedTupleSupportRows = computed(() => {
  const tuple = selectedTuple.value;
  if (!tuple) return [];
  return [
    {
      key: "tuple-holdout-support",
      label: "Holdout support",
      value: formatMetricsSupport(tuple.holdoutMetrics),
    },
    {
      key: "tuple-sealed-support",
      label: "Sealed support",
      value: formatMetricsSupport(tuple.sealedEvaluationMetrics),
    },
    {
      key: "tuple-stress-support",
      label: "Stress support",
      value: formatMetricsSupport(strongestTupleValidationStressMetrics(tuple)),
    },
  ];
});
const selectedTupleValidationScopeRows = computed(() => {
  const tuple = selectedTuple.value;
  if (!tuple) return [];
  const hasRetainedScope = Boolean(
    supportedMetrics(tuple.holdoutMetrics) ||
    supportedMetrics(tuple.stressMetrics) ||
    supportedMetrics(tuple.harsherStressMetrics) ||
    supportedMetrics(tuple.extremeStressMetrics),
  );
  const hasSealedScope = Boolean(
    supportedMetrics(tuple.sealedEvaluationMetrics) ||
    supportedMetrics(tuple.sealedStressMetrics) ||
    supportedMetrics(tuple.sealedHarsherStressMetrics) ||
    supportedMetrics(tuple.sealedExtremeStressMetrics),
  );
  return [
    hasRetainedScope
      ? {
          key: "retained-scope",
          label: "Retained frontier scope",
          value: RETAINED_SCOPE_LABEL,
        }
      : null,
    hasSealedScope
      ? {
          key: "sealed-scope",
          label: "Sealed frontier scope",
          value: SEALED_SCOPE_LABEL,
        }
      : null,
  ].filter((entry): entry is { key: string; label: string; value: string } =>
    Boolean(entry),
  );
});
const activeRealismLane = computed(() =>
  describeRealismLane(
    activePriceSemantics.value,
    activeRequest.value?.fillModel,
  ),
);
const activeBackendRoute = computed(() =>
  describeBackendRoute(
    activeRequest.value?.backendPreference,
    activeBackendUsed.value,
    activeBackendFallbackReason.value,
  ),
);
const selectedTupleValidationFrontierRows = computed(() => {
  const tuple = selectedTuple.value;
  if (!tuple) return [];
  const selectionExpectancy = tuple.selectionMetrics.expectancyPerPremium;
  const rows = [
    {
      key: "selection",
      label: "Selection",
      expectancy: tuple.selectionMetrics.expectancyPerPremium,
      delta: 0,
      status: "baseline",
      support: formatMetricsSupport(tuple.selectionMetrics),
    },
    tuple.holdoutMetrics
      ? {
          key: "holdout",
          label: "Holdout",
          expectancy: tuple.holdoutMetrics.expectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selectionExpectancy,
              tuple.holdoutMetrics.expectancyPerPremium,
            ) ?? 0,
          status: "validation",
          support: formatMetricsSupport(tuple.holdoutMetrics),
        }
      : null,
    tuple.sealedEvaluationMetrics
      ? {
          key: "sealed",
          label: "Sealed finalist",
          expectancy: tuple.sealedEvaluationMetrics.expectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selectionExpectancy,
              tuple.sealedEvaluationMetrics.expectancyPerPremium,
            ) ?? 0,
          status: "validation",
          support: formatMetricsSupport(tuple.sealedEvaluationMetrics),
        }
      : null,
    tuple.stressMetrics
      ? {
          key: "stress",
          label: "Stress",
          expectancy: tuple.stressMetrics.expectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selectionExpectancy,
              tuple.stressMetrics.expectancyPerPremium,
            ) ?? 0,
          status: "stress",
          support: formatMetricsSupport(tuple.stressMetrics),
        }
      : null,
    tuple.sealedStressMetrics
      ? {
          key: "sealed-stress",
          label: "Sealed stress",
          expectancy: tuple.sealedStressMetrics.expectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selectionExpectancy,
              tuple.sealedStressMetrics.expectancyPerPremium,
            ) ?? 0,
          status: "stress",
          support: formatMetricsSupport(tuple.sealedStressMetrics),
        }
      : null,
    tuple.harsherStressMetrics
      ? {
          key: "harsher",
          label: "Harsher",
          expectancy: tuple.harsherStressMetrics.expectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selectionExpectancy,
              tuple.harsherStressMetrics.expectancyPerPremium,
            ) ?? 0,
          status: "stress",
          support: formatMetricsSupport(tuple.harsherStressMetrics),
        }
      : null,
    tuple.sealedHarsherStressMetrics
      ? {
          key: "sealed-harsher",
          label: "Sealed harsher",
          expectancy: tuple.sealedHarsherStressMetrics.expectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selectionExpectancy,
              tuple.sealedHarsherStressMetrics.expectancyPerPremium,
            ) ?? 0,
          status: "stress",
          support: formatMetricsSupport(tuple.sealedHarsherStressMetrics),
        }
      : null,
    tuple.extremeStressMetrics
      ? {
          key: "extreme",
          label: "Extreme",
          expectancy: tuple.extremeStressMetrics.expectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selectionExpectancy,
              tuple.extremeStressMetrics.expectancyPerPremium,
            ) ?? 0,
          status: "stress",
          support: formatMetricsSupport(tuple.extremeStressMetrics),
        }
      : null,
    tuple.sealedExtremeStressMetrics
      ? {
          key: "sealed-extreme",
          label: "Sealed extreme",
          expectancy: tuple.sealedExtremeStressMetrics.expectancyPerPremium,
          delta:
            metricDeltaAgainstSelection(
              selectionExpectancy,
              tuple.sealedExtremeStressMetrics.expectancyPerPremium,
            ) ?? 0,
          status: "stress",
          support: formatMetricsSupport(tuple.sealedExtremeStressMetrics),
        }
      : null,
  ].filter(
    (
      entry,
    ): entry is {
      key: string;
      label: string;
      expectancy: number;
      delta: number;
      status: string;
      support: string;
    } => Boolean(entry),
  );
  return rows;
});
const selectedTupleValidationDriftRows = computed(() => {
  const tuple = selectedTuple.value;
  if (!tuple) return [];
  return [
    buildValidationDriftRow(
      "drift-sel-hold",
      "Selection -> Holdout",
      "Selection",
      metricsExpectancy(tuple.selectionMetrics, false),
      "Holdout",
      metricsExpectancy(tuple.holdoutMetrics),
      SELECTED_TUPLE_SCOPE_LABEL,
    ),
    buildValidationDriftRow(
      "drift-hold-sealed",
      "Holdout -> Sealed",
      "Holdout",
      metricsExpectancy(tuple.holdoutMetrics),
      "Sealed finalist",
      metricsExpectancy(tuple.sealedEvaluationMetrics),
      SELECTED_TUPLE_SCOPE_LABEL,
    ),
    buildValidationDriftRow(
      "drift-hold-stress",
      "Holdout -> Stress",
      "Holdout",
      metricsExpectancy(tuple.holdoutMetrics),
      "Stress",
      metricsExpectancy(tuple.stressMetrics),
      SELECTED_TUPLE_SCOPE_LABEL,
    ),
    buildValidationDriftRow(
      "drift-stress-harsher",
      "Stress -> Harsher",
      "Stress",
      metricsExpectancy(tuple.stressMetrics),
      "Harsher",
      metricsExpectancy(tuple.harsherStressMetrics),
      SELECTED_TUPLE_SCOPE_LABEL,
    ),
    buildValidationDriftRow(
      "drift-harsher-extreme",
      "Harsher -> Extreme",
      "Harsher",
      metricsExpectancy(tuple.harsherStressMetrics),
      "Extreme",
      metricsExpectancy(tuple.extremeStressMetrics),
      SELECTED_TUPLE_SCOPE_LABEL,
    ),
    buildValidationDriftRow(
      "drift-sealed-stress",
      "Sealed -> Sealed stress",
      "Sealed finalist",
      metricsExpectancy(tuple.sealedEvaluationMetrics),
      "Sealed stress",
      metricsExpectancy(tuple.sealedStressMetrics),
      SELECTED_TUPLE_SCOPE_LABEL,
    ),
    buildValidationDriftRow(
      "drift-sealed-stress-harsher",
      "Sealed stress -> Sealed harsher",
      "Sealed stress",
      metricsExpectancy(tuple.sealedStressMetrics),
      "Sealed harsher",
      metricsExpectancy(tuple.sealedHarsherStressMetrics),
      SELECTED_TUPLE_SCOPE_LABEL,
    ),
    buildValidationDriftRow(
      "drift-sealed-harsher-extreme",
      "Sealed harsher -> Sealed extreme",
      "Sealed harsher",
      metricsExpectancy(tuple.sealedHarsherStressMetrics),
      "Sealed extreme",
      metricsExpectancy(tuple.sealedExtremeStressMetrics),
      SELECTED_TUPLE_SCOPE_LABEL,
    ),
  ].filter((entry): entry is ValidationInterpretationValueRow =>
    Boolean(entry),
  );
});
const selectedTupleValidationMonotonicityRows = computed(() => {
  const tuple = selectedTuple.value;
  if (!tuple) return [];
  const holdoutPoints = buildValidationFamilyPoints(tuple, "holdout");
  const sealedPoints = buildValidationFamilyPoints(tuple, "sealed");
  return [
    holdoutPoints.length > 0
      ? (() => {
          const verdict = describeValidationMonotonicity(holdoutPoints);
          return {
            key: "monotonicity-holdout-family",
            label: "Holdout family",
            statusLabel: monotonicityStatusLabel(verdict.state),
            detail: verdict.detail,
            scopeLabel: SELECTED_TUPLE_SCOPE_LABEL,
          };
        })()
      : null,
    sealedPoints.length > 0
      ? (() => {
          const verdict = describeValidationMonotonicity(sealedPoints);
          return {
            key: "monotonicity-sealed-family",
            label: "Sealed family",
            statusLabel: monotonicityStatusLabel(verdict.state),
            detail: verdict.detail,
            scopeLabel: SELECTED_TUPLE_SCOPE_LABEL,
          };
        })()
      : null,
  ].filter((entry): entry is ValidationInterpretationStateRow =>
    Boolean(entry),
  );
});
const selectedTupleValidationRegretRows = computed(() => {
  const tuple = selectedTuple.value;
  const rows = activeRows.value;
  if (!tuple || rows.length === 0) return [];
  return [
    buildValidationRegretRow(
      "regret-holdout",
      "Retained holdout regret",
      RETAINED_SCOPE_LABEL,
      metricsExpectancy(tuple.holdoutMetrics),
      rows.map((entry) => metricsExpectancy(entry.holdoutMetrics)),
    ),
    buildValidationRegretRow(
      "regret-stress",
      "Retained stress regret",
      RETAINED_SCOPE_LABEL,
      metricsExpectancy(tuple.stressMetrics),
      rows.map((entry) => metricsExpectancy(entry.stressMetrics)),
    ),
    buildValidationRegretRow(
      "regret-sealed",
      "Sealed regret",
      SEALED_SCOPE_LABEL,
      metricsExpectancy(tuple.sealedEvaluationMetrics),
      rows
        .filter((entry) => entry.sealedFinalist)
        .map((entry) => metricsExpectancy(entry.sealedEvaluationMetrics)),
    ),
    buildValidationRegretRow(
      "regret-sealed-stress",
      "Sealed stress regret",
      SEALED_SCOPE_LABEL,
      metricsExpectancy(tuple.sealedStressMetrics),
      rows
        .filter((entry) => entry.sealedFinalist)
        .map((entry) => metricsExpectancy(entry.sealedStressMetrics)),
    ),
  ].filter((entry): entry is ValidationInterpretationValueRow =>
    Boolean(entry),
  );
});
const validationWinnersRows = computed(() => {
  const rows = activeRows.value;
  if (!rows.length) return [];
  return [
    {
      key: "robust",
      label: "Best robust score",
      winner: bestTupleForMetric(rows, (row) => row.robustScore),
      metricLabel: "Robust",
    },
    {
      key: "selection",
      label: "Best selection EV",
      winner: bestTupleForMetric(
        rows,
        (row) => row.selectionMetrics.expectancyPerPremium,
      ),
      metricLabel: "Sel EV",
    },
    {
      key: "holdout",
      label: "Best holdout EV",
      winner: bestTupleForMetric(
        rows,
        (row) => row.holdoutMetrics?.expectancyPerPremium,
      ),
      metricLabel: "Hold EV",
    },
    {
      key: "sealed",
      label: "Best sealed EV",
      winner: bestTupleForMetric(
        rows,
        (row) => row.sealedEvaluationMetrics?.expectancyPerPremium,
      ),
      metricLabel: "Sealed EV",
    },
    {
      key: "stress",
      label: "Best stress EV",
      winner: bestTupleForMetric(
        rows,
        (row) => row.stressMetrics?.expectancyPerPremium,
      ),
      metricLabel: "Stress EV",
    },
    {
      key: "harsher",
      label: "Best harsher EV",
      winner: bestTupleForMetric(
        rows,
        (row) => row.harsherStressMetrics?.expectancyPerPremium,
      ),
      metricLabel: "Harsher EV",
    },
    {
      key: "extreme",
      label: "Best extreme EV",
      winner: bestTupleForMetric(
        rows,
        (row) => row.extremeStressMetrics?.expectancyPerPremium,
      ),
      metricLabel: "Extreme EV",
    },
    {
      key: "sealed-extreme",
      label: "Best sealed extreme EV",
      winner: bestTupleForMetric(
        rows,
        (row) => row.sealedExtremeStressMetrics?.expectancyPerPremium,
      ),
      metricLabel: "Sealed extreme EV",
    },
  ]
    .filter((entry) => entry.winner != null)
    .map((entry) => {
      const winner = entry.winner!;
      return {
        key: entry.key,
        label: entry.label,
        metricLabel: entry.metricLabel,
        tupleIndex: winner.index,
        tuple: winner.row,
        value: winner.value,
        validation: describeTupleValidation(winner.row)?.label ?? "Search only",
        stressDrop: strongestTupleStressDrop(winner.row),
      };
    });
});
const winnerDisagreementRows = computed(() => {
  const byKey = new Map(
    validationWinnersRows.value.map((row) => [row.key, row]),
  );
  const rows = ["selection", "holdout", "sealed", "robust"]
    .map((key) => byKey.get(key))
    .filter((entry): entry is NonNullable<typeof entry> => Boolean(entry));
  if (!rows.length) return [];
  const searchWinner = rows.find((row) => row.key === "selection") ?? null;
  const robustWinner = rows.find((row) => row.key === "robust") ?? null;
  return rows.map((row) => {
    let relation = "Different tuple";
    if (row.key === "selection") {
      relation = "Search baseline";
    } else if (
      searchWinner &&
      robustWinner &&
      row.tupleIndex === searchWinner.tupleIndex &&
      row.tupleIndex === robustWinner.tupleIndex
    ) {
      relation = "Same as search & robust";
    } else if (searchWinner && row.tupleIndex === searchWinner.tupleIndex) {
      relation = "Same as search";
    } else if (robustWinner && row.tupleIndex === robustWinner.tupleIndex) {
      relation = "Same as robust";
    }
    return {
      ...row,
      tupleIdentity: formatTupleIdentity(row.tuple),
      relation,
    };
  });
});
const winnerDisagreementSummary =
  computed<WinnerDisagreementPresentationSummary | null>(() => {
    const rows = winnerDisagreementRows.value;
    if (!rows.length) return null;
    const searchWinner = rows.find((row) => row.key === "selection") ?? null;
    const holdoutWinner = rows.find((row) => row.key === "holdout") ?? null;
    const sealedWinner = rows.find((row) => row.key === "sealed") ?? null;
    const robustWinner = rows.find((row) => row.key === "robust") ?? null;
    if (!searchWinner) return null;
    if (!holdoutWinner && !sealedWinner && !robustWinner) {
      return {
        label: "Search-stage only",
        tone: "pending",
        detail: `Tuple #${searchWinner.tupleIndex + 1} is only the search winner so far. Holdout, sealed, and robust confirmation have not diverged because they do not exist yet.`,
      };
    }
    const distinctTupleCount = new Set(rows.map((row) => row.tupleIndex)).size;
    if (distinctTupleCount <= 1) {
      return {
        label: "Aligned winner",
        tone: "pass",
        detail: `Tuple #${searchWinner.tupleIndex + 1} remains the search winner and still leads every currently available validation / robustness lane.`,
      };
    }

    const driftParts: string[] = [];
    if (holdoutWinner && holdoutWinner.tupleIndex !== searchWinner.tupleIndex) {
      driftParts.push(`holdout winner #${holdoutWinner.tupleIndex + 1}`);
    }
    if (
      sealedWinner &&
      sealedWinner.tupleIndex !== searchWinner.tupleIndex &&
      (!holdoutWinner || sealedWinner.tupleIndex !== holdoutWinner.tupleIndex)
    ) {
      driftParts.push(`sealed winner #${sealedWinner.tupleIndex + 1}`);
    }
    if (
      robustWinner &&
      robustWinner.tupleIndex !== searchWinner.tupleIndex &&
      (!sealedWinner || robustWinner.tupleIndex !== sealedWinner.tupleIndex) &&
      (!holdoutWinner || robustWinner.tupleIndex !== holdoutWinner.tupleIndex)
    ) {
      driftParts.push(`robust winner #${robustWinner.tupleIndex + 1}`);
    }
    const detailParts =
      driftParts.length > 0
        ? driftParts.join(", ")
        : `another validation lane favors tuple #${rows.find((row) => row.tupleIndex !== searchWinner.tupleIndex)?.tupleIndex! + 1}`;
    return {
      label: "Winner disagreement",
      tone: "mixed",
      detail: `Selection winner #${searchWinner.tupleIndex + 1} does not fully survive validation. The current frontier shifts toward ${detailParts}.`,
    };
  });
const eventPathSummaryRows = computed(() => {
  const inspection = selectedEventPathInspection.value;
  if (!inspection || inspection.points.length === 0) return [];
  const first = inspection.points[0];
  const last = inspection.points[inspection.points.length - 1];
  const optionLow = inspection.points.reduce(
    (acc, point) => Math.min(acc, point.optionLow),
    Number.POSITIVE_INFINITY,
  );
  const optionHigh = inspection.points.reduce(
    (acc, point) => Math.max(acc, point.optionHigh),
    Number.NEGATIVE_INFINITY,
  );
  const underlierLow = inspection.points.reduce(
    (acc, point) => Math.min(acc, point.underlyingPrice),
    Number.POSITIVE_INFINITY,
  );
  const underlierHigh = inspection.points.reduce(
    (acc, point) => Math.max(acc, point.underlyingPrice),
    Number.NEGATIVE_INFINITY,
  );
  return [
    {
      key: "path-points",
      label: "Path points",
      value: `${inspection.pointCount}`,
    },
    {
      key: "path-window",
      label: "Window",
      value: `${formatTimestamp(first?.timestampMs)} -> ${formatTimestamp(last?.timestampMs)}`,
    },
    {
      key: "option-range",
      label: "Option range",
      value: `${formatOptionalDecimal(optionLow)} -> ${formatOptionalDecimal(optionHigh)}`,
    },
    {
      key: "underlier-range",
      label: "Underlier range",
      value: `${formatOptionalDecimal(underlierLow, 2)} -> ${formatOptionalDecimal(underlierHigh, 2)}`,
    },
    {
      key: "option-close-delta",
      label: "Option open -> close",
      value: `${formatOptionalDecimal(first?.optionOpen)} -> ${formatOptionalDecimal(last?.optionClose)}`,
    },
    {
      key: "underlier-delta",
      label: "Underlier start -> end",
      value: `${formatOptionalDecimal(first?.underlyingPrice, 2)} -> ${formatOptionalDecimal(last?.underlyingPrice, 2)}`,
    },
  ];
});
const eventPathAnalyticsRows = computed(() => {
  const inspection = selectedEventPathInspection.value;
  const selectedRow = selectedInspectionRow.value;
  if (!inspection || !selectedRow || inspection.points.length === 0) return [];
  const entryOption =
    selectedRow.entryOptionPrice ?? inspection.points[0]?.optionClose;
  const entryUnderlier =
    selectedRow.entryUnderlierPrice ?? inspection.points[0]?.underlyingPrice;
  const peakOptionPoint = inspection.points.reduce((best, point) =>
    point.optionHigh > best.optionHigh ? point : best,
  );
  const troughOptionPoint = inspection.points.reduce((best, point) =>
    point.optionLow < best.optionLow ? point : best,
  );
  const peakUnderlierPoint = inspection.points.reduce((best, point) =>
    point.underlyingPrice > best.underlyingPrice ? point : best,
  );
  const troughUnderlierPoint = inspection.points.reduce((best, point) =>
    point.underlyingPrice < best.underlyingPrice ? point : best,
  );
  return [
    {
      key: "entry-raw",
      label: "Signal underlier / requested entry",
      value: `${formatOptionalDecimal(selectedRow.rawUnderlierPrice, 2)} / ${formatTimestamp(selectedRow.requestedEntryTimestamp)}`,
    },
    {
      key: "option-peak",
      label: "Option peak / peak return",
      value: `${formatOptionalDecimal(peakOptionPoint.optionHigh)} @ ${formatTimestamp(peakOptionPoint.timestampMs)}${entryOption && entryOption > 0 ? ` · ${formatPercent((peakOptionPoint.optionHigh - entryOption) / entryOption, 2)}` : ""}`,
    },
    {
      key: "option-trough",
      label: "Option trough / trough return",
      value: `${formatOptionalDecimal(troughOptionPoint.optionLow)} @ ${formatTimestamp(troughOptionPoint.timestampMs)}${entryOption && entryOption > 0 ? ` · ${formatPercent((troughOptionPoint.optionLow - entryOption) / entryOption, 2)}` : ""}`,
    },
    {
      key: "underlier-peak",
      label: "Underlier peak / move",
      value: `${formatOptionalDecimal(peakUnderlierPoint.underlyingPrice, 2)} @ ${formatTimestamp(peakUnderlierPoint.timestampMs)}${entryUnderlier && entryUnderlier > 0 ? ` · ${formatPercent((peakUnderlierPoint.underlyingPrice - entryUnderlier) / entryUnderlier, 2)}` : ""}`,
    },
    {
      key: "underlier-trough",
      label: "Underlier trough / move",
      value: `${formatOptionalDecimal(troughUnderlierPoint.underlyingPrice, 2)} @ ${formatTimestamp(troughUnderlierPoint.timestampMs)}${entryUnderlier && entryUnderlier > 0 ? ` · ${formatPercent((troughUnderlierPoint.underlyingPrice - entryUnderlier) / entryUnderlier, 2)}` : ""}`,
    },
  ];
});

const displayedRowsWithIndex = computed(() => {
  const rawInvalidRate = normalizeText(maxInvalidRateFilterInput.value).length
    ? Number(maxInvalidRateFilterInput.value)
    : Number.POSITIVE_INFINITY;
  const rawMinRobustScore = normalizeText(minRobustScoreFilterInput.value)
    .length
    ? Number(minRobustScoreFilterInput.value)
    : Number.NEGATIVE_INFINITY;
  const rawMaxStressDrop = normalizeText(maxStressDropFilterInput.value).length
    ? Number(maxStressDropFilterInput.value)
    : Number.POSITIVE_INFINITY;
  const maxInvalidRate = Number.isFinite(rawInvalidRate)
    ? rawInvalidRate
    : Number.POSITIVE_INFINITY;
  const minRobustScore = Number.isFinite(rawMinRobustScore)
    ? rawMinRobustScore
    : Number.NEGATIVE_INFINITY;
  const maxStressDrop = Number.isFinite(rawMaxStressDrop)
    ? rawMaxStressDrop
    : Number.POSITIVE_INFINITY;

  const rows = activeRows.value
    .map((row, originalIndex) => ({ row, originalIndex }))
    .filter(({ row }) => !finalistsOnly.value || row.sealedFinalist)
    .filter(({ row }) => row.selectionMetrics.invalidRate <= maxInvalidRate)
    .filter(({ row }) => row.robustScore >= minRobustScore)
    .filter(({ row }) => {
      const stressed =
        row.extremeStressMetrics?.expectancyPerPremium ??
        row.harsherStressMetrics?.expectancyPerPremium ??
        row.stressMetrics?.expectancyPerPremium;
      if (!Number.isFinite(maxStressDrop) || stressed == null) return true;
      return (
        row.selectionMetrics.expectancyPerPremium - stressed <= maxStressDrop
      );
    });

  const metricFor = (entry: (typeof rows)[number]): number => {
    switch (resultSortKey.value) {
      case "selection_ev":
        return entry.row.selectionMetrics.expectancyPerPremium;
      case "holdout_ev":
        return (
          entry.row.holdoutMetrics?.expectancyPerPremium ??
          Number.NEGATIVE_INFINITY
        );
      case "stress_ev":
        return (
          entry.row.stressMetrics?.expectancyPerPremium ??
          Number.NEGATIVE_INFINITY
        );
      case "harsher_ev":
        return (
          entry.row.harsherStressMetrics?.expectancyPerPremium ??
          Number.NEGATIVE_INFINITY
        );
      case "extreme_ev":
        return (
          entry.row.extremeStressMetrics?.expectancyPerPremium ??
          Number.NEGATIVE_INFINITY
        );
      case "invalid_rate":
        return entry.row.selectionMetrics.invalidRate;
      case "valid_trades":
        return entry.row.selectionMetrics.validTrades;
      case "sealed_rank":
        return entry.row.sealedFinalRank ?? Number.POSITIVE_INFINITY;
      case "robust":
      default:
        return entry.row.robustScore;
    }
  };

  rows.sort((left, right) => {
    const leftMetric = metricFor(left);
    const rightMetric = metricFor(right);
    if (leftMetric === rightMetric)
      return left.originalIndex - right.originalIndex;
    return resultSortDirection.value === "asc"
      ? leftMetric - rightMetric
      : rightMetric - leftMetric;
  });
  return rows;
});

watch(displayedRowsWithIndex, (rows) => {
  if (!rows.length) return;
  if (!rows.some((entry) => entry.originalIndex === selectedTupleIndex.value)) {
    selectedTupleIndex.value = rows[0]?.originalIndex ?? 0;
  }
});

watch(
  inspectionRows,
  (rows) => {
    if (!rows.length) {
      selectedInspectionEventId.value = "";
      store.clearEventPathInspection();
      return;
    }
    if (!rows.some((row) => row.eventId === selectedInspectionEventId.value)) {
      selectedInspectionEventId.value = rows[0]?.eventId || "";
    }
  },
  { immediate: true },
);

watch(
  selectedInspectionRow,
  (row) => {
    if (!row) {
      store.clearEventPathInspection();
      return;
    }
    if (row.status !== "executed") {
      store.clearEventPathInspection();
      return;
    }
    const existing = activeEventPathInspection.value;
    if (
      existing &&
      existing.tupleIndex === selectedTupleIndex.value &&
      existing.eventIndex === row.eventIndex
    ) {
      return;
    }
    void store.loadEventPathInspection(
      selectedTupleIndex.value,
      row.eventIndex,
    );
  },
  { immediate: true },
);

const resultSummaryChips = computed(() => {
  const diagnostics = activeDiagnostics.value;
  if (!diagnostics) return [];
  const chips = [
    `${diagnostics.totalEvents} events`,
    `${activeRankedResults.value.length} tuples`,
  ];
  if (activeBackendUsed.value) chips.push(activeBackendUsed.value);
  if (activePriceSemantics.value) chips.push(activePriceSemantics.value);
  if (diagnostics.holdoutEvents > 0)
    chips.push(`${diagnostics.holdoutEvents} holdout`);
  if (diagnostics.sealedTestEvents > 0)
    chips.push(`${diagnostics.sealedTestEvents} sealed`);
  if (activeStaging.value) {
    chips.push(
      `${activeStaging.value.compiledCandidateCount}/${activeStaging.value.candidateSlotCount} candidates`,
    );
    chips.push(
      `${activeStaging.value.baselineExitSummaryCount} baseline summaries`,
    );
    chips.push(
      `${activeStaging.value.conservativeExitSummaryCount} conservative summaries`,
    );
    chips.push(
      `${activeStaging.value.harsherExitSummaryCount} harsher summaries`,
    );
  }
  return chips;
});

const reportingMetricCards = computed(() => {
  const diagnostics = activeDiagnostics.value;
  if (!diagnostics) return [];
  const cards = [
    {
      key: "bull-bear",
      label: "Bull / Bear",
      value: `${diagnostics.bullishEvents} / ${diagnostics.bearishEvents}`,
    },
    {
      key: "search-holdout-sealed",
      label: "Search / Holdout / Sealed",
      value: `${diagnostics.searchEvents} / ${diagnostics.holdoutEvents} / ${diagnostics.sealedTestEvents}`,
    },
  ];
  if (activeStaging.value && activeStaging.value.candidateSlotCount > 0) {
    cards.push({
      key: "candidate-coverage",
      label: "Candidates",
      value: `${activeStaging.value.compiledCandidateCount} / ${activeStaging.value.candidateSlotCount}`,
    });
  }
  if (activeStaging.value) {
    cards.push({
      key: "exit-summaries",
      label: "Exit summaries",
      value: `${activeStaging.value.baselineExitSummaryCount} / ${activeStaging.value.conservativeExitSummaryCount} / ${activeStaging.value.harsherExitSummaryCount}`,
    });
  }
  return cards;
});
const feasibilityCoverageAdequacy = computed(() => {
  const preview = latestFeasibility.value;
  if (!preview) return null;
  return describeCoverageAdequacy({
    verdict: preview.coverageAdequacy.verdict,
    nonPromotable: preview.coverageAdequacy.nonPromotable,
    reasons: preview.coverageAdequacy.reasons,
    searchOverlapRate: preview.overlapWindow.searchOverlapRate,
    holdoutOverlapRate: preview.overlapWindow.holdoutOverlapRate,
    sealedTestOverlapRate: preview.overlapWindow.sealedTestOverlapRate,
    searchCandidateCoverageRate:
      preview.coverageAdequacy.searchCandidateCoverageRate,
    holdoutCandidateCoverageRate:
      preview.coverageAdequacy.holdoutCandidateCoverageRate,
    sealedTestCandidateCoverageRate:
      preview.coverageAdequacy.sealedTestCandidateCoverageRate,
    searchBaselineOutcomeCoverageRate:
      preview.coverageAdequacy.searchBaselineOutcomeCoverageRate,
    holdoutBaselineOutcomeCoverageRate:
      preview.coverageAdequacy.holdoutBaselineOutcomeCoverageRate,
    sealedTestBaselineOutcomeCoverageRate:
      preview.coverageAdequacy.sealedTestBaselineOutcomeCoverageRate,
  });
});
const feasibilityOverlapRows = computed(() => {
  const preview = latestFeasibility.value;
  if (!preview) return [];
  return [
    {
      key: "overlap-events",
      label: "Events in dataset window",
      value: `${preview.overlapWindow.totalEventsInDatasetWindow} / ${preview.diagnostics.totalEvents} (${formatPercent(preview.overlapWindow.totalOverlapRate, 1)})`,
    },
    {
      key: "overlap-search",
      label: "Search overlap",
      value: `${preview.overlapWindow.searchEventsInDatasetWindow} / ${preview.diagnostics.searchEvents} (${formatPercent(preview.overlapWindow.searchOverlapRate, 1)})`,
    },
    {
      key: "overlap-holdout",
      label: "Holdout overlap",
      value: `${preview.overlapWindow.holdoutEventsInDatasetWindow} / ${preview.diagnostics.holdoutEvents} (${formatPercent(preview.overlapWindow.holdoutOverlapRate, 1)})`,
    },
    {
      key: "overlap-sealed",
      label: "Sealed overlap",
      value: `${preview.overlapWindow.sealedTestEventsInDatasetWindow} / ${preview.diagnostics.sealedTestEvents} (${formatPercent(preview.overlapWindow.sealedTestOverlapRate, 1)})`,
    },
    {
      key: "contract-ref",
      label: "Contract reference as-of",
      value: preview.overlapWindow.contractReferenceAsOfUtc || "—",
    },
    {
      key: "contract-ref-search",
      label: "Search events before contract ref",
      value: String(
        preview.overlapWindow.searchEventsBeforeContractReferenceAsOf,
      ),
    },
    {
      key: "candidate-coverage",
      label: "Search / Holdout / Sealed candidates",
      value: `${formatPercent(preview.coverageAdequacy.searchCandidateCoverageRate, 1)} / ${formatPercent(preview.coverageAdequacy.holdoutCandidateCoverageRate, 1)} / ${formatPercent(preview.coverageAdequacy.sealedTestCandidateCoverageRate, 1)}`,
    },
    {
      key: "outcome-coverage",
      label: "Search / Holdout / Sealed outcomes",
      value: `${formatPercent(preview.coverageAdequacy.searchBaselineOutcomeCoverageRate, 1)} / ${formatPercent(preview.coverageAdequacy.holdoutBaselineOutcomeCoverageRate, 1)} / ${formatPercent(preview.coverageAdequacy.sealedTestBaselineOutcomeCoverageRate, 1)}`,
    },
  ];
});
const feasibilitySplitInvalidSections = computed(() => {
  const preview = latestFeasibility.value;
  if (!preview) return [];
  return [
    {
      key: "search",
      label: "Search invalid reasons",
      rows: preview.splitInvalidReasons.searchInvalidReasons,
    },
    {
      key: "holdout",
      label: "Holdout invalid reasons",
      rows: preview.splitInvalidReasons.holdoutInvalidReasons,
    },
    {
      key: "sealed",
      label: "Sealed invalid reasons",
      rows: preview.splitInvalidReasons.sealedTestInvalidReasons,
    },
  ].filter((entry) => entry.rows.length > 0);
});

const feasibilitySummaryRows = computed(() => {
  const preview = latestFeasibility.value;
  if (!preview) return [];
  const gpuRank = preview.timings.gpuRankingMs;
  const gpuRankUs = preview.timings.gpuRankingUs;
  const gpuTensor = preview.timings.gpuTensorScoreMs;
  const gpuTensorUs = preview.timings.gpuTensorScoreUs;
  const gpuExactify = preview.timings.gpuExactifyMs;
  const gpuExactifyUs = preview.timings.gpuExactifyUs;
  const gpuSealed = preview.timings.gpuSealedStageMs;
  const gpuSealedUs = preview.timings.gpuSealedStageUs;
  const gpuSpeedup = preview.timings.gpuSpeedupVsCpu;
  const backendLine =
    gpuRank == null
      ? `${preview.bestAvailableBackend || "cpu_reference"} · cpu ${formatLatency(preview.timings.cpuRankingMs, preview.timings.cpuRankingUs)}`
      : `${preview.bestAvailableBackend || "gpu_compiled_tensor_hybrid"} · cpu ${formatLatency(preview.timings.cpuRankingMs, preview.timings.cpuRankingUs)} / gpu hybrid ${formatLatency(gpuRank, gpuRankUs)}${gpuSpeedup == null ? "" : ` (${gpuSpeedup.toFixed(2)}x)`}`;
  const gpuBreakdownLine =
    gpuTensor == null
      ? "—"
      : `tensor ${formatLatency(gpuTensor, gpuTensorUs)} · exactify ${formatLatency(gpuExactify, gpuExactifyUs)} · sealed ${formatLatency(gpuSealed, gpuSealedUs)}`;
  const gpuUploadReuseLine = `shared ${preview.cacheStats.gpuSharedInputCacheHits}/${preview.cacheStats.gpuSharedInputCacheMisses} · outcome ${preview.cacheStats.gpuOutcomeInputCacheHits}/${preview.cacheStats.gpuOutcomeInputCacheMisses} · out ${preview.cacheStats.gpuOutputBufferReuses}/${preview.cacheStats.gpuOutputBufferAllocations}`;
  const gpuChunkingLine =
    preview.cacheStats.gpuTupleChunkCount > 0
      ? `chunks ${preview.cacheStats.gpuTupleChunkCount}×${preview.cacheStats.gpuTupleChunkSize} · launches ${preview.cacheStats.gpuKernelLaunchCount} · chunk ${formatBytes(preview.cacheStats.gpuTotalDeviceBytesPerChunk)} · outcome ${formatBytes(preview.cacheStats.gpuOutcomeTensorBytes)}`
      : "—";
  const gpuMemoryPlanLine = preview.cacheStats.gpuMemoryBudgetSource
    ? `${preview.cacheStats.gpuMemoryBudgetSource} · free ${formatBytes(preview.cacheStats.gpuDeviceFreeMemoryBytes)} · usable ${formatBytes(preview.cacheStats.gpuDeviceUsableMemoryBytes)} · budget ${formatBytes(preview.cacheStats.gpuTupleBudgetBytes)}`
    : "—";
  const gpuFitLine = preview.cacheStats.gpuMemoryBudgetSource
    ? `outcome ${preview.cacheStats.gpuOutcomeTensorFitsInMemory ? "fits" : "does not fit"} · chunk ${preview.cacheStats.gpuChunkPlanFitsInMemory ? "fits" : "does not fit"}`
    : "—";
  const replayBuildTimingLine = preview.replayBuildTiming
    ? `parent ${formatLatency(Math.floor(preview.replayBuildTiming.parentArtifactValidationUs / 1000), preview.replayBuildTiming.parentArtifactValidationUs)} · signal ${preview.replayBuildTiming.parentSignalProcessCacheHit ? "proc-hit" : preview.replayBuildTiming.persistedSignalManifestCacheHit ? "disk-hit" : "rebuilt"} · dataset ${preview.replayBuildTiming.optionsDatasetProcessCacheHit ? "proc-hit" : preview.replayBuildTiming.optionsDatasetPersistedCacheHit ? "disk-hit" : "csv-load"} · normalize ${formatLatency(Math.floor(preview.replayBuildTiming.requestNormalizationUs / 1000), preview.replayBuildTiming.requestNormalizationUs)}`
    : "—";
  const autoBackendLine = preview.autoBackendDecision.recommendedBackend
    ? `${preview.autoBackendDecision.recommendedBackend}${preview.autoBackendDecision.gpuEligible ? " · GPU-eligible" : " · CPU-favored"}`
    : "—";
  return [
    {
      key: "candidate-density",
      label: "Candidate density",
      value: `${preview.compiledCandidateCount} / ${preview.candidateSlotCount} (${formatPercent(preview.candidateDensity)})`,
    },
    {
      key: "invalid-slots",
      label: "Invalid slots",
      value: `${preview.invalidSlotCount} (${formatPercent(preview.invalidSlotRate)})`,
    },
    {
      key: "event-coverage",
      label: "Event coverage",
      value: `${preview.eventCoverage.totalEventsWithCandidates} candidate-ready / ${preview.eventCoverage.totalEventsWithBaselineOutcomes} baseline / ${preview.eventCoverage.totalEventsWithConservativeOutcomes} conservative / ${preview.eventCoverage.totalEventsWithHarsherOutcomes} harsher`,
    },
    {
      key: "tuple-grid",
      label: "Tuple grid",
      value: `${preview.compiledTupleCount} tuples · ${preview.projectedEventTupleEvaluations} event-tuples`,
    },
    {
      key: "preview-runtime",
      label: "Replay / Total preview",
      value: `${preview.replayBuildMs} ms / ${preview.totalPreviewMs} ms`,
    },
    {
      key: "replay-build-timing",
      label: "Replay build timing",
      value: replayBuildTimingLine,
    },
    {
      key: "compile-runtime",
      label: "Compile / CPU rank",
      value: `${preview.timings.baselineCandidateCompileMs + preview.timings.baselineExitSummaryCompileMs + preview.timings.conservativeCandidateCompileMs + preview.timings.conservativeExitSummaryCompileMs + preview.timings.harsherCandidateCompileMs + preview.timings.harsherExitSummaryCompileMs} ms compile · ${formatLatency(preview.timings.cpuRankingMs, preview.timings.cpuRankingUs)} rank`,
    },
    {
      key: "summary-density",
      label: "Summary density",
      value: `${formatPercent(preview.baselineExitSummaryDensity)} / ${formatPercent(preview.conservativeExitSummaryDensity)} / ${formatPercent(preview.harsherExitSummaryDensity)}`,
    },
    {
      key: "backend-preview",
      label: "Backend / Hybrid rank timing",
      value: backendLine,
    },
    {
      key: "auto-backend",
      label: "Auto backend",
      value: autoBackendLine,
    },
    {
      key: "gpu-breakdown",
      label: "GPU breakdown",
      value: gpuBreakdownLine,
    },
    {
      key: "cache-reuse",
      label: "Cache reuse",
      value: `struct ${preview.cacheStats.structuralCacheHits}/${preview.cacheStats.structuralCacheMisses} · summary ${preview.cacheStats.summaryCacheHits}/${preview.cacheStats.summaryCacheMisses}`,
    },
    {
      key: "gpu-upload-reuse",
      label: "GPU upload reuse",
      value: gpuUploadReuseLine,
    },
    {
      key: "gpu-chunking",
      label: "GPU chunking",
      value: gpuChunkingLine,
    },
    {
      key: "gpu-memory-plan",
      label: "GPU memory plan",
      value: gpuMemoryPlanLine,
    },
    {
      key: "gpu-fit",
      label: "GPU fit",
      value: gpuFitLine,
    },
  ];
});

const feasibilityInvalidReasons = computed(() => {
  const preview = latestFeasibility.value;
  if (!preview || preview.invalidReasons.length === 0) return "";
  return preview.invalidReasons
    .slice(0, 3)
    .map((entry) => `${entry.reason} (${entry.count})`)
    .join(" | ");
});

const feasibilityRecommendations = computed(() => {
  const preview = latestFeasibility.value;
  if (!preview) return [];
  const out: string[] = [];
  const axisCoverage = preview.axisCoverage ?? {
    delayMinutes: [],
    dteDays: [],
    moneynessBuckets: [],
  };
  if (preview.candidateDensity < 0.3 || preview.invalidSlotRate > 0.7) {
    pushUniqueRecommendation(
      out,
      `This grid is still quality-bound rather than scorer-bound: density is ${formatPercent(preview.candidateDensity)} and invalid slots are ${formatPercent(preview.invalidSlotRate)}. Narrow the representative search space before treating GPU as product-safe.`,
    );
  }
  const totalInvalid = Math.max(1, preview.invalidSlotCount);
  for (const entry of preview.invalidReasons) {
    const share = entry.count / totalInvalid;
    if (share < 0.1) continue;
    switch (entry.reason) {
      case "no_expiration_in_bucket":
        pushUniqueRecommendation(
          out,
          `${entry.reason} is ${formatPercent(share)} of invalid slots. Reduce the default DTE spread for this dataset or download a wider expiration window before widening the tuple grid.`,
        );
        break;
      case "no_strike_in_tolerance":
        pushUniqueRecommendation(
          out,
          `${entry.reason} is ${formatPercent(share)} of invalid slots. Re-center the strike window closer to spot or relax the moneyness selector tolerance before widening the tuple grid.`,
        );
        break;
      case "entry_past_cutoff":
        pushUniqueRecommendation(
          out,
          `${entry.reason} is ${formatPercent(share)} of invalid slots. Trim late delay buckets, lower max entry lag, or avoid late-session parent signals for this profile.`,
        );
        break;
      case "no_quote_after_entry_time":
      case "missing_data":
        pushUniqueRecommendation(
          out,
          `${entry.reason} is ${formatPercent(share)} of invalid slots. Check dataset coverage continuity before widening the search grid.`,
        );
        break;
    }
  }
  const zeroCoverageDte = axisCoverage.dteDays
    .filter((entry) => entry.validSlotCount === 0)
    .map((entry) => `${entry.dteDays}d`);
  if (zeroCoverageDte.length > 0) {
    pushUniqueRecommendation(
      out,
      `These DTE buckets are producing zero valid slots on the current dataset: ${zeroCoverageDte.join(", ")}. Drop them from the request or widen the downloaded expiration window before treating this as a representative grid.`,
    );
  }
  const cutoffHeavyDelays = axisCoverage.delayMinutes
    .filter((entry) => {
      const cutoffMisses =
        entry.invalidReasons.find(
          (reason) => reason.reason === "entry_past_cutoff",
        )?.count ?? 0;
      return (
        entry.invalidSlotCount > 0 &&
        cutoffMisses / entry.invalidSlotCount >= 0.5
      );
    })
    .map((entry) => `${entry.delayMinutes}m`);
  if (cutoffHeavyDelays.length > 0) {
    pushUniqueRecommendation(
      out,
      `Late-entry pressure is concentrated in these delay buckets: ${cutoffHeavyDelays.join(", ")}. Trim them, lower max entry lag, or keep the parent signal earlier in the session for this profile.`,
    );
  }
  const zeroCoverageMoneyness = axisCoverage.moneynessBuckets
    .filter((entry) => entry.validSlotCount === 0)
    .map((entry) => formatMoneynessBucketLabel(entry.moneynessBucket));
  if (zeroCoverageMoneyness.length > 0) {
    pushUniqueRecommendation(
      out,
      `These moneyness buckets never found a valid contract in the imported strike window: ${zeroCoverageMoneyness.join(", ")}. Re-center the strike range or prune those selectors before widening the grid.`,
    );
  }
  const gpuSpeedup = preview.timings.gpuSpeedupVsCpu;
  if (gpuSpeedup != null && gpuSpeedup >= 3.0) {
    pushUniqueRecommendation(
      out,
      `GPU hybrid ranking already clears the current speed gate at ${gpuSpeedup.toFixed(2)}x vs CPU. The remaining blocker is candidate quality, not scorer throughput.`,
    );
  }
  if (preview.cacheStats.gpuPlanNotes.length > 0) {
    for (const note of preview.cacheStats.gpuPlanNotes) {
      pushUniqueRecommendation(out, note);
    }
  }
  for (const reason of preview.autoBackendDecision.reasons ?? []) {
    pushUniqueRecommendation(out, reason);
  }
  return out;
});

const feasibilityGateRows = computed(() => {
  const preview = latestFeasibility.value;
  if (!preview) return [];
  const structuralAttempts =
    preview.cacheStats.structuralCacheHits +
    preview.cacheStats.structuralCacheMisses;
  const summaryAttempts =
    preview.cacheStats.summaryCacheHits + preview.cacheStats.summaryCacheMisses;
  const structuralReuse =
    structuralAttempts > 0
      ? preview.cacheStats.structuralCacheHits / structuralAttempts
      : null;
  const summaryReuse =
    summaryAttempts > 0
      ? preview.cacheStats.summaryCacheHits / summaryAttempts
      : null;
  const gpuSpeedup = preview.timings.gpuSpeedupVsCpu;
  return [
    {
      key: "density",
      label: "Candidate density >= 30%",
      value: formatPercent(preview.candidateDensity),
      passed: preview.candidateDensity >= 0.3,
      pending: false,
    },
    {
      key: "invalid-rate",
      label: "Invalid rate <= 70%",
      value: formatPercent(preview.invalidSlotRate),
      passed: preview.invalidSlotRate <= 0.7,
      pending: false,
    },
    {
      key: "struct-reuse",
      label: "Structural reuse >= 95%",
      value: structuralReuse == null ? "—" : formatPercent(structuralReuse),
      passed: structuralReuse != null && structuralReuse >= 0.95,
      pending: structuralReuse == null,
    },
    {
      key: "summary-reuse",
      label: "Summary reuse >= 95%",
      value: summaryReuse == null ? "—" : formatPercent(summaryReuse),
      passed: summaryReuse != null && summaryReuse >= 0.95,
      pending: summaryReuse == null,
    },
    {
      key: "gpu-speedup",
      label: "GPU hybrid speedup >= 3.0x",
      value: gpuSpeedup == null ? "pending" : `${gpuSpeedup.toFixed(2)}x`,
      passed: gpuSpeedup != null && gpuSpeedup >= 3.0,
      pending: gpuSpeedup == null,
    },
  ];
});

const feasibilityOverallStatus = computed<"pass" | "mixed" | "pending">(() => {
  const rows = feasibilityGateRows.value;
  if (!rows.length) return "pending";
  if (rows.every((row) => row.passed || row.pending)) {
    return rows.every((row) => row.passed) ? "pass" : "pending";
  }
  return "mixed";
});

const feasibilityLaneRows = computed(() => {
  const preview = latestFeasibility.value;
  if (!preview) return [];
  return [
    {
      key: "baseline",
      label: "Baseline",
      summaries: `${preview.baselineCompiledExitSummaryCount} / ${preview.baselineExitSummaryCount}`,
      density: formatPercent(preview.baselineExitSummaryDensity),
      compileMs: `${preview.timings.baselineCandidateCompileMs + preview.timings.baselineExitSummaryCompileMs} ms`,
      bytes: formatBytes(preview.baselineSummaryBytesEstimate),
    },
    {
      key: "conservative",
      label: "Conservative",
      summaries: `${preview.conservativeCompiledExitSummaryCount} / ${preview.conservativeExitSummaryCount}`,
      density: formatPercent(preview.conservativeExitSummaryDensity),
      compileMs: `${preview.timings.conservativeCandidateCompileMs + preview.timings.conservativeExitSummaryCompileMs} ms`,
      bytes: formatBytes(preview.conservativeSummaryBytesEstimate),
    },
    {
      key: "harsher",
      label: "Harsher",
      summaries: `${preview.harsherCompiledExitSummaryCount} / ${preview.harsherExitSummaryCount}`,
      density: formatPercent(preview.harsherExitSummaryDensity),
      compileMs: `${preview.timings.harsherCandidateCompileMs + preview.timings.harsherExitSummaryCompileMs} ms`,
      bytes: formatBytes(preview.harsherSummaryBytesEstimate),
    },
  ];
});

const feasibilityInvalidReasonRows = computed(() => {
  const preview = latestFeasibility.value;
  if (!preview) return [];
  const totalInvalid = preview.invalidSlotCount || 1;
  return preview.invalidReasons.slice(0, 6).map((entry) => ({
    ...entry,
    share: entry.count / totalInvalid,
  }));
});

const feasibilityAxisCoverageSections = computed(() => {
  const preview = latestFeasibility.value;
  if (!preview) return [];
  const axisCoverage = preview.axisCoverage ?? {
    delayMinutes: [],
    dteDays: [],
    moneynessBuckets: [],
  };
  return [
    {
      key: "delay",
      label: "Delay buckets",
      bucketLabel: "Delay",
      rows: axisCoverage.delayMinutes.map((entry) => ({
        key: `delay-${entry.delayMinutes}`,
        bucket: `${entry.delayMinutes}m`,
        validSlots: `${entry.validSlotCount} / ${entry.slotCount}`,
        validRate: formatPercent(entry.validRate),
        invalidReasons: formatBucketInvalidReasons(entry.invalidReasons),
      })),
    },
    {
      key: "dte",
      label: "DTE buckets",
      bucketLabel: "DTE",
      rows: axisCoverage.dteDays.map((entry) => ({
        key: `dte-${entry.dteDays}`,
        bucket: `${entry.dteDays}d`,
        validSlots: `${entry.validSlotCount} / ${entry.slotCount}`,
        validRate: formatPercent(entry.validRate),
        invalidReasons: formatBucketInvalidReasons(entry.invalidReasons),
      })),
    },
    {
      key: "moneyness",
      label: "Moneyness buckets",
      bucketLabel: "Bucket",
      rows: axisCoverage.moneynessBuckets.map((entry) => ({
        key: `moneyness-${entry.moneynessBucket}`,
        bucket: formatMoneynessBucketLabel(entry.moneynessBucket),
        validSlots: `${entry.validSlotCount} / ${entry.slotCount}`,
        validRate: formatPercent(entry.validRate),
        invalidReasons: formatBucketInvalidReasons(entry.invalidReasons),
      })),
    },
  ].filter((section) => section.rows.length > 0);
});

const requestSummaryRows = computed(() => {
  const request = activeRequest.value;
  if (!request) return [];
  const filters = request.filters;
  const filterSegments = [
    `entry lag ${filters.maxEntryLagMinutes ?? "—"}`,
    `max prem ${filters.maxPremium ?? "—"}`,
    `min prem ${filters.minPremium ?? "—"}`,
    `min vol ${filters.minVolume ?? "—"}`,
    `min OI ${filters.minOpenInterest ?? "—"}`,
  ];
  return [
    {
      key: "artifact-ts",
      label: "Artifact timestamp",
      value: normalizeText(activeArtifactTimestamp.value) || "—",
    },
    {
      key: "dataset-hash",
      label: "Dataset hash",
      value: normalizeText(activeOptionsDatasetHash.value) || "—",
    },
    {
      key: "delays-dte-holds",
      label: "Delays / DTE / Holds",
      value: `${formatList(request.delayMinutes)} | ${formatList(request.dteBuckets)} | ${formatList(request.holdMinutes)}`,
    },
    {
      key: "targets-stops",
      label: "Targets / Stops",
      value: `${formatList(request.targetReturnBuckets)} | ${formatList(request.stopReturnBuckets)}`,
    },
    {
      key: "exit-fill",
      label: "Exit / Fill",
      value: `${normalizeText(request.exitTemplate) || "—"} | ${normalizeText(request.fillModel) || "—"}`,
    },
    {
      key: "realism-lane",
      label: "Realism lane",
      value: `${activeRealismLane.value.label} | ${normalizeText(request.fillModel) || "—"} | ${normalizeText(activePriceSemantics.value) || "—"}`,
    },
    {
      key: "structure-family",
      label: "Structure",
      value: normalizeText(request.structureFamily) || "long_single",
    },
    {
      key: "backend-route",
      label: "Backend route",
      value: activeBackendRoute.value.label,
    },
    {
      key: "backend-detail",
      label: "Backend detail",
      value: activeBackendRoute.value.detail,
    },
    {
      key: "dataset-provenance",
      label: "Dataset provenance",
      value: activeDatasetProvenanceSummary.value || "—",
    },
    {
      key: "splits",
      label: "Split assignment / ratios",
      value:
        `${formatSplitAssignmentMode(request.splitAssignmentMode)} | ` +
        `${request.splitRatioSearch.toFixed(2)} / ${request.splitRatioHoldout.toFixed(2)}`,
    },
    {
      key: "sealed-finalists",
      label: "Sealed finalists",
      value: `${request.sealedFinalistCount ?? 0}`,
    },
    {
      key: "filters",
      label: "Filters",
      value: filterSegments.join(" | "),
    },
  ];
});

function toggleMoneyness(bucket: string, checked: boolean): void {
  const next = new Set(selectedMoneynessBuckets.value);
  if (checked) next.add(bucket);
  else next.delete(bucket);
  selectedMoneynessBuckets.value = Array.from(next);
}

function onMoneynessToggle(bucket: string, event: Event): void {
  const target = event.target;
  const checked = target instanceof HTMLInputElement ? target.checked : false;
  toggleMoneyness(bucket, checked);
}

function selectTuple(index: number): void {
  selectedTupleIndex.value = index;
}

function onTupleRowKeydown(event: KeyboardEvent, index: number): void {
  if (event.key !== "Enter" && event.key !== " ") return;
  event.preventDefault();
  selectTuple(index);
}

function tupleRowLabel(row: OptionsExpressionTupleSummary, index: number): string {
  return `Select tuple #${index + 1}: ${row.dteDays} DTE, ${row.moneynessBucket}, ${row.holdMinutes} minute hold`;
}

function loadSelectedTupleInspection(): void {
  if (!selectedTuple.value) return;
  void store.loadTupleInspection(selectedTupleIndex.value);
}

function loadSelectedEventPath(): void {
  const row = selectedInspectionRow.value;
  if (!row || row.status !== "executed") return;
  void store.loadEventPathInspection(selectedTupleIndex.value, row.eventIndex);
}

function selectInspectionRow(eventId: string): void {
  selectedInspectionEventId.value = normalizeText(eventId);
  store.clearEventPathInspection();
}

function onInspectionRowKeydown(event: KeyboardEvent, eventId: string): void {
  if (event.key !== "Enter" && event.key !== " ") return;
  event.preventDefault();
  selectInspectionRow(eventId);
}

function inspectionRowLabel(row: OptionsExpressionEventInspectionRow): string {
  return `Select inspection event ${normalizeText(row.eventId) || row.eventIndex + 1}`;
}

function formatTupleIdentityShort(
  index: number | null | undefined,
): string | null {
  if (index == null || index < 0 || !Number.isFinite(index)) return null;
  return `#${index + 1}`;
}

function hasArtifactLeaderSummaryData(
  entry: OptionsExpressionArtifactSummary,
): boolean {
  const summary = entry.leaderSummary;
  if (!summary) return false;
  return (
    summary.distinctLeaderCount > 0 ||
    summary.searchLeaderTupleIndex != null ||
    summary.robustLeaderTupleIndex != null ||
    summary.holdoutLeaderTupleIndex != null ||
    summary.sealedLeaderTupleIndex != null ||
    summary.stressLeaderTupleIndex != null ||
    summary.robustLeaderReachedSealedFinalist != null
  );
}

function artifactLeadershipLabel(
  entry: OptionsExpressionArtifactSummary,
): string {
  if (!hasArtifactLeaderSummaryData(entry)) return "—";
  const count = entry.leaderSummary?.distinctLeaderCount ?? 0;
  if (count <= 0) return "—";
  if (count === 1) return "same retained leader";
  return `${count} retained leaders`;
}

function artifactLeadershipMap(
  entry: OptionsExpressionArtifactSummary,
): string {
  if (!hasArtifactLeaderSummaryData(entry)) return "—";
  const parts = [
    [
      "Sel",
      formatTupleIdentityShort(entry.leaderSummary?.searchLeaderTupleIndex),
    ],
    [
      "Robust",
      formatTupleIdentityShort(entry.leaderSummary?.robustLeaderTupleIndex),
    ],
    [
      "Hold",
      formatTupleIdentityShort(entry.leaderSummary?.holdoutLeaderTupleIndex),
    ],
    [
      "Sealed",
      formatTupleIdentityShort(entry.leaderSummary?.sealedLeaderTupleIndex),
    ],
    [
      "Stress",
      formatTupleIdentityShort(entry.leaderSummary?.stressLeaderTupleIndex),
    ],
  ]
    .filter((entry): entry is [string, string] => Boolean(entry[1]))
    .map(([label, value]) => `${label} ${value}`);
  return parts.length > 0 ? parts.join(" · ") : "—";
}

function artifactLeadershipDisagreement(
  entry: OptionsExpressionArtifactSummary,
): string {
  if (!hasArtifactLeaderSummaryData(entry)) return "—";
  const parts: string[] = [];
  if (entry.leaderSummary?.searchVsRobustDisagree) parts.push("Sel≠Robust");
  if (entry.leaderSummary?.robustVsHoldoutDisagree === true)
    parts.push("Robust≠Hold");
  if (entry.leaderSummary?.robustVsSealedDisagree === true)
    parts.push("Robust≠Sealed");
  if (parts.length === 0) return "Leadership aligned across present lanes";
  return parts.join(" · ");
}

function artifactRobustLeaderFinalistDetail(
  entry: OptionsExpressionArtifactSummary,
): string {
  if (!hasArtifactLeaderSummaryData(entry)) return "—";
  const reached = entry.leaderSummary?.robustLeaderReachedSealedFinalist;
  const rank = entry.leaderSummary?.robustLeaderSealedFinalRank;
  if (reached == null) return "—";
  if (!reached) return "Robust leader did not reach the sealed finalist set";
  if (rank != null) return `Robust leader reached sealed finalists (#${rank})`;
  return "Robust leader reached the sealed finalist set";
}

function formatArtifactOptionLabel(
  entry: OptionsExpressionArtifactSummary,
): string {
  const validation = describeArtifactValidation(entry);
  const lane = describeRealismLane(
    entry.priceSemantics,
    entry.requestFillModel,
  );
  const backendRoute = describeBackendRoute(
    entry.requestBackendPreference,
    entry.backendUsed,
    entry.backendFallbackReason,
  );
  const parts = [
    normalizeText(entry.ts) || normalizeText(entry.path),
    normalizeText(entry.optionsDatasetId) || "unknown dataset",
    `${lane.shortLabel} · ${normalizeText(entry.requestFillModel) || "aggregate_research"}`,
    backendRoute.label,
    normalizeText(entry.requestStructureFamily) || "long_single",
    `${entry.tupleCount} tuples`,
    `validation ${validation?.label ?? "Search only"}`,
    `robust ${entry.bestRobustScore.toFixed(3)}`,
    `sel ${entry.bestSelectionExpectancyPerPremium.toFixed(3)}`,
  ];
  if (entry.bestHoldoutExpectancyPerPremium != null) {
    parts.push(`hold ${entry.bestHoldoutExpectancyPerPremium.toFixed(3)}`);
  } else if (entry.hasHoldoutMetrics) {
    parts.push("holdout");
  }
  if (entry.bestSealedExpectancyPerPremium != null) {
    parts.push(`sealed ${entry.bestSealedExpectancyPerPremium.toFixed(3)}`);
  } else if (entry.hasSealedEvaluationMetrics) {
    parts.push("sealed");
  }
  if (entry.bestStressExpectancyPerPremium != null) {
    parts.push(`stress ${entry.bestStressExpectancyPerPremium.toFixed(3)}`);
  } else if (entry.hasStressMetrics) {
    parts.push("stress");
  }
  if (entry.bestSealedStressExpectancyPerPremium != null) {
    parts.push(
      `sealed stress ${entry.bestSealedStressExpectancyPerPremium.toFixed(3)}`,
    );
  }
  if (entry.bestHarsherStressExpectancyPerPremium != null) {
    parts.push(
      `harsher ${entry.bestHarsherStressExpectancyPerPremium.toFixed(3)}`,
    );
  } else if (entry.hasHarsherStressMetrics) {
    parts.push("harsher");
  }
  if (entry.bestSealedHarsherStressExpectancyPerPremium != null) {
    parts.push(
      `sealed harsher ${entry.bestSealedHarsherStressExpectancyPerPremium.toFixed(3)}`,
    );
  }
  if (entry.bestExtremeStressExpectancyPerPremium != null) {
    parts.push(
      `extreme ${entry.bestExtremeStressExpectancyPerPremium.toFixed(3)}`,
    );
  } else if (entry.hasExtremeStressMetrics) {
    parts.push("extreme");
  }
  if (entry.bestSealedExtremeStressExpectancyPerPremium != null) {
    parts.push(
      `sealed extreme ${entry.bestSealedExtremeStressExpectancyPerPremium.toFixed(3)}`,
    );
  }
  if ((entry.sealedFinalistBudget || 0) > 0) {
    parts.push(
      `finalists ${entry.sealedEvaluatedTupleCount || 0}/${entry.sealedFinalistBudget || 0}`,
    );
  }
  const leadership = artifactLeadershipLabel(entry);
  if (leadership !== "—") {
    parts.push(leadership);
  }
  if (normalizeText(entry.backendFallbackReason)) {
    parts.push("fallback");
  }
  return parts.join(" · ");
}

function loadArtifactFromComparison(path: string): void {
  selectedArtifactPath.value = path;
  void store.loadArtifact();
}

function handleRealismLaneChange(event: Event): void {
  const value =
    (event.target as HTMLSelectElement | null)?.value || "aggregate";
  store.setRealismLane(value, true);
}

function focusTupleIndex(index: number): void {
  selectedTupleIndex.value = index;
}
</script>

<template>
  <section
    class="optionsPanel surfaceFocus"
    aria-label="Options expression follow-up"
  >
    <div class="panelSectionHeader">
      <div class="panelHeaderVisual">
        <AppVisualIcon
          :icon="optionsFollowupWorkflowIcon"
          class="optionsPaneGlyph"
        />
        <div>
          <div class="panelEyebrow">Follow-up workflow</div>
          <h3 class="panelPaneTitle" style="margin: 0">Options Expression</h3>
          <div class="panelSectionLead">
            Express a saved validated directional artifact through a constrained
            Polygon.io options search without mutating the main workflow state.
          </div>
          <div class="panelMuted" style="margin-top: 6px">
            This is an options follow-up surface. Conventional stock
            optimization and validation remain in the main workflow.
          </div>
        </div>
      </div>
    </div>

    <section v-if="!hasParentSelection" class="emptyState">
      <img
        class="emptyStateIllustration"
        :src="emptyOptionsWorkspaceIllustration"
        alt=""
        aria-hidden="true"
        draggable="false"
      />
      <div class="emptyTitle">Select a saved run first.</div>
      <div class="emptyBody">
        Options expression optimization is launched from a saved artifact in the
        History workspace.
      </div>
    </section>

    <section v-else-if="!supportedParent" class="emptyState">
      <img
        class="emptyStateIllustration"
        :src="emptyOptionsWorkspaceIllustration"
        alt=""
        aria-hidden="true"
        draggable="false"
      />
      <div class="emptyTitle">Saved run not supported yet.</div>
      <div class="emptyBody">
        The current vertical slice supports saved
        <span class="mono">DoubleMa</span>,
        <span class="mono">CompositeTrend</span>,
        <span class="mono">RegimeHybrid</span>, and
        <span class="mono">BreakoutMomentum</span> artifacts. Other strategy
        families stay read-only until their manifest projector is implemented.
      </div>
    </section>

    <template v-else>
      <div class="panelSummaryChips" aria-label="Options expression summary">
        <span
          v-for="chip in resultSummaryChips"
          :key="chip"
          class="panelSummaryChip mono"
          >{{ chip }}</span
        >
      </div>

      <div class="optionsGrid">
        <div class="controlBlock">
          <label for="optionsRealismLane">Realism lane</label>
          <div class="row inlineControls">
            <select
              id="optionsRealismLane"
              :value="realismLane"
              @change="handleRealismLaneChange"
            >
              <option
                v-for="entry in OPTIONS_REALISM_LANE_OPTIONS"
                :key="entry"
                :value="entry"
              >
                {{ formatRealismLaneLabel(entry) }}
              </option>
            </select>
          </div>
          <div class="panelMuted">{{ realismLaneSummary }}</div>
          <div class="panelMuted">
            {{ selectedRealismLaneGuidance.laneLabel }} ·
            {{ selectedRealismLaneGuidance.detail }}
          </div>
          <div class="panelMuted">
            {{ realismLaneRouteNote }}
          </div>
          <details class="panelDetails">
            <summary>Lane product status</summary>
            <div class="panelDetailsBody">
              <div class="detailGrid detailGridTight">
                <div v-for="row in realismLaneProductRows" :key="row.key">
                  <span class="panelMetricKey">{{ row.label }}</span>
                  <span class="panelMetricValue mono panelNumeric">{{
                    row.value
                  }}</span>
                </div>
              </div>
              <div v-if="realismLaneHistoryRows.length > 0" class="tableWrap">
                <table class="resultTable">
                  <thead>
                    <tr>
                      <th>Artifact</th>
                      <th>Dataset</th>
                      <th>Backend</th>
                      <th>Validation</th>
                      <th>Sel EV</th>
                      <th>Val EV</th>
                      <th>Stress EV</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="row in realismLaneHistoryRows" :key="row.path">
                      <td class="mono">{{ normalizeText(row.ts) || "—" }}</td>
                      <td class="mono">{{ row.datasetId }}</td>
                      <td class="mono">{{ row.backend }}</td>
                      <td class="mono">{{ row.validation }}</td>
                      <td class="mono">
                        {{ formatOptionalDecimal(row.selection, 3) }}
                      </td>
                      <td class="mono">
                        {{ formatOptionalDecimal(row.validated, 3) }}
                      </td>
                      <td class="mono">
                        {{ formatOptionalDecimal(row.stress, 3) }}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div v-else class="panelMuted">
                No saved artifacts currently match the selected
                {{ formatRealismLaneLabel(realismLane) }} lane for this parent
                run.
              </div>
            </div>
          </details>
        </div>

        <div class="controlBlock">
          <label for="optionsDatasetSelect">Options dataset</label>
          <div class="row inlineControls">
            <select
              id="optionsDatasetSelect"
              v-model="selectedDatasetId"
              data-qa-control="options.dataset.select"
            >
              <option value="">
                Select {{ formatRealismLaneLabel(realismLane) }} Polygon.io
                dataset…
              </option>
              <option
                v-for="entry in realismLaneDatasets"
                :key="entry.optionsDatasetId"
                :value="entry.optionsDatasetId"
              >
                {{ entry.optionsDatasetId }} ·
                {{ entry.rootSymbol || "unknown root" }} ·
                {{ entry.priceSemantics }}
              </option>
            </select>
            <button
              type="button"
              data-qa-action="options.datasets.refresh"
              :disabled="datasetsBusy"
              @click="store.refreshDatasets()"
            >
              Refresh
            </button>
          </div>
          <div class="panelMuted">
            {{
              datasetSummary ||
              `Import or select a ${formatRealismLaneLabel(realismLane)} Polygon.io options dataset.`
            }}
          </div>
          <div v-if="datasetProvenanceSummary" class="panelMuted mono">
            {{ datasetProvenanceSummary }}
          </div>
          <div v-if="selectedDataset" class="panelMuted">
            {{ fillModelGuidance.laneLabel }} · {{ fillModelGuidance.detail }}
          </div>
          <div v-else class="panelMuted">
            No saved dataset currently matches the selected
            {{ formatRealismLaneLabel(realismLane) }} lane.
          </div>
          <div v-if="rootMismatch" class="warningText">
            Selected dataset root does not match the current saved run root hint
            ({{ parentRootHint || "unknown" }}). Continue only if that mismatch
            is intentional.
          </div>
        </div>

        <div class="controlBlock">
          <label for="optionsImportRoot"
            >Polygon.io {{ formatRealismLaneLabel(realismLane) }} dataset
            import</label
          >
          <div class="downloadFieldGrid">
            <label>
              Root
              <input
                id="optionsImportRoot"
                v-model="importRootSymbol"
                type="text"
                placeholder="SPY"
                spellcheck="false"
              />
            </label>
            <label>
              CSV semantics
              <select
                id="optionsImportPriceSemantics"
                v-model="importPriceSemantics"
                disabled
              >
                <option
                  v-for="entry in OPTIONS_IMPORT_PRICE_SEMANTICS_OPTIONS"
                  :key="entry"
                  :value="entry"
                >
                  {{ entry }}
                </option>
              </select>
            </label>
          </div>
          <div class="row inlineControls">
            <button
              type="button"
              data-qa-action="options.datasets.importCsv"
              :disabled="importBusy"
              @click="store.importDatasetFromPicker()"
              class="buttonWithIcon"
            >
              <AppVisualIcon
                :icon="optionsDatasetImportIcon"
                class="optionsActionIcon"
              />
              <span>Import Polygon.io CSV</span>
            </button>
          </div>
          <div class="panelMuted">
            Uses the existing native CSV picker and stores a normalized
            Polygon.io options dataset under the app market-data root. The
            selected realism lane controls the expected CSV semantics for this
            import.
          </div>
        </div>

        <div class="controlBlock">
          <label for="optionsDownloadFrom"
            >Polygon.io 1m options download</label
          >
          <div class="downloadFieldGrid">
            <label>
              From
              <input
                id="optionsDownloadFrom"
                v-model="downloadFromDateInput"
                type="date"
              />
            </label>
            <label>
              To
              <input v-model="downloadToDateInput" type="date" />
            </label>
            <label>
              Contract type
              <select
                id="optionsDownloadContractType"
                v-model="downloadContractType"
              >
                <option value="call">Calls</option>
                <option value="put">Puts</option>
                <option value="both">Calls + puts</option>
              </select>
            </label>
            <label>
              Dataset id override
              <input
                v-model="downloadDatasetIdInput"
                type="text"
                placeholder="optional_dataset_id"
                spellcheck="false"
              />
            </label>
            <label>
              Expiration from
              <input v-model="downloadExpirationFromDateInput" type="date" />
            </label>
            <label>
              Expiration to
              <input v-model="downloadExpirationToDateInput" type="date" />
            </label>
            <label>
              Max DTE days
              <input
                v-model="downloadMaxDteDaysInput"
                type="text"
                inputmode="numeric"
                placeholder="3"
                spellcheck="false"
              />
            </label>
            <label>
              Strike min
              <input
                v-model="downloadStrikeMinInput"
                type="text"
                inputmode="decimal"
                placeholder="645"
                spellcheck="false"
              />
            </label>
            <label>
              Strike max
              <input
                v-model="downloadStrikeMaxInput"
                type="text"
                inputmode="decimal"
                placeholder="665"
                spellcheck="false"
              />
            </label>
            <label>
              Max contracts
              <input
                v-model="downloadMaxContractsInput"
                type="text"
                inputmode="numeric"
                placeholder="30"
                spellcheck="false"
              />
            </label>
          </div>
          <div class="row inlineControls">
            <button
              type="button"
              data-qa-action="options.datasets.downloadPolygon"
              :disabled="downloadBusy || realismLane !== 'aggregate'"
              @click="store.downloadDatasetFromPolygon()"
              class="buttonWithIcon"
            >
              <AppVisualIcon
                :icon="optionsProviderDownloadIcon"
                class="optionsActionIcon"
              />
              <span>Download Polygon.io 1m data</span>
            </button>
          </div>
          <div class="panelMuted">
            This download is options-only and stays separate from the normal
            stock data workflow. Keep the date, expiry, strike, and contract
            windows bounded so the resulting 1m dataset remains usable for
            options-expression optimization. It uses the saved Polygon.io API
            key; free REST keys are rate-limited, so cached or imported data is
            preferable for broad research sweeps.
          </div>
          <div v-if="realismLane !== 'aggregate'" class="warningText">
            Direct provider download in this panel is currently available only
            for the Aggregate lane. Use CSV import for
            {{ formatRealismLaneLabel(realismLane) }} datasets.
          </div>
        </div>
      </div>

      <details v-if="selectedDataset" class="panelDetails" open>
        <summary>Dataset manifest</summary>
        <div class="panelDetailsBody">
          <div class="reportingGrid">
            <div
              v-for="row in selectedDatasetDetailRows"
              :key="row.key"
              class="metricCard"
            >
              <div class="panelMetricKey">{{ row.label }}</div>
              <div class="metricValue mono">{{ row.value }}</div>
            </div>
          </div>
          <div class="actions">
            <button
              type="button"
              data-qa-action="options.datasets.revealSelected"
              @click="store.revealSelectedDatasetPath()"
            >
              Reveal dataset path
            </button>
          </div>
        </div>
      </details>

      <details v-if="latestDatasetDownload" class="panelDetails" open>
        <summary>Latest download audit</summary>
        <div class="panelDetailsBody">
          <div class="reportingGrid">
            <div
              v-for="row in latestDownloadAuditRows"
              :key="row.key"
              class="metricCard"
            >
              <div class="panelMetricKey">{{ row.label }}</div>
              <div class="metricValue mono">{{ row.value }}</div>
            </div>
          </div>
          <div class="actions">
            <button
              type="button"
              data-qa-action="options.datasets.revealLatestDownload"
              @click="store.revealLatestDownloadUnderlyingPath()"
            >
              Reveal underlying CSV
            </button>
          </div>
        </div>
      </details>

      <details class="panelDetails" open>
        <summary>Search grid and execution settings</summary>
        <div class="panelDetailsBody optionsFields">
          <label>
            Delay minutes
            <input
              v-model="delayMinutesInput"
              type="text"
              placeholder="0,1,5"
              spellcheck="false"
            />
          </label>
          <label>
            DTE buckets
            <input
              v-model="dteBucketsInput"
              type="text"
              placeholder="1,3,5"
              spellcheck="false"
            />
          </label>
          <label>
            Hold minutes
            <input
              v-model="holdMinutesInput"
              type="text"
              placeholder="5,15,30,60"
              spellcheck="false"
            />
          </label>
          <label>
            Target buckets
            <input
              v-model="targetReturnInput"
              type="text"
              placeholder="0.25,0.5,1"
              spellcheck="false"
            />
          </label>
          <label>
            Stop buckets
            <input
              v-model="stopReturnInput"
              type="text"
              placeholder="0.25,0.5"
              spellcheck="false"
            />
          </label>
          <label>
            Exit template
            <select v-model="exitTemplate">
              <option
                v-for="entry in OPTIONS_EXIT_TEMPLATE_OPTIONS"
                :key="entry"
                :value="entry"
              >
                {{ entry }}
              </option>
            </select>
          </label>
          <label>
            Fill model
            <select v-model="fillModel">
              <option
                v-for="entry in fillModelOptionRows"
                :key="entry.value"
                :value="entry.value"
                :disabled="entry.disabled"
              >
                {{ entry.label }}
              </option>
            </select>
          </label>
          <label>
            Structure family
            <select v-model="structureFamily">
              <option
                v-for="entry in OPTIONS_STRUCTURE_FAMILY_OPTIONS"
                :key="entry"
                :value="entry"
              >
                {{ entry }}
              </option>
            </select>
          </label>
          <label>
            Backend preference
            <select v-model="backendPreference">
              <option
                v-for="entry in OPTIONS_BACKEND_PREFERENCE_OPTIONS"
                :key="entry"
                :value="entry"
              >
                {{ entry }}
              </option>
            </select>
          </label>
          <label>
            Top K
            <input
              v-model="topKInput"
              type="text"
              inputmode="numeric"
              spellcheck="false"
            />
          </label>
          <label>
            Sealed finalists
            <input
              v-model="sealedFinalistCountInput"
              type="text"
              inputmode="numeric"
              spellcheck="false"
            />
          </label>
          <label>
            Min valid trades
            <input
              v-model="minValidTradesInput"
              type="text"
              inputmode="numeric"
              spellcheck="false"
            />
          </label>
          <label>
            Search split
            <input
              v-model="splitRatioSearchInput"
              type="text"
              inputmode="decimal"
              spellcheck="false"
            />
          </label>
          <label>
            Holdout split
            <input
              v-model="splitRatioHoldoutInput"
              type="text"
              inputmode="decimal"
              spellcheck="false"
            />
          </label>
          <label>
            Split assignment
            <select v-model="splitAssignmentMode">
              <option
                v-for="entry in OPTIONS_SPLIT_ASSIGNMENT_MODE_OPTIONS"
                :key="entry"
                :value="entry"
              >
                {{ entry }}
              </option>
            </select>
          </label>
          <label>
            Max entry lag
            <input
              v-model="maxEntryLagMinutesInput"
              type="text"
              inputmode="numeric"
              spellcheck="false"
            />
          </label>
          <label>
            Max premium
            <input
              v-model="maxPremiumInput"
              type="text"
              inputmode="decimal"
              spellcheck="false"
            />
          </label>
          <label>
            Min premium
            <input
              v-model="minPremiumInput"
              type="text"
              inputmode="decimal"
              spellcheck="false"
            />
          </label>
          <label>
            Min volume
            <input
              v-model="minVolumeInput"
              type="text"
              inputmode="decimal"
              spellcheck="false"
            />
          </label>
          <label>
            Min open interest
            <input
              v-model="minOpenInterestInput"
              type="text"
              inputmode="decimal"
              spellcheck="false"
            />
          </label>
          <div class="full">
            <div class="panelMuted">
              <span class="mono">event_chronological</span> preserves the
              current default. Use
              <span class="mono">session_chronological</span> for multi-session
              feasibility work when holdout events are clustering late in the
              day.
            </div>
          </div>
          <div class="full">
            <div class="panelMuted">
              {{ fillModelGuidance.laneLabel }} · {{ fillModelGuidance.detail }}
            </div>
          </div>
          <div class="full">
            <div class="panelMetricKey">Moneyness buckets</div>
            <div class="moneynessList">
              <label
                v-for="entry in OPTIONS_MONEYNESS_BUCKET_OPTIONS"
                :key="entry"
                class="bucketToggle"
              >
                <input
                  :checked="selectedMoneynessBuckets.includes(entry)"
                  type="checkbox"
                  @change="onMoneynessToggle(entry, $event)"
                />
                <span class="mono">{{ entry }}</span>
              </label>
            </div>
          </div>
        </div>
      </details>

      <div class="actions">
        <button
          type="button"
          data-qa-action="options.feasibility.preview"
          :disabled="feasibilityBusy || !selectedDatasetId"
          @click="store.previewCurrentFeasibility()"
          class="buttonWithIcon"
        >
          <AppVisualIcon
            :icon="optionsFeasibilityPreviewIcon"
            class="optionsActionIcon"
          />
          <span>{{
            feasibilityBusy ? "Previewing…" : "Preview feasibility"
          }}</span>
        </button>
        <button
          type="button"
          data-qa-action="options.expression.run"
          :disabled="runBusy || !selectedDatasetId"
          @click="store.runCurrentExpression()"
          class="buttonWithIcon"
        >
          <AppVisualIcon
            :icon="optionsFollowupWorkflowIcon"
            class="optionsActionIcon"
          />
          <span>{{ runBusy ? "Running…" : "Optimize expression" }}</span>
        </button>
        <button
          type="button"
          :disabled="artifactsBusy"
          data-qa-action="options.artifacts.refresh"
          @click="store.refreshArtifacts()"
        >
          Refresh artifacts
        </button>
        <button
          type="button"
          :disabled="loadBusy || !selectedArtifactPath"
          data-qa-action="options.artifacts.load"
          @click="store.loadArtifact()"
        >
          Load selected artifact
        </button>
        <button
          type="button"
          :disabled="!activeArtifactPath"
          data-qa-action="options.artifacts.revealActive"
          @click="store.revealActiveArtifact()"
        >
          Reveal artifact
        </button>
        <button
          type="button"
          data-qa-action="options.expression.exportCsv"
          :disabled="activeRankedResults.length === 0"
          @click="store.exportActiveCsv()"
        >
          Export CSV
        </button>
      </div>

      <div class="controlBlock">
        <label for="optionsArtifactSelect"
          >Saved options expression artifacts</label
        >
        <select
          id="optionsArtifactSelect"
          v-model="selectedArtifactPath"
          data-qa-control="options.artifact.select"
        >
          <option value="">Select saved options expression artifact…</option>
          <option
            v-for="entry in filteredArtifacts"
            :key="entry.path"
            :value="entry.path"
          >
            {{ formatArtifactOptionLabel(entry) }}
          </option>
        </select>
        <div class="panelMuted">
          Filtered to the currently selected saved run.
        </div>
        <div v-if="selectedArtifactSummary" class="panelMuted mono">
          {{ selectedArtifactBackendRoute.label }} ·
          {{ selectedArtifactRealismLane.label }} ·
          {{ selectedArtifactSummary.requestFillModel || "aggregate_research" }}
          · validation
          {{ selectedArtifactValidation?.label ?? "Search only" }} · sel
          {{
            selectedArtifactSummary.bestSelectionExpectancyPerPremium.toFixed(
              3,
            )
          }}<template
            v-if="
              selectedArtifactSummary.bestHoldoutExpectancyPerPremium != null
            "
          >
            · hold
            {{
              selectedArtifactSummary.bestHoldoutExpectancyPerPremium.toFixed(3)
            }}</template
          ><template
            v-if="
              selectedArtifactSummary.bestSealedExpectancyPerPremium != null
            "
          >
            · sealed
            {{
              selectedArtifactSummary.bestSealedExpectancyPerPremium.toFixed(3)
            }}</template
          ><template
            v-if="
              selectedArtifactSummary.bestStressExpectancyPerPremium != null
            "
          >
            · stress
            {{
              selectedArtifactSummary.bestStressExpectancyPerPremium.toFixed(3)
            }}</template
          ><template
            v-if="
              selectedArtifactSummary.bestSealedStressExpectancyPerPremium !=
              null
            "
          >
            · sealed stress
            {{
              selectedArtifactSummary.bestSealedStressExpectancyPerPremium.toFixed(
                3,
              )
            }}</template
          ><template
            v-if="
              selectedArtifactSummary.bestHarsherStressExpectancyPerPremium !=
              null
            "
          >
            · harsher
            {{
              selectedArtifactSummary.bestHarsherStressExpectancyPerPremium.toFixed(
                3,
              )
            }}</template
          ><template
            v-if="
              selectedArtifactSummary.bestSealedHarsherStressExpectancyPerPremium !=
              null
            "
          >
            · sealed harsher
            {{
              selectedArtifactSummary.bestSealedHarsherStressExpectancyPerPremium.toFixed(
                3,
              )
            }}</template
          ><template
            v-if="
              selectedArtifactSummary.bestExtremeStressExpectancyPerPremium !=
              null
            "
          >
            · extreme
            {{
              selectedArtifactSummary.bestExtremeStressExpectancyPerPremium.toFixed(
                3,
              )
            }}</template
          ><template
            v-if="
              selectedArtifactSummary.bestSealedExtremeStressExpectancyPerPremium !=
              null
            "
          >
            · sealed extreme
            {{
              selectedArtifactSummary.bestSealedExtremeStressExpectancyPerPremium.toFixed(
                3,
              )
            }}</template
          >
        </div>
        <div
          v-if="selectedArtifactLeadershipRows.length > 0"
          class="panelMuted mono"
        >
          {{ selectedArtifactLeadershipRows[0]?.value }} ·
          {{ selectedArtifactLeadershipRows[1]?.value }}
        </div>
        <div
          v-if="selectedArtifactLeadershipRows.length > 0"
          class="panelMuted"
        >
          {{ selectedArtifactLeadershipRows[2]?.value }} ·
          {{ selectedArtifactLeadershipRows[3]?.value }}
        </div>
        <div
          v-if="selectedArtifactSupportRows.length > 0"
          class="panelMuted mono"
        >
          Hold {{ selectedArtifactSupportRows[0]?.value }} · Sealed
          {{ selectedArtifactSupportRows[1]?.value }} · Stress
          {{ selectedArtifactSupportRows[2]?.value }}
        </div>
        <div v-if="selectedArtifactValidation" class="panelMuted">
          {{ selectedArtifactValidation.detail }}
        </div>
        <div v-if="selectedArtifactSummary" class="panelMuted">
          {{ selectedArtifactRealismLane.detail }}
        </div>
        <div v-if="selectedArtifactSummary" class="panelMuted">
          {{ selectedArtifactBackendRoute.detail }}
        </div>
        <div v-if="selectedArtifactCoverageAdequacy" class="gateBadgeRow">
          <span
            class="gateBadge"
            :class="{
              isPass: selectedArtifactCoverageAdequacy.tone === 'pass',
              isPending: selectedArtifactCoverageAdequacy.tone === 'pending',
              isMixed: selectedArtifactCoverageAdequacy.tone === 'mixed',
            }"
          >
            Coverage {{ selectedArtifactCoverageAdequacy.label }}
          </span>
        </div>
        <details
          v-if="selectedArtifactDerivedMetrics.length > 0"
          class="panelDetails"
        >
          <summary>Reopened artifact validation frontier</summary>
          <div class="panelDetailsBody">
            <div v-if="selectedArtifactCoverageAdequacy" class="panelMuted">
              {{ selectedArtifactCoverageAdequacy.detail }}
            </div>
            <div class="detailGrid detailGridTight">
              <div v-for="row in selectedArtifactDerivedMetrics" :key="row.key">
                <span class="panelMetricKey">{{ row.label }}</span>
                <span class="panelMetricValue mono panelNumeric">{{
                  row.value
                }}</span>
              </div>
            </div>
            <div
              class="detailGrid detailGridTight"
              v-if="selectedArtifactSupportRows.length > 0"
            >
              <div v-for="row in selectedArtifactSupportRows" :key="row.key">
                <span class="panelMetricKey">{{ row.label }}</span>
                <span class="panelMetricValue mono panelNumeric">{{
                  row.value
                }}</span>
              </div>
            </div>
            <div
              class="detailGrid detailGridTight"
              v-if="selectedArtifactCoverageRows.length > 0"
            >
              <div v-for="row in selectedArtifactCoverageRows" :key="row.key">
                <span class="panelMetricKey">{{ row.label }}</span>
                <span class="panelMetricValue mono panelNumeric">{{
                  row.value
                }}</span>
              </div>
            </div>
            <div
              class="tableWrap"
              v-if="selectedArtifactValidationFrontierRows.length > 0"
            >
              <table class="resultTable">
                <thead>
                  <tr>
                    <th>Lane</th>
                    <th>EV / premium</th>
                    <th>Δ vs selection</th>
                    <th>Status</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="row in selectedArtifactValidationFrontierRows"
                    :key="row.key"
                  >
                    <td class="mono">{{ row.label }}</td>
                    <td class="mono">
                      {{ formatOptionalDecimal(row.expectancy, 4) }}
                    </td>
                    <td class="mono">
                      {{
                        row.delta === 0
                          ? "baseline"
                          : formatOptionalDecimal(row.delta, 4)
                      }}
                    </td>
                    <td class="mono">{{ row.status }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </details>
      </div>

      <details
        v-if="artifactComparisonRows.length > 0"
        class="panelDetails"
        open
      >
        <summary>Saved artifact comparison</summary>
        <div class="panelDetailsBody">
          <div class="matrixTableWrap">
            <table class="matrixTable compactTable">
              <thead>
                <tr>
                  <th>Artifact</th>
                  <th>Dataset</th>
                  <th>Lane</th>
                  <th>Backend</th>
                  <th>Adequacy</th>
                  <th>Overlap</th>
                  <th>Validation</th>
                  <th>Leadership</th>
                  <th>Disagree</th>
                  <th>Validated / Fragile</th>
                  <th>Validated %</th>
                  <th>Stress %</th>
                  <th>Diff vs selected</th>
                  <th>Val Δ</th>
                  <th>Stress Δ</th>
                  <th>Robust</th>
                  <th>Sel EV</th>
                  <th>Hold EV</th>
                  <th>Sealed EV</th>
                  <th>Stress</th>
                  <th>Harsher</th>
                  <th>Extreme</th>
                  <th>Finalists</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="entry in artifactComparisonRows"
                  :key="entry.path"
                  :class="{
                    selectedTupleRow:
                      normalizeText(entry.path) ===
                      normalizeText(selectedArtifactPath),
                  }"
                >
                  <td class="mono">{{ normalizeText(entry.ts) || "—" }}</td>
                  <td class="mono">{{ entry.optionsDatasetId }}</td>
                  <td class="mono">
                    {{
                      describeRealismLane(
                        entry.priceSemantics,
                        entry.requestFillModel,
                      ).shortLabel
                    }}
                    · {{ entry.requestFillModel || "aggregate_research" }}
                  </td>
                  <td class="mono">
                    {{
                      describeBackendRoute(
                        entry.requestBackendPreference,
                        entry.backendUsed,
                        entry.backendFallbackReason,
                      ).label
                    }}
                  </td>
                  <td class="mono">
                    {{
                      formatCoverageAdequacyLabel(
                        entry.coverageAdequacyVerdict,
                      )
                    }}<template v-if="entry.coverageNonPromotable">
                      · no-promote</template
                    >
                  </td>
                  <td class="mono">
                    {{ formatPercent(entry.searchOverlapRate, 1) }} /
                    {{ formatPercent(entry.holdoutOverlapRate, 1) }} /
                    {{ formatPercent(entry.sealedTestOverlapRate, 1) }}
                  </td>
                  <td class="mono">
                    {{
                      describeArtifactValidation(entry)?.label ?? "Search only"
                    }}
                  </td>
                  <td class="mono">
                    {{ artifactLeadershipLabel(entry)
                    }}<template
                      v-if="(entry.leaderSummary?.distinctLeaderCount ?? 0) > 0"
                    >
                      · {{ artifactLeadershipMap(entry) }}</template
                    >
                  </td>
                  <td class="mono">
                    {{ artifactLeadershipDisagreement(entry)
                    }}<template
                      v-if="
                        entry.leaderSummary
                          ?.robustLeaderReachedSealedFinalist != null
                      "
                    >
                      ·
                      {{ artifactRobustLeaderFinalistDetail(entry) }}</template
                    >
                  </td>
                  <td class="mono">
                    {{ entry.validatedTupleCount ?? 0 }} /
                    {{ entry.stressFragileTupleCount ?? 0 }}
                  </td>
                  <td class="mono">
                    {{
                      formatPercent(artifactValidationSupportRatio(entry), 1)
                    }}
                  </td>
                  <td class="mono">
                    {{ formatPercent(artifactStressFragilityRatio(entry), 1) }}
                  </td>
                  <td class="mono">
                    {{
                      describeArtifactProvenanceDiff(
                        entry,
                        selectedArtifactSummary,
                      )
                    }}
                  </td>
                  <td class="mono">
                    {{
                      formatOptionalDecimal(artifactValidationDelta(entry), 3)
                    }}
                  </td>
                  <td class="mono">
                    {{ formatOptionalDecimal(artifactStressDrop(entry), 3) }}
                  </td>
                  <td class="mono">{{ entry.bestRobustScore.toFixed(3) }}</td>
                  <td class="mono">
                    {{ entry.bestSelectionExpectancyPerPremium.toFixed(3) }}
                  </td>
                  <td class="mono">
                    {{
                      entry.bestHoldoutExpectancyPerPremium != null
                        ? entry.bestHoldoutExpectancyPerPremium.toFixed(3)
                        : "—"
                    }}
                  </td>
                  <td class="mono">
                    {{
                      entry.bestSealedExpectancyPerPremium != null
                        ? entry.bestSealedExpectancyPerPremium.toFixed(3)
                        : "—"
                    }}
                  </td>
                  <td class="mono">
                    {{
                      entry.bestStressExpectancyPerPremium != null
                        ? entry.bestStressExpectancyPerPremium.toFixed(3)
                        : "—"
                    }}
                  </td>
                  <td class="mono">
                    {{
                      entry.bestHarsherStressExpectancyPerPremium != null
                        ? entry.bestHarsherStressExpectancyPerPremium.toFixed(3)
                        : "—"
                    }}
                  </td>
                  <td class="mono">
                    {{
                      entry.bestExtremeStressExpectancyPerPremium != null
                        ? entry.bestExtremeStressExpectancyPerPremium.toFixed(3)
                        : "—"
                    }}
                  </td>
                  <td class="mono">
                    {{ entry.sealedEvaluatedTupleCount ?? 0 }}/{{
                      entry.sealedFinalistBudget ?? 0
                    }}
                  </td>
                  <td>
                    <button
                      type="button"
                      @click="loadArtifactFromComparison(entry.path)"
                    >
                      Load
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </details>

      <details
        v-if="artifactFrontierLeaderRows.length > 0"
        class="panelDetails"
      >
        <summary>Artifact frontier leaders</summary>
        <div class="panelDetailsBody tableWrap">
          <table class="resultTable">
            <thead>
              <tr>
                <th>Stage</th>
                <th>Artifact</th>
                <th>Metric</th>
                <th>Validation</th>
                <th>Val Δ</th>
                <th>Stress Δ</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in artifactFrontierLeaderRows" :key="row.key">
                <td class="mono">{{ row.label }}</td>
                <td class="mono">{{ row.artifactLabel }}</td>
                <td class="mono">
                  {{ row.metricLabel }} ·
                  {{ formatOptionalDecimal(row.value, 3) }}
                </td>
                <td class="mono">{{ row.validation }}</td>
                <td class="mono">{{ formatOptionalDecimal(row.drift, 3) }}</td>
                <td class="mono">
                  {{ formatOptionalDecimal(row.stressDrop, 3) }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </details>

      <details v-if="artifactMetaEntries.length > 0" class="panelDetails">
        <summary>Options artifact meta-analysis</summary>
        <div class="panelDetailsBody">
          <div class="panelMuted">
            Cross-artifact summary across all saved options-expression artifacts
            currently indexed in the app data store.
          </div>
          <div class="detailGrid detailGridTight">
            <div v-for="row in artifactMetaOverviewRows" :key="row.key">
              <span class="panelMetricKey">{{ row.label }}</span>
              <span class="panelMetricValue mono panelNumeric">{{
                row.value
              }}</span>
            </div>
          </div>
          <div
            v-for="section in artifactMetaSections"
            :key="section.key"
            class="coverageSection"
          >
            <div class="panelMuted">{{ section.label }}</div>
            <div class="tableWrap">
              <table class="resultTable">
                <thead>
                  <tr>
                    <th>Group</th>
                    <th>Artifacts</th>
                    <th>Validated</th>
                    <th>Fragile</th>
                    <th>Avg robust</th>
                    <th>Avg sel EV</th>
                    <th>Avg validated EV</th>
                    <th>Avg stress drop</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="row in section.rows" :key="row.key">
                    <td class="mono">{{ row.label }}</td>
                    <td class="mono">{{ row.artifactCount }}</td>
                    <td class="mono">{{ row.validatedCount }}</td>
                    <td class="mono">{{ row.fragileCount }}</td>
                    <td class="mono">
                      {{ formatOptionalDecimal(row.avgRobust, 3) }}
                    </td>
                    <td class="mono">
                      {{ formatOptionalDecimal(row.avgSelectionEv, 3) }}
                    </td>
                    <td class="mono">
                      {{ formatOptionalDecimal(row.avgValidationEv, 3) }}
                    </td>
                    <td class="mono">
                      {{ formatOptionalDecimal(row.avgStressDrop, 3) }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
          <div
            v-if="artifactLaneDisagreementRows.length > 0"
            class="coverageSection"
          >
            <div class="panelMuted">Realism lane disagreement</div>
            <div class="tableWrap">
              <table class="resultTable">
                <thead>
                  <tr>
                    <th>Group</th>
                    <th>Lanes</th>
                    <th>Selection winner</th>
                    <th>Validation winner</th>
                    <th>Drift</th>
                    <th>Validation spread</th>
                    <th>Lane summary</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="row in artifactLaneDisagreementRows"
                    :key="row.key"
                  >
                    <td class="mono">{{ row.label }}</td>
                    <td class="mono">{{ row.laneCount }}</td>
                    <td class="mono">{{ row.selectionWinnerLane }}</td>
                    <td class="mono">{{ row.validationWinnerLane }}</td>
                    <td class="mono">{{ row.driftLabel }}</td>
                    <td class="mono">
                      {{ formatOptionalDecimal(row.validationSpread, 3) }}
                    </td>
                    <td class="mono">{{ row.laneSummary }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </details>

      <section class="resultCard" data-qa-surface="options-validation">
        <div class="panelSectionHeader">
          <div class="panelHeaderVisual">
            <AppVisualIcon
              :icon="optionsCurrentResultIcon"
              class="sectionGlyph"
            />
          </div>
          <div>
            <h3>Validate</h3>
            <p>
              Subsequent validation replays selected tuples from a saved options
              artifact against explicit Polygon.io dataset provenance.
            </p>
          </div>
        </div>

        <div class="formGrid compactGrid">
          <label>
            Dataset policy
            <select
              v-model="validationDatasetPolicy"
              data-qa-control="options.validation.datasetPolicy"
            >
              <option
                v-for="entry in OPTIONS_VALIDATION_DATASET_POLICY_OPTIONS"
                :key="entry"
                :value="entry"
              >
                {{ formatValidationDatasetPolicy(entry) }}
              </option>
            </select>
          </label>
          <label>
            Validation plan
            <select
              v-model="validationPlanMode"
              data-qa-control="options.validation.planMode"
            >
              <option
                v-for="entry in OPTIONS_VALIDATION_PLAN_MODE_OPTIONS"
                :key="entry"
                :value="entry"
              >
                {{ formatValidationPlanMode(entry) }}
              </option>
            </select>
          </label>
          <label v-if="validationPlanMode === 'date_range_holdout'">
            Holdout start
            <input
              v-model="validationStartDateInput"
              data-qa-control="options.validation.startDate"
              type="date"
            />
          </label>
          <label v-if="validationPlanMode === 'date_range_holdout'">
            Holdout end
            <input
              v-model="validationEndDateInput"
              data-qa-control="options.validation.endDate"
              type="date"
            />
          </label>
          <label
            v-if="
              validationPlanMode === 'anchored_walk_forward' ||
              validationPlanMode === 'rolling_session_walk_forward'
            "
          >
            Train sessions
            <input
              v-model="validationTrainSessionCountInput"
              data-qa-control="options.validation.trainSessions"
              type="number"
              min="1"
              step="1"
            />
          </label>
          <label
            v-if="
              validationPlanMode === 'anchored_walk_forward' ||
              validationPlanMode === 'rolling_session_walk_forward'
            "
          >
            Validation sessions
            <input
              v-model="validationSessionCountInput"
              data-qa-control="options.validation.validationSessions"
              type="number"
              min="1"
              step="1"
            />
          </label>
          <label
            v-if="
              validationPlanMode === 'anchored_walk_forward' ||
              validationPlanMode === 'rolling_session_walk_forward'
            "
          >
            Step sessions
            <input
              v-model="validationStepSessionCountInput"
              data-qa-control="options.validation.stepSessions"
              type="number"
              min="1"
              step="1"
            />
          </label>
          <label>
            Tuple indexes
            <input
              v-model="validationTupleIndexesInput"
              type="text"
              inputmode="numeric"
              spellcheck="false"
              placeholder="Optional, comma-separated"
            />
          </label>
          <div class="full">
            <div class="moneynessList">
              <label class="bucketToggle">
                <input
                  v-model="validationIncludeSealedFinalists"
                  type="checkbox"
                />
                <span>Sealed finalists</span>
              </label>
              <label class="bucketToggle">
                <input
                  v-model="validationIncludeRobustLeader"
                  type="checkbox"
                />
                <span>Robust leader</span>
              </label>
              <label class="bucketToggle">
                <input
                  v-model="validationIncludeHoldoutLeader"
                  type="checkbox"
                />
                <span>Holdout leader</span>
              </label>
            </div>
          </div>
        </div>

        <div
          v-if="validationFormErrorMessages.length"
          class="panelMuted validationFormErrors"
          role="status"
          aria-live="polite"
        >
          {{ validationFormErrorMessages.join(" ") }}
        </div>

        <div class="actions">
          <button
            type="button"
            data-qa-action="options.validation.preview"
            :disabled="
              validationFeasibilityBusy ||
              !activeArtifactPath ||
              !validationFormValid
            "
            @click="store.previewValidationFeasibility()"
          >
            {{
              validationFeasibilityBusy
                ? "Previewing validation…"
                : "Preview validation"
            }}
          </button>
          <button
            type="button"
            data-qa-action="options.validation.run"
            :disabled="
              validationBusy || !activeArtifactPath || !validationFormValid
            "
            @click="store.validateActiveArtifact()"
          >
            {{ validationBusy ? "Validating…" : "Validate artifact" }}
          </button>
          <button
            type="button"
            data-qa-action="options.validation.reveal"
            :disabled="!latestValidation?.artifactPath"
            @click="store.revealLatestValidationArtifact()"
          >
            Reveal validation
          </button>
        </div>

        <div v-if="latestValidationFeasibility" class="panelMuted mono">
          {{ latestValidationFeasibility.selectedTupleCount }} selected tuple{{
            latestValidationFeasibility.selectedTupleCount === 1 ? "" : "s"
          }}
          · {{ latestValidationFeasibility.datasetPolicy }} ·
          {{ latestValidationFeasibility.optionsDatasetId }} ·
          {{ latestValidationFeasibility.priceSemantics }} ·
          {{
            formatValidationPlanMode(
              latestValidationFeasibility.validationPlan.mode,
            )
          }}
          <template
            v-if="
              formatValidationPlanDetail(
                latestValidationFeasibility.validationPlan,
              )
            "
          >
            ·
            {{
              formatValidationPlanDetail(
                latestValidationFeasibility.validationPlan,
              )
            }}
          </template>
        </div>
        <div
          v-if="latestValidationFeasibility?.notes.length"
          class="panelMuted"
        >
          {{ latestValidationFeasibility.notes.join(" ") }}
        </div>

        <div v-if="validationAcceptance" class="gateBadgeRow">
          <span
            class="gateBadge"
            :class="{
              isPass: validationAcceptance.verdict === 'accepted',
              isPending: validationAcceptance.verdict === 'support_deficient',
              isMixed:
                validationAcceptance.verdict === 'warning' ||
                validationAcceptance.verdict === 'rejected',
            }"
          >
            Subsequent validation ·
            {{ validationAcceptance.verdict.replace(/_/g, " ") }}
          </span>
          <span v-if="latestValidation" class="gateBadge isPending">
            {{
              formatValidationPlanMode(
                latestValidation.result.validationPlan.mode,
              )
            }}
            <template
              v-if="
                formatValidationPlanDetail(
                  latestValidation.result.validationPlan,
                )
              "
            >
              ·
              {{
                formatValidationPlanDetail(
                  latestValidation.result.validationPlan,
                )
              }}
            </template>
          </span>
        </div>
        <div v-if="validationAcceptance" class="detailGrid detailGridTight">
          <div>
            <span class="panelMetricKey">Accepted</span>
            <span class="panelMetricValue mono panelNumeric">
              {{ validationAcceptance.acceptedTupleCount }} /
              {{ validationAcceptance.validatedTupleCount }}
            </span>
          </div>
          <div>
            <span class="panelMetricKey">Support deficient</span>
            <span class="panelMetricValue mono panelNumeric">
              {{ validationAcceptance.supportDeficientTupleCount }}
            </span>
          </div>
          <div>
            <span class="panelMetricKey">Stress fragile</span>
            <span class="panelMetricValue mono panelNumeric">
              {{ validationAcceptance.stressFragileTupleCount }}
            </span>
          </div>
          <div>
            <span class="panelMetricKey">Best validation EV</span>
            <span class="panelMetricValue mono panelNumeric">
              {{
                formatOptionalDecimal(
                  validationAcceptance.bestValidatedExpectancyPerPremium,
                  3,
                )
              }}
            </span>
          </div>
        </div>
        <div v-if="validationAcceptance?.notes.length" class="panelMuted">
          {{ validationAcceptance.notes.join(" ") }}
        </div>
        <div
          v-if="latestValidation?.result.datasetProvenance?.quoteQuality"
          class="detailGrid detailGridTight"
        >
          <div>
            <span class="panelMetricKey">Quote rows</span>
            <span class="panelMetricValue mono panelNumeric">
              {{
                latestValidation.result.datasetProvenance.quoteQuality
                  .anyQuoteRowCount
              }}
              /
              {{
                latestValidation.result.datasetProvenance.quoteQuality.rowCount
              }}
            </span>
          </div>
          <div>
            <span class="panelMetricKey">Bid/ask coverage</span>
            <span class="panelMetricValue mono panelNumeric">
              {{
                formatPercent(
                  latestValidation.result.datasetProvenance.quoteQuality
                    .bidAskCoverageRate,
                  1,
                )
              }}
            </span>
          </div>
          <div>
            <span class="panelMetricKey">Median spread</span>
            <span class="panelMetricValue mono panelNumeric">
              {{
                formatOptionalDecimal(
                  latestValidation.result.datasetProvenance.quoteQuality
                    .medianSpreadBps,
                  1,
                )
              }}
              bps
            </span>
          </div>
          <div>
            <span class="panelMetricKey">Stale quotes</span>
            <span class="panelMetricValue mono panelNumeric">
              {{
                formatPercent(
                  latestValidation.result.datasetProvenance.quoteQuality
                    .staleQuoteRate,
                  1,
                )
              }}
            </span>
          </div>
          <div>
            <span class="panelMetricKey">Quote warnings</span>
            <span class="panelMetricValue mono panelNumeric">
              {{
                latestValidation.result.datasetProvenance.quoteQuality.warnings
                  .length
              }}
            </span>
          </div>
          <div>
            <span class="panelMetricKey">Invalid bid/ask</span>
            <span class="panelMetricValue mono panelNumeric">
              {{
                latestValidation.result.datasetProvenance.quoteQuality
                  .invalidBidAskRowCount
              }}
            </span>
          </div>
        </div>
        <div
          v-if="
            latestValidation?.result.datasetProvenance?.quoteQuality?.warnings
              .length
          "
          class="panelMuted"
        >
          {{
            latestValidation.result.datasetProvenance.quoteQuality.warnings.join(
              " ",
            )
          }}
        </div>

        <div
          v-if="latestValidation?.result.laneSummaries.length"
          class="tableWrap"
        >
          <table class="resultTable">
            <thead>
              <tr>
                <th>Lane</th>
                <th>Supported</th>
                <th>Best EV</th>
                <th>Worst EV</th>
                <th>Median EV</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="row in latestValidation.result.laneSummaries"
                :key="row.lane"
              >
                <td class="mono">{{ row.lane }}</td>
                <td class="mono">
                  {{ row.supportedTupleCount }} / {{ row.tupleCount }}
                </td>
                <td class="mono">
                  {{ formatOptionalDecimal(row.bestExpectancyPerPremium, 3) }}
                </td>
                <td class="mono">
                  {{ formatOptionalDecimal(row.worstExpectancyPerPremium, 3) }}
                </td>
                <td class="mono">
                  {{ formatOptionalDecimal(row.medianExpectancyPerPremium, 3) }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div
          v-if="optionsFinalistDashboard"
          class="optionsFinalistDashboard"
          data-qa-panel="options.finalistDashboard"
        >
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Finalist acceptance</div>
              <div class="panelSectionLead">
                {{ optionsFinalistDashboard.acceptedTupleCount }} /
                {{ optionsFinalistDashboard.validatedTupleCount }} accepted ·
                {{ optionsFinalistDashboard.verdict }}
              </div>
            </div>
          </div>
          <div class="gateBadgeRow finalistGateGrid">
            <span
              v-for="gate in optionsFinalistDashboard.gates"
              :key="gate.key"
              class="gateBadge finalistGate"
              :class="{
                isPass: gate.tone === 'pass',
                isPending: gate.tone === 'pending',
                isMixed: gate.tone === 'mixed',
              }"
              :title="gate.detail"
            >
              {{ gate.label }} · {{ gate.status }}
            </span>
          </div>
          <div v-if="optionsFinalistDashboard.rows.length" class="tableWrap">
            <table class="resultTable">
              <thead>
                <tr>
                  <th>Tuple</th>
                  <th>Verdict</th>
                  <th>Support</th>
                  <th>Robust</th>
                  <th>Search EV</th>
                  <th>Holdout EV</th>
                  <th>Stress EV</th>
                  <th>Harsher EV</th>
                  <th>Extreme EV</th>
                  <th>Strongest stress</th>
                  <th>Trades</th>
                  <th>Drift</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in optionsFinalistDashboard.rows" :key="row.key">
                  <td class="mono">{{ row.tupleLabel }}</td>
                  <td>
                    <span
                      class="gateBadge miniBadge"
                      :class="{
                        isPass: row.tone === 'pass',
                        isPending: row.tone === 'pending',
                        isMixed: row.tone === 'mixed',
                      }"
                    >
                      {{ row.verdict }}
                    </span>
                  </td>
                  <td class="mono">{{ row.support }}</td>
                  <td class="mono">
                    {{ formatOptionsFinalistNumber(row.validatedRobustScore) }}
                  </td>
                  <td class="mono">
                    {{ formatOptionsFinalistNumber(row.searchEv) }}
                  </td>
                  <td class="mono">
                    {{ formatOptionsFinalistNumber(row.holdoutEv) }}
                  </td>
                  <td class="mono">
                    {{ formatOptionsFinalistNumber(row.stressEv) }}
                  </td>
                  <td class="mono">
                    {{ formatOptionsFinalistNumber(row.harsherStressEv) }}
                  </td>
                  <td class="mono">
                    {{ formatOptionsFinalistNumber(row.extremeStressEv) }}
                  </td>
                  <td class="mono">
                    {{ row.strongestStressLabel }} ·
                    {{ formatOptionsFinalistNumber(row.strongestStressEv) }}
                  </td>
                  <td class="mono">
                    {{ formatOptionsFinalistNumber(row.validTrades, 0) }}
                  </td>
                  <td class="mono">{{ row.drift || "—" }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-else class="panelMuted">
            No tuple-level validation rows were included in this artifact.
          </div>
        </div>
      </section>

      <section v-if="latestFeasibility" class="resultCard">
        <div class="panelSectionHeader">
          <div class="panelHeaderVisual">
            <AppVisualIcon
              :icon="optionsFeasibilityPreviewIcon"
              class="sectionGlyph"
            />
            <div>
              <div class="panelEyebrow">Phase 0.5 preview</div>
              <div class="panelSectionLead">
                Review candidate density, event coverage, and replay/compile
                timing before committing to a full optimization run or GPU work.
              </div>
            </div>
          </div>
        </div>
        <div class="gateBadgeRow">
          <span
            class="gateBadge"
            :class="{
              isPass: feasibilityOverallStatus === 'pass',
              isPending: feasibilityOverallStatus === 'pending',
              isMixed: feasibilityOverallStatus === 'mixed',
            }"
          >
            GPU viability
            {{
              feasibilityOverallStatus === "pass"
                ? "pass"
                : feasibilityOverallStatus === "pending"
                  ? "pending"
                  : "mixed"
            }}
          </span>
          <span
            v-for="row in feasibilityGateRows"
            :key="row.key"
            class="gateBadge"
            :class="{
              isPass: row.passed,
              isPending: row.pending && !row.passed,
              isMixed: !row.passed && !row.pending,
            }"
          >
            {{ row.label }} · {{ row.value }}
          </span>
        </div>
        <div v-if="feasibilityCoverageAdequacy" class="gateBadgeRow">
          <span
            class="gateBadge"
            :class="{
              isPass: feasibilityCoverageAdequacy.tone === 'pass',
              isPending: feasibilityCoverageAdequacy.tone === 'pending',
              isMixed: feasibilityCoverageAdequacy.tone === 'mixed',
            }"
          >
            Coverage {{ feasibilityCoverageAdequacy.label }}
          </span>
        </div>
        <div v-if="feasibilityCoverageAdequacy" class="panelMuted">
          {{ feasibilityCoverageAdequacy.detail }}
        </div>
        <div class="detailGrid detailGridTight">
          <div v-for="row in feasibilitySummaryRows" :key="row.key">
            <span class="panelMetricKey">{{ row.label }}</span>
            <span class="panelMetricValue mono panelNumeric">{{
              row.value
            }}</span>
          </div>
        </div>
        <details v-if="feasibilityOverlapRows.length > 0" class="panelDetails">
          <summary>Overlap adequacy</summary>
          <div class="panelDetailsBody">
            <div class="detailGrid detailGridTight">
              <div v-for="row in feasibilityOverlapRows" :key="row.key">
                <span class="panelMetricKey">{{ row.label }}</span>
                <span class="panelMetricValue mono panelNumeric">{{
                  row.value
                }}</span>
              </div>
            </div>
          </div>
        </details>
        <details
          v-if="feasibilitySplitInvalidSections.length > 0"
          class="panelDetails"
        >
          <summary>Split invalid reasons</summary>
          <div class="panelDetailsBody">
            <div
              v-for="section in feasibilitySplitInvalidSections"
              :key="section.key"
              class="coverageSection"
            >
              <div class="panelMuted">{{ section.label }}</div>
              <div class="tableWrap">
                <table class="resultTable">
                  <thead>
                    <tr>
                      <th>Reason</th>
                      <th>Count</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr
                      v-for="row in section.rows"
                      :key="`${section.key}-${row.reason}`"
                    >
                      <td class="mono">{{ row.reason }}</td>
                      <td class="mono">{{ row.count }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </details>
        <details class="panelDetails">
          <summary>Lane diagnostics</summary>
          <div class="panelDetailsBody">
            <div class="detailGrid detailGridTight">
              <div>
                <span class="panelMetricKey">Structural cache</span>
                <span class="panelMetricValue mono panelNumeric">
                  {{
                    formatBytes(latestFeasibility.structuralCacheBytesEstimate)
                  }}
                </span>
              </div>
            </div>
            <div class="tableWrap">
              <table class="resultTable">
                <thead>
                  <tr>
                    <th>Lane</th>
                    <th>Compiled / Total summaries</th>
                    <th>Density</th>
                    <th>Compile time</th>
                    <th>Bytes</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="row in feasibilityLaneRows" :key="row.key">
                    <td class="mono">{{ row.label }}</td>
                    <td class="mono">{{ row.summaries }}</td>
                    <td class="mono">{{ row.density }}</td>
                    <td class="mono">{{ row.compileMs }}</td>
                    <td class="mono">{{ row.bytes }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </details>
        <details
          v-if="feasibilityInvalidReasonRows.length > 0"
          class="panelDetails"
        >
          <summary>Invalid reasons</summary>
          <div class="panelDetailsBody tableWrap">
            <table class="resultTable">
              <thead>
                <tr>
                  <th>Reason</th>
                  <th>Count</th>
                  <th>Share</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="row in feasibilityInvalidReasonRows"
                  :key="row.reason"
                >
                  <td class="mono">{{ row.reason }}</td>
                  <td class="mono">{{ row.count }}</td>
                  <td class="mono">{{ formatPercent(row.share) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </details>
        <details
          v-if="feasibilityAxisCoverageSections.length > 0"
          class="panelDetails"
        >
          <summary>Bucket coverage</summary>
          <div class="panelDetailsBody">
            <div
              v-for="section in feasibilityAxisCoverageSections"
              :key="section.key"
              class="coverageSection"
            >
              <div class="panelMuted">{{ section.label }}</div>
              <div class="tableWrap">
                <table class="resultTable">
                  <thead>
                    <tr>
                      <th>{{ section.bucketLabel }}</th>
                      <th>Valid / Total slots</th>
                      <th>Valid rate</th>
                      <th>Top invalid reasons</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="row in section.rows" :key="row.key">
                      <td class="mono">{{ row.bucket }}</td>
                      <td class="mono">{{ row.validSlots }}</td>
                      <td class="mono">{{ row.validRate }}</td>
                      <td class="mono">{{ row.invalidReasons }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </details>
        <div v-if="feasibilityInvalidReasons" class="panelMuted mono">
          Invalid reasons: {{ feasibilityInvalidReasons }}
        </div>
        <details
          v-if="feasibilityRecommendations.length > 0"
          class="panelDetails"
        >
          <summary>Recommendations</summary>
          <div class="panelDetailsBody">
            <ul class="recommendationList">
              <li
                v-for="recommendation in feasibilityRecommendations"
                :key="recommendation"
              >
                {{ recommendation }}
              </li>
            </ul>
          </div>
        </details>
      </section>

      <section v-if="activeDiagnostics" class="resultCard">
        <div class="panelSectionHeader">
          <div class="panelHeaderVisual">
            <AppVisualIcon
              :icon="optionsCurrentResultIcon"
              class="sectionGlyph"
            />
            <div>
              <div class="panelEyebrow">Current result</div>
              <div class="panelSectionLead">
                Review the current top tuples before deciding whether this
                expression family is worth promoting into a wider options
                workflow.
              </div>
            </div>
          </div>
        </div>
        <div class="detailGrid">
          <div>
            <span class="panelMetricKey">Root</span>
            <span class="panelMetricValue mono panelNumeric">{{
              activeDiagnostics.rootSymbol
            }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Dataset</span>
            <span class="panelMetricValue mono panelNumeric">{{
              activeDiagnostics.optionsDatasetId
            }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Events</span>
            <span class="panelMetricValue mono panelNumeric">{{
              activeDiagnostics.totalEvents
            }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Search / Holdout / Sealed</span>
            <span class="panelMetricValue mono panelNumeric">
              {{ activeDiagnostics.searchEvents }} /
              {{ activeDiagnostics.holdoutEvents }} /
              {{ activeDiagnostics.sealedTestEvents }}
            </span>
          </div>
          <div>
            <span class="panelMetricKey">Contracts</span>
            <span class="panelMetricValue mono panelNumeric">{{
              activeDiagnostics.datasetContracts
            }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Rows</span>
            <span class="panelMetricValue mono panelNumeric">{{
              activeDiagnostics.datasetRows
            }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Realism lane</span>
            <span class="panelMetricValue mono panelNumeric">{{
              activeRealismLane.label
            }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Backend route</span>
            <span class="panelMetricValue mono panelNumeric">{{
              activeBackendRoute.label
            }}</span>
          </div>
        </div>
        <div class="panelMuted">{{ activeRealismLane.detail }}</div>
        <div class="panelMuted">{{ activeBackendRoute.detail }}</div>

        <div class="detailGrid detailGridTight">
          <div v-for="card in reportingMetricCards" :key="card.key">
            <span class="panelMetricKey">{{ card.label }}</span>
            <span class="panelMetricValue mono panelNumeric">{{
              card.value
            }}</span>
          </div>
        </div>

        <div v-if="selectedTupleValidation" class="gateBadgeRow">
          <span
            class="gateBadge"
            :class="{
              isPass: selectedTupleValidation.tone === 'pass',
              isPending: selectedTupleValidation.tone === 'pending',
              isMixed: selectedTupleValidation.tone === 'mixed',
            }"
          >
            Selected tuple validation · {{ selectedTupleValidation.label }}
          </span>
        </div>
        <div v-if="selectedTupleValidation" class="panelMuted">
          {{ selectedTupleValidation.detail }}
        </div>
        <div v-if="selectedTupleSupportRows.length > 0" class="panelMuted mono">
          Holdout support {{ selectedTupleSupportRows[0]?.value }} · Sealed
          support {{ selectedTupleSupportRows[1]?.value }} · Strongest stress
          support {{ selectedTupleSupportRows[2]?.value }}
        </div>
        <div v-if="winnerDisagreementSummary" class="gateBadgeRow">
          <span
            class="gateBadge"
            :class="{
              isPass: winnerDisagreementSummary.tone === 'pass',
              isPending: winnerDisagreementSummary.tone === 'pending',
              isMixed: winnerDisagreementSummary.tone === 'mixed',
            }"
          >
            Winner alignment · {{ winnerDisagreementSummary.label }}
          </span>
        </div>
        <div v-if="winnerDisagreementSummary" class="panelMuted">
          {{ winnerDisagreementSummary.detail }}
        </div>

        <details
          v-if="winnerDisagreementRows.length > 0"
          class="panelDetails"
          open
        >
          <summary>Winner disagreement</summary>
          <div class="panelDetailsBody tableWrap">
            <table class="resultTable">
              <thead>
                <tr>
                  <th>Stage</th>
                  <th>Tuple</th>
                  <th>Shape</th>
                  <th>Metric</th>
                  <th>Relation</th>
                  <th>Validation</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in winnerDisagreementRows" :key="row.key">
                  <td class="mono">{{ row.label }}</td>
                  <td class="mono">#{{ row.tupleIndex + 1 }}</td>
                  <td class="mono">{{ row.tupleIdentity }}</td>
                  <td class="mono">
                    {{ row.metricLabel }} ·
                    {{ formatOptionalDecimal(row.value, 4) }}
                  </td>
                  <td class="mono">{{ row.relation }}</td>
                  <td class="mono">{{ row.validation }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </details>

        <details
          v-if="validationWinnersRows.length > 0"
          class="panelDetails"
          open
        >
          <summary>Validation winners</summary>
          <div class="panelDetailsBody tableWrap">
            <table class="resultTable">
              <thead>
                <tr>
                  <th>Stage</th>
                  <th>Tuple</th>
                  <th>Structure</th>
                  <th>Metric</th>
                  <th>Validation</th>
                  <th>Stress drop</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in validationWinnersRows" :key="row.key">
                  <td class="mono">{{ row.label }}</td>
                  <td class="mono">#{{ row.tupleIndex + 1 }}</td>
                  <td class="mono">
                    {{
                      row.tuple.structureFamily ||
                      activeRequest?.structureFamily ||
                      "long_single"
                    }}
                  </td>
                  <td class="mono">
                    {{ row.metricLabel }} ·
                    {{ formatOptionalDecimal(row.value, 4) }}
                  </td>
                  <td class="mono">{{ row.validation }}</td>
                  <td class="mono">
                    {{
                      row.stressDrop == null ? "—" : row.stressDrop.toFixed(4)
                    }}
                  </td>
                  <td>
                    <button
                      type="button"
                      @click="focusTupleIndex(row.tupleIndex)"
                    >
                      Focus
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </details>

        <details v-if="requestSummaryRows.length > 0" class="panelDetails">
          <summary>Run provenance</summary>
          <div class="panelDetailsBody">
            <div class="detailGrid detailGridTight">
              <div v-for="row in requestSummaryRows" :key="row.key">
                <span class="panelMetricKey">{{ row.label }}</span>
                <span class="panelMetricValue mono panelNumeric">{{
                  row.value
                }}</span>
              </div>
            </div>
            <div
              v-if="activeDatasetQuoteQuality"
              class="detailGrid detailGridTight"
            >
              <div>
                <span class="panelMetricKey">Quote rows</span>
                <span class="panelMetricValue mono panelNumeric">
                  {{ activeDatasetQuoteQuality.anyQuoteRowCount }} /
                  {{ activeDatasetQuoteQuality.rowCount }}
                </span>
              </div>
              <div>
                <span class="panelMetricKey">Bid/ask coverage</span>
                <span class="panelMetricValue mono panelNumeric">
                  {{
                    formatPercent(
                      activeDatasetQuoteQuality.bidAskCoverageRate,
                      1,
                    )
                  }}
                </span>
              </div>
              <div>
                <span class="panelMetricKey">Median spread</span>
                <span class="panelMetricValue mono panelNumeric">
                  {{
                    formatOptionalDecimal(
                      activeDatasetQuoteQuality.medianSpreadBps,
                      1,
                    )
                  }}
                  bps
                </span>
              </div>
              <div>
                <span class="panelMetricKey">Stale quotes</span>
                <span class="panelMetricValue mono panelNumeric">
                  {{
                    formatPercent(activeDatasetQuoteQuality.staleQuoteRate, 1)
                  }}
                </span>
              </div>
              <div>
                <span class="panelMetricKey">Quote warnings</span>
                <span class="panelMetricValue mono panelNumeric">
                  {{ activeDatasetQuoteQuality.warnings.length }}
                </span>
              </div>
              <div>
                <span class="panelMetricKey">Invalid bid/ask</span>
                <span class="panelMetricValue mono panelNumeric">
                  {{ activeDatasetQuoteQuality.invalidBidAskRowCount }}
                </span>
              </div>
            </div>
            <div v-if="activeDatasetQuoteQualityWarnings" class="panelMuted">
              {{ activeDatasetQuoteQualityWarnings }}
            </div>
          </div>
        </details>

        <div v-if="activeRows.length > 0" class="resultTableControls">
          <label>
            Sort by
            <select v-model="resultSortKey">
              <option value="robust">robust</option>
              <option value="selection_ev">selection EV</option>
              <option value="holdout_ev">holdout EV</option>
              <option value="stress_ev">stress EV</option>
              <option value="harsher_ev">harsher EV</option>
              <option value="extreme_ev">extreme EV</option>
              <option value="invalid_rate">invalid rate</option>
              <option value="valid_trades">valid trades</option>
              <option value="sealed_rank">sealed rank</option>
            </select>
          </label>
          <label>
            Direction
            <select v-model="resultSortDirection">
              <option value="desc">desc</option>
              <option value="asc">asc</option>
            </select>
          </label>
          <label class="toggleField">
            <input v-model="finalistsOnly" type="checkbox" />
            <span>Finalists only</span>
          </label>
          <label>
            Max invalid rate
            <input
              v-model="maxInvalidRateFilterInput"
              type="text"
              inputmode="decimal"
              spellcheck="false"
              placeholder="0.35"
            />
          </label>
          <label>
            Min robust
            <input
              v-model="minRobustScoreFilterInput"
              type="text"
              inputmode="decimal"
              spellcheck="false"
              placeholder="0.20"
            />
          </label>
          <label>
            Max stress drop
            <input
              v-model="maxStressDropFilterInput"
              type="text"
              inputmode="decimal"
              spellcheck="false"
              placeholder="0.10"
            />
          </label>
        </div>
        <div v-if="activeRows.length > 0" class="warningText">
          Ranked tuples remain search-stage candidates unless the validation
          verdict below shows supported holdout or sealed trades.
        </div>

        <div
          v-if="activeRows.length > 0 && displayedRowsWithIndex.length > 0"
          class="tableWrap"
        >
          <table class="resultTable">
            <thead>
              <tr>
                <th>DTE</th>
                <th>Moneyness</th>
                <th>Delay</th>
                <th>Hold</th>
                <th>Target / Stop</th>
                <th>Robust</th>
                <th>Validation</th>
                <th>Stress drop</th>
                <th>EV / Premium</th>
                <th>Invalid %</th>
                <th v-if="hasHoldoutMetrics">Holdout EV / Premium</th>
                <th v-if="hasSealedMetrics">Finalist</th>
                <th v-if="hasSealedMetrics">Sealed EV / Premium</th>
                <th v-if="hasStressMetrics">Stress EV / Premium</th>
                <th v-if="hasSealedStressMetrics">
                  Sealed stress EV / Premium
                </th>
                <th v-if="hasHarsherStressMetrics">Harsher EV / Premium</th>
                <th v-if="hasSealedHarsherStressMetrics">
                  Sealed harsher EV / Premium
                </th>
                <th v-if="hasExtremeStressMetrics">Extreme EV / Premium</th>
                <th v-if="hasSealedExtremeStressMetrics">
                  Sealed extreme EV / Premium
                </th>
                <th>Win</th>
                <th>Trades</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="{ row, originalIndex } in displayedRowsWithIndex"
                :key="`${originalIndex}-${row.dteDays}-${row.moneynessBucket}-${row.delayMinutes}-${row.holdMinutes}-${row.targetReturn}-${row.stopReturn}`"
                :class="{ isSelected: originalIndex === selectedTupleIndex }"
                tabindex="0"
                :aria-selected="
                  originalIndex === selectedTupleIndex ? 'true' : 'false'
                "
                :aria-label="tupleRowLabel(row, originalIndex)"
                @click="selectTuple(originalIndex)"
                @keydown="onTupleRowKeydown($event, originalIndex)"
              >
                <td class="mono">{{ row.dteDays }}</td>
                <td class="mono">{{ row.moneynessBucket }}</td>
                <td class="mono">{{ row.delayMinutes }}</td>
                <td class="mono">{{ row.holdMinutes }}</td>
                <td class="mono">
                  {{ row.targetReturn == null ? "—" : row.targetReturn }} /
                  {{ row.stopReturn == null ? "—" : row.stopReturn }}
                </td>
                <td class="mono">{{ row.robustScore.toFixed(3) }}</td>
                <td class="mono">
                  {{ describeTupleValidation(row)?.label ?? "Search only" }}
                </td>
                <td class="mono">
                  {{
                    strongestTupleStressDrop(row) == null
                      ? "—"
                      : strongestTupleStressDrop(row)?.toFixed(4)
                  }}
                </td>
                <td class="mono">
                  {{ row.selectionMetrics.expectancyPerPremium.toFixed(4) }}
                </td>
                <td class="mono">
                  {{ row.selectionMetrics.invalidRate.toFixed(3) }}
                </td>
                <td v-if="hasHoldoutMetrics" class="mono">
                  {{
                    row.holdoutMetrics
                      ? row.holdoutMetrics.expectancyPerPremium.toFixed(4)
                      : "—"
                  }}
                </td>
                <td v-if="hasSealedMetrics" class="mono">
                  {{
                    row.sealedFinalist
                      ? `#${row.sealedFinalRank ?? row.preSealedRank ?? 0}`
                      : "—"
                  }}
                </td>
                <td v-if="hasSealedMetrics" class="mono">
                  {{
                    row.sealedEvaluationMetrics
                      ? row.sealedEvaluationMetrics.expectancyPerPremium.toFixed(
                          4,
                        )
                      : "—"
                  }}
                </td>
                <td v-if="hasStressMetrics" class="mono">
                  {{
                    row.stressMetrics
                      ? row.stressMetrics.expectancyPerPremium.toFixed(4)
                      : "—"
                  }}
                </td>
                <td v-if="hasSealedStressMetrics" class="mono">
                  {{
                    row.sealedStressMetrics
                      ? row.sealedStressMetrics.expectancyPerPremium.toFixed(4)
                      : "—"
                  }}
                </td>
                <td v-if="hasHarsherStressMetrics" class="mono">
                  {{
                    row.harsherStressMetrics
                      ? row.harsherStressMetrics.expectancyPerPremium.toFixed(4)
                      : "—"
                  }}
                </td>
                <td v-if="hasSealedHarsherStressMetrics" class="mono">
                  {{
                    row.sealedHarsherStressMetrics
                      ? row.sealedHarsherStressMetrics.expectancyPerPremium.toFixed(
                          4,
                        )
                      : "—"
                  }}
                </td>
                <td v-if="hasExtremeStressMetrics" class="mono">
                  {{
                    row.extremeStressMetrics
                      ? row.extremeStressMetrics.expectancyPerPremium.toFixed(4)
                      : "—"
                  }}
                </td>
                <td v-if="hasSealedExtremeStressMetrics" class="mono">
                  {{
                    row.sealedExtremeStressMetrics
                      ? row.sealedExtremeStressMetrics.expectancyPerPremium.toFixed(
                          4,
                        )
                      : "—"
                  }}
                </td>
                <td class="mono">
                  {{ row.selectionMetrics.winRate.toFixed(3) }}
                </td>
                <td class="mono">{{ row.selectionMetrics.validTrades }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-else-if="activeRows.length > 0" class="panelMuted">
          No tuples match the current client-side filters.
        </div>
        <div v-if="selectedTuple" class="detailInspector">
          <div class="panelSectionHeader">
            <div class="panelHeaderVisual">
              <AppVisualIcon
                :icon="optionsTupleDetailIcon"
                class="sectionGlyph"
              />
              <div>
                <div class="panelEyebrow">Selected tuple</div>
                <div class="panelSectionLead">
                  Inspect the current top-row configuration and robustness
                  metrics before promoting it into a wider options workflow.
                </div>
              </div>
            </div>
          </div>
          <div class="actions">
            <button
              type="button"
              data-qa-action="options.tupleInspection.load"
              :disabled="inspectionBusy || !activeArtifactPath"
              @click="loadSelectedTupleInspection()"
              class="buttonWithIcon"
            >
              <AppVisualIcon
                :icon="optionsTupleDetailIcon"
                class="optionsActionIcon"
              />
              <span>{{
                inspectionBusy ? "Loading events…" : "Load event detail"
              }}</span>
            </button>
          </div>
          <div class="detailGrid detailGridTight">
            <div>
              <span class="panelMetricKey">Validation</span>
              <span class="panelMetricValue mono panelNumeric">{{
                selectedTupleValidation?.label ?? "Search only"
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Delay / DTE / Hold</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.delayMinutes }} / {{ selectedTuple.dteDays }} /
                {{ selectedTuple.holdMinutes }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Moneyness</span>
              <span class="panelMetricValue mono panelNumeric">{{
                selectedTuple.moneynessBucket
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Structure</span>
              <span class="panelMetricValue mono panelNumeric">{{
                selectedTuple.structureFamily || "long_single"
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Target / Stop</span>
              <span class="panelMetricValue mono panelNumeric">
                {{
                  selectedTuple.targetReturn == null
                    ? "—"
                    : selectedTuple.targetReturn
                }}
                /
                {{
                  selectedTuple.stopReturn == null
                    ? "—"
                    : selectedTuple.stopReturn
                }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Exit template</span>
              <span class="panelMetricValue mono panelNumeric">{{
                selectedTuple.exitTemplate
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Fill model</span>
              <span class="panelMetricValue mono panelNumeric">{{
                selectedTuple.fillModel
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Robust score</span>
              <span class="panelMetricValue mono panelNumeric">{{
                selectedTuple.robustScore.toFixed(3)
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Pre-sealed rank</span>
              <span class="panelMetricValue mono panelNumeric">{{
                selectedTuple.preSealedRank ?? "—"
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Sealed final rank</span>
              <span class="panelMetricValue mono panelNumeric">{{
                selectedTuple.sealedFinalRank ?? "—"
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Sealed finalist</span>
              <span class="panelMetricValue mono panelNumeric">{{
                selectedTuple.sealedFinalist ? "yes" : "no"
              }}</span>
            </div>
            <div class="detailGridSpan2">
              <span class="panelMetricKey">Validation note</span>
              <span class="panelMetricValue">{{
                selectedTupleValidation?.detail ??
                "No supported holdout or sealed trades are present yet."
              }}</span>
            </div>
            <div v-for="row in selectedTupleSupportRows" :key="row.key">
              <span class="panelMetricKey">{{ row.label }}</span>
              <span class="panelMetricValue mono panelNumeric">{{
                row.value
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">EV / Trade</span>
              <span class="panelMetricValue mono panelNumeric">
                {{
                  selectedTuple.selectionMetrics.expectancyPerTrade.toFixed(4)
                }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">EV / Premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{
                  selectedTuple.selectionMetrics.expectancyPerPremium.toFixed(4)
                }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Median return / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{
                  selectedTuple.selectionMetrics.medianReturnOnPremium.toFixed(
                    4,
                  )
                }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Win rate</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.selectionMetrics.winRate.toFixed(3) }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">Valid / Invalid</span>
              <span class="panelMetricValue mono panelNumeric">
                {{ selectedTuple.selectionMetrics.validTrades }} /
                {{ selectedTuple.selectionMetrics.invalidEvents }}
              </span>
            </div>
            <div>
              <span class="panelMetricKey">PnL / contract</span>
              <span class="panelMetricValue mono panelNumeric">
                {{
                  selectedTuple.selectionMetrics.totalPnlPerContract.toFixed(4)
                }}
              </span>
            </div>
            <div v-if="selectedTuple.holdoutMetrics">
              <span class="panelMetricKey">Holdout EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{
                  selectedTuple.holdoutMetrics.expectancyPerPremium.toFixed(4)
                }}
              </span>
            </div>
            <div v-if="selectedTuple.sealedEvaluationMetrics">
              <span class="panelMetricKey">Sealed EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{
                  selectedTuple.sealedEvaluationMetrics.expectancyPerPremium.toFixed(
                    4,
                  )
                }}
              </span>
            </div>
            <div v-if="selectedTuple.stressMetrics">
              <span class="panelMetricKey">Stress EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{
                  selectedTuple.stressMetrics.expectancyPerPremium.toFixed(4)
                }}
              </span>
            </div>
            <div v-if="selectedTuple.sealedStressMetrics">
              <span class="panelMetricKey">Sealed stress EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{
                  selectedTuple.sealedStressMetrics.expectancyPerPremium.toFixed(
                    4,
                  )
                }}
              </span>
            </div>
            <div v-if="selectedTuple.harsherStressMetrics">
              <span class="panelMetricKey">Harsher EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{
                  selectedTuple.harsherStressMetrics.expectancyPerPremium.toFixed(
                    4,
                  )
                }}
              </span>
            </div>
            <div v-if="selectedTuple.sealedHarsherStressMetrics">
              <span class="panelMetricKey">Sealed harsher EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{
                  selectedTuple.sealedHarsherStressMetrics.expectancyPerPremium.toFixed(
                    4,
                  )
                }}
              </span>
            </div>
            <div v-if="selectedTuple.extremeStressMetrics">
              <span class="panelMetricKey">Extreme EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{
                  selectedTuple.extremeStressMetrics.expectancyPerPremium.toFixed(
                    4,
                  )
                }}
              </span>
            </div>
            <div v-if="selectedTuple.sealedExtremeStressMetrics">
              <span class="panelMetricKey">Sealed extreme EV / premium</span>
              <span class="panelMetricValue mono panelNumeric">
                {{
                  selectedTuple.sealedExtremeStressMetrics.expectancyPerPremium.toFixed(
                    4,
                  )
                }}
              </span>
            </div>
          </div>
          <details
            v-if="selectedTupleValidationFrontierRows.length > 0"
            class="panelDetails"
            open
          >
            <summary>Validation frontier</summary>
            <div class="panelDetailsBody tableWrap">
              <table class="resultTable">
                <thead>
                  <tr>
                    <th>Stage</th>
                    <th>Status</th>
                    <th>Support</th>
                    <th>EV / Premium</th>
                    <th>Delta vs selection</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="row in selectedTupleValidationFrontierRows"
                    :key="row.key"
                  >
                    <td class="mono">{{ row.label }}</td>
                    <td class="mono">{{ row.status }}</td>
                    <td class="mono">{{ row.support }}</td>
                    <td class="mono">{{ row.expectancy.toFixed(4) }}</td>
                    <td class="mono">{{ row.delta.toFixed(4) }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </details>
          <details
            v-if="
              selectedTupleValidationScopeRows.length > 0 ||
              selectedTupleValidationDriftRows.length > 0 ||
              selectedTupleValidationMonotonicityRows.length > 0 ||
              selectedTupleValidationRegretRows.length > 0
            "
            class="panelDetails"
            open
          >
            <summary>Validation interpretation</summary>
            <div class="panelDetailsBody">
              <div
                v-if="selectedTupleValidationScopeRows.length > 0"
                class="detailGrid detailGridTight"
              >
                <div
                  v-for="row in selectedTupleValidationScopeRows"
                  :key="row.key"
                >
                  <span class="panelMetricKey">{{ row.label }}</span>
                  <span class="panelMetricValue mono">{{ row.value }}</span>
                </div>
              </div>
              <div class="panelMuted">
                Support rows above reflect valid trades over total events for
                this retained tuple. They describe retained evidence depth, not
                a confidence interval. Regret rows below compare only within the
                stated scope.
              </div>
              <div
                v-if="selectedTupleValidationDriftRows.length > 0"
                class="tableWrap"
              >
                <table class="resultTable">
                  <thead>
                    <tr>
                      <th>Transition</th>
                      <th>Delta EV / Premium</th>
                      <th>Scope</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr
                      v-for="row in selectedTupleValidationDriftRows"
                      :key="row.key"
                    >
                      <td class="mono">{{ row.label }}</td>
                      <td class="mono">{{ row.value }}</td>
                      <td>{{ row.scopeLabel }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div
                v-if="selectedTupleValidationMonotonicityRows.length > 0"
                class="tableWrap"
              >
                <table class="resultTable">
                  <thead>
                    <tr>
                      <th>Family</th>
                      <th>Status</th>
                      <th>Scope</th>
                      <th>Detail</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr
                      v-for="row in selectedTupleValidationMonotonicityRows"
                      :key="row.key"
                    >
                      <td class="mono">{{ row.label }}</td>
                      <td class="mono">{{ row.statusLabel }}</td>
                      <td>{{ row.scopeLabel }}</td>
                      <td>{{ row.detail }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div
                v-if="selectedTupleValidationRegretRows.length > 0"
                class="tableWrap"
              >
                <table class="resultTable">
                  <thead>
                    <tr>
                      <th>Lane</th>
                      <th>Regret</th>
                      <th>Scope</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr
                      v-for="row in selectedTupleValidationRegretRows"
                      :key="row.key"
                    >
                      <td class="mono">{{ row.label }}</td>
                      <td class="mono">{{ row.value }}</td>
                      <td>{{ row.scopeLabel }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </details>
          <details v-if="selectedInspection" class="panelDetails" open>
            <summary>Tuple event detail</summary>
            <div class="panelDetailsBody">
              <div class="detailGrid detailGridTight">
                <div>
                  <span class="panelMetricKey">Events</span>
                  <span class="panelMetricValue mono panelNumeric">{{
                    selectedInspection.totalEvents
                  }}</span>
                </div>
                <div>
                  <span class="panelMetricKey">Executed</span>
                  <span class="panelMetricValue mono panelNumeric">{{
                    selectedInspection.executedEvents
                  }}</span>
                </div>
                <div>
                  <span class="panelMetricKey">Candidate missing</span>
                  <span class="panelMetricValue mono panelNumeric">{{
                    selectedInspection.candidateMissingEvents
                  }}</span>
                </div>
                <div>
                  <span class="panelMetricKey">Outcome missing</span>
                  <span class="panelMetricValue mono panelNumeric">{{
                    selectedInspection.outcomeMissingEvents
                  }}</span>
                </div>
              </div>

              <div
                v-if="inspectionSplitStatusRows.length"
                class="detailInspector"
              >
                <div class="panelSectionHeader">
                  <div>
                    <div class="panelEyebrow">Split / status rollup</div>
                    <div class="panelSectionLead">
                      Quick inspection of where this tuple executed cleanly
                      versus failed across search, holdout, and sealed slices.
                    </div>
                  </div>
                </div>
                <div class="tableWrap">
                  <table class="resultTable">
                    <thead>
                      <tr>
                        <th>Split</th>
                        <th>Status</th>
                        <th>Count</th>
                        <th>Avg return / premium</th>
                        <th>Total PnL / contract</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr
                        v-for="row in inspectionSplitStatusRows"
                        :key="row.key"
                      >
                        <td class="mono">{{ row.split }}</td>
                        <td class="mono">{{ row.status }}</td>
                        <td class="mono">{{ row.count }}</td>
                        <td class="mono">
                          {{ formatOptionalDecimal(row.avgReturn) }}
                        </td>
                        <td class="mono">
                          {{ formatOptionalDecimal(row.totalPnl) }}
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>

              <div
                v-if="inspectionInvalidReasonRows.length"
                class="detailInspector"
              >
                <div class="panelSectionHeader">
                  <div>
                    <div class="panelEyebrow">Invalid reason rollup</div>
                    <div class="panelSectionLead">
                      Candidate-missing causes grouped by split so data-quality
                      and cutoff failures are visible before reading raw rows.
                    </div>
                  </div>
                </div>
                <div class="tableWrap">
                  <table class="resultTable">
                    <thead>
                      <tr>
                        <th>Split</th>
                        <th>Reason</th>
                        <th>Count</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr
                        v-for="row in inspectionInvalidReasonRows"
                        :key="row.key"
                      >
                        <td class="mono">{{ row.split }}</td>
                        <td class="mono">{{ row.reason }}</td>
                        <td class="mono">{{ row.count }}</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>

              <div class="row inlineControls">
                <label>
                  Split filter
                  <select
                    v-model="inspectionSplitFilter"
                    data-qa-control="options.tupleInspection.splitFilter"
                  >
                    <option value="all">all</option>
                    <option value="search">search</option>
                    <option value="holdout">holdout</option>
                    <option value="sealed_test">sealed_test</option>
                  </select>
                </label>
                <button
                  type="button"
                  data-qa-action="options.tupleInspection.exportCsv"
                  @click="store.exportActiveInspectionCsv()"
                >
                  Export inspection CSV
                </button>
                <button
                  v-if="inspectionRows.length > 50 && !showAllInspectionRows"
                  type="button"
                  @click="showAllInspectionRows = true"
                >
                  Show all {{ inspectionRows.length }}
                </button>
              </div>

              <div class="panelMuted">
                Showing {{ displayedInspectionRows.length }} of
                {{ inspectionRows.length }} event rows for tuple #{{
                  selectedInspection.tupleIndex + 1
                }}.
              </div>

              <div class="tableWrap">
                <table class="resultTable inspectionTable">
                  <thead>
                    <tr>
                      <th>Event</th>
                      <th>Split</th>
                      <th>Side</th>
                      <th>Status / reason</th>
                      <th>Contract</th>
                      <th>Requested entry</th>
                      <th>Entry</th>
                      <th>Lag / expiry</th>
                      <th>Exit</th>
                      <th>Exit reason</th>
                      <th>Path range</th>
                      <th>Return / premium</th>
                      <th>PnL / contract</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr
                      v-for="row in displayedInspectionRows"
                      :key="row.eventId"
                      :class="{
                        isSelected:
                          selectedInspectionRow?.eventId === row.eventId,
                      }"
                      tabindex="0"
                      :aria-selected="
                        selectedInspectionRow?.eventId === row.eventId
                          ? 'true'
                          : 'false'
                      "
                      :aria-label="inspectionRowLabel(row)"
                      @click="selectInspectionRow(row.eventId)"
                      @keydown="onInspectionRowKeydown($event, row.eventId)"
                    >
                      <td class="mono">{{ row.eventId }}</td>
                      <td class="mono">{{ row.split }}</td>
                      <td class="mono">{{ row.side }}</td>
                      <td class="mono">
                        {{ row.status }}
                        <template v-if="row.candidateInvalidReason">
                          <br />
                          <span class="inspectionReason">{{
                            row.candidateInvalidReason
                          }}</span>
                        </template>
                      </td>
                      <td class="mono">
                        <template v-if="row.contractId">
                          {{ row.contractId }}
                          <span v-if="row.strike != null">
                            @ {{ formatOptionalDecimal(row.strike, 2) }}</span
                          >
                        </template>
                        <template v-else>—</template>
                      </td>
                      <td class="mono">
                        {{ formatTimestamp(row.requestedEntryTimestamp) }}
                      </td>
                      <td class="mono">
                        <template v-if="row.entryTimestamp != null">
                          {{ formatTimestamp(row.entryTimestamp) }} ·
                          {{ formatOptionalDecimal(row.entryOptionPrice) }}
                          <br />
                          <span class="inspectionReason">
                            underlier
                            {{
                              formatOptionalDecimal(row.entryUnderlierPrice, 2)
                            }}
                          </span>
                        </template>
                        <template v-else>—</template>
                      </td>
                      <td class="mono">
                        {{ formatOptionalMinutes(row.entryLagMinutes) }} /
                        {{ formatOptionalMinutes(row.minutesToExpiryAtEntry) }}
                      </td>
                      <td class="mono">
                        <template v-if="row.exitTimestamp != null">
                          {{ formatTimestamp(row.exitTimestamp) }} ·
                          {{ formatOptionalDecimal(row.exitOptionPrice) }}
                          <br />
                          <span class="inspectionReason">
                            hold
                            {{ formatOptionalMinutes(row.realizedHoldMinutes) }}
                          </span>
                        </template>
                        <template v-else>—</template>
                      </td>
                      <td class="mono">{{ row.exitReason || "—" }}</td>
                      <td class="mono">
                        <template
                          v-if="
                            row.pathOptionHigh != null ||
                            row.pathUnderlierHigh != null
                          "
                        >
                          opt {{ formatOptionalDecimal(row.pathOptionLow) }} ->
                          {{ formatOptionalDecimal(row.pathOptionHigh) }}
                          <br />
                          <span class="inspectionReason">
                            und
                            {{ formatOptionalDecimal(row.pathUnderlierLow, 2) }}
                            ->
                            {{
                              formatOptionalDecimal(row.pathUnderlierHigh, 2)
                            }}
                          </span>
                        </template>
                        <template v-else>—</template>
                      </td>
                      <td class="mono">
                        {{ formatOptionalDecimal(row.returnOnPremium) }}
                      </td>
                      <td class="mono">
                        {{ formatOptionalDecimal(row.pnlPerContract) }}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <div v-if="selectedInspectionRow" class="inspectionDetailCard">
                <div class="panelSectionHeader">
                  <div>
                    <div class="panelEyebrow">Selected event</div>
                    <div class="panelSectionLead">
                      Inspect the chosen event row without leaving the current
                      tuple detail.
                    </div>
                  </div>
                </div>
                <div class="actions">
                  <button
                    v-if="selectedInspectionRow.status === 'executed'"
                    type="button"
                    data-qa-action="options.eventPath.load"
                    :disabled="eventPathBusy"
                    @click="loadSelectedEventPath()"
                    class="buttonWithIcon"
                  >
                    <AppVisualIcon
                      :icon="optionsEventPathReplayIcon"
                      class="optionsActionIcon"
                    />
                    <span>{{
                      eventPathBusy ? "Loading path…" : "Reload path replay"
                    }}</span>
                  </button>
                </div>
                <div class="detailGrid detailGridTight">
                  <div>
                    <span class="panelMetricKey">Event / split / side</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ selectedInspectionRow.eventId }} /
                      {{ selectedInspectionRow.split }} /
                      {{ selectedInspectionRow.side }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Status / reason</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ selectedInspectionRow.status
                      }}{{
                        selectedInspectionRow.candidateInvalidReason
                          ? ` / ${selectedInspectionRow.candidateInvalidReason}`
                          : ""
                      }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Contract</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ selectedInspectionRow.contractId || "—"
                      }}{{
                        selectedInspectionRow.strike != null
                          ? ` @ ${formatOptionalDecimal(selectedInspectionRow.strike, 2)}`
                          : ""
                      }}
                    </span>
                  </div>
                  <div v-if="selectedInspectionRow.companionContractId">
                    <span class="panelMetricKey">Companion contract</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ selectedInspectionRow.companionContractId
                      }}{{
                        selectedInspectionRow.companionStrike != null
                          ? ` @ ${formatOptionalDecimal(selectedInspectionRow.companionStrike, 2)}`
                          : ""
                      }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Expiry / multiplier</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{ selectedInspectionRow.expirationDate || "—" }} /
                      {{
                        selectedInspectionRow.multiplier == null
                          ? "—"
                          : selectedInspectionRow.multiplier
                      }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Decision / tradable</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{
                        formatTimestamp(selectedInspectionRow.decisionTimestamp)
                      }}
                      /
                      {{
                        formatTimestamp(
                          selectedInspectionRow.earliestTradableTimestamp,
                        )
                      }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey"
                      >Signal underlier / requested entry</span
                    >
                    <span class="panelMetricValue mono panelNumeric">
                      {{
                        formatOptionalDecimal(
                          selectedInspectionRow.rawUnderlierPrice,
                          2,
                        )
                      }}
                      /
                      {{
                        formatTimestamp(
                          selectedInspectionRow.requestedEntryTimestamp,
                        )
                      }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Requested / base exit</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{
                        formatTimestamp(
                          selectedInspectionRow.requestedEntryTimestamp,
                        )
                      }}
                      /
                      {{
                        formatTimestamp(selectedInspectionRow.baseExitTimestamp)
                      }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Entry / exit</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{
                        formatTimestamp(selectedInspectionRow.entryTimestamp)
                      }}
                      /
                      {{ formatTimestamp(selectedInspectionRow.exitTimestamp) }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Lag / expiry / hold</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{
                        formatOptionalMinutes(
                          selectedInspectionRow.entryLagMinutes,
                        )
                      }}
                      /
                      {{
                        formatOptionalMinutes(
                          selectedInspectionRow.minutesToExpiryAtEntry,
                        )
                      }}
                      /
                      {{
                        formatOptionalMinutes(
                          selectedInspectionRow.realizedHoldMinutes,
                        )
                      }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Entry underlier / option</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{
                        formatOptionalDecimal(
                          selectedInspectionRow.entryUnderlierPrice,
                          2,
                        )
                      }}
                      /
                      {{
                        formatOptionalDecimal(
                          selectedInspectionRow.entryOptionPrice,
                        )
                      }}
                    </span>
                  </div>
                  <div
                    v-if="
                      selectedInspectionRow.companionEntryOptionPrice != null
                    "
                  >
                    <span class="panelMetricKey">Companion entry / exit</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{
                        formatOptionalDecimal(
                          selectedInspectionRow.companionEntryOptionPrice,
                        )
                      }}
                      /
                      {{
                        formatOptionalDecimal(
                          selectedInspectionRow.companionExitOptionPrice,
                        )
                      }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Exit option / reason</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{
                        formatOptionalDecimal(
                          selectedInspectionRow.exitOptionPrice,
                        )
                      }}
                      / {{ selectedInspectionRow.exitReason || "—" }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Path range</span>
                    <span class="panelMetricValue mono panelNumeric">
                      opt
                      {{
                        formatOptionalDecimal(
                          selectedInspectionRow.pathOptionLow,
                        )
                      }}
                      ->
                      {{
                        formatOptionalDecimal(
                          selectedInspectionRow.pathOptionHigh,
                        )
                      }}
                      / und
                      {{
                        formatOptionalDecimal(
                          selectedInspectionRow.pathUnderlierLow,
                          2,
                        )
                      }}
                      ->
                      {{
                        formatOptionalDecimal(
                          selectedInspectionRow.pathUnderlierHigh,
                          2,
                        )
                      }}
                    </span>
                  </div>
                  <div>
                    <span class="panelMetricKey">Premium / PnL / return</span>
                    <span class="panelMetricValue mono panelNumeric">
                      {{
                        formatOptionalDecimal(selectedInspectionRow.premiumPaid)
                      }}
                      /
                      {{
                        formatOptionalDecimal(
                          selectedInspectionRow.pnlPerContract,
                        )
                      }}
                      /
                      {{
                        formatOptionalDecimal(
                          selectedInspectionRow.returnOnPremium,
                        )
                      }}
                    </span>
                  </div>
                </div>
                <div v-if="selectedEventPathInspection" class="detailInspector">
                  <div class="panelSectionHeader">
                    <div class="panelHeaderVisual">
                      <AppVisualIcon
                        :icon="optionsEventPathReplayIcon"
                        class="sectionGlyph"
                      />
                      <div>
                        <div class="panelEyebrow">Event path replay</div>
                        <div class="panelSectionLead">
                          Replayed option OHLC and underlier path for the
                          selected executed event.
                        </div>
                      </div>
                    </div>
                    <div class="actions">
                      <button
                        type="button"
                        data-qa-action="options.eventPath.exportCsv"
                        @click="store.exportActiveEventPathCsv()"
                      >
                        Export path CSV
                      </button>
                    </div>
                  </div>
                  <div class="detailGrid detailGridTight">
                    <div v-for="row in eventPathSummaryRows" :key="row.key">
                      <span class="panelMetricKey">{{ row.label }}</span>
                      <span class="panelMetricValue mono panelNumeric">{{
                        row.value
                      }}</span>
                    </div>
                  </div>
                  <div
                    v-if="eventPathAnalyticsRows.length"
                    class="detailGrid detailGridTight"
                  >
                    <div v-for="row in eventPathAnalyticsRows" :key="row.key">
                      <span class="panelMetricKey">{{ row.label }}</span>
                      <span class="panelMetricValue mono panelNumeric">{{
                        row.value
                      }}</span>
                    </div>
                  </div>
                  <div class="tableWrap">
                    <table class="resultTable">
                      <thead>
                        <tr>
                          <th>Timestamp</th>
                          <th>Option O/H/L/C</th>
                          <th>Underlier</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr
                          v-for="point in selectedEventPathInspection.points"
                          :key="`${selectedEventPathInspection.eventId}-${point.timestampMs}`"
                        >
                          <td class="mono">
                            {{ formatTimestamp(point.timestampMs) }}
                          </td>
                          <td class="mono">
                            {{ formatOptionalDecimal(point.optionOpen) }} /
                            {{ formatOptionalDecimal(point.optionHigh) }} /
                            {{ formatOptionalDecimal(point.optionLow) }} /
                            {{ formatOptionalDecimal(point.optionClose) }}
                          </td>
                          <td class="mono">
                            {{
                              formatOptionalDecimal(point.underlyingPrice, 2)
                            }}
                          </td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                </div>
              </div>
            </div>
          </details>
        </div>
        <div v-else class="emptyState">
          <img
            class="emptyStateIllustration"
            :src="emptyOptionsWorkspaceIllustration"
            alt=""
            aria-hidden="true"
            draggable="false"
          />
          <div class="emptyTitle">No ranked tuples available yet.</div>
          <div class="emptyBody">
            Run an options expression job or load a saved artifact.
          </div>
        </div>
      </section>
    </template>

    <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">
      {{ statusLine }}
    </div>
  </section>
</template>

<style scoped>
.optionsPanel {
  display: grid;
  gap: 0.85rem;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: var(--panel-footer);
  padding: 0.9rem;
}

.panelHeaderVisual {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
}

.optionsPaneGlyph {
  width: 1.35rem;
  height: 1.35rem;
  flex: 0 0 auto;
  margin-top: 0.08rem;
}

.optionsGrid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
}

.controlBlock {
  display: grid;
  gap: 0.35rem;
}

.inlineControls {
  display: flex;
  gap: 0.55rem;
  align-items: center;
}

.resultTableControls {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
  align-items: end;
}

.resultTableControls label {
  display: grid;
  gap: 0.3rem;
  font-size: 0.82rem;
}

.toggleField {
  display: inline-flex !important;
  grid-auto-flow: column;
  align-items: center;
  gap: 0.45rem;
  align-self: center;
  padding-top: 1.2rem;
}

.toggleField input {
  width: auto;
}

.inlineControls > * {
  min-width: 0;
}

.downloadFieldGrid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.7rem;
}

.optionsFields {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.7rem;
}

.optionsFields label,
.controlBlock label {
  display: grid;
  gap: 0.3rem;
  font-size: 0.82rem;
}

.optionsFields .full {
  grid-column: 1 / -1;
}

.moneynessList {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.bucketToggle {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.3rem 0.45rem;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--panel);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
}

.buttonWithIcon {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.optionsActionIcon {
  width: 0.95rem;
  height: 0.95rem;
  flex: 0 0 auto;
}

.sectionGlyph {
  width: 1.05rem;
  height: 1.05rem;
  flex: 0 0 auto;
  margin-top: 0.08rem;
}

.gateBadgeRow {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.gateBadge {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  border-radius: 999px;
  border: 1px solid var(--border);
  padding: 0.3rem 0.55rem;
  font-size: 0.76rem;
  background: color-mix(in srgb, var(--panel) 82%, transparent);
}

.gateBadge.isPass {
  border-color: color-mix(in srgb, var(--success, #4caf50) 45%, var(--border));
  color: color-mix(in srgb, var(--success, #4caf50) 78%, white);
}

.gateBadge.isPending {
  border-color: color-mix(in srgb, var(--accent, #4da3ff) 35%, var(--border));
  color: color-mix(in srgb, var(--accent, #4da3ff) 80%, white);
}

.gateBadge.isMixed {
  border-color: color-mix(in srgb, var(--warning, #f0b429) 45%, var(--border));
  color: color-mix(in srgb, var(--warning, #f0b429) 85%, white);
}

.optionsFinalistDashboard {
  display: grid;
  gap: 0.65rem;
  border-top: 1px solid var(--border);
  padding-top: 0.75rem;
}

.finalistGateGrid {
  align-items: flex-start;
}

.finalistGate {
  max-width: 100%;
}

.miniBadge {
  padding: 0.18rem 0.4rem;
  font-size: 0.7rem;
}

.coverageSection {
  display: grid;
  gap: 0.55rem;
}

.warningText {
  color: var(--warning, #f0b429);
  font-size: 0.78rem;
}

.resultCard {
  display: grid;
  gap: 0.75rem;
  border-top: 1px solid var(--border);
  padding-top: 0.75rem;
}

.detailGrid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.7rem;
}

.detailGridTight {
  gap: 0.55rem;
}

.detailGridSpan2 {
  grid-column: span 2;
}

.metricValue {
  overflow-wrap: anywhere;
}

.tableWrap {
  overflow: auto;
}

.resultTable {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.78rem;
}

.resultTable th,
.resultTable td {
  border-bottom: 1px solid var(--border);
  padding: 0.45rem 0.4rem;
  text-align: left;
}

.resultTable tbody tr {
  cursor: pointer;
}

.resultTable tbody tr.isSelected {
  background: color-mix(in srgb, var(--accent, #4da3ff) 12%, transparent);
}

.inspectionReason {
  color: color-mix(in oklab, var(--text-muted) 86%, var(--warning, #f0b429));
}

.emptyState {
  display: grid;
  gap: 0.35rem;
  padding: 0.7rem 0.2rem 0.1rem;
}

.emptyStateIllustration {
  width: 4.25rem;
  height: 4.25rem;
  display: block;
}

.emptyTitle {
  font-weight: 600;
}

.emptyBody {
  color: var(--muted);
  font-size: 0.84rem;
}

.detailInspector {
  display: grid;
  gap: 0.7rem;
  border-top: 1px solid var(--border);
  padding-top: 0.75rem;
}

.recommendationList {
  display: grid;
  gap: 0.45rem;
  margin: 0;
  padding-left: 1.1rem;
}

.inspectionDetailCard {
  display: grid;
  gap: 0.7rem;
  border-top: 1px solid var(--border);
  padding-top: 0.75rem;
}

input,
select {
  width: 100%;
}

@media (max-width: 980px) {
  .optionsGrid,
  .downloadFieldGrid,
  .optionsFields,
  .detailGrid {
    grid-template-columns: 1fr;
  }
}
</style>
