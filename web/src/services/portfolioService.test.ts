import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCommand } from "../tauri/invoke";
import {
  analyzePortfolioFeasibility,
  normalizePortfolioFeasibilityResult,
} from "./portfolioService";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => ({})),
}));

const invokeCommandMock = vi.mocked(invokeCommand);

describe("portfolioService", () => {
  beforeEach(() => {
    invokeCommandMock.mockReset();
  });

  it("normalizes insufficient covariance evidence as a diagnostic state", () => {
    const result = normalizePortfolioFeasibilityResult({
      version: 1,
      status: "insufficient_covariance_evidence",
      source_artifacts_seen: 1,
      eligible_candidates: [
        {
          candidate_id: "candidate-1",
          source_artifact_index: 0,
          source_role: "validation_artifact",
          strategy_kind: "double_ma",
          symbol_root: "AAPL",
          timeframe: "1m",
          params_hash: "hash",
          asset_class: "equity",
          admission: {
            validation_passed: true,
            psr: 0.91,
          },
          summary_metrics: {
            sharpe: 1.2,
            trades: 12,
          },
          replay_status: "missing_replay_matrix",
        },
      ],
      rejected_candidates: [],
      missing_evidence: [
        {
          source_artifact_index: 0,
          candidate_id: "candidate-1",
          evidence_kind: "co_timed_replay_matrix",
          message: "missing replay",
        },
      ],
      warnings: ["research only"],
      next_required_step: "Build replay first.",
    });

    expect(result.status).toBe("insufficient_covariance_evidence");
    expect(result.eligibleCandidates).toHaveLength(1);
    expect(result.eligibleCandidates[0]?.summaryMetrics.sharpe).toBe(1.2);
    expect(result.missingEvidence[0]?.evidenceKind).toBe(
      "co_timed_replay_matrix",
    );
    expect(JSON.stringify(result)).not.toContain("targetWeights");
    expect(JSON.stringify(result)).not.toContain("allocationFrontier");
  });

  it("falls back safely for unknown and partial payloads", () => {
    const result = normalizePortfolioFeasibilityResult({
      status: "unexpected",
      eligible_candidates: [{ source_role: "not-real", asset_class: "nope" }],
      missing_evidence: "not-array",
    });

    expect(result.status).toBe("insufficient_covariance_evidence");
    expect(result.eligibleCandidates[0]?.sourceRole).toBe("unknown");
    expect(result.eligibleCandidates[0]?.assetClass).toBe("unknown");
    expect(result.missingEvidence).toEqual([]);
  });

  it("invokes only the read-only portfolio feasibility command", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      version: 1,
      status: "insufficient_covariance_evidence",
      source_artifacts_seen: 1,
      eligible_candidates: [],
      rejected_candidates: [],
      missing_evidence: [],
      warnings: [],
      next_required_step: "Provide replay evidence.",
    });

    const result = await analyzePortfolioFeasibility({
      sourceArtifacts: [{ artifact_kind: "validation" }],
      maxCandidates: 25,
      requireApprovedValidation: true,
      requireReplayMatrix: true,
      requireSourceProvenance: true,
    });

    expect(result.status).toBe("insufficient_covariance_evidence");
    expect(invokeCommandMock).toHaveBeenCalledTimes(1);
    expect(invokeCommandMock).toHaveBeenCalledWith(
      "analyze_portfolio_feasibility",
      {
        req: {
          source_artifacts: [{ artifact_kind: "validation" }],
          max_candidates: 25,
          require_approved_validation: true,
          require_replay_matrix: true,
          require_source_provenance: true,
        },
      },
    );
    expect(invokeCommandMock).not.toHaveBeenCalledWith(
      "execute_strategy_signal_once",
      expect.anything(),
    );
  });
});

