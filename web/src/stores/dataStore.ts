import { defineStore, type Pinia } from "pinia";
import {
  applyTimeframe as applyTimeframeAction,
  browseCsv as browseCsvAction,
  browseMarketDataDir as browseMarketDataDirAction,
  clearPolygonApiKey as clearPolygonApiKeyAction,
  clearData as clearDataAction,
  closePolygon as closePolygonAction,
  downloadPolygonDataset as downloadPolygonDatasetAction,
  getPolygonCacheInfo as getPolygonCacheInfoAction,
  type DataLoadResult,
  getMarketDataDirInfo as getMarketDataDirInfoAction,
  listSampleCsvs as listSampleCsvsAction,
  loadCsv as loadCsvAction,
  loadSampleCsv as loadSampleCsvAction,
  type MarketDataDirInfoSummary,
  openMarketDataDir as openMarketDataDirAction,
  openPolygon as openPolygonAction,
  type PolygonCacheInfoSummary,
  type PolygonCredentialStatusSummary,
  type PolygonDatasetSummary,
  repairPolygonDataset as repairPolygonDatasetAction,
  resetMarketDataDir as resetMarketDataDirAction,
  setPolygonApiKey as setPolygonApiKeyAction,
  setMarketDataDir as setMarketDataDirAction,
  topOffPolygonDataset as topOffPolygonDatasetAction,
  validatePolygonDataset as validatePolygonDatasetAction,
} from "../services/dataService";
import { DATA_STARTUP_SNAPSHOT_KEY } from "../utils/core/storageKeys";
import { redactKnownSecrets } from "../utils/redaction";
import { readStorageJson, writeStorageJson } from "../utils/storage";
import { formatUtcDateFromUnixMs } from "../utils/uiFormat";
import { resolveRuntimePinia } from "./runtimePinia";
import { useWorkflowArtifactsStore } from "./workflowArtifactsStore";
import { useWorkflowStore } from "./workflowStore";

export interface DataSampleOption {
  label: string;
  path: string;
  disabled: boolean;
}

type DataSourceKind = "csv" | "polygon";

interface DataState {
  activeControlsTab: string;
  csvPath: string;
  recentCsvPaths: string[];
  sampleCsvOptions: DataSampleOption[];
  sampleCsvVisible: boolean;
  marketDataDir: string;
  marketDataMoveExisting: boolean;
  marketDataDirHint: string;
  dataId: string;
  datasetLabel: string;
  datasetRange: string;
  dataLen: string;
  dataTimeframe: string;
  dataTimeframeHint: string;
  autoRestoreDataset: boolean;
  datasetContext: string;
  dataFields: string[];
  dataFieldHint: string;
  canBrowseCsv: boolean;
  canPolygonOpen: boolean;
  canLoadCsv: boolean;
  canClearData: boolean;
  canBrowseMarketDataDir: boolean;
  canSetMarketDataDir: boolean;
  canResetMarketDataDir: boolean;
  canOpenMarketDataDir: boolean;
  canApplyTimeframe: boolean;
  polygonOpen: boolean;
  polygonApiKey: string;
  polygonMarket: string;
  polygonSymbol: string;
  polygonFrom: string;
  polygonTo: string;
  polygonTimeframe: string;
  polygonAdjusted: boolean;
  polygonCacheDir: string;
  polygonHasApiKey: boolean;
  polygonCredentialStatus: PolygonCredentialStatusSummary;
  polygonDatasets: PolygonDatasetSummary[];
  polygonSelectedPath: string;
  polygonSelectedMeta: string;
  polygonCacheMeta: string;
  canPolygonRefresh: boolean;
  canPolygonSaveApiKey: boolean;
  canPolygonClearApiKey: boolean;
  canPolygonDownload: boolean;
  canPolygonLoadSelected: boolean;
  canPolygonValidateSelected: boolean;
  canPolygonRepairSelected: boolean;
  statusLine: string;
  baseDataId: string;
  baseDataMeta: DataLoadResult["meta"] | null;
  baseCsvPath: string;
  baseSourceKind: DataSourceKind;
  baseSourceSizeBytes: number | null;
  baseSourceMtimeUtcMs: number | null;
  baseDatasetLabel: string;
  baseDatasetContext: string;
  startupHydratedOnce: boolean;
  startupDeferredHydratedOnce: boolean;
  polygonCacheLoadedOnce: boolean;
  source: "vue" | "unknown";
  updatedAtUnixMs: number;
}

interface DataStartupSnapshot {
  autoRestoreDataset: boolean;
  baseCsvPath: string;
  selectedTimeframe: string;
  source: DataSourceKind;
  baseSourceSizeBytes: number | null;
  baseSourceMtimeUtcMs: number | null;
  savedAtUnixMs: number;
}

const POLYGON_SELECTED_PROMPT =
  "Select a cached Polygon dataset to inspect, validate, repair, or load.";
const POLYGON_CACHE_UNAVAILABLE_TEXT = "Polygon cache unavailable.";
const STARTUP_RESTORE_PENDING_STATUS = "Startup restore pending.";

let startupHydratePromise: Promise<void> | null = null;
let startupDeferredHydratePromise: Promise<void> | null = null;
let polygonCacheLoadPromise: Promise<void> | null = null;

function fileBasename(path: string): string {
  const normalized = String(path || "")
    .trim()
    .replace(/\\/g, "/");
  if (!normalized) return "";
  const parts = normalized.split("/");
  return String(parts[parts.length - 1] || "").trim();
}

function normalizeUnixToDateText(value: number): string {
  const ts = Number(value);
  if (!Number.isFinite(ts) || ts <= 0) return "—";
  const ms = ts >= 1_000_000_000_000 ? ts : ts * 1000;
  return formatUtcDateFromUnixMs(ms);
}

const RELATIVE_TIME_AXIS_EPOCH_CUTOFF_MS = 315_532_800_000;

function isRelativeTimeAxis(meta: DataLoadResult["meta"] | null): boolean {
  const row = meta && typeof meta === "object" ? meta : null;
  if (!row) return false;
  const minAbs = Math.abs(Number(row.minTs) || 0);
  const maxAbs = Math.abs(Number(row.maxTs) || 0);
  const largest = Math.max(minAbs, maxAbs);
  return largest > 0 && largest < RELATIVE_TIME_AXIS_EPOCH_CUTOFF_MS;
}

function formatRelativeDurationText(valueMs: number): string {
  const ms = Math.max(0, Math.floor(Number(valueMs) || 0));
  if (ms < 1_000) return `${ms}ms`;

  const totalSeconds = Math.floor(ms / 1_000);
  const days = Math.floor(totalSeconds / 86_400);
  const hours = Math.floor((totalSeconds % 86_400) / 3_600);
  const minutes = Math.floor((totalSeconds % 3_600) / 60);
  const seconds = totalSeconds % 60;
  const parts: string[] = [];
  if (days > 0) parts.push(`${days}d`);
  if (hours > 0 || parts.length > 0) parts.push(`${hours}h`);
  if (minutes > 0 || parts.length > 0) parts.push(`${minutes}m`);
  if (seconds > 0 || parts.length === 0) parts.push(`${seconds}s`);
  return parts.join(" ");
}

function datasetRangeText(
  meta: DataLoadResult["meta"] | null,
  timeframe: string,
): string {
  const row = meta && typeof meta === "object" ? meta : null;
  if (!row) return "—";
  const tf = String(timeframe || row.timeframe || "").trim();
  const parts: string[] = [];
  if (tf) parts.push(tf);
  if (isRelativeTimeAxis(row)) {
    parts.push(
      `relative ${formatRelativeDurationText(Number(row.minTs))} → ${formatRelativeDurationText(Number(row.maxTs))}`,
    );
    return parts.join(" • ");
  }
  const minTs = normalizeUnixToDateText(Number(row.minTs));
  const maxTs = normalizeUnixToDateText(Number(row.maxTs));
  if (minTs !== "—" || maxTs !== "—") parts.push(`${minTs} → ${maxTs}`);
  return parts.length ? parts.join(" • ") : "—";
}

function loadSourceText(
  loadSource: DataLoadResult["loadSource"] | string,
): string {
  switch (String(loadSource || "").trim()) {
    case "binary_cache":
      return "binary cache";
    case "csv_parsed_and_cached":
      return "csv parsed + cached";
    case "csv_parsed_cache_write_skipped":
      return "csv parsed (cache write skipped)";
    case "in_memory_base_reuse":
      return "in-memory base reuse";
    case "derived_fresh_resample":
      return "fresh resample";
    case "derived_in_memory_reuse":
      return "in-memory derived reuse";
    default:
      return "load path unknown";
  }
}

function baseLoadStatusText(
  result: DataLoadResult,
  source: DataSourceKind,
): string {
  const subject = source === "polygon" ? "Loaded Polygon dataset" : "Loaded";
  return `${subject} (${loadSourceText(result.loadSource)}).`;
}

function derivedLoadStatusText(result: DataLoadResult): string {
  const effective = String(result.meta?.timeframe || "").trim();
  const prefix = effective ? `Using ${effective}` : "Using derived timeframe";
  return `${prefix} (${loadSourceText(result.loadSource)}).`;
}

function datasetContextText(
  path: string,
  source: DataSourceKind,
  loadSource = "",
  cachePath = "",
): string {
  const parts: string[] = [];
  const src = String(source || "").trim();
  if (src) parts.push(`source=${src}`);
  const name = fileBasename(path);
  if (name) parts.push(`file=${name}`);
  const load = String(loadSource || "").trim();
  if (load) parts.push(`load=${load}`);
  const cacheName = fileBasename(cachePath);
  if (cacheName) parts.push(`cache=${cacheName}`);
  return parts.length ? parts.join(" • ") : "—";
}

function dataFieldList(meta: DataLoadResult["meta"] | null): string[] {
  const row = meta && typeof meta === "object" ? meta : null;
  if (!row) return [];
  const fields: string[] = [];
  if (row.hasOpen) fields.push("open");
  if (row.hasHigh) fields.push("high");
  if (row.hasLow) fields.push("low");
  if (row.hasClose) fields.push("close");
  if (row.hasVolume) fields.push("volume");
  return fields;
}

function dataFieldHint(meta: DataLoadResult["meta"] | null): string {
  const row = meta && typeof meta === "object" ? meta : null;
  if (!row) return "";
  const msgs: string[] = [];
  if (!row.hasClose) msgs.push("Missing close (required).");
  if (!row.hasVolume) msgs.push("No volume -> VWMA/VPWMA disabled.");
  if (!row.hasHigh || !row.hasLow)
    msgs.push("No high/low -> FRAMA + some sources disabled.");
  if (!row.hasOpen) msgs.push("No open -> open/ohlc4 disabled.");
  return msgs.join(" · ");
}

function hasLoadedDataset(dataId: string): boolean {
  const value = String(dataId || "").trim();
  return value.length > 0 && value !== "—" && value !== "-";
}

function timeframeHintText(
  baseTimeframe: string,
  currentTimeframe: string,
  hasBase: boolean,
  relativeTimeAxis = false,
): string {
  if (!hasBase) return "Load a dataset to enable timeframe resampling.";
  const baseLabel = String(baseTimeframe || "").trim() || "unknown";
  const effective = String(currentTimeframe || "").trim() || baseLabel;
  const derived = !!baseTimeframe && !!effective && baseLabel !== effective;
  const axisNote = relativeTimeAxis
    ? " Relative timeline detected; calendar dates are unavailable."
    : "";
  if (derived) {
    return `Using ${effective} derived from base ${baseLabel} (no extra downloads). Periods are in candles.${axisNote}`;
  }
  return `Base timeframe: ${baseLabel}. Choose a higher timeframe to derive candles (no extra downloads). Periods are in candles.${axisNote}`;
}

function markVueUpdate(state: DataState): void {
  state.source = "vue";
  state.updatedAtUnixMs = Date.now();
}

function setControlsBusy(state: DataState, busy: boolean): void {
  const canMutate = !busy;
  state.canBrowseCsv = canMutate;
  state.canPolygonOpen = canMutate;
  state.canLoadCsv = canMutate;
  state.canClearData = canMutate;
  state.canBrowseMarketDataDir = canMutate;
  state.canSetMarketDataDir = canMutate;
  state.canResetMarketDataDir = canMutate;
  state.canOpenMarketDataDir = canMutate;
  state.canApplyTimeframe =
    canMutate && hasLoadedDataset(state.baseDataId || state.dataId);
  state.canPolygonRefresh = canMutate;
  state.canPolygonSaveApiKey =
    canMutate && !!String(state.polygonApiKey || "").trim();
  state.canPolygonClearApiKey = canMutate && state.polygonHasApiKey;
  state.canPolygonDownload =
    canMutate &&
    state.polygonHasApiKey &&
    !!String(state.polygonSymbol || "").trim() &&
    !!String(state.polygonFrom || "").trim() &&
    !!String(state.polygonTo || "").trim();
  const hasSelectedPolygon = !!String(state.polygonSelectedPath || "").trim();
  state.canPolygonLoadSelected = canMutate && hasSelectedPolygon;
  state.canPolygonValidateSelected = canMutate && hasSelectedPolygon;
  state.canPolygonRepairSelected = canMutate && hasSelectedPolygon;
}

function pushRecentCsvPath(recent: string[], path: string): string[] {
  const nextPath = String(path || "").trim();
  if (!nextPath) return recent;
  const out = [nextPath];
  for (const item of recent) {
    const normalized = String(item || "").trim();
    if (!normalized || normalized === nextPath) continue;
    out.push(normalized);
    if (out.length >= 8) break;
  }
  return out;
}

function normalizedList(values: unknown): string[] {
  if (!Array.isArray(values)) return [];
  return values
    .map((value) => String(value || "").trim())
    .filter((value) => value.length > 0);
}

function normalizedSamples(values: unknown): DataSampleOption[] {
  if (!Array.isArray(values)) return [];
  const entries: DataSampleOption[] = [];
  for (const item of values) {
    if (!item || typeof item !== "object") continue;
    const typed = item as Partial<DataSampleOption>;
    const label = String(typed.label || "").trim();
    if (!label) continue;
    entries.push({
      label,
      path: String(typed.path || "").trim(),
      disabled: Boolean(typed.disabled),
    });
  }
  return entries;
}

function normalizeDatasetDateInput(value: string): string {
  const next = String(value || "").trim();
  if (!next) return "";
  return /^\d{4}-\d{2}-\d{2}$/.test(next) ? next : "";
}

function polygonDatasetDateText(ts: number): string {
  return normalizeDatasetDateInput(formatUtcDateFromUnixMs(ts, ""));
}

function polygonDatasetMatchesDownloadInputs(
  state: DataState,
  dataset: PolygonDatasetSummary,
): boolean {
  const requestedMarket = String(state.polygonMarket || "stocks")
    .trim()
    .toUpperCase();
  const requestedSymbol = String(state.polygonSymbol || "")
    .trim()
    .toUpperCase();
  if (!requestedSymbol) return false;
  const datasetMarket = String(dataset.market || "")
    .trim()
    .toUpperCase();
  if (!datasetMarket || datasetMarket !== requestedMarket) return false;
  if (Boolean(dataset.adjusted) !== Boolean(state.polygonAdjusted))
    return false;
  const displaySymbol = String(dataset.displaySymbol || "")
    .trim()
    .toUpperCase();
  const polygonTicker = String(dataset.polygonTicker || "")
    .trim()
    .toUpperCase();
  return requestedSymbol === displaySymbol || requestedSymbol === polygonTicker;
}

function resolveAutoTopOffDatasetForDownload(
  state: DataState,
): PolygonDatasetSummary | null {
  const requestedFrom = normalizeDatasetDateInput(
    String(state.polygonFrom || ""),
  );
  const requestedTo = normalizeDatasetDateInput(String(state.polygonTo || ""));
  if (!requestedFrom || !requestedTo) return null;
  const today = polygonDatasetDateText(Date.now());
  if (!today) return null;

  for (const dataset of state.polygonDatasets) {
    if (!polygonDatasetMatchesDownloadInputs(state, dataset)) continue;
    const cachedFrom = polygonDatasetDateText(Number(dataset.minTs || 0));
    const cachedTo = polygonDatasetDateText(Number(dataset.maxTs || 0));
    if (!cachedFrom || !cachedTo) continue;
    if (requestedFrom !== cachedFrom || requestedTo !== cachedTo) continue;
    if (cachedTo >= today) return null;
    return dataset;
  }

  return null;
}

function normalizePath(value: unknown): string {
  return String(value || "")
    .trim()
    .replace(/\\/g, "/");
}

function normalizePathKey(value: unknown): string {
  return normalizePath(value).toLowerCase();
}

function normalizeRootPath(value: unknown): string {
  return normalizePath(value).replace(/\/+$/, "");
}

function pathIsWithinRoot(path: unknown, root: unknown): boolean {
  const normalizedPath = normalizePathKey(path);
  const normalizedRoot = normalizeRootPath(root).toLowerCase();
  if (!normalizedPath || !normalizedRoot) return false;
  return (
    normalizedPath === normalizedRoot ||
    normalizedPath.startsWith(`${normalizedRoot}/`)
  );
}

function rewritePathForMovedRoot(
  path: unknown,
  fromRoot: unknown,
  toRoot: unknown,
): string {
  const currentPath = normalizePath(path);
  const currentRoot = normalizeRootPath(fromRoot);
  const nextRoot = normalizeRootPath(toRoot);
  if (!currentPath || !currentRoot || !nextRoot) return currentPath;
  if (!pathIsWithinRoot(currentPath, currentRoot)) return currentPath;
  const relative = currentPath.slice(currentRoot.length).replace(/^\/+/, "");
  return relative ? `${nextRoot}/${relative}` : nextRoot;
}

function rewriteRecentPathsForMovedRoot(
  paths: string[],
  fromRoot: string,
  toRoot: string,
): string[] {
  const rewritten = paths.map((path) =>
    rewritePathForMovedRoot(path, fromRoot, toRoot),
  );
  return rewritten
    .filter((path, index, rows) => {
      const normalized = normalizePathKey(path);
      return (
        !!normalized &&
        rows.findIndex((entry) => normalizePathKey(entry) === normalized) ===
          index
      );
    })
    .slice(0, 8);
}

function polygonIntegrityText(dataset: PolygonDatasetSummary): string {
  const integrity = dataset.integrity;
  if (!integrity) return "Integrity: unchecked";
  return `Integrity: parse=${integrity.parseErrors} dup=${integrity.duplicates} order=${integrity.outOfOrder} gaps=${integrity.sameDayGapsGt2Min}`;
}

function polygonDatasetMetaText(dataset: PolygonDatasetSummary): string {
  const label = String(
    dataset.displaySymbol ||
      dataset.polygonTicker ||
      fileBasename(dataset.csvPath) ||
      dataset.csvPath,
  ).trim();
  const parts = [
    label,
    dataset.timeframe || "1m",
    dataset.adjusted ? "adjusted" : "raw",
    dataset.rowCount > 0 ? `${dataset.rowCount} rows` : "",
    polygonIntegrityText(dataset),
  ].filter((item) => item.length > 0);
  return parts.join(" · ");
}

function defaultPolygonCredentialStatus(
  hasApiKey = false,
): PolygonCredentialStatusSummary {
  return {
    hasApiKey,
    state: hasApiKey ? "keyring" : "missing",
    storageBackend: hasApiKey ? "unknown" : "missing",
    degraded: false,
    message: "",
  };
}

function polygonCredentialStorageText(
  status: PolygonCredentialStatusSummary | undefined,
): string {
  if (!status || !status.hasApiKey) return "";
  switch (status.state) {
    case "keyring":
      return "Credential storage: system keyring";
    case "json_file_fallback":
      return "Credential storage: legacy JSON fallback";
    case "env_fallback":
      return "Credential source: environment variable";
    case "dotenv_fallback":
      return "Credential source: local .env fallback";
    case "unavailable":
      return "Credential storage unavailable";
    case "missing":
    default:
      return "";
  }
}

function polygonCacheMetaText(
  info: Pick<PolygonCacheInfoSummary, "cacheDir" | "hasApiKey" | "datasets"> & {
    credentialStatus?: PolygonCredentialStatusSummary;
  },
): string {
  const count = info.datasets.length;
  const credentialStatus =
    info.credentialStatus || defaultPolygonCredentialStatus(info.hasApiKey);
  const storageText = polygonCredentialStorageText(credentialStatus);
  const parts = [
    info.hasApiKey ? "API key saved" : "API key missing",
    info.cacheDir ? `Cache: ${info.cacheDir}` : "",
    `${count} cached dataset${count === 1 ? "" : "s"}`,
    storageText
      ? credentialStatus.degraded
        ? `${storageText} (degraded)`
        : storageText
      : "",
  ].filter((item) => item.length > 0);
  return parts.join(" · ") || POLYGON_CACHE_UNAVAILABLE_TEXT;
}

function normalizeDataSourceKind(value: unknown): DataSourceKind {
  return String(value || "").trim() === "polygon" ? "polygon" : "csv";
}

function redactDataStoreText(
  state: Pick<DataState, "polygonApiKey">,
  raw: unknown,
): string {
  return redactKnownSecrets(raw, [state.polygonApiKey]);
}

function normalizeStartupSnapshot(raw: unknown): DataStartupSnapshot | null {
  if (!raw || typeof raw !== "object") return null;
  const row = raw as Partial<DataStartupSnapshot>;
  return {
    autoRestoreDataset: Boolean(row.autoRestoreDataset),
    baseCsvPath: String(row.baseCsvPath || "").trim(),
    selectedTimeframe: String(row.selectedTimeframe || "").trim(),
    source: normalizeDataSourceKind(row.source),
    baseSourceSizeBytes:
      typeof row.baseSourceSizeBytes === "number" &&
      Number.isFinite(row.baseSourceSizeBytes)
        ? row.baseSourceSizeBytes
        : null,
    baseSourceMtimeUtcMs:
      typeof row.baseSourceMtimeUtcMs === "number" &&
      Number.isFinite(row.baseSourceMtimeUtcMs)
        ? row.baseSourceMtimeUtcMs
        : null,
    savedAtUnixMs:
      typeof row.savedAtUnixMs === "number" &&
      Number.isFinite(row.savedAtUnixMs)
        ? row.savedAtUnixMs
        : 0,
  };
}

function startupRestoreSourceIdentityWarning(
  snapshot: DataStartupSnapshot | null,
  meta: DataLoadResult["meta"] | null,
): string {
  const expectedSize =
    snapshot && typeof snapshot.baseSourceSizeBytes === "number"
      ? snapshot.baseSourceSizeBytes
      : null;
  const expectedMtime =
    snapshot && typeof snapshot.baseSourceMtimeUtcMs === "number"
      ? snapshot.baseSourceMtimeUtcMs
      : null;
  const actualSize =
    meta && typeof meta.sourceSizeBytes === "number" ? meta.sourceSizeBytes : null;
  const actualMtime =
    meta && typeof meta.sourceMtimeUtcMs === "number"
      ? meta.sourceMtimeUtcMs
      : null;

  const warnings: string[] = [];
  if (
    expectedSize != null &&
    actualSize != null &&
    Number(expectedSize) !== Number(actualSize)
  ) {
    warnings.push("size changed");
  }
  if (
    expectedMtime != null &&
    actualMtime != null &&
    Number(expectedMtime) !== Number(actualMtime)
  ) {
    warnings.push("mtime changed");
  }
  if (!warnings.length) return "";
  return `Source changed since last session (${warnings.join("; ")}).`;
}

function readStartupSnapshot(): DataStartupSnapshot | null {
  return normalizeStartupSnapshot(
    readStorageJson<DataStartupSnapshot>(DATA_STARTUP_SNAPSHOT_KEY),
  );
}

function writeStartupSnapshot(snapshot: DataStartupSnapshot): void {
  writeStorageJson(DATA_STARTUP_SNAPSHOT_KEY, snapshot);
}

function persistStartupSnapshot(state: DataState): void {
  const hasBase =
    hasLoadedDataset(state.baseDataId) &&
    !!String(state.baseCsvPath || "").trim();
  writeStartupSnapshot({
    autoRestoreDataset: !!state.autoRestoreDataset,
    baseCsvPath: hasBase ? String(state.baseCsvPath || "").trim() : "",
    selectedTimeframe: hasBase
      ? String(
          state.dataTimeframe || state.baseDataMeta?.timeframe || "",
        ).trim()
      : "",
    source: normalizeDataSourceKind(state.baseSourceKind),
    baseSourceSizeBytes: hasBase ? (state.baseSourceSizeBytes ?? null) : null,
    baseSourceMtimeUtcMs: hasBase ? (state.baseSourceMtimeUtcMs ?? null) : null,
    savedAtUnixMs: Date.now(),
  });
}

function selectPolygonDatasetPath(state: DataState, preferredPath = ""): void {
  const nextPath = String(preferredPath || "").trim();
  if (
    nextPath &&
    state.polygonDatasets.some((row) => row.csvPath === nextPath)
  ) {
    state.polygonSelectedPath = nextPath;
  } else if (
    state.polygonSelectedPath &&
    state.polygonDatasets.some(
      (row) => row.csvPath === state.polygonSelectedPath,
    )
  ) {
      } else {
    state.polygonSelectedPath = state.polygonDatasets[0]?.csvPath || "";
  }
  const selected =
    state.polygonDatasets.find(
      (row) => row.csvPath === state.polygonSelectedPath,
    ) || null;
  state.polygonSelectedMeta = selected
    ? polygonDatasetMetaText(selected)
    : POLYGON_SELECTED_PROMPT;
}

function applyPolygonCacheInfo(
  state: DataState,
  info: PolygonCacheInfoSummary,
  preferredPath = "",
): void {
  state.polygonCacheDir = String(info.cacheDir || "").trim();
  state.polygonHasApiKey = !!info.hasApiKey;
  state.polygonCredentialStatus =
    info.credentialStatus || defaultPolygonCredentialStatus(info.hasApiKey);
  state.polygonDatasets = Array.isArray(info.datasets)
    ? [...info.datasets]
    : [];
  state.polygonCacheMeta = polygonCacheMetaText(info);
  selectPolygonDatasetPath(state, preferredPath);
}

function resetPolygonCacheRuntimeState(
  state: DataState,
  nextCacheDir = "",
  preferredSelectedPath = "",
): void {
  polygonCacheLoadPromise = null;
  state.polygonCacheLoadedOnce = false;
  state.polygonCacheDir = String(nextCacheDir || "").trim();
  state.polygonHasApiKey = false;
  state.polygonCredentialStatus = defaultPolygonCredentialStatus(false);
  state.polygonDatasets = [];
  state.polygonSelectedPath = String(preferredSelectedPath || "").trim();
  state.polygonSelectedMeta = POLYGON_SELECTED_PROMPT;
  state.polygonCacheMeta = state.polygonCacheDir
    ? "Polygon cache metadata not loaded."
    : POLYGON_CACHE_UNAVAILABLE_TEXT;
}

function upsertPolygonDatasetFromDownload(
  state: DataState,
  result: { csvPath: string; baseLoad: DataLoadResult },
): void {
  const csvPath = String(result.csvPath || "").trim();
  if (!csvPath) return;
  const next: PolygonDatasetSummary = {
    market: String(state.polygonMarket || "stocks").trim() || "stocks",
    displaySymbol: String(state.polygonSymbol || fileBasename(csvPath))
      .trim()
      .toUpperCase(),
    polygonTicker: String(state.polygonSymbol || fileBasename(csvPath))
      .trim()
      .toUpperCase(),
    timeframe: String(result.baseLoad.meta?.timeframe || "1m").trim() || "1m",
    csvPath,
    minTs: Number(result.baseLoad.meta?.minTs || 0),
    maxTs: Number(result.baseLoad.meta?.maxTs || 0),
    lastUpdatedUtcMs: Number(
      result.baseLoad.meta?.sourceMtimeUtcMs || Date.now(),
    ),
    rowCount: Number(result.baseLoad.meta?.len || 0),
    adjusted: !!state.polygonAdjusted,
    integrity: null,
  };
  const rows = state.polygonDatasets.filter((row) => row.csvPath !== csvPath);
  rows.unshift(next);
  state.polygonDatasets = rows;
  state.polygonCacheMeta = polygonCacheMetaText({
    cacheDir: state.polygonCacheDir,
    hasApiKey: state.polygonHasApiKey,
    credentialStatus: state.polygonCredentialStatus,
    datasets: state.polygonDatasets,
  });
  selectPolygonDatasetPath(state, csvPath);
}

function applyEmptyDatasetState(state: DataState, statusLine = "Ready."): void {
  state.dataId = "—";
  state.datasetLabel = "—";
  state.datasetRange = "—";
  state.dataLen = "—";
  state.dataTimeframe = "";
  state.dataTimeframeHint = "Load a dataset to enable timeframe resampling.";
  state.datasetContext = "—";
  state.dataFields = [];
  state.dataFieldHint = "";
  state.baseDataId = "";
  state.baseDataMeta = null;
  state.baseCsvPath = "";
  state.baseSourceKind = "csv";
  state.baseSourceSizeBytes = null;
  state.baseSourceMtimeUtcMs = null;
  state.baseDatasetLabel = "";
  state.baseDatasetContext = "";
  state.canApplyTimeframe = false;
  state.statusLine = String(statusLine || "Ready.").trim() || "Ready.";
}

function applyMarketDataDirInfo(
  state: DataState,
  info: MarketDataDirInfoSummary,
): boolean {
  const previousCacheDir = String(state.polygonCacheDir || "").trim();
  const nextHint =
    String(info?.hint || state.marketDataDirHint || "—").trim() || "—";
  const nextRootDir = String(info?.configuredRootDir || "").trim();
  const nextCacheDir = String(info?.polygonCacheDir || "").trim();
  state.marketDataDir = nextRootDir;
  state.marketDataDirHint = nextHint;
  return previousCacheDir !== nextCacheDir;
}

async function ensureStartupAffordancesLoaded(state: DataState): Promise<void> {
  if (state.startupDeferredHydratedOnce) return;
  if (startupDeferredHydratePromise) return startupDeferredHydratePromise;
  startupDeferredHydratePromise = (async () => {
    const samples = await listSampleCsvsAction().catch(() => []);
    state.sampleCsvOptions = Array.isArray(samples) ? samples : [];
    state.sampleCsvVisible = state.sampleCsvOptions.length > 0;
    state.startupDeferredHydratedOnce = true;
    markVueUpdate(state);
  })().finally(() => {
    startupDeferredHydratePromise = null;
  });
  return startupDeferredHydratePromise;
}

function rewriteMovedPolygonStartupState(
  state: DataState,
  previousCacheDir: string,
  nextCacheDir: string,
): string {
  const priorBaseCsvPath = String(state.baseCsvPath || "").trim();
  const priorCsvPath = String(state.csvPath || "").trim();
  const priorSelectedPath = String(state.polygonSelectedPath || "").trim();
  if (
    state.baseSourceKind !== "polygon" ||
    !previousCacheDir ||
    !nextCacheDir ||
    normalizePathKey(previousCacheDir) === normalizePathKey(nextCacheDir) ||
    !pathIsWithinRoot(priorBaseCsvPath, previousCacheDir)
  ) {
    return "";
  }

  const nextBaseCsvPath = rewritePathForMovedRoot(
    priorBaseCsvPath,
    previousCacheDir,
    nextCacheDir,
  );
  const nextCsvPath = rewritePathForMovedRoot(
    priorCsvPath,
    previousCacheDir,
    nextCacheDir,
  );
  const nextSelectedPath = rewritePathForMovedRoot(
    priorSelectedPath,
    previousCacheDir,
    nextCacheDir,
  );

  state.baseCsvPath = nextBaseCsvPath;
  if (pathIsWithinRoot(priorCsvPath, previousCacheDir)) {
    state.csvPath = nextCsvPath;
  }
  if (pathIsWithinRoot(priorSelectedPath, previousCacheDir)) {
    state.polygonSelectedPath = nextSelectedPath;
  }
  state.recentCsvPaths = rewriteRecentPathsForMovedRoot(
    state.recentCsvPaths,
    previousCacheDir,
    nextCacheDir,
  );
  persistStartupSnapshot(state);
  return state.polygonSelectedPath;
}

async function ensurePolygonCacheLoaded(
  state: DataState,
  force = false,
  preferredPath = "",
): Promise<void> {
  if (state.polygonCacheLoadedOnce && !force) {
    if (preferredPath) selectPolygonDatasetPath(state, preferredPath);
    return;
  }
  if (polygonCacheLoadPromise) {
    await polygonCacheLoadPromise;
    if (!force || state.polygonCacheLoadedOnce) {
      if (preferredPath) selectPolygonDatasetPath(state, preferredPath);
      return;
    }
  }
  polygonCacheLoadPromise = (async () => {
    const info = await getPolygonCacheInfoAction();
    applyPolygonCacheInfo(
      state,
      info,
      preferredPath || state.polygonSelectedPath,
    );
    state.polygonCacheLoadedOnce = true;
  })();
  try {
    await polygonCacheLoadPromise;
  } finally {
    polygonCacheLoadPromise = null;
  }
}

function applyVueTextFieldUpdate(
  state: DataState,
  field: string,
  value: string,
): boolean {
  const key = String(field || "").trim();
  const next = String(value || "").trim();
  if (!key) return false;
  switch (key) {
    case "csvPath":
      state.csvPath = next;
      return true;
    case "marketDataDir":
      state.marketDataDir = next;
      return true;
    case "dataTimeframe":
      state.dataTimeframe = next;
      return true;
    case "polygonApiKey":
      state.polygonApiKey = next;
      return true;
    case "polygonMarket":
      state.polygonMarket = next || "stocks";
      return true;
    case "polygonSymbol":
      state.polygonSymbol = next.toUpperCase();
      return true;
    case "polygonFrom":
      state.polygonFrom = normalizeDatasetDateInput(next) || next;
      return true;
    case "polygonTo":
      state.polygonTo = normalizeDatasetDateInput(next) || next;
      return true;
    case "polygonTimeframe":
      state.polygonTimeframe = next || "1m";
      return true;
    case "polygonSelectedPath":
      state.polygonSelectedPath = next;
      selectPolygonDatasetPath(state, next);
      return true;
    default:
      return false;
  }
}

function applyVueCheckboxFieldUpdate(
  state: DataState,
  field: string,
  checked: boolean,
): boolean {
  const key = String(field || "").trim();
  const next = !!checked;
  if (!key) return false;
  switch (key) {
    case "marketDataMoveExisting":
      state.marketDataMoveExisting = next;
      return true;
    case "autoRestoreDataset":
      state.autoRestoreDataset = next;
      return true;
    case "polygonAdjusted":
      state.polygonAdjusted = next;
      return true;
    default:
      return false;
  }
}

export const useDataStore = defineStore("data", {
  state: (): DataState => ({
    activeControlsTab: "data",
    csvPath: "",
    recentCsvPaths: [],
    sampleCsvOptions: [],
    sampleCsvVisible: false,
    marketDataDir: "",
    marketDataMoveExisting: true,
    marketDataDirHint: "—",
    dataId: "—",
    datasetLabel: "—",
    datasetRange: "—",
    dataLen: "—",
    dataTimeframe: "",
    dataTimeframeHint: "—",
    autoRestoreDataset: true,
    datasetContext: "—",
    dataFields: [],
    dataFieldHint: "",
    canBrowseCsv: true,
    canPolygonOpen: true,
    canLoadCsv: true,
    canClearData: true,
    canBrowseMarketDataDir: true,
    canSetMarketDataDir: true,
    canResetMarketDataDir: true,
    canOpenMarketDataDir: true,
    canApplyTimeframe: false,
    polygonOpen: false,
    polygonApiKey: "",
    polygonMarket: "stocks",
    polygonSymbol: "",
    polygonFrom: "",
    polygonTo: "",
    polygonTimeframe: "1m",
    polygonAdjusted: true,
    polygonCacheDir: "",
    polygonHasApiKey: false,
    polygonCredentialStatus: defaultPolygonCredentialStatus(false),
    polygonDatasets: [],
    polygonSelectedPath: "",
    polygonSelectedMeta: POLYGON_SELECTED_PROMPT,
    polygonCacheMeta: POLYGON_CACHE_UNAVAILABLE_TEXT,
    canPolygonRefresh: true,
    canPolygonSaveApiKey: false,
    canPolygonClearApiKey: false,
    canPolygonDownload: false,
    canPolygonLoadSelected: false,
    canPolygonValidateSelected: false,
    canPolygonRepairSelected: false,
    statusLine: "Ready.",
    baseDataId: "",
    baseDataMeta: null,
    baseCsvPath: "",
    baseSourceKind: "csv",
    baseSourceSizeBytes: null,
    baseSourceMtimeUtcMs: null,
    baseDatasetLabel: "",
    baseDatasetContext: "",
    startupHydratedOnce: false,
    startupDeferredHydratedOnce: false,
    polygonCacheLoadedOnce: false,
    source: "unknown",
    updatedAtUnixMs: 0,
  }),

  actions: {
    hydrateStartupState(options?: {
      includeDeferred?: boolean;
    }): Promise<void> {
      const includeDeferred = options?.includeDeferred !== false;
      const hydrateCore = (): Promise<void> => {
        if (this.startupHydratedOnce) return Promise.resolve();
        if (startupHydratePromise) return startupHydratePromise;
        startupHydratePromise = (async () => {
          const startup = readStartupSnapshot();
          const restorePath = String(startup?.baseCsvPath || "").trim();
          const restoreSource = normalizeDataSourceKind(startup?.source);
          const restoreTimeframe = String(
            startup?.selectedTimeframe || "",
          ).trim();
          this.autoRestoreDataset =
            startup?.autoRestoreDataset ?? this.autoRestoreDataset;
          if (this.autoRestoreDataset && restorePath) {
            this.statusLine = STARTUP_RESTORE_PENDING_STATUS;
            markVueUpdate(this);
          }

          const marketInfo = await getMarketDataDirInfoAction().catch(
            () => null,
          );

          if (marketInfo) {
            const nextCacheDir = String(
              marketInfo.polygonCacheDir || "",
            ).trim();
            const cacheDirChanged = applyMarketDataDirInfo(this, marketInfo);
            if (cacheDirChanged)
              resetPolygonCacheRuntimeState(this, nextCacheDir);
            else this.polygonCacheDir = nextCacheDir;
          }

          if (this.autoRestoreDataset && restorePath) {
            const restoreLabel = fileBasename(restorePath) || restorePath;
            this.statusLine = `Restoring ${restoreLabel}...`;
            markVueUpdate(this);
            try {
              const base = await loadCsvAction(restorePath);
              const restoreIdentityWarning = startupRestoreSourceIdentityWarning(
                startup,
                base.meta,
              );
              this.applyLoadResult(base, {
                treatAsBase: true,
                statusLine: baseLoadStatusText(base, restoreSource),
                source: restoreSource,
              });
              const baseTf = String(base.meta?.timeframe || "").trim();
              if (restoreTimeframe && restoreTimeframe !== baseTf) {
                const derived = await applyTimeframeAction(
                  restoreTimeframe,
                  base.dataId,
                );
                this.applyLoadResult(derived, {
                  treatAsBase: false,
                  statusLine: derivedLoadStatusText(derived),
                  source: restoreSource,
                });
              } else {
                this.statusLine = `Restored ${restoreLabel}.`;
                markVueUpdate(this);
              }
              if (restoreIdentityWarning) {
                this.statusLine = `${this.statusLine} ${restoreIdentityWarning}`.trim();
                markVueUpdate(this);
              }
            } catch (error: unknown) {
              applyEmptyDatasetState(
                this,
                `Startup restore failed: ${String(error || "").trim() || "restore failed."}`,
              );
              markVueUpdate(this);
            }
          }

          this.startupHydratedOnce = true;
          persistStartupSnapshot(this);
          setControlsBusy(this, false);
          markVueUpdate(this);
        })().finally(() => {
          startupHydratePromise = null;
        });
        return startupHydratePromise;
      };

      const critical = hydrateCore();
      if (!includeDeferred) return critical;
      return critical.then(() => ensureStartupAffordancesLoaded(this));
    },

    applyLegacySnapshot(snapshot: Partial<DataState>): void {
      const startup = readStartupSnapshot();
      const pendingRestore =
        Boolean(snapshot.autoRestoreDataset ?? this.autoRestoreDataset) &&
        !!String(startup?.baseCsvPath || "").trim();

      applyEmptyDatasetState(
        this,
        pendingRestore ? STARTUP_RESTORE_PENDING_STATUS : "Ready.",
      );
      this.activeControlsTab =
        String(
          snapshot.activeControlsTab ?? this.activeControlsTab ?? "data",
        ).trim() || "data";
      this.csvPath = String(snapshot.csvPath ?? this.csvPath ?? "").trim();
      this.recentCsvPaths = normalizedList(
        snapshot.recentCsvPaths ?? this.recentCsvPaths ?? [],
      );
      this.sampleCsvOptions = normalizedSamples(
        snapshot.sampleCsvOptions ?? this.sampleCsvOptions ?? [],
      );
      this.sampleCsvVisible = Boolean(
        snapshot.sampleCsvVisible ?? this.sampleCsvVisible,
      );
      this.marketDataDir = String(
        snapshot.marketDataDir ?? this.marketDataDir ?? "",
      ).trim();
      this.marketDataMoveExisting = Boolean(
        snapshot.marketDataMoveExisting ?? this.marketDataMoveExisting,
      );
      this.marketDataDirHint =
        String(
          snapshot.marketDataDirHint ?? this.marketDataDirHint ?? "—",
        ).trim() || "—";
      this.dataTimeframe = String(
        snapshot.dataTimeframe ?? this.dataTimeframe ?? "",
      ).trim();
      this.dataTimeframeHint = timeframeHintText("", this.dataTimeframe, false);
      this.autoRestoreDataset = Boolean(
        snapshot.autoRestoreDataset ?? this.autoRestoreDataset,
      );
      this.polygonOpen = Boolean(snapshot.polygonOpen ?? this.polygonOpen);
      this.polygonApiKey = String(this.polygonApiKey ?? "").trim();
      this.polygonMarket =
        String(
          snapshot.polygonMarket ?? this.polygonMarket ?? "stocks",
        ).trim() || "stocks";
      this.polygonSymbol = String(
        snapshot.polygonSymbol ?? this.polygonSymbol ?? "",
      )
        .trim()
        .toUpperCase();
      this.polygonFrom = normalizeDatasetDateInput(
        String(snapshot.polygonFrom ?? this.polygonFrom ?? "").trim(),
      );
      this.polygonTo = normalizeDatasetDateInput(
        String(snapshot.polygonTo ?? this.polygonTo ?? "").trim(),
      );
      this.polygonTimeframe =
        String(
          snapshot.polygonTimeframe ?? this.polygonTimeframe ?? "1m",
        ).trim() || "1m";
      this.polygonAdjusted = Boolean(
        snapshot.polygonAdjusted ?? this.polygonAdjusted,
      );
      resetPolygonCacheRuntimeState(this);
      setControlsBusy(this, false);
      startupHydratePromise = null;
      startupDeferredHydratePromise = null;
      this.startupHydratedOnce = false;
      this.startupDeferredHydratedOnce = false;
      this.polygonCacheLoadedOnce = false;
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();
    },

    applyLoadResult(
      result: DataLoadResult,
      options?: {
        treatAsBase?: boolean;
        statusLine?: string;
        source?: DataSourceKind;
      },
    ): void {
      const row = result && typeof result === "object" ? result : null;
      if (!row) return;
      const dataId = String(row.dataId || "").trim();
      const meta = row.meta && typeof row.meta === "object" ? row.meta : null;
      const path = String(row.path || "").trim();
      const treatAsBase = options?.treatAsBase !== false;
      const source = options?.source || this.baseSourceKind || "csv";
      const loadText = loadSourceText(row.loadSource);
      const cachePath = String(row.cachePath || "").trim();
      const contextPath = treatAsBase ? path : this.baseCsvPath || path;
      const contextText = datasetContextText(
        contextPath,
        treatAsBase ? source : this.baseSourceKind,
        loadText,
        cachePath,
      );

      if (path) {
        this.csvPath = path;
        this.recentCsvPaths = pushRecentCsvPath(this.recentCsvPaths, path);
      }

      if (treatAsBase) {
        this.baseDataId = dataId;
        this.baseDataMeta = meta;
        this.baseCsvPath = path;
        this.baseSourceKind = source;
        this.baseSourceSizeBytes = meta?.sourceSizeBytes ?? null;
        this.baseSourceMtimeUtcMs = meta?.sourceMtimeUtcMs ?? null;
        this.baseDatasetLabel =
          fileBasename(path) ||
          this.baseDatasetLabel ||
          this.datasetLabel ||
          "—";
        this.baseDatasetContext = contextText;
      }

      const baseMeta =
        this.baseDataMeta && typeof this.baseDataMeta === "object"
          ? this.baseDataMeta
          : meta;
      const baseTf = String(baseMeta?.timeframe || "").trim();
      const effectiveTf = String(meta?.timeframe || "").trim() || baseTf;
      const derived = !!baseTf && !!effectiveTf && baseTf !== effectiveTf;

      this.dataId = dataId || this.dataId || "—";
      this.dataLen = meta ? String(meta.len) : this.dataLen || "—";
      this.datasetLabel = treatAsBase
        ? fileBasename(path) || this.datasetLabel || "—"
        : this.baseDatasetLabel || this.datasetLabel || "—";
      this.datasetContext = contextText;
      this.datasetRange = datasetRangeText(meta, effectiveTf);
      this.dataFields = dataFieldList(meta);
      this.dataFieldHint = dataFieldHint(meta);
      this.dataTimeframe = derived ? effectiveTf : "";
      this.dataTimeframeHint = timeframeHintText(
        baseTf,
        effectiveTf,
        hasLoadedDataset(this.baseDataId || this.dataId),
        isRelativeTimeAxis(baseMeta),
      );
      this.canApplyTimeframe = hasLoadedDataset(this.baseDataId || this.dataId);
      this.statusLine =
        String(
          options?.statusLine ||
            (treatAsBase
              ? baseLoadStatusText(row, source)
              : derivedLoadStatusText(row)) ||
            this.statusLine ||
            "Ready.",
        ).trim() || "Ready.";
      persistStartupSnapshot(this);
      markVueUpdate(this);
    },

    applyBaseDatasetState(): void {
      const baseId = String(this.baseDataId || "").trim();
      const baseMeta =
        this.baseDataMeta && typeof this.baseDataMeta === "object"
          ? this.baseDataMeta
          : null;
      if (!hasLoadedDataset(baseId) || !baseMeta) return;
      const baseTf = String(baseMeta.timeframe || "").trim();

      this.dataId = baseId;
      this.dataLen = String(baseMeta.len);
      this.datasetLabel = this.baseDatasetLabel || this.datasetLabel || "—";
      this.datasetContext =
        this.baseDatasetContext || this.datasetContext || "—";
      this.datasetRange = datasetRangeText(baseMeta, baseTf);
      this.dataFields = dataFieldList(baseMeta);
      this.dataFieldHint = dataFieldHint(baseMeta);
      this.dataTimeframe = "";
      this.dataTimeframeHint = timeframeHintText(
        baseTf,
        baseTf,
        true,
        isRelativeTimeAxis(baseMeta),
      );
      this.canApplyTimeframe = true;
      this.statusLine = baseTf
        ? `Using base timeframe (${baseTf}).`
        : "Using base timeframe.";
      persistStartupSnapshot(this);
      markVueUpdate(this);
    },

    setInputField(field: string, value: string): void {
      const key = String(field || "").trim();
      const next = String(value || "");
      if (applyVueTextFieldUpdate(this, key, next)) {
        setControlsBusy(this, false);
        markVueUpdate(this);
      }
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || "").trim();
      const next = String(value || "");
      if (applyVueTextFieldUpdate(this, key, next)) {
        setControlsBusy(this, false);
        markVueUpdate(this);
      }
    },

    setCheckboxField(field: string, checked: boolean): void {
      const key = String(field || "").trim();
      const next = !!checked;
      if (applyVueCheckboxFieldUpdate(this, key, next)) {
        if (key === "autoRestoreDataset") {
          persistStartupSnapshot(this);
        }
        setControlsBusy(this, false);
        markVueUpdate(this);
      }
    },

    browseCsv(): void {
      setControlsBusy(this, true);
      this.statusLine = "Loading data...";
      markVueUpdate(this);

      void browseCsvAction()
        .then((result) => {
          if (!result) return;
          this.applyLoadResult(result, {
            treatAsBase: true,
            statusLine: baseLoadStatusText(result, "csv"),
            source: "csv",
          });
        })
        .catch((error: unknown) => {
          this.statusLine = redactDataStoreText(this, error) || "Data load failed.";
          markVueUpdate(this);
        })
        .finally(() => {
          setControlsBusy(this, false);
          markVueUpdate(this);
        });
    },

    openPolygon(): void {
      const pinia = resolveRuntimePinia();
      if (pinia) {
        useWorkflowStore(pinia).selectWorkspace("workflow", "data");
      }
      this.polygonOpen = true;
      markVueUpdate(this);
      void openPolygonAction().catch(() => undefined);
      void ensurePolygonCacheLoaded(this)
        .then(() => {
          markVueUpdate(this);
        })
        .catch((error: unknown) => {
          this.statusLine =
            redactDataStoreText(this, error) ||
            "Failed to refresh Polygon cache.";
          markVueUpdate(this);
        });
    },

    closePolygon(): void {
      this.polygonOpen = false;
      markVueUpdate(this);
      void closePolygonAction().catch(() => undefined);
    },

    refreshPolygon(): void {
      setControlsBusy(this, true);
      this.statusLine = "Refreshing Polygon cache...";
      markVueUpdate(this);
      void ensurePolygonCacheLoaded(this, true)
        .then(() => {
          setControlsBusy(this, false);
          this.statusLine = "Refreshed Polygon cache.";
          markVueUpdate(this);
        })
        .catch((error: unknown) => {
          setControlsBusy(this, false);
          this.statusLine =
            redactDataStoreText(this, error) ||
            "Failed to refresh Polygon cache.";
          markVueUpdate(this);
        });
    },

    savePolygonApiKey(): void {
      const apiKey = String(this.polygonApiKey || "").trim();
      if (!apiKey) {
        this.statusLine = "Enter a Polygon API key.";
        markVueUpdate(this);
        return;
      }
      setControlsBusy(this, true);
      this.statusLine = "Saving Polygon API key...";
      markVueUpdate(this);
      void setPolygonApiKeyAction(apiKey)
        .then((info) => {
          applyPolygonCacheInfo(this, info, this.polygonSelectedPath);
          this.polygonCacheLoadedOnce = true;
          setControlsBusy(this, false);
          this.statusLine = "Saved Polygon API key.";
          markVueUpdate(this);
        })
        .catch((error: unknown) => {
          setControlsBusy(this, false);
          this.statusLine =
            redactDataStoreText(this, error) ||
            "Failed to save Polygon API key.";
          markVueUpdate(this);
        });
    },

    clearPolygonApiKey(): void {
      setControlsBusy(this, true);
      this.statusLine = "Clearing Polygon API key...";
      markVueUpdate(this);
      void clearPolygonApiKeyAction()
        .then((info) => {
          this.polygonApiKey = "";
          applyPolygonCacheInfo(this, info, this.polygonSelectedPath);
          this.polygonCacheLoadedOnce = true;
          setControlsBusy(this, false);
          this.statusLine = "Cleared Polygon API key.";
          markVueUpdate(this);
        })
        .catch((error: unknown) => {
          setControlsBusy(this, false);
          this.statusLine =
            redactDataStoreText(this, error) ||
            "Failed to clear Polygon API key.";
          markVueUpdate(this);
        });
    },

    selectPolygonDataset(path: string): void {
      this.polygonSelectedPath = String(path || "").trim();
      selectPolygonDatasetPath(this, this.polygonSelectedPath);
      markVueUpdate(this);
    },

    loadPolygonDataset(path?: string): void {
      const nextPath = String(path || this.polygonSelectedPath || "").trim();
      if (!nextPath) {
        this.statusLine = "Select a cached Polygon dataset first.";
        markVueUpdate(this);
        return;
      }
      this.polygonSelectedPath = nextPath;
      setControlsBusy(this, true);
      this.statusLine = "Loading Polygon dataset...";
      markVueUpdate(this);
      void loadCsvAction(nextPath)
        .then((result) => {
          this.applyLoadResult(result, {
            treatAsBase: true,
            statusLine: baseLoadStatusText(result, "polygon"),
            source: "polygon",
          });
        })
        .catch((error: unknown) => {
          this.statusLine =
            redactDataStoreText(this, error) || "Polygon data load failed.";
          markVueUpdate(this);
        })
        .finally(() => {
          setControlsBusy(this, false);
          markVueUpdate(this);
        });
    },

    downloadPolygonDataset(): void {
      setControlsBusy(this, true);
      this.statusLine = "Downloading Polygon dataset...";
      markVueUpdate(this);
      const matchingCachedDataset = resolveAutoTopOffDatasetForDownload(this);
      const request = matchingCachedDataset
        ? topOffPolygonDatasetAction({
            csvPath: matchingCachedDataset.csvPath,
            timeframe: this.polygonTimeframe,
          })
        : downloadPolygonDatasetAction({
            market: this.polygonMarket,
            symbol: this.polygonSymbol,
            from: this.polygonFrom,
            to: this.polygonTo,
            adjusted: this.polygonAdjusted,
            timeframe: this.polygonTimeframe,
          });
      void request
        .then((result) => {
          upsertPolygonDatasetFromDownload(this, result);
          this.polygonCacheLoadedOnce = true;
          const sourcePath = String(result.csvPath || "").trim();
          const symbol = String(this.polygonSymbol || "")
            .trim()
            .toUpperCase();
          const cacheNote = matchingCachedDataset
            ? result.usedCacheOnly
              ? "cache reused"
              : result.downloadedBars > 0
                ? `${result.downloadedBars} bars downloaded`
                : "no new bars available"
            : result.usedCacheOnly
              ? "cache reused"
              : `${result.downloadedBars} bars downloaded`;
          const baseTimeframe = String(
            result.baseLoad?.meta?.timeframe || "",
          ).trim();
          const effectiveTimeframe = String(
            result.load?.meta?.timeframe || "",
          ).trim();
          this.applyLoadResult(result.baseLoad, {
            treatAsBase: true,
            statusLine: `${baseLoadStatusText(result.baseLoad, "polygon")} ${cacheNote}.`,
            source: "polygon",
          });
          if (
            effectiveTimeframe &&
            baseTimeframe &&
            effectiveTimeframe !== baseTimeframe
          ) {
            this.applyLoadResult(result.load, {
              treatAsBase: false,
              statusLine: `${derivedLoadStatusText(result.load)} ${cacheNote}.`,
              source: "polygon",
            });
          }
          this.statusLine = symbol
            ? `${this.statusLine} ${symbol}.`
            : this.statusLine;
          this.csvPath = sourcePath || this.csvPath;
          setControlsBusy(this, false);
          markVueUpdate(this);
        })
        .catch((error: unknown) => {
          setControlsBusy(this, false);
          this.statusLine =
            redactDataStoreText(this, error) || "Polygon download failed.";
          markVueUpdate(this);
        });
    },

    topOffPolygonDataset(path?: string): void {
      const nextPath = String(path || this.polygonSelectedPath || "").trim();
      if (!nextPath) {
        this.statusLine = "Select a cached Polygon dataset first.";
        markVueUpdate(this);
        return;
      }

      const currentBasePath = String(this.baseCsvPath || "").trim();
      const baseTf = String(this.baseDataMeta?.timeframe || "").trim();
      const activeTf = String(this.dataTimeframe || "").trim();
      const requestedTf =
        normalizePathKey(currentBasePath) === normalizePathKey(nextPath)
          ? activeTf || baseTf || "1m"
          : "1m";

      setControlsBusy(this, true);
      this.statusLine = "Topping off Polygon dataset...";
      markVueUpdate(this);

      void topOffPolygonDatasetAction({
        csvPath: nextPath,
        timeframe: requestedTf,
      })
        .then((result) => {
          const sourcePath = String(result.csvPath || "").trim();
          const cacheNote = result.usedCacheOnly
            ? "cache reused"
            : result.downloadedBars > 0
              ? `${result.downloadedBars} bars downloaded`
              : "no new bars available";
          const baseTimeframe = String(
            result.baseLoad?.meta?.timeframe || "",
          ).trim();
          const effectiveTimeframe = String(
            result.load?.meta?.timeframe || "",
          ).trim();
          this.applyLoadResult(result.baseLoad, {
            treatAsBase: true,
            statusLine: `${baseLoadStatusText(result.baseLoad, "polygon")} ${cacheNote}.`,
            source: "polygon",
          });
          if (
            effectiveTimeframe &&
            baseTimeframe &&
            effectiveTimeframe !== baseTimeframe
          ) {
            this.applyLoadResult(result.load, {
              treatAsBase: false,
              statusLine: `${derivedLoadStatusText(result.load)} ${cacheNote}.`,
              source: "polygon",
            });
          }
          if (sourcePath) {
            void ensurePolygonCacheLoaded(this, true, sourcePath).catch(
              () => undefined,
            );
          }
          setControlsBusy(this, false);
          markVueUpdate(this);
        })
        .catch((error: unknown) => {
          setControlsBusy(this, false);
          this.statusLine =
            redactDataStoreText(this, error) || "Polygon top-off failed.";
          markVueUpdate(this);
        });
    },

    validatePolygonDataset(path?: string): void {
      const nextPath = String(path || this.polygonSelectedPath || "").trim();
      if (!nextPath) {
        this.statusLine = "Select a cached Polygon dataset first.";
        markVueUpdate(this);
        return;
      }
      setControlsBusy(this, true);
      this.statusLine = "Validating Polygon dataset...";
      markVueUpdate(this);
      void validatePolygonDatasetAction(nextPath)
        .then((dataset) => {
          const updated = this.polygonDatasets.map((row) =>
            row.csvPath === dataset.csvPath ? dataset : row,
          );
          this.polygonDatasets = updated;
          selectPolygonDatasetPath(this, dataset.csvPath);
          setControlsBusy(this, false);
          this.statusLine = `Validated Polygon dataset: ${fileBasename(dataset.csvPath) || dataset.displaySymbol || dataset.polygonTicker}.`;
          this.polygonCacheMeta = polygonCacheMetaText({
            cacheDir: this.polygonCacheDir,
            hasApiKey: this.polygonHasApiKey,
            credentialStatus: this.polygonCredentialStatus,
            datasets: this.polygonDatasets,
          });
          markVueUpdate(this);
        })
        .catch((error: unknown) => {
          setControlsBusy(this, false);
          this.statusLine =
            redactDataStoreText(this, error) || "Polygon validation failed.";
          markVueUpdate(this);
        });
    },

    repairPolygonDataset(path?: string): void {
      const nextPath = String(path || this.polygonSelectedPath || "").trim();
      if (!nextPath) {
        this.statusLine = "Select a cached Polygon dataset first.";
        markVueUpdate(this);
        return;
      }
      setControlsBusy(this, true);
      this.statusLine = "Repairing Polygon dataset...";
      markVueUpdate(this);
      void repairPolygonDatasetAction(nextPath)
        .then((dataset) => {
          const updated = this.polygonDatasets.map((row) =>
            row.csvPath === dataset.csvPath ? dataset : row,
          );
          this.polygonDatasets = updated;
          selectPolygonDatasetPath(this, dataset.csvPath);
          setControlsBusy(this, false);
          this.statusLine = `Repaired Polygon dataset: ${fileBasename(dataset.csvPath) || dataset.displaySymbol || dataset.polygonTicker}.`;
          this.polygonCacheMeta = polygonCacheMetaText({
            cacheDir: this.polygonCacheDir,
            hasApiKey: this.polygonHasApiKey,
            credentialStatus: this.polygonCredentialStatus,
            datasets: this.polygonDatasets,
          });
          markVueUpdate(this);
        })
        .catch((error: unknown) => {
          setControlsBusy(this, false);
          this.statusLine =
            redactDataStoreText(this, error) || "Polygon repair failed.";
          markVueUpdate(this);
        });
    },

    loadCsv(path?: string): void {
      const nextPath = String(path ?? this.csvPath ?? "").trim();
      if (!nextPath) {
        this.statusLine = "Enter a CSV path.";
        markVueUpdate(this);
        return;
      }

      this.csvPath = nextPath;
      setControlsBusy(this, true);
      this.statusLine = "Loading data...";
      markVueUpdate(this);

      void loadCsvAction(nextPath)
        .then((result) => {
          this.applyLoadResult(result, {
            treatAsBase: true,
            statusLine: baseLoadStatusText(result, "csv"),
            source: "csv",
          });
        })
        .catch((error: unknown) => {
          this.statusLine = redactDataStoreText(this, error) || "Data load failed.";
          markVueUpdate(this);
        })
        .finally(() => {
          setControlsBusy(this, false);
          markVueUpdate(this);
        });
    },

    clearData(): void {
      const baseId = String(this.baseDataId || "").trim();
      void clearDataAction(baseId).catch((error: unknown) => {
        this.statusLine = `Cleared UI state (backend clear warning: ${redactDataStoreText(this, error) || "clear failed"}).`;
        markVueUpdate(this);
      });
      applyEmptyDatasetState(this, "Cleared data.");
      persistStartupSnapshot(this);
      const pinia =
        (this as unknown as { $pinia?: Pinia }).$pinia ?? resolveRuntimePinia();
      if (pinia) useWorkflowArtifactsStore(pinia).clearArtifacts();
      setControlsBusy(this, false);
      markVueUpdate(this);
    },

    applyTimeframe(timeframe: string): void {
      const baseId = String(this.baseDataId || "").trim();
      if (!hasLoadedDataset(baseId)) {
        this.statusLine = "Load a dataset first (missing base data id).";
        markVueUpdate(this);
        return;
      }

      const target = String(timeframe || "").trim();
      this.dataTimeframe = target;
      markVueUpdate(this);

      const baseTf = String(this.baseDataMeta?.timeframe || "").trim();
      if (!target || (baseTf && target === baseTf)) {
        this.applyBaseDatasetState();
        return;
      }

      setControlsBusy(this, true);
      this.statusLine = `Resampling to ${target}...`;
      markVueUpdate(this);

      void applyTimeframeAction(target, baseId)
        .then((result) => {
          this.applyLoadResult(result, {
            treatAsBase: false,
            statusLine: derivedLoadStatusText(result),
          });
        })
        .catch((error: unknown) => {
          this.statusLine =
            redactDataStoreText(this, error) ||
            `Resample to ${target} failed.`;
          this.dataTimeframe = "";
          markVueUpdate(this);
        })
        .finally(() => {
          setControlsBusy(this, false);
          markVueUpdate(this);
        });
    },

    browseMarketDataDir(): void {
      void browseMarketDataDirAction()
        .then((pickedPath) => {
          const path = String(pickedPath || "").trim();
          if (!path) return;
          this.marketDataDir = path;
          this.statusLine = "Selected market data folder.";
          markVueUpdate(this);
        })
        .catch((error: unknown) => {
          this.statusLine =
            `Market data browse failed: ${redactDataStoreText(this, error)}`.trim();
          markVueUpdate(this);
        });
    },

    setMarketDataDir(dir: string, moveExisting: boolean): void {
      const nextDir = String(dir || "").trim();
      const nextMoveExisting = !!moveExisting;
      if (nextDir) {
        this.marketDataDir = nextDir;
      }
      this.marketDataMoveExisting = nextMoveExisting;
      markVueUpdate(this);

      void setMarketDataDirAction(nextDir, nextMoveExisting)
        .then((info) => {
          const previousCacheDir = String(this.polygonCacheDir || "").trim();
          const nextCacheDir = String(info.polygonCacheDir || "").trim();
          const cacheDirChanged = applyMarketDataDirInfo(this, info);
          let preferredSelectedPath = "";
          if (cacheDirChanged) {
            if (nextMoveExisting) {
              preferredSelectedPath = rewriteMovedPolygonStartupState(
                this,
                previousCacheDir,
                nextCacheDir,
              );
            }
            resetPolygonCacheRuntimeState(
              this,
              nextCacheDir,
              preferredSelectedPath,
            );
            if (this.polygonOpen) {
              void ensurePolygonCacheLoaded(this).catch(() => undefined);
            }
          } else {
            this.polygonCacheDir = nextCacheDir;
          }
          this.statusLine = "Updated market data folder.";
          markVueUpdate(this);
        })
        .catch((error: unknown) => {
          this.statusLine =
            redactDataStoreText(this, error) ||
            "Failed to update market data folder.";
          markVueUpdate(this);
        });
    },

    resetMarketDataDir(): void {
      void resetMarketDataDirAction()
        .then((info) => {
          const nextCacheDir = String(info.polygonCacheDir || "").trim();
          const cacheDirChanged = applyMarketDataDirInfo(this, info);
          if (cacheDirChanged) {
            resetPolygonCacheRuntimeState(this, nextCacheDir);
            if (this.polygonOpen) {
              void ensurePolygonCacheLoaded(this).catch(() => undefined);
            }
          } else {
            this.polygonCacheDir = nextCacheDir;
          }
          this.statusLine = "Reset market data folder to default.";
          markVueUpdate(this);
        })
        .catch((error: unknown) => {
          this.statusLine =
            redactDataStoreText(this, error) ||
            "Failed to reset market data folder.";
          markVueUpdate(this);
        });
    },

    openMarketDataDir(): void {
      void openMarketDataDirAction()
        .then((result) => {
          const path = String(result?.path || "").trim();
          if (!path) return;
          this.statusLine = `${result.openedWith === "reveal_path" ? "Revealed" : "Opened"}: ${path}`;
          markVueUpdate(this);
          if (!this.marketDataDirHint || this.marketDataDirHint === "—") {
            void getMarketDataDirInfoAction()
              .then((info) => {
                this.marketDataDirHint =
                  String(info?.hint || this.marketDataDirHint || "—").trim() ||
                  "—";
                markVueUpdate(this);
              })
              .catch(() => undefined);
          }
        })
        .catch((error: unknown) => {
          this.statusLine =
            redactDataStoreText(this, error) ||
            "Failed to open market data folder.";
          markVueUpdate(this);
        });
    },

    loadSampleCsv(index: number, path?: string): void {
      setControlsBusy(this, true);
      this.statusLine = "Loading data...";
      markVueUpdate(this);

      void loadSampleCsvAction(index, path)
        .then((result) => {
          if (!result) return;
          this.applyLoadResult(result, {
            treatAsBase: true,
            statusLine: baseLoadStatusText(result, "csv"),
            source: "csv",
          });
        })
        .catch((error: unknown) => {
          this.statusLine = redactDataStoreText(this, error) || "Data load failed.";
          markVueUpdate(this);
        })
        .finally(() => {
          setControlsBusy(this, false);
          markVueUpdate(this);
        });
    },
  },
});
