import { defineStore, getActivePinia } from "pinia";
import { markRaw } from "vue";

export interface WorkflowRunArtifact {
  ts: string;
  artifactPath: string;
  reportXlsxPath: string;
  csvPath: string;
  datasetMeta: Record<string, unknown> | null;
  mlSignal: WorkflowMlSignalArtifact | null;
  tags: string[];
  req: unknown;
  res: unknown;
}

export interface WorkflowValidationArtifact {
  ts: string;
  req: unknown;
  res: unknown;
}

interface WorkflowRunSnapshot {
  req: unknown;
  res: unknown;
}

export interface WorkflowMlSignalArtifact {
  bundleId: string;
  scoreCachePath: string;
  scoreManifestPath: string;
  featureSchemaPath: string;
  featureSchemaVersion: string;
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

function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
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
    bundle_id?: unknown;
    bundleId?: unknown;
    score_cache_path?: unknown;
    scoreCachePath?: unknown;
    score_manifest_path?: unknown;
    scoreManifestPath?: unknown;
    feature_schema_path?: unknown;
    featureSchemaPath?: unknown;
    feature_schema_version?: unknown;
    featureSchemaVersion?: unknown;
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
    bundleId: normalizeText(row.bundle_id ?? row.bundleId),
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
    policy: markRawUnknown(policy),
    policySweep: markRawUnknown(policySweep),
  };
  if (
    !out.bundleId &&
    !out.scoreCachePath &&
    !out.scoreManifestPath &&
    !out.featureSchemaPath &&
    !out.featureSchemaVersion &&
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
  return markRaw({
    ts: normalizeText(row.ts),
    artifactPath: normalizeText(row.artifact_path ?? row.artifactPath),
    reportXlsxPath: normalizeText(row.report_xlsx_path ?? row.reportXlsxPath),
    csvPath: normalizeText(row.csv_path ?? row.csvPath),
    datasetMeta: markRawUnknown(datasetMeta),
    mlSignal: normalizeMlSignalArtifact(row.ml_signal ?? row.mlSignal),
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
      applyWorkflowArtifactsPayload(payload, "legacy-runtime");
    },

    applyVueRuntimePayload(payload: unknown): void {
      applyWorkflowArtifactsPayload(payload, "vue");
    },

    clearArtifacts(): void {
      applyNormalizedArtifacts(null, null, "vue");
    },
  },
});

export function readLatestWorkflowRunSnapshot(): {
  req: unknown;
  res: unknown;
} | null {
  return latestWorkflowRunSnapshot;
}

function applyNormalizedArtifacts(
  run: WorkflowRunArtifact | null,
  validation: WorkflowValidationArtifact | null,
  source: WorkflowArtifactsSource,
): void {
  latestWorkflowRunSnapshot = run
    ? { req: run.req ?? null, res: run.res ?? null }
    : null;

  const pinia = getActivePinia();
  if (!pinia) return;
  const store = useWorkflowArtifactsStore(pinia);
  store.currentRun = run;
  store.currentValidation = validation;
  store.source = source;
  store.updatedAtUnixMs = Date.now();
}

export function applyWorkflowArtifactsPayload(
  payload: unknown,
  source: WorkflowArtifactsSource,
): boolean {
  if (!payload || typeof payload !== "object") return false;
  const row = payload as WorkflowArtifactsPayload;
  const run = normalizeRunArtifact(row.currentRun);
  const validation = normalizeValidationArtifact(row.currentValidation);
  applyNormalizedArtifacts(run, validation, source);
  return true;
}
