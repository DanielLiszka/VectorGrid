// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { invokeCommand } from "../tauri/invoke";
import { bindRuntimePinia } from "../stores/runtimePinia";
import {
  __analysisTestUtils,
  clearBaseline,
  clearTopFilters,
  drillClearTradeSelection,
  drillNextTrade,
  drillPrevTrade,
  invalidateAnalysisExecutionCaches,
  refreshAnalysisFromWorkflowRun,
  refreshHeatmapFromWorkflowRun,
  restoreHeatmapVisible,
  refreshParetoFromWorkflowRun,
  refreshTopTableFromWorkflowRun,
  rerenderDrill,
  runHeatmapRepresentativeDrilldown,
  selectTopRow,
  setBaseline,
  sortTopTable,
} from "./analysisService";
import { useAnalysisStore } from "../stores/analysisStore";
import { useHistoryStore } from "../stores/historyStore";
import { useWorkflowStore } from "../stores/workflowStore";
import { heatmapTooltipText } from "../visualizations/heatmapCanvasModel";

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

const createChartMock = vi.fn();
const addSeriesMock = vi.fn();
const addCandlestickSeriesMock = vi.fn();
const setSeriesDataMock = vi.fn();
const fitContentMock = vi.fn();
const removeChartMock = vi.fn();
const applyChartOptionsMock = vi.fn();
const subscribeClickMock = vi.fn();
const unsubscribeClickMock = vi.fn();
const createSeriesMarkersMock = vi.fn();
const setMarkersMock = vi.fn();
let lastChartClickHandler: ((param: unknown) => void) | null = null;

vi.mock("lightweight-charts", () => ({
  CandlestickSeries: {},
  LineSeries: {},
  HistogramSeries: {},
  createChart: (...args: unknown[]) => createChartMock(...args),
  createSeriesMarkers: (...args: unknown[]) => createSeriesMarkersMock(...args),
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
    <div id="drillTvChartVue"></div>
    <div id="drillTvChartMetaVue">-</div>
    <canvas id="equityCanvasVue" width="1024" height="160"></canvas>
    <canvas id="drawdownCanvasVue" width="1024" height="160"></canvas>
    <div id="tradeStatsWrapVue" style="display:none;"></div>
    <div id="tradeStatsVue">-</div>
    <table><tbody id="tradeTableBodyVue"></tbody></table>
    <div id="stabilityWrapVue" style="display:none;"></div>
    <div id="stabilityStatusVue">-</div>
    <div id="stabilityMetaVue">-</div>
    <div id="rejectMetaVue">-</div>
    <div id="stabilityTableWrapVue" style="display:none;"></div>
    <table><tbody id="stabilityTableBodyVue"></tbody></table>
  `;
}

function appendDrillDom(): void {
  document.body.insertAdjacentHTML(
    "beforeend",
    `
      <div id="drillEmptyHintVue"></div>
      <div id="drillWrapVue" style="display:none;"></div>
      <div id="drillMetaVue">-</div>
      <div id="drillStatusVue">-</div>
      <div id="drillTvChartVue"></div>
      <div id="drillTvChartMetaVue">-</div>
      <canvas id="equityCanvasVue" width="1024" height="160"></canvas>
      <canvas id="drawdownCanvasVue" width="1024" height="160"></canvas>
      <div id="tradeStatsWrapVue" style="display:none;"></div>
      <div id="tradeStatsVue">-</div>
      <table><tbody id="tradeTableBodyVue"></tbody></table>
      <div id="stabilityWrapVue" style="display:none;"></div>
      <div id="stabilityStatusVue">-</div>
      <div id="stabilityMetaVue">-</div>
      <div id="rejectMetaVue">-</div>
      <div id="stabilityTableWrapVue" style="display:none;"></div>
      <table><tbody id="stabilityTableBodyVue"></tbody></table>
    `,
  );
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
    lastChartClickHandler = null;
    setSeriesDataMock.mockReset();
    fitContentMock.mockReset();
    removeChartMock.mockReset();
    applyChartOptionsMock.mockReset();
    subscribeClickMock.mockReset();
    unsubscribeClickMock.mockReset();
    setMarkersMock.mockReset();
    addSeriesMock.mockReset();
    addCandlestickSeriesMock.mockReset();
    createChartMock.mockReset();
    createSeriesMarkersMock.mockReset();
    addSeriesMock.mockImplementation(() => ({
      setData: setSeriesDataMock,
    }));
    addCandlestickSeriesMock.mockImplementation(() => ({
      setData: setSeriesDataMock,
    }));
    subscribeClickMock.mockImplementation(
      (handler: (param: unknown) => void) => {
        lastChartClickHandler = handler;
      },
    );
    unsubscribeClickMock.mockImplementation(
      (handler: (param: unknown) => void) => {
        if (lastChartClickHandler === handler) lastChartClickHandler = null;
      },
    );
    createChartMock.mockImplementation(() => ({
      addSeries: addSeriesMock,
      addCandlestickSeries: addCandlestickSeriesMock,
      timeScale: () => ({ fitContent: fitContentMock }),
      applyOptions: applyChartOptionsMock,
      subscribeClick: subscribeClickMock,
      unsubscribeClick: unsubscribeClickMock,
      remove: removeChartMock,
    }));
    createSeriesMarkersMock.mockImplementation(() => ({
      setMarkers: setMarkersMock,
    }));
    vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
      cb(0);
      return 1;
    });
    vi.stubGlobal("cancelAnimationFrame", () => undefined);
    invalidateAnalysisExecutionCaches();
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

  it("builds drill trade markers from drilldown trades and publishes drill chart/table models", async () => {
    mountDrillDom();
    const drillCurves = {
      equity: { start_t: 1, end_t: 4, bins: 2, minmax: [1, 1.02, 1.01, 1.05] },
      drawdown: { start_t: 1, end_t: 4, bins: 2, minmax: [0, -0.01, -0.02, -0.01] },
      trades: [
        { entry_t: 1, exit_t: 2, direction: 1, pnl: 0.03, bars: 1, open: false },
        { entry_t: 3, exit_t: 4, direction: -1, pnl: -0.01, bars: 1, open: false },
      ],
      trade_stats: {
        closed_trades: 2,
        open_trades: 0,
        wins: 1,
        losses: 1,
        win_rate: 0.5,
        avg_win: 0.03,
        avg_loss: -0.01,
        profit_factor: 3,
        expectancy: 0.01,
      },
      indicator_bars: [
        { time: 1_744_500_000, from_index: 1, to_index: 1, fast: 100.2, slow: 100.0 },
        { time: 1_744_500_060, from_index: 2, to_index: 2, fast: 100.5, slow: 100.1 },
        { time: 1_744_500_120, from_index: 3, to_index: 3, fast: 100.3, slow: 100.2 },
        { time: 1_744_500_180, from_index: 4, to_index: 4, fast: 100.1, slow: 100.25 },
      ],
    };
    const drillBars = [
      { time: 1_744_500_000, open: 100, high: 101, low: 99.8, close: 100.6, fromIndex: 1, toIndex: 1, volume: 0 },
      { time: 1_744_500_060, open: 100.6, high: 101.2, low: 100.3, close: 101.0, fromIndex: 2, toIndex: 2, volume: 0 },
      { time: 1_744_500_120, open: 101.0, high: 101.1, low: 100.2, close: 100.4, fromIndex: 3, toIndex: 3, volume: 0 },
      { time: 1_744_500_180, open: 100.4, high: 100.8, low: 99.9, close: 100.0, fromIndex: 4, toIndex: 4, volume: 0 },
    ];
    const markers = __analysisTestUtils.buildDrillTradeMarkers(
      drillCurves,
      drillBars,
    );
    expect(markers.some((marker) => marker.id === "trade-0-entry" && marker.text === "L1")).toBe(true);
    expect(markers.some((marker) => marker.id === "trade-0-exit")).toBe(true);
    expect(markers.some((marker) => marker.id === "trade-1-entry" && marker.text === "S2")).toBe(true);

    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        data_id: "dataset-drill-markers",
        ma_source: "close",
        objective: "Sharpe",
        backend: { backend: "GpuKernel", device_id: 0 },
        strategy: {},
      },
      res: {
        top: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 7,
              slow_ma_type: "ema",
              slow_len: 30,
            },
            { pnl: 0.2, sharpe: 1.2, max_dd: 0.1, trades: 2, exposure: 0.5 },
          ],
        ],
      },
    });
    refreshTopTableFromWorkflowRun();
    invokeCommandMock.mockImplementation(async (command: unknown) => {
      if (command === "compute_double_ma_drilldown") {
        return drillCurves as never;
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
        return drillBars.map((bar) => ({
          time: bar.time,
          open: bar.open,
          high: bar.high,
          low: bar.low,
          close: bar.close,
          from_index: bar.fromIndex,
          to_index: bar.toIndex,
        })) as never;
      }
      return undefined as never;
    });
    const store = useAnalysisStore(pinia);

    await selectTopRow(0);

    expect(store.drillChartModel?.trades).toHaveLength(2);
    expect(store.drillTradeTableModel?.rows).toHaveLength(2);
    expect(store.drillTradeTableModel?.rows[0]?.selected).toBe(false);
    expect(__analysisTestUtils.selectDrillTradeByIndex(1)).toBe(false);
    const selectedMarkers = __analysisTestUtils.buildDrillTradeMarkers(
      drillCurves,
      drillBars,
      1,
    );
    expect(selectedMarkers.find((marker) => marker.id === "trade-1-entry")?.size).toBe(1.4);
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

  it("routes CompositeCpu drilldown and local stability through the composite command family", async () => {
    useAnalysisStore(pinia);
    seedWorkflowRun({
      req: {
        strategy_kind: "CompositeCpu",
        data_id: "dataset-composite",
        ma_source: "close",
        objective: "Sharpe",
        backend: { backend: "GpuKernel", device_id: 0 },
        signal_mode: "hard_gate",
        halftrend: { amplitude: 7, atr_period: 14, channel_deviation: 1.8 },
        macz: { fast_length: 11, slow_length: 28, signal_length: 8 },
        ott: { period: 5, percent: 1.4, ma_type: "EMA" },
        asr: { squeeze_length: 20 },
        strategy: { long_only: true, allow_flip: false },
      },
      res: {
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
      if (command === "compute_composite_cpu_drilldown") {
        return {
          equity: { start_t: 1, end_t: 2, bins: 1, minmax: [1, 1] },
          drawdown: { start_t: 1, end_t: 2, bins: 1, minmax: [0, 0] },
          trades: [],
          trade_stats: null,
          indicator_bars: [],
        } as never;
      }
      if (command === "compute_composite_cpu_local_stability") {
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
      if (command === "get_price_bars") return [] as never;
      return undefined as never;
    });

    await selectTopRow(0);

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "compute_composite_cpu_drilldown",
      expect.objectContaining({
        req: expect.objectContaining({
          backend_hint: { backend: "GpuKernel", device_id: 0 },
          signal_mode: "hard_gate",
          halftrend: { amplitude: 7, atr_period: 14, channel_deviation: 1.8 },
          macz: { fast_length: 11, slow_length: 28, signal_length: 8 },
          ott: { period: 5, percent: 1.4, ma_type: "EMA" },
          asr: { squeeze_length: 20 },
        }),
      }),
    );
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "compute_composite_cpu_local_stability",
      expect.objectContaining({
        req: expect.objectContaining({
          backend: { backend: "GpuKernel", device_id: 0 },
          signal_mode: "hard_gate",
          halftrend: { amplitude: 7, atr_period: 14, channel_deviation: 1.8 },
          macz: { fast_length: 11, slow_length: 28, signal_length: 8 },
          ott: { period: 5, percent: 1.4, ma_type: "EMA" },
          asr: { squeeze_length: 20 },
        }),
      }),
    );
  });

  it("routes CompositeMo follow-up analysis through row-scoped params instead of the base request", async () => {
    useAnalysisStore(pinia);
    seedWorkflowRun({
      req: {
        strategy_kind: "CompositeMo",
        data_id: "dataset-mo",
        ma_source: "close",
        objective: "Sharpe",
        backend: { backend: "CpuOnly" },
        base_strategy: { long_only: true, allow_flip: true },
        signal_mode: "hard_gate",
      },
      res: {
        pareto: [
          {
            params: {
              fast_ma_type: "ema",
              fast_len: 5,
              slow_ma_type: "sma",
              slow_len: 20,
              halftrend: { amplitude: 9, atr_period: 21, channel_deviation: 1.6 },
              macz: { fast_length: 12, slow_length: 26, signal_length: 9 },
              ott: { period: 6, percent: 1.3, ma_type: "EMA" },
              signal_mode: "entry_core_exit_overlay",
              strategy: { long_only: false, allow_flip: false },
            },
            metrics: {
              pnl: 0.25,
              sharpe: 1.05,
              max_dd: 0.12,
              trades: 14,
              exposure: 0.38,
            },
            maximize_value: 1.05,
            minimize_value: 0.12,
            violation: 0,
          },
        ],
      },
    });
    refreshTopTableFromWorkflowRun();
    invokeCommandMock.mockImplementation(async (command: unknown) => {
      if (command === "compute_composite_cpu_drilldown") {
        return {
          equity: { start_t: 1, end_t: 2, bins: 1, minmax: [1, 1] },
          drawdown: { start_t: 1, end_t: 2, bins: 1, minmax: [0, 0] },
          trades: [],
          trade_stats: null,
          indicator_bars: [],
        } as never;
      }
      if (command === "compute_composite_cpu_local_stability") {
        return {
          base: {
            fast_len: 5,
            slow_len: 20,
            fast_ma_type: "ema",
            slow_ma_type: "sma",
          },
          objective: "Sharpe",
          radius: 1,
          step: 1,
          start_t: 1,
          points: [],
          summary: { base: 0, min: 0, median: 0, max: 0 },
        } as never;
      }
      if (command === "get_price_bars") return [] as never;
      return undefined as never;
    });

    await selectTopRow(0);

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "compute_composite_cpu_drilldown",
      expect.objectContaining({
        req: expect.objectContaining({
          backend_hint: { backend: "CpuOnly" },
          signal_mode: "entry_core_exit_overlay",
          halftrend: { amplitude: 9, atr_period: 21, channel_deviation: 1.6 },
          macz: { fast_length: 12, slow_length: 26, signal_length: 9 },
          ott: { period: 6, percent: 1.3, ma_type: "EMA" },
          strategy: { long_only: false, allow_flip: false },
        }),
      }),
    );
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "compute_composite_cpu_local_stability",
      expect.objectContaining({
        req: expect.objectContaining({
          backend: { backend: "CpuOnly" },
          signal_mode: "entry_core_exit_overlay",
          halftrend: { amplitude: 9, atr_period: 21, channel_deviation: 1.6 },
          macz: { fast_length: 12, slow_length: 26, signal_length: 9 },
          ott: { period: 6, percent: 1.3, ma_type: "EMA" },
          strategy: { long_only: false, allow_flip: false },
        }),
      }),
    );
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

  it("publishes a component-owned heatmap model when the heatmap DOM is remounted", () => {
    mountHeatmapDom();
    const store = useAnalysisStore(pinia);
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
    mountHeatmapDom();
    expect(restoreHeatmapVisible()).toBe(true);
    expect(store.heatmapCanvasModel).toMatchObject({
      displayLabel: "Sharpe",
      request: {
        fastMaType: "ema",
        slowMaType: "ema",
      },
      markers: [
        {
          col: 0,
          drawRow: 1,
          label: "1",
        },
      ],
    });
  });

  it("keeps derived plateau hover scores stable under additive shifts of the full density-aware plateau surface", () => {
    mountHeatmapDom();
    const store = useAnalysisStore(pinia);
    store.heatmapMode = "plateau";
    const readPlateauTooltip = (
      values: number[],
      meanValues: number[],
    ): string => {
      seedWorkflowRun({
        req: {
          strategy_kind: "DoubleMa",
          objective: "Sharpe",
          top_k: 20,
          heatmap_bins: 3,
          fast_ma_type: "ema",
          slow_ma_type: "ema",
        },
        res: {
          top: [
            [
              {
                fast_ma_type: "ema",
                fast_len: 7,
                slow_ma_type: "ema",
                slow_len: 25,
              },
              { pnl: 0.2, sharpe: 1.1, max_dd: 0.12, trades: 18, exposure: 0.45 },
            ],
          ],
          heatmap: {
            bins_fast: 3,
            bins_slow: 3,
            fast_min: 5,
            fast_max: 9,
            slow_min: 20,
            slow_max: 30,
            values,
            mean_values: meanValues,
            sample_counts: [
              1, 1, 1,
              1, 4, 1,
              1, 1, 1,
            ],
          },
        },
      });

      expect(refreshHeatmapFromWorkflowRun()).toBe(true);
      expect(store.heatmapCanvasModel).toBeTruthy();
      return heatmapTooltipText(store.heatmapCanvasModel!, {
        col: 1,
        drawRow: 1,
        logicalRow: 1,
        idx: 4,
      }) ?? "";
    };

    const centeredTooltip = readPlateauTooltip(
      [
        1, 2, 1,
        2, 3, 2,
        1, 2, 1,
      ],
      [
        0.9, 1.8, 0.9,
        1.8, 2.6, 1.8,
        0.9, 1.8, 0.9,
      ],
    );
    const shiftedTooltip = readPlateauTooltip(
      [
        11, 12, 11,
        12, 13, 12,
        11, 12, 11,
      ],
      [
        10.9, 11.8, 10.9,
        11.8, 12.6, 11.8,
        10.9, 11.8, 10.9,
      ],
    );
    const centeredScore = centeredTooltip.match(/Plateau\s+\w+\s+([0-9.]+)/)?.[1];
    const shiftedScore = shiftedTooltip.match(/Plateau\s+\w+\s+([0-9.]+)/)?.[1];

    expect(centeredScore).toBeTruthy();
    expect(shiftedScore).toBeTruthy();
    expect(shiftedScore).toBe(centeredScore);
  });

  it("switches cached heatmaps into derived plateau mode and exposes plateau hover details", () => {
    mountHeatmapDom();
    const store = useAnalysisStore(pinia);
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        top_k: 20,
        heatmap_bins: 3,
        fast_ma_type: "ema",
        slow_ma_type: "ema",
      },
      res: {
        top: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 7,
              slow_ma_type: "ema",
              slow_len: 25,
            },
            { pnl: 0.2, sharpe: 1.1, max_dd: 0.12, trades: 18, exposure: 0.45 },
          ],
        ],
        heatmap: {
          bins_fast: 3,
          bins_slow: 3,
          fast_min: 5,
          fast_max: 9,
          slow_min: 20,
          slow_max: 30,
          values: [
            0.42, 0.61, 0.39,
            0.57, 0.86, 0.58,
            0.35, 0.54, 0.33,
          ],
          mean_values: [
            0.40, 0.58, 0.37,
            0.54, 0.74, 0.55,
            0.34, 0.52, 0.32,
          ],
          sample_counts: [
            1, 1, 1,
            1, 4, 1,
            1, 1, 1,
          ],
        },
        shortlist_slice_diagnostics: {
          ranking_mode: "score",
          neighborhood_robustness: {
            plateau_score: 0.8125,
            plateau_label: "Strong",
            neighbor_count: 8,
            radius: 2,
            step: 1,
          },
          candidates: [],
        },
      },
    });

    expect(refreshAnalysisFromWorkflowRun()).toBe(true);
    store.heatmapMode = "plateau";
    expect(restoreHeatmapVisible()).toBe(false);
    expect(refreshHeatmapFromWorkflowRun()).toBe(true);
    expect(String(store.heatmapMeta || "")).toContain("Derived plateau heatmap");
    expect(String(store.heatmapMeta || "")).toContain("supportive=8-neighbor agreement");
    expect(String(store.heatmapMeta || "")).toContain("density-aware=1 multi-sample cells");
    expect(String(store.heatmapStatusLine || "")).toContain(
      "exact winner robustness Strong 0.8125",
    );
    expect(String(store.heatmapStatusLine || "")).toContain(
      "representative parameter pair",
    );

    expect(store.heatmapCanvasModel).toBeTruthy();
    const tooltip = heatmapTooltipText(store.heatmapCanvasModel!, {
      col: 1,
      drawRow: 1,
      logicalRow: 1,
      idx: 4,
    }) ?? "";
    expect(tooltip).toContain("Plateau");
    expect(tooltip).toContain("Sharpe");
    expect(tooltip).toContain("supportive ");
    expect(tooltip).toContain("samples 4");
    expect(tooltip).toContain("mean 0.74");
    expect(tooltip).toContain("bin ");
    expect(store.heatmapCanvasModel?.range.min).toBe(0);
    expect(store.heatmapCanvasModel?.range.max).toBe(1);
  });

  it("clears the heatmap tooltip when the pointer moves from a populated cell onto an empty cell", () => {
    mountHeatmapDom();
    const store = useAnalysisStore(pinia);
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
        heatmap: {
          bins_fast: 2,
          bins_slow: 2,
          fast_min: 5,
          fast_max: 9,
          slow_min: 20,
          slow_max: 30,
          values: [0.1, null, -0.1, 0.05],
        },
      },
    });

    expect(refreshAnalysisFromWorkflowRun()).toBe(true);
    expect(store.heatmapWrapVisible).toBe(true);
    expect(store.heatmapCanvasModel).toBeTruthy();
    expect(heatmapTooltipText(store.heatmapCanvasModel!, {
      col: 0,
      drawRow: 1,
      logicalRow: 0,
      idx: 0,
    })).toContain("Sharpe");
    expect(heatmapTooltipText(store.heatmapCanvasModel!, {
      col: 1,
      drawRow: 1,
      logicalRow: 0,
      idx: 1,
    })).toBeNull();
  });

  it("keeps heatmap markers aligned with the visible filtered shortlist", () => {
    mountHeatmapDom();
    const store = useAnalysisStore(pinia);
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        top_k: 20,
        heatmap_bins: 3,
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
          [
            {
              fast_ma_type: "ema",
              fast_len: 7,
              slow_ma_type: "ema",
              slow_len: 25,
            },
            { pnl: 0.2, sharpe: 1.1, max_dd: 0.12, trades: 18, exposure: 0.45 },
          ],
          [
            {
              fast_ma_type: "ema",
              fast_len: 9,
              slow_ma_type: "ema",
              slow_len: 30,
            },
            { pnl: 0.18, sharpe: 0.8, max_dd: 0.15, trades: 16, exposure: 0.5 },
          ],
        ],
        heatmap: {
          bins_fast: 3,
          bins_slow: 3,
          fast_min: 5,
          fast_max: 9,
          slow_min: 20,
          slow_max: 30,
          values: [
            0.42, 0.61, 0.39,
            0.57, 0.86, 0.58,
            0.35, 0.54, 0.33,
          ],
        },
      },
    });

    store.filterMinSharpe = "0.75";
    expect(refreshHeatmapFromWorkflowRun()).toBe(true);

    const markers = store.heatmapCanvasModel?.markers ?? [];
    const marker2 = markers.find((marker) => marker.label === "2");
    const marker3 = markers.find((marker) => marker.label === "3");
    expect(marker2).toMatchObject({ col: 1, drawRow: 1 });
    expect(marker3).toMatchObject({ col: 2, drawRow: 0 });
    expect(markers.some((marker) => marker.label === "1")).toBe(false);
  });

  it("keeps the best-candidate marker available when top_k is disabled", () => {
    mountHeatmapDom();
    const store = useAnalysisStore(pinia);
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        top_k: 0,
        heatmap_bins: 2,
        fast_ma_type: "ema",
        slow_ma_type: "ema",
      },
      res: {
        best_params: {
          fast_ma_type: "ema",
          fast_len: 5,
          slow_ma_type: "ema",
          slow_len: 20,
        },
        best_metrics: {
          pnl: 0.12,
          sharpe: 0.9,
          max_dd: 0.18,
          trades: 9,
          exposure: 0.4,
        },
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

    expect(refreshHeatmapFromWorkflowRun()).toBe(true);
    expect(store.heatmapStatusLine).toContain("markers use the best candidate only.");
    expect(store.heatmapCanvasModel?.markers.map((marker) => marker.label)).toContain("1");
  });

  it("omits synthetic heatmap metrics from representative-bin drill headers", async () => {
    mountHeatmapDom();
    appendDrillDom();
    const workflowStore = useWorkflowStore(pinia);
    workflowStore.selectWorkflow("analyze", { persist: false });
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        data_id: "dataset-1",
        ma_source: "close",
        objective: "Sharpe",
        top_k: 20,
        heatmap_bins: 3,
        fast_ma_type: "ema",
        slow_ma_type: "ema",
      },
      res: {
        heatmap: {
          bins_fast: 3,
          bins_slow: 3,
          fast_min: 5,
          fast_max: 9,
          slow_min: 20,
          slow_max: 30,
          values: [
            0.42, 0.61, 0.39,
            0.57, 0.86, 0.58,
            0.35, 0.54, 0.33,
          ],
        },
      },
    });
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
            slow_len: 25,
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

    expect(refreshHeatmapFromWorkflowRun()).toBe(true);
    runHeatmapRepresentativeDrilldown({
      fast_len: 7,
      fast_ma_type: "ema",
      slow_len: 25,
      slow_ma_type: "ema",
      fast_ma_params: null,
      slow_ma_params: null,
    });

    expect(workflowStore.resultsTab).toBe("drill");
    await vi.waitFor(() => {
      const store = useAnalysisStore(pinia);
      expect(String(store.drillMeta || "")).toContain("ema(7) vs ema(25)");
      expect(String(store.drillMeta || "")).not.toContain("pnl=");
      expect(String(store.drillMeta || "")).not.toContain("sharpe=");
    });
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

  it("surfaces composite-family drilldown gaps explicitly instead of implying support", async () => {
    seedWorkflowRun({
      req: {
        strategy_kind: "Vcet",
        data_id: "dataset-vcet",
        ma_source: "close",
        objective: "Sharpe",
        backend: { backend: "GpuKernel", device_id: 0 },
        strategy: { fee_bps: 1.0 },
        vcet: {
          vidya_short_period: 8,
          vidya_long_period: 34,
          ce_period: 34,
          ce_mult: 2.8,
        },
      },
      res: {
        top: [
          [
            {
              fast_ma_type: "vidya",
              fast_len: 8,
              slow_ma_type: "vidya",
              slow_len: 34,
              vcet: {
                vidya_short_period: 8,
                vidya_long_period: 34,
                ce_period: 34,
                ce_mult: 2.8,
              },
            },
            { pnl: 0.2, sharpe: 1.1, max_dd: 0.15, trades: 14, exposure: 0.45 },
          ],
        ],
      },
    });
    const store = useAnalysisStore(pinia);
    refreshTopTableFromWorkflowRun();

    invokeCommandMock.mockImplementation(async (command: unknown) => {
      if (command === "compute_composite_cpu_drilldown") {
        throw new Error("VCET drilldown is not implemented yet.");
      }
      return undefined as never;
    });

    await selectTopRow(0);

    expect(String(store.drillStatus || "")).toContain(
      "VCET drilldown is not implemented yet.",
    );
    expect(String(store.statusLine || "")).toContain(
      "VCET drilldown is not implemented yet.",
    );
  });

  it("publishes a model-backed Pareto scatter for the Vue canvas host", () => {
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
    expect(store.paretoModel?.status).toBe("ready");
    expect(store.paretoModel?.points).toHaveLength(2);
    expect(store.paretoModel?.points[0]?.summary).toContain("#1");
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
      "metric surface: changed (- -> Core17)",
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
