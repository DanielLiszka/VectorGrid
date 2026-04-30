export type StageCompletionStage =
  | "optimization"
  | "validation"
  | "meta"
  | "stage5_ml_confidence"
  | "policy_evaluation"
  | "ml_signal"
  | "options_expression"
  | "options_expression_validation"
  | "campaign";

export type StageCompletionStatus =
  | "success"
  | "failed"
  | "cancelled"
  | "blocked"
  | "stale";

export type StageCompletionVerdict =
  | "search_complete_requires_validation"
  | "passed"
  | "approved"
  | "approved_with_warnings"
  | "review"
  | "rejected"
  | "inspect_only"
  | "not_applicable";

export interface StageCompletionPacketV1 {
  schemaVersion: 1;
  stage: StageCompletionStage;
  substage?: string;
  status: StageCompletionStatus;
  verdict: StageCompletionVerdict;
  completedAt: string;
  durationMs?: number | null;
  artifactKind?: string;
  artifactPath?: string;
  reportXlsxPath?: string;
  csvPath?: string;
  strategyKind?: string;
  timeframe?: string;
  configFingerprint?: string;
  backendUsed?: string;
  route?: string;
  recommendation?: string;
  summary?: Record<string, unknown>;
  warnings?: string[];
  blockers?: string[];
  caveats?: string[];
  nextActions?: string[];
  backfilled?: boolean;
}

export interface StageCompletionBuildContext {
  ts?: unknown;
  completedAt?: unknown;
  durationMs?: unknown;
  artifactKind?: unknown;
  artifactPath?: unknown;
  reportXlsxPath?: unknown;
  csvPath?: unknown;
  configFingerprint?: unknown;
  datasetMeta?: unknown;
  routeProvenance?: unknown;
  mlSignal?: unknown;
  req?: unknown;
  res?: unknown;
  validationReq?: unknown;
  validationRes?: unknown;
  backfilled?: boolean;
}

const STAGES: ReadonlySet<string> = new Set([
  "optimization",
  "validation",
  "meta",
  "stage5_ml_confidence",
  "policy_evaluation",
  "ml_signal",
  "options_expression",
  "options_expression_validation",
  "campaign",
]);

const STATUSES: ReadonlySet<string> = new Set([
  "success",
  "failed",
  "cancelled",
  "blocked",
  "stale",
]);

const VERDICTS: ReadonlySet<string> = new Set([
  "search_complete_requires_validation",
  "passed",
  "approved",
  "approved_with_warnings",
  "review",
  "rejected",
  "inspect_only",
  "not_applicable",
]);

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  return value as Record<string, unknown>;
}

function text(value: unknown): string {
  return String(value ?? "").trim();
}

function firstText(...values: unknown[]): string {
  for (const value of values) {
    const next = text(value);
    if (next) return next;
  }
  return "";
}

function finiteNumber(value: unknown): number | null {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function optionalNumber(value: unknown): number | null | undefined {
  if (value == null || text(value) === "") return undefined;
  return finiteNumber(value);
}

function optionalBoolean(value: unknown): boolean | null {
  if (typeof value === "boolean") return value;
  const next = text(value).toLowerCase();
  if (!next) return null;
  if (next === "1" || next === "true" || next === "yes") return true;
  if (next === "0" || next === "false" || next === "no") return false;
  return null;
}

function stringList(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((entry) => text(entry))
    .filter(
      (entry, index, values) =>
        entry.length > 0 && values.indexOf(entry) === index,
    );
}

function objectEntries(value: Record<string, unknown>): [string, unknown][] {
  return Object.entries(value).filter(([, entry]) => {
    if (entry == null) return false;
    if (typeof entry === "string") return entry.trim().length > 0;
    if (Array.isArray(entry)) return entry.length > 0;
    if (typeof entry === "object")
      return Object.keys(entry as Record<string, unknown>).length > 0;
    return true;
  });
}

function compactRecord(
  value: Record<string, unknown>,
): Record<string, unknown> | undefined {
  const entries = objectEntries(value);
  if (!entries.length) return undefined;
  return Object.fromEntries(entries);
}

function defaultVerdictForStage(
  stage: StageCompletionStage,
): StageCompletionVerdict {
  if (stage === "optimization") return "search_complete_requires_validation";
  if (stage === "validation") return "review";
  return "inspect_only";
}

function normalizeStage(value: unknown): StageCompletionStage | null {
  const normalized = text(value);
  return STAGES.has(normalized) ? (normalized as StageCompletionStage) : null;
}

function normalizeStatus(value: unknown): StageCompletionStatus {
  const normalized = text(value);
  return STATUSES.has(normalized)
    ? (normalized as StageCompletionStatus)
    : "success";
}

function normalizeVerdict(
  value: unknown,
  stage: StageCompletionStage,
): StageCompletionVerdict {
  const normalized = text(value);
  return VERDICTS.has(normalized)
    ? (normalized as StageCompletionVerdict)
    : defaultVerdictForStage(stage);
}

function getRouteValue(routeProvenance: unknown, key: string): unknown {
  const route = asRecord(routeProvenance);
  if (!route) return undefined;
  if (key in route) return route[key];
  const camel = key.replace(/_([a-z])/g, (_, letter: string) =>
    letter.toUpperCase(),
  );
  return route[camel];
}

function strategyKindFrom(req: unknown): string {
  const row = asRecord(req);
  return firstText(row?.strategy_kind, row?.strategyKind);
}

function timeframeFrom(datasetMeta: unknown): string {
  const row = asRecord(datasetMeta);
  return firstText(
    row?.timeframe,
    row?.derived_timeframe,
    row?.derivedTimeframe,
    row?.base_timeframe,
    row?.baseTimeframe,
  );
}

function artifactRouteFrom(res: unknown, routeProvenance: unknown): string {
  const row = asRecord(res);
  return firstText(
    row?.backend_route,
    row?.backendRoute,
    getRouteValue(routeProvenance, "realized_route"),
    getRouteValue(routeProvenance, "resolved_route"),
    getRouteValue(routeProvenance, "requested_route"),
  );
}

function backendUsedFrom(res: unknown, routeProvenance: unknown): string {
  const row = asRecord(res);
  return firstText(
    row?.backend_used,
    row?.backendUsed,
    row?.backend,
    getRouteValue(routeProvenance, "realized_route"),
    getRouteValue(routeProvenance, "resolved_route"),
  );
}

function arrayLen(value: unknown): number | null {
  return Array.isArray(value) ? value.length : null;
}

function nonNegativeInt(value: unknown): number | null {
  const parsed = Math.floor(Number(value));
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : null;
}

function topCandidateCount(res: unknown): number | null {
  const row = asRecord(res);
  if (!row) return null;
  return (
    arrayLen(row.top) ??
    arrayLen(row.candidates) ??
    nonNegativeInt(row.top_count) ??
    nonNegativeInt(row.topCount) ??
    nonNegativeInt(row.candidate_count) ??
    nonNegativeInt(row.candidateCount)
  );
}

function validationCandidateCount(res: unknown): number | null {
  const row = asRecord(res);
  if (!row) return null;
  const verdict = asRecord(row.run_verdict ?? row.runVerdict);
  return (
    nonNegativeInt(verdict?.candidates_tested) ??
    nonNegativeInt(verdict?.candidatesTested) ??
    arrayLen(row.candidates) ??
    nonNegativeInt(row.candidate_count) ??
    nonNegativeInt(row.candidateCount)
  );
}

function verdictWarnings(runVerdict: Record<string, unknown> | null): string[] {
  return stringList(runVerdict?.warning_flags ?? runVerdict?.warningFlags);
}

function verdictBlockers(runVerdict: Record<string, unknown> | null): string[] {
  return stringList(runVerdict?.fail_reasons ?? runVerdict?.failReasons);
}

function validationVerdict(res: unknown): StageCompletionVerdict {
  const row = asRecord(res);
  const runVerdict = asRecord(row?.run_verdict ?? row?.runVerdict);
  if (!runVerdict) return "review";
  const warnings = verdictWarnings(runVerdict);
  const blockers = verdictBlockers(runVerdict);
  const approved = optionalBoolean(runVerdict.approved);
  const passed = optionalBoolean(runVerdict.passed);
  if (approved === true)
    return warnings.length > 0 ? "approved_with_warnings" : "approved";
  if (passed === false || blockers.length > 0) return "rejected";
  if (passed === true)
    return warnings.length > 0 ? "approved_with_warnings" : "passed";
  return warnings.length > 0 ? "review" : "review";
}

function genericVerdictFromText(value: unknown): StageCompletionVerdict | null {
  const normalized = text(value).toLowerCase();
  if (!normalized) return null;
  if (
    normalized.includes("reject") ||
    normalized.includes("fail") ||
    normalized.includes("blocked") ||
    normalized.includes("non_promotable")
  ) {
    return "rejected";
  }
  if (normalized.includes("warning") || normalized.includes("review")) {
    return "approved_with_warnings";
  }
  if (normalized.includes("approved") || normalized.includes("accepted")) {
    return "approved";
  }
  if (normalized.includes("pass") || normalized.includes("valid")) {
    return "passed";
  }
  return null;
}

function statusFromCancelledOrFailed(value: unknown): StageCompletionStatus {
  const row = asRecord(value);
  if (optionalBoolean(row?.cancelled ?? row?.canceled) === true) return "cancelled";
  const failed = nonNegativeInt(row?.failed ?? row?.failed_count ?? row?.failedCount);
  if (failed != null && failed > 0) return "failed";
  return "success";
}

function resultRecord(context: StageCompletionBuildContext): Record<string, unknown> | null {
  return asRecord(context.res);
}

function packetKey(
  packet: Pick<StageCompletionPacketV1, "stage" | "substage">,
): string {
  return `${packet.stage}:${packet.substage || ""}`;
}

export function normalizeStageCompletions(
  value: unknown,
): StageCompletionPacketV1[] {
  if (!Array.isArray(value)) return [];
  const out: StageCompletionPacketV1[] = [];
  for (const item of value) {
    const row = asRecord(item);
    if (!row) continue;
    const stage = normalizeStage(row.stage);
    if (!stage) continue;
    const durationMs = optionalNumber(row.duration_ms ?? row.durationMs);
    const summary = asRecord(row.summary);
    const packet: StageCompletionPacketV1 = {
      schemaVersion: 1,
      stage,
      status: normalizeStatus(row.status),
      verdict: normalizeVerdict(row.verdict, stage),
      completedAt: firstText(row.completed_at, row.completedAt),
    };
    const optionalStrings: Array<[keyof StageCompletionPacketV1, unknown]> = [
      ["substage", row.substage],
      ["artifactKind", row.artifact_kind ?? row.artifactKind],
      ["artifactPath", row.artifact_path ?? row.artifactPath],
      ["reportXlsxPath", row.report_xlsx_path ?? row.reportXlsxPath],
      ["csvPath", row.csv_path ?? row.csvPath],
      ["strategyKind", row.strategy_kind ?? row.strategyKind],
      ["timeframe", row.timeframe],
      ["configFingerprint", row.config_fingerprint ?? row.configFingerprint],
      ["backendUsed", row.backend_used ?? row.backendUsed],
      ["route", row.route],
      ["recommendation", row.recommendation],
    ];
    for (const [key, raw] of optionalStrings) {
      const next = text(raw);
      if (next) {
        (packet as unknown as Record<string, unknown>)[key] = next;
      }
    }
    if (durationMs !== undefined) packet.durationMs = durationMs;
    if (summary && Object.keys(summary).length > 0) packet.summary = summary;
    const warnings = stringList(row.warnings);
    const blockers = stringList(row.blockers);
    const caveats = stringList(row.caveats);
    const nextActions = stringList(row.next_actions ?? row.nextActions);
    if (warnings.length) packet.warnings = warnings;
    if (blockers.length) packet.blockers = blockers;
    if (caveats.length) packet.caveats = caveats;
    if (nextActions.length) packet.nextActions = nextActions;
    if (optionalBoolean(row.backfilled) === true) packet.backfilled = true;
    out.push(packet);
  }
  return out;
}

export function buildOptimizationStageCompletion(
  context: StageCompletionBuildContext,
): StageCompletionPacketV1 {
  const res = asRecord(context.res);
  const topCount = topCandidateCount(context.res);
  const summary = compactRecord({
    rankedCandidates: topCount,
    objective: firstText(
      asRecord(context.req)?.objective,
      asRecord(context.req)?.objective_kind,
      asRecord(context.req)?.objectiveKind,
    ),
    backendFallbackReason: firstText(
      res?.backend_fallback_reason,
      res?.backendFallbackReason,
    ),
  });
  const packet: StageCompletionPacketV1 = {
    schemaVersion: 1,
    stage: "optimization",
    status: "success",
    verdict: "search_complete_requires_validation",
    completedAt: firstText(context.completedAt, context.ts),
    artifactKind: firstText(context.artifactKind, "vectorgrid_run_artifact"),
    nextActions: ["Run validation on the selected candidate."],
  };
  const optionalStrings: Array<[keyof StageCompletionPacketV1, unknown]> = [
    ["artifactPath", context.artifactPath],
    ["reportXlsxPath", context.reportXlsxPath],
    ["csvPath", context.csvPath],
    ["strategyKind", strategyKindFrom(context.req)],
    ["timeframe", timeframeFrom(context.datasetMeta)],
    ["configFingerprint", context.configFingerprint],
    ["backendUsed", backendUsedFrom(context.res, context.routeProvenance)],
    ["route", artifactRouteFrom(context.res, context.routeProvenance)],
  ];
  for (const [key, raw] of optionalStrings) {
    const next = text(raw);
    if (next) {
      (packet as unknown as Record<string, unknown>)[key] = next;
    }
  }
  if (summary) packet.summary = summary;
  const durationMs = optionalNumber(context.durationMs);
  if (durationMs !== undefined) packet.durationMs = durationMs;
  if (context.backfilled) packet.backfilled = true;
  return packet;
}

export function buildValidationStageCompletion(
  context: StageCompletionBuildContext,
): StageCompletionPacketV1 {
  const res = asRecord(context.validationRes ?? context.res);
  const runVerdict = asRecord(res?.run_verdict ?? res?.runVerdict);
  const warnings = verdictWarnings(runVerdict);
  const blockers = verdictBlockers(runVerdict);
  const summary = compactRecord({
    candidatesTested: validationCandidateCount(res),
    candidatesSurvivedHardGates: nonNegativeInt(
      runVerdict?.candidates_survived_hard_gates ??
        runVerdict?.candidatesSurvivedHardGates,
    ),
    approvedCandidateCount: nonNegativeInt(
      runVerdict?.approved_candidate_count ??
        runVerdict?.approvedCandidateCount,
    ),
    firstApprovedRank: nonNegativeInt(
      runVerdict?.first_approved_rank ?? runVerdict?.firstApprovedRank,
    ),
    pbo: finiteNumber(runVerdict?.pbo),
  });
  const packet: StageCompletionPacketV1 = {
    schemaVersion: 1,
    stage: "validation",
    status: "success",
    verdict: validationVerdict(res),
    completedAt: firstText(context.completedAt, context.ts),
    artifactKind: firstText(context.artifactKind, "vectorgrid_run_artifact"),
  };
  const optionalStrings: Array<[keyof StageCompletionPacketV1, unknown]> = [
    ["artifactPath", context.artifactPath],
    ["reportXlsxPath", context.reportXlsxPath],
    ["csvPath", context.csvPath],
    [
      "strategyKind",
      strategyKindFrom(context.validationReq) || strategyKindFrom(context.req),
    ],
    ["timeframe", timeframeFrom(context.datasetMeta)],
    ["configFingerprint", context.configFingerprint],
    ["backendUsed", backendUsedFrom(res, context.routeProvenance)],
    ["route", artifactRouteFrom(res, context.routeProvenance)],
    [
      "recommendation",
      firstText(
        runVerdict?.final_recommendation,
        runVerdict?.finalRecommendation,
      ),
    ],
  ];
  for (const [key, raw] of optionalStrings) {
    const next = text(raw);
    if (next) {
      (packet as unknown as Record<string, unknown>)[key] = next;
    }
  }
  if (summary) packet.summary = summary;
  const durationMs = optionalNumber(context.durationMs);
  if (durationMs !== undefined) packet.durationMs = durationMs;
  if (warnings.length) packet.warnings = warnings;
  if (blockers.length) packet.blockers = blockers;
  if (context.backfilled) packet.backfilled = true;
  return packet;
}

export function buildMlSignalStageCompletion(
  context: StageCompletionBuildContext,
): StageCompletionPacketV1 | null {
  const mlSignal = asRecord(context.mlSignal);
  const substage = firstText(mlSignal?.stage);
  if (!substage) return null;
  const summary = compactRecord({
    bundleId: firstText(mlSignal?.bundle_id, mlSignal?.bundleId),
    rowCount: finiteNumber(mlSignal?.row_count ?? mlSignal?.rowCount),
    featureCount: finiteNumber(
      mlSignal?.feature_count ?? mlSignal?.featureCount,
    ),
    scoreSpace: firstText(mlSignal?.score_space, mlSignal?.scoreSpace),
    scoreScope: firstText(mlSignal?.score_scope, mlSignal?.scoreScope),
  });
  const packet: StageCompletionPacketV1 = {
    schemaVersion: 1,
    stage: "ml_signal",
    substage,
    status: "success",
    verdict: "inspect_only",
    completedAt: firstText(context.completedAt, context.ts),
    artifactKind: firstText(context.artifactKind, "vectorgrid_run_artifact"),
  };
  const optionalStrings: Array<[keyof StageCompletionPacketV1, unknown]> = [
    ["artifactPath", context.artifactPath],
    ["reportXlsxPath", context.reportXlsxPath],
    ["csvPath", context.csvPath],
    ["strategyKind", "MlSignal"],
    ["timeframe", timeframeFrom(context.datasetMeta)],
    ["configFingerprint", context.configFingerprint],
    [
      "backendUsed",
      firstText(mlSignal?.native_backend_used, mlSignal?.nativeBackendUsed),
    ],
    ["route", firstText(mlSignal?.score_space, mlSignal?.scoreSpace)],
  ];
  for (const [key, raw] of optionalStrings) {
    const next = text(raw);
    if (next) {
      (packet as unknown as Record<string, unknown>)[key] = next;
    }
  }
  if (summary) packet.summary = summary;
  if (context.backfilled) packet.backfilled = true;
  return packet;
}

export function buildMetaStageCompletion(
  context: StageCompletionBuildContext,
): StageCompletionPacketV1 {
  const res = resultRecord(context);
  const summary = compactRecord({
    groups: arrayLen(res?.groups) ?? nonNegativeInt(res?.groups),
    selectedGroups: nonNegativeInt(res?.groups_selected ?? res?.groupsSelected),
    candidates: nonNegativeInt(res?.candidate_count ?? res?.candidateCount),
    sourceArtifacts: nonNegativeInt(
      res?.source_artifact_count ?? res?.sourceArtifactCount,
    ),
  });
  const packet: StageCompletionPacketV1 = {
    schemaVersion: 1,
    stage: "meta",
    status: statusFromCancelledOrFailed(res),
    verdict: genericVerdictFromText(res?.verdict ?? res?.status) ?? "inspect_only",
    completedAt: firstText(context.completedAt, context.ts, res?.ts),
    artifactKind: firstText(context.artifactKind, "vectorgrid_meta_selection"),
    artifactPath: firstText(context.artifactPath, res?.meta_json_path, res?.artifact_path),
  };
  const optionalStrings: Array<[keyof StageCompletionPacketV1, unknown]> = [
    ["reportXlsxPath", context.reportXlsxPath ?? res?.report_xlsx_path],
    ["strategyKind", res?.strategy_kind ?? res?.strategyKind],
    ["timeframe", res?.timeframe],
    ["configFingerprint", context.configFingerprint ?? res?.config_fingerprint],
    ["recommendation", res?.recommendation],
  ];
  for (const [key, raw] of optionalStrings) {
    const next = text(raw);
    if (next) (packet as unknown as Record<string, unknown>)[key] = next;
  }
  if (summary) packet.summary = summary;
  if (context.backfilled) packet.backfilled = true;
  return packet;
}

export function buildStage5MlConfidenceStageCompletion(
  context: StageCompletionBuildContext,
): StageCompletionPacketV1 {
  const res = resultRecord(context);
  const summary = compactRecord({
    groupsSelected: nonNegativeInt(res?.groups_selected ?? res?.groupsSelected),
    groupsScored: nonNegativeInt(res?.groups_scored ?? res?.groupsScored),
    groupsConsidered: nonNegativeInt(
      res?.groups_considered ?? res?.groupsConsidered,
    ),
    minMlConfidence: finiteNumber(
      res?.min_ml_confidence ?? res?.minMlConfidence,
    ),
    topK: nonNegativeInt(res?.top_k ?? res?.topK),
    modelHealthy: optionalBoolean(res?.model_healthy ?? res?.modelHealthy),
    modelEvidenceTier: firstText(
      res?.model_evidence_tier,
      res?.modelEvidenceTier,
    ),
    warningCount: nonNegativeInt(res?.warning_count ?? res?.warningCount),
    trainingArtifactCount: nonNegativeInt(
      res?.training_artifact_count ?? res?.trainingArtifactCount,
    ),
  });
  const modelHealthy = optionalBoolean(res?.model_healthy ?? res?.modelHealthy);
  const packet: StageCompletionPacketV1 = {
    schemaVersion: 1,
    stage: "stage5_ml_confidence",
    status: modelHealthy === false ? "blocked" : statusFromCancelledOrFailed(res),
    verdict: modelHealthy === false
      ? "rejected"
      : genericVerdictFromText(res?.verdict ?? res?.recommendation) ?? "inspect_only",
    completedAt: firstText(context.completedAt, context.ts, res?.ts),
    artifactKind: firstText(
      context.artifactKind,
      "vectorgrid_meta_stage5_ml_confidence",
    ),
    artifactPath: firstText(
      context.artifactPath,
      res?.stage5_json_path,
      res?.stage4_json_path,
      res?.artifact_path,
    ),
  };
  const optionalStrings: Array<[keyof StageCompletionPacketV1, unknown]> = [
    ["strategyKind", res?.strategy_kind ?? res?.strategyKind],
    ["timeframe", res?.timeframe],
    ["configFingerprint", context.configFingerprint ?? res?.config_fingerprint],
    ["recommendation", res?.recommendation],
  ];
  for (const [key, raw] of optionalStrings) {
    const next = text(raw);
    if (next) (packet as unknown as Record<string, unknown>)[key] = next;
  }
  if (summary) packet.summary = summary;
  if (modelHealthy === false) packet.blockers = ["model_health"];
  if (context.backfilled) packet.backfilled = true;
  return packet;
}

export function buildPolicyEvaluationStageCompletion(
  context: StageCompletionBuildContext,
): StageCompletionPacketV1 {
  const res = resultRecord(context);
  const stage5Blended = asRecord(res?.stage5_blended ?? res?.stage5Blended);
  const summary = compactRecord({
    artifactsEvaluated: nonNegativeInt(
      res?.artifacts_evaluated ?? res?.artifactsEvaluated,
    ),
    artifactsConsidered: nonNegativeInt(
      res?.artifacts_considered ?? res?.artifactsConsidered,
    ),
    bestPolicy: firstText(res?.best_policy, res?.bestPolicy),
    stage5PrecisionAtK: finiteNumber(
      stage5Blended?.precision_at_k ?? stage5Blended?.precisionAtK,
    ),
    stage5VsMetaPrecisionDelta: finiteNumber(
      res?.stage5_vs_meta_precision_delta ?? res?.stage5VsMetaPrecisionDelta,
    ),
  });
  const packet: StageCompletionPacketV1 = {
    schemaVersion: 1,
    stage: "policy_evaluation",
    status: statusFromCancelledOrFailed(res),
    verdict: genericVerdictFromText(res?.verdict ?? res?.status) ?? "inspect_only",
    completedAt: firstText(context.completedAt, context.ts, res?.ts),
    artifactKind: firstText(
      context.artifactKind,
      "vectorgrid_ml_confidence_policy_evaluation",
    ),
    artifactPath: firstText(context.artifactPath, res?.eval_json_path, res?.artifact_path),
  };
  const optionalStrings: Array<[keyof StageCompletionPacketV1, unknown]> = [
    ["strategyKind", res?.strategy_kind ?? res?.strategyKind],
    ["timeframe", res?.timeframe],
    ["configFingerprint", context.configFingerprint ?? res?.config_fingerprint],
    ["recommendation", res?.best_policy ?? res?.bestPolicy],
  ];
  for (const [key, raw] of optionalStrings) {
    const next = text(raw);
    if (next) (packet as unknown as Record<string, unknown>)[key] = next;
  }
  if (summary) packet.summary = summary;
  if (context.backfilled) packet.backfilled = true;
  return packet;
}

export function buildOptionsExpressionValidationStageCompletion(
  context: StageCompletionBuildContext,
): StageCompletionPacketV1 {
  const res = resultRecord(context);
  const acceptance = asRecord(res?.acceptance_summary ?? res?.acceptanceSummary);
  const accepted = nonNegativeInt(
    acceptance?.accepted_tuple_count ?? acceptance?.acceptedTupleCount,
  );
  const supportDeficient = nonNegativeInt(
    acceptance?.support_deficient_tuple_count ??
      acceptance?.supportDeficientTupleCount,
  );
  const notes = stringList(acceptance?.notes);
  const verdict =
    genericVerdictFromText(acceptance?.verdict) ??
    (accepted != null && accepted > 0
      ? supportDeficient != null && supportDeficient > 0
        ? "approved_with_warnings"
        : "approved"
      : "review");
  const summary = compactRecord({
    selectedTupleCount: nonNegativeInt(
      acceptance?.selected_tuple_count ?? acceptance?.selectedTupleCount,
    ),
    validatedTupleCount: nonNegativeInt(
      acceptance?.validated_tuple_count ?? acceptance?.validatedTupleCount,
    ),
    acceptedTupleCount: accepted,
    supportDeficientTupleCount: supportDeficient,
    stressFragileTupleCount: nonNegativeInt(
      acceptance?.stress_fragile_tuple_count ??
        acceptance?.stressFragileTupleCount,
    ),
    rootSymbol: firstText(res?.root_symbol, res?.rootSymbol),
    optionsDatasetId: firstText(
      res?.options_dataset_id,
      res?.optionsDatasetId,
    ),
  });
  const packet: StageCompletionPacketV1 = {
    schemaVersion: 1,
    stage: "options_expression_validation",
    status: statusFromCancelledOrFailed(res),
    verdict,
    completedAt: firstText(context.completedAt, context.ts, res?.ts),
    artifactKind: firstText(
      context.artifactKind,
      "vectorgrid_options_expression_validation",
    ),
    artifactPath: firstText(
      context.artifactPath,
      res?.validation_artifact_path,
      res?.artifact_path,
    ),
    strategyKind: "OptionsExpression",
  };
  const optionalStrings: Array<[keyof StageCompletionPacketV1, unknown]> = [
    ["timeframe", res?.timeframe],
    ["configFingerprint", context.configFingerprint ?? res?.source_artifact_hash],
    ["recommendation", acceptance?.verdict],
  ];
  for (const [key, raw] of optionalStrings) {
    const next = text(raw);
    if (next) (packet as unknown as Record<string, unknown>)[key] = next;
  }
  if (summary) packet.summary = summary;
  if (notes.length) packet.caveats = notes;
  if (supportDeficient != null && supportDeficient > 0) {
    packet.warnings = [`${supportDeficient} support-deficient tuple${supportDeficient === 1 ? "" : "s"}.`];
  }
  if (context.backfilled) packet.backfilled = true;
  return packet;
}

export function buildCampaignStageCompletion(
  context: StageCompletionBuildContext,
): StageCompletionPacketV1 {
  const res = resultRecord(context);
  const counts = asRecord(res?.counts);
  const failed = nonNegativeInt(res?.failed ?? res?.failed_count ?? res?.failedCount ?? counts?.failed) ?? 0;
  const skipped = nonNegativeInt(res?.skipped ?? res?.skipped_count ?? res?.skippedCount ?? counts?.skipped) ?? 0;
  const cancelled = optionalBoolean(res?.cancelled ?? res?.canceled ?? counts?.cancelled) === true;
  const summary = compactRecord({
    validated: nonNegativeInt(res?.validated ?? res?.validated_count ?? res?.validatedCount ?? counts?.validated),
    total: nonNegativeInt(res?.total ?? res?.dataset_count ?? res?.datasetCount ?? counts?.total),
    failed,
    skipped,
    autoMeta: optionalBoolean(res?.auto_meta ?? res?.autoMeta),
    autoStage5: optionalBoolean(res?.auto_stage5 ?? res?.autoStage5),
  });
  const packet: StageCompletionPacketV1 = {
    schemaVersion: 1,
    stage: "campaign",
    status: cancelled ? "cancelled" : failed > 0 ? "failed" : "success",
    verdict: cancelled || failed > 0 || skipped > 0 ? "review" : "passed",
    completedAt: firstText(context.completedAt, context.ts, res?.ts),
    artifactKind: firstText(context.artifactKind, "vectorgrid_campaign_summary"),
    artifactPath: firstText(context.artifactPath, res?.summary_path, res?.artifact_path),
  };
  const optionalStrings: Array<[keyof StageCompletionPacketV1, unknown]> = [
    ["strategyKind", res?.strategy_kind ?? res?.strategyKind],
    ["timeframe", res?.timeframe],
    ["recommendation", res?.recommendation],
  ];
  for (const [key, raw] of optionalStrings) {
    const next = text(raw);
    if (next) (packet as unknown as Record<string, unknown>)[key] = next;
  }
  if (summary) packet.summary = summary;
  const blockers = stringList(res?.fail_reasons ?? res?.failReasons);
  if (blockers.length) packet.blockers = blockers;
  if (context.backfilled) packet.backfilled = true;
  return packet;
}

export function ensureDerivedStageCompletions(
  completions: unknown,
  context: StageCompletionBuildContext,
): StageCompletionPacketV1[] {
  const out = normalizeStageCompletions(completions);
  const hasStage = (stage: StageCompletionStage) =>
    out.some((entry) => entry.stage === stage);
  if (!hasStage("optimization") && context.res != null) {
    out.push(
      buildOptimizationStageCompletion({ ...context, backfilled: true }),
    );
  }
  if (
    !hasStage("validation") &&
    (context.validationRes ?? context.res) != null &&
    context.validationRes != null
  ) {
    out.push(buildValidationStageCompletion({ ...context, backfilled: true }));
  }
  if (!hasStage("ml_signal")) {
    const mlSignalPacket = buildMlSignalStageCompletion({
      ...context,
      backfilled: true,
    });
    if (mlSignalPacket) out.push(mlSignalPacket);
  }
  return out;
}

export function upsertStageCompletion(
  completions: unknown,
  packet: StageCompletionPacketV1,
): StageCompletionPacketV1[] {
  const normalized = normalizeStageCompletions(completions);
  const key = packetKey(packet);
  return [...normalized.filter((entry) => packetKey(entry) !== key), packet];
}

export function latestStageCompletionForStage(
  completions: unknown,
  stage: StageCompletionStage,
  substage?: string,
): StageCompletionPacketV1 | null {
  const normalized = normalizeStageCompletions(completions);
  for (let index = normalized.length - 1; index >= 0; index -= 1) {
    const packet = normalized[index];
    if (!packet) continue;
    if (packet.stage !== stage) continue;
    if (substage != null && packet.substage !== substage) continue;
    return packet;
  }
  return null;
}

export function stageCompletionNeedsReview(
  packet: StageCompletionPacketV1 | null | undefined,
): boolean {
  if (!packet) return false;
  if (packet.status !== "success") return true;
  return (
    packet.verdict === "approved_with_warnings" ||
    packet.verdict === "review" ||
    packet.verdict === "rejected"
  );
}

export function stageCompletionIsComplete(
  packet: StageCompletionPacketV1 | null | undefined,
): boolean {
  if (!packet) return false;
  return packet.status === "success";
}

export function updateStageCompletionReportPath(
  completions: unknown,
  reportXlsxPath: unknown,
): StageCompletionPacketV1[] {
  const path = text(reportXlsxPath);
  return normalizeStageCompletions(completions).map((packet) =>
    packet.stage === "validation" && path
      ? { ...packet, reportXlsxPath: path }
      : packet,
  );
}

export function serializeStageCompletionForArtifact(
  packet: StageCompletionPacketV1,
): Record<string, unknown> {
  return (
    compactRecord({
      schema_version: packet.schemaVersion,
      stage: packet.stage,
      substage: packet.substage,
      status: packet.status,
      verdict: packet.verdict,
      completed_at: packet.completedAt,
      duration_ms: packet.durationMs,
      artifact_kind: packet.artifactKind,
      artifact_path: packet.artifactPath,
      report_xlsx_path: packet.reportXlsxPath,
      csv_path: packet.csvPath,
      strategy_kind: packet.strategyKind,
      timeframe: packet.timeframe,
      config_fingerprint: packet.configFingerprint,
      backend_used: packet.backendUsed,
      route: packet.route,
      recommendation: packet.recommendation,
      summary: packet.summary,
      warnings: packet.warnings,
      blockers: packet.blockers,
      caveats: packet.caveats,
      next_actions: packet.nextActions,
      backfilled: packet.backfilled,
    }) ?? {}
  );
}

export function serializeStageCompletionsForArtifact(
  completions: unknown,
): Record<string, unknown>[] {
  return normalizeStageCompletions(completions).map(
    serializeStageCompletionForArtifact,
  );
}
