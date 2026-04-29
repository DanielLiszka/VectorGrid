import { describe, expect, it } from "vitest";
import type { HistorySavedRunDetail } from "../services/historyService";
import type {
  WorkflowRunArtifact,
  WorkflowValidationArtifact,
} from "../stores/workflowArtifactsStore";
import {
  buildCurrentRunPortfolioSource,
  buildCurrentValidationPortfolioSource,
  buildPortfolioConstructionReadiness,
  buildPortfolioFeasibilityCsv,
  buildPortfolioFeasibilityExportPayload,
  buildSavedRunPortfolioSource,
  formatPortfolioEvidenceStatus,
  formatPortfolioNumber,
  formatPortfolioReplayStatus,
} from "./portfolioWorkspaceModel";

function runArtifact(): WorkflowRunArtifact {
  return {
    ts: "2026-04-29T12:00:00Z",
    artifactPath: "runs/run.json",
    reportXlsxPath: "runs/report.xlsx",
    reportXlsxExists: true,
    csvPath: "data/SPY.csv",
    configFingerprint: "cfg-1",
    configFingerprintVersion: 1,
    datasetMeta: { dataset_hash: "data-hash-1" },
    routeProvenance: null,
    lineage: null,
    mlSignal: null,
    optimizationMetricSurface: null,
    stageCompletions: [],
    tags: [],
    req: { strategy_kind: "DoubleMa", timeframe: "5m" },
    res: {
      top: [
        {
          candidate_id: "top-1",
          validation_passed: true,
          metrics: { sharpe: 1.2 },
        },
      ],
    },
  };
}

describe("portfolioWorkspaceModel", () => {
  it("packages current optimization rows as read-only feasibility sources", () => {
    const source = buildCurrentRunPortfolioSource(runArtifact());

    expect(source?.kind).toBe("current_run");
    expect(source?.artifact.artifact_kind).toBe("validation_artifact_bundle");
    expect(source?.artifact.config_fingerprint).toBe("cfg-1");
    expect(source?.artifact.dataset_hash).toBe("data-hash-1");
    expect(source?.artifact.top).toEqual([
      {
        candidate_id: "top-1",
        validation_passed: true,
        metrics: { sharpe: 1.2 },
      },
    ]);
    expect(JSON.stringify(source?.artifact)).not.toContain("targetWeights");
  });

  it("packages validation rows with run provenance", () => {
    const validation: WorkflowValidationArtifact = {
      ts: "2026-04-29T12:05:00Z",
      req: { cscv_slices: 16 },
      res: {
        candidates: [{ candidate_id: "validated-1", validation_passed: true }],
      },
    };

    const source = buildCurrentValidationPortfolioSource(
      runArtifact(),
      validation,
    );

    expect(source?.kind).toBe("current_validation");
    expect(source?.artifact.artifact_kind).toBe("validation_artifact");
    expect(source?.artifact.validation_req).toEqual({ cscv_slices: 16 });
    expect(source?.artifact.candidates).toEqual([
      { candidate_id: "validated-1", validation_passed: true },
    ]);
    expect(source?.artifact.config_fingerprint).toBe("cfg-1");
  });

  it("hydrates saved run details without dropping validation payloads", () => {
    const detail: HistorySavedRunDetail = {
      ts: "2026-04-29T12:00:00Z",
      artifactPath: "runs/saved.json",
      strategyKind: "CompositeCpu",
      configFingerprint: "cfg-saved",
      timeframe: "1m",
      csvPath: "data/AAPL.csv",
      reportPath: "runs/report.xlsx",
      paramsText: "",
      hasValidation: true,
      datasetMeta: { nested: { sha256: "nested-hash" } },
      routeProvenance: null,
      lineage: null,
      optimizationMetricSurface: null,
      stageCompletions: [],
      runReq: { strategy_kind: "CompositeCpu" },
      runRes: { top: [{ candidate_id: "run-top" }] },
      validationReq: { min_psr: 0.8 },
      validationRes: { candidates: [{ candidate_id: "validated" }] },
      tags: [],
    };

    const source = buildSavedRunPortfolioSource(detail);

    expect(source?.kind).toBe("saved_run");
    expect(source?.artifact.dataset_hash).toBe("nested-hash");
    expect(source?.artifact.validation_req).toEqual({ min_psr: 0.8 });
    expect(source?.artifact.candidates).toEqual([
      { candidate_id: "validated" },
    ]);
    expect(source?.artifact.top).toEqual([{ candidate_id: "run-top" }]);
  });

  it("formats portfolio feasibility statuses for compact UI surfaces", () => {
    expect(formatPortfolioEvidenceStatus("replay_required")).toBe(
      "Replay required",
    );
    expect(formatPortfolioReplayStatus("missing_replay_matrix")).toBe(
      "Missing replay matrix",
    );
    expect(formatPortfolioNumber(1.23456, 2)).toBe("1.23");
    expect(formatPortfolioNumber(null)).toBe("-");
  });

  it("builds compact research-only export payloads and CSV without raw artifacts or allocation fields", () => {
    const source = buildCurrentRunPortfolioSource(runArtifact());
    expect(source).not.toBeNull();
    const result = {
      version: 1,
      status: "insufficient_covariance_evidence" as const,
      sourceArtifactsSeen: 1,
      eligibleCandidates: [
        {
          candidateId: "top-1",
          sourceArtifactIndex: 0,
          sourceRole: "validation_artifact" as const,
          strategyKind: "DoubleMa",
          symbolRoot: "SPY",
          timeframe: "5m",
          paramsHash: "params-1",
          assetClass: "equity" as const,
          admission: {
            validationPassed: true,
            runApproved: true,
            finalRecommendation: "approve",
            psr: 0.92,
            dsr: null,
            pbo: null,
            crossTickerScore: null,
            robustScore: null,
            optionsValidationVerdict: "",
            optionsNonPromotable: null,
            optionsStructureFamily: "",
          },
          summaryMetrics: {
            pnl: null,
            sharpe: 1.4,
            sortino: null,
            calmar: null,
            omega: null,
            volatility: null,
            maxDrawdown: null,
            trades: 20,
            exposure: null,
            netExposure: null,
          },
          replayStatus: "missing_replay_matrix" as const,
        },
      ],
      rejectedCandidates: [],
      missingEvidence: [],
      warnings: [],
      nextRequiredStep: "Collect covariance evidence.",
    };

    const payload = buildPortfolioFeasibilityExportPayload(
      result,
      [source!],
      { maxCandidates: 50, requireApprovedValidation: true },
      123,
    );
    const serialized = JSON.stringify(payload);

    expect(payload.research_only).toBe(true);
    expect(payload.sources[0]).toMatchObject({
      artifact_path: "runs/run.json",
      source_hash: "cfg-1",
      dataset_hash: "data-hash-1",
    });
    expect(serialized).not.toContain("run_req");
    expect(serialized).not.toContain("run_res");
    expect(serialized).not.toContain("target_weights");
    expect(serialized).not.toContain("portfolio_target");

    const csv = buildPortfolioFeasibilityCsv(result, [source!], {
      maxCandidates: 50,
    });
    expect(csv).toContain("row_type,source_index,candidate_id");
    expect(csv).toContain("eligible,0,top-1");
    expect(csv).toContain("Collect covariance evidence.");
  });

  it("keeps construction readiness diagnostic-only", () => {
    expect(buildPortfolioConstructionReadiness(null).status).toBe("blocked");
    const readiness = buildPortfolioConstructionReadiness({
      version: 1,
      status: "replay_required",
      sourceArtifactsSeen: 1,
      eligibleCandidates: [
        {
          candidateId: "top-1",
          sourceArtifactIndex: 0,
          sourceRole: "validation_artifact",
          strategyKind: "DoubleMa",
          symbolRoot: "SPY",
          timeframe: "5m",
          paramsHash: "",
          assetClass: "equity",
          admission: {
            validationPassed: true,
            runApproved: true,
            finalRecommendation: "",
            psr: null,
            dsr: null,
            pbo: null,
            crossTickerScore: null,
            robustScore: null,
            optionsValidationVerdict: "",
            optionsNonPromotable: null,
            optionsStructureFamily: "",
          },
          summaryMetrics: {
            pnl: null,
            sharpe: null,
            sortino: null,
            calmar: null,
            omega: null,
            volatility: null,
            maxDrawdown: null,
            trades: null,
            exposure: null,
            netExposure: null,
          },
          replayStatus: "missing_replay_matrix",
        },
      ],
      rejectedCandidates: [],
      missingEvidence: [],
      warnings: [],
      nextRequiredStep: "Replay required.",
    });

    expect(readiness.status).toBe("needs_replay_matrix");
    expect(
      readiness.gates.find((gate) => gate.key === "execution_isolation")?.tone,
    ).toBe("blocked");
  });
});
