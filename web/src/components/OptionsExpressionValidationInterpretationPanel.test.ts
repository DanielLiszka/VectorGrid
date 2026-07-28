import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import OptionsExpressionValidationInterpretationPanel from "./OptionsExpressionValidationInterpretationPanel.vue";

describe("OptionsExpressionValidationInterpretationPanel", () => {
  it("stays absent when no validation interpretation rows are available", () => {
    const wrapper = mount(OptionsExpressionValidationInterpretationPanel, {
      props: {
        scopeRows: [],
        driftRows: [],
        monotonicityRows: [],
        regretRows: [],
      },
    });

    expect(wrapper.find("details").exists()).toBe(false);
  });

  it("renders every interpretation lane with keyboard-scrollable tables", () => {
    const wrapper = mount(OptionsExpressionValidationInterpretationPanel, {
      props: {
        scopeRows: [
          {
            key: "retained",
            label: "Retained frontier scope",
            value: "Within retained top_k only",
          },
        ],
        driftRows: [
          {
            key: "drift",
            label: "Selection -> Holdout",
            value: "-0.0123",
            scopeLabel: "Selected tuple only",
          },
        ],
        monotonicityRows: [
          {
            key: "monotonicity",
            label: "Holdout family",
            statusLabel: "Monotone",
            scopeLabel: "Selected tuple only",
            detail: "Stress remains ordered.",
          },
        ],
        regretRows: [
          {
            key: "regret",
            label: "Retained holdout regret",
            value: "0.0042",
            scopeLabel: "Within retained top_k only",
          },
        ],
      },
    });

    expect(wrapper.get("summary").text()).toBe("Validation interpretation");
    expect(wrapper.text()).toContain("Retained frontier scope");
    expect(wrapper.text()).toContain("Selection -> Holdout");
    expect(wrapper.text()).toContain("Stress remains ordered.");
    expect(wrapper.text()).toContain("Retained holdout regret");

    const tables = wrapper.findAll("table");
    expect(tables.map((table) => table.attributes("aria-label"))).toEqual([
      "Selected tuple validation drift",
      "Selected tuple validation monotonicity",
      "Selected tuple validation regret",
    ]);
    expect(wrapper.findAll(".tableWrap")).toHaveLength(3);
    expect(
      wrapper
        .findAll(".tableWrap")
        .every((table) => table.attributes("tabindex") === "0"),
    ).toBe(true);
  });
});
