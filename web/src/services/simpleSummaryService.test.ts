import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCommand } from "../tauri/invoke";
import {
  buildSimpleSummaryDefaultName,
  saveSimpleSummaryMarkdown,
} from "./simpleSummaryService";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => undefined),
}));

const invokeCommandMock = vi.mocked(invokeCommand);

describe("simpleSummaryService", () => {
  beforeEach(() => {
    invokeCommandMock.mockReset();
  });

  it("builds safe Markdown default names", () => {
    expect(buildSimpleSummaryDefaultName("AAPL 15m / sample")).toBe(
      "vectorgrid-simple-summary-AAPL_15m_sample.md",
    );
    expect(buildSimpleSummaryDefaultName("")).toBe(
      "vectorgrid-simple-summary-run.md",
    );
  });

  it("saves Simple summary Markdown through the narrow Tauri command", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      path: " C:/exports/summary.md ",
      bytes: "128",
    });

    const result = await saveSimpleSummaryMarkdown(
      "# Summary\n\nBody",
      "summary name",
    );

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "save_simple_summary_markdown",
      {
        markdown: "# Summary\n\nBody",
        defaultName: "summary_name.md",
      },
    );
    expect(result).toEqual({
      path: "C:/exports/summary.md",
      bytes: 128,
    });
  });

  it("returns null when the save dialog is canceled", async () => {
    invokeCommandMock.mockResolvedValueOnce(null);

    await expect(saveSimpleSummaryMarkdown("# Summary")).resolves.toBeNull();
  });

  it("rejects empty summaries before opening the native save dialog", async () => {
    await expect(saveSimpleSummaryMarkdown("   \n  ")).rejects.toThrow(
      "Simple summary is empty.",
    );
    expect(invokeCommandMock).not.toHaveBeenCalled();
  });
});
