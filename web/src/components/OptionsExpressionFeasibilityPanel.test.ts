import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import OptionsExpressionFeasibilityPanel from "./OptionsExpressionFeasibilityPanel.vue";

const completeProps = {
  overallStatus: "mixed" as const,
  gateRows: [
    {
      key: "density",
      label: "Candidate density >= 30%",
      value: "25.0%",
      passed: false,
      pending: false,
    },
    {
      key: "gpu-speedup",
      label: "GPU hybrid speedup >= 3.0x",
      value: "pending",
      passed: false,
      pending: true,
    },
  ],
  coverageAdequacy: {
    label: "adequate",
    detail: "Search, holdout, and sealed coverage clear the policy floor.",
    tone: "pass" as const,
  },
  summaryRows: [
    {
      key: "candidate-density",
      label: "Candidate density",
      value: "25 / 100 (25.0%)",
    },
  ],
  overlapRows: [
    {
      key: "overlap-search",
      label: "Search overlap",
      value: "20 / 25 (80.0%)",
    },
  ],
  splitInvalidSections: [
    {
      key: "search",
      label: "Search invalid reasons",
      rows: [{ reason: "no_contract", count: 3 }],
    },
  ],
  structuralCacheBytes: "2.00 MiB",
  laneRows: [
    {
      key: "baseline",
      label: "Baseline",
      summaries: "20 / 25",
      density: "80.0%",
      compileMs: "12 ms",
      bytes: "1.0 KiB",
    },
  ],
  invalidReasonRows: [{ reason: "no_contract", count: 3, share: 0.375 }],
  axisCoverageSections: [
    {
      key: "delay",
      label: "Delay buckets",
      bucketLabel: "Delay",
      rows: [
        {
          key: "delay-5",
          bucket: "5m",
          validSlots: "8 / 10",
          validRate: "80.0%",
          invalidReasons: "no_contract 2",
        },
      ],
    },
  ],
  invalidReasons: "no_contract=3",
  recommendations: ["Trim unsupported strike buckets."],
};

describe("OptionsExpressionFeasibilityPanel", () => {
  it("renders the complete preview with stable accessible table names", () => {
    const wrapper = mount(OptionsExpressionFeasibilityPanel, {
      props: completeProps,
    });

    expect(wrapper.text()).toContain("GPU viability mixed");
    expect(wrapper.get(".gateBadge.isPending").text()).toContain(
      "GPU hybrid speedup >= 3.0x",
    );
    expect(wrapper.text()).toContain(
      "Search, holdout, and sealed coverage clear the policy floor.",
    );
    expect(wrapper.text()).toContain("Structural cache");
    expect(wrapper.text()).toContain("2.00 MiB");
    expect(wrapper.text()).toContain("37.5%");
    expect(wrapper.text()).toContain("Trim unsupported strike buckets.");

    const labels = wrapper
      .findAll("table")
      .map((table) => table.attributes("aria-label"));
    expect(labels).toEqual([
      "Search invalid reasons split invalid reasons",
      "Options feasibility lane diagnostics",
      "Options feasibility invalid reasons",
      "Delay buckets bucket coverage",
    ]);
    expect(
      wrapper
        .findAll(".tableWrap")
        .every((table) => table.attributes("tabindex") === "0"),
    ).toBe(true);
  });

  it("omits optional evidence sections without disturbing lane diagnostics", () => {
    const wrapper = mount(OptionsExpressionFeasibilityPanel, {
      props: {
        ...completeProps,
        overallStatus: "pending",
        coverageAdequacy: null,
        overlapRows: [],
        splitInvalidSections: [],
        invalidReasonRows: [],
        axisCoverageSections: [],
        invalidReasons: "",
        recommendations: [],
      },
    });

    expect(wrapper.text()).toContain("GPU viability pending");
    expect(wrapper.text()).not.toContain("Overlap adequacy");
    expect(wrapper.text()).not.toContain("Recommendations");
    expect(
      wrapper
        .find('table[aria-label="Options feasibility lane diagnostics"]')
        .exists(),
    ).toBe(true);
  });
});
