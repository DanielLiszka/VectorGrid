import { describe, expect, it } from "vitest";
import { buildValidationEvidenceSummary } from "./validationEvidenceSummary";

describe("validationEvidenceSummary", () => {
  it("returns a not-run summary when no validation result is present", () => {
    const summary = buildValidationEvidenceSummary({
      validationReq: {
        backend: { backend: "Auto" },
        walk_forward: { window: 100 },
      },
      datasetMeta: { dataset_id: "dataset-1" },
    });

    expect(summary).toMatchObject({
      status: "not_run",
      requestedBackend: "Auto",
      reportState: "not_generated",
      datasetIdentity: "dataset-1",
      overfit: {
        status: "not_run",
        summaryLine: "Overfit evidence not run.",
        items: [],
      },
    });
    expect(summary.summaryLine).toContain("Validation evidence not run.");
    expect(summary.summaryLine).toContain("Data realism incomplete;");
    expect(summary.summaryLine).toContain("Promotion blocked;");
    expect(summary.promotionEligibility.eligible).toBe(false);
  });

  it("summarizes available and missing requested modules", () => {
    const summary = buildValidationEvidenceSummary({
      validationReq: {
        backend: { backend: "Gpu" },
        walk_forward: { window: 200 },
        cscv_pbo: { slices: 16 },
        spa: { slices: 16 },
        trade_bootstrap: { simulations: 500 },
      },
      validationRes: {
        requested_backend: "Gpu",
        backend_used: "Gpu",
        backend_route: "GpuKernel",
        walk_forward: { windows: 4 },
        cscv_pbo: { pbo: 0.12 },
        spa: { skipped_reason: "insufficient slices" },
      },
      datasetMeta: { source_content_fingerprint: "sha256:abc" },
      reportPath: "C:/reports/validation.xlsx",
      replayClass: "exact_evidence_review",
    });

    expect(summary.status).toBe("incomplete");
    expect(summary.summaryLine).toContain(
      "Validation evidence incomplete; 2/4 requested modules available; route Gpu.",
    );
    expect(summary.reportState).toBe("generated");
    expect(summary.replayClass).toBe("exact_evidence_review");
    expect(summary.datasetIdentity).toBe("sha256:abc");
    expect(summary.overfit.status).toBe("attention");
    expect(summary.overfit.items.find((item) => item.id === "spa")).toMatchObject(
      {
        value: "skipped",
        detail: "insufficient slices",
        tone: "warn",
      },
    );
    expect(
      summary.modules.map((module) => [
        module.id,
        module.availability,
        module.detail,
      ]),
    ).toEqual([
      ["walk_forward", "available", "result present"],
      ["reopt_walk_forward", "not_requested", "not requested"],
      ["cscv_pbo", "available", "result present"],
      ["spa", "skipped", "insufficient slices"],
      ["monte_carlo", "unavailable", "requested but result missing"],
      ["cross_ticker", "not_requested", "not requested"],
    ]);
  });

  it("keeps optional module absence distinct from missing evidence", () => {
    const summary = buildValidationEvidenceSummary({
      validationReq: {
        backend: { backend: "CpuOnly" },
      },
      validationRes: {
        backend_used: "Cpu",
        backend_route: "CpuOnly",
      },
    });

    expect(summary.status).toBe("available");
    expect(summary.summaryLine).toContain(
      "Validation evidence available; no optional evidence modules requested; route Cpu.",
    );
    expect(
      summary.modules.every(
        (module) => module.availability === "not_requested",
      ),
    ).toBe(true);
  });

  it("prefers validation evidence pack data when present", () => {
    const summary = buildValidationEvidenceSummary({
      validationReq: {
        backend: { backend: "CpuOnly" },
        cscv_pbo: { slices: 16 },
        spa: { slices: 16 },
      },
      validationRes: {
        requested_backend: "CpuOnly",
        backend_used: "Cpu",
        backend_route: "CpuOnly",
        cscv_pbo: { pbo: 0.99 },
        validation_evidence_pack: {
          schema_version: "1-0-0",
          version: 1,
          workflow: "double_ma_validation",
          status: "incomplete",
          status_reason: "requested validation evidence incomplete: spa",
          route: {
            requested_backend: "GpuKernel",
            realized_backend: "Gpu",
            route_label: "GpuKernel",
            residency_tier: "gpu_kernel",
          },
          candidate_universe: {
            source: "validation_request_candidates",
            requested_candidates: 4,
            evaluated_candidates: 4,
            retained_candidates: 2,
            survived_hard_gates: 2,
            approved_candidates: 1,
            optimization_trials: 58300,
          },
          verdict: {
            passed: true,
            approved: true,
            recommendation: "approved_with_warnings",
            summary: "approved",
            warning_flags: ["high_pbo"],
          },
          metrics: [
            {
              metric: "psr",
              availability: "available",
              value: 0.91,
              threshold: 0.75,
              sample_unit: "raw_bars",
            },
            {
              metric: "dsr",
              availability: "available",
              value: 0.72,
              threshold: 0.6,
              sample_unit: "raw_bars",
            },
            {
              metric: "cscv_pbo",
              availability: "available",
              value: 0.14,
              threshold: 0.25,
              comparability_basis: "candidate_set_slices",
            },
            {
              metric: "spa_p_value",
              availability: "skipped",
              unavailable_reason: "spa_skipped_not_enough_slices",
            },
            {
              metric: "min_track_record_length",
              availability: "available",
              value: 512,
              threshold: 256,
            },
          ],
          modules: [
            {
              module: "cscv_pbo",
              availability: "available",
              requested: true,
              summary: "evaluated",
              combinations: 70,
            },
            {
              module: "spa",
              availability: "skipped",
              requested: true,
              summary: "skipped",
              skipped_reason: "spa_skipped_not_enough_slices",
            },
          ],
        },
      },
      optimizationRes: {
        num_combos: 1,
      },
    });

    expect(summary.status).toBe("incomplete");
    expect(summary.summaryLine).toContain(
      "Validation evidence incomplete; pack schema 1-0-0 v1; 1/2 requested modules available; route Gpu.",
    );
    expect(summary.requestedBackend).toBe("GpuKernel");
    expect(summary.realizedBackend).toBe("Gpu");
    expect(summary.route).toBe("GpuKernel");
    expect(summary.modules.find((module) => module.id === "spa")).toMatchObject({
      availability: "skipped",
      detail: "spa_skipped_not_enough_slices",
    });
    expect(summary.overfit.summaryLine).toBe(
      "Overfit evidence needs review; approved; 58,300 trials; PBO 14.0%; PSR 0.910 · DSR 0.720.",
    );
    expect(summary.overfit.items.find((item) => item.id === "pbo")).toMatchObject({
      value: "14.0%",
      detail: "max 25.0% · 70 combinations",
      tone: "good",
    });
    expect(summary.overfit.items.find((item) => item.id === "spa")).toMatchObject({
      value: "skipped",
      detail: "spa_skipped_not_enough_slices",
      tone: "warn",
    });
  });

  it("surfaces complete adjusted-vs-unadjusted realism evidence without calling it deployment approval", () => {
    const summary = buildValidationEvidenceSummary({
      validationRes: {
        validation_evidence_pack: {
          schema_version: "1-0-0",
          version: 1,
          status: "available",
          route: {
            realized_backend: "Cpu",
          },
          dataset: {
            data_realism: {
              price_adjustment: "unadjusted",
              corporate_actions: "documented",
              survivorship_bias: "documented",
              liquidity_capacity: "not_applicable",
              market_impact: "documented",
              borrow_availability: "not_applicable",
              complete: true,
              missing_or_unknown: [],
            },
          },
          promotion_eligibility: {
            eligible: true,
            state: "eligible_for_review",
            gates_passed: ["statistical_validation", "data_realism"],
            gates_failed: [],
            reason: "all research promotion gates passed",
          },
        },
      },
    });

    expect(summary.dataRealism).toMatchObject({
      status: "complete",
      complete: true,
      missingOrUnknown: [],
    });
    expect(
      summary.dataRealism.items.find(
        (item) => item.id === "price_adjustment",
      ),
    ).toMatchObject({
      value: "unadjusted",
      blocking: false,
      detail: "unadjusted prices documented",
    });
    expect(summary.promotionEligibility).toMatchObject({
      eligible: true,
      state: "eligible_for_review",
      gatesPassed: ["statistical_validation", "data_realism"],
      gatesFailed: [],
    });
    expect(summary.summaryLine).toContain(
      "Data realism complete; price adjustment unadjusted;",
    );
    expect(summary.summaryLine).toContain("Promotion eligible for review;");
    expect(summary.summaryLine).toContain(
      "Statistical validation approval is not deployability or deployment approval.",
    );
  });

  it("blocks inconsistent promotion claims when realism is unknown and does not infer capacity or impact from friction stress", () => {
    const summary = buildValidationEvidenceSummary({
      validationRes: {
        friction_stress: {
          pass_count: 4,
          scenario_count: 4,
          worst_scenario_label: "slippage_2x",
        },
        validation_evidence_pack: {
          schema_version: "1-0-0",
          version: 1,
          status: "available",
          dataset: {
            data_realism: {
              price_adjustment: "adjusted",
              corporate_actions: "documented",
              survivorship_bias: "unknown",
              liquidity_capacity: "unknown",
              market_impact: "unknown",
              borrow_availability: "unknown",
              complete: false,
              missing_or_unknown: [
                "survivorship_bias",
                "liquidity_capacity",
                "market_impact",
                "borrow_availability",
              ],
            },
          },
          promotion_eligibility: {
            eligible: true,
            state: "eligible_for_review",
            gates_passed: ["statistical_validation"],
            gates_failed: [],
            reason: "upstream payload was inconsistent",
          },
        },
      },
    });

    expect(summary.dataRealism.status).toBe("incomplete");
    expect(summary.dataRealism.missingOrUnknown).toEqual([
      "survivorship_bias",
      "liquidity_capacity",
      "market_impact",
      "borrow_availability",
    ]);
    expect(
      summary.dataRealism.items.find(
        (item) => item.id === "liquidity_capacity",
      ),
    ).toMatchObject({
      value: "unknown",
      blocking: true,
    });
    expect(
      summary.dataRealism.items.find((item) => item.id === "market_impact"),
    ).toMatchObject({
      value: "unknown",
      blocking: true,
    });
    expect(summary.promotionEligibility).toMatchObject({
      eligible: false,
      state: "blocked",
    });
    expect(summary.summaryLine).toContain("Promotion blocked;");
    expect(summary.summaryLine).toContain(
      "Statistical validation approval is not deployability or deployment approval.",
    );
  });

  it("builds an overfit evidence panel model from validation verdicts and optional evidence blocks", () => {
    const summary = buildValidationEvidenceSummary({
      validationReq: {
        candidates: 4,
        admission_policy: "scalar_union",
        min_psr: 0.8,
        min_dsr: 0.6,
        max_pbo: 0.2,
        walk_forward: { window: 200 },
        cscv_pbo: { slices: 16 },
        spa: { slices: 16 },
        trade_bootstrap: { simulations: 500 },
      },
      optimizationRes: {
        num_combos: 58300,
      },
      validationRes: {
        backend_used: "Gpu",
        candidates: [
          {
            psr: 0.88,
            dsr: 0.61,
            track_record_bars: 24000,
            min_track_record_length_bars: 20000,
          },
          {
            psr: 0.91,
            dsr: 0.65,
            track_record_bars: 25000,
            min_track_record_length_bars: 20000,
            walk_forward: {
              windows: 4,
              sharpe_worst: 0.35,
              pnl_worst: -0.02,
            },
            trade_bootstrap: {
              simulations: 500,
              trade_count: 35,
              prob_pnl_positive: 0.62,
              pnl_p05: -0.04,
            },
            friction_stress: {
              pass_count: 3,
              scenario_count: 4,
              pass_frac: 0.75,
              worst_scenario_label: "fee_2x",
            },
          },
        ],
        cscv_pbo: {
          pbo: 0.12,
          combinations: 100,
          combinations_possible: 128,
        },
        spa: {
          p_value: 0.041,
          passed: true,
          slice_count: 16,
          bootstrap_samples: 500,
        },
        run_verdict: {
          passed: true,
          approved: true,
          candidates_tested: 4,
          candidates_survived_hard_gates: 2,
          approved_candidate_count: 1,
          final_recommendation: "approved_with_warnings",
          warning_flags: ["high_pbo"],
          first_approved_rank: 2,
          top_candidate_psr: 0.91,
          top_candidate_dsr: 0.65,
          top_candidate_track_record_bars: 25000,
          top_candidate_min_track_record_length_bars: 20000,
          pbo: 0.12,
          max_pbo: 0.2,
        },
      },
    });

    expect(summary.overfit.status).toBe("attention");
    expect(summary.overfit.statusLabel).toBe("Needs review");
    expect(summary.overfit.summaryLine).toBe(
      "Overfit evidence needs review; approved; 58,300 trials; PBO 12.0%; PSR 0.910 · DSR 0.650.",
    );
    expect(
      summary.overfit.items.map((item) => [
        item.id,
        item.value,
        item.detail,
        item.tone,
      ]),
    ).toEqual([
      [
        "verdict",
        "approved",
        "hard gates 2/4 · approved 1/2 · high_pbo",
        "warn",
      ],
      [
        "search",
        "58,300 trials",
        "validated 2/4 candidates · admission scalar_union",
        "neutral",
      ],
      [
        "probability",
        "PSR 0.910 · DSR 0.650",
        "gates PSR >= 0.80, DSR >= 0.60",
        "good",
      ],
      ["pbo", "12.0%", "max 20.0% · 100/128 combinations", "good"],
      ["spa", "pass · p=0.041", "16 slices · 500 samples", "good"],
      [
        "track_record",
        "25,000/20,000 bars",
        "meets minimum evidence length",
        "good",
      ],
      [
        "walk_forward",
        "WF 4",
        "worst Sharpe 0.350 · worst PnL -0.020",
        "good",
      ],
      [
        "distribution",
        "500 bootstrap",
        "35 trades · PnL+ 62.0% · p05 -0.040",
        "good",
      ],
      ["friction", "3/4 pass", "worst fee_2x", "warn"],
    ]);
  });
});
