// @vitest-environment jsdom
import { createPinia, setActivePinia } from "pinia";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import RunFooterPanel from "./RunFooterPanel.vue";
import { useCommandActionStore } from "../stores/commandActionStore";
import { useDataStore } from "../stores/dataStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useTaskStore } from "../stores/taskStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";
import { useWorkflowStore } from "../stores/workflowStore";

function installFooterMountShell(): void {
  document.body.innerHTML = `
    <footer id="runFooterShell">
      <div id="runFooterVueShellMount"></div>
    </footer>
  `;
}

async function flushPanel(): Promise<void> {
  await nextTick();
  await nextTick();
}

describe("RunFooterPanel", () => {
  beforeEach(() => {
    installFooterMountShell();
    localStorage.clear();
  });

  it("uses Run optimization as the primary action on the optimize workflow step", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    const actions = useCommandActionStore(pinia);
    workflow.selectWorkflowStep("optimize", { persist: false });
    optimize.runDisabled = false;
    const runSpy = vi
      .spyOn(actions, "runOptimization")
      .mockImplementation(() => {});

    const wrapper = mount(RunFooterPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const primary = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Run optimization",
    ) as HTMLButtonElement | undefined;
    expect(primary).toBeDefined();
    primary?.click();
    expect(runSpy).toHaveBeenCalledTimes(1);

    wrapper.unmount();
  });

  it("keeps optimization as the primary footer action on the validation workflow step", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const dataStore = useDataStore(pinia);
    const optimize = useOptimizeStore(pinia);
    const artifacts = useWorkflowArtifactsStore(pinia);
    const actions = useCommandActionStore(pinia);
    workflow.selectWorkflowStep("validate", { persist: false });
    dataStore.dataId = "AAPL_1D";
    dataStore.datasetLabel = "AAPL";
    artifacts.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-03-09T15:00:00Z",
        req: {},
        res: { top: [] },
      },
    });
    optimize.runDisabled = false;
    const runSpy = vi
      .spyOn(actions, "runOptimization")
      .mockImplementation(() => {});

    const wrapper = mount(RunFooterPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const primary = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Run optimization",
    ) as HTMLButtonElement | undefined;
    expect(primary).toBeDefined();
    expect(primary?.disabled).toBe(false);
    primary?.click();
    expect(runSpy).toHaveBeenCalledTimes(1);
    expect(document.body.textContent || "").toContain("Execution readiness");

    wrapper.unmount();
  });

  it("still renders the optimization footer action when the validation step has no prior run context", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    workflow.selectWorkflowStep("validate", { persist: false });
    optimize.runDisabled = true;

    const wrapper = mount(RunFooterPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const primary = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Run optimization",
    ) as HTMLButtonElement | undefined;
    expect(primary).toBeDefined();
    expect(primary?.disabled).toBe(true);

    wrapper.unmount();
  });

  it("restores expanded footer details from storage and persists hide/show toggles", async () => {
    localStorage.setItem("VTA_RUN_FOOTER_EXPANDED_V1", "true");

    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    workflow.selectWorkflowStep("optimize", { persist: false });

    const wrapper = mount(RunFooterPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    expect(document.body.textContent || "").toContain("Readiness");
    expect(document.body.textContent || "").toContain("Insights");

    const toggle = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Hide details",
    ) as HTMLButtonElement | undefined;
    expect(toggle).toBeDefined();

    toggle?.click();
    await flushPanel();

    expect(localStorage.getItem("VTA_RUN_FOOTER_EXPANDED_V1")).toBe("false");
    expect(document.querySelector(".summaryGrid")).toBeNull();
    expect(document.body.textContent || "").toContain("Show details");

    wrapper.unmount();
  });

  it("renames the primary footer action for MlSignal mode while keeping the same dispatch hook", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const actions = useCommandActionStore(pinia);
    workflow.selectWorkflowStep("optimize", { persist: false });
    workflow.selectMode("ml-signal", { persist: false });
    useOptimizeStore(pinia).runDisabled = false;
    const runSpy = vi
      .spyOn(actions, "runOptimization")
      .mockImplementation(() => {});

    const wrapper = mount(RunFooterPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const primary = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Build score cache",
    ) as HTMLButtonElement | undefined;
    expect(primary).toBeDefined();
    primary?.click();
    expect(runSpy).toHaveBeenCalledTimes(1);

    wrapper.unmount();
  });

  it("routes footer cancellation through the existing command action when a run is cancellable", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    const actions = useCommandActionStore(pinia);
    workflow.selectWorkflowStep("optimize", { persist: false });
    optimize.$patch({ canCancel: true, runDisabled: false, progressMeta: "Optimize 84/100" });
    const cancelSpy = vi
      .spyOn(actions, "cancelOptimization")
      .mockImplementation(() => {});

    const wrapper = mount(RunFooterPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const cancel = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Cancel",
    ) as HTMLButtonElement | undefined;
    expect(cancel).toBeDefined();
    expect(cancel?.disabled).toBe(false);

    cancel?.click();
    expect(cancelSpy).toHaveBeenCalledTimes(1);
    expect(document.body.textContent || "").toContain("Optimize 84/100");

    wrapper.unmount();
  });

  it("does not render the workflow footer outside the workflow workspace", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    workflow.selectWorkspace("settings");

    const wrapper = mount(RunFooterPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    expect(document.body.textContent || "").not.toContain("Execution readiness");
    expect(
      Array.from(document.querySelectorAll("button")).some(
        (button) => button.textContent?.trim() === "Run optimization",
      ),
    ).toBe(false);

    wrapper.unmount();
  });

  it("surfaces grouped task posture in the footer task card", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    const taskStore = useTaskStore(pinia);
    workflow.selectWorkflowStep("optimize", { persist: false });
    optimize.progressMeta = "Optimize 42/100";
    taskStore.$patch({
      activeById: {
        task_running: {
          id: "task_running",
          command: "run_double_ma_optimization",
          label: "Optimization",
          status: "running",
          startedAt: 1_700_000_000_000,
          message: "Evaluate (42.0%)",
        },
        task_queued: {
          id: "task_queued",
          command: "run_double_ma_validation",
          label: "Validation",
          status: "queued",
          startedAt: 1_700_000_000_100,
          message: "Queued.",
        },
      },
      activeOrder: ["task_running", "task_queued"],
      history: [
        {
          id: "task_failed",
          command: "run_double_ma_validation",
          label: "Validation",
          status: "failed",
          startedAt: 1_700_000_000_000,
          endedAt: 1_700_000_001_000,
          error: "Validation failed.",
        },
      ],
    });

    const wrapper = mount(RunFooterPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const text = document.body.textContent || "";
    expect(text).toContain("1 running");
    expect(text).toContain("1 queued");
    expect(text).toContain("1 failed");
    expect(text).toContain("Attention");

    wrapper.unmount();
  });
});
