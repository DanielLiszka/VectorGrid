import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { LAST_BATCH_SUMMARY_KEY } from "../utils/core/storageKeys";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => undefined),
}));

vi.mock("../tauri/events", () => ({
  listenEvent: vi.fn(async () => () => undefined),
}));

vi.mock("./optimizeService", () => ({
  buildOptimizationRequestFromVueStores: vi.fn(async () => ({
    strategy_kind: "DoubleMa",
    data_id: "data-1",
    ma_source: "close",
  })),
}));

function installMemoryStorage(): Storage {
  const data = new Map<string, string>();
  const storage: Storage = {
    get length() {
      return data.size;
    },
    clear() {
      data.clear();
    },
    getItem(key: string) {
      return data.has(key) ? data.get(key)! : null;
    },
    key(index: number) {
      return Array.from(data.keys())[index] ?? null;
    },
    removeItem(key: string) {
      data.delete(key);
    },
    setItem(key: string, value: string) {
      data.set(key, String(value));
    },
  };
  (globalThis as { localStorage?: Storage }).localStorage = storage;
  return storage;
}

function summaryFor(state: string, items: Array<Record<string, unknown>>) {
  return {
    version: 1,
    kind: "vectorgrid_campaign_summary",
    ts: "2026-04-03T12:00:00Z",
    state,
    strategy_kind: "DoubleMa",
    counts: {
      total: 2,
      optimized: 1,
      validated: 1,
      skipped: 0,
      failed: items.filter(
        (item) => String(item.status ?? "").toLowerCase() === "failed",
      ).length,
      cancelled: state === "cancelled",
    },
    items,
    warnings: [],
    runtime_ms: 10,
    tags: ["campaign"],
    holdout_pct: 0,
    holdout_tag: null,
    request: {
      strategy_kind: "DoubleMa",
      datasets: [
        { csv_path: "C:/a.csv", label: "AAPL", timeframe: "1m" },
        { csv_path: "C:/b.csv", label: "MSFT", timeframe: "1m" },
      ],
      timeframe_override: "1m",
      tags: ["campaign"],
      holdout_pct: 0,
      holdout_tag: null,
      skip_existing: false,
      auto_meta: false,
      auto_stage5: false,
      optimization_template: {
        strategy_kind: "DoubleMa",
        data_id: "data-1",
        ma_source: "close",
      },
      validation_config: {
        max_candidates: 8,
        seed: 1,
      },
      preload_policy: "host_only",
    },
  };
}

async function setupHarness(
  summaryPath: string,
  summary: Record<string, unknown>,
) {
  vi.resetModules();
  const storage = installMemoryStorage();
  storage.clear();
  storage.setItem(LAST_BATCH_SUMMARY_KEY, summaryPath);

  const { bindRuntimePinia } = await import("../stores/runtimePinia");
  const { useAnalysisStore } = await import("../stores/analysisStore");
  const { useOptimizeStore } = await import("../stores/optimizeStore");
  const { invokeCommand } = await import("../tauri/invoke");
  const invokeCommandMock = vi.mocked(invokeCommand);

  invokeCommandMock.mockReset();
  invokeCommandMock.mockImplementation(async (command, args) => {
    if (command === "polygon_get_cache_info") {
      return { datasets: [] };
    }
    if (command === "list_repo_data_csvs") {
      return {
        dir: "C:/repo-data",
        truncated: false,
        csv_entries: [
          {
            csv_path: "C:/a.csv",
            label: "AAPL",
            timeframe: "1m",
            market: "local",
            source: "repo_data",
          },
          {
            csv_path: "C:/b.csv",
            label: "MSFT",
            timeframe: "1m",
            market: "local",
            source: "repo_data",
          },
        ],
      };
    }
    if (command === "load_artifact") {
      return summary;
    }
    if (command === "run_research_campaign") {
      const req = (args?.req ?? {}) as Record<string, unknown>;
      return {
        summary_json_path: "C:/runs/vectorgrid_campaign_resumed.json",
        summary: {
          ...summary,
          state: "completed",
          request: req,
          counts: {
            total: Array.isArray(req.datasets) ? req.datasets.length : 0,
            optimized: Array.isArray(req.datasets) ? req.datasets.length : 0,
            validated: Array.isArray(req.datasets) ? req.datasets.length : 0,
            skipped: 0,
            failed: 0,
            cancelled: false,
          },
          items: Array.isArray(req.datasets)
            ? req.datasets.map((row) => ({
                csv_path: (row as Record<string, unknown>).csv_path,
                label: (row as Record<string, unknown>).label,
                timeframe: (row as Record<string, unknown>).timeframe,
                is_holdout: false,
                status: "validated",
              }))
            : [],
        },
      };
    }
    if (command === "list_run_artifacts") return [];
    if (command === "list_artifact_library") {
      return { meta: [], stage5: [], policyEval: [] };
    }
    return undefined;
  });

  const pinia = createPinia();
  setActivePinia(pinia);
  bindRuntimePinia(pinia);

  const optimizeStore = useOptimizeStore();
  optimizeStore.strategyKind = "DoubleMa";
  optimizeStore.updatedAtUnixMs = Date.now();

  return {
    invokeCommandMock,
    analysisStore: useAnalysisStore(),
  };
}

describe("meta-service campaign resume and retry", () => {
  beforeEach(() => {
    installMemoryStorage().clear();
  });

  it("aborts cleanly when no saved summary path exists", async () => {
    vi.resetModules();
    installMemoryStorage().clear();

    const { bindRuntimePinia } = await import("../stores/runtimePinia");
    const { useAnalysisStore } = await import("../stores/analysisStore");
    const { useOptimizeStore } = await import("../stores/optimizeStore");
    const { invokeCommand } = await import("../tauri/invoke");
    const invokeCommandMock = vi.mocked(invokeCommand);
    invokeCommandMock.mockReset();
    invokeCommandMock.mockResolvedValue(undefined);

    const pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);

    const optimizeStore = useOptimizeStore();
    optimizeStore.strategyKind = "DoubleMa";
    optimizeStore.updatedAtUnixMs = Date.now();

    const analysisStore = useAnalysisStore();
    const { batchResumeIncomplete } = await import("./metaService");

    await batchResumeIncomplete();

    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === "run_research_campaign",
      ),
    ).toBe(false);
    expect(analysisStore.statusLine).toBe("No batch summary loaded.");
  });

  it("resumes an incomplete campaign from the saved summary path on disk", async () => {
    const summaryPath = "C:/runs/incomplete_campaign.json";
    const summary = summaryFor("cancelled", [
      {
        csv_path: "C:/a.csv",
        label: "AAPL",
        timeframe: "1m",
        is_holdout: false,
        status: "validated",
      },
      {
        csv_path: "C:/b.csv",
        label: "MSFT",
        timeframe: "1m",
        is_holdout: false,
        status: "failed",
        error: "validation: cancelled",
      },
    ]);
    const { invokeCommandMock } = await setupHarness(summaryPath, summary);
    const { batchResumeIncomplete } = await import("./metaService");

    await batchResumeIncomplete();

    expect(invokeCommandMock).toHaveBeenCalledWith("load_artifact", {
      path: summaryPath,
    });
    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === "run_research_campaign",
    );
    expect(call).toBeTruthy();
    expect(call?.[1]).toMatchObject({
      req: {
        strategy_kind: "DoubleMa",
        skip_existing: true,
        datasets: [
          { csv_path: "C:/a.csv", label: "AAPL", timeframe: "1m" },
          { csv_path: "C:/b.csv", label: "MSFT", timeframe: "1m" },
        ],
      },
    });
  });

  it("retries from a saved summary path using the original dataset set plus skip_existing", async () => {
    const summaryPath = "C:/runs/failed_campaign.json";
    const summary = summaryFor("completed_with_failures", [
      {
        csv_path: "C:/a.csv",
        label: "AAPL",
        timeframe: "1m",
        is_holdout: false,
        status: "validated",
      },
      {
        csv_path: "C:/b.csv",
        label: "MSFT",
        timeframe: "1m",
        is_holdout: false,
        status: "failed",
        error: "validation: decode error",
      },
    ]);
    const { invokeCommandMock, analysisStore } = await setupHarness(
      summaryPath,
      summary,
    );
    const { batchRetryFailed } = await import("./metaService");

    await batchRetryFailed();

    expect(invokeCommandMock).toHaveBeenCalledWith("load_artifact", {
      path: summaryPath,
    });
    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === "run_research_campaign",
    );
    expect(call).toBeTruthy();
    expect(call?.[1]).toMatchObject({
      req: {
        strategy_kind: "DoubleMa",
        skip_existing: true,
        datasets: [
          { csv_path: "C:/a.csv", label: "AAPL", timeframe: "1m" },
          { csv_path: "C:/b.csv", label: "MSFT", timeframe: "1m" },
        ],
      },
    });
    expect(analysisStore.batchSkipDone).toBe(true);
  });

  it("does not resume when the saved summary lacks a resumable request", async () => {
    const summaryPath = "C:/runs/non_resumable_campaign.json";
    const summary = {
      ...summaryFor("cancelled", [
        {
          csv_path: "C:/a.csv",
          label: "AAPL",
          timeframe: "1m",
          is_holdout: false,
          status: "failed",
          error: "validation: cancelled",
        },
      ]),
      request: null,
    };
    const { invokeCommandMock, analysisStore } = await setupHarness(
      summaryPath,
      summary,
    );
    const { batchResumeIncomplete } = await import("./metaService");

    await batchResumeIncomplete();

    expect(invokeCommandMock).toHaveBeenCalledWith("load_artifact", {
      path: summaryPath,
    });
    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === "run_research_campaign",
      ),
    ).toBe(false);
    expect(analysisStore.statusLine).toBe(
      "Current summary does not contain a resumable campaign request.",
    );
  });

  it("reveals the saved summary path by hydrating the last batch summary from storage", async () => {
    const summaryPath = "C:/runs/completed_campaign.json";
    const summary = summaryFor("completed", [
      {
        csv_path: "C:/a.csv",
        label: "AAPL",
        timeframe: "1m",
        is_holdout: false,
        status: "validated",
      },
    ]);
    const { invokeCommandMock, analysisStore } = await setupHarness(
      summaryPath,
      summary,
    );
    const { batchRevealSummary } = await import("./metaService");

    await batchRevealSummary();

    expect(invokeCommandMock).toHaveBeenCalledWith("load_artifact", {
      path: summaryPath,
    });
    expect(invokeCommandMock).toHaveBeenCalledWith("reveal_path", {
      path: summaryPath,
    });
    expect(analysisStore.batchOutputPreview).toContain(summaryPath);
    expect(analysisStore.statusLine).toBe("Revealed batch summary.");
  });
});
