import { invokeCommand } from "../tauri/invoke";

function normalizeText(value: unknown): string {
  return String(value || "").trim();
}

function normalizeInteger(value: unknown, fallback = 0): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  if (parsed < 0) return 0;
  return Math.floor(parsed);
}

function normalizeUnixTs(value: unknown): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return 0;
  return Math.floor(parsed);
}

function normalizeMaybeNumber(value: unknown): number | null {
  if (value == null) return null;
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return null;
  return parsed;
}

interface MarketDataDirInfoResponse {
  default_root_dir?: unknown;
  configured_root_dir?: unknown;
  effective_root_dir?: unknown;
  polygon_cache_dir?: unknown;
}

interface PolygonDatasetIntegrityResponse {
  checked_utc_ms?: unknown;
  parse_errors?: unknown;
  duplicates?: unknown;
  out_of_order?: unknown;
  same_day_gaps_gt_2min?: unknown;
}

interface PolygonDatasetMetaResponse {
  market?: unknown;
  display_symbol?: unknown;
  polygon_ticker?: unknown;
  timeframe?: unknown;
  csv_path?: unknown;
  min_ts?: unknown;
  max_ts?: unknown;
  last_updated_utc_ms?: unknown;
  row_count?: unknown;
  adjusted?: unknown;
  integrity?: unknown;
}

interface PolygonCacheInfoResponse {
  cache_dir?: unknown;
  has_api_key?: unknown;
  datasets?: unknown;
}

interface PolygonDownloadAndLoadResponse {
  csv_path?: unknown;
  downloaded_bars?: unknown;
  used_cache_only?: unknown;
  base_load?: unknown;
  id?: unknown;
  meta?: unknown;
  cache_hit?: unknown;
  cache_path?: unknown;
  load_source?: unknown;
}

interface SampleCsvEntry {
  label?: unknown;
  path?: unknown;
}

interface PriceDataMetaResponse {
  len?: unknown;
  has_open?: unknown;
  has_high?: unknown;
  has_low?: unknown;
  has_close?: unknown;
  has_volume?: unknown;
  min_ts?: unknown;
  max_ts?: unknown;
  timeframe?: unknown;
  source_size_bytes?: unknown;
  source_mtime_utc_ms?: unknown;
}

interface LoadPriceDataResponse {
  id?: unknown;
  meta?: unknown;
  cache_hit?: unknown;
  cache_path?: unknown;
  load_source?: unknown;
}

export type DataLoadSource =
  | "binary_cache"
  | "csv_parsed_and_cached"
  | "csv_parsed_cache_write_skipped"
  | "in_memory_base_reuse"
  | "derived_fresh_resample"
  | "derived_in_memory_reuse";

export interface DataMetaSummary {
  len: number;
  hasOpen: boolean;
  hasHigh: boolean;
  hasLow: boolean;
  hasClose: boolean;
  hasVolume: boolean;
  minTs: number;
  maxTs: number;
  timeframe: string;
  sourceSizeBytes: number | null;
  sourceMtimeUtcMs: number | null;
}

export interface DataLoadResult {
  path: string;
  dataId: string;
  meta: DataMetaSummary;
  cacheHit: boolean;
  cachePath: string;
  loadSource: DataLoadSource;
}

export interface DataSampleCsvSummary {
  label: string;
  path: string;
  disabled: boolean;
}

export interface MarketDataDirInfoSummary {
  defaultRootDir: string;
  configuredRootDir: string;
  effectiveRootDir: string;
  polygonCacheDir: string;
  hint: string;
}

export interface MarketDataDirOpenResult {
  path: string;
  openedWith: "open_path" | "reveal_path";
}

export interface PolygonDatasetIntegritySummary {
  checkedUtcMs: number;
  parseErrors: number;
  duplicates: number;
  outOfOrder: number;
  sameDayGapsGt2Min: number;
}

export interface PolygonDatasetSummary {
  market: string;
  displaySymbol: string;
  polygonTicker: string;
  timeframe: string;
  csvPath: string;
  minTs: number;
  maxTs: number;
  lastUpdatedUtcMs: number;
  rowCount: number;
  adjusted: boolean;
  integrity: PolygonDatasetIntegritySummary | null;
}

export interface PolygonCacheInfoSummary {
  cacheDir: string;
  hasApiKey: boolean;
  datasets: PolygonDatasetSummary[];
}

export interface PolygonDownloadRequestSummary {
  market: string;
  symbol: string;
  from: string;
  to: string;
  adjusted: boolean;
  timeframe: string;
}

export interface PolygonTopOffRequestSummary {
  csvPath: string;
  timeframe?: string;
}

export interface PolygonDownloadAndLoadSummary {
  csvPath: string;
  downloadedBars: number;
  usedCacheOnly: boolean;
  baseLoad: DataLoadResult;
  load: DataLoadResult;
}

function ensureValidPolygonDownloadAndLoadSummary(
  summary: PolygonDownloadAndLoadSummary,
  action: "download" | "top-off",
): PolygonDownloadAndLoadSummary {
  if (!summary.csvPath) {
    throw new Error(`Polygon ${action} returned an invalid csv path.`);
  }
  if (!summary.baseLoad.dataId) {
    throw new Error(`Polygon ${action} returned an invalid base dataset.`);
  }
  if (!summary.load.dataId) {
    throw new Error(`Polygon ${action} returned an invalid loaded dataset.`);
  }
  return summary;
}

function buildMarketDataDirHint(summary: MarketDataDirInfoSummary): string {
  const polygonCache = summary.polygonCacheDir || "—";
  if (summary.configuredRootDir) {
    return `Using custom root: ${summary.configuredRootDir} · Polygon cache: ${polygonCache}`;
  }
  return `Using default root: ${summary.defaultRootDir || "—"} · Polygon cache: ${polygonCache}`;
}

function normalizeMarketDataDirInfo(raw: unknown): MarketDataDirInfoSummary {
  const info = (
    raw && typeof raw === "object" ? raw : {}
  ) as MarketDataDirInfoResponse;
  const defaultRootDir = normalizeText(info.default_root_dir);
  const configuredRootDir = normalizeText(info.configured_root_dir);
  const effectiveRootDir = normalizeText(info.effective_root_dir);
  const polygonCacheDir = normalizeText(info.polygon_cache_dir);
  const summary: MarketDataDirInfoSummary = {
    defaultRootDir,
    configuredRootDir,
    effectiveRootDir,
    polygonCacheDir,
    hint: "",
  };
  summary.hint = buildMarketDataDirHint(summary);
  return summary;
}

function normalizeDataMeta(raw: unknown): DataMetaSummary {
  const meta = (
    raw && typeof raw === "object" ? raw : {}
  ) as PriceDataMetaResponse;
  return {
    len: normalizeInteger(meta.len),
    hasOpen: Boolean(meta.has_open),
    hasHigh: Boolean(meta.has_high),
    hasLow: Boolean(meta.has_low),
    hasClose: Boolean(meta.has_close),
    hasVolume: Boolean(meta.has_volume),
    minTs: normalizeUnixTs(meta.min_ts),
    maxTs: normalizeUnixTs(meta.max_ts),
    timeframe: normalizeText(meta.timeframe),
    sourceSizeBytes: normalizeMaybeNumber(meta.source_size_bytes),
    sourceMtimeUtcMs: normalizeMaybeNumber(meta.source_mtime_utc_ms),
  };
}

function normalizeLoadSource(
  value: unknown,
  row?: LoadPriceDataResponse,
  path = "",
): DataLoadSource {
  const normalized = normalizeText(value).toLowerCase();
  switch (normalized) {
    case "binary_cache":
    case "csv_parsed_and_cached":
    case "csv_parsed_cache_write_skipped":
    case "in_memory_base_reuse":
    case "derived_fresh_resample":
    case "derived_in_memory_reuse":
      return normalized;
    default:
      if (row?.cache_hit) {
        if (normalizeText(row?.cache_path)) return "binary_cache";
        return normalizeText(path)
          ? "in_memory_base_reuse"
          : "derived_in_memory_reuse";
      }
      if (normalizeText(path)) {
        return normalizeText(row?.cache_path)
          ? "csv_parsed_and_cached"
          : "csv_parsed_cache_write_skipped";
      }
      return "derived_fresh_resample";
  }
}

function normalizeLoadPriceDataResponse(
  raw: unknown,
  path: string,
): DataLoadResult {
  const row = (
    raw && typeof raw === "object" ? raw : {}
  ) as LoadPriceDataResponse;
  return {
    path: normalizeText(path),
    dataId: normalizeText(row.id),
    meta: normalizeDataMeta(row.meta),
    cacheHit: Boolean(row.cache_hit),
    cachePath: normalizeText(row.cache_path),
    loadSource: normalizeLoadSource(row.load_source, row, path),
  };
}

function normalizePolygonDatasetIntegrity(
  raw: unknown,
): PolygonDatasetIntegritySummary | null {
  if (!raw || typeof raw !== "object") return null;
  const row = raw as PolygonDatasetIntegrityResponse;
  return {
    checkedUtcMs: normalizeUnixTs(row.checked_utc_ms),
    parseErrors: normalizeInteger(row.parse_errors),
    duplicates: normalizeInteger(row.duplicates),
    outOfOrder: normalizeInteger(row.out_of_order),
    sameDayGapsGt2Min: normalizeInteger(row.same_day_gaps_gt_2min),
  };
}

function normalizePolygonDatasetMeta(
  raw: unknown,
): PolygonDatasetSummary | null {
  if (!raw || typeof raw !== "object") return null;
  const row = raw as PolygonDatasetMetaResponse;
  const csvPath = normalizeText(row.csv_path);
  if (!csvPath) return null;
  return {
    market: normalizeText(row.market),
    displaySymbol: normalizeText(row.display_symbol),
    polygonTicker: normalizeText(row.polygon_ticker),
    timeframe: normalizeText(row.timeframe) || "1m",
    csvPath,
    minTs: normalizeUnixTs(row.min_ts),
    maxTs: normalizeUnixTs(row.max_ts),
    lastUpdatedUtcMs: normalizeUnixTs(row.last_updated_utc_ms),
    rowCount: normalizeInteger(row.row_count),
    adjusted: Boolean(row.adjusted),
    integrity: normalizePolygonDatasetIntegrity(row.integrity),
  };
}

function normalizePolygonCacheInfo(raw: unknown): PolygonCacheInfoSummary {
  const info = (
    raw && typeof raw === "object" ? raw : {}
  ) as PolygonCacheInfoResponse;
  const rows = Array.isArray(info.datasets) ? info.datasets : [];
  const datasets: PolygonDatasetSummary[] = [];
  for (const row of rows) {
    const normalized = normalizePolygonDatasetMeta(row);
    if (normalized) datasets.push(normalized);
  }
  return {
    cacheDir: normalizeText(info.cache_dir),
    hasApiKey: Boolean(info.has_api_key),
    datasets,
  };
}

function fileBasename(path: string): string {
  const normalized = normalizeText(path).replace(/\\/g, "/");
  if (!normalized) return "";
  const parts = normalized.split("/");
  return normalizeText(parts[parts.length - 1] || "");
}

export async function getMarketDataDirInfo(): Promise<MarketDataDirInfoSummary> {
  const info = await invokeCommand("get_market_data_dir_info");
  return normalizeMarketDataDirInfo(info);
}

export async function listSampleCsvs(): Promise<DataSampleCsvSummary[]> {
  const rows = (await invokeCommand("list_sample_csvs")) as unknown;
  const samples = Array.isArray(rows) ? rows : [];
  const out: DataSampleCsvSummary[] = [];
  for (const entry of samples) {
    const typed = (
      entry && typeof entry === "object" ? entry : {}
    ) as SampleCsvEntry;
    const path = normalizeText(typed.path);
    if (!path) continue;
    const label = normalizeText(typed.label) || fileBasename(path) || path;
    out.push({
      label,
      path,
      disabled: false,
    });
  }
  return out;
}

export async function browseCsv(): Promise<DataLoadResult | null> {
  try {
    const picked = await invokeCommand<string | null>("pick_csv_file");
    const path = normalizeText(picked);
    if (!path) return null;
    return await loadCsv(path);
  } catch {
    // Keep browse action non-throwing to match legacy runtime behavior.
    return null;
  }
}

export async function openPolygon(): Promise<void> {
  return undefined;
}

export async function closePolygon(): Promise<void> {
  return undefined;
}

export async function loadCsv(path?: string): Promise<DataLoadResult> {
  const nextPath = normalizeText(path);
  if (!nextPath) throw new Error("Enter a CSV path.");
  const response = await invokeCommand("load_price_data", { path: nextPath });
  return normalizeLoadPriceDataResponse(response, nextPath);
}

export async function clearData(baseDataId?: string): Promise<void> {
  const baseId = normalizeText(baseDataId);
  if (!baseId) return;
  await invokeCommand("clear_loaded_data", { baseDataId: baseId });
}

export async function applyTimeframe(
  timeframe: string,
  baseDataId?: string,
): Promise<DataLoadResult> {
  const nextTimeframe = normalizeText(timeframe);
  const baseId = normalizeText(baseDataId);
  if (!baseId) throw new Error("Load a dataset first (missing base data id).");
  if (!nextTimeframe) throw new Error("Choose a timeframe first.");
  const response = await invokeCommand("resample_price_data", {
    baseDataId: baseId,
    timeframe: nextTimeframe,
  });
  return normalizeLoadPriceDataResponse(response, "");
}

export async function browseMarketDataDir(): Promise<string> {
  const picked = await invokeCommand<string | null>("pick_folder");
  return normalizeText(picked);
}

export async function setMarketDataDir(
  dir: string,
  moveExisting: boolean,
): Promise<MarketDataDirInfoSummary> {
  const normalizedDir = normalizeText(dir);
  if (!normalizedDir) {
    throw new Error("Enter a market data folder path first.");
  }
  const info = await invokeCommand("set_market_data_dir", {
    dir: normalizedDir,
    moveExisting: !!moveExisting,
  });
  return normalizeMarketDataDirInfo(info);
}

export async function resetMarketDataDir(): Promise<MarketDataDirInfoSummary> {
  const info = await invokeCommand("clear_market_data_dir");
  return normalizeMarketDataDirInfo(info);
}

export async function openMarketDataDir(): Promise<MarketDataDirOpenResult> {
  const info = await getMarketDataDirInfo();
  const path = normalizeText(info.polygonCacheDir || info.effectiveRootDir);
  if (!path) throw new Error("No market data folder path available.");

  try {
    await invokeCommand("open_path", { path });
    return { path, openedWith: "open_path" };
  } catch {
    await invokeCommand("reveal_path", { path });
    return { path, openedWith: "reveal_path" };
  }
}

export async function loadSampleCsv(
  index: number,
  path?: string,
): Promise<DataLoadResult | null> {
  const nextPath = normalizeText(path);
  if (nextPath) {
    return await loadCsv(nextPath);
  }
  if (!Number.isFinite(index) || index < 0) return null;
  try {
    const samples = await listSampleCsvs();
    const entry = samples[Math.floor(index)] as
      | DataSampleCsvSummary
      | undefined;
    const samplePath = normalizeText(entry?.path);
    if (!samplePath) return null;
    return await loadCsv(samplePath);
  } catch {
    // Keep sample-load action non-throwing to match legacy runtime behavior.
    return null;
  }
}

export async function getPolygonCacheInfo(): Promise<PolygonCacheInfoSummary> {
  const raw = await invokeCommand("polygon_get_cache_info");
  return normalizePolygonCacheInfo(raw);
}

export async function setPolygonApiKey(
  apiKey: string,
): Promise<PolygonCacheInfoSummary> {
  const normalizedKey = normalizeText(apiKey);
  if (!normalizedKey) throw new Error("Enter a Polygon API key.");
  await invokeCommand("polygon_set_api_key", { apiKey: normalizedKey });
  return getPolygonCacheInfo();
}

export async function clearPolygonApiKey(): Promise<PolygonCacheInfoSummary> {
  await invokeCommand("polygon_clear_api_key");
  return getPolygonCacheInfo();
}

export async function downloadPolygonDataset(
  inputs: PolygonDownloadRequestSummary,
): Promise<PolygonDownloadAndLoadSummary> {
  const market = normalizeText(inputs.market || "stocks") || "stocks";
  const symbol = normalizeText(inputs.symbol).toUpperCase();
  const from = normalizeText(inputs.from);
  const to = normalizeText(inputs.to);
  const timeframe = normalizeText(inputs.timeframe || "1m") || "1m";
  if (!symbol) throw new Error("Enter a symbol.");
  if (!from) throw new Error("Choose a start date.");
  if (!to) throw new Error("Choose an end date.");
  const raw = (await invokeCommand("polygon_download_1m_and_load", {
    req: {
      market,
      symbol,
      from,
      to,
      adjusted: !!inputs.adjusted,
      timeframe,
    },
  })) as unknown;
  const row = (
    raw && typeof raw === "object" ? raw : {}
  ) as PolygonDownloadAndLoadResponse;
  return ensureValidPolygonDownloadAndLoadSummary({
    csvPath: normalizeText(row.csv_path),
    downloadedBars: normalizeInteger(row.downloaded_bars),
    usedCacheOnly: Boolean(row.used_cache_only),
    baseLoad: normalizeLoadPriceDataResponse(
      row.base_load,
      normalizeText(row.csv_path),
    ),
    load: normalizeLoadPriceDataResponse(row, normalizeText(row.csv_path)),
  }, "download");
}

export async function topOffPolygonDataset(
  inputs: PolygonTopOffRequestSummary,
): Promise<PolygonDownloadAndLoadSummary> {
  const csvPath = normalizeText(inputs.csvPath);
  const timeframe = normalizeText(inputs.timeframe || "");
  if (!csvPath) throw new Error("Select a Polygon dataset first.");
  const raw = (await invokeCommand("polygon_top_off_dataset", {
    req: {
      csv_path: csvPath,
      timeframe,
    },
  })) as unknown;
  const row = (
    raw && typeof raw === "object" ? raw : {}
  ) as PolygonDownloadAndLoadResponse;
  return ensureValidPolygonDownloadAndLoadSummary({
    csvPath: normalizeText(row.csv_path),
    downloadedBars: normalizeInteger(row.downloaded_bars),
    usedCacheOnly: Boolean(row.used_cache_only),
    baseLoad: normalizeLoadPriceDataResponse(
      row.base_load,
      normalizeText(row.csv_path),
    ),
    load: normalizeLoadPriceDataResponse(row, normalizeText(row.csv_path)),
  }, "top-off");
}

export async function validatePolygonDataset(
  csvPath: string,
): Promise<PolygonDatasetSummary> {
  const path = normalizeText(csvPath);
  if (!path) throw new Error("Select a Polygon dataset first.");
  const raw = await invokeCommand("polygon_validate_dataset", {
    csvPath: path,
  });
  const normalized = normalizePolygonDatasetMeta(raw);
  if (!normalized)
    throw new Error("Polygon validation returned an invalid dataset.");
  return normalized;
}

export async function repairPolygonDataset(
  csvPath: string,
): Promise<PolygonDatasetSummary> {
  const path = normalizeText(csvPath);
  if (!path) throw new Error("Select a Polygon dataset first.");
  const raw = await invokeCommand("polygon_repair_dataset", { csvPath: path });
  const normalized = normalizePolygonDatasetMeta(raw);
  if (!normalized)
    throw new Error("Polygon repair returned an invalid dataset.");
  return normalized;
}
