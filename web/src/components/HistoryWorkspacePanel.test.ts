// @vitest-environment jsdom
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { nextTick } from "vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
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
    vi.restoreAllMocks();
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
    history.savedRunDetailMetricSurface = "Core17 • 17 metrics";
    history.savedRunDetailMetricSurfaceDetail =
      "fields=pnl, trades, max_dd, mean, std, exposure +11 more";
    history.savedRunDetailAdmission = "scalar_union • kept 16/20";
    history.savedRunDetailAdmissionDetail =
      "src=top_rows • heads=Sharpe/MaxDrawdown";
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
    expect(bodyText).toContain("Admission");
    expect(bodyText).toContain("scalar_union • kept 16/20");
    expect(bodyText).toContain("Metric Surface");
    expect(bodyText).toContain("Core17 • 17 metrics");
    expect(bodyText).toContain("Metric Detail");
    expect(bodyText).toContain(
      "fields=pnl, trades, max_dd, mean, std, exposure +11 more",
    );
    expect(bodyText).toContain("Admission Detail");
    expect(bodyText).toContain("src=top_rows • heads=Sharpe/MaxDrawdown");
    expect(bodyText).toContain("Top Candidates");
    expect(bodyText).toContain(
      "best_sweep | robust=1.44 | psr=0.72 | dsr=0.68",
    );
  });

  it("uses inline confirmation before clearing session history or deleting a saved run", async () => {
    const history = useHistoryStore();
    const workflow = useWorkflowStore();
    vi.spyOn(history, "refreshSessionHistory").mockImplementation(() => {});

    workflow.workspace = "library";
    history.savedRunCount = 1;
    history.canClearHistory = true;
    history.canDeleteSavedRun = true;
    history.savedSelectedValue = "C:/runs/demo.json";
    history.savedOptions = [
      { value: "C:/runs/demo.json", label: "demo.json", disabled: false },
    ];
    history.savedRunDetailTitle = "demo";

    const clearSpy = vi
      .spyOn(history, "clearSession")
      .mockImplementation(() => {});
    const deleteSpy = vi
      .spyOn(history, "deleteSavedRun")
      .mockImplementation(() => {});

    mount(HistoryWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    });

    await nextTick();
    await nextTick();

    const buttons = Array.from(document.querySelectorAll("button"));
    const clearButton = buttons.find(
      (button) => button.textContent?.trim() === "Clear",
    ) as HTMLButtonElement | undefined;
    const deleteButton = buttons.find(
      (button) => button.textContent?.trim() === "Delete",
    ) as HTMLButtonElement | undefined;

    expect(clearButton).toBeDefined();
    expect(deleteButton).toBeDefined();

    clearButton?.click();
    await nextTick();
    await nextTick();

    expect(document.body.textContent || "").toContain("Clear session history");
    expect(document.querySelector('[role="alertdialog"]')).not.toBeNull();
    expect(clearSpy).not.toHaveBeenCalled();

    const confirmClearButton = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Confirm clear",
    ) as HTMLButtonElement | undefined;

    expect(confirmClearButton).toBeDefined();
    confirmClearButton?.click();
    await nextTick();
    expect(clearSpy).toHaveBeenCalledTimes(1);

    deleteButton?.click();
    await nextTick();
    await nextTick();

    expect(document.body.textContent || "").toContain("Delete saved run");
    expect(document.querySelector('[role="alertdialog"]')).not.toBeNull();
    expect(document.body.textContent || "").toContain("C:/runs/demo.json");
    expect(deleteSpy).not.toHaveBeenCalled();

    const confirmDeleteButton = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Delete artifact",
    ) as HTMLButtonElement | undefined;

    expect(confirmDeleteButton).toBeDefined();
    confirmDeleteButton?.click();
    await nextTick();
    expect(deleteSpy).toHaveBeenCalledWith("C:/runs/demo.json");
  });

  it("supports arrow-key navigation across session and saved run lists", async () => {
    const history = useHistoryStore();
    const workflow = useWorkflowStore();
    vi.spyOn(history, "refreshSessionHistory").mockImplementation(() => {});

    workflow.workspace = "library";
    history.savedRunCount = 2;
    history.sessionRunsRaw = [
      { ts: "2026-03-20T10:00:00Z", backend: "cpu", pairs: 1 },
      { ts: "2026-03-20T11:00:00Z", backend: "gpu", pairs: 2 },
    ];
    history.savedRunsVisible = [
      {
        ts: "2026-03-21T10:00:00Z",
        path: "C:/runs/a.json",
        bytes: 1024,
        strategyKind: "CompositeMo",
        hasValidation: true,
        reportXlsxPath: "C:/runs/a.xlsx",
        csvPath: "C:/data/a.csv",
        tags: ["a"],
      },
      {
        ts: "2026-03-22T10:00:00Z",
        path: "C:/runs/b.json",
        bytes: 2048,
        strategyKind: "CompositeMo",
        hasValidation: false,
        reportXlsxPath: "",
        csvPath: "C:/data/b.csv",
        tags: ["b"],
      },
    ];

    mount(HistoryWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    });

    await nextTick();
    await nextTick();

    const sessionButtons = Array.from(
      document.querySelectorAll<HTMLButtonElement>(
        'ul[aria-label="Session history runs"] button.runListButton',
      ),
    );
    expect(sessionButtons).toHaveLength(2);

    sessionButtons[0]?.focus();
    sessionButtons[0]?.dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }),
    );
    expect(document.activeElement).toBe(sessionButtons[1]);

    const savedButtons = Array.from(
      document.querySelectorAll<HTMLButtonElement>(
        'ul[aria-label="Saved run artifacts"] button.runListButton',
      ),
    );
    expect(savedButtons).toHaveLength(2);

    savedButtons[1]?.focus();
    savedButtons[1]?.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Home", bubbles: true }),
    );
    expect(document.activeElement).toBe(savedButtons[0]);
  });

  it("keeps the options follow-up panel mounted from History detail for supported parent runs", async () => {
    const history = useHistoryStore();
    const workflow = useWorkflowStore();
    vi.spyOn(history, "refreshSessionHistory").mockImplementation(() => {});

    workflow.workspace = "library";
    history.savedRunCount = 1;
    history.savedSelectedValue = "C:/runs/double_ma.json";
    history.savedRunDetailTitle = "double_ma run";
    history.savedRunDetailMeta = "artifact: double_ma.json";
    history.savedRunDetailStrategy = "DoubleMa";
    history.savedRunDetailDataset = "spy.csv";
    history.savedRunDetailTimeframe = "1d";
    history.savedRunDetailValidation = "Available";
    history.savedRunDetailMetricSurface = "Core17";
    history.savedRunDetailAdmission = "scalar_union";
    history.savedRunDetailReport = "Not exported";
    history.savedRunDetailSize = "12 KiB";
    history.savedRunDetailTags = "baseline";
    history.savedRunDetailParams = "strategy: DoubleMa";

    mount(HistoryWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    });

    await nextTick();
    await nextTick();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain("Options Expression");
    expect(bodyText).toContain("Follow-up workflow");
    expect(bodyText).toContain("This is an options follow-up surface.");
    expect(
      document.querySelector('[aria-label="Options expression follow-up"]'),
    ).not.toBeNull();
  });

  it("documents that saved-run search is local while command search stays global", async () => {
    const history = useHistoryStore();
    const workflow = useWorkflowStore();
    vi.spyOn(history, "refreshSessionHistory").mockImplementation(() => {});

    workflow.workspace = "library";
    history.savedRunCount = 1;

    mount(HistoryWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    });

    await nextTick();
    await nextTick();

    const text = document.body.textContent || "";
    expect(text).toContain("Search filters the saved-run library only.");
    expect(text).toContain("Use Ctrl+K for global commands.");
  });

  it("shows the no-match library state without implying a global search miss", async () => {
    const history = useHistoryStore();
    const workflow = useWorkflowStore();
    vi.spyOn(history, "refreshSessionHistory").mockImplementation(() => {});

    workflow.workspace = "library";
    history.savedRunCount = 2;
    history.savedRunsMeta = "saved runs: 0/2";
    history.savedRunsSearch = "zzz";
    history.savedRunsVisible = [];

    mount(HistoryWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    });

    await nextTick();
    await nextTick();

    const text = document.body.textContent || "";
    expect(text).toContain("No saved runs match the current filters.");
    expect(text).toContain("0 matches for");
    expect(text).toContain("Use Ctrl+K for global commands.");
  });
});
