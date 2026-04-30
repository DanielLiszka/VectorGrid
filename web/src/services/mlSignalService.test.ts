import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { invokeCommand } from "../tauri/invoke";
import { bindRuntimePinia } from "../stores/runtimePinia";
import { useDataStore } from "../stores/dataStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useStrategyStore } from "../stores/strategyStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";
import {
  exportMlSignalDataset,
  generateMlSignalScoreCache,
  listMlSignalModelBundles,
  openMlSignalArtifactPath,
  refreshMlSignalModelStatus,
  revealMlSignalArtifactPath,
  runMlSignalPolicySweep,
  runMlSignalValidation,
} from "./mlSignalService";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => ({})),
}));

const invokeCommandMock = vi.mocked(invokeCommand);
let pinia: ReturnType<typeof createPinia>;

function createDeferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("mlSignalService", () => {
  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    vi.clearAllMocks();
  });

  function seedStores() {
    const data = useDataStore(pinia);
    const optimize = useOptimizeStore(pinia);
    const strategy = useStrategyStore(pinia);

    data.dataId = "data-1";
    data.csvPath = "C:\\data\\prices.csv";
    data.datasetLabel = "prices.csv";
    data.baseDatasetLabel = "prices.csv";
    data.dataTimeframe = "1d";
    data.datasetContext = "source=csv • file=prices.csv";
    data.baseDataMeta = {
      len: 100,
      hasOpen: true,
      hasHigh: true,
      hasLow: true,
      hasClose: true,
      hasVolume: true,
      minTs: 1_700_000_000_000,
      maxTs: 1_700_086_400_000,
      timeframe: "1d",
      sourceSizeBytes: null,
      sourceMtimeUtcMs: null,
      sourceContentFingerprint: "fp-prices-1d",
    } as never;

    optimize.mlSignalDatasetExportPath = "C:\\data\\ml_dataset.csv";
    optimize.mlSignalScoreInputPath = "C:\\data\\scores.csv";
    optimize.mlSignalScoreCachePath = "C:\\data\\score_cache.json";
    optimize.mlSignalModelBundleId = "bundle-a";
    optimize.mlSignalMaSource = "close";
    optimize.mlSignalHorizonBars = "10";
    optimize.mlSignalSplitTrainBars = "252";
    optimize.mlSignalSplitCalibrationBars = "63";
    optimize.mlSignalSplitTestBars = "63";
    optimize.mlSignalSplitGapBars = "10";
    optimize.mlSignalSplitStepBars = "63";
    optimize.mlSignalHoldoutModulus = "5";
    optimize.mlSignalHoldoutResidue = "1";
    optimize.mlSignalLongThreshold = "0.60";
    optimize.mlSignalShortThreshold = "0.61";
    optimize.mlSignalDelta = "0.08";
    optimize.mlSignalMaxHoldBars = "12";
    optimize.mlSignalMinPsr = "0.60";
    optimize.mlSignalMinDsr = "0.40";
    optimize.mlSignalSweepLongStart = "0.50";
    optimize.mlSignalSweepLongEnd = "0.70";
    optimize.mlSignalSweepLongStep = "0.10";
    optimize.mlSignalSweepShortStart = "0.50";
    optimize.mlSignalSweepShortEnd = "0.70";
    optimize.mlSignalSweepShortStep = "0.10";
    optimize.mlSignalSweepDeltaStart = "0.00";
    optimize.mlSignalSweepDeltaEnd = "0.10";
    optimize.mlSignalSweepDeltaStep = "0.05";
    optimize.mlSignalSweepHoldStart = "5";
    optimize.mlSignalSweepHoldEnd = "15";
    optimize.mlSignalSweepHoldStep = "5";
    optimize.mlSignalSweepTopK = "4";
    optimize.mlSignalExitOnFlat = true;
    optimize.mlSignalExitOnOpposite = true;

    strategy.longOnly = false;
    strategy.allowFlip = true;
    strategy.tradeOnNext = true;
    strategy.feeBps = "0.10";
    strategy.spreadBps = "1.00";
    strategy.slippageBps = "0.50";
    strategy.cooldownBars = "2";
    strategy.signalPersistenceBars = "3";
  }

  it("exports ML datasets with the dedicated command payload", async () => {
    seedStores();
    invokeCommandMock.mockImplementation(async (command) => {
      if (command === "export_ml_signal_dataset") {
        return {
          row_count: 321,
          feature_count: 88,
          pattern_count: 61,
          horizon_bars: 10,
          split_count: 3,
          split_manifest_path: "C:\\data\\ml_dataset.split_manifest.json",
          ticker_holdout: true,
          feature_schema_path: "C:\\data\\ml_dataset.feature_schema.json",
          export_manifest_path: "C:\\data\\ml_dataset.export_manifest.json",
        };
      }
      if (command === "save_run_artifact") {
        return { path: "runs/vectorgrid_ml_dataset.json" };
      }
      return {};
    });

    await exportMlSignalDataset();

    expect(invokeCommandMock).toHaveBeenCalledWith("export_ml_signal_dataset", {
      req: expect.objectContaining({
        data_id: "data-1",
        output_path: "C:\\data\\ml_dataset.csv",
        ma_source: "close",
        horizon_bars: 10,
        split_plan: {
          version: 1,
          train_bars: 252,
          calibration_bars: 63,
          test_bars: 63,
          gap_bars: 10,
          step_bars: 63,
        },
        ticker_holdout: {
          modulus: 5,
          residue: 1,
        },
      }),
    });
    expect(invokeCommandMock).toHaveBeenCalledWith("save_run_artifact", {
      artifact: expect.objectContaining({
        dataset_meta: expect.objectContaining({
          csv_path: "C:\\data\\prices.csv",
          timeframe: "1d",
          base_timeframe: "1d",
          derived_timeframe: "1d",
          min_ts: 1_700_000_000_000,
          max_ts: 1_700_086_400_000,
          source_content_fingerprint: "fp-prices-1d",
        }),
      }),
    });
    expect(useOptimizeStore(pinia).mlSignalDatasetMeta).toContain("321 rows");
    expect(useOptimizeStore(pinia).mlSignalDatasetMeta).toContain("splits 3");
    expect(useOptimizeStore(pinia).mlSignalDatasetMeta).toContain(
      "ticker holdout on",
    );
    const workflowArtifacts = useWorkflowArtifactsStore(pinia);
    expect(workflowArtifacts.currentRun?.artifactPath).toBe(
      "runs/vectorgrid_ml_dataset.json",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.stage).toBe(
      "dataset_export",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.datasetExportPath).toBe(
      "C:\\data\\ml_dataset.csv",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.exportManifestPath).toBe(
      "C:\\data\\ml_dataset.export_manifest.json",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.scoreCachePath).toBe("");
  });

  it("omits split plans when blank and normalizes ticker-holdout residue before export", async () => {
    seedStores();
    const optimize = useOptimizeStore(pinia);
    optimize.mlSignalSplitTrainBars = "";
    optimize.mlSignalSplitCalibrationBars = "";
    optimize.mlSignalSplitTestBars = "";
    optimize.mlSignalSplitGapBars = "";
    optimize.mlSignalSplitStepBars = "";
    optimize.mlSignalHoldoutModulus = "5";
    optimize.mlSignalHoldoutResidue = "7";

    invokeCommandMock.mockImplementation(async (command) => {
      if (command === "export_ml_signal_dataset") {
        return {
          row_count: 120,
          feature_count: 40,
          pattern_count: 24,
          horizon_bars: 10,
          split_count: 0,
          ticker_holdout: true,
          feature_schema_path: "C:\\data\\ml_dataset.feature_schema.json",
          export_manifest_path: "C:\\data\\ml_dataset.export_manifest.json",
        };
      }
      if (command === "save_run_artifact") {
        return { path: "runs/vectorgrid_ml_dataset_plain.json" };
      }
      return {};
    });

    await exportMlSignalDataset();

    expect(invokeCommandMock).toHaveBeenCalledWith("export_ml_signal_dataset", {
      req: expect.objectContaining({
        output_path: "C:\\data\\ml_dataset.csv",
        split_plan: null,
        ticker_holdout: {
          modulus: 5,
          residue: 2,
        },
      }),
    });
    expect(useOptimizeStore(pinia).mlSignalDatasetMeta).not.toContain(
      "splits ",
    );
    expect(useWorkflowArtifactsStore(pinia).currentRun?.artifactPath).toBe(
      "runs/vectorgrid_ml_dataset_plain.json",
    );
  });

  it("defaults split-plan gap and step bars when windows are present but optional fields are blank", async () => {
    seedStores();
    const optimize = useOptimizeStore(pinia);
    optimize.mlSignalSplitTrainBars = "252";
    optimize.mlSignalSplitCalibrationBars = "63";
    optimize.mlSignalSplitTestBars = "63";
    optimize.mlSignalSplitGapBars = "";
    optimize.mlSignalSplitStepBars = "";

    invokeCommandMock.mockImplementation(async (command) => {
      if (command === "export_ml_signal_dataset") {
        return {
          row_count: 321,
          feature_count: 88,
          pattern_count: 61,
          horizon_bars: 10,
          split_count: 3,
          split_manifest_path: "C:\\data\\ml_dataset.split_manifest.json",
          feature_schema_path: "C:\\data\\ml_dataset.feature_schema.json",
          export_manifest_path: "C:\\data\\ml_dataset.export_manifest.json",
        };
      }
      if (command === "save_run_artifact") {
        return { path: "runs/vectorgrid_ml_dataset_gap_step_defaults.json" };
      }
      return {};
    });

    await exportMlSignalDataset();

    expect(invokeCommandMock).toHaveBeenCalledWith("export_ml_signal_dataset", {
      req: expect.objectContaining({
        split_plan: {
          version: 1,
          train_bars: 252,
          calibration_bars: 63,
          test_bars: 63,
          gap_bars: 0,
          step_bars: 1,
        },
      }),
    });
  });

  it("builds score caches and persists validation artifacts through the workflow store", async () => {
    seedStores();
    const workflowArtifacts = useWorkflowArtifactsStore(pinia);
    workflowArtifacts.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-20T09:00:00.000Z",
        artifact_path: "runs/vectorgrid_ml_dataset.json",
        csv_path: "C:\\data\\prices.csv",
        req: {
          strategy_kind: "MlSignal",
          data_id: "data-1",
          output_path: "C:\\data\\ml_dataset.csv",
          ma_source: "close",
          horizon_bars: 10,
        },
        res: {
          output_path: "C:\\data\\ml_dataset.csv",
          export_manifest_path: "C:\\data\\ml_dataset.export_manifest.json",
          split_manifest_path: "C:\\data\\ml_dataset.split_manifest.json",
          feature_schema_path: "C:\\data\\ml_dataset.feature_schema.json",
          feature_schema_version: 1,
          row_count: 321,
          split_count: 3,
        },
        ml_signal: {
          stage: "dataset_export",
          dataset_export_path: "C:\\data\\ml_dataset.csv",
          source_bundle_manifest_path:
            "C:\\models\\bundle-a\\manifest.json",
          export_manifest_path: "C:\\data\\ml_dataset.export_manifest.json",
          split_manifest_path: "C:\\data\\ml_dataset.split_manifest.json",
          feature_schema_path: "C:\\data\\ml_dataset.feature_schema.json",
          feature_schema_version: 1,
          feature_schema_hash: "schemahash-source-abcdef",
          row_count: 321,
          split_count: 3,
        },
      },
      currentValidation: null,
    });
    invokeCommandMock.mockImplementation(async (command) => {
      if (command === "generate_ml_signal_score_cache") {
        return {
          matched_rows: 75,
          skipped_rows: 4,
          model_bundle_id: "bundle-a",
          manifest_path: "C:\\data\\score_cache.manifest.json",
          feature_schema_hash: "schemahash0123456789abcdef",
          exporter: {
            exporter_version: "desktop-ml-exporter@2026.04.15",
          },
          provenance: {
            data_timeframe: "1d",
            source_export_manifest_hash: "exporthash0123456789abcdef",
          },
          score_contract: {
            validation_scores_scope: "calibration_slice",
            validation_scores_preferred_probability_columns_after_calibration: [
              "p_long_cal",
              "p_short_cal",
            ],
          },
          score_contract_version: 3,
          score_cache_provenance: {
            source_bundle_id: "bundle-a",
            source_bundle_manifest_hash: "bundlehash0123456789abcdef",
            calibration_state: "calibrated",
            threshold_policy: "probability_threshold_only",
            producer_kind: "catboost_pair",
          },
          score_space: "Probability",
          score_scope: "CalibrationSlice",
          coverage_ratio: 0.75,
          duplicate_timestamps: 2,
        };
      }
      if (command === "run_ml_signal_validation") {
        return {
          runtime_ms: 42,
          candidates: [
            {
              params: {},
              robust_score: 1.2,
              boundary_diagnostics: {
                near_long_threshold_frac: 0.25,
                near_delta_boundary_frac: 0.125,
              },
            },
          ],
          score_space: "Probability",
          score_scope: "CalibrationSlice",
          coverage_ratio: 0.75,
          duplicate_timestamps: 2,
          dependence_summary: {
            raw_score_observations: 75,
            effective_score_observations_lower_bound: 48,
            raw_walkforward_test_windows: 5,
            effective_walkforward_test_windows_upper_bound: 2,
          },
          run_verdict: {
            approved_candidate_count: 1,
            candidates_survived_hard_gates: 1,
            warning_flags: [
              "score_scope_calibration_slice",
              "score_scope_walkforward_overlap_adjusted",
            ],
          },
          evidence_policy_summary: {
            version: 1,
            workflow: "ml_signal_validation",
            summary_role: "interpretation_metadata_only",
            metrics: [
              {
                metric: "psr",
                metric_class: "confidence_bounded",
                gate_role: "advisory",
                sample_unit: "effective_score_observations_lower_bound",
                interpretation: "Probabilistic Sharpe Ratio.",
              },
              {
                metric: "deployability",
                metric_class: "metadata_only",
                gate_role: "provenance",
                sample_unit: "validation_run",
                interpretation: "Deployability metadata.",
              },
            ],
          },
        };
      }
      if (command === "save_run_artifact") {
        return { path: "runs/vectorgrid_run_ml_signal.json" };
      }
      return {};
    });

    await generateMlSignalScoreCache();
    await runMlSignalValidation();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "generate_ml_signal_score_cache",
      {
        req: expect.objectContaining({
          data_id: "data-1",
          input_path: "C:\\data\\scores.csv",
          output_path: "C:\\data\\score_cache.json",
          source_bundle_manifest_path: "C:\\models\\bundle-a\\manifest.json",
          feature_schema_path: "C:\\data\\ml_dataset.feature_schema.json",
          feature_schema_hash: "schemahash-source-abcdef",
          export_manifest_path: "C:\\data\\ml_dataset.export_manifest.json",
        }),
      },
    );
    expect(invokeCommandMock).toHaveBeenCalledWith("run_ml_signal_validation", {
      req: expect.objectContaining({
        data_id: "data-1",
        score_cache_path: "C:\\data\\score_cache.json",
        min_psr: 0.6,
        min_dsr: 0.4,
        validation_context: {
          strategy_kind: "MlSignal",
          csv_path: "C:\\data\\prices.csv",
          timeframe: "1d",
          label: "prices.csv",
        },
      }),
    });
    expect(invokeCommandMock).toHaveBeenCalledWith("save_run_artifact", {
      artifact: expect.objectContaining({
        config_fingerprint: expect.any(String),
        dataset_meta: expect.objectContaining({
          csv_path: "C:\\data\\prices.csv",
          timeframe: "1d",
        }),
        validation: expect.objectContaining({
          req: expect.objectContaining({
            validation_context: expect.objectContaining({
              strategy_kind: "MlSignal",
              timeframe: "1d",
            }),
          }),
        }),
        ml_signal: expect.objectContaining({
          stage: "validation",
          feature_schema_hash: "schemahash0123456789abcdef",
          score_cache_path: "C:\\data\\score_cache.json",
          score_contract_version: "3",
          score_space: "Probability",
          score_scope: "CalibrationSlice",
          evidence_policy_summary: expect.objectContaining({
            workflow: "ml_signal_validation",
          }),
        }),
      }),
    });

    expect(workflowArtifacts.currentRun?.req).toMatchObject({
      strategy_kind: "MlSignal",
    });
    expect(workflowArtifacts.currentRun?.mlSignal?.stage).toBe("validation");
    expect(workflowArtifacts.currentRun?.mlSignal?.datasetExportPath).toBe(
      "C:\\data\\ml_dataset.csv",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.sourceBundleManifestPath).toBe(
      "C:\\models\\bundle-a\\manifest.json",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.exportManifestPath).toBe(
      "C:\\data\\ml_dataset.export_manifest.json",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.splitManifestPath).toBe(
      "C:\\data\\ml_dataset.split_manifest.json",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.scoreManifestPath).toBe(
      "C:\\data\\score_cache.manifest.json",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.featureSchemaPath).toBe(
      "C:\\data\\ml_dataset.feature_schema.json",
    );
    expect(
      Number(workflowArtifacts.currentRun?.mlSignal?.featureSchemaVersion),
    ).toBe(1);
    expect(workflowArtifacts.currentRun?.mlSignal?.featureSchemaHash).toBe(
      "schemahash0123456789abcdef",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.exporter).toMatchObject({
      exporter_version: "desktop-ml-exporter@2026.04.15",
    });
    expect(workflowArtifacts.currentRun?.mlSignal?.provenance).toMatchObject({
      data_timeframe: "1d",
      source_export_manifest_hash: "exporthash0123456789abcdef",
    });
    expect(workflowArtifacts.currentRun?.mlSignal?.scoreContract).toMatchObject(
      {
        validation_scores_scope: "calibration_slice",
      },
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.scoreContractVersion).toBe(
      "3",
    );
    expect(
      workflowArtifacts.currentRun?.mlSignal?.scoreCacheProvenance,
    ).toMatchObject({
      source_bundle_id: "bundle-a",
      source_bundle_manifest_hash: "bundlehash0123456789abcdef",
      calibration_state: "calibrated",
      threshold_policy: "probability_threshold_only",
      producer_kind: "catboost_pair",
    });
    expect(workflowArtifacts.currentRun?.mlSignal?.scoreSpace).toBe(
      "Probability",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.scoreScope).toBe(
      "CalibrationSlice",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.matchedRows).toBe(75);
    expect(workflowArtifacts.currentRun?.mlSignal?.skippedRows).toBe(4);
    expect(workflowArtifacts.currentRun?.mlSignal?.coverageRatio).toBe(0.75);
    expect(workflowArtifacts.currentRun?.mlSignal?.duplicateTimestamps).toBe(2);
    expect(
      workflowArtifacts.currentRun?.mlSignal?.evidencePolicySummary,
    ).toMatchObject({
      workflow: "ml_signal_validation",
    });
    expect(workflowArtifacts.currentValidation?.req).toMatchObject({
      data_id: "data-1",
      score_cache_path: "C:\\data\\score_cache.json",
    });
    expect(useOptimizeStore(pinia).mlSignalValidationMeta).toContain(
      "candidate",
    );
    expect(useOptimizeStore(pinia).mlSignalValidationMeta).toContain(
      "coverage 75.0%",
    );
    expect(useOptimizeStore(pinia).mlSignalValidationMeta).toContain(
      "calibration-only score scope",
    );
    expect(useOptimizeStore(pinia).mlSignalValidationMeta).toContain(
      "walk-forward overlap adjusted (obs 48/75, windows 2/5)",
    );
    expect(useOptimizeStore(pinia).mlSignalValidationMeta).toContain(
      "boundary long~25.0%, delta~12.5%",
    );
    expect(useOptimizeStore(pinia).mlSignalValidationMeta).toContain(
      "evidence prob effective_score_observations_lower_bound, deployability metadata",
    );
    expect(useOptimizeStore(pinia).statusLine).toContain(
      "calibration-only score scope",
    );
    expect(useOptimizeStore(pinia).statusLine).toContain(
      "walk-forward overlap adjusted (obs 48/75, windows 2/5)",
    );
    expect(useOptimizeStore(pinia).statusLine).toContain(
      "boundary long~25.0%, delta~12.5%",
    );
  });

  it("builds native score caches through bundle-backed runtime inference", async () => {
    seedStores();
    const optimize = useOptimizeStore(pinia);
    optimize.mlSignalScoreSource = "native";
    optimize.mlSignalScoreInputPath = "";
    optimize.mlSignalNativeBackend = "native_cuda";
    optimize.mlSignalNativeLibraryPath = "C:\\catboost\\catboostmodel.dll";
    optimize.mlSignalNativeGpuDeviceId = "1";
    optimize.mlSignalNativePredictionBatchSize = "512";
    optimize.mlSignalNativeScoreScope = "walkforward_test";
    optimize.mlSignalAllowNonWalkforwardScoreScope = true;

    invokeCommandMock.mockImplementation(async (command) => {
      if (command === "generate_ml_signal_score_cache") {
        return {
          matched_rows: 75,
          skipped_rows: 0,
          model_bundle_id: "bundle-a",
          manifest_path: "C:\\data\\score_cache.manifest.json",
          native_backend_requested: "native_cuda",
          native_backend_used: "native_cpu",
          native_library_path: "C:\\catboost\\catboostmodel.dll",
          native_library_sha256:
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
          native_library_discovery_policy: "developer_debug",
          native_gpu_device_id: 1,
          native_prediction_batch_size: 512,
          native_fallback_reason: "EnableGPUEvaluation unavailable",
          score_space: "Probability",
          score_scope: "WalkforwardTest",
          score_scope_policy: "WalkforwardTestRequired",
          raw_score_observations: 120,
          effective_score_observations: 75,
          bundle_manifest_sha256:
            "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
          model_long_sha256:
            "1111111111111111111111111111111111111111111111111111111111111111",
          model_short_sha256:
            "2222222222222222222222222222222222222222222222222222222222222222",
          score_cache_provenance: {
            source_bundle_id: "bundle-a",
            producer_kind: "catboost_native_cpu",
            native_backend_requested: "native_cuda",
            native_backend_used: "native_cpu",
            native_library_path: "C:\\catboost\\catboostmodel.dll",
            native_library_sha256:
              "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            native_library_discovery_policy: "developer_debug",
            score_scope_policy: "WalkforwardTestRequired",
            native_gpu_device_id: 1,
            native_prediction_batch_size: 512,
            native_fallback_reason: "EnableGPUEvaluation unavailable",
          },
        };
      }
      if (command === "save_run_artifact") {
        return { path: "runs/vectorgrid_run_ml_signal_native.json" };
      }
      return {};
    });

    await generateMlSignalScoreCache();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "generate_ml_signal_score_cache",
      {
        req: expect.objectContaining({
          data_id: "data-1",
          input_path: "",
          output_path: "C:\\data\\score_cache.json",
          model_bundle_id: "bundle-a",
          native_backend_preference: "native_cuda",
          native_library_path: "C:\\catboost\\catboostmodel.dll",
          native_gpu_device_id: 1,
          native_prediction_batch_size: 512,
          native_library_discovery_policy: "developer_debug",
          score_scope: "walkforward_test",
          allow_non_walkforward_score_scope: true,
        }),
      },
    );
    expect(useOptimizeStore(pinia).mlSignalScoreMeta).toContain(
      "Native score cache: 75 matched rows -> C:\\data\\score_cache.json",
    );
    expect(useOptimizeStore(pinia).mlSignalScoreMeta).toContain("native_cpu");
    expect(
      useWorkflowArtifactsStore(pinia).currentRun?.mlSignal
        ?.scoreCacheProvenance,
    ).toMatchObject({
      producer_kind: "catboost_native_cpu",
      native_backend_requested: "native_cuda",
      native_backend_used: "native_cpu",
      native_library_path: "C:\\catboost\\catboostmodel.dll",
      native_library_sha256:
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      native_library_discovery_policy: "developer_debug",
      score_scope_policy: "WalkforwardTestRequired",
      native_gpu_device_id: 1,
      native_prediction_batch_size: 512,
      native_fallback_reason: "EnableGPUEvaluation unavailable",
    });
    expect(
      useWorkflowArtifactsStore(pinia).currentRun?.mlSignal
        ?.nativePredictionBatchSize,
    ).toBe(512);
    expect(
      useWorkflowArtifactsStore(pinia).currentRun?.mlSignal
        ?.rawScoreObservations,
    ).toBe(120);
    expect(
      useWorkflowArtifactsStore(pinia).currentRun?.mlSignal
        ?.effectiveScoreObservations,
    ).toBe(75);
  });

  it("defaults native score-cache generation to native_cpu when no backend is set", async () => {
    seedStores();
    const optimize = useOptimizeStore(pinia);
    optimize.mlSignalScoreSource = "native";
    optimize.mlSignalScoreInputPath = "";
    optimize.mlSignalNativeBackend = "";

    invokeCommandMock.mockImplementation(async (command) => {
      if (command === "generate_ml_signal_score_cache") {
        return {
          matched_rows: 40,
          skipped_rows: 0,
          model_bundle_id: "bundle-a",
          manifest_path: "C:\\data\\score_cache.manifest.json",
          native_backend_requested: "native_cpu",
          native_backend_used: "native_cpu",
          score_space: "Probability",
          score_scope: "FullDataset",
        };
      }
      if (command === "save_run_artifact") {
        return { path: "runs/vectorgrid_run_ml_signal_native_cpu.json" };
      }
      return {};
    });

    await generateMlSignalScoreCache();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "generate_ml_signal_score_cache",
      {
        req: expect.objectContaining({
          native_backend_preference: "native_cpu",
        }),
      },
    );
  });

  it("runs ML policy sweep through the dedicated command and persists the best policy", async () => {
    seedStores();
    invokeCommandMock.mockImplementation(async (command) => {
      if (command === "run_ml_signal_policy_sweep") {
        return {
          sweep_size: 27,
          top_k: 4,
          validation: {
            runtime_ms: 24,
            dependence_summary: {
              raw_score_observations: 120,
              effective_score_observations_lower_bound: 60,
              raw_walkforward_test_windows: 6,
              effective_walkforward_test_windows_upper_bound: 3,
            },
            run_verdict: {
              warning_flags: [
                "score_scope_custom_partial",
                "score_scope_walkforward_overlap_adjusted",
              ],
            },
            candidates: [
              {
                robust_score: 1.44,
                boundary_diagnostics: {
                  near_long_threshold_frac: 0.2,
                  near_short_threshold_frac: 0.1,
                  near_delta_boundary_frac: 0.05,
                },
                params: {
                  label: "best_sweep",
                  t_long: 0.6,
                  t_short: 0.6,
                  delta: 0.05,
                  max_hold_bars: 10,
                },
              },
            ],
          },
        };
      }
      if (command === "save_run_artifact") {
        return { path: "runs/vectorgrid_run_ml_signal_sweep.json" };
      }
      return {};
    });

    const workflowArtifacts = useWorkflowArtifactsStore(pinia);
    workflowArtifacts.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-20T10:00:00.000Z",
        artifact_path: "runs/vectorgrid_run_ml_signal.json",
        csv_path: "C:\\data\\prices.csv",
        req: {
          strategy_kind: "MlSignal",
          data_id: "data-1",
          output_path: "C:\\data\\score_cache.json",
          model_bundle_id: "bundle-a",
        },
        res: {
          output_path: "C:\\data\\score_cache.json",
          manifest_path: "C:\\data\\score_cache.manifest.json",
          feature_schema_path: "C:\\data\\ml_dataset.feature_schema.json",
          feature_schema_version: 1,
        },
      },
      currentValidation: null,
    });

    await runMlSignalPolicySweep();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "run_ml_signal_policy_sweep",
      {
        req: expect.objectContaining({
          data_id: "data-1",
          score_cache_path: "C:\\data\\score_cache.json",
          t_long: { start: 0.5, end: 0.7, step: 0.1 },
          t_short: { start: 0.5, end: 0.7, step: 0.1 },
          delta: { start: 0, end: 0.1, step: 0.05 },
          max_hold_bars: { start: 5, end: 15, step: 5 },
          min_psr: 0.6,
          min_dsr: 0.4,
          top_k: 4,
          validation_context: {
            strategy_kind: "MlSignal",
            csv_path: "C:\\data\\prices.csv",
            timeframe: "1d",
            label: "prices.csv",
          },
        }),
      },
    );
    expect(invokeCommandMock).toHaveBeenCalledWith("save_run_artifact", {
      artifact: expect.objectContaining({
        config_fingerprint: expect.any(String),
        dataset_meta: expect.objectContaining({
          csv_path: "C:\\data\\prices.csv",
          timeframe: "1d",
        }),
        validation: expect.objectContaining({
          req: expect.objectContaining({
            validation_context: expect.objectContaining({
              strategy_kind: "MlSignal",
              timeframe: "1d",
            }),
          }),
        }),
        ml_signal: expect.objectContaining({
          stage: "policy_sweep",
          score_cache_path: "C:\\data\\score_cache.json",
          policy_sweep: expect.objectContaining({
            sweep_size: 27,
            top_k: 4,
          }),
        }),
      }),
    });
    expect(useOptimizeStore(pinia).mlSignalSweepMeta).toContain(
      "27 candidates",
    );
    expect(useOptimizeStore(pinia).mlSignalSweepMeta).toContain(
      "partial score scope",
    );
    expect(useOptimizeStore(pinia).mlSignalSweepMeta).toContain(
      "walk-forward overlap adjusted (obs 60/120, windows 3/6)",
    );
    expect(useOptimizeStore(pinia).mlSignalSweepMeta).toContain(
      "boundary long~20.0%, short~10.0%, delta~5.0%",
    );
    expect(useOptimizeStore(pinia).statusLine).toContain("partial score scope");
    expect(useOptimizeStore(pinia).statusLine).toContain(
      "walk-forward overlap adjusted (obs 60/120, windows 3/6)",
    );
    expect(useOptimizeStore(pinia).statusLine).toContain(
      "boundary long~20.0%, short~10.0%, delta~5.0%",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.policy).toMatchObject({
      label: "best_sweep",
      t_long: 0.6,
      max_hold_bars: 10,
    });
    expect(workflowArtifacts.currentRun?.mlSignal?.stage).toBe("policy_sweep");
    expect(workflowArtifacts.currentRun?.mlSignal?.featureSchemaPath).toBe(
      "C:\\data\\ml_dataset.feature_schema.json",
    );
    expect(
      Number(workflowArtifacts.currentRun?.mlSignal?.featureSchemaVersion),
    ).toBe(1);
    expect(workflowArtifacts.currentRun?.mlSignal?.policySweep).toMatchObject({
      sweep_size: 27,
      top_k: 4,
      best_label: "best_sweep",
    });
  });

  it("lists model bundles and refreshes bundle status through explicit bundle metadata", async () => {
    seedStores();
    invokeCommandMock.mockImplementation(async (command) => {
      if (command === "list_ml_signal_model_bundles") {
        return [
          {
            bundle_id: "bundle-a",
            manifest_path: "C:\\models\\ml_signal\\bundle-a\\manifest.json",
            state: "healthy",
            model_family: "CatBoost",
          },
          {
            bundle_id: "bundle-b",
            manifest_path: "",
            state: "warning",
            model_family: "",
            issues: ["schema mismatch", "missing native sidecar"],
          },
        ];
      }
      if (command === "ml_signal_model_status") {
        return {
          models_dir: "C:\\models\\ml_signal",
          status: {
            bundle_id: "bundle-a",
            state: "healthy",
            manifest_path: "C:\\models\\ml_signal\\bundle-a\\manifest.json",
          },
        };
      }
      return {};
    });

    const listed = await listMlSignalModelBundles();
    expect(listed).toEqual([
      {
        bundleId: "bundle-a",
        manifestPath: "C:\\models\\ml_signal\\bundle-a\\manifest.json",
        state: "healthy",
        label: "bundle-a • CatBoost • healthy",
      },
      {
        bundleId: "bundle-b",
        manifestPath: "",
        state: "warning",
        label: "bundle-b • warning • 2 issues",
      },
    ]);

    await refreshMlSignalModelStatus();
    expect(invokeCommandMock).toHaveBeenCalledWith("ml_signal_model_status", {
      bundleId: "bundle-a",
    });
    expect(useOptimizeStore(pinia).mlSignalModelStatus).toContain("bundle-a");
    expect(useOptimizeStore(pinia).mlSignalModelStatus).toContain("healthy");
    expect(useOptimizeStore(pinia).mlSignalModelStatus).toContain(
      "manifest.json",
    );
  });

  it("hydrates bundle status details onto the current ML artifact when status is refreshed", async () => {
    seedStores();
    const workflowArtifacts = useWorkflowArtifactsStore(pinia);
    workflowArtifacts.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-20T10:00:00.000Z",
        artifact_path: "runs/vectorgrid_run_ml_signal.json",
        csv_path: "C:\\data\\prices.csv",
        ml_signal: {
          stage: "score_cache",
          bundle_id: "bundle-a",
          score_cache_path: "C:\\data\\score_cache.json",
        },
        req: {},
        res: {},
      },
      currentValidation: null,
    });
    invokeCommandMock.mockImplementation(async (command) => {
      if (command === "ml_signal_model_status") {
        return {
          models_dir: "C:\\models\\ml_signal",
          status: {
            bundle_id: "bundle-a",
            state: "warning",
            manifest_path: "C:\\models\\ml_signal\\bundle-a\\manifest.json",
            manifest: {
              model_family: "catboost_binary_pair",
              model_format: "cbm",
              calibration_path: "calibration.json",
              score_contract: {
                score_contract_version: 3,
              },
            },
            issues: ["schema hash mismatch"],
          },
        };
      }
      return {};
    });

    await refreshMlSignalModelStatus();

    expect(useOptimizeStore(pinia).mlSignalModelStatus).toContain(
      "contract v3",
    );
    expect(useOptimizeStore(pinia).mlSignalModelStatus).toContain(
      "schema hash mismatch",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.bundleStatus).toMatchObject({
      state: "warning",
      issues: ["schema hash mismatch"],
    });
  });

  it("keeps bundle status optional when no bundle is selected", async () => {
    seedStores();
    useOptimizeStore(pinia).mlSignalModelBundleId = "";

    const result = await refreshMlSignalModelStatus();

    expect(result).toBeNull();
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "ml_signal_model_status",
      expect.anything(),
    );
    expect(useOptimizeStore(pinia).mlSignalModelStatus).toContain(
      "optional for score-cache-backed validation",
    );
  });

  it("requires a bundle status when native score-cache generation is selected", async () => {
    seedStores();
    const optimize = useOptimizeStore(pinia);
    optimize.mlSignalScoreSource = "native";
    optimize.mlSignalModelBundleId = "";

    const result = await refreshMlSignalModelStatus();

    expect(result).toBeNull();
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "ml_signal_model_status",
      expect.anything(),
    );
    expect(useOptimizeStore(pinia).mlSignalModelStatus).toContain(
      "required for native score-cache generation",
    );
  });

  it("owns ML-specific busy and status state while dataset export is in flight", async () => {
    seedStores();
    const deferred = createDeferred<Record<string, unknown>>();
    invokeCommandMock.mockImplementation(async (command) => {
      if (command === "export_ml_signal_dataset") {
        return deferred.promise;
      }
      if (command === "save_run_artifact") {
        return { path: "runs/vectorgrid_ml_dataset_busy.json" };
      }
      return {};
    });

    const optimize = useOptimizeStore(pinia);
    let settled = false;
    const runPromise = exportMlSignalDataset().then(() => {
      settled = true;
    });

    await Promise.resolve();
    await Promise.resolve();

    expect(settled).toBe(false);
    expect(optimize.runDisabled).toBe(true);
    expect(optimize.validateDisabled).toBe(true);
    expect(optimize.canCancel).toBe(true);
    expect(optimize.progressMeta).toBe("Exporting ML feature dataset…");
    expect(optimize.statusLine).toBe("Exporting ML feature dataset…");

    deferred.resolve({
      row_count: 111,
      feature_count: 44,
      split_count: 0,
      feature_schema_path: "C:\\data\\ml_dataset.feature_schema.json",
      export_manifest_path: "C:\\data\\ml_dataset.export_manifest.json",
    });
    await runPromise;

    expect(optimize.runDisabled).toBe(false);
    expect(optimize.validateDisabled).toBe(false);
    expect(optimize.canCancel).toBe(false);
    expect(optimize.statusLine).toContain("ML feature dataset exported.");
    expect(optimize.progressMeta).toContain("ML dataset export");
  });

  it("fails validation and policy sweep early when no score cache path is configured", async () => {
    seedStores();
    const optimize = useOptimizeStore(pinia);
    optimize.mlSignalScoreCachePath = "";

    await expect(runMlSignalValidation()).rejects.toThrow(
      "Generate or choose a score cache path first.",
    );
    await expect(runMlSignalPolicySweep()).rejects.toThrow(
      "Generate or choose a score cache path first.",
    );
  });

  it("normalizes percent-style ML Signal probabilities before validation", async () => {
    seedStores();
    const optimize = useOptimizeStore(pinia);
    optimize.mlSignalLongThreshold = "60";
    optimize.mlSignalShortThreshold = "61%";
    optimize.mlSignalDelta = "8";
    optimize.mlSignalMinPsr = "90";
    optimize.mlSignalMinDsr = "40%";
    invokeCommandMock.mockImplementation(async (command) => {
      if (command === "run_ml_signal_validation") {
        return { candidates: [{ params: {}, robust_score: 1.0 }] };
      }
      if (command === "save_run_artifact") {
        return { path: "runs/vectorgrid_run_ml_signal_percent.json" };
      }
      return {};
    });

    await runMlSignalValidation();

    expect(invokeCommandMock).toHaveBeenCalledWith("run_ml_signal_validation", {
      req: expect.objectContaining({
        min_psr: 0.9,
        min_dsr: 0.4,
        candidates: [
          expect.objectContaining({
            t_long: 0.6,
            t_short: 0.61,
            delta: 0.08,
          }),
        ],
      }),
    });
  });

  it("surfaces ML validation artifact persistence failures without hiding the validation result", async () => {
    seedStores();
    invokeCommandMock.mockImplementation(async (command) => {
      if (command === "run_ml_signal_validation") {
        return { candidates: [{ params: {}, robust_score: 1.0 }] };
      }
      if (command === "save_run_artifact") {
        throw new Error("disk full");
      }
      return {};
    });

    await runMlSignalValidation();

    expect(useOptimizeStore(pinia).statusLine).toContain(
      "validation artifact not saved",
    );
    expect(useOptimizeStore(pinia).statusLine).toContain("disk full");
    expect(useOptimizeStore(pinia).mlSignalValidationMeta).toContain(
      "validation artifact not saved",
    );
  });

  it("rejects invalid ML Signal policy-sweep axes before invoking the backend", async () => {
    seedStores();
    const optimize = useOptimizeStore(pinia);
    optimize.mlSignalSweepLongStart = "0.70";
    optimize.mlSignalSweepLongEnd = "0.50";

    await expect(runMlSignalPolicySweep()).rejects.toThrow(
      "Long threshold sweep end must be greater than or equal to start.",
    );
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "run_ml_signal_policy_sweep",
      expect.anything(),
    );
    expect(useOptimizeStore(pinia).statusLine).toContain(
      "ML policy sweep failed",
    );
  });

  it("omits unset sweep axes while keeping the dedicated policy-sweep contract", async () => {
    seedStores();
    const optimize = useOptimizeStore(pinia);
    optimize.mlSignalSweepDeltaStart = "";
    optimize.mlSignalSweepDeltaEnd = "";
    optimize.mlSignalSweepDeltaStep = "";
    optimize.mlSignalSweepHoldStart = "";
    optimize.mlSignalSweepHoldEnd = "";
    optimize.mlSignalSweepHoldStep = "";
    invokeCommandMock.mockImplementation(async (command) => {
      if (command === "run_ml_signal_policy_sweep") {
        return {
          sweep_size: 9,
          top_k: 3,
          validation: {
            candidates: [
              { params: { label: "best_sweep" }, robust_score: 1.2 },
            ],
          },
        };
      }
      if (command === "save_run_artifact") {
        return { path: "runs/vectorgrid_run_ml_signal_sweep.json" };
      }
      return {};
    });

    await runMlSignalPolicySweep();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "run_ml_signal_policy_sweep",
      {
        req: expect.objectContaining({
          t_long: { start: 0.5, end: 0.7, step: 0.1 },
          t_short: { start: 0.5, end: 0.7, step: 0.1 },
          delta: null,
          max_hold_bars: null,
          top_k: 4,
        }),
      },
    );
    expect(useOptimizeStore(pinia).mlSignalSweepMeta).toContain(
      "9 candidates evaluated",
    );
    expect(useOptimizeStore(pinia).mlSignalSweepMeta).toContain("retained top_k");
    expect(useOptimizeStore(pinia).mlSignalValidationMeta).toContain(
      "retained from 9 full-sweep candidates",
    );
  });

  it("opens ML artifact paths with open-path fallback to reveal-path", async () => {
    invokeCommandMock.mockImplementation(async (command) => {
      if (command === "open_path") throw new Error("no shell open");
      return {};
    });

    const result = await openMlSignalArtifactPath("C:\\data\\score_cache.json");

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, "open_path", {
      path: "C:\\data\\score_cache.json",
    });
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, "reveal_path", {
      path: "C:\\data\\score_cache.json",
    });
    expect(result).toEqual({
      path: "C:\\data\\score_cache.json",
      openedWith: "reveal_path",
    });
  });

  it("reveals ML artifact paths directly", async () => {
    await revealMlSignalArtifactPath("C:\\data\\feature_schema.json");

    expect(invokeCommandMock).toHaveBeenCalledWith("reveal_path", {
      path: "C:\\data\\feature_schema.json",
    });
  });
});
