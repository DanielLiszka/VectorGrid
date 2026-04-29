import { describe, expect, it } from "vitest";
import {
  buildOptimizationStageCompletion,
  buildValidationStageCompletion,
  ensureDerivedStageCompletions,
  normalizeStageCompletions,
  serializeStageCompletionsForArtifact,
} from "./stageCompletion";

describe("stageCompletion", () => {
  it("normalizes snake_case packets to the TypeScript shape", () => {
    const packets = normalizeStageCompletions([
      {
        schema_version: 1,
        stage: "validation",
        status: "success",
        verdict: "approved_with_warnings",
        completed_at: "2026-04-28T12:00:00Z",
        report_xlsx_path: "C:/runs/report.xlsx",
        backend_used: "GpuHybridKernel",
        next_actions: ["Review warning flags."],
        warnings: ["spa skipped"],
        backfilled: true,
      },
    ]);

    expect(packets).toEqual([
      expect.objectContaining({
        schemaVersion: 1,
        stage: "validation",
        verdict: "approved_with_warnings",
        completedAt: "2026-04-28T12:00:00Z",
        reportXlsxPath: "C:/runs/report.xlsx",
        backendUsed: "GpuHybridKernel",
        nextActions: ["Review warning flags."],
        warnings: ["spa skipped"],
        backfilled: true,
      }),
    ]);
  });

  it("derives optimization and validation packets from legacy run payloads", () => {
    const packets = ensureDerivedStageCompletions([], {
      ts: "2026-04-28T12:00:00Z",
      artifactPath: "C:/runs/run.json",
      csvPath: "C:/data/SPY.csv",
      datasetMeta: { timeframe: "15m" },
      req: { strategy_kind: "DoubleMa", objective: "Sharpe" },
      res: {
        top: [{ params: {} }, { params: {} }],
        backend_used: "Gpu",
      },
      validationReq: { strategy_kind: "DoubleMa" },
      validationRes: {
        candidates: [{}, {}],
        run_verdict: {
          approved: true,
          passed: true,
          approved_candidate_count: 1,
          warning_flags: ["pbo elevated"],
          final_recommendation: "review_before_live",
        },
      },
    });

    expect(packets.map((packet) => packet.stage)).toEqual([
      "optimization",
      "validation",
    ]);
    expect(packets[0]).toMatchObject({
      stage: "optimization",
      backfilled: true,
      summary: { rankedCandidates: 2, objective: "Sharpe" },
    });
    expect(packets[1]).toMatchObject({
      stage: "validation",
      verdict: "approved_with_warnings",
      recommendation: "review_before_live",
      warnings: ["pbo elevated"],
      backfilled: true,
    });
  });

  it("serializes packets to the artifact snake_case contract", () => {
    const packet = buildOptimizationStageCompletion({
      ts: "2026-04-28T12:00:00Z",
      durationMs: 42,
      req: { strategy_kind: "CompositeTrend" },
      res: { backend_route: "GpuKernel" },
      configFingerprint: "cfg1-test",
    });

    expect(serializeStageCompletionsForArtifact([packet])).toEqual([
      expect.objectContaining({
        schema_version: 1,
        stage: "optimization",
        completed_at: "2026-04-28T12:00:00Z",
        duration_ms: 42,
        strategy_kind: "CompositeTrend",
        config_fingerprint: "cfg1-test",
        route: "GpuKernel",
      }),
    ]);
  });

  it("maps failed validation verdict evidence to a rejected packet", () => {
    const packet = buildValidationStageCompletion({
      ts: "2026-04-28T12:00:00Z",
      validationReq: { strategy_kind: "DoubleMa" },
      validationRes: {
        run_verdict: {
          passed: false,
          approved: false,
          fail_reasons: ["min_psr"],
        },
      },
    });

    expect(packet).toMatchObject({
      stage: "validation",
      verdict: "rejected",
      blockers: ["min_psr"],
    });
  });

  it("backfills ML Signal packet when a run artifact carries ml_signal stage metadata", () => {
    const packets = ensureDerivedStageCompletions([], {
      ts: "2026-04-28T12:00:00Z",
      mlSignal: {
        stage: "score_cache",
        bundle_id: "bundle-a",
        native_backend_used: "native_cpu",
      },
    });

    expect(packets).toEqual([
      expect.objectContaining({
        stage: "ml_signal",
        substage: "score_cache",
        verdict: "inspect_only",
        backendUsed: "native_cpu",
        summary: { bundleId: "bundle-a" },
        backfilled: true,
      }),
    ]);
  });
});
