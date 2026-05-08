import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCommand } from "../tauri/invoke";
import { openPathWithRevealFallback } from "./openPath";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => undefined),
}));

const invokeCommandMock = vi.mocked(invokeCommand);

describe("openPathWithRevealFallback", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("opens the path when open_path succeeds", async () => {
    const result = await openPathWithRevealFallback(" C:/reports/report.xlsx ");

    expect(result).toEqual({
      path: "C:/reports/report.xlsx",
      openedWith: "open_path",
    });
    expect(invokeCommandMock).toHaveBeenCalledTimes(1);
    expect(invokeCommandMock).toHaveBeenCalledWith("open_path", {
      path: "C:/reports/report.xlsx",
    });
  });

  it("falls back to reveal_path when open_path fails", async () => {
    invokeCommandMock.mockRejectedValueOnce(new Error("open failed"));

    const result = await openPathWithRevealFallback("C:/reports/report.xlsx");

    expect(result).toEqual({
      path: "C:/reports/report.xlsx",
      openedWith: "reveal_path",
    });
    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, "open_path", {
      path: "C:/reports/report.xlsx",
    });
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, "reveal_path", {
      path: "C:/reports/report.xlsx",
    });
  });

  it("reports both failures when open and reveal fail", async () => {
    invokeCommandMock
      .mockRejectedValueOnce(new Error("open denied"))
      .mockRejectedValueOnce(new Error("reveal denied"));

    await expect(
      openPathWithRevealFallback("C:/reports/missing.xlsx"),
    ).rejects.toThrow(
      "open_path failed: open denied; reveal_path failed: reveal denied",
    );
  });
});
