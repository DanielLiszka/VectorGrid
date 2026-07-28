import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import type { OptionsFinalistDashboardModel } from "../utils/optionsFinalistDashboardModel";
import OptionsExpressionFinalistDashboardPanel from "./OptionsExpressionFinalistDashboardPanel.vue";

function buildDashboard(
  overrides: Partial<OptionsFinalistDashboardModel> = {},
): OptionsFinalistDashboardModel {
  return {
    verdict: "holdout supported",
    acceptedTupleCount: 1,
    validatedTupleCount: 1,
    supportDeficientTupleCount: 0,
    stressFragileTupleCount: 0,
    gates: [
      {
        key: "acceptance",
        label: "Acceptance",
        status: "holdout supported",
        detail: "1/1 accepted; 0 support deficient.",
        tone: "pass",
      },
    ],
    rows: [
      {
        key: "tuple-0",
        sourceTupleIndex: 0,
        tupleLabel: "long_single | 5m delay | 30d DTE",
        verdict: "Holdout supported",
        tone: "pass",
        support: "Supported",
        sourceRobustScore: 0.41,
        validatedRobustScore: 0.4567,
        searchEv: 0.1234,
        holdoutEv: 0.1111,
        sealedEv: 0.101,
        stressEv: 0.09,
        harsherStressEv: 0.08,
        extremeStressEv: 0.07,
        strongestStressLabel: "Extreme",
        strongestStressEv: 0.07,
        validTrades: 42,
        drift: "robust +0.047",
      },
    ],
    ...overrides,
  };
}

describe("OptionsExpressionFinalistDashboardPanel", () => {
  it("preserves the finalist QA hook, table label, gate detail, and values", () => {
    const wrapper = mount(OptionsExpressionFinalistDashboardPanel, {
      props: { dashboard: buildDashboard() },
    });

    expect(
      wrapper
        .get('[data-qa-panel="options.finalistDashboard"]')
        .attributes("data-qa-panel"),
    ).toBe("options.finalistDashboard");
    expect(wrapper.text()).toContain("1 / 1 accepted · holdout supported");
    expect(wrapper.get(".finalistGate").attributes("title")).toBe(
      "1/1 accepted; 0 support deficient.",
    );

    const table = wrapper.get('table[aria-label="Options finalist dashboard"]');
    expect(table.element.parentElement?.getAttribute("tabindex")).toBe("0");
    expect(table.text()).toContain("0.457");
    expect(table.text()).toContain("Extreme · 0.070");
    expect(table.text()).toContain("42");
  });

  it("keeps the artifact-level empty state when tuple rows are absent", () => {
    const wrapper = mount(OptionsExpressionFinalistDashboardPanel, {
      props: { dashboard: buildDashboard({ rows: [] }) },
    });

    expect(wrapper.find("table").exists()).toBe(false);
    expect(wrapper.text()).toContain(
      "No tuple-level validation rows were included in this artifact.",
    );
  });
});
