import { createPinia, setActivePinia } from "pinia";
import { flushPromises, mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { LAST_BATCH_SUMMARY_KEY } from "../utils/core/storageKeys";
import { useWorkflowStore } from "../stores/workflowStore";
import MetaWorkspacePanel from "./MetaWorkspacePanel.vue";

const invokeCommandMock = vi.hoisted(() => vi.fn());

vi.mock("../tauri/invoke", async (importOriginal) => {
  const original = await importOriginal<typeof import("../tauri/invoke")>();
  return {
    ...original,
    invokeCommand: (...args: unknown[]) => invokeCommandMock(...args),
  };
});

function eligibleVcetCampaignSummary(): Record<string, unknown> {
  const candidateManifestHash = "b".repeat(64);
  return {
    kind: "vectorgrid_campaign_summary",
    version: 2,
    state: "completed",
    strategy_kind: "Vcet",
    counts: { cancelled: false },
    request: {
      strategy_kind: "Vcet",
      execution_mode: "evaluate_candidates",
      datasets: [{ csv_path: "C:/a.csv" }, { csv_path: "C:/b.csv" }],
    },
    manifest: {
      strategy_kind: "Vcet",
      execution_mode: "evaluate_candidates",
      manifest_hash: "a".repeat(64),
      candidate_manifest_hash: candidateManifestHash,
    },
    fixed_candidate_evaluation: {
      schema_version: 4,
      candidate_manifest_hash: candidateManifestHash,
      admitted_candidate_count: 2,
      admitted_candidate_ids: ["candidate-1", "candidate-2"],
    },
    items: [
      {
        item_id: "dataset-a",
        is_holdout: false,
        status: "ok",
        artifact_path: "C:/runs/a.json",
        artifact_sha256: "c".repeat(64),
      },
      {
        item_id: "dataset-b",
        is_holdout: true,
        status: "ok",
        artifact_path: "C:/runs/b.json",
        artifact_sha256: "d".repeat(64),
      },
    ],
  };
}

describe("MetaWorkspacePanel temporal source refresh", () => {
  beforeEach(() => {
    document.body.innerHTML = `
      <section data-vue-panel="meta"></section>
      <div id="metaVueShellMount"></div>
    `;
    localStorage.clear();
    invokeCommandMock.mockReset();
  });

  it("settles an eligible mounted panel after one disk-backed artifact read", async () => {
    const summaryPath = "C:/campaign/vcet-summary.json";
    localStorage.setItem(LAST_BATCH_SUMMARY_KEY, summaryPath);
    invokeCommandMock.mockImplementation(
      async (command: string, args: unknown) => {
        expect(command).toBe("load_artifact");
        expect(args).toEqual({ path: summaryPath });
        return eligibleVcetCampaignSummary();
      },
    );

    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    workflow.selectResultsTab("meta", { persist: false });
    workflow.selectAggregateTab("batch", { persist: false });

    const wrapper = mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPromises();
    await nextTick();
    await flushPromises();
    await nextTick();

    const loadArtifactCalls = () =>
      invokeCommandMock.mock.calls.filter(
        ([command]) => command === "load_artifact",
      );
    expect(loadArtifactCalls()).toHaveLength(1);
    const temporalHost = document.querySelector(
      '[data-testid="vcet-temporal-campaign-host"]',
    );
    expect(temporalHost?.textContent || "").toContain(
      "Structurally eligible terminal VCET source",
    );
    expect(
      temporalHost?.querySelector<HTMLButtonElement>(
        '[data-testid="vcet-temporal-prepare"]',
      )?.disabled,
    ).toBe(false);

    await flushPromises();
    await nextTick();
    expect(loadArtifactCalls()).toHaveLength(1);

    wrapper.unmount();
  });
});
