export type ValidationEvidenceAvailability =
  | "not_requested"
  | "available"
  | "unavailable"
  | "skipped"
  | "unknown";

export interface ValidationEvidenceModuleSummary {
  id: string;
  label: string;
  availability: ValidationEvidenceAvailability;
  detail: string;
}

export interface ValidationEvidenceSummary {
  status: "not_run" | "incomplete" | "available";
  summaryLine: string;
  requestedBackend: string;
  realizedBackend: string;
  route: string;
  fallbackReason: string;
  replayClass: string;
  reportState: "not_generated" | "generated" | "unknown";
  datasetIdentity: string;
  modules: ValidationEvidenceModuleSummary[];
}

export interface BuildValidationEvidenceSummaryInput {
  validationReq?: unknown;
  validationRes?: unknown;
  datasetMeta?: unknown;
  reportPath?: unknown;
  replayClass?: unknown;
}

const MODULES: Array<{
  id: string;
  label: string;
  requestKeys: string[];
  resultKeys: string[];
}> = [
  {
    id: "walk_forward",
    label: "Walk-forward",
    requestKeys: ["walk_forward", "walkForward"],
    resultKeys: ["walk_forward", "walkForward"],
  },
  {
    id: "reopt_walk_forward",
    label: "Re-opt walk-forward",
    requestKeys: ["reopt_walk_forward", "reoptWalkForward"],
    resultKeys: ["reopt_walk_forward", "reoptWalkForward"],
  },
  {
    id: "cscv_pbo",
    label: "CSCV/PBO",
    requestKeys: ["cscv_pbo", "cscvPbo"],
    resultKeys: ["cscv_pbo", "cscvPbo"],
  },
  {
    id: "spa",
    label: "SPA",
    requestKeys: ["spa"],
    resultKeys: ["spa"],
  },
  {
    id: "monte_carlo",
    label: "Monte Carlo",
    requestKeys: ["monte_carlo", "monteCarlo", "trade_bootstrap", "tradeBootstrap"],
    resultKeys: ["monte_carlo", "monteCarlo", "trade_bootstrap", "tradeBootstrap"],
  },
  {
    id: "cross_ticker",
    label: "Cross-ticker",
    requestKeys: ["cross_ticker", "crossTicker"],
    resultKeys: ["cross_ticker", "crossTicker"],
  },
];

function text(value: unknown): string {
  return String(value ?? "").trim();
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function firstText(...values: unknown[]): string {
  for (const value of values) {
    const next = text(value);
    if (next) return next;
  }
  return "";
}

function firstRecord(
  row: Record<string, unknown>,
  keys: string[],
): Record<string, unknown> | null {
  for (const key of keys) {
    const next = asRecord(row[key]);
    if (Object.keys(next).length > 0) return next;
  }
  return null;
}

function hasRequestModule(row: Record<string, unknown>, keys: string[]): boolean {
  return keys.some((key) => {
    const value = row[key];
    return value != null && value !== false;
  });
}

function moduleSkippedReason(row: Record<string, unknown> | null): string {
  if (!row) return "";
  return firstText(
    row.skipped_reason,
    row.skippedReason,
    row.unavailable_reason,
    row.unavailableReason,
  );
}

function buildModuleSummary(
  validationReq: Record<string, unknown>,
  validationRes: Record<string, unknown>,
  module: (typeof MODULES)[number],
): ValidationEvidenceModuleSummary {
  const requested = hasRequestModule(validationReq, module.requestKeys);
  const result = firstRecord(validationRes, module.resultKeys);
  if (!requested) {
    return {
      id: module.id,
      label: module.label,
      availability: "not_requested",
      detail: "not requested",
    };
  }
  const skippedReason = moduleSkippedReason(result);
  if (skippedReason) {
    return {
      id: module.id,
      label: module.label,
      availability: "skipped",
      detail: skippedReason,
    };
  }
  if (result) {
    return {
      id: module.id,
      label: module.label,
      availability: "available",
      detail: "result present",
    };
  }
  return {
    id: module.id,
    label: module.label,
    availability: "unavailable",
    detail: "requested but result missing",
  };
}

function datasetIdentityFromMeta(datasetMeta: Record<string, unknown>): string {
  return (
    firstText(
      datasetMeta.dataset_id,
      datasetMeta.datasetId,
      datasetMeta.source_content_fingerprint,
      datasetMeta.sourceContentFingerprint,
      datasetMeta.csv_path,
      datasetMeta.csvPath,
      datasetMeta.path,
    ) || "unknown"
  );
}

export function buildValidationEvidenceSummary(
  input: BuildValidationEvidenceSummaryInput,
): ValidationEvidenceSummary {
  const validationReq = asRecord(input.validationReq);
  const validationRes = asRecord(input.validationRes);
  const datasetMeta = asRecord(input.datasetMeta);

  if (Object.keys(validationRes).length === 0) {
    return {
      status: "not_run",
      summaryLine: "Validation evidence not run.",
      requestedBackend: firstText(asRecord(validationReq.backend).backend),
      realizedBackend: "",
      route: "",
      fallbackReason: "",
      replayClass: text(input.replayClass) || "unknown",
      reportState: text(input.reportPath) ? "generated" : "not_generated",
      datasetIdentity: datasetIdentityFromMeta(datasetMeta),
      modules: MODULES.map((module) =>
        buildModuleSummary(validationReq, validationRes, module),
      ),
    };
  }

  const requestedBackend = firstText(
    validationRes.requested_backend,
    asRecord(validationReq.backend).backend,
  );
  const realizedBackend = firstText(
    validationRes.realized_backend,
    validationRes.backend_used,
  );
  const route = firstText(validationRes.backend_route);
  const fallbackReason = firstText(validationRes.backend_fallback_reason);
  const replayClass = text(input.replayClass) || "unknown";
  const reportState = text(input.reportPath) ? "generated" : "not_generated";
  const modules = MODULES.map((module) =>
    buildModuleSummary(validationReq, validationRes, module),
  );
  const availableCount = modules.filter(
    (module) => module.availability === "available",
  ).length;
  const requestedCount = modules.filter(
    (module) => module.availability !== "not_requested",
  ).length;
  const missingCount = modules.filter(
    (module) =>
      module.availability === "unavailable" ||
      module.availability === "skipped",
  ).length;
  const status = missingCount > 0 ? "incomplete" : "available";
  const routeLabel =
    realizedBackend || route || requestedBackend || "backend unknown";
  const summaryLine =
    requestedCount > 0
      ? `Validation evidence ${status}; ${availableCount}/${requestedCount} requested modules available; route ${routeLabel}.`
      : `Validation evidence ${status}; no optional evidence modules requested; route ${routeLabel}.`;

  return {
    status,
    summaryLine,
    requestedBackend,
    realizedBackend,
    route,
    fallbackReason,
    replayClass,
    reportState,
    datasetIdentity: datasetIdentityFromMeta(datasetMeta),
    modules,
  };
}
