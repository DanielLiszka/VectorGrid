import { invokeCommand } from "../tauri/invoke";
import {
  parseBoundaryArray,
  parseBoundaryObject,
} from "./serviceBoundarySchemas";

export type PortfolioEvidenceStatus =
  | "feasibility_only"
  | "insufficient_covariance_evidence"
  | "replay_required"
  | "replay_parity_passed"
  | "replay_parity_failed"
  | "artifact_provenance_blocked"
  | "unsupported_candidate_family";

export type PortfolioSourceRole =
  | "validation_artifact"
  | "meta_selection_artifact"
  | "options_expression_artifact"
  | "ml_signal_artifact"
  | "unknown";

export type PortfolioAssetClass = "equity" | "options" | "mixed" | "unknown";

export type PortfolioReplayEvidenceStatus =
  | "missing_replay_matrix"
  | "replay_required"
  | "replay_matrix_present"
  | "not_applicable";

export interface PortfolioFeasibilityRequest {
  sourceArtifacts?: unknown[];
  maxCandidates?: number;
  requireApprovedValidation?: boolean;
  requireReplayMatrix?: boolean;
  requireSourceProvenance?: boolean;
}

export interface PortfolioAdmissionEvidence {
  validationPassed: boolean | null;
  runApproved: boolean | null;
  finalRecommendation: string;
  psr: number | null;
  dsr: number | null;
  pbo: number | null;
  crossTickerScore: number | null;
  robustScore: number | null;
  optionsValidationVerdict: string;
  optionsNonPromotable: boolean | null;
  optionsStructureFamily: string;
}

export interface PortfolioSummaryMetrics {
  pnl: number | null;
  sharpe: number | null;
  sortino: number | null;
  calmar: number | null;
  omega: number | null;
  volatility: number | null;
  maxDrawdown: number | null;
  trades: number | null;
  exposure: number | null;
  netExposure: number | null;
}

export interface PortfolioCandidateEvidence {
  candidateId: string;
  sourceArtifactIndex: number;
  sourceRole: PortfolioSourceRole;
  strategyKind: string;
  symbolRoot: string;
  timeframe: string;
  paramsHash: string;
  assetClass: PortfolioAssetClass;
  admission: PortfolioAdmissionEvidence;
  summaryMetrics: PortfolioSummaryMetrics;
  replayStatus: PortfolioReplayEvidenceStatus;
}

export interface PortfolioRejectedCandidate {
  candidateId: string;
  sourceArtifactIndex: number;
  sourceRole: PortfolioSourceRole;
  reason: string;
}

export interface PortfolioMissingEvidence {
  sourceArtifactIndex: number | null;
  candidateId: string;
  evidenceKind: string;
  message: string;
}

export interface PortfolioFeasibilityResult {
  version: number;
  status: PortfolioEvidenceStatus;
  sourceArtifactsSeen: number;
  eligibleCandidates: PortfolioCandidateEvidence[];
  rejectedCandidates: PortfolioRejectedCandidate[];
  missingEvidence: PortfolioMissingEvidence[];
  warnings: string[];
  nextRequiredStep: string;
}

function text(value: unknown): string {
  return String(value || "").trim();
}

function integer(value: unknown, fallback = 0): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.max(0, Math.floor(parsed));
}

function maybeNumber(value: unknown): number | null {
  if (value == null) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function maybeBool(value: unknown): boolean | null {
  if (typeof value === "boolean") return value;
  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    if (normalized === "true") return true;
    if (normalized === "false") return false;
  }
  return null;
}

function normalizeStatus(value: unknown): PortfolioEvidenceStatus {
  const normalized = text(value);
  switch (normalized) {
    case "feasibility_only":
    case "insufficient_covariance_evidence":
    case "replay_required":
    case "replay_parity_passed":
    case "replay_parity_failed":
    case "artifact_provenance_blocked":
    case "unsupported_candidate_family":
      return normalized;
    default:
      return "insufficient_covariance_evidence";
  }
}

function normalizeSourceRole(value: unknown): PortfolioSourceRole {
  const normalized = text(value);
  switch (normalized) {
    case "validation_artifact":
    case "meta_selection_artifact":
    case "options_expression_artifact":
    case "ml_signal_artifact":
    case "unknown":
      return normalized;
    default:
      return "unknown";
  }
}

function normalizeAssetClass(value: unknown): PortfolioAssetClass {
  const normalized = text(value);
  switch (normalized) {
    case "equity":
    case "options":
    case "mixed":
    case "unknown":
      return normalized;
    default:
      return "unknown";
  }
}

function normalizeReplayStatus(value: unknown): PortfolioReplayEvidenceStatus {
  const normalized = text(value);
  switch (normalized) {
    case "missing_replay_matrix":
    case "replay_required":
    case "replay_matrix_present":
    case "not_applicable":
      return normalized;
    default:
      return "missing_replay_matrix";
  }
}

function normalizeAdmission(raw: unknown): PortfolioAdmissionEvidence {
  const row = parseBoundaryObject(raw);
  return {
    validationPassed: maybeBool(row.validation_passed),
    runApproved: maybeBool(row.run_approved),
    finalRecommendation: text(row.final_recommendation),
    psr: maybeNumber(row.psr),
    dsr: maybeNumber(row.dsr),
    pbo: maybeNumber(row.pbo),
    crossTickerScore: maybeNumber(row.cross_ticker_score),
    robustScore: maybeNumber(row.robust_score),
    optionsValidationVerdict: text(row.options_validation_verdict),
    optionsNonPromotable: maybeBool(row.options_non_promotable),
    optionsStructureFamily: text(row.options_structure_family),
  };
}

function normalizeMetrics(raw: unknown): PortfolioSummaryMetrics {
  const row = parseBoundaryObject(raw);
  return {
    pnl: maybeNumber(row.pnl),
    sharpe: maybeNumber(row.sharpe),
    sortino: maybeNumber(row.sortino),
    calmar: maybeNumber(row.calmar),
    omega: maybeNumber(row.omega),
    volatility: maybeNumber(row.volatility),
    maxDrawdown: maybeNumber(row.max_drawdown),
    trades: maybeNumber(row.trades),
    exposure: maybeNumber(row.exposure),
    netExposure: maybeNumber(row.net_exposure),
  };
}

function normalizeCandidate(raw: unknown): PortfolioCandidateEvidence {
  const row = parseBoundaryObject(raw);
  return {
    candidateId: text(row.candidate_id),
    sourceArtifactIndex: integer(row.source_artifact_index),
    sourceRole: normalizeSourceRole(row.source_role),
    strategyKind: text(row.strategy_kind),
    symbolRoot: text(row.symbol_root),
    timeframe: text(row.timeframe),
    paramsHash: text(row.params_hash),
    assetClass: normalizeAssetClass(row.asset_class),
    admission: normalizeAdmission(row.admission),
    summaryMetrics: normalizeMetrics(row.summary_metrics),
    replayStatus: normalizeReplayStatus(row.replay_status),
  };
}

function normalizeRejectedCandidate(raw: unknown): PortfolioRejectedCandidate {
  const row = parseBoundaryObject(raw);
  return {
    candidateId: text(row.candidate_id),
    sourceArtifactIndex: integer(row.source_artifact_index),
    sourceRole: normalizeSourceRole(row.source_role),
    reason: text(row.reason),
  };
}

function normalizeMissingEvidence(raw: unknown): PortfolioMissingEvidence {
  const row = parseBoundaryObject(raw);
  const index =
    row.source_artifact_index == null ? null : integer(row.source_artifact_index);
  return {
    sourceArtifactIndex: index,
    candidateId: text(row.candidate_id),
    evidenceKind: text(row.evidence_kind),
    message: text(row.message),
  };
}

export function normalizePortfolioFeasibilityResult(
  raw: unknown,
): PortfolioFeasibilityResult {
  const row = parseBoundaryObject(raw);
  return {
    version: integer(row.version, 1),
    status: normalizeStatus(row.status),
    sourceArtifactsSeen: integer(row.source_artifacts_seen),
    eligibleCandidates: parseBoundaryArray(row.eligible_candidates).map(
      normalizeCandidate,
    ),
    rejectedCandidates: parseBoundaryArray(row.rejected_candidates).map(
      normalizeRejectedCandidate,
    ),
    missingEvidence: parseBoundaryArray(row.missing_evidence).map(
      normalizeMissingEvidence,
    ),
    warnings: parseBoundaryArray(row.warnings).map(text).filter(Boolean),
    nextRequiredStep: text(row.next_required_step),
  };
}

function normalizeRequest(req: PortfolioFeasibilityRequest): Record<string, unknown> {
  return {
    source_artifacts: Array.isArray(req.sourceArtifacts) ? req.sourceArtifacts : [],
    max_candidates:
      req.maxCandidates == null ? undefined : integer(req.maxCandidates, 0),
    require_approved_validation: Boolean(req.requireApprovedValidation),
    require_replay_matrix: Boolean(req.requireReplayMatrix),
    require_source_provenance: Boolean(req.requireSourceProvenance),
  };
}

export async function analyzePortfolioFeasibility(
  req: PortfolioFeasibilityRequest,
): Promise<PortfolioFeasibilityResult> {
  const raw = await invokeCommand("analyze_portfolio_feasibility", {
    req: normalizeRequest(req),
  });
  return normalizePortfolioFeasibilityResult(raw);
}

