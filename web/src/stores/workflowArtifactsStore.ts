import { defineStore } from "pinia";
import { markRaw } from "vue";
import { resolveRuntimePinia } from "./runtimePinia";

export interface WorkflowRunArtifact {
  ts: string;
  artifactPath: string;
  reportXlsxPath: string;
  csvPath: string;
  datasetMeta: Record<string, unknown> | null;
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
    datasetMeta: run.datasetMeta,
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
    dataset_meta?: unknown;
    datasetMeta?: unknown;
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
    datasetMeta: markRawUnknown(datasetMeta),
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
