// @vitest-environment jsdom
import { createPinia, setActivePinia } from "pinia";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import OptimizeWorkspacePanel from "./OptimizeWorkspacePanel.vue";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useStrategyStore } from "../stores/strategyStore";
import { useWorkflowStore } from "../stores/workflowStore";

function installOptimizePanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="opt"></section>
    <div id="optVueShellMount"></div>
  `;
}

async function flushPanel(): Promise<void> {
  await nextTick();
  await nextTick();
}

describe("OptimizeWorkspacePanel", () => {
  beforeEach(() => {
    installOptimizePanelMountShell();
    localStorage.clear();
  });

  it("shows backend and mode controls for Double MA", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "DoubleMa";
    vi.spyOn(optimize, "refreshGpuStatus").mockImplementation(() => {});

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    expect(document.querySelector("#vueBackend")).not.toBeNull();
    expect(document.querySelector("#vueMode")).not.toBeNull();

    wrapper.unmount();
  });

  it("hides backend and mode controls when Composite MO manages its own execution path", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "CompositeMo";
    vi.spyOn(optimize, "refreshGpuStatus").mockImplementation(() => {});

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    expect(document.querySelector("#vueBackend")).toBeNull();
    expect(document.querySelector("#vueMode")).toBeNull();
    expect(document.body.textContent || "").toContain(
      "Composite MO runs a CPU-side multi-objective evaluator",
    );

    wrapper.unmount();
  });

  it("normalizes legacy GPU backend aliases into the unified GPU panel messaging", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "DoubleMa";
    optimize.backend = "GpuOnly";
    vi.spyOn(optimize, "refreshGpuStatus").mockImplementation(() => {});

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain("Route: GPU unified");
    expect(bodyText).toContain("Unified route:");
    expect(bodyText).not.toContain("GpuOnly");

    wrapper.unmount();
  });

  it("shows the hybrid composite route preview when CompositeTrend keeps ADX enabled", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    const strategy = useStrategyStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "CompositeTrend";
    optimize.backend = "Auto";
    strategy.trendUseAdx = true;
    vi.spyOn(optimize, "refreshGpuStatus").mockImplementation(() => {});

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain("Route: GPU hybrid");
    expect(bodyText).toContain(
      "Composite route preview: current inputs resolve to the hybrid GPU route.",
    );
    expect(bodyText).toContain("Export-all stays disabled on hybrid execution.");

    wrapper.unmount();
  });

  it("shows constrained dynamic builder controls for DynamicDoubleMa and keeps the normal search-space pane visible", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "DynamicDoubleMa";
    vi.spyOn(optimize, "refreshGpuStatus").mockImplementation(() => {});

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    expect(document.querySelector("#vueDynamicStrategyIrJson")).not.toBeNull();
    expect(document.body.textContent || "").toContain(
      "Constrained Dynamic Double-MA builder",
    );
    expect(document.body.textContent || "").toContain("Preview Builder IR");
    expect(document.body.textContent || "").toContain("Constant");
    expect(document.body.textContent || "").toContain(
      "Desktop-only preview commands are unavailable in this plain browser session",
    );
    expect(document.body.textContent || "").toContain(
      "Fast and slow sweep ranges",
    );
    expect(
      (
        document.querySelector(
          'button[title*="desktop shell"]',
        ) as HTMLButtonElement | null
      )?.disabled,
    ).toBe(true);

    wrapper.unmount();
  });

  it("keeps the optimize panel conventional even if strategy state is stale MlSignal", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "MlSignal";
    vi.spyOn(optimize, "refreshGpuStatus").mockImplementation(() => {});

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    expect(document.body.textContent || "").not.toContain(
      "Pattern Recognition + ML signal validation",
    );
    expect(document.querySelector("#mlSignalDatasetExportPath")).toBeNull();
    expect(document.body.textContent || "").toContain("Run optimization");

    wrapper.unmount();
  });

  it("renders live optimize status and action readiness from store state", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "DoubleMa";
    optimize.strategyHint = "Live strategy hint.";
    optimize.pairsPreview = "Pairs: 120";
    optimize.progressMeta = "42% complete";
    optimize.statusLine = "Optimization ready.";
    optimize.runDisabled = true;
    optimize.validateDisabled = false;
    vi.spyOn(optimize, "refreshGpuStatus").mockImplementation(() => {});

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain("Live strategy hint.");
    expect(bodyText).toContain("Pairs: 120");
    expect(bodyText).toContain("42% complete");
    expect(bodyText).toContain("Optimization ready.");
    const buttons = Array.from(
      document.querySelectorAll<HTMLButtonElement>("button"),
    );
    const runButton = buttons.find((button) =>
      String(button.textContent || "").includes("Run optimization"),
    );
    const validateButton = buttons.find((button) =>
      String(button.textContent || "").includes("Run validation"),
    );
    expect(runButton?.disabled).toBe(true);
    expect(validateButton?.disabled).toBe(false);

    wrapper.unmount();
  });

  it("restores persisted DynamicDoubleMa builder state from the store snapshot", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "DynamicDoubleMa";
    optimize.dynamicStrategyBuilderStateJson = JSON.stringify({
      longOnly: true,
      allowFlip: false,
      indicators: [
        {
          id: "saved_indicator",
          indicatorId: "vidya",
          outputId: "vidya",
          params: {},
        },
      ],
      conditions: [
        {
          id: "saved_condition",
          op: "gt",
          lhs: { kind: "indicator", value: "saved_indicator" },
          rhs: { kind: "close", value: "close" },
        },
      ],
      groups: [
        {
          id: "saved_group",
          mode: "all",
          conditionIds: ["saved_condition"],
        },
      ],
      signals: {
        longEntryGroup: "saved_group",
        longExitGroup: "",
        shortEntryGroup: "",
        shortExitGroup: "",
      },
    });
    vi.spyOn(optimize, "refreshGpuStatus").mockImplementation(() => {});

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    expect(document.body.textContent || "").toContain("saved_indicator");
    expect(document.body.textContent || "").toContain("saved_group");

    wrapper.unmount();
  });
});
