// @vitest-environment jsdom
import { describe, expect, it } from "vitest"
import {
  buildRouteProvenanceFromRun,
  buildRunCompareSummary,
  formatArtifactLineageCompact,
  formatRouteProvenanceCompact,
  type RunComparisonTarget,
} from "./runArtifactReview"

function makeTarget(
  overrides: Partial<RunComparisonTarget> = {},
): RunComparisonTarget {
  return {
    source: "saved",
    ts: "2026-04-11T10:00:00Z",
    strategyKind: "DoubleMa",
    csvPath: "C:/data/SPY.csv",
    timeframe: "15m",
    reportPath: "C:/runs/report.xlsx",
    hasValidation: true,
    configFingerprint: "cfg1-fnv1a64-alpha",
    datasetMeta: {
      timeframe: "15m",
      base_timeframe: "1m",
      derived_timeframe: "15m",
      resample_applied: true,
      source_content_fingerprint: "fnv1a64-full-dataset",
    },
    optimizationMetricSurface: {
      kind: "optimization",
      label: "trade22",
      rule: "default",
    },
    routeProvenance: {
      requestedRoute: "GpuOnly",
      resolvedRoute: "GpuKernel",
      realizedRoute: "CpuOnly",
      routeChanged: true,
      fallbackReason: "cuda unavailable",
    },
    lineage: {
      parentArtifactPath: "C:/runs/parent.json",
      parentArtifactKind: "vectorgrid_run_artifact",
      lineageRelation: "derived_from",
    },
    req: {
      strategy_kind: "DoubleMa",
      objective: "Sharpe",
    },
    res: {
      runtime_ms: 12,
      best_params: { fast_len: 10, slow_len: 40 },
      best_metrics: {
        pnl: 1.2,
        sharpe: 1.5,
        max_dd: 0.2,
        trades: 12,
        exposure: 0.4,
      },
      top: [],
    },
    ...overrides,
  }
}

describe("runArtifactReview", () => {
  it("builds compact route provenance from current run req/res", () => {
    const provenance = buildRouteProvenanceFromRun(
      { backend: { backend: "GpuOnly", device_id: 0 } },
      {
        backend_route: "GpuKernel -> CpuOnly",
        backend_used: "Cpu",
        backend_fallback_reason: "cuda unavailable",
      },
    )

    expect(provenance).toEqual({
      requestedRoute: "GpuOnly",
      resolvedRoute: "GpuKernel",
      realizedRoute: "CpuOnly",
      routeChanged: true,
      fallbackReason: "cuda unavailable",
    })
    expect(formatRouteProvenanceCompact(provenance)).toContain("requested=GpuOnly")
    expect(formatRouteProvenanceCompact(provenance)).toContain("actual=CpuOnly")
  })

  it("formats lineage compactly for compare surfaces", () => {
    expect(
      formatArtifactLineageCompact({
        parentArtifactPath: "C:/runs/parent.json",
        parentArtifactKind: "vectorgrid_run_artifact",
        lineageRelation: "compare_baseline",
      }),
    ).toBe("compare_baseline vectorgrid_run_artifact:parent.json")
  })

  it("builds compare summaries with dataset, route, lineage, and parameter deltas", () => {
    const baseline = makeTarget()
    const current = makeTarget({
      source: "current",
      ts: "2026-04-11T10:05:00Z",
      configFingerprint: "cfg1-fnv1a64-beta",
      routeProvenance: {
        requestedRoute: "GpuOnly",
        resolvedRoute: "GpuKernel",
        realizedRoute: "GpuKernel",
        routeChanged: false,
        fallbackReason: "",
      },
      req: {
        strategy_kind: "DoubleMa",
        objective: "Sharpe",
      },
      res: {
        runtime_ms: 8,
        best_params: { fast_len: 12, slow_len: 44 },
        best_metrics: {
          pnl: 1.35,
          sharpe: 1.7,
          max_dd: 0.18,
          trades: 14,
          exposure: 0.45,
        },
        top: [],
      },
    })

    const summary = buildRunCompareSummary(current, baseline)

    expect(summary).toContain("config: changed")
    expect(summary).toContain("dataset: same")
    expect(summary).toContain("route: changed")
    expect(summary).toContain("lineage: same")
    expect(summary).toContain("Optimization best:")
    expect(summary).toContain("Params diff (current - baseline):")
    expect(summary).toContain("fast_len")
    expect(summary).toContain("slow_len")
  })
})
