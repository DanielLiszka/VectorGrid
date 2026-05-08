import type { HistorySavedRunDetail } from "../services/historyService";
import type {
  WorkflowRunArtifact,
  WorkflowValidationArtifact,
} from "../stores/workflowArtifactsStore";
import type {
  PortfolioEvidenceStatus,
  PortfolioFeasibilityRequest,
  PortfolioFeasibilityResult,
  PortfolioReplayEvidenceStatus,
} from "../services/portfolioService";

export type PortfolioWorkspaceSourceKind =
  | "current_run"
  | "current_validation"
  | "saved_run";

export interface PortfolioWorkspaceSource {
  id: string;
  kind: PortfolioWorkspaceSourceKind;
  label: string;
  detail: string;
  artifact: Record<string, unknown>;
}

export interface PortfolioFeasibilityExportPayload {
  schema_version: 1;
  generated_at_unix_ms: number;
  research_only: true;
  sources: PortfolioWorkspaceSourceSummary[];
  request_gates: {
    max_candidates: number;
    require_approved_validation: boolean;
    require_replay_matrix: boolean;
    require_source_provenance: boolean;
  };
  result: PortfolioFeasibilityResult;
  next_required_step: string;
}

export interface PortfolioWorkspaceSourceSummary {
  id: string;
  kind: PortfolioWorkspaceSourceKind;
  label: string;
  detail: string;
  artifact_kind: string;
  source_kind: string;
  artifact_path: string;
  strategy_kind: string;
  timeframe: string;
  source_hash: string;
  dataset_hash: string;
}

export type PortfolioConstructionReadinessStatus =
  | "blocked"
  | "needs_replay_matrix"
  | "ready_for_covariance_diagnostics";

export interface PortfolioConstructionReadinessGate {
  key: string;
  label: string;
  status: string;
  detail: string;
  tone: "pass" | "warn" | "blocked";
}

export interface PortfolioConstructionReadiness {
  status: PortfolioConstructionReadinessStatus;
  label: string;
  gates: PortfolioConstructionReadinessGate[];
}

function text(value: unknown): string {
  return String(value || "").trim();
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  return value as Record<string, unknown>;
}

function hasKeys(
  value: Record<string, unknown> | null,
): value is Record<string, unknown> {
  return !!value && Object.keys(value).length > 0;
}

function firstText(
  row: Record<string, unknown> | null,
  keys: readonly string[],
): string {
  if (!row) return "";
  for (const key of keys) {
    const value = text(row[key]);
    if (value) return value;
  }
  return "";
}

function firstNestedText(value: unknown, keys: readonly string[]): string {
  if (!value || typeof value !== "object") return "";
  if (Array.isArray(value)) {
    for (const item of value) {
      const found = firstNestedText(item, keys);
      if (found) return found;
    }
    return "";
  }
  const row = value as Record<string, unknown>;
  const direct = firstText(row, keys);
  if (direct) return direct;
  for (const key in row) {
    const found = firstNestedText(row[key], keys);
    if (found) return found;
  }
  return "";
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function candidateArrayEntries(
  row: Record<string, unknown> | null,
): Array<[string, unknown]> {
  if (!row) return [];
  const out: Array<[string, unknown]> = [];
  for (const key of [
    "top",
    "candidates",
    "finalists",
    "shortlist",
    "results",
  ]) {
    const items = asArray(row[key]);
    if (items.length > 0) out.push([key, items]);
  }
  return out;
}

function compactRecord(
  entries: Array<[string, unknown]>,
): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [key, value] of entries) {
    if (value == null) continue;
    if (typeof value === "string" && !value.trim()) continue;
    if (Array.isArray(value) && value.length === 0) continue;
    if (
      typeof value === "object" &&
      !Array.isArray(value) &&
      Object.keys(value as Record<string, unknown>).length === 0
    ) {
      continue;
    }
    out[key] = value;
  }
  return out;
}

function sourceHashFromFingerprint(value: unknown): string {
  const fingerprint = text(value);
  return fingerprint ? fingerprint : "";
}

function sourceLabel(parts: unknown[]): string {
  return parts.map(text).filter(Boolean).join(" | ");
}

function inferStrategyKind(
  req: Record<string, unknown> | null,
  res: Record<string, unknown> | null,
  fallback = "",
): string {
  return (
    firstText(req, ["strategy_kind", "strategyKind", "strategy"]) ||
    firstText(res, ["strategy_kind", "strategyKind", "strategy"]) ||
    fallback
  );
}

function inferTimeframe(
  req: Record<string, unknown> | null,
  res: Record<string, unknown> | null,
  fallback = "",
): string {
  return (
    firstText(req, [
      "timeframe",
      "bar_timeframe",
      "base_timeframe",
      "derived_timeframe",
    ]) ||
    firstText(res, [
      "timeframe",
      "bar_timeframe",
      "base_timeframe",
      "derived_timeframe",
    ]) ||
    fallback
  );
}

function datasetHash(
  datasetMeta: Record<string, unknown> | null | undefined,
): string {
  return firstNestedText(datasetMeta, [
    "dataset_hash",
    "datasetHash",
    "source_content_fingerprint",
    "sourceContentFingerprint",
    "content_hash",
    "contentHash",
    "sha256",
  ]);
}

export function buildCurrentRunPortfolioSource(
  run: WorkflowRunArtifact | null,
): PortfolioWorkspaceSource | null {
  if (!run) return null;
  const runReq = asRecord(run.req);
  const runRes = asRecord(run.res);
  const strategy = inferStrategyKind(runReq, runRes);
  const timeframe = inferTimeframe(runReq, runRes);
  const label =
    sourceLabel(["Current run", strategy, timeframe]) || "Current run";
  const artifact = compactRecord([
    ["artifact_kind", "validation_artifact_bundle"],
    ["source_kind", "current_run"],
    ["source_label", label],
    ["artifact_path", run.artifactPath],
    ["csv_path", run.csvPath],
    ["config_fingerprint", run.configFingerprint],
    ["source_hash", sourceHashFromFingerprint(run.configFingerprint)],
    ["dataset_hash", datasetHash(run.datasetMeta)],
    ["strategy_kind", strategy],
    ["timeframe", timeframe],
    ["dataset_meta", run.datasetMeta],
    ["route_provenance", run.routeProvenance],
    ["lineage", run.lineage],
    ["stage_completions", run.stageCompletions],
    ["run_req", runReq],
    ["run_res", runRes],
    ...candidateArrayEntries(runRes),
  ]);
  return {
    id: `current-run:${text(run.artifactPath) || text(run.ts) || "latest"}`,
    kind: "current_run",
    label,
    detail: sourceLabel([run.ts, run.artifactPath]),
    artifact,
  };
}

export function buildCurrentValidationPortfolioSource(
  run: WorkflowRunArtifact | null,
  validation: WorkflowValidationArtifact | null,
): PortfolioWorkspaceSource | null {
  if (!validation) return null;
  const validationReq = asRecord(validation.req);
  const validationRes = asRecord(validation.res);
  const runReq = asRecord(run?.req);
  const runRes = asRecord(run?.res);
  const strategy = inferStrategyKind(
    validationReq,
    validationRes,
    inferStrategyKind(runReq, runRes),
  );
  const timeframe = inferTimeframe(
    validationReq,
    validationRes,
    inferTimeframe(runReq, runRes),
  );
  const label =
    sourceLabel(["Current validation", strategy, timeframe]) ||
    "Current validation";
  const artifact = compactRecord([
    ["artifact_kind", "validation_artifact"],
    ["source_kind", "current_validation"],
    ["source_label", label],
    ["artifact_path", run?.artifactPath],
    ["csv_path", run?.csvPath],
    ["config_fingerprint", run?.configFingerprint],
    ["source_hash", sourceHashFromFingerprint(run?.configFingerprint)],
    ["dataset_hash", datasetHash(run?.datasetMeta)],
    ["strategy_kind", strategy],
    ["timeframe", timeframe],
    ["dataset_meta", run?.datasetMeta],
    ["route_provenance", run?.routeProvenance],
    ["lineage", run?.lineage],
    ["stage_completions", run?.stageCompletions],
    ["run_req", runReq],
    ["run_res", runRes],
    ["validation_req", validationReq],
    ["validation_res", validationRes],
    ...candidateArrayEntries(validationRes),
  ]);
  return {
    id: `current-validation:${text(run?.artifactPath) || text(validation.ts) || "latest"}`,
    kind: "current_validation",
    label,
    detail: sourceLabel([validation.ts, run?.artifactPath]),
    artifact,
  };
}

export function buildSavedRunPortfolioSource(
  detail: HistorySavedRunDetail,
): PortfolioWorkspaceSource | null {
  const runReq = asRecord(detail.runReq);
  const runRes = asRecord(detail.runRes);
  const validationReq = asRecord(detail.validationReq);
  const validationRes = asRecord(detail.validationRes);
  const strategy = detail.strategyKind || inferStrategyKind(runReq, runRes);
  const timeframe = detail.timeframe || inferTimeframe(runReq, runRes);
  const label =
    sourceLabel(["Saved run", detail.ts, strategy, timeframe]) || "Saved run";
  const artifactPath = text(detail.artifactPath);
  const artifact = compactRecord([
    ["artifact_kind", "validation_artifact_bundle"],
    ["source_kind", "saved_run"],
    ["source_label", label],
    ["artifact_path", artifactPath],
    ["csv_path", detail.csvPath],
    ["report_xlsx_path", detail.reportPath],
    ["config_fingerprint", detail.configFingerprint],
    ["source_hash", sourceHashFromFingerprint(detail.configFingerprint)],
    ["dataset_hash", datasetHash(detail.datasetMeta)],
    ["strategy_kind", strategy],
    ["timeframe", timeframe],
    ["dataset_meta", detail.datasetMeta],
    ["route_provenance", detail.routeProvenance],
    ["lineage", detail.lineage],
    ["stage_completions", detail.stageCompletions],
    ["run_req", runReq],
    ["run_res", runRes],
    ["validation_req", validationReq],
    ["validation_res", validationRes],
    ...candidateArrayEntries(runRes),
    ...candidateArrayEntries(validationRes),
  ]);
  if (!artifactPath && !hasKeys(runRes) && !hasKeys(validationRes)) return null;
  return {
    id: `saved-run:${artifactPath || text(detail.ts) || "unknown"}`,
    kind: "saved_run",
    label,
    detail: sourceLabel([artifactPath, detail.csvPath]),
    artifact,
  };
}

export function formatPortfolioEvidenceStatus(
  status: PortfolioEvidenceStatus,
): string {
  switch (status) {
    case "feasibility_only":
      return "Feasibility only";
    case "insufficient_covariance_evidence":
      return "Covariance evidence needed";
    case "replay_required":
      return "Replay required";
    case "replay_parity_passed":
      return "Replay parity passed";
    case "replay_parity_failed":
      return "Replay parity failed";
    case "artifact_provenance_blocked":
      return "Provenance blocked";
    case "unsupported_candidate_family":
      return "Unsupported family";
  }
}

export function formatPortfolioReplayStatus(
  status: PortfolioReplayEvidenceStatus,
): string {
  switch (status) {
    case "missing_replay_matrix":
      return "Missing replay matrix";
    case "replay_required":
      return "Replay required";
    case "replay_matrix_present":
      return "Replay matrix present";
    case "not_applicable":
      return "Not applicable";
  }
}

export function formatPortfolioNumber(
  value: number | null | undefined,
  digits = 3,
): string {
  if (value == null || !Number.isFinite(value)) return "-";
  return Number(value).toFixed(digits);
}

function compactSourceSummary(
  source: PortfolioWorkspaceSource,
): PortfolioWorkspaceSourceSummary {
  const artifact = asRecord(source.artifact) || {};
  return {
    id: source.id,
    kind: source.kind,
    label: source.label,
    detail: source.detail,
    artifact_kind: text(artifact.artifact_kind),
    source_kind: text(artifact.source_kind),
    artifact_path: text(artifact.artifact_path),
    strategy_kind: text(artifact.strategy_kind),
    timeframe: text(artifact.timeframe),
    source_hash:
      text(artifact.source_hash) || text(artifact.config_fingerprint),
    dataset_hash: text(artifact.dataset_hash),
  };
}

function requestGateSnapshot(
  req: PortfolioFeasibilityRequest,
): PortfolioFeasibilityExportPayload["request_gates"] {
  const parsedMax = Number(req.maxCandidates);
  return {
    max_candidates:
      Number.isFinite(parsedMax) && parsedMax > 0
        ? Math.min(250, Math.floor(parsedMax))
        : 50,
    require_approved_validation: Boolean(req.requireApprovedValidation),
    require_replay_matrix: Boolean(req.requireReplayMatrix),
    require_source_provenance: Boolean(req.requireSourceProvenance),
  };
}

export function buildPortfolioFeasibilityExportPayload(
  result: PortfolioFeasibilityResult,
  sources: PortfolioWorkspaceSource[],
  req: PortfolioFeasibilityRequest,
  generatedAtUnixMs = Date.now(),
): PortfolioFeasibilityExportPayload {
  return {
    schema_version: 1,
    generated_at_unix_ms: generatedAtUnixMs,
    research_only: true,
    sources: sources.map(compactSourceSummary),
    request_gates: requestGateSnapshot(req),
    result,
    next_required_step: result.nextRequiredStep,
  };
}

function csvCell(value: unknown): string {
  const raw = value == null ? "" : String(value);
  return /[",\r\n]/.test(raw) ? `"${raw.replace(/"/g, '""')}"` : raw;
}

export function buildPortfolioFeasibilityCsv(
  result: PortfolioFeasibilityResult,
  sources: PortfolioWorkspaceSource[],
  req: PortfolioFeasibilityRequest,
): string {
  const payload = buildPortfolioFeasibilityExportPayload(
    result,
    sources,
    req,
    0,
  );
  const rows: unknown[][] = [
    [
      "row_type",
      "source_index",
      "candidate_id",
      "source_role",
      "strategy_kind",
      "symbol",
      "status_or_reason",
      "sharpe",
      "psr",
      "replay_status",
      "next_required_step",
    ],
    [
      "metadata",
      "",
      "",
      "",
      "",
      "",
      result.status,
      "",
      "",
      "",
      result.nextRequiredStep,
    ],
  ];
  payload.sources.forEach((source, index) => {
    rows.push([
      "source",
      index,
      source.id,
      source.source_kind,
      source.strategy_kind,
      "",
      source.artifact_path || source.label,
      "",
      "",
      "",
      result.nextRequiredStep,
    ]);
  });
  result.eligibleCandidates.forEach((candidate) => {
    rows.push([
      "eligible",
      candidate.sourceArtifactIndex,
      candidate.candidateId,
      candidate.sourceRole,
      candidate.strategyKind,
      candidate.symbolRoot,
      "eligible",
      candidate.summaryMetrics.sharpe ?? "",
      candidate.admission.psr ?? "",
      candidate.replayStatus,
      result.nextRequiredStep,
    ]);
  });
  result.rejectedCandidates.forEach((candidate) => {
    rows.push([
      "rejected",
      candidate.sourceArtifactIndex,
      candidate.candidateId,
      candidate.sourceRole,
      "",
      "",
      candidate.reason,
      "",
      "",
      "",
      result.nextRequiredStep,
    ]);
  });
  result.missingEvidence.forEach((missing) => {
    rows.push([
      "missing_evidence",
      missing.sourceArtifactIndex ?? "",
      missing.candidateId,
      "",
      "",
      "",
      `${missing.evidenceKind}: ${missing.message}`,
      "",
      "",
      "",
      result.nextRequiredStep,
    ]);
  });
  return `${rows.map((row) => row.map(csvCell).join(",")).join("\n")}\n`;
}

export function buildPortfolioConstructionReadiness(
  result: PortfolioFeasibilityResult | null,
): PortfolioConstructionReadiness {
  if (!result) {
    return {
      status: "blocked",
      label: "Run feasibility first",
      gates: [
        {
          key: "feasibility",
          label: "Feasibility",
          status: "missing",
          detail:
            "Run the read-only feasibility analysis before construction research.",
          tone: "blocked",
        },
      ],
    };
  }
  const hasEligible = result.eligibleCandidates.length > 0;
  const missingKinds = new Set(
    result.missingEvidence.map((row) => text(row.evidenceKind)),
  );
  const replayMissing =
    missingKinds.has("replay_matrix") || result.status === "replay_required";
  const provenanceBlocked =
    result.status === "artifact_provenance_blocked" ||
    missingKinds.has("source_provenance");
  const covarianceMissing =
    result.status === "insufficient_covariance_evidence";
  const status: PortfolioConstructionReadinessStatus =
    !hasEligible || provenanceBlocked
      ? "blocked"
      : replayMissing
        ? "needs_replay_matrix"
        : "ready_for_covariance_diagnostics";
  return {
    status,
    label:
      status === "ready_for_covariance_diagnostics"
        ? "Ready for covariance diagnostics"
        : status === "needs_replay_matrix"
          ? "Replay matrix required"
          : "Construction blocked",
    gates: [
      {
        key: "admission",
        label: "Admission evidence",
        status: hasEligible ? "eligible candidates" : "no eligible candidates",
        detail: `${result.eligibleCandidates.length} eligible candidate${result.eligibleCandidates.length === 1 ? "" : "s"} found.`,
        tone: hasEligible ? "pass" : "blocked",
      },
      {
        key: "source_provenance",
        label: "Source provenance",
        status: provenanceBlocked ? "blocked" : "usable",
        detail: provenanceBlocked
          ? "Source identity evidence is missing or blocked."
          : "No source provenance blocker was reported.",
        tone: provenanceBlocked ? "blocked" : "pass",
      },
      {
        key: "replay_matrix",
        label: "Replay matrix",
        status: replayMissing ? "missing" : "not blocking",
        detail: replayMissing
          ? "Portfolio construction research needs replay evidence before covariance diagnostics."
          : "Replay matrix evidence is not the current blocker.",
        tone: replayMissing ? "warn" : "pass",
      },
      {
        key: "covariance",
        label: "Covariance evidence",
        status: covarianceMissing ? "missing" : "diagnostic only",
        detail:
          "No weights or targets are emitted; covariance diagnostics remain a future research-only step.",
        tone: covarianceMissing ? "warn" : "pass",
      },
      {
        key: "execution_isolation",
        label: "Execution isolation",
        status: "blocked by design",
        detail:
          "This workspace cannot create broker orders, target quantities, or allocation weights.",
        tone: "blocked",
      },
    ],
  };
}
