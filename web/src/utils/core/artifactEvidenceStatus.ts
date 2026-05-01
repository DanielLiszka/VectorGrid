export type ArtifactReportState =
  | "not_exported"
  | "available"
  | "missing_on_disk"
  | "unknown";

export type ArtifactCompatibilityStatus =
  | "legacy_unknown"
  | "supported"
  | "future_unsupported"
  | "invalid"
  | "unknown";

export interface ArtifactCompatibilitySummary {
  artifactFamily: string;
  artifactKind: string;
  artifactSchemaVersion: number | null;
  supportedSchemaVersion: number;
  commandResponseVersion: number;
  reportSchemaVersion: number | null;
  status: ArtifactCompatibilityStatus;
  reason: string;
  reviewOnly: boolean;
  replayBlocked: boolean;
}

function normalizeText(value: unknown): string {
  return String(value || "").trim();
}

function normalizeOptionalNumber(value: unknown): number | null {
  if (value == null || value === "") return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function normalizeOptionalBoolean(value: unknown): boolean {
  if (typeof value === "boolean") return value;
  const text = normalizeText(value).toLowerCase();
  return text === "1" || text === "true" || text === "yes" || text === "on";
}

function normalizeTags(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  const out: string[] = [];
  for (const item of value) {
    const next = normalizeText(item);
    if (next) out.push(next);
  }
  return out;
}

export function resolveReportState(
  path: unknown,
  exists: boolean | null | undefined,
): ArtifactReportState {
  if (!normalizeText(path)) return "not_exported";
  if (exists === true) return "available";
  if (exists === false) return "missing_on_disk";
  return "unknown";
}

export function canOpenPersistedReport(
  path: unknown,
  exists: boolean | null | undefined,
): boolean {
  const state = resolveReportState(path, exists);
  return state === "available" || state === "unknown";
}

function normalizeCompatibilityStatus(
  value: unknown,
): ArtifactCompatibilityStatus {
  const next = normalizeText(value);
  if (
    next === "legacy_unknown" ||
    next === "supported" ||
    next === "future_unsupported" ||
    next === "invalid"
  ) {
    return next;
  }
  return "unknown";
}

export function normalizeArtifactCompatibility(
  value: unknown,
  fallback: Partial<ArtifactCompatibilitySummary> = {},
): ArtifactCompatibilitySummary {
  const row =
    value && typeof value === "object"
      ? (value as Record<string, unknown>)
      : {};
  const status = normalizeCompatibilityStatus(row.status ?? fallback.status);
  const replayBlocked =
    normalizeOptionalBoolean(row.replayBlocked ?? row.replay_blocked) ||
    status === "future_unsupported" ||
    status === "invalid";
  const reviewOnly =
    normalizeOptionalBoolean(row.reviewOnly ?? row.review_only) ||
    replayBlocked;

  return {
    artifactFamily:
      normalizeText(
        row.artifactFamily ?? row.artifact_family ?? fallback.artifactFamily,
      ) || "run",
    artifactKind:
      normalizeText(
        row.artifactKind ?? row.artifact_kind ?? fallback.artifactKind,
      ) || "",
    artifactSchemaVersion: normalizeOptionalNumber(
      row.artifactSchemaVersion ??
        row.artifact_schema_version ??
        fallback.artifactSchemaVersion,
    ),
    supportedSchemaVersion:
      normalizeOptionalNumber(
        row.supportedSchemaVersion ??
          row.supported_schema_version ??
          fallback.supportedSchemaVersion,
      ) ?? 1,
    commandResponseVersion:
      normalizeOptionalNumber(
        row.commandResponseVersion ??
          row.command_response_version ??
          fallback.commandResponseVersion,
      ) ?? 1,
    reportSchemaVersion: normalizeOptionalNumber(
      row.reportSchemaVersion ??
        row.report_schema_version ??
        fallback.reportSchemaVersion,
    ),
    status,
    reason: normalizeText(row.reason ?? fallback.reason),
    reviewOnly,
    replayBlocked,
  };
}

export function artifactCompatibilityAllowsReplay(
  value: ArtifactCompatibilitySummary | null | undefined,
): boolean {
  const next = normalizeArtifactCompatibility(value ?? null);
  return !next.reviewOnly && !next.replayBlocked;
}

export function artifactCompatibilityLabel(
  value: ArtifactCompatibilitySummary | null | undefined,
): string {
  const next = normalizeArtifactCompatibility(value ?? null);
  const version =
    next.artifactSchemaVersion == null
      ? "legacy"
      : `v${next.artifactSchemaVersion}`;
  switch (next.status) {
    case "supported":
      return `Schema ${version} supported`;
    case "legacy_unknown":
      return "Legacy schema compatible";
    case "future_unsupported":
      return `Schema ${version} review-only`;
    case "invalid":
      return "Schema invalid";
    case "unknown":
    default:
      return "Schema status unknown";
  }
}

export function artifactCompatibilityMetaToken(
  value: ArtifactCompatibilitySummary | null | undefined,
): string {
  const next = normalizeArtifactCompatibility(value ?? null);
  if (next.status === "supported" && next.artifactSchemaVersion != null) {
    return `schema=v${next.artifactSchemaVersion}`;
  }
  if (next.status === "legacy_unknown") return "schema=legacy";
  if (next.status === "future_unsupported") return "schema=review-only";
  if (next.status === "invalid") return "schema=invalid";
  return "schema=unknown";
}

export function artifactCompatibilitySummary(
  value: ArtifactCompatibilitySummary | null | undefined,
): string {
  const next = normalizeArtifactCompatibility(value ?? null);
  const label = artifactCompatibilityLabel(next);
  return next.reason ? `${label} - ${next.reason}` : label;
}

export function formatReportStateLabel(state: ArtifactReportState): string {
  switch (state) {
    case "available":
      return "XLSX available";
    case "missing_on_disk":
      return "Missing on disk";
    case "unknown":
      return "Availability unverified";
    case "not_exported":
    default:
      return "Not exported";
  }
}

export function formatReportStateShortLabel(
  state: ArtifactReportState,
): string {
  switch (state) {
    case "available":
      return "xlsx";
    case "missing_on_disk":
      return "missing xlsx";
    case "unknown":
      return "xlsx unverified";
    case "not_exported":
    default:
      return "artifact only";
  }
}

export function formatReportStateMetaToken(state: ArtifactReportState): string {
  switch (state) {
    case "available":
      return "xlsx=yes";
    case "missing_on_disk":
      return "xlsx=missing";
    case "unknown":
      return "xlsx=unknown";
    case "not_exported":
    default:
      return "";
  }
}

export function replayClassLabel(value: unknown): string {
  const next = normalizeText(value);
  if (next === "exact_evidence_review") return "Exact evidence review";
  if (next === "compatible_recompute") return "Compatible recompute";
  if (next === "replacement_scoped_validation")
    return "Replacement-scoped validation";
  if (next === "review_only_saved_result") return "Review-only saved result";
  if (next === "blocked") return "Blocked";
  return next;
}

export function replayClassSummary(
  entry: { replayClass?: unknown; replayReasons?: unknown },
  detail?: { replayClass?: unknown; replayReasons?: unknown } | null,
): string {
  const replayClass = replayClassLabel(
    detail?.replayClass || entry.replayClass,
  );
  if (!replayClass) return "-";
  const reasons = normalizeTags(detail?.replayReasons ?? entry.replayReasons)
    .slice(0, 3)
    .map((reason) => reason.split("_").join(" "));
  return reasons.length
    ? `${replayClass} - ${reasons.join(", ")}`
    : replayClass;
}
