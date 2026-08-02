export type VcetCampaignEvidenceMetricKey =
  | "profitable_share_requested"
  | "hard_gate_pass_share_requested"
  | "pnl_p10"
  | "sharpe_p10"
  | "missing_coverage";

export type VcetCampaignEvidenceTone =
  | "favorable"
  | "neutral"
  | "risk"
  | "unavailable";

export type VcetCampaignEvidenceParameterAxisKey =
  | "vidya_short_period"
  | "vidya_long_period"
  | "vidya_alpha"
  | "ce_period"
  | "ce_mult"
  | "ce_use_close"
  | "require_vidya_slope"
  | "max_trade_bars";

export interface VcetCampaignEffectiveControls {
  vidyaShortPeriod: number;
  vidyaLongPeriod: number;
  vidyaAlpha: number;
  cePeriod: number;
  ceMult: number;
  ceUseClose: boolean;
  requireVidyaSlope: boolean;
  maxTradeBars: number;
}

export interface VcetCampaignEvidenceParameterAxis {
  key: VcetCampaignEvidenceParameterAxisKey;
  label: string;
  shortLabel: string;
  description: string;
  format: "integer" | "number" | "boolean";
}

export interface VcetCampaignEvidenceMetricDefinition {
  key: VcetCampaignEvidenceMetricKey;
  label: string;
  shortLabel: string;
  direction: "higher" | "lower";
  format: "percent" | "number";
  description: string;
}

export interface VcetCampaignEvidenceCandidate {
  candidateId: string;
  fastLen: number;
  slowLen: number;
  requestedDatasets: number;
  evaluatedDatasets: number;
  missingOrFailedDatasets: number;
  profitableShareRequested: number | null;
  hardGatePassShareRequested: number | null;
  pnlP10: number | null;
  sharpeP10: number | null;
  maxDrawdownP90: number | null;
  selected: boolean;
  admitted: boolean | null;
  searchCandidateId: string;
  searchOrdinal: number | null;
  effective: VcetCampaignEffectiveControls | null;
}

export interface VcetCampaignEvidenceCell {
  key: string;
  fastLen: number;
  slowLen: number;
  candidate: VcetCampaignEvidenceCandidate;
  aliasedCandidateCount: number;
}

export interface VcetCampaignEvidenceModel {
  signature: string;
  state: "ready" | "degraded" | "empty";
  designMode: "period_grid" | "halton";
  schemaVersion: number;
  cohort: "training" | "legacy" | "unavailable";
  cohortLabel: string;
  candidateManifestHash: string;
  requestedDatasets: number;
  evaluatedDatasets: number;
  missingOrFailedDatasets: number;
  selectedCandidateId: string;
  admissionSource: "artifact" | "derived" | "unavailable";
  admittedCandidateCount: number;
  searchId: string;
  searchSequence: string;
  searchRequestedBudget: number | null;
  searchEffectiveBudget: number | null;
  validSearchSpaceSize: string;
  searchGenerationStats: VcetCampaignEvidenceSearchGenerationStats | null;
  candidates: VcetCampaignEvidenceCandidate[];
  cells: VcetCampaignEvidenceCell[];
  fastValues: number[];
  slowValues: number[];
  visualGridAvailable: boolean;
  visualGridUnavailableReason: string;
  warnings: string[];
}

export interface VcetCampaignEvidenceSearchGenerationStats {
  haltonAttempts: number;
  invalidPeriodPairsDiscarded: number;
  duplicateCandidatesDiscarded: number;
  lexicographicFallbackCandidates: number;
  exhaustiveCandidates: number;
}

export interface VcetCampaignEvidenceMetricCell {
  cell: VcetCampaignEvidenceCell;
  value: number | null;
  displayValue: string;
  tone: VcetCampaignEvidenceTone;
  intensity: 0 | 1 | 2 | 3 | 4;
  ariaLabel: string;
}

export interface VcetCampaignEvidenceMetricView {
  metric: VcetCampaignEvidenceMetricDefinition;
  cells: VcetCampaignEvidenceMetricCell[];
  valueByCellKey: ReadonlyMap<string, VcetCampaignEvidenceMetricCell>;
  finiteMinimum: number | null;
  finiteMaximum: number | null;
  finiteCellCount: number;
}

export interface VcetCampaignEvidencePlateau {
  state: "broad" | "developing" | "isolated" | "insufficient";
  focusCandidateId: string;
  focusLabel: string;
  stableNeighbors: number;
  evaluatedNeighbors: number;
  totalNeighborCells: number;
  summary: string;
}

export interface VcetCampaignEvidenceScatterPoint {
  candidate: VcetCampaignEvidenceCandidate;
  metricCell: VcetCampaignEvidenceMetricCell;
  xValue: number;
  yValue: number;
  xRatio: number;
  yRatio: number;
  xLabel: string;
  yLabel: string;
  ariaLabel: string;
}

export interface VcetCampaignEvidenceScatterView {
  xAxis: VcetCampaignEvidenceParameterAxis;
  yAxis: VcetCampaignEvidenceParameterAxis;
  points: VcetCampaignEvidenceScatterPoint[];
  omittedCandidateCount: number;
  xMinimum: number | null;
  xMaximum: number | null;
  yMinimum: number | null;
  yMaximum: number | null;
}

export interface VcetCampaignEvidenceScatterSample {
  points: VcetCampaignEvidenceScatterPoint[];
  totalPointCount: number;
  omittedPointCount: number;
}

export const VCET_CAMPAIGN_EVIDENCE_METRICS: readonly VcetCampaignEvidenceMetricDefinition[] =
  Object.freeze([
    {
      key: "profitable_share_requested",
      label: "Profitability share",
      shortLabel: "Profitable",
      direction: "higher",
      format: "percent",
      description:
        "Share of every requested dataset with positive net PnL; missing and failed datasets remain in the denominator.",
    },
    {
      key: "hard_gate_pass_share_requested",
      label: "Hard-gate share",
      shortLabel: "Hard gate",
      direction: "higher",
      format: "percent",
      description:
        "Share of every requested dataset that passed the configured validation hard gates.",
    },
    {
      key: "pnl_p10",
      label: "PnL p10",
      shortLabel: "PnL p10",
      direction: "higher",
      format: "number",
      description:
        "Requested-universe lower-tail net PnL. Schema-v4 missing-tail rules can make this unavailable.",
    },
    {
      key: "sharpe_p10",
      label: "Sharpe p10",
      shortLabel: "Sharpe p10",
      direction: "higher",
      format: "number",
      description:
        "Requested-universe lower-tail Sharpe ratio. Schema-v4 missing-tail rules can make this unavailable.",
    },
    {
      key: "missing_coverage",
      label: "Missing coverage",
      shortLabel: "Missing",
      direction: "lower",
      format: "percent",
      description:
        "Share of requested datasets that were missing or failed for the candidate; lower is better.",
    },
  ] satisfies VcetCampaignEvidenceMetricDefinition[]);

export const VCET_CAMPAIGN_EVIDENCE_PARAMETER_AXES: readonly VcetCampaignEvidenceParameterAxis[] =
  Object.freeze([
    {
      key: "vidya_short_period",
      label: "VIDYA short period",
      shortLabel: "Short",
      description: "Effective short VIDYA horizon.",
      format: "integer",
    },
    {
      key: "vidya_long_period",
      label: "VIDYA long period",
      shortLabel: "Long",
      description: "Effective long VIDYA horizon.",
      format: "integer",
    },
    {
      key: "vidya_alpha",
      label: "VIDYA alpha",
      shortLabel: "Alpha",
      description: "Effective VIDYA adaptation strength.",
      format: "number",
    },
    {
      key: "ce_period",
      label: "Chandelier period",
      shortLabel: "CE period",
      description: "Effective Chandelier Exit lookback.",
      format: "integer",
    },
    {
      key: "ce_mult",
      label: "Chandelier multiplier",
      shortLabel: "CE mult",
      description: "Effective Chandelier Exit ATR multiplier.",
      format: "number",
    },
    {
      key: "ce_use_close",
      label: "Chandelier close source",
      shortLabel: "CE close",
      description: "Whether Chandelier Exit uses close rather than high/low.",
      format: "boolean",
    },
    {
      key: "require_vidya_slope",
      label: "VIDYA slope gate",
      shortLabel: "Slope",
      description: "Whether entry requires favorable VIDYA slope.",
      format: "boolean",
    },
    {
      key: "max_trade_bars",
      label: "Maximum trade bars",
      shortLabel: "Max bars",
      description: "Effective maximum trade duration.",
      format: "integer",
    },
  ] satisfies VcetCampaignEvidenceParameterAxis[]);

const MAX_VISUAL_GRID_CELLS = 2_500;

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  return value as Record<string, unknown>;
}

function hasOwn(value: Record<string, unknown>, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(value, key);
}

function finiteNumber(value: unknown): number | null {
  if (value == null || value === "") return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function nonNegativeInteger(value: unknown): number {
  const parsed = finiteNumber(value);
  return parsed == null ? 0 : Math.max(0, Math.floor(parsed));
}

function exactNonNegativeInteger(value: unknown): number | null {
  const parsed = finiteNumber(value);
  return parsed != null && Number.isSafeInteger(parsed) && parsed >= 0
    ? parsed
    : null;
}

function positiveInteger(value: unknown): number | null {
  const parsed = finiteNumber(value);
  if (parsed == null) return null;
  const normalized = Math.floor(parsed);
  return normalized > 0 ? normalized : null;
}

function normalizedText(value: unknown): string {
  return String(value ?? "").trim();
}

function recordArray(value: unknown): Record<string, unknown>[] {
  if (!Array.isArray(value)) return [];
  return value
    .map(asRecord)
    .filter((row): row is Record<string, unknown> => row != null);
}

function configuredHoldout(summary: Record<string, unknown>): boolean {
  if (nonNegativeInteger(summary.holdout_pct) > 0) return true;
  return recordArray(summary.items).some((item) => item.is_holdout === true);
}

function metricValue(
  candidate: VcetCampaignEvidenceCandidate,
  key: VcetCampaignEvidenceMetricKey,
): number | null {
  switch (key) {
    case "profitable_share_requested":
      return candidate.profitableShareRequested;
    case "hard_gate_pass_share_requested":
      return candidate.hardGatePassShareRequested;
    case "pnl_p10":
      return candidate.pnlP10;
    case "sharpe_p10":
      return candidate.sharpeP10;
    case "missing_coverage":
      return candidate.requestedDatasets > 0
        ? candidate.missingOrFailedDatasets / candidate.requestedDatasets
        : null;
  }
}

function compareNullableDescending(
  left: number | null,
  right: number | null,
): number {
  const leftValue = left ?? Number.NEGATIVE_INFINITY;
  const rightValue = right ?? Number.NEGATIVE_INFINITY;
  return rightValue - leftValue;
}

function compareCandidates(
  left: VcetCampaignEvidenceCandidate,
  right: VcetCampaignEvidenceCandidate,
): number {
  return (
    compareNullableDescending(
      left.profitableShareRequested,
      right.profitableShareRequested,
    ) ||
    compareNullableDescending(
      left.hardGatePassShareRequested,
      right.hardGatePassShareRequested,
    ) ||
    compareNullableDescending(left.sharpeP10, right.sharpeP10) ||
    compareNullableDescending(left.pnlP10, right.pnlP10) ||
    left.candidateId.localeCompare(right.candidateId)
  );
}

function completeAdmissionPolicy(
  evaluation: Record<string, unknown>,
): {
  minProfitable: number;
  minHardGate: number;
  minPnlP10: number;
  minSharpeP10: number;
} | null {
  const policy = asRecord(evaluation.cross_universe_admission);
  if (!policy) return null;
  const minProfitable = finiteNumber(policy.min_profitable_share_requested);
  const minHardGate = finiteNumber(policy.min_hard_gate_pass_share_requested);
  const minPnlP10 = finiteNumber(policy.min_pnl_p10);
  const minSharpeP10 = finiteNumber(policy.min_sharpe_p10);
  if (
    minProfitable == null ||
    minHardGate == null ||
    minPnlP10 == null ||
    minSharpeP10 == null
  ) {
    return null;
  }
  return { minProfitable, minHardGate, minPnlP10, minSharpeP10 };
}

function derivedAdmission(
  candidate: VcetCampaignEvidenceCandidate,
  policy: NonNullable<ReturnType<typeof completeAdmissionPolicy>>,
): boolean {
  return (
    candidate.profitableShareRequested != null &&
    candidate.hardGatePassShareRequested != null &&
    candidate.pnlP10 != null &&
    candidate.sharpeP10 != null &&
    candidate.profitableShareRequested >= policy.minProfitable &&
    candidate.hardGatePassShareRequested >= policy.minHardGate &&
    candidate.pnlP10 >= policy.minPnlP10 &&
    candidate.sharpeP10 >= policy.minSharpeP10
  );
}

interface VcetSearchBinding {
  searchCandidateId: string;
  evaluationCandidateId: string;
  ordinal: number | null;
  effective: VcetCampaignEffectiveControls | null;
}

function strictBoolean(value: unknown): boolean | null {
  return typeof value === "boolean" ? value : null;
}

function effectiveControlsFromRecord(
  value: unknown,
): VcetCampaignEffectiveControls | null {
  const row = asRecord(value);
  if (!row) return null;
  const vidyaShortPeriod = positiveInteger(row.vidya_short_period);
  const vidyaLongPeriod = positiveInteger(row.vidya_long_period);
  const vidyaAlpha = finiteNumber(row.vidya_alpha);
  const cePeriod = positiveInteger(row.ce_period);
  const ceMult = finiteNumber(row.ce_mult);
  const ceUseClose = strictBoolean(row.ce_use_close);
  const requireVidyaSlope = strictBoolean(row.require_vidya_slope);
  const maxTradeBars = positiveInteger(row.max_trade_bars);
  if (
    vidyaShortPeriod == null ||
    vidyaShortPeriod < 2 ||
    vidyaLongPeriod == null ||
    vidyaLongPeriod <= vidyaShortPeriod ||
    vidyaAlpha == null ||
    vidyaAlpha < 0 ||
    vidyaAlpha > 1 ||
    cePeriod == null ||
    ceMult == null ||
    ceMult <= 0 ||
    ceUseClose == null ||
    requireVidyaSlope == null ||
    maxTradeBars == null
  ) {
    return null;
  }
  return {
    vidyaShortPeriod,
    vidyaLongPeriod,
    vidyaAlpha,
    cePeriod,
    ceMult,
    ceUseClose,
    requireVidyaSlope,
    maxTradeBars,
  };
}

function parseSearchBindings(
  searchManifest: Record<string, unknown>,
  warnings: string[],
): {
  bindings: Map<string, VcetSearchBinding>;
  searchId: string;
  sequence: string;
  requestedBudget: number | null;
  effectiveBudget: number | null;
  validSearchSpaceSize: string;
  generationStats: VcetCampaignEvidenceSearchGenerationStats | null;
} {
  const bindings = new Map<string, VcetSearchBinding>();
  let malformedRows = 0;
  let missingEvaluationIds = 0;
  let duplicateEvaluationIds = 0;
  for (const row of recordArray(searchManifest.candidates)) {
    const searchCandidateId = normalizedText(row.candidate_id);
    const evaluationCandidateId = normalizedText(row.evaluation_candidate_id);
    const effective = effectiveControlsFromRecord(row.effective);
    const ordinalValue = finiteNumber(row.ordinal);
    const ordinal =
      ordinalValue != null &&
      Number.isSafeInteger(ordinalValue) &&
      ordinalValue >= 0
        ? ordinalValue
        : null;
    if (!searchCandidateId || !effective || ordinal == null) malformedRows += 1;
    if (!evaluationCandidateId) {
      missingEvaluationIds += 1;
      continue;
    }
    if (bindings.has(evaluationCandidateId)) {
      duplicateEvaluationIds += 1;
      continue;
    }
    bindings.set(evaluationCandidateId, {
      searchCandidateId,
      evaluationCandidateId,
      ordinal,
      effective,
    });
  }
  if (malformedRows > 0) {
    warnings.push(
      `${malformedRows} search-manifest candidate row${malformedRows === 1 ? " has" : "s have"} malformed identity or effective controls.`,
    );
  }
  if (missingEvaluationIds > 0) {
    warnings.push(
      `${missingEvaluationIds} search-manifest candidate row${missingEvaluationIds === 1 ? " is" : "s are"} missing the evaluation_candidate_id binding.`,
    );
  }
  if (duplicateEvaluationIds > 0) {
    warnings.push(
      `${duplicateEvaluationIds} duplicate evaluation_candidate_id binding${duplicateEvaluationIds === 1 ? " was" : "s were"} omitted from the search manifest join.`,
    );
  }
  const requestedBudgetValue = finiteNumber(searchManifest.requested_budget);
  const effectiveBudgetValue = finiteNumber(searchManifest.effective_budget);
  const stats = asRecord(searchManifest.generation_stats);
  const haltonAttempts = exactNonNegativeInteger(stats?.halton_attempts);
  const invalidPeriodPairsDiscarded = exactNonNegativeInteger(
    stats?.invalid_period_pairs_discarded,
  );
  const duplicateCandidatesDiscarded = exactNonNegativeInteger(
    stats?.duplicate_candidates_discarded,
  );
  const lexicographicFallbackCandidates = exactNonNegativeInteger(
    stats?.lexicographic_fallback_candidates,
  );
  const exhaustiveCandidates = exactNonNegativeInteger(
    stats?.exhaustive_candidates,
  );
  const generationStats =
    haltonAttempts != null &&
    invalidPeriodPairsDiscarded != null &&
    duplicateCandidatesDiscarded != null &&
    lexicographicFallbackCandidates != null &&
    exhaustiveCandidates != null
      ? {
          haltonAttempts,
          invalidPeriodPairsDiscarded,
          duplicateCandidatesDiscarded,
          lexicographicFallbackCandidates,
          exhaustiveCandidates,
        }
      : null;
  return {
    bindings,
    searchId: normalizedText(searchManifest.search_id),
    sequence: normalizedText(searchManifest.sequence),
    requestedBudget:
      requestedBudgetValue != null &&
      Number.isSafeInteger(requestedBudgetValue) &&
      requestedBudgetValue >= 0
        ? requestedBudgetValue
        : null,
    effectiveBudget:
      effectiveBudgetValue != null &&
      Number.isSafeInteger(effectiveBudgetValue) &&
      effectiveBudgetValue >= 0
        ? effectiveBudgetValue
        : null,
    validSearchSpaceSize: normalizedText(
      searchManifest.valid_search_space_size,
    ),
    generationStats,
  };
}

function candidateFromRecord(
  row: Record<string, unknown>,
  selectedCandidateId: string,
): VcetCampaignEvidenceCandidate | null {
  const fastLen = positiveInteger(row.fast_len);
  const slowLen = positiveInteger(row.slow_len);
  if (fastLen == null || slowLen == null || fastLen >= slowLen) return null;
  const candidateId =
    normalizedText(row.candidate_id) || `vcet-${fastLen}-${slowLen}`;
  return {
    candidateId,
    fastLen,
    slowLen,
    requestedDatasets: nonNegativeInteger(row.requested_datasets),
    evaluatedDatasets: nonNegativeInteger(row.evaluated_datasets),
    missingOrFailedDatasets: nonNegativeInteger(
      row.missing_or_failed_datasets,
    ),
    profitableShareRequested: finiteNumber(row.profitable_share_requested),
    hardGatePassShareRequested: finiteNumber(
      row.hard_gate_pass_share_requested,
    ),
    pnlP10: finiteNumber(row.pnl_p10),
    sharpeP10: finiteNumber(row.sharpe_p10),
    maxDrawdownP90: finiteNumber(row.max_drawdown_p90),
    selected: candidateId === selectedCandidateId,
    admitted: null,
    searchCandidateId: "",
    searchOrdinal: null,
    effective: null,
  };
}

export function buildVcetCampaignEvidenceModel(
  summaryValue: unknown,
): VcetCampaignEvidenceModel | null {
  const summary = asRecord(summaryValue);
  if (!summary) return null;
  const evaluation = asRecord(summary.fixed_candidate_evaluation);
  if (!evaluation) return null;

  const warnings: string[] = [];
  const schemaVersion = nonNegativeInteger(evaluation.schema_version);
  const candidateManifestHash = normalizedText(
    evaluation.candidate_manifest_hash,
  );
  if (!candidateManifestHash) {
    warnings.push("The candidate-manifest hash is unavailable.");
  }
  const hasSearchManifest =
    hasOwn(evaluation, "search_manifest") &&
    evaluation.search_manifest != null;
  const searchManifest = asRecord(evaluation.search_manifest);
  const designMode: VcetCampaignEvidenceModel["designMode"] = hasSearchManifest
    ? "halton"
    : "period_grid";
  if (hasSearchManifest && !searchManifest) {
    warnings.push("The VCET search manifest is malformed.");
  }
  const parsedSearch = searchManifest
    ? parseSearchBindings(searchManifest, warnings)
    : {
        bindings: new Map<string, VcetSearchBinding>(),
        searchId: "",
        sequence: "",
        requestedBudget: null,
        effectiveBudget: null,
        validSearchSpaceSize: "",
        generationStats: null,
      };
  if (searchManifest && parsedSearch.bindings.size === 0) {
    warnings.push(
      "The multidimensional search manifest contains no usable evaluation bindings.",
    );
  }
  if (searchManifest) {
    const normalizedSequence = parsedSearch.sequence.toLowerCase();
    if (
      normalizedSequence !== "halton_cranley_patterson" &&
      normalizedSequence !== "exhaustive_lexicographic"
    ) {
      warnings.push(
        normalizedSequence === "halton"
          ? "The search manifest uses the legacy unshifted Halton sequence rather than the current deterministic sequence contract."
          : "The search manifest does not identify a supported multidimensional sequence.",
      );
    }
    if (!parsedSearch.searchId) {
      warnings.push("The search manifest ID is unavailable.");
    }
    if (
      parsedSearch.requestedBudget == null ||
      parsedSearch.effectiveBudget == null ||
      !parsedSearch.validSearchSpaceSize
    ) {
      warnings.push(
        "The search manifest is missing budget or search-space cardinality metadata.",
      );
    }
    if (
      parsedSearch.effectiveBudget != null &&
      parsedSearch.effectiveBudget !== recordArray(searchManifest.candidates).length
    ) {
      warnings.push(
        "The search manifest effective budget does not match its retained candidate rows.",
      );
    }
    if (!parsedSearch.generationStats) {
      warnings.push("The search manifest is missing generation statistics.");
    } else if (
      parsedSearch.generationStats.lexicographicFallbackCandidates !== 0
    ) {
      warnings.push(
        "The search manifest reports legacy lexicographic fallback candidates; current runs must not mix fallback candidates into a sampled design.",
      );
    }
    if (
      normalizedSequence === "exhaustive_lexicographic" &&
      parsedSearch.generationStats &&
      parsedSearch.effectiveBudget != null &&
      parsedSearch.generationStats.exhaustiveCandidates !==
        parsedSearch.effectiveBudget
    ) {
      warnings.push(
        "The exhaustive-generation count does not match the manifest effective budget.",
      );
    }
    if (
      normalizedSequence === "halton_cranley_patterson" &&
      parsedSearch.generationStats &&
      parsedSearch.generationStats.exhaustiveCandidates !== 0
    ) {
      warnings.push(
        "The sampled search manifest unexpectedly reports exhaustive candidates.",
      );
    }
  }
  const selectedCandidateId = normalizedText(evaluation.selected_candidate_id);
  const hasSeparatedCohorts =
    hasOwn(evaluation, "training") || hasOwn(evaluation, "holdout");
  const training = asRecord(evaluation.training);
  let cohort: VcetCampaignEvidenceModel["cohort"] = "unavailable";
  let cohortRecord: Record<string, unknown> | null = null;

  if (hasSeparatedCohorts) {
    if (training) {
      cohort = "training";
      cohortRecord = training;
    } else {
      warnings.push(
        "The artifact declares separated cohorts but its training aggregate is unavailable.",
      );
    }
  } else if (!configuredHoldout(summary)) {
    cohort = "legacy";
    cohortRecord = evaluation;
    warnings.push(
      "Legacy aggregate: candidate evidence is not backed by an explicit training-cohort object.",
    );
  } else {
    warnings.push(
      "Legacy aggregate includes a configured holdout, so its combined candidates are intentionally not visualized as selection evidence.",
    );
  }

  const sourceRows = recordArray(cohortRecord?.candidates);
  const candidates: VcetCampaignEvidenceCandidate[] = [];
  let invalidCandidateCount = 0;
  for (const row of sourceRows) {
    const candidate = candidateFromRecord(row, selectedCandidateId);
    if (candidate) candidates.push(candidate);
    else invalidCandidateCount += 1;
  }
  if (invalidCandidateCount > 0) {
    warnings.push(
      `${invalidCandidateCount} candidate row${invalidCandidateCount === 1 ? " was" : "s were"} omitted because Fast/Slow coordinates were invalid.`,
    );
  }

  if (designMode === "halton") {
    const matchedEvaluationIds = new Set<string>();
    let missingBindings = 0;
    let inconsistentPeriods = 0;
    for (const candidate of candidates) {
      const binding = parsedSearch.bindings.get(candidate.candidateId);
      if (!binding) {
        missingBindings += 1;
        continue;
      }
      matchedEvaluationIds.add(candidate.candidateId);
      candidate.searchCandidateId = binding.searchCandidateId;
      candidate.searchOrdinal = binding.ordinal;
      candidate.effective = binding.effective;
      if (
        binding.effective &&
        (candidate.fastLen !== binding.effective.vidyaShortPeriod ||
          candidate.slowLen !== binding.effective.vidyaLongPeriod)
      ) {
        inconsistentPeriods += 1;
      }
    }
    if (missingBindings > 0) {
      warnings.push(
        `${missingBindings} aggregate candidate${missingBindings === 1 ? " has" : "s have"} no evaluation_candidate_id match in the search manifest.`,
      );
    }
    const unmatchedBindings = Array.from(parsedSearch.bindings.keys()).filter(
      (candidateId) => !matchedEvaluationIds.has(candidateId),
    );
    if (unmatchedBindings.length > 0) {
      warnings.push(
        `${unmatchedBindings.length} sealed search candidate${unmatchedBindings.length === 1 ? " has" : "s have"} no selection-safe aggregate row.`,
      );
    }
    if (inconsistentPeriods > 0) {
      warnings.push(
        `${inconsistentPeriods} aggregate candidate${inconsistentPeriods === 1 ? " disagrees" : "s disagree"} with the bound effective VIDYA periods.`,
      );
    }
  }

  let admissionSource: VcetCampaignEvidenceModel["admissionSource"] =
    "unavailable";
  const admittedIdsPresent = hasOwn(evaluation, "admitted_candidate_ids");
  if (
    admittedIdsPresent &&
    !Array.isArray(evaluation.admitted_candidate_ids)
  ) {
    warnings.push("The artifact's admitted-candidate ID list is malformed.");
  }
  const admittedIds = new Set(
    Array.isArray(evaluation.admitted_candidate_ids)
      ? evaluation.admitted_candidate_ids
          .map(normalizedText)
          .filter((value) => value.length > 0)
      : [],
  );
  if (admittedIdsPresent && Array.isArray(evaluation.admitted_candidate_ids)) {
    admissionSource = "artifact";
    for (const candidate of candidates) {
      candidate.admitted = admittedIds.has(candidate.candidateId);
    }
    const declaredAdmittedCount = nonNegativeInteger(
      evaluation.admitted_candidate_count,
    );
    if (declaredAdmittedCount !== admittedIds.size) {
      warnings.push(
        `The artifact's admitted-candidate count (${declaredAdmittedCount}) does not match its admitted ID list (${admittedIds.size}).`,
      );
    }
    const retainedCandidateIds = new Set(
      candidates.map((candidate) => candidate.candidateId),
    );
    const missingAdmittedIds = Array.from(admittedIds).filter(
      (candidateId) => !retainedCandidateIds.has(candidateId),
    );
    if (missingAdmittedIds.length > 0) {
      warnings.push(
        `${missingAdmittedIds.length} admitted candidate ID${missingAdmittedIds.length === 1 ? " is" : "s are"} absent from the selection-safe candidate rows.`,
      );
    }
  } else if (
    schemaVersion >= 4 &&
    finiteNumber(evaluation.admitted_candidate_count) === 0
  ) {
    admissionSource = "artifact";
    for (const candidate of candidates) candidate.admitted = false;
  } else {
    const policy = completeAdmissionPolicy(evaluation);
    if (policy) {
      admissionSource = "derived";
      for (const candidate of candidates) {
        candidate.admitted = derivedAdmission(candidate, policy);
      }
      warnings.push(
        "Per-candidate admission markers were reconstructed from the artifact thresholds because admitted IDs were not retained.",
      );
    }
  }
  if (admissionSource === "unavailable" && candidates.length > 0) {
    warnings.push(
      "Per-candidate admission identity is unavailable because neither admitted IDs nor a complete threshold policy was retained.",
    );
  }

  if (
    selectedCandidateId &&
    !candidates.some((candidate) => candidate.selected)
  ) {
    warnings.push(
      "The backend-selected candidate is not present in the selection-safe candidate rows.",
    );
  }

  const cells: VcetCampaignEvidenceCell[] = [];
  let aliasedCoordinateCount = 0;
  if (designMode === "halton") {
    const seenCandidateIds = new Set<string>();
    for (const candidate of candidates) {
      if (seenCandidateIds.has(candidate.candidateId)) {
        warnings.push(
          `Duplicate aggregate candidate identity ${candidate.candidateId} was omitted from the multidimensional view.`,
        );
        continue;
      }
      seenCandidateIds.add(candidate.candidateId);
      cells.push({
        key: candidate.candidateId,
        fastLen: candidate.fastLen,
        slowLen: candidate.slowLen,
        candidate,
        aliasedCandidateCount: 1,
      });
    }
  } else {
    const byCoordinate = new Map<string, VcetCampaignEvidenceCandidate[]>();
    for (const candidate of candidates) {
      const key = `${candidate.fastLen}:${candidate.slowLen}`;
      const rows = byCoordinate.get(key) ?? [];
      rows.push(candidate);
      byCoordinate.set(key, rows);
    }
    for (const [key, rows] of byCoordinate.entries()) {
      rows.sort(compareCandidates);
      const selected = rows.find((candidate) => candidate.selected);
      const candidate = selected ?? rows[0];
      if (!candidate) continue;
      if (rows.length > 1) aliasedCoordinateCount += 1;
      cells.push({
        key,
        fastLen: candidate.fastLen,
        slowLen: candidate.slowLen,
        candidate,
        aliasedCandidateCount: rows.length,
      });
    }
  }
  if (aliasedCoordinateCount > 0) {
    warnings.push(
      `${aliasedCoordinateCount} Fast/Slow cell${aliasedCoordinateCount === 1 ? " contains" : "s contain"} multiple candidate identities; each cell shows the selected candidate or the strongest retained row.`,
    );
  }

  cells.sort((left, right) => {
    if (designMode === "halton") {
      return (
        (left.candidate.searchOrdinal ?? Number.MAX_SAFE_INTEGER) -
          (right.candidate.searchOrdinal ?? Number.MAX_SAFE_INTEGER) ||
        left.candidate.candidateId.localeCompare(right.candidate.candidateId)
      );
    }
    return left.slowLen - right.slowLen || left.fastLen - right.fastLen;
  });
  const fastValues = Array.from(
    new Set(cells.map((cell) => cell.fastLen)),
  ).sort((left, right) => left - right);
  const slowValues = Array.from(
    new Set(cells.map((cell) => cell.slowLen)),
  ).sort((left, right) => left - right);
  const visualGridCellCount = fastValues.length * slowValues.length;
  const visualGridAvailable =
    designMode === "period_grid" &&
    cells.length > 0 &&
    visualGridCellCount <= MAX_VISUAL_GRID_CELLS;
  const visualGridUnavailableReason =
    cells.length === 0
      ? "No selection-safe candidate cells are available."
      : designMode === "halton"
        ? "The sealed multidimensional design varies eight effective controls, so it is not collapsed into a potentially aliased Fast×Slow heatmap. Use the parameter scatter and complete candidate table."
      : visualGridAvailable
        ? ""
        : `The sparse Fast×Slow domain expands to ${visualGridCellCount.toLocaleString()} visual cells, above the ${MAX_VISUAL_GRID_CELLS.toLocaleString()} interactive limit. Use the candidate table instead.`;
  if (
    designMode === "period_grid" &&
    !visualGridAvailable &&
    cells.length > 0
  ) {
    warnings.push(visualGridUnavailableReason);
  }

  const requestedDatasets = nonNegativeInteger(
    cohortRecord?.requested_datasets ?? evaluation.requested_datasets,
  );
  const evaluatedDatasets = nonNegativeInteger(
    cohortRecord?.evaluated_datasets ?? evaluation.evaluated_datasets,
  );
  const missingOrFailedDatasets = nonNegativeInteger(
    cohortRecord?.missing_or_failed_datasets ??
      evaluation.missing_or_failed_datasets,
  );
  if (requestedDatasets === 0 && candidates.length > 0) {
    warnings.push(
      "Candidate rows are present, but the requested-dataset denominator is zero or unavailable.",
    );
  }
  const admittedCandidateCount = candidates.filter(
    (candidate) => candidate.admitted === true,
  ).length;
  if (
    schemaVersion < 4 &&
    !warnings.some((warning) => warning.startsWith("Legacy"))
  ) {
    warnings.push(
      `Schema v${schemaVersion || "unknown"} predates the complete schema-v4 evidence contract; review this visualization as degraded evidence.`,
    );
  }
  const state: VcetCampaignEvidenceModel["state"] =
    cells.length === 0
      ? "empty"
      : warnings.length > 0 || schemaVersion < 4
        ? "degraded"
        : "ready";

  return {
    signature: [
      candidateManifestHash || "unknown",
      parsedSearch.searchId || designMode,
      schemaVersion,
      cohort,
      selectedCandidateId || "none",
      cells.length,
      requestedDatasets,
      evaluatedDatasets,
    ].join(":"),
    state,
    designMode,
    schemaVersion,
    cohort,
    cohortLabel:
      cohort === "training"
        ? "Training cohort"
        : cohort === "legacy"
          ? "Legacy no-holdout aggregate"
          : "Selection cohort unavailable",
    candidateManifestHash,
    requestedDatasets,
    evaluatedDatasets,
    missingOrFailedDatasets,
    selectedCandidateId,
    admissionSource,
    admittedCandidateCount,
    searchId: parsedSearch.searchId,
    searchSequence: parsedSearch.sequence,
    searchRequestedBudget: parsedSearch.requestedBudget,
    searchEffectiveBudget: parsedSearch.effectiveBudget,
    validSearchSpaceSize: parsedSearch.validSearchSpaceSize,
    searchGenerationStats: parsedSearch.generationStats,
    candidates: candidates.slice().sort((left, right) =>
      designMode === "halton"
        ? (left.searchOrdinal ?? Number.MAX_SAFE_INTEGER) -
            (right.searchOrdinal ?? Number.MAX_SAFE_INTEGER) ||
          left.candidateId.localeCompare(right.candidateId)
        : compareCandidates(left, right),
    ),
    cells,
    fastValues,
    slowValues,
    visualGridAvailable,
    visualGridUnavailableReason,
    warnings,
  };
}

function metricDefinition(
  key: VcetCampaignEvidenceMetricKey,
): VcetCampaignEvidenceMetricDefinition {
  const metric = VCET_CAMPAIGN_EVIDENCE_METRICS.find(
    (candidate) => candidate.key === key,
  );
  if (!metric) throw new Error(`Unsupported VCET evidence metric: ${key}`);
  return metric;
}

export function formatVcetCampaignEvidenceValue(
  value: number | null,
  metric: VcetCampaignEvidenceMetricDefinition,
): string {
  if (value == null || !Number.isFinite(value)) return "Unavailable";
  if (metric.format === "percent") return `${(value * 100).toFixed(1)}%`;
  return value.toFixed(metric.key === "pnl_p10" ? 6 : 4);
}

export function formatVcetCampaignEvidenceCandidateMetric(
  candidate: VcetCampaignEvidenceCandidate,
  metricKey: VcetCampaignEvidenceMetricKey,
): string {
  const metric = metricDefinition(metricKey);
  return formatVcetCampaignEvidenceValue(
    metricValue(candidate, metricKey),
    metric,
  );
}

function metricVisual(
  value: number | null,
  metric: VcetCampaignEvidenceMetricDefinition,
  finiteMinimum: number | null,
  finiteMaximum: number | null,
): { tone: VcetCampaignEvidenceTone; intensity: 0 | 1 | 2 | 3 | 4 } {
  if (value == null || !Number.isFinite(value)) {
    return { tone: "unavailable", intensity: 0 };
  }

  if (metric.format === "percent") {
    const favorableRatio = Math.max(
      0,
      Math.min(1, metric.direction === "lower" ? 1 - value : value),
    );
    if (favorableRatio >= 0.5) {
      return {
        tone: "favorable",
        intensity: Math.max(1, Math.ceil((favorableRatio - 0.5) * 8)) as
          | 1
          | 2
          | 3
          | 4,
      };
    }
    return {
      tone: "risk",
      intensity: Math.max(1, Math.ceil((0.5 - favorableRatio) * 8)) as
        | 1
        | 2
        | 3
        | 4,
    };
  }

  if (value === 0) return { tone: "neutral", intensity: 0 };
  const anchor = Math.max(
    Math.abs(finiteMinimum ?? 0),
    Math.abs(finiteMaximum ?? 0),
    Number.EPSILON,
  );
  return {
    tone: value > 0 ? "favorable" : "risk",
    intensity: Math.max(1, Math.ceil(Math.min(1, Math.abs(value) / anchor) * 4)) as
      | 1
      | 2
      | 3
      | 4,
  };
}

export function buildVcetCampaignEvidenceMetricView(
  model: VcetCampaignEvidenceModel,
  metricKey: VcetCampaignEvidenceMetricKey,
): VcetCampaignEvidenceMetricView {
  const metric = metricDefinition(metricKey);
  const values = model.cells
    .map((cell) => metricValue(cell.candidate, metric.key))
    .filter((value): value is number => value != null && Number.isFinite(value));
  const finiteMinimum = values.length > 0 ? Math.min(...values) : null;
  const finiteMaximum = values.length > 0 ? Math.max(...values) : null;
  const cells = model.cells.map((cell) => {
    const value = metricValue(cell.candidate, metric.key);
    const displayValue = formatVcetCampaignEvidenceValue(value, metric);
    const visual = metricVisual(
      value,
      metric,
      finiteMinimum,
      finiteMaximum,
    );
    const markers = [
      cell.candidate.selected ? "backend selected" : "",
      cell.candidate.admitted === true
        ? "admitted"
        : cell.candidate.admitted === false
          ? "not admitted"
          : "admission unavailable",
      cell.aliasedCandidateCount > 1
        ? `${cell.aliasedCandidateCount} candidate identities share this cell`
        : "",
    ].filter(Boolean);
    return {
      cell,
      value,
      displayValue,
      tone: visual.tone,
      intensity: visual.intensity,
      ariaLabel: `Fast ${cell.fastLen}, Slow ${cell.slowLen}: ${metric.label} ${displayValue}; ${markers.join("; ")}.`,
    };
  });
  return {
    metric,
    cells,
    valueByCellKey: new Map(cells.map((cell) => [cell.cell.key, cell])),
    finiteMinimum,
    finiteMaximum,
    finiteCellCount: values.length,
  };
}

function parameterAxis(
  key: VcetCampaignEvidenceParameterAxisKey,
): VcetCampaignEvidenceParameterAxis {
  const axis = VCET_CAMPAIGN_EVIDENCE_PARAMETER_AXES.find(
    (candidate) => candidate.key === key,
  );
  if (!axis) throw new Error(`Unsupported VCET evidence parameter axis: ${key}`);
  return axis;
}

export function formatVcetCampaignEvidenceSearchSequence(
  sequence: string,
): string {
  switch (String(sequence || "").trim().toLowerCase()) {
    case "halton_cranley_patterson":
      return "Halton–Cranley–Patterson";
    case "exhaustive_lexicographic":
      return "Exhaustive lexicographic";
    case "halton":
      return "Legacy Halton";
    default:
      return "Sequence unavailable";
  }
}

function effectiveParameterValue(
  effective: VcetCampaignEffectiveControls | null,
  key: VcetCampaignEvidenceParameterAxisKey,
): number | null {
  if (!effective) return null;
  switch (key) {
    case "vidya_short_period":
      return effective.vidyaShortPeriod;
    case "vidya_long_period":
      return effective.vidyaLongPeriod;
    case "vidya_alpha":
      return effective.vidyaAlpha;
    case "ce_period":
      return effective.cePeriod;
    case "ce_mult":
      return effective.ceMult;
    case "ce_use_close":
      return effective.ceUseClose ? 1 : 0;
    case "require_vidya_slope":
      return effective.requireVidyaSlope ? 1 : 0;
    case "max_trade_bars":
      return effective.maxTradeBars;
  }
}

export function formatVcetCampaignEvidenceParameter(
  effective: VcetCampaignEffectiveControls | null,
  key: VcetCampaignEvidenceParameterAxisKey,
): string {
  const value = effectiveParameterValue(effective, key);
  return formatVcetCampaignEvidenceAxisValue(value, key);
}

export function formatVcetCampaignEvidenceAxisValue(
  value: number | null,
  key: VcetCampaignEvidenceParameterAxisKey,
): string {
  const axis = parameterAxis(key);
  if (value == null || !Number.isFinite(value)) return "Unavailable";
  if (axis.format === "boolean") return value === 1 ? "Enabled" : "Disabled";
  if (axis.format === "integer") return Math.round(value).toString();
  return value.toFixed(6).replace(/0+$/u, "").replace(/\.$/u, "");
}

function normalizedScatterRatio(
  value: number,
  minimum: number,
  maximum: number,
): number {
  if (maximum <= minimum) return 0.5;
  return Math.max(0, Math.min(1, (value - minimum) / (maximum - minimum)));
}

export function buildVcetCampaignEvidenceScatterView(
  model: VcetCampaignEvidenceModel,
  xAxisKey: VcetCampaignEvidenceParameterAxisKey,
  yAxisKey: VcetCampaignEvidenceParameterAxisKey,
  metricKey: VcetCampaignEvidenceMetricKey,
): VcetCampaignEvidenceScatterView {
  const xAxis = parameterAxis(xAxisKey);
  const yAxis = parameterAxis(yAxisKey);
  const metricView = buildVcetCampaignEvidenceMetricView(model, metricKey);
  const raw = metricView.cells
    .map((metricCell) => {
      const effective = metricCell.cell.candidate.effective;
      return {
        metricCell,
        xValue: effectiveParameterValue(effective, xAxisKey),
        yValue: effectiveParameterValue(effective, yAxisKey),
      };
    })
    .filter(
      (
        row,
      ): row is {
        metricCell: VcetCampaignEvidenceMetricCell;
        xValue: number;
        yValue: number;
      } =>
        row.xValue != null &&
        Number.isFinite(row.xValue) &&
        row.yValue != null &&
        Number.isFinite(row.yValue),
    );
  const xValues = raw.map((row) => row.xValue);
  const yValues = raw.map((row) => row.yValue);
  const xMinimum = xValues.length > 0 ? Math.min(...xValues) : null;
  const xMaximum = xValues.length > 0 ? Math.max(...xValues) : null;
  const yMinimum = yValues.length > 0 ? Math.min(...yValues) : null;
  const yMaximum = yValues.length > 0 ? Math.max(...yValues) : null;
  const points = raw.map((row) => {
    const candidate = row.metricCell.cell.candidate;
    const xLabel = formatVcetCampaignEvidenceParameter(
      candidate.effective,
      xAxisKey,
    );
    const yLabel = formatVcetCampaignEvidenceParameter(
      candidate.effective,
      yAxisKey,
    );
    return {
      candidate,
      metricCell: row.metricCell,
      xValue: row.xValue,
      yValue: row.yValue,
      xRatio: normalizedScatterRatio(
        row.xValue,
        xMinimum ?? row.xValue,
        xMaximum ?? row.xValue,
      ),
      yRatio: normalizedScatterRatio(
        row.yValue,
        yMinimum ?? row.yValue,
        yMaximum ?? row.yValue,
      ),
      xLabel,
      yLabel,
      ariaLabel: `${candidate.searchOrdinal != null ? `Search candidate ${candidate.searchOrdinal + 1}` : candidate.candidateId}: ${xAxis.label} ${xLabel}; ${yAxis.label} ${yLabel}; ${metricView.metric.label} ${row.metricCell.displayValue}; ${candidate.selected ? "backend selected; " : ""}${candidate.admitted === true ? "admitted" : candidate.admitted === false ? "not admitted" : "admission unavailable"}.`,
    };
  });
  return {
    xAxis,
    yAxis,
    points,
    omittedCandidateCount: Math.max(0, model.cells.length - points.length),
    xMinimum,
    xMaximum,
    yMinimum,
    yMaximum,
  };
}

export function sampleVcetCampaignEvidenceScatterPoints(
  points: readonly VcetCampaignEvidenceScatterPoint[],
  maximumPoints = 800,
): VcetCampaignEvidenceScatterSample {
  const limit = Math.max(1, Math.floor(maximumPoints));
  if (points.length <= limit) {
    return {
      points: points.slice(),
      totalPointCount: points.length,
      omittedPointCount: 0,
    };
  }
  const sampled: VcetCampaignEvidenceScatterPoint[] = [];
  const sampledIds = new Set<string>();
  const append = (point: VcetCampaignEvidenceScatterPoint | undefined) => {
    if (!point || sampledIds.has(point.candidate.candidateId)) return;
    sampledIds.add(point.candidate.candidateId);
    sampled.push(point);
  };
  append(points.find((point) => point.candidate.selected));
  const remainingSlots = limit - sampled.length;
  for (let index = 0; index < remainingSlots; index += 1) {
    const ratio = remainingSlots <= 1 ? 0.5 : index / (remainingSlots - 1);
    append(points[Math.round(ratio * (points.length - 1))]);
  }
  for (const point of points) {
    if (sampled.length >= limit) break;
    append(point);
  }
  return {
    points: sampled,
    totalPointCount: points.length,
    omittedPointCount: points.length - sampled.length,
  };
}

export function resolveVcetCampaignEvidencePlateau(
  model: VcetCampaignEvidenceModel,
  metricKey: VcetCampaignEvidenceMetricKey,
  requestedFocusCandidateId = "",
): VcetCampaignEvidencePlateau {
  if (model.designMode === "halton") {
    const focusCandidate =
      model.candidates.find(
        (candidate) => candidate.candidateId === requestedFocusCandidateId,
      ) ??
      model.candidates.find((candidate) => candidate.selected) ??
      model.candidates[0];
    const focusLabel = focusCandidate
      ? `candidate ${focusCandidate.searchOrdinal ?? focusCandidate.candidateId}`
      : "No metric focus";
    return {
      state: "insufficient",
      focusCandidateId: focusCandidate?.candidateId ?? "",
      focusLabel,
      stableNeighbors: 0,
      evaluatedNeighbors: 0,
      totalNeighborCells: 0,
      summary:
        "Fast×Slow plateau scoring is intentionally unavailable for the multidimensional design because candidates sharing those periods can differ in six other effective controls. Use the parameter scatter and complete table; robustness still requires out-of-sample validation.",
    };
  }
  const view = buildVcetCampaignEvidenceMetricView(model, metricKey);
  const focusId =
    requestedFocusCandidateId || model.selectedCandidateId || "";
  let focus = view.cells.find(
    (cell) => cell.cell.candidate.candidateId === focusId,
  );
  if (!focus) {
    const finite = view.cells.filter((cell) => cell.value != null);
    finite.sort((left, right) => {
      const leftValue = left.value ?? Number.NEGATIVE_INFINITY;
      const rightValue = right.value ?? Number.NEGATIVE_INFINITY;
      return view.metric.direction === "higher"
        ? rightValue - leftValue
        : leftValue - rightValue;
    });
    focus = finite[0];
  }
  if (!focus || focus.value == null) {
    return {
      state: "insufficient",
      focusCandidateId: "",
      focusLabel: "No metric focus",
      stableNeighbors: 0,
      evaluatedNeighbors: 0,
      totalNeighborCells: 0,
      summary: `Local plateau unavailable because ${view.metric.label} has no finite candidate values.`,
    };
  }

  const fastIndex = model.fastValues.indexOf(focus.cell.fastLen);
  const slowIndex = model.slowValues.indexOf(focus.cell.slowLen);
  const neighbors = view.cells.filter((candidate) => {
    if (candidate.cell.key === focus?.cell.key) return false;
    const candidateFastIndex = model.fastValues.indexOf(candidate.cell.fastLen);
    const candidateSlowIndex = model.slowValues.indexOf(candidate.cell.slowLen);
    return (
      Math.abs(candidateFastIndex - fastIndex) <= 1 &&
      Math.abs(candidateSlowIndex - slowIndex) <= 1
    );
  });
  const evaluated = neighbors.filter((candidate) => candidate.value != null);
  const metricSpan = Math.abs(
    (view.finiteMaximum ?? focus.value) - (view.finiteMinimum ?? focus.value),
  );
  const tolerance =
    view.metric.format === "percent"
      ? 0.05
      : Math.max(metricSpan * 0.1, Math.abs(focus.value) * 0.05, 1.0e-9);
  const stable = evaluated.filter((candidate) => {
    const value = candidate.value ?? Number.NEGATIVE_INFINITY;
    return view.metric.direction === "higher"
      ? value >= focus.value! - tolerance
      : value <= focus.value! + tolerance;
  }).length;
  const ratio = evaluated.length > 0 ? stable / evaluated.length : 0;
  const state: VcetCampaignEvidencePlateau["state"] =
    evaluated.length < 2
      ? "insufficient"
      : ratio >= 0.75
        ? "broad"
        : ratio >= 0.5
          ? "developing"
          : "isolated";
  const focusLabel = `Fast ${focus.cell.fastLen} / Slow ${focus.cell.slowLen}`;
  const stateText =
    state === "broad"
      ? "Broad local plateau"
      : state === "developing"
        ? "Developing local plateau"
        : state === "isolated"
          ? "Isolated local peak"
          : "Local plateau evidence is sparse";
  return {
    state,
    focusCandidateId: focus.cell.candidate.candidateId,
    focusLabel,
    stableNeighbors: stable,
    evaluatedNeighbors: evaluated.length,
    totalNeighborCells: neighbors.length,
    summary: `${stateText}: ${stable}/${evaluated.length} evaluated neighboring cells stay within the ${view.metric.label} tolerance around ${focusLabel}. This is descriptive grid context, not out-of-sample proof.`,
  };
}
