import { defineStore } from "pinia";
import { markRaw } from "vue";
import { useDataStore } from "./dataStore";
import { resolveRuntimePinia } from "./runtimePinia";
import { storeErrorText } from "../utils/storeErrorText";
import type {
  ValidationOverfitEvidenceItem,
  ValidationOverfitEvidenceStatus,
} from "../utils/validationEvidenceSummary";

let validationServicePromise: Promise<
  typeof import("../services/validationService")
> | null = null;

function loadValidationService(): Promise<
  typeof import("../services/validationService")
> {
  if (!validationServicePromise)
    validationServicePromise = import("../services/validationService");
  return validationServicePromise;
}

function formatActionError(prefix: string, error: unknown): string {
  const detail = storeErrorText(error);
  return detail ? `${prefix}: ${detail}` : prefix;
}

function rawArray<T>(value: unknown, fallback: T[]): T[] {
  return Array.isArray(value) ? markRaw(value as T[]) : fallback;
}

interface CrossDatasetEntry {
  path: string;
  label: string;
  meta: string;
  checked: boolean;
  disabled: boolean;
  market?: string;
  timeframe?: string;
  adjusted?: boolean | null;
}

function normalizePath(value: string): string {
  return String(value || "").trim();
}

function crossFilterTokens(value: string): string[] {
  const query = String(value || "")
    .trim()
    .toLowerCase();
  if (!query) return [];
  const out: string[] = [];
  const tokens = query.split(/[\s,]+/g);
  for (const token of tokens) {
    const next = String(token || "").trim();
    if (!next) continue;
    out.push(next);
  }
  return out;
}

function crossSearchText(entry: CrossDatasetEntry): string {
  return `${String(entry.path || "")} ${String(entry.label || "")} ${String(entry.meta || "")}`
    .trim()
    .toLowerCase();
}

function crossVisible(entry: CrossDatasetEntry, tokens: string[]): boolean {
  if (!tokens.length) return true;
  const haystack = crossSearchText(entry);
  return tokens.every((token) => haystack.includes(token));
}

function buildCrossMeta(entries: CrossDatasetEntry[], search: string): string {
  const tokens = crossFilterTokens(search);
  const total = entries.length;
  let selected = 0;
  let visible = 0;
  const hasFilter = tokens.length > 0;
  for (const entry of entries) {
    if (entry.checked) selected += 1;
    if (!hasFilter || crossVisible(entry, tokens)) visible += 1;
  }
  return buildCrossMetaFromCounts(total, selected, visible, tokens);
}

function buildCrossMetaFromCounts(
  total: number,
  selected: number,
  visible: number,
  tokens: string[],
): string {
  const parts: string[] = [];
  parts.push(
    `selected ${selected}/${total}${visible !== total ? ` (showing ${visible})` : ""}`,
  );
  if (tokens.length) parts.push(`filter=${tokens.join(" ")}`);
  return parts.join(" • ");
}

interface CrossSelectionChange {
  path: string;
  checked: boolean;
}

interface ValidationMonteCarloHistogramBar {
  label: string;
  count: number;
  pct: number;
}

function applyCrossSelection(
  rows: CrossDatasetEntry[],
  tokens: string[],
  resolveChecked: (entry: CrossDatasetEntry) => boolean,
): {
  nextRows: CrossDatasetEntry[];
  changes: CrossSelectionChange[];
  selectedCount: number;
  visibleCount: number;
} {
  const nextRows: CrossDatasetEntry[] = [];
  const changes: CrossSelectionChange[] = [];
  let selectedCount = 0;
  let visibleCount = 0;
  const hasFilter = tokens.length > 0;
  for (const entry of rows) {
    const isVisible = !hasFilter || crossVisible(entry, tokens);
    if (isVisible) visibleCount += 1;

    let nextEntry = entry;
    if (!entry.disabled && isVisible) {
      const nextChecked = !!resolveChecked(entry);
      if (entry.checked !== nextChecked) {
        nextEntry = { ...entry, checked: nextChecked };
        changes.push({ path: nextEntry.path, checked: nextChecked });
      }
    }

    if (nextEntry.checked) selectedCount += 1;
    nextRows.push(nextEntry);
  }

  return {
    nextRows: markRaw(nextRows),
    changes,
    selectedCount,
    visibleCount,
  };
}

function parseMarketLabel(value: string): {
  market: string;
  adjusted: boolean | null;
} {
  const text = String(value || "")
    .trim()
    .toLowerCase();
  if (!text) return { market: "", adjusted: null };
  if (text.startsWith("stocks")) {
    if (text.includes("(adj)")) return { market: "stocks", adjusted: true };
    if (text.includes("(raw)")) return { market: "stocks", adjusted: false };
    return { market: "stocks", adjusted: null };
  }
  return { market: text, adjusted: null };
}

function parseCurrentMarket(datasetContext: string): {
  market: string;
  adjusted: boolean | null;
} {
  const text = String(datasetContext || "");
  const match = text.match(/\bmarket=([^•]+)/i);
  if (!match || !Array.isArray(match) || match.length < 2)
    return { market: "", adjusted: null };
  return parseMarketLabel(String(match[1] || ""));
}

function parseCurrentTimeframe(
  dataTimeframe: string,
  datasetRange: string,
): string {
  const tf = String(dataTimeframe || "")
    .trim()
    .toLowerCase();
  if (tf) return tf;
  const head = String(datasetRange || "").split("•")[0];
  const rangeHead = String(head || "")
    .trim()
    .toLowerCase();
  if (!rangeHead || rangeHead.includes("→")) return "";
  return rangeHead;
}

function parseEntryContext(entry: CrossDatasetEntry): {
  market: string;
  timeframe: string;
  adjusted: boolean | null;
} {
  const meta = String(entry.meta || "");
  const metaLead = String(meta.split("•")[0] || "").trim();
  const parsedMarket = parseMarketLabel(metaLead);
  const market = String(entry.market || parsedMarket.market || "")
    .trim()
    .toLowerCase();

  const inlineAdjusted =
    entry.adjusted === true ? true : entry.adjusted === false ? false : null;
  const adjusted =
    inlineAdjusted == null ? parsedMarket.adjusted : inlineAdjusted;

  let timeframe = String(entry.timeframe || "")
    .trim()
    .toLowerCase();
  if (!timeframe) {
    const match = meta.match(/\btf=([^\s•]+)/i);
    if (match && Array.isArray(match) && match.length >= 2) {
      timeframe = String(match[1] || "")
        .trim()
        .toLowerCase();
    }
  }

  return {
    market,
    timeframe,
    adjusted,
  };
}

function applyVueTextFieldUpdate(
  state: ValidationState,
  field: string,
  value: string,
): boolean {
  const key = String(field || "").trim();
  const next = String(value || "").trim();
  if (!key) return false;
  switch (key) {
    case "valCandidates":
      state.candidates = next;
      return true;
    case "valAdmissionPolicy":
      state.admissionPolicy = next;
      return true;
    case "valSeed":
      state.seed = next;
      return true;
    case "valMinPsr":
      state.minPsr = next;
      return true;
    case "valMinDsr":
      state.minDsr = next;
      return true;
    case "valMinCscvRank":
      state.minCscvRank = next;
      return true;
    case "valMinCscvMetric":
      state.minCscvMetric = next;
      return true;
    case "valMaxPbo":
      state.maxPbo = next;
      return true;
    case "valSpaSlices":
      state.spaSlices = next;
      return true;
    case "valSpaBootstrapSamples":
      state.spaBootstrapSamples = next;
      return true;
    case "valSpaBlockLength":
      state.spaBlockLength = next;
      return true;
    case "valReoptWfWindow":
      state.reoptWfWindow = next;
      return true;
    case "valReoptWfStep":
      state.reoptWfStep = next;
      return true;
    case "valReoptWfTopN":
      state.reoptWfTopN = next;
      return true;
    case "valCscvSlices":
      state.cscvSlices = next;
      return true;
    case "valAutoExportTopN":
      state.autoExportTopN = next;
      return true;
    case "valWfWindow":
      state.wfWindow = next;
      return true;
    case "valWfStep":
      state.wfStep = next;
      return true;
    case "valPerturbIters":
      state.perturbIters = next;
      return true;
    case "valPerturbMin":
      state.perturbMin = next;
      return true;
    case "valPerturbMax":
      state.perturbMax = next;
      return true;
    case "valMcSims":
      state.mcSims = next;
      return true;
    case "valMcBlockLen":
      state.mcBlockLen = next;
      return true;
    case "valMcMode":
      state.mcMode = next;
      return true;
    case "valCrossMax":
      state.crossMax = next;
      return true;
    case "valCrossMode":
      state.crossMode = next;
      return true;
    case "valCrossSearch":
      state.crossSearch = next;
      state.crossMeta = buildCrossMeta(state.crossDatasets, next);
      return true;
    default:
      return false;
  }
}

function applyVueCheckboxFieldUpdate(
  state: ValidationState,
  field: string,
  checked: boolean,
): boolean {
  const key = String(field || "").trim();
  const next = !!checked;
  if (!key) return false;
  switch (key) {
    case "valAutoRun":
      state.autoRun = next;
      return true;
    case "valAutoExportXlsx":
      state.autoExportXlsx = next;
      return true;
    case "valAutoRevealReport":
      state.autoRevealReport = next;
      return true;
    case "valWfEnable":
      state.wfEnable = next;
      return true;
    case "valPerturbEnable":
      state.perturbEnable = next;
      return true;
    case "valMcEnable":
      state.mcEnable = next;
      return true;
    case "valCscvEnable":
      state.cscvEnable = next;
      return true;
    case "valSpaEnable":
      state.spaEnable = next;
      return true;
    case "valReoptWfEnable":
      state.reoptWfEnable = next;
      return true;
    case "valCrossEnable":
      state.crossEnable = next;
      return true;
    default:
      return false;
  }
}

interface ValidationState {
  activeResultsTab: string;
  validationHint: string;
  validationAccelerationStatus: string;
  validationMeta: string;
  validationRouteSummary: string;
  validationRouteDetailSummary: string;
  validationCategorySummary: string;
  validationAuxCpuSummary: string;
  validationBootstrapSummary: string;
  validationMonteCarloSummary: string;
  validationMonteCarloHistogram: ValidationMonteCarloHistogramBar[];
  validationRegimeSummary: string;
  validationFrictionSummary: string;
  validationCrossSummary: string;
  validationSkippedSummary: string;
  validationEvidenceSummary: string;
  validationOverfitSummary: string;
  validationOverfitStatus: ValidationOverfitEvidenceStatus;
  validationOverfitItems: ValidationOverfitEvidenceItem[];
  validationWarningSummary: string;
  validationReoptMeta: string;
  validationShortlistMeta: string;
  validationColumns: { key?: string; label: string; alignLeft: boolean }[];
  validationRows: {
    cells: { text: string; className: string; title: string }[];
  }[];
  validationOutputPreview: string;
  crossMeta: string;
  tableRowCount: number;
  candidates: string;
  admissionPolicy: string;
  seed: string;
  minPsr: string;
  minDsr: string;
  minCscvRank: string;
  minCscvMetric: string;
  maxPbo: string;
  spaSlices: string;
  spaBootstrapSamples: string;
  spaBlockLength: string;
  reoptWfWindow: string;
  reoptWfStep: string;
  reoptWfTopN: string;
  cscvSlices: string;
  autoRun: boolean;
  autoExportXlsx: boolean;
  autoRevealReport: boolean;
  autoExportTopN: string;
  wfEnable: boolean;
  wfWindow: string;
  wfStep: string;
  perturbEnable: boolean;
  perturbIters: string;
  perturbMin: string;
  perturbMax: string;
  mcEnable: boolean;
  mcSims: string;
  mcBlockLen: string;
  mcMode: string;
  cscvEnable: boolean;
  spaEnable: boolean;
  reoptWfEnable: boolean;
  crossEnable: boolean;
  crossMax: string;
  crossMode: string;
  crossSearch: string;
  crossDatasets: CrossDatasetEntry[];
  crossSelectedCount: number;
  crossTotalCount: number;
  tableVisible: boolean;
  canOpenReportsDir: boolean;
  canExportValidationCsv: boolean;
  canExportValidationJson: boolean;
  canExportReportJson: boolean;
  canExportReportXlsx: boolean;
  canCrossRefresh: boolean;
  canCrossSelectAll: boolean;
  canCrossSelectNone: boolean;
  canCrossSelectMatch: boolean;
  statusLine: string;
  validateDisabled: boolean;
  source: "vue" | "unknown";
  updatedAtUnixMs: number;
}

export const useValidationStore = defineStore("validation", {
  state: (): ValidationState => ({
    activeResultsTab: "overview",
    validationHint: "—",
    validationAccelerationStatus: "Validation not run.",
    validationMeta: "-",
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
    validationOverfitSummary: "Overfit evidence not run.",
    validationOverfitStatus: "not_run",
    validationOverfitItems: [],
    validationWarningSummary: "-",
    validationReoptMeta: "-",
    validationShortlistMeta: "-",
    validationColumns: [],
    validationRows: [],
    validationOutputPreview: "-",
    crossMeta: "—",
    tableRowCount: 0,
    candidates: "32",
    admissionPolicy: "legacy_topk",
    seed: "1",
    minPsr: "0.80",
    minDsr: "0.60",
    minCscvRank: "0.55",
    minCscvMetric: "",
    maxPbo: "0.20",
    spaSlices: "16",
    spaBootstrapSamples: "500",
    spaBlockLength: "4",
    reoptWfWindow: "20000",
    reoptWfStep: "5000",
    reoptWfTopN: "8",
    cscvSlices: "16",
    autoRun: true,
    autoExportXlsx: false,
    autoRevealReport: false,
    autoExportTopN: "20",
    wfEnable: true,
    wfWindow: "20000",
    wfStep: "5000",
    perturbEnable: false,
    perturbIters: "8",
    perturbMin: "0.015",
    perturbMax: "0.06",
    mcEnable: false,
    mcSims: "500",
    mcBlockLen: "",
    mcMode: "compare",
    cscvEnable: true,
    spaEnable: false,
    reoptWfEnable: false,
    crossEnable: false,
    crossMax: "6",
    crossMode: "auto",
    crossSearch: "",
    crossDatasets: [],
    crossSelectedCount: 0,
    crossTotalCount: 0,
    tableVisible: false,
    canOpenReportsDir: true,
    canExportValidationCsv: false,
    canExportValidationJson: false,
    canExportReportJson: false,
    canExportReportXlsx: false,
    canCrossRefresh: true,
    canCrossSelectAll: true,
    canCrossSelectNone: true,
    canCrossSelectMatch: true,
    statusLine: "Ready.",
    validateDisabled: false,
    source: "unknown",
    updatedAtUnixMs: 0,
  }),

  actions: {
    applyLegacySnapshot(snapshot: Partial<ValidationState>): void {
      this.activeResultsTab =
        String(
          snapshot.activeResultsTab ?? this.activeResultsTab ?? "overview",
        ).trim() || "overview";
      this.validationHint =
        String(snapshot.validationHint ?? this.validationHint ?? "—").trim() ||
        "—";
      this.validationAccelerationStatus =
        String(
          snapshot.validationAccelerationStatus ??
            this.validationAccelerationStatus ??
            "Validation not run.",
        ).trim() || "Validation not run.";
      this.validationMeta =
        String(snapshot.validationMeta ?? this.validationMeta ?? "-").trim() ||
        "-";
      this.validationRouteSummary =
        String(
          snapshot.validationRouteSummary ?? this.validationRouteSummary ?? "-",
        ).trim() || "-";
      this.validationRouteDetailSummary =
        String(
          snapshot.validationRouteDetailSummary ??
            this.validationRouteDetailSummary ??
            "-",
        ).trim() || "-";
      this.validationCategorySummary =
        String(
          snapshot.validationCategorySummary ??
            this.validationCategorySummary ??
            "-",
        ).trim() || "-";
      this.validationAuxCpuSummary =
        String(
          snapshot.validationAuxCpuSummary ??
            this.validationAuxCpuSummary ??
            "-",
        ).trim() || "-";
      this.validationBootstrapSummary =
        String(
          snapshot.validationBootstrapSummary ??
            this.validationBootstrapSummary ??
            "-",
        ).trim() || "-";
      this.validationMonteCarloSummary =
        String(
          snapshot.validationMonteCarloSummary ??
            this.validationMonteCarloSummary ??
            "not run",
        ).trim() || "not run";
      this.validationMonteCarloHistogram = rawArray(
        snapshot.validationMonteCarloHistogram,
        this.validationMonteCarloHistogram,
      );
      this.validationRegimeSummary =
        String(
          snapshot.validationRegimeSummary ??
            this.validationRegimeSummary ??
            "-",
        ).trim() || "-";
      this.validationFrictionSummary =
        String(
          snapshot.validationFrictionSummary ??
            this.validationFrictionSummary ??
            "-",
        ).trim() || "-";
      this.validationCrossSummary =
        String(
          snapshot.validationCrossSummary ?? this.validationCrossSummary ?? "-",
        ).trim() || "-";
      this.validationSkippedSummary =
        String(
          snapshot.validationSkippedSummary ??
            this.validationSkippedSummary ??
            "-",
        ).trim() || "-";
      this.validationEvidenceSummary =
        String(
          snapshot.validationEvidenceSummary ??
            this.validationEvidenceSummary ??
            "Validation evidence not run.",
        ).trim() || "Validation evidence not run.";
      this.validationOverfitSummary =
        String(
          snapshot.validationOverfitSummary ??
            this.validationOverfitSummary ??
            "Overfit evidence not run.",
        ).trim() || "Overfit evidence not run.";
      const nextOverfitStatus = String(
        snapshot.validationOverfitStatus ??
          this.validationOverfitStatus ??
          "not_run",
      ).trim();
      this.validationOverfitStatus =
        nextOverfitStatus === "available" || nextOverfitStatus === "attention"
          ? nextOverfitStatus
          : "not_run";
      this.validationOverfitItems = rawArray(
        snapshot.validationOverfitItems,
        this.validationOverfitItems,
      );
      this.validationWarningSummary =
        String(
          snapshot.validationWarningSummary ??
            this.validationWarningSummary ??
            "-",
        ).trim() || "-";
      this.validationReoptMeta =
        String(
          snapshot.validationReoptMeta ?? this.validationReoptMeta ?? "-",
        ).trim() || "-";
      this.validationShortlistMeta =
        String(
          snapshot.validationShortlistMeta ??
            this.validationShortlistMeta ??
            "-",
        ).trim() || "-";
      this.validationColumns = rawArray(
        snapshot.validationColumns,
        this.validationColumns,
      );
      this.validationRows = rawArray(
        snapshot.validationRows,
        this.validationRows,
      );
      this.validationOutputPreview =
        String(
          snapshot.validationOutputPreview ??
            this.validationOutputPreview ??
            "-",
        ).trim() || "-";
      this.crossMeta =
        String(snapshot.crossMeta ?? this.crossMeta ?? "—").trim() || "—";
      this.tableRowCount = Math.max(
        0,
        Math.floor(Number(snapshot.tableRowCount ?? this.tableRowCount ?? 0)),
      );
      this.candidates =
        String(snapshot.candidates ?? this.candidates ?? "32").trim() || "32";
      this.admissionPolicy =
        String(
          snapshot.admissionPolicy ?? this.admissionPolicy ?? "legacy_topk",
        ).trim() || "legacy_topk";
      this.seed = String(snapshot.seed ?? this.seed ?? "1").trim() || "1";
      this.minPsr = String(snapshot.minPsr ?? this.minPsr ?? "").trim();
      this.minDsr = String(snapshot.minDsr ?? this.minDsr ?? "").trim();
      this.minCscvRank = String(
        snapshot.minCscvRank ?? this.minCscvRank ?? "",
      ).trim();
      this.minCscvMetric = String(
        snapshot.minCscvMetric ?? this.minCscvMetric ?? "",
      ).trim();
      this.maxPbo = String(snapshot.maxPbo ?? this.maxPbo ?? "").trim();
      this.spaSlices =
        String(snapshot.spaSlices ?? this.spaSlices ?? "16").trim() || "16";
      this.spaBootstrapSamples =
        String(
          snapshot.spaBootstrapSamples ?? this.spaBootstrapSamples ?? "500",
        ).trim() || "500";
      this.spaBlockLength =
        String(snapshot.spaBlockLength ?? this.spaBlockLength ?? "4").trim() ||
        "4";
      this.reoptWfWindow =
        String(
          snapshot.reoptWfWindow ?? this.reoptWfWindow ?? "20000",
        ).trim() || "20000";
      this.reoptWfStep =
        String(snapshot.reoptWfStep ?? this.reoptWfStep ?? "5000").trim() ||
        "5000";
      this.reoptWfTopN =
        String(snapshot.reoptWfTopN ?? this.reoptWfTopN ?? "8").trim() || "8";
      this.cscvSlices = String(
        snapshot.cscvSlices ?? this.cscvSlices ?? "",
      ).trim();
      this.autoRun = Boolean(snapshot.autoRun ?? this.autoRun);
      this.autoExportXlsx = Boolean(
        snapshot.autoExportXlsx ?? this.autoExportXlsx,
      );
      this.autoRevealReport = Boolean(
        snapshot.autoRevealReport ?? this.autoRevealReport,
      );
      this.autoExportTopN =
        String(snapshot.autoExportTopN ?? this.autoExportTopN ?? "20").trim() ||
        "20";
      this.wfEnable = Boolean(snapshot.wfEnable ?? this.wfEnable);
      this.wfWindow =
        String(snapshot.wfWindow ?? this.wfWindow ?? "20000").trim() || "20000";
      this.wfStep =
        String(snapshot.wfStep ?? this.wfStep ?? "5000").trim() || "5000";
      this.perturbEnable = Boolean(
        snapshot.perturbEnable ?? this.perturbEnable,
      );
      this.perturbIters =
        String(snapshot.perturbIters ?? this.perturbIters ?? "8").trim() || "8";
      this.perturbMin =
        String(snapshot.perturbMin ?? this.perturbMin ?? "0.015").trim() ||
        "0.015";
      this.perturbMax =
        String(snapshot.perturbMax ?? this.perturbMax ?? "0.06").trim() ||
        "0.06";
      this.mcEnable = Boolean(snapshot.mcEnable ?? this.mcEnable);
      this.mcSims =
        String(snapshot.mcSims ?? this.mcSims ?? "500").trim() || "500";
      this.mcBlockLen = String(
        snapshot.mcBlockLen ?? this.mcBlockLen ?? "",
      ).trim();
      this.mcMode =
        String(snapshot.mcMode ?? this.mcMode ?? "compare").trim() || "compare";
      this.cscvEnable = Boolean(snapshot.cscvEnable ?? this.cscvEnable);
      this.spaEnable = Boolean(snapshot.spaEnable ?? this.spaEnable);
      this.reoptWfEnable = Boolean(
        snapshot.reoptWfEnable ?? this.reoptWfEnable,
      );
      this.crossEnable = Boolean(snapshot.crossEnable ?? this.crossEnable);
      this.crossMax =
        String(snapshot.crossMax ?? this.crossMax ?? "6").trim() || "6";
      this.crossMode =
        String(snapshot.crossMode ?? this.crossMode ?? "auto").trim() || "auto";
      this.crossSearch = String(
        snapshot.crossSearch ?? this.crossSearch ?? "",
      ).trim();
      this.crossDatasets = rawArray(snapshot.crossDatasets, this.crossDatasets);
      this.crossSelectedCount = Math.max(
        0,
        Math.floor(
          Number(snapshot.crossSelectedCount ?? this.crossSelectedCount ?? 0),
        ),
      );
      this.crossTotalCount = Math.max(
        0,
        Math.floor(
          Number(snapshot.crossTotalCount ?? this.crossTotalCount ?? 0),
        ),
      );
      this.tableVisible = Boolean(snapshot.tableVisible ?? this.tableVisible);
      this.canOpenReportsDir = Boolean(
        snapshot.canOpenReportsDir ?? this.canOpenReportsDir,
      );
      this.canExportValidationCsv = Boolean(
        snapshot.canExportValidationCsv ?? this.canExportValidationCsv,
      );
      this.canExportValidationJson = Boolean(
        snapshot.canExportValidationJson ?? this.canExportValidationJson,
      );
      this.canExportReportJson = Boolean(
        snapshot.canExportReportJson ?? this.canExportReportJson,
      );
      this.canExportReportXlsx = Boolean(
        snapshot.canExportReportXlsx ?? this.canExportReportXlsx,
      );
      this.canCrossRefresh = Boolean(
        snapshot.canCrossRefresh ?? this.canCrossRefresh,
      );
      this.canCrossSelectAll = Boolean(
        snapshot.canCrossSelectAll ?? this.canCrossSelectAll,
      );
      this.canCrossSelectNone = Boolean(
        snapshot.canCrossSelectNone ?? this.canCrossSelectNone,
      );
      this.canCrossSelectMatch = Boolean(
        snapshot.canCrossSelectMatch ?? this.canCrossSelectMatch,
      );
      this.statusLine =
        String(snapshot.statusLine ?? this.statusLine ?? "Ready.").trim() ||
        "Ready.";
      this.validateDisabled = Boolean(
        snapshot.validateDisabled ?? this.validateDisabled,
      );
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();
    },

    setInputField(field: string, value: string): void {
      const key = String(field || "").trim();
      const next = String(value || "");
      if (applyVueTextFieldUpdate(this, key, next)) {
        this.source = "vue";
        this.updatedAtUnixMs = Date.now();
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

    runValidation(): void {
      void loadValidationService()
        .then((service) => service.runValidation())
        .catch((error: unknown) => {
          const message = formatActionError("Validation failed", error);
          this.statusLine = message;
          this.validationHint = message;
          this.validationAccelerationStatus = "Validation failed.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    exportShortlistCsv(): void {
      void loadValidationService()
        .then((service) => service.exportShortlistCsv())
        .catch((error: unknown) => {
          this.statusLine = formatActionError(
            "Shortlist CSV export failed",
            error,
          );
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    exportValidationJson(): void {
      void loadValidationService()
        .then((service) => service.exportValidationJson())
        .catch((error: unknown) => {
          this.statusLine = formatActionError(
            "Validation JSON export failed",
            error,
          );
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    exportReportJson(): void {
      void loadValidationService()
        .then((service) => service.exportReportJson())
        .catch((error: unknown) => {
          this.statusLine = formatActionError(
            "Report JSON export failed",
            error,
          );
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    exportReportXlsx(): void {
      void loadValidationService()
        .then((service) => service.exportReportXlsx())
        .catch((error: unknown) => {
          this.statusLine = formatActionError(
            "Report XLSX export failed",
            error,
          );
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    openReportsDir(): void {
      if (!this.canOpenReportsDir) return;
      void loadValidationService()
        .then((service) => service.openReportsDir())
        .catch(() => undefined);
    },

    crossRefresh(): void {
      if (!this.canCrossRefresh) return;
      const previousSelected = new Set<string>();
      for (const entry of this.crossDatasets) {
        if (!entry.checked) continue;
        const path = normalizePath(entry.path);
        if (!path) continue;
        previousSelected.add(path);
      }

      void loadValidationService()
        .then((service) =>
          service.crossRefresh().then((summary) => {
            const rows = Array.isArray(summary?.datasets)
              ? summary.datasets
              : [];
            const activePaths = new Set<string>();
            const next: CrossDatasetEntry[] = [];
            const tokens = crossFilterTokens(this.crossSearch);
            const hasFilter = tokens.length > 0;
            let selectedCount = 0;
            let visibleCount = 0;
            for (const row of rows) {
              const path = normalizePath(row.path);
              if (!path || activePaths.has(path)) continue;
              activePaths.add(path);
              const label = String(row.label || path).trim() || path;
              const meta = String(row.meta || "—").trim() || "—";
              const nextEntry: CrossDatasetEntry = {
                path,
                label,
                meta,
                checked: previousSelected.has(path),
                disabled: this.validateDisabled,
                market: String(row.market || "")
                  .trim()
                  .toLowerCase(),
                timeframe: String(row.timeframe || "")
                  .trim()
                  .toLowerCase(),
                adjusted:
                  row.adjusted === true
                    ? true
                    : row.adjusted === false
                      ? false
                      : null,
              };
              if (nextEntry.checked) selectedCount += 1;
              if (!hasFilter || crossVisible(nextEntry, tokens))
                visibleCount += 1;
              next.push(nextEntry);
            }

            const staleSelections: string[] = [];
            for (const path of previousSelected) {
              if (!activePaths.has(path)) staleSelections.push(path);
            }
            this.crossDatasets = markRaw(next);
            this.crossSelectedCount = selectedCount;
            this.crossTotalCount = next.length;
            this.crossMeta = buildCrossMetaFromCounts(
              next.length,
              selectedCount,
              visibleCount,
              tokens,
            );
            this.statusLine = "Refreshed cross-ticker list.";
            this.source = "vue";
            this.updatedAtUnixMs = Date.now();

            if (!staleSelections.length) return;
            void Promise.all(
              staleSelections.map((path) =>
                service.toggleCrossDataset(path, false),
              ),
            ).catch((error: unknown) => {
              this.statusLine = formatActionError(
                "Cross-ticker refresh sync failed",
                error,
              );
              this.source = "vue";
              this.updatedAtUnixMs = Date.now();
            });
          }),
        )
        .catch((error: unknown) => {
          this.statusLine = formatActionError(
            "Cross-ticker refresh failed",
            error,
          );
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    crossSelectAll(): void {
      const tokens = crossFilterTokens(this.crossSearch);
      const { nextRows, changes, selectedCount, visibleCount } =
        applyCrossSelection(this.crossDatasets, tokens, () => true);

      this.crossDatasets = nextRows;
      this.crossSelectedCount = selectedCount;
      this.crossTotalCount = nextRows.length;
      this.crossMeta = buildCrossMetaFromCounts(
        nextRows.length,
        selectedCount,
        visibleCount,
        tokens,
      );
      this.statusLine = "Cross-ticker: selected all (filtered view).";
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();

      void Promise.all(
        changes.map((entry) =>
          loadValidationService().then((service) =>
            service.toggleCrossDataset(entry.path, true),
          ),
        ),
      ).catch((error: unknown) => {
        this.statusLine = formatActionError(
          "Cross-ticker selection failed",
          error,
        );
        this.source = "vue";
        this.updatedAtUnixMs = Date.now();
      });
    },

    crossSelectNone(): void {
      const tokens = crossFilterTokens(this.crossSearch);
      const { nextRows, changes, selectedCount, visibleCount } =
        applyCrossSelection(this.crossDatasets, tokens, () => false);

      this.crossDatasets = nextRows;
      this.crossSelectedCount = selectedCount;
      this.crossTotalCount = nextRows.length;
      this.crossMeta = buildCrossMetaFromCounts(
        nextRows.length,
        selectedCount,
        visibleCount,
        tokens,
      );
      this.statusLine = "Cross-ticker: cleared selection (filtered view).";
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();

      void Promise.all(
        changes.map((entry) =>
          loadValidationService().then((service) =>
            service.toggleCrossDataset(entry.path, false),
          ),
        ),
      ).catch((error: unknown) => {
        this.statusLine = formatActionError(
          "Cross-ticker selection failed",
          error,
        );
        this.source = "vue";
        this.updatedAtUnixMs = Date.now();
      });
    },

    crossSelectMatch(): void {
      if (!this.canCrossSelectMatch) return;

      const pinia = resolveRuntimePinia();
      if (!pinia) return;
      const dataStore = useDataStore(pinia);
      const currentPath = normalizePath(dataStore.csvPath);
      const currentMarket = parseCurrentMarket(dataStore.datasetContext);
      const timeframe = parseCurrentTimeframe(
        dataStore.dataTimeframe,
        dataStore.datasetRange,
      );

      if (!currentMarket.market || !timeframe) {
        this.statusLine =
          "Cross-ticker: unable to match current dataset (load data + timeframe first).";
        this.source = "vue";
        this.updatedAtUnixMs = Date.now();
        return;
      }

      const tokens = crossFilterTokens(this.crossSearch);
      const { nextRows, changes, selectedCount, visibleCount } =
        applyCrossSelection(this.crossDatasets, tokens, (entry) => {
          const path = normalizePath(entry.path);
          if (currentPath && path === currentPath) return false;

          const ctx = parseEntryContext(entry);
          let checked =
            ctx.market === currentMarket.market && ctx.timeframe === timeframe;
          if (
            checked &&
            currentMarket.market === "stocks" &&
            currentMarket.adjusted != null
          ) {
            checked = ctx.adjusted === currentMarket.adjusted;
          }
          return checked;
        });

      this.crossDatasets = nextRows;
      this.crossSelectedCount = selectedCount;
      this.crossTotalCount = nextRows.length;
      this.crossMeta = buildCrossMetaFromCounts(
        nextRows.length,
        selectedCount,
        visibleCount,
        tokens,
      );
      this.statusLine =
        "Cross-ticker: selected datasets matching current dataset.";
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();

      void Promise.all(
        changes.map((entry) =>
          loadValidationService().then((service) =>
            service.toggleCrossDataset(entry.path, entry.checked),
          ),
        ),
      ).catch((error: unknown) => {
        this.statusLine = formatActionError(
          "Cross-ticker match selection failed",
          error,
        );
        this.source = "vue";
        this.updatedAtUnixMs = Date.now();
      });
    },

    toggleCrossDataset(path: string, checked: boolean): void {
      const nextPath = String(path || "").trim();
      if (!nextPath) return;
      const nextChecked = !!checked;
      const tokens = crossFilterTokens(this.crossSearch);
      const next: CrossDatasetEntry[] = [];
      const hasFilter = tokens.length > 0;
      let selectedCount = 0;
      let visibleCount = 0;
      for (const entry of this.crossDatasets) {
        let nextEntry = entry;
        if (String(entry.path || "").trim() === nextPath && !entry.disabled) {
          nextEntry = {
            ...entry,
            checked: nextChecked,
          };
        }
        if (nextEntry.checked) selectedCount += 1;
        if (!hasFilter || crossVisible(nextEntry, tokens)) visibleCount += 1;
        next.push(nextEntry);
      }
      this.crossDatasets = markRaw(next);
      this.crossSelectedCount = selectedCount;
      this.crossTotalCount = next.length;
      this.crossMeta = buildCrossMetaFromCounts(
        next.length,
        selectedCount,
        visibleCount,
        tokens,
      );
      this.statusLine = `Cross-ticker: ${nextChecked ? "selected" : "cleared"} dataset.`;
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();

      void loadValidationService()
        .then((service) => service.toggleCrossDataset(nextPath, nextChecked))
        .catch((error: unknown) => {
          this.statusLine = formatActionError(
            "Cross-ticker update failed",
            error,
          );
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },
  },
});
