// @vitest-environment jsdom
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { computed, nextTick, ref } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import HistoryWorkspacePanel from "./HistoryWorkspacePanel.vue";
import { useHistoryStore } from "../stores/historyStore";
import { useWorkflowStore } from "../stores/workflowStore";

const historyVirtualState = vi.hoisted(() => ({
  visibleIndexes: [1, 2] as number[],
  topSpacerPx: 84,
  bottomSpacerPx: 168,
  scrollToTop: vi.fn(),
}));

vi.mock("../composables/useVirtualTableRows", () => ({
  useVirtualTableRows: (rows: { value: unknown[] }) => ({
    scrollElementRef: ref(null),
    virtualRows: computed(() =>
      historyVirtualState.visibleIndexes
        .map((index) => ({ index, row: rows.value[index] }))
        .filter((entry) => entry.row !== undefined),
    ),
    topSpacerPx: computed(() => historyVirtualState.topSpacerPx),
    bottomSpacerPx: computed(() => historyVirtualState.bottomSpacerPx),
    scrollToTop: historyVirtualState.scrollToTop,
  }),
}));

describe("HistoryWorkspacePanel virtualization", () => {
  let pinia: ReturnType<typeof createPinia>;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    historyVirtualState.scrollToTop.mockReset();
    document.body.innerHTML = `
      <div data-vue-panel="history"></div>
      <div id="historyVueShellMount"></div>
    `;
  });

  it("renders the saved-run virtual window inside the owned saved-run viewport", async () => {
    const history = useHistoryStore();
    const workflow = useWorkflowStore();
    vi.spyOn(history, "refreshSessionHistory").mockImplementation(() => {});

    workflow.workspace = "library";
    history.savedRunCount = 4;
    history.savedRunsVisible = [
      {
        ts: "2026-04-11T10:00:00Z",
        path: "C:/runs/run_01.json",
        bytes: 1024,
        strategyKind: "DoubleMa",
        hasValidation: true,
        reportXlsxPath: "C:/runs/run_01.xlsx",
        csvPath: "C:/data/a.csv",
        tags: ["one"],
      },
      {
        ts: "2026-04-11T11:00:00Z",
        path: "C:/runs/run_02.json",
        bytes: 2048,
        strategyKind: "DoubleMa",
        hasValidation: true,
        reportXlsxPath: "",
        csvPath: "C:/data/b.csv",
        tags: ["two"],
      },
      {
        ts: "2026-04-11T12:00:00Z",
        path: "C:/runs/run_03.json",
        bytes: 4096,
        strategyKind: "CompositeTrend",
        hasValidation: false,
        reportXlsxPath: "",
        csvPath: "C:/data/c.csv",
        tags: ["three"],
      },
      {
        ts: "2026-04-11T13:00:00Z",
        path: "C:/runs/run_04.json",
        bytes: 8192,
        strategyKind: "MlSignal",
        hasValidation: true,
        reportXlsxPath: "C:/runs/run_04.xlsx",
        csvPath: "C:/data/d.csv",
        tags: ["four"],
      },
    ];

    mount(HistoryWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    });

    await nextTick();
    await nextTick();

    const scrollHost = document.querySelector<HTMLElement>(
      'ul[aria-label="Saved run artifacts"]',
    );
    expect(scrollHost?.classList.contains("historyVirtualListViewport")).toBe(
      true,
    );

    const spacerRows = Array.from(
      document.querySelectorAll<HTMLLIElement>("li.runListSpacer"),
    );
    expect(spacerRows).toHaveLength(2);
    expect(spacerRows[0]?.getAttribute("style")).toContain("84px");
    expect(spacerRows[1]?.getAttribute("style")).toContain("168px");

    const renderedButtons = Array.from(
      document.querySelectorAll<HTMLButtonElement>('button[data-saved-run="1"]'),
    );
    expect(renderedButtons).toHaveLength(2);
    expect(renderedButtons[0]?.textContent).toContain("2026-04-11T11:00:00Z");
    expect(renderedButtons[1]?.textContent).toContain("2026-04-11T12:00:00Z");
    expect(document.body.textContent || "").not.toContain("2026-04-11T10:00:00Z");
    expect(document.body.textContent || "").not.toContain("2026-04-11T13:00:00Z");
    expect(renderedButtons[0]?.getAttribute("aria-posinset")).toBe("2");
    expect(renderedButtons[0]?.getAttribute("aria-setsize")).toBe("4");
    expect(historyVirtualState.scrollToTop).toHaveBeenCalledTimes(1);
  });
});
