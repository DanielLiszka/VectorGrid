import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { buildVcetCampaignEvidenceModel } from "../../visualizations/vcetCampaignEvidenceModel";
import VcetCampaignEvidenceHeatmap from "./VcetCampaignEvidenceHeatmap.vue";

function readyModel() {
  const candidates = [
    {
      candidate_id: "vcet-8-20",
      fast_len: 8,
      slow_len: 20,
      requested_datasets: 10,
      evaluated_datasets: 10,
      missing_or_failed_datasets: 0,
      profitable_share_requested: 0.7,
      hard_gate_pass_share_requested: 0.6,
      pnl_p10: 0.02,
      sharpe_p10: 0.3,
      max_drawdown_p90: 0.1,
    },
    {
      candidate_id: "vcet-10-30",
      fast_len: 10,
      slow_len: 30,
      requested_datasets: 10,
      evaluated_datasets: 8,
      missing_or_failed_datasets: 2,
      profitable_share_requested: 0.4,
      hard_gate_pass_share_requested: 0.3,
      pnl_p10: -0.01,
      sharpe_p10: -0.2,
      max_drawdown_p90: 0.2,
    },
  ];
  const model = buildVcetCampaignEvidenceModel({
    holdout_pct: 20,
    fixed_candidate_evaluation: {
      schema_version: 4,
      candidate_manifest_hash: "sha256-component",
      selected_candidate_id: "vcet-8-20",
      admitted_candidate_ids: ["vcet-8-20"],
      admitted_candidate_count: 1,
      requested_datasets: 12,
      evaluated_datasets: 12,
      missing_or_failed_datasets: 0,
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
  });
  if (!model) throw new Error("expected VCET evidence model");
  return model;
}

function multidimensionalModel(candidateCount = 2) {
  const candidates = Array.from({ length: candidateCount }, (_, index) => ({
    candidate_id: `eval-${index}`,
    fast_len: 6,
    slow_len: 24,
    requested_datasets: 10,
    evaluated_datasets: 10,
    missing_or_failed_datasets: 0,
    profitable_share_requested: 0.5 + (index % 4) * 0.1,
    hard_gate_pass_share_requested: 0.5,
    pnl_p10: index / 10_000,
    sharpe_p10: index / 1_000,
    max_drawdown_p90: 0.1,
  }));
  const model = buildVcetCampaignEvidenceModel({
    holdout_pct: 0,
    fixed_candidate_evaluation: {
      schema_version: 4,
      candidate_manifest_hash: "sha256-component-search",
      selected_candidate_id: "eval-1",
      admitted_candidate_ids: ["eval-0", "eval-1"],
      admitted_candidate_count: 2,
      search_manifest: {
        schema_version: 1,
        sequence: "exhaustive_lexicographic",
        search_id: "search-component",
        seed: 3,
        requested_budget: candidateCount + 10,
        effective_budget: candidateCount,
        valid_search_space_size: String(candidateCount),
        float_identity_scale: 1_000_000,
        generation_stats: {
          halton_attempts: 0,
          invalid_period_pairs_discarded: 0,
          duplicate_candidates_discarded: 0,
          lexicographic_fallback_candidates: 0,
          exhaustive_candidates: candidateCount,
        },
        candidates: candidates.map((candidate, index) => ({
          ordinal: index,
          candidate_id: `search-${index}`,
          evaluation_candidate_id: candidate.candidate_id,
          identity_inputs: {},
          effective: {
            vidya_short_period: 6,
            vidya_long_period: 24,
            vidya_alpha: 0.05 + index / 10_000,
            ce_period: 20 + (index % 5),
            ce_mult: 2.5 + (index % 4) * 0.25,
            ce_use_close: index % 2 === 0,
            require_vidya_slope: index % 3 === 0,
            max_trade_bars: 90 + (index % 4) * 30,
          },
        })),
      },
      training: {
        requested_datasets: 10,
        evaluated_datasets: 10,
        missing_or_failed_datasets: 0,
        candidates,
      },
      holdout: { candidates: [] },
    },
  });
  if (!model) throw new Error("expected multidimensional VCET evidence model");
  return model;
}

describe("VcetCampaignEvidenceHeatmap", () => {
  it("renders the Fast×Slow landscape with selected and admitted markers", () => {
    const wrapper = mount(VcetCampaignEvidenceHeatmap, {
      props: { model: readyModel() },
    });

    expect(wrapper.get("#vcetCampaignEvidenceTitle").text()).toBe(
      "VCET parameter landscape",
    );
    expect(wrapper.get("#vcetCampaignEvidenceSummary").text()).toContain(
      "2 candidates across 10 requested datasets",
    );
    expect(wrapper.get(".vcetMatrix").attributes("aria-label")).toContain(
      "Fast by Slow",
    );
    expect(wrapper.findAll("button.vcetMetricCell")).toHaveLength(2);
    expect(wrapper.findAll(".vcetMetricCell.unavailable")).toHaveLength(2);
    const selected = wrapper.get("button.vcetMetricCell.selected");
    expect(selected.attributes("aria-label")).toContain(
      "Fast 8, Slow 20: Profitability share 70.0%",
    );
    expect(selected.attributes("aria-label")).toContain("backend selected");
    expect(selected.attributes("aria-label")).toContain("admitted");
    expect(selected.find(".markerSelected").exists()).toBe(true);
    expect(selected.find(".markerAdmitted").exists()).toBe(true);
  });

  it("switches metrics and lets a user focus a candidate for plateau context", async () => {
    const wrapper = mount(VcetCampaignEvidenceHeatmap, {
      props: { model: readyModel() },
    });

    await wrapper
      .get("#vcetCampaignEvidenceMetric")
      .setValue("missing_coverage");
    expect(wrapper.get(".vcetMetricExplanation").text()).toContain(
      "lower is better",
    );
    const weak = wrapper.get(
      'button[aria-label^="Fast 10, Slow 30: Missing coverage"]',
    );
    expect(weak.text()).toContain("20.0%");
    await weak.trigger("click");

    expect(wrapper.get(".vcetActiveHeading").text()).toContain(
      "Fast 10 / Slow 30",
    );
    expect(wrapper.get(".vcetActiveHeading").text()).toContain("Not admitted");
    expect(wrapper.get("#vcetCampaignEvidencePlateau").text()).toContain(
      "descriptive grid context",
    );
  });

  it("provides a detailed semantic table with the same candidate evidence", () => {
    const wrapper = mount(VcetCampaignEvidenceHeatmap, {
      props: { model: readyModel() },
    });

    const details = wrapper.get(".vcetAccessibleTable");
    expect(details.get("summary").text()).toContain(
      "Candidate evidence table · 2 retained",
    );
    expect(details.get("caption").text()).toContain(
      "every retained candidate",
    );
    expect(details.findAll("tbody tr")).toHaveLength(2);
    expect(details.text()).toContain("vcet-8-20");
    expect(details.text()).toContain("Selected · Admitted");
    expect(details.text()).toContain("Not admitted");
  });

  it("renders a truthful multidimensional projection and pages the complete identity table", async () => {
    const wrapper = mount(VcetCampaignEvidenceHeatmap, {
      props: { model: multidimensionalModel(105) },
    });

    expect(wrapper.find(".vcetMatrix").exists()).toBe(false);
    expect(wrapper.get("#vcetCampaignEvidenceSummary").text()).toContain(
      "Exhaustive lexicographic design",
    );
    expect(wrapper.findAll("button.vcetScatterPoint")).toHaveLength(105);
    expect(wrapper.get(".vcetScatterNote").text()).toContain(
      "Projected points can overlap",
    );
    expect(wrapper.get(".vcetActiveControls").text()).toContain("Alpha");

    const table = wrapper.get(".vcetAccessibleTable");
    expect(table.findAll("tbody tr")).toHaveLength(100);
    expect(table.text()).toContain("search-0");
    expect(table.text()).toContain("eval-0");
    await table.get('button[aria-label="Next candidate table page"]').trigger("click");
    expect(table.findAll("tbody tr")).toHaveLength(5);
    expect(table.text()).toContain("Showing 101–105 of 105 candidates");
    expect(table.text()).toContain("search-104");
  });

  it("renders fail-closed empty guidance for a degraded combined holdout schema", () => {
    const model = buildVcetCampaignEvidenceModel({
      holdout_pct: 25,
      fixed_candidate_evaluation: {
        schema_version: 1,
        requested_datasets: 4,
        evaluated_datasets: 4,
        candidates: [
          {
            candidate_id: "unsafe-combined",
            fast_len: 8,
            slow_len: 20,
            requested_datasets: 4,
            evaluated_datasets: 4,
            profitable_share_requested: 1,
          },
        ],
      },
    });
    if (!model) throw new Error("expected degraded VCET evidence model");
    const wrapper = mount(VcetCampaignEvidenceHeatmap, { props: { model } });

    expect(wrapper.attributes("data-evidence-state")).toBe("empty");
    expect(wrapper.get(".appEmptyState").text()).toContain(
      "No selection-safe VCET landscape",
    );
    expect(wrapper.get(".vcetEvidenceWarning").text()).toContain(
      "combined candidates are intentionally not visualized",
    );
    expect(wrapper.find(".vcetMatrix").exists()).toBe(false);
  });
});
