import { invokeCommand } from "../tauri/invoke";
import {
  requireBoundaryArray,
  requireBoundaryObject,
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

function requiredInteger(value: unknown, field: string): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    throw new Error(`Portfolio feasibility response field '${field}' must be a finite number.`);
  }
  return Math.max(0, Math.floor(parsed));
}

function requiredText(value: unknown, field: string): string {
  const normalized = text(value);
  if (!normalized) {
    throw new Error(`Portfolio feasibility response field '${field}' must be a non-empty string.`);
  }
  return normalized;
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
      throw new Error(`Unknown portfolio feasibility status: '${normalized}'.`);
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
      throw new Error(`Unknown portfolio source role: '${normalized}'.`);
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
      throw new Error(`Unknown portfolio asset class: '${normalized}'.`);
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
      throw new Error(`Unknown portfolio replay status: '${normalized}'.`);
  }
}

function normalizeAdmission(raw: unknown): PortfolioAdmissionEvidence {
  const row = requireBoundaryObject(raw, "portfolio admission evidence");
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
  const row = requireBoundaryObject(raw, "portfolio summary metrics");
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
  const row = requireBoundaryObject(raw, "portfolio candidate evidence");
  return {
    candidateId: requiredText(row.candidate_id, "candidate_id"),
    sourceArtifactIndex: requiredInteger(row.source_artifact_index, "source_artifact_index"),
    sourceRole: normalizeSourceRole(row.source_role),
    strategyKind: requiredText(row.strategy_kind, "strategy_kind"),
    symbolRoot: requiredText(row.symbol_root, "symbol_root"),
    timeframe: requiredText(row.timeframe, "timeframe"),
    paramsHash: requiredText(row.params_hash, "params_hash"),
    assetClass: normalizeAssetClass(row.asset_class),
    admission: normalizeAdmission(row.admission),
    summaryMetrics: normalizeMetrics(row.summary_metrics),
    replayStatus: normalizeReplayStatus(row.replay_status),
  };
}

function normalizeRejectedCandidate(raw: unknown): PortfolioRejectedCandidate {
  const row = requireBoundaryObject(raw, "portfolio rejected candidate");
  return {
    candidateId: requiredText(row.candidate_id, "candidate_id"),
    sourceArtifactIndex: requiredInteger(row.source_artifact_index, "source_artifact_index"),
    sourceRole: normalizeSourceRole(row.source_role),
    reason: requiredText(row.reason, "reason"),
  };
}

function normalizeMissingEvidence(raw: unknown): PortfolioMissingEvidence {
  const row = requireBoundaryObject(raw, "portfolio missing evidence");
  const index =
    row.source_artifact_index == null ? null : integer(row.source_artifact_index);
  return {
    sourceArtifactIndex: index,
    candidateId: requiredText(row.candidate_id, "candidate_id"),
    evidenceKind: requiredText(row.evidence_kind, "evidence_kind"),
    message: requiredText(row.message, "message"),
  };
}

export function normalizePortfolioFeasibilityResult(
  raw: unknown,
): PortfolioFeasibilityResult {
  const row = requireBoundaryObject(raw, "portfolio feasibility response");
  return {
    version: requiredInteger(row.version, "version"),
    status: normalizeStatus(row.status),
    sourceArtifactsSeen: requiredInteger(
      row.source_artifacts_seen,
      "source_artifacts_seen",
    ),
    eligibleCandidates: requireBoundaryArray(
      row.eligible_candidates,
      "portfolio eligible_candidates",
    ).map(normalizeCandidate),
    rejectedCandidates: requireBoundaryArray(
      row.rejected_candidates,
      "portfolio rejected_candidates",
    ).map(normalizeRejectedCandidate),
    missingEvidence: requireBoundaryArray(
      row.missing_evidence,
      "portfolio missing_evidence",
    ).map(normalizeMissingEvidence),
    warnings: requireBoundaryArray(row.warnings, "portfolio warnings")
      .map(text)
      .filter(Boolean),
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
