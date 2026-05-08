import { invokeCommand } from "../tauri/invoke";
import {
  applyWorkflowArtifactsPayload,
  readLatestWorkflowRunIdentitySnapshot,
  useWorkflowArtifactsStore,
} from "../stores/workflowArtifactsStore";
import { resolveRuntimePinia } from "../stores/runtimePinia";
import { applyShellStateForRunRequest } from "../utils/core/workflowRunRestore";
import { normalizeStageCompletions } from "../utils/core/stageCompletion";
import { normalizeArtifactCompatibility } from "../utils/core/artifactEvidenceStatus";
import { refreshAnalysisFromWorkflowRun } from "./analysisService";
import { measureArtifactHistoryPerf } from "./artifactHistoryPerf";

type WorkflowArtifactsSource = "legacy-runtime" | "vue";

function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
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
  return out;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  return value as Record<string, unknown>;
}

function stableEqual(left: unknown, right: unknown): boolean {
  if (left === right) return true;
  if (left == null || right == null) return left == null && right == null;
  const leftKind = typeof left;
  const rightKind = typeof right;
  if (leftKind !== rightKind) return false;
  if (
    leftKind === "number" ||
    leftKind === "boolean" ||
    leftKind === "string"
  ) {
    return JSON.stringify(left) === JSON.stringify(right);
  }
  if (Array.isArray(left) || Array.isArray(right)) {
    if (!Array.isArray(left) || !Array.isArray(right)) return false;
    if (left.length !== right.length) return false;
    for (let i = 0; i < left.length; i += 1) {
      if (!stableEqual(left[i], right[i])) return false;
    }
    return true;
  }
  if (leftKind !== "object") {
    return JSON.stringify(String(left)) === JSON.stringify(String(right));
  }

  const leftRow = left as Record<string, unknown>;
  const rightRow = right as Record<string, unknown>;
  const leftKeys = Object.keys(leftRow);
  if (leftKeys.length !== Object.keys(rightRow).length) return false;
  for (const key of leftKeys) {
    if (!Object.prototype.hasOwnProperty.call(rightRow, key)) return false;
    if (!stableEqual(leftRow[key], rightRow[key])) return false;
  }
  return true;
}

function syncSourceIfStoreBound(source: WorkflowArtifactsSource): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  useWorkflowArtifactsStore(pinia).source = source;
}

function realignShellStateFromCurrentArtifacts(): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  const store = useWorkflowArtifactsStore(pinia);
  applyShellStateForRunRequest(
    asRecord(store.currentRun?.req),
    asRecord(store.currentValidation?.req),
    store.currentRun?.mlSignal?.stage,
  );
}

function latestSummaryMatchesCurrentStore(
  latest: Record<string, unknown>,
): boolean {
  const pinia = resolveRuntimePinia();
  if (!pinia) return true;
  const currentRun = useWorkflowArtifactsStore(pinia).currentRun;
  if (!currentRun) return true;
  const currentCalibration = asRecord(currentRun.mlSignal?.calibration);
  const currentScoreCacheProvenance = asRecord(
    currentRun.mlSignal?.scoreCacheProvenance,
  );
  const latestStageCompletions =
    latest.stage_completions ?? latest.stageCompletions;
  const compareStageCompletions = latestStageCompletions != null;
  const latestMl = {
    featureSchemaHash: normalizeText(
      latest.ml_feature_schema_hash ?? latest.mlFeatureSchemaHash,
    ),
    calibrationState: normalizeText(
      latest.ml_calibration_state ?? latest.mlCalibrationState,
    ),
    thresholdPolicy: normalizeText(
      latest.ml_threshold_policy ?? latest.mlThresholdPolicy,
    ),
    producerKind: normalizeText(
      latest.ml_producer_kind ?? latest.mlProducerKind,
    ),
    nativeBackendUsed: normalizeText(
      latest.ml_native_backend_used ?? latest.mlNativeBackendUsed,
    ),
    nativeFallbackReason: normalizeText(
      latest.ml_native_fallback_reason ?? latest.mlNativeFallbackReason,
    ),
    scoreContractVersion: normalizeText(
      latest.ml_score_contract_version ?? latest.mlScoreContractVersion,
    ),
    sourceBundleManifestHash: normalizeText(
      latest.ml_source_bundle_manifest_hash ??
        latest.mlSourceBundleManifestHash,
    ),
    scoreSpace: normalizeText(latest.ml_score_space ?? latest.mlScoreSpace),
    scoreScope: normalizeText(latest.ml_score_scope ?? latest.mlScoreScope),
  };
  const currentMl = {
    featureSchemaHash: normalizeText(currentRun.mlSignal?.featureSchemaHash),
    calibrationState: normalizeText(
      currentCalibration?.calibration_state ??
        currentCalibration?.calibrationState ??
        currentScoreCacheProvenance?.calibration_state ??
        currentScoreCacheProvenance?.calibrationState,
    ),
    thresholdPolicy: normalizeText(
      currentScoreCacheProvenance?.threshold_policy ??
        currentScoreCacheProvenance?.thresholdPolicy,
    ),
    producerKind: normalizeText(
      currentScoreCacheProvenance?.producer_kind ??
        currentScoreCacheProvenance?.producerKind,
    ),
    nativeBackendUsed: normalizeText(currentRun.mlSignal?.nativeBackendUsed),
    nativeFallbackReason: normalizeText(
      currentRun.mlSignal?.nativeFallbackReason,
    ),
    scoreContractVersion: normalizeText(
      currentRun.mlSignal?.scoreContractVersion,
    ),
    sourceBundleManifestHash: normalizeText(
      currentScoreCacheProvenance?.source_bundle_manifest_hash ??
        currentScoreCacheProvenance?.sourceBundleManifestHash,
    ),
    scoreSpace: normalizeText(currentRun.mlSignal?.scoreSpace),
    scoreScope: normalizeText(currentRun.mlSignal?.scoreScope),
  };
  const latestSummary = {
      reportXlsxPath: normalizeText(
        latest.report_xlsx_path ?? latest.reportXlsxPath,
      ),
      artifactKind: normalizeText(
        latest.kind ?? latest.artifact_kind ?? latest.artifactKind,
      ),
      artifactCompatibility: normalizeArtifactCompatibility(
        latest.artifact_compatibility ?? latest.artifactCompatibility,
        {
          artifactFamily: "run",
          artifactKind: normalizeText(
            latest.kind ?? latest.artifact_kind ?? latest.artifactKind,
          ),
        },
      ),
      reportXlsxExists: normalizeOptionalBoolean(
        latest.report_xlsx_exists ?? latest.reportXlsxExists,
      ),
      csvPath: normalizeText(latest.csv_path ?? latest.csvPath),
      configFingerprint: normalizeText(
        latest.config_fingerprint ?? latest.configFingerprint,
      ),
      configFingerprintVersion: normalizeText(
        latest.config_fingerprint_version ?? latest.configFingerprintVersion,
      ),
      routeProvenance:
        latest.route_provenance ?? latest.routeProvenance ?? null,
      lineage: latest.lineage ?? null,
      tags: normalizeTags(latest.tags),
      ml: latestMl,
      ...(compareStageCompletions
        ? {
            stageCompletions: normalizeStageCompletions(latestStageCompletions),
          }
        : {}),
    };
  const currentSummary = {
      reportXlsxPath: normalizeText(currentRun.reportXlsxPath),
      artifactKind: normalizeText(currentRun.artifactKind),
      artifactCompatibility: normalizeArtifactCompatibility(
        currentRun.artifactCompatibility,
        {
          artifactFamily: "run",
          artifactKind: currentRun.artifactKind || "",
        },
      ),
      reportXlsxExists: normalizeOptionalBoolean(currentRun.reportXlsxExists),
      csvPath: normalizeText(currentRun.csvPath),
      configFingerprint: normalizeText(currentRun.configFingerprint),
      configFingerprintVersion: normalizeText(
        currentRun.configFingerprintVersion,
      ),
      routeProvenance: currentRun.routeProvenance ?? null,
      lineage: currentRun.lineage ?? null,
      tags: normalizeTags(currentRun.tags),
      ml: currentMl,
      ...(compareStageCompletions
        ? {
            stageCompletions: normalizeStageCompletions(
              currentRun.stageCompletions,
            ),
          }
        : {}),
    };
  return stableEqual(latestSummary, currentSummary);
}

export async function syncWorkflowArtifactsFromLegacyRuntime(
  source: WorkflowArtifactsSource = "vue",
): Promise<boolean> {
  const currentIdentity = readLatestWorkflowRunIdentitySnapshot();
  if (!currentIdentity) {
    return await measureArtifactHistoryPerf(
      "workflowArtifactsSync.latestRunFastPathMs",
      async () => {
        const latestHydrationRaw = (await invokeCommand(
          "load_latest_run_artifact_hydration",
        )) as unknown;
        const latestHydration = asRecord(latestHydrationRaw);
        const payload = asRecord(latestHydration?.hydration);
        if (!payload) return false;

        const applied = applyWorkflowArtifactsPayload(payload, source);
        const meaningfullyApplied =
          applied.runChanged || applied.validationChanged;
        if (meaningfullyApplied) {
          realignShellStateFromCurrentArtifacts();
          refreshAnalysisFromWorkflowRun();
        }
        return meaningfullyApplied;
      },
      {
        meta: {
          source,
          hasCurrentIdentity: false,
        },
        onSuccessMeta: (synced) => ({ synced }),
      },
    );
  }

  const latestRaw = (await measureArtifactHistoryPerf(
    "workflowArtifactsSync.latestRunSummaryCheckMs",
    () => invokeCommand("load_latest_run_artifact_summary"),
    {
      meta: {
        source,
        hasCurrentIdentity: true,
      },
      onSuccessMeta: (raw) => ({
        hasLatestPath: !!normalizeText(asRecord(raw)?.path),
      }),
    },
  )) as unknown;
  const latest = asRecord(latestRaw);
  const latestPath = normalizeText(latest?.path);
  const latestTs = normalizeText(latest?.ts);
  if (!latest || !latestPath) return false;

  const canSkipLoad = !!(
    currentIdentity &&
    currentIdentity.artifactPath === latestPath &&
    currentIdentity.ts === latestTs &&
    currentIdentity.hasValidation &&
    latestSummaryMatchesCurrentStore(latest)
  );
  if (canSkipLoad) {
    syncSourceIfStoreBound(source);
    return false;
  }

  const payloadRaw = (await measureArtifactHistoryPerf(
    "workflowArtifactsSync.latestRunRehydrationMs",
    () =>
      invokeCommand("load_run_artifact_hydration", {
        path: latestPath,
      }),
    {
      meta: {
        source,
        path: latestPath,
      },
    },
  )) as unknown;
  const payload = asRecord(payloadRaw);
  if (!payload) return false;

  const applied = applyWorkflowArtifactsPayload(payload, source);
  const meaningfullyApplied = applied.runChanged || applied.validationChanged;
  if (meaningfullyApplied) {
    realignShellStateFromCurrentArtifacts();
    refreshAnalysisFromWorkflowRun();
  }
  return meaningfullyApplied;
}
