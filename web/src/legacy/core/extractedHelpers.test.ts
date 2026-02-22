import { describe, expect, it } from "vitest";
import {
  computeConfigFingerprint,
  parseTagsInput,
  splitHoldoutPaths,
} from "./artifactIdentity.js";
import {
  backendProgressLabel,
  compactProgressText,
  formatDoubleConstraintsProgressSummary,
  formatMaPairProgressSummary,
  formatRiskProgressSummary,
  samplerProgressLabel,
} from "./progressSummary.js";

describe("artifact identity helpers", () => {
  it("parses and deduplicates tags case-insensitively", () => {
    expect(parseTagsInput("alpha, beta Alpha\nGAMMA beta")).toEqual(["alpha", "beta", "GAMMA"]);
  });

  it("builds stable fingerprints that ignore dataset path ids", () => {
    const runA = { strategy_kind: "DoubleMa", data_id: "foo", objective: "Sharpe" };
    const runB = { strategy_kind: "DoubleMa", data_id: "bar", objective: "Sharpe" };
    const valA = { data_id: "foo", min_psr: 0.8, candidates: [{ a: 1 }], cross_ticker: { datasets: [{ csv_path: "b.csv" }, { csv_path: "a.csv" }] } };
    const valB = { data_id: "bar", min_psr: 0.8, candidates: [{ a: 999 }], cross_ticker: { datasets: [{ csv_path: "a.csv" }, { csv_path: "b.csv" }] } };
    expect(computeConfigFingerprint(runA, valA)).toBe(computeConfigFingerprint(runB, valB));
  });

  it("splits holdout paths deterministically for a given seed", () => {
    const paths = ["a.csv", "b.csv", "c.csv", "d.csv", "e.csv"];
    const out1 = splitHoldoutPaths(paths, 40, "seed-1");
    const out2 = splitHoldoutPaths(paths, 40, "seed-1");
    expect(out1).toEqual(out2);
    expect(out1.holdout.length).toBe(2);
    expect(new Set([...out1.train, ...out1.holdout]).size).toBe(paths.length);
  });
});

describe("progress summary helpers", () => {
  it("maps backend/sampler labels", () => {
    expect(backendProgressLabel("GpuKernel")).toBe("GPU kernel");
    expect(backendProgressLabel("Auto")).toBe("Auto backend");
    expect(samplerProgressLabel("Nsga2")).toBe("NSGA-II");
    expect(samplerProgressLabel("Random")).toBe("Random");
  });

  it("formats risk and constraints summaries", () => {
    expect(
      formatRiskProgressSummary({
        stop_loss_pct: 0.02,
        take_profit_pct: 0.04,
        trailing_stop_pct: 0.01,
        cooldown_bars: 3,
        signal_persistence_bars: 2,
      }),
    ).toContain("risk");
    expect(formatDoubleConstraintsProgressSummary({ min_trades: 10, max_drawdown: 0.15 })).toBe(
      "filters trades>=10, dd<=15.00%",
    );
  });

  it("formats MA pair labels and truncates progress text", () => {
    expect(formatMaPairProgressSummary(["sma"], ["ema"])).toBe("SMA vs EMA");
    expect(formatMaPairProgressSummary(["sma", "ema"], ["alma"])).toBe("MA sets f2×s1");
    const long = "x".repeat(500);
    const out = compactProgressText(long, 100);
    expect(out.length).toBeLessThanOrEqual(100);
    expect(out.endsWith("…")).toBe(true);
  });
});
