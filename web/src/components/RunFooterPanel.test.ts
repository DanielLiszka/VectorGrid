// @vitest-environment jsdom
import { createPinia, setActivePinia } from "pinia";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import RunFooterPanel from "./RunFooterPanel.vue";
import { useCommandActionStore } from "../stores/commandActionStore";
import { useDataStore } from "../stores/dataStore";
import { useOptimizeStore } from "../stores/optimizeStore";
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

  it("renames the primary footer action for MlSignal while keeping the same dispatch hook", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    const actions = useCommandActionStore(pinia);
    workflow.selectWorkflowStep("optimize", { persist: false });
    optimize.strategyKind = "MlSignal";
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
      (button) => button.textContent?.trim() === "Build score cache",
    ) as HTMLButtonElement | undefined;
    expect(primary).toBeDefined();
    primary?.click();
    expect(runSpy).toHaveBeenCalledTimes(1);

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
});
