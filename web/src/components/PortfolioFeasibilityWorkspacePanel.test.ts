import { createPinia, setActivePinia } from "pinia";
import { flushPromises, mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  loadSavedRunDetail,
  type HistorySavedRunDetail,
  type HistorySavedRunSummary,
} from "../services/historyService";
import {
  analyzePortfolioFeasibility,
  type PortfolioFeasibilityResult,
} from "../services/portfolioService";
import { useHistoryStore } from "../stores/historyStore";
import PortfolioFeasibilityWorkspacePanel from "./PortfolioFeasibilityWorkspacePanel.vue";

const copyTextToDesktopClipboardMock = vi.hoisted(() => vi.fn());

vi.mock("../services/desktopClipboard", () => ({
  copyTextToDesktopClipboard: copyTextToDesktopClipboardMock,
}));

vi.mock("../services/historyService", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("../services/historyService")>();
  return {
    ...actual,
    loadSavedRunDetail: vi.fn(),
  };
});

vi.mock("../services/portfolioService", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("../services/portfolioService")>();
  return {
    ...actual,
    analyzePortfolioFeasibility: vi.fn(),
  };
});

const loadSavedRunDetailMock = vi.mocked(loadSavedRunDetail);
const analyzePortfolioFeasibilityMock = vi.mocked(
  analyzePortfolioFeasibility,
);

const savedRunPath =
  "C:\\Users\\researcher\\VectorGrid\\artifacts\\portfolio-source.json";
const configFingerprint =
  "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const savedRun: HistorySavedRunSummary = {
  ts: "",
  path: savedRunPath,
  bytes: 2048,
  strategyKind: "",
  configFingerprint,
  hasValidation: true,
  reportXlsxPath: "",
  csvPath: "C:\\Users\\researcher\\VectorGrid\\data\\spy.csv",
  timeframe: "",
  tags: [],
};
const secondSavedRun: HistorySavedRunSummary = {
  ...savedRun,
  path: "C:\\Users\\researcher\\VectorGrid\\artifacts\\second-source.json",
  configFingerprint: "",
  hasValidation: false,
};
const savedRunDetail: HistorySavedRunDetail = {
  ts: "2026-07-28T12:00:00Z",
  artifactPath: savedRunPath,
  strategyKind: "DoubleMa",
  configFingerprint,
  timeframe: "5m",
  csvPath: "C:\\Users\\researcher\\VectorGrid\\data\\spy.csv",
  reportPath: "C:\\Users\\researcher\\VectorGrid\\reports\\portfolio.xlsx",
  paramsText: "",
  hasValidation: true,
  datasetMeta: { dataset_hash: "dataset-hash-1" },
  routeProvenance: null,
  lineage: null,
  optimizationMetricSurface: null,
  stageCompletions: [],
  runReq: { strategy_kind: "DoubleMa", timeframe: "5m" },
  runRes: { top: [{ candidate_id: "candidate-1" }] },
  validationReq: { min_psr: 0.8 },
  validationRes: {
    candidates: [{ candidate_id: "candidate-1", validation_passed: true }],
  },
  tags: [],
};
const feasibilityResult: PortfolioFeasibilityResult = {
  version: 1,
  status: "replay_parity_passed",
  sourceArtifactsSeen: 1,
  eligibleCandidates: [
    {
      candidateId: "candidate-1",
      sourceArtifactIndex: 0,
      sourceRole: "validation_artifact",
      strategyKind: "DoubleMa",
      symbolRoot: "SPY",
      timeframe: "5m",
      paramsHash: "params-1",
      assetClass: "equity",
      admission: {
        validationPassed: true,
        runApproved: true,
        finalRecommendation: "approve",
        psr: 0.92,
        dsr: null,
        pbo: null,
        crossTickerScore: null,
        robustScore: null,
        optionsValidationVerdict: "",
        optionsNonPromotable: null,
        optionsStructureFamily: "",
      },
      summaryMetrics: {
        pnl: 125,
        sharpe: 1.45,
        sortino: null,
        calmar: null,
        omega: null,
        volatility: null,
        maxDrawdown: -12,
        trades: 42,
        exposure: null,
        netExposure: null,
      },
      replayStatus: "replay_matrix_present",
    },
  ],
  rejectedCandidates: [],
  missingEvidence: [],
  warnings: [],
  nextRequiredStep: "Proceed to covariance research.",
};

let unmountPanel: (() => void) | null = null;
let restoreObjectUrlMethods: (() => void) | null = null;

function installPortfolioPanelMountShell(): void {
  document.body.innerHTML = `
    <section id="portfolioShellGrid">
      <div id="portfolioVueShellMount"></div>
    </section>
  `;
}

async function flushPanel(): Promise<void> {
  await flushPromises();
  await nextTick();
  await nextTick();
}

function mountPanel(
  runs: HistorySavedRunSummary[] = [savedRun, secondSavedRun],
  configure?: (historyStore: ReturnType<typeof useHistoryStore>) => void,
) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const historyStore = useHistoryStore(pinia);
  historyStore.savedRunsRaw = runs;
  historyStore.savedRunsVisible = runs;
  configure?.(historyStore);
  const wrapper = mount(PortfolioFeasibilityWorkspacePanel, {
    global: { plugins: [pinia] },
    attachTo: document.body,
  });
  unmountPanel = () => wrapper.unmount();
  return { historyStore, wrapper };
}

function installObjectUrlMocks(): {
  createObjectURL: ReturnType<typeof vi.fn>;
  revokeObjectURL: ReturnType<typeof vi.fn>;
} {
  type MutableUrlObject = {
    createObjectURL?: (value: Blob | MediaSource) => string;
    revokeObjectURL?: (url: string) => void;
  };
  const mutableUrl = URL as unknown as MutableUrlObject;
  const createDescriptor = Object.getOwnPropertyDescriptor(
    URL,
    "createObjectURL",
  );
  const revokeDescriptor = Object.getOwnPropertyDescriptor(
    URL,
    "revokeObjectURL",
  );
  const createObjectURL = vi.fn(
    (_value: Blob | MediaSource) => "blob:portfolio-export",
  );
  const revokeObjectURL = vi.fn((_url: string) => undefined);
  Object.defineProperty(URL, "createObjectURL", {
    configurable: true,
    value: createObjectURL,
  });
  Object.defineProperty(URL, "revokeObjectURL", {
    configurable: true,
    value: revokeObjectURL,
  });
  restoreObjectUrlMethods = () => {
    if (createDescriptor) {
      Object.defineProperty(URL, "createObjectURL", createDescriptor);
    } else {
      delete mutableUrl.createObjectURL;
    }
    if (revokeDescriptor) {
      Object.defineProperty(URL, "revokeObjectURL", revokeDescriptor);
    } else {
      delete mutableUrl.revokeObjectURL;
    }
  };
  return { createObjectURL, revokeObjectURL };
}

function selectFirstSavedRun(): HTMLInputElement {
  const checkbox = document.querySelector<HTMLInputElement>(
    '.savedRunRow input[type="checkbox"]',
  );
  if (!checkbox) throw new Error("Expected a saved-run source checkbox.");
  checkbox.click();
  return checkbox;
}

describe("PortfolioFeasibilityWorkspacePanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    loadSavedRunDetailMock.mockReset();
    loadSavedRunDetailMock.mockResolvedValue(savedRunDetail);
    analyzePortfolioFeasibilityMock.mockReset();
    copyTextToDesktopClipboardMock.mockResolvedValue({
      ok: true,
      method: "navigator-clipboard",
    });
    installPortfolioPanelMountShell();
    localStorage.clear();
  });

  afterEach(() => {
    unmountPanel?.();
    unmountPanel = null;
    restoreObjectUrlMethods?.();
    restoreObjectUrlMethods = null;
    vi.restoreAllMocks();
    document.body.innerHTML = "";
  });

  it("compacts saved-run evidence without letting evidence actions change selection", async () => {
    const { historyStore } = mountPanel();
    const revealSavedRunSpy = vi
      .spyOn(historyStore, "revealSavedRun")
      .mockImplementation(() => {});

    await flushPanel();

    const row = document.querySelector(".savedRunRow") as HTMLElement;
    const checkbox = row.querySelector(
      'input[type="checkbox"]',
    ) as HTMLInputElement;
    const selectionLabel = row.querySelector(
      ".savedRunSelectionLabel",
    ) as HTMLLabelElement;
    const evidenceValues = Array.from(
      row.querySelectorAll(".compactEvidenceText"),
    ) as HTMLElement[];

    expect(row.tagName).toBe("DIV");
    expect(selectionLabel.htmlFor).toBe(checkbox.id);
    expect(selectionLabel.textContent).toContain("portfolio-source.json");
    expect(selectionLabel.textContent).toContain("validation");
    expect(selectionLabel.querySelector("button")).toBeNull();
    expect(checkbox.checked).toBe(false);

    expect(evidenceValues).toHaveLength(2);
    expect(evidenceValues[0]?.textContent).toBe("portfolio-source.json");
    expect(evidenceValues[0]?.getAttribute("title")).toBe(savedRunPath);
    expect(evidenceValues[0]?.getAttribute("aria-label")).toBe(
      `saved run path: ${savedRunPath}`,
    );
    expect(evidenceValues[1]?.textContent).toBe("0123456789…abcdef");
    expect(evidenceValues[1]?.getAttribute("title")).toBe(configFingerprint);
    expect(evidenceValues[1]?.getAttribute("aria-label")).toBe(
      `config fingerprint: ${configFingerprint}`,
    );

    const copyButtons = Array.from(
      row.querySelectorAll<HTMLButtonElement>(
        '[data-qa-action="evidence.copy"]',
      ),
    );
    expect(copyButtons).toHaveLength(2);
    copyButtons[0]?.click();
    copyButtons[1]?.click();
    await flushPanel();

    expect(copyTextToDesktopClipboardMock).toHaveBeenNthCalledWith(
      1,
      savedRunPath,
    );
    expect(copyTextToDesktopClipboardMock).toHaveBeenNthCalledWith(
      2,
      configFingerprint,
    );
    expect(checkbox.checked).toBe(false);

    const revealButtons = Array.from(
      document.querySelectorAll<HTMLButtonElement>(
        '[data-qa-action="portfolio.saved.reveal"]',
      ),
    );
    expect(revealButtons.map((button) => button.getAttribute("aria-label"))).toEqual([
      "Reveal saved run portfolio-source.json",
      "Reveal saved run second-source.json",
    ]);
    const revealButton = revealButtons[0] as HTMLButtonElement;
    expect(revealButton.getAttribute("aria-label")).toBe(
      "Reveal saved run portfolio-source.json",
    );
    revealButton.click();
    await flushPanel();

    expect(revealSavedRunSpy).toHaveBeenCalledOnce();
    expect(revealSavedRunSpy).toHaveBeenCalledWith(savedRunPath);
    expect(checkbox.checked).toBe(false);

    checkbox.click();
    await flushPanel();

    expect(checkbox.checked).toBe(true);
    expect(row.classList.contains("active")).toBe(true);
    expect(document.body.textContent).toContain(
      "1 catalog artifact selected",
    );
  });

  it("uses standard empty and status states when no sources are available", async () => {
    mountPanel([], (historyStore) => {
      historyStore.canRefreshSavedRuns = false;
    });
    await flushPanel();

    const workspace = document.querySelector<HTMLElement>(
      '[data-qa-panel="portfolio.feasibility"]',
    );
    const sourcePane = document.querySelector<HTMLElement>(".sourcePane");
    const status = document.querySelector<HTMLElement>(".statusStrip");
    const emptyStates = Array.from(
      document.querySelectorAll<HTMLElement>(".appEmptyState"),
    );
    const refreshButton = document.querySelector<HTMLButtonElement>(
      '[data-qa-action="portfolio.refreshSources"]',
    );
    const analyzeButton = document.querySelector<HTMLButtonElement>(
      '[data-qa-action="portfolio.runFeasibility"]',
    );

    expect(workspace?.dataset.state).toBe("idle");
    expect(workspace?.getAttribute("aria-busy")).toBe("false");
    expect(sourcePane?.getAttribute("aria-busy")).toBe("false");
    expect(status?.getAttribute("role")).toBe("status");
    expect(status?.getAttribute("aria-live")).toBe("polite");
    expect(status?.dataset.state).toBe("idle");
    expect(status?.textContent).toContain("Not analyzed");
    expect(emptyStates.map((state) => state.textContent)).toEqual(
      expect.arrayContaining([
        expect.stringContaining("No saved run artifacts"),
        expect.stringContaining("No feasibility analysis yet"),
      ]),
    );
    expect(refreshButton?.disabled).toBe(true);
    expect(refreshButton?.title).toBe("Saved-run refresh is unavailable.");
    expect(analyzeButton?.disabled).toBe(true);
    expect(analyzeButton?.title).toBe(
      "Select at least one portfolio source.",
    );
  });

  it("prevents duplicate refresh actions and reports catalog completion truthfully", async () => {
    const { historyStore } = mountPanel();
    const refreshSavedRunsSpy = vi
      .spyOn(historyStore, "refreshSavedRuns")
      .mockImplementation(() => {});
    await flushPanel();

    const refreshButton = document.querySelector<HTMLButtonElement>(
      '[data-qa-action="portfolio.refreshSources"]',
    );
    const analyzeButton = document.querySelector<HTMLButtonElement>(
      '[data-qa-action="portfolio.runFeasibility"]',
    );
    const workspace = document.querySelector<HTMLElement>(
      '[data-qa-panel="portfolio.feasibility"]',
    );
    const sourcePane = document.querySelector<HTMLElement>(".sourcePane");
    const status = document.querySelector<HTMLElement>(".statusStrip");
    if (!refreshButton || !analyzeButton || !workspace || !sourcePane || !status) {
      throw new Error("Expected portfolio refresh controls and status.");
    }

    refreshButton.click();
    refreshButton.click();
    await nextTick();

    expect(refreshSavedRunsSpy).toHaveBeenCalledOnce();
    expect(refreshButton.disabled).toBe(true);
    expect(refreshButton.getAttribute("aria-busy")).toBe("true");
    expect(refreshButton.textContent).toContain("Refreshing…");
    expect(analyzeButton.disabled).toBe(true);
    expect(workspace.getAttribute("aria-busy")).toBe("true");
    expect(workspace.dataset.operation).toBe("refresh");
    expect(sourcePane.getAttribute("aria-busy")).toBe("true");
    expect(status.dataset.state).toBe("loading");
    expect(status.getAttribute("role")).toBe("status");
    expect(status.textContent).toContain("Refreshing saved sources");

    historyStore.statusLine = "Refreshed saved runs.";
    historyStore.updatedAtUnixMs += 1;
    await flushPanel();

    expect(workspace.getAttribute("aria-busy")).toBe("false");
    expect(workspace.dataset.state).toBe("success");
    expect(workspace.hasAttribute("data-operation")).toBe(false);
    expect(sourcePane.getAttribute("aria-busy")).toBe("false");
    expect(refreshButton.disabled).toBe(false);
    expect(refreshButton.hasAttribute("aria-busy")).toBe(false);
    expect(status.dataset.state).toBe("success");
    expect(status.textContent).toContain("Saved sources refreshed");
    expect(status.textContent).toContain("Saved run sources refreshed.");
  });

  it("runs one analysis request, renders the result, and exports JSON and CSV", async () => {
    let resolveAnalysis:
      | ((value: PortfolioFeasibilityResult) => void)
      | undefined;
    const analysisPending = new Promise<PortfolioFeasibilityResult>(
      (resolve) => {
        resolveAnalysis = resolve;
      },
    );
    analyzePortfolioFeasibilityMock.mockReturnValue(analysisPending);
    mountPanel();
    await flushPanel();
    const checkbox = selectFirstSavedRun();
    await nextTick();

    const analyzeButton = document.querySelector<HTMLButtonElement>(
      '[data-qa-action="portfolio.runFeasibility"]',
    );
    const refreshButton = document.querySelector<HTMLButtonElement>(
      '[data-qa-action="portfolio.refreshSources"]',
    );
    const workspace = document.querySelector<HTMLElement>(
      '[data-qa-panel="portfolio.feasibility"]',
    );
    const resultPane = document.querySelector<HTMLElement>(".resultPane");
    const status = document.querySelector<HTMLElement>(".statusStrip");
    if (
      !analyzeButton ||
      !refreshButton ||
      !workspace ||
      !resultPane ||
      !status
    ) {
      throw new Error("Expected portfolio analysis controls and status.");
    }

    analyzeButton.click();
    analyzeButton.click();
    await flushPanel();

    expect(loadSavedRunDetailMock).toHaveBeenCalledOnce();
    expect(loadSavedRunDetailMock).toHaveBeenCalledWith(savedRunPath);
    expect(analyzePortfolioFeasibilityMock).toHaveBeenCalledOnce();
    expect(analyzePortfolioFeasibilityMock).toHaveBeenCalledWith({
      sourceArtifacts: [
        expect.objectContaining({
          source_kind: "saved_run",
          artifact_path: savedRunPath,
          strategy_kind: "DoubleMa",
        }),
      ],
      maxCandidates: 50,
      requireApprovedValidation: true,
      requireReplayMatrix: false,
      requireSourceProvenance: false,
    });
    expect(workspace.dataset.state).toBe("loading");
    expect(workspace.dataset.operation).toBe("analyze");
    expect(workspace.getAttribute("aria-busy")).toBe("true");
    expect(resultPane.dataset.state).toBe("loading");
    expect(resultPane.getAttribute("aria-busy")).toBe("true");
    expect(analyzeButton.disabled).toBe(true);
    expect(analyzeButton.getAttribute("aria-busy")).toBe("true");
    expect(analyzeButton.textContent).toContain("Analyzing…");
    expect(refreshButton.disabled).toBe(true);
    expect(checkbox.disabled).toBe(true);
    expect(status.dataset.state).toBe("loading");
    expect(status.getAttribute("role")).toBe("status");

    resolveAnalysis?.(feasibilityResult);
    await flushPanel();

    expect(workspace.dataset.state).toBe("success");
    expect(workspace.getAttribute("aria-busy")).toBe("false");
    expect(resultPane.dataset.state).toBe("success");
    expect(resultPane.getAttribute("aria-busy")).toBe("false");
    expect(analyzeButton.disabled).toBe(false);
    expect(analyzeButton.hasAttribute("aria-busy")).toBe(false);
    expect(refreshButton.disabled).toBe(false);
    expect(checkbox.disabled).toBe(false);
    expect(status.dataset.state).toBe("success");
    expect(status.textContent).toContain("Replay parity passed");
    expect(status.textContent).toContain("Analyzed 1 source.");
    expect(document.body.textContent).toContain("candidate-1");
    expect(document.body.textContent).toContain(
      "Proceed to covariance research.",
    );

    const { createObjectURL, revokeObjectURL } = installObjectUrlMocks();
    const downloads: string[] = [];
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(
      function clickDownload(this: HTMLAnchorElement): void {
        downloads.push(this.download);
      },
    );
    const jsonButton = document.querySelector<HTMLButtonElement>(
      '[data-qa-action="portfolio.exportJson"]',
    );
    const csvButton = document.querySelector<HTMLButtonElement>(
      '[data-qa-action="portfolio.exportCsv"]',
    );
    if (!jsonButton || !csvButton) {
      throw new Error("Expected portfolio export controls.");
    }

    jsonButton.click();
    csvButton.click();
    await nextTick();

    expect(downloads).toHaveLength(2);
    expect(downloads[0]).toMatch(/^portfolio-feasibility-\d+\.json$/);
    expect(downloads[1]).toMatch(/^portfolio-feasibility-\d+\.csv$/);
    expect(createObjectURL).toHaveBeenCalledTimes(2);
    expect(revokeObjectURL).toHaveBeenCalledTimes(2);
    expect(revokeObjectURL).toHaveBeenNthCalledWith(
      1,
      "blob:portfolio-export",
    );
    const exportedBlobs = createObjectURL.mock.calls.map((call) => call[0]);
    expect(exportedBlobs[0]).toBeInstanceOf(Blob);
    expect(exportedBlobs[0]?.type).toBe("application/json");
    expect(exportedBlobs[1]?.type).toBe("text/csv");
    expect(await exportedBlobs[0]?.text()).toContain('"research_only": true');
    expect(await exportedBlobs[1]?.text()).toContain(
      "row_type,source_index,candidate_id",
    );
    expect(status.textContent).toContain(
      "Exported compact portfolio feasibility CSV.",
    );
  });

  it("announces analysis failures and restores retry controls", async () => {
    analyzePortfolioFeasibilityMock.mockRejectedValue(
      new Error("Portfolio backend unavailable."),
    );
    mountPanel();
    await flushPanel();
    selectFirstSavedRun();
    await nextTick();

    const analyzeButton = document.querySelector<HTMLButtonElement>(
      '[data-qa-action="portfolio.runFeasibility"]',
    );
    if (!analyzeButton) throw new Error("Expected Analyze control.");
    analyzeButton.click();
    await flushPanel();

    const workspace = document.querySelector<HTMLElement>(
      '[data-qa-panel="portfolio.feasibility"]',
    );
    const resultPane = document.querySelector<HTMLElement>(".resultPane");
    const status = document.querySelector<HTMLElement>(".statusStrip");
    const resultEmpty = document.querySelector<HTMLElement>(
      ".resultPane .appEmptyState",
    );

    expect(analyzePortfolioFeasibilityMock).toHaveBeenCalledOnce();
    expect(workspace?.dataset.state).toBe("error");
    expect(workspace?.getAttribute("aria-busy")).toBe("false");
    expect(resultPane?.dataset.state).toBe("error");
    expect(resultPane?.getAttribute("aria-busy")).toBe("false");
    expect(status?.dataset.state).toBe("error");
    expect(status?.getAttribute("role")).toBe("alert");
    expect(status?.getAttribute("aria-live")).toBe("assertive");
    expect(status?.getAttribute("aria-busy")).toBe("false");
    expect(status?.textContent).toContain("Analysis failed");
    expect(status?.textContent).toContain("Portfolio backend unavailable.");
    expect(resultEmpty?.textContent).toContain(
      "No feasibility result available",
    );
    expect(analyzeButton.disabled).toBe(false);
    expect(analyzeButton.hasAttribute("aria-busy")).toBe(false);
  });
});
