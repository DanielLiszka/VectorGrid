import { describe, expect, it } from "vitest";
import {
  buildValidationCapabilitySummary,
  buildValidationRequest,
} from "./validationRequestBuilder";

function buildTrialAccountingRequest(
  currentRunRes: unknown,
  adjusted?: unknown,
) {
  return buildValidationRequest({
    currentRunReq: {
      strategy_kind: "DoubleMa",
      data_id: "data-trial-accounting",
      ma_source: "close",
      fast_ma_types: ["sma"],
      slow_ma_types: ["ema"],
    },
    currentRunRes,
    runDatasetMeta: {
      csv_path: "C:/data/SPY.csv",
      timeframe: "1h",
      symbol: "SPY",
      adjusted,
    },
    runCsvPath: "C:/fallback.csv",
    topRows: [
      {
        params: {
          fast_ma_type: "sma",
          slow_ma_type: "ema",
          fast_len: 10,
          slow_len: 40,
        },
      },
    ],
  });
}

describe("validationRequestBuilder", () => {
  it("builds double-like validation request from top rows with gates and aux configs", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "DoubleMa",
        data_id: "data-123",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["alma"],
        strategy: { fee_bps: 1.5 },
        halftrend_filter: { enabled: true },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "alma",
            fast_len: 10,
            slow_len: 40,
            fast_ma_params: { alpha: 2.0 },
            slow_ma_params: { offset: 0.85 },
          },
        },
      ],
      topRowsFiltered: [],
      candidates: "32",
      seed: "7",
      minPsr: "0.6",
      minDsr: "0.4",
      spaEnable: true,
      spaSlices: "12",
      spaBootstrapSamples: "750",
      spaBlockLength: "5",
      reoptWfEnable: true,
      reoptWfWindow: "12000",
      reoptWfStep: "3000",
      reoptWfTopN: "5",
      wfEnable: true,
      wfWindow: "20000",
      wfStep: "5000",
      perturbEnable: true,
      perturbIters: "8",
      perturbMin: "0.01",
      perturbMax: "0.05",
      mcEnable: true,
      mcSims: "500",
      mcBlockLen: "9",
      cscvEnable: true,
      cscvSlices: "17",
      crossEnable: true,
      crossMax: "2",
      crossDatasets: [
        { csv_path: "C:/data/a.csv", label: "A" },
        { csv_path: "C:/data/b.csv", label: "B" },
        { csv_path: "C:/data/c.csv", label: "C" },
      ],
    });

    expect(req.data_id).toBe("data-123");
    expect(req.backend).toEqual({ backend: "GpuKernel", device_id: 0 });
    expect(req.seed).toBe(7);
    expect(req.min_psr).toBe(0.6);
    expect(req.min_dsr).toBe(0.4);
    expect(req.spa).toEqual({
      slices: 12,
      bootstrap_samples: 750,
      block_length: 5,
      benchmark: "buy_hold",
    });
    expect(req.reopt_walk_forward).toEqual({
      window_bars: 12000,
      step_bars: 3000,
      top_n_candidates: 5,
    });
    expect(req.candidates).toHaveLength(1);
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: "sma",
      slow_ma_type: "alma",
      fast_len: 10,
      slow_len: 40,
    });
    expect(req.walk_forward).toEqual({ window_bars: 20000, step_bars: 5000 });
    expect(req.perturb).toEqual({
      iterations: 8,
      pct_min: 0.01,
      pct_max: 0.05,
      flip_bool_prob: 0.02,
    });
    expect(req.trade_bootstrap).toEqual({
      simulations: 500,
      stationary_mean_block_len: 9,
    });
    expect(req.monte_carlo).toEqual({
      schema_version: 1,
      simulations: 500,
      modes: ["trade_iid", "trade_stationary"],
      stationary_mean_block_len: 9,
      distribution_bins: 40,
      tail_alpha: 0.05,
    });
    expect(req.cscv_pbo).toEqual({
      slices: 16,
      max_combinations: 50000,
      threshold: 0,
    });
    expect(req.cross_ticker).toEqual({
      max_datasets: 2,
      mode: "preserved_finalists",
      datasets: [
        { csv_path: "C:/data/a.csv", label: "A" },
        { csv_path: "C:/data/b.csv", label: "B" },
      ],
    });
  });

  it("bridges canonical and legacy optimization trial-count fields", () => {
    const cases: Array<[Record<string, unknown>, number]> = [
      [
        {
          num_combos: 123.9,
          numCombos: 999,
          total_evaluations: 1000,
          totalEvaluations: 1001,
        },
        123,
      ],
      [{ numCombos: "456" }, 456],
      [{ total_evaluations: "789.8" }, 789],
      [{ totalEvaluations: 321 }, 321],
      [
        {
          num_combos: 0,
          numCombos: "invalid",
          total_evaluations: 42,
        },
        42,
      ],
    ];

    for (const [currentRunRes, expected] of cases) {
      const req = buildTrialAccountingRequest(currentRunRes);
      expect(req.validation_context.optimization_trials).toBe(expected);
      expect(req.validation_context).toMatchObject({
        strategy_kind: "DoubleMa",
        csv_path: "C:/data/SPY.csv",
        timeframe: "1h",
        label: "SPY",
      });
    }
  });

  it("bridges a structurally valid full-search Sharpe distribution", () => {
    const searchDistribution = {
      metric_surface_kind: "core17",
      sharpe_metric_id: "double_ma_log_return_sharpe_v1",
      attempted_count: 600,
      unique_count: 550,
      feasible_count: 510,
      finite_sharpe_count: 500,
      finite_sharpe_mean: 0.15,
      finite_sharpe_sample_std: 0.4,
    };
    const req = buildTrialAccountingRequest({
      num_combos: 600,
      search_semantics: {
        search_distribution: searchDistribution,
      },
    });

    expect(req.validation_context.search_distribution).toEqual(
      searchDistribution,
    );
  });

  it("omits malformed or finalist-only Sharpe distributions", () => {
    const malformed = [
      {
        metric_surface_kind: "core17",
        sharpe_metric_id: "double_ma_log_return_sharpe_v1",
        attempted_count: 10,
        unique_count: 11,
        feasible_count: 9,
        finite_sharpe_count: 8,
        finite_sharpe_mean: 0.15,
        finite_sharpe_sample_std: 0.4,
      },
      {
        metric_surface_kind: "core17",
        sharpe_metric_id: "double_ma_log_return_sharpe_v1",
        attempted_count: 10,
        unique_count: 10,
        feasible_count: 9,
        finite_sharpe_count: 8,
        finite_sharpe_mean: Number.NaN,
        finite_sharpe_sample_std: 0.4,
      },
      null,
    ];

    for (const search_distribution of malformed) {
      const req = buildTrialAccountingRequest({
        num_combos: 10,
        search_semantics: {
          search_distribution,
          search_distribution_unavailable_reason:
            "only retained finalists were materialized",
        },
      });
      expect(req.validation_context).not.toHaveProperty("search_distribution");
    }
  });

  it("omits invalid optimization trial counts without changing validation context", () => {
    const invalidResults: Array<Record<string, unknown>> = [
      { num_combos: 0 },
      { num_combos: -1 },
      { num_combos: 0.5 },
      { num_combos: "" },
      { num_combos: true },
      { num_combos: Number.POSITIVE_INFINITY },
      { num_combos: Number.MAX_SAFE_INTEGER + 1 },
      {
        num_combos: "invalid",
        numCombos: null,
        total_evaluations: undefined,
        totalEvaluations: " ",
      },
    ];

    for (const currentRunRes of invalidResults) {
      const req = buildTrialAccountingRequest(currentRunRes);
      expect(req.validation_context).toEqual({
        strategy_kind: "DoubleMa",
        csv_path: "C:/data/SPY.csv",
        timeframe: "1h",
        label: "SPY",
      });
      expect(req.validation_context).not.toHaveProperty("optimization_trials");
    }
  });

  it("propagates only a known boolean price-adjustment state and leaves other realism evidence unknown", () => {
    const adjusted = buildTrialAccountingRequest({}, true);
    expect(adjusted.validation_context.data_realism).toEqual({
      price_adjustment: "adjusted",
      corporate_actions: "unknown",
      survivorship_bias: "unknown",
      liquidity_capacity: "unknown",
      market_impact: "unknown",
      borrow_availability: "unknown",
    });

    const unadjusted = buildTrialAccountingRequest({}, false);
    expect(unadjusted.validation_context.data_realism).toEqual({
      price_adjustment: "unadjusted",
      corporate_actions: "unknown",
      survivorship_bias: "unknown",
      liquidity_capacity: "unknown",
      market_impact: "unknown",
      borrow_availability: "unknown",
    });
  });

  it("does not coerce absent or non-boolean adjusted metadata into realism evidence", () => {
    for (const adjusted of [undefined, null, 0, 1, "true", "false"]) {
      const req = buildTrialAccountingRequest({}, adjusted);
      expect(req.validation_context).not.toHaveProperty("data_realism");
    }
  });

  it("blocks enabled cross-ticker validation until its dataset contract is valid", () => {
    const baseInputs = {
      currentRunReq: {
        strategy_kind: "DoubleMa",
        data_id: "data-cross-blocked",
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["ema"],
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "ema",
            fast_len: 10,
            slow_len: 40,
          },
        },
      ],
      crossEnable: true,
    };

    expect(() =>
      buildValidationRequest({
        ...baseInputs,
        crossMax: "6",
        crossDatasets: [],
      }),
    ).toThrow(/no eligible datasets are selected/i);
    expect(() =>
      buildValidationRequest({
        ...baseInputs,
        crossMax: "0",
        crossDatasets: [{ csv_path: "C:/data/SPY.csv", label: "SPY" }],
      }),
    ).toThrow(/Max datasets must be at least 1/i);
  });

  it("normalizes string false module toggles instead of enabling them", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "DoubleMa",
        data_id: "data-string-false",
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["ema"],
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "ema",
            fast_len: 10,
            slow_len: 40,
          },
        },
      ],
      wfEnable: "false",
      reoptWfEnable: "false",
      spaEnable: "false",
      perturbEnable: "false",
      mcEnable: "false",
      cscvEnable: "false",
      crossEnable: "false",
    });

    expect(req.walk_forward).toBeNull();
    expect(req.reopt_walk_forward).toBeNull();
    expect(req.spa).toBeNull();
    expect(req.perturb).toBeNull();
    expect(req.trade_bootstrap).toBeNull();
    expect(req.monte_carlo).toBeNull();
    expect(req.cscv_pbo).toBeNull();
    expect(req.cross_ticker).toBeNull();
  });

  it("builds composite request and enforces halftrend presence", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "CompositeMo",
        data_id: "data-456",
        ma_source: "hlc3",
        strategy: { eps_rel: 0.01 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 9,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: "soft_vote",
            strategy: { fee_bps: 2 },
          },
        },
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 7,
            slow_len: 25,
          },
        },
      ],
      candidates: "10",
    });

    expect(req.candidates).toHaveLength(1);
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: "ema",
      slow_ma_type: "sma",
      signal_mode: "soft_vote",
    });
  });

  it("defaults validation admission to legacy_topk and records provenance", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "DoubleMa",
        data_id: "data-admission-default",
        objective: "Sharpe",
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["ema"],
        strategy: { fee_bps: 1.5 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "ema",
            fast_len: 10,
            slow_len: 30,
          },
          metrics: {
            pnl: 0.12,
            sharpe: 1.1,
            calmar: 0.9,
            omega: 1.05,
            max_dd: 0.18,
          },
        },
      ],
      candidates: "8",
    });

    expect(req.admission_policy).toBe("legacy_topk");
    expect(req.admission_provenance).toMatchObject({
      requested_policy: "legacy_topk",
      applied_policy: "legacy_topk",
      source_kind: "top_rows",
      source_candidate_count: 1,
      post_cap_count: 1,
    });
  });

  it("builds a diversified scalar-union shortlist without changing the legacy default", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "DoubleMa",
        data_id: "data-scalar-union",
        objective: "Sharpe",
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["ema"],
        strategy: { fee_bps: 1.5 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "ema",
            fast_len: 10,
            slow_len: 40,
          },
          metrics: {
            pnl: 0.15,
            sharpe: 1.8,
            calmar: 0.7,
            omega: 1.02,
            max_dd: 0.32,
          },
        },
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "ema",
            fast_len: 12,
            slow_len: 48,
          },
          metrics: {
            pnl: 0.13,
            sharpe: 1.4,
            calmar: 0.85,
            omega: 1.08,
            max_dd: 0.24,
          },
        },
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "ema",
            fast_len: 8,
            slow_len: 26,
          },
          metrics: {
            pnl: 0.1,
            sharpe: 0.9,
            calmar: 1.6,
            omega: 1.35,
            max_dd: 0.06,
          },
        },
      ],
      candidates: "2",
      admissionPolicy: "scalar_union",
    });

    expect(req.admission_policy).toBe("scalar_union");
    expect(req.candidates).toHaveLength(2);
    expect(req.candidates[0]).toMatchObject({ fast_len: 10, slow_len: 40 });
    expect(req.candidates[1]).toMatchObject({ fast_len: 8, slow_len: 26 });
    expect(req.admission_provenance).toMatchObject({
      requested_policy: "scalar_union",
      applied_policy: "scalar_union",
      post_cap_count: 2,
    });
    expect(
      req.admission_provenance.heads.some(
        (head) => head.objective === "MaxDrawdown",
      ),
    ).toBe(true);
  });

  it("keeps scalar-union equal-score admission order stable by input index", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "DoubleMa",
        data_id: "data-scalar-union-ties",
        objective: "Sharpe",
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["ema"],
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "ema",
            fast_len: 6,
            slow_len: 24,
          },
          metrics: {
            sharpe: Number.NaN,
            omega: Number.NaN,
            calmar: Number.NaN,
            max_dd: Number.NaN,
          },
        },
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "ema",
            fast_len: 10,
            slow_len: 40,
          },
          metrics: { sharpe: 1.25, omega: 1.0, calmar: 0.8, max_dd: 0.2 },
        },
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "ema",
            fast_len: 12,
            slow_len: 48,
          },
          metrics: { sharpe: 1.25, omega: 0.9, calmar: 0.7, max_dd: 0.22 },
        },
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "ema",
            fast_len: 8,
            slow_len: 30,
          },
          metrics: { sharpe: Number.NaN, omega: 1.6, calmar: 1.4, max_dd: 0.1 },
        },
      ],
      candidates: "2",
      admissionPolicy: "scalar_union",
    });

    expect(req.candidates.map((row) => row.fast_len)).toEqual([10, 8]);
    expect(req.admission_provenance.heads[0]).toMatchObject({
      objective: "Sharpe",
      candidate_count: 2,
      note: null,
    });
  });

  it("uses pareto-frontier admission automatically for CompositeMo artifacts", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "CompositeMo",
        data_id: "data-composite-mo",
        ma_source: "hlc3",
        objectives: { maximize: "Sharpe", minimize: "MaxDrawdown" },
        strategy: { fee_bps: 1.0 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 9,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: "soft_vote",
            strategy: { fee_bps: 2 },
          },
          metrics: { pnl: 0.12, sharpe: 1.1, max_dd: 0.12 },
          mo: { maximize_value: 1.1, minimize_value: 0.12, violation: 0 },
          source_kind: "pareto_frontier",
        },
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 7,
            slow_len: 25,
            halftrend: { amplitude: 3 },
            macz: { fast_length: 10 },
            ott: { period: 3 },
            signal_mode: "hard_gate",
            strategy: { fee_bps: 2 },
          },
          metrics: { pnl: 0.1, sharpe: 1.0, max_dd: 0.08 },
          mo: { maximize_value: 1.0, minimize_value: 0.08, violation: 0 },
          source_kind: "pareto_frontier",
        },
      ],
      candidates: "4",
      admissionPolicy: "auto",
    });

    expect(req.admission_policy).toBe("pareto_frontier");
    expect(req.admission_provenance).toMatchObject({
      requested_policy: "auto",
      applied_policy: "pareto_frontier",
      source_kind: "pareto_frontier",
      post_cap_count: 2,
    });
    expect(req.candidates).toHaveLength(2);
  });

  it("diversity-trims dense CompositeMo frontiers when the cap is smaller than the frontier", () => {
    const buildParetoRow = (
      fastLen: number,
      maximize: number,
      minimize: number,
    ) => ({
      params: {
        fast_ma_type: "ema",
        slow_ma_type: "sma",
        fast_len: fastLen,
        slow_len: 40 + fastLen,
        halftrend: { amplitude: 2 },
        macz: { fast_length: 12 },
        ott: { period: 2 },
        signal_mode: "soft_vote",
        strategy: { fee_bps: 2 },
      },
      metrics: {
        pnl: 0.1 + fastLen / 1000,
        sharpe: maximize,
        max_dd: minimize,
      },
      mo: { maximize_value: maximize, minimize_value: minimize, violation: 0 },
      source_kind: "pareto_frontier",
    });

    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "CompositeMo",
        data_id: "data-composite-mo-dense-frontier",
        ma_source: "hlc3",
        objectives: { maximize: "Sharpe", minimize: "MaxDrawdown" },
        strategy: { fee_bps: 1.0 },
      },
      currentRunRes: { ok: true },
      topRows: [
        buildParetoRow(10, 1.5, 0.12),
        buildParetoRow(11, 1.4, 0.11),
        buildParetoRow(12, 1.3, 0.1),
        buildParetoRow(13, 1.2, 0.09),
        buildParetoRow(14, 1.1, 0.08),
      ],
      candidates: "3",
      admissionPolicy: "auto",
    });

    expect(req.admission_policy).toBe("pareto_frontier");
    expect(req.candidates).toHaveLength(3);
    expect(req.candidates.map((row) => row.fast_len)).toEqual([10, 12, 14]);
    expect(req.admission_provenance).toMatchObject({
      requested_policy: "auto",
      applied_policy: "pareto_frontier",
      post_dedupe_count: 5,
      post_cap_count: 3,
      note: "pareto_diversity_trim",
    });
    expect(req.admission_provenance.heads[0]?.note).toBe(
      "pareto_diversity_trim",
    );
  });

  it("dedupes normalized validation candidates before applying the max candidate limit", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "DoubleMa",
        data_id: "data-dedupe",
        ma_source: "close",
        fast_ma_types: ["alma"],
        slow_ma_types: ["alma"],
        strategy: { fee_bps: 1.0 },
        halftrend_filter: { enabled: true },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "alma",
            slow_ma_type: "alma",
            fast_len: 9,
            slow_len: 30,
            fast_ma_params: { sigma: 6, offset: 0.85 },
            slow_ma_params: { offset: 0.85, sigma: 6 },
          },
        },
        {
          params: {
            fast_ma_type: "alma",
            slow_ma_type: "alma",
            fast_len: 9,
            slow_len: 30,
            fast_ma_params: { offset: 0.85, sigma: 6 },
            slow_ma_params: { sigma: 6, offset: 0.85 },
          },
        },
        {
          params: {
            fast_ma_type: "alma",
            slow_ma_type: "alma",
            fast_len: 11,
            slow_len: 40,
            fast_ma_params: { offset: 0.85, sigma: 6 },
            slow_ma_params: { offset: 0.85, sigma: 6 },
          },
        },
      ],
      candidates: "2",
    });

    expect(req.candidates).toHaveLength(2);
    expect(req.candidates[0]).toMatchObject({ fast_len: 9, slow_len: 30 });
    expect(req.candidates[1]).toMatchObject({ fast_len: 11, slow_len: 40 });
  });

  it("dedupes cross datasets by normalized path", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "DoubleMa",
        data_id: "data-cross-dedupe",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["sma"],
        strategy: { fee_bps: 1.0 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "sma",
            fast_len: 10,
            slow_len: 40,
          },
        },
      ],
      candidates: "4",
      crossEnable: true,
      crossMax: "3",
      crossDatasets: [
        { csv_path: "C:/data/AAPL.csv", label: "AAPL first" },
        { csv_path: " c:\\DATA\\aapl.csv ", label: "AAPL duplicate" },
        { csv_path: "D:/data/MSFT.csv", label: "MSFT" },
      ],
    });

    expect(req.cross_ticker).toEqual({
      max_datasets: 3,
      mode: "preserved_finalists",
      datasets: [
        { csv_path: "C:/data/AAPL.csv", label: "AAPL first" },
        { csv_path: "D:/data/MSFT.csv", label: "MSFT" },
      ],
    });
  });

  it("treats Marrt as a double-like validation strategy", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Marrt",
        data_id: "data-ribbon",
        ma_source: "close",
        fast_ma_types: ["alma"],
        slow_ma_types: ["vwma"],
        strategy: { fee_bps: 1.0 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "alma",
            slow_ma_type: "vwma",
            fast_len: 8,
            slow_len: 34,
            fast_ma_params: { offset: 0.85 },
          },
        },
      ],
      candidates: "8",
    });

    expect(req.validation_context.strategy_kind).toBe("Marrt");
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: "alma",
      slow_ma_type: "vwma",
      fast_len: 8,
      slow_len: 34,
    });
  });

  it.each([
    [
      "CompositeTrend",
      { backend: "GpuKernel", device_id: 0 },
      "expected CpuOnly fallback for fixed-candidate validation (CompositeTrend)",
    ],
    [
      "RegimeHybrid",
      { backend: "GpuHybridKernel", device_id: 0 },
      "expected GPU hybrid main route (RegimeHybrid)",
    ],
    [
      "BreakoutMomentum",
      { backend: "GpuKernel", device_id: 0 },
      "expected GPU kernel main route (BreakoutMomentum)",
    ],
  ] as const)(
    "treats %s as a double-like validation strategy with preserved-finalists cross reruns",
    (strategy_kind, backend, expectedRouteFragment) => {
      const req = buildValidationRequest({
        currentRunReq: {
          strategy_kind,
          data_id: `${strategy_kind}-data`,
          backend,
          ma_source: "close",
          fast_ma_types: ["ema"],
          slow_ma_types: ["sma"],
          strategy: { fee_bps: 1.25 },
          halftrend_filter: { enabled: true },
        },
        currentRunRes: { ok: true },
        topRows: [
          {
            params: {
              fast_ma_type: "ema",
              slow_ma_type: "sma",
              fast_len: 11,
              slow_len: 44,
            },
          },
        ],
        crossEnable: true,
        crossMax: "1",
        crossDatasets: [{ csv_path: "C:/data/SPY.csv", label: "SPY" }],
        candidates: "4",
      });

      expect(req.validation_context.strategy_kind).toBe(strategy_kind);
      expect(req.candidates[0]).toMatchObject({
        fast_ma_type: "ema",
        slow_ma_type: "sma",
        fast_len: 11,
        slow_len: 44,
        strategy: { fee_bps: 1.25 },
        halftrend_filter: { enabled: true },
      });
      expect(req.cross_ticker).toEqual({
        max_datasets: 1,
        mode: "preserved_finalists",
        datasets: [{ csv_path: "C:/data/SPY.csv", label: "SPY" }],
      });

      const summary = buildValidationCapabilitySummary(req);
      expect(summary.routeSummary).toContain(expectedRouteFragment);
      expect(summary.crossSummary).toContain("finalists only (cap=4)");
      expect(summary.warningSummary).toContain(
        "Preserved-finalists cross reruns keep CSCV/PBO + hard gates",
      );
    },
  );

  it.each([
    [
      "CompositeTrend",
      { backend: "CpuOnly" },
      "expected CPU route (CompositeTrend)",
    ],
    [
      "RegimeHybrid",
      { backend: "CpuOnly" },
      "expected CPU route (RegimeHybrid)",
    ],
    [
      "BreakoutMomentum",
      { backend: "GpuOnly", device_id: 0 },
      "expected CpuOnly fallback for fixed-candidate validation (BreakoutMomentum)",
    ],
  ] as const)(
    "keeps strategy-specific route wording for %s when validation cannot stay on the GPU main route",
    (strategy_kind, backend, expectedRouteFragment) => {
      const req = buildValidationRequest({
        currentRunReq: {
          strategy_kind,
          data_id: `${strategy_kind}-route-check`,
          backend,
          ma_source: "close",
          fast_ma_types: ["ema"],
          slow_ma_types: ["sma"],
          strategy: { fee_bps: 1.25 },
        },
        currentRunRes: { ok: true },
        topRows: [
          {
            params: {
              fast_ma_type: "ema",
              slow_ma_type: "sma",
              fast_len: 11,
              slow_len: 44,
            },
          },
        ],
        candidates: "4",
      });

      const summary = buildValidationCapabilitySummary(req);
      expect(summary.routeSummary).toContain(expectedRouteFragment);
    },
  );

  it.each([
    ["CompositeTrend", { backend: "CpuOnly" }],
    ["RegimeHybrid", { backend: "CpuOnly" }],
    ["BreakoutMomentum", { backend: "CpuOnly" }],
  ] as const)(
    "keeps %s on the CPU validation route when CpuOnly is requested",
    (strategy_kind, backend) => {
      const req = buildValidationRequest({
        currentRunReq: {
          strategy_kind,
          data_id: `${strategy_kind}-cpu-data`,
          backend,
          ma_source: "close",
          fast_ma_types: ["ema"],
          slow_ma_types: ["sma"],
          strategy: { fee_bps: 1.0 },
        },
        currentRunRes: { ok: true },
        topRows: [
          {
            params: {
              fast_ma_type: "ema",
              slow_ma_type: "sma",
              fast_len: 10,
              slow_len: 40,
            },
          },
        ],
        candidates: "4",
      });

      const summary = buildValidationCapabilitySummary(req);
      expect(summary.routeSummary).toContain(
        "CpuOnly requested | expected CPU route",
      );
      expect(summary.routeSummary).toContain(strategy_kind);
      expect(summary.moduleSummary).toContain(`CPU route (${strategy_kind}):`);
      expect(summary.warningSummary).toBe("none");
    },
  );

  it("treats BreakoutMomentum ADX-on winners as hybrid-route validation reruns", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "BreakoutMomentum",
        data_id: "breakout-hybrid-data",
        backend: { backend: "GpuHybridKernel", device_id: 0 },
        ma_source: "close",
        fast_ma_types: ["ema"],
        slow_ma_types: ["sma"],
        strategy: { fee_bps: 1.0, breakout_use_adx: true },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 13,
            slow_len: 55,
          },
        },
      ],
      candidates: "4",
    });

    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain("GpuHybridKernel requested");
    expect(summary.routeSummary).toContain("expected GPU hybrid main route");
    expect(summary.moduleSummary).toContain("Expected GPU hybrid route");
  });

  it("surfaces Auto CompositeTrend ADX-off validation previews as CPU-route expectations", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "CompositeTrend",
        data_id: "composite-trend-auto-kernel",
        backend: { backend: "Auto", device_id: 0 },
        ma_source: "close",
        fast_ma_types: ["ema"],
        slow_ma_types: ["sma"],
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 12,
            slow_len: 48,
          },
        },
      ],
      candidates: "4",
    });

    req.candidates[0]!.strategy = { trend_quality_use_adx: false };

    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain(
      "Auto requested | expected CPU route (CompositeTrend)",
    );
    expect(summary.moduleSummary).toContain("CPU route (CompositeTrend)");
    expect(summary.categorySummary).toContain("CPU-only:");
  });

  it("surfaces Auto CompositeTrend ADX-on validation previews as hybrid-route expectations", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "CompositeTrend",
        data_id: "composite-trend-auto-hybrid",
        backend: { backend: "Auto", device_id: 0 },
        ma_source: "close",
        fast_ma_types: ["ema"],
        slow_ma_types: ["sma"],
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 12,
            slow_len: 48,
          },
        },
      ],
      candidates: "4",
    });

    req.candidates[0]!.strategy = { trend_quality_use_adx: true };

    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain(
      "Auto requested | expected GPU hybrid main route (CompositeTrend) when available",
    );
    expect(summary.moduleSummary).toContain(
      "Expected GPU hybrid route (CompositeTrend) when available",
    );
  });

  it("reroutes explicit GpuKernel BreakoutMomentum ADX-on validation previews to the hybrid route", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "BreakoutMomentum",
        data_id: "breakout-explicit-kernel-reroute",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
        fast_ma_types: ["ema"],
        slow_ma_types: ["sma"],
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 13,
            slow_len: 55,
          },
        },
      ],
      candidates: "4",
    });

    req.candidates[0]!.strategy = { breakout_use_adx: true };

    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain(
      "GpuKernel requested | expected GPU hybrid main route (BreakoutMomentum)",
    );
    expect(summary.moduleSummary).toContain(
      "Expected GPU hybrid route (BreakoutMomentum)",
    );
  });

  it("preserves DynamicDoubleMa strategy IR for validation replay", () => {
    const dynamicStrategyIr = {
      profile: "double_ma",
      entry: { kind: "and", nodes: [{ kind: "fast_above_slow" }] },
      exit: { kind: "fast_below_slow" },
    };

    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "DynamicDoubleMa",
        data_id: "data-dynamic",
        ma_source: "close",
        dynamic_strategy_ir: dynamicStrategyIr,
        strategy: { long_only: true },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "ema",
            fast_len: 12,
            slow_len: 48,
          },
        },
      ],
      candidates: "8",
    });

    expect(req.validation_context.strategy_kind).toBe("DynamicDoubleMa");
    expect(req.dynamic_strategy_ir).toEqual(dynamicStrategyIr);
    expect(req.candidates[0]?.strategy).toEqual({ long_only: true });
  });

  it("rejects DynamicDoubleMa validation when the optimization artifact has no dynamic IR", () => {
    expect(() =>
      buildValidationRequest({
        currentRunReq: {
          strategy_kind: "DynamicDoubleMa",
          data_id: "data-dynamic",
          ma_source: "close",
          strategy: { long_only: true },
        },
        currentRunRes: { ok: true },
        topRows: [
          {
            params: {
              fast_ma_type: "ema",
              slow_ma_type: "ema",
              fast_len: 12,
              slow_len: 48,
            },
          },
        ],
        candidates: "8",
      }),
    ).toThrow("DynamicDoubleMa validation requires dynamic_strategy_ir");
  });

  it("treats Set as a double-like validation strategy", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Set",
        data_id: "data-set",
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["sma"],
        strategy: { slope_efficiency_trend_enable: true, set_er_period: 14 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "sma",
            fast_len: 8,
            slow_len: 34,
          },
        },
      ],
      candidates: "8",
    });

    expect(req.validation_context.strategy_kind).toBe("Set");
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: "sma",
      slow_ma_type: "sma",
      fast_len: 8,
      slow_len: 34,
      strategy: { slope_efficiency_trend_enable: true, set_er_period: 14 },
    });
  });

  it("treats Pit as a double-like validation strategy", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Pit",
        data_id: "data-pit",
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["sma"],
        strategy: { pressure_impulse_trend_enable: true, pit_efi_period: 13 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "sma",
            fast_len: 8,
            slow_len: 34,
          },
        },
      ],
      candidates: "8",
    });

    expect(req.validation_context.strategy_kind).toBe("Pit");
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: "sma",
      slow_ma_type: "sma",
      fast_len: 8,
      slow_len: 34,
      strategy: { pressure_impulse_trend_enable: true, pit_efi_period: 13 },
    });
  });

  it("treats Mts as a double-like validation strategy", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Mts",
        data_id: "data-mts",
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["sma"],
        strategy: { momentum_triple_stack_enable: true, mts_roc_period: 10 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "sma",
            fast_len: 8,
            slow_len: 34,
          },
        },
      ],
      candidates: "8",
    });

    expect(req.validation_context.strategy_kind).toBe("Mts");
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: "sma",
      slow_ma_type: "sma",
      fast_len: 8,
      slow_len: 34,
      strategy: { momentum_triple_stack_enable: true, mts_roc_period: 10 },
    });
  });

  it("treats Orc as a double-like validation strategy", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Orc",
        data_id: "data-orc",
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["sma"],
        strategy: {
          oscillator_reversion_confluence_enable: true,
          orc_rsi_period: 14,
        },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "sma",
            fast_len: 8,
            slow_len: 34,
          },
        },
      ],
      candidates: "8",
    });

    expect(req.validation_context.strategy_kind).toBe("Orc");
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: "sma",
      slow_ma_type: "sma",
      fast_len: 8,
      slow_len: 34,
      strategy: {
        oscillator_reversion_confluence_enable: true,
        orc_rsi_period: 14,
      },
    });
  });

  it("treats Vfct as a double-like validation strategy", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Vfct",
        data_id: "data-vfct",
        ma_source: "close",
        fast_ma_types: ["vwma"],
        slow_ma_types: ["vwma"],
        strategy: {
          volume_flow_confirm_enable: true,
          volume_flow_mfi_period: 14,
        },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "vwma",
            slow_ma_type: "vwma",
            fast_len: 8,
            slow_len: 34,
          },
        },
      ],
      candidates: "8",
    });

    expect(req.validation_context.strategy_kind).toBe("Vfct");
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: "vwma",
      slow_ma_type: "vwma",
      fast_len: 8,
      slow_len: 34,
      strategy: {
        volume_flow_confirm_enable: true,
        volume_flow_mfi_period: 14,
      },
    });
  });

  it("builds ROCB validation request using run-level ROCB config", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Rocb",
        data_id: "rocb-data",
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["ema"],
        strategy: { fee_bps: 1.0 },
        rocb: {
          channel_period: 20,
          atr_period: 14,
          atr_stop_mult: 2.0,
          max_trade_bars: 80,
        },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "ema",
            fast_len: 10,
            slow_len: 40,
            rocb: {
              channel_period: 22,
              atr_period: 16,
              atr_stop_mult: 2.2,
              max_trade_bars: 90,
            },
          },
        },
      ],
      candidates: "5",
      seed: "6",
    });

    expect(req.data_id).toBe("rocb-data");
    expect(req.seed).toBe(6);
    expect(req.validation_context.strategy_kind).toBe("Rocb");
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: "sma",
      slow_ma_type: "ema",
      fast_len: 10,
      slow_len: 40,
      rocb: {
        channel_period: 22,
        atr_period: 16,
        atr_stop_mult: 2.2,
        max_trade_bars: 90,
      },
    });
  });

  it("builds ASR validation request using run-level ASR config", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Asr",
        data_id: "asr-data",
        ma_source: "close",
        fast_ma_types: ["alma"],
        slow_ma_types: ["alma"],
        strategy: { fee_bps: 1.0 },
        asr: { angle_deg_min: 12, z_entry: 1.8 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "alma",
            slow_ma_type: "alma",
            fast_len: 20,
            slow_len: 80,
          },
        },
      ],
      candidates: "5",
      seed: "3",
    });

    expect(req.data_id).toBe("asr-data");
    expect(req.seed).toBe(3);
    expect(req.candidates).toHaveLength(1);
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: "alma",
      slow_ma_type: "alma",
      fast_len: 20,
      slow_len: 80,
      asr: { angle_deg_min: 12, z_entry: 1.8 },
    });
  });

  it("builds VEMR validation request using run-level VEMR config", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Vemr",
        data_id: "vemr-data",
        ma_source: "close",
        fast_ma_types: ["ema"],
        slow_ma_types: ["ema"],
        strategy: { fee_bps: 1.0 },
        vemr: { z_entry: 2.1, atr_stop_mult: 1.8 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "ema",
            fast_len: 14,
            slow_len: 55,
          },
        },
      ],
      candidates: "5",
      seed: "9",
    });

    expect(req.data_id).toBe("vemr-data");
    expect(req.seed).toBe(9);
    expect(req.candidates).toHaveLength(1);
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: "ema",
      slow_ma_type: "ema",
      fast_len: 14,
      slow_len: 55,
      vemr: { z_entry: 2.1, atr_stop_mult: 1.8 },
    });
  });

  it("builds MKCS validation request using run-level MKCS config", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Mkcs",
        data_id: "mkcs-data",
        ma_source: "close",
        backend: { backend: "GpuKernel", device_id: 0 },
        fast_ma_types: ["ema"],
        slow_ma_types: ["sma"],
        strategy: { fee_bps: 1.25 },
        mkcs: { compression_threshold: 0.01, reexpansion_threshold: 0.03 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 12,
            slow_len: 48,
            mkcs: { stddev_period: 20, breakout_lookback: 30 },
          },
        },
      ],
      candidates: "5",
      seed: "5",
    });

    expect(req.data_id).toBe("mkcs-data");
    expect(req.seed).toBe(5);
    expect(req.validation_context.strategy_kind).toBe("Mkcs");
    expect(req.backend).toEqual({ backend: "Auto", device_id: 0 });
    expect(req.candidates).toHaveLength(1);
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: "ema",
      slow_ma_type: "sma",
      fast_len: 12,
      slow_len: 48,
      mkcs: { stddev_period: 20, breakout_lookback: 30 },
    });
    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain(
      "expected CPU-preferred composite route (MKCS)",
    );
    expect(summary.warningSummary).toContain(
      "Auto composite validation currently prefers CPU for MKCS",
    );
  });

  it("builds KQSR validation request using run-level KQSR config", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Kqsr",
        data_id: "kqsr-data",
        ma_source: "close",
        fast_ma_types: ["ema"],
        slow_ma_types: ["ema"],
        strategy: { fee_bps: 0.5 },
        kqsr: { oversold: 18, overbought: 82 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "ema",
            fast_len: 12,
            slow_len: 36,
            kqsr: { prb_period: 24, prb_threshold: 0.15 },
          },
        },
      ],
    });

    expect(req.validation_context.strategy_kind).toBe("Kqsr");
    expect(req.candidates[0]).toMatchObject({
      fast_len: 12,
      slow_len: 36,
      kqsr: { prb_period: 24, prb_threshold: 0.15 },
    });
  });

  it("builds CARM validation request using run-level CARM config", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Carm",
        data_id: "carm-data",
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["ema"],
        strategy: { fee_bps: 0.5 },
        carm: { otto_threshold: 0.1 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "ema",
            fast_len: 10,
            slow_len: 42,
            carm: { rocr_period: 12, otto_period: 24 },
          },
        },
      ],
    });

    expect(req.validation_context.strategy_kind).toBe("Carm");
    expect(req.candidates[0]).toMatchObject({
      fast_len: 10,
      slow_len: 42,
      carm: { rocr_period: 12, otto_period: 24 },
    });
  });

  it("builds KVDF validation request using run-level KVDF config", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Kvdf",
        data_id: "kvdf-data",
        ma_source: "close",
        fast_ma_types: ["ema"],
        slow_ma_types: ["sma"],
        strategy: { fee_bps: 0.5 },
        kvdf: { keltner_mult: 1.8, dti_threshold: 22 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 15,
            slow_len: 55,
            kvdf: { fvg_lookback: 18, devstop_period: 16 },
          },
        },
      ],
    });

    expect(req.validation_context.strategy_kind).toBe("Kvdf");
    expect(req.candidates[0]).toMatchObject({
      fast_len: 15,
      slow_len: 55,
      kvdf: { fvg_lookback: 18, devstop_period: 16 },
    });
  });

  it("builds CDMC validation request using run-level CDMC config", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Cdmc",
        data_id: "cdmc-data",
        ma_source: "close",
        fast_ma_types: ["ema"],
        slow_ma_types: ["ema"],
        strategy: { fee_bps: 0.5 },
        cdmc: { mama_fast_limit: 0.45, mama_slow_limit: 0.08 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "ema",
            fast_len: 11,
            slow_len: 44,
            cdmc: { dpo_period: 22, decycler_period: 34 },
          },
        },
      ],
    });

    expect(req.validation_context.strategy_kind).toBe("Cdmc");
    expect(req.candidates[0]).toMatchObject({
      fast_len: 11,
      slow_len: 44,
      cdmc: { dpo_period: 22, decycler_period: 34 },
    });
  });

  it("builds VCC validation request using run-level VCC config", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Vcc",
        data_id: "vcc-data",
        ma_source: "close",
        fast_ma_types: ["ema"],
        slow_ma_types: ["ema"],
        strategy: { fee_bps: 1.0 },
        vcc: { squeeze_period: 20, trend_period: 6 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "ema",
            fast_len: 10,
            slow_len: 40,
          },
        },
      ],
      candidates: "5",
      seed: "4",
    });

    expect(req.data_id).toBe("vcc-data");
    expect(req.seed).toBe(4);
    expect(req.backend).toEqual({ backend: "Auto", device_id: 0 });
    expect(req.candidates).toHaveLength(1);
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: "ema",
      slow_ma_type: "ema",
      fast_len: 10,
      slow_len: 40,
      vcc: { squeeze_period: 20, trend_period: 6 },
    });
    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain(
      "expected GPU composite route (VCC)",
    );
    expect(summary.warningSummary).toContain(
      "VCC GPU validation now runs the current fixed-candidate validation module stack on the GpuKernel route",
    );
  });

  it("summarizes double-ma gpu preserved-finalists capability expectations", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "DoubleMa",
        data_id: "data-gpu",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
        fast_ma_types: ["alma"],
        slow_ma_types: ["alma"],
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "alma",
            slow_ma_type: "alma",
            fast_len: 9,
            slow_len: 30,
          },
        },
      ],
      wfEnable: true,
      wfWindow: "12000",
      wfStep: "3000",
      reoptWfEnable: true,
      reoptWfWindow: "12000",
      reoptWfStep: "3000",
      reoptWfTopN: "4",
      spaEnable: true,
      spaSlices: "12",
      spaBootstrapSamples: "300",
      spaBlockLength: "4",
      perturbEnable: true,
      perturbIters: "8",
      perturbMin: "0.01",
      perturbMax: "0.05",
      mcEnable: true,
      mcSims: "500",
      mcBlockLen: "9",
      cscvEnable: true,
      cscvSlices: "16",
      crossEnable: true,
      crossMax: "2",
      crossMode: "preserved_finalists",
      crossDatasets: [
        { csv_path: "C:/data/SPY.csv", label: "SPY" },
        { csv_path: "C:/data/QQQ.csv", label: "QQQ" },
      ],
    });

    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain("GpuKernel requested");
    expect(summary.crossSummary).toContain("2 datasets");
    expect(summary.crossSummary).toContain("finalists only (cap=4)");
    expect(summary.moduleSummary).toContain("GPU main route");
    expect(summary.moduleSummary).toContain(
      "auxiliary CPU tails: perturb + trade bootstrap compare (iid + stationary, stat block=9)",
    );
    expect(summary.categorySummary).toContain(
      "GPU-native: base reruns + hard gates + walk_forward + reopt_walk_forward + cscv_pbo + spa",
    );
    expect(summary.categorySummary).toContain(
      "GPU main + CPU tail: perturb + trade_bootstrap",
    );
    expect(summary.warningSummary).toContain(
      "Preserved-finalists cross reruns keep CSCV/PBO + hard gates",
    );
  });

  it("warns when gpuonly is requested for fixed-candidate double-ma validation", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "DoubleMa",
        data_id: "data-gpuonly",
        backend: { backend: "GpuOnly", device_id: 0 },
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["sma"],
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "sma",
            fast_len: 8,
            slow_len: 34,
          },
        },
      ],
      candidates: "4",
    });

    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain("expected CpuOnly fallback");
    expect(summary.moduleSummary).toContain("Expected CPU fallback route");
    expect(summary.categorySummary).toContain(
      "CPU-only: base reruns + hard gates",
    );
    expect(summary.warningSummary).toContain(
      "GpuOnly is not supported for fixed-candidate Double-MA validation",
    );
  });

  it("supports iid-only bootstrap mode for validation requests and summaries", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "DoubleMa",
        data_id: "data-iid",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
        fast_ma_types: ["sma"],
        slow_ma_types: ["alma"],
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "sma",
            slow_ma_type: "alma",
            fast_len: 10,
            slow_len: 40,
          },
        },
      ],
      mcEnable: true,
      mcSims: "400",
      mcMode: "iid_only",
      mcBlockLen: "9",
      candidates: "8",
    });

    expect(req.trade_bootstrap).toEqual({
      simulations: 400,
      stationary_mean_block_len: 0,
    });
    expect(req.monte_carlo).toEqual({
      schema_version: 1,
      simulations: 400,
      modes: ["trade_iid"],
      stationary_mean_block_len: 0,
      distribution_bins: 40,
      tail_alpha: 0.05,
    });
    const summary = buildValidationCapabilitySummary(req);
    expect(summary.moduleSummary).toContain("trade bootstrap iid-only");
  });

  it("surfaces CompositeMo cross-ticker fallback and lightweight-rerun expectations explicitly", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "CompositeMo",
        data_id: "data-composite-cross",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 9,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: "soft_vote",
          },
        },
      ],
      wfEnable: true,
      wfWindow: "12000",
      wfStep: "3000",
      cscvEnable: true,
      cscvSlices: "16",
      crossEnable: true,
      crossMax: "2",
      crossMode: "lightweight",
      crossDatasets: [
        { csv_path: "C:/data/SPY.csv", label: "SPY" },
        { csv_path: "C:/data/QQQ.csv", label: "QQQ" },
      ],
      candidates: "8",
    });

    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain(
      "expected CPU-preferred composite route (CompositeMo)",
    );
    expect(summary.moduleSummary).toContain("Expected CPU route (CompositeMo)");
    expect(summary.categorySummary).toContain(
      "CPU-only: base reruns + hard gates + walk_forward + cscv_pbo",
    );
    expect(summary.crossSummary).toContain("lightweight reruns");
    expect(summary.skippedSummary).toContain(
      "Cross reruns: composite main route does not support inline cross_ticker; lightweight composite reruns omit walk_forward + reopt_walk_forward + SPA + perturb + trade bootstrap + CSCV + hard gates",
    );
    expect(summary.routeSummary).toContain(
      "expected CPU-preferred composite route (CompositeMo)",
    );
    expect(summary.warningSummary).toContain(
      "CompositeMo uses the CPU-preferred validation route",
    );
    expect(summary.warningSummary).toContain(
      "Composite fixed-candidate main-route validation does not support inline cross_ticker; composite cross reruns are lightweight and omit walk_forward",
    );
  });

  it("surfaces composite preserved-finalists cross reruns as finalists-only with verdict-relevant retention", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Asr",
        data_id: "data-composite-preserved",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 9,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            asr: {},
          },
        },
      ],
      wfEnable: true,
      wfWindow: "12000",
      wfStep: "3000",
      cscvEnable: true,
      cscvSlices: "16",
      crossEnable: true,
      crossMax: "2",
      crossMode: "preserved_finalists",
      crossDatasets: [{ csv_path: "C:/data/SPY.csv", label: "SPY" }],
      candidates: "8",
    });

    const summary = buildValidationCapabilitySummary(req);
    expect(summary.crossSummary).toContain("finalists only (cap=4)");
    expect(summary.skippedSummary).toContain(
      "Cross reruns: preserved_finalists omit walk_forward + reopt_walk_forward + SPA + perturb + trade bootstrap",
    );
    expect(summary.warningSummary).toContain(
      "preserved-finalists composite cross reruns keep CSCV/PBO + hard gates",
    );
  });

  it("surfaces full-stack cross reruns without auxiliary omissions", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Asr",
        data_id: "data-composite-full-stack",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 9,
            slow_len: 30,
            asr: {},
          },
        },
      ],
      wfEnable: true,
      wfWindow: "12000",
      wfStep: "3000",
      cscvEnable: true,
      cscvSlices: "16",
      crossEnable: true,
      crossMax: "2",
      crossMode: "full_stack",
      crossDatasets: [{ csv_path: "C:/data/SPY.csv", label: "SPY" }],
      candidates: "8",
    });

    const summary = buildValidationCapabilitySummary(req);
    expect(req.cross_ticker?.mode).toBe("full_stack");
    expect(summary.crossSummary).toContain("full stack finalists (cap=4)");
    expect(summary.skippedSummary).not.toContain("Cross reruns:");
    expect(summary.warningSummary).toContain(
      "Composite cross reruns use full-stack validation modules",
    );
  });

  it("surfaces plain CompositeCpu GPU route restrictions explicitly in previews", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "CompositeCpu",
        data_id: "data-composite-cpu",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
        strategy: { fee_bps: 1.0 },
        halftrend: { amplitude: 2, channel_deviation: 2.0, atr_period: 100 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 9,
            slow_len: 30,
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: "hard_gate",
            strategy: { fee_bps: 1.0 },
          },
        },
      ],
      candidates: "8",
    });

    expect(req.backend).toEqual({ backend: "Auto", device_id: 0 });
    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain(
      "expected GPU composite route (base composite)",
    );
    expect(summary.warningSummary).toContain(
      "Plain CompositeCpu GPU validation currently requires uniform HalfTrend parameters across finalists",
    );
  });

  it("keeps the product-default plain CompositeCpu CpuOnly route explicit in previews", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "CompositeCpu",
        data_id: "data-composite-cpu-default",
        backend: { backend: "CpuOnly" },
        ma_source: "close",
        strategy: { fee_bps: 1.0 },
        halftrend: { amplitude: 2, channel_deviation: 2.0, atr_period: 100 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 9,
            slow_len: 30,
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: "hard_gate",
            strategy: { fee_bps: 1.0 },
          },
        },
      ],
      candidates: "8",
    });

    expect(req.backend).toEqual({ backend: "CpuOnly" });
    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain(
      "CpuOnly requested | CPU composite route expected (base composite)",
    );
    expect(summary.moduleSummary).toContain("CPU route (base composite):");
    expect(summary.warningSummary).toBe("none");
  });

  it("defaults Carm composite validation to Auto so CPU-preferred policy applies after GPU optimization", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Carm",
        data_id: "data-carm-preview",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 10,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            carm: {},
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: "hard_gate",
          },
        },
      ],
      candidates: "8",
    });

    expect(req.backend).toEqual({ backend: "Auto", device_id: 0 });
    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain(
      "expected CPU-preferred composite route (CARM)",
    );
    expect(summary.moduleSummary).toContain("Expected CPU route (CARM)");
    expect(summary.categorySummary).toContain(
      "CPU-only: base reruns + hard gates",
    );
    expect(summary.warningSummary).toContain(
      "Auto composite validation currently prefers CPU for CARM",
    );
  });

  it("defaults VCET validation to Auto and carries vcet params into candidates", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Vcet",
        data_id: "data-vcet",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
        vcet: {
          vidya_short_period: 6,
          vidya_long_period: 24,
          ce_period: 22,
          ce_mult: 3.0,
        },
        strategy: { fee_bps: 1.0 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "vidya",
            slow_ma_type: "vidya",
            fast_len: 8,
            slow_len: 34,
            vcet: {
              vidya_short_period: 8,
              vidya_long_period: 34,
              ce_period: 34,
              ce_mult: 2.8,
            },
            strategy: { fee_bps: 1.0 },
          },
        },
      ],
      candidates: "8",
    });

    expect(req.backend).toEqual({ backend: "Auto", device_id: 0 });
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: "vidya",
      slow_ma_type: "vidya",
      fast_len: 8,
      slow_len: 34,
      vcet: {
        vidya_short_period: 8,
        vidya_long_period: 34,
        ce_period: 34,
        ce_mult: 2.8,
      },
    });
    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain(
      "expected GPU composite route (VCET)",
    );
    expect(summary.categorySummary).toContain(
      "GPU-native: base reruns + hard gates",
    );
    expect(summary.categorySummary).not.toContain("GPU main + CPU tail:");
    expect(summary.warningSummary).toContain(
      "VCET GPU validation now supports the current fixed-candidate validation module stack on the GpuKernel route",
    );
  });

  it("treats Carm Auto composite validation as CPU-preferred in previews", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Carm",
        data_id: "data-carm-auto-preview",
        backend: { backend: "Auto", device_id: 0 },
        ma_source: "close",
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 10,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            carm: {},
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: "hard_gate",
          },
        },
      ],
      candidates: "8",
    });

    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain(
      "expected CPU-preferred composite route (CARM)",
    );
    expect(summary.moduleSummary).toContain("Expected CPU route (CARM)");
    expect(summary.categorySummary).toContain(
      "CPU-only: base reruns + hard gates",
    );
    expect(summary.warningSummary).toContain(
      "Auto composite validation currently prefers CPU for CARM",
    );
  });

  it("treats Cdmc Auto composite validation as GPU-routed in previews", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Cdmc",
        data_id: "data-cdmc-auto-preview",
        backend: { backend: "Auto", device_id: 0 },
        ma_source: "close",
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 10,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            cdmc: {},
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: "hard_gate",
          },
        },
      ],
      candidates: "8",
    });

    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain(
      "expected GPU composite route (CDMC)",
    );
    expect(summary.moduleSummary).toContain(
      "Expected GPU composite route (CDMC)",
    );
    expect(summary.categorySummary).toContain(
      "GPU-native: base reruns + hard gates",
    );
    expect(summary.warningSummary).toContain(
      "CDMC GPU validation now runs the current fixed-candidate validation module stack on the GpuKernel route",
    );
  });

  it("defaults Cdmc composite validation to Auto so the measured GPU-preferred policy remains active", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "Cdmc",
        data_id: "data-cdmc-preview",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 10,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            cdmc: {},
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: "hard_gate",
          },
        },
      ],
      candidates: "8",
    });

    expect(req.backend).toEqual({ backend: "Auto", device_id: 0 });
    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain(
      "expected GPU composite route (CDMC)",
    );
    expect(summary.moduleSummary).toContain(
      "Expected GPU composite route (CDMC)",
    );
    expect(summary.categorySummary).toContain(
      "GPU-native: base reruns + hard gates",
    );
    expect(summary.warningSummary).toContain(
      "CDMC GPU validation now runs the current fixed-candidate validation module stack on the GpuKernel route",
    );
  });

  it("treats CompositeMo validation as CPU-preferred in previews", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "CompositeMo",
        data_id: "data-composite-mo",
        backend: { backend: "GpuKernel", device_id: 0 },
        ma_source: "close",
        strategy: { fee_bps: 1.0 },
        halftrend: { amplitude: 2 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 9,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: "soft_vote",
          },
        },
      ],
      candidates: "8",
    });

    const summary = buildValidationCapabilitySummary(req);
    expect(summary.routeSummary).toContain(
      "expected CPU-preferred composite route (CompositeMo)",
    );
    expect(summary.warningSummary).toContain(
      "CompositeMo uses the CPU-preferred validation route",
    );
  });

  it("defaults CompositeMo validation without an explicit backend to Auto", () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: "CompositeMo",
        data_id: "data-composite-auto",
        ma_source: "close",
        strategy: { fee_bps: 1.0 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: "ema",
            slow_ma_type: "sma",
            fast_len: 9,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: "soft_vote",
          },
        },
      ],
      candidates: "8",
    });

    expect(req.backend).toEqual({ backend: "Auto", device_id: 0 });
  });

  it("throws for missing run context or rows", () => {
    expect(() =>
      buildValidationRequest({
        currentRunReq: null,
        currentRunRes: { ok: true },
        topRows: [],
      }),
    ).toThrow("Run an optimization first.");

    expect(() =>
      buildValidationRequest({
        currentRunReq: { strategy_kind: "DoubleMa", data_id: "" },
        currentRunRes: { ok: true },
        topRows: [],
      }),
    ).toThrow("Missing data id (run an optimization first).");

    expect(() =>
      buildValidationRequest({
        currentRunReq: { strategy_kind: "DoubleMa", data_id: "x" },
        currentRunRes: { ok: true },
        topRows: [],
        topRowsFiltered: [],
      }),
    ).toThrow(
      "No Top rows available. Go to the Top tab and run an optimization.",
    );
  });
});
