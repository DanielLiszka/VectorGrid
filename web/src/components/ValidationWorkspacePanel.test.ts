import { createPinia, setActivePinia } from "pinia";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it } from "vitest";
import ValidationWorkspacePanel from "./ValidationWorkspacePanel.vue";
import { useAnalysisStore } from "../stores/analysisStore";
import { useDataStore } from "../stores/dataStore";
import { bindRuntimePinia } from "../stores/runtimePinia";
import { useValidationStore } from "../stores/validationStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";
import { useWorkflowStore } from "../stores/workflowStore";
import { VALIDATION_RESULTS_TABLE_STATE_KEY } from "../utils/core/storageKeys";

function installValidationPanelMountShell(): void {
  document.body.innerHTML = `
    <section id="shellResultsColumn" class="resultsColumn">
      <div id="results_panel_validate" class="panelHost" data-vue-panel="validate">
        <div id="validationVueShellMount"></div>
      </div>
    </section>
  `;
}

async function flushPanel(): Promise<void> {
  await nextTick();
  await nextTick();
}

describe("ValidationWorkspacePanel", () => {
  beforeEach(() => {
    installValidationPanelMountShell();
    localStorage.clear();
  });

  it("shows empty-state guidance before data load and before optimization runs exist", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const dataStore = useDataStore(pinia);
    workflow.selectResultsTab("validate", { persist: false });

    const first = mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    expect(document.body.textContent || "").toContain(
      "Load data before running validation.",
    );
    expect(document.body.textContent || "").toContain(
      "Validation works on shortlisted optimization candidates, so the first step is loading a dataset.",
    );

    first.unmount();

    dataStore.dataId = "dataset_validation_ready";
    dataStore.datasetLabel = "AAPL 1m";

    const second = mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    expect(document.body.textContent || "").toContain(
      "Run an optimization before validation.",
    );
    expect(document.body.textContent || "").toContain(
      "Your dataset is ready. Generate candidates with an optimization run, then return here to execute robustness checks.",
    );

    second.unmount();
  });

  it("keeps the primary validation action inline in the panel", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    workflow.selectResultsTab("validate", { persist: false });

    const wrapper = mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    expect(document.body.textContent || "").toContain("Run validation");
    expect(document.querySelector("#vueValAdmissionPolicy")).not.toBeNull();
    expect(document.body.textContent || "").toContain("Modules and automation");
    expect(document.querySelector("#vueValWfWindow")).not.toBeNull();
    expect(document.querySelector("#vueValCrossMode")).not.toBeNull();
    expect(document.body.textContent || "").toContain("Export shortlist CSV");
    expect(document.body.textContent || "").toContain("Export validation JSON");
    expect(document.body.textContent || "").toContain("Export report JSON");
    expect(document.body.textContent || "").toContain("Export report XLSX");
    expect(document.body.textContent || "").toContain("Open reports folder");
    const runButtons = Array.from(document.querySelectorAll("button")).filter(
      (button) => button.textContent?.trim() === "Run validation",
    );
    expect(runButtons).toHaveLength(1);

    wrapper.unmount();
  });

  it("shows expected execution guidance and disables actions that are not currently available", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const dataStore = useDataStore(pinia);
    const validationStore = useValidationStore(pinia);
    const artifactsStore = useWorkflowArtifactsStore(pinia);

    workflow.selectResultsTab("validate", { persist: false });
    dataStore.dataId = "dataset_validation_preview";
    dataStore.csvPath = "C:/data/aapl.csv";
    dataStore.dataTimeframe = "1m";
    validationStore.candidates = "2";
    validationStore.admissionPolicy = "scalar_union";
    validationStore.canExportValidationCsv = false;
    validationStore.canExportValidationJson = false;
    validationStore.canExportReportJson = false;
    validationStore.canExportReportXlsx = false;
    validationStore.canOpenReportsDir = false;
    validationStore.canCrossRefresh = false;
    validationStore.canCrossSelectAll = false;
    validationStore.canCrossSelectNone = false;
    validationStore.canCrossSelectMatch = false;
    validationStore.crossDatasets = [
      {
        path: "C:/data/msft.csv",
        label: "MSFT",
        meta: "stocks (adj) • tf=1m",
        checked: false,
        disabled: false,
      },
    ];
    validationStore.crossSelectedCount = 0;
    validationStore.crossTotalCount = 1;
    validationStore.crossMeta = "selected 0/1";
    artifactsStore.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-09T12:00:00Z",
        csv_path: "C:/data/aapl.csv",
        dataset_meta: {
          csv_path: "C:/data/aapl.csv",
          timeframe: "1m",
          label: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-validation-preview-disabled",
          objective: "Sharpe",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
              metrics: {
                pnl: 0.15,
                sharpe: 1.8,
                calmar: 0.7,
                omega: 1.02,
                max_dd: 0.32,
              },
            },
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 8,
                slow_len: 26,
              },
              metrics: {
                pnl: 0.1,
                sharpe: 0.9,
                calmar: 1.6,
                omega: 1.35,
                max_dd: 0.06,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const wrapper = mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain("Validation is ready to run.");
    expect(bodyText).toContain("Expected Route");
    expect(bodyText).toContain("Expected Cross");
    expect(bodyText).toContain("Expected Modules");
    expect(bodyText).toContain("Expected Categories");
    expect(bodyText).toContain("Expected Skips");
    expect(bodyText).toContain("Expected Notes");
    expect(bodyText).toContain("Admission Compare");
    expect(bodyText).toContain("Selected 0/1");

    const buttonByText = (label: string) =>
      Array.from(document.querySelectorAll<HTMLButtonElement>("button")).find(
        (button) => String(button.textContent || "").trim() === label,
      );

    expect(buttonByText("Export shortlist CSV")?.disabled).toBe(true);
    expect(buttonByText("Export validation JSON")?.disabled).toBe(true);
    expect(buttonByText("Export report JSON")?.disabled).toBe(true);
    expect(buttonByText("Export report XLSX")?.disabled).toBe(true);
    expect(buttonByText("Open reports folder")?.disabled).toBe(true);
    expect(buttonByText("Refresh list")?.disabled).toBe(true);
    expect(buttonByText("All")?.disabled).toBe(true);
    expect(buttonByText("None")?.disabled).toBe(true);
    expect(buttonByText("Match current")?.disabled).toBe(true);
    expect(document.querySelector("table.validationTableVue")).toBeNull();

    wrapper.unmount();
  });

  it("shows a legacy-vs-current admission preview when policy differs from legacy", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const dataStore = useDataStore(pinia);
    const validationStore = useValidationStore(pinia);
    const artifactsStore = useWorkflowArtifactsStore(pinia);

    workflow.selectResultsTab("validate", { persist: false });
    dataStore.csvPath = "C:/data/aapl.csv";
    dataStore.dataTimeframe = "1m";
    validationStore.candidates = "2";
    validationStore.admissionPolicy = "scalar_union";
    artifactsStore.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-04T10:00:00Z",
        csv_path: "C:/data/aapl.csv",
        dataset_meta: {
          csv_path: "C:/data/aapl.csv",
          timeframe: "1m",
          label: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-validation-preview",
          objective: "Sharpe",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
              metrics: {
                pnl: 0.15,
                sharpe: 1.8,
                calmar: 0.7,
                omega: 1.02,
                max_dd: 0.32,
              },
            },
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 12,
                slow_len: 48,
              },
              metrics: {
                pnl: 0.13,
                sharpe: 1.4,
                calmar: 0.85,
                omega: 1.08,
                max_dd: 0.24,
              },
            },
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 8,
                slow_len: 26,
              },
              metrics: {
                pnl: 0.1,
                sharpe: 0.9,
                calmar: 1.6,
                omega: 1.35,
                max_dd: 0.06,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });

    const wrapper = mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain("Admission Compare");
    expect(bodyText).toContain(
      "scalar_union vs legacy • overlap=1 • current-only=1 • legacy-only=1",
    );
    expect(bodyText).toContain("Admission Delta");
    expect(bodyText).toContain("heads=Sharpe/MaxDrawdown/Omega/Calmar");

    wrapper.unmount();
  });

  it("shows validation summaries, dense result rows, and JSON preview once validation results are ready", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const dataStore = useDataStore(pinia);
    const validationStore = useValidationStore(pinia);
    const artifactsStore = useWorkflowArtifactsStore(pinia);

    workflow.selectResultsTab("validate", { persist: false });
    dataStore.dataId = "dataset_validation_results";
    dataStore.datasetLabel = "AAPL 1m";
    artifactsStore.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-09T12:30:00Z",
        csv_path: "C:/data/aapl.csv",
        dataset_meta: {
          csv_path: "C:/data/aapl.csv",
          timeframe: "1m",
          label: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "data-validation-results",
          objective: "Sharpe",
          ma_source: "close",
          fast_ma_types: ["sma"],
          slow_ma_types: ["ema"],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: "sma",
                slow_ma_type: "ema",
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: null,
    });
    validationStore.tableVisible = true;
    validationStore.tableRowCount = 1;
    validationStore.validationAccelerationStatus =
      "GPU validation completed with CPU follow-up checks.";
    validationStore.validationRouteSummary = "Cpu • Auto->CpuOnly";
    validationStore.validationRouteDetailSummary =
      "GPU validation runtime not implemented yet; using CPU validation.";
    validationStore.validationCategorySummary =
      "CPU-only: base reruns + hard gates";
    validationStore.validationAuxCpuSummary =
      "perturb + trade bootstrap compare (iid + stationary, stat block=auto)";
    validationStore.validationBootstrapSummary =
      "iid + stationary closed-trade compare • stat block=auto";
    validationStore.validationRegimeSummary = "broad support";
    validationStore.validationFrictionSummary = "3/4 stress scenarios";
    validationStore.validationCrossSummary =
      "requested=preserved_finalists • ran 1/1";
    validationStore.validationSkippedSummary =
      "Cross reruns: preserved_finalists omit walk_forward + reopt WF + SPA + perturb + trade bootstrap";
    validationStore.validationWarningSummary =
      "cross_ticker_filtered_all_candidates";
    validationStore.validationOverfitSummary =
      "Overfit evidence needs review; approved; 58,300 trials; PBO 12.0%; PSR 0.910 · DSR 0.650.";
    validationStore.validationOverfitStatus = "attention";
    validationStore.validationOverfitItems = [
      {
        id: "verdict",
        label: "Run Verdict",
        value: "approved",
        detail: "hard gates 2/4 · approved 1/2 · high_pbo",
        tone: "warn",
      },
      {
        id: "pbo",
        label: "CSCV / PBO",
        value: "12.0%",
        detail: "max 20.0% · 100/128 combinations",
        tone: "good",
      },
      {
        id: "track_record",
        label: "Track Record",
        value: "25,000/20,000 bars",
        detail: "meets minimum evidence length",
        tone: "good",
      },
    ];
    validationStore.validationMeta =
      "backend=Cpu • route=Auto->CpuOnly • heads=legacy_topk • rec=single_candidate_supported";
    validationStore.validationReoptMeta = "reopt_wf: windows=3 • winners=2";
    validationStore.validationShortlistMeta = "shortlist=1 • approved=1";
    validationStore.validationOutputPreview =
      '{"backend_used":"Cpu","approved":true}';
    validationStore.validationColumns = [
      { label: "Gate", alignLeft: true },
      { label: "Cross", alignLeft: true },
      { label: "Sharpe", alignLeft: false },
    ];
    validationStore.validationRows = [
      {
        cells: [
          { text: "pass", className: "left mono", title: "pass" },
          { text: "cross", className: "left", title: "cross" },
          { text: "1.2400", className: "mono", title: "1.2400" },
        ],
      },
    ];

    const wrapper = mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain("Validation summary table");
    expect(bodyText).toContain(
      "GPU validation completed with CPU follow-up checks.",
    );
    expect(bodyText).toContain("Validation diagnostics");
    expect(bodyText).toContain("Cpu • Auto->CpuOnly");
    expect(bodyText).toContain(
      "GPU validation runtime not implemented yet; using CPU validation.",
    );
    expect(bodyText).toContain("requested=preserved_finalists • ran 1/1");
    expect(bodyText).toContain("cross_ticker_filtered_all_candidates");
    expect(bodyText).toContain("Overfit Evidence");
    expect(bodyText).toContain("Needs review");
    expect(bodyText).toContain(
      "Overfit evidence needs review; approved; 58,300 trials; PBO 12.0%; PSR 0.910 · DSR 0.650.",
    );
    expect(bodyText).toContain("hard gates 2/4 · approved 1/2 · high_pbo");
    expect(bodyText).toContain("25,000/20,000 bars");
    expect(
      document
        .querySelector('[data-qa-panel="validation.overfitEvidence"]')
        ?.getAttribute("data-qa-state"),
    ).toBe("attention");
    expect(
      document.querySelector('[data-qa-state="validation.overfitEvidence.pbo"]')
        ?.textContent,
    ).toContain("12.0%");
    expect(
      document
        .querySelector('[data-qa-state="validation.overfitEvidence.pbo"]')
        ?.getAttribute("title"),
    ).toContain("max 20.0%");
    expect(bodyText).toContain("shortlist=1 • approved=1");
    expect(bodyText).toContain("Validation JSON preview");
    expect(bodyText).toContain('{"backend_used":"Cpu","approved":true}');
    expect(
      Array.from(document.querySelectorAll("th")).map((cell) =>
        String(cell.textContent || "").trim(),
      ),
    ).toEqual(["Gate", "Cross", "Sharpe"]);
    expect(
      document.querySelector('[data-qa-panel="validation.rowDetail"]')
        ?.textContent,
    ).toContain("Row 1 of 1");
    expect(
      document.querySelector('[data-qa-state="validation.rowDetail.col_0"]')
        ?.textContent,
    ).toContain("pass");
    expect(
      document.querySelector('[data-qa-state="validation.rowDetail.col_2"]')
        ?.textContent,
    ).toContain("1.2400");
    wrapper.unmount();
  });

  it("persists visible validation result columns and restores defaults", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const dataStore = useDataStore(pinia);
    const analysisStore = useAnalysisStore(pinia);
    const validationStore = useValidationStore(pinia);
    workflow.selectResultsTab("validate", { persist: false });
    dataStore.dataId = "dataset_1";
    analysisStore.topRows = 1;
    validationStore.tableRowCount = 1;
    validationStore.tableVisible = true;
    validationStore.validationColumns = [
      { label: "Route", alignLeft: true },
      { label: "Warning", alignLeft: true },
      { label: "Score", alignLeft: false },
    ];
    validationStore.validationRows = [
      {
        cells: [
          { text: "GpuKernel", className: "left", title: "GpuKernel" },
          { text: "Stable", className: "left", title: "Stable" },
          { text: "0.92", className: "", title: "0.92" },
        ],
      },
    ];

    const wrapper = mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const scoreToggle = document.querySelector<HTMLInputElement>(
      "#validationColumnToggle_col_2",
    );
    expect(scoreToggle).not.toBeNull();
    scoreToggle!.checked = false;
    scoreToggle!.dispatchEvent(new Event("change", { bubbles: true }));
    await flushPanel();

    const headerText = Array.from(document.querySelectorAll("th")).map((cell) =>
      String(cell.textContent || "").trim(),
    );
    expect(headerText).toEqual(["Route", "Warning"]);
    expect(localStorage.getItem(VALIDATION_RESULTS_TABLE_STATE_KEY)).toContain(
      '"visibleColumnKeys":["col_0","col_1"]',
    );

    wrapper.unmount();

    mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const remountHeaderText = Array.from(document.querySelectorAll("th")).map(
      (cell) => String(cell.textContent || "").trim(),
    );
    expect(remountHeaderText).toEqual(["Route", "Warning"]);

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
    expect(restoredHeaderText).toEqual(["Route", "Warning", "Score"]);
    expect(localStorage.getItem(VALIDATION_RESULTS_TABLE_STATE_KEY)).toBeNull();
  });

  it("updates validation header sort state and persists column width adjustments from table headers", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const dataStore = useDataStore(pinia);
    const analysisStore = useAnalysisStore(pinia);
    const validationStore = useValidationStore(pinia);
    const artifactsStore = useWorkflowArtifactsStore(pinia);
    workflow.selectResultsTab("validate", { persist: false });
    dataStore.dataId = "dataset_1";
    analysisStore.topRows = 1;
    artifactsStore.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-09T12:30:00Z",
        csv_path: "C:/data/aapl.csv",
        dataset_meta: {
          csv_path: "C:/data/aapl.csv",
          timeframe: "1m",
          label: "AAPL",
        },
        req: {
          strategy_kind: "DoubleMa",
          data_id: "dataset_1",
        },
        res: {
          top: [],
        },
      },
      currentValidation: null,
    });
    validationStore.tableRowCount = 2;
    validationStore.tableVisible = true;
    validationStore.validationColumns = [
      { key: "gate", label: "Gate", alignLeft: true },
      { key: "sharpe", label: "Sharpe", alignLeft: false },
    ];
    validationStore.validationRows = [
      {
        cells: [
          { text: "pass", className: "left", title: "pass" },
          { text: "1.20", className: "mono", title: "1.20" },
        ],
      },
      {
        cells: [
          { text: "warn", className: "left", title: "warn" },
          { text: "2.40", className: "mono", title: "2.40" },
        ],
      },
    ];

    mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    const sharpeHeaderButton = document.querySelector<HTMLButtonElement>(
      ".headerButton:not(.left)",
    );
    const sharpeHeader =
      document.querySelectorAll<HTMLTableCellElement>("th")[1];
    expect(sharpeHeaderButton).not.toBeNull();
    sharpeHeaderButton?.click();
    await flushPanel();
    expect(sharpeHeader?.getAttribute("aria-sort")).toBe("ascending");

    sharpeHeaderButton?.click();
    await flushPanel();
    expect(sharpeHeader?.getAttribute("aria-sort")).toBe("descending");

    const resizeHandle = document.querySelector<HTMLButtonElement>(
      '[aria-label="Resize Sharpe column"]',
    );
    resizeHandle?.dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }),
    );
    await flushPanel();

    expect(localStorage.getItem(VALIDATION_RESULTS_TABLE_STATE_KEY)).toContain(
      '"columnWidths":{"sharpe":144}',
    );
    expect(
      document
        .querySelectorAll<HTMLTableCellElement>("th")[1]
        ?.getAttribute("style"),
    ).toContain("144px");
  });

  it("applies built-in validation table presets without misaligning row cells", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    const workflow = useWorkflowStore(pinia);
    const dataStore = useDataStore(pinia);
    const analysisStore = useAnalysisStore(pinia);
    const validationStore = useValidationStore(pinia);
    workflow.selectResultsTab("validate", { persist: false });
    dataStore.dataId = "dataset_1";
    analysisStore.topRows = 1;
    validationStore.tableRowCount = 1;
    validationStore.tableVisible = true;
    validationStore.validationColumns = [
      { key: "rank", label: "#", alignLeft: true },
      { key: "fast", label: "Fast", alignLeft: true },
      { key: "slow", label: "Slow", alignLeft: true },
      { key: "psr", label: "PSR", alignLeft: false },
      { key: "dsr", label: "DSR", alignLeft: false },
      { key: "gate", label: "Gate", alignLeft: true },
      { key: "robust_pareto", label: "Robust Pareto", alignLeft: true },
      { key: "cross", label: "Cross", alignLeft: false },
    ];
    validationStore.validationRows = [
      {
        cells: [
          { text: "1", className: "left", title: "1" },
          { text: "fast=8", className: "left", title: "fast=8" },
          { text: "slow=24", className: "left", title: "slow=24" },
          { text: "0.91", className: "", title: "0.91" },
          { text: "0.87", className: "", title: "0.87" },
          { text: "pass", className: "left", title: "pass" },
          { text: "frontier", className: "left", title: "frontier" },
          { text: "3/4", className: "", title: "3/4" },
        ],
      },
    ];

    mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    document
      .querySelector<HTMLButtonElement>(
        '[data-qa="validation.table.view.gates"]',
      )
      ?.click();
    await flushPanel();

    expect(
      Array.from(document.querySelectorAll("th")).map((cell) =>
        String(cell.textContent || "").trim(),
      ),
    ).toEqual([
      "#",
      "Gate",
      "Robust Pareto",
      "Cross",
      "PSR",
      "DSR",
      "Fast",
      "Slow",
    ]);
    expect(localStorage.getItem(VALIDATION_RESULTS_TABLE_STATE_KEY)).toContain(
      '"columnOrderKeys":["rank","gate","robust_pareto","cross","psr","dsr","fast","slow"]',
    );
    expect(
      document.querySelector('[data-qa-panel="validation.rowDetail"]')
        ?.textContent,
    ).toContain("Row 1 of 1");
    expect(
      document.querySelector('[data-qa-state="validation.rowDetail.gate"]')
        ?.textContent,
    ).toContain("pass");
    expect(
      document.querySelector(
        '[data-qa-state="validation.rowDetail.robust_pareto"]',
      )?.textContent,
    ).toContain("frontier");
    expect(
      document.querySelector('[data-qa-state="validation.rowDetail.cross"]')
        ?.textContent,
    ).toContain("3/4");
  });
});
