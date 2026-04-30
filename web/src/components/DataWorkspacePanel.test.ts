import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import DataWorkspacePanel from "./DataWorkspacePanel.vue";
import { useDataStore } from "../stores/dataStore";
import { useWorkflowStore } from "../stores/workflowStore";

describe("DataWorkspacePanel", () => {
  beforeEach(() => {
    document.body.innerHTML = `
      <section data-vue-panel="data"></section>
      <div id="dataVueShellMount"></div>
    `;
  });

  it("exposes cached Polygon selection state, keyboard navigation, and announces the current selection", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const dataStore = useDataStore(pinia);
    const workflowStore = useWorkflowStore(pinia);

    vi.spyOn(dataStore, "hydrateStartupState").mockResolvedValue(undefined);
    const forwardSetInputField = dataStore.setInputField.bind(dataStore);
    const setInputSpy = vi
      .spyOn(dataStore, "setInputField")
      .mockImplementation((field: string, value: string) => {
        forwardSetInputField(field, value);
      });

    workflowStore.selectControlsTab("data", { persist: false });
    dataStore.polygonOpen = true;
    dataStore.polygonDatasets = [
      {
        csvPath: "C:/cache/aapl_1m.csv",
        displaySymbol: "AAPL",
        polygonTicker: "AAPL",
        timeframe: "1m",
        rowCount: 5000,
      },
      {
        csvPath: "C:/cache/msft_5m.csv",
        displaySymbol: "MSFT",
        polygonTicker: "MSFT",
        timeframe: "5m",
        rowCount: 1200,
      },
    ] as never[];
    dataStore.setInputField("polygonSelectedPath", "C:/cache/aapl_1m.csv");

    mount(DataWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    });

    await nextTick();
    await nextTick();

    const buttons = Array.from(
      document.querySelectorAll<HTMLButtonElement>(".stackList .listCard"),
    );
    expect(buttons).toHaveLength(2);
    expect(buttons[0]?.getAttribute("aria-pressed")).toBe("true");
    expect(buttons[1]?.getAttribute("aria-pressed")).toBe("false");

    const selectionStatus = document.querySelector<HTMLElement>(
      "#polygonSelectionStatus",
    );
    expect(selectionStatus?.textContent).toContain("AAPL");
    expect(selectionStatus?.textContent).toContain("1m");
    expect(selectionStatus?.textContent).toContain("5000 rows");

    const loadSelected = Array.from(
      document.querySelectorAll<HTMLButtonElement>("button"),
    ).find((button) =>
      String(button.textContent || "").includes("Load selected"),
    );
    const validateSelected = Array.from(
      document.querySelectorAll<HTMLButtonElement>("button"),
    ).find((button) =>
      String(button.textContent || "").includes("Validate selected"),
    );
    const repairSelected = Array.from(
      document.querySelectorAll<HTMLButtonElement>("button"),
    ).find((button) =>
      String(button.textContent || "").includes("Repair selected"),
    );
    expect(loadSelected?.disabled).toBe(false);
    expect(validateSelected?.disabled).toBe(false);
    expect(repairSelected?.disabled).toBe(false);

    buttons[1]?.click();
    expect(setInputSpy).toHaveBeenCalledWith(
      "polygonSelectedPath",
      "C:/cache/msft_5m.csv",
    );

    buttons[0]?.focus();
    buttons[0]?.dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }),
    );
    expect(document.activeElement).toBe(buttons[1]);

    buttons[1]?.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Home", bubbles: true }),
    );
    expect(document.activeElement).toBe(buttons[0]);
  });

  it("renders dataset context, sample affordances, recent paths, and storage hints together", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const dataStore = useDataStore(pinia);
    const workflowStore = useWorkflowStore(pinia);

    vi.spyOn(dataStore, "hydrateStartupState").mockResolvedValue(undefined);

    workflowStore.selectControlsTab("data", { persist: false });
    dataStore.csvPath = "C:/tmp/aapl.csv";
    dataStore.recentCsvPaths = ["C:/tmp/recent.csv"];
    dataStore.sampleCsvOptions = [
      { label: "10k candles", path: "C:/samples/10k.csv", disabled: false },
    ] as never[];
    dataStore.sampleCsvVisible = true;
    dataStore.dataId = "data-1@15m";
    dataStore.datasetLabel = "aapl.csv";
    dataStore.datasetRange = "15m • 2023-11-14 -> 2023-11-15";
    dataStore.dataLen = "500";
    dataStore.baseDataMeta = {
      timeframe: "1m",
    } as never;
    dataStore.dataTimeframe = "15m";
    dataStore.dataTimeframeHint =
      "Using 15m derived from base 1m (no extra downloads). Periods are in candles.";
    dataStore.datasetContext =
      "source=csv • file=aapl.csv • load=csv parsed + cached";
    dataStore.dataFields = ["open", "high", "low", "close"];
    dataStore.dataFieldHint = "No volume -> VWMA/VPWMA disabled.";
    dataStore.marketDataDirHint =
      "Using default root: C:/default · Polygon cache: C:/default/polygon-cache";
    dataStore.statusLine = "Restored aapl.csv.";

    mount(DataWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    });

    await nextTick();
    await nextTick();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain("Dataset: aapl.csv");
    expect(bodyText).toContain("Rows: 500");
    expect(bodyText).toContain("Timeframe: 15m");
    expect(bodyText).toContain("Base timeframe: 1m");
    expect(bodyText).toContain("Fields: 4");
    expect(bodyText).toContain("10k candles");
    expect(bodyText).toContain("C:/tmp/recent.csv");
    expect(bodyText).toContain("source=csv");
    expect(bodyText).toContain("No volume -> VWMA/VPWMA disabled.");
    expect(bodyText).toContain("Using default root: C:/default");
    expect(bodyText).toContain("Restored aapl.csv.");
  });

  it("renders Polygon cache summaries with accurate API-key storage wording", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const dataStore = useDataStore(pinia);
    const workflowStore = useWorkflowStore(pinia);

    vi.spyOn(dataStore, "hydrateStartupState").mockResolvedValue(undefined);

    workflowStore.selectControlsTab("data", { persist: false });
    dataStore.polygonOpen = true;
    dataStore.polygonHasApiKey = true;
    dataStore.polygonCacheDir = "C:/default/polygon-cache";
    dataStore.polygonCacheMeta =
      "API key saved · Cache: C:/default/polygon-cache · 1 cached dataset";
    dataStore.polygonDatasets = [
      {
        csvPath: "C:/default/polygon-cache/AAPL_1m.csv",
        displaySymbol: "AAPL",
        polygonTicker: "AAPL",
        timeframe: "1m",
        rowCount: 5000,
        adjusted: true,
        integrity: null,
      },
    ] as never[];
    dataStore.polygonSelectedPath = "C:/default/polygon-cache/AAPL_1m.csv";

    mount(DataWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    });

    await nextTick();
    await nextTick();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain(
      "API key saved in secure system storage when available.",
    );
    expect(bodyText).toContain(
      "Local Polygon settings remain a compatibility fallback.",
    );
    expect(bodyText).toContain(
      "API key saved · Cache: C:/default/polygon-cache · 1 cached dataset",
    );
    expect(bodyText).toContain(
      "Downloaded 1m bars are cached here and reused automatically",
    );
    expect(bodyText).toContain(
      "selected timeframes load from local resampling",
    );
  });

  it("renders the empty Polygon listing guidance and selected-dataset prompt", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const dataStore = useDataStore(pinia);
    const workflowStore = useWorkflowStore(pinia);

    vi.spyOn(dataStore, "hydrateStartupState").mockResolvedValue(undefined);

    workflowStore.selectControlsTab("data", { persist: false });
    dataStore.polygonOpen = true;
    dataStore.polygonHasApiKey = false;
    dataStore.polygonCacheDir = "C:/default/polygon-cache";
    dataStore.polygonCacheMeta =
      "API key missing · Cache: C:/default/polygon-cache · 0 cached datasets";

    mount(DataWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    });

    await nextTick();
    await nextTick();

    const bodyText = document.body.textContent || "";
    expect(bodyText).toContain("No cached Polygon datasets yet.");
    expect(bodyText).toContain(
      "Select a cached Polygon dataset to inspect, validate, repair, or load.",
    );
    expect(bodyText).toContain(
      "API key missing · Cache: C:/default/polygon-cache · 0 cached datasets",
    );
  });
});
