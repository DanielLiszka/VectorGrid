import { resolveRuntimePinia } from "../stores/runtimePinia";
import { invokeCommand } from "../tauri/invoke";
import {
  buildValidationCapabilitySummary,
  buildValidationRequest,
} from "./validationRequestBuilder";
import {
  applyWorkflowArtifactsPayload,
  readLatestWorkflowRunSnapshot,
} from "../stores/workflowArtifactsStore";
import { useWorkflowStore } from "../stores/workflowStore";
import { computeConfigFingerprint } from "../utils/core/artifactIdentity";
import { buildDatasetMetaForArtifact } from "../utils/core/datasetMeta";
import {
  buildValidationStageCompletion,
  normalizeStageCompletions,
  serializeStageCompletionsForArtifact,
  updateStageCompletionReportPath,
  upsertStageCompletion,
  type StageCompletionPacketV1,
} from "../utils/core/stageCompletion";
import { openPathWithRevealFallback } from "../utils/openPath";
import { formatUiCount, formatUtcDateFromUnixMs } from "../utils/uiFormat";
import { buildValidationEvidenceSummary } from "../utils/validationEvidenceSummary";
import {
  measureValidationReportPerf,
  recordValidationReportPerf,
} from "./validationReportPerf";

interface PolygonCrossDatasetResponse {
  csv_path?: unknown;
  display_symbol?: unknown;
  polygon_ticker?: unknown;
  market?: unknown;
  timeframe?: unknown;
  min_ts?: unknown;
  max_ts?: unknown;
  last_updated_utc_ms?: unknown;
  row_count?: unknown;
  adjusted?: unknown;
}

interface PolygonCacheInfoResponse {
  datasets?: unknown;
}

export interface ValidationCrossDatasetSummary {
  path: string;
  label: string;
  meta: string;
  market: string;
  timeframe: string;
  adjusted: boolean | null;
}

export interface ValidationCrossRefreshSummary {
  datasets: ValidationCrossDatasetSummary[];
}

export interface ValidationContextCompatibilityPreview {
  state: "compatible" | "warning" | "incompatible" | "missing";
  summary: string;
  detail: string;
}

function normalizePath(value: string): string {
  return String(value || "").trim();
}

function canonicalPathKey(value: unknown): string {
  return normalizePath(String(value || "").trim())
    .replace(/\//g, "\\")
    .toLowerCase();
}

function normalizeText(value: unknown): string {
  return String(value || "").trim();
}

function canonicalizePreviewValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonicalizePreviewValue);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, nextValue]) => [key, canonicalizePreviewValue(nextValue)]),
    );
  }
  return value;
}

function canonicalPreviewKey(value: unknown): string {
  return JSON.stringify(canonicalizePreviewValue(value));
}

function normalizeStrategyKind(kindRaw: unknown, emptyFallback = ""): string {
  const kind = normalizeText(kindRaw);
  if (!kind) return emptyFallback;
  if (kind === "Pit") return "Pit";
  if (kind === "Set") return "Set";
  if (kind === "Mts") return "Mts";
  if (kind === "Orc") return "Orc";
  if (kind === "Marrt") return "Marrt";
  if (kind === "Vfct") return "Vfct";
  if (kind === "DynamicDoubleMa") return "DynamicDoubleMa";
  if (kind === "Rocb") return "Rocb";
  if (kind === "Vcet") return "Vcet";
  if (kind === "Asr") return "Asr";
  if (kind === "Vcc") return "Vcc";
  if (kind === "Vemr") return "Vemr";
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

function normalizeStrategyKindRaw(kindRaw: unknown): string {
  return normalizeStrategyKind(kindRaw, "DoubleMa");
}

function normalizeOptionalStrategyKind(kindRaw: unknown): string {
  return normalizeStrategyKind(kindRaw);
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

function isCompositeGpuNativeAdvancedKind(kindRaw: unknown): boolean {
  const kind = normalizeStrategyKindRaw(kindRaw);
  return (
    kind === "Asr" ||
    kind === "Vcc" ||
    kind === "Vemr" ||
    kind === "Rocb" ||
    kind === "Vcet" ||
    kind === "Mkcs" ||
    kind === "Kqsr" ||
    kind === "Carm" ||
    kind === "Kvdf" ||
    kind === "Cdmc"
  );
}

function describeTradeBootstrapRequestConfig(raw: unknown): string {
  const tradeBootstrap =
    raw && typeof raw === "object" ? (raw as Record<string, unknown>) : null;
  if (!tradeBootstrap) return "not run";
  const statBlockRaw = tradeBootstrap.stationary_mean_block_len;
  if (
    typeof statBlockRaw === "number" &&
    Number.isFinite(statBlockRaw) &&
    statBlockRaw === 0
  ) {
    return "trade bootstrap iid-only";
  }
  const statBlock =
    typeof statBlockRaw === "number" &&
    Number.isFinite(statBlockRaw) &&
    statBlockRaw > 0
      ? Math.floor(statBlockRaw)
      : null;
  return statBlock != null
    ? `trade bootstrap compare (iid + stationary, stat block=${statBlock})`
    : "trade bootstrap compare (iid + stationary, stat block=auto)";
}

function buildBootstrapSummary(raw: unknown): string {
  const tradeBootstrap =
    raw && typeof raw === "object" ? (raw as Record<string, unknown>) : null;
  if (!tradeBootstrap) return "not run";
  const statBlockRaw = tradeBootstrap.stationary_mean_block_len;
  if (
    typeof statBlockRaw === "number" &&
    Number.isFinite(statBlockRaw) &&
    statBlockRaw === 0
  ) {
    return "iid-only closed-trade bootstrap";
  }
  const statBlock =
    typeof statBlockRaw === "number" &&
    Number.isFinite(statBlockRaw) &&
    statBlockRaw > 0
      ? Math.floor(statBlockRaw)
      : null;
  return statBlock != null
    ? `iid + stationary closed-trade compare • stat block=${statBlock}`
    : "iid + stationary closed-trade compare • stat block=auto";
}

interface ValidationMonteCarloHistogramBar {
  label: string;
  count: number;
  pct: number;
}

interface ValidationMonteCarloSummary {
  summary: string;
  histogram: ValidationMonteCarloHistogramBar[];
}

function primaryMonteCarloMode(
  monteCarlo: Record<string, unknown>,
): { key: string; stats: Record<string, unknown> } | null {
  const modes = asRecord(monteCarlo.modes);
  if (!modes) return null;
  for (const key of ["trade_stationary", "trade_iid"]) {
    const stats = asRecord(modes[key]);
    if (stats) return { key, stats };
  }
  for (const [key, value] of Object.entries(modes)) {
    const stats = asRecord(value);
    if (stats) return { key, stats };
  }
  return null;
}

function formatMonteCarloModeLabel(key: string): string {
  if (key === "trade_stationary") return "stationary";
  if (key === "trade_iid") return "iid";
  return key.replace(/_/g, " ");
}

function buildMonteCarloHistogramBars(
  histogram: Record<string, unknown> | null,
): ValidationMonteCarloHistogramBar[] {
  const counts = Array.isArray(histogram?.counts) ? histogram.counts : [];
  if (!counts.length) return [];
  const min = Number(histogram?.min ?? Number.NaN);
  const max = Number(histogram?.max ?? Number.NaN);
  const total = counts.reduce((sum, value) => {
    const count = Math.max(0, Math.floor(Number(value ?? 0)));
    return sum + count;
  }, 0);
  const span = Number.isFinite(min) && Number.isFinite(max) ? max - min : 0;
  return counts.map((value, index) => {
    const count = Math.max(0, Math.floor(Number(value ?? 0)));
    const left =
      Number.isFinite(min) && Number.isFinite(span)
        ? min + (span * index) / counts.length
        : Number.NaN;
    const right =
      Number.isFinite(min) && Number.isFinite(span)
        ? min + (span * (index + 1)) / counts.length
        : Number.NaN;
    const label =
      Number.isFinite(left) && Number.isFinite(right)
        ? `${left.toFixed(3)} to ${right.toFixed(3)}`
        : `bin ${index + 1}`;
    return {
      label,
      count,
      pct: total > 0 ? count / total : 0,
    };
  });
}

function buildLegacyTradeBootstrapMonteCarloSummary(
  tradeBootstrap: Record<string, unknown>,
): ValidationMonteCarloSummary {
  const stationaryProb = Number(
    tradeBootstrap.stationary_prob_pnl_positive ?? Number.NaN,
  );
  const iidProb = Number(tradeBootstrap.prob_pnl_positive ?? Number.NaN);
  const hasStationary = Number.isFinite(stationaryProb);
  const prefix = hasStationary ? "stationary legacy" : "iid legacy";
  const prob = hasStationary ? stationaryProb : iidProb;
  const pnlP05 = Number(
    hasStationary ? tradeBootstrap.stationary_pnl_p05 : tradeBootstrap.pnl_p05,
  );
  const pnlP50 = Number(
    hasStationary ? tradeBootstrap.stationary_pnl_p50 : tradeBootstrap.pnl_p50,
  );
  const maxDdP95 = Number(
    hasStationary
      ? tradeBootstrap.stationary_max_dd_p95
      : tradeBootstrap.max_dd_p95,
  );
  const simulations = Math.max(
    0,
    Math.floor(Number(tradeBootstrap.simulations ?? 0)),
  );
  const tradeCount = Math.max(
    0,
    Math.floor(Number(tradeBootstrap.trade_count ?? 0)),
  );
  return {
    summary:
      `${prefix} • sims=${simulations} • trades=${tradeCount}` +
      `${Number.isFinite(prob) ? ` • p_pos=${fmtPct(prob, 1)}` : ""}` +
      `${Number.isFinite(pnlP05) ? ` • pnl_p05=${fmtFixed(pnlP05, 3)}` : ""}` +
      `${Number.isFinite(pnlP50) ? ` • pnl_p50=${fmtFixed(pnlP50, 3)}` : ""}` +
      `${Number.isFinite(maxDdP95) ? ` • dd_p95=${fmtFixed(maxDdP95, 3)}` : ""}`,
    histogram: [],
  };
}

function buildMonteCarloValidationSummary(
  result: unknown,
): ValidationMonteCarloSummary {
  const candidate = topValidationCandidateRecord(result);
  const monteCarlo = asRecord(candidate?.monte_carlo);
  if (monteCarlo) {
    const primary = primaryMonteCarloMode(monteCarlo);
    const advisory = asRecord(monteCarlo.advisory);
    const warnings = Array.isArray(advisory?.warning_flags)
      ? advisory.warning_flags.map((entry) => normalizeText(entry))
      : [];
    const failures = Array.isArray(advisory?.fail_reasons)
      ? advisory.fail_reasons.map((entry) => normalizeText(entry))
      : [];
    const advisoryNote =
      failures.find((entry) => entry.length > 0) ||
      warnings.find((entry) => entry.length > 0) ||
      "";
    if (!primary) {
      return {
        summary: `monte carlo • sims=${Math.max(0, Math.floor(Number(monteCarlo.simulations ?? 0)))} • trades=${Math.max(0, Math.floor(Number(monteCarlo.trade_count ?? 0)))} • no distribution`,
        histogram: [],
      };
    }
    const stats = primary.stats;
    const mode = formatMonteCarloModeLabel(primary.key);
    const simulations = Math.max(
      0,
      Math.floor(Number(stats.simulations ?? monteCarlo.simulations ?? 0)),
    );
    const tradeCount = Math.max(
      0,
      Math.floor(Number(monteCarlo.trade_count ?? 0)),
    );
    const prob = Number(stats.prob_pnl_positive ?? Number.NaN);
    const pnlP05 = Number(stats.pnl_p05 ?? Number.NaN);
    const pnlP50 = Number(stats.pnl_p50 ?? Number.NaN);
    const maxDdP95 = Number(stats.max_dd_p95 ?? Number.NaN);
    const passed = asRecord(monteCarlo.advisory)?.passed;
    const verdict =
      typeof passed === "boolean" ? (passed ? "pass" : "fail") : "";
    return {
      summary:
        `${mode} • sims=${simulations} • trades=${tradeCount}` +
        `${Number.isFinite(prob) ? ` • p_pos=${fmtPct(prob, 1)}` : ""}` +
        `${Number.isFinite(pnlP05) ? ` • pnl_p05=${fmtFixed(pnlP05, 3)}` : ""}` +
        `${Number.isFinite(pnlP50) ? ` • pnl_p50=${fmtFixed(pnlP50, 3)}` : ""}` +
        `${Number.isFinite(maxDdP95) ? ` • dd_p95=${fmtFixed(maxDdP95, 3)}` : ""}` +
        `${verdict ? ` • ${verdict}` : ""}` +
        `${advisoryNote ? ` • ${advisoryNote}` : ""}`,
      histogram: buildMonteCarloHistogramBars(asRecord(stats.pnl_histogram)),
    };
  }
  const tradeBootstrap = asRecord(candidate?.trade_bootstrap);
  if (tradeBootstrap) {
    return buildLegacyTradeBootstrapMonteCarloSummary(tradeBootstrap);
  }
  return { summary: "not run", histogram: [] };
}

function resolveValidationCommand(
  runReq: unknown,
): "run_double_ma_validation" | "run_composite_validation" {
  const req =
    runReq && typeof runReq === "object"
      ? (runReq as Record<string, unknown>)
      : null;
  const kind = normalizeStrategyKindRaw(
    req?.strategy_kind ?? req?.strategyKind,
  );
  return isDoubleLikeKind(kind)
    ? "run_double_ma_validation"
    : "run_composite_validation";
}

interface CrossSelectionEntry {
  csv_path?: unknown;
  path?: unknown;
  label?: unknown;
  checked?: unknown;
}

interface LegacyValidationBuildContext {
  currentRunReq: unknown;
  currentRunRes: unknown;
  topRows: Array<{
    params?: unknown;
    metrics?: unknown;
    mo?: unknown;
    source_kind?: unknown;
  }>;
  topRowsFiltered: Array<{
    params?: unknown;
    metrics?: unknown;
    mo?: unknown;
    source_kind?: unknown;
  }>;
}

interface RunSnapshot {
  req: unknown;
  res: unknown;
}

export interface ValidationRunOptions {
  useFiltered?: boolean;
  switchTab?: boolean;
}

interface SaveRunArtifactResponse {
  path?: unknown;
}

interface SaveReportXlsxResponse {
  path?: unknown;
  artifact_updated?: unknown;
  artifact_update_warning?: unknown;
}

interface ReportPayload {
  version: number;
  report_schema_version: number;
  kind: string;
  ts: string;
  data: {
    meta: Record<string, unknown> | null;
    ctx: Record<string, unknown>;
  };
  run: { req: unknown; res: unknown };
  validation: {
    req: unknown;
    res: unknown;
    acceleration_status?: string;
  } | null;
  optimization_metric_surface?: Record<string, unknown> | null;
}

interface WorkflowRunArtifactSnapshot {
  ts: string;
  artifactPath: string;
  reportXlsxPath: string;
  csvPath: string;
  datasetMeta: Record<string, unknown> | null;
  routeProvenance: Record<string, unknown> | null;
  mlSignal: Record<string, unknown> | null;
  optimizationMetricSurface: Record<string, unknown> | null;
  stageCompletions: StageCompletionPacketV1[];
  tags: string[];
  req: unknown;
  res: unknown;
}

interface WorkflowValidationArtifactSnapshot {
  ts: string;
  req: unknown;
  res: unknown;
}

interface ValidationTableColumn {
  key?: string;
  label: string;
  alignLeft: boolean;
}

interface ValidationTableCell {
  text: string;
  className: string;
  title: string;
}

interface ValidationTableRow {
  cells: ValidationTableCell[];
}

interface PersistValidationArtifactResult {
  path: string;
  ts: string;
  csvPath: string;
  reportXlsxPath: string;
  datasetMeta: Record<string, unknown> | null;
  mlSignal: Record<string, unknown> | null;
  optimizationMetricSurface: Record<string, unknown> | null;
  stageCompletions: StageCompletionPacketV1[];
  tags: string[];
  persistenceWarning: string;
}

interface ReportXlsxExportResult {
  statusLine: string;
  reportXlsxPath: string;
  artifactUpdated: boolean;
  artifactUpdateWarning: string;
}

interface ReportXlsxExportPrefs {
  autoReveal: boolean;
  maxRows: number | null;
  trigger: "auto" | "manual";
}

interface VueStoreFieldBinding {
  key: string;
}

const VUE_TEXT_FIELD_BINDINGS: Record<string, VueStoreFieldBinding> = {
  valCandidates: { key: "candidates" },
  valAdmissionPolicy: { key: "admissionPolicy" },
  valSeed: { key: "seed" },
  valMinPsr: { key: "minPsr" },
  valMinDsr: { key: "minDsr" },
  valMinCscvRank: { key: "minCscvRank" },
  valMinCscvMetric: { key: "minCscvMetric" },
  valMaxPbo: { key: "maxPbo" },
  valSpaSlices: { key: "spaSlices" },
  valSpaBootstrapSamples: { key: "spaBootstrapSamples" },
  valSpaBlockLength: { key: "spaBlockLength" },
  valReoptWfWindow: { key: "reoptWfWindow" },
  valReoptWfStep: { key: "reoptWfStep" },
  valReoptWfTopN: { key: "reoptWfTopN" },
  valWfWindow: { key: "wfWindow" },
  valWfStep: { key: "wfStep" },
  valPerturbIters: { key: "perturbIters" },
  valPerturbMin: { key: "perturbMin" },
  valPerturbMax: { key: "perturbMax" },
  valMcSims: { key: "mcSims" },
  valMcBlockLen: { key: "mcBlockLen" },
  valMcMode: { key: "mcMode" },
  valCscvSlices: { key: "cscvSlices" },
  valCrossMax: { key: "crossMax" },
  valCrossMode: { key: "crossMode" },
};

const VUE_CHECKBOX_FIELD_BINDINGS: Record<string, VueStoreFieldBinding> = {
  valWfEnable: { key: "wfEnable" },
  valPerturbEnable: { key: "perturbEnable" },
  valMcEnable: { key: "mcEnable" },
  valCscvEnable: { key: "cscvEnable" },
  valSpaEnable: { key: "spaEnable" },
  valReoptWfEnable: { key: "reoptWfEnable" },
  valCrossEnable: { key: "crossEnable" },
};

function readPiniaStoreState(storeId: string): Record<string, unknown> | null {
  const pinia = resolveRuntimePinia();
  if (!pinia) return null;
  const root = pinia.state.value as Record<string, unknown>;
  const store = root[storeId];
  if (!store || typeof store !== "object") return null;
  return store as Record<string, unknown>;
}

const VALIDATION_PANEL_SELECTOR = '[data-vue-panel="validate"]';

function readValidationStoreState(): Record<string, unknown> | null {
  return readPiniaStoreState("validation");
}

function readVueTextFieldValue(fieldId: string): string | null {
  const binding = VUE_TEXT_FIELD_BINDINGS[fieldId];
  if (!binding) return null;
  const store = readValidationStoreState();
  if (!store) return null;
  if (!Object.prototype.hasOwnProperty.call(store, binding.key)) return null;
  return normalizeText(store[binding.key]);
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

function readVueCheckboxFieldValue(fieldId: string): boolean | null {
  const binding = VUE_CHECKBOX_FIELD_BINDINGS[fieldId];
  if (!binding) return null;
  const store = readValidationStoreState();
  if (!store) return null;
  if (!Object.prototype.hasOwnProperty.call(store, binding.key)) return null;
  return normalizeBoolean(store[binding.key]);
}

function readVueCrossSelectionFromStore(): CrossSelectionEntry[] | null {
  const store = readValidationStoreState();
  if (!store) return null;
  const rows = store.crossDatasets;
  if (!Array.isArray(rows)) return null;

  const selected: CrossSelectionEntry[] = [];
  for (const entry of rows) {
    if (!entry || typeof entry !== "object") continue;
    const row = entry as CrossSelectionEntry;
    if (!normalizeBoolean(row.checked)) continue;
    const csv_path = normalizeText(row.csv_path ?? row.path);
    if (!csv_path) continue;
    selected.push({
      csv_path,
      label: normalizeText(row.label),
    });
  }
  return selected;
}

function readVueCrossSelection(): CrossSelectionEntry[] {
  const selectedFromStore = readVueCrossSelectionFromStore();
  return selectedFromStore || [];
}

function normalizeCrossSelectionEntries(
  entries: CrossSelectionEntry[],
): Array<{ csv_path: string; label: string }> {
  const selected: Array<{ csv_path: string; label: string }> = [];
  const seen = new Set<string>();
  const rows = Array.isArray(entries) ? entries : [];
  for (const entry of rows) {
    const csv_path = normalizePath(
      normalizeText(entry?.csv_path ?? entry?.path),
    );
    if (!csv_path || seen.has(csv_path)) continue;
    seen.add(csv_path);
    selected.push({
      csv_path,
      label: normalizeText(entry?.label),
    });
  }
  return selected;
}

function writeVueCrossSelection(entries: CrossSelectionEntry[]): void {
  const store = readValidationStoreState();
  if (!store) return;
  const rows = Array.isArray(store.crossDatasets) ? store.crossDatasets : [];
  if (!rows.length) return;

  const selected = normalizeCrossSelectionEntries(entries);
  const selectedPaths = new Set(selected.map((entry) => entry.csv_path));
  let changed = false;

  const nextRows = rows.map((entry) => {
    if (!entry || typeof entry !== "object") return entry;
    const row = entry as Record<string, unknown>;
    const path = normalizePath(normalizeText(row.path));
    if (!path) return entry;
    const nextChecked = selectedPaths.has(path);
    const prevChecked = normalizeBoolean(row.checked);
    if (prevChecked === nextChecked) return entry;
    changed = true;
    return {
      ...row,
      checked: nextChecked,
    };
  });

  if (!changed) return;
  store.crossDatasets = nextRows;
  store.crossTotalCount = nextRows.length;
  store.crossSelectedCount = nextRows.filter(
    (entry) =>
      !!entry &&
      typeof entry === "object" &&
      normalizeBoolean((entry as Record<string, unknown>).checked),
  ).length;
  store.source = "vue";
  store.updatedAtUnixMs = Date.now();
}

function normalizeMarketLabel(market: string, adjusted: boolean): string {
  if (!market) return "market";
  if (market !== "stocks") return market;
  return adjusted ? "stocks (adj)" : "stocks (raw)";
}

function normalizeRowsHint(value: unknown): string {
  return formatUiCount(value);
}

function formatYmdUtc(value: unknown): string {
  return formatUtcDateFromUnixMs(value);
}

function readStoreTextInputValue(id: string): string {
  const key = String(id || "").trim();
  if (!key) return "";
  const vueValue = readVueTextFieldValue(key);
  return vueValue != null ? vueValue : "";
}

function readStoreCheckboxValue(id: string): boolean {
  const key = String(id || "").trim();
  if (!key) return false;
  const vueValue = readVueCheckboxFieldValue(key);
  return vueValue != null ? vueValue : false;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object") return null;
  return value as Record<string, unknown>;
}

function extractTopRowsFromRunResult(runRes: unknown): Array<{
  params?: unknown;
  metrics?: unknown;
  mo?: unknown;
  source_kind?: unknown;
}> {
  const row = asRecord(runRes);
  if (!row) return [];
  const rows: Array<{
    params?: unknown;
    metrics?: unknown;
    mo?: unknown;
    source_kind?: unknown;
  }> = [];

  const paretoRowsRaw = Array.isArray(row.pareto) ? row.pareto : [];
  if (paretoRowsRaw.length > 0) {
    const pts = paretoRowsRaw
      .map((entry) => {
        const point = asRecord(entry);
        return {
          params: asRecord(point?.params),
          metrics: asRecord(point?.metrics),
          maximize_value: Number(
            point?.maximize_value ?? Number.NEGATIVE_INFINITY,
          ),
          minimize_value: Number(
            point?.minimize_value ?? Number.POSITIVE_INFINITY,
          ),
          violation: Number(point?.violation ?? 0),
        };
      })
      .filter((entry) => !!entry.params);
    pts.sort((left, right) => {
      const leftFeasible = left.violation <= 0;
      const rightFeasible = right.violation <= 0;
      if (leftFeasible !== rightFeasible) return leftFeasible ? -1 : 1;
      if (left.maximize_value !== right.maximize_value)
        return right.maximize_value - left.maximize_value;
      if (left.minimize_value !== right.minimize_value)
        return left.minimize_value - right.minimize_value;
      return 0;
    });
    for (const entry of pts) {
      rows.push({
        params: entry.params,
        metrics: entry.metrics,
        mo: {
          maximize_value: entry.maximize_value,
          minimize_value: entry.minimize_value,
          violation: entry.violation,
        },
        source_kind: "pareto_frontier",
      });
    }
    return rows;
  }

  const topRowsRaw = Array.isArray(row.top) ? row.top : [];
  for (const entry of topRowsRaw) {
    if (Array.isArray(entry)) {
      rows.push({
        params: entry[0],
        metrics: entry[1],
        source_kind: "top_rows",
      });
      continue;
    }
    const item = asRecord(entry);
    if (!item || !item.params) continue;
    rows.push({
      params: item.params,
      metrics: item.metrics,
      source_kind: "top_rows",
    });
  }

  if (rows.length > 0) return rows;
  const bestParams = row.best_params;
  if (!bestParams || typeof bestParams !== "object") return [];
  return [
    {
      params: bestParams,
      metrics: row.best_metrics,
      source_kind: "best_params_fallback",
    },
  ];
}

function readRunSnapshotFromStore(): RunSnapshot | null {
  const snapshot = readLatestWorkflowRunSnapshot();
  if (!snapshot) return null;
  return {
    req: snapshot.req ?? null,
    res: snapshot.res ?? null,
  };
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

function readWorkflowRunArtifactFromStore(): WorkflowRunArtifactSnapshot | null {
  const workflowArtifactsStore = readPiniaStoreState("workflowArtifacts");
  if (!workflowArtifactsStore) return null;
  const run = workflowArtifactsStore.currentRun;
  if (!run || typeof run !== "object") return null;
  const row = run as Record<string, unknown>;
  return {
    ts: normalizeText(row.ts) || new Date().toISOString(),
    artifactPath: normalizeText(row.artifactPath ?? row.artifact_path),
    reportXlsxPath: normalizeText(row.reportXlsxPath ?? row.report_xlsx_path),
    csvPath: normalizeText(row.csvPath ?? row.csv_path),
    datasetMeta:
      row.datasetMeta && typeof row.datasetMeta === "object"
        ? (row.datasetMeta as Record<string, unknown>)
        : row.dataset_meta && typeof row.dataset_meta === "object"
          ? (row.dataset_meta as Record<string, unknown>)
          : null,
    routeProvenance:
      row.routeProvenance && typeof row.routeProvenance === "object"
        ? (row.routeProvenance as Record<string, unknown>)
        : row.route_provenance && typeof row.route_provenance === "object"
          ? (row.route_provenance as Record<string, unknown>)
          : null,
    mlSignal:
      row.mlSignal && typeof row.mlSignal === "object"
        ? (row.mlSignal as Record<string, unknown>)
        : row.ml_signal && typeof row.ml_signal === "object"
          ? (row.ml_signal as Record<string, unknown>)
          : null,
    optimizationMetricSurface:
      row.optimizationMetricSurface &&
      typeof row.optimizationMetricSurface === "object"
        ? (row.optimizationMetricSurface as Record<string, unknown>)
        : row.optimization_metric_surface &&
            typeof row.optimization_metric_surface === "object"
          ? (row.optimization_metric_surface as Record<string, unknown>)
          : null,
    stageCompletions: normalizeStageCompletions(
      row.stageCompletions ?? row.stage_completions,
    ),
    tags: normalizeStringList(row.tags),
    req: row.req ?? null,
    res: row.res ?? null,
  };
}

function readWorkflowValidationArtifactFromStore(): WorkflowValidationArtifactSnapshot | null {
  const workflowArtifactsStore = readPiniaStoreState("workflowArtifacts");
  if (!workflowArtifactsStore) return null;
  const validation = workflowArtifactsStore.currentValidation;
  if (!validation || typeof validation !== "object") return null;
  const row = validation as Record<string, unknown>;
  return {
    ts: normalizeText(row.ts) || new Date().toISOString(),
    req: row.req ?? null,
    res: row.res ?? null,
  };
}

function readLatestMetaReportPath(): string {
  if (typeof document === "undefined") return "";
  const fromRuntime = normalizeText(
    document.documentElement?.dataset?.latestMetaReportPath,
  );
  const fromRuntimeExists = normalizeOptionalBoolean(
    document.documentElement?.dataset?.latestMetaReportExists,
  );
  if (fromRuntime && fromRuntimeExists !== false) return fromRuntime;
  const commandActionsStore = readPiniaStoreState("commandActions");
  const fromStore = normalizeText(commandActionsStore?.latestMetaReportPath);
  const fromStoreExists = normalizeOptionalBoolean(
    commandActionsStore?.latestMetaReportExists,
  );
  if (fromStore && fromStoreExists !== false) return fromStore;
  return "";
}

function readValidationExportPrefs(): {
  autoExport: boolean;
  autoReveal: boolean;
  maxRows: number | null;
} {
  const store = readValidationStoreState();
  const autoExport = normalizeBoolean(store?.autoExportXlsx);
  const autoReveal = normalizeBoolean(store?.autoRevealReport);
  const rawTopN = Math.floor(Number(store?.autoExportTopN));
  const maxRows = Number.isFinite(rawTopN) && rawTopN > 0 ? rawTopN : null;
  return { autoExport, autoReveal, maxRows };
}

function nowMs(): number {
  return typeof performance !== "undefined" ? performance.now() : Date.now();
}

function formatValidationStatus(
  commandMs: number,
  endToEndMs: number,
  suffix = "",
): string {
  const base = `Validation completed in ${Math.max(0, endToEndMs).toFixed(0)}ms end-to-end (${Math.max(0, commandMs).toFixed(0)}ms command)`;
  return suffix ? `${base}. ${suffix}` : base;
}

function buildReportDataFromDatasetMeta(
  datasetMeta: Record<string, unknown> | null,
): ReportPayload["data"] {
  const datasetState = buildRunDatasetContextState();
  if (!datasetMeta) {
    const meta = datasetState.currentDataMeta
      ? { ...datasetState.currentDataMeta }
      : {};
    const ctx = { ...datasetState.currentDatasetCtx };
    return {
      meta: Object.keys(meta).length > 0 ? meta : null,
      ctx,
    };
  }

  const dataStore = readPiniaStoreState("data");
  const artifactCsvPath = normalizeText(datasetMeta.csv_path);
  const artifactTimeframe = normalizeText(
    datasetMeta.timeframe ??
      datasetMeta.derived_timeframe ??
      datasetMeta.base_timeframe,
  );
  const liveCsvPath = normalizeText(dataStore?.csvPath);
  const liveTimeframe = normalizeText(
    datasetState.currentDatasetCtx.timeframe ??
      datasetState.currentDataMeta?.timeframe,
  );
  const canReuseLiveDatasetState =
    artifactCsvPath.length > 0 &&
    liveCsvPath.length > 0 &&
    artifactCsvPath === liveCsvPath &&
    (!artifactTimeframe ||
      !liveTimeframe ||
      artifactTimeframe === liveTimeframe);
  const meta =
    canReuseLiveDatasetState && datasetState.currentDataMeta
      ? { ...datasetState.currentDataMeta }
      : {};
  const ctx = canReuseLiveDatasetState
    ? { ...datasetState.currentDatasetCtx }
    : {};

  const textKeys = [
    "timeframe",
    "base_timeframe",
    "derived_timeframe",
    "source",
    "market",
    "symbol",
    "label",
    "csv_path",
    "resample_rule",
    "resample_anchor",
    "source_content_fingerprint",
  ];
  for (const key of textKeys) {
    const next = normalizeText(datasetMeta[key]);
    if (!next) continue;
    if (
      key === "label" ||
      key === "symbol" ||
      key === "source" ||
      key === "market" ||
      key === "csv_path"
    ) {
      ctx[key] = next;
    } else {
      meta[key] = next;
      if (
        key === "timeframe" ||
        key === "base_timeframe" ||
        key === "derived_timeframe" ||
        key === "resample_rule" ||
        key === "resample_anchor"
      ) {
        ctx[key] = next;
      }
    }
  }

  const numericKeys = [
    "len",
    "min_ts",
    "max_ts",
    "source_size_bytes",
    "source_mtime_utc_ms",
  ];
  for (const key of numericKeys) {
    const next = Number(datasetMeta[key]);
    if (!Number.isFinite(next)) continue;
    meta[key] = next;
  }

  if (typeof datasetMeta.resample_applied === "boolean") {
    ctx.resample_applied = datasetMeta.resample_applied;
  }
  if (typeof datasetMeta.adjusted === "boolean") {
    ctx.adjusted = datasetMeta.adjusted;
  }

  return {
    meta: Object.keys(meta).length > 0 ? meta : null,
    ctx,
  };
}

function buildReportPayload(input: {
  ts?: unknown;
  datasetMeta?: Record<string, unknown> | null;
  optimizationMetricSurface?: Record<string, unknown> | null;
  runReq: unknown;
  runRes: unknown;
  validationReq: unknown;
  validationRes: unknown;
}): ReportPayload {
  const ts = normalizeText(input.ts) || new Date().toISOString();
  const accelerationStatus =
    input.validationRes != null
      ? buildValidationAccelerationStatus(
          input.validationRes,
          input.validationReq,
        )
      : "";
  const payload: ReportPayload = {
    version: 1,
    report_schema_version: 1,
    kind: "vectorgrid_report",
    ts,
    data: buildReportDataFromDatasetMeta(input.datasetMeta ?? null),
    run: {
      req: input.runReq ?? null,
      res: input.runRes ?? null,
    },
    validation:
      input.validationRes != null
        ? {
            req: input.validationReq ?? null,
            res: input.validationRes,
            acceleration_status: accelerationStatus,
          }
        : null,
  };
  if (input.optimizationMetricSurface) {
    payload.optimization_metric_surface = input.optimizationMetricSurface;
  }
  return payload;
}

function buildCurrentReportPayloadFromStore(): ReportPayload {
  const run = readWorkflowRunArtifactFromStore();
  const validation = readWorkflowValidationArtifactFromStore();
  return buildReportPayload({
    ts: run?.ts || validation?.ts,
    datasetMeta: run?.datasetMeta ?? null,
    optimizationMetricSurface: run?.optimizationMetricSurface ?? null,
    runReq: run?.req ?? null,
    runRes: run?.res ?? null,
    validationReq: validation?.req ?? null,
    validationRes: validation?.res ?? null,
  });
}

function downloadBlob(filename: string, blob: Blob): boolean {
  if (typeof document === "undefined") return false;
  if (typeof URL === "undefined" || typeof URL.createObjectURL !== "function")
    return false;
  const href = URL.createObjectURL(blob);
  try {
    const link = document.createElement("a");
    link.href = href;
    link.download = filename;
    link.style.display = "none";
    document.body?.appendChild(link);
    link.click();
    link.remove();
    return true;
  } finally {
    URL.revokeObjectURL(href);
  }
}

function downloadText(filename: string, text: string, mime: string): boolean {
  try {
    return downloadBlob(filename, new Blob([text], { type: mime }));
  } catch {
    return false;
  }
}

function decodeBase64(value: string): Uint8Array | null {
  const src = String(value || "").trim();
  if (!src) return null;
  if (typeof atob === "function") {
    const bin = atob(src);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i += 1) {
      out[i] = bin.charCodeAt(i) & 0xff;
    }
    return out;
  }
  return null;
}

function downloadBase64(
  filename: string,
  base64: string,
  mime: string,
): boolean {
  const bytes = decodeBase64(base64);
  if (!bytes) return false;
  const copy = new Uint8Array(bytes.length);
  copy.set(bytes);
  return downloadBlob(filename, new Blob([copy.buffer], { type: mime }));
}

function buildRunDatasetContextState(): {
  currentDataMeta: Record<string, unknown> | null;
  currentBaseDataMeta: Record<string, unknown> | null;
  currentBaseDatasetCtx: Record<string, unknown>;
  currentDatasetCtx: Record<string, unknown>;
} {
  const dataStore = readPiniaStoreState("data");
  const dataFields = Array.isArray(dataStore?.dataFields)
    ? dataStore.dataFields
    : [];
  const fields = new Set<string>();
  for (const entry of dataFields) {
    const key = normalizeText(entry).toLowerCase();
    if (!key) continue;
    fields.add(key);
  }
  const len = Number(dataStore?.dataLen);
  const baseMeta = asRecord(dataStore?.baseDataMeta);
  const timeframe = normalizeText(
    dataStore?.dataTimeframe || baseMeta?.timeframe,
  );
  const dataMeta: Record<string, unknown> = {
    has_open: fields.has("open"),
    has_high: fields.has("high"),
    has_low: fields.has("low"),
    has_close: fields.has("close"),
    has_volume: fields.has("volume"),
  };
  if (Number.isFinite(len) && len >= 0) dataMeta.len = Math.floor(len);
  if (timeframe) dataMeta.timeframe = timeframe;

  const datasetCtx: Record<string, unknown> = {};
  const label = normalizeText(dataStore?.datasetLabel);
  const source = normalizeText(dataStore?.datasetContext);
  const symbol = normalizeText(dataStore?.polygonSymbol || label);
  if (label && label !== "—") datasetCtx.label = label;
  if (symbol && symbol !== "—") datasetCtx.symbol = symbol;
  if (source && source !== "—") datasetCtx.source = source;
  if (timeframe) {
    datasetCtx.timeframe = timeframe;
    datasetCtx.base_timeframe = timeframe;
    datasetCtx.derived_timeframe = timeframe;
    datasetCtx.resample_applied = false;
  }

  return {
    currentDataMeta: dataMeta,
    currentBaseDataMeta: dataMeta,
    currentBaseDatasetCtx: datasetCtx,
    currentDatasetCtx: datasetCtx,
  };
}

function canonicalLower(value: unknown): string {
  return normalizeText(value).toLowerCase();
}

function firstNonEmptyText(...values: unknown[]): string {
  for (const value of values) {
    const next = normalizeText(value);
    if (next) return next;
  }
  return "";
}

function assertValidationRunMatchesCurrentDataset(
  run: WorkflowRunArtifactSnapshot,
  runReq: Record<string, unknown>,
): void {
  const datasetMeta = run.datasetMeta;
  const expectedCsvPath = normalizePath(
    normalizeText(datasetMeta?.csv_path ?? run.csvPath),
  );
  const expectedTimeframe = canonicalLower(
    firstNonEmptyText(
      datasetMeta?.timeframe,
      datasetMeta?.derived_timeframe,
      datasetMeta?.base_timeframe,
    ),
  );
  if (!expectedCsvPath && !expectedTimeframe) return;

  const dataStore = readPiniaStoreState("data");
  const currentCsvPath = normalizePath(normalizeText(dataStore?.csvPath));
  if (expectedCsvPath && !currentCsvPath) {
    throw new Error(
      "Load the same dataset used by the optimization artifact before running validation.",
    );
  }

  if (expectedCsvPath && currentCsvPath !== expectedCsvPath) {
    const expectedLabel = firstNonEmptyText(
      datasetMeta?.symbol,
      datasetMeta?.label,
      expectedCsvPath,
    );
    throw new Error(
      `Validation requires the optimization dataset ${expectedLabel} (${expectedCsvPath}).`,
    );
  }

  const currentTimeframe = canonicalLower(
    firstNonEmptyText(
      dataStore?.dataTimeframe,
      asRecord(dataStore?.baseDataMeta)?.timeframe,
    ),
  );
  if (
    currentTimeframe &&
    expectedTimeframe &&
    currentTimeframe !== expectedTimeframe
  ) {
    throw new Error(
      `Validation requires timeframe ${expectedTimeframe}; current dataset is ${currentTimeframe}.`,
    );
  }

  const artifactStrategy = normalizeOptionalStrategyKind(
    runReq.strategy_kind ?? runReq.strategyKind,
  );
  const runtimeStrategy = normalizeOptionalStrategyKind(
    run.req && typeof run.req === "object"
      ? ((run.req as Record<string, unknown>).strategy_kind ??
          (run.req as Record<string, unknown>).strategyKind)
      : "",
  );
  if (runtimeStrategy && artifactStrategy !== runtimeStrategy) {
    throw new Error(
      `Validation requires strategy ${runtimeStrategy}; current run context is ${artifactStrategy}.`,
    );
  }
}

function resolveValidationCompatibilityContext(
  workflowRun: WorkflowRunArtifactSnapshot,
  runReq: Record<string, unknown> | null,
  validationReq: Record<string, unknown> | null,
): {
  expectedCsvPath: string;
  expectedLabel: string;
  expectedTimeframe: string;
  expectedStrategy: string;
} {
  const validationContext = asRecord(validationReq?.validation_context);
  const datasetMeta = workflowRun.datasetMeta;
  return {
    expectedCsvPath: normalizePath(
      firstNonEmptyText(
        validationContext?.csv_path,
        datasetMeta?.csv_path,
        workflowRun.csvPath,
      ),
    ),
    expectedLabel:
      firstNonEmptyText(
        validationContext?.label,
        datasetMeta?.symbol,
        datasetMeta?.label,
        validationContext?.csv_path,
        datasetMeta?.csv_path,
        workflowRun.csvPath,
      ) || "artifact dataset",
    expectedTimeframe: canonicalLower(
      firstNonEmptyText(
        validationContext?.timeframe,
        datasetMeta?.timeframe,
        datasetMeta?.derived_timeframe,
        datasetMeta?.base_timeframe,
      ),
    ),
    expectedStrategy: normalizeOptionalStrategyKind(
      validationContext?.strategy_kind ??
        runReq?.strategy_kind ??
        runReq?.strategyKind,
    ),
  };
}

export function previewValidationContextCompatibilityFromStores(): ValidationContextCompatibilityPreview {
  const workflowRun = readWorkflowRunArtifactFromStore();
  if (!workflowRun) {
    return {
      state: "missing",
      summary: "Awaiting optimization artifact",
      detail:
        "Run or reopen an optimization artifact before validating so dataset, timeframe, and strategy compatibility can be checked.",
    };
  }

  const artifactReq = asRecord(workflowRun.req);
  const runSnapshot = readRunSnapshotFromStore();
  const runReq = asRecord(runSnapshot?.req ?? artifactReq);
  const validationReq = asRecord(
    readWorkflowValidationArtifactFromStore()?.req,
  );
  const {
    expectedCsvPath,
    expectedLabel,
    expectedTimeframe,
    expectedStrategy,
  } = resolveValidationCompatibilityContext(workflowRun, runReq, validationReq);
  const currentStrategy = normalizeOptionalStrategyKind(
    runReq?.strategy_kind ?? runReq?.strategyKind,
  );

  if (
    expectedStrategy &&
    currentStrategy &&
    expectedStrategy !== currentStrategy
  ) {
    return {
      state: "incompatible",
      summary: "Incompatible: strategy mismatch",
      detail: `Validation artifact expects strategy ${expectedStrategy}; current run context is ${currentStrategy}.`,
    };
  }

  const dataStore = readPiniaStoreState("data");
  const currentCsvPath = normalizePath(normalizeText(dataStore?.csvPath));
  const currentLabel =
    firstNonEmptyText(dataStore?.datasetLabel, currentCsvPath) ||
    "loaded dataset";
  const currentTimeframe = canonicalLower(
    firstNonEmptyText(
      dataStore?.dataTimeframe,
      asRecord(dataStore?.baseDataMeta)?.timeframe,
    ),
  );

  if (expectedCsvPath && !currentCsvPath) {
    return {
      state: "missing",
      summary: `Load ${expectedLabel}`,
      detail: `Validation artifact expects dataset ${expectedLabel} (${expectedCsvPath}). Load that dataset before running validation.`,
    };
  }

  if (
    expectedCsvPath &&
    currentCsvPath &&
    canonicalPathKey(currentCsvPath) !== canonicalPathKey(expectedCsvPath)
  ) {
    return {
      state: "incompatible",
      summary: "Incompatible: dataset mismatch",
      detail: `Validation artifact expects dataset ${expectedLabel} (${expectedCsvPath}); loaded dataset is ${currentLabel} (${currentCsvPath}).`,
    };
  }

  if (
    expectedTimeframe &&
    currentTimeframe &&
    currentTimeframe !== expectedTimeframe
  ) {
    return {
      state: "incompatible",
      summary: "Incompatible: timeframe mismatch",
      detail: `Validation artifact expects timeframe ${expectedTimeframe}; loaded dataset is ${currentTimeframe}.`,
    };
  }

  if (expectedTimeframe && !currentTimeframe) {
    return {
      state: "warning",
      summary: "Partial: current timeframe unavailable",
      detail: `Validation artifact expects timeframe ${expectedTimeframe}, but the currently loaded dataset does not expose an active timeframe label yet.`,
    };
  }

  const missingArtifactContext: string[] = [];
  if (!expectedCsvPath) missingArtifactContext.push("dataset");
  if (!expectedTimeframe) missingArtifactContext.push("timeframe");
  if (!expectedStrategy) missingArtifactContext.push("strategy");
  if (missingArtifactContext.length > 0) {
    return {
      state: "warning",
      summary: `Partial: missing ${missingArtifactContext.join(" + ")}`,
      detail: `This artifact is missing ${missingArtifactContext.join(", ")} metadata, so compatibility cannot be fully confirmed before validation runs.`,
    };
  }

  return {
    state: "compatible",
    summary: "Compatible: dataset + timeframe + strategy aligned",
    detail: `Loaded dataset ${expectedLabel}${expectedTimeframe ? ` (${expectedTimeframe})` : ""} matches the artifact context${expectedStrategy ? ` for ${expectedStrategy}` : ""}.`,
  };
}

function patchValidationStoreState(patch: Record<string, unknown>): void {
  const store = readValidationStoreState();
  if (!store) return;
  Object.assign(store, patch);
  store.source = "vue";
  store.updatedAtUnixMs = Date.now();
}

function safePrettyJson(value: unknown): string {
  try {
    return JSON.stringify(value ?? null, null, 2);
  } catch {
    return String(value ?? "");
  }
}

function toOutputPreview(value: unknown): string {
  const text = safePrettyJson(value);
  return text.length > 200 ? `${text.slice(0, 200).trimEnd()}…` : text;
}

function queryDomById<T extends Element>(
  id: string,
  scopeSelector?: string,
): T | null {
  if (typeof document === "undefined") return null;
  const selector = `[id="${id}"]`;
  if (scopeSelector) {
    if (typeof document.querySelector !== "function") return null;
    const scopeRoot = document.querySelector(
      scopeSelector,
    ) as HTMLElement | null;
    if (!scopeRoot) return null;
    return scopeRoot.querySelector(selector) as T | null;
  }
  const byQuery =
    typeof document.querySelector === "function"
      ? document.querySelector(selector)
      : null;
  if (byQuery) return byQuery as T;
  const getById = (
    document as unknown as {
      getElementById?: (nextId: string) => Element | null;
    }
  ).getElementById;
  if (typeof getById !== "function") return null;
  return (getById.call(document, id) as T | null) ?? null;
}

function writeDomTextContent(
  id: string,
  value: string,
  scopeSelector?: string,
): void {
  const el = queryDomById<HTMLElement>(id, scopeSelector);
  if (!el) return;
  el.textContent = value;
}

function selectWorkflowResultsTab(tab: "validate", persist: boolean): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  const workflowStore = useWorkflowStore(pinia);
  workflowStore.selectResultsTab(tab, { persist });
}

function formatValidationEvidencePolicyMeta(result: unknown): string {
  const row = asRecord(result);
  const summary = asRecord(row?.evidence_policy_summary);
  const metrics = Array.isArray(summary?.metrics)
    ? summary.metrics
        .map((entry) => asRecord(entry))
        .filter((entry): entry is Record<string, unknown> => !!entry)
    : [];
  if (!metrics.length) return "";

  const probabilitySampleUnits = metrics
    .filter((metric) => {
      const name = normalizeText(metric?.metric).toLowerCase();
      return name === "psr" || name === "dsr";
    })
    .map((metric) => normalizeText(metric?.sample_unit))
    .filter(
      (value, index, values) =>
        value.length > 0 && values.indexOf(value) === index,
    );
  const hasSetLevelCscv = metrics.some(
    (metric) => normalizeText(metric?.metric).toLowerCase() === "cscv_pbo",
  );

  const parts: string[] = [];
  if (probabilitySampleUnits.length > 0) {
    parts.push(`prob ${probabilitySampleUnits.join("/")}`);
  }
  if (hasSetLevelCscv) parts.push("cscv set-level");
  return parts.length > 0 ? ` • evidence=${parts.join(", ")}` : "";
}

function buildValidationMetaText(
  result: unknown,
  validationReq?: unknown,
): string {
  const row = asRecord(result);
  const candidates = Array.isArray(row?.candidates) ? row.candidates.length : 0;
  const runtimeMs = Math.max(0, Math.floor(Number(row?.runtime_ms ?? 0)));
  const backendUsed = normalizeText(row?.backend_used);
  const backendRoute = normalizeText(row?.backend_route);
  const backendFallbackReason = normalizeText(row?.backend_fallback_reason);
  const backendParts: string[] = [];
  if (backendUsed) backendParts.push(`backend=${backendUsed}`);
  if (backendRoute) backendParts.push(`route=${backendRoute}`);
  if (backendFallbackReason) {
    const routeLabel = backendRoute.includes("->CpuOnly")
      ? "fallback"
      : "route_note";
    backendParts.push(`${routeLabel}=${backendFallbackReason}`);
  }
  const backendSummary =
    backendParts.length > 0 ? backendParts.join(" • ") : "backend";
  const cross = asRecord(row?.cross_ticker);
  const crossDatasets = Array.isArray(cross?.datasets)
    ? cross.datasets.length
    : 0;
  const crossModeRaw = normalizeText(cross?.mode).toLowerCase();
  const crossMode =
    crossModeRaw === "preserved_finalists"
      ? "preserved_finalists"
      : crossModeRaw === "full_stack"
        ? "full_stack"
        : crossModeRaw === "lightweight"
          ? "lightweight"
          : "";
  const crossSuffix =
    crossDatasets > 0
      ? ` • cross=${crossDatasets}${crossMode ? ` (${crossMode})` : ""}`
      : "";

  const cscv = asRecord(row?.cscv_pbo);
  const cscvSkipped = normalizeText(cscv?.skipped_reason);
  const cscvCombos = Math.max(0, Math.floor(Number(cscv?.combinations ?? 0)));
  const cscvPbo = Number(cscv?.pbo ?? Number.NaN);
  let cscvSuffix = "";
  if (cscvSkipped) cscvSuffix = ` • cscv skipped=${cscvSkipped}`;
  else if (cscv && cscvCombos > 0 && Number.isFinite(cscvPbo))
    cscvSuffix = ` • pbo=${cscvPbo.toFixed(3)} (cs=${cscvCombos})`;

  const reopt = asRecord(row?.reopt_walk_forward);
  const reoptWindows = Math.max(0, Math.floor(Number(reopt?.windows ?? 0)));
  const reoptSharpe = Number(reopt?.forward_sharpe_median ?? Number.NaN);
  const reoptUlcer = Number(reopt?.forward_ulcer_index_worst ?? Number.NaN);
  const reoptMartin = Number(reopt?.forward_martin_ratio_median ?? Number.NaN);
  const reoptSuffix =
    reoptWindows > 0
      ? ` • reopt_wf=${reoptWindows}w` +
        `${Number.isFinite(reoptSharpe) ? ` sh_med=${reoptSharpe.toFixed(3)}` : ""}` +
        `${Number.isFinite(reoptUlcer) ? ` ulcer_worst=${reoptUlcer.toFixed(3)}` : ""}` +
        `${Number.isFinite(reoptMartin) ? ` martin_med=${reoptMartin.toFixed(2)}` : ""}`
      : "";
  const spa = asRecord(row?.spa);
  const spaP = Number(spa?.p_value ?? Number.NaN);
  const spaSkipped = normalizeText(spa?.skipped_reason);
  const spaSuffix = spaSkipped
    ? ` • spa skipped=${spaSkipped}`
    : spa
      ? ` • spa=${Number.isFinite(spaP) ? spaP.toFixed(3) : "-"}${spa?.passed === false ? " fail" : " pass"}`
      : "";

  const runVerdict = asRecord(row?.run_verdict);
  const survived = Math.max(
    0,
    Math.floor(Number(runVerdict?.candidates_survived_hard_gates ?? 0)),
  );
  const tested = Math.max(
    0,
    Math.floor(Number(runVerdict?.candidates_tested ?? candidates)),
  );
  const approved = Math.max(
    0,
    Math.floor(Number(runVerdict?.approved_candidate_count ?? 0)),
  );
  const recommendation = normalizeText(runVerdict?.final_recommendation);
  const warningFlags = Array.isArray(runVerdict?.warning_flags)
    ? runVerdict?.warning_flags
        .map((value) => normalizeText(value))
        .filter((value) => value.length > 0)
    : [];
  const failReasons = Array.isArray(runVerdict?.fail_reasons)
    ? runVerdict?.fail_reasons
        .map((value) => normalizeText(value))
        .filter((value) => value.length > 0)
    : [];
  const topObs = Math.max(
    0,
    Math.floor(Number(runVerdict?.top_candidate_track_record_bars ?? 0)),
  );
  const topMinTrl = Math.max(
    0,
    Math.floor(
      Number(runVerdict?.top_candidate_min_track_record_length_bars ?? 0),
    ),
  );
  const approvalSuffix = runVerdict
    ? ` • approved=${approved}/${survived}${recommendation ? ` • rec=${recommendation}` : ""}`
    : "";
  const minTrlSuffix =
    topObs > 0 && topMinTrl > 0 ? ` • top_mintrl=${topObs}/${topMinTrl}` : "";
  const hasRunVerdict = !!runVerdict;
  const runState = hasRunVerdict
    ? ` • hard_gates=${survived}/${tested}${runVerdict?.passed === false ? " fail" : ""}`
    : "";
  const runReason = failReasons[0] || warningFlags[0] || "";
  const runReasonSuffix = runReason ? ` • note=${runReason}` : "";
  const req = asRecord(validationReq);
  const admissionPolicy = normalizeText(req?.admission_policy);
  const admissionProvenance = asRecord(req?.admission_provenance);
  const admissionSource = normalizeText(admissionProvenance?.source_kind);
  const admissionPostCap = Math.max(
    0,
    Math.floor(Number(admissionProvenance?.post_cap_count ?? 0)),
  );
  const admissionPostDedupe = Math.max(
    0,
    Math.floor(Number(admissionProvenance?.post_dedupe_count ?? 0)),
  );
  const admissionRequested = normalizeText(
    admissionProvenance?.requested_policy,
  );
  const admissionHeads = Array.isArray(admissionProvenance?.heads)
    ? admissionProvenance.heads
        .map((entry) => asRecord(entry))
        .map((entry) => normalizeText(entry?.objective ?? entry?.head))
        .filter(
          (value, index, values) =>
            value.length > 0 && values.indexOf(value) === index,
        )
        .slice(0, 4)
        .join("/")
    : "";
  const admissionSummary = admissionPolicy
    ? ` • admission=${admissionPolicy}` +
      (admissionRequested && admissionRequested !== admissionPolicy
        ? ` req=${admissionRequested}`
        : "") +
      (admissionSource ? ` src=${admissionSource}` : "") +
      (admissionPostDedupe > 0
        ? ` kept=${admissionPostCap}/${admissionPostDedupe}`
        : "") +
      (admissionHeads ? ` heads=${admissionHeads}` : "")
    : "";
  const evidencePolicySuffix = formatValidationEvidencePolicyMeta(result);

  return `validated ${candidates} candidates in ${runtimeMs}ms (${backendSummary})${admissionSummary}${crossSuffix}${cscvSuffix}${reoptSuffix}${spaSuffix}${runState}${approvalSuffix}${runReasonSuffix}${minTrlSuffix}${evidencePolicySuffix}`;
}

function buildValidationHintText(result: unknown): string {
  const row = asRecord(result);
  const warnings: string[] = [];

  const backendRoute = normalizeText(row?.backend_route);
  const backendFallbackReason = normalizeText(row?.backend_fallback_reason);
  if (backendFallbackReason) {
    const label = backendRoute.includes("->CpuOnly")
      ? "route fallback"
      : "route note";
    warnings.push(`${label}: ${backendFallbackReason}`);
  }

  const cscv = asRecord(row?.cscv_pbo);
  const cscvSkipped = normalizeText(cscv?.skipped_reason);
  if (cscvSkipped) warnings.push(`CSCV skipped`);

  const spa = asRecord(row?.spa);
  const spaSkipped = normalizeText(spa?.skipped_reason);
  if (spaSkipped) warnings.push(`SPA skipped`);

  const runVerdict = asRecord(row?.run_verdict);
  const warningFlags = Array.isArray(runVerdict?.warning_flags)
    ? runVerdict.warning_flags
        .map((value) => normalizeText(value))
        .filter((value) => value.length > 0)
    : [];
  if (warningFlags.length > 0) warnings.push(`run notes: ${warningFlags[0]}`);

  if (!warnings.length) return "Validation complete.";
  return `Validation complete with warnings: ${warnings.join(" • ")}`;
}

function topValidationCandidateRecord(
  result: unknown,
): Record<string, unknown> | null {
  const row = asRecord(result);
  const candidates = validationCandidates(result);
  if (candidates.length === 0) return null;
  const runVerdict = asRecord(row?.run_verdict);
  const firstApprovedRank = Math.max(
    0,
    Math.floor(Number(runVerdict?.first_approved_rank ?? 0)),
  );
  if (firstApprovedRank > 0) {
    const approvedCandidate = candidates[firstApprovedRank - 1];
    if (approvedCandidate) return approvedCandidate;
  }
  return candidates[0] || null;
}

function deriveRegimeFragilityLabel(
  stableBucketCount: number,
  worstSharpe: number,
  worstMaxDd: number,
): string {
  if (stableBucketCount >= 4 && worstSharpe >= 0 && worstMaxDd <= 0.35)
    return "stable";
  if (stableBucketCount >= 2 && worstSharpe >= -0.1 && worstMaxDd <= 0.45)
    return "selective";
  return "fragile";
}

function deriveRegimeSupportLabel(regime: Record<string, unknown>): string {
  const explicit = normalizeText(regime.support_label);
  if (explicit) return explicit;
  const trend = asRecord(regime.trend);
  const range = asRecord(regime.range);
  const trendStable =
    Number(trend?.pnl ?? Number.NaN) >= 0 &&
    Number(trend?.sharpe ?? Number.NaN) >= 0;
  const rangeStable =
    Number(range?.pnl ?? Number.NaN) >= 0 &&
    Number(range?.sharpe ?? Number.NaN) >= 0;
  const stableBucketCount = Math.max(
    0,
    Math.floor(Number(regime.stable_bucket_count ?? 0)),
  );
  if (trendStable && rangeStable) return "broad";
  if (trendStable) return "trend_only";
  if (rangeStable) return "range_only";
  return stableBucketCount >= 2 ? "vol_selective" : "weak";
}

function buildValidationRegimeSummary(result: unknown): string {
  const candidate = topValidationCandidateRecord(result);
  const regime = asRecord(candidate?.regime_slice_robustness);
  if (!regime) return "not run";
  const stableBucketCount = Math.max(
    0,
    Math.floor(Number(regime.stable_bucket_count ?? 0)),
  );
  const stableBucketFrac = Number(regime.stable_bucket_frac ?? Number.NaN);
  const worstBucket = normalizeText(regime.worst_bucket_label) || "unknown";
  const worstSharpe = Number(regime.worst_sharpe ?? Number.NaN);
  const worstMaxDd = Number(regime.worst_max_dd ?? Number.NaN);
  const worstUlcer = Number(regime.worst_ulcer_index ?? Number.NaN);
  const worstMartin = Number(regime.worst_martin_ratio ?? Number.NaN);
  const flag = deriveRegimeFragilityLabel(
    stableBucketCount,
    Number.isFinite(worstSharpe) ? worstSharpe : Number.NEGATIVE_INFINITY,
    Number.isFinite(worstMaxDd) ? worstMaxDd : Number.POSITIVE_INFINITY,
  );
  const support = deriveRegimeSupportLabel(regime);
  const sharpeSuffix = Number.isFinite(worstSharpe)
    ? ` • sh=${worstSharpe.toFixed(2)}`
    : "";
  const ulcerSuffix = Number.isFinite(worstUlcer)
    ? ` • ulcer=${worstUlcer.toFixed(3)}`
    : "";
  const martinSuffix = Number.isFinite(worstMartin)
    ? ` • martin=${worstMartin.toFixed(2)}`
    : "";
  const stableSuffix = Number.isFinite(stableBucketFrac)
    ? ` • stable=${stableBucketCount}/5 (${(stableBucketFrac * 100).toFixed(0)}%)`
    : ` • stable=${stableBucketCount}/5`;
  return `${flag} • support=${support} • worst=${worstBucket}${stableSuffix}${sharpeSuffix}${ulcerSuffix}${martinSuffix}`;
}

function buildValidationFrictionSummary(result: unknown): string {
  const candidate = topValidationCandidateRecord(result);
  const friction = asRecord(candidate?.friction_stress);
  if (!friction) return "not run";
  const passFrac = Number(friction.pass_frac ?? Number.NaN);
  const maxRankDrift = Math.max(
    0,
    Math.floor(Number(friction.max_rank_drift ?? 0)),
  );
  const worstScenario =
    normalizeText(friction.worst_scenario_label) || "unknown";
  const worstUlcer = Number(friction.worst_ulcer_index ?? Number.NaN);
  const worstMartin = Number(friction.worst_martin_ratio ?? Number.NaN);
  const flag =
    Number.isFinite(passFrac) && passFrac >= 1 && maxRankDrift < 2
      ? "stable"
      : "fragile";
  const passSuffix = Number.isFinite(passFrac)
    ? ` • pass=${(passFrac * 100).toFixed(1)}%`
    : "";
  const ulcerSuffix = Number.isFinite(worstUlcer)
    ? ` • ulcer=${worstUlcer.toFixed(3)}`
    : "";
  const martinSuffix = Number.isFinite(worstMartin)
    ? ` • martin=${worstMartin.toFixed(2)}`
    : "";
  return `${flag} • worst=${worstScenario}${passSuffix} • drift=${maxRankDrift}${ulcerSuffix}${martinSuffix} • absolute pnl/sharpe rule`;
}

type ValidationModuleCategory =
  | "GPU-native"
  | "GPU main + CPU tail"
  | "CPU-only"
  | "lightweight rerun only"
  | "not supported";

function enabledExecutionCategoryModules(
  req: Record<string, unknown> | null | undefined,
): string[] {
  const modules = ["base reruns", "hard gates"];
  if (req?.walk_forward && typeof req.walk_forward === "object")
    modules.push("walk_forward");
  if (req?.reopt_walk_forward && typeof req.reopt_walk_forward === "object")
    modules.push("reopt_walk_forward");
  if (req?.cscv_pbo && typeof req.cscv_pbo === "object")
    modules.push("cscv_pbo");
  if (req?.spa && typeof req.spa === "object") modules.push("spa");
  if (req?.perturb && typeof req.perturb === "object") modules.push("perturb");
  if (req?.trade_bootstrap && typeof req.trade_bootstrap === "object")
    modules.push("trade_bootstrap");
  else if (req?.monte_carlo && typeof req.monte_carlo === "object")
    modules.push("monte_carlo");
  return modules;
}

function formatExecutionCategorySummary(
  groups: Array<[ValidationModuleCategory, string[]]>,
): string {
  const parts = groups
    .filter(([, labels]) => labels.length > 0)
    .map(([category, labels]) => `${category}: ${labels.join(" + ")}`);
  return parts.join(" | ") || "none";
}

function buildActualValidationCategorySummary(
  validationReq: Record<string, unknown> | null | undefined,
  row: Record<string, unknown> | null | undefined,
  strategyKind: string,
): string {
  const modules = enabledExecutionCategoryModules(validationReq);
  const backendUsed = normalizeText(row?.backend_used).toLowerCase();
  const backendRoute = normalizeText(row?.backend_route);
  if (modules.length === 0) return "none";
  if (backendUsed !== "gpu" || backendRoute.includes("->CpuOnly")) {
    return formatExecutionCategorySummary([["CPU-only", modules]]);
  }
  if (!isDoubleLikeKind(strategyKind)) {
    const gpuNative: string[] = [];
    const gpuMainCpuTail: string[] = [];
    const gpuNativeAdvancedKind =
      isCompositeGpuNativeAdvancedKind(strategyKind);
    const baseCategory = gpuNative;
    baseCategory.push("base reruns", "hard gates");
    if (
      validationReq?.walk_forward &&
      typeof validationReq.walk_forward === "object"
    ) {
      gpuNative.push("walk_forward");
    }
    if (
      validationReq?.reopt_walk_forward &&
      typeof validationReq.reopt_walk_forward === "object"
    ) {
      gpuNative.push("reopt_walk_forward");
    }
    if (validationReq?.cscv_pbo && typeof validationReq.cscv_pbo === "object") {
      gpuNative.push("cscv_pbo");
    }
    if (validationReq?.spa && typeof validationReq.spa === "object") {
      gpuNative.push("spa");
    }
    if (validationReq?.perturb && typeof validationReq.perturb === "object") {
      (gpuNativeAdvancedKind ? gpuNative : gpuMainCpuTail).push("perturb");
    }
    if (
      validationReq?.trade_bootstrap &&
      typeof validationReq.trade_bootstrap === "object"
    ) {
      (gpuNativeAdvancedKind ? gpuNative : gpuMainCpuTail).push(
        "trade_bootstrap",
      );
    } else if (
      validationReq?.monte_carlo &&
      typeof validationReq.monte_carlo === "object"
    ) {
      (gpuNativeAdvancedKind ? gpuNative : gpuMainCpuTail).push("monte_carlo");
    }
    return formatExecutionCategorySummary([
      ["GPU-native", gpuNative],
      ["GPU main + CPU tail", gpuMainCpuTail],
    ]);
  }
  const gpuNative: string[] = ["base reruns", "hard gates"];
  if (
    validationReq?.walk_forward &&
    typeof validationReq.walk_forward === "object"
  )
    gpuNative.push("walk_forward");
  if (
    validationReq?.reopt_walk_forward &&
    typeof validationReq.reopt_walk_forward === "object"
  )
    gpuNative.push("reopt_walk_forward");
  if (validationReq?.cscv_pbo && typeof validationReq.cscv_pbo === "object")
    gpuNative.push("cscv_pbo");
  if (validationReq?.spa && typeof validationReq.spa === "object")
    gpuNative.push("spa");
  const gpuMainCpuTail: string[] = [];
  if (validationReq?.perturb && typeof validationReq.perturb === "object")
    gpuMainCpuTail.push("perturb");
  if (
    validationReq?.trade_bootstrap &&
    typeof validationReq.trade_bootstrap === "object"
  )
    gpuMainCpuTail.push("trade_bootstrap");
  else if (
    validationReq?.monte_carlo &&
    typeof validationReq.monte_carlo === "object"
  )
    gpuMainCpuTail.push("monte_carlo");
  return formatExecutionCategorySummary([
    ["GPU-native", gpuNative],
    ["GPU main + CPU tail", gpuMainCpuTail],
  ]);
}

function buildValidationAccelerationStatus(
  result: unknown,
  validationReq?: unknown,
): string {
  const row = asRecord(result);
  const backendUsed = normalizeText(row?.backend_used);
  const backendUsedLower = backendUsed.toLowerCase();
  const backendRoute = normalizeText(row?.backend_route);
  const backendRouteLower = backendRoute.toLowerCase();
  const backendFallbackReason = normalizeText(row?.backend_fallback_reason);
  const requestedBackend =
    normalizeText(row?.requested_backend) ||
    normalizeText(asRecord(asRecord(validationReq)?.backend)?.backend);
  const validationContext = asRecord(
    asRecord(validationReq)?.validation_context,
  );
  const strategyKind = normalizeStrategyKindRaw(
    validationContext?.strategy_kind,
  );
  const categorySummary = buildActualValidationCategorySummary(
    asRecord(validationReq),
    row,
    strategyKind,
  );
  const executionProfile = asRecord(row?.execution_profile);
  const cpuTailWallMs = Number(
    executionProfile?.cpu_tail_wall_ms ?? Number.NaN,
  );
  const hasCpuTail =
    categorySummary.includes("GPU main + CPU tail") ||
    (Number.isFinite(cpuTailWallMs) && cpuTailWallMs > 0);

  if (backendUsedLower === "gpu" && !backendRouteLower.includes("->cpuonly")) {
    return hasCpuTail
      ? "GPU validation completed with CPU follow-up checks."
      : "GPU validation completed.";
  }

  if (
    backendRouteLower.includes("->cpuonly") ||
    (backendUsedLower === "cpu" &&
      requestedBackend &&
      requestedBackend.toLowerCase() !== "cpuonly")
  ) {
    return backendFallbackReason
      ? `GPU validation unavailable: ${backendFallbackReason}`
      : "CPU validation used because GPU validation was unavailable.";
  }

  if (backendUsedLower === "cpu") return "CPU validation completed.";
  return "Validation acceleration status unavailable.";
}

function compositeValidationRouteDetailNote(strategyKind: string): string {
  switch (strategyKind) {
    case "CompositeCpu":
      return "Plain CompositeCpu GPU validation currently requires uniform HalfTrend parameters across finalists; mixed finalist sets can fall back to CPU.";
    case "Asr":
      return "ASR GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.";
    case "Vcc":
      return "VCC GPU validation now runs the current fixed-candidate validation module stack on the GpuKernel route while keeping strict validation parity.";
    case "Vemr":
      return "VEMR GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.";
    case "Rocb":
      return "ROCB GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.";
    case "Vcet":
      return "VCET GPU validation now supports the current fixed-candidate validation module stack on the GpuKernel route, including walk-forward, reopt walk-forward, CSCV/PBO, SPA, perturb, and trade bootstrap.";
    case "Mkcs":
      return "MKCS GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.";
    case "Kqsr":
      return "KQSR GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.";
    case "Kvdf":
      return "KVDF GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.";
    case "Carm":
      return "CARM GPU validation now runs the current fixed-candidate validation module stack on the GpuKernel route. Current advanced-stack benches show CPU faster than GPU on the measured 50k/8 and 100k/32 workloads.";
    case "Cdmc":
      return "CDMC GPU validation now runs the current fixed-candidate validation module stack on the GpuKernel route. Fresh advanced-stack benches show GPU faster than CPU on the measured 50k/8 and 100k/32 workloads.";
    default:
      return "";
  }
}

function compositeValidationAutoCpuPreferenceNote(
  strategyKind: string,
): string {
  switch (strategyKind) {
    case "Asr":
      return "Auto composite validation currently prefers CPU for ASR because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.";
    case "Vcc":
      return "";
    case "Vemr":
      return "Auto composite validation currently prefers CPU for VEMR because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.";
    case "Rocb":
      return "Auto composite validation currently prefers CPU for ROCB because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.";
    case "Mkcs":
      return "Auto composite validation currently prefers CPU for MKCS because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.";
    case "Kqsr":
      return "Auto composite validation currently prefers CPU for KQSR because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.";
    case "Kvdf":
      return "Auto composite validation currently prefers CPU for KVDF because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload.";
    case "Carm":
      return "Auto composite validation currently prefers CPU for CARM because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 and 100k/32 workloads.";
    default:
      return "";
  }
}

function buildValidationExecutionSummary(
  result: unknown,
  validationReq?: unknown,
): {
  accelerationStatus: string;
  routeSummary: string;
  routeDetailSummary: string;
  categorySummary: string;
  auxCpuSummary: string;
  bootstrapSummary: string;
  monteCarloSummary: string;
  monteCarloHistogram: ValidationMonteCarloHistogramBar[];
  regimeSummary: string;
  frictionSummary: string;
  crossSummary: string;
  skippedSummary: string;
  warningSummary: string;
} {
  const row = asRecord(result);
  const req = asRecord(validationReq);
  const validationContext = asRecord(req?.validation_context);
  const strategyKind = normalizeStrategyKindRaw(
    validationContext?.strategy_kind,
  );
  const isDoubleLike = isDoubleLikeKind(strategyKind);
  const backendUsed = normalizeText(row?.backend_used) || "unknown";
  const backendRoute = normalizeText(row?.backend_route) || "-";
  const backendFallbackReason = normalizeText(row?.backend_fallback_reason);
  const requestedBackend =
    normalizeText(row?.requested_backend) ||
    normalizeText(asRecord(asRecord(validationReq)?.backend)?.backend);
  const validationResidencyTier = normalizeText(row?.validation_residency_tier);
  const executionProfile = asRecord(row?.execution_profile);
  const cpuTailWallMs = Number(
    executionProfile?.cpu_tail_wall_ms ?? Number.NaN,
  );
  const compositeMoCpuPreferred =
    strategyKind === "CompositeMo" &&
    backendUsed.toLowerCase() === "cpu" &&
    backendRoute.toLowerCase().includes("cpuonly");
  const compositeMoRouteNote = compositeMoCpuPreferred
    ? "CompositeMo uses the CPU-preferred validation route; GPU fixed-candidate validation currently follows the same plain CompositeCpu HardGate eligibility contract (uniform strategy/indicator settings, vectorized engine, no inline cross_ticker)."
    : "";
  const compositeAutoCpuPreferred =
    strategyKind === "Carm" &&
    requestedBackend === "Auto" &&
    backendUsed.toLowerCase() === "cpu" &&
    backendRoute.toLowerCase().includes("cpuonly");
  const compositeAutoCpuRouteNote = compositeAutoCpuPreferred
    ? compositeValidationAutoCpuPreferenceNote(strategyKind)
    : "";
  const compositeGpuRouteDetail =
    backendUsed.toLowerCase() === "gpu"
      ? compositeValidationRouteDetailNote(strategyKind)
      : "";
  const routeNoteParts = [
    backendFallbackReason,
    compositeMoRouteNote,
    compositeAutoCpuRouteNote,
    compositeGpuRouteDetail,
  ].filter((entry) => entry.length > 0);
  const routeNoteSummary = routeNoteParts.join(" • ");
  const routeModeLabel =
    backendRoute.includes("->CpuOnly") &&
    !compositeMoCpuPreferred &&
    !compositeAutoCpuPreferred
      ? "fallback"
      : "route note";
  const actualAuxCpuTails: string[] = [];
  const compositeGpuNativeAdvanced =
    !isDoubleLike && isCompositeGpuNativeAdvancedKind(strategyKind);
  if (
    backendUsed.toLowerCase() === "gpu" &&
    !backendRoute.includes("->CpuOnly")
  ) {
    if (
      (isDoubleLike || !compositeGpuNativeAdvanced) &&
      req?.perturb &&
      typeof req.perturb === "object"
    ) {
      actualAuxCpuTails.push("perturb");
    }
    if (req?.trade_bootstrap && typeof req.trade_bootstrap === "object") {
      if (isDoubleLike || !compositeGpuNativeAdvanced) {
        actualAuxCpuTails.push(
          describeTradeBootstrapRequestConfig(req.trade_bootstrap),
        );
      }
    } else if (req?.monte_carlo && typeof req.monte_carlo === "object") {
      if (isDoubleLike || !compositeGpuNativeAdvanced) {
        actualAuxCpuTails.push(
          describeTradeBootstrapRequestConfig(req.monte_carlo),
        );
      }
    }
  }
  const routeSummaryBase = routeNoteSummary
    ? `${backendUsed} • ${backendRoute} • ${routeModeLabel}: ${routeNoteSummary}`
    : `${backendUsed} • ${backendRoute}`;
  const routeDetailParts = [
    routeNoteSummary || "none",
    requestedBackend ? `requested_backend=${requestedBackend}` : "",
    validationResidencyTier ? `residency=${validationResidencyTier}` : "",
    Number.isFinite(cpuTailWallMs)
      ? `cpu_tail=${Math.max(0, Math.floor(cpuTailWallMs))}ms`
      : "",
  ].filter((entry) => entry.length > 0);
  const routeDetailSummary = routeDetailParts.join(" • ");
  const auxCpuSummary =
    actualAuxCpuTails.length > 0 ? actualAuxCpuTails.join(" + ") : "none";
  const routeSummary =
    actualAuxCpuTails.length > 0
      ? `${routeSummaryBase} • aux CPU tails: ${auxCpuSummary}`
      : routeSummaryBase;
  const auxCpuDesignNote =
    actualAuxCpuTails.length > 0
      ? `Aux CPU tails are intentional in the current validation route: ${auxCpuSummary}.`
      : "";
  const categorySummary = buildActualValidationCategorySummary(
    req,
    row,
    strategyKind,
  );
  const accelerationStatus = buildValidationAccelerationStatus(
    result,
    validationReq,
  );

  const bootstrapSummary = buildBootstrapSummary(
    req?.monte_carlo ?? req?.trade_bootstrap,
  );
  const monteCarlo = buildMonteCarloValidationSummary(result);
  const regimeSummary = buildValidationRegimeSummary(result);
  const frictionSummary = buildValidationFrictionSummary(result);

  const cross = asRecord(row?.cross_ticker);
  const crossDatasets = Array.isArray(cross?.datasets)
    ? cross.datasets.length
    : 0;
  const requestedCross = asRecord(req?.cross_ticker);
  const crossMode =
    normalizeText(cross?.mode) || normalizeText(requestedCross?.mode);
  const preservedFinalistsMode = crossMode === "preserved_finalists";
  const fullStackMode = crossMode === "full_stack";
  const crossSummary =
    crossDatasets > 0
      ? isDoubleLike
        ? `${crossDatasets} dataset${crossDatasets === 1 ? "" : "s"}${crossMode ? ` • ${crossMode}` : ""}${preservedFinalistsMode || fullStackMode ? " • finalists only (cap=4)" : ""}${fullStackMode ? " • full stack" : ""}`
        : `${crossDatasets} dataset${crossDatasets === 1 ? "" : "s"}${crossMode ? ` • requested=${crossMode}` : ""} • ${fullStackMode ? "full stack finalists (cap=4)" : preservedFinalistsMode ? "finalists only (cap=4)" : "lightweight reruns"}`
      : "not run";

  const skipped: string[] = [];
  if (
    req?.cross_ticker &&
    typeof req.cross_ticker === "object" &&
    crossDatasets === 0
  ) {
    skipped.push("Cross: unavailable");
  }
  if (isDoubleLike && preservedFinalistsMode) {
    skipped.push(
      "Cross reruns: omit walk_forward + reopt WF + SPA + perturb + trade bootstrap",
    );
  } else if (!isDoubleLike && requestedCross && !fullStackMode) {
    skipped.push(
      preservedFinalistsMode
        ? "Cross reruns: preserved_finalists omit walk_forward + reopt WF + SPA + perturb + trade bootstrap"
        : "Cross reruns: composite main route does not support inline cross_ticker; lightweight composite reruns omit walk_forward + reopt WF + SPA + perturb + trade bootstrap + CSCV + hard gates",
    );
  }
  if (
    req?.reopt_walk_forward &&
    typeof req.reopt_walk_forward === "object" &&
    !asRecord(row?.reopt_walk_forward)
  ) {
    skipped.push("Reopt WF: unavailable");
  }
  const cscv = asRecord(row?.cscv_pbo);
  const cscvSkipped = normalizeText(cscv?.skipped_reason);
  if (req?.cscv_pbo && typeof req.cscv_pbo === "object") {
    skipped.push(
      cscvSkipped ? `CSCV: ${cscvSkipped}` : !cscv ? "CSCV: unavailable" : "",
    );
  }
  const spa = asRecord(row?.spa);
  const spaSkipped = normalizeText(spa?.skipped_reason);
  if (req?.spa && typeof req.spa === "object") {
    skipped.push(
      spaSkipped ? `SPA: ${spaSkipped}` : !spa ? "SPA: unavailable" : "",
    );
  }
  const skippedSummary =
    skipped.filter((entry) => entry.length > 0).join(" • ") || "none";

  const runVerdict = asRecord(row?.run_verdict);
  const failReasons = Array.isArray(runVerdict?.fail_reasons)
    ? runVerdict.fail_reasons
        .map((entry) => normalizeText(entry))
        .filter((entry) => entry.length > 0)
    : [];
  const warningFlags = Array.isArray(runVerdict?.warning_flags)
    ? runVerdict.warning_flags
        .map((entry) => normalizeText(entry))
        .filter((entry) => entry.length > 0)
    : [];
  const warningSummary =
    failReasons[0] ||
    warningFlags[0] ||
    compositeMoRouteNote ||
    auxCpuDesignNote ||
    "none";

  return {
    accelerationStatus,
    routeSummary,
    routeDetailSummary,
    categorySummary,
    auxCpuSummary,
    bootstrapSummary,
    monteCarloSummary: monteCarlo.summary,
    monteCarloHistogram: monteCarlo.histogram,
    regimeSummary,
    frictionSummary,
    crossSummary,
    skippedSummary,
    warningSummary,
  };
}

function compactWinnerLabel(value: unknown): string {
  const raw = normalizeText(value);
  if (!raw) return "—";
  try {
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const fastMa = normalizeText(parsed.fast_ma_type || "?");
    const slowMa = normalizeText(parsed.slow_ma_type || "?");
    const fastLen = Math.max(0, Math.floor(Number(parsed.fast_len || 0)));
    const slowLen = Math.max(0, Math.floor(Number(parsed.slow_len || 0)));
    const base = `${fastMa || "?"}(${fastLen || "?"})/${slowMa || "?"}(${slowLen || "?"})`;
    const overlays: string[] = [];
    for (const key of [
      "asr",
      "vcc",
      "vemr",
      "rocb",
      "mkcs",
      "kqsr",
      "carm",
      "kvdf",
      "cdmc",
    ]) {
      const entry = asRecord(parsed[key]);
      if (entry) overlays.push(key);
    }
    if (parsed.halftrend || parsed.halftrend_filter) overlays.push("halftrend");
    return overlays.length > 0 ? `${base}+${overlays.join("+")}` : base;
  } catch {
    return raw.length > 40 ? `${raw.slice(0, 37)}...` : raw;
  }
}

function buildValidationReoptMetaText(result: unknown): string {
  const row = asRecord(result);
  const reopt = asRecord(row?.reopt_walk_forward);
  if (!reopt) return "-";
  const windows = Math.max(0, Math.floor(Number(reopt.windows ?? 0)));
  if (windows <= 0) return "-";

  const topN = Math.max(1, Math.floor(Number(reopt.top_n_candidates ?? 1)));
  const uniqueWinners = Math.max(
    0,
    Math.floor(Number(reopt.unique_winners ?? 0)),
  );
  const winnerChanges = Math.max(
    0,
    Math.floor(Number(reopt.winner_changes ?? 0)),
  );
  const driftMean = Number(reopt.winner_param_drift_mean ?? Number.NaN);
  const driftMax = Number(reopt.winner_param_drift_max ?? Number.NaN);
  const hitRate = Number(reopt.forward_positive_pnl_frac ?? Number.NaN);
  const sharpeMedian = Number(reopt.forward_sharpe_median ?? Number.NaN);
  const ddWorst = Number(reopt.forward_max_dd_worst ?? Number.NaN);
  const ulcerWorst = Number(reopt.forward_ulcer_index_worst ?? Number.NaN);
  const martinMedian = Number(reopt.forward_martin_ratio_median ?? Number.NaN);
  const sequenceRaw = Array.isArray(reopt.winner_sequence)
    ? reopt.winner_sequence
    : [];
  const sequence = sequenceRaw
    .slice(0, 4)
    .map((entry) => compactWinnerLabel(entry))
    .filter((entry) => entry.length > 0);
  const sequenceSuffix =
    sequence.length > 0
      ? ` • winners=${sequence.join(" -> ")}${sequenceRaw.length > sequence.length ? " -> ..." : ""}`
      : "";

  return (
    `reopt_wf: ${windows} windows • topN=${topN} • unique=${uniqueWinners} • changes=${winnerChanges}` +
    `${Number.isFinite(hitRate) ? ` • oos_hit=${fmtPct(hitRate, 1)}` : ""}` +
    `${Number.isFinite(sharpeMedian) ? ` • sh_med=${sharpeMedian.toFixed(3)}` : ""}` +
    `${Number.isFinite(ddWorst) ? ` • dd_worst=${ddWorst.toFixed(3)}` : ""}` +
    `${Number.isFinite(ulcerWorst) ? ` • ulcer_worst=${ulcerWorst.toFixed(3)}` : ""}` +
    `${Number.isFinite(martinMedian) ? ` • martin_med=${martinMedian.toFixed(2)}` : ""}` +
    `${Number.isFinite(driftMean) ? ` • drift_mean=${driftMean.toFixed(3)}` : ""}` +
    `${Number.isFinite(driftMax) ? ` • drift_max=${driftMax.toFixed(3)}` : ""}` +
    sequenceSuffix
  );
}

function fmt(value: unknown, digits = 6): string {
  const n = Number(value);
  if (!Number.isFinite(n)) return "-";
  return n.toFixed(digits);
}

function fmtFixed(value: number, digits = 2): string {
  if (!Number.isFinite(value)) return "-";
  return value.toFixed(digits);
}

function fmtPct(value: unknown, digits = 1): string {
  const n = Number(value);
  if (!Number.isFinite(n)) return "-";
  return `${(n * 100).toFixed(digits)}%`;
}

function parseProbabilityGateValue(value: unknown): number | null {
  const raw = normalizeText(value);
  if (!raw) return null;
  const n = Number(raw);
  if (!Number.isFinite(n)) return null;
  return Math.max(0, Math.min(1, n));
}

function parseFiniteNumberValue(value: unknown): number | null {
  const raw = normalizeText(value);
  if (!raw) return null;
  const n = Number(raw);
  return Number.isFinite(n) ? n : null;
}

function validationCandidates(result: unknown): Array<Record<string, unknown>> {
  const row = asRecord(result);
  const xs = Array.isArray(row?.candidates) ? row.candidates : [];
  const out: Array<Record<string, unknown>> = [];
  for (const entry of xs) {
    const candidate = asRecord(entry);
    if (!candidate) continue;
    out.push(candidate);
  }
  return out;
}

function computeValidationShortlist(
  result: unknown,
  minPsr: number | null,
  minDsr: number | null,
  minCscvRank: number | null,
  minCscvMetric: number | null,
): {
  rows: Array<Record<string, unknown>>;
  candidates: Array<Record<string, unknown>>;
  strictBase: Array<Record<string, unknown>>;
  hasCross: boolean;
  usedFallback: boolean;
  hasCscv: boolean;
  cscvCombos: number;
  cscvThreshold: number;
  cscvRankMin: number;
  usedCscvFallback: boolean;
  hasProbGate: boolean;
  probPassN: number;
  hasBackendVerdicts: boolean;
  backendPassN: number;
  runVerdict: Record<string, unknown> | null;
} {
  const CSCV_RANK_FRAC_MIN = 0.55;
  const candidates = validationCandidates(result);
  const hasProbGate = minPsr != null || minDsr != null;
  const cscvRankThreshold = minCscvRank ?? CSCV_RANK_FRAC_MIN;
  const runVerdict = asRecord(asRecord(result)?.run_verdict);
  const hasBackendVerdicts = candidates.some((candidate) => {
    const verdict = asRecord(candidate.validation_verdict);
    return !!verdict && typeof verdict.passed === "boolean";
  });
  const passProbabilityGates = (
    candidate: Record<string, unknown>,
  ): boolean => {
    if (!hasProbGate) return true;
    const psr = Number(candidate.psr);
    const dsr = Number(candidate.dsr);
    if (minPsr != null && (!Number.isFinite(psr) || psr < minPsr)) return false;
    if (minDsr != null && (!Number.isFinite(dsr) || dsr < minDsr)) return false;
    return true;
  };

  const hasCross = candidates.some((candidate) => {
    const cross = candidate.cross_ticker;
    return !!(cross && typeof cross === "object");
  });

  const strict = candidates.filter((candidate) => {
    if (!candidate.robust_pareto) return false;
    if (hasCross) return !!candidate.cross_ticker_pareto;
    return true;
  });

  const resultRecord = asRecord(result);
  const cscv = asRecord(resultRecord?.cscv_pbo);
  const cscvCombos = Math.max(0, Math.floor(Number(cscv?.combinations ?? 0)));
  const cscvThreshold = Number(cscv?.threshold ?? 0);
  const hasCscv =
    cscvCombos > 0 &&
    candidates.some((candidate) => {
      const stats = candidate.cscv_pbo;
      return !!(stats && typeof stats === "object");
    });

  const baseRows = strict.length > 0 ? strict : candidates;
  const usedFallback = strict.length === 0;

  const passCscv = (candidate: Record<string, unknown>): boolean => {
    const verdict = asRecord(candidate.validation_verdict);
    if (verdict && typeof verdict.passed === "boolean")
      return verdict.passed === true;
    const stats = asRecord(candidate.cscv_pbo);
    if (!stats) return false;
    const selectedFrac = Number(stats.is_selected_frac ?? 0);
    const rankFrac = Number(stats.oos_rank_frac_p50 ?? 0);
    const metric = Number(stats.oos_metric_p50 ?? Number.NaN);
    if (!Number.isFinite(selectedFrac) || selectedFrac <= 0) return false;
    if (!Number.isFinite(rankFrac) || rankFrac < cscvRankThreshold)
      return false;
    const requiredMetric =
      minCscvMetric != null ? minCscvMetric : cscvThreshold;
    if (!Number.isFinite(metric) || metric < requiredMetric) return false;
    return true;
  };

  const probFiltered = hasProbGate
    ? baseRows.filter(passProbabilityGates)
    : baseRows;
  const cscvFiltered =
    hasCscv || hasBackendVerdicts
      ? probFiltered.filter(passCscv)
      : probFiltered;
  const usedCscvFallback =
    !hasBackendVerdicts &&
    hasCscv &&
    baseRows.length > 0 &&
    cscvFiltered.length === 0;
  const pickedRows = hasProbGate
    ? cscvFiltered
    : usedCscvFallback
      ? baseRows
      : cscvFiltered;
  const backendPassN = hasBackendVerdicts
    ? candidates.filter(
        (candidate) => asRecord(candidate.validation_verdict)?.passed === true,
      ).length
    : 0;

  const sortByFinal = (
    left: Record<string, unknown>,
    right: Record<string, unknown>,
  ): number => {
    const robustL = Number(left.robust_score ?? 0);
    const robustR = Number(right.robust_score ?? 0);
    if (robustR !== robustL) return robustR - robustL;

    const cscvL = asRecord(left.cscv_pbo);
    const cscvR = asRecord(right.cscv_pbo);
    const rankL = Number(cscvL?.oos_rank_frac_p50 ?? 0);
    const rankR = Number(cscvR?.oos_rank_frac_p50 ?? 0);
    if (rankR !== rankL) return rankR - rankL;

    const selectedL = Number(cscvL?.is_selected_frac ?? 0);
    const selectedR = Number(cscvR?.is_selected_frac ?? 0);
    if (selectedR !== selectedL) return selectedR - selectedL;

    const metricL = Number(cscvL?.oos_metric_p50 ?? 0);
    const metricR = Number(cscvR?.oos_metric_p50 ?? 0);
    if (metricR !== metricL) return metricR - metricL;

    return 0;
  };

  return {
    rows: [...pickedRows].sort(sortByFinal),
    candidates,
    strictBase: strict,
    hasCross,
    usedFallback,
    hasCscv,
    cscvCombos,
    cscvThreshold,
    cscvRankMin: cscvRankThreshold,
    usedCscvFallback,
    hasProbGate,
    probPassN: probFiltered.length,
    hasBackendVerdicts,
    backendPassN,
    runVerdict,
  };
}

function buildValidationShortlistMetaText(
  shortlist: ReturnType<typeof computeValidationShortlist>,
  minPsr: number | null,
  minDsr: number | null,
): string {
  const robustParetoN = shortlist.candidates.filter(
    (candidate) => !!candidate.robust_pareto,
  ).length;
  const crossParetoN = shortlist.hasCross
    ? shortlist.candidates.filter(
        (candidate) => !!candidate.cross_ticker_pareto,
      ).length
    : 0;
  const baseRule = shortlist.hasCross
    ? "robust_pareto && cross_ticker_pareto"
    : "robust_pareto";
  const probRule = shortlist.hasProbGate
    ? `prob(${minPsr != null ? `psr>=${fmtFixed(minPsr, 2)}` : ""}${minPsr != null && minDsr != null ? "," : ""}${minDsr != null ? `dsr>=${fmtFixed(minDsr, 2)}` : ""})`
    : "";
  const cscvRule = shortlist.hasCscv
    ? `cscv(oos_rank_p50>=${shortlist.cscvRankMin}, oos_sharpe_p50>=${shortlist.cscvThreshold})`
    : "";
  const backendRule = shortlist.hasBackendVerdicts ? "backend_hard_gates" : "";
  const rule = [baseRule, probRule, cscvRule, backendRule]
    .filter((part) => part.length > 0)
    .join(" && ");
  const baseFallbackNote = shortlist.usedFallback
    ? " • (fallback: top robust score)"
    : "";
  const cscvFallbackNote =
    shortlist.usedCscvFallback && !shortlist.hasProbGate
      ? " • (cscv fallback)"
      : "";
  const crossNote = shortlist.hasCross
    ? ` • cross_pareto=${crossParetoN}/${shortlist.candidates.length}`
    : "";
  const strictCount =
    shortlist.strictBase.length > 0
      ? shortlist.strictBase.length
      : shortlist.candidates.length;
  const probNote = shortlist.hasProbGate
    ? ` • prob_pass=${shortlist.probPassN}/${strictCount}`
    : "";
  const cscvNote = shortlist.hasCscv
    ? ` • cscv=${shortlist.cscvCombos} combos`
    : "";
  const cscvPassNote = shortlist.hasCscv
    ? ` • cscv_pass=${shortlist.rows.length}/${strictCount}`
    : "";
  const backendNote = shortlist.hasBackendVerdicts
    ? ` • backend_pass=${shortlist.backendPassN}/${shortlist.candidates.length}`
    : "";
  const approvedCount = Math.max(
    0,
    Math.floor(Number(shortlist.runVerdict?.approved_candidate_count ?? 0)),
  );
  const firstApprovedRank = Math.max(
    0,
    Math.floor(Number(shortlist.runVerdict?.first_approved_rank ?? 0)),
  );
  const runVerdictRecommendation = normalizeText(
    shortlist.runVerdict?.final_recommendation,
  );
  const runVerdictNote = shortlist.runVerdict
    ? ` • run=${shortlist.runVerdict.passed === false ? "fail" : "pass"} • approved=${approvedCount}${firstApprovedRank > 0 ? `@#${firstApprovedRank}` : ""}${runVerdictRecommendation ? ` • rec=${runVerdictRecommendation}` : ""}`
    : "";
  return `shortlist=${shortlist.rows.length}/${shortlist.candidates.length} • robust_pareto=${robustParetoN}/${shortlist.candidates.length}${crossNote}${probNote}${cscvNote}${cscvPassNote}${backendNote}${runVerdictNote} • rule=${rule}${baseFallbackNote}${cscvFallbackNote}`;
}

function buildValidationTableColumns(): ValidationTableColumn[] {
  return [
    { key: "rank", label: "#", alignLeft: true },
    { key: "fast", label: "Fast", alignLeft: true },
    { key: "slow", label: "Slow", alignLeft: true },
    { key: "robust", label: "Robust", alignLeft: false },
    { key: "psr", label: "PSR", alignLeft: false },
    { key: "dsr", label: "DSR", alignLeft: false },
    { key: "cscv_rank", label: "CSCV Rank", alignLeft: false },
    { key: "gate", label: "Gate", alignLeft: true },
    { key: "robust_pareto", label: "Robust Pareto", alignLeft: true },
    { key: "cross", label: "Cross", alignLeft: false },
    { key: "cross_pareto", label: "Cross Pareto", alignLeft: true },
    { key: "pnl", label: "PnL", alignLeft: false },
    { key: "sharpe", label: "Sharpe", alignLeft: false },
    { key: "max_dd", label: "Max DD", alignLeft: false },
    { key: "trades", label: "Trades", alignLeft: false },
  ];
}

function buildValidationTableRows(
  shortlistRows: Array<Record<string, unknown>>,
  hasCross: boolean,
): ValidationTableRow[] {
  const rows: ValidationTableRow[] = [];
  for (let i = 0; i < shortlistRows.length; i += 1) {
    const candidate = shortlistRows[i] || {};
    const params = asRecord(candidate.params) ?? {};
    const inSample = asRecord(candidate.in_sample) ?? {};
    const cross = asRecord(candidate.cross_ticker);
    const cscv = asRecord(candidate.cscv_pbo);
    const verdict = asRecord(candidate.validation_verdict);
    const failReasons = Array.isArray(verdict?.fail_reasons)
      ? verdict.fail_reasons
          .map((entry) => normalizeText(entry))
          .filter((entry) => entry.length > 0)
      : [];
    const warningFlags = Array.isArray(verdict?.warning_flags)
      ? verdict.warning_flags
          .map((entry) => normalizeText(entry))
          .filter((entry) => entry.length > 0)
      : [];
    const gateText = verdict
      ? verdict.passed === false
        ? failReasons[0] || "fail"
        : warningFlags.length
          ? `pass*`
          : "pass"
      : "-";
    const gateTitle = verdict
      ? [...failReasons, ...warningFlags]
          .filter((entry) => entry.length > 0)
          .join(" • ")
      : "";

    rows.push({
      cells: [
        { text: String(i + 1), className: "left mono", title: "" },
        {
          text: `${normalizeText(params.fast_ma_type || "?")}(${Math.max(0, Math.floor(Number(params.fast_len || 0))) || "?"})`,
          className: "left mono",
          title: "",
        },
        {
          text: `${normalizeText(params.slow_ma_type || "?")}(${Math.max(0, Math.floor(Number(params.slow_len || 0))) || "?"})`,
          className: "left mono",
          title: "",
        },
        { text: fmt(candidate.robust_score, 6), className: "mono", title: "" },
        {
          text: candidate.psr != null ? fmt(candidate.psr, 4) : "-",
          className: "mono",
          title: "",
        },
        {
          text: candidate.dsr != null ? fmt(candidate.dsr, 4) : "-",
          className: "mono",
          title: "",
        },
        {
          text: cscv ? fmtPct(cscv.oos_rank_frac_p50, 1) : "-",
          className: "mono",
          title: "",
        },
        {
          text: gateText,
          className: "left mono",
          title: gateTitle,
        },
        {
          text: candidate.robust_pareto ? "robust" : "—",
          className: "left",
          title: "",
        },
        {
          text: cross ? fmt(cross.score, 6) : "-",
          className: "mono",
          title: "",
        },
        {
          text: hasCross
            ? candidate.cross_ticker_pareto
              ? "cross"
              : "—"
            : "—",
          className: "left",
          title: "",
        },
        { text: fmt(inSample.pnl, 6), className: "mono", title: "" },
        { text: fmt(inSample.sharpe, 6), className: "mono", title: "" },
        { text: fmt(inSample.max_dd, 6), className: "mono", title: "" },
        {
          text: String(Math.max(0, Math.floor(Number(inSample.trades || 0)))),
          className: "mono",
          title: "",
        },
      ],
    });
  }
  return rows;
}

async function persistValidationArtifact(
  runReq: Record<string, unknown>,
  runRes: unknown,
  validationReq: Record<string, unknown>,
  validationRes: unknown,
  options: {
    reportXlsxPath?: string;
    datasetMeta?: Record<string, unknown> | null;
  } = {},
): Promise<PersistValidationArtifactResult> {
  const currentRun = readWorkflowRunArtifactFromStore();
  const dataStore = readPiniaStoreState("data");
  const ts = normalizeText(currentRun?.ts) || new Date().toISOString();
  const csvPath = normalizeText(currentRun?.csvPath || dataStore?.csvPath);
  const reportXlsxPath = normalizeText(
    options.reportXlsxPath || currentRun?.reportXlsxPath,
  );
  const tags = currentRun?.tags ?? [];
  const datasetMeta =
    options.datasetMeta ??
    currentRun?.datasetMeta ??
    buildDatasetMetaForArtifact(csvPath, buildRunDatasetContextState());
  const mlSignal = currentRun?.mlSignal ?? null;
  const optimizationMetricSurface =
    currentRun?.optimizationMetricSurface ?? null;
  const configFingerprint = computeConfigFingerprint(runReq, validationReq);
  const validationStageCompletion = buildValidationStageCompletion({
    ts,
    completedAt: new Date().toISOString(),
    artifactKind: "vectorgrid_run_artifact",
    artifactPath: currentRun?.artifactPath,
    reportXlsxPath,
    csvPath,
    configFingerprint,
    datasetMeta,
    routeProvenance: currentRun?.routeProvenance,
    req: runReq,
    res: runRes,
    validationReq,
    validationRes,
  });
  let stageCompletions = upsertStageCompletion(
    currentRun?.stageCompletions ?? [],
    validationStageCompletion,
  );

  return measureValidationReportPerf(
    "validationService.persistValidationArtifactMs",
    async () => {
      const artifact: Record<string, unknown> = {
        version: 1,
        kind: "vectorgrid_run_artifact",
        ts,
        config_fingerprint: configFingerprint,
        run: { req: runReq, res: runRes },
        validation: { req: validationReq, res: validationRes },
        report_xlsx_path: reportXlsxPath,
        data: { csv_path: csvPath },
        dataset_meta: datasetMeta,
        ml_signal: mlSignal,
        optimization_metric_surface: optimizationMetricSurface,
        stage_completions:
          serializeStageCompletionsForArtifact(stageCompletions),
        tags,
      };
      let path = normalizeText(currentRun?.artifactPath);
      let persistenceWarning = "";
      try {
        const saveArgs: Record<string, unknown> = { artifact };
        if (normalizeText(currentRun?.artifactPath)) {
          saveArgs.existing_path = normalizeText(currentRun?.artifactPath);
        }
        const saved = (await invokeCommand(
          "save_run_artifact",
          saveArgs,
        )) as SaveRunArtifactResponse;
        const next = normalizeText(saved?.path);
        if (next) path = next;
      } catch (error) {
        const detail = normalizeText(error);
        persistenceWarning = detail
          ? `Validation artifact persistence failed: ${detail}`
          : "Validation artifact persistence failed.";
      }
      if (path) {
        stageCompletions = stageCompletions.map((packet) =>
          packet.artifactPath ? packet : { ...packet, artifactPath: path },
        );
      }

      return {
        path,
        ts,
        csvPath,
        reportXlsxPath,
        datasetMeta,
        mlSignal,
        optimizationMetricSurface,
        stageCompletions,
        tags,
        persistenceWarning,
      };
    },
    {
      meta: {
        hasExistingArtifactPath: !!normalizeText(currentRun?.artifactPath),
        hasReportXlsxPath: !!reportXlsxPath,
      },
      onSuccessMeta: (result) => ({
        persistedPathReturned: !!normalizeText(result.path),
        persistenceWarning: !!normalizeText(result.persistenceWarning),
      }),
    },
  );
}

function applyVueValidationArtifacts(
  runReq: Record<string, unknown>,
  runRes: unknown,
  validationReq: Record<string, unknown>,
  validationRes: unknown,
  persisted: PersistValidationArtifactResult,
): void {
  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ts: persisted.ts,
        artifact_path: persisted.path,
        report_xlsx_path: persisted.reportXlsxPath,
        csv_path: persisted.csvPath,
        dataset_meta: persisted.datasetMeta,
        ml_signal: persisted.mlSignal,
        optimization_metric_surface: persisted.optimizationMetricSurface,
        stage_completions: serializeStageCompletionsForArtifact(
          persisted.stageCompletions,
        ),
        tags: persisted.tags,
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

function resolveValidationBuildContextFromWorkflowArtifacts(): LegacyValidationBuildContext | null {
  const runSnapshot = readRunSnapshotFromStore();
  const runArtifact = readWorkflowRunArtifactFromStore();
  const runReq = runSnapshot?.req ?? runArtifact?.req ?? null;
  const runRes = runSnapshot?.res ?? runArtifact?.res ?? null;
  const snapshotTopRows = extractTopRowsFromRunResult(runRes);
  if (!runReq || !runRes || snapshotTopRows.length === 0) return null;
  return {
    currentRunReq: runReq,
    currentRunRes: runRes,
    topRows: snapshotTopRows,
    topRowsFiltered: [],
  };
}

export async function runValidation(
  options: ValidationRunOptions = {},
): Promise<void> {
  const validationStartedAt = nowMs();
  const buildContext = resolveValidationBuildContextFromWorkflowArtifacts();
  if (!buildContext) {
    const message = "Run an optimization first.";
    patchValidationStoreState({
      statusLine: message,
      validationHint: message,
      validationAccelerationStatus: "Validation not run.",
      validationRouteSummary: "-",
      validationRouteDetailSummary: "-",
      validationCategorySummary: "-",
      validationAuxCpuSummary: "-",
      validationCrossSummary: "-",
      validationSkippedSummary: "-",
      validationEvidenceSummary: "Validation evidence not run.",
      validationWarningSummary: "-",
    });
    writeDomTextContent("status", message, VALIDATION_PANEL_SELECTOR);
    throw new Error(message);
  }
  const runSnapshot = readRunSnapshotFromStore();
  const workflowRun = readWorkflowRunArtifactFromStore();
  const runReq = asRecord(runSnapshot?.req ?? buildContext.currentRunReq);
  const runRes = runSnapshot?.res ?? buildContext.currentRunRes;
  if (!runReq || !runRes) {
    const message = "Run artifact is unavailable. Re-run optimization first.";
    patchValidationStoreState({
      statusLine: message,
      validationHint: message,
      validationAccelerationStatus: "Validation not run.",
      validationRouteSummary: "-",
      validationRouteDetailSummary: "-",
      validationCategorySummary: "-",
      validationAuxCpuSummary: "-",
      validationCrossSummary: "-",
      validationSkippedSummary: "-",
      validationEvidenceSummary: "Validation evidence not run.",
      validationWarningSummary: "-",
    });
    writeDomTextContent("status", message, VALIDATION_PANEL_SELECTOR);
    throw new Error(message);
  }
  if (!workflowRun) {
    const message = "Run artifact is unavailable. Re-run optimization first.";
    patchValidationStoreState({
      statusLine: message,
      validationHint: message,
      validationAccelerationStatus: "Validation not run.",
      validationRouteSummary: "-",
      validationRouteDetailSummary: "-",
      validationCategorySummary: "-",
      validationAuxCpuSummary: "-",
      validationCrossSummary: "-",
      validationSkippedSummary: "-",
      validationEvidenceSummary: "Validation evidence not run.",
      validationWarningSummary: "-",
    });
    writeDomTextContent("status", message, VALIDATION_PANEL_SELECTOR);
    throw new Error(message);
  }
  assertValidationRunMatchesCurrentDataset(workflowRun, runReq);

  const buildInputs: Parameters<typeof buildValidationRequest>[0] = {
    currentRunReq: runReq,
    currentRunRes: runRes,
    runDatasetMeta: workflowRun.datasetMeta,
    runCsvPath: workflowRun.csvPath,
    topRows: buildContext.topRows,
    topRowsFiltered: buildContext.topRowsFiltered,
    candidates: readStoreTextInputValue("valCandidates"),
    admissionPolicy: readStoreTextInputValue("valAdmissionPolicy"),
    seed: readStoreTextInputValue("valSeed"),
    minPsr: readStoreTextInputValue("valMinPsr"),
    minDsr: readStoreTextInputValue("valMinDsr"),
    minCscvOosRankFrac: readStoreTextInputValue("valMinCscvRank"),
    minCscvOosMetric: readStoreTextInputValue("valMinCscvMetric"),
    maxPbo: readStoreTextInputValue("valMaxPbo"),
    spaEnable: readStoreCheckboxValue("valSpaEnable"),
    spaSlices: readStoreTextInputValue("valSpaSlices"),
    spaBootstrapSamples: readStoreTextInputValue("valSpaBootstrapSamples"),
    spaBlockLength: readStoreTextInputValue("valSpaBlockLength"),
    reoptWfEnable: readStoreCheckboxValue("valReoptWfEnable"),
    reoptWfWindow: readStoreTextInputValue("valReoptWfWindow"),
    reoptWfStep: readStoreTextInputValue("valReoptWfStep"),
    reoptWfTopN: readStoreTextInputValue("valReoptWfTopN"),
    wfEnable: readStoreCheckboxValue("valWfEnable"),
    wfWindow: readStoreTextInputValue("valWfWindow"),
    wfStep: readStoreTextInputValue("valWfStep"),
    perturbEnable: readStoreCheckboxValue("valPerturbEnable"),
    perturbIters: readStoreTextInputValue("valPerturbIters"),
    perturbMin: readStoreTextInputValue("valPerturbMin"),
    perturbMax: readStoreTextInputValue("valPerturbMax"),
    mcEnable: readStoreCheckboxValue("valMcEnable"),
    mcSims: readStoreTextInputValue("valMcSims"),
    mcBlockLen: readStoreTextInputValue("valMcBlockLen"),
    mcMode: readStoreTextInputValue("valMcMode"),
    cscvEnable: readStoreCheckboxValue("valCscvEnable"),
    cscvSlices: readStoreTextInputValue("valCscvSlices"),
    crossEnable: readStoreCheckboxValue("valCrossEnable"),
    crossMax: readStoreTextInputValue("valCrossMax"),
    crossMode: readStoreTextInputValue("valCrossMode"),
    crossDatasets: readVueCrossSelection(),
  };
  if (typeof options.useFiltered === "boolean")
    buildInputs.useFiltered = options.useFiltered;

  const validationReq = buildValidationRequest(
    buildInputs,
  ) as unknown as Record<string, unknown>;
  const command = resolveValidationCommand(runReq);
  const switchTab = options.switchTab !== false;

  patchValidationStoreState({
    validateDisabled: true,
    statusLine: "Running validation…",
    validationHint: "Running validation…",
    validationAccelerationStatus: "Validation running…",
    validationRouteSummary: "-",
    validationRouteDetailSummary: "-",
    validationCategorySummary: "-",
    validationAuxCpuSummary: "-",
    validationBootstrapSummary: "-",
    validationMonteCarloSummary: "not run",
    validationMonteCarloHistogram: [],
    validationRegimeSummary: "-",
    validationFrictionSummary: "-",
    validationCrossSummary: "-",
    validationSkippedSummary: "-",
    validationEvidenceSummary: "Validation evidence not run.",
    validationWarningSummary: "-",
    validationReoptMeta: "-",
  });
  writeDomTextContent(
    "status",
    "Running validation…",
    VALIDATION_PANEL_SELECTOR,
  );

  try {
    const commandStartedAt = nowMs();
    const validationRes = await measureValidationReportPerf(
      "validationService.validationCommandMs",
      async () => invokeCommand(command, { req: validationReq }),
      {
        meta: {
          command,
          autoExport: readValidationExportPrefs().autoExport,
        },
        onSuccessMeta: (result) => {
          const row = asRecord(result);
          const candidates = Array.isArray(row?.candidates)
            ? row.candidates.length
            : 0;
          return {
            reportedRuntimeMs: Number(row?.runtime_ms ?? 0),
            candidateCount: candidates,
          };
        },
      },
    );
    const commandMs = Math.max(0, nowMs() - commandStartedAt);
    const reportedRuntimeMs = Number(asRecord(validationRes)?.runtime_ms ?? 0);
    const validationMeta = buildValidationMetaText(
      validationRes,
      validationReq,
    );
    const validationReoptMeta = buildValidationReoptMetaText(validationRes);
    const executionSummary = buildValidationExecutionSummary(
      validationRes,
      validationReq,
    );
    const minPsrGate = parseProbabilityGateValue(validationReq.min_psr);
    const minDsrGate = parseProbabilityGateValue(validationReq.min_dsr);
    const minCscvRankGate = parseProbabilityGateValue(
      validationReq.min_cscv_oos_rank_frac,
    );
    const minCscvMetricGate = parseFiniteNumberValue(
      validationReq.min_cscv_oos_metric,
    );
    const shortlist = computeValidationShortlist(
      validationRes,
      minPsrGate,
      minDsrGate,
      minCscvRankGate,
      minCscvMetricGate,
    );
    const validationColumns = buildValidationTableColumns();
    const validationRows = buildValidationTableRows(
      shortlist.rows,
      shortlist.hasCross,
    );
    const validationShortlistMeta =
      validationRows.length > 0
        ? buildValidationShortlistMetaText(shortlist, minPsrGate, minDsrGate)
        : "No validation result yet.";
    const { autoExport } = readValidationExportPrefs();
    const datasetMeta =
      workflowRun.datasetMeta ??
      buildDatasetMetaForArtifact(
        workflowRun.csvPath,
        buildRunDatasetContextState(),
      );
    let autoExportStatus = "";
    let reportXlsxPath = "";
    if (autoExport) {
      const report = buildReportPayload({
        ts: workflowRun.ts,
        datasetMeta,
        optimizationMetricSurface: workflowRun.optimizationMetricSurface,
        runReq,
        runRes,
        validationReq,
        validationRes,
      });
      try {
        const exportPrefs = readValidationExportPrefs();
        const exportResult = await measureValidationReportPerf(
          "validationService.autoExportReportXlsxMs",
          async () =>
            exportReportXlsxInternal(report, {
              autoReveal: exportPrefs.autoReveal,
              maxRows: exportPrefs.maxRows,
              trigger: "auto",
            }),
          {
            meta: {
              autoReveal: exportPrefs.autoReveal,
              maxRows: exportPrefs.maxRows,
            },
            onSuccessMeta: (result) => ({
              returnedPath: !!normalizeText(result.reportXlsxPath),
            }),
          },
        );
        autoExportStatus = [
          exportResult.statusLine,
          exportResult.artifactUpdateWarning,
        ]
          .map((part) => normalizeText(part))
          .filter((part) => part.length > 0)
          .join(" • ");
        reportXlsxPath = exportResult.reportXlsxPath;
      } catch (error) {
        const reason = String(error || "").trim() || "unknown error";
        autoExportStatus = `Auto-export XLSX failed: ${reason}`;
      }
    }
    const evidenceSummary = buildValidationEvidenceSummary({
      validationReq,
      validationRes,
      datasetMeta,
      reportPath: reportXlsxPath,
      replayClass: "exact_evidence_review",
    });
    const persisted = await persistValidationArtifact(
      runReq,
      runRes,
      validationReq,
      validationRes,
      {
        reportXlsxPath,
        datasetMeta,
      },
    );

    applyVueValidationArtifacts(
      runReq,
      runRes,
      validationReq,
      validationRes,
      persisted,
    );
    const endToEndMs = Math.max(0, nowMs() - validationStartedAt);
    recordValidationReportPerf(
      "validationService.runValidationEndToEndMs",
      endToEndMs,
      {
        command,
        autoExport,
        commandMs,
        reportedRuntimeMs: Number.isFinite(reportedRuntimeMs)
          ? reportedRuntimeMs
          : null,
        returnedReportXlsxPath: !!reportXlsxPath,
        persistenceWarning: !!normalizeText(persisted.persistenceWarning),
      },
    );
    const statusLine = formatValidationStatus(
      commandMs,
      endToEndMs,
      [autoExportStatus, persisted.persistenceWarning]
        .map((part) => normalizeText(part))
        .filter((part) => part.length > 0)
        .join(" • "),
    );
    patchValidationStoreState({
      validationHint: buildValidationHintText(validationRes),
      validationAccelerationStatus: executionSummary.accelerationStatus,
      validationMeta,
      validationRouteSummary: executionSummary.routeSummary,
      validationRouteDetailSummary: executionSummary.routeDetailSummary,
      validationCategorySummary: executionSummary.categorySummary,
      validationAuxCpuSummary: executionSummary.auxCpuSummary,
      validationBootstrapSummary: executionSummary.bootstrapSummary,
      validationMonteCarloSummary: executionSummary.monteCarloSummary,
      validationMonteCarloHistogram: executionSummary.monteCarloHistogram,
      validationRegimeSummary: executionSummary.regimeSummary,
      validationFrictionSummary: executionSummary.frictionSummary,
      validationCrossSummary: executionSummary.crossSummary,
      validationSkippedSummary: executionSummary.skippedSummary,
      validationEvidenceSummary: evidenceSummary.summaryLine,
      validationWarningSummary: executionSummary.warningSummary,
      validationReoptMeta,
      validationColumns,
      validationRows,
      validationOutputPreview: toOutputPreview(validationRes),
      tableRowCount: validationRows.length,
      tableVisible: validationRows.length > 0,
      canExportValidationCsv: true,
      canExportValidationJson: true,
      canExportReportJson: true,
      canExportReportXlsx: true,
      validationShortlistMeta,
      statusLine,
    });
    writeDomTextContent(
      "validationMeta",
      validationMeta,
      VALIDATION_PANEL_SELECTOR,
    );
    writeDomTextContent("status", statusLine, VALIDATION_PANEL_SELECTOR);

    if (switchTab) selectWorkflowResultsTab("validate", true);
  } catch (error) {
    const message = String(error || "").trim() || "Validation failed.";
    patchValidationStoreState({
      statusLine: message,
      validationHint: message,
      validationAccelerationStatus: "Validation failed.",
      validationRouteSummary: "-",
      validationRouteDetailSummary: "-",
      validationCategorySummary: "-",
      validationAuxCpuSummary: "-",
      validationBootstrapSummary: "-",
      validationMonteCarloSummary: "not run",
      validationMonteCarloHistogram: [],
      validationRegimeSummary: "-",
      validationFrictionSummary: "-",
      validationCrossSummary: "-",
      validationSkippedSummary: "-",
      validationEvidenceSummary: "Validation evidence not run.",
      validationWarningSummary: "-",
      validationReoptMeta: "-",
    });
    writeDomTextContent("status", message, VALIDATION_PANEL_SELECTOR);
    throw error;
  } finally {
    patchValidationStoreState({ validateDisabled: false });
  }
}

export function previewValidationExecutionSummaryFromStores(): {
  routeSummary: string;
  crossSummary: string;
  moduleSummary: string;
  categorySummary: string;
  skippedSummary: string;
  warningSummary: string;
} {
  const buildContext = resolveValidationBuildContextFromWorkflowArtifacts();
  if (!buildContext) {
    return {
      routeSummary: "Run an optimization first.",
      crossSummary: "-",
      moduleSummary: "-",
      categorySummary: "-",
      skippedSummary: "-",
      warningSummary:
        "No validation preview is available without an optimization artifact.",
    };
  }

  const runSnapshot = readRunSnapshotFromStore();
  const workflowRun = readWorkflowRunArtifactFromStore();
  const runReq = asRecord(runSnapshot?.req ?? buildContext.currentRunReq);
  const runRes = runSnapshot?.res ?? buildContext.currentRunRes;
  if (!runReq || !runRes || !workflowRun) {
    return {
      routeSummary: "Run artifact unavailable.",
      crossSummary: "-",
      moduleSummary: "-",
      categorySummary: "-",
      skippedSummary: "-",
      warningSummary:
        "Re-run optimization before previewing validation execution.",
    };
  }

  try {
    assertValidationRunMatchesCurrentDataset(workflowRun, runReq);
    const buildInputs: Parameters<typeof buildValidationRequest>[0] = {
      currentRunReq: runReq,
      currentRunRes: runRes,
      runDatasetMeta: workflowRun.datasetMeta,
      runCsvPath: workflowRun.csvPath,
      topRows: buildContext.topRows,
      topRowsFiltered: buildContext.topRowsFiltered,
      candidates: readStoreTextInputValue("valCandidates"),
      admissionPolicy: readStoreTextInputValue("valAdmissionPolicy"),
      seed: readStoreTextInputValue("valSeed"),
      minPsr: readStoreTextInputValue("valMinPsr"),
      minDsr: readStoreTextInputValue("valMinDsr"),
      minCscvOosRankFrac: readStoreTextInputValue("valMinCscvRank"),
      minCscvOosMetric: readStoreTextInputValue("valMinCscvMetric"),
      maxPbo: readStoreTextInputValue("valMaxPbo"),
      spaEnable: readStoreCheckboxValue("valSpaEnable"),
      spaSlices: readStoreTextInputValue("valSpaSlices"),
      spaBootstrapSamples: readStoreTextInputValue("valSpaBootstrapSamples"),
      spaBlockLength: readStoreTextInputValue("valSpaBlockLength"),
      reoptWfEnable: readStoreCheckboxValue("valReoptWfEnable"),
      reoptWfWindow: readStoreTextInputValue("valReoptWfWindow"),
      reoptWfStep: readStoreTextInputValue("valReoptWfStep"),
      reoptWfTopN: readStoreTextInputValue("valReoptWfTopN"),
      wfEnable: readStoreCheckboxValue("valWfEnable"),
      wfWindow: readStoreTextInputValue("valWfWindow"),
      wfStep: readStoreTextInputValue("valWfStep"),
      perturbEnable: readStoreCheckboxValue("valPerturbEnable"),
      perturbIters: readStoreTextInputValue("valPerturbIters"),
      perturbMin: readStoreTextInputValue("valPerturbMin"),
      perturbMax: readStoreTextInputValue("valPerturbMax"),
      mcEnable: readStoreCheckboxValue("valMcEnable"),
      mcSims: readStoreTextInputValue("valMcSims"),
      mcBlockLen: readStoreTextInputValue("valMcBlockLen"),
      mcMode: readStoreTextInputValue("valMcMode"),
      cscvEnable: readStoreCheckboxValue("valCscvEnable"),
      cscvSlices: readStoreTextInputValue("valCscvSlices"),
      crossEnable: readStoreCheckboxValue("valCrossEnable"),
      crossMax: readStoreTextInputValue("valCrossMax"),
      crossMode: readStoreTextInputValue("valCrossMode"),
      crossDatasets: readVueCrossSelection(),
    };
    const validationReq = buildValidationRequest(buildInputs);
    return buildValidationCapabilitySummary(validationReq);
  } catch (error) {
    const message = normalizeText(error);
    return {
      routeSummary: "Validation preview unavailable.",
      crossSummary: "-",
      moduleSummary: "-",
      categorySummary: "-",
      skippedSummary: "-",
      warningSummary:
        message ||
        "Unable to build validation preview from the current workflow state.",
    };
  }
}

export function previewValidationAdmissionComparisonFromStores(): {
  summary: string;
  detail: string;
} {
  const buildContext = resolveValidationBuildContextFromWorkflowArtifacts();
  if (!buildContext) {
    return {
      summary: "Run an optimization first.",
      detail:
        "No shortlist comparison is available without an optimization artifact.",
    };
  }

  const runSnapshot = readRunSnapshotFromStore();
  const workflowRun = readWorkflowRunArtifactFromStore();
  const runReq = asRecord(runSnapshot?.req ?? buildContext.currentRunReq);
  const runRes = runSnapshot?.res ?? buildContext.currentRunRes;
  if (!runReq || !runRes || !workflowRun) {
    return {
      summary: "Run artifact unavailable.",
      detail:
        "Re-run optimization before previewing shortlist admission changes.",
    };
  }

  try {
    const buildPreviewRequest = (admissionPolicyOverride?: string) =>
      buildValidationRequest({
        currentRunReq: runReq,
        currentRunRes: runRes,
        runDatasetMeta: workflowRun.datasetMeta,
        runCsvPath: workflowRun.csvPath,
        topRows: buildContext.topRows,
        topRowsFiltered: buildContext.topRowsFiltered,
        candidates: readStoreTextInputValue("valCandidates"),
        admissionPolicy:
          admissionPolicyOverride ??
          readStoreTextInputValue("valAdmissionPolicy"),
        seed: readStoreTextInputValue("valSeed"),
        minPsr: readStoreTextInputValue("valMinPsr"),
        minDsr: readStoreTextInputValue("valMinDsr"),
        minCscvOosRankFrac: readStoreTextInputValue("valMinCscvRank"),
        minCscvOosMetric: readStoreTextInputValue("valMinCscvMetric"),
        maxPbo: readStoreTextInputValue("valMaxPbo"),
        spaEnable: readStoreCheckboxValue("valSpaEnable"),
        spaSlices: readStoreTextInputValue("valSpaSlices"),
        spaBootstrapSamples: readStoreTextInputValue("valSpaBootstrapSamples"),
        spaBlockLength: readStoreTextInputValue("valSpaBlockLength"),
        reoptWfEnable: readStoreCheckboxValue("valReoptWfEnable"),
        reoptWfWindow: readStoreTextInputValue("valReoptWfWindow"),
        reoptWfStep: readStoreTextInputValue("valReoptWfStep"),
        reoptWfTopN: readStoreTextInputValue("valReoptWfTopN"),
        wfEnable: readStoreCheckboxValue("valWfEnable"),
        wfWindow: readStoreTextInputValue("valWfWindow"),
        wfStep: readStoreTextInputValue("valWfStep"),
        perturbEnable: readStoreCheckboxValue("valPerturbEnable"),
        perturbIters: readStoreTextInputValue("valPerturbIters"),
        perturbMin: readStoreTextInputValue("valPerturbMin"),
        perturbMax: readStoreTextInputValue("valPerturbMax"),
        mcEnable: readStoreCheckboxValue("valMcEnable"),
        mcSims: readStoreTextInputValue("valMcSims"),
        mcBlockLen: readStoreTextInputValue("valMcBlockLen"),
        mcMode: readStoreTextInputValue("valMcMode"),
        cscvEnable: readStoreCheckboxValue("valCscvEnable"),
        cscvSlices: readStoreTextInputValue("valCscvSlices"),
        crossEnable: readStoreCheckboxValue("valCrossEnable"),
        crossMax: readStoreTextInputValue("valCrossMax"),
        crossMode: readStoreTextInputValue("valCrossMode"),
        crossDatasets: readVueCrossSelection(),
      });

    const currentReq = buildPreviewRequest();
    const currentPolicy =
      normalizeText(currentReq.admission_policy) || "legacy_topk";
    const currentProvenance = asRecord(currentReq.admission_provenance);
    const requestedPolicy =
      normalizeText(currentProvenance?.requested_policy) || currentPolicy;
    const currentNote = normalizeText(currentProvenance?.note);
    const currentHeads = Array.isArray(currentProvenance?.heads)
      ? (currentProvenance?.heads as unknown[])
          .map((entry) => asRecord(entry))
          .map((entry) => normalizeText(entry?.objective ?? entry?.head))
          .filter(
            (value, index, values) =>
              value.length > 0 && values.indexOf(value) === index,
          )
          .slice(0, 4)
      : [];
    if (currentPolicy === "legacy_topk") {
      if (requestedPolicy !== currentPolicy) {
        const detailParts = [];
        if (currentNote) detailParts.push(currentNote);
        if (currentHeads.length > 0) {
          detailParts.push(`heads=${currentHeads.join("/")}`);
        }
        return {
          summary: `${requestedPolicy} -> legacy_topk fallback`,
          detail:
            detailParts.join(" • ") ||
            "The requested admission policy is unavailable for this artifact, so preview falls back to the legacy shortlist.",
        };
      }
      return {
        summary: "Legacy Top K active.",
        detail:
          "Switch the admission policy to compare a diversified shortlist against the legacy ordering.",
      };
    }

    const legacyReq = buildPreviewRequest("legacy_topk");
    const currentCandidates = Array.isArray(currentReq.candidates)
      ? currentReq.candidates
      : [];
    const legacyCandidates = Array.isArray(legacyReq.candidates)
      ? legacyReq.candidates
      : [];
    const currentKeys = currentCandidates.map((candidate) =>
      canonicalPreviewKey(candidate),
    );
    const legacyKeys = legacyCandidates.map((candidate) =>
      canonicalPreviewKey(candidate),
    );
    const legacySet = new Set(legacyKeys);
    const currentOnlyCount = currentKeys.filter(
      (key) => !legacySet.has(key),
    ).length;
    const currentSet = new Set(currentKeys);
    const legacyOnlyCount = legacyKeys.filter(
      (key) => !currentSet.has(key),
    ).length;
    const overlap = currentKeys.filter((key) => legacySet.has(key)).length;

    const summary =
      currentOnlyCount === 0 && legacyOnlyCount === 0
        ? `${currentPolicy} matches legacy • ${currentCandidates.length} candidates`
        : `${currentPolicy} vs legacy • overlap=${overlap} • current-only=${currentOnlyCount} • legacy-only=${legacyOnlyCount}`;
    const detailParts = [
      `current=${currentCandidates.length}`,
      `legacy=${legacyCandidates.length}`,
    ];
    if (currentHeads.length > 0) {
      detailParts.push(`heads=${currentHeads.join("/")}`);
    }
    return {
      summary,
      detail: detailParts.join(" • "),
    };
  } catch (error) {
    return {
      summary: "Validation preview unavailable.",
      detail:
        normalizeText(error) ||
        "Unable to compare shortlist admission policies.",
    };
  }
}

export async function exportShortlistCsv(): Promise<void> {
  const run = readWorkflowRunArtifactFromStore();
  const validation = readWorkflowValidationArtifactFromStore();
  const validationRes = validation?.res ?? null;
  if (!validationRes) return;
  const ts =
    normalizeText(run?.ts || validation?.ts) || new Date().toISOString();
  const csv = await invokeCommand("build_validation_shortlist_csv", {
    validation_res: validationRes,
  });
  const downloaded = downloadText(
    `vectorgrid_validation_shortlist_${ts}.csv`,
    String(csv || ""),
    "text/csv;charset=utf-8",
  );
  patchValidationStoreState({
    statusLine: downloaded
      ? "Exported validation shortlist CSV."
      : "Built validation shortlist CSV (download unavailable in this environment).",
  });
}

export async function exportValidationJson(): Promise<void> {
  const run = readWorkflowRunArtifactFromStore();
  const validation = readWorkflowValidationArtifactFromStore();
  const validationRes = validation?.res ?? null;
  if (!validationRes) return;
  const ts =
    normalizeText(run?.ts || validation?.ts) || new Date().toISOString();
  const downloaded = downloadText(
    `vectorgrid_validation_${ts}.json`,
    safePrettyJson(validationRes),
    "application/json;charset=utf-8",
  );
  patchValidationStoreState({
    statusLine: downloaded
      ? "Exported validation JSON."
      : "Built validation JSON (download unavailable in this environment).",
  });
}

export async function exportReportJson(): Promise<void> {
  const report = buildCurrentReportPayloadFromStore();
  const hasRun = !!(report.run && report.run.res);
  if (!hasRun) return;
  const ts = normalizeText(report.ts) || new Date().toISOString();
  const downloaded = downloadText(
    `vectorgrid_report_${ts}.json`,
    safePrettyJson(report),
    "application/json;charset=utf-8",
  );
  patchValidationStoreState({
    statusLine: downloaded
      ? "Exported report JSON."
      : "Built report JSON (download unavailable in this environment).",
  });
}

async function exportReportXlsxInternal(
  report: ReportPayload,
  prefs: ReportXlsxExportPrefs,
): Promise<ReportXlsxExportResult> {
  const ts = normalizeText(report.ts) || new Date().toISOString();
  const validationReq = asRecord(report.validation?.req ?? null);
  const validationRes = report.validation?.res ?? null;
  const currentRun = readWorkflowRunArtifactFromStore();
  const currentValidation = readWorkflowValidationArtifactFromStore();
  let exportBranch = "direct";
  let usedFallbackBuild = false;

  return measureValidationReportPerf(
    "validationService.exportReportXlsxTotalMs",
    async () => {
      if (!validationReq || validationRes == null) {
        exportBranch = "stage3_meta";
        const stage3Path = readLatestMetaReportPath();
        if (!stage3Path) {
          return {
            statusLine:
              "Run validation first to export report XLSX (or run Meta Analysis to open Stage 3 XLSX).",
            reportXlsxPath: "",
            artifactUpdated: false,
            artifactUpdateWarning: "",
          };
        }
        const opened = await measureValidationReportPerf(
          "validationService.exportReportXlsxOpenMs",
          async () => openPathWithRevealFallback(stage3Path),
          {
            meta: {
              branch: exportBranch,
              trigger: prefs.trigger,
            },
            onSuccessMeta: (result) => ({
              openedWith: normalizeText(result.openedWith),
            }),
          },
        );
        if (prefs.autoReveal && opened.openedWith !== "reveal_path") {
          await measureValidationReportPerf(
            "validationService.exportReportXlsxRevealMs",
            async () => invokeCommand("reveal_path", { path: stage3Path }),
            {
              meta: {
                branch: exportBranch,
                trigger: prefs.trigger,
              },
            },
          ).catch(() => undefined);
        }
        return {
          statusLine: `Opened Stage 3 XLSX report: ${stage3Path}`,
          reportXlsxPath: "",
          artifactUpdated: false,
          artifactUpdateWarning: "",
        };
      }

      let path = "";
      let artifactUpdated = false;
      let artifactUpdateWarning = "";
      try {
        const artifactPath = normalizePath(currentRun?.artifactPath ?? "");
        const canUseArtifactExport =
          !!artifactPath &&
          currentValidation?.req === validationReq &&
          currentValidation?.res === validationRes;
        exportBranch = canUseArtifactExport ? "artifact" : "direct";
        const saved = (await measureValidationReportPerf(
          "validationService.exportReportXlsxSaveMs",
          async () =>
            invokeCommand(
              canUseArtifactExport
                ? "save_report_xlsx_from_artifact"
                : "save_report_xlsx",
              canUseArtifactExport
                ? {
                    path: artifactPath,
                    max_rows: prefs.maxRows,
                  }
                : {
                    report,
                    max_rows: prefs.maxRows,
                  },
            ) as Promise<SaveReportXlsxResponse>,
          {
            meta: {
              branch: exportBranch,
              trigger: prefs.trigger,
              maxRows: prefs.maxRows,
            },
            onSuccessMeta: (result) => ({
              returnedPath: !!normalizeText(result?.path),
            }),
          },
        )) as SaveReportXlsxResponse;
        path = normalizeText(saved?.path);
        artifactUpdated = normalizeBoolean(saved?.artifact_updated);
        artifactUpdateWarning = normalizeText(saved?.artifact_update_warning);
      } catch (saveError) {
        usedFallbackBuild = true;
        exportBranch = "fallback_build";
        try {
          const base64 = await measureValidationReportPerf(
            "validationService.exportReportXlsxFallbackBuildMs",
            async () =>
              invokeCommand("build_report_xlsx", {
                report,
                max_rows: prefs.maxRows,
              }),
            {
              meta: {
                branch: exportBranch,
                trigger: prefs.trigger,
                maxRows: prefs.maxRows,
              },
            },
          );
          const downloaded = downloadBase64(
            `vectorgrid_report_${ts}.xlsx`,
            String(base64 || ""),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
          );
          return {
            statusLine: downloaded
              ? "Exported XLSX report as a file download."
              : "Built XLSX report (download unavailable in this environment).",
            reportXlsxPath: "",
            artifactUpdated: false,
            artifactUpdateWarning: "",
          };
        } catch (fallbackError) {
          throw new Error(
            `Failed to export XLSX report: ${String(saveError)} | fallback failed: ${String(fallbackError)}`,
          );
        }
      }

      if (!path) {
        return {
          statusLine: "Saved XLSX report, but no path was returned.",
          reportXlsxPath: "",
          artifactUpdated: false,
          artifactUpdateWarning,
        };
      }

      const opened = await measureValidationReportPerf(
        "validationService.exportReportXlsxOpenMs",
        async () => openPathWithRevealFallback(path),
        {
          meta: {
            branch: exportBranch,
            trigger: prefs.trigger,
          },
          onSuccessMeta: (result) => ({
            openedWith: normalizeText(result.openedWith),
          }),
        },
      );
      if (prefs.autoReveal && opened.openedWith !== "reveal_path") {
        await measureValidationReportPerf(
          "validationService.exportReportXlsxRevealMs",
          async () => invokeCommand("reveal_path", { path }),
          {
            meta: {
              branch: exportBranch,
              trigger: prefs.trigger,
            },
          },
        ).catch(() => undefined);
      }
      return {
        statusLine: `Saved XLSX report: ${path}`,
        reportXlsxPath: path,
        artifactUpdated,
        artifactUpdateWarning,
      };
    },
    {
      meta: {
        trigger: prefs.trigger,
        autoReveal: prefs.autoReveal,
        maxRows: prefs.maxRows,
      },
      onSuccessMeta: (result) => ({
        branch: exportBranch,
        usedFallbackBuild,
        returnedPath: !!normalizeText(result.reportXlsxPath),
        artifactUpdated: result.artifactUpdated,
        artifactUpdateWarning: !!normalizeText(result.artifactUpdateWarning),
      }),
      onErrorMeta: () => ({
        branch: exportBranch,
        usedFallbackBuild,
      }),
    },
  );
}

function buildPersistedValidationArtifactFromCurrentRun(
  reportXlsxPath: string,
): PersistValidationArtifactResult | null {
  const currentRun = readWorkflowRunArtifactFromStore();
  if (!currentRun) return null;
  return {
    path: currentRun.artifactPath,
    ts: normalizeText(currentRun.ts) || new Date().toISOString(),
    csvPath: currentRun.csvPath,
    reportXlsxPath: normalizeText(reportXlsxPath),
    datasetMeta: currentRun.datasetMeta,
    mlSignal: currentRun.mlSignal,
    optimizationMetricSurface: currentRun.optimizationMetricSurface,
    stageCompletions: updateStageCompletionReportPath(
      currentRun.stageCompletions,
      reportXlsxPath,
    ),
    tags: currentRun.tags,
    persistenceWarning: "",
  };
}

export async function exportReportXlsx(): Promise<void> {
  const report = buildCurrentReportPayloadFromStore();
  const runReq = asRecord(report.run?.req);
  const runRes = report.run?.res ?? null;
  const validationReq = asRecord(report.validation?.req ?? null);
  const validationRes = report.validation?.res ?? null;
  const exportPrefs = readValidationExportPrefs();
  const result = await exportReportXlsxInternal(report, {
    autoReveal: exportPrefs.autoReveal,
    maxRows: exportPrefs.maxRows,
    trigger: "manual",
  });
  let statusLine = result.statusLine;
  if (
    runReq &&
    runRes != null &&
    validationReq &&
    validationRes != null &&
    result.reportXlsxPath
  ) {
    if (result.artifactUpdated) {
      const persisted = buildPersistedValidationArtifactFromCurrentRun(
        result.reportXlsxPath,
      );
      if (persisted) {
        applyVueValidationArtifacts(
          runReq,
          runRes,
          validationReq,
          validationRes,
          persisted,
        );
      }
      statusLine = [result.statusLine, result.artifactUpdateWarning]
        .map((part) => normalizeText(part))
        .filter((part) => part.length > 0)
        .join(" • ");
    } else {
      const persisted = await persistValidationArtifact(
        runReq,
        runRes,
        validationReq,
        validationRes,
        {
          reportXlsxPath: result.reportXlsxPath,
        },
      );
      applyVueValidationArtifacts(
        runReq,
        runRes,
        validationReq,
        validationRes,
        persisted,
      );
      statusLine = [
        result.statusLine,
        result.artifactUpdateWarning,
        persisted.persistenceWarning,
      ]
        .map((part) => normalizeText(part))
        .filter((part) => part.length > 0)
        .join(" • ");
    }
  }
  patchValidationStoreState({ statusLine });
}

export async function openReportsDir(): Promise<void> {
  await invokeCommand("open_reports_dir");
}

export async function crossRefresh(): Promise<ValidationCrossRefreshSummary> {
  const raw = (await invokeCommand(
    "polygon_get_cache_info",
  )) as PolygonCacheInfoResponse;
  const datasets = Array.isArray(raw?.datasets) ? raw.datasets : [];

  const entries: ValidationCrossDatasetSummary[] = [];
  for (const row of datasets) {
    const dataset = (
      row && typeof row === "object" ? row : {}
    ) as PolygonCrossDatasetResponse;
    const path = normalizeText(dataset.csv_path);
    if (!path) continue;

    const label =
      normalizeText(dataset.display_symbol || dataset.polygon_ticker || path) ||
      path;
    const market = normalizeText(dataset.market);
    const timeframe = normalizeText(dataset.timeframe) || "1m";
    const adjusted = market === "stocks" ? !!dataset.adjusted : null;
    const minTs = formatYmdUtc(dataset.min_ts);
    const maxTs = formatYmdUtc(dataset.max_ts);
    const updated = formatYmdUtc(dataset.last_updated_utc_ms);
    const rowsHint = normalizeRowsHint(dataset.row_count);
    const marketLabel = normalizeMarketLabel(market, adjusted === true);

    entries.push({
      path,
      label,
      meta: `${marketLabel} • tf=${timeframe} • ${minTs} → ${maxTs} • rows=${rowsHint} • updated=${updated}`,
      market,
      timeframe: timeframe.toLowerCase(),
      adjusted,
    });
  }

  entries.sort((a, b) =>
    `${a.market}${a.label}`.localeCompare(`${b.market}${b.label}`),
  );
  return { datasets: entries };
}

export async function toggleCrossDataset(
  path: string,
  checked: boolean,
): Promise<void> {
  const nextPath = normalizePath(path);
  if (!nextPath) return;
  const selected = readVueCrossSelection();
  const nextSelected: CrossSelectionEntry[] = [];
  let present = false;
  for (const item of selected) {
    const csvPath = normalizeText(item?.csv_path ?? item?.path);
    if (!csvPath) continue;
    if (csvPath === nextPath) {
      present = true;
      if (checked) {
        nextSelected.push({
          csv_path: csvPath,
          label: normalizeText(item?.label),
        });
      }
      continue;
    }
    nextSelected.push({
      csv_path: csvPath,
      label: normalizeText(item?.label),
    });
  }
  if (checked && !present) {
    nextSelected.push({ csv_path: nextPath, label: "" });
  }
  writeVueCrossSelection(nextSelected);
}
