import { createPinia, setActivePinia } from "pinia";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { HistorySavedRunSummary } from "../services/historyService";
import { useHistoryStore } from "../stores/historyStore";
import PortfolioFeasibilityWorkspacePanel from "./PortfolioFeasibilityWorkspacePanel.vue";

const copyTextToDesktopClipboardMock = vi.hoisted(() => vi.fn());

vi.mock("../services/desktopClipboard", () => ({
  copyTextToDesktopClipboard: copyTextToDesktopClipboardMock,
}));

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

function installPortfolioPanelMountShell(): void {
  document.body.innerHTML = `
    <section id="portfolioShellGrid">
      <div id="portfolioVueShellMount"></div>
    </section>
  `;
}

async function flushPanel(): Promise<void> {
  await nextTick();
  await nextTick();
}

describe("PortfolioFeasibilityWorkspacePanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    copyTextToDesktopClipboardMock.mockResolvedValue({
      ok: true,
      method: "navigator-clipboard",
    });
    installPortfolioPanelMountShell();
    localStorage.clear();
  });

  it("compacts saved-run evidence without letting evidence actions change selection", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const historyStore = useHistoryStore(pinia);
    historyStore.savedRunsRaw = [savedRun, secondSavedRun];
    historyStore.savedRunsVisible = [savedRun, secondSavedRun];
    const revealSavedRunSpy = vi
      .spyOn(historyStore, "revealSavedRun")
      .mockImplementation(() => {});

    mount(PortfolioFeasibilityWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
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
});
