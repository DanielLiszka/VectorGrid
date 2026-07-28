import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import type { OptionsExpressionTupleSummary } from "../services/optionsExpressionService";
import OptionsExpressionSelectedTupleSummaryPanel from "./OptionsExpressionSelectedTupleSummaryPanel.vue";

const tuple = {
  delayMinutes: 5,
  dteDays: 30,
  holdMinutes: 15,
  moneynessBucket: "atm",
  structureFamily: "long_single",
  targetReturn: 0.5,
  stopReturn: 0.25,
  exitTemplate: "fixed_hold_or_target_stop",
  fillModel: "quote_aware",
  robustScore: 0.45678,
  preSealedRank: 2,
  sealedFinalRank: 1,
  sealedFinalist: true,
  selectionMetrics: {
    expectancyPerTrade: 12.34567,
    expectancyPerPremium: 0.12345,
    medianReturnOnPremium: 0.1,
    winRate: 0.6,
    validTrades: 20,
    invalidEvents: 2,
    totalPnlPerContract: 246.9134,
  },
  holdoutMetrics: {
    expectancyPerPremium: 0.11111,
  },
  sealedEvaluationMetrics: null,
  stressMetrics: {
    expectancyPerPremium: 0.08,
  },
} as unknown as OptionsExpressionTupleSummary;

const completeProps = {
  tuple,
  validation: {
    label: "Holdout supported",
    detail: "Holdout support cleared the configured threshold.",
  },
  supportRows: [
    { key: "holdout-support", label: "Holdout support", value: "18 / 20" },
  ],
  validationFrontierRows: [
    {
      key: "holdout",
      label: "Holdout",
      status: "supported",
      support: "18 / 20",
      expectancy: 0.11111,
      delta: -0.01234,
    },
  ],
  validationScopeRows: [
    {
      key: "scope",
      label: "Retained frontier scope",
      value: "Within retained top_k only",
    },
  ],
  validationDriftRows: [
    {
      key: "drift",
      label: "Selection -> Holdout",
      value: "-0.0123",
      detail: "Holdout drift",
      scopeLabel: "Selected tuple only",
    },
  ],
  validationMonotonicityRows: [
    {
      key: "monotonicity",
      label: "Stress ladder",
      statusLabel: "Monotone",
      detail: "Stress remains ordered.",
      scopeLabel: "Selected tuple only",
    },
  ],
  validationRegretRows: [
    {
      key: "regret",
      label: "Retained regret",
      value: "0.0042",
      detail: "Retained-scope regret",
      scopeLabel: "Within retained top_k only",
    },
  ],
};

describe("OptionsExpressionSelectedTupleSummaryPanel", () => {
  it("renders tuple metrics and preserves validation table contracts", () => {
    const wrapper = mount(OptionsExpressionSelectedTupleSummaryPanel, {
      props: completeProps,
    });

    expect(wrapper.text()).toContain("5 / 30 / 15");
    expect(wrapper.text()).toContain("0.457");
    expect(wrapper.text()).toContain("12.3457");
    expect(wrapper.text()).toContain("Holdout EV / premium");
    expect(wrapper.text()).toContain("0.1111");
    expect(wrapper.text()).toContain(
      "Holdout support cleared the configured threshold.",
    );

    const labels = wrapper
      .findAll("table")
      .map((table) => table.attributes("aria-label"));
    expect(labels).toEqual([
      "Selected tuple validation frontier",
      "Selected tuple validation drift",
      "Selected tuple validation monotonicity",
      "Selected tuple validation regret",
    ]);
    expect(
      wrapper
        .findAll(".tableWrap")
        .every((table) => table.attributes("tabindex") === "0"),
    ).toBe(true);
  });

  it("keeps search-only fallback copy when validation lanes are unavailable", () => {
    const wrapper = mount(OptionsExpressionSelectedTupleSummaryPanel, {
      props: {
        ...completeProps,
        validation: null,
        supportRows: [],
        validationFrontierRows: [],
        validationScopeRows: [],
        validationDriftRows: [],
        validationMonotonicityRows: [],
        validationRegretRows: [],
      },
    });

    expect(wrapper.text()).toContain("Search only");
    expect(wrapper.text()).toContain(
      "No supported holdout or sealed trades are present yet.",
    );
    expect(wrapper.find("details").exists()).toBe(false);
  });
});
