import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import {
  createDefaultVcetSearchDraft,
  type VcetSearchDraft,
} from "../services/vcetSearchRequestBuilder";
import VcetCampaignSearchControls from "./VcetCampaignSearchControls.vue";

describe("VcetCampaignSearchControls", () => {
  it("presents the legacy period grid without implying multidimensional search", () => {
    const wrapper = mount(VcetCampaignSearchControls, {
      props: {
        designMode: "period_grid",
        draft: createDefaultVcetSearchDraft(),
      },
    });

    expect(wrapper.attributes("data-design-mode")).toBe("period_grid");
    expect(wrapper.get(".designValidation").text()).toContain(
      "current Optimization ranges",
    );
    expect(wrapper.find("#vcetSearchBudget").exists()).toBe(false);
    expect(wrapper.text()).toContain("Fast×Slow grid");
    expect(wrapper.text()).toContain("Bounded multidimensional");
  });

  it("renders every bounded backend axis and a deterministic design preview", () => {
    const wrapper = mount(VcetCampaignSearchControls, {
      props: {
        designMode: "halton",
        draft: createDefaultVcetSearchDraft(),
      },
    });

    expect(wrapper.attributes("data-design-valid")).toBe("true");
    expect(wrapper.get("#vcetSearchBudget").attributes("max")).toBe("8192");
    expect(wrapper.get("#vcetSearchSeed").attributes("max")).toBe(
      String(Number.MAX_SAFE_INTEGER),
    );
    expect(wrapper.findAll(".rangeCard")).toHaveLength(6);
    expect(wrapper.text()).toContain("VIDYA short period");
    expect(wrapper.text()).toContain("VIDYA long period");
    expect(wrapper.text()).toContain("VIDYA alpha");
    expect(wrapper.text()).toContain("Chandelier period");
    expect(wrapper.text()).toContain("Chandelier multiplier");
    expect(wrapper.text()).toContain("Maximum trade bars");
    expect(wrapper.get("#vcetSearchCeUseClose").attributes("id")).toBe(
      "vcetSearchCeUseClose",
    );
    expect(wrapper.get("#vcetSearchRequireSlope").attributes("id")).toBe(
      "vcetSearchRequireSlope",
    );
    expect(wrapper.get(".designValidation").text()).toContain(
      "2,048 unique candidates",
    );
  });

  it("emits immutable draft edits and reports invalid quantization", async () => {
    const wrapper = mount(VcetCampaignSearchControls, {
      props: {
        designMode: "halton",
        draft: createDefaultVcetSearchDraft(),
      },
    });
    const original = wrapper.props("draft");

    await wrapper.get("#vcetSearch_ce_mult_max").setValue("5.1");
    const draftEvents = wrapper.emitted("update:draft") ?? [];
    const emitted = draftEvents[draftEvents.length - 1]?.[0] as VcetSearchDraft;
    expect(emitted).toMatchObject({ ceMultMax: "5.1", budget: "2048" });
    expect(original.ceMultMax).toBe("5");

    await wrapper.setProps({ draft: emitted });
    expect(wrapper.attributes("data-design-valid")).toBe("false");
    expect(wrapper.get(".designValidation").text()).toContain(
      "endpoints must align exactly",
    );
  });

  it("switches design modes and resets all Halton controls explicitly", async () => {
    const draft = { ...createDefaultVcetSearchDraft(), budget: "17" };
    const wrapper = mount(VcetCampaignSearchControls, {
      props: { designMode: "halton", draft },
    });

    await wrapper
      .get('input[name="vcetCandidateDesignMode"][value="period_grid"]')
      .setValue(true);
    const modeEvents = wrapper.emitted("update:design-mode") ?? [];
    expect(modeEvents[modeEvents.length - 1]?.[0]).toBe("period_grid");

    await wrapper
      .get('[data-qa-action="aggregate.batch.vcetSearch.reset"]')
      .trigger("click");
    const draftEvents = wrapper.emitted("update:draft") ?? [];
    expect(draftEvents[draftEvents.length - 1]?.[0]).toMatchObject({
      budget: "2048",
      seed: "0",
      ceUseClose: "both",
      requireVidyaSlope: "both",
    });
  });
});
