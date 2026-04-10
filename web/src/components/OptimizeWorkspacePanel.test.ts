// @vitest-environment jsdom
import { createPinia, setActivePinia } from "pinia";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import OptimizeWorkspacePanel from "./OptimizeWorkspacePanel.vue";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useStrategyStore } from "../stores/strategyStore";
import { useWorkflowStore } from "../stores/workflowStore";

vi.mock("../services/movingAverageCatalogService", async () => {
  const actual = await vi.importActual<
    typeof import("../services/movingAverageCatalogService")
  >("../services/movingAverageCatalogService");
  return {
    ...actual,
    listMovingAverageCatalog: vi.fn(async () => [
      {
        id: "sma",
        label: "SMA",
        supportsCpu: true,
        supportsCuda: true,
        supportsCudaMaSweep: true,
        supportsCudaKernel: true,
        prefersCudaKernel: true,
        requiresCandles: false,
        periodBased: true,
        singleOutput: true,
        notes: "",
        params: [],
      },
      {
        id: "ema",
        label: "EMA",
        supportsCpu: true,
        supportsCuda: true,
        supportsCudaMaSweep: true,
        supportsCudaKernel: true,
        prefersCudaKernel: true,
        requiresCandles: false,
        periodBased: true,
        singleOutput: true,
        notes: "",
        params: [],
      },
      {
        id: "alma",
        label: "ALMA",
        supportsCpu: true,
        supportsCuda: true,
        supportsCudaMaSweep: true,
        supportsCudaKernel: true,
        prefersCudaKernel: true,
        requiresCandles: false,
        periodBased: true,
        singleOutput: true,
        notes: "",
        params: [
          {
            key: "offset",
            label: "Offset",
            kind: "float",
            defaultValue: 0.85,
            min: 0,
            max: 1,
            step: 0.05,
            notes: "ALMA offset.",
          },
          {
            key: "sigma",
            label: "Sigma",
            kind: "float",
            defaultValue: 6,
            min: 1,
            max: 10,
            step: 0.5,
            notes: "ALMA sigma.",
          },
        ],
      },
      {
        id: "vidya",
        label: "VIDYA",
        supportsCpu: true,
        supportsCuda: true,
        supportsCudaMaSweep: true,
        supportsCudaKernel: false,
        prefersCudaKernel: false,
        requiresCandles: false,
        periodBased: true,
        singleOutput: true,
        notes: "",
        params: [
          {
            key: "alpha",
            label: "Alpha",
            kind: "float",
            defaultValue: 0.2,
            min: 0.01,
            max: 1,
            step: 0.01,
            notes: "VIDYA alpha.",
          },
        ],
      },
    ]),
  };
});

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

function setInputValue(selector: string, value: string): void {
  const input = document.querySelector<HTMLInputElement>(selector);
  expect(input).not.toBeNull();
  input!.value = value;
  input!.dispatchEvent(new Event("input", { bubbles: true }));
}

function setCheckboxValue(selector: string, checked: boolean): void {
  const input = document.querySelector<HTMLInputElement>(selector);
  expect(input).not.toBeNull();
  input!.checked = checked;
  input!.dispatchEvent(new Event("change", { bubbles: true }));
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
    expect(bodyText).toContain(
      "explicit metric-contract overrides remain available",
    );
    expect(bodyText).toContain(
      "exact-kernel shortlist controls stay disabled on hybrid execution.",
    );

    wrapper.unmount();
  });

  it("shows the GPU kernel composite route preview when BreakoutMomentum keeps ADX disabled", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    const strategy = useStrategyStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "BreakoutMomentum";
    optimize.backend = "GpuKernel";
    strategy.breakoutUseAdx = false;
    vi.spyOn(optimize, "refreshGpuStatus").mockImplementation(() => {});

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain("Route: GPU kernel");
    expect(bodyText).toContain(
      "Composite route preview: current inputs resolve to the GPU kernel route.",
    );

    wrapper.unmount();
  });

  it("shows the hybrid composite route preview for RegimeHybrid without promising a pure kernel route", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "RegimeHybrid";
    optimize.backend = "GpuKernel";
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
    expect(bodyText).not.toContain("Route: GPU kernel");

    wrapper.unmount();
  });

  it("shows the kernel composite route preview when BreakoutMomentum keeps ADX disabled", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    const strategy = useStrategyStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "BreakoutMomentum";
    optimize.backend = "GpuKernel";
    strategy.breakoutUseAdx = false;
    vi.spyOn(optimize, "refreshGpuStatus").mockImplementation(() => {});

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain("Route: GPU kernel");
    expect(bodyText).toContain(
      "Composite route preview: current inputs resolve to the GPU kernel route.",
    );
    expect(bodyText).toContain(
      "Export-all and exact-kernel shortlist controls stay available on this route.",
    );

    wrapper.unmount();
  });

  it("shows the hybrid composite route preview when RegimeHybrid is selected", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "RegimeHybrid";
    optimize.backend = "GpuKernel";
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
    expect(bodyText).toContain(
      "explicit metric-contract overrides remain available",
    );
    expect(bodyText).toContain(
      "exact-kernel shortlist controls stay disabled on hybrid execution.",
    );

    wrapper.unmount();
  });

  it("shows the CPU composite route preview when CompositeTrend explicitly requests CpuOnly", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    const strategy = useStrategyStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "CompositeTrend";
    optimize.backend = "CpuOnly";
    strategy.trendUseAdx = false;
    vi.spyOn(optimize, "refreshGpuStatus").mockImplementation(() => {});

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain("Route: CPU");
    expect(bodyText).toContain(
      "Composite route preview: current inputs resolve to the CPU route.",
    );
    expect(bodyText).toContain(
      "explicit metric-contract overrides remain available",
    );
    expect(bodyText).toContain(
      "export-all and exact-kernel shortlist controls do not.",
    );

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

  it("renders persisted DynamicDoubleMa preview resolution and reasons from the store snapshot", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "DynamicDoubleMa";
    optimize.dynamicStrategyPreviewMeta =
      "Dynamic strategy preview: backend=gpu_unified | resolved=cpu | indicators=2";
    optimize.dynamicStrategyPreviewReasons =
      "Reasons: halftrend unsupported in gpu v1";
    optimize.dynamicStrategyPreviewPrettyJson =
      '{\n  "requested_backend": "gpu_unified",\n  "resolved_backend": "cpu"\n}';
    vi.spyOn(optimize, "refreshGpuStatus").mockImplementation(() => {});

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain("resolved=cpu");
    expect(bodyText).toContain("halftrend unsupported in gpu v1");
    expect(document.querySelector(".jsonPreview")?.textContent || "").toContain(
      '"profile": "double_ma"',
    );

    wrapper.unmount();
  });

  it("restores seeded MA parameter values and sweep fields into the live grid editor", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "DoubleMa";
    optimize.fastMaType = "alma";
    optimize.slowMaType = "vidya";
    optimize.syncMaParamState({
      fast: {
        params: { offset: 0.72, sigma: 5.5 },
        sweeps: { offset: [0.5, 0.9, 0.1] },
      },
      slow: {
        params: { alpha: 0.33 },
        sweeps: { alpha: [0.2, 0.5, 0.05] },
      },
    });
    vi.spyOn(optimize, "refreshGpuStatus").mockImplementation(() => {});

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });

    await vi.waitFor(() => {
      expect(
        document.querySelector<HTMLInputElement>("#vue_fast_alma_param_offset")
          ?.value,
      ).toBe("0.72");
    });

    expect(
      document.querySelector<HTMLInputElement>(
        "#vue_fast_alma_param_sigma",
      )?.value,
    ).toBe("5.5");
    expect(
      document.querySelector<HTMLInputElement>(
        "#vue_fast_alma_param_offset_sweep_start",
      )?.value,
    ).toBe("0.5");
    expect(
      document.querySelector<HTMLInputElement>(
        "#vue_fast_alma_param_offset_sweep_end",
      )?.value,
    ).toBe("0.9");
    expect(
      document.querySelector<HTMLInputElement>(
        "#vue_fast_alma_param_offset_sweep_step",
      )?.value,
    ).toBe("0.1");
    expect(
      document.querySelector<HTMLInputElement>("#vue_slow_vidya_param_alpha")
        ?.value,
    ).toBe("0.33");
    expect(
      document.querySelector<HTMLInputElement>(
        "#vue_slow_vidya_param_alpha_sweep_start",
      )?.value,
    ).toBe("0.2");
    expect(
      document.querySelector<HTMLInputElement>(
        "#vue_slow_vidya_param_alpha_sweep_end",
      )?.value,
    ).toBe("0.5");
    expect(
      document.querySelector<HTMLInputElement>(
        "#vue_slow_vidya_param_alpha_sweep_step",
      )?.value,
    ).toBe("0.05");

    wrapper.unmount();
  });

  it("writes MA parameter sweep edits back into the optimize store", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const optimize = useOptimizeStore(pinia);
    workflow.selectControlsTab("opt", { persist: false });
    optimize.strategyKind = "DoubleMa";
    optimize.fastMaType = "alma";
    optimize.slowMaType = "alma";
    optimize.syncMaParamState({
      fast: {
        params: { offset: 0.75, sigma: 6 },
        sweeps: null,
      },
      slow: {
        params: { offset: 0.85, sigma: 7 },
        sweeps: null,
      },
    });
    vi.spyOn(optimize, "refreshGpuStatus").mockImplementation(() => {});

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });

    await vi.waitFor(() => {
      expect(
        document.querySelector<HTMLInputElement>("#vue_fast_alma_param_offset"),
      ).not.toBeNull();
    });

    setInputValue("#vue_fast_alma_param_offset", "0.61");
    setCheckboxValue(
      ".maParamSide .maParamBlock .maParamSweepToggle input[type='checkbox']",
      true,
    );
    setInputValue("#vue_fast_alma_param_offset_sweep_start", "0.4");
    setInputValue("#vue_fast_alma_param_offset_sweep_end", "0.8");
    setInputValue("#vue_fast_alma_param_offset_sweep_step", "0.2");
    await flushPanel();

    expect(optimize.fastMaParams).toEqual({ offset: 0.61, sigma: 6 });
    expect(optimize.fastMaParamSweeps).toEqual({ offset: [0.4, 0.8, 0.2] });
    expect(optimize.slowMaParams).toEqual({ offset: 0.85, sigma: 7 });
    expect(optimize.slowMaParamSweeps).toBeNull();

    wrapper.unmount();
  });
});
