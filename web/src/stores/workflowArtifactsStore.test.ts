import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { bindRuntimePinia } from "./runtimePinia";
import {
  applyWorkflowArtifactsPayload,
  readLatestWorkflowRunIdentitySnapshot,
  useWorkflowArtifactsStore,
} from "./workflowArtifactsStore";

let pinia: ReturnType<typeof createPinia>;

describe("workflowArtifactsStore", () => {
  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    vi.restoreAllMocks();
    useWorkflowArtifactsStore(pinia).clearArtifacts();
  });

  it("treats identical normalized payloads as a no-op without timestamp churn", () => {
    const nowSpy = vi
      .spyOn(Date, "now")
      .mockReturnValueOnce(1_000)
      .mockReturnValueOnce(2_000);
    const store = useWorkflowArtifactsStore(pinia);
    const payload = {
      currentRun: {
        ts: "2026-04-01T12:00:00Z",
        artifact_path: "C:/runs/run.json",
        req: { strategy_kind: "DoubleMa" },
        res: { top: [] },
      },
      currentValidation: null,
    };

    const first = applyWorkflowArtifactsPayload(payload, "vue");
    const firstUpdatedAt = store.updatedAtUnixMs;
    const second = applyWorkflowArtifactsPayload(payload, "vue");

    expect(first).toMatchObject({
      applied: true,
      runChanged: true,
      validationChanged: false,
      sourceChanged: false,
    });
    expect(second).toMatchObject({
      applied: false,
      runChanged: false,
      validationChanged: false,
      sourceChanged: false,
    });
    expect(store.updatedAtUnixMs).toBe(firstUpdatedAt);
    expect(readLatestWorkflowRunIdentitySnapshot()).toEqual({
      ts: "2026-04-01T12:00:00Z",
      artifactPath: "C:/runs/run.json",
      hasValidation: false,
    });
    expect(nowSpy).toHaveBeenCalledTimes(1);
  });

  it("reports validation changes when validation arrives later for the same run", () => {
    vi.spyOn(Date, "now").mockReturnValueOnce(1_000).mockReturnValueOnce(2_000);
    const store = useWorkflowArtifactsStore(pinia);
    const baseRun = {
      ts: "2026-04-01T12:00:00Z",
      artifact_path: "C:/runs/run.json",
      req: { strategy_kind: "DoubleMa" },
      res: { top: [] },
    };

    applyWorkflowArtifactsPayload(
      {
        currentRun: baseRun,
        currentValidation: null,
      },
      "vue",
    );

    const next = applyWorkflowArtifactsPayload(
      {
        currentRun: baseRun,
        currentValidation: {
          ts: "2026-04-01T12:00:00Z",
          req: { candidates: 16 },
          res: { rows: [] },
        },
      },
      "vue",
    );

    expect(next).toMatchObject({
      applied: true,
      runChanged: false,
      validationChanged: true,
      sourceChanged: false,
    });
    expect(store.currentValidation?.req).toMatchObject({ candidates: 16 });
    expect(readLatestWorkflowRunIdentitySnapshot()).toEqual({
      ts: "2026-04-01T12:00:00Z",
      artifactPath: "C:/runs/run.json",
      hasValidation: true,
    });
    expect(store.updatedAtUnixMs).toBe(2_000);
  });

  it("normalizes optimization metric surface metadata from hydrated artifacts", () => {
    const store = useWorkflowArtifactsStore(pinia);

    applyWorkflowArtifactsPayload(
      {
        currentRun: {
          ts: "2026-04-01T12:00:00Z",
          artifact_path: "C:/runs/run.json",
          optimization_metric_surface: {
            surface: "core17",
            fields: ["pnl", "trades", "max_dd"],
          },
          req: { strategy_kind: "CompositeTrend" },
          res: { top: [] },
        },
        currentValidation: null,
      },
      "vue",
    );

    expect(store.currentRun?.optimizationMetricSurface).toMatchObject({
      surface: "core17",
      fields: ["pnl", "trades", "max_dd"],
    });
  });

  it("normalizes snake_case hydrated run payload fields without losing report, dataset, tags, or ml signal details", () => {
    const store = useWorkflowArtifactsStore(pinia);

    applyWorkflowArtifactsPayload(
      {
        currentRun: {
          ts: "2026-04-01T12:00:00Z",
          artifact_path: "C:/runs/run.json",
          report_xlsx_path: "C:/runs/report.xlsx",
          csv_path: "C:/data/spy.csv",
          dataset_meta: {
            csv_path: "C:/data/spy.csv",
            timeframe: "15m",
            label: "SPY",
          },
          ml_signal: {
            stage: "validation",
            bundle_id: "bundle-a",
            score_cache_path: "C:/runs/cache.json",
          },
          tags: [" alpha ", "beta"],
          req: { strategy_kind: "MlSignal" },
          res: { matched_rows: 123 },
        },
        currentValidation: null,
      },
      "legacy-runtime",
    );

    expect(store.currentRun).toMatchObject({
      artifactPath: "C:/runs/run.json",
      reportXlsxPath: "C:/runs/report.xlsx",
      csvPath: "C:/data/spy.csv",
      datasetMeta: {
        csv_path: "C:/data/spy.csv",
        timeframe: "15m",
        label: "SPY",
      },
      tags: ["alpha", "beta"],
      mlSignal: {
        stage: "validation",
        bundleId: "bundle-a",
        scoreCachePath: "C:/runs/cache.json",
      },
    });
  });

  it("normalizes additive ML provenance metadata from hydrated artifacts", () => {
    const store = useWorkflowArtifactsStore(pinia);

    applyWorkflowArtifactsPayload(
      {
        currentRun: {
          ts: "2026-04-01T12:00:00Z",
          artifact_path: "C:/runs/ml_signal.json",
          ml_signal: {
            stage: "score_cache",
            feature_schema_hash: "schemahash0123456789abcdef",
            exporter: {
              exporter_version: "desktop-ml-exporter@2026.04.15",
            },
            provenance: {
              data_timeframe: "1d",
              source_export_manifest_hash: "exporthash0123456789abcdef",
            },
            calibration: {
              calibration_state: "calibrated",
            },
            score_contract: {
              validation_scores_scope: "calibration_slice",
            },
            score_contract_version: 3,
            score_cache_provenance: {
              source_bundle_id: "bundle-a",
              producer_kind: "catboost_pair",
              native_backend_requested: "native_cuda",
              native_backend_used: "native_cpu",
              native_library_path: "C:/catboost/catboostmodel.dll",
              native_gpu_device_id: 0,
              native_fallback_reason: "EnableGPUEvaluation unavailable",
            },
            bundle_status_details: {
              state: "warning",
              issues: ["schema hash mismatch"],
            },
          },
          req: { strategy_kind: "MlSignal" },
          res: { matched_rows: 123 },
        },
        currentValidation: null,
      },
      "vue",
    );

    expect(store.currentRun?.mlSignal).toMatchObject({
      stage: "score_cache",
      featureSchemaHash: "schemahash0123456789abcdef",
      exporter: {
        exporter_version: "desktop-ml-exporter@2026.04.15",
      },
      provenance: {
        data_timeframe: "1d",
        source_export_manifest_hash: "exporthash0123456789abcdef",
      },
      calibration: {
        calibration_state: "calibrated",
      },
      scoreContract: {
        validation_scores_scope: "calibration_slice",
      },
      scoreContractVersion: "3",
      scoreCacheProvenance: {
        source_bundle_id: "bundle-a",
        producer_kind: "catboost_pair",
        native_backend_requested: "native_cuda",
        native_backend_used: "native_cpu",
        native_library_path: "C:/catboost/catboostmodel.dll",
        native_gpu_device_id: 0,
        native_fallback_reason: "EnableGPUEvaluation unavailable",
      },
      bundleStatus: {
        state: "warning",
        issues: ["schema hash mismatch"],
      },
      nativeBackendRequested: "native_cuda",
      nativeBackendUsed: "native_cpu",
      nativeLibraryPath: "C:/catboost/catboostmodel.dll",
      nativeGpuDeviceId: 0,
      nativeFallbackReason: "EnableGPUEvaluation unavailable",
    });
  });

  it("treats source-only updates as applied without timestamp churn or state reshaping", () => {
    const nowSpy = vi
      .spyOn(Date, "now")
      .mockReturnValueOnce(1_000)
      .mockReturnValueOnce(2_000);
    const store = useWorkflowArtifactsStore(pinia);
    const payload = {
      currentRun: {
        ts: "2026-04-01T12:00:00Z",
        artifact_path: "C:/runs/run.json",
        report_xlsx_path: "C:/runs/report.xlsx",
        req: { strategy_kind: "DoubleMa" },
        res: { top: [] },
      },
      currentValidation: null,
    };

    const first = applyWorkflowArtifactsPayload(payload, "vue");
    const firstUpdatedAt = store.updatedAtUnixMs;
    const second = applyWorkflowArtifactsPayload(payload, "legacy-runtime");

    expect(first).toMatchObject({
      applied: true,
      runChanged: true,
      validationChanged: false,
      sourceChanged: false,
    });
    expect(second).toMatchObject({
      applied: true,
      runChanged: false,
      validationChanged: false,
      sourceChanged: true,
    });
    expect(store.updatedAtUnixMs).toBe(firstUpdatedAt);
    expect(store.source).toBe("legacy-runtime");
    expect(nowSpy).toHaveBeenCalledTimes(1);
  });

  it("treats tag and report-path changes as meaningful run updates for the same artifact identity", () => {
    const store = useWorkflowArtifactsStore(pinia);
    const basePayload = {
      currentRun: {
        ts: "2026-04-01T12:00:00Z",
        artifact_path: "C:/runs/run.json",
        report_xlsx_path: "C:/runs/report_a.xlsx",
        tags: ["alpha"],
        req: { strategy_kind: "DoubleMa" },
        res: { top: [] },
      },
      currentValidation: {
        ts: "2026-04-01T12:00:00Z",
        req: { candidates: 16 },
        res: { rows: [] },
      },
    };

    applyWorkflowArtifactsPayload(basePayload, "vue");
    const next = applyWorkflowArtifactsPayload(
      {
        currentRun: {
          ...basePayload.currentRun,
          report_xlsx_path: "C:/runs/report_b.xlsx",
          tags: ["alpha", "beta"],
        },
        currentValidation: basePayload.currentValidation,
      },
      "vue",
    );

    expect(next).toMatchObject({
      applied: true,
      runChanged: true,
      validationChanged: false,
      sourceChanged: false,
    });
    expect(store.currentRun?.reportXlsxPath).toBe("C:/runs/report_b.xlsx");
    expect(store.currentRun?.tags).toEqual(["alpha", "beta"]);
  });

  it("clears stale store state even when the latest global signatures are already null", () => {
    const store = useWorkflowArtifactsStore(pinia);
    store.currentRun = {
      ts: "2026-04-01T12:00:00Z",
      artifactPath: "C:/runs/run.json",
      reportXlsxPath: "",
      csvPath: "",
      datasetMeta: null,
      mlSignal: null,
      optimizationMetricSurface: null,
      tags: [],
      req: null,
      res: null,
    };
    store.currentValidation = {
      ts: "2026-04-01T12:00:00Z",
      req: { candidates: 16 },
      res: { rows: [] },
    };

    store.clearArtifacts();

    expect(store.currentRun).toBeNull();
    expect(store.currentValidation).toBeNull();
    expect(store.source).toBe("vue");
  });
});
