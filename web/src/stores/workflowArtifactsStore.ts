import { defineStore } from "pinia";
import { markRaw } from "vue";
import { resolveRuntimePinia } from "./runtimePinia";
import type {
  ArtifactLineageSummary,
  RouteProvenanceSummary,
} from "../utils/core/runArtifactReview";
import {
  parseArtifactLineage,
  parseRouteProvenance,
} from "../utils/core/runArtifactReview";

export interface WorkflowRunArtifact {
  ts: string;
  artifactPath: string;
  reportXlsxPath: string;
  csvPath: string;
  configFingerprint?: string;
  configFingerprintVersion?: number | null;
  datasetMeta: Record<string, unknown> | null;
  routeProvenance?: RouteProvenanceSummary | null;
  lineage?: ArtifactLineageSummary | null;
  mlSignal: WorkflowMlSignalArtifact | null;
  optimizationMetricSurface: Record<string, unknown> | null;
  tags: string[];
  req: unknown;
  res: unknown;
}

export interface WorkflowValidationArtifact {
  ts: string;
  req: unknown;
  res: unknown;
}

export interface WorkflowRunSnapshot {
  ts: string;
  artifactPath: string;
  configFingerprint?: string;
  routeProvenance?: RouteProvenanceSummary | null;
  req: unknown;
  res: unknown;
}

export interface WorkflowRunIdentitySnapshot {
  ts: string;
  artifactPath: string;
  hasValidation: boolean;
}

export interface WorkflowArtifactsApplyResult {
  applied: boolean;
  runChanged: boolean;
  validationChanged: boolean;
  sourceChanged: boolean;
}

export interface WorkflowMlSignalArtifact {
  stage: string;
  bundleId: string;
  datasetExportPath: string;
  exportManifestPath: string;
  splitManifestPath: string;
  rowCount: number | null;
  featureCount: number | null;
  splitCount: number | null;
  scoreCachePath: string;
  scoreManifestPath: string;
  featureSchemaPath: string;
  featureSchemaVersion: string;
  featureSchemaHash: string;
  exporter: Record<string, unknown> | null;
  provenance: Record<string, unknown> | null;
  calibration: Record<string, unknown> | null;
  scoreContract: Record<string, unknown> | null;
  scoreContractVersion: string;
  scoreCacheProvenance: Record<string, unknown> | null;
  bundleStatus: Record<string, unknown> | null;
  nativeBackendRequested: string;
  nativeBackendUsed: string;
  nativeLibraryPath: string;
  nativeGpuDeviceId: number | null;
  nativeFallbackReason: string;
  scoreSpace: string;
  scoreScope: string;
  matchedRows: number | null;
  skippedRows: number | null;
  barsScored: number | null;
  coverageRatio: number | null;
  duplicateTimestamps: number | null;
  policy: Record<string, unknown> | null;
  policySweep: Record<string, unknown> | null;
}

interface WorkflowArtifactsState {
  currentRun: WorkflowRunArtifact | null;
  currentValidation: WorkflowValidationArtifact | null;
  source: "legacy-runtime" | "vue" | "unknown";
  updatedAtUnixMs: number;
}

type WorkflowArtifactsSource = WorkflowArtifactsState["source"];

interface WorkflowArtifactsPayload {
  currentRun?: unknown;
  currentValidation?: unknown;
}

let latestWorkflowRunSnapshot: WorkflowRunSnapshot | null = null;
let latestWorkflowRunSignature = "null";
let latestWorkflowValidationSignature = "null";

function stableStringify(value: unknown): string {
  if (value == null) return "null";
  const kind = typeof value;
  if (kind === "number" || kind === "boolean") return JSON.stringify(value);
  if (kind === "string") return JSON.stringify(value);
  if (Array.isArray(value)) {
    return `[${value.map((entry) => stableStringify(entry)).join(",")}]`;
  }
  if (kind !== "object") return JSON.stringify(String(value));
  const row = value as Record<string, unknown>;
  const keys = Object.keys(row).sort();
  return `{${keys
    .map((key) => `${JSON.stringify(key)}:${stableStringify(row[key])}`)
    .join(",")}}`;
}

function workflowRunSignature(run: WorkflowRunArtifact | null): string {
  if (!run) return "null";
  return stableStringify({
    ts: run.ts,
    artifactPath: run.artifactPath,
    reportXlsxPath: run.reportXlsxPath,
    csvPath: run.csvPath,
    configFingerprint: run.configFingerprint,
    datasetMeta: run.datasetMeta,
    routeProvenance: run.routeProvenance,
    lineage: run.lineage,
    mlSignal: run.mlSignal,
    optimizationMetricSurface: run.optimizationMetricSurface,
    tags: run.tags,
    req: run.req,
    res: run.res,
  });
}

function workflowValidationSignature(
  validation: WorkflowValidationArtifact | null,
): string {
  if (!validation) return "null";
  return stableStringify({
    ts: validation.ts,
    req: validation.req,
    res: validation.res,
  });
}

function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
}

function normalizeOptionalNumber(value: unknown): number | null {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function normalizeTags(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  const out: string[] = [];
  for (const item of value) {
    const next = normalizeText(item);
    if (!next) continue;
    out.push(next);
  }
  return out;
}

function markRawUnknown<T>(value: T): T {
  if (!value) return value;
  if (typeof value !== "object") return value;
  return markRaw(value);
}

function normalizeOptionalRecord(
  value: unknown,
): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const row = value as Record<string, unknown>;
  return Object.keys(row).length > 0 ? row : null;
}

function hasMeaningfulMetadataValue(value: unknown): boolean {
  if (value == null) return false;
  if (typeof value === "string") return normalizeText(value).length > 0;
  if (typeof value === "number") return Number.isFinite(value);
  if (typeof value === "boolean") return true;
  if (Array.isArray(value)) return value.length > 0;
  if (typeof value === "object") {
    return Object.keys(value as Record<string, unknown>).length > 0;
  }
  return true;
}

function compactMetadataRecord(
  entries: Array<[string, unknown]>,
): Record<string, unknown> | null {
  const out: Record<string, unknown> = {};
  for (const [key, value] of entries) {
    if (!hasMeaningfulMetadataValue(value)) continue;
    out[key] = value;
  }
  return Object.keys(out).length > 0 ? out : null;
}

function normalizeMlSignalArtifact(
  value: unknown,
): WorkflowMlSignalArtifact | null {
  if (!value || typeof value !== "object") return null;
  const row = value as {
    stage?: unknown;
    bundle_id?: unknown;
    bundleId?: unknown;
    dataset_export_path?: unknown;
    datasetExportPath?: unknown;
    export_manifest_path?: unknown;
    exportManifestPath?: unknown;
    split_manifest_path?: unknown;
    splitManifestPath?: unknown;
    row_count?: unknown;
    rowCount?: unknown;
    feature_count?: unknown;
    featureCount?: unknown;
    split_count?: unknown;
    splitCount?: unknown;
    score_cache_path?: unknown;
    scoreCachePath?: unknown;
    score_manifest_path?: unknown;
    scoreManifestPath?: unknown;
    feature_schema_path?: unknown;
    featureSchemaPath?: unknown;
    feature_schema_version?: unknown;
    featureSchemaVersion?: unknown;
    feature_schema_hash?: unknown;
    featureSchemaHash?: unknown;
    schema_hash?: unknown;
    schemaHash?: unknown;
    exporter?: unknown;
    exporter_metadata?: unknown;
    exporterMetadata?: unknown;
    exporter_version?: unknown;
    exporterVersion?: unknown;
    provenance?: unknown;
    indicator_config?: unknown;
    indicatorConfig?: unknown;
    warmup_policy?: unknown;
    warmupPolicy?: unknown;
    label_policy?: unknown;
    labelPolicy?: unknown;
    deadband_policy?: unknown;
    deadbandPolicy?: unknown;
    sample_weight_policy?: unknown;
    sampleWeightPolicy?: unknown;
    data_timeframe?: unknown;
    dataTimeframe?: unknown;
    created_at?: unknown;
    createdAt?: unknown;
    source_export_manifest_hash?: unknown;
    sourceExportManifestHash?: unknown;
    source_feature_schema_hash?: unknown;
    sourceFeatureSchemaHash?: unknown;
    training_config?: unknown;
    trainingConfig?: unknown;
    training_windows_summary?: unknown;
    trainingWindowsSummary?: unknown;
    calibration?: unknown;
    calibration_summary?: unknown;
    calibrationSummary?: unknown;
    calibration_state?: unknown;
    calibrationState?: unknown;
    class_balance_policy?: unknown;
    classBalancePolicy?: unknown;
    threshold_policy_default?: unknown;
    thresholdPolicyDefault?: unknown;
    explainability_artifacts?: unknown;
    explainabilityArtifacts?: unknown;
    bundle_parent_id?: unknown;
    bundleParentId?: unknown;
    feature_block_versions?: unknown;
    featureBlockVersions?: unknown;
    indicator_params?: unknown;
    indicatorParams?: unknown;
    label_horizon_bars?: unknown;
    labelHorizonBars?: unknown;
    first_valid_reasoning?: unknown;
    firstValidReasoning?: unknown;
    score_contract?: unknown;
    scoreContract?: unknown;
    score_contract_version?: unknown;
    scoreContractVersion?: unknown;
    score_cache_provenance?: unknown;
    scoreCacheProvenance?: unknown;
    source_bundle_id?: unknown;
    sourceBundleId?: unknown;
    source_bundle_manifest_hash?: unknown;
    sourceBundleManifestHash?: unknown;
    threshold_policy?: unknown;
    thresholdPolicy?: unknown;
    producer_kind?: unknown;
    producerKind?: unknown;
    native_backend_requested?: unknown;
    nativeBackendRequested?: unknown;
    native_backend_used?: unknown;
    nativeBackendUsed?: unknown;
    native_library_path?: unknown;
    nativeLibraryPath?: unknown;
    native_gpu_device_id?: unknown;
    nativeGpuDeviceId?: unknown;
    native_fallback_reason?: unknown;
    nativeFallbackReason?: unknown;
    bundle_status?: unknown;
    bundleStatus?: unknown;
    bundle_status_details?: unknown;
    bundleStatusDetails?: unknown;
    score_space?: unknown;
    scoreSpace?: unknown;
    score_scope?: unknown;
    scoreScope?: unknown;
    matched_rows?: unknown;
    matchedRows?: unknown;
    skipped_rows?: unknown;
    skippedRows?: unknown;
    bars_scored?: unknown;
    barsScored?: unknown;
    coverage_ratio?: unknown;
    coverageRatio?: unknown;
    duplicate_timestamps?: unknown;
    duplicateTimestamps?: unknown;
    policy?: unknown;
    policy_sweep?: unknown;
    policySweep?: unknown;
  };
  const policy =
    row.policy && typeof row.policy === "object"
      ? (row.policy as Record<string, unknown>)
      : null;
  const policySweep =
    row.policy_sweep && typeof row.policy_sweep === "object"
      ? (row.policy_sweep as Record<string, unknown>)
      : row.policySweep && typeof row.policySweep === "object"
        ? (row.policySweep as Record<string, unknown>)
        : null;
  const exporter =
    normalizeOptionalRecord(
      row.exporter ?? row.exporter_metadata ?? row.exporterMetadata,
    ) ??
    compactMetadataRecord([
      ["exporter_version", row.exporter_version ?? row.exporterVersion],
      ["created_at", row.created_at ?? row.createdAt],
    ]);
  const provenance =
    normalizeOptionalRecord(row.provenance) ??
    compactMetadataRecord([
      ["indicator_config", row.indicator_config ?? row.indicatorConfig],
      ["warmup_policy", row.warmup_policy ?? row.warmupPolicy],
      ["label_policy", row.label_policy ?? row.labelPolicy],
      ["deadband_policy", row.deadband_policy ?? row.deadbandPolicy],
      [
        "sample_weight_policy",
        row.sample_weight_policy ?? row.sampleWeightPolicy,
      ],
      ["data_timeframe", row.data_timeframe ?? row.dataTimeframe],
      [
        "source_export_manifest_hash",
        row.source_export_manifest_hash ?? row.sourceExportManifestHash,
      ],
      [
        "source_feature_schema_hash",
        row.source_feature_schema_hash ?? row.sourceFeatureSchemaHash,
      ],
      ["training_config", row.training_config ?? row.trainingConfig],
      [
        "training_windows_summary",
        row.training_windows_summary ?? row.trainingWindowsSummary,
      ],
      [
        "calibration_summary",
        row.calibration_summary ?? row.calibrationSummary,
      ],
      [
        "class_balance_policy",
        row.class_balance_policy ?? row.classBalancePolicy,
      ],
      [
        "threshold_policy_default",
        row.threshold_policy_default ?? row.thresholdPolicyDefault,
      ],
      [
        "explainability_artifacts",
        row.explainability_artifacts ?? row.explainabilityArtifacts,
      ],
      ["bundle_parent_id", row.bundle_parent_id ?? row.bundleParentId],
      [
        "feature_block_versions",
        row.feature_block_versions ?? row.featureBlockVersions,
      ],
      ["indicator_params", row.indicator_params ?? row.indicatorParams],
      ["label_horizon_bars", row.label_horizon_bars ?? row.labelHorizonBars],
      [
        "first_valid_reasoning",
        row.first_valid_reasoning ?? row.firstValidReasoning,
      ],
    ]);
  const calibration =
    normalizeOptionalRecord(row.calibration) ??
    compactMetadataRecord([
      [
        "calibration_summary",
        row.calibration_summary ?? row.calibrationSummary,
      ],
      ["calibration_state", row.calibration_state ?? row.calibrationState],
    ]);
  const scoreContract =
    normalizeOptionalRecord(row.score_contract ?? row.scoreContract) ??
    compactMetadataRecord([
      [
        "score_contract_version",
        row.score_contract_version ?? row.scoreContractVersion,
      ],
    ]);
  const scoreCacheProvenance =
    normalizeOptionalRecord(
      row.score_cache_provenance ?? row.scoreCacheProvenance,
    ) ??
    compactMetadataRecord([
      ["source_bundle_id", row.source_bundle_id ?? row.sourceBundleId],
      [
        "source_bundle_manifest_hash",
        row.source_bundle_manifest_hash ?? row.sourceBundleManifestHash,
      ],
      ["calibration_state", row.calibration_state ?? row.calibrationState],
      ["threshold_policy", row.threshold_policy ?? row.thresholdPolicy],
      ["producer_kind", row.producer_kind ?? row.producerKind],
      [
        "native_backend_requested",
        row.native_backend_requested ?? row.nativeBackendRequested,
      ],
      ["native_backend_used", row.native_backend_used ?? row.nativeBackendUsed],
      ["native_library_path", row.native_library_path ?? row.nativeLibraryPath],
      [
        "native_gpu_device_id",
        row.native_gpu_device_id ?? row.nativeGpuDeviceId,
      ],
      [
        "native_fallback_reason",
        row.native_fallback_reason ?? row.nativeFallbackReason,
      ],
      [
        "score_contract_version",
        row.score_contract_version ?? row.scoreContractVersion,
      ],
    ]);
  const bundleStatus = normalizeOptionalRecord(
    row.bundle_status_details ??
      row.bundleStatusDetails ??
      row.bundle_status ??
      row.bundleStatus,
  );
  const out = {
    stage: normalizeText(row.stage),
    bundleId: normalizeText(row.bundle_id ?? row.bundleId),
    datasetExportPath: normalizeText(
      row.dataset_export_path ?? row.datasetExportPath,
    ),
    exportManifestPath: normalizeText(
      row.export_manifest_path ?? row.exportManifestPath,
    ),
    splitManifestPath: normalizeText(
      row.split_manifest_path ?? row.splitManifestPath,
    ),
    rowCount: normalizeOptionalNumber(row.row_count ?? row.rowCount),
    featureCount: normalizeOptionalNumber(
      row.feature_count ?? row.featureCount,
    ),
    splitCount: normalizeOptionalNumber(row.split_count ?? row.splitCount),
    scoreCachePath: normalizeText(row.score_cache_path ?? row.scoreCachePath),
    scoreManifestPath: normalizeText(
      row.score_manifest_path ?? row.scoreManifestPath,
    ),
    featureSchemaPath: normalizeText(
      row.feature_schema_path ?? row.featureSchemaPath,
    ),
    featureSchemaVersion: normalizeText(
      row.feature_schema_version ?? row.featureSchemaVersion,
    ),
    featureSchemaHash: normalizeText(
      row.feature_schema_hash ??
        row.featureSchemaHash ??
        row.schema_hash ??
        row.schemaHash,
    ),
    exporter: markRawUnknown(exporter),
    provenance: markRawUnknown(provenance),
    calibration: markRawUnknown(calibration),
    scoreContract: markRawUnknown(scoreContract),
    scoreContractVersion: normalizeText(
      row.score_contract_version ??
        row.scoreContractVersion ??
        scoreContract?.score_contract_version ??
        scoreContract?.scoreContractVersion ??
        scoreCacheProvenance?.score_contract_version,
    ),
    scoreCacheProvenance: markRawUnknown(scoreCacheProvenance),
    bundleStatus: markRawUnknown(bundleStatus),
    nativeBackendRequested: normalizeText(
      row.native_backend_requested ??
        row.nativeBackendRequested ??
        scoreCacheProvenance?.native_backend_requested ??
        scoreCacheProvenance?.nativeBackendRequested,
    ),
    nativeBackendUsed: normalizeText(
      row.native_backend_used ??
        row.nativeBackendUsed ??
        scoreCacheProvenance?.native_backend_used ??
        scoreCacheProvenance?.nativeBackendUsed,
    ),
    nativeLibraryPath: normalizeText(
      row.native_library_path ??
        row.nativeLibraryPath ??
        scoreCacheProvenance?.native_library_path ??
        scoreCacheProvenance?.nativeLibraryPath,
    ),
    nativeGpuDeviceId: normalizeOptionalNumber(
      row.native_gpu_device_id ??
        row.nativeGpuDeviceId ??
        scoreCacheProvenance?.native_gpu_device_id ??
        scoreCacheProvenance?.nativeGpuDeviceId,
    ),
    nativeFallbackReason: normalizeText(
      row.native_fallback_reason ??
        row.nativeFallbackReason ??
        scoreCacheProvenance?.native_fallback_reason ??
        scoreCacheProvenance?.nativeFallbackReason,
    ),
    scoreSpace: normalizeText(row.score_space ?? row.scoreSpace),
    scoreScope: normalizeText(row.score_scope ?? row.scoreScope),
    matchedRows: normalizeOptionalNumber(row.matched_rows ?? row.matchedRows),
    skippedRows: normalizeOptionalNumber(row.skipped_rows ?? row.skippedRows),
    barsScored: normalizeOptionalNumber(row.bars_scored ?? row.barsScored),
    coverageRatio: normalizeOptionalNumber(
      row.coverage_ratio ?? row.coverageRatio,
    ),
    duplicateTimestamps: normalizeOptionalNumber(
      row.duplicate_timestamps ?? row.duplicateTimestamps,
    ),
    policy: markRawUnknown(policy),
    policySweep: markRawUnknown(policySweep),
  };
  if (
    !out.stage &&
    !out.bundleId &&
    !out.datasetExportPath &&
    !out.exportManifestPath &&
    !out.splitManifestPath &&
    out.rowCount == null &&
    out.featureCount == null &&
    out.splitCount == null &&
    !out.scoreCachePath &&
    !out.scoreManifestPath &&
    !out.featureSchemaPath &&
    !out.featureSchemaVersion &&
    !out.featureSchemaHash &&
    !out.exporter &&
    !out.provenance &&
    !out.calibration &&
    !out.scoreContract &&
    !out.scoreContractVersion &&
    !out.scoreCacheProvenance &&
    !out.bundleStatus &&
    !out.scoreSpace &&
    !out.scoreScope &&
    out.matchedRows == null &&
    out.skippedRows == null &&
    out.barsScored == null &&
    out.coverageRatio == null &&
    out.duplicateTimestamps == null &&
    !out.policy &&
    !out.policySweep
  ) {
    return null;
  }
  return markRaw(out);
}

function normalizeRunArtifact(value: unknown): WorkflowRunArtifact | null {
  if (!value || typeof value !== "object") return null;
  const row = value as {
    ts?: unknown;
    artifact_path?: unknown;
    artifactPath?: unknown;
    report_xlsx_path?: unknown;
    reportXlsxPath?: unknown;
    csv_path?: unknown;
    csvPath?: unknown;
    config_fingerprint?: unknown;
    configFingerprint?: unknown;
    config_fingerprint_version?: unknown;
    configFingerprintVersion?: unknown;
    dataset_meta?: unknown;
    datasetMeta?: unknown;
    route_provenance?: unknown;
    routeProvenance?: unknown;
    lineage?: unknown;
    ml_signal?: unknown;
    mlSignal?: unknown;
    optimization_metric_surface?: unknown;
    optimizationMetricSurface?: unknown;
    tags?: unknown;
    req?: unknown;
    res?: unknown;
  };
  const datasetMeta =
    row.dataset_meta && typeof row.dataset_meta === "object"
      ? (row.dataset_meta as Record<string, unknown>)
      : row.datasetMeta && typeof row.datasetMeta === "object"
        ? (row.datasetMeta as Record<string, unknown>)
        : null;
  const optimizationMetricSurface =
    row.optimization_metric_surface &&
    typeof row.optimization_metric_surface === "object"
      ? (row.optimization_metric_surface as Record<string, unknown>)
      : row.optimizationMetricSurface &&
          typeof row.optimizationMetricSurface === "object"
        ? (row.optimizationMetricSurface as Record<string, unknown>)
        : null;
  return markRaw({
    ts: normalizeText(row.ts),
    artifactPath: normalizeText(row.artifact_path ?? row.artifactPath),
    reportXlsxPath: normalizeText(row.report_xlsx_path ?? row.reportXlsxPath),
    csvPath: normalizeText(row.csv_path ?? row.csvPath),
    configFingerprint: normalizeText(
      row.config_fingerprint ?? row.configFingerprint,
    ),
    configFingerprintVersion: normalizeOptionalNumber(
      row.config_fingerprint_version ?? row.configFingerprintVersion,
    ),
    datasetMeta: markRawUnknown(datasetMeta),
    routeProvenance: parseRouteProvenance(
      row.route_provenance ?? row.routeProvenance,
    ),
    lineage: parseArtifactLineage(row.lineage),
    mlSignal: normalizeMlSignalArtifact(row.ml_signal ?? row.mlSignal),
    optimizationMetricSurface: markRawUnknown(optimizationMetricSurface),
    tags: markRaw(normalizeTags(row.tags)),
    req: markRawUnknown(row.req ?? null),
    res: markRawUnknown(row.res ?? null),
  });
}

function normalizeValidationArtifact(
  value: unknown,
): WorkflowValidationArtifact | null {
  if (!value || typeof value !== "object") return null;
  const row = value as {
    ts?: unknown;
    req?: unknown;
    res?: unknown;
  };
  return markRaw({
    ts: normalizeText(row.ts),
    req: markRawUnknown(row.req ?? null),
    res: markRawUnknown(row.res ?? null),
  });
}

export const useWorkflowArtifactsStore = defineStore("workflowArtifacts", {
  state: (): WorkflowArtifactsState => ({
    currentRun: null,
    currentValidation: null,
    source: "unknown",
    updatedAtUnixMs: 0,
  }),

  getters: {
    hasRun(state): boolean {
      return !!state.currentRun?.res;
    },
    hasValidation(state): boolean {
      return !!state.currentValidation?.res;
    },
    currentRunTs(state): string {
      return String(state.currentRun?.ts || "").trim();
    },
  },

  actions: {
    applyLegacyRuntimePayload(payload: unknown): void {
      applyWorkflowArtifactsPayload(payload, "legacy-runtime", this);
    },

    applyVueRuntimePayload(payload: unknown): void {
      applyWorkflowArtifactsPayload(payload, "vue", this);
    },

    clearArtifacts(): void {
      applyNormalizedArtifacts(null, null, "vue", this);
    },
  },
});

export function readLatestWorkflowRunSnapshot(): {
  ts: string;
  artifactPath: string;
  configFingerprint?: string;
  routeProvenance?: RouteProvenanceSummary | null;
  req: unknown;
  res: unknown;
} | null {
  return latestWorkflowRunSnapshot;
}

export function readLatestWorkflowRunIdentitySnapshot(): WorkflowRunIdentitySnapshot | null {
  if (!latestWorkflowRunSnapshot) return null;
  return {
    ts: latestWorkflowRunSnapshot.ts,
    artifactPath: latestWorkflowRunSnapshot.artifactPath,
    hasValidation: latestWorkflowValidationSignature !== "null",
  };
}

function applyNormalizedArtifacts(
  run: WorkflowRunArtifact | null,
  validation: WorkflowValidationArtifact | null,
  source: WorkflowArtifactsSource,
  targetStore?: WorkflowArtifactsState,
): WorkflowArtifactsApplyResult {
  const nextRunSignature = workflowRunSignature(run);
  const nextValidationSignature = workflowValidationSignature(validation);
  const runChanged = nextRunSignature !== latestWorkflowRunSignature;
  const validationChanged =
    nextValidationSignature !== latestWorkflowValidationSignature;

  if (runChanged || validationChanged) {
    latestWorkflowRunSignature = nextRunSignature;
    latestWorkflowValidationSignature = nextValidationSignature;
    latestWorkflowRunSnapshot = run
      ? {
          ts: run.ts,
          artifactPath: run.artifactPath,
          configFingerprint: run.configFingerprint ?? "",
          routeProvenance: run.routeProvenance ?? null,
          req: run.req ?? null,
          res: run.res ?? null,
        }
      : null;
  }

  const store =
    targetStore ??
    (() => {
      const pinia = resolveRuntimePinia();
      return pinia ? useWorkflowArtifactsStore(pinia) : null;
    })();

  if (!store) {
    return {
      applied: runChanged || validationChanged,
      runChanged,
      validationChanged,
      sourceChanged: false,
    };
  }

  const sourceChanged = store.source !== source;
  const storeNeedsExplicitClear =
    !run &&
    !validation &&
    (store.currentRun !== null || store.currentValidation !== null);

  if (!runChanged && !validationChanged && !storeNeedsExplicitClear) {
    if (sourceChanged) store.source = source;
    return {
      applied: sourceChanged,
      runChanged: false,
      validationChanged: false,
      sourceChanged,
    };
  }

  store.currentRun = run;
  store.currentValidation = validation;
  store.source = source;
  store.updatedAtUnixMs = Date.now();
  return {
    applied: true,
    runChanged,
    validationChanged,
    sourceChanged,
  };
}

export function applyWorkflowArtifactsPayload(
  payload: unknown,
  source: WorkflowArtifactsSource,
  targetStore?: WorkflowArtifactsState,
): WorkflowArtifactsApplyResult {
  if (!payload || typeof payload !== "object") {
    return {
      applied: false,
      runChanged: false,
      validationChanged: false,
      sourceChanged: false,
    };
  }
  const row = payload as WorkflowArtifactsPayload;
  const run = normalizeRunArtifact(row.currentRun);
  const validation = normalizeValidationArtifact(row.currentValidation);
  return applyNormalizedArtifacts(run, validation, source, targetStore);
}
