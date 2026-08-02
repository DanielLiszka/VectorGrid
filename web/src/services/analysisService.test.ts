import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { invokeCommand } from "../tauri/invoke";
import { bindRuntimePinia } from "../stores/runtimePinia";
import {
  __analysisTestUtils,
  buildExecutionStrategyFromRunContext,
  clearBaseline,
  clearTopFilters,
  drillClearTradeSelection,
  drillNextTrade,
  drillPrevTrade,
  disposeDrillSupportCharts,
  invalidateAnalysisExecutionCaches,
  refreshAnalysisFromWorkflowRun,
  refreshHeatmapFromWorkflowRun,
  restoreHeatmapVisible,
  refreshParetoFromWorkflowRun,
  refreshTopTableFromWorkflowRun,
  rerenderDrill,
  runHeatmapRepresentativeCandidateDetail,
  selectTopRow,
  setBaseline,
  sortTopTable,
  exportTopCsv,
} from "./analysisService";
import { useAnalysisStore } from "../stores/analysisStore";
import { useDataStore } from "../stores/dataStore";
import { useHistoryStore } from "../stores/historyStore";
import { useWorkflowStore } from "../stores/workflowStore";
import {
  buildHeatmapCellParams,
  heatmapTooltipText,
} from "../visualizations/heatmapCanvasModel";

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
    <div id="equityChartStatusVue" hidden></div>
    <canvas id="drawdownCanvasVue" width="1024" height="160"></canvas>
    <div id="drawdownChartStatusVue" hidden></div>
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

function seedRerenderCandidateRun(dataId = "dataset-rerender"): void {
  seedWorkflowRun({
    req: {
      strategy_kind: "DoubleMa",
      data_id: dataId,
      ma_source: "close",
      objective: "Sharpe",
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
          {
            pnl: 0.2,
            sharpe: 1.2,
            max_dd: 0.1,
            trades: 20,
            exposure: 0.5,
          },
        ],
      ],
    },
  });
  refreshTopTableFromWorkflowRun();
  mountDrillDom();
}

function installRerenderDrillCommandResults(): void {
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
    if (command === "get_price_bars") return [] as never;
    return null as never;
  });
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
            scale: () => undefined,
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

  it("neutralizes spreadsheet formulas in exported Top CSV text columns", async () => {
    seedWorkflowRun({
      ts: "2026-02-26T12:00:00.000Z",
      req: { strategy_kind: "DoubleMa", objective: "Sharpe", top_k: 1 },
      res: {
        top: [
          [
            {
              fast_ma_type: "=cmd",
              fast_len: 5,
              slow_ma_type: "+slow",
              slow_len: 20,
            },
            {
              pnl: 0.2,
              sharpe: 1.1,
              max_dd: 0.1,
              trades: 12,
              exposure: 0.4,
            },
          ],
        ],
      },
    });

    let capturedBlob: Blob | null = null;
    const createObjectUrlMock = vi.fn((blob: Blob) => {
      capturedBlob = blob;
      return "blob:top-csv";
    });
    const revokeObjectUrlMock = vi.fn();
    type MutableUrlObjectUrl = {
      createObjectURL?: (blob: Blob | MediaSource) => string;
      revokeObjectURL?: (url: string) => void;
    };
    const urlObjectUrl = URL as unknown as MutableUrlObjectUrl;
    const originalCreateObjectURL = urlObjectUrl.createObjectURL;
    const originalRevokeObjectURL = urlObjectUrl.revokeObjectURL;
    const hasOwn = Object.prototype.hasOwnProperty;
    const hadCreateObjectURL = hasOwn.call(URL, "createObjectURL");
    const hadRevokeObjectURL = hasOwn.call(URL, "revokeObjectURL");
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      value: createObjectUrlMock,
    });
    Object.defineProperty(URL, "revokeObjectURL", {
      configurable: true,
      value: revokeObjectUrlMock,
    });
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(
      () => undefined,
    );

    try {
      await exportTopCsv();
      expect(capturedBlob).not.toBeNull();
      const csv = await capturedBlob!.text();
      expect(csv).toContain("1,'=cmd,5,'+slow,20");
      expect(csv).not.toContain(",=cmd,");
      expect(csv).not.toContain(",+slow,");
      expect(createObjectUrlMock).toHaveBeenCalledOnce();
      expect(revokeObjectUrlMock).toHaveBeenCalledWith("blob:top-csv");
    } finally {
      if (hadCreateObjectURL) {
        Object.defineProperty(URL, "createObjectURL", {
          configurable: true,
          value: originalCreateObjectURL,
        });
      } else {
        delete urlObjectUrl.createObjectURL;
      }
      if (hadRevokeObjectURL) {
        Object.defineProperty(URL, "revokeObjectURL", {
          configurable: true,
          value: originalRevokeObjectURL,
        });
      } else {
        delete urlObjectUrl.revokeObjectURL;
      }
    }
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

  it("invalidates cached top-row score sort when the run objective mutates in place", async () => {
    const req: Record<string, unknown> = {
      strategy_kind: "DoubleMa",
      objective: "Sharpe",
    };
    const res = {
      top: [
        [
          {
            fast_ma_type: "ema",
            fast_len: 5,
            slow_ma_type: "ema",
            slow_len: 20,
          },
          { pnl: 10, sharpe: 0.2, max_dd: 0.2, trades: 10, exposure: 0.4 },
        ],
        [
          {
            fast_ma_type: "ema",
            fast_len: 7,
            slow_ma_type: "ema",
            slow_len: 30,
          },
          { pnl: 1, sharpe: 2.0, max_dd: 0.1, trades: 20, exposure: 0.5 },
        ],
      ],
    };
    seedWorkflowRun({ req, res });
    const store = useAnalysisStore(pinia);

    refreshTopTableFromWorkflowRun();
    await sortTopTable("score");

    expect(store.topTableRows[0]?.cells[1]?.text).toBe("ema(7)");

    req.objective = "Pnl";
    refreshTopTableFromWorkflowRun();

    expect(store.topTableRows[0]?.cells[1]?.text).toBe("ema(5)");
  });

  it("computes candidate detail/stability via Vue command path on top-row selection", async () => {
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

  it("preserves DynamicDoubleMa identity and IR in Candidate Detail without fabricating static stability", async () => {
    const dynamicStrategyIr = {
      schema_version: 1,
      profile: "double_ma",
      ma_source: "hlc3",
      fast_ma: { ma_type: "ema", params: {} },
      slow_ma: { ma_type: "sma", params: {} },
      entry: { op: "cross_over", left: "fast_ma", right: "slow_ma" },
    };
    const params = {
      fast_ma_type: "ema",
      fast_len: 7,
      slow_ma_type: "sma",
      slow_len: 30,
    };
    const req = {
      strategy_kind: "DynamicDoubleMa",
      data_id: "dataset-dynamic",
      ma_source: "hlc3",
      objective: "Sharpe",
      backend: { backend: "GpuKernel", device_id: 0 },
      strategy: {},
      dynamic_strategy_ir: dynamicStrategyIr,
    };

    expect(buildExecutionStrategyFromRunContext(params, req)).toMatchObject({
      strategyKind: "DynamicDoubleMa",
      strategy: {
        kind: "double_ma",
        req: {
          data_id: "dataset-dynamic",
          dynamic_strategy_ir: dynamicStrategyIr,
        },
      },
    });
    expect(
      buildExecutionStrategyFromRunContext(params, {
        ...req,
        dynamic_strategy_ir: null,
      }),
    ).toBeNull();

    mountDrillDom();
    seedWorkflowRun({
      req,
      res: {
        top: [
          [
            params,
            {
              pnl: 0.2,
              sharpe: 1.2,
              max_dd: 0.1,
              trades: 20,
              exposure: 0.5,
            },
          ],
        ],
      },
    });
    const store = useAnalysisStore(pinia);
    useDataStore(pinia).dataId = "dataset-dynamic";
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
      if (command === "get_price_bars") return [] as never;
      return null as never;
    });

    await selectTopRow(0);

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "compute_double_ma_drilldown",
      expect.objectContaining({
        req: expect.objectContaining({
          dynamic_strategy_ir: dynamicStrategyIr,
        }),
      }),
    );
    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === "compute_double_ma_local_stability",
      ),
    ).toBe(false);
    expect(store.stabilityWrapVisible).toBe(true);
    expect(store.stabilityStatus).toContain(
      "Local stability unavailable for Dynamic Double MA",
    );

    await rerenderDrill();

    expect(store.stabilityWrapVisible).toBe(true);
    expect(store.stabilityStatus).toContain(
      "dynamic-IR neighborhood replay is not supported yet",
    );
  });

  it("preserves static DoubleMa and Orc identities without adding dynamic IR to Candidate Detail", () => {
    const params = {
      fast_ma_type: "ema",
      fast_len: 7,
      slow_ma_type: "sma",
      slow_len: 30,
    };

    for (const strategyKind of ["DoubleMa", "Orc"]) {
      const snapshot = buildExecutionStrategyFromRunContext(params, {
        strategy_kind: strategyKind,
        data_id: `dataset-${strategyKind.toLowerCase()}`,
        ma_source: "close",
        strategy: {},
        dynamic_strategy_ir: null,
      });

      expect(snapshot?.strategyKind).toBe(strategyKind);
      expect(snapshot?.strategy.kind).toBe("double_ma");
      expect(snapshot?.strategy.req).not.toHaveProperty("dynamic_strategy_ir");
    }
  });

  it("paints continuous support-chart bounds when every min/max bin is degenerate", () => {
    const canvas = document.createElement("canvas");
    const strokes: Array<Array<{ x: number; y: number }>> = [];
    let currentPath: Array<{ x: number; y: number }> = [];
    Object.defineProperty(canvas, "getContext", {
      value: () =>
        ({
          clearRect: vi.fn(),
          beginPath: () => {
            currentPath = [];
          },
          moveTo: (x: number, y: number) => {
            currentPath.push({ x, y });
          },
          lineTo: (x: number, y: number) => {
            currentPath.push({ x, y });
          },
          stroke: () => {
            strokes.push([...currentPath]);
          },
        }) as unknown as CanvasRenderingContext2D,
    });

    __analysisTestUtils.drawMinMaxChartDom(
      canvas,
      [1, 1, 1.04, 1.04, 0.99, 0.99, 1.08, 1.08],
      { stroke: "#00aa77", height: 160 },
    );

    expect(
      strokes.some(
        (path) =>
          new Set(path.map((point) => point.x)).size >= 4 &&
          new Set(path.map((point) => point.y)).size >= 3,
      ),
    ).toBe(true);

    strokes.length = 0;
    __analysisTestUtils.drawMinMaxChartDom(canvas, [1, 1], {
      stroke: "#00aa77",
      height: 160,
    });
    expect(
      strokes.some(
        (path) =>
          path.length >= 2 &&
          path.some(
            (point, index) =>
              index > 0 &&
              (point.x !== path[0]?.x || point.y !== path[0]?.y),
        ),
      ),
    ).toBe(true);

    strokes.length = 0;
    __analysisTestUtils.drawMinMaxChartDom(
      canvas,
      [1, 1, Number.NaN, Number.NaN, 1.08, 1.08],
      { stroke: "#00aa77", height: 160 },
    );
    const logicalMidpoint =
      Number(canvas.dataset.logicalWidth || 300) / 2;
    expect(
      strokes.some((path) =>
        path.some((point) => point.x < logicalMidpoint),
      ),
    ).toBe(true);
    expect(
      strokes.some((path) =>
        path.some((point) => point.x > logicalMidpoint),
      ),
    ).toBe(true);
    expect(
      strokes.every((path) => {
        const hasLeft = path.some((point) => point.x < logicalMidpoint);
        const hasRight = path.some((point) => point.x > logicalMidpoint);
        return !(hasLeft && hasRight);
      }),
    ).toBe(true);
  });

  it("clears and unregisters a previously valid support chart when its envelope becomes empty or malformed", async () => {
    const canvas = document.createElement("canvas");
    document.body.appendChild(canvas);
    const clearRect = vi.fn();
    const strokes: string[] = [];
    const context = {
      strokeStyle: "",
      clearRect,
      setTransform: vi.fn(),
      beginPath: () => undefined,
      moveTo: () => undefined,
      lineTo: () => undefined,
      stroke() {
        strokes.push(String(this.strokeStyle));
      },
    };
    Object.defineProperty(canvas, "getContext", {
      value: () => context as unknown as CanvasRenderingContext2D,
    });
    const originalTheme = document.documentElement.dataset.theme;
    document.documentElement.dataset.theme = "light";

    try {
      __analysisTestUtils.drawMinMaxChartDom(canvas, [1, 1, 1.05, 1.05], {
        tone: "positive",
        baseline: 1,
        height: 160,
      });
      expect(strokes.length).toBeGreaterThan(0);
      expect(canvas.dataset.supportChartStroke).toBe("#047857");

      __analysisTestUtils.drawMinMaxChartDom(canvas, [], {
        tone: "positive",
        baseline: 1,
        height: 160,
      });
      expect(clearRect).toHaveBeenLastCalledWith(
        0,
        0,
        canvas.width,
        canvas.height,
      );
      expect(canvas.dataset.supportChartStroke).toBeUndefined();
      expect(canvas.dataset.supportChartTheme).toBeUndefined();

      __analysisTestUtils.drawMinMaxChartDom(canvas, [1, 1.05], {
        tone: "positive",
        baseline: 1,
        height: 160,
      });
      expect(canvas.dataset.supportChartStroke).toBe("#047857");
      __analysisTestUtils.drawMinMaxChartDom(canvas, [1, "not-a-number"], {
        tone: "positive",
        baseline: 1,
        height: 160,
      });
      expect(canvas.dataset.supportChartStroke).toBeUndefined();
      expect(canvas.dataset.supportChartTheme).toBeUndefined();

      const strokeCountAfterClear = strokes.length;
      document.documentElement.dataset.theme = "dark";
      await Promise.resolve();
      await Promise.resolve();
      expect(strokes).toHaveLength(strokeCountAfterClear);
    } finally {
      disposeDrillSupportCharts();
      if (originalTheme == null) {
        delete document.documentElement.dataset.theme;
      } else {
        document.documentElement.dataset.theme = originalTheme;
      }
    }
  });

  it("clears all-nonfinite support-chart envelopes without painting a baseline", () => {
    const canvas = document.createElement("canvas");
    document.body.appendChild(canvas);
    const clearRect = vi.fn();
    const stroke = vi.fn();
    Object.defineProperty(canvas, "getContext", {
      value: () =>
        ({
          clearRect,
          setTransform: vi.fn(),
          beginPath: vi.fn(),
          moveTo: vi.fn(),
          lineTo: vi.fn(),
          stroke,
        }) as unknown as CanvasRenderingContext2D,
    });

    __analysisTestUtils.drawMinMaxChartDom(
      canvas,
      [Number.NaN, Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY],
      { tone: "negative", baseline: 0, height: 160 },
    );

    expect(clearRect).toHaveBeenCalledWith(
      0,
      0,
      canvas.width,
      canvas.height,
    );
    expect(stroke).not.toHaveBeenCalled();
    expect(canvas.dataset.supportChartStroke).toBeUndefined();
    expect(canvas.dataset.supportChartTheme).toBeUndefined();
  });

  it("sizes support-chart backing pixels to CSS geometry and redraws across resize", () => {
    const canvas = document.createElement("canvas");
    canvas.width = 1024;
    canvas.height = 160;
    document.body.appendChild(canvas);
    let cssWidth = 640;
    const paths: Array<Array<{ x: number; y: number }>> = [];
    let currentPath: Array<{ x: number; y: number }> = [];
    const clearRect = vi.fn();
    const setTransform = vi.fn();
    Object.defineProperty(canvas, "clientWidth", {
      configurable: true,
      get: () => cssWidth,
    });
    Object.defineProperty(canvas, "clientHeight", {
      configurable: true,
      get: () => 160,
    });
    Object.defineProperty(canvas, "getBoundingClientRect", {
      value: () => ({
        x: 0,
        y: 0,
        top: 0,
        right: cssWidth + 2,
        bottom: 162,
        left: 0,
        width: cssWidth + 2,
        height: 162,
        toJSON: () => ({}),
      }),
    });
    Object.defineProperty(canvas, "getContext", {
      value: () =>
        ({
          clearRect,
          setTransform,
          beginPath: () => {
            currentPath = [];
          },
          moveTo: (x: number, y: number) => {
            currentPath.push({ x, y });
          },
          lineTo: (x: number, y: number) => {
            currentPath.push({ x, y });
          },
          stroke: () => {
            paths.push([...currentPath]);
          },
        }) as unknown as CanvasRenderingContext2D,
    });

    class TestResizeObserver implements ResizeObserver {
      static callback: ResizeObserverCallback = () => undefined;
      static instance: TestResizeObserver | null = null;

      readonly observed: Element[] = [];
      readonly unobserved: Element[] = [];
      disconnectCount = 0;

      constructor(callback: ResizeObserverCallback) {
        TestResizeObserver.callback = callback;
        TestResizeObserver.instance = this;
      }

      observe(target: Element): void {
        this.observed.push(target);
      }

      unobserve(target: Element): void {
        this.unobserved.push(target);
      }

      disconnect(): void {
        this.disconnectCount += 1;
      }
    }
    class TestMutationObserver implements MutationObserver {
      static instance: TestMutationObserver | null = null;

      disconnectCount = 0;

      constructor(_callback: MutationCallback) {
        TestMutationObserver.instance = this;
      }

      observe(): void {}

      takeRecords(): MutationRecord[] {
        return [];
      }

      disconnect(): void {
        this.disconnectCount += 1;
      }
    }
    const resizeObserverDescriptor = Object.getOwnPropertyDescriptor(
      globalThis,
      "ResizeObserver",
    );
    const mutationObserverDescriptor = Object.getOwnPropertyDescriptor(
      globalThis,
      "MutationObserver",
    );
    const devicePixelRatioDescriptor = Object.getOwnPropertyDescriptor(
      window,
      "devicePixelRatio",
    );
    Object.defineProperty(globalThis, "ResizeObserver", {
      configurable: true,
      value: TestResizeObserver,
    });
    Object.defineProperty(globalThis, "MutationObserver", {
      configurable: true,
      value: TestMutationObserver,
    });
    Object.defineProperty(window, "devicePixelRatio", {
      configurable: true,
      value: 2,
    });

    try {
      __analysisTestUtils.drawMinMaxChartDom(
        canvas,
        [1, 1, 1.04, 1.04, 0.99, 0.99, 1.08, 1.08],
        { stroke: "#00aa77", baseline: 1, height: 160 },
      );

      expect(canvas.width).toBe(1280);
      expect(canvas.height).toBe(320);
      expect(canvas.dataset.pixelRatio).toBe("2");
      expect(canvas.dataset.logicalWidth).toBe("640");
      expect(canvas.dataset.logicalHeight).toBe("160");
      expect(setTransform).toHaveBeenCalledWith(2, 0, 0, 2, 0, 0);
      expect(clearRect).toHaveBeenCalledWith(0, 0, 640, 160);
      expect(
        paths.some((path) => {
          const xs = path.map((point) => point.x);
          return (
            path.length >= 4 &&
            Math.min(...xs) <= 2 &&
            Math.max(...xs) >= 638
          );
        }),
      ).toBe(true);

      cssWidth = 420;
      TestResizeObserver.callback(
        [{ target: canvas } as unknown as ResizeObserverEntry],
        TestResizeObserver.instance as unknown as ResizeObserver,
      );

      expect(canvas.width).toBe(840);
      expect(canvas.height).toBe(320);
      expect(canvas.dataset.logicalWidth).toBe("420");
      expect(clearRect).toHaveBeenLastCalledWith(0, 0, 420, 160);
      expect(TestResizeObserver.instance?.observed).toEqual([canvas]);
      const renderCountAfterResize = clearRect.mock.calls.length;
      TestResizeObserver.callback(
        [{ target: canvas } as unknown as ResizeObserverEntry],
        TestResizeObserver.instance as unknown as ResizeObserver,
      );
      expect(clearRect).toHaveBeenCalledTimes(renderCountAfterResize);

      canvas.id = "equityCanvasVue";
      disposeDrillSupportCharts();
      expect(TestResizeObserver.instance?.unobserved).toEqual([canvas]);
      expect(TestResizeObserver.instance?.disconnectCount).toBe(1);
      expect(TestMutationObserver.instance?.disconnectCount).toBe(1);
    } finally {
      if (resizeObserverDescriptor) {
        Object.defineProperty(
          globalThis,
          "ResizeObserver",
          resizeObserverDescriptor,
        );
      } else {
        Reflect.deleteProperty(globalThis, "ResizeObserver");
      }
      if (mutationObserverDescriptor) {
        Object.defineProperty(
          globalThis,
          "MutationObserver",
          mutationObserverDescriptor,
        );
      } else {
        Reflect.deleteProperty(globalThis, "MutationObserver");
      }
      if (devicePixelRatioDescriptor) {
        Object.defineProperty(
          window,
          "devicePixelRatio",
          devicePixelRatioDescriptor,
        );
      }
    }
  });

  it("redraws support charts when only the device pixel ratio changes", () => {
    const canvas = document.createElement("canvas");
    document.body.appendChild(canvas);
    Object.defineProperty(canvas, "clientWidth", {
      configurable: true,
      value: 300,
    });
    Object.defineProperty(canvas, "clientHeight", {
      configurable: true,
      value: 160,
    });
    const clearRect = vi.fn();
    Object.defineProperty(canvas, "getContext", {
      value: () =>
        ({
          clearRect,
          setTransform: vi.fn(),
          beginPath: vi.fn(),
          moveTo: vi.fn(),
          lineTo: vi.fn(),
          stroke: vi.fn(),
        }) as unknown as CanvasRenderingContext2D,
    });

    let pixelRatio = 1;
    const mediaQueries: Array<{
      media: string;
      listener: EventListener | null;
      removed: boolean;
    }> = [];
    const matchMedia = vi.fn((media: string): MediaQueryList => {
      const record = {
        media,
        listener: null as EventListener | null,
        removed: false,
      };
      mediaQueries.push(record);
      return {
        media,
        matches: true,
        onchange: null,
        addEventListener: (_type: string, listener: EventListener) => {
          record.listener = listener;
        },
        removeEventListener: (_type: string, listener: EventListener) => {
          if (record.listener === listener) record.removed = true;
        },
        addListener: (listener: EventListener) => {
          record.listener = listener;
        },
        removeListener: (listener: EventListener) => {
          if (record.listener === listener) record.removed = true;
        },
        dispatchEvent: () => true,
      } as MediaQueryList;
    });
    const matchMediaDescriptor = Object.getOwnPropertyDescriptor(
      window,
      "matchMedia",
    );
    const devicePixelRatioDescriptor = Object.getOwnPropertyDescriptor(
      window,
      "devicePixelRatio",
    );
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      value: matchMedia,
    });
    Object.defineProperty(window, "devicePixelRatio", {
      configurable: true,
      get: () => pixelRatio,
    });

    try {
      __analysisTestUtils.drawMinMaxChartDom(
        canvas,
        [1, 1, 1.05, 1.05],
        { tone: "positive", baseline: 1, height: 160 },
      );
      expect(canvas.width).toBe(300);
      expect(canvas.height).toBe(160);
      expect(canvas.dataset.pixelRatio).toBe("1");
      expect(mediaQueries[0]?.media).toBe("(resolution: 1dppx)");
      const renderCountAtOneX = clearRect.mock.calls.length;

      pixelRatio = 2;
      mediaQueries[0]?.listener?.(new Event("change"));

      expect(canvas.width).toBe(600);
      expect(canvas.height).toBe(320);
      expect(canvas.dataset.pixelRatio).toBe("2");
      expect(clearRect).toHaveBeenCalledTimes(renderCountAtOneX + 1);
      expect(mediaQueries[0]?.removed).toBe(true);
      expect(mediaQueries[1]?.media).toBe("(resolution: 2dppx)");

      disposeDrillSupportCharts();
      expect(mediaQueries[1]?.removed).toBe(true);
    } finally {
      disposeDrillSupportCharts();
      if (matchMediaDescriptor) {
        Object.defineProperty(window, "matchMedia", matchMediaDescriptor);
      } else {
        Reflect.deleteProperty(window, "matchMedia");
      }
      if (devicePixelRatioDescriptor) {
        Object.defineProperty(
          window,
          "devicePixelRatio",
          devicePixelRatioDescriptor,
        );
      }
    }
  });

  it("tears down observers for detached support canvases instead of rediscovering replacement IDs", () => {
    class TestResizeObserver implements ResizeObserver {
      static instance: TestResizeObserver | null = null;

      readonly observed: Element[] = [];
      readonly unobserved: Element[] = [];
      disconnectCount = 0;

      constructor(_callback: ResizeObserverCallback) {
        TestResizeObserver.instance = this;
      }

      observe(target: Element): void {
        this.observed.push(target);
      }

      unobserve(target: Element): void {
        this.unobserved.push(target);
      }

      disconnect(): void {
        this.disconnectCount += 1;
      }
    }
    const resizeObserverDescriptor = Object.getOwnPropertyDescriptor(
      globalThis,
      "ResizeObserver",
    );
    Object.defineProperty(globalThis, "ResizeObserver", {
      configurable: true,
      value: TestResizeObserver,
    });
    const original = document.createElement("canvas");
    original.id = "equityCanvasVue";
    document.body.appendChild(original);

    try {
      __analysisTestUtils.drawMinMaxChartDom(original, [1, 1, 1.05, 1.05], {
        tone: "positive",
        baseline: 1,
        height: 160,
      });
      expect(TestResizeObserver.instance?.observed).toEqual([original]);

      original.remove();
      const replacement = document.createElement("canvas");
      replacement.id = "equityCanvasVue";
      document.body.appendChild(replacement);
      disposeDrillSupportCharts();

      expect(TestResizeObserver.instance?.unobserved).toEqual([original]);
      expect(TestResizeObserver.instance?.disconnectCount).toBe(1);
    } finally {
      disposeDrillSupportCharts();
      if (resizeObserverDescriptor) {
        Object.defineProperty(
          globalThis,
          "ResizeObserver",
          resizeObserverDescriptor,
        );
      } else {
        Reflect.deleteProperty(globalThis, "ResizeObserver");
      }
    }
  });

  it("repaints active support charts with the resolved light and dark series palette", async () => {
    const originalTheme = document.documentElement.dataset.theme;
    document.documentElement.dataset.theme = "light";
    const equity = document.createElement("canvas");
    equity.id = "equityCanvasVue";
    const drawdown = document.createElement("canvas");
    drawdown.id = "drawdownCanvasVue";
    document.body.append(equity, drawdown);
    const equityStrokes: string[] = [];
    const drawdownStrokes: string[] = [];
    const installContext = (
      canvas: HTMLCanvasElement,
      strokes: string[],
    ): void => {
      const context = {
        strokeStyle: "",
        clearRect: () => undefined,
        setTransform: () => undefined,
        beginPath: () => undefined,
        moveTo: () => undefined,
        lineTo: () => undefined,
        stroke() {
          strokes.push(String(this.strokeStyle));
        },
      };
      Object.defineProperty(canvas, "getContext", {
        value: () => context as unknown as CanvasRenderingContext2D,
      });
    };
    installContext(equity, equityStrokes);
    installContext(drawdown, drawdownStrokes);

    try {
      __analysisTestUtils.drawMinMaxChartDom(
        equity,
        [1, 1, 1.04, 1.04],
        { tone: "positive", baseline: 1, height: 160 },
      );
      __analysisTestUtils.drawMinMaxChartDom(
        drawdown,
        [0, 0, -0.04, -0.04],
        { tone: "negative", baseline: 0, height: 160 },
      );

      expect(equity.dataset.supportChartStroke).toBe("#047857");
      expect(drawdown.dataset.supportChartStroke).toBe("#be123c");
      expect(equityStrokes).toContain("#047857");
      expect(drawdownStrokes).toContain("#be123c");

      document.documentElement.dataset.theme = "dark";
      await vi.waitFor(() => {
        expect(equity.dataset.supportChartStroke).toBe("#22c55e");
        expect(drawdown.dataset.supportChartStroke).toBe("#fb7185");
      });
      expect(equityStrokes).toContain("#22c55e");
      expect(drawdownStrokes).toContain("#fb7185");
    } finally {
      disposeDrillSupportCharts();
      if (originalTheme == null) {
        delete document.documentElement.dataset.theme;
      } else {
        document.documentElement.dataset.theme = originalTheme;
      }
    }
  });

  it("keeps valid candidate evidence visible when only the drawdown envelope is invalid", async () => {
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        data_id: "dataset-invalid-drawdown",
        ma_source: "close",
        objective: "Sharpe",
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
            {
              pnl: 0.2,
              sharpe: 1.2,
              max_dd: 0.1,
              trades: 20,
              exposure: 0.5,
            },
          ],
        ],
      },
    });
    const store = useAnalysisStore(pinia);
    refreshTopTableFromWorkflowRun();
    mountDrillDom();
    const equity = document.querySelector<HTMLCanvasElement>("#equityCanvasVue");
    const drawdown = document.querySelector<HTMLCanvasElement>(
      "#drawdownCanvasVue",
    );
    __analysisTestUtils.drawMinMaxChartDom(equity, [1, 1.05], {
      tone: "positive",
      baseline: 1,
      height: 160,
    });
    __analysisTestUtils.drawMinMaxChartDom(drawdown, [0, -0.05], {
      tone: "negative",
      baseline: 0,
      height: 160,
    });
    expect(drawdown?.dataset.supportChartStroke).toBeTruthy();
    invokeCommandMock.mockImplementation(async (command: unknown) => {
      if (command === "compute_double_ma_drilldown") {
        return {
          equity: { start_t: 1, end_t: 2, bins: 1, minmax: [1, 1.05] },
          drawdown: { start_t: 1, end_t: 2, bins: 1, minmax: [] },
          trades: [],
          trade_stats: null,
          indicator_bars: [],
        } as never;
      }
      if (command === "get_price_bars") return [] as never;
      if (command === "compute_double_ma_local_stability") return null as never;
      return null as never;
    });

    await selectTopRow(0);

    expect(store.drillWrapVisible).toBe(true);
    expect(store.drillChartModel).not.toBeNull();
    expect(equity?.dataset.supportChartStroke).toBeTruthy();
    expect(drawdown?.dataset.supportChartStroke).toBeUndefined();
    expect(equity?.dataset.renderReady).toBe("true");
    expect(drawdown?.dataset.renderReady).toBe("false");
    expect(
      document.querySelector<HTMLElement>("#equityChartStatusVue")?.hidden,
    ).toBe(true);
    expect(
      document.querySelector<HTMLElement>("#drawdownChartStatusVue")?.textContent,
    ).toContain("Drawdown envelope unavailable");
  });

  it("clears prior candidate evidence while a replacement drilldown is pending and after it fails", async () => {
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        data_id: "dataset-replacement-failure",
        ma_source: "close",
        objective: "Sharpe",
        strategy: {},
      },
      res: {
        top: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 8,
              slow_ma_type: "ema",
              slow_len: 32,
            },
            {
              pnl: 0.2,
              sharpe: 1.2,
              max_dd: 0.1,
              trades: 20,
              exposure: 0.5,
            },
          ],
        ],
      },
    });
    const store = useAnalysisStore(pinia);
    refreshTopTableFromWorkflowRun();
    mountDrillDom();
    const equity = document.querySelector<HTMLCanvasElement>("#equityCanvasVue");
    const drawdown = document.querySelector<HTMLCanvasElement>(
      "#drawdownCanvasVue",
    );
    __analysisTestUtils.drawMinMaxChartDom(equity, [1, 1.05], {
      tone: "positive",
      baseline: 1,
      height: 160,
    });
    __analysisTestUtils.drawMinMaxChartDom(drawdown, [0, -0.05], {
      tone: "negative",
      baseline: 0,
      height: 160,
    });
    if (equity) equity.dataset.renderReady = "true";
    if (drawdown) drawdown.dataset.renderReady = "true";
    store.drillWrapVisible = true;
    store.tradeStatsWrapVisible = true;
    store.stabilityWrapVisible = true;
    store.stabilityTableRows = [
      {
        key: "previous",
        cells: [{ text: "previous candidate", className: "mono" }],
      },
    ];
    store.drillChartModel = {} as never;
    store.drillTradeTableModel = { visible: true, rows: [] } as never;

    const drillDeferred = createDeferred<unknown>();
    invokeCommandMock.mockImplementation(async (command: unknown) => {
      if (command === "compute_double_ma_drilldown") {
        return await drillDeferred.promise as never;
      }
      return [] as never;
    });

    const selection = selectTopRow(0);

    expect(store.drillStatus).toBe("Computing candidate detail…");
    expect(store.drillWrapVisible).toBe(false);
    expect(store.drillChartModel).toBeNull();
    expect(store.drillTradeTableModel).toBeNull();
    expect(store.tradeStatsWrapVisible).toBe(false);
    expect(store.stabilityWrapVisible).toBe(false);
    expect(equity?.dataset.renderReady).toBe("false");
    expect(drawdown?.dataset.renderReady).toBe("false");
    expect(equity?.dataset.supportChartStroke).toBeUndefined();
    expect(drawdown?.dataset.supportChartStroke).toBeUndefined();
    expect(store.stabilityTableRows).toHaveLength(0);

    drillDeferred.reject(new Error("replacement unavailable"));
    await selection;

    expect(store.drillStatus).toContain("Candidate detail failed:");
    expect(store.drillStatus).toContain("replacement unavailable");
    expect(store.drillWrapVisible).toBe(false);
    expect(store.drillChartModel).toBeNull();
    expect(store.stabilityWrapVisible).toBe(false);
  });

  it("clears prior evidence when a historical row cannot form a candidate detail request", async () => {
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        data_id: "dataset-malformed-row",
        ma_source: "close",
        objective: "Sharpe",
        strategy: {},
      },
      res: {
        top: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 8,
              slow_len: 32,
            },
            {
              pnl: 0.2,
              sharpe: 1.2,
              max_dd: 0.1,
              trades: 20,
              exposure: 0.5,
            },
          ],
        ],
      },
    });
    const store = useAnalysisStore(pinia);
    refreshTopTableFromWorkflowRun();
    mountDrillDom();
    store.drillWrapVisible = true;
    store.tradeStatsWrapVisible = true;
    store.stabilityWrapVisible = true;
    store.stabilityTableRows = [
      {
        key: "previous",
        cells: [{ text: "previous candidate", className: "mono" }],
      },
    ];
    store.drillChartModel = {} as never;
    store.drillTradeTableModel = { visible: true, rows: [] } as never;

    await selectTopRow(0);

    expect(invokeCommandMock).not.toHaveBeenCalled();
    expect(store.drillStatus).toContain("Candidate detail unavailable:");
    expect(store.drillStatus).toContain("complete dataset and moving-average");
    expect(store.drillWrapVisible).toBe(false);
    expect(store.drillChartModel).toBeNull();
    expect(store.drillTradeTableModel).toBeNull();
    expect(store.stabilityWrapVisible).toBe(false);
    expect(store.stabilityTableRows).toHaveLength(0);
  });

  it("keeps valid replacement detail visible while isolating a stability failure", async () => {
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        data_id: "dataset-stability-failure",
        ma_source: "close",
        objective: "Sharpe",
        strategy: {},
      },
      res: {
        top: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 9,
              slow_ma_type: "ema",
              slow_len: 36,
            },
            {
              pnl: 0.25,
              sharpe: 1.4,
              max_dd: 0.08,
              trades: 24,
              exposure: 0.55,
            },
          ],
        ],
      },
    });
    const store = useAnalysisStore(pinia);
    refreshTopTableFromWorkflowRun();
    mountDrillDom();
    store.stabilityTableRows = [
      {
        key: "previous",
        cells: [{ text: "previous stability", className: "mono" }],
      },
    ];
    invokeCommandMock.mockImplementation(async (command: unknown) => {
      if (command === "compute_double_ma_drilldown") {
        return {
          equity: { start_t: 1, end_t: 2, bins: 1, minmax: [1, 1.08] },
          drawdown: { start_t: 1, end_t: 2, bins: 1, minmax: [0, -0.08] },
          trades: [],
          trade_stats: null,
          indicator_bars: [],
        } as never;
      }
      if (command === "get_price_bars") return [] as never;
      if (command === "compute_double_ma_local_stability") {
        throw new Error("stability unavailable");
      }
      return null as never;
    });

    await selectTopRow(0);

    expect(store.drillWrapVisible).toBe(true);
    expect(store.drillChartModel).not.toBeNull();
    expect(store.drillStatus).toBe("Done.");
    expect(store.stabilityWrapVisible).toBe(true);
    expect(store.stabilityStatus).toContain("Stability failed:");
    expect(store.stabilityStatus).toContain("stability unavailable");
    expect(store.stabilityTableRows).toHaveLength(0);
    expect(
      document.querySelector<HTMLCanvasElement>("#equityCanvasVue")?.dataset
        .renderReady,
    ).toBe("true");
  });

  it("clears visible candidate evidence when analysis execution caches are invalidated", () => {
    const store = useAnalysisStore(pinia);
    mountDrillDom();
    const equity = document.querySelector<HTMLCanvasElement>("#equityCanvasVue");
    const drawdown = document.querySelector<HTMLCanvasElement>(
      "#drawdownCanvasVue",
    );
    __analysisTestUtils.drawMinMaxChartDom(equity, [1, 1.05], {
      tone: "positive",
      baseline: 1,
      height: 160,
    });
    __analysisTestUtils.drawMinMaxChartDom(drawdown, [0, -0.05], {
      tone: "negative",
      baseline: 0,
      height: 160,
    });
    if (equity) equity.dataset.renderReady = "true";
    if (drawdown) drawdown.dataset.renderReady = "true";
    store.drillWrapVisible = true;
    store.tradeStatsWrapVisible = true;
    store.stabilityWrapVisible = true;
    store.stabilityTableRows = [
      {
        key: "stale",
        cells: [{ text: "stale", className: "mono" }],
      },
    ];
    store.drillChartModel = {} as never;
    store.drillTradeTableModel = { visible: true, rows: [] } as never;

    invalidateAnalysisExecutionCaches();

    expect(store.drillStatus).toBe(
      "Click a Top row to view candidate detail curves.",
    );
    expect(store.drillWrapVisible).toBe(false);
    expect(store.tradeStatsWrapVisible).toBe(false);
    expect(store.stabilityWrapVisible).toBe(false);
    expect(store.drillChartModel).toBeNull();
    expect(store.drillTradeTableModel).toBeNull();
    expect(equity?.dataset.renderReady).toBe("false");
    expect(drawdown?.dataset.renderReady).toBe("false");
    expect(equity?.dataset.supportChartStroke).toBeUndefined();
    expect(drawdown?.dataset.supportChartStroke).toBeUndefined();
    expect(store.stabilityTableRows).toHaveLength(0);
  });

  it("builds detail trade markers from candidate detail trades and publishes chart/table models", async () => {
    mountDrillDom();
    const drillCurves = {
      equity: { start_t: 1, end_t: 4, bins: 2, minmax: [1, 1.02, 1.01, 1.05] },
      drawdown: {
        start_t: 1,
        end_t: 4,
        bins: 2,
        minmax: [0, -0.01, -0.02, -0.01],
      },
      trades: [
        {
          entry_t: 1,
          exit_t: 2,
          direction: 1,
          pnl: 0.03,
          bars: 1,
          open: false,
        },
        {
          entry_t: 3,
          exit_t: 4,
          direction: -1,
          pnl: -0.01,
          bars: 1,
          open: false,
        },
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
        {
          time: 1_744_500_000,
          from_index: 1,
          to_index: 1,
          fast: 100.2,
          slow: 100.0,
        },
        {
          time: 1_744_500_060,
          from_index: 2,
          to_index: 2,
          fast: 100.5,
          slow: 100.1,
        },
        {
          time: 1_744_500_120,
          from_index: 3,
          to_index: 3,
          fast: 100.3,
          slow: 100.2,
        },
        {
          time: 1_744_500_180,
          from_index: 4,
          to_index: 4,
          fast: 100.1,
          slow: 100.25,
        },
      ],
    };
    const drillBars = [
      {
        time: 1_744_500_000,
        open: 100,
        high: 101,
        low: 99.8,
        close: 100.6,
        fromIndex: 1,
        toIndex: 1,
        volume: 0,
      },
      {
        time: 1_744_500_060,
        open: 100.6,
        high: 101.2,
        low: 100.3,
        close: 101.0,
        fromIndex: 2,
        toIndex: 2,
        volume: 0,
      },
      {
        time: 1_744_500_120,
        open: 101.0,
        high: 101.1,
        low: 100.2,
        close: 100.4,
        fromIndex: 3,
        toIndex: 3,
        volume: 0,
      },
      {
        time: 1_744_500_180,
        open: 100.4,
        high: 100.8,
        low: 99.9,
        close: 100.0,
        fromIndex: 4,
        toIndex: 4,
        volume: 0,
      },
    ];
    const markers = __analysisTestUtils.buildDrillTradeMarkers(
      drillCurves,
      drillBars,
    );
    expect(
      markers.some(
        (marker) => marker.id === "trade-0-entry" && marker.text === "L1",
      ),
    ).toBe(true);
    expect(markers.some((marker) => marker.id === "trade-0-exit")).toBe(true);
    expect(
      markers.some(
        (marker) => marker.id === "trade-1-entry" && marker.text === "S2",
      ),
    ).toBe(true);

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
    expect(
      selectedMarkers.find((marker) => marker.id === "trade-1-entry")?.size,
    ).toBe(1.4);
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
    const req =
      (stabilityCall?.[1] as { req?: Record<string, unknown> }).req ?? {};
    expect(req.backend_hint).toEqual({ backend: "GpuKernel", device_id: 0 });
    expect(req.followup_cache_key).toBe("cache-token-1");
    expect(req.score_mode).toBe("balanced_risk_adjusted_10");
    expect(req).not.toHaveProperty("bins");
  });

  it("routes CompositeCpu candidate detail and local stability through the composite command family", async () => {
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
          backend: { backend: "GpuKernel", device_id: 0 },
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
              halftrend: {
                amplitude: 9,
                atr_period: 21,
                channel_deviation: 1.6,
              },
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
          backend: { backend: "CpuOnly" },
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

    invokeCommandMock.mockImplementation(
      async (command: unknown, args?: unknown) => {
        if (command === "compute_double_ma_drilldown") {
          const req = (args as { req?: Record<string, unknown> })?.req ?? {};
          const fastLen = Number(
            (req.params as Record<string, unknown>)?.fast_len ?? 0,
          );
          return {
            equity: {
              start_t: 1,
              end_t: 2,
              bins: 1,
              minmax: [fastLen, fastLen],
            },
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
      },
    );

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

  it("keeps an in-flight stability result live when Candidate Detail remounts from cached drill evidence", async () => {
    mountDrillDom();
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        data_id: "dataset-stability-remount",
        ma_source: "close",
        objective: "Sharpe",
        backend: { backend: "CpuOnly" },
        strategy: {},
        halftrend_filter: null,
      },
      res: {
        followup_cache_key: "cache-token-stability-remount",
        top: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 7,
              slow_ma_type: "ema",
              slow_len: 30,
            },
            {
              pnl: 0.2,
              sharpe: 1.2,
              max_dd: 0.1,
              trades: 20,
              exposure: 0.5,
            },
          ],
        ],
      },
    });
    useDataStore(pinia).dataId = "dataset-stability-remount";
    const store = useAnalysisStore(pinia);
    refreshTopTableFromWorkflowRun();
    const stabilityDeferred = createDeferred<unknown>();
    let drillCalls = 0;
    let stabilityCalls = 0;
    invokeCommandMock.mockImplementation(async (command: unknown) => {
      if (command === "compute_double_ma_drilldown") {
        drillCalls += 1;
        return {
          equity: { start_t: 1, end_t: 2, bins: 1, minmax: [1, 1.05] },
          drawdown: { start_t: 1, end_t: 2, bins: 1, minmax: [0, -0.05] },
          trades: [],
          trade_stats: null,
          indicator_bars: [],
        } as never;
      }
      if (command === "get_price_bars") return [] as never;
      if (command === "compute_double_ma_local_stability") {
        stabilityCalls += 1;
        return await stabilityDeferred.promise as never;
      }
      return null as never;
    });

    const selection = selectTopRow(0);
    await vi.waitFor(() => {
      expect(store.stabilityStatus).toBe("Computing stability…");
    });

    await rerenderDrill();

    expect(store.stabilityStatus).toBe("Computing stability…");
    expect(drillCalls).toBe(1);
    expect(stabilityCalls).toBe(1);

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
      points: [
        {
          fast_len: 7,
          slow_len: 30,
          score: 1.2,
          metrics: {
            pnl: 0.2,
            sharpe: 1.2,
            max_dd: 0.1,
            trades: 20,
          },
        },
      ],
      summary: { base: 1.2, min: 1.2, median: 1.2, max: 1.2 },
    });
    await selection;

    expect(store.stabilityStatus).toBe("Done.");
    expect(store.stabilityTableRows).toHaveLength(1);
    expect(store.stabilityTableRows[0]?.cells[0]?.text).toBe("7");
    expect(drillCalls).toBe(1);
    expect(stabilityCalls).toBe(1);
  });

  it("invalidates an in-flight candidate detail response when the active dataset is cleared", async () => {
    mountDrillDom();
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        data_id: "dataset-clear-race",
        ma_source: "close",
        objective: "Sharpe",
        backend: { backend: "CpuOnly" },
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
            {
              pnl: 0.2,
              sharpe: 1.2,
              max_dd: 0.1,
              trades: 20,
              exposure: 0.5,
            },
          ],
        ],
      },
    });
    const dataStore = useDataStore(pinia);
    dataStore.baseDataId = "dataset-clear-race";
    dataStore.dataId = "dataset-clear-race";
    const store = useAnalysisStore(pinia);
    refreshTopTableFromWorkflowRun();
    const drillDeferred = createDeferred<unknown>();
    invokeCommandMock.mockImplementation(async (command: unknown) => {
      if (command === "compute_double_ma_drilldown") {
        return await drillDeferred.promise as never;
      }
      return null as never;
    });

    const selection = selectTopRow(0);
    await vi.waitFor(() => {
      expect(store.drillStatus).toBe("Computing candidate detail…");
    });

    dataStore.clearData();

    expect(store.drillWrapVisible).toBe(false);
    expect(store.drillChartModel).toBeNull();
    expect(store.drillStatus).toBe(
      "Click a Top row to view candidate detail curves.",
    );

    drillDeferred.resolve({
      equity: { start_t: 1, end_t: 2, bins: 1, minmax: [1, 1.05] },
      drawdown: { start_t: 1, end_t: 2, bins: 1, minmax: [0, -0.05] },
      trades: [],
      trade_stats: null,
      indicator_bars: [],
    });
    await selection;
    await Promise.resolve();

    expect(store.drillWrapVisible).toBe(false);
    expect(store.drillChartModel).toBeNull();
    expect(
      document.querySelector<HTMLCanvasElement>("#equityCanvasVue")?.dataset
        .renderReady,
    ).toBe("false");
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
        return {
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
          scale: () => undefined,
          rotate: () => undefined,
          createLinearGradient: () => ({ addColorStop: () => undefined }),
        } as never;
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
        fastAxisLabel: "Fast EMA length",
        slowAxisLabel: "Slow EMA length",
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

  it("keeps a populated aggregate heatmap available while explaining why 3D needs retained multi-slice rows", () => {
    const store = useAnalysisStore(pinia);
    store.heatmapViewMode = "3d";
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        top_k: 20,
        heatmap_bins: 2,
        include_all: false,
        fast_ma_types: ["sma"],
        slow_ma_types: ["alma"],
      },
      res: {
        heatmap: {
          bins_fast: 2,
          bins_slow: 2,
          fast_min: 5,
          fast_max: 6,
          slow_min: 20,
          slow_max: 30,
          values: [0.1, 0.2, 0.3, 0.4],
        },
      },
    });

    expect(refreshHeatmapFromWorkflowRun()).toBe(true);
    expect(store.heatmapWrapVisible).toBe(true);
    expect(store.heatmapCanvasModel).not.toBeNull();
    expect(store.heatmapCanvasModel?.slice3d).toBeNull();
    expect(store.heatmapViewMode).toBe("2d");
    expect(store.heatmapViewModeOptions).toContainEqual({
      value: "3d",
      label: "3D slices",
      disabled: true,
      disabledReason: expect.stringContaining("Return all results"),
    });
    expect(
      store.heatmapCanvasModel?.aggregateSelectionRequiresExactParams,
    ).toBe(false);
    expect(
      buildHeatmapCellParams(store.heatmapCanvasModel!, {
        col: 0,
        drawRow: 1,
        logicalRow: 0,
        idx: 0,
      }),
    ).toMatchObject({
      fast_len: 5,
      fast_ma_type: "sma",
      slow_len: 20,
      slow_ma_type: "alma",
    });
    expect(String(store.heatmapStatusLine)).toContain(
      "representative parameter pair",
    );
  });

  it("opens the exact retained aggregate winner for an ambiguous MA-parameter sweep", () => {
    const store = useAnalysisStore(pinia);
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        top_k: 20,
        heatmap_bins: 2,
        include_all: true,
        fast_ma_types: ["alma"],
        slow_ma_types: ["ema"],
        fast_ma_param_sweeps: {
          offset: [0.75, 0.85],
        },
      },
      res: {
        heatmap: {
          bins_fast: 2,
          bins_slow: 2,
          fast_min: 5,
          fast_max: 6,
          slow_min: 20,
          slow_max: 30,
          values: [0.8, null, null, null],
        },
        all: [
          [
            {
              fast_len: 5,
              fast_ma_type: "alma",
              fast_ma_params: { offset: 0.75, sigma: 6 },
              slow_len: 20,
              slow_ma_type: "ema",
            },
            { sharpe: 0.4 },
          ],
          [
            {
              fast_len: 5,
              fast_ma_type: "alma",
              fast_ma_params: { offset: 0.85, sigma: 6 },
              slow_len: 20,
              slow_ma_type: "ema",
            },
            { sharpe: 0.8 },
          ],
        ],
        all_scores: [0.4, 0.8],
      },
    });

    expect(refreshHeatmapFromWorkflowRun()).toBe(true);
    const renderModel = store.heatmapCanvasModel!;
    const cell = {
      col: 0,
      drawRow: 1,
      logicalRow: 0,
      idx: 0,
    };
    expect(renderModel.aggregateSelectionRequiresExactParams).toBe(true);
    expect(renderModel.exactAggregateWinnerCells?.[0]).toMatchObject({
      score: 0.8,
      params: {
        fast_ma_type: "alma",
        fast_ma_params: { offset: 0.85, sigma: 6 },
      },
    });
    expect(buildHeatmapCellParams(renderModel, cell)).toMatchObject({
      fast_ma_type: "alma",
      fast_ma_params: { offset: 0.85, sigma: 6 },
    });
    expect(heatmapTooltipText(renderModel, cell)).toContain(
      "exact retained best-in-bin winner",
    );
    expect(String(store.heatmapMeta)).toContain(
      "exact retained best-in-bin winners",
    );
    expect(String(store.heatmapStatusLine)).toContain(
      "exact retained best-in-bin winners",
    );
  });

  it("withholds ambiguous aggregate drilldown when the exact winning MA variant was not retained", () => {
    const store = useAnalysisStore(pinia);
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        top_k: 20,
        heatmap_bins: 2,
        include_all: false,
        fast_ma_types: ["alma"],
        slow_ma_types: ["ema"],
        fast_ma_param_sweeps: {
          offset: [0.75, 0.85],
        },
      },
      res: {
        heatmap: {
          bins_fast: 2,
          bins_slow: 2,
          fast_min: 5,
          fast_max: 6,
          slow_min: 20,
          slow_max: 30,
          values: [0.8, null, null, null],
        },
      },
    });

    expect(refreshHeatmapFromWorkflowRun()).toBe(true);
    const renderModel = store.heatmapCanvasModel!;
    const cell = {
      col: 0,
      drawRow: 1,
      logicalRow: 0,
      idx: 0,
    };
    expect(renderModel.aggregateSelectionRequiresExactParams).toBe(true);
    expect(buildHeatmapCellParams(renderModel, cell)).toBeNull();
    expect(heatmapTooltipText(renderModel, cell)).toContain(
      "exact winning MA variant was not retained",
    );
    expect(String(store.heatmapMeta)).toContain(
      "exact winning MA variant was not retained",
    );
    expect(String(store.heatmapStatusLine)).toContain("Return all results");
  });

  it("publishes strategy-local VCET axes in heatmap metadata and hover text", () => {
    const store = useAnalysisStore(pinia);
    seedWorkflowRun({
      req: {
        strategy_kind: "Vcet",
        objective: "Sharpe",
        top_k: 20,
        heatmap_bins: 2,
        fast_ma_types: ["sma"],
        slow_ma_types: ["ema"],
        vcet: {
          map_fast_to_vidya_short_period: true,
          map_slow_to_vidya_long_period: true,
          map_slow_to_ce_period: true,
        },
      },
      res: {
        heatmap: {
          bins_fast: 2,
          bins_slow: 2,
          fast_min: 5,
          fast_max: 6,
          slow_min: 20,
          slow_max: 30,
          values: [0.1, 0.2, 0.3, 0.4],
        },
      },
    });

    expect(refreshHeatmapFromWorkflowRun()).toBe(true);
    expect(store.heatmapCanvasModel?.request).toMatchObject({
      fastAxisLabel: "VIDYA short period",
      slowAxisLabel: "VIDYA long / Chandelier periods",
      fastMaType: "sma",
      slowMaType: "ema",
      supports3dSlices: false,
    });
    expect(String(store.heatmapMeta)).toContain("VIDYA short period=5-6");
    expect(String(store.heatmapMeta)).toContain(
      "VIDYA long / Chandelier periods=20-30",
    );

    const tooltip = heatmapTooltipText(store.heatmapCanvasModel!, {
      col: 0,
      drawRow: 1,
      logicalRow: 0,
      idx: 0,
    });
    expect(tooltip).toContain("VIDYA short period bin 5");
    expect(tooltip).toContain("VIDYA long / Chandelier periods bin 20");
    expect(tooltip).not.toContain("SMA bin");
    expect(tooltip).not.toContain("EMA bin");
    expect(tooltip).toContain("representative parameter pair");
    expect(
      buildHeatmapCellParams(store.heatmapCanvasModel!, {
        col: 0,
        drawRow: 1,
        logicalRow: 0,
        idx: 0,
      }),
    ).toMatchObject({
      fast_len: 5,
      slow_len: 20,
    });
  });

  it("labels explicitly disconnected VCC heatmap coordinates as unmapped", () => {
    const store = useAnalysisStore(pinia);
    seedWorkflowRun({
      req: {
        strategy_kind: "Vcc",
        objective: "Sharpe",
        top_k: 20,
        heatmap_bins: 2,
        vcc: {
          map_fast_to_squeeze_period: false,
          map_fast_to_momentum_period: false,
          map_slow_to_trend_period: false,
        },
      },
      res: {
        heatmap: {
          bins_fast: 2,
          bins_slow: 2,
          fast_min: 5,
          fast_max: 6,
          slow_min: 20,
          slow_max: 30,
          values: [0.1, 0.2, 0.3, 0.4],
        },
      },
    });

    expect(refreshHeatmapFromWorkflowRun()).toBe(true);
    expect(store.heatmapCanvasModel?.request).toMatchObject({
      fastAxisLabel: "Fast grid (unmapped)",
      slowAxisLabel: "Slow grid (unmapped)",
      fastAxisDescription: expect.stringContaining(
        "does not alter any mapped strategy periods",
      ),
      slowAxisDescription: expect.stringContaining(
        "does not alter any mapped strategy periods",
      ),
    });
  });

  it("keeps 3D slices unavailable for standalone strategies even with synthetic MA variants", () => {
    const store = useAnalysisStore(pinia);
    store.heatmapViewMode = "3d";
    seedWorkflowRun({
      req: {
        strategy_kind: "Vcet",
        objective: "Sharpe",
        top_k: 20,
        heatmap_bins: 2,
        include_all: true,
      },
      res: {
        heatmap: {
          bins_fast: 2,
          bins_slow: 2,
          fast_min: 5,
          fast_max: 6,
          slow_min: 20,
          slow_max: 30,
          values: [0.1, 0.2, 0.3, 0.4],
        },
        all: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 5,
              slow_ma_type: "ema",
              slow_len: 20,
            },
            { sharpe: 0.4 },
          ],
          [
            {
              fast_ma_type: "sma",
              fast_len: 6,
              slow_ma_type: "sma",
              slow_len: 30,
            },
            { sharpe: 0.3 },
          ],
        ],
        all_scores: [0.4, 0.3],
      },
    });

    expect(refreshHeatmapFromWorkflowRun()).toBe(true);
    expect(store.heatmapCanvasModel?.slice3d).toBeNull();
    expect(store.heatmapViewMode).toBe("2d");
    expect(store.heatmapViewModeOptions).toContainEqual({
      value: "3d",
      label: "3D slices",
      disabled: true,
      disabledReason: expect.stringContaining("standalone strategy heatmaps"),
    });
  });

  it("keeps objective-only 3D slices unavailable while plateau mode is active", () => {
    const store = useAnalysisStore(pinia);
    store.heatmapMode = "plateau";
    store.heatmapViewMode = "3d";
    seedWorkflowRun({
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
        top_k: 20,
        heatmap_bins: 2,
        include_all: true,
      },
      res: {
        heatmap: {
          bins_fast: 2,
          bins_slow: 2,
          fast_min: 5,
          fast_max: 6,
          slow_min: 20,
          slow_max: 30,
          values: [0.1, 0.2, 0.3, 0.4],
        },
        all: [
          [
            {
              fast_ma_type: "ema",
              fast_len: 5,
              slow_ma_type: "ema",
              slow_len: 20,
            },
            { sharpe: 0.4 },
          ],
          [
            {
              fast_ma_type: "sma",
              fast_len: 6,
              slow_ma_type: "sma",
              slow_len: 30,
            },
            { sharpe: 0.3 },
          ],
        ],
        all_scores: [0.4, 0.3],
      },
    });

    expect(refreshHeatmapFromWorkflowRun()).toBe(true);
    expect(store.heatmapCanvasModel?.slice3d?.slices).toHaveLength(2);
    expect(store.heatmapViewMode).toBe("2d");
    expect(store.heatmapViewModeOptions).toContainEqual({
      value: "3d",
      label: "3D slices",
      disabled: true,
      disabledReason: "3D slices are available only in Objective score mode.",
    });

    store.heatmapMode = "objective";
    expect(refreshHeatmapFromWorkflowRun()).toBe(true);
    expect(store.heatmapViewModeOptions).toContainEqual({
      value: "3d",
      label: "3D slices",
      disabled: false,
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
              {
                pnl: 0.2,
                sharpe: 1.1,
                max_dd: 0.12,
                trades: 18,
                exposure: 0.45,
              },
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
            sample_counts: [1, 1, 1, 1, 4, 1, 1, 1, 1],
          },
        },
      });

      expect(refreshHeatmapFromWorkflowRun()).toBe(true);
      expect(store.heatmapCanvasModel).toBeTruthy();
      return (
        heatmapTooltipText(store.heatmapCanvasModel!, {
          col: 1,
          drawRow: 1,
          logicalRow: 1,
          idx: 4,
        }) ?? ""
      );
    };

    const centeredTooltip = readPlateauTooltip(
      [1, 2, 1, 2, 3, 2, 1, 2, 1],
      [0.9, 1.8, 0.9, 1.8, 2.6, 1.8, 0.9, 1.8, 0.9],
    );
    const shiftedTooltip = readPlateauTooltip(
      [11, 12, 11, 12, 13, 12, 11, 12, 11],
      [10.9, 11.8, 10.9, 11.8, 12.6, 11.8, 10.9, 11.8, 10.9],
    );
    const centeredScore = centeredTooltip.match(
      /Plateau\s+\w+\s+([0-9.]+)/,
    )?.[1];
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
          values: [0.42, 0.61, 0.39, 0.57, 0.86, 0.58, 0.35, 0.54, 0.33],
          mean_values: [0.4, 0.58, 0.37, 0.54, 0.74, 0.55, 0.34, 0.52, 0.32],
          sample_counts: [1, 1, 1, 1, 4, 1, 1, 1, 1],
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
    expect(String(store.heatmapMeta || "")).toContain(
      "Derived plateau heatmap",
    );
    expect(String(store.heatmapMeta || "")).toContain(
      "supportive=8-neighbor agreement",
    );
    expect(String(store.heatmapMeta || "")).toContain(
      "density-aware=1 multi-sample cells",
    );
    expect(String(store.heatmapStatusLine || "")).toContain(
      "exact winner robustness Strong 0.8125",
    );
    expect(String(store.heatmapStatusLine || "")).toContain(
      "representative parameter pair",
    );

    expect(store.heatmapCanvasModel).toBeTruthy();
    const tooltip =
      heatmapTooltipText(store.heatmapCanvasModel!, {
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
    expect(
      heatmapTooltipText(store.heatmapCanvasModel!, {
        col: 0,
        drawRow: 1,
        logicalRow: 0,
        idx: 0,
      }),
    ).toContain("Sharpe");
    expect(
      heatmapTooltipText(store.heatmapCanvasModel!, {
        col: 1,
        drawRow: 1,
        logicalRow: 0,
        idx: 1,
      }),
    ).toBeNull();
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
          values: [0.42, 0.61, 0.39, 0.57, 0.86, 0.58, 0.35, 0.54, 0.33],
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

  it("groups visible Top ranks that map to the same heatmap bin", () => {
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
            { pnl: 0.1, sharpe: 1.2, max_dd: 0.1, trades: 10, exposure: 0.4 },
          ],
          [
            {
              fast_ma_type: "ema",
              fast_len: 6,
              slow_ma_type: "ema",
              slow_len: 21,
            },
            { pnl: 0.09, sharpe: 1.1, max_dd: 0.11, trades: 11, exposure: 0.4 },
          ],
          [
            {
              fast_ma_type: "ema",
              fast_len: 6,
              slow_ma_type: "ema",
              slow_len: 22,
            },
            { pnl: 0.08, sharpe: 1, max_dd: 0.12, trades: 12, exposure: 0.4 },
          ],
          [
            {
              fast_ma_type: "ema",
              fast_len: 6,
              slow_ma_type: "ema",
              slow_len: 23,
            },
            { pnl: 0.07, sharpe: 0.9, max_dd: 0.13, trades: 13, exposure: 0.4 },
          ],
          [
            {
              fast_ma_type: "ema",
              fast_len: 9,
              slow_ma_type: "ema",
              slow_len: 30,
            },
            { pnl: 0.06, sharpe: 0.8, max_dd: 0.14, trades: 14, exposure: 0.4 },
          ],
        ],
        heatmap: {
          bins_fast: 2,
          bins_slow: 2,
          fast_min: 5,
          fast_max: 9,
          slow_min: 20,
          slow_max: 30,
          values: [0.42, 0.61, 0.39, 0.57],
        },
      },
    });

    expect(refreshHeatmapFromWorkflowRun()).toBe(true);

    expect(store.heatmapCanvasModel?.markers).toEqual([
      { col: 0, drawRow: 1, label: "1,2 +2" },
      { col: 1, drawRow: 0, label: "5" },
    ]);
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
    expect(store.heatmapStatusLine).toContain(
      "markers use the best candidate only.",
    );
    expect(
      store.heatmapCanvasModel?.markers.map((marker) => marker.label),
    ).toContain("1");
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
          values: [0.42, 0.61, 0.39, 0.57, 0.86, 0.58, 0.35, 0.54, 0.33],
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
    runHeatmapRepresentativeCandidateDetail({
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

  it("forwards composite-family configs for advanced candidate detail and local stability", async () => {
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
            fast_len: 8,
            slow_len: 34,
            fast_ma_type: "vidya",
            slow_ma_type: "vidya",
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
          vcet: {
            vidya_short_period: 8,
            vidya_long_period: 34,
            ce_period: 34,
            ce_mult: 2.8,
          },
        }),
      }),
    );
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "compute_composite_cpu_local_stability",
      expect.objectContaining({
        req: expect.objectContaining({
          vcet: {
            vidya_short_period: 8,
            vidya_long_period: 34,
            ce_period: 34,
            ce_mult: 2.8,
          },
        }),
      }),
    );
    expect(String(store.drillStatus || "")).not.toContain("not implemented");
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

  it("passively rerenders Candidate Detail when the saved and loaded data_id match", async () => {
    seedRerenderCandidateRun("dataset-rerender");
    useDataStore(pinia).dataId = "dataset-rerender";
    installRerenderDrillCommandResults();

    await rerenderDrill();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "compute_double_ma_drilldown",
      expect.any(Object),
    );
    expect(useAnalysisStore(pinia).topTableRows).toHaveLength(1);
  });

  it("keeps historical Candidate Detail rows passive when no dataset is loaded", async () => {
    seedRerenderCandidateRun("dataset-rerender");
    const store = useAnalysisStore(pinia);
    useDataStore(pinia).dataId = "—";
    installRerenderDrillCommandResults();

    await rerenderDrill();

    expect(invokeCommandMock).not.toHaveBeenCalled();
    expect(store.topTableRows).toHaveLength(1);
    expect(store.drillWrapVisible).toBe(false);
    expect(store.drillChartModel).toBeNull();
    expect(store.drillStatus).toContain(
      "saved result data_id dataset-rerender is not currently loaded",
    );
    expect(store.drillStatus).toContain(
      "Historical candidate rows remain available in Top Results",
    );
  });

  it("marks passive Candidate Detail stale when the loaded dataset does not match", async () => {
    seedRerenderCandidateRun("dataset-rerender");
    const store = useAnalysisStore(pinia);
    useDataStore(pinia).dataId = "dataset-other";
    installRerenderDrillCommandResults();

    await rerenderDrill();

    expect(invokeCommandMock).not.toHaveBeenCalled();
    expect(store.topTableRows).toHaveLength(1);
    expect(store.drillWrapVisible).toBe(false);
    expect(store.drillChartModel).toBeNull();
    expect(store.drillMeta).toContain(
      "saved data_id=dataset-rerender | loaded data_id=dataset-other",
    );
    expect(store.drillStatus).toContain("Candidate detail stale/incompatible");
    expect(store.drillStatus).toContain(
      "Historical candidate rows remain available in Top Results",
    );
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
