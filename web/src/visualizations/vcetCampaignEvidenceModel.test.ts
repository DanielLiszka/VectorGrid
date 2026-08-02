import { describe, expect, it } from "vitest";
import {
  buildVcetCampaignEvidenceMetricView,
  buildVcetCampaignEvidenceModel,
  buildVcetCampaignEvidenceScatterView,
  resolveVcetCampaignEvidencePlateau,
  sampleVcetCampaignEvidenceScatterPoints,
} from "./vcetCampaignEvidenceModel";

function candidate(
  candidateId: string,
  fastLen: number,
  slowLen: number,
  profitable: number | null,
  gate: number | null,
  pnlP10: number | null,
  sharpeP10: number | null,
  missing = 0,
) {
  return {
    candidate_id: candidateId,
    fast_len: fastLen,
    slow_len: slowLen,
    requested_datasets: 10,
    evaluated_datasets: 10 - missing,
    missing_or_failed_datasets: missing,
    profitable_share_requested: profitable,
    hard_gate_pass_share_requested: gate,
    pnl_p10: pnlP10,
    sharpe_p10: sharpeP10,
    max_drawdown_p90: 0.12,
  };
}

function schemaFourSummary() {
  const candidates = [
    candidate("vcet-8-20", 8, 20, 0.7, 0.6, 0.02, 0.3),
    candidate("vcet-10-20", 10, 20, 0.68, 0.58, 0.018, 0.28),
    candidate("vcet-8-30", 8, 30, 0.66, 0.55, 0.015, 0.25),
    candidate("vcet-10-30", 10, 30, 0.4, 0.3, -0.01, -0.2, 2),
  ];
  return {
    strategy_kind: "Vcet",
    holdout_pct: 20,
    items: [],
    fixed_candidate_evaluation: {
      schema_version: 4,
      candidate_manifest_hash: "sha256-grid",
      selected_candidate_id: "vcet-8-20",
      admitted_candidate_ids: [
        "vcet-8-20",
        "vcet-10-20",
        "vcet-8-30",
      ],
      admitted_candidate_count: 3,
      cross_universe_admission: {
        min_profitable_share_requested: 0.55,
        min_hard_gate_pass_share_requested: 0.5,
        min_pnl_p10: 0,
        min_sharpe_p10: 0,
      },
      requested_datasets: 12,
      evaluated_datasets: 12,
      missing_or_failed_datasets: 0,
      candidates: [],
      training: {
        requested_datasets: 10,
        evaluated_datasets: 10,
        missing_or_failed_datasets: 0,
        candidates,
      },
      holdout: {
        requested_datasets: 2,
        evaluated_datasets: 2,
        missing_or_failed_datasets: 0,
        candidates: [],
      },
    },
  };
}

describe("vcetCampaignEvidenceModel", () => {
  it("builds a selection-safe schema-v4 Fast×Slow grid with backend markers", () => {
    const model = buildVcetCampaignEvidenceModel(schemaFourSummary());

    expect(model).toMatchObject({
      state: "ready",
      cohort: "training",
      schemaVersion: 4,
      requestedDatasets: 10,
      evaluatedDatasets: 10,
      selectedCandidateId: "vcet-8-20",
      admissionSource: "artifact",
      admittedCandidateCount: 3,
      fastValues: [8, 10],
      slowValues: [20, 30],
      visualGridAvailable: true,
    });
    expect(model?.cells).toHaveLength(4);
    expect(model?.warnings).toEqual([]);
    expect(
      model?.candidates.find(
        (row) => row.candidateId === "vcet-8-20",
      ),
    ).toMatchObject({ selected: true, admitted: true });
    expect(
      model?.candidates.find(
        (row) => row.candidateId === "vcet-10-30",
      ),
    ).toMatchObject({ selected: false, admitted: false });
  });

  it("uses complete requested denominators for the missing-coverage view", () => {
    const model = buildVcetCampaignEvidenceModel(schemaFourSummary());
    expect(model).not.toBeNull();

    const view = buildVcetCampaignEvidenceMetricView(
      model!,
      "missing_coverage",
    );
    const missing = view.cells.find(
      (cell) => cell.cell.candidate.candidateId === "vcet-10-30",
    );
    const complete = view.cells.find(
      (cell) => cell.cell.candidate.candidateId === "vcet-8-20",
    );

    expect(missing).toMatchObject({
      value: 0.2,
      displayValue: "20.0%",
      tone: "favorable",
    });
    expect(complete).toMatchObject({
      value: 0,
      displayValue: "0.0%",
      tone: "favorable",
      intensity: 4,
    });
    expect(missing?.ariaLabel).toContain("not admitted");
  });

  it("summarizes the selected candidate's descriptive local plateau", () => {
    const model = buildVcetCampaignEvidenceModel(schemaFourSummary());
    expect(model).not.toBeNull();

    const plateau = resolveVcetCampaignEvidencePlateau(
      model!,
      "profitable_share_requested",
    );

    expect(plateau).toMatchObject({
      state: "developing",
      focusCandidateId: "vcet-8-20",
      stableNeighbors: 2,
      evaluatedNeighbors: 3,
    });
    expect(plateau.summary).toContain("descriptive grid context");
    expect(plateau.summary).toContain("not out-of-sample proof");
  });

  it("derives admission for a legacy no-holdout aggregate and labels it degraded", () => {
    const model = buildVcetCampaignEvidenceModel({
      holdout_pct: 0,
      fixed_candidate_evaluation: {
        schema_version: 2,
        candidate_manifest_hash: "legacy",
        selected_candidate_id: "good",
        requested_datasets: 10,
        evaluated_datasets: 10,
        missing_or_failed_datasets: 0,
        cross_universe_admission: {
          min_profitable_share_requested: 0.55,
          min_hard_gate_pass_share_requested: 0.5,
          min_pnl_p10: 0,
          min_sharpe_p10: 0,
        },
        candidates: [
          candidate("good", 8, 20, 0.7, 0.6, 0.02, 0.3),
          candidate("weak", 10, 30, 0.4, 0.3, -0.01, -0.2),
        ],
      },
    });

    expect(model).toMatchObject({
      state: "degraded",
      cohort: "legacy",
      admissionSource: "derived",
      admittedCandidateCount: 1,
    });
    expect(model?.warnings.join(" ")).toContain("Legacy aggregate");
    expect(model?.warnings.join(" ")).toContain(
      "reconstructed from the artifact thresholds",
    );
  });

  it("treats an omitted empty schema-v4 admitted-ID list as authoritative zero", () => {
    const summary = schemaFourSummary();
    summary.fixed_candidate_evaluation.admitted_candidate_ids = [];
    summary.fixed_candidate_evaluation.admitted_candidate_count = 0;
    delete (summary.fixed_candidate_evaluation as Record<string, unknown>)
      .admitted_candidate_ids;

    const model = buildVcetCampaignEvidenceModel(summary);

    expect(model).toMatchObject({
      state: "ready",
      admissionSource: "artifact",
      admittedCandidateCount: 0,
    });
    expect(model?.candidates.every((candidate) => candidate.admitted === false)).toBe(
      true,
    );
    expect(model?.warnings).toEqual([]);
  });

  it("fails closed when a legacy combined aggregate contains holdout data", () => {
    const model = buildVcetCampaignEvidenceModel({
      holdout_pct: 30,
      fixed_candidate_evaluation: {
        schema_version: 1,
        candidates: [candidate("combined", 8, 20, 1, 1, 1, 1)],
      },
    });

    expect(model).toMatchObject({
      state: "empty",
      cohort: "unavailable",
      candidates: [],
      cells: [],
      visualGridAvailable: false,
    });
    expect(model?.warnings.join(" ")).toContain(
      "intentionally not visualized as selection evidence",
    );
  });

  it("omits invalid coordinates and discloses aliased Fast/Slow cells", () => {
    const model = buildVcetCampaignEvidenceModel({
      holdout_pct: 0,
      fixed_candidate_evaluation: {
        schema_version: 4,
        candidate_manifest_hash: "aliases",
        selected_candidate_id: "selected-alias",
        admitted_candidate_ids: ["selected-alias"],
        candidates: [
          candidate("first", 8, 20, 0.6, 0.6, 0.01, 0.1),
          candidate("selected-alias", 8, 20, 0.5, 0.5, 0, 0),
          candidate("bad", 30, 20, 1, 1, 1, 1),
        ],
      },
    });

    expect(model?.cells).toHaveLength(1);
    expect(model?.cells[0]).toMatchObject({
      aliasedCandidateCount: 2,
      candidate: { candidateId: "selected-alias", selected: true },
    });
    expect(model?.warnings.join(" ")).toContain("omitted");
    expect(model?.warnings.join(" ")).toContain(
      "multiple candidate identities",
    );
  });

  it("joins multidimensional search candidates by evaluation identity without Fast×Slow aliasing", () => {
    const aggregateCandidates = [
      candidate("eval-a", 6, 24, 0.7, 0.6, 0.02, 0.3),
      candidate("eval-b", 6, 24, 0.65, 0.55, 0.01, 0.2),
    ];
    const effective = (alpha: number, ceMult: number) => ({
      vidya_short_period: 6,
      vidya_long_period: 24,
      vidya_alpha: alpha,
      ce_period: 20,
      ce_mult: ceMult,
      ce_use_close: true,
      require_vidya_slope: false,
      max_trade_bars: 120,
    });
    const model = buildVcetCampaignEvidenceModel({
      holdout_pct: 0,
      fixed_candidate_evaluation: {
        schema_version: 4,
        candidate_manifest_hash: "sha256-qmc",
        selected_candidate_id: "eval-b",
        admitted_candidate_ids: ["eval-a", "eval-b"],
        admitted_candidate_count: 2,
        search_manifest: {
          schema_version: 1,
          sequence: "halton_cranley_patterson",
          search_id: "vcet-search-1",
          seed: 42,
          requested_budget: 2,
          effective_budget: 2,
          valid_search_space_size: "32",
          float_identity_scale: 1_000_000,
          generation_stats: {
            halton_attempts: 2,
            invalid_period_pairs_discarded: 0,
            duplicate_candidates_discarded: 0,
            lexicographic_fallback_candidates: 0,
            exhaustive_candidates: 0,
          },
          candidates: [
            {
              ordinal: 0,
              candidate_id: "qmc-a",
              evaluation_candidate_id: "eval-a",
              identity_inputs: {},
              effective: effective(0.2, 2.5),
            },
            {
              ordinal: 1,
              candidate_id: "qmc-b",
              evaluation_candidate_id: "eval-b",
              identity_inputs: {},
              effective: effective(0.3, 3),
            },
          ],
        },
        training: {
          requested_datasets: 10,
          evaluated_datasets: 10,
          missing_or_failed_datasets: 0,
          candidates: aggregateCandidates,
        },
        holdout: {
          requested_datasets: 0,
          evaluated_datasets: 0,
          missing_or_failed_datasets: 0,
          candidates: [],
        },
      },
    });

    expect(model).toMatchObject({
      state: "ready",
      designMode: "halton",
      searchId: "vcet-search-1",
      searchSequence: "halton_cranley_patterson",
      searchRequestedBudget: 2,
      searchEffectiveBudget: 2,
      validSearchSpaceSize: "32",
      searchGenerationStats: {
        haltonAttempts: 2,
        exhaustiveCandidates: 0,
        lexicographicFallbackCandidates: 0,
      },
      visualGridAvailable: false,
    });
    expect(model?.cells).toHaveLength(2);
    expect(model?.cells.map((cell) => cell.key)).toEqual(["eval-a", "eval-b"]);
    expect(model?.cells.every((cell) => cell.aliasedCandidateCount === 1)).toBe(
      true,
    );
    expect(model?.candidates[1]).toMatchObject({
      candidateId: "eval-b",
      searchCandidateId: "qmc-b",
      searchOrdinal: 1,
      effective: {
        vidyaShortPeriod: 6,
        vidyaLongPeriod: 24,
        vidyaAlpha: 0.3,
        ceMult: 3,
      },
    });
    expect(model?.warnings).toEqual([]);

    const plateau = resolveVcetCampaignEvidencePlateau(
      model!,
      "profitable_share_requested",
      "eval-b",
    );
    expect(plateau.state).toBe("insufficient");
    expect(plateau.summary).toContain("intentionally unavailable");
    expect(plateau.summary).toContain("six other effective controls");

    const scatter = buildVcetCampaignEvidenceScatterView(
      model!,
      "vidya_alpha",
      "ce_mult",
      "sharpe_p10",
    );
    expect(scatter.points).toHaveLength(2);
    expect(scatter.points[1]).toMatchObject({
      xValue: 0.3,
      yValue: 3,
      xRatio: 1,
      yRatio: 1,
    });
    expect(scatter.points[1]?.ariaLabel).toContain("backend selected");

    const expandedPoints = Array.from({ length: 1_001 }, (_, index) => ({
      ...scatter.points[index % scatter.points.length]!,
      candidate: {
        ...scatter.points[index % scatter.points.length]!.candidate,
        candidateId: `sample-${index}`,
        selected: index === 777,
      },
    }));
    const sample = sampleVcetCampaignEvidenceScatterPoints(expandedPoints, 80);
    expect(sample.points).toHaveLength(80);
    expect(sample.totalPointCount).toBe(1_001);
    expect(sample.omittedPointCount).toBe(921);
    expect(sample.points.some((point) => point.candidate.selected)).toBe(true);
  });

  it("degrades a Halton artifact when aggregate identities cannot join its manifest", () => {
    const model = buildVcetCampaignEvidenceModel({
      fixed_candidate_evaluation: {
        schema_version: 4,
        candidate_manifest_hash: "sha256-broken-qmc",
        admitted_candidate_ids: [],
        admitted_candidate_count: 0,
        search_manifest: {
          sequence: "halton_cranley_patterson",
          search_id: "broken",
          requested_budget: 1,
          effective_budget: 1,
          valid_search_space_size: "1",
          generation_stats: {
            halton_attempts: 1,
            invalid_period_pairs_discarded: 0,
            duplicate_candidates_discarded: 0,
            lexicographic_fallback_candidates: 0,
            exhaustive_candidates: 0,
          },
          candidates: [
            {
              ordinal: 0,
              candidate_id: "sampled",
              evaluation_candidate_id: "different-eval-id",
              effective: {
                vidya_short_period: 6,
                vidya_long_period: 24,
                vidya_alpha: 0.2,
                ce_period: 20,
                ce_mult: 3,
                ce_use_close: true,
                require_vidya_slope: true,
                max_trade_bars: 120,
              },
            },
          ],
        },
        training: {
          requested_datasets: 1,
          evaluated_datasets: 1,
          missing_or_failed_datasets: 0,
          candidates: [candidate("aggregate-id", 6, 24, 1, 1, 1, 1)],
        },
        holdout: { candidates: [] },
      },
    });

    expect(model?.state).toBe("degraded");
    expect(model?.warnings.join(" ")).toContain(
      "no evaluation_candidate_id match",
    );
    expect(model?.warnings.join(" ")).toContain(
      "no selection-safe aggregate row",
    );
  });

  it("returns null when the summary has no VCET fixed-candidate evidence", () => {
    expect(buildVcetCampaignEvidenceModel({ strategy_kind: "Vcet" })).toBeNull();
    expect(buildVcetCampaignEvidenceModel(null)).toBeNull();
  });
});
