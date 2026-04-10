import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { bindRuntimePinia } from "../stores/runtimePinia";
import {
  applySavedRun,
  applySession,
  clearSession,
  deleteSavedRun,
  exportPreset,
  importPreset,
  loadSavedRun,
  loadSavedRunDetail,
  openSavedRunReport,
  readSessionHistory,
  refreshSavedRuns,
  revealSavedRun,
  rerunSavedRun,
  saveSavedRunTags,
} from "./historyService";
import { invokeCommand } from "../tauri/invoke";
import { HISTORY_KEY, SETTINGS_KEY } from "../utils/core/storageKeys";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";
import { useAdvancedStore } from "../stores/advancedStore";
import { useDataStore } from "../stores/dataStore";
import { useExecutionStore } from "../stores/executionStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useStrategyStore } from "../stores/strategyStore";
import { useValidationStore } from "../stores/validationStore";
import { useWorkflowStore } from "../stores/workflowStore";
import {
  applyTimeframe as applyDataTimeframe,
  loadCsv as loadDataCsv,
} from "./dataService";
import { runOptimization } from "./optimizeService";
import { refreshAnalysisFromWorkflowRun } from "./analysisService";
import {
  exportMlSignalDataset,
  generateMlSignalScoreCache,
  runMlSignalPolicySweep,
  runMlSignalValidation,
} from "./mlSignalService";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => undefined),
}));

vi.mock("./dataService", () => ({
  loadCsv: vi.fn(async () => ({
    path: "C:/data/aapl.csv",
    dataId: "data-1",
    cacheHit: false,
    cachePath: "",
    loadSource: "csv_parsed_and_cached",
    meta: {
      len: 100,
      hasOpen: true,
      hasHigh: true,
      hasLow: true,
      hasClose: true,
      hasVolume: true,
      minTs: 1_700_000_000,
      maxTs: 1_700_100_000,
      timeframe: "1m",
      sourceSizeBytes: null,
      sourceMtimeUtcMs: null,
    },
  })),
  applyTimeframe: vi.fn(async () => ({
    path: "",
    dataId: "data-1@5m",
    cacheHit: false,
    cachePath: "",
    loadSource: "derived_fresh_resample",
    meta: {
      len: 20,
      hasOpen: true,
      hasHigh: true,
      hasLow: true,
      hasClose: true,
      hasVolume: true,
      minTs: 1_700_000_000,
      maxTs: 1_700_100_000,
      timeframe: "5m",
      sourceSizeBytes: null,
      sourceMtimeUtcMs: null,
    },
  })),
}));

vi.mock("./optimizeService", () => ({
  runOptimization: vi.fn(async () => undefined),
}));

vi.mock("./analysisService", () => ({
  refreshAnalysisFromWorkflowRun: vi.fn(() => true),
}));

vi.mock("./mlSignalService", () => ({
  exportMlSignalDataset: vi.fn(async () => undefined),
  generateMlSignalScoreCache: vi.fn(async () => undefined),
  runMlSignalValidation: vi.fn(async () => undefined),
  runMlSignalPolicySweep: vi.fn(async () => undefined),
}));

const exportMlSignalDatasetMock = vi.mocked(exportMlSignalDataset);
const invokeCommandMock = vi.mocked(invokeCommand);
const loadDataCsvMock = vi.mocked(loadDataCsv);
const applyDataTimeframeMock = vi.mocked(applyDataTimeframe);
const runOptimizationMock = vi.mocked(runOptimization);
const refreshAnalysisFromWorkflowRunMock = vi.mocked(
  refreshAnalysisFromWorkflowRun,
);
const generateMlSignalScoreCacheMock = vi.mocked(generateMlSignalScoreCache);
const runMlSignalValidationMock = vi.mocked(runMlSignalValidation);
const runMlSignalPolicySweepMock = vi.mocked(runMlSignalPolicySweep);

function installStorageHarness(
  seed: Record<string, string> = {},
): Map<string, string> {
  const state = new Map<string, string>(Object.entries(seed));
  vi.stubGlobal("window", {
    localStorage: {
      getItem: (key: string) =>
        state.has(key) ? String(state.get(key)) : null,
      setItem: (key: string, value: string) => {
        state.set(String(key), String(value));
      },
      removeItem: (key: string) => {
        state.delete(String(key));
      },
    },
  });
  return state;
}

describe("historyService", () => {
  let pinia: ReturnType<typeof createPinia>;

  beforeEach(() => {
    vi.unstubAllGlobals();
    pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    vi.clearAllMocks();
    installStorageHarness();
    loadDataCsvMock.mockResolvedValue({
      path: "C:/data/aapl.csv",
      dataId: "data-1",
      cacheHit: false,
      cachePath: "",
      loadSource: "csv_parsed_and_cached",
      meta: {
        len: 100,
        hasOpen: true,
        hasHigh: true,
        hasLow: true,
        hasClose: true,
        hasVolume: true,
        minTs: 1_700_000_000,
        maxTs: 1_700_100_000,
        timeframe: "1m",
        sourceSizeBytes: null,
        sourceMtimeUtcMs: null,
      },
    });
    applyDataTimeframeMock.mockResolvedValue({
      path: "",
      dataId: "data-1@5m",
      cacheHit: false,
      cachePath: "",
      loadSource: "derived_fresh_resample",
      meta: {
        len: 20,
        hasOpen: true,
        hasHigh: true,
        hasLow: true,
        hasClose: true,
        hasVolume: true,
        minTs: 1_700_000_000,
        maxTs: 1_700_100_000,
        timeframe: "5m",
        sourceSizeBytes: null,
        sourceMtimeUtcMs: null,
      },
    });
    runOptimizationMock.mockResolvedValue(undefined);
    refreshAnalysisFromWorkflowRunMock.mockClear();
    refreshAnalysisFromWorkflowRunMock.mockReturnValue(true);
    exportMlSignalDatasetMock.mockResolvedValue(undefined);
    generateMlSignalScoreCacheMock.mockResolvedValue(undefined);
    runMlSignalValidationMock.mockResolvedValue(undefined);
    runMlSignalPolicySweepMock.mockResolvedValue(undefined);
  });

  it("exports preset from settings storage through browser download flow", async () => {
    installStorageHarness({
      [SETTINGS_KEY]: JSON.stringify({
        objective: "Sharpe",
        backend: "GpuKernel",
      }),
    });
    const click = vi.fn();
    const remove = vi.fn();
    const appendChild = vi.fn();
    const anchor = {
      href: "",
      download: "",
      style: { display: "" },
      click,
      remove,
    };
    const createElement = vi.fn(() => anchor);
    vi.stubGlobal("document", {
      createElement,
      body: { appendChild },
    });
    const createObjectURL = vi.fn((value: Blob) => {
      void value;
      return "blob://preset";
    });
    const revokeObjectURL = vi.fn();
    vi.stubGlobal("URL", {
      createObjectURL,
      revokeObjectURL,
    });

    await exportPreset();

    expect(createElement).toHaveBeenCalledWith("a");
    expect(appendChild).toHaveBeenCalled();
    expect(click).toHaveBeenCalled();
    expect(revokeObjectURL).toHaveBeenCalledWith("blob://preset");
    const firstCreateObjectUrlCall = createObjectURL.mock.calls[0];
    expect(firstCreateObjectUrlCall).toBeTruthy();
    const blob = firstCreateObjectUrlCall?.[0] as Blob;
    const parsed = JSON.parse(await blob.text()) as {
      kind?: string;
      preset?: Record<string, unknown>;
    };
    expect(parsed.kind).toBe("vectorgrid_preset");
    expect(parsed.preset).toEqual({
      objective: "Sharpe",
      backend: "GpuKernel",
    });
  });

  it("imports preset through file picker and applies values to Vue stores", async () => {
    const storage = installStorageHarness();
    const fileText = JSON.stringify({
      version: 1,
      kind: "vectorgrid_preset",
      preset: {
        backend: "GpuKernel",
        objective: "Sharpe",
        valAutoRun: true,
        execMode: "live",
        settingsThemeSelect: "dark",
      },
    });
    const showOpenFilePicker = vi.fn(async () => [
      {
        getFile: async () => ({
          text: async () => fileText,
        }),
      },
    ]);
    vi.stubGlobal("showOpenFilePicker", showOpenFilePicker);

    await importPreset();

    const storedPreset = JSON.parse(
      String(storage.get(SETTINGS_KEY) || "{}"),
    ) as Record<string, unknown>;
    expect(storedPreset.backend).toBe("GpuKernel");
    expect(useOptimizeStore(pinia).backend).toBe("GpuKernel");
    expect(useOptimizeStore(pinia).objective).toBe("Sharpe");
    expect(useValidationStore(pinia).autoRun).toBe(true);
    expect(useExecutionStore(pinia).mode).toBe("live");
    expect(useSettingsStore(pinia).theme).toBe("dark");
  });

  it("reads session history from storage and applies selected session run to workflow artifacts", async () => {
    installStorageHarness({
      [HISTORY_KEY]: JSON.stringify([
        {
          ts: "2026-02-25T00:00:00Z",
          backend: "CpuOnly",
          pairs: 1234,
          req: { strategy_kind: "DoubleMa", data_id: "data-1" },
          res: { top: [[{ fast_len: 5, slow_len: 20 }, { sharpe: 1.2 }]] },
        },
      ]),
    });

    const sessions = readSessionHistory();
    await applySession("2026-02-25T00:00:00Z");

    const artifacts = useWorkflowArtifactsStore(pinia);
    expect(sessions).toEqual([
      {
        ts: "2026-02-25T00:00:00Z",
        backend: "CpuOnly",
        pairs: 1234,
      },
    ]);
    expect(artifacts.currentRun?.ts).toBe("2026-02-25T00:00:00Z");
    expect(
      (artifacts.currentRun?.req as { strategy_kind?: string } | null)
        ?.strategy_kind,
    ).toBe("DoubleMa");
    expect(artifacts.currentValidation).toBeNull();
  });

  it("clears session history storage and workflow artifacts", async () => {
    const storage = installStorageHarness({
      [HISTORY_KEY]: JSON.stringify([
        {
          ts: "2026-02-25T00:00:00Z",
          req: { strategy_kind: "DoubleMa" },
          res: { top: [] },
        },
      ]),
    });
    const artifacts = useWorkflowArtifactsStore(pinia);
    artifacts.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T00:00:00Z",
        req: { strategy_kind: "DoubleMa" },
        res: { top: [] },
      },
      currentValidation: {
        ts: "2026-02-25T00:00:00Z",
        req: {},
        res: {},
      },
    });

    await clearSession();

    expect(storage.get(HISTORY_KEY)).toBe("[]");
    expect(artifacts.currentRun).toBeNull();
    expect(artifacts.currentValidation).toBeNull();
  });

  it("loads saved run artifact directly and hydrates workflow artifacts store", async () => {
    useWorkflowStore(pinia).selectMode("ml-signal", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      currentRun: {
        ts: "2026-02-25T00:00:00Z",
        artifact_path: "C:/runs/run1.json",
        report_xlsx_path: "C:/runs/run1.xlsx",
        csv_path: "C:/data/aapl.csv",
        tags: ["holdout", " 1m "],
        dataset_meta: {
          csv_path: "C:/data/aapl.csv",
          timeframe: "15m",
          base_timeframe: "1m",
          derived_timeframe: "15m",
          resample_applied: true,
          source_content_fingerprint: "fp-aapl-1m",
          symbol: "AAPL",
          label: "AAPL",
        },
        req: { strategy_kind: "DoubleMa", data_id: "data-1" },
        res: { top: [[{ fast_len: 5, slow_len: 20 }, { sharpe: 1.2 }]] },
      },
      currentValidation: {
        ts: "2026-02-25T00:00:00Z",
        req: { strategy_kind: "DoubleMa" },
        res: { candidates: [{ score: 1.0 }] },
      },
    });

    await loadSavedRun(" C:/runs/run1.json ");

    const artifacts = useWorkflowArtifactsStore(pinia);
    expect(useWorkflowStore(pinia).mode).toBe("conventional");
    expect(invokeCommandMock).toHaveBeenCalledWith("load_run_artifact_hydration", {
      path: "C:/runs/run1.json",
    });
    expect(artifacts.currentRun?.artifactPath).toBe("C:/runs/run1.json");
    expect(artifacts.currentRun?.reportXlsxPath).toBe("C:/runs/run1.xlsx");
    expect(artifacts.currentRun?.csvPath).toBe("C:/data/aapl.csv");
    expect(artifacts.currentRun?.datasetMeta).toEqual({
      csv_path: "C:/data/aapl.csv",
      timeframe: "15m",
      base_timeframe: "1m",
      derived_timeframe: "15m",
      resample_applied: true,
      source_content_fingerprint: "fp-aapl-1m",
      symbol: "AAPL",
      label: "AAPL",
    });
    expect(artifacts.currentRun?.tags).toEqual(["holdout", "1m"]);
    expect(artifacts.currentValidation?.ts).toBe("2026-02-25T00:00:00Z");
    expect(refreshAnalysisFromWorkflowRunMock).toHaveBeenCalled();
  });

  it("loads MlSignal saved run artifact without routing into optimizer analysis refresh", async () => {
    useWorkflowStore(pinia).selectMode("conventional", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      currentRun: {
        ts: "2026-03-20T00:00:00Z",
        artifact_path: "C:/runs/ml_signal.json",
        csv_path: "C:/data/spy.csv",
        ml_signal: {
          stage: "validation",
          score_cache_path: "C:/runs/ml_score_cache.json",
          score_scope: "CalibrationSlice",
          coverage_ratio: 0.75,
        },
        req: {
          strategy_kind: "MlSignal",
          data_id: "data-ml",
          input_path: "C:/scores/ml_scores.csv",
          output_path: "C:/runs/ml_score_cache.json",
          model_bundle_id: "bundle-a",
        },
        res: { matched_rows: 123, output_path: "C:/runs/ml_score_cache.json" },
      },
      currentValidation: {
        ts: "2026-03-20T00:00:00Z",
        req: {
          candidates: [
            { t_long: 0.6, t_short: 0.55, delta: 0.1, max_hold_bars: 10 },
          ],
        },
        res: { candidates: [{ robust_score: 1.1 }] },
      },
    });

    await loadSavedRun(" C:/runs/ml_signal.json ");

    const artifacts = useWorkflowArtifactsStore(pinia);
    expect(useWorkflowStore(pinia).mode).toBe("ml-signal");
    expect(useWorkflowStore(pinia).workflowStep).toBe("validate");
    expect(artifacts.currentRun?.artifactPath).toBe("C:/runs/ml_signal.json");
    expect(artifacts.currentRun?.mlSignal?.stage).toBe("validation");
    expect(artifacts.currentRun?.mlSignal?.scoreScope).toBe(
      "CalibrationSlice",
    );
    expect(
      (artifacts.currentRun?.req as { strategy_kind?: string } | null)
        ?.strategy_kind,
    ).toBe("MlSignal");
    expect(refreshAnalysisFromWorkflowRunMock).not.toHaveBeenCalled();
  });

  it("applies saved run request into Vue optimize/advanced/data stores", async () => {
    useWorkflowStore(pinia).selectMode("ml-signal", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      strategyKind: "DoubleMa",
      timeframe: "5m",
      csvPath: "C:/data/msft.csv",
      runReq: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        mode: "GpuProgressive",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
        score_mode: "balanced_risk_adjusted_10",
        hurdle_tau: 0.02,
        metric_contract: "trade_22",
        include_trade_metrics: true,
        extra_shortlist_objectives: ["Pnl", "Omega"],
        shortlist_confidence_mode: "dsr",
        shortlist_union_mode: "weighted_rank",
        shortlist_slice_mode: "halves",
        shortlist_slice_benchmark: "buy_and_hold",
        shortlist_neighborhood_radius: 2,
        shortlist_neighborhood_step: 3,
        top_k: 32,
        include_all: false,
        heatmap_bins: 256,
        export_all_csv_path: "C:/runs/all_pairs.csv",
        fast_range: { start: 5, end: 20, step: 1 },
        slow_range: { start: 21, end: 80, step: 1 },
        fast_ma_types: ["sma"],
        slow_ma_types: ["alma"],
      },
      validationReq: {
        max_candidates: 12,
        seed: 7,
        min_psr: 0.55,
        min_dsr: 0.4,
        admission_policy: "scalar_union",
      },
    });

    await applySavedRun(" C:/runs/run1.json ");

    const optimize = useOptimizeStore(pinia);
    const advanced = useAdvancedStore(pinia);
    const data = useDataStore(pinia);
    const validation = useValidationStore(pinia);
    expect(useWorkflowStore(pinia).mode).toBe("conventional");
    expect(invokeCommandMock).toHaveBeenCalledWith("load_run_artifact_detail", {
      path: "C:/runs/run1.json",
    });
    expect(optimize.strategyKind).toBe("DoubleMa");
    expect(optimize.backend).toBe("GpuKernel");
    expect(optimize.mode).toBe("Grid");
    expect(optimize.fastStart).toBe("5");
    expect(optimize.slowEnd).toBe("80");
    expect(advanced.maSource).toBe("close");
    expect(advanced.mode).toBe("Grid");
    expect(advanced.scoreMode).toBe("balanced_risk_adjusted_10");
    expect(advanced.hurdleTau).toBe("0.02");
    expect(advanced.metricContract).toBe("trade_22");
    expect(advanced.includeTradeMetrics).toBe(true);
    expect(advanced.extraShortlistObjectives).toBe("Pnl, Omega");
    expect(advanced.shortlistConfidenceMode).toBe("dsr");
    expect(advanced.shortlistUnionMode).toBe("weighted_rank");
    expect(advanced.shortlistSliceMode).toBe("halves");
    expect(advanced.shortlistSliceBenchmark).toBe("buy_and_hold");
    expect(advanced.shortlistNeighborhoodRadius).toBe("2");
    expect(advanced.shortlistNeighborhoodStep).toBe("3");
    expect(advanced.topK).toBe("32");
    expect(advanced.includeAll).toBe("false");
    expect(advanced.heatmapBins).toBe("256");
    expect(advanced.exportAllCsvPath).toBe("C:/runs/all_pairs.csv");
    expect(data.csvPath).toBe("C:/data/msft.csv");
    expect(data.dataTimeframe).toBe("5m");
    expect(validation.candidates).toBe("12");
    expect(validation.seed).toBe("7");
    expect(validation.minPsr).toBe("0.55");
    expect(validation.minDsr).toBe("0.4");
    expect(validation.admissionPolicy).toBe("scalar_union");
  });

  it("clears stale advanced builder fields when the saved run omits them", async () => {
    useWorkflowStore(pinia).selectMode("ml-signal", { persist: false });
    const advanced = useAdvancedStore(pinia);
    advanced.scoreMode = "balanced_risk_adjusted_10";
    advanced.hurdleTau = "0.05";
    advanced.metricContract = "trade_22";
    advanced.includeTradeMetrics = true;
    advanced.extraShortlistObjectives = "Pnl, Omega";
    advanced.shortlistConfidenceMode = "dsr";
    advanced.shortlistUnionMode = "weighted_rank";
    advanced.shortlistSliceMode = "halves";
    advanced.shortlistSliceBenchmark = "buy_and_hold";
    advanced.shortlistNeighborhoodRadius = "2";
    advanced.shortlistNeighborhoodStep = "3";
    advanced.topK = "64";
    advanced.includeAll = "true";
    advanced.heatmapBins = "512";
    advanced.exportAllCsvPath = "C:/runs/stale_pairs.csv";

    invokeCommandMock.mockResolvedValueOnce({
      strategyKind: "DoubleMa",
      timeframe: "5m",
      csvPath: "C:/data/msft.csv",
      runReq: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        mode: "Grid",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
        fast_range: { start: 5, end: 20, step: 1 },
        slow_range: { start: 21, end: 80, step: 1 },
        fast_ma_types: ["sma"],
        slow_ma_types: ["alma"],
      },
    });

    await applySavedRun(" C:/runs/run_minimal.json ");

    expect(advanced.scoreMode).toBe("");
    expect(advanced.hurdleTau).toBe("");
    expect(advanced.metricContract).toBe("auto");
    expect(advanced.includeTradeMetrics).toBe(false);
    expect(advanced.extraShortlistObjectives).toBe("");
    expect(advanced.shortlistConfidenceMode).toBe("");
    expect(advanced.shortlistUnionMode).toBe("");
    expect(advanced.shortlistSliceMode).toBe("");
    expect(advanced.shortlistSliceBenchmark).toBe("");
    expect(advanced.shortlistNeighborhoodRadius).toBe("");
    expect(advanced.shortlistNeighborhoodStep).toBe("");
    expect(advanced.topK).toBe("20");
    expect(advanced.includeAll).toBe("false");
    expect(advanced.heatmapBins).toBe("128");
    expect(advanced.exportAllCsvPath).toBe("");
  });

  it("applies MlSignal saved run request into Vue optimize/data stores", async () => {
    useWorkflowStore(pinia).selectMode("conventional", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      strategyKind: "MlSignal",
      timeframe: "1d",
      csvPath: "C:/data/spy.csv",
      runReq: {
        strategy_kind: "MlSignal",
        data_id: "data-ml",
        input_path: "C:/scores/ml_scores.csv",
        output_path: "C:/runs/ml_score_cache.json",
        model_bundle_id: "bundle-a",
      },
      mlSignalStage: "validation",
      validationReq: {
        candidates: [
          {
            t_long: 0.63,
            t_short: 0.58,
            delta: 0.07,
            max_hold_bars: 11,
            exit_on_flat: true,
            exit_on_opposite: false,
            strategy: {
              long_only: true,
              allow_flip: false,
              trade_on_next_bar: false,
              fee_bps: 0.5,
              spread_bps: 1.0,
              slippage_bps: 0.25,
              cooldown_bars: 2,
              signal_persistence_bars: 3,
            },
          },
        ],
        min_psr: 0.55,
        min_dsr: 0.4,
      },
    });

    await applySavedRun(" C:/runs/ml_signal.json ");

    const optimize = useOptimizeStore(pinia);
    const data = useDataStore(pinia);
    const strategy = useStrategyStore(pinia);
    expect(useWorkflowStore(pinia).mode).toBe("ml-signal");
    expect(useWorkflowStore(pinia).workflowStep).toBe("validate");
    expect(optimize.strategyKind).not.toBe("MlSignal");
    expect(optimize.mlSignalScoreInputPath).toBe("C:/scores/ml_scores.csv");
    expect(optimize.mlSignalScoreCachePath).toBe("C:/runs/ml_score_cache.json");
    expect(optimize.mlSignalModelBundleId).toBe("bundle-a");
    expect(optimize.mlSignalLongThreshold).toBe("0.63");
    expect(optimize.mlSignalShortThreshold).toBe("0.58");
    expect(optimize.mlSignalDelta).toBe("0.07");
    expect(optimize.mlSignalMaxHoldBars).toBe("11");
    expect(optimize.mlSignalMinPsr).toBe("0.55");
    expect(optimize.mlSignalMinDsr).toBe("0.4");
    expect(strategy.longOnly).toBe(true);
    expect(strategy.allowFlip).toBe(false);
    expect(strategy.tradeOnNext).toBe(true);
    expect(strategy.feeBps).toBe("0.5");
    expect(strategy.spreadBps).toBe("1");
    expect(strategy.slippageBps).toBe("0.25");
    expect(strategy.cooldownBars).toBe("2");
    expect(strategy.signalPersistenceBars).toBe("3");
    expect(strategy.commission).toBe("0.000125");
    expect(strategy.costHint).toBe(
      "Total: 1.25 bps/side (2.50 bps round-trip)",
    );
    expect(data.csvPath).toBe("C:/data/spy.csv");
    expect(data.dataTimeframe).toBe("1d");
  });

  it("reruns saved run by loading data, optional resample, then executing optimization", async () => {
    useWorkflowStore(pinia).selectMode("ml-signal", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      timeframe: "5m",
      csvPath: "C:/data/aapl.csv",
      runReq: {
        strategy_kind: "DoubleMa",
        data_id: "stale-id",
        backend: { backend: "CpuOnly" },
        fast_range: { start: 5, end: 10, step: 1 },
        slow_range: { start: 20, end: 50, step: 1 },
      },
    });

    await rerunSavedRun(" C:/runs/run1.json ");

    expect(invokeCommandMock).toHaveBeenCalledWith("load_run_artifact_detail", {
      path: "C:/runs/run1.json",
    });
    expect(useWorkflowStore(pinia).mode).toBe("conventional");
    expect(loadDataCsvMock).toHaveBeenCalledWith("C:/data/aapl.csv");
    expect(applyDataTimeframeMock).toHaveBeenCalledWith("5m", "data-1");
    expect(runOptimizationMock).toHaveBeenCalledWith({
      request: expect.objectContaining({
        strategy_kind: "DoubleMa",
        data_id: "data-1@5m",
        dataId: "data-1@5m",
      }),
      switchTab: true,
    });
  });

  it("reruns MlSignal saved validation artifact through score-cache build then validation", async () => {
    useWorkflowStore(pinia).selectMode("conventional", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      timeframe: "1d",
      csvPath: "C:/data/spy.csv",
      runReq: {
        strategy_kind: "MlSignal",
        data_id: "stale-id",
        input_path: "C:/scores/ml_scores.csv",
        output_path: "C:/runs/ml_score_cache.json",
        model_bundle_id: "bundle-a",
      },
      mlSignalStage: "validation",
      validationReq: {
        candidates: [
          {
            t_long: 0.6,
            t_short: 0.55,
            delta: 0.1,
            max_hold_bars: 10,
          },
        ],
      },
    });

    await rerunSavedRun(" C:/runs/ml_signal.json ");

    expect(loadDataCsvMock).toHaveBeenCalledWith("C:/data/spy.csv");
    expect(useWorkflowStore(pinia).mode).toBe("ml-signal");
    expect(useWorkflowStore(pinia).workflowStep).toBe("validate");
    expect(runOptimizationMock).not.toHaveBeenCalled();
    expect(generateMlSignalScoreCacheMock).toHaveBeenCalledTimes(1);
    expect(runMlSignalValidationMock).toHaveBeenCalledTimes(1);
    expect(runMlSignalPolicySweepMock).not.toHaveBeenCalled();
    const optimize = useOptimizeStore(pinia);
    expect(optimize.mlSignalScoreInputPath).toBe("C:/scores/ml_scores.csv");
    expect(optimize.mlSignalScoreCachePath).toBe("C:/runs/ml_score_cache.json");
    expect(optimize.mlSignalModelBundleId).toBe("bundle-a");
  });

  it("reruns MlSignal dataset-export artifact through dataset export only", async () => {
    useWorkflowStore(pinia).selectMode("conventional", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      timeframe: "1d",
      csvPath: "C:/data/spy.csv",
      mlSignalStage: "dataset_export",
      runReq: {
        strategy_kind: "MlSignal",
        data_id: "stale-id",
        output_path: "C:/runs/ml_dataset.csv",
        ma_source: "close",
        horizon_bars: 10,
      },
    });

    await rerunSavedRun(" C:/runs/ml_export.json ");

    expect(exportMlSignalDatasetMock).toHaveBeenCalledTimes(1);
    expect(useWorkflowStore(pinia).mode).toBe("ml-signal");
    expect(useWorkflowStore(pinia).workflowStep).toBe("data");
    expect(generateMlSignalScoreCacheMock).not.toHaveBeenCalled();
    expect(runMlSignalValidationMock).not.toHaveBeenCalled();
    expect(runMlSignalPolicySweepMock).not.toHaveBeenCalled();
    const optimize = useOptimizeStore(pinia);
    expect(optimize.mlSignalDatasetExportPath).toBe("C:/runs/ml_dataset.csv");
    expect(optimize.mlSignalScoreCachePath).not.toBe("C:/runs/ml_dataset.csv");
  });

  it("uses direct reveal command for saved-run reveal", async () => {
    await revealSavedRun(" C:/runs/run1.json ");

    expect(invokeCommandMock).toHaveBeenCalledWith("reveal_path", {
      path: "C:/runs/run1.json",
    });
  });

  it("deletes saved run via direct command", async () => {
    await deleteSavedRun(" C:/runs/run1.json ");

    expect(invokeCommandMock).toHaveBeenCalledWith("delete_run_artifact", {
      path: "C:/runs/run1.json",
    });
  });

  it("refreshes saved runs through direct command and normalizes rows", async () => {
    invokeCommandMock.mockResolvedValueOnce([
      {
        ts: "2026-02-25T00:00:00Z",
        path: " C:/runs/run1.json ",
        bytes: 2048,
        strategy_kind: "DoubleMA",
        has_validation: true,
        report_xlsx_path: "C:/runs/run1.xlsx",
        csv_path: "C:/data/aapl.csv",
        tags: ["aapl", " 1m "],
      },
      { path: "" },
    ]);

    const summary = await refreshSavedRuns();

    expect(invokeCommandMock).toHaveBeenCalledWith("list_run_artifacts");
    expect(summary).toEqual({
      runs: [
        {
          ts: "2026-02-25T00:00:00Z",
          path: "C:/runs/run1.json",
          bytes: 2048,
          strategyKind: "DoubleMA",
          hasValidation: true,
          reportXlsxPath: "C:/runs/run1.xlsx",
          csvPath: "C:/data/aapl.csv",
          tags: ["aapl", "1m"],
        },
      ],
    });
  });

  it("saves saved-run tags via direct command", async () => {
    await saveSavedRunTags(" C:/runs/run1.json ", "holdout, 1m,,");

    expect(invokeCommandMock).toHaveBeenCalledWith("update_artifact_tags", {
      path: "C:/runs/run1.json",
      tags: ["holdout", "1m"],
    });
  });

  it("loads saved-run detail via artifact command", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      strategyKind: "CompositeMo",
      timeframe: "5m",
      csvPath: "C:/data/msft.csv",
      reportPath: "C:/runs/report.xlsx",
      hasValidation: true,
      optimization_metric_surface: {
        surface: "core17",
        fields: [
          "pnl",
          "trades",
          "max_dd",
          "mean",
          "std",
          "exposure",
          "net_exposure",
        ],
      },
      validationReq: {
        admission_policy: "scalar_union",
        admission_provenance: {
          source_kind: "top_rows",
          post_cap_count: 16,
          post_dedupe_count: 20,
          heads: [
            { objective: "Sharpe" },
            { objective: "MaxDrawdown" },
          ],
        },
      },
      tags: ["tag1", " tag2 "],
      paramsText: [
        "strategy: CompositeMo",
        "backend: CpuOnly",
        "objective: Sharpe",
        "ma source: close",
        "fast range: 5-20 x1",
        "slow range: 21-100 x1",
        "fast ma: sma",
        "slow ma: ema",
        "starting equity: 10000",
      ].join("\n"),
    });

    const detail = await loadSavedRunDetail(" C:/runs/run1.json ");

    expect(invokeCommandMock).toHaveBeenCalledWith("load_run_artifact_detail", {
      path: "C:/runs/run1.json",
    });
    expect(detail).toEqual({
      strategyKind: "CompositeMo",
      timeframe: "5m",
      csvPath: "C:/data/msft.csv",
      reportPath: "C:/runs/report.xlsx",
      hasValidation: true,
      metricSurfaceSummary: "Core17 • 7 metrics",
      metricSurfaceDetail:
        "fields=pnl, trades, max_dd, mean, std, exposure +1 more",
      validationAdmissionSummary: "scalar_union • kept 16/20",
      validationAdmissionDetail: "src=top_rows • heads=Sharpe/MaxDrawdown",
      tags: ["tag1", "tag2"],
      mlBundleId: "",
      mlScoreCachePath: "",
      mlScoreManifestPath: "",
      mlFeatureSchemaPath: "",
      mlFeatureSchemaVersion: "",
      mlPolicySummary: "",
      mlPolicySweepSummary: "",
      mlRunVerdictSummary: "",
      mlTopCandidatesSummary: "",
      paramsText: [
        "strategy: CompositeMo",
        "backend: CpuOnly",
        "objective: Sharpe",
        "ma source: close",
        "fast range: 5-20 x1",
        "slow range: 21-100 x1",
        "fast ma: sma",
        "slow ma: ema",
        "starting equity: 10000",
      ].join("\n"),
    });
  });

  it("loads MlSignal saved-run detail with policy summary", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      strategyKind: "MlSignal",
      timeframe: "1d",
      csvPath: "C:/data/spy.csv",
      reportPath: "",
      hasValidation: true,
      tags: ["ml", " research "],
      mlBundleId: "bundle-a",
      mlScoreCachePath: "C:/runs/ml_score_cache.json",
      mlScoreManifestPath: "C:/runs/ml_score_cache.manifest.json",
      mlFeatureSchemaPath: "C:/runs/ml_dataset.feature_schema.json",
      mlFeatureSchemaVersion: "1",
      mlPolicySummary: "t_long=0.6 | t_short=0.55 | delta=0.1 | hold=10",
      mlPolicySweepSummary: "size=27 | top_k=4 | best=best_sweep | score=1.44",
      mlRunVerdictSummary: "approved=1 | survived=1 | considered=27",
      mlTopCandidatesSummary: "best_sweep | robust=1 | psr=0.7 | dsr=0.6",
      paramsText: [
        "strategy: MlSignal",
        "data id: data-ml",
        "score input: C:/scores/ml_scores.csv",
        "dataset/score output: C:/runs/ml_score_cache.json",
        "model bundle: bundle-a",
        "policy t_long: 0.6",
        "policy t_short: 0.55",
        "policy delta: 0.1",
        "policy max_hold_bars: 10",
      ].join("\n"),
    });

    const detail = await loadSavedRunDetail(" C:/runs/ml_signal.json ");

    expect(detail).toEqual({
      strategyKind: "MlSignal",
      timeframe: "1d",
      csvPath: "C:/data/spy.csv",
      reportPath: "",
      hasValidation: true,
      tags: ["ml", "research"],
      mlBundleId: "bundle-a",
      mlScoreCachePath: "C:/runs/ml_score_cache.json",
      mlScoreManifestPath: "C:/runs/ml_score_cache.manifest.json",
      mlFeatureSchemaPath: "C:/runs/ml_dataset.feature_schema.json",
      mlFeatureSchemaVersion: "1",
      mlPolicySummary: "t_long=0.6 | t_short=0.55 | delta=0.1 | hold=10",
      mlPolicySweepSummary: "size=27 | top_k=4 | best=best_sweep | score=1.44",
      mlRunVerdictSummary: "approved=1 | survived=1 | considered=27",
      mlTopCandidatesSummary: "best_sweep | robust=1 | psr=0.7 | dsr=0.6",
      paramsText: [
        "strategy: MlSignal",
        "data id: data-ml",
        "score input: C:/scores/ml_scores.csv",
        "dataset/score output: C:/runs/ml_score_cache.json",
        "model bundle: bundle-a",
        "policy t_long: 0.6",
        "policy t_short: 0.55",
        "policy delta: 0.1",
        "policy max_hold_bars: 10",
      ].join("\n"),
    });
  });

  it("opens saved report from artifact detail", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      reportPath: "C:/runs/report.xlsx",
      paramsText: "",
    });
    invokeCommandMock.mockResolvedValueOnce(undefined);

    await openSavedRunReport(" C:/runs/run1.json ");

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, "load_run_artifact_detail", {
      path: "C:/runs/run1.json",
    });
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, "open_path", {
      path: "C:/runs/report.xlsx",
    });
  });

  it("falls back to reveal when opening a saved report fails", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      reportPath: "C:/runs/report.xlsx",
      paramsText: "",
    });
    invokeCommandMock.mockRejectedValueOnce(new Error("blocked"));
    invokeCommandMock.mockResolvedValueOnce(undefined);

    await openSavedRunReport(" C:/runs/run1.json ");

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, "load_run_artifact_detail", {
      path: "C:/runs/run1.json",
    });
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, "open_path", {
      path: "C:/runs/report.xlsx",
    });
    expect(invokeCommandMock).toHaveBeenNthCalledWith(3, "reveal_path", {
      path: "C:/runs/report.xlsx",
    });
  });

  it("does not treat ML attachment paths as a saved run report path", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      reportPath: "",
      mlScoreCachePath: "C:/runs/cache.json",
      mlFeatureSchemaPath: "C:/runs/schema.json",
      paramsText: "",
    });

    await expect(openSavedRunReport("C:/runs/run1.json")).rejects.toThrow(
      "No report path for this saved run yet",
    );

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, "load_run_artifact_detail", {
      path: "C:/runs/run1.json",
    });
    expect(invokeCommandMock).not.toHaveBeenCalledWith("open_path", expect.anything());
    expect(invokeCommandMock).not.toHaveBeenCalledWith("reveal_path", expect.anything());
  });
});
