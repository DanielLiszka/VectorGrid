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
  repairSavedRun,
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
import {
  clearArtifactHistoryPerfEntries,
  readArtifactHistoryPerfEntries,
} from "./artifactHistoryPerf";

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
    clearArtifactHistoryPerfEntries();
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
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "load_run_artifact_hydration",
      {
        path: "C:/runs/run1.json",
      },
    );
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
    expect(readArtifactHistoryPerfEntries()).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          name: "historyService.loadSavedRunHydrationMs",
          meta: expect.objectContaining({
            path: "C:/runs/run1.json",
            hasRun: true,
            hasValidation: true,
            outcome: "ok",
          }),
        }),
      ]),
    );
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
    expect(artifacts.currentRun?.mlSignal?.scoreScope).toBe("CalibrationSlice");
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
        min_cscv_oos_rank_frac: 0.63,
        min_cscv_oos_metric: 0.11,
        max_pbo: 0.17,
        walk_forward: {
          window_bars: 30000,
          step_bars: 6000,
        },
        reopt_walk_forward: {
          window_bars: 40000,
          step_bars: 8000,
          top_n_candidates: 5,
        },
        spa: {
          slices: 12,
          bootstrap_samples: 900,
          block_length: 7,
        },
        perturb: {
          iterations: 11,
          pct_min: 0.02,
          pct_max: 0.08,
        },
        trade_bootstrap: {
          simulations: 700,
          stationary_mean_block_len: 0,
        },
        cscv_pbo: {
          slices: 10,
        },
        cross_ticker: {
          max_datasets: 4,
          mode: "manual",
          datasets: [
            {
              csv_path: "C:/data/qqq.csv",
              label: "QQQ",
              market: "stocks",
              timeframe: "1d",
              adjusted: true,
            },
            {
              path: "C:/data/btc.csv",
              label: "BTC",
              market: "crypto",
              timeframe: "4h",
            },
          ],
        },
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
    expect(validation.minCscvRank).toBe("0.63");
    expect(validation.minCscvMetric).toBe("0.11");
    expect(validation.maxPbo).toBe("0.17");
    expect(validation.wfEnable).toBe(true);
    expect(validation.wfWindow).toBe("30000");
    expect(validation.wfStep).toBe("6000");
    expect(validation.reoptWfEnable).toBe(true);
    expect(validation.reoptWfWindow).toBe("40000");
    expect(validation.reoptWfStep).toBe("8000");
    expect(validation.reoptWfTopN).toBe("5");
    expect(validation.spaEnable).toBe(true);
    expect(validation.spaSlices).toBe("12");
    expect(validation.spaBootstrapSamples).toBe("900");
    expect(validation.spaBlockLength).toBe("7");
    expect(validation.perturbEnable).toBe(true);
    expect(validation.perturbIters).toBe("11");
    expect(validation.perturbMin).toBe("0.02");
    expect(validation.perturbMax).toBe("0.08");
    expect(validation.mcEnable).toBe(true);
    expect(validation.mcSims).toBe("700");
    expect(validation.mcBlockLen).toBe("0");
    expect(validation.mcMode).toBe("iid_only");
    expect(validation.cscvEnable).toBe(true);
    expect(validation.cscvSlices).toBe("10");
    expect(validation.crossEnable).toBe(true);
    expect(validation.crossMax).toBe("4");
    expect(validation.crossMode).toBe("manual");
    expect(validation.crossSelectedCount).toBe(2);
    expect(validation.crossTotalCount).toBe(2);
    expect(validation.crossMeta).toBe("selected 2/2");
    expect(validation.crossDatasets).toEqual([
      {
        path: "C:/data/qqq.csv",
        label: "QQQ",
        meta: "stocks • 1d",
        checked: true,
        disabled: false,
        market: "stocks",
        timeframe: "1d",
        adjusted: true,
      },
      {
        path: "C:/data/btc.csv",
        label: "BTC",
        meta: "crypto • 4h",
        checked: true,
        disabled: false,
        market: "crypto",
        timeframe: "4h",
        adjusted: null,
      },
    ]);
  });

  it("blocks apply for future-schema saved runs while keeping detail readable", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      strategyKind: "DoubleMa",
      csvPath: "C:/data/spy.csv",
      artifactCompatibility: {
        artifactSchemaVersion: 2,
        status: "future_unsupported",
        reason: "newer than supported version",
      },
      runReq: { strategy_kind: "DoubleMa" },
      paramsText: "",
      tags: [],
    });

    await expect(applySavedRun(" C:/runs/future.json ")).rejects.toThrow(
      "Apply saved run is unavailable",
    );
    expect(invokeCommandMock).toHaveBeenCalledWith("load_run_artifact_detail", {
      path: "C:/runs/future.json",
    });
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

  it("replays conventional composite config, MA params, and CompositeMo advanced settings", async () => {
    useWorkflowStore(pinia).selectMode("ml-signal", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      strategyKind: "CompositeMo",
      timeframe: "4h",
      csvPath: "C:/data/eth.csv",
      runReq: {
        strategy_kind: "CompositeMo",
        objective: "Sharpe",
        mode: "Grid",
        backend: { backend: "GpuHybridKernel", device_id: 0 },
        ma_source: "hlc3",
        fast_range: { start: 8, end: 24, step: 2 },
        slow_range: { start: 34, end: 120, step: 4 },
        fast_ma_type: "alma",
        slow_ma_type: "ema",
        fast_ma_params: { offset: 0.85, sigma: 6 },
        slow_ma_params: { length: 55 },
        fast_ma_param_sweeps: { sigma: { start: 4, end: 8, step: 1 } },
        slow_ma_param_sweeps: { length: [40, 80, 5] },
        signal_mode: "weighted",
        halftrend: {
          amplitude: 3,
          atr_period: 14,
          channel_deviation: 1.5,
        },
        macz: {
          fast_length: 12,
          slow_length: 26,
          signal_length: 9,
          lengthz: 20,
          length_stdev: 21,
          gamma: 0.3,
          a: 1.1,
          b: 0.8,
          use_lag: true,
          use_volume: false,
        },
        ott: {
          period: 10,
          percent: 1.4,
          ma_type: "ema",
        },
        strategy: {
          long_only: true,
          allow_flip: false,
          trade_on_next_bar: true,
          fee_bps: 0.4,
          spread_bps: 0.6,
          slippage_bps: 0.2,
          cooldown_bars: 3,
          signal_persistence_bars: 4,
          trend_quality_linearreg_period: 150,
          trend_quality_angle_min_deg: 12,
          trend_quality_bbw_period: 20,
          trend_quality_bbw_mult: 2.2,
          trend_quality_bbw_min: 0.04,
          trend_quality_roc_period: 8,
          trend_quality_adx_period: 14,
          trend_quality_adx_min: 18,
          trend_quality_use_roc: true,
          trend_quality_use_adx: true,
          regime_hybrid_adx_period: 16,
          regime_hybrid_adx_enter: 25,
          regime_hybrid_adx_exit: 18,
          regime_hybrid_reversion_entry_rel: 0.9,
          regime_hybrid_reversion_exit_rel: 0.35,
          breakout_bbands_period: 20,
          breakout_bbands_mult: 2.5,
          breakout_confirm_bars: 3,
          breakout_macd_fast_period: 12,
          breakout_macd_slow_period: 26,
          breakout_macd_signal_period: 9,
          breakout_cmo_period: 14,
          breakout_cmo_min_abs: 35,
          breakout_adosc_short_period: 3,
          breakout_adosc_long_period: 10,
          breakout_adx_period: 14,
          breakout_adx_min: 22,
          breakout_use_adx: true,
        },
        search_space: {
          fast_len: { kind: "int", low: 4, high: 48 },
          slow_len: { kind: "int", low: 32, high: 160 },
        },
        sampler: {
          sampler: "Tpe",
          config: {
            seed: 17,
            trials: 120,
            startup_trials: 20,
            candidates: 32,
            gamma: 0.15,
            batch_size: 8,
            max_model_points: 256,
          },
        },
        constraints: {
          min_trades: 40,
          max_drawdown: 0.18,
          max_abs_exposure: 1.4,
        },
        objectives: {
          minimize: ["MaxDrawdown", "Exposure"],
        },
        gpu_prescan: {
          require_gpu: true,
          top_k: 128,
          window: 512,
          predictive_top_k: 16,
          predictive_stride: 4,
          objectives: "Sharpe,Pnl",
          mode: "predictive",
          ma_pairs: [{ fast: "ema", slow: "alma" }],
        },
      },
    });

    await applySavedRun(" C:/runs/composite_mo.json ");

    const optimize = useOptimizeStore(pinia);
    const advanced = useAdvancedStore(pinia);
    const strategy = useStrategyStore(pinia);
    const data = useDataStore(pinia);
    expect(useWorkflowStore(pinia).mode).toBe("conventional");
    expect(useWorkflowStore(pinia).workflowStep).toBe("optimize");
    expect(optimize.strategyKind).toBe("CompositeMo");
    expect(optimize.backend).toBe("GpuKernel");
    expect(optimize.fastMaType).toBe("alma");
    expect(optimize.slowMaType).toBe("ema");
    expect(optimize.fastMaParams).toEqual({ offset: 0.85, sigma: 6 });
    expect(optimize.slowMaParams).toEqual({ length: 55 });
    expect(optimize.fastMaParamSweeps).toEqual({ sigma: [4, 8, 1] });
    expect(optimize.slowMaParamSweeps).toEqual({ length: [40, 80, 5] });
    expect(strategy.compositeSignalMode).toBe("weighted");
    expect(strategy.halfTrendAmplitude).toBe("3");
    expect(strategy.halfTrendAtrPeriod).toBe("14");
    expect(strategy.halfTrendChannelDev).toBe("1.5");
    expect(strategy.maczFastLength).toBe("12");
    expect(strategy.maczUseLag).toBe(true);
    expect(strategy.maczUseVolume).toBe(false);
    expect(strategy.ottPeriod).toBe("10");
    expect(strategy.ottMaType).toBe("ema");
    expect(strategy.trendLrPeriod).toBe("150");
    expect(strategy.trendUseRoc).toBe(true);
    expect(strategy.trendUseAdx).toBe(true);
    expect(strategy.regimeAdxPeriod).toBe("16");
    expect(strategy.breakoutBbPeriod).toBe("20");
    expect(strategy.breakoutUseAdx).toBe(true);
    expect(strategy.longOnly).toBe(true);
    expect(strategy.allowFlip).toBe(false);
    expect(advanced.maSource).toBe("hlc3");
    expect(advanced.moUseRangeBuilder).toBe(false);
    expect(advanced.moSearchSpace).toContain('"fast_len"');
    expect(advanced.moSampler).toBe("Tpe");
    expect(advanced.moConstraintMinTrades).toBe("40");
    expect(advanced.moConstraintMaxDd).toBe("0.18");
    expect(advanced.moConstraintMaxAbsExposure).toBe("1.4");
    expect(advanced.moPrescanEnable).toBe(true);
    expect(advanced.moPrescanRequireGpu).toBe(true);
    expect(advanced.moPrescanTopK).toBe("128");
    expect(advanced.moPrescanObjectives).toBe("Sharpe,Pnl");
    expect(advanced.moPrescanMode).toBe("predictive");
    expect(advanced.moPrescanMaPairs).toContain('"ema"');
    expect(advanced.moTpeSeed).toBe("17");
    expect(advanced.moTpeTrials).toBe("120");
    expect(advanced.moTpeStartupTrials).toBe("20");
    expect(advanced.moTpeCandidates).toBe("32");
    expect(advanced.moTpeGamma).toBe("0.15");
    expect(advanced.moTpeBatchSize).toBe("8");
    expect(advanced.moTpeMaxModelPoints).toBe("256");
    expect(data.csvPath).toBe("C:/data/eth.csv");
    expect(data.dataTimeframe).toBe("4h");
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

  it("applies native MlSignal saved run request into Vue optimize/data stores", async () => {
    useWorkflowStore(pinia).selectMode("conventional", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      strategyKind: "MlSignal",
      timeframe: "1d",
      csvPath: "C:/data/spy.csv",
      runReq: {
        strategy_kind: "MlSignal",
        data_id: "data-ml",
        input_path: "",
        output_path: "C:/runs/ml_score_cache.json",
        model_bundle_id: "bundle-native",
        native_backend_preference: "native_cuda",
        native_library_path: "C:/catboost/catboostmodel.dll",
        native_gpu_device_id: 1,
      },
      mlSignalStage: "validation",
      validationReq: {
        candidates: [
          {
            t_long: 0.61,
            t_short: 0.57,
            delta: 0.09,
            max_hold_bars: 9,
          },
        ],
      },
    });

    await applySavedRun(" C:/runs/ml_signal_native.json ");

    const optimize = useOptimizeStore(pinia);
    expect(useWorkflowStore(pinia).mode).toBe("ml-signal");
    expect(useWorkflowStore(pinia).workflowStep).toBe("validate");
    expect(optimize.mlSignalScoreSource).toBe("native");
    expect(optimize.mlSignalScoreInputPath).toBe("");
    expect(optimize.mlSignalScoreCachePath).toBe("C:/runs/ml_score_cache.json");
    expect(optimize.mlSignalModelBundleId).toBe("bundle-native");
    expect(optimize.mlSignalNativeBackend).toBe("native_cuda");
    expect(optimize.mlSignalNativeLibraryPath).toBe(
      "C:/catboost/catboostmodel.dll",
    );
    expect(optimize.mlSignalNativeGpuDeviceId).toBe("1");
  });

  it("defaults replayed native MlSignal runs to native_cpu when backend is omitted", async () => {
    useWorkflowStore(pinia).selectMode("conventional", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      strategyKind: "MlSignal",
      timeframe: "1d",
      csvPath: "C:/data/spy.csv",
      runReq: {
        strategy_kind: "MlSignal",
        data_id: "data-ml",
        input_path: "",
        output_path: "C:/runs/ml_score_cache.json",
        model_bundle_id: "bundle-native",
      },
      mlSignalStage: "validation",
      validationReq: {
        candidates: [
          {
            t_long: 0.61,
            t_short: 0.57,
            delta: 0.09,
            max_hold_bars: 9,
          },
        ],
      },
    });

    await applySavedRun(" C:/runs/ml_signal_native_cpu_default.json ");

    const optimize = useOptimizeStore(pinia);
    expect(optimize.mlSignalScoreSource).toBe("native");
    expect(optimize.mlSignalNativeBackend).toBe("native_cpu");
    expect(optimize.mlSignalNativeGpuDeviceId).toBe("");
  });

  it("does not infer native MlSignal replay mode from backend preference alone", async () => {
    useWorkflowStore(pinia).selectMode("conventional", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      strategyKind: "MlSignal",
      timeframe: "1d",
      csvPath: "C:/data/spy.csv",
      runReq: {
        strategy_kind: "MlSignal",
        data_id: "data-ml",
        input_path: "",
        output_path: "C:/runs/ml_score_cache.json",
        native_backend_preference: "native_cuda",
        native_library_path: "C:/catboost/catboostmodel.dll",
        native_gpu_device_id: 3,
      },
      mlSignalStage: "validation",
      validationReq: {
        candidates: [
          {
            t_long: 0.61,
            t_short: 0.57,
            delta: 0.09,
            max_hold_bars: 9,
          },
        ],
      },
    });

    await applySavedRun(" C:/runs/ml_signal_invalid_native.json ");

    const optimize = useOptimizeStore(pinia);
    expect(optimize.mlSignalScoreSource).toBe("csv");
    expect(optimize.mlSignalModelBundleId).toBe("");
    expect(optimize.mlSignalNativeBackend).toBe("native_cpu");
    expect(optimize.mlSignalNativeLibraryPath).toBe("");
    expect(optimize.mlSignalNativeGpuDeviceId).toBe("");
  });

  it("treats saved dataset-export artifacts as MlSignal even when strategy_kind is omitted", async () => {
    useWorkflowStore(pinia).selectMode("conventional", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      timeframe: "1d",
      csvPath: "C:/data/spy.csv",
      mlSignalStage: "dataset_export",
      runReq: {
        data_id: "data-ml",
        output_path: "C:/runs/ml_dataset.csv",
        ma_source: "hlc3",
        horizon_bars: 12,
      },
    });

    await applySavedRun(" C:/runs/ml_export.json ");

    const optimize = useOptimizeStore(pinia);
    expect(useWorkflowStore(pinia).mode).toBe("ml-signal");
    expect(useWorkflowStore(pinia).workflowStep).toBe("data");
    expect(optimize.mlSignalDatasetExportPath).toBe("C:/runs/ml_dataset.csv");
    expect(optimize.mlSignalMaSource).toBe("hlc3");
    expect(optimize.mlSignalHorizonBars).toBe("12");
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

  it("reruns native MlSignal saved validation artifact through score-cache build then validation", async () => {
    useWorkflowStore(pinia).selectMode("conventional", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      timeframe: "1d",
      csvPath: "C:/data/spy.csv",
      runReq: {
        strategy_kind: "MlSignal",
        data_id: "stale-id",
        input_path: "",
        output_path: "C:/runs/ml_score_cache.json",
        model_bundle_id: "bundle-native",
        native_backend_preference: "native_cuda",
        native_library_path: "C:/catboost/catboostmodel.dll",
        native_gpu_device_id: 2,
      },
      mlSignalStage: "validation",
      validationReq: {
        candidates: [
          {
            t_long: 0.62,
            t_short: 0.56,
            delta: 0.08,
            max_hold_bars: 10,
          },
        ],
      },
    });

    await rerunSavedRun(" C:/runs/ml_signal_native.json ");

    expect(loadDataCsvMock).toHaveBeenCalledWith("C:/data/spy.csv");
    expect(useWorkflowStore(pinia).mode).toBe("ml-signal");
    expect(useWorkflowStore(pinia).workflowStep).toBe("validate");
    expect(generateMlSignalScoreCacheMock).toHaveBeenCalledTimes(1);
    expect(runMlSignalValidationMock).toHaveBeenCalledTimes(1);
    const optimize = useOptimizeStore(pinia);
    expect(optimize.mlSignalScoreSource).toBe("native");
    expect(optimize.mlSignalScoreInputPath).toBe("");
    expect(optimize.mlSignalScoreCachePath).toBe("C:/runs/ml_score_cache.json");
    expect(optimize.mlSignalModelBundleId).toBe("bundle-native");
    expect(optimize.mlSignalNativeBackend).toBe("native_cuda");
    expect(optimize.mlSignalNativeLibraryPath).toBe(
      "C:/catboost/catboostmodel.dll",
    );
    expect(optimize.mlSignalNativeGpuDeviceId).toBe("2");
  });

  it("reseeds MlSignal artifact provenance before rerunning score-cache generation", async () => {
    const artifacts = useWorkflowArtifactsStore(pinia);
    artifacts.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-24T00:00:00Z",
        artifact_path: "C:/runs/old_ml_signal.json",
        req: { strategy_kind: "MlSignal" },
        res: {},
        ml_signal: {
          stage: "validation",
          bundle_id: "bundle-old",
          score_cache_path: "C:/runs/old_score_cache.json",
          export_manifest_path: "C:/runs/old_export_manifest.json",
          source_bundle_manifest_path: "C:/models/old_bundle/manifest.json",
          feature_schema_hash: "old-schema-hash",
        },
      },
      currentValidation: null,
    });
    useWorkflowStore(pinia).selectMode("conventional", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      ts: "2026-02-25T00:00:00Z",
      artifactPath: "C:/runs/ml_signal.json",
      strategyKind: "MlSignal",
      timeframe: "1d",
      csvPath: "C:/data/spy.csv",
      runReq: {
        strategy_kind: "MlSignal",
        data_id: "stale-id",
        input_path: "",
        output_path: "C:/runs/new_score_cache.json",
        model_bundle_id: "bundle-new",
      },
      mlSignalStage: "validation",
      mlExportManifestPath: "C:/runs/new_export_manifest.json",
      mlFeatureSchemaHash: "new-schema-hash",
      mlSourceBundleManifestPath: "C:/models/new_bundle/manifest.json",
      mlSourceBundleManifestHash: "bundle-hash-new",
      mlCalibrationState: "calibrated",
      mlThresholdPolicy: "long_short_v2",
      mlProducerKind: "native_catboost",
      validationReq: {
        candidates: [
          {
            t_long: 0.62,
            t_short: 0.56,
            delta: 0.08,
            max_hold_bars: 10,
          },
        ],
      },
      runRes: {},
    });

    await rerunSavedRun(" C:/runs/ml_signal.json ");

    expect(generateMlSignalScoreCacheMock).toHaveBeenCalledTimes(1);
    expect(artifacts.currentRun?.mlSignal).toMatchObject({
      bundleId: "bundle-new",
      scoreCachePath: "C:/runs/new_score_cache.json",
      exportManifestPath: "C:/runs/new_export_manifest.json",
      sourceBundleManifestPath: "C:/models/new_bundle/manifest.json",
      featureSchemaHash: "new-schema-hash",
      scoreCacheProvenance: expect.objectContaining({
        source_bundle_manifest_hash: "bundle-hash-new",
        threshold_policy: "long_short_v2",
        producer_kind: "native_catboost",
      }),
      calibration: expect.objectContaining({
        calibration_state: "calibrated",
      }),
    });
  });

  it("reruns MlSignal dataset-export artifact through dataset export only", async () => {
    useWorkflowStore(pinia).selectMode("conventional", { persist: false });
    invokeCommandMock.mockResolvedValueOnce({
      timeframe: "1d",
      csvPath: "C:/data/spy.csv",
      mlSignalStage: "dataset_export",
      runReq: {
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
    const artifacts = useWorkflowArtifactsStore(pinia);
    artifacts.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-02-25T00:00:00Z",
        artifact_path: "C:/runs/run1.json",
        req: { strategy_kind: "DoubleMa" },
        res: { top: [] },
      },
      currentValidation: {
        ts: "2026-02-25T00:00:00Z",
        req: { candidates: [] },
        res: { rows: [] },
      },
    });

    await deleteSavedRun(" C:/runs/run1.json ");

    expect(invokeCommandMock).toHaveBeenCalledWith("delete_run_artifact", {
      path: "C:/runs/run1.json",
    });
    expect(artifacts.currentRun).toBeNull();
    expect(artifacts.currentValidation).toBeNull();
  });

  it("checks and writes repaired saved-run artifacts through the backend repair command", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      source_path: "C:/runs/run1.json",
      result_path: "C:/runs/run1.json",
      dry_run: true,
      changed: true,
      wrote: false,
      changes: ["normalized route provenance"],
    });

    const dryRun = await repairSavedRun(" C:/runs/run1.json ");

    expect(invokeCommandMock).toHaveBeenCalledWith("repair_run_artifact", {
      path: "C:/runs/run1.json",
      dry_run: true,
    });
    expect(dryRun.changed).toBe(true);
    expect(dryRun.changes).toEqual(["normalized route provenance"]);

    invokeCommandMock.mockResolvedValueOnce({
      source_path: "C:/runs/run1.json",
      result_path: "C:/runs/run1.repaired.json",
      dry_run: false,
      changed: true,
      wrote: true,
      changes: ["normalized route provenance"],
      summary: {
        ts: "2026-02-25T00:00:00Z",
        path: "C:/runs/run1.repaired.json",
        bytes: 4096,
        strategy_kind: "DoubleMa",
        has_validation: false,
        tags: [],
      },
    });

    const wrote = await repairSavedRun(" C:/runs/run1.json ", {
      dryRun: false,
    });

    expect(invokeCommandMock).toHaveBeenLastCalledWith("repair_run_artifact", {
      path: "C:/runs/run1.json",
      dry_run: false,
      write_target: "C:/runs/run1.repaired.json",
    });
    expect(wrote.wrote).toBe(true);
    expect(wrote.summary?.path).toBe("C:/runs/run1.repaired.json");
  });

  it("refreshes saved runs through direct command and normalizes rows", async () => {
    invokeCommandMock.mockResolvedValueOnce([
      {
        ts: "2026-02-25T00:00:00Z",
        path: " C:/runs/run1.json ",
        bytes: 2048,
        kind: "vectorgrid_run_artifact",
        artifact_compatibility: {
          artifactSchemaVersion: 1,
          status: "supported",
          replayBlocked: false,
        },
        strategy_kind: "MlSignal",
        has_validation: true,
        replay_class: "exact_evidence_review",
        replay_reasons: [
          "saved_run_result_payload_present",
          "compatible_recompute_inputs_available",
        ],
        report_xlsx_path: "C:/runs/run1.xlsx",
        report_xlsx_exists: false,
        csv_path: "C:/data/aapl.csv",
        ml_score_space: "Probability",
        ml_score_scope: "CalibrationSlice",
        tags: ["aapl", " 1m "],
      },
      { path: "" },
    ]);

    const summary = await refreshSavedRuns();

    expect(invokeCommandMock).toHaveBeenCalledWith("list_run_artifacts");
    expect(summary.runs).toHaveLength(1);
    expect(summary.runs[0]).toMatchObject({
      ts: "2026-02-25T00:00:00Z",
      path: "C:/runs/run1.json",
      bytes: 2048,
      artifactKind: "vectorgrid_run_artifact",
      artifactCompatibility: expect.objectContaining({
        artifactSchemaVersion: 1,
        status: "supported",
        replayBlocked: false,
      }),
      strategyKind: "MlSignal",
      hasValidation: true,
      replayClass: "exact_evidence_review",
      replayReasons: [
        "saved_run_result_payload_present",
        "compatible_recompute_inputs_available",
      ],
      reportXlsxPath: "C:/runs/run1.xlsx",
      reportXlsxExists: false,
      csvPath: "C:/data/aapl.csv",
      mlScoreSpace: "Probability",
      mlScoreScope: "CalibrationSlice",
      mlScoreContractSummary: "Probability • CalibrationSlice",
      mlScoreNote:
        "Calibration-only scores; saved validation metrics reflect calibration rows, not holdout-like evidence.",
      tags: ["aapl", "1m"],
    });
    expect(readArtifactHistoryPerfEntries()).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          name: "historyService.refreshSavedRunsMs",
          meta: expect.objectContaining({
            runCount: 1,
            validationCount: 1,
            outcome: "ok",
          }),
        }),
      ]),
    );
  });

  it("rejects malformed saved-run list payloads instead of reporting empty history", async () => {
    invokeCommandMock.mockResolvedValueOnce({ runs: [] });

    await expect(refreshSavedRuns()).rejects.toThrow(
      "Saved run artifact list payload was malformed.",
    );
  });

  it("parses string false validation flags as false in saved-run summaries", async () => {
    invokeCommandMock.mockResolvedValueOnce([
      {
        ts: "2026-02-25T00:00:00Z",
        path: "C:/runs/run1.json",
        bytes: 2048,
        strategy_kind: "DoubleMa",
        has_validation: "false",
        tags: [],
      },
    ]);

    const summary = await refreshSavedRuns();

    expect(summary.runs[0]?.hasValidation).toBe(false);
    expect(readArtifactHistoryPerfEntries()).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          name: "historyService.refreshSavedRunsMs",
          meta: expect.objectContaining({
            validationCount: 0,
            outcome: "ok",
          }),
        }),
      ]),
    );
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
      kind: "vectorgrid_run_artifact",
      artifactCompatibility: {
        artifactSchemaVersion: 1,
        status: "supported",
        replayBlocked: false,
      },
      strategyKind: "CompositeMo",
      timeframe: "5m",
      csvPath: "C:/data/msft.csv",
      reportPath: "C:/runs/report.xlsx",
      hasValidation: true,
      replayClass: "exact_evidence_review",
      replayReasons: ["saved_run_result_payload_present", "report_available"],
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
          heads: [{ objective: "Sharpe" }, { objective: "MaxDrawdown" }],
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
    expect(detail).toMatchObject({
      strategyKind: "CompositeMo",
      kind: "vectorgrid_run_artifact",
      artifactCompatibility: expect.objectContaining({
        artifactSchemaVersion: 1,
        status: "supported",
        replayBlocked: false,
      }),
      timeframe: "5m",
      csvPath: "C:/data/msft.csv",
      reportPath: "C:/runs/report.xlsx",
      hasValidation: true,
      replayClass: "exact_evidence_review",
      replayReasons: ["saved_run_result_payload_present", "report_available"],
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
    expect(readArtifactHistoryPerfEntries()).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          name: "historyService.loadSavedRunDetailMs",
          meta: expect.objectContaining({
            path: "C:/runs/run1.json",
            hasValidation: true,
            strategyKind: "CompositeMo",
            outcome: "ok",
          }),
        }),
      ]),
    );
  });

  it("rejects malformed saved-run detail payloads", async () => {
    invokeCommandMock.mockResolvedValueOnce("not an object");

    await expect(loadSavedRunDetail("C:/runs/run1.json")).rejects.toThrow(
      "Saved run detail payload was malformed.",
    );
  });

  it("rejects malformed saved-run hydration before mutating workflow artifacts", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      currentRun: {
        ts: "2026-02-25T00:00:00Z",
        req: { strategy_kind: "DoubleMa" },
      },
      currentValidation: null,
    });

    await expect(loadSavedRun("C:/runs/run1.json")).rejects.toThrow(
      "Saved run hydration currentRun is missing res.",
    );

    const artifacts = useWorkflowArtifactsStore(pinia);
    expect(artifacts.currentRun).toBeNull();
    expect(refreshAnalysisFromWorkflowRunMock).not.toHaveBeenCalled();
  });

  it("loads MlSignal saved-run detail with policy summary", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      ts: "2026-02-25T12:00:00Z",
      artifactPath: "C:/runs/ml_signal.json",
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
      mlFeatureSchemaHash: "schemahash0123456789abcdef",
      mlFeatureSchemaVersion: "1",
      mlExportManifestPath: "C:/runs/ml_export.manifest.json",
      mlSourceBundleManifestPath: "C:/models/bundle-a/manifest.json",
      mlSourceBundleManifestHash: "bundlehash0123456789abcdef",
      mlCalibrationState: "calibrated",
      mlThresholdPolicy: "long_short_v2",
      mlProducerKind: "xgboost",
      mlScoreContractVersion: 3,
      mlScoreSpace: "Probability",
      mlScoreScope: "CalibrationSlice",
      mlScoreContractSummary: "Probability • CalibrationSlice",
      mlScoreNote:
        "Calibration-only scores; saved validation metrics reflect calibration rows, not holdout-like evidence.",
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

    expect(detail).toMatchObject({
      ts: "2026-02-25T12:00:00Z",
      artifactPath: "C:/runs/ml_signal.json",
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
      mlFeatureSchemaHash: "schemahash0123456789abcdef",
      mlFeatureSchemaVersion: "1",
      mlExportManifestPath: "C:/runs/ml_export.manifest.json",
      mlSourceBundleManifestPath: "C:/models/bundle-a/manifest.json",
      mlSourceBundleManifestHash: "bundlehash0123456789abcdef",
      mlCalibrationState: "calibrated",
      mlThresholdPolicy: "long_short_v2",
      mlProducerKind: "xgboost",
      mlScoreContractVersion: 3,
      mlScoreSpace: "Probability",
      mlScoreScope: "CalibrationSlice",
      mlScoreContractSummary: "Probability • CalibrationSlice",
      mlScoreNote:
        "Calibration-only scores; saved validation metrics reflect calibration rows, not holdout-like evidence.",
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

  it("loads native MlSignal saved-run detail with backend note", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      strategyKind: "MlSignal",
      timeframe: "1d",
      csvPath: "C:/data/spy.csv",
      reportPath: "",
      hasValidation: true,
      tags: ["ml", "native"],
      mlBundleId: "bundle-native",
      mlNativeBackendRequested: "native_cuda",
      mlNativeBackendUsed: "native_cpu",
      mlNativeLibraryPath: "C:/catboost/catboostmodel.dll",
      mlNativeGpuDeviceId: 0,
      mlNativeFallbackReason: "EnableGPUEvaluation unavailable",
      mlScoreCachePath: "C:/runs/ml_score_cache.json",
      mlScoreManifestPath: "C:/runs/ml_score_cache.manifest.json",
      mlFeatureSchemaPath: "C:/runs/ml_dataset.feature_schema.json",
      mlFeatureSchemaVersion: "1",
      mlScoreSpace: "Probability",
      mlScoreScope: "CalibrationSlice",
      mlScoreContractSummary: "Probability • CalibrationSlice",
      mlScoreNote:
        "Calibration-only scores; saved validation metrics reflect calibration rows, not holdout-like evidence.",
      mlPolicySummary: "t_long=0.61 | t_short=0.57 | delta=0.09 | hold=9",
      mlPolicySweepSummary: "",
      mlRunVerdictSummary: "",
      mlTopCandidatesSummary: "",
      paramsText: [
        "strategy: MlSignal",
        "data id: data-ml",
        "dataset/score output: C:/runs/ml_score_cache.json",
        "model bundle: bundle-native",
      ].join("\n"),
    });

    const detail = await loadSavedRunDetail(" C:/runs/ml_signal_native.json ");

    expect(detail).toMatchObject({
      strategyKind: "MlSignal",
      mlBundleId: "bundle-native",
      mlNativeBackendRequested: "native_cuda",
      mlNativeBackendUsed: "native_cpu",
      mlNativeLibraryPath: "C:/catboost/catboostmodel.dll",
      mlNativeGpuDeviceId: 0,
      mlNativeFallbackReason: "EnableGPUEvaluation unavailable",
      mlScoreNote:
        "Calibration-only scores; saved validation metrics reflect calibration rows, not holdout-like evidence. • native backend native_cpu • fallback EnableGPUEvaluation unavailable",
    });
  });

  it("opens saved reports through the backend artifact-report command", async () => {
    invokeCommandMock.mockResolvedValueOnce(undefined);

    await openSavedRunReport(" C:/runs/run1.json ");

    expect(invokeCommandMock).toHaveBeenCalledWith("open_run_artifact_report", {
      path: "C:/runs/run1.json",
    });
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "open_path",
      expect.anything(),
    );
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "reveal_path",
      expect.anything(),
    );
  });

  it("propagates saved report open failures from the backend", async () => {
    invokeCommandMock.mockRejectedValueOnce(
      new Error("Saved run report file no longer exists on disk."),
    );

    await expect(openSavedRunReport("C:/runs/run1.json")).rejects.toThrow(
      "Saved run report file no longer exists on disk.",
    );

    expect(invokeCommandMock).toHaveBeenCalledWith("open_run_artifact_report", {
      path: "C:/runs/run1.json",
    });
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "open_path",
      expect.anything(),
    );
  });
});
