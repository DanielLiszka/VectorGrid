// @vitest-environment jsdom
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import MlSignalWorkspacePanel from "./MlSignalWorkspacePanel.vue";
import { useDataStore } from "../stores/dataStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => ({})),
  tauriRuntimeAvailable: vi.fn(() => false),
}));

describe("MlSignalWorkspacePanel", () => {
  let pinia: ReturnType<typeof createPinia>;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("renders sweep controls and current ML artifact details", () => {
    const data = useDataStore();
    const optimize = useOptimizeStore();
    const workflowArtifacts = useWorkflowArtifactsStore();

    data.dataId = "data-1";
    data.datasetLabel = "prices.csv";
    data.dataTimeframe = "1d";

    optimize.mlSignalScoreCachePath = "C:\\data\\score_cache.json";
    workflowArtifacts.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-20T10:00:00.000Z",
        artifact_path: "runs/vectorgrid_run_ml_signal.json",
        csv_path: "C:\\data\\prices.csv",
        ml_signal: {
          bundle_id: "bundle-a",
          score_cache_path: "C:\\data\\score_cache.json",
          score_manifest_path: "C:\\data\\score_cache.manifest.json",
          feature_schema_path: "C:\\data\\ml_dataset.feature_schema.json",
          feature_schema_version: 1,
          policy: {
            label: "best_policy",
            t_long: 0.6,
            t_short: 0.61,
            delta: 0.08,
            max_hold_bars: 12,
          },
          policy_sweep: {
            sweep_size: 27,
            top_k: 4,
            best_label: "best_policy",
            best_robust_score: 1.44,
          },
        },
        req: {},
        res: {},
      },
      currentValidation: {
        ts: "2026-03-20T10:00:00.000Z",
        req: {},
        res: {
          sweep_size: 27,
          validation: {
            run_verdict: {
              approved_candidate_count: 1,
              candidates_survived_hard_gates: 1,
              candidates_considered: 27,
            },
            candidates: [
              {
                robust_score: 1.44,
                psr: 0.72,
                dsr: 0.68,
                in_sample: {
                  sharpe: 1.23,
                  pnl: 0.34,
                  max_dd: 0.12,
                  trades: 18,
                },
                params: {
                  label: "best_policy",
                  t_long: 0.6,
                  t_short: 0.61,
                  delta: 0.08,
                  max_hold_bars: 12,
                },
              },
            ],
          },
        },
      },
    });

    const wrapper = mount(MlSignalWorkspacePanel, {
      global: { plugins: [pinia] },
    });

    expect(wrapper.find("#mlSignalSweepLongStart").exists()).toBe(true);
    expect(wrapper.find("#mlSignalSweepTopK").exists()).toBe(true);
    expect(wrapper.find("#mlSignalMinPsr").exists()).toBe(true);
    expect(wrapper.find("#mlSignalMinDsr").exists()).toBe(true);
    expect(wrapper.text()).toContain("Current ML artifact");
    expect(wrapper.text()).toContain("bundle-a");
    expect(wrapper.text()).toContain("score_cache.json");
    expect(wrapper.text()).toContain("ml_dataset.feature_schema.json");
    expect(wrapper.text()).toContain("best_policy");
    expect(wrapper.text()).toContain("size 27");
    expect(wrapper.text()).toContain("Policy sweep attached");
    expect(wrapper.text()).toContain("Top retained candidates");
    expect(wrapper.text()).toContain("robust 1.4400");
    expect(wrapper.text()).toContain("approved 1");
    expect(wrapper.text()).toContain("sharpe 1.2300");
    expect(wrapper.text()).toContain("psr 0.7200");
  });
});
