export type ValidationEvidenceAvailability =
  | "not_requested"
  | "available"
  | "unavailable"
  | "skipped"
  | "not_computed"
  | "not_comparable"
  | "unknown";

export interface ValidationEvidenceModuleSummary {
  id: string;
  label: string;
  availability: ValidationEvidenceAvailability;
  detail: string;
}

export type ValidationOverfitEvidenceTone =
  | "neutral"
  | "good"
  | "warn"
  | "bad";

export type ValidationOverfitEvidenceStatus =
  | "not_run"
  | "available"
  | "attention";

export interface ValidationOverfitEvidenceItem {
  id: string;
  label: string;
  value: string;
  detail: string;
  tone: ValidationOverfitEvidenceTone;
}

export interface ValidationOverfitEvidenceSummary {
  status: ValidationOverfitEvidenceStatus;
  statusLabel: string;
  summaryLine: string;
  items: ValidationOverfitEvidenceItem[];
}

export type ValidationDataRealismStatus = "complete" | "incomplete";

export interface ValidationDataRealismEvidenceItem {
  id:
    | "price_adjustment"
    | "corporate_actions"
    | "survivorship_bias"
    | "liquidity_capacity"
    | "market_impact"
    | "borrow_availability";
  label: string;
  value: string;
  blocking: boolean;
  detail: string;
}

export interface ValidationDataRealismSummary {
  status: ValidationDataRealismStatus;
  complete: boolean;
  summaryLine: string;
  missingOrUnknown: string[];
  items: ValidationDataRealismEvidenceItem[];
}

export type ValidationPromotionEligibilityState =
  | "blocked"
  | "research_only"
  | "eligible_for_review";

export interface ValidationPromotionEligibilitySummary {
  eligible: boolean;
  state: ValidationPromotionEligibilityState;
  summaryLine: string;
  reason: string;
  gatesPassed: string[];
  gatesFailed: string[];
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
  overfit: ValidationOverfitEvidenceSummary;
  dataRealism: ValidationDataRealismSummary;
  promotionEligibility: ValidationPromotionEligibilitySummary;
}

export interface BuildValidationEvidenceSummaryInput {
  validationReq?: unknown;
  validationRes?: unknown;
  validationEvidencePack?: unknown;
  optimizationRes?: unknown;
  datasetMeta?: unknown;
  reportPath?: unknown;
  replayClass?: unknown;
}

interface NormalizedValidationEvidencePack {
  schemaVersion: string;
  version: number | null;
  workflow: string;
  status: string;
  statusReason: string;
  route: Record<string, unknown>;
  candidateUniverse: Record<string, unknown>;
  verdict: Record<string, unknown>;
  dataset: Record<string, unknown>;
  promotionEligibility: Record<string, unknown>;
  metrics: Record<string, unknown>[];
  modules: Record<string, unknown>[];
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

function numberOrNull(value: unknown): number | null {
  const next = Number(value);
  return Number.isFinite(next) ? next : null;
}

function integerOrNull(value: unknown): number | null {
  const next = numberOrNull(value);
  if (next == null) return null;
  return Math.max(0, Math.floor(next));
}

function formatCount(value: unknown): string {
  const next = integerOrNull(value);
  if (next == null) return "-";
  return new Intl.NumberFormat("en-US").format(next);
}

function formatDecimal(value: unknown, digits = 3): string {
  const next = numberOrNull(value);
  if (next == null) return "-";
  return next.toFixed(digits);
}

function formatProbability(value: unknown, digits = 1): string {
  const next = numberOrNull(value);
  if (next == null) return "-";
  return `${(next * 100).toFixed(digits)}%`;
}

function formatGate(value: unknown): string {
  const next = numberOrNull(value);
  if (next == null) return "";
  return next.toFixed(2);
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

function normalizeAvailability(
  value: unknown,
): ValidationEvidenceAvailability {
  const next = text(value) as ValidationEvidenceAvailability;
  if (
    next === "available" ||
    next === "not_requested" ||
    next === "unavailable" ||
    next === "skipped" ||
    next === "not_computed" ||
    next === "not_comparable"
  ) {
    return next;
  }
  return "unknown";
}

const DATA_REALISM_FIELDS: Array<{
  id: ValidationDataRealismEvidenceItem["id"];
  label: string;
  priceAdjustment: boolean;
}> = [
  {
    id: "price_adjustment",
    label: "Price adjustment",
    priceAdjustment: true,
  },
  {
    id: "corporate_actions",
    label: "Corporate actions",
    priceAdjustment: false,
  },
  {
    id: "survivorship_bias",
    label: "Survivorship bias",
    priceAdjustment: false,
  },
  {
    id: "liquidity_capacity",
    label: "Liquidity / capacity",
    priceAdjustment: false,
  },
  {
    id: "market_impact",
    label: "Market impact",
    priceAdjustment: false,
  },
  {
    id: "borrow_availability",
    label: "Borrow availability",
    priceAdjustment: false,
  },
];

function stringList(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((entry) => text(entry))
    .filter(
      (entry, index, entries) =>
        entry.length > 0 && entries.indexOf(entry) === index,
    );
}

function normalizeDataRealismValue(
  value: unknown,
  priceAdjustment: boolean,
): string {
  const normalized = text(value).toLowerCase();
  if (priceAdjustment) {
    return normalized === "adjusted" ||
      normalized === "unadjusted" ||
      normalized === "not_applicable"
      ? normalized
      : "unknown";
  }
  return normalized === "documented" || normalized === "not_applicable"
    ? normalized
    : "unknown";
}

function dataRealismDetail(value: string, priceAdjustment: boolean): string {
  if (value === "not_applicable") return "not applicable";
  if (value === "unknown") return "missing or unknown";
  if (priceAdjustment) return `${value} prices documented`;
  return "documented";
}

function buildDataRealismSummary(
  pack: NormalizedValidationEvidencePack | null,
): ValidationDataRealismSummary {
  const dataset = pack?.dataset ?? {};
  const dataRealism = asRecord(
    dataset.data_realism ?? dataset.dataRealism,
  );
  const items = DATA_REALISM_FIELDS.map((field) => {
    const value = normalizeDataRealismValue(
      dataRealism[field.id],
      field.priceAdjustment,
    );
    return {
      id: field.id,
      label: field.label,
      value,
      blocking: value === "unknown",
      detail: dataRealismDetail(value, field.priceAdjustment),
    };
  });
  const derivedMissing = items
    .filter((item) => item.blocking)
    .map((item) => item.id);
  const reportedMissing = stringList(
    dataRealism.missing_or_unknown ?? dataRealism.missingOrUnknown,
  );
  const missingOrUnknown = [...reportedMissing, ...derivedMissing].filter(
    (entry, index, entries) => entries.indexOf(entry) === index,
  );
  const complete =
    dataRealism.complete === true &&
    missingOrUnknown.length === 0 &&
    items.every((item) => !item.blocking);
  const priceAdjustment =
    items.find((item) => item.id === "price_adjustment")?.value ?? "unknown";
  const missingLabels = missingOrUnknown.map((entry) => {
    const normalized = entry.trim().toLowerCase();
    return (
      DATA_REALISM_FIELDS.find((field) => field.id === normalized)?.label ??
      entry.replace(/_/g, " ")
    );
  });
  const summaryLine = complete
    ? `Data realism complete; price adjustment ${priceAdjustment}; corporate-action, survivorship, liquidity/capacity, market-impact, and borrow evidence documented or not applicable.`
    : missingLabels.length > 0
      ? `Data realism incomplete; missing/unknown ${missingLabels.join(", ")}.`
      : "Data realism incomplete; the evidence pack did not mark the contract complete.";
  return {
    status: complete ? "complete" : "incomplete",
    complete,
    summaryLine,
    missingOrUnknown,
    items,
  };
}

function normalizePromotionState(
  value: unknown,
): ValidationPromotionEligibilityState {
  const normalized = text(value);
  if (
    normalized === "research_only" ||
    normalized === "eligible_for_review"
  ) {
    return normalized;
  }
  return "blocked";
}

function buildPromotionEligibilitySummary(
  pack: NormalizedValidationEvidencePack | null,
  dataRealism: ValidationDataRealismSummary,
  validationPresent: boolean,
): ValidationPromotionEligibilitySummary {
  const raw = pack?.promotionEligibility ?? {};
  const reportedState = normalizePromotionState(raw.state);
  const gatesPassed = stringList(raw.gates_passed ?? raw.gatesPassed);
  const gatesFailed = stringList(raw.gates_failed ?? raw.gatesFailed);
  const reportedReason = firstText(raw.reason);
  const rawPresent = Object.keys(raw).length > 0;
  const eligible =
    raw.eligible === true &&
    reportedState === "eligible_for_review" &&
    dataRealism.complete &&
    gatesFailed.length === 0;
  const state: ValidationPromotionEligibilityState = eligible
    ? "eligible_for_review"
    : !dataRealism.complete || gatesFailed.length > 0
      ? "blocked"
      : reportedState === "research_only"
        ? "research_only"
        : "blocked";
  const reason = !validationPresent
    ? "validation evidence has not run"
    : !dataRealism.complete
      ? `data-realism evidence is incomplete${
          dataRealism.missingOrUnknown.length > 0
            ? ` (${dataRealism.missingOrUnknown.join(", ")})`
            : ""
        }`
      : gatesFailed.length > 0
        ? `promotion gates failed (${gatesFailed.join(", ")})`
        : !rawPresent
          ? "promotion eligibility was not reported"
          : reportedReason ||
            (state === "research_only"
              ? "backend classified this artifact as research-only"
              : eligible
                ? "backend gates permit promotion review"
                : "backend did not mark this artifact eligible for review");
  const stateLabel =
    state === "eligible_for_review"
      ? "eligible for review"
      : state === "research_only"
        ? "research-only"
        : "blocked";
  return {
    eligible,
    state,
    summaryLine: `Promotion ${stateLabel}; ${reason}. Statistical validation approval is not deployability or deployment approval.`,
    reason,
    gatesPassed,
    gatesFailed,
  };
}

function appendRealismAndPromotionSummary(
  base: string,
  dataRealism: ValidationDataRealismSummary,
  promotionEligibility: ValidationPromotionEligibilitySummary,
): string {
  return [
    text(base),
    dataRealism.summaryLine,
    promotionEligibility.summaryLine,
  ]
    .filter((part) => part.length > 0)
    .join(" ");
}

function validationEvidencePackFromInput(
  input: BuildValidationEvidenceSummaryInput,
  validationRes: Record<string, unknown>,
): NormalizedValidationEvidencePack | null {
  const raw =
    input.validationEvidencePack ??
    validationRes.validation_evidence_pack ??
    validationRes.validationEvidencePack;
  const pack = asRecord(raw);
  if (Object.keys(pack).length === 0) return null;
  const version = integerOrNull(pack.version);
  if (version != null && version > 1) return null;
  const schemaVersion = firstText(pack.schema_version, pack.schemaVersion);
  if (version == null && !schemaVersion) return null;
  const candidateUniverse = asRecord(
    pack.candidate_universe ?? pack.candidateUniverse,
  );
  return {
    schemaVersion,
    version,
    workflow: firstText(pack.workflow),
    status: firstText(pack.status),
    statusReason: firstText(pack.status_reason, pack.statusReason),
    route: asRecord(pack.route),
    candidateUniverse,
    verdict: asRecord(pack.verdict),
    dataset: asRecord(pack.dataset),
    promotionEligibility: asRecord(
      pack.promotion_eligibility ?? pack.promotionEligibility,
    ),
    metrics: Array.isArray(pack.metrics)
      ? pack.metrics.map((entry) => asRecord(entry))
      : [],
    modules: Array.isArray(pack.modules)
      ? pack.modules.map((entry) => asRecord(entry))
      : [],
  };
}

function packMetric(
  pack: NormalizedValidationEvidencePack,
  name: string,
): Record<string, unknown> {
  return (
    pack.metrics.find((metric) => firstText(metric.metric) === name) ?? {}
  );
}

function packModule(
  pack: NormalizedValidationEvidencePack,
  name: string,
): Record<string, unknown> {
  return (
    pack.modules.find(
      (module) => firstText(module.module, module.id) === name,
    ) ?? {}
  );
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

function buildPackModuleSummary(
  pack: NormalizedValidationEvidencePack,
  fallbackReq: Record<string, unknown>,
  fallbackRes: Record<string, unknown>,
  module: (typeof MODULES)[number],
): ValidationEvidenceModuleSummary {
  const packModuleRow = packModule(pack, module.id);
  if (Object.keys(packModuleRow).length === 0) {
    return buildModuleSummary(fallbackReq, fallbackRes, module);
  }
  const availability = normalizeAvailability(packModuleRow.availability);
  const detail =
    firstText(
      packModuleRow.skipped_reason,
      packModuleRow.skippedReason,
      packModuleRow.unavailable_reason,
      packModuleRow.unavailableReason,
      packModuleRow.summary,
    ) || availability.replace(/_/g, " ");
  return {
    id: module.id,
    label: module.label,
    availability,
    detail,
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

function validationCandidates(
  validationRes: Record<string, unknown>,
): Record<string, unknown>[] {
  return Array.isArray(validationRes.candidates)
    ? validationRes.candidates
        .map((entry) => asRecord(entry))
        .filter((entry) => Object.keys(entry).length > 0)
    : [];
}

function topCandidateForEvidence(
  validationRes: Record<string, unknown>,
): Record<string, unknown> {
  const candidates = validationCandidates(validationRes);
  const verdict = asRecord(validationRes.run_verdict);
  const firstApprovedRank = integerOrNull(verdict.first_approved_rank);
  if (firstApprovedRank != null && firstApprovedRank > 0) {
    return candidates[firstApprovedRank - 1] ?? candidates[0] ?? {};
  }
  return candidates[0] ?? {};
}

function requestModuleSummary(
  validationReq: Record<string, unknown>,
  moduleId: string,
): boolean {
  const module = MODULES.find((entry) => entry.id === moduleId);
  return module ? hasRequestModule(validationReq, module.requestKeys) : false;
}

function toneForGate(
  value: number | null,
  gate: number | null,
  direction: "min" | "max",
): ValidationOverfitEvidenceTone {
  if (value == null) return gate == null ? "neutral" : "warn";
  if (gate == null) return "neutral";
  if (direction === "min") return value >= gate ? "good" : "bad";
  return value <= gate ? "good" : "bad";
}

function evidenceStatusFromItems(
  validationRes: Record<string, unknown>,
  items: ValidationOverfitEvidenceItem[],
): ValidationOverfitEvidenceStatus {
  if (Object.keys(validationRes).length === 0) return "not_run";
  return items.some((item) => item.tone === "bad" || item.tone === "warn")
    ? "attention"
    : "available";
}

function statusLabel(status: ValidationOverfitEvidenceStatus): string {
  if (status === "not_run") return "Not run";
  if (status === "attention") return "Needs review";
  return "Evidence available";
}

function resultModuleRecord(
  validationRes: Record<string, unknown>,
  moduleId: string,
): Record<string, unknown> {
  const module = MODULES.find((entry) => entry.id === moduleId);
  return module ? (firstRecord(validationRes, module.resultKeys) ?? {}) : {};
}

function buildVerdictEvidenceItem(
  validationRes: Record<string, unknown>,
): ValidationOverfitEvidenceItem {
  const candidates = validationCandidates(validationRes);
  const verdict = asRecord(validationRes.run_verdict);
  if (Object.keys(verdict).length === 0) {
    return {
      id: "verdict",
      label: "Run Verdict",
      value: "not reported",
      detail: `${formatCount(candidates.length)} candidates returned`,
      tone: candidates.length > 0 ? "neutral" : "warn",
    };
  }
  const approved = verdict.approved === true;
  const passed = verdict.passed !== false;
  const warningFlags = Array.isArray(verdict.warning_flags)
    ? verdict.warning_flags.map(text).filter((entry) => entry.length > 0)
    : [];
  const failReasons = Array.isArray(verdict.fail_reasons)
    ? verdict.fail_reasons.map(text).filter((entry) => entry.length > 0)
    : [];
  const tested = integerOrNull(verdict.candidates_tested) ?? candidates.length;
  const survived = integerOrNull(verdict.candidates_survived_hard_gates) ?? 0;
  const approvedCount = integerOrNull(verdict.approved_candidate_count) ?? 0;
  const recommendation = firstText(verdict.final_recommendation);
  const firstNote = failReasons[0] || warningFlags[0] || recommendation;
  return {
    id: "verdict",
    label: "Run Verdict",
    value: approved ? "approved" : passed ? "not approved" : "failed",
    detail: [
      `hard gates ${formatCount(survived)}/${formatCount(tested)}`,
      `approved ${formatCount(approvedCount)}/${formatCount(survived)}`,
      firstNote,
    ]
      .filter((part) => part.length > 0)
      .join(" · "),
    tone: !passed || !approved ? "bad" : warningFlags.length ? "warn" : "good",
  };
}

function buildSearchEvidenceItem(
  optimizationRes: Record<string, unknown>,
  validationReq: Record<string, unknown>,
  validationRes: Record<string, unknown>,
): ValidationOverfitEvidenceItem {
  const candidates = validationCandidates(validationRes);
  const requestedCandidates =
    integerOrNull(validationReq.candidates) ??
    integerOrNull(validationReq.top_n) ??
    integerOrNull(validationReq.topN);
  const searchTrials =
    integerOrNull(optimizationRes.num_combos) ??
    integerOrNull(optimizationRes.numCombos) ??
    integerOrNull(optimizationRes.evaluations) ??
    integerOrNull(optimizationRes.evaluated);
  const admission = firstText(validationReq.admission_policy);
  return {
    id: "search",
    label: "Search Exposure",
    value:
      searchTrials != null
        ? `${formatCount(searchTrials)} trials`
        : "trials unknown",
    detail: [
      `validated ${formatCount(candidates.length)}${requestedCandidates != null ? `/${formatCount(requestedCandidates)}` : ""} candidates`,
      admission ? `admission ${admission}` : "",
    ]
      .filter((part) => part.length > 0)
      .join(" · "),
    tone: candidates.length <= 1 ? "warn" : "neutral",
  };
}

function buildProbabilityEvidenceItem(
  validationReq: Record<string, unknown>,
  validationRes: Record<string, unknown>,
): ValidationOverfitEvidenceItem {
  const verdict = asRecord(validationRes.run_verdict);
  const candidate = topCandidateForEvidence(validationRes);
  const psr =
    numberOrNull(verdict.top_candidate_psr) ?? numberOrNull(candidate.psr);
  const dsr =
    numberOrNull(verdict.top_candidate_dsr) ?? numberOrNull(candidate.dsr);
  const minPsr = numberOrNull(validationReq.min_psr);
  const minDsr = numberOrNull(validationReq.min_dsr);
  const psrTone = toneForGate(psr, minPsr, "min");
  const dsrTone = toneForGate(dsr, minDsr, "min");
  const tone =
    psrTone === "bad" || dsrTone === "bad"
      ? "bad"
      : psrTone === "warn" || dsrTone === "warn"
        ? "warn"
        : psrTone === "good" || dsrTone === "good"
          ? "good"
          : "neutral";
  const gateDetail = [
    minPsr != null ? `PSR >= ${formatGate(minPsr)}` : "",
    minDsr != null ? `DSR >= ${formatGate(minDsr)}` : "",
  ]
    .filter((part) => part.length > 0)
    .join(", ");
  return {
    id: "probability",
    label: "PSR / DSR",
    value:
      psr != null || dsr != null
        ? `PSR ${formatDecimal(psr)} · DSR ${formatDecimal(dsr)}`
        : "not reported",
    detail: gateDetail ? `gates ${gateDetail}` : "no probability gate set",
    tone,
  };
}

function buildPboEvidenceItem(
  validationReq: Record<string, unknown>,
  validationRes: Record<string, unknown>,
): ValidationOverfitEvidenceItem {
  const requested = requestModuleSummary(validationReq, "cscv_pbo");
  const cscv = resultModuleRecord(validationRes, "cscv_pbo");
  const verdict = asRecord(validationRes.run_verdict);
  const skippedReason = moduleSkippedReason(cscv);
  const pbo = numberOrNull(cscv.pbo) ?? numberOrNull(verdict.pbo);
  const maxPbo =
    numberOrNull(validationReq.max_pbo) ?? numberOrNull(verdict.max_pbo);
  if (skippedReason) {
    return {
      id: "pbo",
      label: "CSCV / PBO",
      value: "skipped",
      detail: skippedReason,
      tone: requested ? "warn" : "neutral",
    };
  }
  if (pbo == null) {
    return {
      id: "pbo",
      label: "CSCV / PBO",
      value: requested ? "missing" : "not requested",
      detail: requested ? "requested but no PBO result was returned" : "no CSCV/PBO request",
      tone: requested ? "warn" : "neutral",
    };
  }
  const combinations = integerOrNull(cscv.combinations);
  const combinationsPossible = integerOrNull(cscv.combinations_possible);
  const comboDetail =
    combinations != null && combinationsPossible != null
      ? `${formatCount(combinations)}/${formatCount(combinationsPossible)} combinations`
      : combinations != null
        ? `${formatCount(combinations)} combinations`
        : "";
  const tone =
    maxPbo != null
      ? toneForGate(pbo, maxPbo, "max")
      : pbo > 0.3
        ? "warn"
        : "good";
  return {
    id: "pbo",
    label: "CSCV / PBO",
    value: formatProbability(pbo),
    detail: [
      maxPbo != null ? `max ${formatProbability(maxPbo)}` : "",
      comboDetail,
    ]
      .filter((part) => part.length > 0)
      .join(" · "),
    tone,
  };
}

function buildSpaEvidenceItem(
  validationReq: Record<string, unknown>,
  validationRes: Record<string, unknown>,
): ValidationOverfitEvidenceItem {
  const requested = requestModuleSummary(validationReq, "spa");
  const spa = resultModuleRecord(validationRes, "spa");
  const skippedReason = moduleSkippedReason(spa);
  if (skippedReason) {
    return {
      id: "spa",
      label: "SPA",
      value: "skipped",
      detail: skippedReason,
      tone: requested ? "warn" : "neutral",
    };
  }
  if (Object.keys(spa).length === 0) {
    return {
      id: "spa",
      label: "SPA",
      value: requested ? "missing" : "not requested",
      detail: requested ? "requested but no SPA result was returned" : "no SPA request",
      tone: requested ? "warn" : "neutral",
    };
  }
  const pValue = numberOrNull(spa.p_value);
  const passed = spa.passed !== false;
  const slices = integerOrNull(spa.slice_count);
  const samples = integerOrNull(spa.bootstrap_samples);
  return {
    id: "spa",
    label: "SPA",
    value: `${passed ? "pass" : "fail"}${pValue != null ? ` · p=${formatDecimal(pValue)}` : ""}`,
    detail: [
      slices != null ? `${formatCount(slices)} slices` : "",
      samples != null ? `${formatCount(samples)} samples` : "",
    ]
      .filter((part) => part.length > 0)
      .join(" · "),
    tone: passed ? "good" : "bad",
  };
}

function buildTrackRecordEvidenceItem(
  validationRes: Record<string, unknown>,
): ValidationOverfitEvidenceItem {
  const verdict = asRecord(validationRes.run_verdict);
  const candidate = topCandidateForEvidence(validationRes);
  const observed =
    integerOrNull(verdict.top_candidate_track_record_bars) ??
    integerOrNull(candidate.track_record_bars);
  const required =
    integerOrNull(verdict.top_candidate_min_track_record_length_bars) ??
    integerOrNull(candidate.min_track_record_length_bars);
  if (observed == null && required == null) {
    return {
      id: "track_record",
      label: "Track Record",
      value: "not reported",
      detail: "minimum track-record length was not returned",
      tone: "neutral",
    };
  }
  const passes = observed != null && required != null ? observed >= required : true;
  return {
    id: "track_record",
    label: "Track Record",
    value:
      observed != null
        ? `${formatCount(observed)}${required != null ? `/${formatCount(required)} bars` : " bars"}`
        : `min ${formatCount(required)} bars`,
    detail:
      observed != null && required != null
        ? passes
          ? "meets minimum evidence length"
          : "below minimum evidence length"
        : "minimum evidence length is advisory",
    tone: observed != null && required != null ? (passes ? "good" : "bad") : "neutral",
  };
}

function buildWalkForwardEvidenceItem(
  validationReq: Record<string, unknown>,
  validationRes: Record<string, unknown>,
): ValidationOverfitEvidenceItem {
  const candidate = topCandidateForEvidence(validationRes);
  const walkForward = asRecord(candidate.walk_forward);
  const reopt = resultModuleRecord(validationRes, "reopt_walk_forward");
  const walkRequested = requestModuleSummary(validationReq, "walk_forward");
  const reoptRequested = requestModuleSummary(validationReq, "reopt_walk_forward");
  const wfWindows = integerOrNull(walkForward.windows);
  const reoptWindows = integerOrNull(reopt.windows);
  const wfSkipped = moduleSkippedReason(walkForward);
  const reoptSkipped = moduleSkippedReason(reopt);
  if (wfSkipped || reoptSkipped) {
    return {
      id: "walk_forward",
      label: "Walk-forward",
      value: "skipped",
      detail: wfSkipped || reoptSkipped,
      tone: walkRequested || reoptRequested ? "warn" : "neutral",
    };
  }
  if (wfWindows == null && reoptWindows == null) {
    return {
      id: "walk_forward",
      label: "Walk-forward",
      value: walkRequested || reoptRequested ? "missing" : "not requested",
      detail:
        walkRequested || reoptRequested
          ? "requested but no walk-forward evidence was returned"
          : "no walk-forward request",
      tone: walkRequested || reoptRequested ? "warn" : "neutral",
    };
  }
  const worstSharpe =
    numberOrNull(walkForward.sharpe_worst) ??
    numberOrNull(reopt.forward_sharpe_worst);
  const worstPnl =
    numberOrNull(walkForward.pnl_worst) ?? numberOrNull(reopt.forward_pnl_worst);
  return {
    id: "walk_forward",
    label: "Walk-forward",
    value: [
      wfWindows != null ? `WF ${formatCount(wfWindows)}` : "",
      reoptWindows != null ? `reopt ${formatCount(reoptWindows)}` : "",
    ]
      .filter((part) => part.length > 0)
      .join(" · "),
    detail: [
      worstSharpe != null ? `worst Sharpe ${formatDecimal(worstSharpe)}` : "",
      worstPnl != null ? `worst PnL ${formatDecimal(worstPnl)}` : "",
    ]
      .filter((part) => part.length > 0)
      .join(" · "),
    tone: worstSharpe != null && worstSharpe < 0 ? "warn" : "good",
  };
}

function buildDistributionEvidenceItem(
  validationReq: Record<string, unknown>,
  validationRes: Record<string, unknown>,
): ValidationOverfitEvidenceItem {
  const candidate = topCandidateForEvidence(validationRes);
  const monteCarlo = asRecord(candidate.monte_carlo);
  const bootstrap = asRecord(candidate.trade_bootstrap);
  const requested = requestModuleSummary(validationReq, "monte_carlo");
  const mcAdvisory = asRecord(monteCarlo.advisory);
  if (Object.keys(monteCarlo).length > 0) {
    const failReasons = Array.isArray(mcAdvisory.fail_reasons)
      ? mcAdvisory.fail_reasons.map(text).filter((entry) => entry.length > 0)
      : [];
    const warnings = Array.isArray(mcAdvisory.warning_flags)
      ? mcAdvisory.warning_flags.map(text).filter((entry) => entry.length > 0)
      : [];
    const passed = mcAdvisory.passed !== false;
    return {
      id: "distribution",
      label: "Distribution",
      value: `${formatCount(monteCarlo.simulations)} MC sims`,
      detail: [
        `${formatCount(monteCarlo.trade_count)} trades`,
        failReasons[0] || warnings[0] || "",
      ]
        .filter((part) => part.length > 0)
        .join(" · "),
      tone: !passed || failReasons.length > 0 ? "bad" : warnings.length > 0 ? "warn" : "good",
    };
  }
  if (Object.keys(bootstrap).length > 0) {
    return {
      id: "distribution",
      label: "Distribution",
      value: `${formatCount(bootstrap.simulations)} bootstrap`,
      detail: [
        `${formatCount(bootstrap.trade_count)} trades`,
        `PnL+ ${formatProbability(bootstrap.prob_pnl_positive)}`,
        `p05 ${formatDecimal(bootstrap.pnl_p05)}`,
      ].join(" · "),
      tone:
        numberOrNull(bootstrap.prob_pnl_positive) != null &&
        Number(bootstrap.prob_pnl_positive) < 0.5
          ? "warn"
          : "good",
    };
  }
  return {
    id: "distribution",
    label: "Distribution",
    value: requested ? "missing" : "not requested",
    detail: requested
      ? "requested but no Monte Carlo or bootstrap result was returned"
      : "no distribution stress request",
    tone: requested ? "warn" : "neutral",
  };
}

function buildFrictionEvidenceItem(
  validationRes: Record<string, unknown>,
): ValidationOverfitEvidenceItem {
  const candidate = topCandidateForEvidence(validationRes);
  const friction = asRecord(candidate.friction_stress);
  if (Object.keys(friction).length === 0) {
    return {
      id: "friction",
      label: "Friction",
      value: "not reported",
      detail: "no fee/slippage stress result",
      tone: "neutral",
    };
  }
  const passCount = integerOrNull(friction.pass_count) ?? 0;
  const scenarioCount = integerOrNull(friction.scenario_count) ?? 0;
  const passFrac = numberOrNull(friction.pass_frac);
  const worst = firstText(friction.worst_scenario_label);
  return {
    id: "friction",
    label: "Friction",
    value:
      scenarioCount > 0
        ? `${formatCount(passCount)}/${formatCount(scenarioCount)} pass`
        : `${formatProbability(passFrac)} pass`,
    detail: worst ? `worst ${worst}` : "fee/slippage stress evaluated",
    tone:
      scenarioCount > 0 && passCount < scenarioCount
        ? "warn"
        : passFrac != null && passFrac < 1
          ? "warn"
          : "good",
  };
}

function toneForAvailability(
  availability: ValidationEvidenceAvailability,
  requested: boolean,
): ValidationOverfitEvidenceTone {
  if (availability === "available") return "good";
  if (availability === "not_requested") return "neutral";
  if (availability === "skipped" || availability === "unavailable") {
    return requested ? "warn" : "neutral";
  }
  if (availability === "not_comparable") return "warn";
  return "neutral";
}

function packMetricAvailability(
  metric: Record<string, unknown>,
): ValidationEvidenceAvailability {
  return normalizeAvailability(metric.availability);
}

function packRequested(module: Record<string, unknown>): boolean {
  return module.requested === true;
}

function packMetricTone(
  metric: Record<string, unknown>,
  direction: "min" | "max",
): ValidationOverfitEvidenceTone {
  const availability = packMetricAvailability(metric);
  const value = numberOrNull(metric.value);
  const threshold = numberOrNull(metric.threshold);
  if (availability !== "available") {
    return toneForAvailability(availability, availability !== "not_requested");
  }
  return toneForGate(value, threshold, direction);
}

function buildPackVerdictEvidenceItem(
  pack: NormalizedValidationEvidencePack,
): ValidationOverfitEvidenceItem {
  const approved = pack.verdict.approved === true;
  const passed = pack.verdict.passed !== false;
  const evaluated =
    integerOrNull(pack.candidateUniverse.evaluated_candidates) ??
    integerOrNull(pack.candidateUniverse.evaluatedCandidates);
  const survived =
    integerOrNull(pack.candidateUniverse.survived_hard_gates) ??
    integerOrNull(pack.candidateUniverse.survivedHardGates);
  const approvedCount =
    integerOrNull(pack.candidateUniverse.approved_candidates) ??
    integerOrNull(pack.candidateUniverse.approvedCandidates);
  const warnings = Array.isArray(pack.verdict.warning_flags)
    ? pack.verdict.warning_flags.map(text).filter((entry) => entry.length > 0)
    : [];
  const blockers = Array.isArray(pack.verdict.fail_reasons)
    ? pack.verdict.fail_reasons.map(text).filter((entry) => entry.length > 0)
    : [];
  const recommendation = firstText(
    pack.verdict.recommendation,
    pack.verdict.final_recommendation,
    pack.verdict.finalRecommendation,
  );
  return {
    id: "verdict",
    label: "Run Verdict",
    value: approved ? "approved" : passed ? "not approved" : "failed",
    detail: [
      `hard gates ${formatCount(survived)}/${formatCount(evaluated)}`,
      `approved ${formatCount(approvedCount)}/${formatCount(survived)}`,
      blockers[0] || warnings[0] || recommendation,
    ]
      .filter((part) => text(part).length > 0)
      .join(" · "),
    tone:
      !passed || blockers.length > 0
        ? "bad"
        : approved && warnings.length === 0
          ? "good"
          : "warn",
  };
}

function buildPackSearchEvidenceItem(
  pack: NormalizedValidationEvidencePack,
): ValidationOverfitEvidenceItem {
  const trials =
    integerOrNull(pack.candidateUniverse.optimization_trials) ??
    integerOrNull(pack.candidateUniverse.optimizationTrials);
  const requested =
    integerOrNull(pack.candidateUniverse.requested_candidates) ??
    integerOrNull(pack.candidateUniverse.requestedCandidates);
  const evaluated =
    integerOrNull(pack.candidateUniverse.evaluated_candidates) ??
    integerOrNull(pack.candidateUniverse.evaluatedCandidates);
  const retained =
    integerOrNull(pack.candidateUniverse.retained_candidates) ??
    integerOrNull(pack.candidateUniverse.retainedCandidates);
  const source = firstText(pack.candidateUniverse.source);
  return {
    id: "search",
    label: "Search Exposure",
    value:
      trials != null
        ? `${formatCount(trials)} trials`
        : `${formatCount(evaluated)} validated`,
    detail: [
      `retained ${formatCount(retained)}${requested != null ? `/${formatCount(requested)}` : ""}`,
      source,
    ]
      .filter((part) => text(part).length > 0)
      .join(" · "),
    tone: (evaluated ?? retained ?? 0) <= 1 ? "warn" : "neutral",
  };
}

function buildPackProbabilityEvidenceItem(
  pack: NormalizedValidationEvidencePack,
): ValidationOverfitEvidenceItem {
  const psr = packMetric(pack, "psr");
  const dsr = packMetric(pack, "dsr");
  const psrTone = packMetricTone(psr, "min");
  const dsrTone = packMetricTone(dsr, "min");
  const tone =
    psrTone === "bad" || dsrTone === "bad"
      ? "bad"
      : psrTone === "warn" || dsrTone === "warn"
        ? "warn"
        : psrTone === "good" || dsrTone === "good"
          ? "good"
          : "neutral";
  const psrValue = numberOrNull(psr.value);
  const dsrValue = numberOrNull(dsr.value);
  const psrGate = numberOrNull(psr.threshold);
  const dsrGate = numberOrNull(dsr.threshold);
  return {
    id: "probability",
    label: "PSR / DSR",
    value:
      psrValue != null || dsrValue != null
        ? `PSR ${formatDecimal(psrValue)} · DSR ${formatDecimal(dsrValue)}`
        : "not reported",
    detail: [
      psrGate != null ? `PSR >= ${formatGate(psrGate)}` : "",
      dsrGate != null ? `DSR >= ${formatGate(dsrGate)}` : "",
      firstText(psr.sample_unit, dsr.sample_unit),
    ]
      .filter((part) => text(part).length > 0)
      .join(" · "),
    tone,
  };
}

function buildPackPboEvidenceItem(
  pack: NormalizedValidationEvidencePack,
): ValidationOverfitEvidenceItem {
  const metric = packMetric(pack, "cscv_pbo");
  const module = packModule(pack, "cscv_pbo");
  const availability = normalizeAvailability(
    metric.availability ?? module.availability,
  );
  const requested = packRequested(module);
  const skippedReason = firstText(
    module.skipped_reason,
    module.skippedReason,
    metric.unavailable_reason,
    metric.unavailableReason,
  );
  const value = numberOrNull(metric.value ?? module.value);
  const threshold = numberOrNull(metric.threshold ?? module.threshold);
  if (availability !== "available") {
    return {
      id: "pbo",
      label: "CSCV / PBO",
      value:
        availability === "not_requested"
          ? "not requested"
          : availability.replace(/_/g, " "),
      detail: skippedReason || firstText(module.summary, metric.comparability_basis),
      tone: toneForAvailability(availability, requested),
    };
  }
  return {
    id: "pbo",
    label: "CSCV / PBO",
    value: formatProbability(value),
    detail: [
      threshold != null ? `max ${formatProbability(threshold)}` : "",
      integerOrNull(module.combinations) != null
        ? `${formatCount(module.combinations)} combinations`
        : "",
    ]
      .filter((part) => text(part).length > 0)
      .join(" · "),
    tone: toneForGate(value, threshold, "max"),
  };
}

function buildPackSpaEvidenceItem(
  pack: NormalizedValidationEvidencePack,
): ValidationOverfitEvidenceItem {
  const metric = packMetric(pack, "spa_p_value");
  const module = packModule(pack, "spa");
  const availability = normalizeAvailability(
    metric.availability ?? module.availability,
  );
  const requested = packRequested(module);
  const pValue = numberOrNull(metric.value ?? module.p_value ?? module.pValue);
  const summary = firstText(module.summary);
  if (availability !== "available") {
    return {
      id: "spa",
      label: "SPA",
      value:
        availability === "not_requested"
          ? "not requested"
          : availability.replace(/_/g, " "),
      detail:
        firstText(module.skipped_reason, module.skippedReason, metric.unavailable_reason) ||
        summary,
      tone: toneForAvailability(availability, requested),
    };
  }
  const passed = summary === "passed";
  return {
    id: "spa",
    label: "SPA",
    value: `${passed ? "pass" : "review"}${pValue != null ? ` · p=${formatDecimal(pValue)}` : ""}`,
    detail: [
      integerOrNull(module.slice_count) != null
        ? `${formatCount(module.slice_count)} slices`
        : "",
      integerOrNull(module.candidate_count) != null
        ? `${formatCount(module.candidate_count)} candidates`
        : "",
    ]
      .filter((part) => text(part).length > 0)
      .join(" · "),
    tone: passed ? "good" : "warn",
  };
}

function buildPackTrackRecordEvidenceItem(
  pack: NormalizedValidationEvidencePack,
): ValidationOverfitEvidenceItem {
  const metric = packMetric(pack, "min_track_record_length");
  const observed = integerOrNull(metric.value);
  const required = integerOrNull(metric.threshold);
  if (observed == null && required == null) {
    return {
      id: "track_record",
      label: "Track Record",
      value: "not reported",
      detail:
        firstText(metric.unavailable_reason) ||
        "minimum track-record length was not returned",
      tone: "neutral",
    };
  }
  const passes = observed != null && required != null ? observed >= required : true;
  return {
    id: "track_record",
    label: "Track Record",
    value:
      observed != null
        ? `${formatCount(observed)}${required != null ? `/${formatCount(required)} bars` : " bars"}`
        : `min ${formatCount(required)} bars`,
    detail: passes
      ? "meets minimum evidence length"
      : "below minimum evidence length",
    tone: passes ? "good" : "bad",
  };
}

function buildPackOverfitEvidenceSummary(
  pack: NormalizedValidationEvidencePack,
): ValidationOverfitEvidenceSummary {
  const items: ValidationOverfitEvidenceItem[] = [
    buildPackVerdictEvidenceItem(pack),
    buildPackSearchEvidenceItem(pack),
    buildPackProbabilityEvidenceItem(pack),
    buildPackPboEvidenceItem(pack),
    buildPackSpaEvidenceItem(pack),
    buildPackTrackRecordEvidenceItem(pack),
  ];
  const status = items.some((item) => item.tone === "bad" || item.tone === "warn")
    ? "attention"
    : "available";
  const headline = [
    items.find((item) => item.id === "verdict")?.value,
    items.find((item) => item.id === "search")?.value,
    items.find((item) => item.id === "pbo")?.value !== "not requested"
      ? `PBO ${items.find((item) => item.id === "pbo")?.value}`
      : "",
    items.find((item) => item.id === "probability")?.value,
  ]
    .map((part) => text(part))
    .filter((part) => part.length > 0)
    .join("; ");
  return {
    status,
    statusLabel: statusLabel(status),
    summaryLine: `Overfit evidence ${status === "attention" ? "needs review" : "available"}${headline ? `; ${headline}` : ""}.`,
    items,
  };
}

export function buildValidationOverfitEvidenceSummary(
  input: BuildValidationEvidenceSummaryInput,
): ValidationOverfitEvidenceSummary {
  const validationReq = asRecord(input.validationReq);
  const validationRes = asRecord(input.validationRes);
  const optimizationRes = asRecord(input.optimizationRes);
  const pack = validationEvidencePackFromInput(input, validationRes);
  if (pack) return buildPackOverfitEvidenceSummary(pack);
  if (Object.keys(validationRes).length === 0) {
    return {
      status: "not_run",
      statusLabel: "Not run",
      summaryLine: "Overfit evidence not run.",
      items: [],
    };
  }

  const items: ValidationOverfitEvidenceItem[] = [
    buildVerdictEvidenceItem(validationRes),
    buildSearchEvidenceItem(optimizationRes, validationReq, validationRes),
    buildProbabilityEvidenceItem(validationReq, validationRes),
    buildPboEvidenceItem(validationReq, validationRes),
    buildSpaEvidenceItem(validationReq, validationRes),
    buildTrackRecordEvidenceItem(validationRes),
    buildWalkForwardEvidenceItem(validationReq, validationRes),
    buildDistributionEvidenceItem(validationReq, validationRes),
    buildFrictionEvidenceItem(validationRes),
  ];
  const status = evidenceStatusFromItems(validationRes, items);
  const headline = [
    items.find((item) => item.id === "verdict")?.value,
    items.find((item) => item.id === "search")?.value,
    items.find((item) => item.id === "pbo")?.value !== "not requested"
      ? `PBO ${items.find((item) => item.id === "pbo")?.value}`
      : "",
    items.find((item) => item.id === "probability")?.value,
  ]
    .map((part) => text(part))
    .filter((part) => part.length > 0)
    .join("; ");
  return {
    status,
    statusLabel: statusLabel(status),
    summaryLine: `Overfit evidence ${status === "attention" ? "needs review" : "available"}${headline ? `; ${headline}` : ""}.`,
    items,
  };
}

export function buildValidationEvidenceSummary(
  input: BuildValidationEvidenceSummaryInput,
): ValidationEvidenceSummary {
  const validationReq = asRecord(input.validationReq);
  const validationRes = asRecord(input.validationRes);
  const datasetMeta = asRecord(input.datasetMeta);
  const pack = validationEvidencePackFromInput(input, validationRes);
  const overfit = buildValidationOverfitEvidenceSummary(input);
  const validationPresent =
    Object.keys(validationRes).length > 0 || pack != null;
  const dataRealism = buildDataRealismSummary(pack);
  const promotionEligibility = buildPromotionEligibilitySummary(
    pack,
    dataRealism,
    validationPresent,
  );

  if (!validationPresent) {
    return {
      status: "not_run",
      summaryLine: appendRealismAndPromotionSummary(
        "Validation evidence not run.",
        dataRealism,
        promotionEligibility,
      ),
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
      overfit,
      dataRealism,
      promotionEligibility,
    };
  }

  if (pack) {
    const modules = MODULES.map((module) =>
      buildPackModuleSummary(pack, validationReq, validationRes, module),
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
        module.availability === "skipped" ||
        module.availability === "not_comparable",
    ).length;
    const status =
      pack.status === "incomplete" || missingCount > 0
        ? "incomplete"
        : "available";
    const requestedBackend = firstText(
      pack.route.requested_backend,
      pack.route.requestedBackend,
    );
    const realizedBackend = firstText(
      pack.route.realized_backend,
      pack.route.realizedBackend,
    );
    const route = firstText(pack.route.route_label, pack.route.routeLabel);
    const fallbackReason = firstText(
      pack.route.fallback_reason,
      pack.route.fallbackReason,
    );
    const routeLabel =
      realizedBackend || route || requestedBackend || "backend unknown";
    const packLabel = [
      pack.schemaVersion ? `schema ${pack.schemaVersion}` : "",
      pack.version != null ? `v${pack.version}` : "",
    ]
      .filter((part) => part.length > 0)
      .join(" ");
    const summaryLine =
      requestedCount > 0
        ? `Validation evidence ${status}; pack ${packLabel || "present"}; ${availableCount}/${requestedCount} requested modules available; route ${routeLabel}.`
        : `Validation evidence ${status}; pack ${packLabel || "present"}; no optional evidence modules requested; route ${routeLabel}.`;
    return {
      status,
      summaryLine: appendRealismAndPromotionSummary(
        summaryLine,
        dataRealism,
        promotionEligibility,
      ),
      requestedBackend,
      realizedBackend,
      route,
      fallbackReason,
      replayClass: text(input.replayClass) || "unknown",
      reportState: text(input.reportPath) ? "generated" : "not_generated",
      datasetIdentity: datasetIdentityFromMeta(datasetMeta),
      modules,
      overfit,
      dataRealism,
      promotionEligibility,
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
    summaryLine: appendRealismAndPromotionSummary(
      summaryLine,
      dataRealism,
      promotionEligibility,
    ),
    requestedBackend,
    realizedBackend,
    route,
    fallbackReason,
    replayClass,
    reportState,
    datasetIdentity: datasetIdentityFromMeta(datasetMeta),
    modules,
    overfit,
    dataRealism,
    promotionEligibility,
  };
}
