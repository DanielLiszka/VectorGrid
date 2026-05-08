import { invokeCommand } from "../tauri/invoke";
import type { WindowEffect } from "../utils/windowEffects";
import { readCachedCatalog, writeCachedCatalog } from "./catalogReadCache";
import { artifactCatalogSummaryQueryKey } from "./catalogQueryKeys";
import {
  requireBoundaryArray,
  requireBoundaryObject,
} from "./serviceBoundarySchemas";

export const ARTIFACT_RETENTION_FAMILY_ORDER = [
  "run",
  "meta",
  "stage5",
  "mlPolicyEval",
  "batchSummary",
  "campaignSummary",
  "optionsExpression",
] as const;

export type ArtifactRetentionFamilyKey =
  (typeof ARTIFACT_RETENTION_FAMILY_ORDER)[number];

export interface ArtifactRetentionFamilyMeta {
  label: string;
  description: string;
  envVar: string;
}

export const ARTIFACT_RETENTION_FAMILY_META: Record<
  ArtifactRetentionFamilyKey,
  ArtifactRetentionFamilyMeta
> = {
  run: {
    label: "Run artifacts",
    description: "Saved optimization and validation runs.",
    envVar: "VECTORGRID_RUN_ARTIFACT_KEEP_NEWEST",
  },
  meta: {
    label: "Meta artifacts",
    description: "Saved meta-selection outputs.",
    envVar: "VECTORGRID_META_ARTIFACT_KEEP_NEWEST",
  },
  stage5: {
    label: "Stage 5 artifacts",
    description: "Saved Stage 5 ML-confidence results.",
    envVar: "VECTORGRID_STAGE5_ARTIFACT_KEEP_NEWEST",
  },
  mlPolicyEval: {
    label: "ML policy evaluations",
    description: "Offline ML policy evaluation artifacts.",
    envVar: "VECTORGRID_ML_POLICY_EVAL_ARTIFACT_KEEP_NEWEST",
  },
  batchSummary: {
    label: "Batch summaries",
    description: "Saved batch summary artifacts.",
    envVar: "VECTORGRID_BATCH_SUMMARY_ARTIFACT_KEEP_NEWEST",
  },
  campaignSummary: {
    label: "Campaign summaries",
    description: "Saved research campaign summaries.",
    envVar: "VECTORGRID_CAMPAIGN_SUMMARY_ARTIFACT_KEEP_NEWEST",
  },
  optionsExpression: {
    label: "Options expression artifacts",
    description: "Saved options expression runs and follow-ups.",
    envVar: "VECTORGRID_OPTIONS_EXPRESSION_ARTIFACT_KEEP_NEWEST",
  },
};

type ArtifactRetentionRawField =
  | "run_keep_newest"
  | "meta_keep_newest"
  | "stage5_keep_newest"
  | "ml_policy_eval_keep_newest"
  | "batch_summary_keep_newest"
  | "campaign_summary_keep_newest"
  | "options_expression_keep_newest";

const ARTIFACT_RETENTION_FIELD_BY_FAMILY: Record<
  ArtifactRetentionFamilyKey,
  ArtifactRetentionRawField
> = {
  run: "run_keep_newest",
  meta: "meta_keep_newest",
  stage5: "stage5_keep_newest",
  mlPolicyEval: "ml_policy_eval_keep_newest",
  batchSummary: "batch_summary_keep_newest",
  campaignSummary: "campaign_summary_keep_newest",
  optionsExpression: "options_expression_keep_newest",
};

type ArtifactRetentionRawSettings = Partial<
  Record<ArtifactRetentionRawField, unknown>
>;

interface ArtifactRetentionInfoResponse {
  defaults?: unknown;
  configured?: unknown;
  env_overrides?: unknown;
  effective?: unknown;
}

interface ArtifactLibrarySnapshotResponse {
  runs?: unknown;
  meta?: unknown;
  stage5?: unknown;
  policyEvals?: unknown;
  policy_evals?: unknown;
  optionsExpression?: unknown;
  options_expression?: unknown;
}

interface MarketDataDirInfoResponse {
  default_root_dir?: unknown;
  configured_root_dir?: unknown;
  effective_root_dir?: unknown;
  polygon_cache_dir?: unknown;
}

interface PolygonCacheInfoResponse {
  cache_dir?: unknown;
  has_api_key?: unknown;
  credential_status?: unknown;
  datasets?: unknown;
}

interface PolygonCredentialStatusResponse {
  has_api_key?: unknown;
  state?: unknown;
  storage_backend?: unknown;
  degraded?: unknown;
  message?: unknown;
}

export type ArtifactRetentionConfiguredSettingsSummary = Record<
  ArtifactRetentionFamilyKey,
  number | null
>;
export type ArtifactRetentionResolvedSettingsSummary = Record<
  ArtifactRetentionFamilyKey,
  number
>;

export interface ArtifactRetentionInfoSummary {
  defaults: ArtifactRetentionResolvedSettingsSummary;
  configured: ArtifactRetentionConfiguredSettingsSummary;
  envOverrides: ArtifactRetentionConfiguredSettingsSummary;
  effective: ArtifactRetentionResolvedSettingsSummary;
}

export interface ArtifactCatalogSummary {
  runs: number;
  meta: number;
  stage5: number;
  policyEvals: number;
  optionsExpression: number;
  total: number;
}

export interface LocalDataInventoryPathSummary {
  defaultRootDir: string;
  configuredRootDir: string;
  effectiveRootDir: string;
  polygonCacheDir: string;
}

export interface LocalDataInventoryCredentialSummary {
  polygonHasApiKey: boolean;
  polygonCredentialState: string;
  polygonCredentialStorageBackend: string;
  polygonCredentialDegraded: boolean;
  polygonCredentialMessage: string;
}

export interface LocalDataInventorySummary {
  generatedAtUnixMs: number;
  scanMode: "dry_run";
  rawMarketDataContentsScanned: false;
  pathSummary: LocalDataInventoryPathSummary;
  credentialSummary: LocalDataInventoryCredentialSummary;
  polygonDatasetCount: number;
  artifactCatalog: ArtifactCatalogSummary;
  artifactRetention: ArtifactRetentionInfoSummary;
  warnings: string[];
}

function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
}

function normalizeOptionalPositiveInteger(
  value: unknown,
  context: string,
): number | null {
  if (value == null) return null;
  return requirePositiveInteger(value, context);
}

function requirePositiveInteger(value: unknown, context: string): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    throw new Error(`${context} must be a positive integer.`);
  }
  return Math.floor(parsed);
}

function buildOptionalArtifactRetentionSettings(): ArtifactRetentionConfiguredSettingsSummary {
  return {
    run: null,
    meta: null,
    stage5: null,
    mlPolicyEval: null,
    batchSummary: null,
    campaignSummary: null,
    optionsExpression: null,
  };
}

function buildResolvedArtifactRetentionSettings(): ArtifactRetentionResolvedSettingsSummary {
  return {
    run: 200,
    meta: 100,
    stage5: 100,
    mlPolicyEval: 100,
    batchSummary: 100,
    campaignSummary: 100,
    optionsExpression: 100,
  };
}

function normalizeOptionalArtifactRetentionSettings(
  raw: unknown,
  context: string,
): ArtifactRetentionConfiguredSettingsSummary {
  const row = requireBoundaryObject(
    raw,
    context,
  ) as ArtifactRetentionRawSettings;
  const summary = buildOptionalArtifactRetentionSettings();
  for (const family of ARTIFACT_RETENTION_FAMILY_ORDER) {
    summary[family] = normalizeOptionalPositiveInteger(
      row[ARTIFACT_RETENTION_FIELD_BY_FAMILY[family]],
      `${context}.${ARTIFACT_RETENTION_FIELD_BY_FAMILY[family]}`,
    );
  }
  return summary;
}

function normalizeResolvedArtifactRetentionSettings(
  raw: unknown,
  context: string,
): ArtifactRetentionResolvedSettingsSummary {
  const row = requireBoundaryObject(
    raw,
    context,
  ) as ArtifactRetentionRawSettings;
  const summary = buildResolvedArtifactRetentionSettings();
  for (const family of ARTIFACT_RETENTION_FAMILY_ORDER) {
    summary[family] = requirePositiveInteger(
      row[ARTIFACT_RETENTION_FIELD_BY_FAMILY[family]],
      `${context}.${ARTIFACT_RETENTION_FIELD_BY_FAMILY[family]}`,
    );
  }
  return summary;
}

function normalizeArtifactRetentionInfo(
  raw: unknown,
): ArtifactRetentionInfoSummary {
  const info = requireBoundaryObject(
    raw,
    "artifact retention response",
  ) as ArtifactRetentionInfoResponse;
  return {
    defaults: normalizeResolvedArtifactRetentionSettings(
      info.defaults,
      "artifact retention defaults",
    ),
    configured: normalizeOptionalArtifactRetentionSettings(
      info.configured,
      "artifact retention configured settings",
    ),
    envOverrides: normalizeOptionalArtifactRetentionSettings(
      info.env_overrides,
      "artifact retention environment overrides",
    ),
    effective: normalizeResolvedArtifactRetentionSettings(
      info.effective,
      "artifact retention effective settings",
    ),
  };
}

function countRequiredArray(value: unknown, context: string): number {
  return requireBoundaryArray(value, context).length;
}

function countRequiredAliasArray(
  row: Record<string, unknown>,
  firstKey: string,
  secondKey: string,
  context: string,
): number {
  if (row[firstKey] != null)
    return countRequiredArray(row[firstKey], `${context}.${firstKey}`);
  if (row[secondKey] != null)
    return countRequiredArray(row[secondKey], `${context}.${secondKey}`);
  throw new Error(`${context}.${firstKey} must be an array.`);
}

function normalizeArtifactCatalogSummary(raw: unknown): ArtifactCatalogSummary {
  const row = requireBoundaryObject(
    raw,
    "artifact library response",
  ) as ArtifactLibrarySnapshotResponse & Record<string, unknown>;
  const runs = countRequiredArray(row.runs, "artifact library response.runs");
  const meta = countRequiredArray(row.meta, "artifact library response.meta");
  const stage5 = countRequiredArray(
    row.stage5,
    "artifact library response.stage5",
  );
  const policyEvals = countRequiredAliasArray(
    row,
    "policyEvals",
    "policy_evals",
    "artifact library response",
  );
  const optionsExpression = countRequiredAliasArray(
    row,
    "optionsExpression",
    "options_expression",
    "artifact library response",
  );
  return {
    runs,
    meta,
    stage5,
    policyEvals,
    optionsExpression,
    total: runs + meta + stage5 + policyEvals + optionsExpression,
  };
}

function normalizeMarketDataDirInfo(
  raw: unknown,
): LocalDataInventoryPathSummary {
  const row = requireBoundaryObject(
    raw,
    "market data directory response",
  ) as MarketDataDirInfoResponse;
  return {
    defaultRootDir: normalizeText(row.default_root_dir),
    configuredRootDir: normalizeText(row.configured_root_dir),
    effectiveRootDir: normalizeText(row.effective_root_dir),
    polygonCacheDir: normalizeText(row.polygon_cache_dir),
  };
}

function normalizePolygonCredentialStatus(
  raw: unknown,
  fallbackHasApiKey: boolean,
): LocalDataInventoryCredentialSummary {
  const row =
    raw == null
      ? {}
      : (requireBoundaryObject(
          raw,
          "Polygon credential status response",
        ) as PolygonCredentialStatusResponse);
  const state =
    normalizeText(row.state) || (fallbackHasApiKey ? "unknown" : "missing");
  return {
    polygonHasApiKey:
      row.has_api_key == null ? fallbackHasApiKey : Boolean(row.has_api_key),
    polygonCredentialState: state,
    polygonCredentialStorageBackend:
      normalizeText(row.storage_backend) || state,
    polygonCredentialDegraded: Boolean(row.degraded),
    polygonCredentialMessage: normalizeText(row.message),
  };
}

function normalizePolygonCacheInventory(raw: unknown): {
  cacheDir: string;
  credentialSummary: LocalDataInventoryCredentialSummary;
  datasetCount: number;
} {
  const row = requireBoundaryObject(
    raw,
    "Polygon cache response",
  ) as PolygonCacheInfoResponse;
  const hasApiKey = Boolean(row.has_api_key);
  return {
    cacheDir: normalizeText(row.cache_dir),
    credentialSummary: normalizePolygonCredentialStatus(
      row.credential_status,
      hasApiKey,
    ),
    datasetCount: requireBoundaryArray(
      row.datasets,
      "Polygon cache response.datasets",
    ).length,
  };
}

function buildLocalDataInventoryWarnings(args: {
  pathSummary: LocalDataInventoryPathSummary;
  polygonCacheDir: string;
  credentialSummary: LocalDataInventoryCredentialSummary;
  artifactCatalog: ArtifactCatalogSummary;
}): string[] {
  const warnings: string[] = [];
  if (
    args.polygonCacheDir &&
    args.pathSummary.polygonCacheDir &&
    args.polygonCacheDir !== args.pathSummary.polygonCacheDir
  ) {
    warnings.push(
      "Polygon cache path differs between directory and cache summaries.",
    );
  }
  if (args.credentialSummary.polygonCredentialDegraded) {
    warnings.push("Polygon credential storage is degraded.");
  }
  if (args.artifactCatalog.total === 0) {
    warnings.push("Artifact catalog is empty or has not been built yet.");
  }
  return warnings;
}

function serializeArtifactRetentionSettings(
  settings: ArtifactRetentionConfiguredSettingsSummary,
): Partial<Record<ArtifactRetentionRawField, number>> {
  const payload: Partial<Record<ArtifactRetentionRawField, number>> = {};
  for (const family of ARTIFACT_RETENTION_FAMILY_ORDER) {
    const value = settings[family];
    if (value == null) continue;
    payload[ARTIFACT_RETENTION_FIELD_BY_FAMILY[family]] =
      requirePositiveInteger(value, `${family} artifact retention setting`);
  }
  return payload;
}

export async function setWindowEffect(
  effect: WindowEffect,
  dark: boolean,
): Promise<void> {
  await invokeCommand("set_window_effect", {
    effect: effect === "none" ? null : effect,
    dark: !!dark,
  });
}

export async function openRunsDir(): Promise<void> {
  await invokeCommand("open_runs_dir");
}

export async function openReportsDir(): Promise<void> {
  await invokeCommand("open_reports_dir");
}

export async function getArtifactRetentionInfo(): Promise<ArtifactRetentionInfoSummary> {
  return normalizeArtifactRetentionInfo(
    await invokeCommand("get_artifact_retention_info"),
  );
}

export async function setArtifactRetentionSettings(
  settings: ArtifactRetentionConfiguredSettingsSummary,
): Promise<ArtifactRetentionInfoSummary> {
  return normalizeArtifactRetentionInfo(
    await invokeCommand("set_artifact_retention_settings", {
      settings: serializeArtifactRetentionSettings(settings),
    }),
  );
}

export async function clearArtifactRetentionSettings(): Promise<ArtifactRetentionInfoSummary> {
  return normalizeArtifactRetentionInfo(
    await invokeCommand("clear_artifact_retention_settings"),
  );
}

export async function loadArtifactCatalogSummary(): Promise<ArtifactCatalogSummary> {
  return readCachedCatalog(artifactCatalogSummaryQueryKey, async () =>
    normalizeArtifactCatalogSummary(
      await invokeCommand("list_artifact_library"),
    ),
  );
}

export async function rebuildArtifactCatalogSummary(): Promise<ArtifactCatalogSummary> {
  const summary = normalizeArtifactCatalogSummary(
    await invokeCommand("rebuild_artifact_catalog"),
  );
  writeCachedCatalog(artifactCatalogSummaryQueryKey, summary);
  return summary;
}

export async function buildLocalDataInventoryDryRun(): Promise<LocalDataInventorySummary> {
  const [
    marketDataInfoRaw,
    polygonCacheInfoRaw,
    artifactRetentionRaw,
    artifactCatalogRaw,
  ] = await Promise.all([
    invokeCommand("get_market_data_dir_info"),
    invokeCommand("polygon_get_cache_info"),
    invokeCommand("get_artifact_retention_info"),
    invokeCommand("list_artifact_library"),
  ]);

  const pathSummary = normalizeMarketDataDirInfo(marketDataInfoRaw);
  const polygonCache = normalizePolygonCacheInventory(polygonCacheInfoRaw);
  const artifactCatalog = normalizeArtifactCatalogSummary(artifactCatalogRaw);
  const artifactRetention =
    normalizeArtifactRetentionInfo(artifactRetentionRaw);

  return {
    generatedAtUnixMs: Date.now(),
    scanMode: "dry_run",
    rawMarketDataContentsScanned: false,
    pathSummary,
    credentialSummary: polygonCache.credentialSummary,
    polygonDatasetCount: polygonCache.datasetCount,
    artifactCatalog,
    artifactRetention,
    warnings: buildLocalDataInventoryWarnings({
      pathSummary,
      polygonCacheDir: polygonCache.cacheDir,
      credentialSummary: polygonCache.credentialSummary,
      artifactCatalog,
    }),
  };
}
