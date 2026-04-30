import { describe, expect, it } from "vitest";
import {
  buildSimpleRunSummary,
  formatSimpleRunSummaryMarkdown,
  isSimpleOptimizationStrategySupported,
  resolveSimpleValidationRecommendation,
  simpleEngineLabel,
  simpleStrategyLabel,
} from "./simpleWorkflowModel";

describe("simpleWorkflowModel", () => {
  it("uses public labels for strategy and engine names", () => {
    expect(simpleStrategyLabel("DoubleMa")).toBe("Double MA crossover");
    expect(simpleStrategyLabel("CompositeCpu")).toBe("Composite signal");
    expect(simpleEngineLabel("CpuOnly")).toBe("CPU only");
    expect(simpleEngineLabel("GpuHybridKernel")).toBe("GPU hybrid");
  });

  it("keeps Simple optimization support separate from arbitrary strategy names", () => {
    expect(isSimpleOptimizationStrategySupported("DoubleMa")).toBe(true);
    expect(isSimpleOptimizationStrategySupported("CompositeCpu")).toBe(true);
    expect(isSimpleOptimizationStrategySupported("MlSignal")).toBe(false);
  });

  it("builds an optimization summary without inventing unavailable metrics", () => {
    const summary = buildSimpleRunSummary({
      datasetLabel: "AAPL sample",
      dataTimeframe: "15m",
      dataLen: "1200",
      datasetRange: "2024-01-01 -> 2024-03-01",
      strategyKind: "DoubleMa",
      objective: "Sharpe",
      presetProfile: "Balanced",
      backend: "Auto",
      optimizationReady: true,
      selectedCandidateTitle: "Rank 1 · Sharpe 1.42",
      selectedCandidateSummary: "Return 12.4% · max drawdown 4.1%",
      topRows: 12,
      bestStats: [
        { label: "Return", value: "12.4%", tone: "success" },
        { label: "Max DD", value: "4.1%", tone: "warning" },
      ],
    });

    expect(summary.dataset.ready).toBe(true);
    expect(summary.optimization.candidateTitle).toBe("Rank 1 · Sharpe 1.42");
    expect(summary.optimization.metricItems.map((item) => item.label)).toEqual([
      "Return",
      "Max DD",
      "Ranked",
    ]);
  });

  it("carries a basic validation execution status into the Simple summary", () => {
    const summary = buildSimpleRunSummary({
      datasetLabel: "AAPL sample",
      strategyKind: "DoubleMa",
      optimizationReady: true,
      validationReady: true,
      validationRows: 8,
      validationAccelerationStatus: "GPU validation path used for core checks.",
      validationVerdictTitle: "Validation passed.",
    });

    expect(summary.validation.accelerationLabel).toBe(
      "GPU validation path used for core checks.",
    );
    expect(summary.validation.metricItems).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          key: "validation_engine",
          label: "Engine",
          value: "GPU validation path used for core checks.",
        }),
      ]),
    );
  });

  it("marks validation as recommended after optimization and before validation", () => {
    expect(
      resolveSimpleValidationRecommendation({
        optimizationReady: true,
        validationReady: false,
      }),
    ).toMatchObject({
      kind: "recommended",
      tone: "ready",
      title: "Validation recommended.",
    });
  });

  it("distinguishes passed, review, rejected, and blocked validation recommendations", () => {
    expect(
      resolveSimpleValidationRecommendation({
        optimizationReady: false,
        validationReady: false,
      }).kind,
    ).toBe("blocked");
    expect(
      resolveSimpleValidationRecommendation({
        optimizationReady: true,
        validationReady: true,
        validationVerdictTitle: "Validation passed.",
        validationVerdictBody: "Holdout evidence is available.",
      }).kind,
    ).toBe("passed");
    expect(
      resolveSimpleValidationRecommendation({
        optimizationReady: true,
        validationReady: true,
        validationVerdictTitle: "Validation approved with warnings.",
        validationWarnings: "Fragile candidate.",
      }).kind,
    ).toBe("review");
    expect(
      resolveSimpleValidationRecommendation({
        optimizationReady: true,
        validationReady: true,
        validationVerdictTitle: "Validation rejected.",
      }).kind,
    ).toBe("rejected");
  });

  it("formats a Markdown summary for clipboard export", () => {
    const summary = buildSimpleRunSummary({
      datasetLabel: "AAPL sample",
      strategyKind: "DoubleMa",
      optimizationReady: true,
      selectedCandidateTitle: "Rank 1",
      topRows: 1,
    });

    const markdown = formatSimpleRunSummaryMarkdown(
      summary,
      new Date("2026-04-29T12:00:00.000Z"),
    );

    expect(markdown).toContain("# VectorGrid Simple Run Summary");
    expect(markdown).toContain("Generated: 2026-04-29T12:00:00.000Z");
    expect(markdown).toContain("- Dataset: AAPL sample");
    expect(markdown).toContain("- Best candidate: Rank 1");
    expect(markdown).toContain("- Engine: Not run");
    expect(markdown).toContain("not trading advice");
  });
});
