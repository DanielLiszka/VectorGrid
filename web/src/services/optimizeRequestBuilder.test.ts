import { describe, expect, it } from "vitest";
import {
  buildCompositeCpuOptimizationRequest,
  buildCompositeMoOptimizationRequest,
  buildDoubleMaOptimizationRequest,
} from "./optimizeRequestBuilder";

describe("optimizeRequestBuilder", () => {
  it("builds a DoubleMa optimization request from typed inputs", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-123",
        current_data_meta: { has_close: true },
      },
      backend: "Auto",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "64",
      exportAllCsvPath: "",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "1",
      spreadBps: "2",
      slippageBps: "0.5",
      epsRel: "0.001",
      stopLossPct: "0.01",
      takeProfitPct: "0.03",
      cooldownBars: "0",
      signalPersistenceBars: "1",
      exitMode: "cross",
      kaufmanStopPeriod: "22",
      kaufmanStopMult: "2.0",
      kaufmanStopMaType: "sma",
      longOnly: true,
      allowFlip: true,
      tradeOnNext: false,
      dmConstraintMinTrades: "5",
      dmConstraintMaxDd: "0.25",
      fastMaParams: { alpha: 2 },
      slowMaParams: { alpha: 3 },
      fastMaParamSweeps: { alpha: [1, 3, 1] },
      slowMaParamSweeps: null,
    });

    expect(req).toBeTruthy();
    expect(req).toMatchObject({
      strategy_kind: "DoubleMa",
      data_id: "data-123",
      objective: "Sharpe",
      mode: "Grid",
      gpu_execution: "PreferGpuResident",
      persist_followup_cache: true,
      include_trade_metrics: false,
      top_k: 20,
      include_all: false,
      fast_ma_types: ["sma"],
      slow_ma_types: ["ema"],
      fast_ma_params: { alpha: 2 },
      slow_ma_params: { alpha: 3 },
      fast_ma_param_sweeps: { alpha: [1, 3, 1] },
      strategy: {
        long_only: true,
        allow_flip: true,
        trade_on_next_bar: true,
      },
      constraints: {
        min_trades: 5,
        max_drawdown: 0.25,
      },
    });
  });

  it("returns null for non-DoubleMa strategy kind", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "CompositeMo",
        data_id: "data-123",
        current_data_meta: { has_close: true },
      },
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
    });
    expect(req).toBeNull();
  });

  it("rejects DoubleMa optimization when the active dataset has fewer than two candles", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DoubleMa",
          data_id: "data-too-short",
          current_data_meta: { has_close: true, len: 1, timeframe: "1d" },
        },
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
      }),
    ).toThrow(/Loaded dataset has 1 candle at 1d\. Optimization needs at least 2 candles\./i);
  });

  it("builds extended DoubleMa constraints when provided", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-constraints",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Omega",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
      dmConstraintMinTrades: "5",
      dmConstraintMaxDd: "0.25",
      dmConstraintMinSharpe: "1.2",
      dmConstraintMinSortino: "1.6",
      dmConstraintMinCalmar: "0.8",
      dmConstraintMaxVolatility: "0.3",
      dmConstraintMinExposure: "0.25",
      dmConstraintMaxExposure: "0.75",
      dmConstraintMaxAbsNetExposure: "0.4",
      dmConstraintMinOmega: "1.1",
      dmConstraintMaxUlcer: "0.15",
      dmConstraintMaxDownsideDeviation: "0.2",
    });

    expect(req).toMatchObject({
      objective: "Omega",
      constraints: {
        min_trades: 5,
        max_drawdown: 0.25,
        min_sharpe: 1.2,
        min_sortino: 1.6,
        min_calmar: 0.8,
        max_volatility: 0.3,
        min_exposure: 0.25,
        max_exposure: 0.75,
        max_abs_net_exposure: 0.4,
        min_omega: 1.1,
        max_ulcer: 0.15,
        max_downside_deviation: 0.2,
      },
    });
  });

  it("normalizes additive DoubleMa score modes into the runtime request", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-score-mode",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      scoreMode: "BalancedRiskAdjusted10",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      objective: "Sharpe",
      score_mode: "balanced_risk_adjusted_10",
    });
  });

  it("normalizes adversarial DoubleMa score modes into the runtime request", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-score-mode-envelope",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      scoreMode: "AdversarialWeightEnvelope10",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      objective: "Sharpe",
      score_mode: "adversarial_envelope_10",
    });
  });

  it("normalizes support-shrunk DoubleMa score modes into the runtime request", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-score-mode-support",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      scoreMode: "EffectiveSupport10",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      objective: "Sharpe",
      score_mode: "support_shrunk_10",
    });
  });

  it("normalizes active-sortino DoubleMa score modes into the runtime request", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-score-mode-active-sortino",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      scoreMode: "ActiveSortino10",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      objective: "Sharpe",
      score_mode: "active_sortino_10",
    });
  });

  it("normalizes effective-sample LCB DoubleMa score modes into the runtime request", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-score-mode-effective-sample-lcb",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      scoreMode: "ObsLcb10",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      objective: "Sharpe",
      score_mode: "effective_sample_lcb_10",
    });
  });

  it("normalizes omega-vs-sharpe DoubleMa score modes into the runtime request", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-score-mode-omega-vs-sharpe",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      scoreMode: "OmegaVsSharpe10",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      objective: "Sharpe",
      score_mode: "omega_vs_sharpe_10",
    });
  });

  it("normalizes robust-margin DoubleMa score modes into the runtime request", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-score-mode-robust-margin",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      scoreMode: "RobustMargin10",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      objective: "Sharpe",
      score_mode: "robust_margin_10",
    });
  });

  it("normalizes extra shortlist objectives into the runtime request", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-extra-shortlist",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      extraShortlistObjectives: "Pnl, omega, Sharpe, pnl",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      objective: "Sharpe",
      extra_shortlist_objectives: ["Pnl", "Omega"],
    });
  });

  it("passes hurdle tau into the runtime request when provided", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-hurdle-tau",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sortino",
      hurdleTau: "0.01",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      objective: "Sortino",
      hurdle_tau: 0.01,
    });
  });

  it("passes include_trade_metrics into the runtime request when enabled", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-trade-metrics",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      includeTradeMetrics: true,
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      objective: "Sharpe",
      include_trade_metrics: true,
    });
  });

  it("allows include_trade_metrics for DynamicDoubleMa on the requested GpuKernel route", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DynamicDoubleMa",
        data_id: "data-trade-metrics-dynamic",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      includeTradeMetrics: true,
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
      dynamicStrategyIr: { profile: "double_ma" },
    });

    expect((req as Record<string, unknown>).include_trade_metrics).toBe(true);
  });

  it("allows include_trade_metrics for DynamicDoubleMa on the CpuOnly route", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DynamicDoubleMa",
        data_id: "data-trade-metrics-dynamic-cpu",
        current_data_meta: { has_close: true },
      },
      backend: "CpuOnly",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      includeTradeMetrics: true,
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
      dynamicStrategyIr: { profile: "double_ma" },
    });

    expect((req as Record<string, unknown>).include_trade_metrics).toBe(true);
  });

  it("allows an explicit full_17 metric contract on the exact GpuKernel route", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-metric-contract-full17",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      metricContract: "full_17",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      metric_contract: "full_17",
      include_trade_metrics: false,
    });
  });

  it("allows an explicit full_17 metric contract on the CpuOnly DoubleMa route", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-metric-contract-full17-cpu",
        current_data_meta: { has_close: true },
      },
      backend: "CpuOnly",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      metricContract: "full_17",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      metric_contract: "full_17",
      include_trade_metrics: false,
    });
  });

  it("allows an explicit full_17 metric contract for DynamicDoubleMa on the CpuOnly route", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DynamicDoubleMa",
        data_id: "data-metric-contract-full17-dynamic-cpu",
        current_data_meta: { has_close: true },
      },
      backend: "CpuOnly",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      metricContract: "full_17",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
      dynamicStrategyIr: { profile: "double_ma" },
    });

    expect(req).toMatchObject({
      metric_contract: "full_17",
      include_trade_metrics: false,
    });
  });

  it("allows an explicit trade_22 metric contract on the exact GpuKernel route", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-metric-contract-trade22",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      metricContract: "trade_22",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      metric_contract: "trade_22",
      include_trade_metrics: true,
    });
  });

  it("rejects an explicit full_17 metric contract when includeTradeMetrics=true", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DoubleMa",
          data_id: "data-metric-contract-conflict",
          current_data_meta: { has_close: true },
        },
        backend: "GpuKernel",
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        metricContract: "full_17",
        includeTradeMetrics: true,
        mode: "Grid",
        topK: "20",
        includeAll: "false",
        heatmapBins: "0",
        startingEquity: "10000",
        costPreset: "custom",
        feeBps: "0",
        spreadBps: "0",
        slippageBps: "0",
        epsRel: "0",
      }),
    ).toThrow(/metricContract=full_17 conflicts with includeTradeMetrics=true/i);
  });

  it("rejects extra shortlist objectives when score mode is enabled", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DoubleMa",
          data_id: "data-extra-shortlist-score-mode",
          current_data_meta: { has_close: true },
        },
        backend: "GpuKernel",
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        scoreMode: "BalancedRiskAdjusted10",
        extraShortlistObjectives: "Pnl,Omega",
        mode: "Grid",
        topK: "20",
        includeAll: "false",
        heatmapBins: "0",
        startingEquity: "10000",
        costPreset: "custom",
        feeBps: "0",
        spreadBps: "0",
        slippageBps: "0",
        epsRel: "0",
      }),
    ).toThrow(/objective only/i);
  });

  it("rejects extra shortlist objectives when includeAll=true", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DoubleMa",
          data_id: "data-extra-shortlist-include-all",
          current_data_meta: { has_close: true },
        },
        backend: "GpuKernel",
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        extraShortlistObjectives: "Pnl,Omega",
        mode: "Grid",
        topK: "20",
        includeAll: "true",
        heatmapBins: "0",
        startingEquity: "10000",
        costPreset: "custom",
        feeBps: "0",
        spreadBps: "0",
        slippageBps: "0",
        epsRel: "0",
      }),
    ).toThrow(/includeAll=false/i);
  });

  it("allows extra shortlist objectives for DynamicDoubleMa on the requested GpuKernel route", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DynamicDoubleMa",
        data_id: "data-extra-shortlist-dynamic",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      extraShortlistObjectives: "Pnl,Omega",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
      dynamicStrategyIr: { profile: "double_ma" },
    });
    expect(req).toBeTruthy();
    expect((req as Record<string, unknown>).extra_shortlist_objectives).toEqual([
      "Pnl",
      "Omega",
    ]);
  });

  it("treats legacy DynamicDoubleMa GPU aliases as the kernel preview route for exact-compatible controls", () => {
    for (const backend of ["GpuOnly", "GpuHybridKernel"] as const) {
      const req = buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DynamicDoubleMa",
          data_id: `data-extra-shortlist-dynamic-${backend}`,
          current_data_meta: { has_close: true },
        },
        backend,
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        extraShortlistObjectives: "Pnl",
        mode: "Grid",
        topK: "20",
        includeAll: "false",
        heatmapBins: "0",
        startingEquity: "10000",
        costPreset: "custom",
        feeBps: "0",
        spreadBps: "0",
        slippageBps: "0",
        epsRel: "0",
        dynamicStrategyIr: { profile: "double_ma" },
      });

      expect((req as Record<string, unknown>).backend).toEqual({
        backend: "GpuKernel",
        device_id: 0,
      });
      expect((req as Record<string, unknown>).extra_shortlist_objectives).toEqual([
        "Pnl",
      ]);
    }
  });

  it("still rejects extra shortlist objectives for DynamicDoubleMa off the GpuKernel route", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DynamicDoubleMa",
          data_id: "data-extra-shortlist-dynamic-cpu",
          current_data_meta: { has_close: true },
        },
        backend: "CpuOnly",
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        extraShortlistObjectives: "Pnl,Omega",
        mode: "Grid",
        topK: "20",
        includeAll: "false",
        heatmapBins: "0",
        startingEquity: "10000",
        costPreset: "custom",
        feeBps: "0",
        spreadBps: "0",
        slippageBps: "0",
        epsRel: "0",
        dynamicStrategyIr: { profile: "double_ma" },
      }),
    ).toThrow(/supported only for the GPU kernel route/i);
  });

  it("normalizes shortlist confidence mode into the runtime request", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-shortlist-confidence",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      shortlistConfidenceMode: "DSR",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      objective: "Sharpe",
      shortlist_confidence_mode: "dsr",
    });
  });

  it("rejects shortlist confidence mode when score mode is enabled", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DoubleMa",
          data_id: "data-shortlist-confidence-score-mode",
          current_data_meta: { has_close: true },
        },
        backend: "GpuKernel",
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        scoreMode: "BalancedRiskAdjusted10",
        shortlistConfidenceMode: "psr",
        mode: "Grid",
        topK: "20",
        includeAll: "false",
        heatmapBins: "0",
        startingEquity: "10000",
        costPreset: "custom",
        feeBps: "0",
        spreadBps: "0",
        slippageBps: "0",
        epsRel: "0",
      }),
    ).toThrow(/objective only/i);
  });

  it("rejects shortlist confidence mode when includeAll=true", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DoubleMa",
          data_id: "data-shortlist-confidence-include-all",
          current_data_meta: { has_close: true },
        },
        backend: "GpuKernel",
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        shortlistConfidenceMode: "psr",
        mode: "Grid",
        topK: "20",
        includeAll: "true",
        heatmapBins: "0",
        startingEquity: "10000",
        costPreset: "custom",
        feeBps: "0",
        spreadBps: "0",
        slippageBps: "0",
        epsRel: "0",
      }),
    ).toThrow(/includeAll=false/i);
  });

  it("allows shortlist confidence mode for DynamicDoubleMa on the requested GpuKernel route", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DynamicDoubleMa",
        data_id: "data-shortlist-confidence-dynamic",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      shortlistConfidenceMode: "psr",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
      dynamicStrategyIr: { profile: "double_ma" },
    });
    expect(req).toBeTruthy();
    expect((req as Record<string, unknown>).shortlist_confidence_mode).toBe(
      "psr",
    );
  });

  it("normalizes shortlist union mode into the runtime request", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-shortlist-union",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      extraShortlistObjectives: "Pnl,Omega",
      shortlistUnionMode: "Pareto",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      extra_shortlist_objectives: ["Pnl", "Omega"],
      shortlist_union_mode: "pareto",
    });
  });

  it("normalizes weighted-rank shortlist union mode into the runtime request", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-shortlist-union-weighted-rank",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      extraShortlistObjectives: "Pnl,Omega",
      shortlistUnionMode: "weighted-rank",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      extra_shortlist_objectives: ["Pnl", "Omega"],
      shortlist_union_mode: "weighted_rank",
    });
  });

  it("rejects shortlist union mode without extra shortlist objectives", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DoubleMa",
          data_id: "data-shortlist-union-missing-heads",
          current_data_meta: { has_close: true },
        },
        backend: "GpuKernel",
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        shortlistUnionMode: "lexicographic",
        mode: "Grid",
        topK: "20",
        includeAll: "false",
        heatmapBins: "0",
        startingEquity: "10000",
        costPreset: "custom",
        feeBps: "0",
        spreadBps: "0",
        slippageBps: "0",
        epsRel: "0",
      }),
    ).toThrow(/requires extra shortlist objectives/i);
  });

  it("allows shortlist union mode for DynamicDoubleMa on the requested GpuKernel route", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DynamicDoubleMa",
        data_id: "data-shortlist-union-dynamic",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      extraShortlistObjectives: "Pnl",
      shortlistUnionMode: "epsilon_front",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
      dynamicStrategyIr: { profile: "double_ma" },
    });
    expect(req).toBeTruthy();
    expect((req as Record<string, unknown>).shortlist_union_mode).toBe(
      "epsilon_front",
    );
  });

  it("allows shortlist slice mode for DynamicDoubleMa on the requested GpuKernel route", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DynamicDoubleMa",
        data_id: "data-shortlist-slice-dynamic",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      shortlistSliceMode: "Halves",
      shortlistSliceBenchmark: "BuyAndHold",
      shortlistNeighborhoodRadius: "1",
      shortlistNeighborhoodStep: "2",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      dynamicStrategyIr: { profile: "double_ma" },
    });

    expect((req as Record<string, unknown>).shortlist_slice_mode).toBe(
      "halves",
    );
    expect((req as Record<string, unknown>).shortlist_slice_benchmark).toBe(
      "buy_and_hold",
    );
    expect(
      (req as Record<string, unknown>).shortlist_neighborhood_radius,
    ).toBe(1);
    expect((req as Record<string, unknown>).shortlist_neighborhood_step).toBe(
      2,
    );
  });

  it("normalizes shortlist slice diagnostics fields into the runtime request", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-shortlist-slice",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      extraShortlistObjectives: "Pnl,Omega",
      shortlistSliceMode: "Halves",
      shortlistSliceBenchmark: "BuyAndHold",
      shortlistNeighborhoodRadius: "1",
      shortlistNeighborhoodStep: "2",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "0",
      spreadBps: "0",
      slippageBps: "0",
      epsRel: "0",
    });

    expect(req).toMatchObject({
      extra_shortlist_objectives: ["Pnl", "Omega"],
      shortlist_slice_mode: "halves",
      shortlist_slice_benchmark: "buy_and_hold",
      shortlist_neighborhood_radius: 1,
      shortlist_neighborhood_step: 2,
    });
  });

  it("rejects shortlist slice benchmark without shortlist slice mode", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DoubleMa",
          data_id: "data-shortlist-slice-benchmark-missing-mode",
          current_data_meta: { has_close: true },
        },
        backend: "GpuKernel",
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        shortlistSliceBenchmark: "flat",
        mode: "Grid",
        topK: "20",
        includeAll: "false",
        heatmapBins: "0",
        startingEquity: "10000",
        costPreset: "custom",
        feeBps: "0",
        spreadBps: "0",
        slippageBps: "0",
        epsRel: "0",
      }),
    ).toThrow(/requires shortlist slice mode/i);
  });

  it("rejects unsupported DoubleMa score modes", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DoubleMa",
          data_id: "data-score-mode-invalid",
          current_data_meta: { has_close: true },
        },
        backend: "GpuKernel",
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        scoreMode: "mystery_mode",
        mode: "Grid",
        topK: "20",
        includeAll: "false",
        heatmapBins: "0",
        startingEquity: "10000",
        costPreset: "custom",
        feeBps: "0",
        spreadBps: "0",
        slippageBps: "0",
        epsRel: "0",
      }),
    ).toThrow(/Unsupported score mode/);
  });

  it("builds a DynamicDoubleMa optimization request with a validated dynamic strategy IR", () => {
    const dynamicStrategyIr = {
      schema_version: 2,
      profile: "DoubleMa",
      ma_source: "close",
      fast_ma: { ma_type: "sma", params: {} },
      slow_ma: { ma_type: "ema", params: {} },
      program: {
        indicators: [],
        conditions: [],
        groups: [],
        signals: {
          long_entry_group: null,
          long_exit_group: null,
          short_entry_group: null,
          short_exit_group: null,
        },
      },
      execution: {
        strategy: {
          long_only: true,
          allow_flip: false,
          trade_on_next_bar: true,
          exit_mode: "cross",
        },
      },
    };
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DynamicDoubleMa",
        data_id: "data-dyn",
        current_data_meta: { has_close: true },
      },
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      dynamicStrategyIr,
    });
    expect(req).toBeTruthy();
    expect(req).toMatchObject({
      strategy_kind: "DynamicDoubleMa",
      dynamic_strategy_ir: dynamicStrategyIr,
    });
  });

  it("allows effective_sample_lcb_10 for DynamicDoubleMa on the exact GpuKernel request surface", () => {
    const dynamicStrategyIr = {
      schema_version: 2,
      profile: "DoubleMa",
      ma_source: "close",
      fast_ma: { ma_type: "sma", params: {} },
      slow_ma: { ma_type: "ema", params: {} },
      program: {
        indicators: [],
        conditions: [],
        groups: [],
        signals: {
          long_entry_group: null,
          long_exit_group: null,
          short_entry_group: null,
          short_exit_group: null,
        },
      },
      execution: {
        strategy: {
          long_only: true,
          allow_flip: false,
          trade_on_next_bar: true,
          exit_mode: "cross",
        },
      },
    };
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DynamicDoubleMa",
        data_id: "data-dyn-score",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      scoreMode: "effective_sample_lcb_10",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      dynamicStrategyIr,
    });
    expect(req).toBeTruthy();
    expect((req as Record<string, unknown>).score_mode).toBe(
      "effective_sample_lcb_10",
    );
    expect((req as Record<string, unknown>).dynamic_strategy_ir).toEqual(
      dynamicStrategyIr,
    );
  });

  it("normalizes removed approximate-mode aliases back to exact Grid mode", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-123",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "GpuProgressive",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      exportAllCsvPath: "",
      vdpsEta: "4",
      vdpsRungs: "0.125,0.25,0.5,1.0",
      vdpsMinInitialPairs: "1024",
      vdpsMaxSurvivorSeeds: "128",
      vdpsMaxPeriodCandidates: "256",
      vdpsRandomInjectionRatio: "0.1",
      vdpsEliteMutationRatio: "0.5",
      vdpsDifferentialRatio: "0.4",
      vdpsDeterministic: true,
      vdpsSeed: "7",
      vdpsQualityCheckMaxPairs: "0",
      vdpsQualityCheckTopK: "20",
    });

    expect(req).toBeTruthy();
    expect(req?.mode).toBe("Grid");
  });

  it("preserves non-grid double-ma modes when exact-grid outputs are disabled", () => {
    const coarseToFineReq = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-ctf",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "CoarseToFine",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      exportAllCsvPath: "",
    });
    const autoReq = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-auto",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "Auto",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      exportAllCsvPath: "",
    });
    const vdpsReq = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-vdps",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "GpuVdps",
      topK: "20",
      includeAll: "false",
      heatmapBins: "0",
      exportAllCsvPath: "",
    });

    expect(coarseToFineReq?.mode).toBe("CoarseToFine");
    expect(autoReq?.mode).toBe("Auto");
    expect(vdpsReq?.mode).toBe("GpuVdps");
    expect(coarseToFineReq?.gpu_execution).toBe("PreferGpuResident");
    expect(autoReq?.gpu_execution).toBe("PreferGpuResident");
    expect(vdpsReq?.gpu_execution).toBe("PreferGpuResident");
  });

  it("pins non-grid double-ma modes back to Grid when exact-grid outputs are requested", () => {
    const heatmapReq = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-heatmap",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "CoarseToFine",
      topK: "20",
      includeAll: "false",
      heatmapBins: "64",
      exportAllCsvPath: "",
    });
    const includeAllReq = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-include-all",
        current_data_meta: { has_close: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "GpuVdps",
      topK: "20",
      includeAll: "true",
      heatmapBins: "0",
      exportAllCsvPath: "",
    });

    expect(heatmapReq?.mode).toBe("Grid");
    expect(includeAllReq?.mode).toBe("Grid");
  });

  it("preserves Marrt strategy kind on the double-ma request path", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "Marrt",
        data_id: "data-ribbon",
        current_data_meta: { has_close: true, has_volume: true },
      },
      maSupportMap: {
        alma: {
          supports_cpu: true,
          supports_cuda: true,
          supports_cuda_kernel: true,
        },
        vwma: {
          supports_cpu: true,
          supports_cuda: true,
          supports_cuda_kernel: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "10",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "20",
      fastMaType: "alma",
      slowMaType: "vwma",
      maSource: "close",
      objective: "Sharpe",
      mode: "Grid",
      topK: "10",
      includeAll: "false",
      heatmapBins: "0",
      exitMode: "cross",
      cooldownBars: "0",
      signalPersistenceBars: "1",
    });

    expect(req).toBeTruthy();
    expect(req?.strategy_kind).toBe("Marrt");
  });

  it("preserves Set strategy kind and emits slope-efficiency config on the double-ma path", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "Set",
        data_id: "data-set",
        current_data_meta: { has_close: true, has_high: true, has_low: true },
      },
      maSupportMap: {
        sma: {
          supports_cpu: true,
          supports_cuda: true,
          supports_cuda_kernel: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "10",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "20",
      fastMaType: "sma",
      slowMaType: "sma",
      maSource: "close",
      exitMode: "cross",
      cooldownBars: "0",
      signalPersistenceBars: "1",
    });

    expect(req?.strategy_kind).toBe("Set");
    expect(req?.strategy).toMatchObject({
      slope_efficiency_trend_enable: true,
      set_linearreg_period: 34,
      set_angle_min_deg: 2.0,
      set_er_period: 14,
      set_er_min: 0.3,
      set_chop_period: 14,
      set_chop_max: 45.0,
    });
  });

  it("preserves Pit strategy kind and emits pressure-impulse config on the double-ma path", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "Pit",
        data_id: "data-pit",
        current_data_meta: {
          has_open: true,
          has_close: true,
          has_high: true,
          has_low: true,
          has_volume: true,
        },
      },
      maSupportMap: {
        sma: {
          supports_cpu: true,
          supports_cuda: true,
          supports_cuda_kernel: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "10",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "20",
      fastMaType: "sma",
      slowMaType: "sma",
      maSource: "close",
      pitEfiPeriod: "11",
      pitVoscShortPeriod: "4",
      pitVoscLongPeriod: "12",
      pitCfoPeriod: "7",
      pitCfoScalar: "80",
      pitConfirmBars: "3",
      exitMode: "cross",
      cooldownBars: "0",
      signalPersistenceBars: "1",
    });

    expect(req?.strategy_kind).toBe("Pit");
    expect(req?.strategy).toMatchObject({
      pressure_impulse_trend_enable: true,
      pit_efi_period: 11,
      pit_vosc_short_period: 4,
      pit_vosc_long_period: 12,
      pit_cfo_period: 7,
      pit_cfo_scalar: 80,
      pit_confirm_bars: 3,
    });
  });

  it("preserves Mts strategy kind and emits momentum-stack config on the double-ma path", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "Mts",
        data_id: "data-mts",
        current_data_meta: { has_close: true },
      },
      maSupportMap: {
        sma: {
          supports_cpu: true,
          supports_cuda: true,
          supports_cuda_kernel: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "10",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "20",
      fastMaType: "sma",
      slowMaType: "sma",
      maSource: "close",
      exitMode: "cross",
      cooldownBars: "0",
      signalPersistenceBars: "1",
    });

    expect(req?.strategy_kind).toBe("Mts");
    expect(req?.strategy).toMatchObject({
      momentum_triple_stack_enable: true,
      mts_roc_period: 10,
      mts_mom_period: 20,
      mts_ppo_fast_period: 12,
      mts_ppo_slow_period: 26,
    });
  });

  it("preserves Orc strategy kind and emits oscillator-reversion config on the double-ma path", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "Orc",
        data_id: "data-orc",
        current_data_meta: { has_close: true, has_high: true, has_low: true },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "9",
      fastStep: "2",
      slowStart: "20",
      slowEnd: "30",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
    });

    expect(req?.strategy_kind).toBe("Orc");
    expect(req?.strategy).toMatchObject({
      oscillator_reversion_confluence_enable: true,
      orc_rsi_period: 14,
      orc_stoch_fastk_period: 5,
      orc_stoch_fastd_period: 3,
      orc_willr_period: 14,
    });
  });

  it("rejects unsupported MA types for Marrt", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "Marrt",
          data_id: "data-ribbon",
          current_data_meta: { has_close: true },
        },
        fastStart: "5",
        fastEnd: "10",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "20",
        fastMaType: "kama",
        slowMaType: "alma",
        maSource: "close",
      }),
    ).toThrow(
      "MARRT currently supports only sma, ema, alma, wma, zlema, and vwma.",
    );
  });

  it("preserves Vfct strategy kind and emits volume-flow config on the double-ma path", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "Vfct",
        data_id: "data-vfct",
        current_data_meta: {
          has_close: true,
          has_high: true,
          has_low: true,
          has_volume: true,
        },
      },
      maSupportMap: {
        vwma: {
          supports_cpu: true,
          supports_cuda: true,
          supports_cuda_kernel: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "10",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "20",
      fastMaType: "vwma",
      slowMaType: "vwma",
      maSource: "close",
      vfctObvSmoothPeriod: "21",
      vfctMfiPeriod: "14",
      vfctMfiLongMin: "55",
      vfctMfiShortMax: "45",
      vfctConfirmBars: "2",
      exitMode: "cross",
      cooldownBars: "0",
      signalPersistenceBars: "1",
    });

    expect(req?.strategy_kind).toBe("Vfct");
    expect(req?.strategy).toMatchObject({
      volume_flow_confirm_enable: true,
      volume_flow_obv_smooth_period: 21,
      volume_flow_mfi_period: 14,
      volume_flow_mfi_long_min: 55,
      volume_flow_mfi_short_max: 45,
      volume_flow_confirm_bars: 2,
    });
  });

  it("rejects unsupported MA types for Vfct", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "Vfct",
          data_id: "data-vfct",
          current_data_meta: {
            has_close: true,
            has_high: true,
            has_low: true,
            has_volume: true,
          },
        },
        fastStart: "5",
        fastEnd: "10",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "20",
        fastMaType: "ema",
        slowMaType: "vwma",
        maSource: "close",
      }),
    ).toThrow("VFCT currently supports only vwma fast/slow trend lines.");
  });

  it("normalizes legacy GpuHybridKernel backend to unified GpuKernel payload", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-hybrid",
        current_data_meta: { has_close: true },
      },
      maSupportMap: {
        sma: {
          supports_cpu: true,
          supports_cuda: true,
          supports_cuda_kernel: true,
        },
        alma: {
          supports_cpu: true,
          supports_cuda: true,
          supports_cuda_kernel: true,
        },
      },
      backend: "GpuHybridKernel",
      fastStart: "5",
      fastEnd: "7",
      fastStep: "1",
      slowStart: "10",
      slowEnd: "12",
      slowStep: "1",
      fastMaType: "sma",
      slowMaType: "alma",
      maSource: "close",
      objective: "Sharpe",
      mode: "Grid",
      topK: "10",
      includeAll: "false",
      heatmapBins: "0",
      exportAllCsvPath: "",
      exitMode: "cross",
      cooldownBars: "0",
      signalPersistenceBars: "1",
    });
    expect(req).toBeTruthy();
    expect(req?.backend).toEqual({ backend: "GpuKernel", device_id: 0 });
  });

  it("enforces includeAll combo guardrail", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DoubleMa",
          data_id: "data-123",
          current_data_meta: { has_close: true },
        },
        backend: "Auto",
        fastStart: "1",
        fastEnd: "2000",
        fastStep: "1",
        slowStart: "2",
        slowEnd: "4000",
        slowStep: "1",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        includeAll: "true",
        fastMaParamSweeps: { alpha: [1, 100, 1] },
        slowMaParamSweeps: { beta: [1, 100, 1] },
      }),
    ).toThrow("includeAll=true would return");
  });

  it("rejects heatmap bins above the exact-grid cap", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DoubleMa",
          data_id: "data-heatmap-cap",
          current_data_meta: { has_close: true },
        },
        backend: "GpuKernel",
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        mode: "Grid",
        topK: "20",
        includeAll: "false",
        heatmapBins: "513",
        exportAllCsvPath: "",
      }),
    ).toThrow("heatmapBins must be <= 512.");
  });

  it("rejects kernel top-k above 256 when includeAll=false on DoubleMa requests", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DoubleMa",
          data_id: "data-kernel-topk-cap",
          current_data_meta: { has_close: true },
        },
        backend: "GpuKernel",
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        mode: "Grid",
        topK: "257",
        includeAll: "false",
        heatmapBins: "0",
        exportAllCsvPath: "",
        exitMode: "cross",
        cooldownBars: "0",
        signalPersistenceBars: "1",
      }),
    ).toThrow(/topK <= 256 when includeAll=false/i);
  });

  it("rejects export-all CSV when includeAll=true on the DoubleMa path", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "DoubleMa",
          data_id: "data-export-all-include-all",
          current_data_meta: { has_close: true },
        },
        backend: "GpuKernel",
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        mode: "Grid",
        topK: "20",
        includeAll: "true",
        heatmapBins: "0",
        exportAllCsvPath: "all-rows.csv",
      }),
    ).toThrow(/exportAllCsvPath requires includeAll=false/i);
  });

  it("enforces the standalone CompositeCpu includeAll range guardrail", () => {
    expect(() =>
      buildCompositeCpuOptimizationRequest({
        context: {
          strategy_kind: "CompositeCpu",
          data_id: "data-composite-include-all-cap",
          current_data_meta: {
            has_open: true,
            has_high: true,
            has_low: true,
            has_close: true,
            has_volume: true,
          },
        },
        fastStart: "1",
        fastEnd: "2000",
        fastStep: "1",
        slowStart: "2",
        slowEnd: "4000",
        slowStep: "1",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        topK: "20",
        includeAll: "true",
        compositeSignalMode: "hard_gate",
      }),
    ).toThrow("includeAll=true would return");
  });

  it("builds a CompositeCpu optimization request from typed inputs", () => {
    const req = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: "CompositeCpu",
        data_id: "data-abc",
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
      },
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "CoarseToFine",
      topK: "20",
      includeAll: "false",
      compositeSignalMode: "hard_gate",
      halfTrendAmplitude: "2",
      halfTrendAtrPeriod: "100",
      halfTrendChannelDev: "2.0",
      maczFastLength: "12",
      maczSlowLength: "25",
      maczSignalLength: "9",
      maczLengthZ: "20",
      maczLengthStdev: "25",
      maczGamma: "0.02",
      maczA: "1",
      maczB: "1",
      maczUseVolume: false,
      maczUseLag: false,
      ottPeriod: "2",
      ottPercent: "1.4",
      ottMaType: "VAR",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "1",
      spreadBps: "2",
      slippageBps: "0.5",
      epsRel: "0.001",
      stopLossPct: "",
      takeProfitPct: "",
      cooldownBars: "0",
      signalPersistenceBars: "1",
      exitMode: "cross",
      kaufmanStopPeriod: "22",
      kaufmanStopMult: "2.0",
      kaufmanStopMaType: "sma",
      longOnly: true,
      allowFlip: true,
      tradeOnNext: false,
      fastMaParams: { alpha: 2 },
      slowMaParams: { alpha: 3 },
    });

    expect(req).toBeTruthy();
    expect(req).toMatchObject({
      strategy_kind: "CompositeCpu",
      data_id: "data-abc",
      backend: { backend: "CpuOnly" },
      mode: "Grid",
      top_k: 20,
      include_all: false,
      heatmap_bins: 0,
      signal_mode: "hard_gate",
      fast_ma_types: ["sma"],
      slow_ma_types: ["ema"],
      halftrend: {
        amplitude: 2,
        atr_period: 100,
        channel_deviation: 2,
      },
      strategy: {
        long_only: true,
        allow_flip: true,
        trade_on_next_bar: true,
      },
    });
  });

  it("builds a VCET optimization request from typed inputs", () => {
    const req = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: "Vcet",
        data_id: "data-vcet",
        current_data_meta: {
          has_high: true,
          has_low: true,
          has_close: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "vidya",
      slowMaType: "vidya",
      maSource: "close",
      objective: "Sharpe",
      mode: "Grid",
      topK: "12",
      includeAll: "false",
      vcetMapFastToVidyaShortPeriod: true,
      vcetMapSlowToVidyaLongPeriod: true,
      vcetMapSlowToCePeriod: true,
      vcetVidyaShortPeriod: "6",
      vcetVidyaLongPeriod: "24",
      vcetVidyaAlpha: "0.22",
      vcetCePeriod: "22",
      vcetCeMult: "3.2",
      vcetCeUseClose: true,
      vcetRequireVidyaSlope: true,
      vcetMaxTradeBars: "90",
    });

    expect(req).toBeTruthy();
    expect(req).toMatchObject({
      strategy_kind: "Vcet",
      data_id: "data-vcet",
      backend: { backend: "GpuKernel" },
      fast_ma_types: ["vidya"],
      slow_ma_types: ["vidya"],
      vcet: {
        map_fast_to_vidya_short_period: true,
        map_slow_to_vidya_long_period: true,
        map_slow_to_ce_period: true,
        vidya_short_period: 6,
        vidya_long_period: 24,
        vidya_alpha: 0.22,
        ce_period: 22,
        ce_mult: 3.2,
        ce_use_close: true,
        require_vidya_slope: true,
        max_trade_bars: 90,
      },
    });
  });

  const compositeDataMeta = {
    has_open: true,
    has_high: true,
    has_low: true,
    has_close: true,
    has_volume: true,
  };

  function buildCompositeDoubleMaRequest(
    strategyKind: "CompositeTrend" | "RegimeHybrid" | "BreakoutMomentum",
    overrides: Record<string, unknown> = {},
  ) {
    return buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: strategyKind,
        data_id: `data-${strategyKind}`,
        current_data_meta: compositeDataMeta,
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "64",
      exportAllCsvPath: "",
      trendUseAdx: false,
      breakoutUseAdx: false,
      ...overrides,
    });
  }

  function buildStandaloneCompositeGpuRequest(
    strategyKind:
      | "Asr"
      | "Vcc"
      | "Vemr"
      | "Rocb"
      | "Vcet"
      | "Kqsr"
      | "Carm"
      | "Kvdf"
      | "Mkcs"
      | "Cdmc",
    overrides: Record<string, unknown> = {},
  ) {
    return buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: strategyKind,
        data_id: `data-${strategyKind.toLowerCase()}`,
        current_data_meta: compositeDataMeta,
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      topK: "20",
      includeAll: "false",
      compositeSignalMode: "hard_gate",
      halfTrendAmplitude: "2",
      halfTrendAtrPeriod: "100",
      halfTrendChannelDev: "2.0",
      maczFastLength: "12",
      maczSlowLength: "25",
      maczSignalLength: "9",
      maczLengthZ: "20",
      maczLengthStdev: "25",
      maczGamma: "0.02",
      maczA: "1",
      maczB: "1",
      maczUseVolume: false,
      maczUseLag: false,
      ottPeriod: "2",
      ottPercent: "1.4",
      ottMaType: "VAR",
      ...overrides,
    });
  }

  it("builds a CompositeTrend request through the double-MA builder path with GPU kernel when ADX is off", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "CompositeTrend",
        data_id: "data-trend",
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
      },
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      backend: "GpuKernel",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "64",
      exportAllCsvPath: "",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "1",
      spreadBps: "2",
      slippageBps: "0.5",
      epsRel: "0.001",
      stopLossPct: "",
      takeProfitPct: "",
      cooldownBars: "0",
      signalPersistenceBars: "1",
      exitMode: "cross",
      kaufmanStopPeriod: "22",
      kaufmanStopMult: "2.0",
      kaufmanStopMaType: "sma",
      longOnly: true,
      allowFlip: true,
      tradeOnNext: false,
      trendUseRoc: true,
      trendUseAdx: false,
      trendLrPeriod: "34",
      trendLrMinDeg: "2.0",
      trendBbwPeriod: "20",
      trendBbwMult: "2.0",
      trendBbwMin: "0.04",
      trendRocPeriod: "9",
      trendAdxPeriod: "14",
      trendAdxMin: "20",
      fastMaParams: { alpha: 2 },
      slowMaParams: { alpha: 3 },
    });

    const strategy =
      req?.strategy && typeof req.strategy === "object"
        ? (req.strategy as Record<string, unknown>)
        : {};
    expect(req).toBeTruthy();
    expect(req?.strategy_kind).toBe("CompositeTrend");
    expect((req as Record<string, unknown>)?.backend).toEqual({
      backend: "GpuKernel",
      device_id: 0,
    });
    expect(strategy.trend_quality_enable).toBe(true);
    expect(strategy.regime_hybrid_enable).toBe(false);
    expect(strategy.breakout_momentum_enable).toBe(false);
  });

  it("builds a CompositeTrend request through the double-MA builder path with GPU hybrid when ADX is on", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "CompositeTrend",
        data_id: "data-trend",
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "64",
      exportAllCsvPath: "",
      trendUseRoc: true,
      trendUseAdx: true,
      trendLrPeriod: "34",
      trendLrMinDeg: "2.0",
      trendBbwPeriod: "20",
      trendBbwMult: "2.0",
      trendBbwMin: "0.04",
      trendRocPeriod: "9",
      trendAdxPeriod: "14",
      trendAdxMin: "20",
    });

    expect(req).toBeTruthy();
    expect((req as Record<string, unknown>)?.backend).toEqual({
      backend: "GpuHybridKernel",
      device_id: 0,
    });
  });

  it("builds RegimeHybrid and BreakoutMomentum through the double-MA builder path", () => {
    const regimeReq = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "RegimeHybrid",
        data_id: "data-regime",
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "64",
      exportAllCsvPath: "",
      regimeAdxPeriod: "14",
      regimeAdxEnter: "22",
      regimeAdxExit: "18",
      regimeRevEntryRel: "0.0035",
      regimeRevExitRel: "0.0012",
    });

    const breakoutReq = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "BreakoutMomentum",
        data_id: "data-breakout",
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "64",
      exportAllCsvPath: "",
      breakoutBbPeriod: "20",
      breakoutBbMult: "2.0",
      breakoutConfirmBars: "1",
      breakoutMacdFast: "12",
      breakoutMacdSlow: "26",
      breakoutMacdSignal: "9",
      breakoutCmoPeriod: "14",
      breakoutCmoMinAbs: "20",
      breakoutAdoscShort: "3",
      breakoutAdoscLong: "10",
      breakoutUseAdx: false,
      breakoutAdxPeriod: "14",
      breakoutAdxMin: "20",
    });

    expect(regimeReq).toBeTruthy();
    expect((regimeReq as Record<string, unknown>)?.backend).toEqual({
      backend: "GpuHybridKernel",
      device_id: 0,
    });
    expect((regimeReq as Record<string, unknown>)?.strategy_kind).toBe(
      "RegimeHybrid",
    );

    expect(breakoutReq).toBeTruthy();
    expect((breakoutReq as Record<string, unknown>)?.backend).toEqual({
      backend: "GpuKernel",
      device_id: 0,
    });
    expect((breakoutReq as Record<string, unknown>)?.strategy_kind).toBe(
      "BreakoutMomentum",
    );
  });

  it("keeps CompositeTrend, RegimeHybrid, and BreakoutMomentum on the double-MA path with CpuOnly fallback when requested", () => {
    const baseContext = {
      current_data_meta: {
        has_open: true,
        has_high: true,
        has_low: true,
        has_close: true,
        has_volume: true,
      },
    };

    const trendReq = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "CompositeTrend",
        data_id: "data-trend-cpu",
        ...baseContext,
      },
      backend: "CpuOnly",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "64",
      trendUseAdx: false,
    });

    const regimeReq = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "RegimeHybrid",
        data_id: "data-regime-cpu",
        ...baseContext,
      },
      backend: "CpuOnly",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "64",
    });

    const breakoutReq = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "BreakoutMomentum",
        data_id: "data-breakout-cpu",
        ...baseContext,
      },
      backend: "CpuOnly",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "64",
      breakoutUseAdx: false,
    });

    expect((trendReq as Record<string, unknown>).backend).toEqual({
      backend: "CpuOnly",
    });
    expect((regimeReq as Record<string, unknown>).backend).toEqual({
      backend: "CpuOnly",
    });
    expect((breakoutReq as Record<string, unknown>).backend).toEqual({
      backend: "CpuOnly",
    });
    expect((trendReq as Record<string, unknown>).heatmap_bins).toBe(64);
    expect((trendReq as Record<string, unknown>).top_k).toBe(20);
    expect((trendReq as Record<string, unknown>).include_all).toBe(false);
  });

  it("still allows include-all and large top-k on CpuOnly composite requests", () => {
    const req = buildCompositeDoubleMaRequest("CompositeTrend", {
      backend: "CpuOnly",
      topK: "257",
      includeAll: "true",
      heatmapBins: "64",
    });

    expect((req as Record<string, unknown>).backend).toEqual({
      backend: "CpuOnly",
    });
    expect((req as Record<string, unknown>).top_k).toBe(257);
    expect((req as Record<string, unknown>).include_all).toBe(true);
    expect((req as Record<string, unknown>).heatmap_bins).toBe(64);
  });

  it("uses route-aware guardrails so hybrid composite requests can keep kaufman-stop and execution overlays", () => {
    const req = buildCompositeDoubleMaRequest("CompositeTrend", {
      trendUseAdx: true,
      exitMode: "kaufman_stop",
      cooldownBars: "3",
      signalPersistenceBars: "2",
    });

    expect(req).toBeTruthy();
    expect((req as Record<string, unknown>).backend).toEqual({
      backend: "GpuHybridKernel",
      device_id: 0,
    });
  });

  it("makes exportAllCsvPath follow the composite route target instead of the requested GPU token", () => {
    const kernelReq = buildCompositeDoubleMaRequest("CompositeTrend", {
      trendUseAdx: false,
      exportAllCsvPath: "trend.csv",
    });
    const hybridReq = () =>
      buildCompositeDoubleMaRequest("CompositeTrend", {
        trendUseAdx: true,
        exportAllCsvPath: "trend-adx.csv",
      });
    const regimeReq = () =>
      buildCompositeDoubleMaRequest("RegimeHybrid", {
        exportAllCsvPath: "regime.csv",
      });
    const cpuReq = () =>
      buildCompositeDoubleMaRequest("CompositeTrend", {
        backend: "CpuOnly",
        exportAllCsvPath: "trend-cpu.csv",
      });

    expect(kernelReq).toBeTruthy();
    expect((kernelReq as Record<string, unknown>).export_all_csv_path).toBe(
      "trend.csv",
    );
    expect(hybridReq).toThrow(
      /exportAllCsvPath is currently supported only for the GPU kernel route/,
    );
    expect(regimeReq).toThrow(
      /exportAllCsvPath is currently supported only for the GPU kernel route/,
    );
    expect(cpuReq).toThrow(
      /exportAllCsvPath is currently supported only for the GPU kernel route/,
    );
  });

  it("resolves Auto on the composite trio to the current target route preview", () => {
    const trendKernel = buildCompositeDoubleMaRequest("CompositeTrend", {
      backend: "Auto",
      trendUseAdx: false,
    });
    const trendHybrid = buildCompositeDoubleMaRequest("CompositeTrend", {
      backend: "Auto",
      trendUseAdx: true,
    });
    const regimeHybrid = buildCompositeDoubleMaRequest("RegimeHybrid", {
      backend: "Auto",
    });
    const breakoutKernel = buildCompositeDoubleMaRequest("BreakoutMomentum", {
      backend: "Auto",
      breakoutUseAdx: false,
    });
    const breakoutHybrid = buildCompositeDoubleMaRequest("BreakoutMomentum", {
      backend: "Auto",
      breakoutUseAdx: true,
    });

    expect((trendKernel as Record<string, unknown>).backend).toEqual({
      backend: "GpuKernel",
      device_id: 0,
    });
    expect((trendHybrid as Record<string, unknown>).backend).toEqual({
      backend: "GpuHybridKernel",
      device_id: 0,
    });
    expect((regimeHybrid as Record<string, unknown>).backend).toEqual({
      backend: "GpuHybridKernel",
      device_id: 0,
    });
    expect((breakoutKernel as Record<string, unknown>).backend).toEqual({
      backend: "GpuKernel",
      device_id: 0,
    });
    expect((breakoutHybrid as Record<string, unknown>).backend).toEqual({
      backend: "GpuHybridKernel",
      device_id: 0,
    });
  });

  it("applies the kernel topK preflight only when the resolved route stays on the kernel path", () => {
    const kernelReq = () =>
      buildCompositeDoubleMaRequest("CompositeTrend", {
        trendUseAdx: false,
        topK: "257",
      });
    const hybridReq = buildCompositeDoubleMaRequest("CompositeTrend", {
      trendUseAdx: true,
      topK: "257",
    });
    const cpuReq = buildCompositeDoubleMaRequest("CompositeTrend", {
      backend: "CpuOnly",
      topK: "257",
    });

    expect(kernelReq).toThrow(/topK <= 256/);
    expect(hybridReq).toBeTruthy();
    expect(cpuReq).toBeTruthy();
  });

  it("allows advanced exact-kernel shortlist and trade-metric controls on ADX-off composite kernel routes", () => {
    const trendReq = buildCompositeDoubleMaRequest("CompositeTrend", {
      trendUseAdx: false,
      extraShortlistObjectives: "Pnl,Omega",
      includeTradeMetrics: true,
      heatmapBins: "0",
    });
    const breakoutReq = buildCompositeDoubleMaRequest("BreakoutMomentum", {
      breakoutUseAdx: false,
      extraShortlistObjectives: "Pnl,Omega",
      includeTradeMetrics: true,
      heatmapBins: "0",
    });

    expect((trendReq as Record<string, unknown>).backend).toEqual({
      backend: "GpuKernel",
      device_id: 0,
    });
    expect((trendReq as Record<string, unknown>).extra_shortlist_objectives).toEqual([
      "Pnl",
      "Omega",
    ]);
    expect((trendReq as Record<string, unknown>).include_trade_metrics).toBe(
      true,
    );

    expect((breakoutReq as Record<string, unknown>).backend).toEqual({
      backend: "GpuKernel",
      device_id: 0,
    });
    expect(
      (breakoutReq as Record<string, unknown>).extra_shortlist_objectives,
    ).toEqual(["Pnl", "Omega"]);
    expect((breakoutReq as Record<string, unknown>).include_trade_metrics).toBe(
      true,
    );
  });

  it("rejects advanced exact-kernel controls on composite routes once ADX promotes them to hybrid", () => {
    expect(() =>
      buildCompositeDoubleMaRequest("CompositeTrend", {
        trendUseAdx: true,
        extraShortlistObjectives: "Pnl",
      }),
    ).toThrow(/supported only for the GPU kernel route/i);

    expect(() =>
      buildCompositeDoubleMaRequest("BreakoutMomentum", {
        breakoutUseAdx: true,
        shortlistSliceMode: "Halves",
      }),
    ).toThrow(/supported only for the GPU kernel route/i);
  });

  it("allows metric-contract requests on non-exact composite routes while keeping exact-only shortlist controls gated", () => {
    const hybridReq = buildCompositeDoubleMaRequest("CompositeTrend", {
      trendUseAdx: true,
      includeTradeMetrics: true,
      metricContract: "trade_22",
    });
    const cpuReq = buildCompositeDoubleMaRequest("RegimeHybrid", {
      backend: "CpuOnly",
      metricContract: "full_17",
    });

    expect((hybridReq as Record<string, unknown>).include_trade_metrics).toBe(true);
    expect((hybridReq as Record<string, unknown>).metric_contract).toBe("trade_22");
    expect((cpuReq as Record<string, unknown>).metric_contract).toBe("full_17");
    expect((cpuReq as Record<string, unknown>).include_trade_metrics).toBe(false);
  });

  it("enforces composite strategy data prerequisites before route shaping", () => {
    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "CompositeTrend",
          data_id: "data-trend-missing-high-low",
          current_data_meta: { has_close: true },
        },
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        backend: "GpuKernel",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        trendUseAdx: true,
      }),
    ).toThrow(/CompositeTrend with ADX enabled requires high and low candle fields/i);

    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "RegimeHybrid",
          data_id: "data-regime-missing-high-low",
          current_data_meta: { has_close: true, has_volume: true },
        },
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        backend: "GpuKernel",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
      }),
    ).toThrow(/RegimeHybrid requires high and low candle fields/i);

    expect(() =>
      buildDoubleMaOptimizationRequest({
        context: {
          strategy_kind: "BreakoutMomentum",
          data_id: "data-breakout-missing-volume",
          current_data_meta: { has_high: true, has_low: true, has_close: true },
        },
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        backend: "GpuKernel",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
      }),
    ).toThrow(/BreakoutMomentum requires high, low, and volume candle fields/i);
  });

  it("accepts canonicalized MA support keys without cloning the whole support map", () => {
    const req = buildCompositeDoubleMaRequest("CompositeTrend", {
      backend: "Auto",
      maSupportMap: {
        SMA: {
          supports_cpu: true,
          supports_cuda: true,
          supports_cuda_kernel: true,
        },
        " EMA ": {
          supports_cpu: true,
          supports_cuda: true,
          supports_cuda_kernel: true,
        },
      },
    });

    expect(req).toBeTruthy();
    expect((req as Record<string, unknown>).backend).toEqual({
      backend: "GpuKernel",
      device_id: 0,
    });
  });

  it("keeps trade_on_next_bar fixed to true on the optimize request path", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-trade-next",
        current_data_meta: { has_close: true },
      },
      backend: "Auto",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      tradeOnNext: false,
    });

    expect((req as Record<string, unknown>).strategy).toMatchObject({
      trade_on_next_bar: true,
    });
  });

  it("still accepts the legacy GpuOnly alias on the double-MA request path", () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-gpu-only",
        current_data_meta: { has_close: true },
      },
      backend: "GpuOnly",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "Grid",
      topK: "20",
      includeAll: "false",
      heatmapBins: "64",
    });

    expect((req as Record<string, unknown>).backend).toEqual({
      backend: "GpuKernel",
      device_id: 0,
    });
  });

  it("builds a VCC request through the composite CPU builder path", () => {
    const req = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: "Vcc",
        data_id: "data-vcc",
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      topK: "20",
      includeAll: "false",
      compositeSignalMode: "hard_gate",
      halfTrendAmplitude: "2",
      halfTrendAtrPeriod: "100",
      halfTrendChannelDev: "2.0",
      maczFastLength: "12",
      maczSlowLength: "25",
      maczSignalLength: "9",
      maczLengthZ: "20",
      maczLengthStdev: "25",
      maczGamma: "0.02",
      maczA: "1",
      maczB: "1",
      maczUseVolume: false,
      maczUseLag: false,
      ottPeriod: "2",
      ottPercent: "1.4",
      ottMaType: "VAR",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "1",
      spreadBps: "2",
      slippageBps: "0.5",
      epsRel: "0.001",
      stopLossPct: "",
      takeProfitPct: "",
      cooldownBars: "0",
      signalPersistenceBars: "1",
      exitMode: "cross",
      kaufmanStopPeriod: "22",
      kaufmanStopMult: "2.0",
      kaufmanStopMaType: "sma",
      longOnly: true,
      allowFlip: true,
      tradeOnNext: false,
      vccMapFastToSqueezePeriod: false,
      vccMapFastToMomentumPeriod: true,
      vccMapSlowToTrendPeriod: false,
      vccSqueezePeriod: "18",
      vccMomentumPeriod: "22",
      vccMomentumBbPeriod: "24",
      vccTrendPeriod: "9",
      vccSqueezeBbMult: "2.2",
      vccSqueezeKcMult: "1.7",
      vccConfirmBars: "3",
      vccRequireMomentumAccel: false,
      vccMaxTradeBars: "140",
      fastMaParams: { alpha: 2 },
      slowMaParams: { alpha: 3 },
    });

    expect(req).toBeTruthy();
    expect(req?.strategy_kind).toBe("Vcc");
    expect(req?.backend).toEqual({ backend: "GpuKernel", device_id: 0 });
    expect(req?.mode).toBe("Grid");
    expect(req?.vcc).toMatchObject({
      map_fast_to_squeeze_period: false,
      map_fast_to_momentum_period: true,
      map_slow_to_trend_period: false,
      squeeze_period: 18,
      momentum_period: 22,
      momentum_bb_period: 24,
      trend_period: 9,
      squeeze_bb_mult: 2.2,
      squeeze_kc_mult: 1.7,
      confirm_bars: 3,
      require_momentum_accel: false,
      max_trade_bars: 140,
    });
  });

  it("builds an ASR request through the composite CPU builder path", () => {
    const req = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: "Asr",
        data_id: "data-asr",
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "alma",
      slowMaType: "alma",
      maSource: "close",
      objective: "Sharpe",
      topK: "20",
      includeAll: "false",
      compositeSignalMode: "hard_gate",
      asrSqueezeBbMult: "2.3",
      asrSqueezeKcMult: "1.7",
      asrAngleDegMin: "12",
      asrAngleDegMr: "7",
      asrUseErFilter: true,
      asrErMin: "0.35",
      asrErMr: "0.28",
      asrZMaType: "ema",
      asrZNbdev: "1.2",
      asrZDevtype: "1",
      asrZEntry: "2.4",
      asrZExit: "0.6",
      asrConfirmBars: "2",
      asrMomFadeBars: "3",
      asrMaxTradeBars: "90",
      asrMaxMrTradeBars: "32",
    });

    expect(req).toBeTruthy();
    expect(req?.strategy_kind).toBe("Asr");
    expect(req?.backend).toEqual({ backend: "GpuKernel", device_id: 0 });
    expect(req?.asr).toMatchObject({
      squeeze_bb_mult: 2.3,
      squeeze_kc_mult: 1.7,
      angle_deg_min: 12,
      angle_deg_mr: 7,
      use_er_filter: true,
      er_min: 0.35,
      er_mr: 0.28,
      z_ma_type: "ema",
      z_nbdev: 1.2,
      z_devtype: 1,
      z_entry: 2.4,
      z_exit: 0.6,
      confirm_bars: 2,
      mom_fade_bars: 3,
      max_trade_bars: 90,
      max_mr_trade_bars: 32,
    });
  });

  it("builds a VEMR request through the composite CPU builder path", () => {
    const req = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: "Vemr",
        data_id: "data-vemr",
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      mode: "CoarseToFine",
      topK: "20",
      includeAll: "false",
      compositeSignalMode: "hard_gate",
      halfTrendAmplitude: "2",
      halfTrendAtrPeriod: "100",
      halfTrendChannelDev: "2.0",
      maczFastLength: "12",
      maczSlowLength: "25",
      maczSignalLength: "9",
      maczLengthZ: "20",
      maczLengthStdev: "25",
      maczGamma: "0.02",
      maczA: "1",
      maczB: "1",
      maczUseVolume: false,
      maczUseLag: false,
      ottPeriod: "2",
      ottPercent: "1.4",
      ottMaType: "VAR",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "1",
      spreadBps: "2",
      slippageBps: "0.5",
      epsRel: "0.001",
      stopLossPct: "",
      takeProfitPct: "",
      cooldownBars: "0",
      signalPersistenceBars: "1",
      exitMode: "cross",
      kaufmanStopPeriod: "22",
      kaufmanStopMult: "2.0",
      kaufmanStopMaType: "sma",
      longOnly: true,
      allowFlip: true,
      tradeOnNext: false,
      vemrMapFastToZPeriod: false,
      vemrMapFastToBbPeriod: true,
      vemrMapSlowToAtrPeriod: false,
      vemrZPeriod: "18",
      vemrZMaType: "sma",
      vemrZNbdev: "1.2",
      vemrZDevtype: "0",
      vemrZEntry: "2.4",
      vemrZExit: "0.7",
      vemrBbPeriod: "21",
      vemrBbMult: "2.3",
      vemrRequireBandTouch: true,
      vemrAtrPeriod: "17",
      vemrAtrStopMult: "2.1",
      vemrConfirmBars: "2",
      vemrMaxTradeBars: "130",
      fastMaParams: { alpha: 2 },
      slowMaParams: { alpha: 3 },
    });

    expect(req).toBeTruthy();
    expect(req?.strategy_kind).toBe("Vemr");
    expect(req?.backend).toEqual({ backend: "GpuKernel", device_id: 0 });
    expect(req?.mode).toBe("Grid");
    expect(req?.vemr).toMatchObject({
      map_fast_to_z_period: false,
      map_fast_to_bb_period: true,
      map_slow_to_atr_period: false,
      z_period: 18,
      z_ma_type: "sma",
      z_nbdev: 1.2,
      z_devtype: 0,
      z_entry: 2.4,
      z_exit: 0.7,
      bb_period: 21,
      bb_mult: 2.3,
      require_band_touch: true,
      atr_period: 17,
      atr_stop_mult: 2.1,
      confirm_bars: 2,
      max_trade_bars: 130,
    });
  });

  it("builds a ROCB request through the composite CPU builder path", () => {
    const req = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: "Rocb",
        data_id: "data-rocb",
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      topK: "20",
      includeAll: "false",
      compositeSignalMode: "hard_gate",
      halfTrendAmplitude: "2",
      halfTrendAtrPeriod: "100",
      halfTrendChannelDev: "2.0",
      maczFastLength: "12",
      maczSlowLength: "25",
      maczSignalLength: "9",
      maczLengthZ: "20",
      maczLengthStdev: "25",
      maczGamma: "0.02",
      maczA: "1",
      maczB: "1",
      maczUseVolume: false,
      maczUseLag: false,
      ottPeriod: "2",
      ottPercent: "1.4",
      ottMaType: "VAR",
      rocbMapFastToChannelPeriod: false,
      rocbMapSlowToAtrPeriod: false,
      rocbChannelPeriod: "22",
      rocbAtrPeriod: "16",
      rocbAtrStopMult: "2.3",
      rocbMaxTradeBars: "90",
    });

    expect(req).toBeTruthy();
    expect(req?.strategy_kind).toBe("Rocb");
    expect(req?.backend).toEqual({ backend: "GpuKernel", device_id: 0 });
    expect(req?.rocb).toMatchObject({
      map_fast_to_channel_period: false,
      map_slow_to_atr_period: false,
      channel_period: 22,
      atr_period: 16,
      atr_stop_mult: 2.3,
      max_trade_bars: 90,
    });
  });

  it("builds an MKCS request through the composite CPU builder path", () => {
    const req = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: "Mkcs",
        data_id: "data-mkcs",
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      topK: "20",
      includeAll: "false",
      compositeSignalMode: "hard_gate",
      halfTrendAmplitude: "2",
      halfTrendAtrPeriod: "100",
      halfTrendChannelDev: "2.0",
      maczFastLength: "12",
      maczSlowLength: "25",
      maczSignalLength: "9",
      maczLengthZ: "20",
      maczLengthStdev: "25",
      maczGamma: "0.02",
      maczA: "1",
      maczB: "1",
      maczUseVolume: false,
      maczUseLag: false,
      ottPeriod: "2",
      ottPercent: "1.4",
      ottMaType: "VAR",
    });

    expect(req).toBeTruthy();
    expect(req?.strategy_kind).toBe("Mkcs");
    expect(req?.backend).toEqual({ backend: "GpuKernel", device_id: 0 });
    expect(req?.mkcs).toMatchObject({
      map_fast_to_stddev_period: true,
      map_slow_to_breakout_lookback: true,
      stddev_period: 20,
      breakout_lookback: 20,
      compression_threshold: 0.01,
      reexpansion_threshold: 0.03,
    });
  });

  it("builds the staged GPU-native expansion requests through the composite CPU builder path", () => {
    const cases = [
      {
        kind: "Kqsr",
        field: "kqsr",
        expected: {
          kdj_period: 14,
          srsi_period: 14,
          srsi_smooth: 3,
          ift_rsi_period: 9,
          qqe_period: 14,
          oversold: 20,
          overbought: 80,
          prb_period: 20,
          prb_threshold: 0.2,
        },
      },
      {
        kind: "Carm",
        field: "carm",
        expected: {
          coppock_long_period: 14,
          coppock_short_period: 11,
          coppock_wma_period: 10,
          ao_fast_period: 5,
          ao_slow_period: 34,
          rocr_period: 10,
          apo_fast_period: 12,
          apo_slow_period: 26,
          otto_period: 20,
          otto_threshold: 0,
        },
      },
      {
        kind: "Kvdf",
        field: "kvdf",
        expected: {
          keltner_period: 20,
          keltner_mult: 1.5,
          vwap_anchor: "session",
          fvg_lookback: 20,
          devstop_period: 14,
          devstop_mult: 2,
          dti_period: 14,
          dti_threshold: 20,
        },
      },
      {
        kind: "Cdmc",
        field: "cdmc",
        expected: {
          dpo_period: 20,
          correlation_cycle_period: 20,
          mama_fast_limit: 0.5,
          mama_slow_limit: 0.05,
          decycler_period: 20,
          phase_threshold: 0,
        },
      },
    ] as const;

    for (const row of cases) {
      const req = buildCompositeCpuOptimizationRequest({
        context: {
          strategy_kind: row.kind,
          data_id: `data-${row.kind.toLowerCase()}`,
          current_data_meta: {
            has_open: true,
            has_high: true,
            has_low: true,
            has_close: true,
            has_volume: true,
          },
        },
        backend: "GpuKernel",
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        topK: "20",
        includeAll: "false",
        compositeSignalMode: "hard_gate",
        halfTrendAmplitude: "2",
        halfTrendAtrPeriod: "100",
        halfTrendChannelDev: "2.0",
        maczFastLength: "12",
        maczSlowLength: "25",
        maczSignalLength: "9",
        maczLengthZ: "20",
        maczLengthStdev: "25",
        maczGamma: "0.02",
        maczA: "1",
        maczB: "1",
        maczUseVolume: false,
        maczUseLag: false,
        ottPeriod: "2",
        ottPercent: "1.4",
        ottMaType: "VAR",
      });

      expect(req).toBeTruthy();
      expect(req?.strategy_kind).toBe(row.kind);
      expect(req?.backend).toEqual({ backend: "GpuKernel", device_id: 0 });
      expect((req as Record<string, unknown>)[row.field]).toMatchObject(
        row.expected,
      );
    }
  });

  it("accepts standalone unified-GPU shortlist extras on requested GpuKernel routes when includeAll=true", () => {
    const req = buildStandaloneCompositeGpuRequest("Vemr", {
      includeAll: "true",
      extraShortlistObjectives: "Pnl,Omega",
      shortlistUnionMode: "weighted-rank",
      shortlistConfidenceMode: "DSR",
    });

    expect(req).toBeTruthy();
    expect(req?.strategy_kind).toBe("Vemr");
    expect(req?.backend).toEqual({ backend: "GpuKernel", device_id: 0 });
    expect((req as Record<string, unknown>).extra_shortlist_objectives).toEqual([
      "Pnl",
      "Omega",
    ]);
    expect((req as Record<string, unknown>).shortlist_union_mode).toBe(
      "weighted_rank",
    );
    expect((req as Record<string, unknown>).shortlist_confidence_mode).toBe(
      "dsr",
    );
    expect((req as Record<string, unknown>).include_all).toBe(true);
  });

  it("rejects standalone unified-GPU extra shortlist objectives without includeAll=true", () => {
    expect(() =>
      buildStandaloneCompositeGpuRequest("Asr", {
        extraShortlistObjectives: "Pnl,Omega",
        includeAll: "false",
      }),
    ).toThrow(/includeAll=true/i);
  });

  it("rejects standalone unified-GPU shortlist controls off the requested GpuKernel route", () => {
    expect(() =>
      buildStandaloneCompositeGpuRequest("Asr", {
        backend: "CpuOnly",
        includeAll: "true",
        extraShortlistObjectives: "Pnl",
      }),
    ).toThrow(/standalone unified-GPU GpuKernel routes/i);

    expect(() =>
      buildStandaloneCompositeGpuRequest("Asr", {
        backend: "CpuOnly",
        shortlistConfidenceMode: "psr",
      }),
    ).toThrow(/standalone unified-GPU GpuKernel routes/i);
  });

  it("rejects standalone shortlist extras on plain CompositeCpu even when a GPU backend is requested", () => {
    expect(() =>
      buildCompositeCpuOptimizationRequest({
        context: {
          strategy_kind: "CompositeCpu",
          data_id: "data-plain-composite",
          current_data_meta: {
            has_open: true,
            has_high: true,
            has_low: true,
            has_close: true,
            has_volume: true,
          },
        },
        backend: "GpuKernel",
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
        objective: "Sharpe",
        includeAll: "true",
        extraShortlistObjectives: "Pnl",
      }),
    ).toThrow(/standalone unified-GPU GpuKernel routes/i);
  });

  it("returns null for non-CompositeCpu strategy kind", () => {
    const req = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-123",
        current_data_meta: { has_high: true, has_low: true, has_close: true },
      },
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
    });
    expect(req).toBeNull();
  });

  it("rejects CompositeCpu optimization when the active dataset has fewer than two candles", () => {
    expect(() =>
      buildCompositeCpuOptimizationRequest({
        context: {
          strategy_kind: "CompositeCpu",
          data_id: "data-composite-too-short",
          current_data_meta: {
            has_open: true,
            has_high: true,
            has_low: true,
            has_close: true,
            has_volume: true,
            len: 0,
            timeframe: "5m",
          },
        },
        fastStart: "5",
        fastEnd: "15",
        fastStep: "5",
        slowStart: "20",
        slowEnd: "40",
        slowStep: "10",
        fastMaType: "sma",
        slowMaType: "ema",
        maSource: "close",
      }),
    ).toThrow(/Loaded dataset has 0 candles at 5m\. Optimization needs at least 2 candles\./i);
  });

  function baseCompositeMoInputs(): Parameters<
    typeof buildCompositeMoOptimizationRequest
  >[0] {
    return {
      context: {
        strategy_kind: "CompositeMo",
        data_id: "data-mo",
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
      },
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      moObjectiveMin: "MaxDrawdown",
      compositeSignalMode: "hard_gate",
      moSampler: "Nsga2",
      moNsga2Seed: "1",
      moNsga2Population: "80",
      moNsga2Generations: "40",
      moNsga2CrossoverProb: "0.9",
      moNsga2SwappingProb: "0.5",
      moNsga2MutationProb: "",
      moUseRangeBuilder: false,
      moSearchSpace: "",
      startingEquity: "10000",
      costPreset: "custom",
      feeBps: "1",
      spreadBps: "2",
      slippageBps: "0.5",
      epsRel: "0.001",
      stopLossPct: "",
      takeProfitPct: "",
      cooldownBars: "0",
      signalPersistenceBars: "1",
      exitMode: "cross",
      kaufmanStopPeriod: "22",
      kaufmanStopMult: "2.0",
      kaufmanStopMaType: "sma",
      longOnly: true,
      allowFlip: true,
      tradeOnNext: false,
      fastMaParams: { alpha: 2 },
      slowMaParams: { alpha: 3 },
    };
  }

  function baseCompositeMoRangeBuilderInputs(): Parameters<
    typeof buildCompositeMoOptimizationRequest
  >[0] {
    return {
      ...baseCompositeMoInputs(),
      moSampler: "Random",
      moUseRangeBuilder: true,
      moHtAmpLow: "2",
      moHtAmpHigh: "2",
      moHtAmpStep: "1",
      moHtDevLow: "1.5",
      moHtDevHigh: "2.5",
      moHtDevStep: "0.5",
      moHtAtrLow: "20",
      moHtAtrHigh: "20",
      moHtAtrStep: "1",
      moMaczFastLow: "5",
      moMaczFastHigh: "5",
      moMaczFastStep: "1",
      moMaczSlowLow: "10",
      moMaczSlowHigh: "10",
      moMaczSlowStep: "1",
      moMaczSignalLow: "3",
      moMaczSignalHigh: "3",
      moMaczSignalStep: "1",
      moMaczLengthZLow: "5",
      moMaczLengthZHigh: "5",
      moMaczLengthZStep: "1",
      moMaczLengthStdevLow: "8",
      moMaczLengthStdevHigh: "8",
      moMaczLengthStdevStep: "1",
      moMaczGammaLow: "0.01",
      moMaczGammaHigh: "0.01",
      moMaczGammaStep: "0.01",
      moMaczALow: "1",
      moMaczAHigh: "1",
      moMaczAStep: "0.1",
      moMaczBLow: "1",
      moMaczBHigh: "1",
      moMaczBStep: "0.1",
      moMaczUseLagMode: "true",
      moMaczUseVolumeMode: "false",
      moOttPeriodLow: "2",
      moOttPeriodHigh: "2",
      moOttPeriodStep: "1",
      moOttPercentLow: "1",
      moOttPercentHigh: "1",
      moOttPercentStep: "0.1",
      moOttMaTypes: "SMA,EMA",
    };
  }

  it("builds a CompositeMo optimization request from typed inputs", () => {
    const req = buildCompositeMoOptimizationRequest(baseCompositeMoInputs());

    expect(req).toBeTruthy();
    expect(req).toMatchObject({
      strategy_kind: "CompositeMo",
      data_id: "data-mo",
      objective: "Sharpe",
      objectives: { maximize: "Sharpe", minimize: "MaxDrawdown" },
      fast_ma_type: "sma",
      slow_ma_type: "ema",
      signal_mode: "hard_gate",
      sampler: {
        sampler: "Nsga2",
      },
      base_strategy: {
        long_only: true,
        allow_flip: true,
        trade_on_next_bar: true,
      },
    });
    expect(req).not.toHaveProperty("strategy");
    expect(
      Array.isArray(
        (req as { search_space?: { params?: unknown[] } }).search_space?.params,
      ),
    ).toBe(true);
  });

  it("stringifies CompositeMo discrete numeric and bool categorical choices", () => {
    const req = buildCompositeMoOptimizationRequest(
      baseCompositeMoRangeBuilderInputs(),
    );

    const params = ((req?.search_space as { params?: Array<{ name: string; domain: unknown }> })
      ?.params ?? []) as Array<{ name: string; domain: unknown }>;
    const findDomain = (name: string) =>
      params.find((param) => String(param.name || "").toLowerCase() === name)
        ?.domain as Record<string, unknown> | undefined;

    expect(findDomain("halftrend_channel_deviation")).toEqual({
      Categorical: { choices: ["1.5", "2", "2.5"] },
    });
    expect(findDomain("macz_use_lag")).toEqual({
      Categorical: { choices: ["true"] },
    });
    expect(findDomain("macz_use_volume")).toEqual({
      Categorical: { choices: ["false"] },
    });
    expect(findDomain("ott_ma_type")).toEqual({
      Categorical: { choices: ["SMA", "EMA"] },
    });
  });

  it("rejects unsupported CompositeMo custom search-space params", () => {
    expect(() =>
      buildCompositeMoOptimizationRequest({
        ...baseCompositeMoInputs(),
        moSampler: "Random",
        moUseRangeBuilder: false,
        moSearchSpace: JSON.stringify({
          params: [{ name: "not_real", domain: { Bool: {} } }],
        }),
      }),
    ).toThrow(/Unsupported Composite MO search-space param/i);
  });

  it("rejects duplicate CompositeMo custom search-space params", () => {
    expect(() =>
      buildCompositeMoOptimizationRequest({
        ...baseCompositeMoInputs(),
        moSampler: "Random",
        moUseRangeBuilder: false,
        moSearchSpace: JSON.stringify({
          params: [
            { name: "ott_ma_type", domain: { Categorical: { choices: ["sma"] } } },
            { name: "ott_ma_type", domain: { Categorical: { choices: ["ema"] } } },
          ],
        }),
      }),
    ).toThrow(/Duplicate Composite MO search-space param/i);
  });

  it("caps default CompositeMo random trials for tiny discrete spaces", () => {
    const req = buildCompositeMoOptimizationRequest({
      ...baseCompositeMoInputs(),
      fastStart: "5",
      fastEnd: "5",
      fastStep: "1",
      slowStart: "20",
      slowEnd: "20",
      slowStep: "1",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      moObjectiveMin: "MaxDrawdown",
      compositeSignalMode: "hard_gate",
      moSampler: "Random",
      moRandomTrials: "",
      moUseRangeBuilder: false,
      moSearchSpace: JSON.stringify({
        params: [
          { name: "ott_ma_type", domain: { Categorical: { choices: ["sma", "ema"] } } },
          { name: "macz_use_lag", domain: "Bool" },
        ],
      }),
    });

    expect(req?.sampler).toEqual({
      sampler: "Random",
      config: { seed: 1, trials: 1 },
    });
  });

  it("omits default CompositeMo prescan device id and auto-expands MA pairs", () => {
    const req = buildCompositeMoOptimizationRequest({
      ...baseCompositeMoInputs(),
      moSampler: "Random",
      moUseRangeBuilder: false,
      moSearchSpace: JSON.stringify({
        params: [
          {
            name: "fast_ma_type",
            domain: { Categorical: { choices: ["sma", "ema"] } },
          },
          {
            name: "slow_ma_type",
            domain: { Categorical: { choices: ["ema", "wma"] } },
          },
          {
            name: "ott_ma_type",
            domain: { Categorical: { choices: ["sma"] } },
          },
        ],
      }),
      moPrescanEnable: true,
      moPrescanMaPairs: "",
    });

    const gpuPrescan = req?.gpu_prescan as
      | {
          device_id?: number;
          ma_pairs?: Array<{ fast_ma_type: string; slow_ma_type: string }>;
        }
      | undefined;
    expect(gpuPrescan).not.toHaveProperty("device_id");
    expect(gpuPrescan?.ma_pairs).toEqual([
      { fast_ma_type: "sma", slow_ma_type: "ema" },
      { fast_ma_type: "sma", slow_ma_type: "wma" },
      { fast_ma_type: "ema", slow_ma_type: "ema" },
      { fast_ma_type: "ema", slow_ma_type: "wma" },
    ]);
  });

  it("builds the explicit CompositeMo prescan contract without leaking global backend semantics", () => {
    const req = buildCompositeMoOptimizationRequest({
      ...baseCompositeMoInputs(),
      moSampler: "Random",
      moPrescanEnable: true,
      moPrescanTopK: "32",
      moPrescanWindow: "4",
      moPrescanPredictiveTopK: "6",
      moPrescanPredictiveStride: "12",
      moPrescanRequireGpu: true,
      moPrescanDeviceId: "3",
      moPrescanObjectives: "maximize",
      moPrescanMode: "CoarseToFine",
      moPrescanMaPairs: "sma,ema\nvidya,ema",
    });

    expect(req?.mode).toBeNull();
    expect(req?.gpu_prescan).toEqual({
      top_k: 32,
      window: 4,
      predictive_top_k: 6,
      predictive_stride: 12,
      device_id: 3,
      require_gpu: true,
      objectives: "maximize",
      ma_pairs: [
        { fast_ma_type: "sma", slow_ma_type: "ema" },
        { fast_ma_type: "vidya", slow_ma_type: "ema" },
      ],
      mode: "CoarseToFine",
    });
  });

  it("fails clearly when CompositeMo MA-pair auto-expansion would exceed eight pairs", () => {
    expect(() =>
      buildCompositeMoOptimizationRequest({
        ...baseCompositeMoInputs(),
        moSampler: "Random",
        moUseRangeBuilder: false,
        moSearchSpace: JSON.stringify({
          params: [
            {
              name: "fast_ma_type",
              domain: { Categorical: { choices: ["sma", "ema", "wma"] } },
            },
            {
              name: "slow_ma_type",
              domain: { Categorical: { choices: ["ema", "wma", "alma"] } },
            },
          ],
        }),
        moPrescanEnable: true,
        moPrescanMaPairs: "",
      }),
    ).toThrow(/more than 8 MA pairs/i);
  });

  it("caps tiny-space CompositeMo defaults without overriding explicit budgets", () => {
    const tinySpace = JSON.stringify({ params: [] });

    const randomDefault = buildCompositeMoOptimizationRequest({
      ...baseCompositeMoInputs(),
      moSampler: "Random",
      fastStart: "5",
      fastEnd: "5",
      fastStep: "1",
      slowStart: "20",
      slowEnd: "20",
      slowStep: "1",
      moUseRangeBuilder: false,
      moSearchSpace: tinySpace,
    });
    const randomExplicit = buildCompositeMoOptimizationRequest({
      ...baseCompositeMoInputs(),
      moSampler: "Random",
      fastStart: "5",
      fastEnd: "5",
      fastStep: "1",
      slowStart: "20",
      slowEnd: "20",
      slowStep: "1",
      moUseRangeBuilder: false,
      moSearchSpace: tinySpace,
      moRandomTrials: "333",
    });
    const tpeDefault = buildCompositeMoOptimizationRequest({
      ...baseCompositeMoInputs(),
      moSampler: "Tpe",
      fastStart: "5",
      fastEnd: "5",
      fastStep: "1",
      slowStart: "20",
      slowEnd: "20",
      slowStep: "1",
      moUseRangeBuilder: false,
      moSearchSpace: tinySpace,
    });

    expect(
      (randomDefault?.sampler as { config?: { trials?: number } })?.config
        ?.trials,
    ).toBe(1);
    expect(
      (randomExplicit?.sampler as { config?: { trials?: number } })?.config
        ?.trials,
    ).toBe(333);
    expect(
      (tpeDefault?.sampler as {
        config?: { trials?: number; startup_trials?: number };
      })?.config,
    ).toMatchObject({
      trials: 1,
      startup_trials: 0,
    });
  });

  it("builds the Qmc CompositeMo sampler contract with explicit sequence settings", () => {
    const req = buildCompositeMoOptimizationRequest({
      ...baseCompositeMoInputs(),
      moSampler: "Qmc",
      moQmcSeed: "17",
      moQmcTrials: "144",
      moQmcSequence: "halton",
    });

    expect(req?.sampler).toEqual({
      sampler: "Qmc",
      config: { seed: 17, trials: 144, sequence: "halton" },
    });
  });

  it("returns null for non-CompositeMo strategy kind", () => {
    const req = buildCompositeMoOptimizationRequest({
      context: {
        strategy_kind: "DoubleMa",
        data_id: "data-123",
        current_data_meta: { has_high: true, has_low: true, has_close: true },
      },
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
    });
    expect(req).toBeNull();
  });

  it("rejects CompositeMo optimization when the active dataset has fewer than two candles", () => {
    expect(() =>
      buildCompositeMoOptimizationRequest({
        ...baseCompositeMoInputs(),
        context: {
          strategy_kind: "CompositeMo",
          data_id: "data-mo-too-short",
          current_data_meta: {
            has_open: true,
            has_high: true,
            has_low: true,
            has_close: true,
            has_volume: true,
            len: 1,
            timeframe: "1h",
          },
        },
      }),
    ).toThrow(/Loaded dataset has 1 candle at 1h\. Optimization needs at least 2 candles\./i);
  });
});
