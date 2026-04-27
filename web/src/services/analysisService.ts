import { resolveRuntimePinia } from "../stores/runtimePinia";
import { markRaw } from "vue";
import { useDataStore } from "../stores/dataStore";
import { useWorkflowStore } from "../stores/workflowStore";
import { readLatestWorkflowRunSnapshot } from "../stores/workflowArtifactsStore";
import { invokeCommand } from "../tauri/invoke";
import {
  buildRunCompareSummary,
  parseArtifactLineage,
  parseRouteProvenance,
  type RunComparisonTarget,
} from "../utils/core/runArtifactReview";
import {
  buildParetoAxisOptions,
  buildParetoViewModel,
  paretoAxisValue as modelParetoAxisValue,
  toParetoRenderModel,
  type ParetoRenderModel,
} from "../visualizations/paretoModel";
import {
  finiteHeatmapValues,
  finiteValues,
  heatmapColForFast,
  heatmapHigherIsBetter as modelHeatmapHigherIsBetter,
  heatmapLogicalRowForSlow,
  heatmapModeFromValue,
  readHeatmapModel,
  resolveHeatmapDisplayModel,
  type HeatmapMode,
  type HeatmapModel,
} from "../visualizations/heatmapModel";
import type {
  HeatmapCanvasRenderModel,
  HeatmapCanvasRequestModel,
  HeatmapCellParams,
} from "../visualizations/heatmapCanvasModel";
import { normalizeIndexedPriceBars } from "../visualizations/priceSeriesModel";
import {
  buildDrillChartModel,
  buildDrillTradeMarkers as buildDrillTradeMarkersForModel,
  buildDrillTradeTableModel,
  mapTradeIndexToChartTime as modelMapTradeIndexToChartTime,
  type DrillChartPriceBar,
} from "../visualizations/drillChartModel";

type TopColumn = {
  key?: unknown;
  label?: unknown;
  sort?: unknown;
  alignLeft?: unknown;
};
type TopRowCell = { text?: unknown; className?: unknown; title?: unknown };
type TopRow = { rowId?: unknown; selected?: unknown; cells?: unknown };
type SortDir = "asc" | "desc";
type TopFilterValues = {
  minPnl?: number;
  minSharpe?: number;
  maxDd?: number;
  minTrades?: number;
  minExposure?: number;
};

interface TopPinnedCandidate {
  id: string;
  rank: number;
  title: string;
  summary: string;
  detail: string;
  badges: string[];
  staleReason: string;
}

interface AnalysisStoreState {
  bestKicker?: unknown;
  bestTitle?: unknown;
  bestMeta?: unknown;
  paretoX?: unknown;
  paretoY?: unknown;
  paretoXOptions?: unknown;
  paretoYOptions?: unknown;
  paretoWrapVisible?: unknown;
  paretoModel?: unknown;
  paretoRows?: unknown;
  paretoMeta?: unknown;
  paretoTruthNote?: unknown;
  paretoRepresentativeSummary?: unknown;
  paretoConfidenceNote?: unknown;
  topColumns?: unknown;
  topTableRows?: unknown;
  selectedTopTitle?: unknown;
  selectedTopSummary?: unknown;
  selectedTopDecisionNote?: unknown;
  selectedTopProvenanceChips?: unknown;
  pinnedTopCandidates?: unknown;
  topRows?: unknown;
  topFilterMeta?: unknown;
  topStaleCues?: unknown;
  drillWrapVisible?: unknown;
  drillChartModel?: unknown;
  drillTradeTableModel?: unknown;
  drillMeta?: unknown;
  drillStatus?: unknown;
  tradeStatsWrapVisible?: unknown;
  stabilityWrapVisible?: unknown;
  stabilityStatus?: unknown;
  stabilityMeta?: unknown;
  heatmapWrapVisible?: unknown;
  heatmapMode?: unknown;
  heatmapModeOptions?: unknown;
  heatmapColor?: unknown;
  heatmapColorOptions?: unknown;
  heatmapRange?: unknown;
  heatmapRangeOptions?: unknown;
  heatmapShowTop?: unknown;
  heatmapShowTopOptions?: unknown;
  heatmapShowCrosshair?: unknown;
  heatmapShowCrosshairOptions?: unknown;
  canHeatmapExportPng?: unknown;
  heatmapCanvasModel?: unknown;
  heatmapMeta?: unknown;
  heatmapStatus?: unknown;
  heatmapStatusLine?: unknown;
  filterMinPnl?: unknown;
  filterMinSharpe?: unknown;
  filterMaxDd?: unknown;
  filterMinTrades?: unknown;
  filterMinExposure?: unknown;
  canClearTopFilters?: unknown;
  topSelectionGuardrail?: unknown;
  compareSummary?: unknown;
  bestSummary?: unknown;
  bestStats?: unknown;
  statusLine?: unknown;
  source?: unknown;
  updatedAtUnixMs?: unknown;
}

interface HistoryStoreState {
  compareSelectedValue?: unknown;
  savedRunsRaw?: unknown;
  source?: unknown;
  updatedAtUnixMs?: unknown;
}

interface WorkflowRunState {
  ts?: unknown;
  artifactPath?: unknown;
  artifact_path?: unknown;
  reportXlsxPath?: unknown;
  report_xlsx_path?: unknown;
  csvPath?: unknown;
  csv_path?: unknown;
  configFingerprint?: unknown;
  config_fingerprint?: unknown;
  datasetMeta?: unknown;
  dataset_meta?: unknown;
  routeProvenance?: unknown;
  route_provenance?: unknown;
  lineage?: unknown;
  optimizationMetricSurface?: unknown;
  optimization_metric_surface?: unknown;
  req?: unknown;
  res?: unknown;
}

interface WorkflowValidationState {
  res?: unknown;
}

interface TopRowModel {
  rank: number;
  params: Record<string, unknown>;
  metrics: Record<string, unknown>;
  mo?: { maximize_value?: number; minimize_value?: number; violation?: number };
}

export interface AnalysisExecutionTopRowSnapshot {
  params: Record<string, unknown>;
  metrics: Record<string, unknown>;
  source: string;
}

export interface AnalysisExecutionStrategySnapshot {
  strategyKind: string;
  strategy: {
    kind: "double_ma" | "composite_cpu";
    req: Record<string, unknown>;
  };
}

interface RunArtifactPayload {
  ts?: unknown;
  run?: unknown;
  validation?: unknown;
}

interface BaselineRun {
  source: "disk" | "hist";
  ts: string;
  csvPath: string;
  reportPath: string;
  hasValidation: boolean;
  configFingerprint: string;
  datasetMeta: Record<string, unknown> | null;
  routeProvenance: ReturnType<typeof parseRouteProvenance>;
  lineage: ReturnType<typeof parseArtifactLineage>;
  optimizationMetricSurface: Record<string, unknown> | null;
  req: Record<string, unknown>;
  res: Record<string, unknown>;
}

type DrillPriceBar = DrillChartPriceBar;

interface DrillInvocation {
  runReq: Record<string, unknown>;
  kind: string;
  command: "compute_double_ma_drilldown" | "compute_composite_cpu_drilldown";
  request: Record<string, unknown>;
}

interface StabilityInvocation {
  command:
    | "compute_double_ma_local_stability"
    | "compute_composite_cpu_local_stability";
  request: Record<string, unknown>;
}

interface CacheLoadResult<T> {
  value: T;
  cacheHit: boolean;
  sharedInFlight: boolean;
}

interface DrillRenderState {
  runIdentity: string;
  rowId: string;
  row: TopRowModel;
  params: Record<string, unknown>;
  metrics: Record<string, unknown>;
  curves: unknown;
  bars: DrillPriceBar[] | null;
  stabilityResult: unknown | null;
  drillMeta: string;
  drillStatus: string;
  stabilityMeta: string;
  stabilityStatus: string;
}

export interface HeatmapCanvasController {
  exportPng: (fileName: string) => boolean;
}

export interface DrillChartController {
  clearTradeSelection: () => boolean;
  selectPrevTrade: () => string | null;
  selectNextTrade: () => string | null;
  selectTradeByIndex: (tradeIdx: unknown) => boolean;
}

interface ExactNeighborhoodRobustnessSummary {
  plateauScore: number;
  plateauLabel: string;
  neighborCount: number;
  radius: number;
  step: number;
}

const DRILL_CURVES_CACHE_MAX = 96;
const STABILITY_CACHE_MAX = 96;
const BASELINE_ARTIFACT_CACHE_MAX = 24;
const DRILL_PRICE_BARS_CACHE_MAX = 24;

let compareSeq = 0;
let drillSeq = 0;
let topRowsModelCache: TopRowModel[] = [];
let topRowsAllModelCache: TopRowModel[] = [];
let registeredDrillChartController: DrillChartController | null = null;
let registeredHeatmapCanvasController: HeatmapCanvasController | null = null;
const baselineArtifactCache = new Map<string, BaselineRun | null>();
const drillCurvesCache = new Map<string, unknown>();
const stabilityCache = new Map<string, unknown>();
const drillPriceBarsCache = new Map<string, DrillPriceBar[]>();
const baselineArtifactPending = new Map<string, Promise<BaselineRun | null>>();
const drillCurvesPending = new Map<string, Promise<unknown>>();
const stabilityPending = new Map<string, Promise<unknown>>();
const drillPriceBarsPending = new Map<string, Promise<DrillPriceBar[]>>();
let drillRenderState: DrillRenderState | null = null;
let heatmapRenderState: HeatmapCanvasRenderModel | null = null;
let lastAnalysisRunIdentity = "";

function text(v: unknown): string {
  return String(v ?? "").trim();
}

function num(v: unknown, fallback = NaN): number {
  const n = Number(v);
  return Number.isFinite(n) ? n : fallback;
}

function bool(v: unknown, fallback = false): boolean {
  if (typeof v === "boolean") {
    return v;
  }
  if (typeof v === "string") {
    const normalized = v.trim().toLowerCase();
    if (normalized === "true" || normalized === "1") {
      return true;
    }
    if (normalized === "false" || normalized === "0") {
      return false;
    }
  }
  if (typeof v === "number") {
    return Number.isFinite(v) ? v !== 0 : fallback;
  }
  return fallback;
}

function asRecord(v: unknown): Record<string, unknown> {
  return v && typeof v === "object" ? (v as Record<string, unknown>) : {};
}

function markRawEntity<T>(value: T): T {
  if (value && typeof value === "object") {
    return markRaw(value as object) as T;
  }
  return value;
}

function asNumberMap(v: unknown): Record<string, number> | null {
  if (!v || typeof v !== "object") return null;
  const src = v as Record<string, unknown>;
  const out: Record<string, number> = {};
  for (const key of Object.keys(src)) {
    const next = Number(src[key]);
    if (!Number.isFinite(next)) continue;
    out[key] = next;
  }
  return Object.keys(out).length ? out : null;
}

function fmt(v: unknown, digits = 6): string {
  const n = Number(v);
  if (!Number.isFinite(n)) return "-";
  return n.toFixed(Math.max(0, Math.floor(digits)));
}

function fmtPct(v: unknown, digits = 2): string {
  const n = Number(v);
  if (!Number.isFinite(n)) return "-";
  return `${(n * 100).toFixed(Math.max(0, Math.floor(digits)))}%`;
}

function trimFixed(src: string): string {
  return src.replace(/(\.\d*?[1-9])0+$/u, "$1").replace(/\.0+$/u, "");
}

function fmtDisplay(v: unknown, digits = 4): string {
  const n = Number(v);
  if (!Number.isFinite(n)) return "-";
  const abs = Math.abs(n);
  if (abs === 0) return "0";
  const precision = Math.max(0, Math.floor(digits));
  const zeroThreshold = 1 / 10 ** Math.max(1, precision);
  if (abs < zeroThreshold) return "0";
  if (abs >= 1_000_000) {
    return n.toLocaleString(undefined, {
      maximumFractionDigits: Math.min(2, precision),
      minimumFractionDigits: 0,
    });
  }
  return trimFixed(n.toFixed(precision));
}

function fmtPctDisplay(v: unknown, digits = 2): string {
  const n = Number(v);
  if (!Number.isFinite(n)) return "-";
  return `${fmtDisplay(n * 100, digits)}%`;
}

function requestedTopK(req: Record<string, unknown>): number {
  return Math.max(0, Math.floor(num(req.top_k ?? req.topK, 0)));
}

function requestedHeatmapBins(req: Record<string, unknown>): number {
  return Math.max(0, Math.floor(num(req.heatmap_bins ?? req.heatmapBins, 0)));
}

const SCORE_CLAMP_MAX = 1e6;
const SCORE_EPS = 1e-12;
const SCORE_CLIP = 4.0;

function scoreModeOf(req: Record<string, unknown>): string {
  const raw = text(req.score_mode ?? req.scoreMode).toLowerCase();
  if (
    raw === "balanced_risk_adjusted_10" ||
    raw === "balancedriskadjusted10" ||
    raw === "balanced-risk-adjusted-10"
  ) {
    return "balanced_risk_adjusted_10";
  }
  if (
    raw === "adversarial_envelope_10" ||
    raw === "adversarialenvelope10" ||
    raw === "adversarial-envelope-10" ||
    raw === "adversarial_weight_envelope_10" ||
    raw === "adversarialweightenvelope10" ||
    raw === "adversarial-weight-envelope-10"
  ) {
    return "adversarial_envelope_10";
  }
  if (
    raw === "support_shrunk_10" ||
    raw === "supportshrunk10" ||
    raw === "support-shrunk-10" ||
    raw === "effective_support_10" ||
    raw === "effectivesupport10" ||
    raw === "effective-support-10"
  ) {
    return "support_shrunk_10";
  }
  if (
    raw === "active_sortino_10" ||
    raw === "activesortino10" ||
    raw === "active-sortino-10" ||
    raw === "activesortino"
  ) {
    return "active_sortino_10";
  }
  if (
    raw === "effective_sample_lcb_10" ||
    raw === "effectivesamplelcb10" ||
    raw === "effective-sample-lcb-10" ||
    raw === "obs_lcb_10" ||
    raw === "obslcb10" ||
    raw === "obs-lcb-10"
  ) {
    return "effective_sample_lcb_10";
  }
  if (
    raw === "omega_vs_sharpe_10" ||
    raw === "omegavssharpe10" ||
    raw === "omega-vs-sharpe-10" ||
    raw === "omega_vs_sharpe" ||
    raw === "omegavssharpe"
  ) {
    return "omega_vs_sharpe_10";
  }
  if (
    raw === "robust_margin_10" ||
    raw === "robustmargin10" ||
    raw === "robust-margin-10" ||
    raw === "hurdle_slack_10" ||
    raw === "hurdleslack10" ||
    raw === "hurdle-slack-10"
  ) {
    return "robust_margin_10";
  }
  return "";
}

function scoreLabelOf(req: Record<string, unknown>): string {
  const scoreMode = scoreModeOf(req);
  if (scoreMode === "balanced_risk_adjusted_10") {
    return "Balanced Risk-Adjusted 10";
  }
  if (scoreMode === "adversarial_envelope_10") {
    return "Adversarial Envelope 10";
  }
  if (scoreMode === "support_shrunk_10") {
    return "Support-Shrunk 10";
  }
  if (scoreMode === "active_sortino_10") {
    return "Active Sortino 10";
  }
  if (scoreMode === "effective_sample_lcb_10") {
    return "Effective-Sample LCB 10";
  }
  if (scoreMode === "omega_vs_sharpe_10") {
    return "Omega vs Sharpe 10";
  }
  if (scoreMode === "robust_margin_10") {
    return "Robust Margin 10";
  }
  return objectiveOf(req);
}

function extraShortlistObjectivesOf(req: Record<string, unknown>): string[] {
  const raw = req.extra_shortlist_objectives ?? req.extraShortlistObjectives;
  if (!Array.isArray(raw)) return [];
  return raw
    .map((value) => text(value))
    .filter(
      (value, index, arr) => value.length > 0 && arr.indexOf(value) === index,
    );
}

function hurdleTauOf(req: Record<string, unknown>): string {
  return text(req.hurdle_tau ?? req.hurdleTau);
}

function shortlistConfidenceModeOf(req: Record<string, unknown>): string {
  const raw = text(
    req.shortlist_confidence_mode ?? req.shortlistConfidenceMode,
  ).toLowerCase();
  if (raw === "psr") return "psr";
  if (raw === "dsr") return "dsr";
  return "";
}

function shortlistUnionModeOf(req: Record<string, unknown>): string {
  const raw = text(
    req.shortlist_union_mode ?? req.shortlistUnionMode,
  ).toLowerCase();
  if (raw === "lexicographic") return "lexicographic";
  if (raw === "pareto") return "pareto";
  if (raw === "epsilon_front") return "epsilon_front";
  if (raw === "weighted_rank") return "weighted_rank";
  return "";
}

function shortlistSliceModeOf(req: Record<string, unknown>): string {
  const raw = text(
    req.shortlist_slice_mode ?? req.shortlistSliceMode,
  ).toLowerCase();
  if (raw === "halves") return "halves";
  if (raw === "quartiles") return "quartiles";
  return "";
}

function shortlistSliceBenchmarkOf(req: Record<string, unknown>): string {
  const raw = text(
    req.shortlist_slice_benchmark ?? req.shortlistSliceBenchmark,
  ).toLowerCase();
  if (raw === "flat") return "flat";
  if (raw === "buy_and_hold") return "buy_and_hold";
  return "";
}

function shortlistNeighborhoodSummaryOf(req: Record<string, unknown>): string {
  const radius = text(
    req.shortlist_neighborhood_radius ?? req.shortlistNeighborhoodRadius,
  );
  if (!radius) return "";
  const step =
    text(req.shortlist_neighborhood_step ?? req.shortlistNeighborhoodStep) ||
    "1";
  return `${radius}x${step}`;
}

function scoreClipSigned(value: number): number {
  return Math.max(-SCORE_CLIP, Math.min(SCORE_CLIP, value));
}

function scoreClipPenalty(value: number): number {
  return Math.max(0, Math.min(SCORE_CLIP, value));
}

function scoreAsinhScaled(value: number, scale: number): number {
  if (!Number.isFinite(value) || !Number.isFinite(scale) || scale <= 0) {
    return 0;
  }
  return scoreClipSigned(Math.asinh(value / scale));
}

function scoreLogRatio(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return scoreClipSigned(Math.log(Math.max(SCORE_EPS, value)));
}

function scoreLog1pPenalty(value: number, scale: number): number {
  if (!Number.isFinite(value) || !Number.isFinite(scale) || scale <= 0) {
    return 0;
  }
  return scoreClipPenalty(Math.log1p(Math.max(0, value) / scale));
}

function scoreTradeUtility(trades: number, target: number): number {
  if (!Number.isFinite(target) || target <= 0) return 0;
  return Math.max(
    0,
    Math.min(1, Math.log1p(Math.max(0, trades)) / Math.log1p(target)),
  );
}

function derivedMean(metrics: Record<string, unknown>): number {
  return num(metrics.sharpe, 0) * num(metrics.volatility, 0);
}

function derivedActiveMean(metrics: Record<string, unknown>): number {
  const exposure = Math.max(num(metrics.exposure, 0), SCORE_EPS);
  if (!Number.isFinite(exposure) || exposure <= SCORE_EPS) return 0;
  return derivedMean(metrics) / exposure;
}

function derivedActiveDownsideDeviation(
  metrics: Record<string, unknown>,
): number {
  const exposure = Math.max(num(metrics.exposure, 0), SCORE_EPS);
  if (!Number.isFinite(exposure) || exposure <= SCORE_EPS) return 0;
  return num(metrics.downside_deviation, 0) / Math.sqrt(exposure);
}

function derivedActiveSortino(metrics: Record<string, unknown>): number {
  const mean = derivedActiveMean(metrics);
  const denom = derivedActiveDownsideDeviation(metrics);
  if (denom > SCORE_EPS) return mean / denom;
  if (mean > 0) return SCORE_CLAMP_MAX;
  if (mean < 0) return -SCORE_CLAMP_MAX;
  return 0;
}

function derivedDrawdownChronicity(metrics: Record<string, unknown>): number {
  const maxDd = num(metrics.max_dd, 0);
  if (maxDd > SCORE_EPS) {
    return Math.max(
      0,
      Math.min(SCORE_CLIP, num(metrics.ulcer_index, 0) / maxDd),
    );
  }
  return 0;
}

function derivedDownsideShare(metrics: Record<string, unknown>): number {
  const volatility = num(metrics.volatility, 0);
  if (volatility > SCORE_EPS) {
    return Math.max(
      0,
      Math.min(SCORE_CLIP, num(metrics.downside_deviation, 0) / volatility),
    );
  }
  return 0;
}

function derivedLongExposure(metrics: Record<string, unknown>): number {
  return Math.max(
    0,
    Math.min(
      1,
      0.5 * (num(metrics.exposure, 0) + num(metrics.net_exposure, 0)),
    ),
  );
}

function derivedShortExposure(metrics: Record<string, unknown>): number {
  return Math.max(
    0,
    Math.min(
      1,
      0.5 * (num(metrics.exposure, 0) - num(metrics.net_exposure, 0)),
    ),
  );
}

function derivedDirectionalBias(metrics: Record<string, unknown>): number {
  const exposure = Math.max(num(metrics.exposure, 0), SCORE_EPS);
  if (!Number.isFinite(exposure) || exposure <= SCORE_EPS) return 0;
  return Math.max(0, Math.min(1, Math.abs(num(metrics.net_exposure, 0)) / exposure));
}

function scoreExposurePenalty(metrics: Record<string, unknown>): number {
  const exposure = Math.max(0, Math.abs(num(metrics.exposure, 0)));
  const net = Math.max(0, Math.abs(num(metrics.net_exposure, 0)));
  const penalty =
    Math.max(0, 0.1 - exposure) / 0.1 +
    Math.max(0, exposure - 0.95) / 0.1 +
    Math.max(0, net - 0.35) / 0.2;
  return scoreClipPenalty(penalty);
}

function scoreDownsideShapePenalty(metrics: Record<string, unknown>): number {
  const downsideShare = derivedDownsideShare(metrics);
  return scoreClipPenalty(Math.max(0, downsideShare - 0.75) / 0.25);
}

function scoreSupportProxyUnits(metrics: Record<string, unknown>): number {
  const trades = Math.max(0, num(metrics.trades, 0));
  const exposure = Math.max(0, Math.min(1, num(metrics.exposure, 0)));
  return 1 + trades + 80 * exposure;
}

function scoreObservationContextOf(
  res: Record<string, unknown>,
): Record<string, unknown> {
  return asRecord(
    res.score_observation_context ?? res.scoreObservationContext,
  );
}

function effectiveSampleObsCount(
  params: Record<string, unknown>,
  scoreContext: Record<string, unknown>,
): number {
  const fastLen = Math.max(1, Math.floor(num(params.fast_len, 0)));
  const slowLen = Math.max(1, Math.floor(num(params.slow_len, 0)));
  const firstValid = Math.max(0, Math.floor(num(scoreContext.first_valid, 0)));
  const regimeFirstValid = Math.max(
    0,
    Math.floor(num(scoreContext.regime_first_valid, 0)),
  );
  const evalStart = Math.max(0, Math.floor(num(scoreContext.eval_start, 0)));
  const evalEnd = Math.max(0, Math.floor(num(scoreContext.eval_end, 0)));
  const totalBars = Math.max(0, Math.floor(num(scoreContext.total_bars, 0)));
  const tradeOffset = bool(scoreContext.trade_on_next_bar, false) ? 1 : 0;
  const pairFirstValid = firstValid + Math.max(fastLen, slowLen) - 1;
  const tValid = Math.max(pairFirstValid, Math.max(regimeFirstValid, evalStart));
  const startT = Math.max(tValid + tradeOffset, evalStart);
  const endT = Math.min(totalBars, evalEnd);
  return Math.max(endT - startT, 2);
}

function showDerivedScoreColumns(scoreMode = ""): boolean {
  return !!text(scoreMode);
}

function balancedRiskAdjusted10Score(metrics: Record<string, unknown>): number {
  const sharpeU = scoreAsinhScaled(num(metrics.sharpe, 0), 1);
  const activeSortinoU = scoreAsinhScaled(derivedActiveSortino(metrics), 1.25);
  const omegaU = scoreLogRatio(num(metrics.omega, 0));
  const calmarU = scoreAsinhScaled(num(metrics.calmar, 0), 1);
  const tradeU = scoreTradeUtility(num(metrics.trades, 0), 40);
  const maxDdP = scoreLog1pPenalty(num(metrics.max_dd, 0), 0.1);
  const ulcerP = scoreLog1pPenalty(num(metrics.ulcer_index, 0), 0.05);
  const chronicityP = Math.max(
    0,
    Math.min(1, derivedDrawdownChronicity(metrics)),
  );
  const exposureP = scoreExposurePenalty(metrics);
  const downsideP = scoreDownsideShapePenalty(metrics);

  return (
    0.42 * sharpeU +
    0.2 * activeSortinoU +
    0.12 * omegaU +
    0.08 * calmarU +
    0.08 * tradeU -
    0.16 * maxDdP -
    0.1 * ulcerP -
    0.08 * chronicityP -
    0.06 * exposureP -
    0.04 * downsideP
  );
}

function adversarialEnvelope10Score(metrics: Record<string, unknown>): number {
  const sharpeU = scoreAsinhScaled(num(metrics.sharpe, 0), 1);
  const activeMeanU = scoreAsinhScaled(derivedActiveMean(metrics), 0.12);
  const activeSortinoU = scoreAsinhScaled(derivedActiveSortino(metrics), 1.25);
  const omegaU = scoreLogRatio(num(metrics.omega, 0));
  const calmarU = scoreAsinhScaled(num(metrics.calmar, 0), 1);
  const tradeU = scoreTradeUtility(num(metrics.trades, 0), 40);
  const maxDdP = scoreLog1pPenalty(num(metrics.max_dd, 0), 0.1);
  const ulcerP = scoreLog1pPenalty(num(metrics.ulcer_index, 0), 0.05);
  const chronicityP = Math.max(
    0,
    Math.min(1, derivedDrawdownChronicity(metrics)),
  );
  const exposureP = scoreExposurePenalty(metrics);
  const downsideP = scoreDownsideShapePenalty(metrics);

  const balanced =
    0.34 * sharpeU +
    0.15 * activeMeanU +
    0.15 * activeSortinoU +
    0.11 * omegaU +
    0.09 * calmarU +
    0.06 * tradeU -
    0.15 * maxDdP -
    0.1 * ulcerP -
    0.07 * chronicityP -
    0.05 * exposureP -
    0.03 * downsideP;
  const pathGuard =
    0.28 * sharpeU +
    0.1 * activeMeanU +
    0.12 * activeSortinoU +
    0.11 * omegaU +
    0.18 * calmarU +
    0.04 * tradeU -
    0.22 * maxDdP -
    0.15 * ulcerP -
    0.12 * chronicityP -
    0.05 * exposureP -
    0.04 * downsideP;
  const fragilityGuard =
    0.3 * sharpeU +
    0.12 * activeMeanU +
    0.18 * activeSortinoU +
    0.15 * omegaU +
    0.1 * calmarU +
    0.05 * tradeU -
    0.16 * maxDdP -
    0.12 * ulcerP -
    0.08 * chronicityP -
    0.05 * exposureP -
    0.11 * downsideP;

  return Math.min(balanced, pathGuard, fragilityGuard);
}

function supportShrunk10Score(metrics: Record<string, unknown>): number {
  const base = balancedRiskAdjusted10Score(metrics);
  const supportUnits = Math.max(1, scoreSupportProxyUnits(metrics));
  const shrinkPenalty = 1.2 / Math.sqrt(supportUnits);
  const chronicityPenalty =
    0.1 * Math.max(0, Math.min(1, derivedDrawdownChronicity(metrics)));
  const downsidePenalty = 0.12 * scoreDownsideShapePenalty(metrics);
  return base - shrinkPenalty - chronicityPenalty - downsidePenalty;
}

function activeSortino10Score(metrics: Record<string, unknown>): number {
  const activeSortinoU = scoreAsinhScaled(derivedActiveSortino(metrics), 1.25);
  const omegaU = scoreLogRatio(num(metrics.omega, 0));
  const calmarU = scoreAsinhScaled(num(metrics.calmar, 0), 1);
  const tradeU = scoreTradeUtility(num(metrics.trades, 0), 36);
  const maxDdP = scoreLog1pPenalty(num(metrics.max_dd, 0), 0.1);
  const ulcerP = scoreLog1pPenalty(num(metrics.ulcer_index, 0), 0.05);
  const chronicityP =
    0.18 * Math.max(0, Math.min(1, derivedDrawdownChronicity(metrics)));
  const exposureP = 0.12 * scoreExposurePenalty(metrics);
  const downsideP = 0.12 * scoreDownsideShapePenalty(metrics);
  return (
    0.58 * activeSortinoU +
    0.12 * omegaU +
    0.08 * calmarU +
    0.06 * tradeU -
    0.18 * maxDdP -
    0.1 * ulcerP -
    chronicityP -
    exposureP -
    downsideP
  );
}

function effectiveSampleLcb10Score(
  metrics: Record<string, unknown>,
  params: Record<string, unknown>,
  scoreContext: Record<string, unknown>,
): number {
  const base = balancedRiskAdjusted10Score(metrics);
  if (!Object.keys(params).length || !Object.keys(scoreContext).length) {
    return base;
  }
  const effectiveN = Math.max(2, effectiveSampleObsCount(params, scoreContext));
  return base - 18 / Math.sqrt(effectiveN);
}

function omegaVsSharpe10Score(metrics: Record<string, unknown>): number {
  const sharpeU = scoreAsinhScaled(num(metrics.sharpe, 0), 1);
  const omegaU = scoreLogRatio(num(metrics.omega, 0));
  const asymmetryGap = scoreClipSigned(omegaU - 0.65 * sharpeU);
  const activeSortinoU = scoreAsinhScaled(derivedActiveSortino(metrics), 1.25);
  const calmarU = scoreAsinhScaled(num(metrics.calmar, 0), 1);
  const tradeU = scoreTradeUtility(num(metrics.trades, 0), 32);
  const maxDdP = scoreLog1pPenalty(num(metrics.max_dd, 0), 0.1);
  const ulcerP = scoreLog1pPenalty(num(metrics.ulcer_index, 0), 0.05);
  const chronicityP =
    0.14 * Math.max(0, Math.min(1, derivedDrawdownChronicity(metrics)));
  const downsideP = 0.1 * scoreDownsideShapePenalty(metrics);
  const exposureP = 0.08 * scoreExposurePenalty(metrics);
  const directionalP =
    0.08 * Math.max(0, Math.min(1, derivedDirectionalBias(metrics)));
  return (
    0.36 * asymmetryGap +
    0.2 * activeSortinoU +
    0.14 * omegaU +
    0.1 * calmarU +
    0.06 * tradeU -
    0.18 * maxDdP -
    0.1 * ulcerP -
    chronicityP -
    downsideP -
    exposureP -
    directionalP
  );
}

function scoreMarginAtLeast(value: number, threshold: number, scale: number): number {
  if (!Number.isFinite(value) || !Number.isFinite(threshold) || !Number.isFinite(scale) || scale <= 0) {
    return 0;
  }
  return scoreClipSigned((value - threshold) / scale);
}

function scoreMarginAtMost(value: number, threshold: number, scale: number): number {
  if (!Number.isFinite(value) || !Number.isFinite(threshold) || !Number.isFinite(scale) || scale <= 0) {
    return 0;
  }
  return scoreClipSigned((threshold - value) / scale);
}

function scoreSoftmin(values: number[], temperature: number): number {
  if (!Array.isArray(values) || values.length === 0 || !Number.isFinite(temperature) || temperature <= 0) {
    return 0;
  }
  const anchor = values.reduce((cur, value) => Math.min(cur, value), Number.POSITIVE_INFINITY);
  if (!Number.isFinite(anchor)) return 0;
  const sumExp = values.reduce((cur, value) => cur + Math.exp(-(value - anchor) / temperature), 0);
  if (!Number.isFinite(sumExp) || sumExp <= 0) return anchor;
  return scoreClipSigned(anchor - temperature * Math.log(sumExp));
}

function robustMargin10Score(metrics: Record<string, unknown>): number {
  const margins = [
    scoreMarginAtLeast(num(metrics.sharpe, 0), 0.9, 0.45),
    scoreMarginAtLeast(derivedActiveSortino(metrics), 1.25, 0.7),
    scoreMarginAtLeast(num(metrics.omega, 0), 1.05, 0.2),
    scoreMarginAtLeast(num(metrics.calmar, 0), 0.65, 0.35),
    scoreMarginAtLeast(scoreTradeUtility(num(metrics.trades, 0), 24), 0.35, 0.18),
    scoreMarginAtMost(num(metrics.max_dd, 0), 0.22, 0.08),
    scoreMarginAtMost(num(metrics.ulcer_index, 0), 0.1, 0.04),
    scoreMarginAtMost(derivedDrawdownChronicity(metrics), 0.7, 0.2),
    scoreMarginAtMost(scoreDownsideShapePenalty(metrics), 0.7, 0.35),
    scoreMarginAtMost(scoreExposurePenalty(metrics), 0.7, 0.35),
  ];
  const softMargin = scoreSoftmin(margins, 0.55);
  const meanMargin = margins.reduce((cur, value) => cur + value, 0) / margins.length;
  return 0.72 * softMargin + 0.28 * meanMargin;
}

function formatDrillMetricsSummary(
  metricsRaw: Record<string, unknown>,
): string {
  const metrics = asRecord(metricsRaw);
  if (!Object.keys(metrics).length) return "";
  return `pnl=${fmt(metrics.pnl, 6)} ret=${fmtPct(metrics.pnl, 2)} sharpe=${fmt(metrics.sharpe, 6)} dd=${fmt(metrics.max_dd, 6)} trades=${num(metrics.trades, 0)} expo=${fmt(metrics.exposure, 4)}`;
}

function formatDrillMoSummary(
  moRaw: Record<string, unknown> | null | undefined,
): string {
  const mo = asRecord(moRaw);
  if (!Object.keys(mo).length) return "";
  return `max=${fmt(mo.maximize_value, 6)} min=${fmt(mo.minimize_value, 6)} viol=${fmt(mo.violation, 6)}`;
}

function parseIndex(v: unknown): number {
  const n = Number(v);
  if (!Number.isFinite(n)) return 0;
  return Math.max(0, Math.floor(n));
}

function normalizeStrategyKindRaw(kindRaw: unknown): string {
  const kind = text(kindRaw || "DoubleMa") || "DoubleMa";
  if (kind === "Pit") return "Pit";
  if (kind === "Set") return "Set";
  if (kind === "Mts") return "Mts";
  if (kind === "Marrt") return "Marrt";
  if (kind === "Vfct") return "Vfct";
  if (kind === "Rocb") return "Rocb";
  if (kind === "Kqsr") return "Kqsr";
  if (kind === "Carm") return "Carm";
  if (kind === "Kvdf") return "Kvdf";
  if (kind === "Mkcs") return "Mkcs";
  if (kind === "Cdmc") return "Cdmc";
  if (kind === "Asr") return "Asr";
  if (kind === "Vcc") return "Vcc";
  if (kind === "Vemr") return "Vemr";
  if (kind === "Vcet") return "Vcet";
  if (kind === "CompositeCpu") return "CompositeCpu";
  if (kind === "CompositeMo") return "CompositeMo";
  if (kind === "CompositeTrend") return "CompositeTrend";
  if (kind === "RegimeHybrid") return "RegimeHybrid";
  if (kind === "BreakoutMomentum") return "BreakoutMomentum";
  return "DoubleMa";
}

function isDoubleLikeKind(kindRaw: unknown): boolean {
  const kind = normalizeStrategyKindRaw(kindRaw);
  return (
    kind === "DoubleMa" ||
    kind === "Pit" ||
    kind === "Set" ||
    kind === "Mts" ||
    kind === "Orc" ||
    kind === "Marrt" ||
    kind === "Vfct" ||
    kind === "CompositeTrend" ||
    kind === "RegimeHybrid" ||
    kind === "BreakoutMomentum"
  );
}

function markAnalysisVue(store: AnalysisStoreState): void {
  store.source = "vue";
  store.updatedAtUnixMs = Date.now();
}

function markHistoryVue(store: HistoryStoreState): void {
  store.source = "vue";
  store.updatedAtUnixMs = Date.now();
}

function readAnalysisStore(): AnalysisStoreState | null {
  const pinia = resolveRuntimePinia();
  if (!pinia) return null;
  const root = pinia.state.value as Record<string, unknown>;
  const state = root.analysis;
  if (!state || typeof state !== "object") return null;
  return state as AnalysisStoreState;
}

function readHistoryStore(): HistoryStoreState | null {
  const pinia = resolveRuntimePinia();
  if (!pinia) return null;
  const root = pinia.state.value as Record<string, unknown>;
  const state = root.history;
  if (!state || typeof state !== "object") return null;
  return state as HistoryStoreState;
}

function readWorkflowRun(): WorkflowRunState | null {
  const pinia = resolveRuntimePinia();
  if (pinia) {
    const root = pinia.state.value as Record<string, unknown>;
    const wf = asRecord(root.workflowArtifacts);
    const run = wf.currentRun;
    if (run && typeof run === "object") return run as WorkflowRunState;
  }
  const snapshot = readLatestWorkflowRunSnapshot() as Record<string, unknown> | null;
  if (!snapshot) return null;
  return {
    ts: snapshot.ts ?? null,
    artifactPath: snapshot.artifactPath ?? null,
    configFingerprint: snapshot.configFingerprint ?? null,
    routeProvenance: snapshot.routeProvenance ?? null,
    req: snapshot.req ?? null,
    res: snapshot.res ?? null,
  };
}

function hasValidationResult(): boolean {
  const pinia = resolveRuntimePinia();
  if (!pinia) return false;
  const root = pinia.state.value as Record<string, unknown>;
  const wf = asRecord(root.workflowArtifacts);
  const val = wf.currentValidation;
  if (!val || typeof val !== "object") return false;
  return !!(val as WorkflowValidationState).res;
}

function readTopColumns(store: AnalysisStoreState): TopColumn[] {
  return Array.isArray(store.topColumns)
    ? (store.topColumns as TopColumn[])
    : [];
}

function readTopRows(store: AnalysisStoreState): TopRow[] {
  return Array.isArray(store.topTableRows)
    ? (store.topTableRows as TopRow[])
    : [];
}

function readSortState(store: AnalysisStoreState): {
  key: string;
  dir: SortDir;
} {
  const columns = readTopColumns(store);
  for (const col of columns) {
    const key = text(col?.key);
    const dir = text(col?.sort).toLowerCase();
    if (!key) continue;
    if (dir === "asc" || dir === "desc") return { key, dir };
  }
  return { key: "rank", dir: "asc" };
}

const ASC_KEYS = new Set(["rank", "minimize_value", "violation"]);
function defaultSortDir(key: string): SortDir {
  return ASC_KEYS.has(text(key)) ? "asc" : "desc";
}

function scoreOf(
  metrics: Record<string, unknown>,
  objective: string,
  scoreMode = "",
  params: Record<string, unknown> = {},
  scoreContext: Record<string, unknown> = {},
): number {
  if (scoreMode === "balanced_risk_adjusted_10") {
    return balancedRiskAdjusted10Score(metrics);
  }
  if (scoreMode === "adversarial_envelope_10") {
    return adversarialEnvelope10Score(metrics);
  }
  if (scoreMode === "support_shrunk_10") {
    return supportShrunk10Score(metrics);
  }
  if (scoreMode === "active_sortino_10") {
    return activeSortino10Score(metrics);
  }
  if (scoreMode === "effective_sample_lcb_10") {
    return effectiveSampleLcb10Score(metrics, params, scoreContext);
  }
  if (scoreMode === "omega_vs_sharpe_10") {
    return omegaVsSharpe10Score(metrics);
  }
  if (scoreMode === "robust_margin_10") {
    return robustMargin10Score(metrics);
  }
  const obj = String(objective || "Sharpe");
  if (obj === "Pnl") return num(metrics.pnl, 0);
  if (obj === "Sharpe") return num(metrics.sharpe, 0);
  if (obj === "Sortino") return num(metrics.sortino, 0);
  if (obj === "Calmar") return num(metrics.calmar, 0);
  if (obj === "Omega") return num(metrics.omega, 0);
  if (obj === "Volatility") return -num(metrics.volatility, 0);
  if (obj === "MaxDrawdown") return -num(metrics.max_dd, 0);
  return num(metrics.sharpe, 0);
}

function objectiveOf(req: Record<string, unknown>): string {
  const direct = text(req.objective);
  if (direct) return direct;
  const objectives = asRecord(req.objectives);
  return text(objectives.maximize) || "Sharpe";
}

function isMo(
  req: Record<string, unknown>,
  res: Record<string, unknown>,
): boolean {
  const kind = text(req.strategy_kind ?? req.strategyKind ?? "DoubleMa");
  return kind === "CompositeMo" && Array.isArray(res.pareto);
}

function resolveRunMaParams(
  runReq: Record<string, unknown>,
  side: "fast" | "slow",
  maId: string,
): Record<string, number> | null {
  const direct =
    side === "fast"
      ? asNumberMap(runReq.fast_ma_params ?? runReq.fastMaParams)
      : asNumberMap(runReq.slow_ma_params ?? runReq.slowMaParams);
  if (direct) return direct;
  const id = text(maId).toLowerCase();
  if (id !== "alma") return null;
  const hasLegacy =
    runReq.alma_offset != null ||
    runReq.almaOffset != null ||
    runReq.alma_sigma != null ||
    runReq.almaSigma != null;
  if (!hasLegacy) return null;
  return {
    offset: num(runReq.alma_offset ?? runReq.almaOffset, 0.85),
    sigma: num(runReq.alma_sigma ?? runReq.almaSigma, 6.0),
  };
}

function desiredDrillBins(): number {
  if (typeof document === "undefined") return 1024;
  const canvas = byIds<HTMLCanvasElement>(["equityCanvasVue", "equityCanvas"]);
  if (!canvas) return 1024;
  const rect = canvas.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  const w = Math.floor((rect.width || 0) * dpr);
  if (w < 64) return 1024;
  return Math.max(64, Math.min(4096, w));
}

function requestCacheKey(
  command: string,
  req: Record<string, unknown>,
): string {
  try {
    return `${command}:${stableStringify(req)}`;
  } catch {
    return `${command}:${Date.now()}`;
  }
}

function collectTopRows(
  res: Record<string, unknown>,
  options: { allowBestFallback?: boolean } = {},
): TopRowModel[] {
  const out: TopRowModel[] = [];
  const allowBestFallback = options.allowBestFallback !== false;
  const pareto = Array.isArray(res.pareto) ? res.pareto : null;
  if (pareto) {
    const pts = pareto.map((entry) => {
      const row = asRecord(entry);
      return {
        params: asRecord(row.params),
        metrics: asRecord(row.metrics),
        maximize_value: num(row.maximize_value, Number.NEGATIVE_INFINITY),
        minimize_value: num(row.minimize_value, Number.POSITIVE_INFINITY),
        violation: num(row.violation, 0),
      };
    });
    pts.sort((a, b) => {
      const af = a.violation <= 0;
      const bf = b.violation <= 0;
      if (af !== bf) return af ? -1 : 1;
      if (a.maximize_value !== b.maximize_value)
        return b.maximize_value - a.maximize_value;
      if (a.minimize_value !== b.minimize_value)
        return a.minimize_value - b.minimize_value;
      return 0;
    });
    for (let i = 0; i < pts.length; i += 1) {
      const p = pts[i];
      if (!p) continue;
      out.push({
        rank: i + 1,
        params: p.params,
        metrics: p.metrics,
        mo: {
          maximize_value: p.maximize_value,
          minimize_value: p.minimize_value,
          violation: p.violation,
        },
      });
    }
    return out;
  }

  const top = Array.isArray(res.top) ? res.top : [];
  for (let i = 0; i < top.length; i += 1) {
    const row = Array.isArray(top[i]) ? (top[i] as unknown[]) : [];
    out.push({
      rank: i + 1,
      params: asRecord(row[0]),
      metrics: asRecord(row[1]),
    });
  }
  if (!out.length && allowBestFallback) {
    const p = asRecord(res.best_params);
    const m = asRecord(res.best_metrics);
    if (Object.keys(p).length && Object.keys(m).length)
      out.push({ rank: 1, params: p, metrics: m });
  }
  return out;
}

function readFilters(store: AnalysisStoreState): TopFilterValues {
  const out: TopFilterValues = {};
  const parse = (value: unknown): number | null => {
    const raw = text(value);
    if (!raw) return null;
    const n = Number(raw);
    return Number.isFinite(n) ? n : null;
  };
  const minPnl = parse(store.filterMinPnl);
  const minSharpe = parse(store.filterMinSharpe);
  const maxDd = parse(store.filterMaxDd);
  const minTrades = parse(store.filterMinTrades);
  const minExposure = parse(store.filterMinExposure);
  if (minPnl != null) out.minPnl = minPnl;
  if (minSharpe != null) out.minSharpe = minSharpe;
  if (maxDd != null) out.maxDd = maxDd;
  if (minTrades != null) out.minTrades = minTrades;
  if (minExposure != null) out.minExposure = minExposure;
  return out;
}

function applyTopFilters(
  rows: TopRowModel[],
  filters: TopFilterValues,
): TopRowModel[] {
  return rows.filter((row) => {
    const m = row.metrics || {};
    const pnl = num(m.pnl, 0);
    const sharpe = num(m.sharpe, 0);
    const dd = num(m.max_dd, 0);
    const trades = num(m.trades, 0);
    const expo = num(m.exposure, 0);
    if (filters.minPnl != null && pnl < filters.minPnl) return false;
    if (filters.minSharpe != null && sharpe < filters.minSharpe) return false;
    if (filters.maxDd != null && dd > filters.maxDd) return false;
    if (filters.minTrades != null && trades < filters.minTrades) return false;
    if (filters.minExposure != null && expo < filters.minExposure) return false;
    return true;
  });
}

function hasActiveTopFilters(filters: TopFilterValues): boolean {
  return Object.values(filters).some((value) => Number.isFinite(value));
}

function topRowIdFor(row: TopRowModel): string {
  return stableStringify(row.params || {});
}

function topBadgesFor(
  rank: number,
  metrics: Record<string, unknown>,
  filters: TopFilterValues,
  baselineSelected: string,
): string[] {
  const badges: string[] = [];
  const trades = num(metrics.trades, 0);
  const exposure = num(metrics.exposure, 0);
  const maxDd = num(metrics.max_dd, 0);
  if (rank === 1) badges.push("Current leader");
  if (hasActiveTopFilters(filters)) badges.push("Filtered shortlist");
  if (trades > 0 && trades < 20) badges.push("Low sample");
  if (maxDd > 0 && maxDd <= 0.12) badges.push("Controlled drawdown");
  else if (maxDd >= 0.25) badges.push("Drawdown heavy");
  if (exposure >= 0.2 && exposure <= 0.8) badges.push("Balanced exposure");
  else if (exposure >= 0.9) badges.push("Exposure concentrated");
  if (hasValidationResult()) badges.push("Validation available");
  if (baselineSelected) badges.push("Baseline armed");
  return badges.slice(0, 4);
}

function buildTopDecisionNote(
  rank: number,
  total: number,
  params: Record<string, unknown>,
  metrics: Record<string, unknown>,
  req: Record<string, unknown>,
  scoreContext: Record<string, unknown>,
): string {
  const scoreMode = scoreModeOf(req);
  const leadingMetricLabel = scoreMode ? scoreLabelOf(req) : objectiveOf(req);
  const leadingMetricValue = scoreMode
    ? fmtDisplay(
        scoreOf(
          metrics,
          objectiveOf(req),
          scoreMode,
          params,
          scoreContext,
        ),
        4,
      )
    : (() => {
        const objective = objectiveOf(req);
        if (objective === "Pnl") return fmtPctDisplay(metrics.pnl, 2);
        if (objective === "MaxDrawdown") return fmtDisplay(metrics.max_dd, 4);
        return fmtDisplay(
          objective === "Sortino"
            ? metrics.sortino
            : objective === "Calmar"
              ? metrics.calmar
              : objective === "Omega"
                ? metrics.omega
                : objective === "Volatility"
                  ? metrics.volatility
                  : metrics.sharpe,
          4,
        );
      })();
  const trades = num(metrics.trades, 0);
  const maxDd = num(metrics.max_dd, 0);
  const exposure = num(metrics.exposure, 0);
  const notes: string[] = [
    `Rank #${rank} of ${Math.max(total, rank)} by ${leadingMetricLabel}${leadingMetricValue !== "-" ? ` (${leadingMetricValue})` : ""}.`,
  ];
  if (trades > 0 && trades < 20) notes.push("Trade count is still light, so this candidate needs extra caution.")
  else if (trades >= 40) notes.push("Trade count is healthy enough for shortlist review.")
  if (maxDd > 0 && maxDd <= 0.12) notes.push("Drawdown stays controlled under the current ranking.")
  else if (maxDd >= 0.25) notes.push("Drawdown is elevated and should be pressure-tested before promotion.")
  if (exposure >= 0.2 && exposure <= 0.8) notes.push("Exposure remains balanced.")
  else if (exposure >= 0.9) notes.push("Exposure is concentrated.")
  return notes.join(" ");
}

function buildTopCandidateSnapshot(
  row: TopRowModel,
  req: Record<string, unknown>,
  filters: TopFilterValues,
  baselineSelected: string,
): TopPinnedCandidate {
  const params = row.params || {};
  const metrics = row.metrics || {};
  const objective = objectiveOf(req);
  const scoreMode = scoreModeOf(req);
  const badges = topBadgesFor(row.rank, metrics, filters, baselineSelected);
  const detailParts = [
    `Return ${fmtPctDisplay(metrics.pnl, 2)}`,
    `Sharpe ${fmtDisplay(metrics.sharpe, 4)}`,
    `Max DD ${fmtDisplay(metrics.max_dd, 4)}`,
    `Trades ${String(num(metrics.trades, 0))}`,
  ];
  return {
    id: topRowIdFor(row),
    rank: row.rank,
    title: `Rank #${row.rank} · ${formatBestTitle(params)}`,
    summary: detailParts.join(" · "),
    detail: [
      `Strategy ${kindFromReq(req)}`,
      objective ? `Objective ${objective}` : "",
      scoreMode ? `Score ${scoreLabelOf(req)}` : "",
    ]
      .filter((part) => part.length > 0)
      .join(" · "),
    badges,
    staleReason: "",
  };
}

function syncTopSelectionState(
  store: AnalysisStoreState,
  filteredRows: TopRowModel[],
  req: Record<string, unknown>,
  res: Record<string, unknown>,
): void {
  const selectedIndex = readTopRows(store).findIndex((row) => !!row?.selected);
  const selected = selectedIndex >= 0 ? filteredRows[selectedIndex] ?? null : null;
  const filters = readFilters(store);
  const baselineSelected = text(readHistoryStore()?.compareSelectedValue);
  if (!selected) {
    store.selectedTopTitle = filteredRows.length
      ? "No ranked candidate selected."
      : "No ranked candidates are available.";
    store.selectedTopSummary = filteredRows.length
      ? "No ranked candidate selected."
      : "No ranked candidates are available.";
    store.selectedTopDecisionNote =
      "Select a ranked candidate to lock a review target before moving into Candidate Detail or Validation.";
    store.selectedTopProvenanceChips = [];
    return;
  }
  const snapshot = buildTopCandidateSnapshot(
    selected,
    req,
    filters,
    baselineSelected,
  );
  store.selectedTopTitle = snapshot.title;
  store.selectedTopSummary = `Selected candidate ${selectedIndex + 1} of ${filteredRows.length}. ${snapshot.summary}.`;
  store.selectedTopDecisionNote = buildTopDecisionNote(
    selected.rank,
    filteredRows.length,
    selected.params || {},
    selected.metrics || {},
    req,
    scoreObservationContextOf(res),
  );
  store.selectedTopProvenanceChips = markRawEntity(snapshot.badges);
}

function syncPinnedTopCandidates(
  store: AnalysisStoreState,
  filteredRows: TopRowModel[],
  allRows: TopRowModel[],
  req: Record<string, unknown>,
): void {
  const filters = readFilters(store);
  const baselineSelected = text(readHistoryStore()?.compareSelectedValue);
  const allById = new Map(allRows.map((row) => [topRowIdFor(row), row] as const));
  const filteredIds = new Set(filteredRows.map((row) => topRowIdFor(row)));
  const current = Array.isArray(store.pinnedTopCandidates)
    ? (store.pinnedTopCandidates as TopPinnedCandidate[])
    : [];
  const reconciled = current.map((entry) => {
    const model = allById.get(text(entry.id));
    if (!model) {
      return markRawEntity({
        ...entry,
        staleReason: "Unavailable in the current shortlist.",
      });
    }
    const next = buildTopCandidateSnapshot(
      model,
      req,
      filters,
      baselineSelected,
    );
    return markRawEntity({
      ...next,
      staleReason: filteredIds.has(next.id) ? "" : "Hidden by the current shortlist filters.",
    });
  });
  store.pinnedTopCandidates = markRawEntity(reconciled.slice(0, 4));
  const staleCues = reconciled
    .map((entry) => text(entry.staleReason))
    .filter((entry, index, items) => entry && items.indexOf(entry) === index);
  store.topStaleCues = markRawEntity(staleCues);
}

function sortRows(
  rows: TopRowModel[],
  key: string,
  dir: SortDir,
  objective: string,
  scoreMode = "",
  scoreContext: Record<string, unknown> = {},
): TopRowModel[] {
  const d = dir === "desc" ? -1 : 1;
  const cmp = (a: number | string, b: number | string): number =>
    a === b ? 0 : a < b ? -1 : 1;
  const k = text(key || "rank");
  return [...rows].sort((ra, rb) => {
    const pa = ra.params || {};
    const pb = rb.params || {};
    const ma = ra.metrics || {};
    const mb = rb.metrics || {};
    const moa = ra.mo || {};
    const mob = rb.mo || {};
    if (k === "rank") return d * cmp(ra.rank, rb.rank);
    if (k === "fast") {
      const aType = String(pa.fast_ma_type || "").toLowerCase();
      const bType = String(pb.fast_ma_type || "").toLowerCase();
      if (aType !== bType) return d * cmp(aType, bType);
      return d * cmp(num(pa.fast_len, 0), num(pb.fast_len, 0));
    }
    if (k === "slow") {
      const aType = String(pa.slow_ma_type || "").toLowerCase();
      const bType = String(pb.slow_ma_type || "").toLowerCase();
      if (aType !== bType) return d * cmp(aType, bType);
      return d * cmp(num(pa.slow_len, 0), num(pb.slow_len, 0));
    }
    if (k === "pnl" || k === "ret_pct")
      return d * cmp(num(ma.pnl, 0), num(mb.pnl, 0));
    if (k === "sharpe") return d * cmp(num(ma.sharpe, 0), num(mb.sharpe, 0));
    if (k === "max_dd") return d * cmp(num(ma.max_dd, 0), num(mb.max_dd, 0));
    if (k === "trades") return d * cmp(num(ma.trades, 0), num(mb.trades, 0));
    if (k === "exposure")
      return d * cmp(num(ma.exposure, 0), num(mb.exposure, 0));
    if (k === "active_mean")
      return d * cmp(derivedActiveMean(ma), derivedActiveMean(mb));
    if (k === "active_downside_deviation")
      return (
        d *
        cmp(
          derivedActiveDownsideDeviation(ma),
          derivedActiveDownsideDeviation(mb),
        )
      );
    if (k === "active_sortino")
      return d * cmp(derivedActiveSortino(ma), derivedActiveSortino(mb));
    if (k === "drawdown_chronicity")
      return (
        d * cmp(derivedDrawdownChronicity(ma), derivedDrawdownChronicity(mb))
      );
    if (k === "downside_share")
      return d * cmp(derivedDownsideShare(ma), derivedDownsideShare(mb));
    if (k === "long_exposure")
      return d * cmp(derivedLongExposure(ma), derivedLongExposure(mb));
    if (k === "short_exposure")
      return d * cmp(derivedShortExposure(ma), derivedShortExposure(mb));
    if (k === "directional_bias")
      return d * cmp(derivedDirectionalBias(ma), derivedDirectionalBias(mb));
    if (k === "maximize_value")
      return (
        d *
        cmp(
          num(moa.maximize_value, Number.NEGATIVE_INFINITY),
          num(mob.maximize_value, Number.NEGATIVE_INFINITY),
        )
      );
    if (k === "minimize_value")
      return (
        d *
        cmp(
          num(moa.minimize_value, Number.POSITIVE_INFINITY),
          num(mob.minimize_value, Number.POSITIVE_INFINITY),
        )
      );
    if (k === "violation")
      return d * cmp(num(moa.violation, 0), num(mob.violation, 0));
    if (k === "score")
      return (
        d *
        cmp(
          scoreOf(ma, objective, scoreMode, pa, scoreContext),
          scoreOf(mb, objective, scoreMode, pb, scoreContext),
        )
      );
    return d * cmp(ra.rank, rb.rank);
  });
}

function topColumnsFor(
  req: Record<string, unknown>,
  res: Record<string, unknown>,
  sort: { key: string; dir: SortDir },
): TopColumn[] {
  const objectives = asRecord(req.objectives);
  const scoreMode = scoreModeOf(req);
  const scoreLabel =
    scoreMode === "balanced_risk_adjusted_10"
      ? "Score (BRA10)"
      : scoreMode === "adversarial_envelope_10"
        ? "Score (AE10)"
        : scoreMode === "support_shrunk_10"
          ? "Score (SS10)"
          : scoreMode === "active_sortino_10"
            ? "Score (AS10)"
            : scoreMode === "effective_sample_lcb_10"
              ? "Score (ELCB10)"
            : scoreMode === "omega_vs_sharpe_10"
              ? "Score (OVS10)"
              : scoreMode === "robust_margin_10"
                ? "Score (RM10)"
            : "Score";
  const columns: TopColumn[] = isMo(req, res)
    ? [
        { key: "rank", label: "#", sort: "", alignLeft: true },
        { key: "fast", label: "Fast", sort: "", alignLeft: true },
        { key: "slow", label: "Slow", sort: "", alignLeft: true },
        { key: "pnl", label: "PnL", sort: "", alignLeft: false },
        { key: "ret_pct", label: "Ret %", sort: "", alignLeft: false },
        { key: "sharpe", label: "Sharpe", sort: "", alignLeft: false },
        { key: "max_dd", label: "MaxDD", sort: "", alignLeft: false },
        { key: "trades", label: "Trades", sort: "", alignLeft: false },
        { key: "exposure", label: "Expo", sort: "", alignLeft: false },
        {
          key: "maximize_value",
          label: `Max(${String(objectives.maximize ?? req.objective ?? "maximize")})`,
          sort: "",
          alignLeft: false,
        },
        {
          key: "minimize_value",
          label: `Min(${String(objectives.minimize ?? "?")})`,
          sort: "",
          alignLeft: false,
        },
        { key: "violation", label: "Viol", sort: "", alignLeft: false },
      ]
    : [
        { key: "rank", label: "#", sort: "", alignLeft: true },
        { key: "fast", label: "Fast", sort: "", alignLeft: true },
        { key: "slow", label: "Slow", sort: "", alignLeft: true },
        { key: "pnl", label: "PnL", sort: "", alignLeft: false },
        { key: "ret_pct", label: "Ret %", sort: "", alignLeft: false },
        { key: "sharpe", label: "Sharpe", sort: "", alignLeft: false },
        { key: "max_dd", label: "MaxDD", sort: "", alignLeft: false },
        { key: "trades", label: "Trades", sort: "", alignLeft: false },
        { key: "exposure", label: "Expo", sort: "", alignLeft: false },
        ...(showDerivedScoreColumns(scoreMode)
          ? [
              {
                key: "active_mean",
                label: "ActMean",
                sort: "",
                alignLeft: false,
              },
              {
                key: "active_downside_deviation",
                label: "ActDnDev",
                sort: "",
                alignLeft: false,
              },
              {
                key: "active_sortino",
                label: "ActSort",
                sort: "",
                alignLeft: false,
              },
              {
                key: "drawdown_chronicity",
                label: "Chron",
                sort: "",
                alignLeft: false,
              },
              {
                key: "downside_share",
                label: "DnShare",
                sort: "",
                alignLeft: false,
              },
              {
                key: "long_exposure",
                label: "LongExpo",
                sort: "",
                alignLeft: false,
              },
              {
                key: "short_exposure",
                label: "ShortExpo",
                sort: "",
                alignLeft: false,
              },
              {
                key: "directional_bias",
                label: "DirBias",
                sort: "",
                alignLeft: false,
              },
            ]
          : []),
        { key: "score", label: scoreLabel, sort: "", alignLeft: false },
      ];
  for (const col of columns)
    if (text(col.key) === sort.key) col.sort = sort.dir;
  return columns;
}

function cacheSet<T>(
  cache: Map<string, T>,
  key: string,
  value: T,
  max = DRILL_CURVES_CACHE_MAX,
): void {
  if (!key) return;
  if (cache.has(key)) cache.delete(key);
  cache.set(key, value);
  while (cache.size > Math.max(8, Math.floor(Number(max || 96)))) {
    const oldest = cache.keys().next();
    if (oldest.done) break;
    cache.delete(oldest.value);
  }
}

async function cacheGetOrLoad<T>(
  cache: Map<string, T>,
  pending: Map<string, Promise<T>>,
  key: string,
  load: () => Promise<T>,
  max: number,
): Promise<CacheLoadResult<T>> {
  if (cache.has(key)) {
    return {
      value: cache.get(key) as T,
      cacheHit: true,
      sharedInFlight: false,
    };
  }

  const inflight = pending.get(key);
  if (inflight) {
    return {
      value: await inflight,
      cacheHit: false,
      sharedInFlight: true,
    };
  }

  const started = (async () => {
    const value = await load();
    cacheSet(cache, key, value, max);
    return value;
  })();
  pending.set(key, started);
  try {
    return {
      value: await started,
      cacheHit: false,
      sharedInFlight: false,
    };
  } finally {
    if (pending.get(key) === started) pending.delete(key);
  }
}

function buildTopRowModelsForCurrentFilters(
  store: AnalysisStoreState,
  req: Record<string, unknown>,
  res: Record<string, unknown>,
  sortedRows?: TopRowModel[],
): TopRowModel[] {
  const sort = readSortState(store);
  const sorted =
    sortedRows ??
    sortRows(
      collectTopRows(res, {
        allowBestFallback: requestedTopK(req) !== 0,
      }),
      sort.key,
      sort.dir,
      objectiveOf(req),
      scoreModeOf(req),
      scoreObservationContextOf(res),
    );
  const filters = readFilters(store);
  return applyTopFilters(sorted, filters);
}

function resolveTopRowModelAtIndex(
  store: AnalysisStoreState,
  index: number,
): TopRowModel | null {
  if (topRowsModelCache.length === readTopRows(store).length) {
    return topRowsModelCache[index] ?? null;
  }
  const run = readWorkflowRun();
  const req = asRecord(run?.req);
  const res = asRecord(run?.res);
  if (!Object.keys(req).length || !Object.keys(res).length) return null;
  const rebuilt = buildTopRowModelsForCurrentFilters(store, req, res);
  topRowsModelCache = rebuilt;
  return rebuilt[index] ?? null;
}

function nonEmptyRecord(value: unknown): Record<string, unknown> | null {
  const row = asRecord(value);
  return Object.keys(row).length ? row : null;
}

export function resolveExecutionTopRowSnapshot(
  preferredSourceRaw: unknown,
): AnalysisExecutionTopRowSnapshot {
  const preferredSource =
    text(preferredSourceRaw).toLowerCase() === "best" ? "best" : "selected";
  const run = readWorkflowRun();
  const runReq = nonEmptyRecord(run?.req);
  const runRes = nonEmptyRecord(run?.res);
  if (!runReq || !runRes) {
    throw new Error(
      "No optimization run loaded. Use Manual parameter source or run an optimization.",
    );
  }

  if (preferredSource === "best") {
    const bestParams = nonEmptyRecord(runRes.best_params);
    if (bestParams) {
      return {
        params: bestParams,
        metrics: asRecord(runRes.best_metrics),
        source: "best",
      };
    }

    const store = readAnalysisStore();
    if (store) {
      const filtered = buildTopRowModelsForCurrentFilters(
        store,
        runReq,
        runRes,
      );
      const top = filtered[0] ?? null;
      if (top) {
        return {
          params: asRecord(top.params),
          metrics: asRecord(top.metrics),
          source: "best_top_fallback",
        };
      }
    }

    const collected = collectTopRows(runRes);
    const fallback = collected[0] ?? null;
    if (fallback) {
      return {
        params: asRecord(fallback.params),
        metrics: asRecord(fallback.metrics),
        source: "best_top_fallback",
      };
    }
    throw new Error("No Top rows available. Run an optimization first.");
  }

  const store = readAnalysisStore();
  if (!store) {
    throw new Error(
      "No Top row selected. Click a row in the Top tab or switch Parameter source to Best row.",
    );
  }
  const rows = readTopRows(store);
  const selectedIndex = rows.findIndex((row) => !!row?.selected);
  if (selectedIndex < 0) {
    throw new Error(
      "No Top row selected. Click a row in the Top tab or switch Parameter source to Best row.",
    );
  }
  const selected = resolveTopRowModelAtIndex(store, selectedIndex);
  if (!selected) {
    throw new Error(
      "Selected Top row is unavailable (filters changed). Select a row again or use Best row.",
    );
  }
  return {
    params: asRecord(selected.params),
    metrics: asRecord(selected.metrics),
    source: "selected",
  };
}

function buildCandidateDetailInvocation(
  paramsRaw: Record<string, unknown>,
  runReqRaw: Record<string, unknown>,
  runResRaw?: Record<string, unknown> | null,
): DrillInvocation | null {
  const runReq = asRecord(runReqRaw);
  const runRes = asRecord(runResRaw);
  const params = asRecord(paramsRaw);
  const dataId = text(runReq.data_id ?? runReq.dataId);
  const fastMaType = text(params.fast_ma_type).toLowerCase();
  const slowMaType = text(params.slow_ma_type).toLowerCase();
  const fastLen = Math.max(0, Math.floor(num(params.fast_len, 0)));
  const slowLen = Math.max(0, Math.floor(num(params.slow_len, 0)));
  if (!dataId || !fastMaType || !slowMaType || fastLen <= 0 || slowLen <= 0)
    return null;

  const kind = normalizeStrategyKindRaw(
    runReq.strategy_kind ?? runReq.strategyKind,
  );
  const fastParams =
    asNumberMap(params.fast_ma_params ?? params.fastMaParams) ??
    resolveRunMaParams(runReq, "fast", fastMaType);
  const slowParams =
    asNumberMap(params.slow_ma_params ?? params.slowMaParams) ??
    resolveRunMaParams(runReq, "slow", slowMaType);
  const followupCacheKey =
    text(runRes.followup_cache_key ?? runRes.followupCacheKey) || null;

  const base: Record<string, unknown> = {
    data_id: dataId,
    backend_hint: runReq.backend || { backend: "CpuOnly" },
    params: {
      fast_len: fastLen,
      slow_len: slowLen,
      fast_ma_type: text(params.fast_ma_type),
      slow_ma_type: text(params.slow_ma_type),
    },
    ma_source: text(runReq.ma_source) || "close",
    fast_ma_params: fastParams,
    slow_ma_params: slowParams,
    strategy: runReq.strategy || runReq.base_strategy || {},
    followup_cache_key: followupCacheKey,
    bins: desiredDrillBins(),
  };

  if (isDoubleLikeKind(kind)) {
    return {
      runReq,
      kind,
      command: "compute_double_ma_drilldown",
      request: {
        ...base,
        backend_hint: runReq.backend || { backend: "CpuOnly" },
        halftrend_filter:
          runReq.halftrend_filter || runReq.halftrendFilter || null,
      },
    };
  }

  if (kind === "CompositeMo") {
    return {
      runReq,
      kind,
      command: "compute_composite_cpu_drilldown",
      request: {
        ...base,
        halftrend: params.halftrend,
        macz: params.macz || {},
        ott: params.ott || {},
        signal_mode:
          params.signal_mode ||
          params.signalMode ||
          runReq.signal_mode ||
          runReq.signalMode ||
          "hard_gate",
        strategy: params.strategy || base.strategy,
      },
    };
  }

  return {
    runReq,
    kind,
    command: "compute_composite_cpu_drilldown",
    request: {
      ...base,
      halftrend: runReq.halftrend,
      macz: runReq.macz || {},
      ott: runReq.ott || {},
      asr: runReq.asr || null,
      vemr: runReq.vemr || null,
      signal_mode: runReq.signal_mode || runReq.signalMode || "hard_gate",
    },
  };
}

export function buildExecutionStrategyFromRunContext(
  paramsRaw: Record<string, unknown>,
  runReqRaw: Record<string, unknown>,
): AnalysisExecutionStrategySnapshot | null {
  const invocation = buildCandidateDetailInvocation(paramsRaw, runReqRaw, null);
  if (!invocation) return null;
  const strategyKind = normalizeStrategyKindRaw(invocation.kind);
  const kind =
    invocation.command === "compute_double_ma_drilldown"
      ? "double_ma"
      : "composite_cpu";
  return {
    strategyKind,
    strategy: {
      kind,
      req: invocation.request,
    },
  };
}

function buildLocalStabilityInvocation(
  paramsRaw: Record<string, unknown>,
  invocation: DrillInvocation,
): StabilityInvocation | null {
  const runReq = invocation.runReq;
  const objective = objectiveOf(runReq);
  const scoreMode = scoreModeOf(runReq);
  const base: Record<string, unknown> = {
    data_id: invocation.request.data_id,
    params: invocation.request.params,
    ma_source: invocation.request.ma_source,
    fast_ma_params: invocation.request.fast_ma_params,
    slow_ma_params: invocation.request.slow_ma_params,
    strategy: invocation.request.strategy,
    objective,
    radius: 1,
    step: 1,
  };
  if (invocation.command === "compute_double_ma_drilldown") {
    return {
      command: "compute_double_ma_local_stability",
      request: {
        ...base,
        backend_hint: invocation.request.backend_hint,
        followup_cache_key: invocation.request.followup_cache_key,
        ...(scoreMode ? { score_mode: scoreMode } : {}),
        halftrend_filter: invocation.request.halftrend_filter ?? null,
      },
    };
  }

  const kind = normalizeStrategyKindRaw(
    runReq.strategy_kind ?? runReq.strategyKind,
  );
  const params = asRecord(paramsRaw);
  if (kind === "CompositeMo") {
    return {
      command: "compute_composite_cpu_local_stability",
      request: {
        ...base,
        backend: invocation.request.backend_hint || { backend: "CpuOnly" },
        halftrend: params.halftrend,
        macz: params.macz || {},
        ott: params.ott || {},
        signal_mode:
          params.signal_mode ||
          params.signalMode ||
          runReq.signal_mode ||
          runReq.signalMode ||
          "hard_gate",
        strategy:
          params.strategy || runReq.strategy || runReq.base_strategy || {},
      },
    };
  }
  return {
    command: "compute_composite_cpu_local_stability",
    request: {
      ...base,
      backend: invocation.request.backend_hint || { backend: "CpuOnly" },
      halftrend: runReq.halftrend,
      macz: runReq.macz || {},
      ott: runReq.ott || {},
      asr: runReq.asr || null,
      vemr: runReq.vemr || null,
      signal_mode: runReq.signal_mode || runReq.signalMode || "hard_gate",
    },
  };
}

function buildDrillMeta(
  params: Record<string, unknown>,
  metrics: Record<string, unknown>,
  row: TopRowModel,
  curves: unknown,
): string {
  const curvesRow = asRecord(curves);
  const eq = asRecord(curvesRow.equity);
  const metricPart = formatDrillMetricsSummary(metrics);
  const moPart = formatDrillMoSummary(row.mo);
  const parts = [
    `${text(params.fast_ma_type) || "?"}(${num(params.fast_len, 0) || "?"}) vs ${text(params.slow_ma_type) || "?"}(${num(params.slow_len, 0) || "?"})`,
  ];
  if (metricPart) parts.push(metricPart);
  if (moPart) parts.push(moPart);
  parts.push(
    `bins=${num(eq.bins, 0)} start_t=${num(eq.start_t, 0)} end_t=${num(eq.end_t, 0)}`,
  );
  return parts.join(" | ");
}

function buildStabilityMeta(stabilityResult: unknown): string {
  const sr = asRecord(stabilityResult);
  const summary = asRecord(sr.summary);
  return `objective=${text(sr.objective) || "?"} radius=${num(sr.radius, 0)} step=${num(sr.step, 0)} points=${Array.isArray(sr.points) ? sr.points.length : 0} | score: base=${fmt(summary.base, 6)} median=${fmt(summary.median, 6)} min=${fmt(summary.min, 6)} max=${fmt(summary.max, 6)}`;
}

function applyDrillCompletionState(
  store: AnalysisStoreState | null,
  drillMeta: string,
  drillStatus: string,
): void {
  if (!store) return;
  store.drillStatus = drillStatus;
  store.drillMeta = drillMeta;
  store.topSelectionGuardrail = "—";
  markAnalysisVue(store);
}

function applyStabilityCompletionState(
  store: AnalysisStoreState | null,
  stabilityResult: unknown | null,
  stabilityMeta: string,
  stabilityStatus: string,
): void {
  if (!store || !stabilityResult) return;
  renderStabilityDom(stabilityResult, store);
  store.stabilityMeta = stabilityMeta;
  store.stabilityStatus = stabilityStatus;
  markAnalysisVue(store);
}

function captureDrillRenderState(
  run: WorkflowRunState | null,
  row: TopRowModel,
  params: Record<string, unknown>,
  metrics: Record<string, unknown>,
  curves: unknown,
  bars: DrillPriceBar[] | null,
  stabilityResult: unknown | null,
  cachedDrill: boolean,
  cachedStability: boolean,
): void {
  const drillStatus = cachedDrill ? "Done (cached)." : "Done.";
  const stabilityStatus = stabilityResult
    ? cachedStability
      ? "Done (cached)."
      : "Done."
    : "-";
  drillRenderState = {
    runIdentity: workflowRunIdentity(run),
    rowId: topRowIdFor(row),
    row,
    params,
    metrics,
    curves,
    bars,
    stabilityResult,
    drillMeta: buildDrillMeta(params, metrics, row, curves),
    drillStatus,
    stabilityMeta: stabilityResult ? buildStabilityMeta(stabilityResult) : "-",
    stabilityStatus,
  };
}

function publishDrillChartModels(
  store: AnalysisStoreState | null,
  curves: unknown,
  bars: DrillPriceBar[] | null,
): void {
  if (!store) return;
  const chartModel = buildDrillChartModel(curves, bars);
  const tradeTableModel = buildDrillTradeTableModel({
    chartModel,
    selectedTradeIdx: -1,
    rangeStart: 0,
    maxRows: 200,
  });
  store.drillChartModel = markRaw(chartModel);
  store.drillTradeTableModel = markRaw(tradeTableModel);
}

async function runVueDrillPipeline(
  row: TopRowModel,
): Promise<boolean> {
  const store = readAnalysisStore();
  const run = readWorkflowRun();
  const runReq = asRecord(run?.req);
  const runRes = asRecord(run?.res);
  const params = asRecord(row.params);
  const metrics = asRecord(row.metrics);
  const invocation = buildCandidateDetailInvocation(params, runReq, runRes);
  if (!invocation) return false;

  drillSeq = (drillSeq + 1) >>> 0;
  const seq = drillSeq;
  if (store) {
    store.drillStatus = "Computing candidate detail…";
    store.stabilityStatus = "-";
    markAnalysisVue(store);
  }
  setStatus(store, "Computing candidate detail…");
  try {
    const drillKey = requestCacheKey(invocation.command, invocation.request);
    const drillLoad = await cacheGetOrLoad(
      drillCurvesCache,
      drillCurvesPending,
      drillKey,
      async () => await invokeCommand(invocation.command, { req: invocation.request }),
      DRILL_CURVES_CACHE_MAX,
    );
    const curves = drillLoad.value;
    const cachedDrill = drillLoad.cacheHit || drillLoad.sharedInFlight;
    if (seq !== drillSeq) return true;

    let bars: DrillPriceBar[] | null = null;
    const dataId = text(runReq.data_id ?? runReq.dataId);
    if (dataId) {
      try {
        bars = await fetchDrillPriceBars(dataId);
      } catch {
        bars = null;
      }
    }
    if (seq !== drillSeq) return true;
    publishDrillChartModels(store, curves, bars);
    renderDrillDomFallback(curves, params, metrics, row);
    if (seq !== drillSeq) return true;

    let drillMeta = "-";
    const drillStatus = cachedDrill ? "Done (cached)." : "Done.";
    if (store) {
      store.drillStatus = drillStatus;
      const curvesRow = asRecord(curves);
      const eq = asRecord(curvesRow.equity);
      const metricPart = formatDrillMetricsSummary(metrics);
      const moPart = formatDrillMoSummary(row.mo);
      const parts = [
        `${text(params.fast_ma_type) || "?"}(${num(params.fast_len, 0) || "?"}) vs ${text(params.slow_ma_type) || "?"}(${num(params.slow_len, 0) || "?"})`,
      ];
      if (metricPart) parts.push(metricPart);
      if (moPart) parts.push(moPart);
      parts.push(
        `bins=${num(eq.bins, 0)} start_t=${num(eq.start_t, 0)} end_t=${num(eq.end_t, 0)}`,
      );
      drillMeta = parts.join(" | ");
      store.drillMeta = drillMeta;
      store.topSelectionGuardrail = "—";
      markAnalysisVue(store);
    }
    setStatus(
      store,
      cachedDrill ? "Candidate detail done (cached)." : "Candidate detail done.",
    );

    const stability = buildLocalStabilityInvocation(params, invocation);
    if (!stability) {
      captureDrillRenderState(
        run,
        row,
        params,
        metrics,
        curves,
        bars,
        null,
        cachedDrill,
        false,
      );
      return true;
    }
    const stabilityKey = requestCacheKey(stability.command, stability.request);
    if (store) {
      store.stabilityStatus = "Computing stability…";
      markAnalysisVue(store);
    }
    const stabilityLoad = await cacheGetOrLoad(
      stabilityCache,
      stabilityPending,
      stabilityKey,
      async () => await invokeCommand(stability.command, { req: stability.request }),
      STABILITY_CACHE_MAX,
    );
    const stabilityRes = stabilityLoad.value;
    const cachedStability =
      stabilityLoad.cacheHit || stabilityLoad.sharedInFlight;
    if (seq !== drillSeq) return true;
    renderStabilityDom(stabilityRes, store);
    let stabilityMeta = "-";
    const stabilityStatus = cachedStability ? "Done (cached)." : "Done.";
    if (store) {
      const sr = asRecord(stabilityRes);
      const summary = asRecord(sr.summary);
      stabilityMeta = `objective=${text(sr.objective) || "?"} radius=${num(sr.radius, 0)} step=${num(sr.step, 0)} points=${Array.isArray(sr.points) ? sr.points.length : 0} | score: base=${fmt(summary.base, 6)} median=${fmt(summary.median, 6)} min=${fmt(summary.min, 6)} max=${fmt(summary.max, 6)}`;
      store.stabilityMeta = stabilityMeta;
      store.stabilityStatus = stabilityStatus;
      markAnalysisVue(store);
    }
    captureDrillRenderState(
      run,
      row,
      params,
      metrics,
      curves,
      bars,
      stabilityRes,
      cachedDrill,
      cachedStability,
    );
    return true;
  } catch (error) {
    if (seq !== drillSeq) return true;
    drillRenderState = null;
    const message = `Candidate detail failed: ${String(error)}`;
    if (store) {
      store.drillStatus = message;
      markAnalysisVue(store);
    }
    setStatus(store, message);
    return false;
  }
}

export function refreshTopTableFromWorkflowRun(): boolean {
  const store = readAnalysisStore();
  const run = readWorkflowRun();
  if (!store || !run) return false;
  const req = asRecord(run.req);
  const res = asRecord(run.res);
  if (!Object.keys(res).length) return false;

  const objective = objectiveOf(req);
  const scoreMode = scoreModeOf(req);
  const scoreContext = scoreObservationContextOf(res);
  const sort = readSortState(store);
  const topK = requestedTopK(req);
  const collected = collectTopRows(res, { allowBestFallback: topK !== 0 });
  const sorted = sortRows(
    collected,
    sort.key,
    sort.dir,
    objective,
    scoreMode,
    scoreContext,
  );
  const filtered = buildTopRowModelsForCurrentFilters(store, req, res, sorted);
  const filters = readFilters(store);
  topRowsModelCache = filtered;
  topRowsAllModelCache = sorted;
  const selectedOldRows = readTopRows(store);
  const previousSelected = selectedOldRows.find((row) => !!row?.selected);
  const previousSelectedId = text(previousSelected?.rowId);
  const selectedOld = selectedOldRows.findIndex((row) => !!row?.selected);
  const selectedByIdIndex = previousSelectedId
    ? filtered.findIndex((row) => topRowIdFor(row) === previousSelectedId)
    : -1;
  const selectedIndex = filtered.length
    ? selectedByIdIndex >= 0
      ? selectedByIdIndex
      : selectedOld >= 0
      ? Math.min(selectedOld, filtered.length - 1)
      : 0
    : -1;
  const mo = isMo(req, res);

  const rows: TopRow[] = filtered.length
    ? filtered.map((row, index) => {
        const p = row.params || {};
        const m = row.metrics || {};
        const d = row.mo || {};
        const cells: TopRowCell[] = [
          { text: String(row.rank), className: "left mono", title: "" },
          {
            text: `${String(p.fast_ma_type || "?")}(${String(p.fast_len || "?")})`,
            className: "left mono",
            title: "",
          },
          {
            text: `${String(p.slow_ma_type || "?")}(${String(p.slow_len || "?")})`,
            className: "left mono",
            title: "",
          },
          {
            text: fmtDisplay(m.pnl, 4),
            className: "mono",
            title: fmt(m.pnl, 6),
          },
          {
            text: fmtPctDisplay(m.pnl, 2),
            className: "mono",
            title: fmtPct(m.pnl, 2),
          },
          {
            text: fmtDisplay(m.sharpe, 4),
            className: "mono",
            title: fmt(m.sharpe, 6),
          },
          {
            text: fmtDisplay(m.max_dd, 4),
            className: "mono",
            title: fmt(m.max_dd, 6),
          },
          { text: String(num(m.trades, 0)), className: "mono", title: "" },
          {
            text: fmtDisplay(m.exposure, 4),
            className: "mono",
            title: fmt(m.exposure, 4),
          },
          ...(showDerivedScoreColumns(scoreMode)
            ? [
                {
                  text: fmtDisplay(derivedActiveMean(m), 4),
                  className: "mono",
                  title: fmt(derivedActiveMean(m), 6),
                },
                {
                  text: fmtDisplay(derivedActiveDownsideDeviation(m), 4),
                  className: "mono",
                  title: fmt(derivedActiveDownsideDeviation(m), 6),
                },
                {
                  text: fmtDisplay(derivedActiveSortino(m), 4),
                  className: "mono",
                  title: fmt(derivedActiveSortino(m), 6),
                },
                {
                  text: fmtDisplay(derivedDrawdownChronicity(m), 4),
                  className: "mono",
                  title: fmt(derivedDrawdownChronicity(m), 6),
                },
                {
                  text: fmtDisplay(derivedDownsideShare(m), 4),
                  className: "mono",
                  title: fmt(derivedDownsideShare(m), 6),
                },
                {
                  text: fmtDisplay(derivedLongExposure(m), 4),
                  className: "mono",
                  title: fmt(derivedLongExposure(m), 6),
                },
                {
                  text: fmtDisplay(derivedShortExposure(m), 4),
                  className: "mono",
                  title: fmt(derivedShortExposure(m), 6),
                },
                {
                  text: fmtDisplay(derivedDirectionalBias(m), 4),
                  className: "mono",
                  title: fmt(derivedDirectionalBias(m), 6),
                },
              ]
            : []),
        ];
        if (!mo)
          cells.push({
            text: fmtDisplay(
              scoreOf(m, objective, scoreMode, p, scoreContext),
              4,
            ),
            className: "mono",
            title: fmt(
              scoreOf(m, objective, scoreMode, p, scoreContext),
              6,
            ),
          });
        else {
          cells.push({
            text: fmtDisplay(d.maximize_value, 4),
            className: "mono",
            title: fmt(d.maximize_value, 6),
          });
          cells.push({
            text: fmtDisplay(d.minimize_value, 4),
            className: "mono",
            title: fmt(d.minimize_value, 6),
          });
          cells.push({
            text: fmtDisplay(d.violation, 4),
            className: "mono",
            title: fmt(d.violation, 6),
          });
        }
        return {
          rowId: topRowIdFor(row),
          selected: index === selectedIndex,
          cells,
        };
      })
    : [
        {
          rowId: "top_empty_state",
          selected: false,
          cells: [
            {
              text: sorted.length
                ? "No rows match the current filters."
                : "No rows.",
              className: "left muted",
              title: "",
            },
          ],
        },
      ];

  store.topColumns = topColumnsFor(req, res, sort);
  store.topTableRows = rows;
  store.topRows = filtered.length;
  syncTopSelectionState(store, filtered, req, res);
  syncPinnedTopCandidates(store, filtered, sorted, req);
  if (!filtered.length && topK === 0 && !Array.isArray(res.pareto)) {
    const combos = combosOf(res);
    store.topFilterMeta =
      combos > 0
        ? `Top K disabled for this run (top_k=0). Evaluated ${combos} combinations; only the best candidate was returned.`
        : "Top K disabled for this run (top_k=0). Only the best candidate was returned.";
    store.topSelectionGuardrail =
      "Set Advanced > Top K above 0, then rerun optimization to inspect a ranked shortlist.";
  } else {
    store.topFilterMeta = `showing ${filtered.length}/${sorted.length} rows`;
    if (filtered.length && previousSelectedId && selectedByIdIndex < 0) {
      store.topSelectionGuardrail = hasActiveTopFilters(filters)
        ? "The prior selection fell out of the current shortlist filters; focus moved to the nearest available candidate."
        : "The prior selection no longer exists in the current shortlist; focus moved to the nearest available candidate."
    } else {
      store.topSelectionGuardrail = filtered.length
        ? hasValidationResult()
          ? `Validation context loaded for ${text(req.strategy_kind ?? req.strategyKind) || "DoubleMa"}. Final selection should still include holdout/meta confirmation.`
          : "Selection-bias guardrail: these are optimization-only winners. Run Validation before treating them as robust."
        : "—";
    }
  }
  store.canClearTopFilters = Object.keys(filters).length > 0;
  markAnalysisVue(store);
  return true;
}

export function refreshAnalysisFromWorkflowRun(): boolean {
  const store = readAnalysisStore();
  const run = readWorkflowRun();
  if (!store || !run) return false;
  const req = asRecord(run.req);
  const res = asRecord(run.res);
  if (!Object.keys(res).length) return false;
  const runIdentity = workflowRunIdentity(run);
  if (runIdentity !== lastAnalysisRunIdentity) {
    if (lastAnalysisRunIdentity) invalidateAnalysisExecutionCaches();
    lastAnalysisRunIdentity = runIdentity;
  }
  if (kindFromReq(req) === "MlSignal") {
    store.bestKicker = "ML signal artifact";
    store.bestTitle = "Score cache / policy validation";
    store.bestMeta =
      "MlSignal artifacts remain outside the standard optimization analysis pipeline.";
    store.bestSummary =
      "MlSignal run loaded.\nUse workflow/history details or the ML workspace to inspect score-cache and validation results.";
    store.bestStats = [];
    store.topColumns = [];
    store.topTableRows = [];
    store.topRows = 0;
    store.topFilterMeta = "MlSignal does not populate optimizer Top rows.";
    store.topSelectionGuardrail =
      "Run or load a conventional optimization artifact to repopulate analysis views.";
    store.paretoRows = [];
    store.paretoMeta = "—";
    store.paretoTruthNote =
      "Pareto points show tradeoffs, not proof of robustness.";
    store.paretoRepresentativeSummary = "—";
    store.paretoConfidenceNote =
      "Validation is still required before trusting a frontier candidate.";
    store.heatmapMeta = "—";
    store.heatmapStatus = "MlSignal artifacts do not emit optimizer heatmaps.";
    markAnalysisVue(store);
    return true;
  }
  refreshOverviewFromWorkflowRunStore(store, req, res);
  refreshTopTableFromWorkflowRun();
  refreshParetoFromWorkflowRun();
  refreshHeatmapFromWorkflowRun();
  markAnalysisVue(store);
  return true;
}

function setStatus(store: AnalysisStoreState | null, message: string): void {
  if (!store) return;
  store.statusLine = text(message) || "Ready.";
  markAnalysisVue(store);
}

function kindFromReq(req: Record<string, unknown>): string {
  return text(req.strategy_kind ?? req.strategyKind) || "DoubleMa";
}

function combosOf(res: Record<string, unknown>): number {
  return num(res.num_combos ?? res.total_evaluations, 0);
}

function runtimeMsOf(res: Record<string, unknown>): number {
  return num(res.runtime_ms, 0);
}

function formatBestTitle(params: Record<string, unknown>): string {
  const fastType = text(params.fast_ma_type).toUpperCase() || "?";
  const fastLen = num(params.fast_len, 0);
  const slowType = text(params.slow_ma_type).toUpperCase() || "?";
  const slowLen = num(params.slow_len, 0);
  return `${fastType} ${fastLen > 0 ? fastLen : "?"} vs ${slowType} ${slowLen > 0 ? slowLen : "?"}`;
}

function buildBestStats(
  metrics: Record<string, unknown>,
  scoreMode = "",
): Array<{ label: string; value: string; tone: string }> {
  const rows = [
    { label: "PnL", value: fmt(metrics.pnl, 6), tone: "positive" },
    { label: "Return", value: fmtPct(metrics.pnl, 2), tone: "neutral" },
    { label: "Sharpe", value: fmt(metrics.sharpe, 6), tone: "positive" },
    {
      label: "Downside Dev",
      value: fmt(metrics.downside_deviation, 6),
      tone: "neutral",
    },
    { label: "Max DD", value: fmt(metrics.max_dd, 6), tone: "danger" },
    { label: "Ulcer", value: fmt(metrics.ulcer_index, 6), tone: "danger" },
    { label: "Martin", value: fmt(metrics.martin_ratio, 6), tone: "positive" },
    { label: "Trades", value: String(num(metrics.trades, 0)), tone: "neutral" },
    { label: "Exposure", value: fmt(metrics.exposure, 4), tone: "neutral" },
  ];
  const kappaTau = num(metrics.kappa_tau, Number.NaN);
  if (Number.isFinite(kappaTau) && Math.abs(kappaTau) > 1e-12) {
    rows.push({ label: "Kappaτ", value: fmt(kappaTau, 6), tone: "positive" });
  }
  if (showDerivedScoreColumns(scoreMode)) {
    rows.push(
      {
        label: "Act Mean",
        value: fmt(derivedActiveMean(metrics), 6),
        tone: "neutral",
      },
      {
        label: "Act Dn Dev",
        value: fmt(derivedActiveDownsideDeviation(metrics), 6),
        tone: "neutral",
      },
      {
        label: "Act Sort",
        value: fmt(derivedActiveSortino(metrics), 6),
        tone: "positive",
      },
      {
        label: "Chronicity",
        value: fmt(derivedDrawdownChronicity(metrics), 6),
        tone: "danger",
      },
      {
        label: "Dn Share",
        value: fmt(derivedDownsideShare(metrics), 6),
        tone: "neutral",
      },
      {
        label: "Long Expo",
        value: fmt(derivedLongExposure(metrics), 6),
        tone: "neutral",
      },
      {
        label: "Short Expo",
        value: fmt(derivedShortExposure(metrics), 6),
        tone: "neutral",
      },
      {
        label: "Dir Bias",
        value: fmt(derivedDirectionalBias(metrics), 6),
        tone: "neutral",
      },
    );
  }
  return rows.filter((entry) => entry.value && entry.value !== "-");
}

function refreshOverviewFromWorkflowRunStore(
  store: AnalysisStoreState,
  req: Record<string, unknown>,
  res: Record<string, unknown>,
): void {
  const best = pickBest(res, req);
  const params = best.params || {};
  const metrics = best.metrics || {};
  const strategyKind =
    text(req.strategy_kind ?? req.strategyKind) || "DoubleMa";
  const objective = objectiveOf(req);
  const scoreMode = scoreModeOf(req);
  const extraShortlistObjectives = extraShortlistObjectivesOf(req);
  const shortlistConfidenceMode = shortlistConfidenceModeOf(req);
  const shortlistUnionMode = shortlistUnionModeOf(req);
  const shortlistSliceMode = shortlistSliceModeOf(req);
  const shortlistSliceBenchmark = shortlistSliceBenchmarkOf(req);
  const shortlistNeighborhood = shortlistNeighborhoodSummaryOf(req);
  const hurdleTau = hurdleTauOf(req);
  const comboCount = combosOf(res);
  const runtimeMs = runtimeMsOf(res);
  const metaParts = [
    `strategy=${strategyKind}`,
    objective ? `objective=${objective}` : "",
    extraShortlistObjectives.length > 0
      ? `extra_shortlist=${extraShortlistObjectives.join(",")}`
      : "",
    shortlistConfidenceMode
      ? `shortlist_confidence=${shortlistConfidenceMode}`
      : "",
    shortlistUnionMode ? `shortlist_union=${shortlistUnionMode}` : "",
    shortlistSliceMode ? `shortlist_slice=${shortlistSliceMode}` : "",
    shortlistSliceBenchmark ? `slice_benchmark=${shortlistSliceBenchmark}` : "",
    shortlistNeighborhood ? `slice_neighborhood=${shortlistNeighborhood}` : "",
    hurdleTau ? `hurdle_tau=${hurdleTau}` : "",
    comboCount > 0 ? `combos=${comboCount}` : "",
    runtimeMs > 0 ? `runtime=${runtimeMs}ms` : "",
  ].filter((part) => part.length > 0);

  store.bestKicker = isMo(req, res) ? "Pareto leader" : "Best candidate";
  store.bestTitle = formatBestTitle(params);
  store.bestMeta = metaParts.join(" · ") || "Optimization result loaded.";
  store.bestStats = markRawEntity(buildBestStats(metrics, scoreMode));
  const summaryParts = [
    `strategy=${strategyKind}`,
    `title=${formatBestTitle(params)}`,
    objective ? `objective=${objective}` : "",
    scoreMode ? `score_mode=${scoreMode}` : "",
    extraShortlistObjectives.length > 0
      ? `extra_shortlist_objectives=${extraShortlistObjectives.join(",")}`
      : "",
    shortlistConfidenceMode
      ? `shortlist_confidence_mode=${shortlistConfidenceMode}`
      : "",
    shortlistUnionMode ? `shortlist_union_mode=${shortlistUnionMode}` : "",
    shortlistSliceMode ? `shortlist_slice_mode=${shortlistSliceMode}` : "",
    shortlistSliceBenchmark
      ? `shortlist_slice_benchmark=${shortlistSliceBenchmark}`
      : "",
    shortlistNeighborhood
      ? `shortlist_neighborhood=${shortlistNeighborhood}`
      : "",
    hurdleTau ? `hurdle_tau=${hurdleTau}` : "",
    `pnl=${fmt(metrics.pnl, 6)}`,
    `return=${fmtPct(metrics.pnl, 2)}`,
    `sharpe=${fmt(metrics.sharpe, 6)}`,
    `downside_dev=${fmt(metrics.downside_deviation, 6)}`,
    `max_dd=${fmt(metrics.max_dd, 6)}`,
    `ulcer_index=${fmt(metrics.ulcer_index, 6)}`,
    `martin_ratio=${fmt(metrics.martin_ratio, 6)}`,
    `trades=${num(metrics.trades, 0)}`,
    `exposure=${fmt(metrics.exposure, 4)}`,
    comboCount > 0 ? `combos=${comboCount}` : "",
    runtimeMs > 0 ? `runtime_ms=${runtimeMs}` : "",
  ];
  const kappaTau = num(metrics.kappa_tau, Number.NaN);
  if (Number.isFinite(kappaTau) && Math.abs(kappaTau) > 1e-12) {
    summaryParts.push(`kappa_tau=${fmt(kappaTau, 6)}`);
  }
  if (showDerivedScoreColumns(scoreMode)) {
    summaryParts.push(
      `active_mean=${fmt(derivedActiveMean(metrics), 6)}`,
      `active_downside_deviation=${fmt(derivedActiveDownsideDeviation(metrics), 6)}`,
      `active_sortino=${fmt(derivedActiveSortino(metrics), 6)}`,
      `drawdown_chronicity=${fmt(derivedDrawdownChronicity(metrics), 6)}`,
      `downside_share=${fmt(derivedDownsideShare(metrics), 6)}`,
      `long_exposure=${fmt(derivedLongExposure(metrics), 6)}`,
      `short_exposure=${fmt(derivedShortExposure(metrics), 6)}`,
      `directional_bias=${fmt(derivedDirectionalBias(metrics), 6)}`,
    );
  }
  store.bestSummary = summaryParts.filter((part) => part.length > 0).join("\n");
}

function pickBest(
  res: Record<string, unknown>,
  req: Record<string, unknown>,
): { params: Record<string, unknown>; metrics: Record<string, unknown> } {
  if (isMo(req, res) && Array.isArray(res.pareto) && res.pareto.length > 0) {
    const rows = collectTopRows(res);
    if (rows.length > 0) {
      const top = rows[0];
      if (top) return { params: top.params || {}, metrics: top.metrics || {} };
    }
  }
  const bestParams = asRecord(res.best_params);
  const bestMetrics = asRecord(res.best_metrics);
  if (Object.keys(bestParams).length && Object.keys(bestMetrics).length) {
    return { params: bestParams, metrics: bestMetrics };
  }
  const top = Array.isArray(res.top) ? res.top : [];
  const first = Array.isArray(top[0]) ? (top[0] as unknown[]) : [];
  return { params: asRecord(first[0]), metrics: asRecord(first[1]) };
}

function paretoAxisOptionsFor(
  req: Record<string, unknown>,
  res: Record<string, unknown>,
): Array<{ value: string; label: string; disabled: boolean }> {
  return buildParetoAxisOptions(isMo(req, res));
}

function paretoAxisValue(
  row: TopRowModel,
  req: Record<string, unknown>,
  res: Record<string, unknown>,
  keyRaw: unknown,
): number {
  const key = text(keyRaw || "pnl") || "pnl";
  const scoreValue =
    key === "score"
      ? scoreOf(
          row.metrics || {},
          objectiveOf(req),
          scoreModeOf(req),
          row.params || {},
          scoreObservationContextOf(res),
        )
      : Number.NaN;
  return modelParetoAxisValue(row, key, scoreValue);
}

function selectedParetoRowModel(store: AnalysisStoreState): TopRowModel | null {
  const selectedIndex = readTopRows(store).findIndex((row) => !!row?.selected);
  if (selectedIndex < 0) return null;
  return resolveTopRowModelAtIndex(store, selectedIndex);
}

function buildParetoRenderModelForRun(
  store: AnalysisStoreState,
  req: Record<string, unknown>,
  res: Record<string, unknown>,
): ParetoRenderModel {
  const rows = collectTopRows(res, { allowBestFallback: true }).slice(0, 512);
  const xKey = text(store.paretoX || "max_dd") || "max_dd";
  const yKey = text(store.paretoY || "pnl") || "pnl";
  const selected = selectedParetoRowModel(store);
  const model = buildParetoViewModel({
    rows,
    xKey,
    yKey,
    axisValueFor: (row, key) => paretoAxisValue(row, req, res, key),
    rowIdFor: (row) => topRowIdFor(row),
    rankFor: (row) => row.rank,
    selectedRowId: selected ? topRowIdFor(selected) : null,
    selectedRank: selected?.rank ?? null,
    hasValidationResult: hasValidationResult(),
    formatValue: fmtDisplay,
  });
  return toParetoRenderModel(model);
}

function updateParetoModel(
  store: AnalysisStoreState,
  req: Record<string, unknown>,
  res: Record<string, unknown>,
): boolean {
  const model = buildParetoRenderModelForRun(store, req, res);
  store.paretoModel = markRaw(model);
  store.paretoWrapVisible = model.status === "ready";
  store.paretoTruthNote = model.truthNote;
  store.paretoRepresentativeSummary = model.representativeSummary;
  store.paretoConfidenceNote = model.confidenceNote;
  store.paretoMeta = model.meta;
  return true;
}

export function refreshParetoFromWorkflowRun(): boolean {
  const store = readAnalysisStore();
  const run = readWorkflowRun();
  if (!store || !run) return false;
  const req = asRecord(run.req);
  const res = asRecord(run.res);
  if (!Object.keys(res).length) return false;
  const options = paretoAxisOptionsFor(req, res);
  store.paretoXOptions = options;
  store.paretoYOptions = options;
  const validKeys = new Set(options.map((option) => option.value));
  if (!validKeys.has(text(store.paretoX)))
    store.paretoX = isMo(req, res) ? "maximize_value" : "max_dd";
  if (!validKeys.has(text(store.paretoY)))
    store.paretoY = isMo(req, res) ? "minimize_value" : "pnl";
  const ok = updateParetoModel(store, req, res);
  markAnalysisVue(store);
  return ok;
}

export async function drillParetoPoint(rowIdRaw: unknown): Promise<boolean> {
  const rowId = text(rowIdRaw);
  if (!rowId) return false;
  const run = readWorkflowRun();
  if (!run) return false;
  const res = asRecord(run.res);
  if (!Object.keys(res).length) return false;
  const rows = collectTopRows(res, { allowBestFallback: true }).slice(0, 512);
  const row = rows.find((candidate) => topRowIdFor(candidate) === rowId) ?? null;
  if (!row) return false;
  const pinia = resolveRuntimePinia();
  if (pinia) {
    useWorkflowStore(pinia).selectResultsTab("drill");
  }
  await runVueDrillPipeline(row);
  return true;
}

function heatmapModeOf(store: AnalysisStoreState | null | undefined):
  HeatmapMode {
  return heatmapModeFromValue(store?.heatmapMode);
}

function readExactNeighborhoodRobustness(
  res: Record<string, unknown>,
): ExactNeighborhoodRobustnessSummary | null {
  const shortlistDiagnostics = asRecord(res.shortlist_slice_diagnostics);
  if (!Object.keys(shortlistDiagnostics).length) return null;
  const neighborhoodRobustness = asRecord(
    shortlistDiagnostics.neighborhood_robustness,
  );
  if (!Object.keys(neighborhoodRobustness).length) return null;
  const plateauScore = Number(neighborhoodRobustness.plateau_score);
  if (!Number.isFinite(plateauScore)) return null;
  return {
    plateauScore,
    plateauLabel: text(neighborhoodRobustness.plateau_label) || "Plateau",
    neighborCount: Math.max(
      0,
      Math.floor(num(neighborhoodRobustness.neighbor_count, 0)),
    ),
    radius: Math.max(0, Math.floor(num(neighborhoodRobustness.radius, 0))),
    step: Math.max(0, Math.floor(num(neighborhoodRobustness.step, 0))),
  };
}

function resolveHeatmapDisplay(
  mode: "objective" | "plateau",
  req: Record<string, unknown>,
  model: HeatmapModel,
): ReturnType<typeof resolveHeatmapDisplayModel> {
  return resolveHeatmapDisplayModel({
    mode,
    objectiveLabel: scoreLabelOf(req),
    model,
    higherIsBetter: modelHeatmapHigherIsBetter(objectiveOf(req), !!scoreModeOf(req)),
  });
}

function buildHeatmapMeta(
  mode: "objective" | "plateau",
  req: Record<string, unknown>,
  model: HeatmapModel,
  range: { min: number; max: number; label: string },
  multiSampleCellCount = 0,
): string {
  if (mode === "plateau") {
    const densityNote =
      multiSampleCellCount > 0
        ? ` • density-aware=${multiSampleCellCount} multi-sample cells`
        : "";
    return `Derived plateau heatmap • objective=${scoreLabelOf(req)} • bins=${model.binsFast}x${model.binsSlow} • fast=${model.fastMin}-${model.fastMax} • slow=${model.slowMin}-${model.slowMax} • scale=${range.label} • supportive=8-neighbor agreement${densityNote}`;
  }
  return `${scoreLabelOf(req)} heatmap • bins=${model.binsFast}x${model.binsSlow} • fast=${model.fastMin}-${model.fastMax} • slow=${model.slowMin}-${model.slowMax} • scale=${range.label}`;
}

function buildHeatmapStatusLine(
  mode: "objective" | "plateau",
  topK: number,
  exactRobustness: ExactNeighborhoodRobustnessSummary | null,
  multiSampleCellCount: number,
): string {
  const markerNote =
    topK === 0
      ? "Markers use the best candidate only."
      : "Markers reflect the visible Top rows.";
  if (mode === "plateau") {
    if (exactRobustness) {
      return `Heatmap ready. Plateau mode is UI-derived from the raw objective surface; exact winner robustness ${exactRobustness.plateauLabel} ${fmtDisplay(exactRobustness.plateauScore, 4)} over ${exactRobustness.neighborCount} neighbors (radius ${exactRobustness.radius}, step ${exactRobustness.step}). Click a populated cell to inspect a representative parameter pair for that heatmap region. ${markerNote}`;
    }
    if (multiSampleCellCount > 0) {
      return `Heatmap ready. Plateau mode uses supportive 8-neighbor agreement, roughness, curvature, and max-vs-mean bin agreement; hover for plateau, raw objective, and bin details. Click a populated cell to inspect a representative parameter pair for that heatmap region. ${markerNote}`;
    }
    return `Heatmap ready. Plateau mode uses supportive 8-neighbor agreement, roughness, and curvature over the raw objective surface; hover for plateau and objective values. Click a populated cell to inspect a representative parameter pair for that heatmap region. ${markerNote}`;
  }
  return topK === 0
    ? "Heatmap ready. This run disabled ranked Top rows (top_k=0), so markers use the best candidate only."
    : "Heatmap ready. Hover for values and click a populated cell to inspect a representative parameter pair for that heatmap region.";
}

function heatmapMarkerRows(
  store: AnalysisStoreState | null,
  req: Record<string, unknown>,
  res: Record<string, unknown>,
  model: HeatmapModel,
): Array<{ col: number; drawRow: number; label: string }> {
  const visibleRows = store
    ? buildTopRowModelsForCurrentFilters(store, req, res).slice(0, 8)
    : [];
  const rows =
    visibleRows.length > 0
      ? visibleRows
      : collectTopRows(res, { allowBestFallback: true }).slice(
          0,
          requestedTopK(req) === 0 ? 1 : 8,
        );
  const out: Array<{ col: number; drawRow: number; label: string }> = [];
  for (let i = 0; i < rows.length; i += 1) {
    const row = rows[i];
    if (!row) continue;
    const fast = Math.max(0, Math.floor(num(row.params.fast_len, 0)));
    const slow = Math.max(0, Math.floor(num(row.params.slow_len, 0)));
    if (fast <= 0 || slow <= 0) continue;
    const col = heatmapColForFast(model, fast);
    const logicalRow = heatmapLogicalRowForSlow(model, slow);
    out.push({
      col,
      drawRow: model.binsSlow - 1 - logicalRow,
      label: String(Math.max(1, Math.floor(num(row.rank, i + 1)))),
    });
  }
  return out;
}

function heatmapBaseSignature(
  run: WorkflowRunState | null,
  req: Record<string, unknown>,
  res: Record<string, unknown>,
  model: HeatmapModel | null,
  mode: "objective" | "plateau",
  store: AnalysisStoreState | null,
): string {
  return stableStringify({
    run: workflowRunIdentity(run),
    mode,
    objective: objectiveOf(req),
    score_mode: scoreModeOf(req),
    top_k: requestedTopK(req),
    heatmap_bins: requestedHeatmapBins(req),
    model,
    markers: model ? heatmapMarkerRows(store, req, res, model) : [],
  });
}

function buildHeatmapCanvasRequest(req: Record<string, unknown>): HeatmapCanvasRequestModel {
  return {
    fastMaType:
      text(
        Array.isArray(req.fast_ma_types)
          ? req.fast_ma_types[0]
          : req.fast_ma_type,
      ) || "?",
    slowMaType:
      text(
        Array.isArray(req.slow_ma_types)
          ? req.slow_ma_types[0]
          : req.slow_ma_type,
      ) || "?",
    fastMaParams: req.fast_ma_params ?? req.fastMaParams ?? null,
    slowMaParams: req.slow_ma_params ?? req.slowMaParams ?? null,
    scoreLabel: scoreLabelOf(req),
  };
}

function setHeatmapCanvasModel(
  store: AnalysisStoreState,
  req: Record<string, unknown>,
  res: Record<string, unknown>,
): boolean {
  const mode = heatmapModeOf(store);
  const model = readHeatmapModel(res);
  const exactRobustness = readExactNeighborhoodRobustness(res);

  const topK = requestedTopK(req);
  const heatmapBins = requestedHeatmapBins(req);
  if (!model) {
    heatmapRenderState = null;
    store.heatmapWrapVisible = false;
    store.canHeatmapExportPng = false;
    store.heatmapCanvasModel = null;
    store.heatmapStatusLine =
      heatmapBins <= 0
        ? "Heatmap disabled for this run. Increase Advanced > Heatmap bins above 0, then rerun optimization."
        : "No heatmap payload was returned for this run.";
    store.heatmapMeta =
      heatmapBins <= 0
        ? "Heatmap disabled for this run (heatmap_bins=0)."
        : "Heatmap data unavailable for this run.";
    return true;
  }

  const finite = finiteHeatmapValues(model);
  if (!finite.length) {
    heatmapRenderState = null;
    store.heatmapWrapVisible = false;
    store.canHeatmapExportPng = false;
    store.heatmapCanvasModel = null;
    store.heatmapMeta =
      "Heatmap payload was returned, but it contained no populated cells.";
    store.heatmapStatusLine = "Heatmap unavailable for this run.";
    return true;
  }

  const display = resolveHeatmapDisplay(mode, req, model);
  const displayFinite = finiteValues(display.values);
  if (!displayFinite.length) {
    heatmapRenderState = null;
    store.heatmapWrapVisible = false;
    store.canHeatmapExportPng = false;
    store.heatmapCanvasModel = null;
    store.heatmapMeta =
      mode === "plateau"
        ? "Derived plateau heatmap could not be built from this surface."
        : "Heatmap payload was returned, but it contained no populated cells.";
    store.heatmapStatusLine = "Heatmap unavailable for this run.";
    return true;
  }

  const meta = buildHeatmapMeta(
    mode,
    req,
    model,
    display.range,
    display.multiSampleCellCount,
  );
  const statusLine = buildHeatmapStatusLine(
    mode,
    topK,
    exactRobustness,
    display.multiSampleCellCount,
  );
  const state: HeatmapCanvasRenderModel = {
    signature: heatmapBaseSignature(readWorkflowRun(), req, res, model, mode, store),
    meta,
    statusLine,
    model,
    displayMode: mode,
    displayValues: display.values,
    displayLabel: display.label,
    displayLegendNote: display.legendNote,
    request: buildHeatmapCanvasRequest(req),
    markers: heatmapMarkerRows(store, req, res, model),
    range: display.range,
    palette: display.palette,
    plateauDiagnostics: display.plateauDiagnostics,
  };

  store.heatmapWrapVisible = true;
  store.canHeatmapExportPng = true;
  store.heatmapMeta = meta;
  store.heatmapStatusLine = statusLine;
  store.heatmapCanvasModel = markRaw(state);
  heatmapRenderState = state;
  return true;
}

export function refreshHeatmapFromWorkflowRun(): boolean {
  const store = readAnalysisStore();
  const run = readWorkflowRun();
  if (!store || !run) return false;
  const req = asRecord(run.req);
  const res = asRecord(run.res);
  if (!Object.keys(res).length) return false;
  const mode = heatmapModeOf(store);

  store.heatmapModeOptions = [
    { value: "objective", label: "Objective score", disabled: false },
    { value: "plateau", label: "Derived plateau", disabled: false },
  ];
  const modeOptions = Array.isArray(store.heatmapModeOptions)
    ? (store.heatmapModeOptions as Array<{ value?: unknown }>)
    : [];
  if (!modeOptions.some((option) => text(option.value) === mode)) {
    store.heatmapMode = "objective";
  }
  const colorOptions =
    mode === "plateau"
      ? [{ value: "derived_plateau", label: "Plateau sequential", disabled: false }]
      : [
          {
            value: "diverging_zero",
            label: "Diverging zero-centered",
            disabled: false,
          },
        ];
  const rangeOptions =
    mode === "plateau"
      ? [{ value: "plateau_score", label: "Plateau score (0-1)", disabled: false }]
      : [{ value: "zero_centered", label: "Zero-centered", disabled: false }];
  const showTopOptions = [
    { value: "true", label: "Show markers", disabled: false },
  ];
  const showCrosshairOptions = [
    { value: "true", label: "Show crosshair", disabled: false },
  ];
  const coerceSelectValue = (
    currentValue: unknown,
    options: Array<{ value: string }>,
    fallbackValue: string,
  ): string =>
    options.some((option) => text(option.value) === text(currentValue))
      ? text(currentValue)
      : fallbackValue;
  store.heatmapColorOptions = colorOptions;
  store.heatmapRangeOptions = rangeOptions;
  store.heatmapShowTopOptions = showTopOptions;
  store.heatmapShowCrosshairOptions = showCrosshairOptions;
  store.heatmapColor = coerceSelectValue(
    store.heatmapColor,
    colorOptions,
    colorOptions[0]?.value ?? "",
  );
  store.heatmapRange = coerceSelectValue(
    store.heatmapRange,
    rangeOptions,
    rangeOptions[0]?.value ?? "",
  );
  store.heatmapShowTop = coerceSelectValue(
    store.heatmapShowTop,
    showTopOptions,
    showTopOptions[0]?.value ?? "true",
  );
  store.heatmapShowCrosshair = coerceSelectValue(
    store.heatmapShowCrosshair,
    showCrosshairOptions,
    showCrosshairOptions[0]?.value ?? "true",
  );

  const requestedBins = requestedHeatmapBins(req);
  const model = readHeatmapModel(res);
  const exactRobustness = readExactNeighborhoodRobustness(res);
  if (requestedBins <= 0) {
    store.heatmapWrapVisible = false;
    store.canHeatmapExportPng = false;
    store.heatmapMeta =
      mode === "plateau"
        ? "Derived plateau heatmap disabled for this run (heatmap_bins=0)."
        : "Heatmap disabled for this run (heatmap_bins=0).";
    store.heatmapStatusLine =
      "Increase Advanced > Heatmap bins above 0, then rerun optimization to generate a heatmap.";
  } else if (model) {
    const display = resolveHeatmapDisplay(mode, req, model);
    store.heatmapWrapVisible = true;
    store.canHeatmapExportPng = true;
    store.heatmapMeta = buildHeatmapMeta(
      mode,
      req,
      model,
      display.range,
      display.multiSampleCellCount,
    );
    store.heatmapStatusLine = buildHeatmapStatusLine(
      mode,
      requestedTopK(req),
      exactRobustness,
      display.multiSampleCellCount,
    );
  } else {
    store.heatmapWrapVisible = false;
    store.canHeatmapExportPng = false;
    store.heatmapMeta =
      mode === "plateau"
        ? "Derived plateau heatmap unavailable because no objective heatmap payload was returned."
        : "Heatmap data unavailable for this run.";
    store.heatmapStatusLine = "No heatmap payload was returned for this run.";
  }

  const ok = setHeatmapCanvasModel(store, req, res);
  markAnalysisVue(store);
  return ok;
}

export function restoreHeatmapVisible(): boolean {
  const store = readAnalysisStore();
  const run = readWorkflowRun();
  if (!store || !run || !heatmapRenderState) return false;
  const req = asRecord(run.req);
  const res = asRecord(run.res);
  if (!Object.keys(res).length) return false;
  const model = readHeatmapModel(res);
  const nextSignature = heatmapBaseSignature(
    run,
    req,
    res,
    model,
    heatmapModeOf(store),
    store,
  );
  if (nextSignature !== heatmapRenderState.signature) return false;

  store.heatmapWrapVisible = true;
  store.canHeatmapExportPng = true;
  store.heatmapCanvasModel = markRaw(heatmapRenderState);
  store.heatmapMeta = heatmapRenderState.meta;
  store.heatmapStatusLine = heatmapRenderState.statusLine;
  markAnalysisVue(store);
  return true;
}

function stableStringify(value: unknown): string {
  if (value == null) return "null";
  if (typeof value !== "object") {
    try {
      return JSON.stringify(value);
    } catch {
      return String(value);
    }
  }
  if (Array.isArray(value)) {
    return `[${value.map((entry) => stableStringify(entry)).join(",")}]`;
  }
  const obj = value as Record<string, unknown>;
  const keys = Object.keys(obj).sort();
  return `{${keys.map((key) => `${JSON.stringify(key)}:${stableStringify(obj[key])}`).join(",")}}`;
}

function workflowRunIdentity(run: WorkflowRunState | null | undefined): string {
  if (!run) return "null";
  return stableStringify({
    ts: text(run.ts),
    artifactPath: text(run.artifactPath ?? run.artifact_path),
    req: run.req ?? null,
    res: run.res ?? null,
  });
}

export function invalidateAnalysisExecutionCaches(): void {
  baselineArtifactCache.clear();
  drillCurvesCache.clear();
  stabilityCache.clear();
  drillPriceBarsCache.clear();
  baselineArtifactPending.clear();
  drillCurvesPending.clear();
  stabilityPending.clear();
  drillPriceBarsPending.clear();
  drillRenderState = null;
  heatmapRenderState = null;
  topRowsModelCache = [];
  topRowsAllModelCache = [];
  drillSeq = (drillSeq + 1) >>> 0;
  const store = readAnalysisStore();
  if (store) {
    store.drillChartModel = null;
    store.drillTradeTableModel = null;
  }
  registeredDrillChartController?.clearTradeSelection();
}

export function registerDrillChartController(controller: DrillChartController | null): void {
  registeredDrillChartController = controller;
}

export function registerHeatmapCanvasController(controller: HeatmapCanvasController | null): void {
  registeredHeatmapCanvasController = controller;
}

export function runHeatmapRepresentativeCandidateDetail(params: HeatmapCellParams): void {
  const pinia = resolveRuntimePinia();
  if (pinia) {
    useWorkflowStore(pinia).selectResultsTab("drill");
  }
  void runVueDrillPipeline({ rank: 1, params, metrics: {} });
}

async function loadBaselineFromDisk(path: string): Promise<BaselineRun | null> {
  const normalizedPath = text(path);
  if (!normalizedPath) return null;
  const loaded = await cacheGetOrLoad(
    baselineArtifactCache,
    baselineArtifactPending,
    normalizedPath,
    async () => {
      const raw = (await invokeCommand("load_run_artifact", {
        path: normalizedPath,
      })) as RunArtifactPayload;
      const row = asRecord(raw);
      const run = asRecord(row.run);
      const req = asRecord(run.req);
      const res = asRecord(run.res);
      if (!Object.keys(req).length || !Object.keys(res).length) {
        return null;
      }
      return {
        source: "disk" as const,
        ts: text(row.ts) || text(run.ts),
        csvPath: text(row.csv_path ?? row.csvPath),
        reportPath: text(row.report_xlsx_path ?? row.reportXlsxPath),
        hasValidation: !!asRecord(row.validation).res,
        configFingerprint: text(
          row.config_fingerprint ?? row.configFingerprint,
        ),
        datasetMeta: asRecord(row.dataset_meta ?? row.datasetMeta),
        routeProvenance: parseRouteProvenance(
          row.route_provenance ?? row.routeProvenance,
        ),
        lineage: parseArtifactLineage(row.lineage),
        optimizationMetricSurface:
          asRecord(
            row.optimization_metric_surface ?? row.optimizationMetricSurface,
          ) || null,
        req,
        res,
      };
    },
    BASELINE_ARTIFACT_CACHE_MAX,
  );
  return loaded.value;
}

async function resolveBaselineSelection(
  selectedValue: string,
  current: WorkflowRunState,
  history: HistoryStoreState | null,
): Promise<BaselineRun | null> {
  const selected = text(selectedValue);
  if (!selected) return null;
  if (selected.startsWith("disk:")) {
    return await loadBaselineFromDisk(selected.slice(5));
  }

  const baselineTs = selected.startsWith("hist:")
    ? text(selected.slice(5))
    : selected;
  const currentTs = text(current.ts);
  const currentReq = asRecord(current.req);
  const currentRes = asRecord(current.res);
  if (
    baselineTs &&
    currentTs === baselineTs &&
    Object.keys(currentReq).length &&
    Object.keys(currentRes).length
  ) {
    return {
      source: "hist",
      ts: baselineTs,
      csvPath: text(current.csv_path ?? current.csvPath),
      reportPath: text(current.report_xlsx_path ?? current.reportXlsxPath),
      hasValidation: hasValidationResult(),
      configFingerprint: text(
        current.config_fingerprint ?? current.configFingerprint,
      ),
      datasetMeta: asRecord(current.dataset_meta ?? current.datasetMeta),
      routeProvenance: parseRouteProvenance(
        current.route_provenance ?? current.routeProvenance,
      ),
      lineage: parseArtifactLineage(current.lineage),
      optimizationMetricSurface: asRecord(
        current.optimization_metric_surface ?? current.optimizationMetricSurface,
      ) || null,
      req: currentReq,
      res: currentRes,
    };
  }

  const savedRuns = Array.isArray(history?.savedRunsRaw)
    ? (history?.savedRunsRaw as Array<Record<string, unknown>>)
    : [];
  const hit = savedRuns.find((entry) => text(entry.ts) === baselineTs);
  const path = text(hit?.path);
  if (!path) return null;
  const loaded = await loadBaselineFromDisk(path);
  if (!loaded) return null;
  return {
    source: "hist",
    ts: baselineTs || loaded.ts,
    csvPath: loaded.csvPath,
    reportPath: loaded.reportPath,
    hasValidation: loaded.hasValidation,
    configFingerprint: loaded.configFingerprint,
    datasetMeta: loaded.datasetMeta,
    routeProvenance: loaded.routeProvenance,
    lineage: loaded.lineage,
    optimizationMetricSurface: loaded.optimizationMetricSurface ?? null,
    req: loaded.req,
    res: loaded.res,
  };
}

function workflowRunToComparisonTarget(
  source: string,
  run: WorkflowRunState,
): RunComparisonTarget | null {
  const req = asRecord(run.req);
  const res = asRecord(run.res);
  if (!Object.keys(req).length || !Object.keys(res).length) return null;
  return {
    source,
    ts: text(run.ts),
    strategyKind: kindFromReq(req),
    csvPath: text(run.csv_path ?? run.csvPath),
    timeframe: text(
      asRecord(run.dataset_meta ?? run.datasetMeta).timeframe ?? req.timeframe,
    ),
    reportPath: text(run.report_xlsx_path ?? run.reportXlsxPath),
    hasValidation: hasValidationResult(),
    configFingerprint: text(run.config_fingerprint ?? run.configFingerprint),
    datasetMeta: asRecord(run.dataset_meta ?? run.datasetMeta),
    optimizationMetricSurface: asRecord(
      run.optimization_metric_surface ?? run.optimizationMetricSurface,
    ) || null,
    routeProvenance: parseRouteProvenance(
      run.route_provenance ?? run.routeProvenance,
    ),
    lineage: parseArtifactLineage(run.lineage),
    req,
    res,
  };
}

function baselineRunToComparisonTarget(
  baseline: BaselineRun,
): RunComparisonTarget | null {
  if (!Object.keys(baseline.req).length || !Object.keys(baseline.res).length) {
    return null;
  }
  return {
    source: baseline.source,
    ts: baseline.ts,
    strategyKind: kindFromReq(baseline.req),
    csvPath: baseline.csvPath,
    timeframe: text(
      asRecord(baseline.datasetMeta).timeframe ?? baseline.req.timeframe,
    ),
    reportPath: baseline.reportPath,
    hasValidation: baseline.hasValidation,
    configFingerprint: baseline.configFingerprint,
    datasetMeta: baseline.datasetMeta,
    optimizationMetricSurface: baseline.optimizationMetricSurface ?? null,
    routeProvenance: baseline.routeProvenance,
    lineage: baseline.lineage,
    req: baseline.req,
    res: baseline.res,
  };
}

function csvEscape(v: unknown): string {
  const s = String(v ?? "");
  if (/[",\n\r]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
  return s;
}

function startEq(req: Record<string, unknown>): number {
  const direct = Number(req.starting_equity ?? req.startingEquity);
  if (Number.isFinite(direct) && direct > 0) return direct;
  const strategy = asRecord(req.strategy);
  const base = asRecord(req.base_strategy);
  const nested = Number(
    strategy.starting_equity ??
      strategy.startingEquity ??
      base.starting_equity ??
      base.startingEquity,
  );
  if (Number.isFinite(nested) && nested > 0) return nested;
  return 10_000;
}

function finalEq(pnl: unknown, equity: unknown): number {
  const p = Number(pnl);
  const e = Number(equity);
  if (!Number.isFinite(p)) return NaN;
  const seed = Number.isFinite(e) && e > 0 ? e : 10_000;
  return seed * (1 + p);
}

function sanitizeTs(v: unknown): string {
  const raw = text(v);
  return (raw || new Date().toISOString()).replace(/[^\w.-]+/g, "-");
}

function downloadText(filename: string, body: string, mime: string): boolean {
  if (typeof document === "undefined") return false;
  if (typeof URL === "undefined" || typeof URL.createObjectURL !== "function")
    return false;
  const blob = new Blob([body], { type: mime });
  const href = URL.createObjectURL(blob);
  try {
    const a = document.createElement("a");
    a.href = href;
    a.download = filename;
    a.style.display = "none";
    document.body?.appendChild(a);
    a.click();
    a.remove();
    return true;
  } finally {
    URL.revokeObjectURL(href);
  }
}

async function copyText(body: string): Promise<boolean> {
  const src = text(body);
  if (!src) return false;
  try {
    const clip = (
      globalThis as {
        navigator?: {
          clipboard?: { writeText?: (value: string) => Promise<void> };
        };
      }
    ).navigator?.clipboard;
    if (clip && typeof clip.writeText === "function") {
      await clip.writeText(src);
      return true;
    }
  } catch {}
  return false;
}

function byIds<T extends HTMLElement>(ids: readonly string[]): T | null {
  if (typeof document === "undefined") return null;
  for (const id of ids) {
    const node =
      typeof document.querySelector === "function"
        ? document.querySelector(`[id="${id}"]`)
        : null;
    if (node) return node as T;
    const getById = (
      document as unknown as {
        getElementById?: (nextId: string) => Element | null;
      }
    ).getElementById;
    if (typeof getById === "function") {
      const fallback = getById.call(document, id);
      if (fallback) return fallback as T;
    }
  }
  return null;
}

function drillWrap(): HTMLElement | null {
  return byIds<HTMLElement>(["drillWrapVue", "drillWrap"]);
}

function selectDrillTradeByIndex(tradeIdxRaw: unknown): boolean {
  return registeredDrillChartController?.selectTradeByIndex(tradeIdxRaw) ?? false;
}

function renderStabilityFilterChecklist(
  metricsRaw: Record<string, unknown>,
  filters: TopFilterValues,
): string {
  const metrics = asRecord(metricsRaw);
  const checks: string[] = [];
  let allOk = true;
  let any = false;
  const mark = (ok: boolean, label: string): void => {
    any = true;
    if (!ok) allOk = false;
    checks.push(`${ok ? "PASS" : "FAIL"} ${label}`);
  };
  const pnl = num(metrics.pnl, NaN);
  const sharpe = num(metrics.sharpe, NaN);
  const dd = num(metrics.max_dd, NaN);
  const trades = num(metrics.trades, NaN);
  const expo = num(metrics.exposure, NaN);
  if (filters.minPnl != null)
    mark(
      Number.isFinite(pnl) && pnl >= filters.minPnl,
      `PnL>=${fmt(filters.minPnl, 6)} (${fmt(pnl, 6)})`,
    );
  if (filters.minSharpe != null)
    mark(
      Number.isFinite(sharpe) && sharpe >= filters.minSharpe,
      `Sharpe>=${fmt(filters.minSharpe, 6)} (${fmt(sharpe, 6)})`,
    );
  if (filters.maxDd != null)
    mark(
      Number.isFinite(dd) && dd <= filters.maxDd,
      `MaxDD<=${fmt(filters.maxDd, 6)} (${fmt(dd, 6)})`,
    );
  if (filters.minTrades != null)
    mark(
      Number.isFinite(trades) && trades >= filters.minTrades,
      `Trades>=${String(filters.minTrades)} (${Number.isFinite(trades) ? String(Math.floor(trades)) : "-"})`,
    );
  if (filters.minExposure != null)
    mark(
      Number.isFinite(expo) && expo >= filters.minExposure,
      `Expo>=${fmt(filters.minExposure, 6)} (${fmt(expo, 6)})`,
    );
  if (!any) return "No Top filters set (nothing to validate).";
  return `${allOk ? "PASS" : "FAIL"} ${checks.join(" | ")}`;
}

function renderStabilityDom(
  resultRaw: unknown,
  store: AnalysisStoreState | null,
): boolean {
  const result = asRecord(resultRaw);
  const wrap = byIds<HTMLElement>(["stabilityWrapVue", "stabilityWrap"]);
  const statusEl = byIds<HTMLElement>([
    "stabilityStatusVue",
    "stabilityStatus",
  ]);
  const metaEl = byIds<HTMLElement>(["stabilityMetaVue", "stabilityMeta"]);
  const rejectEl = byIds<HTMLElement>(["rejectMetaVue", "rejectMeta"]);
  const tableWrap = byIds<HTMLElement>([
    "stabilityTableWrapVue",
    "stabilityTableWrap",
  ]);
  const tbody = byIds<HTMLElement>([
    "stabilityTableBodyVue",
    "stabilityTableBody",
  ]);
  if (!wrap || !statusEl || !metaEl || !rejectEl || !tableWrap || !tbody)
    return false;

  wrap.style.display = "block";
  if (store) store.stabilityWrapVisible = true;
  const points = Array.isArray(result.points)
    ? result.points.map((p) => asRecord(p))
    : [];
  if (!points.length) {
    statusEl.textContent = "No stability data.";
    statusEl.className = "muted";
    metaEl.textContent = "-";
    rejectEl.textContent = "-";
    tableWrap.style.display = "none";
    tbody.innerHTML = "";
    return true;
  }

  const summary = asRecord(result.summary);
  const base = asRecord(result.base);
  const objective = text(result.objective) || "?";
  const radius = num(result.radius, 0);
  const step = num(result.step, 0);
  const startT = num(result.start_t, 0);
  const baseScore = num(summary.base, 0);
  const minScore = num(summary.min, 0);
  const medScore = num(summary.median, 0);
  const maxScore = num(summary.max, 0);
  const peakDelta = baseScore - medScore;

  statusEl.textContent = "Done.";
  statusEl.className = "muted ok";
  metaEl.textContent = `objective=${objective} radius=${radius} step=${step} pts=${points.length} start_t=${startT} | score: base=${fmt(baseScore, 6)} median=${fmt(medScore, 6)} min=${fmt(minScore, 6)} max=${fmt(maxScore, 6)} peakΔ=${fmt(peakDelta, 6)}`;

  const bf = Math.floor(num(base.fast_len, NaN));
  const bs = Math.floor(num(base.slow_len, NaN));
  const basePoint =
    points.find(
      (p) =>
        Math.floor(num(p.fast_len, NaN)) === bf &&
        Math.floor(num(p.slow_len, NaN)) === bs,
    ) ?? null;
  const filters = store ? readFilters(store) : {};
  rejectEl.textContent = renderStabilityFilterChecklist(
    asRecord(basePoint?.metrics),
    filters,
  );

  const sorted = [...points].sort((a, b) => num(b.score, 0) - num(a.score, 0));
  tbody.innerHTML = "";
  for (const point of sorted) {
    const metrics = asRecord(point.metrics);
    const tr = document.createElement("tr");
    const cells: Array<{ text: string; cls: string }> = [
      { text: String(Math.floor(num(point.fast_len, 0))), cls: "left mono" },
      { text: String(Math.floor(num(point.slow_len, 0))), cls: "left mono" },
      { text: fmt(point.score, 6), cls: "mono" },
      { text: fmt(metrics.pnl, 6), cls: "mono" },
      { text: fmt(metrics.sharpe, 6), cls: "mono" },
      { text: fmt(metrics.max_dd, 6), cls: "mono" },
      { text: String(Math.floor(num(metrics.trades, 0))), cls: "mono" },
    ];
    for (const cell of cells) {
      const td = document.createElement("td");
      td.className = cell.cls;
      td.textContent = cell.text;
      tr.appendChild(td);
    }
    tbody.appendChild(tr);
  }
  tableWrap.style.display = "block";
  return true;
}

function drawMinMaxChartDom(
  canvas: HTMLCanvasElement | null,
  minmaxRaw: unknown,
  opts?: { stroke?: string; baseline?: number; height?: number },
): void {
  const mm = Array.isArray(minmaxRaw) ? minmaxRaw : [];
  const bins = Math.floor(mm.length / 2);
  if (!canvas || bins <= 0) return;
  const height = Math.max(60, Math.floor(num(opts?.height, 160)));
  canvas.width = bins;
  canvas.height = height;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.clearRect(0, 0, bins, height);

  let minV = Number.POSITIVE_INFINITY;
  let maxV = Number.NEGATIVE_INFINITY;
  for (let i = 0; i < mm.length; i += 1) {
    const v = Number(mm[i]);
    if (!Number.isFinite(v)) continue;
    if (v < minV) minV = v;
    if (v > maxV) maxV = v;
  }
  if (!Number.isFinite(minV) || !Number.isFinite(maxV)) return;
  if (minV === maxV) {
    minV -= 1;
    maxV += 1;
  }

  const pad = 6;
  const scale = (height - 1 - 2 * pad) / (maxV - minV);
  const yOf = (value: number): number => pad + (maxV - value) * scale;

  if (Number.isFinite(opts?.baseline)) {
    const y = yOf(num(opts?.baseline, 0));
    ctx.strokeStyle = "rgba(148, 163, 184, 0.45)";
    ctx.beginPath();
    ctx.moveTo(0, y + 0.5);
    ctx.lineTo(bins, y + 0.5);
    ctx.stroke();
  }

  ctx.strokeStyle = String(opts?.stroke || "rgba(52, 211, 153, 0.95)");
  ctx.lineWidth = 1;
  for (let x = 0; x < bins; x += 1) {
    const a = Number(mm[2 * x]);
    const b = Number(mm[2 * x + 1]);
    if (!Number.isFinite(a) || !Number.isFinite(b)) continue;
    ctx.beginPath();
    ctx.moveTo(x + 0.5, yOf(a));
    ctx.lineTo(x + 0.5, yOf(b));
    ctx.stroke();
  }
}

async function fetchDrillPriceBars(dataId: string): Promise<DrillPriceBar[]> {
  const id = text(dataId);
  if (!id) return [];
  const loaded = await cacheGetOrLoad(
    drillPriceBarsCache,
    drillPriceBarsPending,
    id,
    async () => {
      let raw: unknown;
      try {
        raw = await invokeCommand("get_price_bars", {
          dataId: id,
          maxPoints: 8000,
        });
      } catch (firstError) {
        try {
          raw = await invokeCommand("get_price_bars", {
            data_id: id,
            max_points: 8000,
          });
        } catch {
          throw firstError;
        }
      }
      return normalizeIndexedPriceBars(raw);
    },
    DRILL_PRICE_BARS_CACHE_MAX,
  );
  return loaded.value;
}

function mapTradeIndexToChartTime(
  indexRaw: unknown,
  barsRaw: DrillPriceBar[] | null | undefined,
): number | null {
  return modelMapTradeIndexToChartTime(indexRaw, barsRaw);
}

function buildDrillTradeMarkers(
  curvesRaw: unknown,
  barsRaw: DrillPriceBar[] | null | undefined,
  selectedTradeIdxRaw: unknown = -1,
): Array<Record<string, unknown>> {
  const selectedTradeIdx = Math.floor(num(selectedTradeIdxRaw, -1));
  return buildDrillTradeMarkersForModel(
    buildDrillChartModel(curvesRaw, barsRaw),
    Number.isFinite(selectedTradeIdx) ? selectedTradeIdx : -1,
  ).map((marker) => ({ ...marker }));
}

function renderDrillDomFallback(
  curvesRaw: unknown,
  paramsRaw: Record<string, unknown>,
  metricsRaw: Record<string, unknown>,
  row: TopRowModel,
): boolean {
  const wrap = drillWrap();
  const hint = byIds<HTMLElement>(["drillEmptyHintVue", "drillEmptyHint"]);
  const meta = byIds<HTMLElement>(["drillMetaVue", "drillMeta"]);
  const eqCanvas = byIds<HTMLCanvasElement>([
    "equityCanvasVue",
    "equityCanvas",
  ]);
  const ddCanvas = byIds<HTMLCanvasElement>([
    "drawdownCanvasVue",
    "drawdownCanvas",
  ]);
  if (!wrap || !eqCanvas || !ddCanvas) return false;

  const curves = asRecord(curvesRaw);
  const eq = asRecord(curves.equity);
  const dd = asRecord(curves.drawdown);
  const bins = Math.floor(num(eq.bins, 0));
  if (bins <= 0) {
    wrap.style.display = "none";
    const store = readAnalysisStore();
    if (store) {
      store.drillWrapVisible = false;
      store.tradeStatsWrapVisible = false;
      store.drillChartModel = null;
      store.drillTradeTableModel = null;
    }
    if (hint) hint.style.display = "block";
    return true;
  }

  wrap.style.display = "block";
  const store = readAnalysisStore();
  if (store) {
    store.drillWrapVisible = true;
    store.tradeStatsWrapVisible = Boolean(
      asRecord(store.drillTradeTableModel).visible,
    );
  }
  if (hint) hint.style.display = "none";
  if (meta) {
    const metricPart = formatDrillMetricsSummary(metricsRaw);
    const moPart = formatDrillMoSummary(row.mo);
    const parts = [
      `${text(paramsRaw.fast_ma_type) || "?"}(${num(paramsRaw.fast_len, 0) || "?"}) vs ${text(paramsRaw.slow_ma_type) || "?"}(${num(paramsRaw.slow_len, 0) || "?"})`,
    ];
    if (metricPart) parts.push(metricPart);
    if (moPart) parts.push(moPart);
    parts.push(
      `bins=${bins} start_t=${num(eq.start_t, 0)} end_t=${num(eq.end_t, 0)} (min/max per bin)`,
    );
    meta.textContent = parts.join(" | ");
  }

  drawMinMaxChartDom(eqCanvas, eq.minmax, {
    stroke: "rgba(52, 211, 153, 0.95)",
    baseline: 1.0,
    height: 160,
  });
  drawMinMaxChartDom(ddCanvas, dd.minmax, {
    stroke: "rgba(251, 113, 133, 0.95)",
    baseline: 0.0,
    height: 160,
  });
  return true;
}

function restoreDrillViewFromCache(
  store: AnalysisStoreState,
  row: TopRowModel,
): boolean {
  if (!drillRenderState) return false;
  const run = readWorkflowRun();
  const runIdentity = workflowRunIdentity(run);
  const rowId = topRowIdFor(row);
  if (
    drillRenderState.runIdentity !== runIdentity ||
    drillRenderState.rowId !== rowId
  ) {
    return false;
  }

  drillSeq = (drillSeq + 1) >>> 0;
  publishDrillChartModels(store, drillRenderState.curves, drillRenderState.bars);
  const rendered = renderDrillDomFallback(
    drillRenderState.curves,
    drillRenderState.params,
    drillRenderState.metrics,
    drillRenderState.row,
  );
  if (!rendered) return false;
  applyDrillCompletionState(
    store,
    drillRenderState.drillMeta,
    drillRenderState.drillStatus,
  );
  applyStabilityCompletionState(
    store,
    drillRenderState.stabilityResult,
    drillRenderState.stabilityMeta,
    drillRenderState.stabilityStatus,
  );
  setStatus(
    store,
    drillRenderState.drillStatus === "Done (cached)."
      ? "Candidate detail restored (cached)."
      : "Candidate detail restored.",
  );
  return true;
}

export async function clearTopFilters(): Promise<void> {
  const store = readAnalysisStore();
  if (!store) return;
  store.filterMinPnl = "";
  store.filterMinSharpe = "";
  store.filterMaxDd = "";
  store.filterMinTrades = "";
  store.filterMinExposure = "";
  store.canClearTopFilters = false;
  const topRefreshed = refreshTopTableFromWorkflowRun();
  const heatmapRefreshed = refreshHeatmapFromWorkflowRun();
  if (!topRefreshed && !heatmapRefreshed) markAnalysisVue(store);
  setStatus(store, "Top filters cleared.");
}

export async function sortTopTable(key: string): Promise<void> {
  const sortKey = text(key);
  const store = readAnalysisStore();
  if (!store || !sortKey) return;
  const current = readSortState(store);
  const nextDir: SortDir =
    current.key === sortKey
      ? current.dir === "asc"
        ? "desc"
        : "asc"
      : defaultSortDir(sortKey);
  const columns = readTopColumns(store);
  if (columns.length > 0) {
    for (const col of columns)
      col.sort = text(col?.key) === sortKey ? nextDir : "";
    store.topColumns = columns;
  } else {
    store.topColumns = [
      { key: sortKey, label: sortKey, sort: nextDir, alignLeft: false },
    ];
  }
  const topRefreshed = refreshTopTableFromWorkflowRun();
  const heatmapRefreshed = refreshHeatmapFromWorkflowRun();
  if (topRefreshed || heatmapRefreshed) {
    setStatus(store, `Sorted top rows by ${sortKey} (${nextDir}).`);
  }
}

export async function selectTopRow(index: number): Promise<void> {
  const idx = parseIndex(index);
  const store = readAnalysisStore();
  if (!store) return;
  const rows = readTopRows(store);
  if (!rows.length) return;
  const bounded = Math.max(0, Math.min(idx, rows.length - 1));
  for (let i = 0; i < rows.length; i += 1) {
    const row = rows[i];
    if (!row) continue;
    row.selected = i === bounded;
  }
  store.topTableRows = rows;
  const run = readWorkflowRun();
  if (run) {
    syncTopSelectionState(store, topRowsModelCache, asRecord(run.req), asRecord(run.res));
  }
  const model = resolveTopRowModelAtIndex(store, bounded);
  store.topSelectionGuardrail = "—";
  setStatus(store, `Selected top row ${bounded + 1}.`);
  if (model) await runVueDrillPipeline(model);
}

export async function pinTopRow(index: number): Promise<void> {
  const store = readAnalysisStore();
  if (!store) return;
  const idx = parseIndex(index);
  const run = readWorkflowRun();
  const req = asRecord(run?.req);
  const rows = topRowsModelCache.length
    ? topRowsModelCache
    : buildTopRowModelsForCurrentFilters(store, req, asRecord(run?.res));
  const model = rows[idx] ?? null;
  if (!model) {
    setStatus(store, "No ranked candidate available to pin.");
    return;
  }
  const filters = readFilters(store);
  const baselineSelected = text(readHistoryStore()?.compareSelectedValue);
  const candidate = buildTopCandidateSnapshot(
    model,
    req,
    filters,
    baselineSelected,
  );
  const current = Array.isArray(store.pinnedTopCandidates)
    ? (store.pinnedTopCandidates as TopPinnedCandidate[])
    : [];
  if (current.some((entry) => text(entry.id) === candidate.id)) {
    setStatus(store, `Candidate #${candidate.rank} is already pinned.`);
    return;
  }
  const next = [...current, markRawEntity(candidate)].slice(-4);
  store.pinnedTopCandidates = markRawEntity(next);
  syncPinnedTopCandidates(store, rows, rows, req);
  setStatus(
    store,
    current.length >= 4
      ? `Pinned candidate #${candidate.rank}. Oldest compare slot was replaced.`
      : `Pinned candidate #${candidate.rank} for compare.`,
  );
}

export async function pinSelectedTopRow(): Promise<void> {
  const store = readAnalysisStore();
  if (!store) return;
  const selectedIndex = readTopRows(store).findIndex((row) => !!row?.selected);
  if (selectedIndex < 0) {
    setStatus(store, "Select a ranked candidate before pinning it.");
    return;
  }
  await pinTopRow(selectedIndex);
}

export async function replacePinnedTopCandidate(idRaw: unknown): Promise<void> {
  const store = readAnalysisStore();
  if (!store) return;
  const targetId = text(idRaw);
  if (!targetId) return;
  const selectedIndex = readTopRows(store).findIndex((row) => !!row?.selected);
  if (selectedIndex < 0) {
    setStatus(store, "Select a ranked candidate before replacing a compare slot.");
    return;
  }
  const run = readWorkflowRun();
  const req = asRecord(run?.req);
  const rows = topRowsModelCache.length
    ? topRowsModelCache
    : buildTopRowModelsForCurrentFilters(store, req, asRecord(run?.res));
  const model = rows[selectedIndex] ?? null;
  if (!model) return;
  const filters = readFilters(store);
  const baselineSelected = text(readHistoryStore()?.compareSelectedValue);
  const candidate = buildTopCandidateSnapshot(
    model,
    req,
    filters,
    baselineSelected,
  );
  store.pinnedTopCandidates = markRawEntity(
    (Array.isArray(store.pinnedTopCandidates)
      ? (store.pinnedTopCandidates as TopPinnedCandidate[])
      : []
    ).map((entry) => (text(entry.id) === targetId ? markRawEntity(candidate) : entry)),
  );
  syncPinnedTopCandidates(store, rows, rows, req);
  setStatus(store, `Replaced compare slot with candidate #${candidate.rank}.`);
}

export async function unpinTopCandidate(idRaw: unknown): Promise<void> {
  const store = readAnalysisStore();
  if (!store) return;
  const targetId = text(idRaw);
  if (!targetId) return;
  const current = Array.isArray(store.pinnedTopCandidates)
    ? (store.pinnedTopCandidates as TopPinnedCandidate[])
    : [];
  const next = current.filter((entry) => text(entry.id) !== targetId);
  if (next.length === current.length) return;
  store.pinnedTopCandidates = markRawEntity(next);
  const run = readWorkflowRun();
  const req = asRecord(run?.req);
  syncPinnedTopCandidates(
    store,
    topRowsModelCache,
    topRowsAllModelCache.length ? topRowsAllModelCache : topRowsModelCache,
    req,
  );
  setStatus(store, "Removed pinned candidate.");
}

export async function clearPinnedTopCandidates(): Promise<void> {
  const store = readAnalysisStore();
  if (!store) return;
  if (!Array.isArray(store.pinnedTopCandidates) || store.pinnedTopCandidates.length === 0) return;
  store.pinnedTopCandidates = markRawEntity([]);
  store.topStaleCues = markRawEntity([]);
  setStatus(store, "Cleared pinned candidates.");
}

export async function copyBest(): Promise<void> {
  const store = readAnalysisStore();
  const summary = text(store?.bestSummary);
  if (summary && summary !== "-" && summary !== "—") {
    const ok = await copyText(summary);
    setStatus(store, ok ? "Copied best summary." : "Copy failed.");
    return;
  }
  setStatus(store, "Nothing to copy yet.");
}

export async function exportTopCsv(): Promise<void> {
  const store = readAnalysisStore();
  const run = readWorkflowRun();
  const req = asRecord(run?.req);
  const res = asRecord(run?.res);
  if (!Object.keys(res).length) {
    setStatus(store, "No result to export yet.");
    return;
  }
  if (requestedTopK(req) === 0 && !Array.isArray(res.pareto)) {
    setStatus(
      store,
      "Top CSV unavailable for this run because Top K was set to 0.",
    );
    return;
  }
  const objective = objectiveOf(req);
  const scoreMode = scoreModeOf(req);
  const sort = store
    ? readSortState(store)
    : { key: "rank", dir: "asc" as const };
  const rows = applyTopFilters(
    sortRows(
      collectTopRows(res, { allowBestFallback: false }),
      sort.key,
      sort.dir,
      objective,
      scoreMode,
      scoreObservationContextOf(res),
    ),
    store ? readFilters(store) : {},
  );
  const seed = startEq(req);
  const lines = [
    "rank,fast_ma,fast_len,slow_ma,slow_len,pnl,pnl_pct,final_equity,sharpe,max_dd,trades,exposure,net_exposure,active_mean,active_downside_deviation,active_sortino,drawdown_chronicity,downside_share,long_exposure,short_exposure,directional_bias,score,params_json",
  ];
  for (const row of rows) {
    const p = row.params || {};
    const m = row.metrics || {};
    const pnl = num(m.pnl, 0);
    const score = scoreOf(
      m,
      objective,
      scoreMode,
      p,
      scoreObservationContextOf(res),
    );
    const activeMean = derivedActiveMean(m);
    const activeDownsideDeviation = derivedActiveDownsideDeviation(m);
    const activeSortino = derivedActiveSortino(m);
    const chronicity = derivedDrawdownChronicity(m);
    const downsideShare = derivedDownsideShare(m);
    const longExposure = derivedLongExposure(m);
    const shortExposure = derivedShortExposure(m);
    const directionalBias = derivedDirectionalBias(m);
    const paramsJson = (() => {
      try {
        return JSON.stringify(p);
      } catch {
        return "{}";
      }
    })();
    lines.push(
      [
        row.rank,
        csvEscape(p.fast_ma_type || ""),
        num(p.fast_len, 0),
        csvEscape(p.slow_ma_type || ""),
        num(p.slow_len, 0),
        pnl,
        pnl * 100,
        num(finalEq(pnl, seed), 0),
        num(m.sharpe, 0),
        num(m.max_dd, 0),
        num(m.trades, 0),
        num(m.exposure, 0),
        num(m.net_exposure, 0),
        num(activeMean, 0),
        num(activeDownsideDeviation, 0),
        num(activeSortino, 0),
        num(chronicity, 0),
        num(downsideShare, 0),
        num(longExposure, 0),
        num(shortExposure, 0),
        num(directionalBias, 0),
        num(score, 0),
        csvEscape(paramsJson),
      ].join(","),
    );
  }
  if (
    downloadText(
      `vectorgrid_top_${sanitizeTs(run?.ts)}.csv`,
      lines.join("\n"),
      "text/csv",
    )
  ) {
    setStatus(store, "Exported top CSV.");
    return;
  }
  setStatus(store, "Failed to export top CSV.");
}

export async function exportTopJson(): Promise<void> {
  const store = readAnalysisStore();
  const run = readWorkflowRun();
  const res = run?.res;
  if (!res || typeof res !== "object") {
    setStatus(store, "No result to export yet.");
    return;
  }
  if (
    downloadText(
      `vectorgrid_run_${sanitizeTs(run?.ts)}.json`,
      JSON.stringify(res, null, 2),
      "application/json",
    )
  ) {
    setStatus(store, "Exported run JSON.");
    return;
  }
  setStatus(store, "Failed to export JSON.");
}

export async function refreshCompareSummary(): Promise<void> {
  const store = readAnalysisStore();
  const history = readHistoryStore();
  if (!store) return;
  const selected = text(history?.compareSelectedValue);
  if (!selected) {
    store.compareSummary = "-";
    markAnalysisVue(store);
    return;
  }

  const current = readWorkflowRun();
  const currentReq = asRecord(current?.req);
  const currentRes = asRecord(current?.res);
  if (
    !current ||
    !Object.keys(currentReq).length ||
    !Object.keys(currentRes).length
  ) {
    store.compareSummary = "Select a current run to compare.";
    markAnalysisVue(store);
    return;
  }

  compareSeq = (compareSeq + 1) >>> 0;
  const seq = compareSeq;
  store.compareSummary = "Loading compare…";
  markAnalysisVue(store);

  try {
    const baseline = await resolveBaselineSelection(selected, current, history);
    if (seq !== compareSeq) return;
    if (!baseline) {
      store.compareSummary =
        "Baseline run not found (refresh saved runs or pick another baseline).";
      markAnalysisVue(store);
      return;
    }
    const currentTarget = workflowRunToComparisonTarget("current", current);
    const baselineTarget = baselineRunToComparisonTarget(baseline);
    if (!currentTarget || !baselineTarget) {
      store.compareSummary =
        "Compare unavailable because one side is missing request/result data.";
      markAnalysisVue(store);
      return;
    }
    store.compareSummary = buildRunCompareSummary(currentTarget, baselineTarget);
    markAnalysisVue(store);
  } catch (error) {
    if (seq !== compareSeq) return;
    store.compareSummary = `Compare failed: ${String(error)}`;
    markAnalysisVue(store);
  }
}

export async function setBaseline(): Promise<void> {
  const store = readAnalysisStore();
  const history = readHistoryStore();
  const run = readWorkflowRun();
  const ts = text(run?.ts);
  const artifact = text(run?.artifactPath ?? run?.artifact_path);
  if (ts) {
    const value = artifact ? `disk:${artifact}` : `hist:${ts}`;
    if (history) {
      history.compareSelectedValue = value;
      markHistoryVue(history);
    }
    if (store) {
      store.compareSummary = `Baseline selected: ${value}`;
      setStatus(store, "Set baseline = current.");
      await refreshCompareSummary();
      return;
    }
  }
  setStatus(store, "Run an optimization or select a run from history first.");
}

export async function clearBaseline(): Promise<void> {
  const store = readAnalysisStore();
  const history = readHistoryStore();
  if (history) {
    history.compareSelectedValue = "";
    markHistoryVue(history);
  }
  if (store) {
    store.compareSummary = "-";
    setStatus(store, "Cleared baseline.");
    return;
  }
}

export async function heatmapExportPng(): Promise<void> {
  const run = readWorkflowRun();
  const req = asRecord(run?.req);
  const file = `heatmap_${String(req.objective || "heatmap").toLowerCase()}_${sanitizeTs(run?.ts)}.png`;
  if (registeredHeatmapCanvasController?.exportPng(file)) {
    setStatus(readAnalysisStore(), "Exported heatmap PNG.");
    return;
  }
  setStatus(readAnalysisStore(), "Failed to export heatmap PNG.");
}

export async function rerenderDrill(): Promise<void> {
  const store = readAnalysisStore();
  if (store) {
    const rows = readTopRows(store);
    if (rows.length) {
      const selected = rows.findIndex((row) => !!row?.selected);
      const bounded = selected >= 0 ? selected : 0;
      const model = resolveTopRowModelAtIndex(store, bounded);
      if (model && restoreDrillViewFromCache(store, model)) {
        return;
      }
      await selectTopRow(bounded);
    }
  }
}

export async function drillClearTradeSelection(): Promise<void> {
  const pinia = resolveRuntimePinia();
  if (pinia && useDataStore(pinia).polygonOpen) {
    return;
  }
  if (registeredDrillChartController?.clearTradeSelection()) {
    setStatus(readAnalysisStore(), "Cleared trade selection.");
    return;
  }
  setStatus(readAnalysisStore(), "No selected trade.");
}

export async function drillPrevTrade(): Promise<void> {
  const selected = registeredDrillChartController?.selectPrevTrade() ?? null;
  if (selected) {
    setStatus(readAnalysisStore(), `Selected trade ${selected}.`);
    return;
  }
  setStatus(readAnalysisStore(), "No previous trade.");
}

export async function drillNextTrade(): Promise<void> {
  const selected = registeredDrillChartController?.selectNextTrade() ?? null;
  if (selected) {
    setStatus(readAnalysisStore(), `Selected trade ${selected}.`);
    return;
  }
  setStatus(readAnalysisStore(), "No next trade.");
}

export const __analysisTestUtils = {
  buildDrillTradeMarkers,
  mapTradeIndexToChartTime,
  selectDrillTradeByIndex,
};
