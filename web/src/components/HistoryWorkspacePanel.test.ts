// @vitest-environment jsdom
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { nextTick } from "vue";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import HistoryWorkspacePanel from "./HistoryWorkspacePanel.vue";
import { useHistoryStore } from "../stores/historyStore";
import { useWorkflowStore } from "../stores/workflowStore";

describe("HistoryWorkspacePanel", () => {
  let pinia: ReturnType<typeof createPinia>;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    document.body.innerHTML = `
      <div data-vue-panel="history"></div>
      <div id="historyVueShellMount"></div>
    `;
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("renders MlSignal saved-run verdict and top candidate summaries", async () => {
    const history = useHistoryStore();
    const workflow = useWorkflowStore();

    workflow.workspace = "library";
    history.savedRunCount = 1;
    history.savedSelectedValue = "C:/runs/ml_signal.json";
    history.savedRunDetailTitle = "2026-03-20T10:00:00Z";
    history.savedRunDetailMeta = "artifact: ml_signal.json";
    history.savedRunDetailStrategy = "MlSignal";
    history.savedRunDetailDataset = "spy.csv";
    history.savedRunDetailTimeframe = "1d";
    history.savedRunDetailValidation = "Available";
    history.savedRunDetailReport = "Not exported";
    history.savedRunDetailSize = "12 KiB";
    history.savedRunDetailTags = "ml, research";
    history.savedRunDetailMlBundle = "bundle-a";
    history.savedRunDetailMlScoreCache = "ml_score_cache.json";
    history.savedRunDetailMlFeatureSchema = "ml_dataset.feature_schema.json";
    history.savedRunDetailMlPolicy =
      "t_long=0.6 | t_short=0.55 | delta=0.1 | hold=10";
    history.savedRunDetailMlSweep =
      "size=27 | top_k=4 | best=best_sweep | score=1.44";
    history.savedRunDetailMlVerdict = "approved=1 | survived=1 | considered=27";
    history.savedRunDetailMlCandidates =
      "best_sweep | robust=1.44 | psr=0.72 | dsr=0.68";
    history.savedRunDetailParams = "strategy: MlSignal";

    mount(HistoryWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    });

    await nextTick();
    await nextTick();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain("Verdict");
    expect(bodyText).toContain("approved=1 | survived=1 | considered=27");
    expect(bodyText).toContain("Top Candidates");
    expect(bodyText).toContain(
      "best_sweep | robust=1.44 | psr=0.72 | dsr=0.68",
    );
  });
});
