// @vitest-environment jsdom
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import MlSignalWorkspacePanel from "./MlSignalWorkspacePanel.vue";
import { useDataStore } from "../stores/dataStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useStrategyStore } from "../stores/strategyStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => ({})),
  tauriRuntimeAvailable: vi.fn(() => false),
}));

describe("MlSignalWorkspacePanel", () => {
  let pinia: ReturnType<typeof createPinia>;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("renders sweep controls and current ML artifact details", () => {
    const data = useDataStore();
    const optimize = useOptimizeStore();
    const workflowArtifacts = useWorkflowArtifactsStore();

    data.dataId = "data-1";
    data.datasetLabel = "prices.csv";
    data.dataTimeframe = "1d";

    optimize.mlSignalScoreCachePath = "C:\\data\\score_cache.json";
    workflowArtifacts.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-20T10:00:00.000Z",
        artifact_path: "runs/vectorgrid_run_ml_signal.json",
        csv_path: "C:\\data\\prices.csv",
        ml_signal: {
          stage: "validation",
          bundle_id: "bundle-a",
          dataset_export_path: "C:\\data\\ml_dataset.csv",
          export_manifest_path: "C:\\data\\ml_dataset.export_manifest.json",
          split_manifest_path: "C:\\data\\ml_dataset.split_manifest.json",
          score_cache_path: "C:\\data\\score_cache.json",
          score_manifest_path: "C:\\data\\score_cache.manifest.json",
          feature_schema_path: "C:\\data\\ml_dataset.feature_schema.json",
          feature_schema_version: 1,
          feature_schema_hash: "schemahash0123456789abcdef",
          exporter: {
            exporter_version: "desktop-ml-exporter@2026.04.15",
          },
          provenance: {
            data_timeframe: "1d",
            source_export_manifest_hash: "exporthash0123456789abcdef",
            source_feature_schema_hash: "sourcefeaturehash0123456789abcdef",
            threshold_policy_default: "probability_threshold_only",
          },
          score_contract: {
            validation_scores_scope: "calibration_slice",
            validation_scores_preferred_probability_columns_after_calibration: [
              "p_long_cal",
              "p_short_cal",
            ],
            raw_margin_columns: ["raw_long", "raw_short"],
            desktop_validation_requires_probability_columns: true,
          },
          score_contract_version: "3",
          score_cache_provenance: {
            source_bundle_id: "bundle-a",
            source_bundle_manifest_hash: "bundlehash0123456789abcdef",
            calibration_state: "calibrated",
            threshold_policy: "probability_threshold_only",
            producer_kind: "catboost_pair",
          },
          bundle_status: {
            state: "warning",
            manifest: {
              model_family: "catboost_binary_pair",
              model_format: "cbm",
              calibration_path: "calibration.json",
            },
            issues: ["schema hash mismatch"],
          },
          score_space: "Probability",
          score_scope: "CalibrationSlice",
          matched_rows: 75,
          skipped_rows: 4,
          bars_scored: 75,
          coverage_ratio: 0.75,
          duplicate_timestamps: 2,
          policy: {
            label: "best_policy",
            t_long: 0.6,
            t_short: 0.61,
            delta: 0.08,
            max_hold_bars: 12,
          },
          policy_sweep: {
            sweep_size: 27,
            top_k: 4,
            best_label: "best_policy",
            best_robust_score: 1.44,
          },
        },
        req: {},
        res: {},
      },
      currentValidation: {
        ts: "2026-03-20T10:00:00.000Z",
        req: {},
        res: {
          sweep_size: 27,
          validation: {
            run_verdict: {
              approved_candidate_count: 1,
              candidates_survived_hard_gates: 1,
              candidates_considered: 27,
            },
            candidates: [
              {
                robust_score: 1.44,
                psr: 0.72,
                dsr: 0.68,
                in_sample: {
                  sharpe: 1.23,
                  pnl: 0.34,
                  max_dd: 0.12,
                  trades: 18,
                },
                params: {
                  label: "best_policy",
                  t_long: 0.6,
                  t_short: 0.61,
                  delta: 0.08,
                  max_hold_bars: 12,
                },
              },
            ],
          },
        },
      },
    });

    const wrapper = mount(MlSignalWorkspacePanel, {
      global: { plugins: [pinia] },
    });

    expect(wrapper.find("#mlSignalSweepLongStart").exists()).toBe(true);
    expect(wrapper.find("#mlSignalSweepTopK").exists()).toBe(true);
    expect(wrapper.find("#mlSignalMinPsr").exists()).toBe(true);
    expect(wrapper.find("#mlSignalMinDsr").exists()).toBe(true);
    expect(wrapper.text()).toContain("Current ML artifact");
    expect(wrapper.text()).toContain("Stage");
    expect(wrapper.text()).toContain("validation");
    expect(wrapper.text()).toContain("bundle-a");
    expect(wrapper.text()).toContain("ml_dataset.csv");
    expect(wrapper.text()).toContain("ml_dataset.export_manifest.json");
    expect(wrapper.text()).toContain("ml_dataset.split_manifest.json");
    expect(wrapper.text()).toContain("score_cache.json");
    expect(wrapper.text()).toContain("ml_dataset.feature_schema.json");
    expect(wrapper.text()).toContain("Schema version");
    expect(wrapper.text()).toContain(
      "Schema v1 • hash schemahash…abcdef is required for score-cache compatibility and saved-run reopen consistency.",
    );
    expect(wrapper.text()).toContain("Provenance");
    expect(wrapper.text()).toContain(
      "schema schemahash…abcdef • exporter desktop-ml-exporter@2026.04.15 • timeframe 1d • contract v3 • source bundle-a • calibration calibrated • bundle warning",
    );
    expect(wrapper.text()).toContain("Score contract");
    expect(wrapper.text()).toContain(
      "v3 • validation calibration_slice • prefer p_long_cal/p_short_cal • raw raw_long/raw_short • probability required",
    );
    expect(wrapper.text()).toContain("Cache lineage");
    expect(wrapper.text()).toContain(
      "producer catboost_pair • threshold probability_threshold_only • bundle manifest bundlehash…abcdef • export exporthash…abcdef • source schema sourcefeat…abcdef",
    );
    expect(wrapper.text()).toContain("Bundle status");
    expect(wrapper.text()).toContain(
      "warning • catboost_binary_pair • cbm • calibration file • schema hash mismatch",
    );
    expect(wrapper.text()).toContain(
      "Probability • CalibrationSlice • matched 75 • skipped 4 • bars 75 • coverage 75.0% • duplicates 2",
    );
    expect(wrapper.text()).toContain(
      "Stage 3 owns the validation result and current policy selection.",
    );
    expect(wrapper.text()).toContain("best_policy");
    expect(wrapper.text()).toContain("size 27");
    expect(wrapper.text()).toContain("Policy sweep attached");
    expect(wrapper.text()).toContain("Top retained candidates");
    expect(wrapper.text()).toContain("robust 1.4400");
    expect(wrapper.text()).toContain("approved 1");
    expect(wrapper.text()).toContain("sharpe 1.2300");
    expect(wrapper.text()).toContain("psr 0.7200");
  });

  it("shows a score-contract mismatch note when the artifact scope disagrees with the declared lanes", () => {
    const workflowArtifacts = useWorkflowArtifactsStore();
    workflowArtifacts.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-20T10:00:00.000Z",
        artifact_path: "runs/vectorgrid_run_ml_signal_mismatch.json",
        csv_path: "C:\\data\\prices.csv",
        ml_signal: {
          stage: "validation",
          score_scope: "CustomPartial",
          score_contract: {
            validation_scores_scope: "calibration_slice",
            walkforward_scores_scope: "walkforward_test",
          },
        },
        req: {},
        res: {},
      },
      currentValidation: null,
    });

    const wrapper = mount(MlSignalWorkspacePanel, {
      global: { plugins: [pinia] },
    });

    expect(wrapper.text()).toContain(
      "Score scope CustomPartial does not match the declared score-contract lanes (calibration_slice / walkforward_test).",
    );
  });

  it("surfaces native score-cache backend provenance in the current artifact summary", () => {
    const workflowArtifacts = useWorkflowArtifactsStore();
    workflowArtifacts.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-20T10:00:00.000Z",
        artifact_path: "runs/vectorgrid_run_ml_signal_native.json",
        csv_path: "C:\\data\\prices.csv",
        ml_signal: {
          stage: "score_cache",
          score_cache_path: "C:\\data\\score_cache.json",
          score_manifest_path: "C:\\data\\score_cache.manifest.json",
          feature_schema_path: "C:\\data\\ml_dataset.feature_schema.json",
          feature_schema_version: 1,
          feature_schema_hash: "schemahash0123456789abcdef",
          native_backend_used: "native_cpu",
          native_gpu_device_id: 1,
          native_fallback_reason: "EnableGPUEvaluation unavailable",
          score_space: "Probability",
          score_scope: "FullDataset",
          matched_rows: 75,
          skipped_rows: 0,
          bars_scored: 75,
          coverage_ratio: 0.75,
          duplicate_timestamps: 0,
          score_cache_provenance: {
            producer_kind: "catboost_native_cpu",
            native_backend_requested: "native_cuda",
            native_backend_used: "native_cpu",
            native_library_path: "C:\\catboost\\catboostmodel.dll",
            native_gpu_device_id: 1,
            native_fallback_reason: "EnableGPUEvaluation unavailable",
          },
        },
        req: {},
        res: {},
      },
      currentValidation: null,
    });

    const wrapper = mount(MlSignalWorkspacePanel, {
      global: { plugins: [pinia] },
    });

    expect(wrapper.text()).toContain("native native_cpu");
    expect(wrapper.text()).toContain("gpu 1");
    expect(wrapper.text()).toContain(
      "fallback EnableGPUEvaluation unavailable",
    );
  });

  it("keeps stage buttons disabled until the matching ML prerequisites exist", async () => {
    const optimize = useOptimizeStore();

    const wrapper = mount(MlSignalWorkspacePanel, {
      global: { plugins: [pinia] },
    });

    const exportButton = wrapper
      .findAll("button")
      .find((button) => button.text().trim() === "Export dataset");
    const buildButton = wrapper
      .findAll("button")
      .find((button) => button.text().trim() === "Build score cache");
    const validateButton = wrapper
      .findAll("button")
      .find((button) => button.text().trim() === "Run ML validation");
    const sweepButton = wrapper
      .findAll("button")
      .find((button) => button.text().trim() === "Run ML policy sweep");

    expect(exportButton?.attributes("disabled")).toBeDefined();
    expect(buildButton?.attributes("disabled")).toBeDefined();
    expect(validateButton?.attributes("disabled")).toBeDefined();
    expect(sweepButton?.attributes("disabled")).toBeDefined();
    expect(wrapper.text()).toContain("No ML validation artifact attached yet.");

    const data = useDataStore();
    data.dataId = "data-1";
    optimize.mlSignalDatasetExportPath = "C:\\data\\ml_dataset.csv";
    optimize.mlSignalScoreInputPath = "C:\\data\\scores.csv";
    optimize.mlSignalScoreCachePath = "C:\\data\\score_cache.json";
    await nextTick();

    expect(exportButton?.attributes("disabled")).toBeUndefined();
    expect(buildButton?.attributes("disabled")).toBeUndefined();
    expect(validateButton?.attributes("disabled")).toBeUndefined();
    expect(sweepButton?.attributes("disabled")).toBeUndefined();
  });

  it("switches Stage 2 between CSV import and native bundle controls", async () => {
    const data = useDataStore();
    const optimize = useOptimizeStore();

    data.dataId = "data-1";
    optimize.mlSignalScoreCachePath = "C:\\data\\score_cache.json";
    optimize.mlSignalScoreInputPath = "C:\\scores\\model_scores.csv";

    const wrapper = mount(MlSignalWorkspacePanel, {
      global: { plugins: [pinia] },
    });

    const sourceSelect = wrapper.get("#mlSignalScoreSource");
    const buildButton = wrapper
      .findAll("button")
      .find((button) => button.text().trim() === "Build score cache");

    expect(wrapper.find("#mlSignalScoreInputPath").exists()).toBe(true);
    expect(wrapper.find("#mlSignalNativeBackend").exists()).toBe(false);
    expect(buildButton?.attributes("disabled")).toBeUndefined();

    await sourceSelect.setValue("native");
    await nextTick();

    expect(optimize.mlSignalScoreSource).toBe("native");
    expect(wrapper.find("#mlSignalScoreInputPath").exists()).toBe(false);
    expect(wrapper.find("#mlSignalNativeBackend").exists()).toBe(true);
    expect(wrapper.find("#mlSignalNativeLibraryPath").exists()).toBe(true);
    expect(wrapper.find("#mlSignalNativeGpuDeviceId").exists()).toBe(true);
    expect(wrapper.text()).toContain("Native bundle inference");
    expect(wrapper.text()).toContain(
      "Bundle selection is required for native score-cache generation.",
    );
    expect(wrapper.text()).toContain(
      "Native mode still emits the standard cache artifact",
    );
    expect(buildButton?.attributes("disabled")).toBeDefined();

    optimize.mlSignalModelBundleId = "bundle-a";
    await nextTick();

    expect(buildButton?.attributes("disabled")).toBeUndefined();
  });

  it("summarizes split-plan, ticker-holdout, and export sidecars in the dataset export stage", () => {
    const data = useDataStore();
    const optimize = useOptimizeStore();
    const workflowArtifacts = useWorkflowArtifactsStore();

    data.dataId = "data-1";
    optimize.mlSignalSplitTrainBars = "252";
    optimize.mlSignalSplitCalibrationBars = "63";
    optimize.mlSignalSplitTestBars = "63";
    optimize.mlSignalSplitGapBars = "10";
    optimize.mlSignalSplitStepBars = "63";
    optimize.mlSignalHoldoutModulus = "5";
    optimize.mlSignalHoldoutResidue = "7";

    workflowArtifacts.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-20T09:00:00.000Z",
        artifact_path: "runs/vectorgrid_ml_dataset.json",
        csv_path: "C:\\data\\prices.csv",
        ml_signal: {
          stage: "dataset_export",
          dataset_export_path: "C:\\data\\ml_dataset.csv",
          export_manifest_path: "C:\\data\\ml_dataset.export_manifest.json",
          split_manifest_path: "C:\\data\\ml_dataset.split_manifest.json",
          feature_schema_path: "C:\\data\\ml_dataset.feature_schema.json",
        },
        req: {},
        res: {},
      },
      currentValidation: null,
    });

    const wrapper = mount(MlSignalWorkspacePanel, {
      global: { plugins: [pinia] },
    });

    const text = wrapper.text();
    expect(text).toContain("Split plan");
    expect(text).toContain(
      "train 252 • calibration 63 • test 63 • gap 10 • step 63",
    );
    expect(text).toContain("ticker_id % 5 == 2");
    expect(text).toContain("ml_dataset.csv");
    expect(text).toContain("schema ml_dataset.feature_schema.json");
    expect(text).toContain("manifest ml_dataset.export_manifest.json");
    expect(text).toContain("splits ml_dataset.split_manifest.json");
    expect(text).toContain("Dataset export always writes the CSV");
    expect(text).toContain("Score-cache manifests are Stage 2 artifacts");
    expect(text).toContain(
      "Gap bars separate train to calibration and calibration to test windows.",
    );
    expect(text).toContain(
      "blank gap and step fields resolve to gap 0 and step 1",
    );
  });

  it("renders the ML score-cache, validation, and sweep contract notes in the panel", () => {
    const data = useDataStore();
    const optimize = useOptimizeStore();
    const strategy = useStrategyStore();
    const workflowArtifacts = useWorkflowArtifactsStore();

    data.dataId = "data-1";
    data.datasetLabel = "prices.csv";
    data.dataTimeframe = "1d";
    data.csvPath = "C:\\data\\prices.csv";

    optimize.mlSignalModelBundleId = "bundle-a";
    optimize.mlSignalModelStatus =
      "Model bundle: bundle-a • healthy • C:\\models\\ml_signal\\bundle-a\\manifest.json";
    optimize.mlSignalScoreInputPath = "C:\\scores\\model_scores.csv";
    optimize.mlSignalScoreCachePath = "C:\\data\\score_cache.json";
    optimize.mlSignalLongThreshold = "0.63";
    optimize.mlSignalShortThreshold = "0.58";
    optimize.mlSignalDelta = "0.07";
    optimize.mlSignalMaxHoldBars = "11";
    optimize.mlSignalSweepTopK = "10";
    strategy.longOnly = false;
    strategy.allowFlip = true;

    workflowArtifacts.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-20T10:00:00.000Z",
        artifact_path: "runs/vectorgrid_run_ml_signal.json",
        csv_path: "C:\\data\\prices.csv",
        ml_signal: {
          stage: "score_cache",
          score_cache_path: "C:\\data\\score_cache.json",
          score_manifest_path: "C:\\data\\score_cache.manifest.json",
        },
        req: {},
        res: {},
      },
      currentValidation: null,
    });

    const wrapper = mount(MlSignalWorkspacePanel, {
      global: { plugins: [pinia] },
    });

    const text = wrapper.text();
    expect(text).toContain(
      "Model bundles are discovered from the desktop app-local inventory.",
    );
    expect(text).toContain("Score source");
    expect(text).toContain("CSV import");
    expect(text).toContain("Native bundle inference");
    expect(text).toContain("timestamp");
    expect(text).toContain("p_long");
    expect(text).toContain("raw_long");
    expect(text).toContain("p_long_cal");
    expect(text).toContain("Raw-margin-only caches can build successfully");
    expect(text).toContain("Validation context");
    expect(text).toContain(
      "MlSignal • prices.csv • timeframe 1d • csv prices.csv",
    );
    expect(text).toContain("Current policy");
    expect(text).toContain("tL 0.63 • tS 0.58 • d 0.07 • hold 11");
    expect(text).toContain(
      "Current strategy is long+short, so validation requires both long and short probability coverage.",
    );
    expect(text).toContain("Sweep axes");
    expect(text).toContain(
      "UI retain top 10. Rust still defaults top_k to 25 when omitted",
    );
  });
});
