import { describe, expect, it } from "vitest";
import {
  buildOptimizationStageCompletion,
  buildOptionsExpressionValidationStageCompletion,
  buildPolicyEvaluationStageCompletion,
  buildStage5MlConfidenceStageCompletion,
  buildCampaignStageCompletion,
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

  it("builds additive packets for Stage 5 and policy evaluation artifacts", () => {
    const stage5 = buildStage5MlConfidenceStageCompletion({
      ts: "2026-04-28T12:00:00Z",
      res: {
        stage5_json_path: "C:/runs/stage5.json",
        strategy_kind: "DoubleMa",
        groups_selected: 4,
        groups_scored: 9,
        model_healthy: true,
        model_evidence_tier: "cross_validation",
        training_artifact_count: 4,
        warning_count: 1,
        min_ml_confidence: 0.72,
      },
      backfilled: true,
    });
    const policy = buildPolicyEvaluationStageCompletion({
      ts: "2026-04-28T12:01:00Z",
      res: {
        eval_json_path: "C:/runs/policy.json",
        artifacts_evaluated: 7,
        artifacts_considered: 8,
        best_policy: "stage5_blended",
        stage5_blended: { precision_at_k: 0.82 },
      },
    });

    expect(stage5).toMatchObject({
      stage: "stage5_ml_confidence",
      artifactKind: "vectorgrid_meta_stage5_ml_confidence",
      summary: {
        groupsSelected: 4,
        modelHealthy: true,
        modelEvidenceTier: "cross_validation",
        trainingArtifactCount: 4,
        warningCount: 1,
      },
      backfilled: true,
    });
    expect(policy).toMatchObject({
      stage: "policy_evaluation",
      artifactKind: "vectorgrid_ml_confidence_policy_evaluation",
      recommendation: "stage5_blended",
      summary: { artifactsEvaluated: 7, stage5PrecisionAtK: 0.82 },
    });
  });

  it("builds options validation and campaign packets with warnings preserved", () => {
    const optionsValidation = buildOptionsExpressionValidationStageCompletion({
      ts: "2026-04-28T12:00:00Z",
      res: {
        validation_artifact_path: "C:/runs/options-validation.json",
        root_symbol: "SPY",
        options_dataset_id: "dataset-a",
        acceptance_summary: {
          verdict: "accepted_with_warning",
          selected_tuple_count: 5,
          validated_tuple_count: 5,
          accepted_tuple_count: 2,
          support_deficient_tuple_count: 1,
          notes: ["stress lane thin"],
        },
      },
    });
    const campaign = buildCampaignStageCompletion({
      ts: "2026-04-28T12:02:00Z",
      res: {
        summary_path: "C:/runs/campaign.json",
        strategy_kind: "DoubleMa",
        validated_count: 10,
        failed_count: 0,
        skipped_count: 1,
      },
    });

    expect(optionsValidation).toMatchObject({
      stage: "options_expression_validation",
      verdict: "approved_with_warnings",
      strategyKind: "OptionsExpression",
      summary: {
        acceptedTupleCount: 2,
        supportDeficientTupleCount: 1,
        rootSymbol: "SPY",
      },
      caveats: ["stress lane thin"],
    });
    expect(optionsValidation.warnings?.[0]).toContain("support-deficient");
    expect(campaign).toMatchObject({
      stage: "campaign",
      status: "success",
      verdict: "review",
      summary: { validated: 10, skipped: 1 },
    });
  });
});
