// `halton` is the persisted compatibility discriminator for the bounded
// multidimensional mode. The backend selects Halton–Cranley–Patterson or
// exhaustive lexicographic generation from the validated space cardinality.
export type VcetCandidateDesignMode = "period_grid" | "halton";
export type VcetBooleanChoice = "both" | "true" | "false";

export interface VcetSearchDraft {
  budget: string;
  seed: string;
  vidyaShortMin: string;
  vidyaShortMax: string;
  vidyaShortStep: string;
  vidyaLongMin: string;
  vidyaLongMax: string;
  vidyaLongStep: string;
  vidyaAlphaMin: string;
  vidyaAlphaMax: string;
  vidyaAlphaQuantum: string;
  cePeriodMin: string;
  cePeriodMax: string;
  cePeriodStep: string;
  ceMultMin: string;
  ceMultMax: string;
  ceMultQuantum: string;
  ceUseClose: VcetBooleanChoice;
  requireVidyaSlope: VcetBooleanChoice;
  maxTradeBarsMin: string;
  maxTradeBarsMax: string;
  maxTradeBarsStep: string;
}

export interface VcetU16SearchRange {
  min: number;
  max: number;
  step: number;
}

export interface VcetFloatSearchRange {
  min: number;
  max: number;
  quantum: number;
}

export interface VcetSearchConfigRequest {
  budget: number;
  seed: number;
  vidya_short_period: VcetU16SearchRange;
  vidya_long_period: VcetU16SearchRange;
  vidya_alpha: VcetFloatSearchRange;
  ce_period: VcetU16SearchRange;
  ce_mult: VcetFloatSearchRange;
  ce_use_close: { values: boolean[] };
  require_vidya_slope: { values: boolean[] };
  max_trade_bars: VcetU16SearchRange;
}

export interface VcetSearchDraftSummary {
  config: VcetSearchConfigRequest;
  validPeriodPairs: bigint;
  validSearchSpaceSize: bigint;
  effectiveBudget: number;
  summary: string;
}

export const MAX_VCET_SEARCH_BUDGET = 8_192;
export const MAX_VCET_SEARCH_CE_MULT = 1_000;
export const MAX_SAFE_VCET_SEARCH_SEED = Number.MAX_SAFE_INTEGER;

const FLOAT_IDENTITY_SCALE = 1_000_000;

export function createDefaultVcetSearchDraft(): VcetSearchDraft {
  return {
    budget: "2048",
    seed: "0",
    vidyaShortMin: "4",
    vidyaShortMax: "48",
    vidyaShortStep: "2",
    vidyaLongMin: "12",
    vidyaLongMax: "160",
    vidyaLongStep: "4",
    vidyaAlphaMin: "0.05",
    vidyaAlphaMax: "0.8",
    vidyaAlphaQuantum: "0.05",
    cePeriodMin: "10",
    cePeriodMax: "80",
    cePeriodStep: "2",
    ceMultMin: "1.5",
    ceMultMax: "5",
    ceMultQuantum: "0.25",
    ceUseClose: "both",
    requireVidyaSlope: "both",
    maxTradeBarsMin: "30",
    maxTradeBarsMax: "480",
    maxTradeBarsStep: "30",
  };
}

export function normalizeVcetCandidateDesignMode(
  value: unknown,
): VcetCandidateDesignMode {
  return String(value ?? "").trim().toLowerCase() === "halton"
    ? "halton"
    : "period_grid";
}

function strictInteger(
  value: unknown,
  label: string,
  minimum: number,
  maximum: number,
): number {
  const text = String(value ?? "").trim();
  if (!/^(?:0|[1-9]\d*)$/u.test(text)) {
    throw new Error(`${label} must be a whole number.`);
  }
  const parsed = Number(text);
  if (!Number.isSafeInteger(parsed) || parsed < minimum || parsed > maximum) {
    throw new Error(`${label} must be within [${minimum}, ${maximum}].`);
  }
  return parsed;
}

function canonicalFloatUnits(
  value: unknown,
  label: string,
  minimum: number,
  maximum: number,
): number {
  const text = String(value ?? "").trim();
  if (!text) throw new Error(`${label} is required.`);
  const parsed = Number(text);
  if (!Number.isFinite(parsed) || parsed < minimum || parsed > maximum) {
    throw new Error(`${label} must be within [${minimum}, ${maximum}].`);
  }
  const scaled = parsed * FLOAT_IDENTITY_SCALE;
  const rounded = Math.round(scaled);
  const tolerance = Math.max(1e-7, Math.abs(scaled) * Number.EPSILON * 8);
  if (Math.abs(scaled - rounded) > tolerance) {
    throw new Error(`${label} must use at most 6 decimal places.`);
  }
  return Object.is(rounded, -0) ? 0 : rounded;
}

function u16Range(
  label: string,
  minimum: number,
  minValue: unknown,
  maxValue: unknown,
  stepValue: unknown,
): VcetU16SearchRange {
  const min = strictInteger(minValue, `${label} minimum`, minimum, 65_535);
  const max = strictInteger(maxValue, `${label} maximum`, minimum, 65_535);
  const step = strictInteger(stepValue, `${label} step`, 1, 65_535);
  if (max < min) throw new Error(`${label} maximum must be at least its minimum.`);
  if ((max - min) % step !== 0) {
    throw new Error(`${label} endpoints must align exactly to its step.`);
  }
  return { min, max, step };
}

function floatRange(
  label: string,
  minimum: number,
  maximum: number,
  minValue: unknown,
  maxValue: unknown,
  quantumValue: unknown,
): VcetFloatSearchRange {
  const minUnits = canonicalFloatUnits(
    minValue,
    `${label} minimum`,
    minimum,
    maximum,
  );
  const maxUnits = canonicalFloatUnits(
    maxValue,
    `${label} maximum`,
    minimum,
    maximum,
  );
  const quantumUnits = canonicalFloatUnits(
    quantumValue,
    `${label} quantum`,
    0.000001,
    maximum,
  );
  if (maxUnits < minUnits) {
    throw new Error(`${label} maximum must be at least its minimum.`);
  }
  if ((maxUnits - minUnits) % quantumUnits !== 0) {
    throw new Error(`${label} endpoints must align exactly to its quantum.`);
  }
  return {
    min: minUnits / FLOAT_IDENTITY_SCALE,
    max: maxUnits / FLOAT_IDENTITY_SCALE,
    quantum: quantumUnits / FLOAT_IDENTITY_SCALE,
  };
}

function booleanValues(
  value: unknown,
  label: string,
): { values: boolean[] } {
  const normalized = String(value ?? "").trim().toLowerCase();
  if (normalized === "both") return { values: [false, true] };
  if (normalized === "true") return { values: [true] };
  if (normalized === "false") return { values: [false] };
  throw new Error(`${label} must be Both, Enabled only, or Disabled only.`);
}

function axisCount(range: VcetU16SearchRange): bigint {
  return BigInt((range.max - range.min) / range.step + 1);
}

function floatAxisCount(range: VcetFloatSearchRange): bigint {
  const min = Math.round(range.min * FLOAT_IDENTITY_SCALE);
  const max = Math.round(range.max * FLOAT_IDENTITY_SCALE);
  const quantum = Math.round(range.quantum * FLOAT_IDENTITY_SCALE);
  return BigInt((max - min) / quantum + 1);
}

function validPeriodPairCount(
  shortRange: VcetU16SearchRange,
  longRange: VcetU16SearchRange,
): bigint {
  let valid = 0n;
  for (
    let short = shortRange.min;
    short <= shortRange.max;
    short += shortRange.step
  ) {
    const firstLong =
      longRange.min > short
        ? longRange.min
        : longRange.min +
          (Math.floor((short - longRange.min) / longRange.step) + 1) *
            longRange.step;
    if (firstLong <= longRange.max) {
      valid += BigInt(
        Math.floor((longRange.max - firstLong) / longRange.step) + 1,
      );
    }
  }
  return valid;
}

export function buildVcetSearchConfigFromDraft(
  draft: VcetSearchDraft,
): VcetSearchDraftSummary {
  const budget = strictInteger(
    draft.budget,
    "VCET search budget",
    1,
    MAX_VCET_SEARCH_BUDGET,
  );
  const seed = strictInteger(
    draft.seed,
    "VCET search seed",
    0,
    MAX_SAFE_VCET_SEARCH_SEED,
  );
  const vidyaShort = u16Range(
    "VIDYA short period",
    2,
    draft.vidyaShortMin,
    draft.vidyaShortMax,
    draft.vidyaShortStep,
  );
  const vidyaLong = u16Range(
    "VIDYA long period",
    2,
    draft.vidyaLongMin,
    draft.vidyaLongMax,
    draft.vidyaLongStep,
  );
  const vidyaAlpha = floatRange(
    "VIDYA alpha",
    0,
    1,
    draft.vidyaAlphaMin,
    draft.vidyaAlphaMax,
    draft.vidyaAlphaQuantum,
  );
  const cePeriod = u16Range(
    "Chandelier period",
    1,
    draft.cePeriodMin,
    draft.cePeriodMax,
    draft.cePeriodStep,
  );
  const ceMult = floatRange(
    "Chandelier multiplier",
    0.000001,
    MAX_VCET_SEARCH_CE_MULT,
    draft.ceMultMin,
    draft.ceMultMax,
    draft.ceMultQuantum,
  );
  const maxTradeBars = u16Range(
    "Maximum trade bars",
    1,
    draft.maxTradeBarsMin,
    draft.maxTradeBarsMax,
    draft.maxTradeBarsStep,
  );
  const ceUseClose = booleanValues(draft.ceUseClose, "CE close-source choice");
  const requireVidyaSlope = booleanValues(
    draft.requireVidyaSlope,
    "VIDYA slope-gate choice",
  );
  const validPeriodPairs = validPeriodPairCount(vidyaShort, vidyaLong);
  if (validPeriodPairs === 0n) {
    throw new Error(
      "VCET search has no valid period pairs: at least one long period must be greater than a short period.",
    );
  }
  const validSearchSpaceSize = [
    floatAxisCount(vidyaAlpha),
    axisCount(cePeriod),
    floatAxisCount(ceMult),
    BigInt(ceUseClose.values.length),
    BigInt(requireVidyaSlope.values.length),
    axisCount(maxTradeBars),
  ].reduce((total, count) => total * count, validPeriodPairs);
  const effectiveBudget = Number(
    validSearchSpaceSize < BigInt(budget)
      ? validSearchSpaceSize
      : BigInt(budget),
  );
  const config: VcetSearchConfigRequest = {
    budget,
    seed,
    vidya_short_period: vidyaShort,
    vidya_long_period: vidyaLong,
    vidya_alpha: vidyaAlpha,
    ce_period: cePeriod,
    ce_mult: ceMult,
    ce_use_close: ceUseClose,
    require_vidya_slope: requireVidyaSlope,
    max_trade_bars: maxTradeBars,
  };
  const sequenceLabel =
    validSearchSpaceSize <= BigInt(budget)
      ? "exhaustive lexicographic"
      : "Halton–Cranley–Patterson";
  return {
    config,
    validPeriodPairs,
    validSearchSpaceSize,
    effectiveBudget,
    summary: `Deterministic ${sequenceLabel} design: ${effectiveBudget.toLocaleString()} unique candidate${effectiveBudget === 1 ? "" : "s"} from ${validSearchSpaceSize.toLocaleString()} valid effective combinations · requested budget ${budget.toLocaleString()} · seed ${seed}.`,
  };
}

function asRecord(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be an object.`);
  }
  return value as Record<string, unknown>;
}

function assertExactKeys(
  value: Record<string, unknown>,
  allowed: readonly string[],
  label: string,
): void {
  const allowedSet = new Set(allowed);
  const unknown = Object.keys(value).filter((key) => !allowedSet.has(key));
  if (unknown.length > 0) {
    throw new Error(`${label} contains unsupported field "${unknown[0]}".`);
  }
}

function rawBooleanChoice(value: unknown, label: string): VcetBooleanChoice {
  const row = asRecord(value, label);
  assertExactKeys(row, ["values"], label);
  if (!Array.isArray(row.values) || row.values.length === 0) {
    throw new Error(`${label}.values must be a non-empty boolean array.`);
  }
  if (row.values.some((entry) => typeof entry !== "boolean")) {
    throw new Error(`${label}.values must contain only booleans.`);
  }
  const values = new Set(row.values as boolean[]);
  return values.size === 2 ? "both" : values.has(true) ? "true" : "false";
}

function rawRange(
  value: unknown,
  label: string,
  stepKey: "step" | "quantum",
): [unknown, unknown, unknown] {
  const row = asRecord(value, label);
  assertExactKeys(row, ["min", "max", stepKey], label);
  return [row.min, row.max, row[stepKey]];
}

export function analyzeVcetSearchConfigRequest(
  value: unknown,
): VcetSearchDraftSummary {
  const row = asRecord(value, "validation_config.vcet_search");
  assertExactKeys(
    row,
    [
      "budget",
      "seed",
      "vidya_short_period",
      "vidya_long_period",
      "vidya_alpha",
      "ce_period",
      "ce_mult",
      "ce_use_close",
      "require_vidya_slope",
      "max_trade_bars",
    ],
    "validation_config.vcet_search",
  );
  const [shortMin, shortMax, shortStep] = rawRange(
    row.vidya_short_period,
    "vidya_short_period",
    "step",
  );
  const [longMin, longMax, longStep] = rawRange(
    row.vidya_long_period,
    "vidya_long_period",
    "step",
  );
  const [alphaMin, alphaMax, alphaQuantum] = rawRange(
    row.vidya_alpha,
    "vidya_alpha",
    "quantum",
  );
  const [cePeriodMin, cePeriodMax, cePeriodStep] = rawRange(
    row.ce_period,
    "ce_period",
    "step",
  );
  const [ceMultMin, ceMultMax, ceMultQuantum] = rawRange(
    row.ce_mult,
    "ce_mult",
    "quantum",
  );
  const [maxTradeBarsMin, maxTradeBarsMax, maxTradeBarsStep] = rawRange(
    row.max_trade_bars,
    "max_trade_bars",
    "step",
  );
  return buildVcetSearchConfigFromDraft({
    budget: String(row.budget ?? ""),
    seed: String(row.seed ?? ""),
    vidyaShortMin: String(shortMin ?? ""),
    vidyaShortMax: String(shortMax ?? ""),
    vidyaShortStep: String(shortStep ?? ""),
    vidyaLongMin: String(longMin ?? ""),
    vidyaLongMax: String(longMax ?? ""),
    vidyaLongStep: String(longStep ?? ""),
    vidyaAlphaMin: String(alphaMin ?? ""),
    vidyaAlphaMax: String(alphaMax ?? ""),
    vidyaAlphaQuantum: String(alphaQuantum ?? ""),
    cePeriodMin: String(cePeriodMin ?? ""),
    cePeriodMax: String(cePeriodMax ?? ""),
    cePeriodStep: String(cePeriodStep ?? ""),
    ceMultMin: String(ceMultMin ?? ""),
    ceMultMax: String(ceMultMax ?? ""),
    ceMultQuantum: String(ceMultQuantum ?? ""),
    ceUseClose: rawBooleanChoice(row.ce_use_close, "ce_use_close"),
    requireVidyaSlope: rawBooleanChoice(
      row.require_vidya_slope,
      "require_vidya_slope",
    ),
    maxTradeBarsMin: String(maxTradeBarsMin ?? ""),
    maxTradeBarsMax: String(maxTradeBarsMax ?? ""),
    maxTradeBarsStep: String(maxTradeBarsStep ?? ""),
  });
}

export function normalizeVcetSearchConfigRequest(
  value: unknown,
): VcetSearchConfigRequest {
  return analyzeVcetSearchConfigRequest(value).config;
}
