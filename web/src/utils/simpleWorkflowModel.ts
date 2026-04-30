export const SIMPLE_OPTIMIZATION_STRATEGIES = [
  "DoubleMa",
  "CompositeCpu",
] as const;

export type SimpleOptimizationStrategy =
  (typeof SIMPLE_OPTIMIZATION_STRATEGIES)[number];

export type SimpleStatusTone = "blocked" | "ready" | "done" | "warning";

export interface SimpleMetricItem {
  key: string;
  label: string;
  value: string;
  tone?: "neutral" | "success" | "warning" | "danger" | "accent";
  trend?: string;
  title?: string;
}

export type SimpleValidationRecommendationKind =
  | "blocked"
  | "recommended"
  | "passed"
  | "review"
  | "rejected";

export interface SimpleValidationRecommendation {
  kind: SimpleValidationRecommendationKind;
  tone: SimpleStatusTone;
  title: string;
  body: string;
}

export interface SimpleRunSummaryInput {
  datasetLabel?: unknown;
  dataId?: unknown;
  dataTimeframe?: unknown;
  dataLen?: unknown;
  datasetRange?: unknown;
  strategyKind?: unknown;
  objective?: unknown;
  presetProfile?: unknown;
  backend?: unknown;
  optimizationReady?: boolean;
  selectedCandidateTitle?: unknown;
  selectedCandidateSummary?: unknown;
  topRows?: unknown;
  bestStats?: Array<{ label?: unknown; value?: unknown; tone?: unknown }>;
  validationReady?: boolean;
  validationRows?: unknown;
  validationWarnings?: unknown;
  validationVerdictTitle?: unknown;
  validationVerdictBody?: unknown;
  validationVerdictTone?: SimpleStatusTone;
  validationPreview?: unknown;
}

export interface SimpleRunSummary {
  dataset: {
    label: string;
    timeframe: string;
    rows: string;
    range: string;
    ready: boolean;
  };
  strategy: {
    label: string;
    objective: string;
    searchDepth: string;
    engineLabel: string;
    simpleOptimizationSupported: boolean;
  };
  optimization: {
    ready: boolean;
    candidateTitle: string;
    candidateSummary: string;
    rankedCountLabel: string;
    metricItems: SimpleMetricItem[];
  };
  validation: {
    ready: boolean;
    rowsLabel: string;
    warningsLabel: string;
    preview: string;
    recommendation: SimpleValidationRecommendation;
    metricItems: SimpleMetricItem[];
  };
}

const EMPTY_VALUES = new Set(["", "-", "—"]);

function text(value: unknown): string {
  return String(value ?? "").trim();
}

function hasText(value: unknown): boolean {
  return !EMPTY_VALUES.has(text(value));
}

function firstText(...values: unknown[]): string {
  for (const value of values) {
    const next = text(value);
    if (!EMPTY_VALUES.has(next)) return next;
  }
  return "";
}

function nonNegativeInteger(value: unknown): number | null {
  const next = Math.floor(Number(value));
  return Number.isFinite(next) && next >= 0 ? next : null;
}

function normalizeMetricTone(value: unknown): NonNullable<SimpleMetricItem["tone"]> {
  const next = text(value).toLowerCase();
  if (next === "success" || next === "ok" || next === "done") return "success";
  if (next === "warning" || next === "warn" || next === "review") return "warning";
  if (next === "danger" || next === "error" || next === "failed") return "danger";
  if (next === "accent") return "accent";
  return "neutral";
}

function plural(value: number, singular: string, pluralLabel = `${singular}s`): string {
  return `${value} ${value === 1 ? singular : pluralLabel}`;
}

function sanitizeCandidateTitle(value: unknown, fallback: string): string {
  const next = text(value);
  if (!hasText(next) || next === "No ranked candidate selected.") return fallback;
  return next;
}

function sanitizeCandidateSummary(value: unknown, fallback: string): string {
  const next = text(value);
  if (!hasText(next) || next === "No ranked candidates are available.") return fallback;
  return next;
}

function validationTextLooksConcerning(value: unknown): boolean {
  const next = text(value).toLowerCase();
  if (!next || EMPTY_VALUES.has(next)) return false;
  return /\b(fail|failed|fragile|warning|rejected|blocked|error)\b/.test(next);
}

export function simpleStrategyLabel(value: unknown): string {
  const next = text(value) || "DoubleMa";
  if (next === "DoubleMa") return "Double MA crossover";
  if (next === "CompositeCpu") return "Composite signal";
  if (next === "CompositeTrend") return "Composite trend";
  if (next === "BreakoutMomentum") return "Breakout momentum";
  if (next === "RegimeHybrid") return "Regime-aware trend";
  return next;
}

export function simpleEngineLabel(value: unknown): string {
  const next = text(value) || "Auto";
  if (next === "Auto") return "Auto";
  if (next === "CpuOnly") return "CPU only";
  if (next === "GpuKernel") return "GPU kernel";
  if (next === "GpuHybridKernel") return "GPU hybrid";
  if (next === "GpuOnly") return "GPU indicators";
  return next;
}

export function isSimpleOptimizationStrategySupported(value: unknown): boolean {
  return SIMPLE_OPTIMIZATION_STRATEGIES.includes(
    text(value) as SimpleOptimizationStrategy,
  );
}

export function normalizeSimpleMetricItems(
  stats: SimpleRunSummaryInput["bestStats"],
): SimpleMetricItem[] {
  if (!Array.isArray(stats)) return [];
  return stats
    .map<SimpleMetricItem | null>((item, index) => {
      const label = firstText(item?.label);
      const value = firstText(item?.value);
      if (!label && !value) return null;
      return {
        key: `${label || "metric"}_${index}`,
        label: label || "Metric",
        value: value || "-",
        tone: normalizeMetricTone(item?.tone),
      } satisfies SimpleMetricItem;
    })
    .filter((item): item is SimpleMetricItem => item != null);
}

export function resolveSimpleValidationRecommendation(
  input: Pick<
    SimpleRunSummaryInput,
    | "optimizationReady"
    | "validationReady"
    | "validationWarnings"
    | "validationVerdictTitle"
    | "validationVerdictBody"
    | "validationVerdictTone"
    | "validationPreview"
  >,
): SimpleValidationRecommendation {
  if (!input.optimizationReady) {
    return {
      kind: "blocked",
      tone: "blocked",
      title: "Run optimization before validation.",
      body: "Validation checks a selected optimization candidate, so it becomes available after the first run.",
    };
  }

  if (!input.validationReady) {
    return {
      kind: "recommended",
      tone: "ready",
      title: "Validation recommended.",
      body: "Optimization results are ready. Validation is optional, but it adds holdout and robustness evidence before you trust a candidate.",
    };
  }

  const title = firstText(input.validationVerdictTitle, "Validation results are available.");
  const body = firstText(
    input.validationVerdictBody,
    input.validationPreview,
    "Open the full validation report for detailed evidence.",
  );
  const warningText = firstText(input.validationWarnings);
  const concerning =
    input.validationVerdictTone === "warning" ||
    validationTextLooksConcerning(title) ||
    validationTextLooksConcerning(body) ||
    validationTextLooksConcerning(warningText);

  if (/\breject|rejected|failed|fail\b/i.test(`${title} ${body} ${warningText}`)) {
    return {
      kind: "rejected",
      tone: "warning",
      title,
      body,
    };
  }

  if (concerning) {
    return {
      kind: "review",
      tone: "warning",
      title,
      body,
    };
  }

  return {
    kind: "passed",
    tone: "done",
    title,
    body,
  };
}

export function buildSimpleRunSummary(
  input: SimpleRunSummaryInput,
): SimpleRunSummary {
  const datasetLabel = firstText(input.datasetLabel, input.dataId);
  const timeframe = firstText(input.dataTimeframe, "base timeframe");
  const rows = firstText(
    input.dataLen ? `${input.dataLen} rows` : "",
    "No rows loaded",
  );
  const topRows = nonNegativeInteger(input.topRows) ?? 0;
  const rankedCountLabel =
    topRows > 0 ? plural(topRows, "ranked result") : "No ranked results yet";
  const optimizationReady = Boolean(input.optimizationReady);
  const validationReady = Boolean(input.validationReady);
  const validationRows = nonNegativeInteger(input.validationRows) ?? 0;
  const validationWarnings = firstText(input.validationWarnings, "None reported");
  const metricItems = normalizeSimpleMetricItems(input.bestStats);

  if (optimizationReady && !metricItems.some((item) => item.key === "ranked_results")) {
    metricItems.push({
      key: "ranked_results",
      label: "Ranked",
      value: rankedCountLabel,
      tone: topRows > 0 ? "success" : "neutral",
    });
  }

  const recommendationInput: Pick<
    SimpleRunSummaryInput,
    | "optimizationReady"
    | "validationReady"
    | "validationWarnings"
    | "validationVerdictTitle"
    | "validationVerdictBody"
    | "validationVerdictTone"
    | "validationPreview"
  > = {
    optimizationReady,
    validationReady,
    validationWarnings,
    validationVerdictTitle: input.validationVerdictTitle,
    validationVerdictBody: input.validationVerdictBody,
    validationPreview: input.validationPreview,
  };
  if (input.validationVerdictTone !== undefined) {
    recommendationInput.validationVerdictTone = input.validationVerdictTone;
  }

  const recommendation = resolveSimpleValidationRecommendation(recommendationInput);

  return {
    dataset: {
      label: datasetLabel || "No dataset loaded",
      timeframe,
      rows,
      range: firstText(input.datasetRange, "No range yet"),
      ready: hasText(datasetLabel),
    },
    strategy: {
      label: simpleStrategyLabel(input.strategyKind),
      objective: firstText(input.objective, "Sharpe"),
      searchDepth: firstText(input.presetProfile, "Balanced"),
      engineLabel: simpleEngineLabel(input.backend),
      simpleOptimizationSupported: isSimpleOptimizationStrategySupported(
        input.strategyKind || "DoubleMa",
      ),
    },
    optimization: {
      ready: optimizationReady,
      candidateTitle: optimizationReady
        ? sanitizeCandidateTitle(input.selectedCandidateTitle, "Best available candidate")
        : "No ranked candidate yet",
      candidateSummary: sanitizeCandidateSummary(
        input.selectedCandidateSummary,
        optimizationReady
          ? "Review the optimization result before validating."
          : "Run optimization to populate the shortlist.",
      ),
      rankedCountLabel,
      metricItems,
    },
    validation: {
      ready: validationReady,
      rowsLabel: validationReady
        ? plural(validationRows, "evidence row")
        : "No validation rows yet",
      warningsLabel: validationReady ? validationWarnings : "Run validation after optimization",
      preview: firstText(input.validationPreview, "No validation output yet."),
      recommendation,
      metricItems: [
        {
          key: "validation_rows",
          label: "Evidence",
          value: validationReady ? plural(validationRows, "row") : "Not run",
          tone: validationReady
            ? recommendation.tone === "done"
              ? "success"
              : "warning"
            : "neutral",
        },
        {
          key: "validation_state",
          label: "Recommendation",
          value: recommendation.kind === "recommended" ? "Recommended" : recommendation.title,
          tone: recommendation.tone === "done"
            ? "success"
            : recommendation.tone === "warning"
              ? "warning"
              : "neutral",
        },
      ],
    },
  };
}

export function formatSimpleRunSummaryMarkdown(
  summary: SimpleRunSummary,
  generatedAt = new Date(),
): string {
  const generated = Number.isNaN(generatedAt.getTime())
    ? ""
    : generatedAt.toISOString();
  const metricLines = summary.optimization.metricItems.length
    ? summary.optimization.metricItems.map((item) => `- ${item.label}: ${item.value}`)
    : ["- Metrics: Not available"];

  return [
    "# VectorGrid Simple Run Summary",
    "",
    generated ? `Generated: ${generated}` : "",
    "",
    "## Dataset",
    `- Dataset: ${summary.dataset.label}`,
    `- Timeframe: ${summary.dataset.timeframe}`,
    `- Rows: ${summary.dataset.rows}`,
    `- Range: ${summary.dataset.range}`,
    "",
    "## Strategy",
    `- Strategy: ${summary.strategy.label}`,
    `- Objective: ${summary.strategy.objective}`,
    `- Search depth: ${summary.strategy.searchDepth}`,
    `- Engine: ${summary.strategy.engineLabel}`,
    "",
    "## Optimization",
    `- Status: ${summary.optimization.ready ? "Results ready" : "Not run"}`,
    `- Best candidate: ${summary.optimization.candidateTitle}`,
    `- Summary: ${summary.optimization.candidateSummary}`,
    ...metricLines,
    "",
    "## Validation",
    `- Recommendation: ${summary.validation.recommendation.title}`,
    `- Detail: ${summary.validation.recommendation.body}`,
    `- Evidence rows: ${summary.validation.rowsLabel}`,
    `- Warnings: ${summary.validation.warningsLabel}`,
    "",
    "This summary is research evidence only, not trading advice or live-trading approval.",
  ]
    .filter((line, index, lines) => line || lines[index - 1] !== "")
    .join("\n")
    .trim();
}
