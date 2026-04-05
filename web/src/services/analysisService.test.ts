// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { invokeCommand } from "../tauri/invoke";
import { bindRuntimePinia } from "../stores/runtimePinia";
import {
  clearBaseline,
  clearTopFilters,
  drillClearTradeSelection,
  drillNextTrade,
  drillPrevTrade,
  invalidateAnalysisExecutionCaches,
  refreshAnalysisFromWorkflowRun,
  restoreHeatmapVisible,
  refreshParetoFromWorkflowRun,
  refreshTopTableFromWorkflowRun,
  rerenderDrill,
  selectTopRow,
  setBaseline,
  sortTopTable,
} from "./analysisService";
import { useAnalysisStore } from "../stores/analysisStore";
import { useHistoryStore } from "../stores/historyStore";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => ({
    ts: "2026-02-26T11:59:00.000Z",
    run: {
      req: { strategy_kind: "DoubleMa", objective: "Sharpe" },
      res: {
        total_evaluations: 2,
        runtime_ms: 10,
        best_params: { fast_len: 5, slow_len: 20 },
        best_metrics: {
          pnl: 0.2,
          sharpe: 1.1,
          max_dd: 0.1,
          trades: 10,
          exposure: 0.4,
        },
      },
    },
  })),
}));

const invokeCommandMock = vi.mocked(invokeCommand);
let pinia: ReturnType<typeof createPinia>;

function createDeferred<T>(): {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (error?: unknown) => void;
} {
  let resolve!: (value: T) => void;
  let reject!: (error?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function seedWorkflowRun(payload: {
  ts?: string;
  artifactPath?: string;
  optimizationMetricSurface?: unknown;
  req?: unknown;
  res?: unknown;
  validationRes?: unknown;
}): void {
  const root = pinia.state.value as Record<string, unknown>;
  root.workflowArtifacts = {
    currentRun: {
      ts: payload.ts || "2026-02-26T12:00:00.000Z",
      artifactPath: payload.artifactPath || "",
      optimizationMetricSurface: payload.optimizationMetricSurface ?? null,
      req: payload.req ?? null,
      res: payload.res ?? null,
    },
    currentValidation: {
      res: payload.validationRes ?? null,
    },
  };
}

function mountDrillDom(): void {
  document.body.innerHTML = `
    <div id="drillEmptyHintVue"></div>
    <div id="drillWrapVue" style="display:none;"></div>
    <div id="drillMetaVue">-</div>
    <div id="drillStatusVue">-</div>
    <canvas id="equityCanvasVue" width="1024" height="160"></canvas>
    <canvas id="drawdownCanvasVue" width="1024" height="160"></canvas>
    <div id="tradeStatsWrapVue" style="display:none;"></div>
    <div id="tradeStatsVue">-</div>
    <tbody id="tradeTableBodyVue"></tbody>
    <div id="stabilityWrapVue" style="display:none;"></div>
    <div id="stabilityStatusVue">-</div>
    <div id="stabilityMetaVue">-</div>
    <div id="rejectMetaVue">-</div>
    <div id="stabilityTableWrapVue" style="display:none;"></div>
    <tbody id="stabilityTableBodyVue"></tbody>
  `;
}

function mountHeatmapDom(): void {
  document.body.innerHTML = `
    <div id="heatmapEmptyHintVue"></div>
    <div id="heatmapWrapVue" style="display:none;"></div>
    <div id="heatmapMetaVue">-</div>
    <canvas id="heatmapCanvasVue" width="512" height="512"></canvas>
    <canvas id="heatmapOverlayVue" width="512" height="512"></canvas>
    <div id="heatmapTooltipVue" style="display:none;"></div>
    <canvas id="heatmapLegendVue" width="512" height="18"></canvas>
    <span id="heatmapLegendMinVue">-</span>
    <span id="heatmapLegendMidVue">-</span>
    <span id="heatmapLegendMaxVue">-</span>
  `;
}

describe("analysis service", () => {
  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    useAnalysisStore(pinia);
    vi.restoreAllMocks();
    vi.clearAllMocks();
    invalidateAnalysisExecutionCaches();
    if (typeof document !== "undefined") document.body.innerHTML = "";
    if (typeof HTMLCanvasElement !== "undefined") {
      vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
        () =>
          ({
            clearRect: () => undefined,
            fillRect: () => undefined,
            strokeRect: () => undefined,
            beginPath: () => undefined,
            arc: () => undefined,
            fill: () => undefined,
            stroke: () => undefined,
            fillText: () => undefined,
            moveTo: () => undefined,
            lineTo: () => undefined,
            save: () => undefined,
            restore: () => undefined,
            translate: () => undefined,
            rotate: () => undefined,
            createLinearGradient: () => ({ addColorStop: () => undefined }),
          }) as never,
      );
    }
    invalidateAnalysisExecutionCaches()
    invokeCommandMock.mockResolvedValue({
      ts: "2026-02-26T11:59:00.000Z",
      run: {
        req: { strategy_kind: "DoubleMa", objective: "Sharpe" },
        res: {
          total_evaluations: 2,
          runtime_ms: 10,
          best_params: { fast_len: 5, slow_len: 20 },
          best_metrics: {
            pnl: 0.2,
            sharpe: 1.1,
            max_dd: 0.1,
            trades: 10,
            exposure: 0.4,
          },
        },
      },
    } as never);
  });

  it("sorts top table rows in Vue store state", async () => {
    seedWorkflowRun({
      req: { strategy_kind: "DoubleMa", objective: "Sharpe" },
      res: {
        top: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 5,
              slow_ma_type: "ema",
              slow_len: 20,
            },
            { pnl: 0.1, sharpe: 0.5, max_dd: 0.2, trades: 10, exposure: 0.4 },
          ],
          [
            {
              fast_ma_type: "ema",
              fast_len: 7,
              slow_ma_type: "ema",
              slow_len: 30,
            },
            { pnl: 0.2, sharpe: 1.2, max_dd: 0.1, trades: 20, exposure: 0.5 },
          ],
          [
            {
              fast_ma_type: "ema",
              fast_len: 9,
              slow_ma_type: "ema",
              slow_len: 40,
            },
            {
              pnl: 0.15,
              sharpe: 0.8,
              max_dd: 0.12,
              trades: 15,
              exposure: 0.45,
            },
          ],
        ],
      },
    });
    const store = useAnalysisStore(pinia);

    await sortTopTable(" sharpe ");

    const sharpeCol = store.topColumns.find(
      (column) => column.key === "sharpe",
    );
    expect(String(sharpeCol?.sort || "")).toBe("desc");
    expect(String(store.topTableRows[0]?.cells?.[5]?.text || "")).toBe("1.2");
    expect(String(store.topTableRows[1]?.cells?.[5]?.text || "")).toBe("0.8");
    expect(String(store.topTableRows[2]?.cells?.[5]?.text || "")).toBe("0.5");
    expect(store.topTableRows[0]?.selected).toBe(true);
    expect(store.source).toBe("vue");
  });

  it("selects top row in Vue store state when row model is unavailable", async () => {
    const store = useAnalysisStore(pinia);
    store.topTableRows = [
      { selected: false, cells: [{ text: "A", className: "", title: "" }] },
      { selected: false, cells: [{ text: "B", className: "", title: "" }] },
      { selected: false, cells: [{ text: "C", className: "", title: "" }] },
    ];

    await selectTopRow(3.9);

    expect(store.topTableRows[0]?.selected).toBe(false);
    expect(store.topTableRows[1]?.selected).toBe(false);
    expect(store.topTableRows[2]?.selected).toBe(true);
    expect(String(store.topSelectionGuardrail || "")).toBe("—");
    expect(String(store.statusLine || "")).toContain("Selected top row 3.");
    expect(store.source).toBe("vue");
  });

  it("computes drilldown/stability via Vue command path on top-row selection", async () => {
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        data_id: "dataset-1",
        ma_source: "close",
        objective: "Sharpe",
        backend: { backend: "GpuKernel", device_id: 0 },
        strategy: {},
        halftrend_filter: null,
      },
      res: {
        followup_cache_key: "cache-token-1",
        top: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 5,
              slow_ma_type: "ema",
              slow_len: 20,
            },
            { pnl: 0.1, sharpe: 0.5, max_dd: 0.2, trades: 10, exposure: 0.4 },
          ],
          [
            {
              fast_ma_type: "ema",
              fast_len: 7,
              slow_ma_type: "ema",
              slow_len: 30,
            },
            { pnl: 0.2, sharpe: 1.2, max_dd: 0.1, trades: 20, exposure: 0.5 },
          ],
        ],
      },
    });
    const store = useAnalysisStore(pinia);
    refreshTopTableFromWorkflowRun();
    invokeCommandMock.mockImplementation(async (command: unknown) => {
      if (command === "compute_double_ma_drilldown") {
        return {
          equity: { start_t: 1, end_t: 2, bins: 1, minmax: [1, 1] },
          drawdown: { start_t: 1, end_t: 2, bins: 1, minmax: [0, 0] },
          trades: [],
          trade_stats: null,
          indicator_bars: [],
        } as never;
      }
      if (command === "compute_double_ma_local_stability") {
        return {
          base: {
            fast_len: 7,
            slow_len: 30,
            fast_ma_type: "ema",
            slow_ma_type: "ema",
          },
          objective: "Sharpe",
          radius: 1,
          step: 1,
          start_t: 1,
          points: [],
          summary: { base: 0, min: 0, median: 0, max: 0 },
        } as never;
      }
      return {
        ts: "2026-02-26T11:59:00.000Z",
        run: {
          req: { strategy_kind: "DoubleMa", objective: "Sharpe" },
          res: {
            total_evaluations: 2,
            runtime_ms: 10,
            best_params: { fast_len: 5, slow_len: 20 },
            best_metrics: {
              pnl: 0.2,
              sharpe: 1.1,
              max_dd: 0.1,
              trades: 10,
              exposure: 0.4,
            },
          },
        },
      } as never;
    });

    await selectTopRow(1);

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "compute_double_ma_drilldown",
      expect.objectContaining({
        req: expect.objectContaining({
          backend_hint: { backend: "GpuKernel", device_id: 0 },
          followup_cache_key: "cache-token-1",
        }),
      }),
    );
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "compute_double_ma_local_stability",
      expect.objectContaining({
        req: expect.objectContaining({
          backend_hint: { backend: "GpuKernel", device_id: 0 },
          followup_cache_key: "cache-token-1",
        }),
      }),
    );
    expect(store.topTableRows[1]?.selected).toBe(true);
  });

  it("omits drill-only fields and propagates DoubleMa score_mode in local stability requests", async () => {
    useAnalysisStore(pinia);
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        data_id: "dataset-1",
        ma_source: "close",
        objective: "Sharpe",
        score_mode: "balanced_risk_adjusted_10",
        backend: { backend: "GpuKernel", device_id: 0 },
        strategy: {},
        halftrend_filter: null,
      },
      res: {
        followup_cache_key: "cache-token-1",
        top: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 7,
              slow_ma_type: "ema",
              slow_len: 30,
            },
            { pnl: 0.2, sharpe: 1.2, max_dd: 0.1, trades: 20, exposure: 0.5 },
          ],
        ],
      },
    });
    refreshTopTableFromWorkflowRun();
    invokeCommandMock.mockImplementation(async (command: unknown) => {
      if (command === "compute_double_ma_drilldown") {
        return {
          equity: { start_t: 1, end_t: 2, bins: 1, minmax: [1, 1] },
          drawdown: { start_t: 1, end_t: 2, bins: 1, minmax: [0, 0] },
          trades: [],
          trade_stats: null,
          indicator_bars: [],
        } as never;
      }
      if (command === "compute_double_ma_local_stability") {
        return {
          base: {
            fast_len: 7,
            slow_len: 30,
            fast_ma_type: "ema",
            slow_ma_type: "ema",
          },
          objective: "Sharpe",
          radius: 1,
          step: 1,
          start_t: 1,
          points: [],
          summary: { base: 0, min: 0, median: 0, max: 0 },
        } as never;
      }
      if (command === "get_price_bars") {
        return [] as never;
      }
      return undefined as never;
    });

    await selectTopRow(0);

    const stabilityCall = invokeCommandMock.mock.calls.find(
      ([command]) => command === "compute_double_ma_local_stability",
    );
    expect(stabilityCall).toBeTruthy();
    const req = (stabilityCall?.[1] as { req?: Record<string, unknown> }).req ?? {};
    expect(req.backend_hint).toEqual({ backend: "GpuKernel", device_id: 0 });
    expect(req.followup_cache_key).toBe("cache-token-1");
    expect(req.score_mode).toBe("balanced_risk_adjusted_10");
    expect(req).not.toHaveProperty("bins");
  });

  it("deduplicates identical in-flight drill, price-bar, and stability work", async () => {
    useAnalysisStore(pinia);
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        data_id: "dataset-1",
        ma_source: "close",
        objective: "Sharpe",
        backend: { backend: "CpuOnly" },
        strategy: {},
        halftrend_filter: null,
      },
      res: {
        followup_cache_key: "cache-token-1",
        top: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 7,
              slow_ma_type: "ema",
              slow_len: 30,
            },
            { pnl: 0.2, sharpe: 1.2, max_dd: 0.1, trades: 20, exposure: 0.5 },
          ],
        ],
      },
    });
    refreshTopTableFromWorkflowRun();
    const drillDeferred = createDeferred<unknown>();
    const barsDeferred = createDeferred<unknown>();
    const stabilityDeferred = createDeferred<unknown>();
    let drillCalls = 0;
    let barCalls = 0;
    let stabilityCalls = 0;

    invokeCommandMock.mockImplementation((async (command: unknown) => {
      if (command === "compute_double_ma_drilldown") {
        drillCalls += 1;
        return await drillDeferred.promise;
      }
      if (command === "get_price_bars") {
        barCalls += 1;
        return await barsDeferred.promise;
      }
      if (command === "compute_double_ma_local_stability") {
        stabilityCalls += 1;
        return await stabilityDeferred.promise;
      }
      return undefined;
    }) as never);

    const first = selectTopRow(0);
    drillDeferred.resolve({
      equity: { start_t: 1, end_t: 2, bins: 1, minmax: [1, 1] },
      drawdown: { start_t: 1, end_t: 2, bins: 1, minmax: [0, 0] },
      trades: [],
      trade_stats: null,
      indicator_bars: [],
    });
    await Promise.resolve();

    const second = selectTopRow(0);
    await Promise.resolve();
    barsDeferred.resolve([]);
    await Promise.resolve();
    await Promise.resolve();

    const third = selectTopRow(0);
    await Promise.resolve();
    stabilityDeferred.resolve({
      base: {
        fast_len: 7,
        slow_len: 30,
        fast_ma_type: "ema",
        slow_ma_type: "ema",
      },
      objective: "Sharpe",
      radius: 1,
      step: 1,
      start_t: 1,
      points: [],
      summary: { base: 0, min: 0, median: 0, max: 0 },
    });

    await Promise.all([first, second, third]);

    expect(drillCalls).toBe(1);
    expect(barCalls).toBe(1);
    expect(stabilityCalls).toBe(1);
  });

  it("suppresses stale drill meta updates after delayed price-bar fetches", async () => {
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        data_id: "dataset-1",
        ma_source: "close",
        objective: "Sharpe",
        backend: { backend: "CpuOnly" },
        strategy: {},
        halftrend_filter: null,
      },
      res: {
        followup_cache_key: "cache-token-1",
        top: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 5,
              slow_ma_type: "ema",
              slow_len: 20,
            },
            { pnl: 0.1, sharpe: 0.5, max_dd: 0.2, trades: 10, exposure: 0.4 },
          ],
          [
            {
              fast_ma_type: "ema",
              fast_len: 7,
              slow_ma_type: "ema",
              slow_len: 30,
            },
            { pnl: 0.2, sharpe: 1.2, max_dd: 0.1, trades: 20, exposure: 0.5 },
          ],
        ],
      },
    });
    const store = useAnalysisStore(pinia);
    refreshTopTableFromWorkflowRun();
    const barsDeferred = createDeferred<unknown>();

    invokeCommandMock.mockImplementation(async (command: unknown, args?: unknown) => {
      if (command === "compute_double_ma_drilldown") {
        const req = (args as { req?: Record<string, unknown> })?.req ?? {};
        const fastLen = Number((req.params as Record<string, unknown>)?.fast_len ?? 0);
        return {
          equity: { start_t: 1, end_t: 2, bins: 1, minmax: [fastLen, fastLen] },
          drawdown: { start_t: 1, end_t: 2, bins: 1, minmax: [0, 0] },
          trades: [],
          trade_stats: null,
          indicator_bars: [],
        } as never;
      }
      if (command === "get_price_bars") {
        return await barsDeferred.promise;
      }
      if (command === "compute_double_ma_local_stability") {
        return {
          base: {
            fast_len: 7,
            slow_len: 30,
            fast_ma_type: "ema",
            slow_ma_type: "ema",
          },
          objective: "Sharpe",
          radius: 1,
          step: 1,
          start_t: 1,
          points: [],
          summary: { base: 0, min: 0, median: 0, max: 0 },
        } as never;
      }
      return undefined as never;
    });

    const first = selectTopRow(0);
    await Promise.resolve();
    const second = selectTopRow(1);
    await Promise.resolve();
    barsDeferred.resolve([]);
    await Promise.all([first, second]);

    expect(store.topTableRows[1]?.selected).toBe(true);
    expect(String(store.drillMeta || "")).toContain("ema(7) vs ema(30)");
    expect(String(store.drillMeta || "")).not.toContain("ema(5) vs ema(20)");
  });

  it("rerenders drill from cached state without recomputing the backend follow-up commands", async () => {
    mountDrillDom();
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        data_id: "dataset-1",
        ma_source: "close",
        objective: "Sharpe",
        backend: { backend: "CpuOnly" },
        strategy: {},
        halftrend_filter: null,
      },
      res: {
        followup_cache_key: "cache-token-1",
        top: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 7,
              slow_ma_type: "ema",
              slow_len: 30,
            },
            { pnl: 0.2, sharpe: 1.2, max_dd: 0.1, trades: 20, exposure: 0.5 },
          ],
        ],
      },
    });
    refreshTopTableFromWorkflowRun();
    invokeCommandMock.mockImplementation(async (command: unknown) => {
      if (command === "compute_double_ma_drilldown") {
        return {
          equity: { start_t: 1, end_t: 2, bins: 1, minmax: [1, 1] },
          drawdown: { start_t: 1, end_t: 2, bins: 1, minmax: [0, 0] },
          trades: [],
          trade_stats: null,
          indicator_bars: [],
        } as never;
      }
      if (command === "compute_double_ma_local_stability") {
        return {
          base: {
            fast_len: 7,
            slow_len: 30,
            fast_ma_type: "ema",
            slow_ma_type: "ema",
          },
          objective: "Sharpe",
          radius: 1,
          step: 1,
          start_t: 1,
          points: [],
          summary: { base: 0, min: 0, median: 0, max: 0 },
        } as never;
      }
      if (command === "get_price_bars") {
        return [] as never;
      }
      return undefined as never;
    });

    await selectTopRow(0);
    const callsAfterFirstRender = invokeCommandMock.mock.calls.length;

    await rerenderDrill();

    expect(invokeCommandMock.mock.calls.length).toBe(callsAfterFirstRender);
  });

  it("clears top filters in Vue store state", async () => {
    const store = useAnalysisStore(pinia);
    store.filterMinPnl = "1.0";
    store.filterMinSharpe = "0.5";
    store.filterMaxDd = "20";
    store.filterMinTrades = "50";
    store.filterMinExposure = "0.1";
    store.canClearTopFilters = true;

    await clearTopFilters();

    expect(store.filterMinPnl).toBe("");
    expect(store.filterMinSharpe).toBe("");
    expect(store.filterMaxDd).toBe("");
    expect(store.filterMinTrades).toBe("");
    expect(store.filterMinExposure).toBe("");
    expect(store.canClearTopFilters).toBe(false);
    expect(store.source).toBe("vue");
  });

  it("rebuilds top table rows from workflow run artifacts", () => {
    seedWorkflowRun({
      req: { strategy_kind: "DoubleMa", objective: "Sharpe" },
      res: {
        top: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 5,
              slow_ma_type: "ema",
              slow_len: 20,
            },
            { pnl: 0.3, sharpe: 1.8, max_dd: 0.1, trades: 40, exposure: 0.5 },
          ],
          [
            {
              fast_ma_type: "sma",
              fast_len: 7,
              slow_ma_type: "sma",
              slow_len: 21,
            },
            { pnl: 0.1, sharpe: 1.2, max_dd: 0.2, trades: 22, exposure: 0.4 },
          ],
        ],
      },
    });
    const store = useAnalysisStore(pinia);
    store.filterMinSharpe = "1.5";
    const ok = refreshTopTableFromWorkflowRun();
    expect(ok).toBe(true);
    expect(store.topRows).toBe(1);
    expect(String(store.topFilterMeta || "")).toBe("showing 1/2 rows");
    expect(Array.isArray(store.topColumns)).toBe(true);
    expect(Array.isArray(store.topTableRows)).toBe(true);
  });

  it("does not fabricate ranked rows when top_k is disabled for the run", () => {
    seedWorkflowRun({
      req: { strategy_kind: "DoubleMa", objective: "Sharpe", top_k: 0 },
      res: {
        num_combos: 136605,
        best_params: {
          fast_ma_type: "sma",
          fast_len: 8,
          slow_ma_type: "alma",
          slow_len: 20,
        },
        best_metrics: {
          pnl: 0.3,
          sharpe: 1.8,
          max_dd: 0.1,
          trades: 40,
          exposure: 0.5,
        },
        top: [],
      },
    });
    const store = useAnalysisStore(pinia);

    const ok = refreshTopTableFromWorkflowRun();

    expect(ok).toBe(true);
    expect(store.topRows).toBe(0);
    expect(String(store.topFilterMeta || "")).toContain("Top K disabled");
    expect(String(store.topSelectionGuardrail || "")).toContain(
      "Top K above 0",
    );
  });

  it("restores the visible heatmap without repainting the base grid", () => {
    mountHeatmapDom();
    useAnalysisStore(pinia);
    let heatmapCanvasFillRectCalls = 0;
    const fillRect = vi.fn();
    const overlayFillRect = vi.fn();
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
      function (this: HTMLCanvasElement) {
        const id = this.id;
        return ({
          clearRect: () => undefined,
          fillRect: (...args: unknown[]) => {
            if (id === "heatmapCanvasVue") {
              heatmapCanvasFillRectCalls += 1;
              fillRect(...args);
            } else if (id === "heatmapOverlayVue") {
              overlayFillRect(...args);
            }
          },
          strokeRect: () => undefined,
          beginPath: () => undefined,
          arc: () => undefined,
          fill: () => undefined,
          stroke: () => undefined,
          fillText: () => undefined,
          moveTo: () => undefined,
          lineTo: () => undefined,
          save: () => undefined,
          restore: () => undefined,
          translate: () => undefined,
          rotate: () => undefined,
          createLinearGradient: () => ({ addColorStop: () => undefined }),
        }) as never;
      },
    );
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        top_k: 20,
        heatmap_bins: 2,
        fast_ma_type: "ema",
        slow_ma_type: "ema",
      },
      res: {
        top: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 5,
              slow_ma_type: "ema",
              slow_len: 20,
            },
            { pnl: 0.1, sharpe: 0.5, max_dd: 0.2, trades: 10, exposure: 0.4 },
          ],
        ],
        heatmap: {
          bins_fast: 2,
          bins_slow: 2,
          fast_min: 5,
          fast_max: 9,
          slow_min: 20,
          slow_max: 30,
          values: [0.1, 0.2, -0.1, 0.05],
        },
      },
    });

    expect(refreshAnalysisFromWorkflowRun()).toBe(true);
    const basePaintCalls = heatmapCanvasFillRectCalls;

    expect(restoreHeatmapVisible()).toBe(true);
    expect(heatmapCanvasFillRectCalls).toBe(basePaintCalls);
  });

  it("marks heatmap as disabled when heatmap_bins is 0", () => {
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        top_k: 20,
        heatmap_bins: 0,
      },
      res: {
        best_params: {
          fast_ma_type: "sma",
          fast_len: 8,
          slow_ma_type: "alma",
          slow_len: 20,
        },
        best_metrics: {
          pnl: 0.3,
          sharpe: 1.8,
          max_dd: 0.1,
          trades: 40,
          exposure: 0.5,
        },
      },
    });
    const store = useAnalysisStore(pinia);

    const ok = refreshAnalysisFromWorkflowRun();

    expect(ok).toBe(true);
    expect(store.heatmapWrapVisible).toBe(false);
    expect(store.canHeatmapExportPng).toBe(false);
    expect(String(store.heatmapMeta || "")).toContain("heatmap_bins=0");
    expect(String(store.heatmapStatusLine || "")).toContain("Heatmap bins");
  });

  it("keeps MlSignal artifacts out of optimizer analysis views", () => {
    seedWorkflowRun({
      req: { strategy_kind: "MlSignal" },
      res: {
        matched_rows: 50,
        output_path: "C:/data/ml_score_cache.json",
      },
    });
    const store = useAnalysisStore(pinia);

    const ok = refreshAnalysisFromWorkflowRun();

    expect(ok).toBe(true);
    expect(String(store.bestTitle || "")).toContain("Score cache");
    expect(store.topRows).toBe(0);
    expect(Array.isArray(store.topTableRows)).toBe(true);
    expect(store.topTableRows.length).toBe(0);
    expect(String(store.topFilterMeta || "")).toContain("MlSignal");
  });

  it("renders pareto scatter into the Vue canvas host when mounted", () => {
    document.body.innerHTML = `
      <div id="paretoWrapVue" style="display:none"></div>
      <div id="paretoMetaVue">-</div>
      <canvas id="paretoCanvasVue" width="1024" height="240"></canvas>
    `;
    seedWorkflowRun({
      req: { strategy_kind: "DoubleMa", objective: "Sharpe", top_k: 20 },
      res: {
        top: [
          [
            {
              fast_ma_type: "sma",
              fast_len: 5,
              slow_ma_type: "alma",
              slow_len: 20,
            },
            { pnl: -0.1, sharpe: -0.2, max_dd: 0.4, trades: 10, exposure: 0.4 },
          ],
          [
            {
              fast_ma_type: "sma",
              fast_len: 8,
              slow_ma_type: "alma",
              slow_len: 24,
            },
            { pnl: 0.05, sharpe: 0.3, max_dd: 0.2, trades: 18, exposure: 0.5 },
          ],
        ],
      },
    });
    const store = useAnalysisStore(pinia);

    const ok = refreshParetoFromWorkflowRun();

    expect(ok).toBe(true);
    expect(store.paretoWrapVisible).toBe(true);
    expect(String(store.paretoMeta || "")).toContain("2 points");
    expect(
      (document.querySelector("#paretoWrapVue") as HTMLElement | null)?.style
        .display,
    ).toBe("block");
  });

  it("surfaces derived diagnostics when a DoubleMa score mode is active", () => {
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        score_mode: "active_sortino_10",
        top_k: 5,
      },
      res: {
        top: [
          [
            {
              fast_ma_type: "sma",
              fast_len: 5,
              slow_ma_type: "ema",
              slow_len: 20,
            },
            {
              pnl: 0.18,
              sharpe: 1.2,
              sortino: 1.9,
              downside_deviation: 0.1,
              calmar: 0.95,
              omega: 1.55,
              volatility: 0.16,
              ulcer_index: 0.05,
              martin_ratio: 2.1,
              max_dd: 0.11,
              trades: 22,
              exposure: 0.4,
              net_exposure: 0.03,
            },
          ],
        ],
        best_params: {
          fast_ma_type: "sma",
          fast_len: 5,
          slow_ma_type: "ema",
          slow_len: 20,
        },
        best_metrics: {
          pnl: 0.18,
          sharpe: 1.2,
          sortino: 1.9,
          downside_deviation: 0.1,
          calmar: 0.95,
          omega: 1.55,
          volatility: 0.16,
          ulcer_index: 0.05,
          martin_ratio: 2.1,
          max_dd: 0.11,
          trades: 22,
          exposure: 0.4,
          net_exposure: 0.03,
        },
      },
    });
    const store = useAnalysisStore(pinia);

    const ok = refreshAnalysisFromWorkflowRun();

    expect(ok).toBe(true);
    const labels = Array.isArray(store.topColumns)
      ? store.topColumns.map((col) =>
          String((col as { label?: unknown }).label || ""),
        )
      : [];
    expect(labels).toContain("ActMean");
    expect(labels).toContain("ActDnDev");
    expect(labels).toContain("ActSort");
    expect(labels).toContain("Chron");
    expect(labels).toContain("DnShare");
    expect(labels).toContain("LongExpo");
    expect(labels).toContain("ShortExpo");
    expect(labels).toContain("DirBias");
    expect(String(store.bestSummary || "")).toContain(
      "active_downside_deviation=",
    );
    expect(String(store.bestSummary || "")).toContain("active_sortino=");
    expect(String(store.bestSummary || "")).toContain("drawdown_chronicity=");
    expect(String(store.bestSummary || "")).toContain("directional_bias=");
  });

  it("surfaces the new Phase 5 score mode labels in the analysis table", () => {
    for (const [scoreMode, expectedLabel] of [
      ["effective_sample_lcb_10", "Score (ELCB10)"],
      ["omega_vs_sharpe_10", "Score (OVS10)"],
      ["robust_margin_10", "Score (RM10)"],
    ] as const) {
      seedWorkflowRun({
        req: {
          strategy_kind: "DoubleMa",
          objective: "Sharpe",
          score_mode: scoreMode,
          top_k: 5,
        },
        res: {
          ...(scoreMode === "effective_sample_lcb_10"
            ? {
                score_observation_context: {
                  first_valid: 0,
                  regime_first_valid: 0,
                  eval_start: 0,
                  eval_end: 200000,
                  total_bars: 200000,
                  trade_on_next_bar: false,
                },
              }
            : {}),
          top: [
            [
              {
                fast_ma_type: "sma",
                fast_len: 5,
                slow_ma_type: "ema",
                slow_len: 20,
              },
              {
                pnl: 0.18,
                sharpe: 1.2,
                sortino: 1.9,
                downside_deviation: 0.1,
                calmar: 0.95,
                omega: 1.55,
                volatility: 0.16,
                ulcer_index: 0.05,
                martin_ratio: 2.1,
                max_dd: 0.11,
                trades: 22,
                exposure: 0.4,
                net_exposure: 0.03,
              },
            ],
          ],
          best_params: {
            fast_ma_type: "sma",
            fast_len: 5,
            slow_ma_type: "ema",
            slow_len: 20,
          },
          best_metrics: {
            pnl: 0.18,
            sharpe: 1.2,
            sortino: 1.9,
            downside_deviation: 0.1,
            calmar: 0.95,
            omega: 1.55,
            volatility: 0.16,
            ulcer_index: 0.05,
            martin_ratio: 2.1,
            max_dd: 0.11,
            trades: 22,
            exposure: 0.4,
            net_exposure: 0.03,
          },
        },
      });
      const store = useAnalysisStore(pinia);

      const ok = refreshAnalysisFromWorkflowRun();

      expect(ok).toBe(true);
      const labels = Array.isArray(store.topColumns)
        ? store.topColumns.map((col) =>
            String((col as { label?: unknown }).label || ""),
          )
        : [];
      expect(labels).toContain(expectedLabel);
      expect(String(store.bestSummary || "")).toContain(
        `score_mode=${scoreMode}`,
      );
    }
  });

  it("surfaces shortlist confidence mode in the best meta and summary", () => {
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        shortlist_confidence_mode: "psr",
        top_k: 5,
      },
      res: {
        best_params: {
          fast_ma_type: "sma",
          fast_len: 5,
          slow_ma_type: "ema",
          slow_len: 20,
        },
        best_metrics: {
          pnl: 0.18,
          sharpe: 1.2,
          sortino: 1.9,
          downside_deviation: 0.1,
          calmar: 0.95,
          omega: 1.55,
          volatility: 0.16,
          ulcer_index: 0.05,
          martin_ratio: 2.1,
          max_dd: 0.11,
          trades: 22,
          exposure: 0.4,
          net_exposure: 0.03,
        },
      },
    });
    const store = useAnalysisStore(pinia);

    const ok = refreshAnalysisFromWorkflowRun();

    expect(ok).toBe(true);
    expect(String(store.bestMeta || "")).toContain("shortlist_confidence=psr");
    expect(String(store.bestSummary || "")).toContain(
      "shortlist_confidence_mode=psr",
    );
  });

  it("surfaces hurdle tau in the best meta and summary", () => {
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sortino",
        hurdle_tau: 0.01,
        top_k: 5,
      },
      res: {
        best_params: {
          fast_ma_type: "sma",
          fast_len: 5,
          slow_ma_type: "ema",
          slow_len: 20,
        },
        best_metrics: {
          pnl: 0.18,
          sharpe: 1.2,
          sortino: 1.9,
          downside_deviation: 0.1,
          calmar: 0.95,
          omega: 1.55,
          volatility: 0.16,
          ulcer_index: 0.05,
          martin_ratio: 2.1,
          max_dd: 0.11,
          trades: 22,
          exposure: 0.4,
          net_exposure: 0.03,
        },
      },
    });
    const store = useAnalysisStore(pinia);

    const ok = refreshAnalysisFromWorkflowRun();

    expect(ok).toBe(true);
    expect(String(store.bestMeta || "")).toContain("hurdle_tau=0.01");
    expect(String(store.bestSummary || "")).toContain("hurdle_tau=0.01");
  });

  it("surfaces nonzero kappa tau in the best stats and summary", () => {
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sortino",
        hurdle_tau: 0.01,
        top_k: 5,
      },
      res: {
        best_params: {
          fast_ma_type: "sma",
          fast_len: 5,
          slow_ma_type: "ema",
          slow_len: 20,
        },
        best_metrics: {
          pnl: 0.18,
          sharpe: 1.2,
          sortino: 1.9,
          downside_deviation: 0.1,
          calmar: 0.95,
          omega: 1.55,
          kappa_tau: 1.42,
          volatility: 0.16,
          ulcer_index: 0.05,
          martin_ratio: 2.1,
          max_dd: 0.11,
          trades: 22,
          exposure: 0.4,
          net_exposure: 0.03,
        },
      },
    });
    const store = useAnalysisStore(pinia);

    const ok = refreshAnalysisFromWorkflowRun();

    expect(ok).toBe(true);
    expect(Array.isArray(store.bestStats)).toBe(true);
    expect(
      (store.bestStats as Array<{ label?: string; value?: string }>).some(
        (entry) =>
          entry.label === "Kappaτ" &&
          String(entry.value || "").includes("1.420000"),
      ),
    ).toBe(true);
    expect(String(store.bestSummary || "")).toContain("kappa_tau=1.420000");
  });

  it("surfaces shortlist union mode in the best meta and summary", () => {
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        extra_shortlist_objectives: ["Pnl", "Omega"],
        shortlist_union_mode: "pareto",
        top_k: 5,
      },
      res: {
        best_params: {
          fast_ma_type: "sma",
          fast_len: 5,
          slow_ma_type: "ema",
          slow_len: 20,
        },
        best_metrics: {
          pnl: 0.18,
          sharpe: 1.2,
          sortino: 1.9,
          downside_deviation: 0.1,
          calmar: 0.95,
          omega: 1.55,
          volatility: 0.16,
          ulcer_index: 0.05,
          martin_ratio: 2.1,
          max_dd: 0.11,
          trades: 22,
          exposure: 0.4,
          net_exposure: 0.03,
        },
      },
    });
    const store = useAnalysisStore(pinia);

    const ok = refreshAnalysisFromWorkflowRun();

    expect(ok).toBe(true);
    expect(String(store.bestMeta || "")).toContain("shortlist_union=pareto");
    expect(String(store.bestSummary || "")).toContain(
      "shortlist_union_mode=pareto",
    );
  });

  it("surfaces shortlist slice diagnostics settings in the best meta and summary", () => {
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        extra_shortlist_objectives: ["Pnl", "Omega"],
        shortlist_slice_mode: "halves",
        shortlist_slice_benchmark: "buy_and_hold",
        shortlist_neighborhood_radius: 1,
        shortlist_neighborhood_step: 2,
        top_k: 5,
      },
      res: {
        best_params: {
          fast_ma_type: "sma",
          fast_len: 5,
          slow_ma_type: "ema",
          slow_len: 20,
        },
        best_metrics: {
          pnl: 0.18,
          sharpe: 1.2,
          sortino: 1.9,
          downside_deviation: 0.1,
          calmar: 0.95,
          omega: 1.55,
          volatility: 0.16,
          ulcer_index: 0.05,
          martin_ratio: 2.1,
          max_dd: 0.11,
          trades: 22,
          exposure: 0.4,
          net_exposure: 0.03,
        },
      },
    });
    const store = useAnalysisStore(pinia);

    const ok = refreshAnalysisFromWorkflowRun();

    expect(ok).toBe(true);
    expect(String(store.bestMeta || "")).toContain("shortlist_slice=halves");
    expect(String(store.bestMeta || "")).toContain("slice_neighborhood=1x2");
    expect(String(store.bestSummary || "")).toContain(
      "shortlist_slice_mode=halves",
    );
    expect(String(store.bestSummary || "")).toContain(
      "shortlist_slice_benchmark=buy_and_hold",
    );
    expect(String(store.bestSummary || "")).toContain(
      "shortlist_neighborhood=1x2",
    );
  });

  it("sets and clears baseline from Vue stores when run context is available", async () => {
    seedWorkflowRun({
      ts: "2026-02-26T12:01:00.000Z",
      artifactPath: "C:/runs/run_a.json",
      optimizationMetricSurface: {
        surface: "core17",
        fields: ["pnl", "trades", "max_dd"],
      },
      req: { objective: "Sharpe" },
      res: { top: [] },
    });
    const analysisStore = useAnalysisStore(pinia);
    const historyStore = useHistoryStore(pinia);

    await setBaseline();
    expect(historyStore.compareSelectedValue).toBe("disk:C:/runs/run_a.json");
    expect(String(analysisStore.statusLine || "")).toContain("Set baseline");
    expect(String(analysisStore.compareSummary || "")).toContain(
      "Optimization best:",
    );
    expect(String(analysisStore.compareSummary || "")).toContain(
      "metric surface: baseline=- | current=Core17",
    );
    expect(invokeCommandMock).toHaveBeenCalledWith("load_run_artifact", {
      path: "C:/runs/run_a.json",
    });

    analysisStore.compareSummary = "placeholder";
    await clearBaseline();
    expect(historyStore.compareSelectedValue).toBe("");
    expect(analysisStore.compareSummary).toBe("-");
  });

  it("is a no-op when Vue rows/columns are unavailable", async () => {
    const store = useAnalysisStore(pinia);
    store.topColumns = [];
    store.topTableRows = [];

    await sortTopTable("sharpe");
    await selectTopRow(2);

    expect(Array.isArray(store.topColumns)).toBe(true);
    expect(Array.isArray(store.topTableRows)).toBe(true);
  });

  it("rerenders drill by selecting current or first Vue top row", async () => {
    const store = useAnalysisStore(pinia);
    store.topTableRows = [
      { selected: false, cells: [{ text: "A", className: "", title: "" }] },
      { selected: true, cells: [{ text: "B", className: "", title: "" }] },
      { selected: false, cells: [{ text: "C", className: "", title: "" }] },
    ];

    await rerenderDrill();

    expect(store.topTableRows[0]?.selected).toBe(false);
    expect(store.topTableRows[1]?.selected).toBe(true);
    expect(store.topTableRows[2]?.selected).toBe(false);
  });

  it("keeps drill navigation non-throwing when DOM surface is unavailable", async () => {
    await drillNextTrade();
    await drillPrevTrade();
  });

  it("keeps drill selection clear non-throwing when DOM surface is unavailable", async () => {
    await drillClearTradeSelection();
    expect(true).toBe(true);
  });
});
