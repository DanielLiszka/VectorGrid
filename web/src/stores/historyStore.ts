import Fuse from "fuse.js";
import type { IFuseOptions } from "fuse.js";
import { defineStore } from "pinia";
import { markRaw } from "vue";
import type {
  HistoryRunArtifactRepairSummary,
  HistorySavedRunDetail,
  HistorySavedRunSummary,
  HistorySessionSummary,
} from "../services/historyService";
import { formatUiBinaryBytes } from "../utils/uiFormat";
import {
  buildRunCompareSummary,
  formatArtifactLineageCompact,
  formatRouteProvenanceCompact,
  type ArtifactLineageSummary,
  type RouteProvenanceSummary,
  type RunComparisonTarget,
} from "../utils/core/runArtifactReview";
import {
  canOpenPersistedReport,
  artifactCompatibilityAllowsReplay,
  artifactCompatibilityMetaToken,
  artifactCompatibilitySummary,
  formatReportStateLabel,
  formatReportStateMetaToken,
  replayClassLabel,
  replayClassSummary,
  resolveReportState,
} from "../utils/core/artifactEvidenceStatus";
import { storeErrorText } from "../utils/storeErrorText";

let historyServicePromise: Promise<
  typeof import("../services/historyService")
> | null = null;

function loadHistoryService(): Promise<
  typeof import("../services/historyService")
> {
  if (!historyServicePromise) {
    historyServicePromise = import("../services/historyService");
  }
  return historyServicePromise;
}

export interface HistoryOption {
  value: string;
  label: string;
  disabled: boolean;
}

type HistorySavedRunEntry = HistorySavedRunSummary;

interface SavedRunSearchRecord {
  entry: HistorySavedRunEntry;
  index: number;
  ts: string;
  strategyKind: string;
  replayClass: string;
  replayReasons: string[];
  csvPath: string;
  csvBasename: string;
  artifactPath: string;
  artifactBasename: string;
  tags: string[];
  shortFields: string[];
  searchText: string;
}

interface SavedRunSearchIndex {
  records: SavedRunSearchRecord[];
  fuse: Fuse<SavedRunSearchRecord>;
}

const SAVED_RUN_FUSE_OPTIONS: IFuseOptions<SavedRunSearchRecord> = {
  includeScore: true,
  ignoreLocation: true,
  shouldSort: true,
  threshold: 0.34,
  minMatchCharLength: 2,
  keys: [
    { name: "ts", weight: 0.25 },
    { name: "strategyKind", weight: 0.22 },
    { name: "csvBasename", weight: 0.2 },
    { name: "artifactBasename", weight: 0.16 },
    { name: "tags", weight: 0.1 },
    { name: "replayClass", weight: 0.04 },
    { name: "replayReasons", weight: 0.02 },
    { name: "csvPath", weight: 0.01 },
  ],
};

function normalizeText(value: unknown): string {
  return String(value || "").trim();
}

function normalizePath(value: string): string {
  return normalizeText(value);
}

function normalizeOptionalBoolean(value: unknown): boolean | null {
  if (typeof value === "boolean") return value;
  const text = normalizeText(value).toLowerCase();
  if (!text) return null;
  if (text === "1" || text === "true" || text === "yes" || text === "on")
    return true;
  if (text === "0" || text === "false" || text === "no" || text === "off")
    return false;
  return null;
}

function normalizeTags(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  const out: string[] = [];
  for (const item of value) {
    const next = normalizeText(item);
    if (!next) continue;
    out.push(next);
  }
  return markRaw(out);
}

function rawArray<T>(value: T[]): T[] {
  return markRaw(value);
}

function filterTokens(value: string): string[] {
  const query = normalizeText(value).toLowerCase();
  if (!query) return [];
  const out: string[] = [];
  const tokens = query.split(/\s+/g);
  for (const token of tokens) {
    const next = normalizeText(token);
    if (!next) continue;
    out.push(next);
  }
  return out;
}

function fileBasename(path: string): string {
  const next = normalizePath(path);
  if (!next) return "";
  const parts = next.split(/[\\/]+/g);
  return normalizeText(parts[parts.length - 1] || "");
}

function formatBytesCompact(bytes: number): string {
  return formatUiBinaryBytes(bytes);
}

function normalizeSearchText(value: unknown): string {
  return normalizeText(value).toLowerCase();
}

function runtimeRunning(): boolean {
  if (typeof document === "undefined") return false;
  const root = document.documentElement as HTMLElement | undefined;
  if (!root || !root.dataset) return false;
  return String(root.dataset.running || "").trim() === "1";
}

function savedRunLabel(entry: HistorySavedRunEntry): string {
  const hasVal = entry.hasValidation ? "val" : "-";
  const reportState = resolveReportState(
    entry.reportXlsxPath,
    entry.reportXlsxExists,
  );
  const hasXlsx =
    reportState === "available"
      ? "xlsx"
      : reportState === "unknown"
        ? "xlsx?"
        : "-";
  const tagsHint = entry.tags.length ? ` | tags=${entry.tags.length}` : "";
  return `${entry.ts || "unknown"} | ${entry.strategyKind || ""} | ${hasVal}/${hasXlsx}${tagsHint}`;
}

function sessionRunLabel(entry: HistorySessionSummary): string {
  return `${entry.ts || "unknown"} | ${entry.backend || ""} | ${Math.max(0, Math.floor(Number(entry.pairs || 0)))} pairs`;
}

const savedRunSearchIndexCache = new WeakMap<
  HistorySavedRunEntry[],
  SavedRunSearchIndex
>();

function buildSavedRunSearchRecords(
  entries: HistorySavedRunEntry[],
): SavedRunSearchRecord[] {
  return entries.map((entry, index) => {
    const ts = normalizeSearchText(entry.ts);
    const strategyKind = normalizeSearchText(entry.strategyKind);
    const csvPath = normalizeSearchText(entry.csvPath);
    const csvBasename = normalizeSearchText(fileBasename(entry.csvPath));
    const artifactPath = normalizeSearchText(entry.path);
    const artifactBasename = normalizeSearchText(fileBasename(entry.path));
    const tags = entry.tags
      .map((tag) => normalizeSearchText(tag))
      .filter((tag) => tag.length > 0);
    const replayClass = normalizeSearchText(entry.replayClass);
    const replayReasons = normalizeTags(entry.replayReasons)
      .map((reason) => normalizeSearchText(reason))
      .filter((reason) => reason.length > 0);
    const shortFields = [
      ts,
      strategyKind,
      replayClass,
      ...replayReasons,
      csvBasename,
      artifactBasename,
      ...tags,
    ]
      .filter((part) => part.length > 0);
    const searchText = [shortFields.join(" "), csvPath, artifactPath]
      .filter((part) => part.length > 0)
      .join(" ");
    return {
      entry,
      index,
      ts,
      strategyKind,
      replayClass,
      replayReasons,
      csvPath,
      csvBasename,
      artifactPath,
      artifactBasename,
      tags,
      shortFields,
      searchText,
    };
  });
}

function getSavedRunSearchIndex(
  entries: HistorySavedRunEntry[],
): SavedRunSearchIndex {
  const cached = savedRunSearchIndexCache.get(entries);
  if (cached) return cached;

  const records = buildSavedRunSearchRecords(entries);
  const nextIndex = {
    records,
    fuse: new Fuse(records, SAVED_RUN_FUSE_OPTIONS),
  };
  savedRunSearchIndexCache.set(entries, nextIndex);
  return nextIndex;
}

function filterSavedRunsEntries(
  entries: HistorySavedRunEntry[],
  search: string,
): HistorySavedRunEntry[] {
  const tokens = filterTokens(search);
  if (!tokens.length) return entries;
  const { records, fuse } = getSavedRunSearchIndex(entries);

  const shortQueryMatches = (): HistorySavedRunEntry[] => {
    const scored: Array<{ record: SavedRunSearchRecord; totalScore: number }> = [];
    for (const record of records) {
      let totalScore = 0;
      let matched = true;
      for (const token of tokens) {
        let tokenScore = -1;
        for (const field of record.shortFields) {
          if (field === token) tokenScore = Math.max(tokenScore, 9);
          else if (field.startsWith(token)) tokenScore = Math.max(tokenScore, 7);
          else if (field.includes(token)) tokenScore = Math.max(tokenScore, 4);
        }
        if (tokenScore < 0) {
          matched = false;
          break;
        }
        totalScore += tokenScore;
      }
      if (matched) scored.push({ record, totalScore });
    }
    scored.sort((left, right) => {
      if (right.totalScore !== left.totalScore)
        return right.totalScore - left.totalScore;
      return left.record.index - right.record.index;
    });
    return scored.map((row) => row.record.entry);
  };

  if (tokens.every((token) => token.length < 3)) {
    return shortQueryMatches();
  }

  const query = normalizeSearchText(search);
  const results = fuse.search(query);
  if (!results.length) return shortQueryMatches();

  const strictTokenMatches = results.filter(
    ({ item }) =>
      tokens.length <= 1 ||
      tokens.every((token) => item.searchText.includes(token)),
  );
  const ranked = strictTokenMatches.length > 0 ? strictTokenMatches : results;
  return ranked.map(({ item }) => item.entry);
}

interface HistoryState {
  sessionRunCount: number;
  sessionRunsRaw: HistorySessionSummary[];
  sessionOptions: HistoryOption[];
  sessionSelectedValue: string;
  sessionSelectedLabel: string;
  savedRunCount: number;
  savedRunsRaw: HistorySavedRunEntry[];
  savedRunsVisible: HistorySavedRunEntry[];
  savedRunDetailCache: Record<string, HistorySavedRunDetail>;
  savedRunDetailLoadingPath: string;
  savedRunDetailSeq: number;
  savedOptions: HistoryOption[];
  savedSelectedValue: string;
  savedSelectedLabel: string;
  compareRunCount: number;
  compareOptions: HistoryOption[];
  compareSelectedValue: string;
  savedRunCompareOptions: HistoryOption[];
  savedRunCompareSelected: string;
  savedRunCompareSummary: string;
  savedRunsSearch: string;
  savedRunsTagsDraft: string;
  savedRunsMeta: string;
  savedRunDetailTitle: string;
  savedRunDetailMeta: string;
  savedRunDetailStrategy: string;
  savedRunDetailConfigFingerprint: string;
  savedRunDetailDataset: string;
  savedRunDetailTimeframe: string;
  savedRunDetailValidation: string;
  savedRunDetailReplay: string;
  savedRunDetailCompatibility: string;
  savedRunDetailProvenance: string;
  savedRunDetailLineage: string;
  savedRunDetailMetricSurface: string;
  savedRunDetailMetricSurfaceDetail: string;
  savedRunDetailAdmission: string;
  savedRunDetailAdmissionDetail: string;
  savedRunDetailReport: string;
  savedRunDetailSize: string;
  savedRunDetailTags: string;
  savedRunDetailMlBundle: string;
  savedRunDetailMlScoreCache: string;
  savedRunDetailMlFeatureSchema: string;
  savedRunDetailMlScoreContract: string;
  savedRunDetailMlScoreNote: string;
  savedRunDetailMlPolicy: string;
  savedRunDetailMlSweep: string;
  savedRunDetailMlVerdict: string;
  savedRunDetailMlCandidates: string;
  savedRunDetailParams: string;
  savedRunRepairSummary: string;
  canApplyHistory: boolean;
  canExportPreset: boolean;
  canImportPreset: boolean;
  canClearHistory: boolean;
  canSaveSavedRunTags: boolean;
  canRefreshSavedRuns: boolean;
  canLoadSavedRun: boolean;
  canApplySavedRun: boolean;
  canRerunSavedRun: boolean;
  canOpenSavedRunReport: boolean;
  canRevealSavedRun: boolean;
  canDeleteSavedRun: boolean;
  canRepairSavedRun: boolean;
  canWriteRepairedSavedRun: boolean;
  canOpenRunsDir: boolean;
  statusLine: string;
  source: "vue" | "unknown";
  updatedAtUnixMs: number;
}

function sanitizeOptionArray(value: unknown): HistoryOption[] {
  if (!Array.isArray(value)) return [];
  const out: HistoryOption[] = [];
  for (const entry of value) {
    if (!entry || typeof entry !== "object") continue;
    const valueText = String((entry as { value?: unknown }).value ?? "").trim();
    const labelText = String((entry as { label?: unknown }).label ?? "").trim();
    const disabled = Boolean((entry as { disabled?: unknown }).disabled);
    out.push({
      value: valueText,
      label: labelText || valueText || "—",
      disabled,
    });
  }
  return rawArray(out);
}

function sanitizeSessionRunsArray(value: unknown): HistorySessionSummary[] {
  if (!Array.isArray(value)) return [];
  const out: HistorySessionSummary[] = [];
  for (const item of value) {
    if (!item || typeof item !== "object") continue;
    const typed = item as Partial<HistorySessionSummary>;
    const ts = normalizeText(typed.ts);
    if (!ts) continue;
    const pairs = Number(typed.pairs);
    out.push({
      ts,
      backend: normalizeText(typed.backend),
      pairs: Number.isFinite(pairs) && pairs > 0 ? Math.floor(pairs) : 0,
    });
  }
  return rawArray(out);
}

function sanitizeSavedRunsArray(value: unknown): HistorySavedRunEntry[] {
  if (!Array.isArray(value)) return [];
  const out: HistorySavedRunEntry[] = [];
  for (const item of value) {
    if (!item || typeof item !== "object") continue;
    const typed = item as Partial<HistorySavedRunEntry>;
    const path = normalizePath(String(typed.path || ""));
    if (!path) continue;
    const bytes = Number(typed.bytes);
    out.push({
      ts: normalizeText(typed.ts),
      path,
      bytes: Number.isFinite(bytes) && bytes > 0 ? Math.floor(bytes) : 0,
      strategyKind: normalizeText(typed.strategyKind),
      configFingerprint: normalizeText(typed.configFingerprint),
      configFingerprintVersion: Number.isFinite(
        Number(typed.configFingerprintVersion),
      )
        ? Math.floor(Number(typed.configFingerprintVersion))
        : null,
      hasValidation: !!typed.hasValidation,
      reportXlsxPath: normalizePath(String(typed.reportXlsxPath || "")),
      reportXlsxExists: normalizeOptionalBoolean(typed.reportXlsxExists),
      csvPath: normalizePath(String(typed.csvPath || "")),
      timeframe: normalizeText(typed.timeframe),
      baseTimeframe: normalizeText(typed.baseTimeframe),
      derivedTimeframe: normalizeText(typed.derivedTimeframe),
      resampleApplied:
        typed.resampleApplied == null ? null : Boolean(typed.resampleApplied),
      sourceContentFingerprint: normalizeText(typed.sourceContentFingerprint),
      replayClass: normalizeText(typed.replayClass),
      replayReasons: normalizeTags(typed.replayReasons),
      mlScoreSpace: normalizeText(typed.mlScoreSpace),
      mlScoreScope: normalizeText(typed.mlScoreScope),
      mlScoreContractSummary: normalizeText(typed.mlScoreContractSummary),
      mlScoreNote: normalizeText(typed.mlScoreNote),
      routeProvenance:
        (typed.routeProvenance as RouteProvenanceSummary | null) ?? null,
      lineage: (typed.lineage as ArtifactLineageSummary | null) ?? null,
      tags: normalizeTags(typed.tags),
    });
  }
  return rawArray(out);
}

function pruneSavedRunDetailCache(
  cache: Record<string, HistorySavedRunDetail>,
  savedRuns: HistorySavedRunEntry[],
): Record<string, HistorySavedRunDetail> {
  const allowed = new Set<string>();
  for (const entry of savedRuns) {
    const path = normalizePath(entry.path);
    if (!path) continue;
    allowed.add(path);
  }

  let removedAny = false;
  const nextCache: Record<string, HistorySavedRunDetail> = {};
  for (const [path, detail] of Object.entries(cache)) {
    if (!allowed.has(path)) {
      removedAny = true;
      continue;
    }
    nextCache[path] = detail;
  }
  return removedAny ? nextCache : cache;
}

function buildSavedRunComparisonTarget(
  source: string,
  entry: HistorySavedRunEntry,
  detail: HistorySavedRunDetail,
): RunComparisonTarget | null {
  const runReq =
    detail.runReq && typeof detail.runReq === "object" ? detail.runReq : null;
  const runRes =
    detail.runRes && typeof detail.runRes === "object" ? detail.runRes : null;
  if (!runReq || !runRes) return null;
  return {
    source,
    ts: normalizeText(entry.ts),
    strategyKind: normalizeText(detail.strategyKind || entry.strategyKind),
    csvPath: normalizePath(detail.csvPath || entry.csvPath),
    timeframe: normalizeText(detail.timeframe || entry.timeframe),
    reportPath: normalizePath(detail.reportPath || entry.reportXlsxPath),
    hasValidation: detail.hasValidation ?? entry.hasValidation,
    configFingerprint: normalizeText(
      detail.configFingerprint || entry.configFingerprint,
    ),
    datasetMeta:
      detail.datasetMeta && typeof detail.datasetMeta === "object"
        ? detail.datasetMeta
        : null,
    optimizationMetricSurface:
      detail.optimizationMetricSurface &&
      typeof detail.optimizationMetricSurface === "object"
        ? detail.optimizationMetricSurface
        : null,
    routeProvenance: detail.routeProvenance ?? entry.routeProvenance ?? null,
    lineage: detail.lineage ?? entry.lineage ?? null,
    req: runReq,
    res: runRes,
  };
}

function buildSavedRunCompareOptions(
  entries: HistorySavedRunEntry[],
  selectedPath: string,
): HistoryOption[] {
  const options: HistoryOption[] = [
    {
      value: "",
      label:
        entries.length > 1
          ? "Compare against another saved run..."
          : "Need another saved run",
      disabled: false,
    },
  ];
  for (const entry of entries) {
    if (entry.path === selectedPath) continue;
    options.push({
      value: entry.path,
      label: savedRunLabel(entry),
      disabled: false,
    });
  }
  return rawArray(options);
}

const overviewCompareOptionsCache = new WeakMap<
  HistorySavedRunEntry[],
  HistoryOption[]
>();

function buildOverviewCompareOptions(
  entries: HistorySavedRunEntry[],
): HistoryOption[] {
  const cached = overviewCompareOptionsCache.get(entries);
  if (cached) return cached;

  const options: HistoryOption[] = [
    {
      value: "",
      label: entries.length ? "Select a baseline..." : "No saved baselines yet",
      disabled: false,
    },
  ];
  for (const entry of entries) {
    const path = normalizePath(entry.path);
    const value = path
      ? `disk:${path}`
      : normalizeText(entry.ts)
        ? `hist:${normalizeText(entry.ts)}`
        : "";
    if (!value) continue;
    options.push({
      value,
      label: savedRunLabel(entry),
      disabled: false,
    });
  }
  const rawOptions = rawArray(options);
  overviewCompareOptionsCache.set(entries, rawOptions);
  return rawOptions;
}

function formatRepairSummary(result: HistoryRunArtifactRepairSummary): string {
  const changes = result.changes.length
    ? result.changes.join(" | ")
    : "No detailed repair changes reported.";
  const disposition = result.dryRun
    ? result.changed
      ? "Dry-run found repairable changes"
      : "Dry-run found no required changes"
    : result.wrote
      ? "Wrote repaired artifact copy"
      : result.changed
        ? "Repair found changes but did not write a copy"
        : "Artifact already canonical";
  const target = result.resultPath || result.sourcePath || "";
  return [disposition, target ? `target=${target}` : "", changes]
    .filter((part) => part.length > 0)
    .join(" · ");
}

function applyVueTextFieldUpdate(
  state: HistoryState,
  field: string,
  value: string,
): boolean {
  const key = String(field || "").trim();
  const next = String(value || "");
  if (!key) return false;
  switch (key) {
    case "savedRunsSearch":
      state.savedRunsSearch = next;
      return true;
    case "savedRunsTags":
      state.savedRunsTagsDraft = next;
      return true;
    default:
      return false;
  }
}

function applyVueSelectFieldUpdate(
  state: HistoryState,
  field: string,
  value: string,
): boolean {
  const key = String(field || "").trim();
  const next = String(value || "").trim();
  if (!key) return false;
  switch (key) {
    case "historySelect":
      state.sessionSelectedValue = next;
      return true;
    case "savedRunsSelect":
      state.savedSelectedValue = next;
      return true;
    case "compareSelect":
      state.compareSelectedValue = next;
      return true;
    case "savedRunCompareSelect":
      state.savedRunCompareSelected = next;
      return true;
    default:
      return false;
  }
}

export const useHistoryStore = defineStore("history", {
  state: (): HistoryState => ({
    sessionRunCount: 0,
    sessionRunsRaw: [],
    sessionOptions: [],
    sessionSelectedValue: "",
    sessionSelectedLabel: "No runs yet",
    savedRunCount: 0,
    savedRunsRaw: [],
    savedRunsVisible: [],
    savedRunDetailCache: {},
    savedRunDetailLoadingPath: "",
    savedRunDetailSeq: 0,
    savedOptions: [],
    savedSelectedValue: "",
    savedSelectedLabel: "No saved runs",
    compareRunCount: 0,
    compareOptions: [],
    compareSelectedValue: "",
    savedRunCompareOptions: [],
    savedRunCompareSelected: "",
    savedRunCompareSummary: "-",
    savedRunsSearch: "",
    savedRunsTagsDraft: "",
    savedRunsMeta: "—",
    savedRunDetailTitle: "No saved run selected",
    savedRunDetailMeta: "Select a run to inspect metadata and settings.",
    savedRunDetailStrategy: "—",
    savedRunDetailConfigFingerprint: "—",
    savedRunDetailDataset: "—",
    savedRunDetailTimeframe: "—",
    savedRunDetailValidation: "—",
    savedRunDetailReplay: "—",
    savedRunDetailCompatibility: "—",
    savedRunDetailProvenance: "—",
    savedRunDetailLineage: "—",
    savedRunDetailMetricSurface: "—",
    savedRunDetailMetricSurfaceDetail: "—",
    savedRunDetailAdmission: "—",
    savedRunDetailAdmissionDetail: "—",
    savedRunDetailReport: "—",
    savedRunDetailSize: "—",
    savedRunDetailTags: "—",
    savedRunDetailMlBundle: "—",
    savedRunDetailMlScoreCache: "—",
    savedRunDetailMlFeatureSchema: "—",
    savedRunDetailMlScoreContract: "—",
    savedRunDetailMlScoreNote: "—",
    savedRunDetailMlPolicy: "—",
    savedRunDetailMlSweep: "—",
    savedRunDetailMlVerdict: "—",
    savedRunDetailMlCandidates: "—",
    savedRunDetailParams: "Select a saved run to inspect parameters.",
    savedRunRepairSummary: "No artifact repair check has been run.",
    canApplyHistory: false,
    canExportPreset: true,
    canImportPreset: true,
    canClearHistory: false,
    canSaveSavedRunTags: false,
    canRefreshSavedRuns: true,
    canLoadSavedRun: false,
    canApplySavedRun: false,
    canRerunSavedRun: false,
    canOpenSavedRunReport: false,
    canRevealSavedRun: false,
    canDeleteSavedRun: false,
    canRepairSavedRun: false,
    canWriteRepairedSavedRun: false,
    canOpenRunsDir: true,
    statusLine: "Ready.",
    source: "unknown",
    updatedAtUnixMs: 0,
  }),

  actions: {
    refreshSessionHistory(preferredTs?: string): void {
      void loadHistoryService().then((service) => {
        const runs = service.readSessionHistory();
        const selectedTs = normalizeText(
          preferredTs ?? this.sessionSelectedValue,
        );
        const options: HistoryOption[] = [
          {
            value: "",
            label: runs.length ? "Select a run..." : "No runs yet",
            disabled: false,
          },
        ];
        for (const entry of runs) {
          options.push({
            value: normalizeText(entry.ts),
            label: sessionRunLabel(entry),
            disabled: false,
          });
        }
        const hasSelection =
          selectedTs &&
          runs.some((entry) => normalizeText(entry.ts) === selectedTs);
        this.sessionRunCount = runs.length;
        this.sessionRunsRaw = rawArray(runs);
        this.sessionOptions = rawArray(options);
        this.sessionSelectedValue = hasSelection ? selectedTs : "";
        this.sessionSelectedLabel = hasSelection
          ? options.find((entry) => entry.value === selectedTs)?.label ||
            "No runs yet"
          : "No runs yet";
        this.canApplyHistory = !!hasSelection;
        this.canClearHistory = runs.length > 0;
        this.source = "vue";
        this.updatedAtUnixMs = Date.now();
      });
    },

    applyLegacySnapshot(snapshot: Partial<HistoryState>): void {
      this.sessionRunCount = Math.max(
        0,
        Math.floor(
          Number(snapshot.sessionRunCount ?? this.sessionRunCount ?? 0),
        ),
      );
      this.sessionRunsRaw = sanitizeSessionRunsArray(
        snapshot.sessionRunsRaw ?? this.sessionRunsRaw,
      );
      this.sessionOptions = sanitizeOptionArray(
        snapshot.sessionOptions ?? this.sessionOptions,
      );
      this.sessionSelectedValue = String(
        snapshot.sessionSelectedValue ?? this.sessionSelectedValue ?? "",
      ).trim();
      this.sessionSelectedLabel =
        String(
          snapshot.sessionSelectedLabel ??
            this.sessionSelectedLabel ??
            "No runs yet",
        ).trim() || "No runs yet";
      this.savedRunCount = Math.max(
        0,
        Math.floor(Number(snapshot.savedRunCount ?? this.savedRunCount ?? 0)),
      );
      this.savedRunsRaw = sanitizeSavedRunsArray(
        snapshot.savedRunsRaw ?? this.savedRunsRaw,
      );
      this.savedRunsVisible = sanitizeSavedRunsArray(
        snapshot.savedRunsVisible ?? this.savedRunsVisible,
      );
      this.savedOptions = sanitizeOptionArray(
        snapshot.savedOptions ?? this.savedOptions,
      );
      this.savedSelectedValue = String(
        snapshot.savedSelectedValue ?? this.savedSelectedValue ?? "",
      ).trim();
      this.savedSelectedLabel =
        String(
          snapshot.savedSelectedLabel ??
            this.savedSelectedLabel ??
            "No saved runs",
        ).trim() || "No saved runs";
      this.compareRunCount = Math.max(
        0,
        Math.floor(
          Number(snapshot.compareRunCount ?? this.compareRunCount ?? 0),
        ),
      );
      this.compareOptions = sanitizeOptionArray(
        snapshot.compareOptions ?? this.compareOptions,
      );
      this.compareSelectedValue = String(
        snapshot.compareSelectedValue ?? this.compareSelectedValue ?? "",
      ).trim();
      this.savedRunCompareOptions = sanitizeOptionArray(
        snapshot.savedRunCompareOptions ?? this.savedRunCompareOptions,
      );
      this.savedRunCompareSelected = String(
        snapshot.savedRunCompareSelected ?? this.savedRunCompareSelected ?? "",
      ).trim();
      this.savedRunCompareSummary =
        String(
          snapshot.savedRunCompareSummary ?? this.savedRunCompareSummary ?? "-",
        ).trim() || "-";
      this.savedRunsSearch = String(
        snapshot.savedRunsSearch ?? this.savedRunsSearch ?? "",
      ).trim();
      this.savedRunsTagsDraft = String(
        snapshot.savedRunsTagsDraft ?? this.savedRunsTagsDraft ?? "",
      ).trim();
      this.savedRunsMeta =
        String(snapshot.savedRunsMeta ?? this.savedRunsMeta ?? "—").trim() ||
        "—";
      this.savedRunDetailTitle =
        String(
          snapshot.savedRunDetailTitle ??
            this.savedRunDetailTitle ??
            "No saved run selected",
        ).trim() || "No saved run selected";
      this.savedRunDetailMeta =
        String(
          snapshot.savedRunDetailMeta ??
            this.savedRunDetailMeta ??
            "Select a run to inspect metadata and settings.",
        ).trim() || "Select a run to inspect metadata and settings.";
      this.savedRunDetailStrategy =
        String(
          snapshot.savedRunDetailStrategy ?? this.savedRunDetailStrategy ?? "—",
        ).trim() || "—";
      this.savedRunDetailConfigFingerprint =
        String(
          snapshot.savedRunDetailConfigFingerprint ??
            this.savedRunDetailConfigFingerprint ??
            "—",
        ).trim() || "—";
      this.savedRunDetailDataset =
        String(
          snapshot.savedRunDetailDataset ?? this.savedRunDetailDataset ?? "—",
        ).trim() || "—";
      this.savedRunDetailTimeframe =
        String(
          snapshot.savedRunDetailTimeframe ??
            this.savedRunDetailTimeframe ??
            "—",
        ).trim() || "—";
      this.savedRunDetailValidation =
        String(
          snapshot.savedRunDetailValidation ??
            this.savedRunDetailValidation ??
            "—",
        ).trim() || "—";
      this.savedRunDetailReplay =
        String(
          snapshot.savedRunDetailReplay ?? this.savedRunDetailReplay ?? "—",
        ).trim() || "—";
      this.savedRunDetailCompatibility =
        String(
          snapshot.savedRunDetailCompatibility ??
            this.savedRunDetailCompatibility ??
            "—",
        ).trim() || "—";
      this.savedRunDetailProvenance =
        String(
          snapshot.savedRunDetailProvenance ??
            this.savedRunDetailProvenance ??
            "—",
        ).trim() || "—";
      this.savedRunDetailLineage =
        String(
          snapshot.savedRunDetailLineage ?? this.savedRunDetailLineage ?? "—",
        ).trim() || "—";
      this.savedRunDetailMetricSurface =
        String(
          snapshot.savedRunDetailMetricSurface ??
            this.savedRunDetailMetricSurface ??
            "—",
        ).trim() || "—";
      this.savedRunDetailMetricSurfaceDetail =
        String(
          snapshot.savedRunDetailMetricSurfaceDetail ??
            this.savedRunDetailMetricSurfaceDetail ??
            "—",
        ).trim() || "—";
      this.savedRunDetailAdmission =
        String(
          snapshot.savedRunDetailAdmission ??
            this.savedRunDetailAdmission ??
            "—",
        ).trim() || "—";
      this.savedRunDetailAdmissionDetail =
        String(
          snapshot.savedRunDetailAdmissionDetail ??
            this.savedRunDetailAdmissionDetail ??
            "—",
        ).trim() || "—";
      this.savedRunDetailReport =
        String(
          snapshot.savedRunDetailReport ?? this.savedRunDetailReport ?? "—",
        ).trim() || "—";
      this.savedRunDetailSize =
        String(
          snapshot.savedRunDetailSize ?? this.savedRunDetailSize ?? "—",
        ).trim() || "—";
      this.savedRunDetailTags =
        String(
          snapshot.savedRunDetailTags ?? this.savedRunDetailTags ?? "—",
        ).trim() || "—";
      this.savedRunDetailMlBundle =
        String(
          snapshot.savedRunDetailMlBundle ?? this.savedRunDetailMlBundle ?? "—",
        ).trim() || "—";
      this.savedRunDetailMlScoreCache =
        String(
          snapshot.savedRunDetailMlScoreCache ??
            this.savedRunDetailMlScoreCache ??
            "—",
        ).trim() || "—";
      this.savedRunDetailMlFeatureSchema =
        String(
          snapshot.savedRunDetailMlFeatureSchema ??
            this.savedRunDetailMlFeatureSchema ??
            "—",
        ).trim() || "—";
      this.savedRunDetailMlScoreContract =
        String(
          snapshot.savedRunDetailMlScoreContract ??
            this.savedRunDetailMlScoreContract ??
            "—",
        ).trim() || "—";
      this.savedRunDetailMlScoreNote =
        String(
          snapshot.savedRunDetailMlScoreNote ??
            this.savedRunDetailMlScoreNote ??
            "—",
        ).trim() || "—";
      this.savedRunDetailMlPolicy =
        String(
          snapshot.savedRunDetailMlPolicy ?? this.savedRunDetailMlPolicy ?? "—",
        ).trim() || "—";
      this.savedRunDetailMlSweep =
        String(
          snapshot.savedRunDetailMlSweep ?? this.savedRunDetailMlSweep ?? "—",
        ).trim() || "—";
      this.savedRunDetailMlVerdict =
        String(
          snapshot.savedRunDetailMlVerdict ??
            this.savedRunDetailMlVerdict ??
            "—",
        ).trim() || "—";
      this.savedRunDetailMlCandidates =
        String(
          snapshot.savedRunDetailMlCandidates ??
            this.savedRunDetailMlCandidates ??
            "—",
        ).trim() || "—";
      this.savedRunDetailParams =
        String(
          snapshot.savedRunDetailParams ??
            this.savedRunDetailParams ??
            "Select a saved run to inspect parameters.",
        ).trim() || "Select a saved run to inspect parameters.";
      this.savedRunRepairSummary =
        String(
          snapshot.savedRunRepairSummary ??
            this.savedRunRepairSummary ??
            "No artifact repair check has been run.",
        ).trim() || "No artifact repair check has been run.";
      this.canApplyHistory = Boolean(
        snapshot.canApplyHistory ?? this.canApplyHistory,
      );
      this.canExportPreset = Boolean(
        snapshot.canExportPreset ?? this.canExportPreset,
      );
      this.canImportPreset = Boolean(
        snapshot.canImportPreset ?? this.canImportPreset,
      );
      this.canClearHistory = Boolean(
        snapshot.canClearHistory ?? this.canClearHistory,
      );
      this.canSaveSavedRunTags = Boolean(
        snapshot.canSaveSavedRunTags ?? this.canSaveSavedRunTags,
      );
      this.canRefreshSavedRuns = Boolean(
        snapshot.canRefreshSavedRuns ?? this.canRefreshSavedRuns,
      );
      this.canLoadSavedRun = Boolean(
        snapshot.canLoadSavedRun ?? this.canLoadSavedRun,
      );
      this.canApplySavedRun = Boolean(
        snapshot.canApplySavedRun ?? this.canApplySavedRun,
      );
      this.canRerunSavedRun = Boolean(
        snapshot.canRerunSavedRun ?? this.canRerunSavedRun,
      );
      this.canOpenSavedRunReport = Boolean(
        snapshot.canOpenSavedRunReport ?? this.canOpenSavedRunReport,
      );
      this.canRevealSavedRun = Boolean(
        snapshot.canRevealSavedRun ?? this.canRevealSavedRun,
      );
      this.canDeleteSavedRun = Boolean(
        snapshot.canDeleteSavedRun ?? this.canDeleteSavedRun,
      );
      this.canRepairSavedRun = Boolean(
        snapshot.canRepairSavedRun ?? this.canRepairSavedRun,
      );
      this.canWriteRepairedSavedRun = Boolean(
        snapshot.canWriteRepairedSavedRun ?? this.canWriteRepairedSavedRun,
      );
      this.canOpenRunsDir = Boolean(
        snapshot.canOpenRunsDir ?? this.canOpenRunsDir,
      );
      this.statusLine =
        String(snapshot.statusLine ?? this.statusLine ?? "Ready.").trim() ||
        "Ready.";
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

    setSelectField(field: string, value: string): void {
      const key = String(field || "").trim();
      const next = String(value || "");
      if (applyVueSelectFieldUpdate(this, key, next)) {
        if (key === "savedRunCompareSelect") {
          this.syncSavedRunsView(this.savedSelectedValue);
        }
        this.source = "vue";
        this.updatedAtUnixMs = Date.now();
      }
    },

    syncSavedRunsView(preferredPath?: string): void {
      const filtered = filterSavedRunsEntries(
        this.savedRunsRaw,
        this.savedRunsSearch,
      );
      const options: HistoryOption[] = [
        {
          value: "",
          label: filtered.length
            ? "Select a saved run..."
            : "No saved runs yet",
          disabled: false,
        },
      ];
      for (const entry of filtered) {
        options.push({
          value: entry.path,
          label: savedRunLabel(entry),
          disabled: false,
        });
      }
      const filteredByPath = new Map(
        filtered.map((entry) => [entry.path, entry] as const),
      );

      const requestedPath = normalizePath(
        preferredPath ?? this.savedSelectedValue,
      );
      const selectedEntry = requestedPath
        ? (filteredByPath.get(requestedPath) ?? null)
        : null;
      const selectedPath = selectedEntry ? requestedPath : "";

      this.savedOptions = rawArray(options);
      this.savedRunsVisible = rawArray(filtered);
      this.savedRunCount = this.savedRunsRaw.length;
      this.compareOptions = buildOverviewCompareOptions(this.savedRunsRaw);
      this.compareRunCount = Math.max(0, this.compareOptions.length - 1);
      const validCompareValues = new Set(
        this.compareOptions
          .map((option) => normalizeText(option.value))
          .filter(Boolean),
      );
      if (!validCompareValues.has(normalizeText(this.compareSelectedValue))) {
        this.compareSelectedValue = "";
      }
      this.savedSelectedValue = selectedPath;
      this.savedSelectedLabel = selectedEntry
        ? savedRunLabel(selectedEntry)
        : "No saved runs";
      this.savedRunsTagsDraft = selectedEntry
        ? selectedEntry.tags.join(", ")
        : "";

      const selectedDetail = selectedEntry
        ? this.savedRunDetailCache[selectedEntry.path]
        : undefined;
      this.savedRunCompareOptions = buildSavedRunCompareOptions(
        filtered,
        selectedPath,
      );
      const requestedComparePath = normalizePath(this.savedRunCompareSelected);
      const compareEntry =
        requestedComparePath && requestedComparePath !== selectedPath
          ? (filteredByPath.get(requestedComparePath) ?? null)
          : null;
      const compareSelected = compareEntry ? requestedComparePath : "";
      this.savedRunCompareSelected = compareSelected;
      const compareDetail = compareEntry
        ? this.savedRunDetailCache[compareEntry.path]
        : undefined;

      if (!selectedEntry) {
        this.savedRunsMeta = `saved runs: ${filtered.length}/${this.savedRunsRaw.length}`;
        this.savedRunDetailTitle = "No saved run selected";
        this.savedRunDetailMeta =
          "Select a run to inspect metadata and settings.";
        this.savedRunDetailStrategy = "—";
        this.savedRunDetailConfigFingerprint = "—";
        this.savedRunDetailDataset = "—";
        this.savedRunDetailTimeframe = "—";
        this.savedRunDetailValidation = "—";
        this.savedRunDetailReplay = "—";
        this.savedRunDetailCompatibility = "—";
        this.savedRunDetailProvenance = "—";
        this.savedRunDetailLineage = "—";
        this.savedRunDetailMetricSurface = "—";
        this.savedRunDetailMetricSurfaceDetail = "—";
        this.savedRunDetailAdmission = "—";
        this.savedRunDetailAdmissionDetail = "—";
        this.savedRunDetailReport = "—";
        this.savedRunDetailSize = "—";
        this.savedRunDetailTags = "—";
        this.savedRunDetailMlBundle = "—";
        this.savedRunDetailMlScoreCache = "—";
        this.savedRunDetailMlFeatureSchema = "—";
        this.savedRunDetailMlScoreContract = "—";
        this.savedRunDetailMlScoreNote = "—";
        this.savedRunDetailMlPolicy = "—";
        this.savedRunDetailMlSweep = "—";
        this.savedRunDetailMlVerdict = "—";
        this.savedRunDetailMlCandidates = "—";
        this.savedRunDetailParams = "Select a saved run to inspect parameters.";
        this.savedRunRepairSummary = "No artifact repair check has been run.";
        this.savedRunCompareSummary = "-";
        this.savedRunDetailLoadingPath = "";
      } else {
        const strategyKind = normalizeText(
          selectedDetail?.strategyKind || selectedEntry.strategyKind,
        );
        const csvPath = normalizePath(
          selectedDetail?.csvPath || selectedEntry.csvPath,
        );
        const timeframe = normalizeText(
          selectedDetail?.timeframe || selectedEntry.timeframe,
        );
        const reportPath = normalizePath(
          selectedDetail?.reportPath || selectedEntry.reportXlsxPath,
        );
        const hasValidation = selectedDetail
          ? !!selectedDetail.hasValidation
          : selectedEntry.hasValidation;
        const tags = selectedDetail
          ? normalizeTags(selectedDetail.tags)
          : normalizeTags(selectedEntry.tags);
        const mlBundleId = normalizeText(selectedDetail?.mlBundleId);
        const mlScoreCachePath = normalizePath(
          selectedDetail?.mlScoreCachePath || "",
        );
        const mlFeatureSchemaPath = normalizePath(
          selectedDetail?.mlFeatureSchemaPath || "",
        );
        const mlScoreContractSummary = normalizeText(
          selectedDetail?.mlScoreContractSummary ??
            selectedEntry.mlScoreContractSummary,
        );
        const mlScoreNote = normalizeText(
          selectedDetail?.mlScoreNote ?? selectedEntry.mlScoreNote,
        );
        const mlPolicySummary = normalizeText(selectedDetail?.mlPolicySummary);
        const mlPolicySweepSummary = normalizeText(
          selectedDetail?.mlPolicySweepSummary,
        );
        const configFingerprint = normalizeText(
          selectedDetail?.configFingerprint || selectedEntry.configFingerprint,
        );
        const provenanceText = formatRouteProvenanceCompact(
          selectedDetail?.routeProvenance ??
            selectedEntry.routeProvenance ??
            null,
        );
        const lineageText = formatArtifactLineageCompact(
          selectedDetail?.lineage ?? selectedEntry.lineage ?? null,
        );
        const reportState = resolveReportState(
          reportPath,
          selectedDetail?.reportPathExists ?? selectedEntry.reportXlsxExists,
        );
        const replaySummary = replayClassSummary(selectedEntry, selectedDetail);
        const artifactCompatibility =
          selectedDetail?.artifactCompatibility ??
          selectedEntry.artifactCompatibility ??
          null;
        const compatibilitySummary = artifactCompatibilitySummary(
          artifactCompatibility,
        );
        const compatibilityMetaToken = artifactCompatibilityMetaToken(
          artifactCompatibility,
        );

        const parts: string[] = [];
        if (selectedEntry.ts) parts.push(`ts=${selectedEntry.ts}`);
        if (strategyKind) parts.push(`strategy=${strategyKind}`);
        if (hasValidation) parts.push("validation=yes");
        if (compatibilityMetaToken) parts.push(compatibilityMetaToken);
        if (replaySummary !== "-") {
          parts.push(
            `replay=${replayClassLabel(selectedDetail?.replayClass || selectedEntry.replayClass)}`,
          );
        }
        if (strategyKind === "MlSignal") parts.push("ml/cache");
        if (mlScoreContractSummary)
          parts.push(`score=${mlScoreContractSummary}`);
        const reportMetaToken = formatReportStateMetaToken(reportState);
        if (reportMetaToken) parts.push(reportMetaToken);
        if (csvPath) parts.push(`csv=${csvPath}`);
        if (tags.length) parts.push(`tags=${tags.join(",")}`);
        if (selectedEntry.bytes > 0) parts.push(`bytes=${selectedEntry.bytes}`);
        this.savedRunsMeta = parts.join(" • ");
        this.savedRunDetailTitle = selectedEntry.ts || "Selected run";
        this.savedRunDetailMeta = `artifact: ${fileBasename(selectedEntry.path) || selectedEntry.path || "unknown"}`;
        this.savedRunDetailStrategy = strategyKind || "—";
        this.savedRunDetailConfigFingerprint = configFingerprint || "—";
        this.savedRunDetailDataset = fileBasename(csvPath) || csvPath || "—";
        this.savedRunDetailTimeframe = timeframe || "—";
        this.savedRunDetailValidation = hasValidation
          ? "Available"
          : "Not saved";
        this.savedRunDetailReplay = replaySummary;
        this.savedRunDetailCompatibility = compatibilitySummary || "—";
        this.savedRunDetailProvenance = provenanceText || "—";
        this.savedRunDetailLineage = lineageText || "—";
        this.savedRunDetailMetricSurface =
          normalizeText(selectedDetail?.metricSurfaceSummary) || "—";
        this.savedRunDetailMetricSurfaceDetail =
          normalizeText(selectedDetail?.metricSurfaceDetail) || "—";
        this.savedRunDetailAdmission =
          normalizeText(selectedDetail?.validationAdmissionSummary) || "—";
        this.savedRunDetailAdmissionDetail =
          normalizeText(selectedDetail?.validationAdmissionDetail) || "—";
        this.savedRunDetailReport = formatReportStateLabel(reportState);
        this.savedRunDetailSize =
          formatBytesCompact(selectedEntry.bytes) || "—";
        this.savedRunDetailTags = tags.length ? tags.join(", ") : "—";
        this.savedRunDetailMlBundle = mlBundleId || "—";
        this.savedRunDetailMlScoreCache = mlScoreCachePath
          ? fileBasename(mlScoreCachePath) || mlScoreCachePath
          : "—";
        this.savedRunDetailMlFeatureSchema = mlFeatureSchemaPath
          ? fileBasename(mlFeatureSchemaPath) || mlFeatureSchemaPath
          : "—";
        this.savedRunDetailMlScoreContract = mlScoreContractSummary || "—";
        this.savedRunDetailMlScoreNote = mlScoreNote || "—";
        this.savedRunDetailMlPolicy = mlPolicySummary || "—";
        this.savedRunDetailMlSweep = mlPolicySweepSummary || "—";
        this.savedRunDetailMlVerdict =
          normalizeText(selectedDetail?.mlRunVerdictSummary) || "—";
        this.savedRunDetailMlCandidates =
          normalizeText(selectedDetail?.mlTopCandidatesSummary) || "—";
        this.savedRunDetailParams =
          normalizeText(selectedDetail?.paramsText) ||
          "Loading parameter snapshot...";

        const selectedTarget =
          selectedDetail &&
          buildSavedRunComparisonTarget("saved", selectedEntry, selectedDetail);
        const compareTarget =
          compareEntry &&
          compareDetail &&
          buildSavedRunComparisonTarget("saved", compareEntry, compareDetail);
        if (!compareSelected) {
          this.savedRunCompareSummary =
            filtered.length > 1
              ? "Select another saved run to compare against the selected artifact."
              : "Need another saved run before compare is available.";
        } else if (!compareEntry) {
          this.savedRunCompareSummary =
            "Comparison run not found. Refresh saved runs or choose another artifact.";
        } else if (!selectedDetail || !compareDetail) {
          this.savedRunCompareSummary = "Loading compare...";
        } else if (!selectedTarget || !compareTarget) {
          this.savedRunCompareSummary =
            "Compare unavailable for this artifact because request/result data is missing.";
        } else {
          this.savedRunCompareSummary = buildRunCompareSummary(
            selectedTarget,
            compareTarget,
          );
        }
      }

      const canMutate = !runtimeRunning();
      const hasSelection = !!selectedEntry;
      const artifactReplayCompatible =
        hasSelection &&
        artifactCompatibilityAllowsReplay(
          selectedDetail?.artifactCompatibility ??
            selectedEntry?.artifactCompatibility ??
            null,
        );
      const replayClassAllowsRerun =
        normalizeText(
          selectedDetail?.replayClass || selectedEntry?.replayClass,
        ) !== "blocked";
      this.canSaveSavedRunTags = canMutate && hasSelection;
      this.canRefreshSavedRuns = canMutate;
      this.canLoadSavedRun = canMutate && artifactReplayCompatible;
      this.canApplySavedRun = canMutate && artifactReplayCompatible;
      this.canRerunSavedRun =
        canMutate && artifactReplayCompatible && replayClassAllowsRerun;
      this.canOpenSavedRunReport =
        canMutate &&
        canOpenPersistedReport(
          selectedDetail?.reportPath || selectedEntry?.reportXlsxPath || "",
          selectedDetail?.reportPathExists ?? selectedEntry?.reportXlsxExists,
        );
      this.canRevealSavedRun = canMutate && hasSelection;
      this.canDeleteSavedRun = canMutate && hasSelection;
      this.canRepairSavedRun = canMutate && hasSelection;
      this.canWriteRepairedSavedRun = canMutate && hasSelection;
      this.canOpenRunsDir = canMutate;

      if (selectedEntry && !selectedDetail) {
        this.loadSavedRunDetail(selectedEntry.path);
      }
      if (compareEntry && !compareDetail) {
        this.loadSavedRunDetail(compareEntry.path);
      }
    },

    loadSavedRunDetail(path: string): void {
      const nextPath = normalizePath(path);
      if (!nextPath) return;
      if (this.savedRunDetailLoadingPath === nextPath) return;

      this.savedRunDetailSeq =
        Math.max(0, Math.floor(Number(this.savedRunDetailSeq || 0))) + 1;
      this.savedRunDetailLoadingPath = nextPath;

      void loadHistoryService()
        .then((service) => service.loadSavedRunDetail(nextPath))
        .then((detail) => {
          this.savedRunDetailCache[nextPath] = {
            strategyKind: normalizeText(detail?.strategyKind),
            configFingerprint: normalizeText(detail?.configFingerprint || ""),
            configFingerprintVersion: Number.isFinite(
              Number(detail?.configFingerprintVersion),
            )
              ? Math.floor(Number(detail?.configFingerprintVersion))
              : null,
            timeframe: normalizeText(detail?.timeframe),
            csvPath: normalizePath(detail?.csvPath || ""),
            reportPath: normalizePath(detail?.reportPath || ""),
            hasValidation: !!detail?.hasValidation,
            datasetMeta:
              detail?.datasetMeta && typeof detail.datasetMeta === "object"
                ? detail.datasetMeta
                : null,
            routeProvenance: detail?.routeProvenance ?? null,
            lineage: detail?.lineage ?? null,
            optimizationMetricSurface:
              detail?.optimizationMetricSurface &&
              typeof detail.optimizationMetricSurface === "object"
                ? detail.optimizationMetricSurface
                : null,
            runReq:
              detail?.runReq && typeof detail.runReq === "object"
                ? detail.runReq
                : null,
            runRes:
              detail?.runRes && typeof detail.runRes === "object"
                ? detail.runRes
                : null,
            metricSurfaceSummary: normalizeText(
              detail?.metricSurfaceSummary || "",
            ),
            metricSurfaceDetail: normalizeText(
              detail?.metricSurfaceDetail || "",
            ),
            validationAdmissionSummary: normalizeText(
              detail?.validationAdmissionSummary || "",
            ),
            validationAdmissionDetail: normalizeText(
              detail?.validationAdmissionDetail || "",
            ),
            tags: normalizeTags(detail?.tags),
            mlBundleId: normalizeText(detail?.mlBundleId || ""),
            mlScoreCachePath: normalizePath(detail?.mlScoreCachePath || ""),
            mlScoreManifestPath: normalizePath(
              detail?.mlScoreManifestPath || "",
            ),
            mlFeatureSchemaPath: normalizePath(
              detail?.mlFeatureSchemaPath || "",
            ),
            mlFeatureSchemaVersion: normalizeText(
              detail?.mlFeatureSchemaVersion || "",
            ),
            mlScoreSpace: normalizeText(detail?.mlScoreSpace || ""),
            mlScoreScope: normalizeText(detail?.mlScoreScope || ""),
            mlScoreContractSummary: normalizeText(
              detail?.mlScoreContractSummary || "",
            ),
            mlScoreNote: normalizeText(detail?.mlScoreNote || ""),
            mlPolicySummary: normalizeText(detail?.mlPolicySummary || ""),
            mlPolicySweepSummary: normalizeText(
              detail?.mlPolicySweepSummary || "",
            ),
            paramsText:
              normalizeText(detail?.paramsText) || "Request unavailable.",
          };
          if (
            this.savedSelectedValue === nextPath ||
            this.savedRunCompareSelected === nextPath
          ) {
            this.syncSavedRunsView(this.savedSelectedValue);
            this.source = "vue";
            this.updatedAtUnixMs = Date.now();
          }
        })
        .catch((error: unknown) => {
          if (
            this.savedSelectedValue !== nextPath &&
            this.savedRunCompareSelected !== nextPath
          )
            return;
          if (this.savedSelectedValue === nextPath) {
            this.savedRunDetailParams =
              `Failed to load artifact:\n${storeErrorText(error)}`.trim();
          }
          if (this.savedRunCompareSelected === nextPath) {
            this.savedRunCompareSummary =
              `Compare failed: ${storeErrorText(error)}`.trim();
          }
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        })
        .finally(() => {
          if (this.savedRunDetailLoadingPath === nextPath) {
            this.savedRunDetailLoadingPath = "";
          }
        });
    },

    setSavedRunsSearch(value: string): void {
      this.savedRunsSearch = String(value || "");
      this.syncSavedRunsView(this.savedSelectedValue);
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();
    },

    setSavedRunsTagsDraft(value: string): void {
      this.savedRunsTagsDraft = String(value || "");
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();
    },

    selectSavedRun(path: string): void {
      this.syncSavedRunsView(path);
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();
    },

    applySession(): void {
      const selectedTs = normalizePath(this.sessionSelectedValue);
      void loadHistoryService()
        .then((service) => service.applySession(selectedTs))
        .then(() => {
          this.statusLine = "Applied history run settings to the form.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        })
        .catch((error: unknown) => {
          this.statusLine =
            storeErrorText(error) || "Failed to apply history run.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    exportPreset(): void {
      void loadHistoryService()
        .then((service) => service.exportPreset())
        .then(() => {
          this.statusLine = "Exported preset.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        })
        .catch((error: unknown) => {
          this.statusLine =
            storeErrorText(error) || "Failed to export preset.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    importPreset(): void {
      void loadHistoryService()
        .then((service) => service.importPreset())
        .then(() => {
          this.statusLine = "Imported preset.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        })
        .catch((error: unknown) => {
          this.statusLine =
            storeErrorText(error) || "Failed to import preset.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    clearSession(): void {
      void loadHistoryService()
        .then((service) => service.clearSession())
        .then(() => {
          this.refreshSessionHistory("");
          this.compareSelectedValue = "";
          this.statusLine = "History cleared.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        })
        .catch((error: unknown) => {
          this.statusLine =
            storeErrorText(error) || "Failed to clear history.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    saveSavedRunTags(path: string, tags: string): void {
      const nextPath = normalizePath(path || this.savedSelectedValue);
      if (!nextPath) return;
      const nextTags = String(tags || this.savedRunsTagsDraft || "");
      this.savedRunsTagsDraft = nextTags;
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();

      void loadHistoryService()
        .then((service) =>
          service
            .saveSavedRunTags(nextPath, nextTags)
            .then(() => service.refreshSavedRuns()),
        )
        .then((summary) => {
          this.savedRunsRaw = sanitizeSavedRunsArray(summary?.runs);
          this.savedRunDetailCache = pruneSavedRunDetailCache(
            this.savedRunDetailCache,
            this.savedRunsRaw,
          );
          this.syncSavedRunsView(nextPath);
          this.statusLine = "Saved tags.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        })
        .catch((error: unknown) => {
          this.statusLine =
            storeErrorText(error) || "Failed to save tags.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    refreshSavedRuns(): void {
      if (!this.canRefreshSavedRuns) return;
      const preferredPath = normalizePath(this.savedSelectedValue);

      void loadHistoryService()
        .then((service) => service.refreshSavedRuns())
        .then((summary) => {
          this.savedRunsRaw = sanitizeSavedRunsArray(summary?.runs);
          this.savedRunDetailCache = pruneSavedRunDetailCache(
            this.savedRunDetailCache,
            this.savedRunsRaw,
          );
          this.syncSavedRunsView(preferredPath);
          this.statusLine = "Refreshed saved runs.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        })
        .catch((error: unknown) => {
          this.statusLine =
            storeErrorText(error) || "Failed to refresh saved runs.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    openRunsDir(): void {
      if (!this.canOpenRunsDir) return;
      void loadHistoryService()
        .then((service) => service.openRunsDir())
        .catch((error: unknown) => {
          this.statusLine =
            storeErrorText(error) || "Failed to open runs directory.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    loadSavedRun(path: string): void {
      const nextPath = normalizePath(path || this.savedSelectedValue);
      if (!nextPath) return;
      void loadHistoryService()
        .then((service) => service.loadSavedRun(nextPath))
        .catch((error: unknown) => {
          this.statusLine =
            storeErrorText(error) || "Failed to load saved run.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    applySavedRun(path: string): void {
      const nextPath = normalizePath(path || this.savedSelectedValue);
      if (!nextPath) return;
      void loadHistoryService()
        .then((service) => service.applySavedRun(nextPath))
        .catch((error: unknown) => {
          this.statusLine =
            storeErrorText(error) || "Failed to apply saved run.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    rerunSavedRun(path: string): void {
      const nextPath = normalizePath(path || this.savedSelectedValue);
      if (!nextPath) return;
      void loadHistoryService()
        .then((service) => service.rerunSavedRun(nextPath))
        .catch((error: unknown) => {
          this.statusLine =
            storeErrorText(error) || "Failed to rerun saved run.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    openSavedRunReport(path: string): void {
      const nextPath = normalizePath(path || this.savedSelectedValue);
      if (!nextPath) return;
      void loadHistoryService()
        .then((service) => service.openSavedRunReport(nextPath))
        .catch((error: unknown) => {
          this.statusLine =
            storeErrorText(error) || "Failed to open saved run report.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    revealSavedRun(path: string): void {
      const nextPath = normalizePath(path || this.savedSelectedValue);
      if (!nextPath) return;
      void loadHistoryService()
        .then((service) => service.revealSavedRun(nextPath))
        .catch((error: unknown) => {
          this.statusLine =
            storeErrorText(error) || "Failed to reveal saved run.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    repairSavedRun(path: string, dryRun = true): void {
      const nextPath = normalizePath(path || this.savedSelectedValue);
      if (!nextPath) return;
      const preferredPath = this.savedSelectedValue;
      this.statusLine = dryRun
        ? "Checking saved run artifact repair scope..."
        : "Writing repaired saved run artifact copy...";
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();

      void loadHistoryService()
        .then((service) =>
          service.repairSavedRun(nextPath, { dryRun }).then((result) => ({
            result,
            refresh: dryRun ? null : service.refreshSavedRuns(),
          })),
        )
        .then(async ({ result, refresh }) => {
          this.savedRunRepairSummary = formatRepairSummary(result);
          if (refresh) {
            const summary = await refresh;
            this.savedRunsRaw = sanitizeSavedRunsArray(summary?.runs);
            this.savedRunDetailCache = pruneSavedRunDetailCache(
              this.savedRunDetailCache,
              this.savedRunsRaw,
            );
            this.syncSavedRunsView(result.resultPath || preferredPath);
          }
          this.statusLine = dryRun
            ? "Artifact repair dry-run complete."
            : "Repaired artifact copy complete.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        })
        .catch((error: unknown) => {
          const message =
            storeErrorText(error) ||
            "Failed to repair saved run artifact.";
          this.savedRunRepairSummary = message;
          this.statusLine = message;
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },

    deleteSavedRun(path: string): void {
      const nextPath = normalizePath(path || this.savedSelectedValue);
      if (!nextPath) return;

      void loadHistoryService()
        .then((service) =>
          service
            .deleteSavedRun(nextPath)
            .then(() => service.refreshSavedRuns()),
        )
        .then((summary) => {
          this.savedRunsRaw = sanitizeSavedRunsArray(summary?.runs);
          this.savedRunDetailCache = pruneSavedRunDetailCache(
            this.savedRunDetailCache,
            this.savedRunsRaw,
          );
          const preferred =
            this.savedSelectedValue === nextPath ? "" : this.savedSelectedValue;
          this.syncSavedRunsView(preferred);
          this.statusLine = "Deleted saved run.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        })
        .catch((error: unknown) => {
          this.statusLine =
            storeErrorText(error) || "Failed to delete saved run.";
          this.source = "vue";
          this.updatedAtUnixMs = Date.now();
        });
    },
  },
});
