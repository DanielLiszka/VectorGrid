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
  openMlSignalArtifactPath,
  revealMlSignalArtifactPath,
  runMlSignalPolicySweep,
  runMlSignalValidation,
} from "./mlSignalService";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => ({})),
}));

const invokeCommandMock = vi.mocked(invokeCommand);
let pinia: ReturnType<typeof createPinia>;

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
    };

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
    expect(useOptimizeStore(pinia).mlSignalDatasetMeta).toContain("321 rows");
    expect(useOptimizeStore(pinia).mlSignalDatasetMeta).toContain("splits 3");
    expect(useOptimizeStore(pinia).mlSignalDatasetMeta).toContain(
      "ticker holdout on",
    );
    const workflowArtifacts = useWorkflowArtifactsStore(pinia);
    expect(workflowArtifacts.currentRun?.artifactPath).toBe(
      "runs/vectorgrid_ml_dataset.json",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.stage).toBe("dataset_export");
    expect(workflowArtifacts.currentRun?.mlSignal?.datasetExportPath).toBe(
      "C:\\data\\ml_dataset.csv",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.exportManifestPath).toBe(
      "C:\\data\\ml_dataset.export_manifest.json",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.scoreCachePath).toBe("");
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
          export_manifest_path: "C:\\data\\ml_dataset.export_manifest.json",
          split_manifest_path: "C:\\data\\ml_dataset.split_manifest.json",
          feature_schema_path: "C:\\data\\ml_dataset.feature_schema.json",
          feature_schema_version: 1,
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
          score_space: "Probability",
          score_scope: "CalibrationSlice",
          coverage_ratio: 0.75,
          duplicate_timestamps: 2,
        };
      }
      if (command === "run_ml_signal_validation") {
        return {
          runtime_ms: 42,
          candidates: [{ params: {}, robust_score: 1.2 }],
          score_space: "Probability",
          score_scope: "CalibrationSlice",
          coverage_ratio: 0.75,
          duplicate_timestamps: 2,
          run_verdict: {
            approved_candidate_count: 1,
            candidates_survived_hard_gates: 1,
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
        }),
      },
    );
    expect(invokeCommandMock).toHaveBeenCalledWith("run_ml_signal_validation", {
      req: expect.objectContaining({
        data_id: "data-1",
        score_cache_path: "C:\\data\\score_cache.json",
        min_psr: 0.6,
        min_dsr: 0.4,
      }),
    });

    expect(workflowArtifacts.currentRun?.req).toMatchObject({
      strategy_kind: "MlSignal",
    });
    expect(workflowArtifacts.currentRun?.mlSignal?.stage).toBe("validation");
    expect(workflowArtifacts.currentRun?.mlSignal?.datasetExportPath).toBe(
      "C:\\data\\ml_dataset.csv",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.scoreSpace).toBe(
      "Probability",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.scoreScope).toBe(
      "CalibrationSlice",
    );
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
            candidates: [
              {
                robust_score: 1.44,
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
        }),
      },
    );
    expect(useOptimizeStore(pinia).mlSignalSweepMeta).toContain(
      "27 candidates",
    );
    expect(workflowArtifacts.currentRun?.mlSignal?.policy).toMatchObject({
      label: "best_sweep",
      t_long: 0.6,
      max_hold_bars: 10,
    });
    expect(workflowArtifacts.currentRun?.mlSignal?.stage).toBe("policy_sweep");
    expect(workflowArtifacts.currentRun?.mlSignal?.policySweep).toMatchObject({
      sweep_size: 27,
      top_k: 4,
      best_label: "best_sweep",
    });
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
