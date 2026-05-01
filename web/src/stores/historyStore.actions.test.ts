import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import {
  applySavedRun,
  applySession,
  clearSession,
  deleteSavedRun,
  exportPreset,
  importPreset,
  loadSavedRun,
  loadSavedRunDetail,
  openRunsDir,
  openSavedRunReport,
  readSessionHistory,
  refreshSavedRuns,
  rerunSavedRun,
  revealSavedRun,
  saveSavedRunTags,
} from "../services/historyService";
import { useHistoryStore } from "./historyStore";

vi.mock("../services/historyService", () => ({
  applySavedRun: vi.fn(async () => undefined),
  applySession: vi.fn(async () => undefined),
  clearSession: vi.fn(async () => undefined),
  deleteSavedRun: vi.fn(async () => undefined),
  exportPreset: vi.fn(async () => undefined),
  importPreset: vi.fn(async () => undefined),
  loadSavedRun: vi.fn(async () => undefined),
  loadSavedRunDetail: vi.fn(async () => ({
    strategyKind: "DoubleMA",
    timeframe: "1m",
    csvPath: "C:/data/aapl.csv",
    reportPath: "C:/runs/aapl.xlsx",
    hasValidation: true,
    tags: ["aapl"],
    paramsText: "strategy: DoubleMA",
  })),
  openRunsDir: vi.fn(async () => undefined),
  openSavedRunReport: vi.fn(async () => undefined),
  readSessionHistory: vi.fn(() => []),
  refreshSavedRuns: vi.fn(async () => ({ runs: [] })),
  rerunSavedRun: vi.fn(async () => undefined),
  revealSavedRun: vi.fn(async () => undefined),
  saveSavedRunTags: vi.fn(async () => undefined),
}));

const applySessionMock = vi.mocked(applySession);
const exportPresetMock = vi.mocked(exportPreset);
const importPresetMock = vi.mocked(importPreset);
const clearSessionMock = vi.mocked(clearSession);
const saveSavedRunTagsMock = vi.mocked(saveSavedRunTags);
const refreshSavedRunsMock = vi.mocked(refreshSavedRuns);
const loadSavedRunMock = vi.mocked(loadSavedRun);
const loadSavedRunDetailMock = vi.mocked(loadSavedRunDetail);
const applySavedRunMock = vi.mocked(applySavedRun);
const rerunSavedRunMock = vi.mocked(rerunSavedRun);
const openSavedRunReportMock = vi.mocked(openSavedRunReport);
const revealSavedRunMock = vi.mocked(revealSavedRun);
const deleteSavedRunMock = vi.mocked(deleteSavedRun);
const openRunsDirMock = vi.mocked(openRunsDir);
const readSessionHistoryMock = vi.mocked(readSessionHistory);

describe("history store actions", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    applySessionMock.mockClear();
    applySessionMock.mockResolvedValue(undefined);
    exportPresetMock.mockClear();
    exportPresetMock.mockResolvedValue(undefined);
    importPresetMock.mockClear();
    importPresetMock.mockResolvedValue(undefined);
    clearSessionMock.mockClear();
    clearSessionMock.mockResolvedValue(undefined);
    saveSavedRunTagsMock.mockClear();
    saveSavedRunTagsMock.mockResolvedValue(undefined);
    refreshSavedRunsMock.mockClear();
    refreshSavedRunsMock.mockResolvedValue({ runs: [] });
    loadSavedRunMock.mockClear();
    loadSavedRunMock.mockResolvedValue(undefined);
    loadSavedRunDetailMock.mockClear();
    loadSavedRunDetailMock.mockResolvedValue({
      strategyKind: "DoubleMA",
      timeframe: "1m",
      csvPath: "C:/data/aapl.csv",
      reportPath: "C:/runs/aapl.xlsx",
      hasValidation: true,
      tags: ["aapl"],
      paramsText: "strategy: DoubleMA",
    });
    applySavedRunMock.mockClear();
    applySavedRunMock.mockResolvedValue(undefined);
    rerunSavedRunMock.mockClear();
    rerunSavedRunMock.mockResolvedValue(undefined);
    openSavedRunReportMock.mockClear();
    openSavedRunReportMock.mockResolvedValue(undefined);
    revealSavedRunMock.mockClear();
    revealSavedRunMock.mockResolvedValue(undefined);
    deleteSavedRunMock.mockClear();
    deleteSavedRunMock.mockResolvedValue(undefined);
    openRunsDirMock.mockClear();
    openRunsDirMock.mockResolvedValue(undefined);
    readSessionHistoryMock.mockClear();
    readSessionHistoryMock.mockReturnValue([]);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  async function flushServiceImports(): Promise<void> {
    await vi.dynamicImportSettled();
    await Promise.resolve();
    await Promise.resolve();
  }

  it("updates Vue history state without legacy DOM mirroring", () => {
    const store = useHistoryStore();
    store.setInputField("savedRunsSearch", "aapl");
    store.setSelectField("historySelect", "run-1");

    expect(store.savedRunsSearch).toBe("aapl");
    expect(store.sessionSelectedValue).toBe("run-1");
    expect(store.source).toBe("vue");
  });

  it("dispatches history session/actions and runs-directory action through history service", async () => {
    const store = useHistoryStore();
    readSessionHistoryMock.mockReturnValue([
      { ts: "2026-02-25T00:00:00Z", backend: "CpuOnly", pairs: 123 },
    ]);
    store.refreshSessionHistory("2026-02-25T00:00:00Z");
    await flushServiceImports();
    store.applySession();
    store.exportPreset();
    store.importPreset();
    store.clearSession();
    store.refreshSavedRuns();
    store.openRunsDir();
    await flushServiceImports();
    await flushServiceImports();

    expect(applySessionMock).toHaveBeenCalledWith("2026-02-25T00:00:00Z");
    expect(exportPresetMock).toHaveBeenCalled();
    expect(importPresetMock).toHaveBeenCalled();
    expect(clearSessionMock).toHaveBeenCalled();
    expect(refreshSavedRunsMock).toHaveBeenCalled();
    expect(openRunsDirMock).toHaveBeenCalled();
  });

  it("surfaces runs-directory failures in the Vue status line", async () => {
    const store = useHistoryStore();
    openRunsDirMock.mockRejectedValueOnce(new Error("runs unavailable"));

    store.openRunsDir();
    await flushServiceImports();
    await flushServiceImports();

    expect(openRunsDirMock).toHaveBeenCalledTimes(1);
    expect(store.statusLine).toBe("Error: runs unavailable");
    expect(store.source).toBe("vue");
  });

  it("dispatches saved-run actions through history service", async () => {
    const store = useHistoryStore();
    store.savedRunsRaw = [
      {
        ts: "2026-02-25T00:00:00Z",
        path: "C:/runs/run1.json",
        bytes: 1024,
        strategyKind: "DoubleMA",
        hasValidation: true,
        reportXlsxPath: "C:/runs/run1.xlsx",
        csvPath: "C:/data/aapl.csv",
        tags: ["holdout"],
      },
    ];
    store.syncSavedRunsView("C:/runs/run1.json");
    await flushServiceImports();
    store.saveSavedRunTags(" C:/runs/run1.json ", "holdout,1m");
    store.loadSavedRun(" C:/runs/run1.json ");
    store.applySavedRun(" C:/runs/run1.json ");
    store.rerunSavedRun(" C:/runs/run1.json ");
    store.openSavedRunReport(" C:/runs/run1.json ");
    store.revealSavedRun(" C:/runs/run1.json ");
    store.deleteSavedRun(" C:/runs/run1.json ");
    await flushServiceImports();
    await flushServiceImports();

    expect(saveSavedRunTagsMock).toHaveBeenCalledWith(
      "C:/runs/run1.json",
      "holdout,1m",
    );
    expect(loadSavedRunDetailMock).toHaveBeenCalledWith("C:/runs/run1.json");
    expect(loadSavedRunMock).toHaveBeenCalledWith("C:/runs/run1.json");
    expect(applySavedRunMock).toHaveBeenCalledWith("C:/runs/run1.json");
    expect(rerunSavedRunMock).toHaveBeenCalledWith("C:/runs/run1.json");
    expect(openSavedRunReportMock).toHaveBeenCalledWith("C:/runs/run1.json");
    expect(revealSavedRunMock).toHaveBeenCalledWith("C:/runs/run1.json");
    expect(deleteSavedRunMock).toHaveBeenCalledWith("C:/runs/run1.json");
    expect(refreshSavedRunsMock).toHaveBeenCalledTimes(2);
  });

  it("surfaces saved-run action failures in the Vue status line", async () => {
    const store = useHistoryStore();
    store.savedRunsRaw = [
      {
        ts: "2026-02-25T00:00:00Z",
        path: "C:/runs/run1.json",
        bytes: 1024,
        strategyKind: "DoubleMA",
        hasValidation: true,
        reportXlsxPath: "C:/runs/run1.xlsx",
        csvPath: "C:/data/aapl.csv",
        tags: ["holdout"],
      },
    ];
    store.syncSavedRunsView("C:/runs/run1.json");
    await flushServiceImports();

    loadSavedRunMock.mockRejectedValueOnce(new Error("load failed"));
    store.loadSavedRun(" C:/runs/run1.json ");
    await flushServiceImports();
    await flushServiceImports();
    expect(store.statusLine).toBe("Error: load failed");

    applySavedRunMock.mockRejectedValueOnce(new Error("apply failed"));
    store.applySavedRun(" C:/runs/run1.json ");
    await flushServiceImports();
    await flushServiceImports();
    expect(store.statusLine).toBe("Error: apply failed");

    rerunSavedRunMock.mockRejectedValueOnce(new Error("rerun failed"));
    store.rerunSavedRun(" C:/runs/run1.json ");
    await flushServiceImports();
    await flushServiceImports();
    expect(store.statusLine).toBe("Error: rerun failed");

    openSavedRunReportMock.mockRejectedValueOnce(new Error("report failed"));
    store.openSavedRunReport(" C:/runs/run1.json ");
    await flushServiceImports();
    await flushServiceImports();
    expect(store.statusLine).toBe("Error: report failed");

    revealSavedRunMock.mockRejectedValueOnce(new Error("reveal failed"));
    store.revealSavedRun(" C:/runs/run1.json ");
    await flushServiceImports();
    await flushServiceImports();
    expect(store.statusLine).toBe("Error: reveal failed");
    expect(store.source).toBe("vue");
  });

  it("hydrates and filters saved runs via Vue-owned refresh/search flow", async () => {
    const store = useHistoryStore();
    refreshSavedRunsMock.mockResolvedValue({
      runs: [
        {
          ts: "2026-02-25T00:00:00Z",
          path: "C:/runs/aapl.json",
          bytes: 2048,
          strategyKind: "DoubleMA",
          hasValidation: true,
          reportXlsxPath: "C:/runs/aapl.xlsx",
          csvPath: "C:/data/aapl.csv",
          tags: ["aapl", "1m"],
        },
        {
          ts: "2026-02-24T00:00:00Z",
          path: "C:/runs/msft.json",
          bytes: 1024,
          strategyKind: "CompositeCpu",
          hasValidation: false,
          reportXlsxPath: "",
          csvPath: "C:/data/msft.csv",
          tags: ["msft"],
        },
      ],
    });

    store.setSavedRunsSearch("aapl");
    store.refreshSavedRuns();
    await flushServiceImports();
    await flushServiceImports();

    expect(refreshSavedRunsMock).toHaveBeenCalledTimes(1);
    expect(store.savedRunCount).toBe(2);
    expect(store.savedOptions.length).toBe(2);
    expect(store.savedOptions[0]?.label).toContain("Select a saved run");
    expect(store.savedOptions[1]?.value).toBe("C:/runs/aapl.json");
    expect(store.savedRunsMeta).toBe("saved runs: 1/2");
    expect(store.statusLine).toBe("Refreshed saved runs.");
  });

  it("uses fuzzy ranking for saved runs while keeping short-query filtering stable", () => {
    const store = useHistoryStore();
    store.savedRunsRaw = [
      {
        ts: "2026-02-25T00:00:00Z",
        path: "C:/runs/aapl.json",
        bytes: 2048,
        strategyKind: "DoubleMA",
        hasValidation: true,
        reportXlsxPath: "C:/runs/aapl.xlsx",
        csvPath: "C:/data/aapl.csv",
        tags: ["aapl", "1m"],
      },
      {
        ts: "2026-02-24T00:00:00Z",
        path: "C:/runs/msft.json",
        bytes: 1024,
        strategyKind: "CompositeCpu",
        hasValidation: false,
        reportXlsxPath: "",
        csvPath: "C:/data/msft.csv",
        tags: ["msft", "swing"],
      },
    ];

    store.setSavedRunsSearch("appl");
    expect(store.savedRunsVisible.map((entry) => entry.path)).toEqual([
      "C:/runs/aapl.json",
    ]);

    store.setSavedRunsSearch("ms");
    expect(store.savedRunsVisible.map((entry) => entry.path)).toEqual([
      "C:/runs/msft.json",
    ]);

    store.setSavedRunsSearch("aapl 1m");
    expect(store.savedRunsVisible.map((entry) => entry.path)).toEqual([
      "C:/runs/aapl.json",
    ]);
  });

  it("rebuilds saved-run search results when the raw library changes under the same query", () => {
    const store = useHistoryStore();
    store.savedRunsRaw = [
      {
        ts: "2026-02-25T00:00:00Z",
        path: "C:/runs/aapl.json",
        bytes: 2048,
        strategyKind: "DoubleMA",
        hasValidation: true,
        replayClass: "exact_evidence_review",
        replayReasons: ["saved_run_result_payload_present"],
        reportXlsxPath: "C:/runs/aapl.xlsx",
        csvPath: "C:/data/aapl.csv",
        tags: ["aapl", "1m"],
      },
      {
        ts: "2026-02-24T00:00:00Z",
        path: "C:/runs/msft.json",
        bytes: 1024,
        strategyKind: "CompositeCpu",
        hasValidation: false,
        reportXlsxPath: "",
        csvPath: "C:/data/msft.csv",
        tags: ["msft"],
      },
    ];

    store.setSavedRunsSearch("ms");
    expect(store.savedRunsVisible.map((entry) => entry.path)).toEqual([
      "C:/runs/msft.json",
    ]);

    store.savedRunsRaw = [
      {
        ts: "2026-02-23T00:00:00Z",
        path: "C:/runs/nvda.json",
        bytes: 4096,
        strategyKind: "CompositeTrend",
        hasValidation: true,
        reportXlsxPath: "C:/runs/nvda.xlsx",
        csvPath: "C:/data/nvda.csv",
        tags: ["nvda"],
      },
    ];

    store.syncSavedRunsView("");
    expect(store.savedRunsVisible).toEqual([]);

    store.setSavedRunsSearch("nv");
    expect(store.savedRunsVisible.map((entry) => entry.path)).toEqual([
      "C:/runs/nvda.json",
    ]);
  });

  it("preserves selected detail while filters still match and clears it when no results remain", () => {
    const store = useHistoryStore();
    store.savedRunsRaw = [
      {
        ts: "2026-02-25T00:00:00Z",
        path: "C:/runs/aapl.json",
        bytes: 2048,
        strategyKind: "DoubleMA",
        hasValidation: true,
        replayClass: "exact_evidence_review",
        replayReasons: ["saved_run_result_payload_present"],
        reportXlsxPath: "C:/runs/aapl.xlsx",
        csvPath: "C:/data/aapl.csv",
        tags: ["aapl", "holdout"],
      },
      {
        ts: "2026-02-24T00:00:00Z",
        path: "C:/runs/msft.json",
        bytes: 1024,
        strategyKind: "CompositeCpu",
        hasValidation: false,
        reportXlsxPath: "",
        csvPath: "C:/data/msft.csv",
        tags: ["msft"],
      },
    ];
    store.savedRunDetailCache = {
      "C:/runs/aapl.json": {
        strategyKind: "DoubleMA",
        timeframe: "1m",
        csvPath: "C:/data/aapl.csv",
        reportPath: "C:/runs/aapl.xlsx",
        hasValidation: true,
        replayClass: "exact_evidence_review",
        replayReasons: [
          "saved_run_result_payload_present",
          "compatible_recompute_inputs_available",
        ],
        metricSurfaceSummary: "Core17",
        metricSurfaceDetail: "fields=pnl",
        validationAdmissionSummary: "scalar_union",
        validationAdmissionDetail: "src=top_rows",
        tags: ["aapl", "holdout"],
        mlBundleId: "",
        mlScoreCachePath: "",
        mlScoreManifestPath: "",
        mlFeatureSchemaPath: "",
        mlFeatureSchemaVersion: "",
        mlPolicySummary: "",
        mlPolicySweepSummary: "",
        mlRunVerdictSummary: "",
        mlTopCandidatesSummary: "",
        paramsText: "strategy: DoubleMA",
      },
    };

    store.syncSavedRunsView("C:/runs/aapl.json");
    expect(store.savedSelectedValue).toBe("C:/runs/aapl.json");
    expect(store.savedRunDetailTitle).toBe("2026-02-25T00:00:00Z");
    expect(store.savedRunDetailReplay).toContain("Exact evidence review");
    expect(store.savedRunsMeta).toContain("replay=Exact evidence review");

    store.setSavedRunsSearch("aapl");
    expect(store.savedSelectedValue).toBe("C:/runs/aapl.json");
    expect(store.savedRunDetailTitle).toBe("2026-02-25T00:00:00Z");
    expect(store.savedRunsVisible.map((entry) => entry.path)).toEqual([
      "C:/runs/aapl.json",
    ]);

    store.setSavedRunsSearch("exact_evidence");
    expect(store.savedRunsVisible.map((entry) => entry.path)).toEqual([
      "C:/runs/aapl.json",
    ]);

    store.setSavedRunsSearch("zzz");
    expect(store.savedSelectedValue).toBe("");
    expect(store.savedRunsVisible).toEqual([]);
    expect(store.savedRunsMeta).toBe("saved runs: 0/2");
    expect(store.savedRunDetailTitle).toBe("No saved run selected");
    expect(store.savedRunDetailParams).toBe(
      "Select a saved run to inspect parameters.",
    );
  });

  it("gates mutating saved-run actions while a runtime is marked active", () => {
    vi.stubGlobal("document", {
      documentElement: {
        dataset: { running: "1" },
      },
    });

    const store = useHistoryStore();
    store.savedRunsRaw = [
      {
        ts: "2026-02-25T00:00:00Z",
        path: "C:/runs/aapl.json",
        bytes: 2048,
        strategyKind: "DoubleMA",
        hasValidation: true,
        reportXlsxPath: "C:/runs/aapl.xlsx",
        csvPath: "C:/data/aapl.csv",
        tags: ["aapl"],
      },
    ];

    store.syncSavedRunsView("C:/runs/aapl.json");

    expect(store.canSaveSavedRunTags).toBe(false);
    expect(store.canRefreshSavedRuns).toBe(false);
    expect(store.canLoadSavedRun).toBe(false);
    expect(store.canApplySavedRun).toBe(false);
    expect(store.canRerunSavedRun).toBe(false);
    expect(store.canOpenSavedRunReport).toBe(false);
    expect(store.canRevealSavedRun).toBe(false);
    expect(store.canDeleteSavedRun).toBe(false);
    expect(store.canOpenRunsDir).toBe(false);
  });

  it("keeps future-schema saved runs inspectable but blocks hydrate/apply/rerun", () => {
    const store = useHistoryStore();
    store.savedRunsRaw = [
      {
        ts: "2026-02-25T00:00:00Z",
        path: "C:/runs/future.json",
        bytes: 2048,
        strategyKind: "DoubleMA",
        hasValidation: true,
        replayClass: "blocked",
        replayReasons: ["artifact_schema_future_unsupported"],
        artifactCompatibility: {
          artifactFamily: "run",
          artifactKind: "vectorgrid_run_artifact",
          artifactSchemaVersion: 2,
          supportedSchemaVersion: 1,
          commandResponseVersion: 1,
          reportSchemaVersion: null,
          status: "future_unsupported",
          reason: "newer than supported version",
          reviewOnly: true,
          replayBlocked: true,
        },
        reportXlsxPath: "C:/runs/future.xlsx",
        reportXlsxExists: null,
        csvPath: "C:/data/aapl.csv",
        tags: ["future"],
      },
    ];

    store.syncSavedRunsView("C:/runs/future.json");

    expect(store.savedSelectedValue).toBe("C:/runs/future.json");
    expect(store.savedRunDetailCompatibility).toContain("review-only");
    expect(store.canLoadSavedRun).toBe(false);
    expect(store.canApplySavedRun).toBe(false);
    expect(store.canRerunSavedRun).toBe(false);
    expect(store.canRevealSavedRun).toBe(true);
    expect(store.canDeleteSavedRun).toBe(true);
  });

  it("disables saved-run report open actions when persisted metadata says the report file is gone", () => {
    const store = useHistoryStore();
    store.savedRunsRaw = [
      {
        ts: "2026-02-25T00:00:00Z",
        path: "C:/runs/aapl.json",
        bytes: 2048,
        strategyKind: "DoubleMA",
        hasValidation: true,
        replayClass: "exact_evidence_review",
        replayReasons: ["saved_run_result_payload_present"],
        reportXlsxPath: "C:/runs/aapl.xlsx",
        reportXlsxExists: false,
        csvPath: "C:/data/aapl.csv",
        tags: ["aapl"],
      },
    ];

    store.syncSavedRunsView("C:/runs/aapl.json");

    expect(store.canOpenSavedRunReport).toBe(false);
  });

  it("renders saved-run detail text as missing-on-disk when the persisted report flag is false", () => {
    const store = useHistoryStore();
    store.savedRunsRaw = [
      {
        ts: "2026-02-25T00:00:00Z",
        path: "C:/runs/aapl.json",
        bytes: 2048,
        strategyKind: "DoubleMA",
        hasValidation: true,
        reportXlsxPath: "C:/runs/aapl.xlsx",
        reportXlsxExists: false,
        csvPath: "C:/data/aapl.csv",
        tags: ["aapl"],
      },
    ];

    store.syncSavedRunsView("C:/runs/aapl.json");

    expect(store.savedRunsMeta).toContain("xlsx=missing");
    expect(store.savedRunDetailReport).toBe("Missing on disk");
  });

  it("keeps saved-run report availability unknown when persisted metadata is unverified", () => {
    const store = useHistoryStore();
    store.savedRunsRaw = [
      {
        ts: "2026-02-25T00:00:00Z",
        path: "C:/runs/aapl.json",
        bytes: 2048,
        strategyKind: "DoubleMA",
        hasValidation: true,
        reportXlsxPath: "C:/runs/aapl.xlsx",
        reportXlsxExists: null,
        csvPath: "C:/data/aapl.csv",
        tags: ["aapl"],
      },
    ];

    store.syncSavedRunsView("C:/runs/aapl.json");

    expect(store.canOpenSavedRunReport).toBe(true);
    expect(store.savedRunsMeta).toContain("xlsx=unknown");
    expect(store.savedRunDetailReport).toBe("Availability unverified");
  });

  it("prunes stale saved-run detail cache entries on refresh while preserving surviving selection", async () => {
    const store = useHistoryStore();
    const detailA = {
      strategyKind: "DoubleMA",
      timeframe: "1m",
      csvPath: "C:/data/aapl.csv",
      reportPath: "C:/runs/aapl.xlsx",
      hasValidation: true,
      metricSurfaceSummary: "Core17",
      metricSurfaceDetail: "",
      validationAdmissionSummary: "",
      validationAdmissionDetail: "",
      tags: ["aapl"],
      mlBundleId: "",
      mlScoreCachePath: "",
      mlScoreManifestPath: "",
      mlFeatureSchemaPath: "",
      mlFeatureSchemaVersion: "",
      mlPolicySummary: "",
      mlPolicySweepSummary: "",
      mlRunVerdictSummary: "",
      mlTopCandidatesSummary: "",
      paramsText: "strategy: DoubleMA",
    };
    const detailB = {
      ...detailA,
      csvPath: "C:/data/msft.csv",
      reportPath: "",
      tags: ["msft"],
      paramsText: "strategy: CompositeCpu",
    };
    store.savedRunsRaw = [
      {
        ts: "2026-02-25T00:00:00Z",
        path: "C:/runs/aapl.json",
        bytes: 2048,
        strategyKind: "DoubleMA",
        hasValidation: true,
        reportXlsxPath: "C:/runs/aapl.xlsx",
        csvPath: "C:/data/aapl.csv",
        tags: ["aapl"],
      },
      {
        ts: "2026-02-24T00:00:00Z",
        path: "C:/runs/msft.json",
        bytes: 1024,
        strategyKind: "CompositeCpu",
        hasValidation: false,
        reportXlsxPath: "",
        csvPath: "C:/data/msft.csv",
        tags: ["msft"],
      },
    ];
    store.savedRunDetailCache = {
      "C:/runs/aapl.json": detailA,
      "C:/runs/msft.json": detailB,
    };
    store.syncSavedRunsView("C:/runs/aapl.json");
    refreshSavedRunsMock.mockResolvedValue({
      runs: [
        {
          ts: "2026-02-25T00:00:00Z",
          path: "C:/runs/aapl.json",
          bytes: 2048,
          strategyKind: "DoubleMA",
          hasValidation: true,
          reportXlsxPath: "C:/runs/aapl.xlsx",
          csvPath: "C:/data/aapl.csv",
          tags: ["aapl"],
        },
      ],
    });

    store.refreshSavedRuns();
    await flushServiceImports();
    await flushServiceImports();

    expect(store.savedSelectedValue).toBe("C:/runs/aapl.json");
    expect(store.savedRunsVisible.map((entry) => entry.path)).toEqual([
      "C:/runs/aapl.json",
    ]);
    expect(Object.keys(store.savedRunDetailCache)).toEqual([
      "C:/runs/aapl.json",
    ]);
    expect(store.statusLine).toBe("Refreshed saved runs.");
  });

  it("prunes deleted saved-run detail cache entries and clears selection when the active run is removed", async () => {
    const store = useHistoryStore();
    store.savedRunsRaw = [
      {
        ts: "2026-02-25T00:00:00Z",
        path: "C:/runs/aapl.json",
        bytes: 2048,
        strategyKind: "DoubleMA",
        hasValidation: true,
        reportXlsxPath: "C:/runs/aapl.xlsx",
        csvPath: "C:/data/aapl.csv",
        tags: ["aapl"],
      },
      {
        ts: "2026-02-24T00:00:00Z",
        path: "C:/runs/msft.json",
        bytes: 1024,
        strategyKind: "CompositeCpu",
        hasValidation: false,
        reportXlsxPath: "",
        csvPath: "C:/data/msft.csv",
        tags: ["msft"],
      },
    ];
    store.savedRunDetailCache = {
      "C:/runs/aapl.json": {
        strategyKind: "DoubleMA",
        timeframe: "1m",
        csvPath: "C:/data/aapl.csv",
        reportPath: "C:/runs/aapl.xlsx",
        hasValidation: true,
        metricSurfaceSummary: "",
        metricSurfaceDetail: "",
        validationAdmissionSummary: "",
        validationAdmissionDetail: "",
        tags: ["aapl"],
        mlBundleId: "",
        mlScoreCachePath: "",
        mlScoreManifestPath: "",
        mlFeatureSchemaPath: "",
        mlFeatureSchemaVersion: "",
        mlPolicySummary: "",
        mlPolicySweepSummary: "",
        mlRunVerdictSummary: "",
        mlTopCandidatesSummary: "",
        paramsText: "strategy: DoubleMA",
      },
      "C:/runs/msft.json": {
        strategyKind: "CompositeCpu",
        timeframe: "1m",
        csvPath: "C:/data/msft.csv",
        reportPath: "",
        hasValidation: false,
        metricSurfaceSummary: "",
        metricSurfaceDetail: "",
        validationAdmissionSummary: "",
        validationAdmissionDetail: "",
        tags: ["msft"],
        mlBundleId: "",
        mlScoreCachePath: "",
        mlScoreManifestPath: "",
        mlFeatureSchemaPath: "",
        mlFeatureSchemaVersion: "",
        mlPolicySummary: "",
        mlPolicySweepSummary: "",
        mlRunVerdictSummary: "",
        mlTopCandidatesSummary: "",
        paramsText: "strategy: CompositeCpu",
      },
    };
    store.syncSavedRunsView("C:/runs/aapl.json");
    refreshSavedRunsMock.mockResolvedValue({
      runs: [
        {
          ts: "2026-02-24T00:00:00Z",
          path: "C:/runs/msft.json",
          bytes: 1024,
          strategyKind: "CompositeCpu",
          hasValidation: false,
          reportXlsxPath: "",
          csvPath: "C:/data/msft.csv",
          tags: ["msft"],
        },
      ],
    });

    store.deleteSavedRun("C:/runs/aapl.json");
    await flushServiceImports();
    await flushServiceImports();

    expect(deleteSavedRunMock).toHaveBeenCalledWith("C:/runs/aapl.json");
    expect(store.savedSelectedValue).toBe("");
    expect(store.savedRunsVisible.map((entry) => entry.path)).toEqual([
      "C:/runs/msft.json",
    ]);
    expect(Object.keys(store.savedRunDetailCache)).toEqual([
      "C:/runs/msft.json",
    ]);
    expect(store.statusLine).toBe("Deleted saved run.");
  });
});
