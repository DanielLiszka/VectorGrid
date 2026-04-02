// @vitest-environment jsdom
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
    const setInputSpy = vi
      .spyOn(dataStore, "setInputField")
      .mockImplementation(() => {});

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
    dataStore.polygonSelectedPath = "C:/cache/aapl_1m.csv";

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
});
