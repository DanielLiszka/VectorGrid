import type {
  OptionsExpressionMetricsSummary,
  OptionsExpressionTupleSummary,
  OptionsExpressionValidationResultSummary,
  OptionsExpressionValidationTupleResultSummary,
  OptionsExpressionValidationVerdict,
  ValidateOptionsExpressionArtifactSummary,
} from "../services/optionsExpressionService";

export type OptionsFinalistDashboardTone = "pass" | "mixed" | "pending";

export interface OptionsFinalistGateRow {
  key: string;
  label: string;
  status: string;
  detail: string;
  tone: OptionsFinalistDashboardTone;
}

export interface OptionsFinalistDashboardRow {
  key: string;
  sourceTupleIndex: number;
  tupleLabel: string;
  verdict: string;
  tone: OptionsFinalistDashboardTone;
  support: string;
  sourceRobustScore: number | null;
  validatedRobustScore: number | null;
  searchEv: number | null;
  holdoutEv: number | null;
  sealedEv: number | null;
  stressEv: number | null;
  harsherStressEv: number | null;
  extremeStressEv: number | null;
  strongestStressLabel: string;
  strongestStressEv: number | null;
  validTrades: number | null;
  drift: string;
}

export interface OptionsFinalistDashboardModel {
  verdict: string;
  acceptedTupleCount: number;
  validatedTupleCount: number;
  supportDeficientTupleCount: number;
  stressFragileTupleCount: number;
  gates: OptionsFinalistGateRow[];
  rows: OptionsFinalistDashboardRow[];
}

function text(value: unknown): string {
  return String(value ?? "").trim();
}

function finiteNumber(value: unknown): number | null {
  if (value == null || value === "") return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function metricEv(
  metrics: OptionsExpressionMetricsSummary | null | undefined,
): number | null {
  return finiteNumber(metrics?.expectancyPerPremium);
}

function metricTrades(
  metrics: OptionsExpressionMetricsSummary | null | undefined,
): number | null {
  return finiteNumber(metrics?.validTrades);
}

function verdictLabel(
  verdict: OptionsExpressionValidationVerdict | string | undefined,
): string {
  switch (text(verdict)) {
    case "holdout_supported":
      return "Holdout supported";
    case "sealed_finalist":
      return "Sealed finalist";
    case "stress_fragile":
      return "Stress fragile";
    case "search_only":
      return "Search only";
    default:
      return text(verdict).replace(/_/g, " ") || "Review";
  }
}

function verdictTone(
  verdict: OptionsExpressionValidationVerdict | string | undefined,
  supportDeficient: boolean,
): OptionsFinalistDashboardTone {
  if (supportDeficient || text(verdict) === "stress_fragile") return "mixed";
  if (
    text(verdict) === "sealed_finalist" ||
    text(verdict) === "holdout_supported"
  )
    return "pass";
  return "pending";
}

function tupleLabel(tuple: OptionsExpressionTupleSummary): string {
  const family = text(tuple.structureFamily);
  return [
    family,
    `${tuple.delayMinutes}m delay`,
    `${tuple.dteDays}d DTE`,
    tuple.moneynessBucket,
    `${tuple.holdMinutes}m hold`,
  ]
    .filter(Boolean)
    .join(" | ");
}

function strongestTrades(tuple: OptionsExpressionTupleSummary): number | null {
  return (
    [
      tuple.sealedEvaluationMetrics,
      tuple.holdoutMetrics,
      tuple.stressMetrics,
      tuple.selectionMetrics,
    ]
      .map(metricTrades)
      .find((value) => value != null) ?? null
  );
}

function strongestStress(tuple: OptionsExpressionTupleSummary): {
  label: string;
  value: number | null;
} {
  const lanes: Array<
    [string, OptionsExpressionMetricsSummary | null | undefined]
  > = [
    ["Extreme", tuple.sealedExtremeStressMetrics ?? tuple.extremeStressMetrics],
    ["Harsher", tuple.sealedHarsherStressMetrics ?? tuple.harsherStressMetrics],
    ["Stress", tuple.sealedStressMetrics ?? tuple.stressMetrics],
  ];
  for (const [label, metrics] of lanes) {
    const value = metricEv(metrics);
    if (value != null) return { label, value };
  }
  return { label: "None", value: null };
}

function rowFromTuple(
  entry: OptionsExpressionValidationTupleResultSummary,
): OptionsFinalistDashboardRow {
  const tuple = entry.validatedTuple;
  const verdict =
    tuple.validationVerdict ?? entry.sourceValidationVerdict ?? "search_only";
  const stress = strongestStress(tuple);
  return {
    key: `tuple-${entry.sourceTupleIndex}`,
    sourceTupleIndex: entry.sourceTupleIndex,
    tupleLabel: tupleLabel(tuple),
    verdict: verdictLabel(verdict),
    tone: verdictTone(verdict, entry.supportDeficient),
    support: entry.supportDeficient ? "Support deficient" : "Supported",
    sourceRobustScore: finiteNumber(entry.sourceRobustScore),
    validatedRobustScore: finiteNumber(tuple.robustScore),
    searchEv: metricEv(tuple.selectionMetrics),
    holdoutEv: metricEv(tuple.holdoutMetrics),
    sealedEv: metricEv(tuple.sealedEvaluationMetrics),
    stressEv: metricEv(tuple.stressMetrics),
    harsherStressEv: metricEv(
      tuple.harsherStressMetrics ?? tuple.sealedHarsherStressMetrics,
    ),
    extremeStressEv: metricEv(
      tuple.extremeStressMetrics ?? tuple.sealedExtremeStressMetrics,
    ),
    strongestStressLabel: stress.label,
    strongestStressEv: stress.value,
    validTrades: strongestTrades(tuple),
    drift: [
      `robust ${formatSigned(entry.drift.robustScoreDelta)}`,
      entry.drift.holdoutExpectancyDelta == null
        ? ""
        : `holdout ${formatSigned(entry.drift.holdoutExpectancyDelta)}`,
      entry.drift.stressExpectancyDelta == null
        ? ""
        : `stress ${formatSigned(entry.drift.stressExpectancyDelta)}`,
    ]
      .filter(Boolean)
      .join(" | "),
  };
}

function formatSigned(value: number | null | undefined): string {
  const parsed = finiteNumber(value);
  if (parsed == null) return "-";
  return `${parsed >= 0 ? "+" : ""}${parsed.toFixed(3)}`;
}

function ratioLabel(left: number, right: number): string {
  return `${Math.max(0, Math.floor(left))}/${Math.max(0, Math.floor(right))}`;
}

function gateRows(
  result: OptionsExpressionValidationResultSummary,
): OptionsFinalistGateRow[] {
  const acceptance = result.acceptanceSummary;
  const coverage = result.coverageAdequacy;
  const quoteQuality = result.datasetProvenance?.quoteQuality ?? null;
  const provenancePresent = [
    result.sourceArtifactHash,
    result.parentArtifactHash,
    result.optionsDatasetHash,
    result.signalManifestId,
  ].filter((value) => text(value).length > 0).length;
  const supportDeficient = acceptance.supportDeficientTupleCount;
  const accepted = acceptance.acceptedTupleCount;
  const stressSupported = result.tupleResults.filter(
    (entry) => strongestStress(entry.validatedTuple).value != null,
  ).length;
  const rows: OptionsFinalistGateRow[] = [
    {
      key: "acceptance",
      label: "Acceptance",
      status: acceptance.verdict.replace(/_/g, " ") || "review",
      detail: `${ratioLabel(accepted, acceptance.validatedTupleCount)} accepted; ${supportDeficient} support deficient.`,
      tone:
        accepted > 0 && supportDeficient === 0
          ? "pass"
          : accepted > 0
            ? "mixed"
            : "pending",
    },
    {
      key: "coverage",
      label: "Data coverage",
      status: coverage.verdict.replace(/_/g, " ") || "unknown",
      detail: coverage.nonPromotable
        ? `Non-promotable: ${coverage.reasons.join(", ") || "coverage policy"}`
        : `Search ${formatPercent(coverage.searchCandidateCoverageRate)}; holdout ${formatPercent(coverage.holdoutCandidateCoverageRate)}; sealed ${formatPercent(coverage.sealedTestCandidateCoverageRate)}.`,
      tone: coverage.nonPromotable ? "mixed" : "pass",
    },
    {
      key: "provenance",
      label: "Provenance",
      status: provenancePresent >= 4 ? "complete" : "partial",
      detail: `${provenancePresent}/4 source, parent, dataset, and signal identifiers present.`,
      tone: provenancePresent >= 4 ? "pass" : "pending",
    },
    {
      key: "holdout",
      label: "Holdout robustness",
      status:
        acceptance.stressFragileTupleCount > 0
          ? "stress disagreement"
          : "stable enough",
      detail: `${acceptance.stressFragileTupleCount} stress-fragile finalist${acceptance.stressFragileTupleCount === 1 ? "" : "s"}; best EV ${formatNumber(acceptance.bestValidatedExpectancyPerPremium)}.`,
      tone: acceptance.stressFragileTupleCount > 0 ? "mixed" : "pass",
    },
    {
      key: "stress_lanes",
      label: "Stress lanes",
      status: stressSupported > 0 ? `${stressSupported} supported` : "missing",
      detail: `${stressSupported}/${result.tupleResults.length} validated tuple${result.tupleResults.length === 1 ? "" : "s"} expose at least one stress, harsher, or extreme stress EV.`,
      tone: stressSupported > 0 ? "pass" : "pending",
    },
    {
      key: "quote_realism",
      label: "Quote realism",
      status: quoteQuality ? "quote evidence" : "aggregate only",
      detail: quoteQuality
        ? `Bid/ask ${formatPercent(quoteQuality.bidAskCoverageRate)}; median spread ${formatNumber(quoteQuality.medianSpreadBps)} bps; warnings ${quoteQuality.warnings.length}.`
        : "No quote-quality packet on this validation artifact.",
      tone: quoteQuality?.warnings.length
        ? "mixed"
        : quoteQuality
          ? "pass"
          : "pending",
    },
  ];
  return rows;
}

function formatNumber(value: number | null | undefined, digits = 3): string {
  const parsed = finiteNumber(value);
  return parsed == null ? "-" : parsed.toFixed(digits);
}

function formatPercent(value: number | null | undefined): string {
  const parsed = finiteNumber(value);
  return parsed == null ? "-" : `${(parsed * 100).toFixed(1)}%`;
}

export function buildOptionsFinalistDashboard(
  validation: ValidateOptionsExpressionArtifactSummary | null | undefined,
): OptionsFinalistDashboardModel | null {
  const result = validation?.result ?? null;
  if (!result) return null;
  const acceptance = result.acceptanceSummary;
  const rows = result.tupleResults
    .map(rowFromTuple)
    .sort((left, right) => left.sourceTupleIndex - right.sourceTupleIndex);
  return {
    verdict: acceptance.verdict.replace(/_/g, " ") || "review",
    acceptedTupleCount: acceptance.acceptedTupleCount,
    validatedTupleCount: acceptance.validatedTupleCount,
    supportDeficientTupleCount: acceptance.supportDeficientTupleCount,
    stressFragileTupleCount: acceptance.stressFragileTupleCount,
    gates: gateRows(result),
    rows,
  };
}

export function formatOptionsFinalistNumber(
  value: number | null | undefined,
  digits = 3,
): string {
  return formatNumber(value, digits);
}
