import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useHistoryStore } from "./historyStore";

function seedSavedRuns() {
  return [
    {
      ts: "2026-03-21T10:00:00Z",
      path: "C:/runs/composite-aapl.json",
      bytes: 1024,
      strategyKind: "CompositeMo",
      hasValidation: true,
      reportXlsxPath: "C:/reports/composite-aapl.xlsx",
      csvPath: "C:/data/AAPL_intraday.csv",
      tags: ["robust", "favorites"],
    },
    {
      ts: "2026-03-20T08:30:00Z",
      path: "C:/runs/orc-msft.json",
      bytes: 2048,
      strategyKind: "Orc",
      hasValidation: false,
      reportXlsxPath: "",
      csvPath: "C:/data/MSFT_daily.csv",
      tags: ["scout", "watch"],
    },
    {
      ts: "2026-03-19T14:15:00Z",
      path: "C:/runs/ml-spy.json",
      bytes: 3072,
      strategyKind: "MlSignal",
      hasValidation: true,
      reportXlsxPath: "C:/reports/ml-spy.xlsx",
      csvPath: "C:/data/SPY_daily.csv",
      tags: ["ml", "baseline"],
    },
  ];
}

describe("historyStore fuzzy saved-run search", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    document.documentElement.dataset.running = "";
  });

  it("keeps saved runs in original order when the query is empty", () => {
    const store = useHistoryStore();
    store.savedRunsRaw = seedSavedRuns();

    store.syncSavedRunsView();

    expect(store.savedRunsVisible.map((entry) => entry.path)).toEqual([
      "C:/runs/composite-aapl.json",
      "C:/runs/orc-msft.json",
      "C:/runs/ml-spy.json",
    ]);
  });

  it("prefers exact and prefix-style matches for short queries", () => {
    const store = useHistoryStore();
    store.savedRunsRaw = seedSavedRuns();

    store.setSavedRunsSearch("co");

    expect(store.savedRunsVisible[0]?.strategyKind).toBe("CompositeMo");
    expect(store.savedRunsVisible[0]?.path).toBe("C:/runs/composite-aapl.json");
  });

  it("matches saved runs with typo-tolerant fuzzy search", () => {
    const store = useHistoryStore();
    store.savedRunsRaw = seedSavedRuns();

    store.setSavedRunsSearch("coposite");

    expect(store.savedRunsVisible[0]?.strategyKind).toBe("CompositeMo");
    expect(store.savedRunsVisible.some((entry) => entry.path === "C:/runs/composite-aapl.json")).toBe(true);
  });

  it("supports multi-token retrieval across dataset and tags", () => {
    const store = useHistoryStore();
    store.savedRunsRaw = seedSavedRuns();

    store.setSavedRunsSearch("aapl robust");

    expect(store.savedRunsVisible).toHaveLength(1);
    expect(store.savedRunsVisible[0]?.path).toBe("C:/runs/composite-aapl.json");
  });
});
