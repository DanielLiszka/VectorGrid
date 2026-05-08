import { describe, expect, it } from "vitest";
import { csvLine, escapeCsvCell } from "./csv";

describe("csv utilities", () => {
  it("escapes CSV syntax and neutralizes text formulas", () => {
    expect(escapeCsvCell('a,"b"')).toBe('"a,""b"""');
    expect(escapeCsvCell("=cmd|' /C calc'!A0")).toBe("'=cmd|' /C calc'!A0");
    expect(escapeCsvCell(" @SUM(A1:A2)")).toBe("' @SUM(A1:A2)");
  });

  it("keeps numeric negative cells numeric while protecting string negatives", () => {
    expect(escapeCsvCell(-0.05)).toBe("-0.05");
    expect(escapeCsvCell("-not-a-number")).toBe("'-not-a-number");
  });

  it("builds deterministic CSV rows", () => {
    expect(csvLine(["safe", "a,b", "+formula"])).toBe('safe,"a,b",\'+formula');
  });
});
