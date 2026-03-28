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
import { useValidationStore } from "../stores/validationStore";
import {
  applyTimeframe as applyDataTimeframe,
  loadCsv as loadDataCsv,
} from "./dataService";
import { runOptimization } from "./optimizeService";
import { refreshAnalysisFromWorkflowRun } from "./analysisService";
import { generateMlSignalScoreCache } from "./mlSignalService";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => undefined),
}));

vi.mock("./dataService", () => ({
  loadCsv: vi.fn(async () => ({
    path: "C:/data/aapl.csv",
    dataId: "data-1",
    cacheHit: false,
    cachePath: "",
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
  generateMlSignalScoreCache: vi.fn(async () => undefined),
}));

const invokeCommandMock = vi.mocked(invokeCommand);
const loadDataCsvMock = vi.mocked(loadDataCsv);
const applyDataTimeframeMock = vi.mocked(applyDataTimeframe);
const runOptimizationMock = vi.mocked(runOptimization);
const refreshAnalysisFromWorkflowRunMock = vi.mocked(
  refreshAnalysisFromWorkflowRun,
);
const generateMlSignalScoreCacheMock = vi.mocked(generateMlSignalScoreCache);

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
    invokeCommandMock.mockResolvedValueOnce({
      ts: "2026-02-25T00:00:00Z",
      report_xlsx_path: "C:/runs/run1.xlsx",
      tags: ["holdout", " 1m "],
      data: { csv_path: "C:/data/aapl.csv" },
      run: {
        req: { strategy_kind: "DoubleMa", data_id: "data-1" },
        res: { top: [[{ fast_len: 5, slow_len: 20 }, { sharpe: 1.2 }]] },
      },
      validation: {
        req: { strategy_kind: "DoubleMa" },
        res: { candidates: [{ score: 1.0 }] },
      },
    });

    await loadSavedRun(" C:/runs/run1.json ");

    const artifacts = useWorkflowArtifactsStore(pinia);
    expect(invokeCommandMock).toHaveBeenCalledWith("load_run_artifact", {
      path: "C:/runs/run1.json",
    });
    expect(artifacts.currentRun?.artifactPath).toBe("C:/runs/run1.json");
    expect(artifacts.currentRun?.reportXlsxPath).toBe("C:/runs/run1.xlsx");
    expect(artifacts.currentRun?.csvPath).toBe("C:/data/aapl.csv");
    expect(artifacts.currentRun?.tags).toEqual(["holdout", "1m"]);
    expect(artifacts.currentValidation?.ts).toBe("2026-02-25T00:00:00Z");
    expect(refreshAnalysisFromWorkflowRunMock).toHaveBeenCalled();
  });

  it("loads MlSignal saved run artifact without routing into optimizer analysis refresh", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      ts: "2026-03-20T00:00:00Z",
      data: { csv_path: "C:/data/spy.csv" },
      run: {
        req: {
          strategy_kind: "MlSignal",
          data_id: "data-ml",
          input_path: "C:/scores/ml_scores.csv",
          output_path: "C:/runs/ml_score_cache.json",
          model_bundle_id: "bundle-a",
        },
        res: { matched_rows: 123, output_path: "C:/runs/ml_score_cache.json" },
      },
      validation: {
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
    expect(artifacts.currentRun?.artifactPath).toBe("C:/runs/ml_signal.json");
    expect(
      (artifacts.currentRun?.req as { strategy_kind?: string } | null)
        ?.strategy_kind,
    ).toBe("MlSignal");
    expect(refreshAnalysisFromWorkflowRunMock).not.toHaveBeenCalled();
  });

  it("applies saved run request into Vue optimize/advanced/data stores", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      data: { csv_path: "C:/data/msft.csv" },
      dataset_meta: { timeframe: "5m" },
      run: {
        req: {
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
      },
    });

    await applySavedRun(" C:/runs/run1.json ");

    const optimize = useOptimizeStore(pinia);
    const advanced = useAdvancedStore(pinia);
    const data = useDataStore(pinia);
    expect(invokeCommandMock).toHaveBeenCalledWith("load_run_artifact", {
      path: "C:/runs/run1.json",
    });
    expect(optimize.strategyKind).toBe("DoubleMa");
    expect(optimize.backend).toBe("GpuKernel");
    expect(optimize.fastStart).toBe("5");
    expect(optimize.slowEnd).toBe("80");
    expect(advanced.maSource).toBe("close");
    expect(data.csvPath).toBe("C:/data/msft.csv");
    expect(data.dataTimeframe).toBe("5m");
  });

  it("applies MlSignal saved run request into Vue optimize/data stores", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      data: { csv_path: "C:/data/spy.csv" },
      dataset_meta: { timeframe: "1d" },
      run: {
        req: {
          strategy_kind: "MlSignal",
          data_id: "data-ml",
          input_path: "C:/scores/ml_scores.csv",
          output_path: "C:/runs/ml_score_cache.json",
          model_bundle_id: "bundle-a",
        },
      },
    });

    await applySavedRun(" C:/runs/ml_signal.json ");

    const optimize = useOptimizeStore(pinia);
    const data = useDataStore(pinia);
    expect(optimize.strategyKind).toBe("MlSignal");
    expect(optimize.mlSignalScoreInputPath).toBe("C:/scores/ml_scores.csv");
    expect(optimize.mlSignalScoreCachePath).toBe("C:/runs/ml_score_cache.json");
    expect(optimize.mlSignalModelBundleId).toBe("bundle-a");
    expect(data.csvPath).toBe("C:/data/spy.csv");
    expect(data.dataTimeframe).toBe("1d");
  });

  it("reruns saved run by loading data, optional resample, then executing optimization", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      data: { csv_path: "C:/data/aapl.csv" },
      dataset_meta: { timeframe: "5m" },
      run: {
        req: {
          strategy_kind: "DoubleMa",
          data_id: "stale-id",
          backend: { backend: "CpuOnly" },
          fast_range: { start: 5, end: 10, step: 1 },
          slow_range: { start: 20, end: 50, step: 1 },
        },
      },
    });

    await rerunSavedRun(" C:/runs/run1.json ");

    expect(invokeCommandMock).toHaveBeenCalledWith("load_run_artifact", {
      path: "C:/runs/run1.json",
    });
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

  it("reruns MlSignal saved run through the dedicated ML service", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      data: { csv_path: "C:/data/spy.csv" },
      dataset_meta: { timeframe: "1d" },
      run: {
        req: {
          strategy_kind: "MlSignal",
          data_id: "stale-id",
          input_path: "C:/scores/ml_scores.csv",
          output_path: "C:/runs/ml_score_cache.json",
          model_bundle_id: "bundle-a",
        },
      },
    });

    await rerunSavedRun(" C:/runs/ml_signal.json ");

    expect(loadDataCsvMock).toHaveBeenCalledWith("C:/data/spy.csv");
    expect(runOptimizationMock).not.toHaveBeenCalled();
    expect(generateMlSignalScoreCacheMock).toHaveBeenCalledTimes(1);
    const optimize = useOptimizeStore(pinia);
    expect(optimize.mlSignalScoreInputPath).toBe("C:/scores/ml_scores.csv");
    expect(optimize.mlSignalScoreCachePath).toBe("C:/runs/ml_score_cache.json");
    expect(optimize.mlSignalModelBundleId).toBe("bundle-a");
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
      strategy_kind: "CompositeMo",
      report_xlsx_path: "C:/runs/report.xlsx",
      tags: ["tag1", " tag2 "],
      dataset_meta: { timeframe: "5m" },
      data: { csv_path: "C:/data/msft.csv" },
      run: {
        req: {
          strategy_kind: "CompositeMo",
          backend: { backend: "CpuOnly" },
          objective: "Sharpe",
          ma_source: "close",
          fast_range: { start: 5, end: 20, step: 1 },
          slow_range: { start: 21, end: 100, step: 1 },
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          starting_equity: 10000,
        },
      },
      validation: { res: { candidates: [] } },
    });

    const detail = await loadSavedRunDetail(" C:/runs/run1.json ");

    expect(invokeCommandMock).toHaveBeenCalledWith("load_run_artifact", {
      path: "C:/runs/run1.json",
    });
    expect(detail).toEqual({
      strategyKind: "CompositeMo",
      timeframe: "5m",
      csvPath: "C:/data/msft.csv",
      reportPath: "C:/runs/report.xlsx",
      hasValidation: true,
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
      strategy_kind: "MlSignal",
      tags: ["ml", " research "],
      dataset_meta: { timeframe: "1d" },
      data: { csv_path: "C:/data/spy.csv" },
      ml_signal: {
        bundle_id: "bundle-a",
        score_cache_path: "C:/runs/ml_score_cache.json",
        score_manifest_path: "C:/runs/ml_score_cache.manifest.json",
        feature_schema_path: "C:/runs/ml_dataset.feature_schema.json",
        feature_schema_version: 1,
        policy: {
          t_long: 0.6,
          t_short: 0.55,
          delta: 0.1,
          max_hold_bars: 10,
        },
        policy_sweep: {
          sweep_size: 27,
          top_k: 4,
          best_label: "best_sweep",
          best_robust_score: 1.44,
        },
      },
      run: {
        req: {
          strategy_kind: "MlSignal",
          data_id: "data-ml",
          input_path: "C:/scores/ml_scores.csv",
          output_path: "C:/runs/ml_score_cache.json",
          model_bundle_id: "bundle-a",
        },
      },
      validation: {
        req: {
          candidates: [
            {
              label: "best_sweep",
              t_long: 0.6,
              t_short: 0.55,
              delta: 0.1,
              max_hold_bars: 10,
            },
          ],
        },
        res: {
          run_verdict: {
            approved_candidate_count: 1,
            candidates_survived_hard_gates: 1,
            candidates_considered: 27,
          },
          candidates: [
            {
              robust_score: 1.0,
              psr: 0.7,
              dsr: 0.6,
              params: { label: "best_sweep" },
            },
          ],
        },
      },
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
      report_xlsx_path: "C:/runs/report.xlsx",
      run: { req: {} },
    });
    invokeCommandMock.mockResolvedValueOnce(undefined);

    await openSavedRunReport(" C:/runs/run1.json ");

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, "load_run_artifact", {
      path: "C:/runs/run1.json",
    });
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, "open_path", {
      path: "C:/runs/report.xlsx",
    });
  });
});
