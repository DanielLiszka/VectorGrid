
import { createPinia, setActivePinia } from "pinia";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import MetaWorkspacePanel from "./MetaWorkspacePanel.vue";
import { useAnalysisStore } from "../stores/analysisStore";
import { useHistoryStore } from "../stores/historyStore";
import { useWorkflowStore } from "../stores/workflowStore";
import { META_RESULTS_TABLE_STATE_KEY } from "../utils/core/storageKeys";

function installMetaPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="meta"></section>
    <div id="metaVueShellMount"></div>
  `;
}

async function flushPanel(): Promise<void> {
  await nextTick();
  await nextTick();
}

describe("MetaWorkspacePanel", () => {
  beforeEach(() => {
    installMetaPanelMountShell();
    localStorage.clear();
  });

  it("persists visible meta result columns and restores defaults", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const historyStore = useHistoryStore(pinia);
    const analysisStore = useAnalysisStore(pinia);
    workflow.selectResultsTab("meta", { persist: false });
    workflow.selectAggregateTab("meta", { persist: false });
    historyStore.savedRunCount = 1;
    analysisStore.metaTableColumns = [
      { label: "Group", alignLeft: true },
      { label: "Selection", alignLeft: true },
      { label: "Score", alignLeft: false },
    ];
    analysisStore.metaTableRows = [
      {
        cells: [
          { text: "Momentum", className: "left", title: "Momentum" },
          { text: "Candidate A", className: "left", title: "Candidate A" },
          { text: "0.88", className: "", title: "0.88" },
        ],
      },
    ];
    analysisStore.metaOutputPreview = '{"summary":"ok"}';

    const wrapper = mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const scoreToggle = document.querySelector<HTMLInputElement>(
      "#metaColumnToggle_col_2",
    );
    expect(scoreToggle).not.toBeNull();
    scoreToggle!.checked = false;
    scoreToggle!.dispatchEvent(new Event("change", { bubbles: true }));
    await flushPanel();

    const headerText = Array.from(document.querySelectorAll("th")).map((cell) =>
      String(cell.textContent || "").trim(),
    );
    expect(headerText).toEqual(["Group", "Selection"]);
    expect(localStorage.getItem(META_RESULTS_TABLE_STATE_KEY)).toContain(
      '"visibleColumnKeys":["col_0","col_1"]',
    );

    wrapper.unmount();

    mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const remountHeaderText = Array.from(document.querySelectorAll("th")).map(
      (cell) => String(cell.textContent || "").trim(),
    );
    expect(remountHeaderText).toEqual(["Group", "Selection"]);

    const restoreButton = Array.from(
      document.querySelectorAll<HTMLButtonElement>("button"),
    ).find((button) =>
      String(button.textContent || "").includes("Restore defaults"),
    );
    restoreButton?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await flushPanel();

    const restoredHeaderText = Array.from(document.querySelectorAll("th")).map(
      (cell) => String(cell.textContent || "").trim(),
    );
    expect(restoredHeaderText).toEqual(["Group", "Selection", "Score"]);
    expect(localStorage.getItem(META_RESULTS_TABLE_STATE_KEY)).toBeNull();
  });

  it("applies persisted meta result column order and keyboard-resizable widths", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const historyStore = useHistoryStore(pinia);
    const analysisStore = useAnalysisStore(pinia);
    workflow.selectResultsTab("meta", { persist: false });
    workflow.selectAggregateTab("meta", { persist: false });
    historyStore.savedRunCount = 1;
    analysisStore.metaTableColumns = [
      { label: "Group", alignLeft: true },
      { label: "Selection", alignLeft: true },
      { label: "Score", alignLeft: false },
    ];
    analysisStore.metaTableRows = [
      {
        cells: [
          { text: "Momentum", className: "left", title: "Momentum" },
          { text: "Candidate A", className: "left", title: "Candidate A" },
          { text: "0.88", className: "", title: "0.88" },
        ],
      },
    ];
    localStorage.setItem(
      META_RESULTS_TABLE_STATE_KEY,
      JSON.stringify({
        visibleColumnKeys: ["col_0", "col_1", "col_2"],
        knownColumnKeys: ["col_0", "col_1", "col_2"],
        columnOrderKeys: ["col_2", "col_0", "col_1"],
        columnWidths: { col_2: 220 },
      }),
    );

    mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const headers = Array.from(document.querySelectorAll("th"));
    expect(headers.map((cell) => String(cell.textContent || "").trim())).toEqual([
      "Score",
      "Group",
      "Selection",
    ]);
    expect((headers[0] as HTMLElement).style.width).toBe("220px");

    const scoreResize = headers[0]?.querySelector<HTMLButtonElement>(
      'button[aria-label="Resize Score column"]',
    );
    expect(scoreResize).not.toBeNull();
    scoreResize?.dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }),
    );
    await flushPanel();

    expect(localStorage.getItem(META_RESULTS_TABLE_STATE_KEY)).toContain(
      '"col_2":236',
    );
  });

  it("reuses the shared contextual action menu across saved aggregate artifact families", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const historyStore = useHistoryStore(pinia);
    const analysisStore = useAnalysisStore(pinia);

    workflow.selectResultsTab("meta", { persist: false });
    workflow.selectAggregateTab("saved", { persist: false });
    historyStore.savedRunCount = 1;

    analysisStore.metaSavedOptions = [
      { value: "meta-a", label: "meta-a", disabled: false },
    ];
    analysisStore.metaSavedSelected = "meta-a";
    analysisStore.canLoadMetaSaved = true;
    analysisStore.canRefreshMetaSaved = true;
    analysisStore.canOpenMetaSavedReport = true;
    analysisStore.canRevealMetaSaved = true;
    analysisStore.canDeleteMetaSaved = true;

    analysisStore.stage4SavedOptions = [
      { value: "stage5-a", label: "stage5-a", disabled: false },
    ];
    analysisStore.stage4SavedSelected = "stage5-a";
    analysisStore.canLoadStage4Saved = true;
    analysisStore.canRefreshStage4Saved = true;
    analysisStore.canRevealStage4Saved = true;
    analysisStore.canDeleteStage4Saved = true;

    analysisStore.policyEvalSavedOptions = [
      { value: "policy-a", label: "policy-a", disabled: false },
    ];
    analysisStore.policyEvalSavedSelected = "policy-a";
    analysisStore.canLoadPolicyEvalSaved = true;
    analysisStore.canRefreshPolicyEvalSaved = true;
    analysisStore.canRevealPolicyEvalSaved = true;
    analysisStore.canDeletePolicyEvalSaved = true;

    const openSavedReportSpy = vi
      .spyOn(analysisStore, "openSavedReport")
      .mockImplementation(() => {});

    mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const familyButtons = Array.from(
      document.querySelectorAll<HTMLButtonElement>(
        "[data-saved-artifact-family]",
      ),
    );
    expect(
      familyButtons.map((button) => String(button.textContent || "").trim()),
    ).toEqual(["Saved Meta", "Saved Stage 5", "Saved Policy Eval"]);

    const menuTriggers = Array.from(
      document.querySelectorAll<HTMLButtonElement>(
        "button.contextActionMenuTrigger",
      ),
    );
    expect(menuTriggers).toHaveLength(1);

    const loadButtons = Array.from(
      document.querySelectorAll<HTMLButtonElement>("button"),
    ).filter((button) => String(button.textContent || "").trim() === "Load");
    expect(loadButtons).toHaveLength(1);
    expect(
      document.querySelector("button.buttonWithIcon img.actionIcon"),
    ).not.toBeNull();

    menuTriggers[0]?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await flushPanel();

    const reportItem = Array.from(
      document.querySelectorAll<HTMLElement>('[role="menuitem"]'),
    ).find((item) => String(item.textContent || "").trim() === "Open report");
    expect(reportItem).toBeTruthy();
    expect(document.querySelector("img.contextActionMenuIcon")).not.toBeNull();

    reportItem?.click();
    await flushPanel();

    expect(openSavedReportSpy).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(menuTriggers[0]);

    familyButtons[1]?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await flushPanel();

    const stage5MenuTrigger = document.querySelector<HTMLButtonElement>(
      "button.contextActionMenuTrigger",
    );
    expect(stage5MenuTrigger).not.toBeNull();
    stage5MenuTrigger?.dispatchEvent(
      new MouseEvent("click", { bubbles: true }),
    );
    await flushPanel();

    const stage5MenuText = Array.from(
      document.querySelectorAll<HTMLElement>('[role="menuitem"]'),
    ).map((item) => String(item.textContent || "").trim());
    expect(stage5MenuText).not.toContain("Open report");

    familyButtons[2]?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await flushPanel();

    expect(document.body.textContent || "").toContain(
      "Saved Policy Evaluation",
    );
  });

  it("surfaces saved aggregate family provenance and active selection context", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const historyStore = useHistoryStore(pinia);
    const analysisStore = useAnalysisStore(pinia);

    workflow.selectResultsTab("meta", { persist: false });
    workflow.selectAggregateTab("saved", { persist: false });
    historyStore.savedRunCount = 1;

    analysisStore.metaSavedOptions = [
      {
        value: "meta-a",
        label: "2026-02-26 | CompositeMo | 1m | groups=3 | arts=2",
        disabled: false,
      },
    ];
    analysisStore.metaSavedSelected = "meta-a";
    analysisStore.stage4SavedOptions = [
      {
        value: "stage5-a",
        label: "2026-02-26 | CompositeMo | 1m | groups=2 | arts=1",
        disabled: false,
      },
    ];
    analysisStore.stage4SavedSelected = "stage5-a";
    analysisStore.policyEvalSavedOptions = [
      {
        value: "policy-a",
        label: "2026-02-26 | CompositeMo | 1m | groups=2 | arts=1",
        disabled: false,
      },
    ];
    analysisStore.policyEvalSavedSelected = "policy-a";

    mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    expect(document.body.textContent || "").toContain(
      "Saved Meta keeps aggregate selection baselines. Saved Stage 5 keeps ML confidence reranks. Saved Policy Eval keeps policy sweep outputs.",
    );
    expect(document.body.textContent || "").toContain("Family role");
    expect(document.body.textContent || "").toContain(
      "Aggregate selection baselines from saved meta artifacts.",
    );
    expect(document.body.textContent || "").toContain(
      "2026-02-26 | CompositeMo | 1m | groups=3 | arts=2",
    );

    const familyButtons = Array.from(
      document.querySelectorAll<HTMLButtonElement>(
        "[data-saved-artifact-family]",
      ),
    );
    familyButtons[1]?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await flushPanel();
    expect(document.body.textContent || "").toContain(
      "ML confidence reranks and Stage 5 decision buckets.",
    );
    expect(document.body.textContent || "").toContain(
      "2026-02-26 | CompositeMo | 1m | groups=2 | arts=1",
    );

    familyButtons[2]?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await flushPanel();
    expect(document.body.textContent || "").toContain(
      "Policy sweep evaluations over saved Stage 5 or meta-derived candidates.",
    );
    expect(document.body.textContent || "").toContain(
      "2026-02-26 | CompositeMo | 1m | groups=2 | arts=1",
    );
  });

  it("shows compare provenance and keeps an empty compare summary explicit", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const historyStore = useHistoryStore(pinia);
    const analysisStore = useAnalysisStore(pinia);

    workflow.selectResultsTab("meta", { persist: false });
    workflow.selectAggregateTab("compare", { persist: false });
    historyStore.savedRunCount = 1;
    analysisStore.metaMeta = "groups=5 · artifacts=3/4 · runtime=120ms";
    analysisStore.metaOutputPreview = '{"groups":5}';
    analysisStore.metaTableRows = [
      {
        cells: [
          { text: "1", className: "", title: "" },
          { text: "3", className: "", title: "" },
        ],
      },
    ];
    analysisStore.metaCompareOptions = [
      { value: "", label: "Compare against saved meta...", disabled: false },
      {
        value: "C:/runs/baseline_meta.json",
        label: "2026-02-26T10:00:00Z | CompositeMo | 1m | groups=4 | arts=2",
        disabled: false,
      },
    ];
    analysisStore.metaCompareSelected = "C:/runs/baseline_meta.json";
    analysisStore.metaCompareSummary = "-";

    mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const body = document.body.textContent || "";
    expect(body).toContain("Saved Meta artifact");
    expect(body).toContain("Active Meta Selection output");
    expect(body).toContain(
      "2026-02-26T10:00:00Z | CompositeMo | 1m | groups=4 | arts=2",
    );
    expect(body).toContain("baseline_meta.json");
    expect(document.querySelector("pre.preview")?.textContent?.trim()).toBe(
      "-",
    );
  });

  it("uses inline confirmation before deleting saved aggregate artifacts", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const historyStore = useHistoryStore(pinia);
    const analysisStore = useAnalysisStore(pinia);

    workflow.selectResultsTab("meta", { persist: false });
    workflow.selectAggregateTab("saved", { persist: false });
    historyStore.savedRunCount = 1;

    analysisStore.metaSavedOptions = [
      { value: "meta-a", label: "meta-a", disabled: false },
    ];
    analysisStore.metaSavedSelected = "meta-a";
    analysisStore.canDeleteMetaSaved = true;

    analysisStore.stage4SavedOptions = [
      { value: "stage5-a", label: "stage5-a", disabled: false },
    ];
    analysisStore.stage4SavedSelected = "stage5-a";
    analysisStore.canDeleteStage4Saved = true;

    analysisStore.policyEvalSavedOptions = [
      { value: "policy-a", label: "policy-a", disabled: false },
    ];
    analysisStore.policyEvalSavedSelected = "policy-a";
    analysisStore.canDeletePolicyEvalSaved = true;

    const deleteSavedSpy = vi
      .spyOn(analysisStore, "deleteSaved")
      .mockImplementation(() => {});
    const deleteStage5Spy = vi
      .spyOn(analysisStore, "deleteStage5Saved")
      .mockImplementation(() => {});
    const deletePolicySpy = vi
      .spyOn(analysisStore, "deletePolicyEvalSaved")
      .mockImplementation(() => {});

    mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const openDeleteDialog = async (): Promise<void> => {
      const trigger = document.querySelector<HTMLButtonElement>(
        "button.contextActionMenuTrigger",
      );
      expect(trigger).not.toBeNull();
      trigger?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await flushPanel();
      const deleteItem = Array.from(
        document.querySelectorAll<HTMLElement>('[role="menuitem"]'),
      ).find((item) => String(item.textContent || "").trim() === "Delete");
      expect(deleteItem).toBeTruthy();
      deleteItem?.click();
      await flushPanel();
    };

    await openDeleteDialog();
    expect(document.body.textContent || "").toContain(
      "Delete saved meta artifact",
    );
    expect(document.body.textContent || "").toContain("meta-a");
    expect(deleteSavedSpy).not.toHaveBeenCalled();

    let confirmButton = Array.from(
      document.querySelectorAll<HTMLButtonElement>("button"),
    ).find(
      (button) => String(button.textContent || "").trim() === "Delete artifact",
    );
    expect(confirmButton).toBeDefined();
    confirmButton?.click();
    await flushPanel();
    expect(deleteSavedSpy).toHaveBeenCalledTimes(1);

    const familyButtons = Array.from(
      document.querySelectorAll<HTMLButtonElement>(
        "[data-saved-artifact-family]",
      ),
    );
    familyButtons[1]?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await flushPanel();

    await openDeleteDialog();
    expect(document.body.textContent || "").toContain(
      "Delete saved Stage 5 artifact",
    );
    expect(document.body.textContent || "").toContain("stage5-a");
    confirmButton = Array.from(
      document.querySelectorAll<HTMLButtonElement>("button"),
    ).find(
      (button) => String(button.textContent || "").trim() === "Delete artifact",
    );
    confirmButton?.click();
    await flushPanel();
    expect(deleteStage5Spy).toHaveBeenCalledTimes(1);

    familyButtons[2]?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await flushPanel();

    await openDeleteDialog();
    expect(document.body.textContent || "").toContain(
      "Delete saved policy evaluation",
    );
    expect(document.body.textContent || "").toContain("policy-a");
    confirmButton = Array.from(
      document.querySelectorAll<HTMLButtonElement>("button"),
    ).find(
      (button) => String(button.textContent || "").trim() === "Delete artifact",
    );
    confirmButton?.click();
    await flushPanel();
    expect(deletePolicySpy).toHaveBeenCalledTimes(1);
  });

  it("shows compare provenance and selected saved-meta baseline context", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const historyStore = useHistoryStore(pinia);
    const analysisStore = useAnalysisStore(pinia);

    workflow.selectResultsTab("meta", { persist: false });
    workflow.selectAggregateTab("compare", { persist: false });
    historyStore.savedRunCount = 1;

    analysisStore.metaCompareOptions = [
      { value: "", label: "Compare against...", disabled: false },
      {
        value: "C:/runs/vectorgrid_meta_baseline.json",
        label: "2026-04-09T10:00:00Z | CompositeMo | 1m | groups=4",
        disabled: false,
      },
    ];
    analysisStore.metaCompareSelected = "C:/runs/vectorgrid_meta_baseline.json";
    analysisStore.metaCompareSummary =
      "Saved Meta baseline selected: C:/runs/vectorgrid_meta_baseline.json";

    mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const text = document.body.textContent || "";
    expect(text).toContain("Compare source");
    expect(text).toContain("Saved Meta artifact");
    expect(text).toContain("Current candidate");
    expect(text).toContain("Active Meta Selection output");
    expect(text).toContain("Selected baseline");
    expect(text).toContain(
      "2026-04-09T10:00:00Z | CompositeMo | 1m | groups=4",
    );
    expect(text).toContain("Baseline file");
    expect(text).toContain("vectorgrid_meta_baseline.json");
    expect(text).toContain(
      "Current Meta Selection output stays on the working side",
    );
    expect(document.querySelector("pre.preview")?.textContent?.trim()).toBe(
      "Saved Meta baseline selected: C:/runs/vectorgrid_meta_baseline.json",
    );
  });

  it("keeps model status, Stage 5 output, and policy evaluation provenance distinct in the ML panel", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const historyStore = useHistoryStore(pinia);
    const analysisStore = useAnalysisStore(pinia);

    workflow.selectResultsTab("meta", { persist: false });
    workflow.selectAggregateTab("meta", { persist: false });
    historyStore.savedRunCount = 1;

    analysisStore.metaMlStatus =
      "scope=CompositeMo/1m state=UNHEALTHY meta_scan_error=scan failed";
    analysisStore.metaStage4Status =
      "Stage 5 complete: selected=1 scored=1/2 accept/review/reject=1/0/1 · file=vectorgrid_stage5.json";
    analysisStore.metaStage4OutputPreview =
      "source_meta=vectorgrid_meta.json model=meta_model.json state=healthy healthy=yes";
    analysisStore.metaPolicyEvalStatus =
      "Policy evaluation complete: evaluated=3/4 · best=stage5_blended · stage5_precision=100.0% · file=vectorgrid_ml_policy_eval.json";
    analysisStore.metaPolicyEvalOutputPreview =
      "best_policy=stage5_blended skipped(no_history/train_fail/unhealthy/no_labels)=1/0/0/0";

    mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const text = document.body.textContent || "";
    expect(text).toContain("Model status");
    expect(text).toContain("Stage 5 output");
    expect(text).toContain("Policy evaluation");
    expect(text).toContain("ML model health needs review");
    expect(text).toContain(
      "scope=CompositeMo/1m state=UNHEALTHY meta_scan_error=scan failed",
    );
    expect(text).toContain(
      "Saved Stage 5 and Saved Policy Eval artifacts stay in the Saved Aggregate library.",
    );
    expect(text).toContain("Current live Stage 5 output");
    expect(text).toContain("Current live policy evaluation");
    expect(text).toContain("vectorgrid_stage5.json");
    expect(text).toContain("vectorgrid_ml_policy_eval.json");
  });

  it("shows batch operating-mode summary without hiding recovery controls", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const historyStore = useHistoryStore(pinia);
    const analysisStore = useAnalysisStore(pinia);

    workflow.selectResultsTab("meta", { persist: false });
    workflow.selectAggregateTab("batch", { persist: false });
    historyStore.savedRunCount = 1;

    analysisStore.batchExecutionMode = "optimize_only";
    analysisStore.batchPreloadPolicy = "host_plus_doublema_vram";
    analysisStore.batchAutoMeta = false;
    analysisStore.batchAutoStage5 = false;
    analysisStore.batchHoldoutPct = "15";
    analysisStore.batchHoldoutTag = "holdout-a";
    analysisStore.batchTags = "alpha, beta alpha";
    analysisStore.batchSkipDone = true;
    analysisStore.batchSelectedCount = 2;
    analysisStore.batchTotalCount = 5;
    analysisStore.batchMeta = "selected 2/5 • mode=optimize_only";
    analysisStore.canRunBatch = true;
    analysisStore.canBatchRetryFailed = true;
    analysisStore.canBatchResumeIncomplete = true;
    analysisStore.canBatchRevealSummary = true;
    analysisStore.batchDatasets = [
      {
        path: "C:/data/a.csv",
        label: "AAPL",
        meta: "stocks | 1m",
        checked: true,
        disabled: false,
      },
      {
        path: "C:/data/b.csv",
        label: "MSFT",
        meta: "stocks | 1m",
        checked: true,
        disabled: false,
      },
    ];

    mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const text = document.body.textContent || "";
    expect(text).toContain("Execution mode");
    expect(text).toContain("Optimize only");
    expect(text).toContain("Preload policy");
    expect(text).toContain("Host + DoubleMa VRAM staging (experimental)");
    expect(text).toContain("Automation");
    expect(text).toContain("Manual follow-up");
    expect(text).toContain("Datasets");
    expect(text).toContain("2/5 selected");
    expect(text).toContain("15% holdout • tag holdout-a • skip completed on");
    expect(text).toContain(
      "tags alpha, beta • same-tf selection unless Data overrides • meta and Stage 5 disabled",
    );
    expect(text).toContain("saved summary available");
    expect(text).toContain("required for retry/resume");
    expect(text).toContain("retry restores the saved dataset set");
    expect(text).toContain("resume needs embedded request");
    expect(text).toContain("recovery forces skip completed");
    expect(text).toContain("Retry failed");
    expect(text).toContain("Resume incomplete");
    expect(text).toContain("Reveal summary");
  });

  it("shows ML confidence contract summaries and keeps apply paths distinct", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const historyStore = useHistoryStore(pinia);
    const analysisStore = useAnalysisStore(pinia);

    workflow.selectResultsTab("meta", { persist: false });
    workflow.selectAggregateTab("meta", { persist: false });
    historyStore.savedRunCount = 1;

    analysisStore.metaStrategyKind = "CompositeMo";
    analysisStore.metaTimeframe = "1m";
    analysisStore.metaMlAutopilot = true;
    analysisStore.metaMlStatus =
      "Model: ml_confidence_CompositeMo_1m.json · state=warning · model_kind=logistic · target=holdout_dataset_pass · trained_rows=320 · label: pass>=0.60, p10>=0.100, datasets>=5 · available: meta=10, rows=300 · stale · retrain recommended";
    analysisStore.metaStage4Status =
      "Stage 5 complete: selected=1 scored=1/2 accept/review/reject=1/0/1 · file=vectorgrid_stage5.json";
    analysisStore.metaPolicyEvalStatus =
      "Policy evaluation complete: evaluated=3/4 · best=stage5_blended · stage5_precision=100.0% · file=vectorgrid_ml_policy_eval.json";

    mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const text = document.body.textContent || "";
    expect(text).toContain(
      "CompositeMo / 1m • model ml_confidence_CompositeMo_1m.json • latest meta result scope first",
    );
    expect(text).toContain(
      "available meta 10 • rows 300 • trained 320 • target holdout_dataset_pass",
    );
    expect(text).toContain("pass>=0.60, p10>=0.100, datasets>=5");
    expect(text).toContain(
      "family Auto (backend owned) • resolved kind logistic • no shipped UI family picker",
    );
    expect(text).toContain(
      "state warning • stale • retrain recommended • modern or missing",
    );
    expect(text).toContain(
      "autopilot on • Apply mutates current meta only • Stage 5 saves rerank artifact • Policy Eval saves offline comparison",
    );
    expect(text).toContain(
      "CompositeMo / 1m • source latest Meta artifact first, then Saved Meta selection • Stage 5 auto-build reuses current controls",
    );
    expect(text).toContain(
      "rank uses meta/ml/unc weights • min_ml filters first • fallback only if shortlist empties and toggle stays on • decision buckets assign after ranking",
    );
    expect(text).toContain("run_meta_stage5_ml_confidence via Run Stage 5");
    expect(text).toContain(
      "Run Stage 5 returns the durable saved replay artifact used by load/reveal/delete",
    );
    expect(text).toContain(
      "saved Stage 5 JSON artifact vectorgrid_stage5.json",
    );
    expect(text).toContain(
      "offline scoped history over saved Meta artifacts • request reuses current Stage 5 thresholds and weights • compares meta_only / ml_only / stage5_blended • JSON artifact replay only",
    );
    expect(text).toContain(
      "Saved Stage 5 UI is backed by durable replay state • load/reveal/delete operate on saved JSON artifacts • current live Stage 5 preview is reused on load",
    );
    expect(text).toContain(
      "metaMlStatus is formatted model status • statusLine is action feedback • buttons gate on cached backend status and scope completeness • autopilot reacts after meta runs",
    );
    expect(text).toContain(
      "Auto target prefers per-dataset holdout rows when available and falls back to aggregate holdout gates.",
    );
    expect(text).toContain(
      "Only Stage 5 and Policy Eval persist durable artifacts; Apply to current meta is intentionally in-memory only.",
    );
    expect(text).toContain(
      "Stage 5 excludes the active source meta artifact from auto-train history, reranks first, then assigns accept/review/reject buckets.",
    );
    expect(text).toContain(
      "If no group clears min ML confidence and fallback stays on, the backend falls back to pure meta-score ranking with a warning.",
    );
    expect(text).toContain(
      "Policy evaluation is offline analysis over saved Meta history, not in-memory Apply and not the current live Stage 5 rerank.",
    );
    expect(text).toContain(
      "Saved Stage 5 uses the same durable replay artifact surface as load/reveal/delete; the UI no longer surfaces Stage 4 path alias wording.",
    );
  });

  it("shows Stage 3 request, grouping, scoring, and evidence contract summaries", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const historyStore = useHistoryStore(pinia);
    const analysisStore = useAnalysisStore(pinia);

    workflow.selectResultsTab("meta", { persist: false });
    workflow.selectAggregateTab("meta", { persist: false });
    historyStore.savedRunCount = 1;

    analysisStore.metaStrategyKind = "CompositeMo";
    analysisStore.metaTimeframe = "1m";
    analysisStore.metaRequiredTag = "holdout";
    analysisStore.metaMinSamples = "30";
    analysisStore.metaTopK = "25";
    analysisStore.metaHoldoutTag = "holdout-a";
    analysisStore.metaEvalHoldout = true;
    analysisStore.metaSaveHoldoutRows = true;
    analysisStore.metaHoldoutMode = "heavy";
    analysisStore.metaHoldoutReps = "2";
    analysisStore.metaHoldoutEvalTopN = "10";
    analysisStore.metaHoldoutEvalRankBy = "auto";
    analysisStore.metaHoldoutExploreN = "3";
    analysisStore.metaGroupingMode = "clustered";
    analysisStore.metaClusterIntBucket = "5";
    analysisStore.metaClusterFloatBucket = "0.25";
    analysisStore.metaHoldoutNote = "Holdout supported for CompositeMo.";

    mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const text = document.body.textContent || "";
    expect(text).toContain("Scope");
    expect(text).toContain(
      "CompositeMo • tf 1m • tag holdout • min 30 • top 25",
    );
    expect(text).toContain("Holdout");
    expect(text).toContain("holdout heavy • tag holdout-a • save rows on");
    expect(text).toContain("Grouping");
    expect(text).toContain(
      "clustered • int 5 • float 0.25 • reps 2 (clamped 1-3)",
    );
    expect(text).toContain("Shortlist");
    expect(text).toContain(
      "strict shortlist on • fallback off • rank auto • topN 10 • explore 3",
    );
    expect(text).toContain("Stage 3 meta_score uses backend MetaScoreWeights.");
    expect(text).toContain("Evidence flags remain warnings, not hard gates.");
    expect(text).toContain(
      "Heavy mode emits nested validation config (walk-forward, perturbation, bootstrap, iterative engine).",
    );
    expect(text).toContain(
      "Backend still owns holdout rank fallback and warning text.",
    );
  });

  it("keeps saved aggregate family identity explicit when switching families", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const historyStore = useHistoryStore(pinia);
    const analysisStore = useAnalysisStore(pinia);

    workflow.selectResultsTab("meta", { persist: false });
    workflow.selectAggregateTab("saved", { persist: false });
    historyStore.savedRunCount = 1;

    analysisStore.metaSavedOptions = [
      { value: "meta-a", label: "Meta baseline A", disabled: false },
    ];
    analysisStore.metaSavedSelected = "meta-a";
    analysisStore.stage4SavedOptions = [
      { value: "stage5-a", label: "Stage 5 shortlist A", disabled: false },
    ];
    analysisStore.stage4SavedSelected = "stage5-a";
    analysisStore.policyEvalSavedOptions = [
      { value: "policy-a", label: "Policy eval A", disabled: false },
    ];
    analysisStore.policyEvalSavedSelected = "policy-a";

    mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    let text = document.body.textContent || "";
    expect(text).toContain("Family role");
    expect(text).toContain(
      "Aggregate selection baselines from saved meta artifacts.",
    );
    expect(text).toContain("Active selection");
    expect(text).toContain("Meta baseline A");

    const familyButtons = Array.from(
      document.querySelectorAll<HTMLButtonElement>(
        "[data-saved-artifact-family]",
      ),
    );
    familyButtons[1]?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await flushPanel();

    text = document.body.textContent || "";
    expect(text).toContain(
      "ML confidence reranks and Stage 5 decision buckets.",
    );
    expect(text).toContain("Stage 5 shortlist A");

    familyButtons[2]?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await flushPanel();

    text = document.body.textContent || "";
    expect(text).toContain(
      "Policy sweep evaluations over saved Stage 5 or meta-derived candidates.",
    );
    expect(text).toContain("Policy eval A");
  });
});
