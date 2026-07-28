import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import type {
  AnalyzeOptionsExpressionValidationFeasibilitySummary,
  OptionsExpressionValidationAcceptanceSummary,
  ValidateOptionsExpressionArtifactSummary,
} from "../services/optionsExpressionService";
import type { OptionsFinalistDashboardModel } from "../utils/optionsFinalistDashboardModel";
import OptionsExpressionValidationResultPanel from "./OptionsExpressionValidationResultPanel.vue";

const validationPlan = {
  mode: "date_range_holdout",
  startTimestampMs: Date.UTC(2026, 0, 1),
  endTimestampMs: Date.UTC(2026, 0, 31),
  trainSessionCount: null,
  validationSessionCount: null,
  stepSessionCount: null,
};

const feasibility = {
  selectedTupleCount: 1,
  datasetPolicy: "require_exact_replay",
  optionsDatasetId: "SPY-quote-aware",
  priceSemantics: "quote_aware",
  validationPlan,
  notes: ["Preview is ready."],
} as unknown as AnalyzeOptionsExpressionValidationFeasibilitySummary;

const acceptance: OptionsExpressionValidationAcceptanceSummary = {
  verdict: "accepted",
  selectedTupleCount: 1,
  validatedTupleCount: 1,
  acceptedTupleCount: 1,
  supportDeficientTupleCount: 0,
  stressFragileTupleCount: 0,
  minValidTrades: 12,
  bestValidatedExpectancyPerPremium: 0.12345,
  worstStressExpectancyPerPremium: 0.08,
  notes: ["Accepted for follow-up."],
};

const validation = {
  result: {
    validationPlan,
    datasetProvenance: {
      quoteQuality: {
        anyQuoteRowCount: 80,
        rowCount: 100,
        bidAskCoverageRate: 0.75,
        medianSpreadBps: 18.45,
        staleQuoteRate: 0.1,
        warnings: ["Sparse quotes."],
        invalidBidAskRowCount: 2,
      },
    },
    laneSummaries: [
      {
        lane: "holdout",
        supportedTupleCount: 1,
        tupleCount: 1,
        bestExpectancyPerPremium: 0.12345,
        worstExpectancyPerPremium: 0.1,
        medianExpectancyPerPremium: 0.11111,
      },
    ],
  },
} as unknown as ValidateOptionsExpressionArtifactSummary;

const finalistDashboard: OptionsFinalistDashboardModel = {
  verdict: "accepted",
  acceptedTupleCount: 1,
  validatedTupleCount: 1,
  supportDeficientTupleCount: 0,
  stressFragileTupleCount: 0,
  gates: [],
  rows: [],
};

describe("OptionsExpressionValidationResultPanel", () => {
  it("renders validation evidence with stable labels and finalist QA hooks", () => {
    const wrapper = mount(OptionsExpressionValidationResultPanel, {
      props: {
        feasibility,
        acceptance,
        validation,
        finalistDashboard,
      },
    });

    expect(wrapper.text()).toContain(
      "1 selected tuple · require_exact_replay · SPY-quote-aware · quote_aware · Date-range holdout",
    );
    expect(wrapper.text()).toContain(
      "2026-01-01T00:00:00.000Z to 2026-01-31T00:00:00.000Z",
    );
    expect(wrapper.get(".gateBadge.isPass").text()).toContain(
      "Subsequent validation · accepted",
    );
    expect(wrapper.text()).toContain("Bid/ask coverage");
    expect(wrapper.text()).toContain("75.0%");
    expect(wrapper.text()).toContain("Sparse quotes.");

    const laneTable = wrapper.get(
      'table[aria-label="Options validation lane summaries"]',
    );
    expect(laneTable.element.parentElement?.getAttribute("tabindex")).toBe("0");
    expect(laneTable.text()).toContain("0.123");
    expect(
      wrapper
        .get('[data-qa-panel="options.finalistDashboard"]')
        .attributes("data-qa-panel"),
    ).toBe("options.finalistDashboard");
  });

  it("stays absent when no validation result evidence exists", () => {
    const wrapper = mount(OptionsExpressionValidationResultPanel, {
      props: {
        feasibility: null,
        acceptance: null,
        validation: null,
        finalistDashboard: null,
      },
    });

    expect(wrapper.find("table").exists()).toBe(false);
    expect(wrapper.text()).toBe("");
  });
});
