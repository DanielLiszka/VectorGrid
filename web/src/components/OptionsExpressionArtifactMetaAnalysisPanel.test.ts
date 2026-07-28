import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import OptionsExpressionArtifactMetaAnalysisPanel from "./OptionsExpressionArtifactMetaAnalysisPanel.vue";

const populatedProps = {
  frontierLeaderRows: [
    {
      key: "selection",
      label: "Selection",
      artifactLabel: "2026-07-25",
      metricLabel: "Sel EV",
      value: 0.12345,
      validation: "Holdout supported",
      drift: -0.01234,
      stressDrop: 0.02345,
    },
  ],
  showMetaAnalysis: true,
  metaOverviewRows: [{ key: "artifacts", label: "Artifacts", value: "4" }],
  metaSections: [
    {
      key: "lane",
      label: "By realism lane",
      rows: [
        {
          key: "quote-aware",
          label: "Quote-aware",
          artifactCount: 2,
          validatedCount: 1,
          fragileCount: 0,
          avgRobust: 0.4567,
          avgSelectionEv: 0.1234,
          avgValidationEv: 0.1111,
          avgStressDrop: 0.0123,
        },
      ],
    },
  ],
  laneDisagreementRows: [
    {
      key: "SPY",
      label: "SPY",
      laneCount: 2,
      selectionWinnerLane: "aggregate",
      validationWinnerLane: "quote-aware",
      driftLabel: "different",
      validationSpread: 0.01234,
      laneSummary: "aggregate 0.10 | quote-aware 0.11",
    },
  ],
};

describe("OptionsExpressionArtifactMetaAnalysisPanel", () => {
  it("preserves artifact table labels, formatting, and scroll affordances", () => {
    const wrapper = mount(OptionsExpressionArtifactMetaAnalysisPanel, {
      props: populatedProps,
    });

    expect(wrapper.text()).toContain("Sel EV · 0.123");
    expect(wrapper.text()).toContain("Quote-aware");
    expect(wrapper.text()).toContain("0.457");
    expect(wrapper.text()).toContain("different");

    expect(
      wrapper.findAll("table").map((table) => table.attributes("aria-label")),
    ).toEqual([
      "Options artifact frontier leaders",
      "By realism lane artifact meta-analysis",
      "Realism lane disagreement",
    ]);
    expect(
      wrapper
        .findAll(".tableWrap")
        .every((table) => table.attributes("tabindex") === "0"),
    ).toBe(true);
  });

  it("renders nothing when neither artifact result is available", () => {
    const wrapper = mount(OptionsExpressionArtifactMetaAnalysisPanel, {
      props: {
        frontierLeaderRows: [],
        showMetaAnalysis: false,
        metaOverviewRows: [],
        metaSections: [],
        laneDisagreementRows: [],
      },
    });

    expect(wrapper.find("details").exists()).toBe(false);
  });
});
