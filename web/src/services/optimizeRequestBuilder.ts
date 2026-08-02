import {
  countF64RangeValues,
  countPairsFastLtSlowRanges,
  num,
  parseU16LikeRange,
} from "../utils/core/ranges";
import { validateCandleFieldRequirements } from "../utils/core/candleFields";
import {
  isCompositeProfileKind,
  resolveCompositeProfileRoute,
  type CompositeProfileRoutePreview,
} from "../strategies/compositeProfileGpuPolicy";

const DEFAULT_STARTING_EQUITY = 10_000;
const MAX_HEATMAP_BINS = 512;
const MAX_INCLUDE_ALL_ROWS = 2_000_000;
const MAX_GPU_KERNEL_TOP_K = 256;
const MAX_EXTRA_SHORTLIST_OBJECTIVES = 3;
const COMPOSITE_CPU_PRODUCT_MODE = "Grid" as const;

export interface OptimizeBuildContext {
  strategy_kind?: unknown;
  data_id?: unknown;
  current_data_meta?: unknown;
}

export interface DoubleMaOptimizationBuildInputs {
  context: OptimizeBuildContext;
  dynamicStrategyIr?: unknown;
  maSupportMap?: unknown;
  backend?: unknown;
  fastStart?: unknown;
  fastEnd?: unknown;
  fastStep?: unknown;
  slowStart?: unknown;
  slowEnd?: unknown;
  slowStep?: unknown;
  fastMaType?: unknown;
  slowMaType?: unknown;
  maSource?: unknown;
  objective?: unknown;
  scoreMode?: unknown;
  hurdleTau?: unknown;
  metricContract?: unknown;
  includeTradeMetrics?: unknown;
  extraShortlistObjectives?: unknown;
  shortlistConfidenceMode?: unknown;
  shortlistUnionMode?: unknown;
  shortlistSliceMode?: unknown;
  shortlistSliceBenchmark?: unknown;
  shortlistNeighborhoodRadius?: unknown;
  shortlistNeighborhoodStep?: unknown;
  mode?: unknown;
  topK?: unknown;
  includeAll?: unknown;
  heatmapBins?: unknown;
  exportAllCsvPath?: unknown;
  startingEquity?: unknown;
  costPreset?: unknown;
  feeBps?: unknown;
  spreadBps?: unknown;
  slippageBps?: unknown;
  epsRel?: unknown;
  stopLossPct?: unknown;
  takeProfitPct?: unknown;
  cooldownBars?: unknown;
  signalPersistenceBars?: unknown;
  exitMode?: unknown;
  kaufmanStopPeriod?: unknown;
  kaufmanStopMult?: unknown;
  kaufmanStopMaType?: unknown;
  trendLrPeriod?: unknown;
  trendLrMinDeg?: unknown;
  trendBbwPeriod?: unknown;
  trendBbwMult?: unknown;
  trendBbwMin?: unknown;
  trendRocPeriod?: unknown;
  trendUseRoc?: unknown;
  trendUseAdx?: unknown;
  trendAdxPeriod?: unknown;
  trendAdxMin?: unknown;
  regimeAdxPeriod?: unknown;
  regimeAdxEnter?: unknown;
  regimeAdxExit?: unknown;
  regimeRevEntryRel?: unknown;
  regimeRevExitRel?: unknown;
  breakoutBbPeriod?: unknown;
  breakoutBbMult?: unknown;
  breakoutConfirmBars?: unknown;
  breakoutMacdFast?: unknown;
  breakoutMacdSlow?: unknown;
  breakoutMacdSignal?: unknown;
  breakoutCmoPeriod?: unknown;
  breakoutCmoMinAbs?: unknown;
  breakoutAdoscShort?: unknown;
  breakoutAdoscLong?: unknown;
  breakoutUseAdx?: unknown;
  breakoutAdxPeriod?: unknown;
  breakoutAdxMin?: unknown;
  pitEfiPeriod?: unknown;
  pitVoscShortPeriod?: unknown;
  pitVoscLongPeriod?: unknown;
  pitCfoPeriod?: unknown;
  pitCfoScalar?: unknown;
  pitConfirmBars?: unknown;
  vfctObvSmoothPeriod?: unknown;
  vfctMfiPeriod?: unknown;
  vfctMfiLongMin?: unknown;
  vfctMfiShortMax?: unknown;
  vfctConfirmBars?: unknown;
  longOnly?: unknown;
  allowFlip?: unknown;
  tradeOnNext?: unknown;
  dmConstraintMinTrades?: unknown;
  dmConstraintMaxDd?: unknown;
  dmConstraintMinSharpe?: unknown;
  dmConstraintMinSortino?: unknown;
  dmConstraintMinCalmar?: unknown;
  dmConstraintMaxVolatility?: unknown;
  dmConstraintMinExposure?: unknown;
  dmConstraintMaxExposure?: unknown;
  dmConstraintMaxAbsNetExposure?: unknown;
  dmConstraintMinOmega?: unknown;
  dmConstraintMaxUlcer?: unknown;
  dmConstraintMaxDownsideDeviation?: unknown;
  fastMaParams?: unknown;
  slowMaParams?: unknown;
  fastMaParamSweeps?: unknown;
  slowMaParamSweeps?: unknown;
  vdpsEta?: unknown;
  vdpsRungs?: unknown;
  vdpsMinInitialPairs?: unknown;
  vdpsMaxSurvivorSeeds?: unknown;
  vdpsMaxPeriodCandidates?: unknown;
  vdpsRandomInjectionRatio?: unknown;
  vdpsEliteMutationRatio?: unknown;
  vdpsDifferentialRatio?: unknown;
  vdpsDeterministic?: unknown;
  vdpsSeed?: unknown;
  vdpsQualityCheckMaxPairs?: unknown;
  vdpsQualityCheckTopK?: unknown;
}

interface StrategyConfigBuildInputs {
  startingEquity?: unknown;
  costPreset?: unknown;
  feeBps?: unknown;
  spreadBps?: unknown;
  slippageBps?: unknown;
  epsRel?: unknown;
  stopLossPct?: unknown;
  takeProfitPct?: unknown;
  cooldownBars?: unknown;
  signalPersistenceBars?: unknown;
  exitMode?: unknown;
  kaufmanStopPeriod?: unknown;
  kaufmanStopMult?: unknown;
  kaufmanStopMaType?: unknown;
  longOnly?: unknown;
  allowFlip?: unknown;
  tradeOnNext?: unknown;
  trendLrPeriod?: unknown;
  trendLrMinDeg?: unknown;
  trendBbwPeriod?: unknown;
  trendBbwMult?: unknown;
  trendBbwMin?: unknown;
  trendRocPeriod?: unknown;
  trendUseRoc?: unknown;
  trendUseAdx?: unknown;
  trendAdxPeriod?: unknown;
  trendAdxMin?: unknown;
  regimeAdxPeriod?: unknown;
  regimeAdxEnter?: unknown;
  regimeAdxExit?: unknown;
  regimeRevEntryRel?: unknown;
  regimeRevExitRel?: unknown;
  breakoutBbPeriod?: unknown;
  breakoutBbMult?: unknown;
  breakoutConfirmBars?: unknown;
  breakoutMacdFast?: unknown;
  breakoutMacdSlow?: unknown;
  breakoutMacdSignal?: unknown;
  breakoutCmoPeriod?: unknown;
  breakoutCmoMinAbs?: unknown;
  breakoutAdoscShort?: unknown;
  breakoutAdoscLong?: unknown;
  breakoutUseAdx?: unknown;
  breakoutAdxPeriod?: unknown;
  breakoutAdxMin?: unknown;
  pitEfiPeriod?: unknown;
  pitVoscShortPeriod?: unknown;
  pitVoscLongPeriod?: unknown;
  pitCfoPeriod?: unknown;
  pitCfoScalar?: unknown;
  pitConfirmBars?: unknown;
  vfctObvSmoothPeriod?: unknown;
  vfctMfiPeriod?: unknown;
  vfctMfiLongMin?: unknown;
  vfctMfiShortMax?: unknown;
  vfctConfirmBars?: unknown;
}

export interface CompositeCpuOptimizationBuildInputs extends StrategyConfigBuildInputs {
  context: OptimizeBuildContext;
  maSupportMap?: unknown;
  backend?: unknown;
  metricContract?: unknown;
  includeTradeMetrics?: unknown;
  fastStart?: unknown;
  fastEnd?: unknown;
  fastStep?: unknown;
  slowStart?: unknown;
  slowEnd?: unknown;
  slowStep?: unknown;
  fastMaType?: unknown;
  slowMaType?: unknown;
  maSource?: unknown;
  objective?: unknown;
  mode?: unknown;
  topK?: unknown;
  includeAll?: unknown;
  heatmapBins?: unknown;
  extraShortlistObjectives?: unknown;
  shortlistConfidenceMode?: unknown;
  shortlistUnionMode?: unknown;
  compositeSignalMode?: unknown;
  halfTrendAmplitude?: unknown;
  halfTrendAtrPeriod?: unknown;
  halfTrendChannelDev?: unknown;
  maczFastLength?: unknown;
  maczSlowLength?: unknown;
  maczSignalLength?: unknown;
  maczLengthZ?: unknown;
  maczLengthStdev?: unknown;
  maczGamma?: unknown;
  maczA?: unknown;
  maczB?: unknown;
  maczUseVolume?: unknown;
  maczUseLag?: unknown;
  ottPeriod?: unknown;
  ottPercent?: unknown;
  ottMaType?: unknown;
  asrSqueezeBbMult?: unknown;
  asrSqueezeKcMult?: unknown;
  asrAngleDegMin?: unknown;
  asrAngleDegMr?: unknown;
  asrUseErFilter?: unknown;
  asrErMin?: unknown;
  asrErMr?: unknown;
  asrZMaType?: unknown;
  asrZNbdev?: unknown;
  asrZDevtype?: unknown;
  asrZEntry?: unknown;
  asrZExit?: unknown;
  asrConfirmBars?: unknown;
  asrMomFadeBars?: unknown;
  asrMaxTradeBars?: unknown;
  asrMaxMrTradeBars?: unknown;
  vccMapFastToSqueezePeriod?: unknown;
  vccMapFastToMomentumPeriod?: unknown;
  vccMapSlowToTrendPeriod?: unknown;
  vccSqueezePeriod?: unknown;
  vccMomentumPeriod?: unknown;
  vccMomentumBbPeriod?: unknown;
  vccTrendPeriod?: unknown;
  vccSqueezeBbMult?: unknown;
  vccSqueezeKcMult?: unknown;
  vccConfirmBars?: unknown;
  vccRequireMomentumAccel?: unknown;
  vccMaxTradeBars?: unknown;
  vemrMapFastToZPeriod?: unknown;
  vemrMapFastToBbPeriod?: unknown;
  vemrMapSlowToAtrPeriod?: unknown;
  vemrZPeriod?: unknown;
  vemrZMaType?: unknown;
  vemrZNbdev?: unknown;
  vemrZDevtype?: unknown;
  vemrZEntry?: unknown;
  vemrZExit?: unknown;
  vemrBbPeriod?: unknown;
  vemrBbMult?: unknown;
  vemrRequireBandTouch?: unknown;
  vemrAtrPeriod?: unknown;
  vemrAtrStopMult?: unknown;
  vemrConfirmBars?: unknown;
  vemrMaxTradeBars?: unknown;
  rocbMapFastToChannelPeriod?: unknown;
  rocbMapSlowToAtrPeriod?: unknown;
  rocbChannelPeriod?: unknown;
  rocbAtrPeriod?: unknown;
  rocbAtrStopMult?: unknown;
  rocbMaxTradeBars?: unknown;
  vcetMapFastToVidyaShortPeriod?: unknown;
  vcetMapSlowToVidyaLongPeriod?: unknown;
  vcetMapSlowToCePeriod?: unknown;
  vcetVidyaShortPeriod?: unknown;
  vcetVidyaLongPeriod?: unknown;
  vcetVidyaAlpha?: unknown;
  vcetCePeriod?: unknown;
  vcetCeMult?: unknown;
  vcetCeUseClose?: unknown;
  vcetRequireVidyaSlope?: unknown;
  vcetMaxTradeBars?: unknown;
  fastMaParams?: unknown;
  slowMaParams?: unknown;
}

export interface CompositeMoOptimizationBuildInputs extends StrategyConfigBuildInputs {
  context: OptimizeBuildContext;
  maSupportMap?: unknown;
  fastStart?: unknown;
  fastEnd?: unknown;
  fastStep?: unknown;
  slowStart?: unknown;
  slowEnd?: unknown;
  slowStep?: unknown;
  fastMaType?: unknown;
  slowMaType?: unknown;
  maSource?: unknown;
  objective?: unknown;
  moObjectiveMin?: unknown;
  compositeSignalMode?: unknown;
  fastMaParams?: unknown;
  slowMaParams?: unknown;
  moSampler?: unknown;
  moRandomSeed?: unknown;
  moRandomTrials?: unknown;
  moTpeSeed?: unknown;
  moTpeTrials?: unknown;
  moTpeStartupTrials?: unknown;
  moTpeCandidates?: unknown;
  moTpeGamma?: unknown;
  moTpeBatchSize?: unknown;
  moTpeMaxModelPoints?: unknown;
  moQmcSeed?: unknown;
  moQmcTrials?: unknown;
  moQmcSequence?: unknown;
  moNsga2Seed?: unknown;
  moNsga2Population?: unknown;
  moNsga2Generations?: unknown;
  moNsga2CrossoverProb?: unknown;
  moNsga2SwappingProb?: unknown;
  moNsga2MutationProb?: unknown;
  moConstraintMinTrades?: unknown;
  moConstraintMaxDd?: unknown;
  moConstraintMaxAbsExposure?: unknown;
  moPrescanEnable?: unknown;
  moPrescanTopK?: unknown;
  moPrescanWindow?: unknown;
  moPrescanPredictiveTopK?: unknown;
  moPrescanPredictiveStride?: unknown;
  moPrescanRequireGpu?: unknown;
  moPrescanDeviceId?: unknown;
  moPrescanObjectives?: unknown;
  moPrescanMode?: unknown;
  moPrescanMaPairs?: unknown;
  moUseRangeBuilder?: unknown;
  moSearchSpace?: unknown;
  moHtAmpLow?: unknown;
  moHtAmpHigh?: unknown;
  moHtAmpStep?: unknown;
  moHtDevLow?: unknown;
  moHtDevHigh?: unknown;
  moHtDevStep?: unknown;
  moHtAtrLow?: unknown;
  moHtAtrHigh?: unknown;
  moHtAtrStep?: unknown;
  moMaczFastLow?: unknown;
  moMaczFastHigh?: unknown;
  moMaczFastStep?: unknown;
  moMaczSlowLow?: unknown;
  moMaczSlowHigh?: unknown;
  moMaczSlowStep?: unknown;
  moMaczSignalLow?: unknown;
  moMaczSignalHigh?: unknown;
  moMaczSignalStep?: unknown;
  moMaczLengthZLow?: unknown;
  moMaczLengthZHigh?: unknown;
  moMaczLengthZStep?: unknown;
  moMaczLengthStdevLow?: unknown;
  moMaczLengthStdevHigh?: unknown;
  moMaczLengthStdevStep?: unknown;
  moMaczGammaLow?: unknown;
  moMaczGammaHigh?: unknown;
  moMaczGammaStep?: unknown;
  moMaczALow?: unknown;
  moMaczAHigh?: unknown;
  moMaczAStep?: unknown;
  moMaczBLow?: unknown;
  moMaczBHigh?: unknown;
  moMaczBStep?: unknown;
  moMaczUseLagMode?: unknown;
  moMaczUseVolumeMode?: unknown;
  moOttPeriodLow?: unknown;
  moOttPeriodHigh?: unknown;
  moOttPeriodStep?: unknown;
  moOttPercentLow?: unknown;
  moOttPercentHigh?: unknown;
  moOttPercentStep?: unknown;
  moOttMaTypes?: unknown;
}

export interface MaParamsAndSweeps {
  params: Record<string, number> | null;
  sweeps: Record<string, [number, number, number]> | null;
}

function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
}

function requireMinimumCandleCount(
  meta: unknown,
  minimum: number,
  workflowLabel: string,
): Record<string, unknown> {
  if (!meta || typeof meta !== "object") throw new Error("Load CSV first.");
  const record = meta as Record<string, unknown>;
  const len = Number(record.len);
  if (Number.isFinite(len) && len >= 0 && len < minimum) {
    const barCount = Math.floor(len);
    const timeframe = normalizeText(record.timeframe);
    const candleLabel = barCount === 1 ? "candle" : "candles";
    const timeframeNote = timeframe ? ` at ${timeframe}` : "";
    throw new Error(
      `Loaded dataset has ${barCount} ${candleLabel}${timeframeNote}. ${workflowLabel} needs at least ${minimum} candles. Load more data or switch back to the base timeframe.`,
    );
  }
  return record;
}

function normalizeDoubleMaScoreMode(value: unknown): string | null {
  const raw = normalizeText(value);
  if (!raw) return null;
  const normalized = raw.toLowerCase();
  if (
    normalized === "objective" ||
    normalized === "none" ||
    normalized === "off"
  ) {
    return null;
  }
  if (
    normalized === "balanced_risk_adjusted_10" ||
    normalized === "balancedriskadjusted10" ||
    normalized === "balanced-risk-adjusted-10"
  ) {
    return "balanced_risk_adjusted_10";
  }
  if (
    normalized === "adversarial_envelope_10" ||
    normalized === "adversarialenvelope10" ||
    normalized === "adversarial-envelope-10" ||
    normalized === "adversarial_weight_envelope_10" ||
    normalized === "adversarialweightenvelope10" ||
    normalized === "adversarial-weight-envelope-10"
  ) {
    return "adversarial_envelope_10";
  }
  if (
    normalized === "support_shrunk_10" ||
    normalized === "supportshrunk10" ||
    normalized === "support-shrunk-10" ||
    normalized === "effective_support_10" ||
    normalized === "effectivesupport10" ||
    normalized === "effective-support-10"
  ) {
    return "support_shrunk_10";
  }
  if (
    normalized === "active_sortino_10" ||
    normalized === "activesortino10" ||
    normalized === "active-sortino-10" ||
    normalized === "activesortino"
  ) {
    return "active_sortino_10";
  }
  if (
    normalized === "effective_sample_lcb_10" ||
    normalized === "effectivesamplelcb10" ||
    normalized === "effective-sample-lcb-10" ||
    normalized === "obs_lcb_10" ||
    normalized === "obslcb10" ||
    normalized === "obs-lcb-10"
  ) {
    return "effective_sample_lcb_10";
  }
  if (
    normalized === "omega_vs_sharpe_10" ||
    normalized === "omegavssharpe10" ||
    normalized === "omega-vs-sharpe-10" ||
    normalized === "omega_vs_sharpe" ||
    normalized === "omegavssharpe"
  ) {
    return "omega_vs_sharpe_10";
  }
  if (
    normalized === "robust_margin_10" ||
    normalized === "robustmargin10" ||
    normalized === "robust-margin-10" ||
    normalized === "hurdle_slack_10" ||
    normalized === "hurdleslack10" ||
    normalized === "hurdle-slack-10"
  ) {
    return "robust_margin_10";
  }
  throw new Error(
    "Unsupported score mode. Supported values: balanced_risk_adjusted_10, adversarial_envelope_10, support_shrunk_10, active_sortino_10, effective_sample_lcb_10, omega_vs_sharpe_10, robust_margin_10.",
  );
}

function normalizeDoubleMaMetricContract(
  value: unknown,
): "auto" | "full_17" | "trade_22" {
  const raw = normalizeText(value);
  if (!raw) return "auto";
  const normalized = raw.toLowerCase();
  if (normalized === "auto" || normalized === "default") {
    return "auto";
  }
  if (
    normalized === "full_17" ||
    normalized === "full17" ||
    normalized === "17" ||
    normalized === "17_row" ||
    normalized === "17-row"
  ) {
    return "full_17";
  }
  if (
    normalized === "trade_22" ||
    normalized === "trade22" ||
    normalized === "22" ||
    normalized === "22_row" ||
    normalized === "22-row"
  ) {
    return "trade_22";
  }
  throw new Error(
    "Unsupported metric contract. Supported values: auto, full_17, trade_22.",
  );
}

function normalizeObjectiveKindToken(value: unknown): string | null {
  const raw = normalizeText(value);
  if (!raw) return null;
  const normalized = raw.toLowerCase().replace(/[\s-]+/g, "_");
  if (normalized === "pnl") return "Pnl";
  if (normalized === "sharpe") return "Sharpe";
  if (normalized === "sortino") return "Sortino";
  if (normalized === "calmar") return "Calmar";
  if (normalized === "omega") return "Omega";
  if (normalized === "volatility" || normalized === "vol") return "Volatility";
  if (
    normalized === "maxdrawdown" ||
    normalized === "max_drawdown" ||
    normalized === "drawdown" ||
    normalized === "dd"
  ) {
    return "MaxDrawdown";
  }
  return null;
}

function normalizeExtraShortlistObjectives(
  value: unknown,
  primaryObjective: string,
): string[] | null {
  const primary = normalizeObjectiveKindToken(primaryObjective) ?? "Sharpe";
  const rawTokens = Array.isArray(value)
    ? value
    : normalizeText(value)
        .split(",")
        .map((token) => token.trim())
        .filter(Boolean);
  const out: string[] = [];
  for (const rawToken of rawTokens) {
    const objective = normalizeObjectiveKindToken(rawToken);
    if (!objective) {
      throw new Error(
        "Unsupported extra shortlist objective. Supported values: Pnl, Sharpe, Sortino, Calmar, Omega, Volatility, MaxDrawdown.",
      );
    }
    if (objective === primary || out.includes(objective)) continue;
    out.push(objective);
    if (out.length > MAX_EXTRA_SHORTLIST_OBJECTIVES) {
      throw new Error(
        `Extra shortlist objectives support at most ${MAX_EXTRA_SHORTLIST_OBJECTIVES} additional objectives.`,
      );
    }
  }
  return out.length > 0 ? out : null;
}

function normalizeShortlistConfidenceMode(value: unknown): string | null {
  const raw = normalizeText(value);
  if (!raw) return null;
  const normalized = raw.toLowerCase().replace(/[\s-]+/g, "_");
  if (normalized === "psr") return "psr";
  if (normalized === "dsr") return "dsr";
  throw new Error(
    "Unsupported shortlist confidence mode. Supported values: psr, dsr.",
  );
}

function normalizeShortlistUnionMode(value: unknown): string | null {
  const raw = normalizeText(value);
  if (!raw) return null;
  const normalized = raw.toLowerCase().replace(/[\s-]+/g, "_");
  if (normalized === "lexicographic") return "lexicographic";
  if (normalized === "pareto") return "pareto";
  if (normalized === "epsilon_front") return "epsilon_front";
  if (normalized === "weighted_rank") return "weighted_rank";
  throw new Error(
    "Unsupported shortlist union mode. Supported values: lexicographic, pareto, epsilon_front, weighted_rank.",
  );
}

function normalizeShortlistSliceMode(value: unknown): string | null {
  const raw = normalizeText(value);
  if (!raw) return null;
  const normalized = raw.toLowerCase().replace(/[\s-]+/g, "_");
  if (normalized === "halves") return "halves";
  if (normalized === "quartiles") return "quartiles";
  throw new Error(
    "Unsupported shortlist slice mode. Supported values: halves, quartiles.",
  );
}

function normalizeShortlistSliceBenchmark(value: unknown): string | null {
  const raw = normalizeText(value);
  if (!raw) return null;
  const normalized = raw.toLowerCase().replace(/[\s-]+/g, "_");
  if (normalized === "flat") return "flat";
  if (normalized === "buy_and_hold" || normalized === "buyandhold") {
    return "buy_and_hold";
  }
  throw new Error(
    "Unsupported shortlist slice benchmark. Supported values: flat, buy_and_hold.",
  );
}

function normalizeStrategyKindRaw(kindRaw: unknown): string {
  const kind = normalizeText(kindRaw || "DoubleMa") || "DoubleMa";
  if (kind === "DynamicDoubleMa") return "DynamicDoubleMa";
  if (kind === "Pit") return "Pit";
  if (kind === "Set") return "Set";
  if (kind === "Mts") return "Mts";
  if (kind === "Orc") return "Orc";
  if (kind === "Marrt") return "Marrt";
  if (kind === "Vfct") return "Vfct";
  if (kind === "Rocb") return "Rocb";
  if (kind === "Vcet") return "Vcet";
  if (kind === "Kqsr") return "Kqsr";
  if (kind === "Carm") return "Carm";
  if (kind === "Kvdf") return "Kvdf";
  if (kind === "Mkcs") return "Mkcs";
  if (kind === "Cdmc") return "Cdmc";
  if (kind === "Asr") return "Asr";
  if (kind === "Vcc") return "Vcc";
  if (kind === "Vemr") return "Vemr";
  if (kind === "CompositeCpu") return "CompositeCpu";
  if (kind === "CompositeMo") return "CompositeMo";
  if (kind === "CompositeTrend") return "CompositeTrend";
  if (kind === "RegimeHybrid") return "RegimeHybrid";
  if (kind === "BreakoutMomentum") return "BreakoutMomentum";
  return "DoubleMa";
}

const MARRT_MA_WHITELIST = new Set([
  "sma",
  "ema",
  "alma",
  "wma",
  "zlema",
  "vwma",
]);
const VFCT_MA_WHITELIST = new Set(["vwma"]);

function normalizeBoolean(value: unknown): boolean {
  if (typeof value === "boolean") return value;
  const text = normalizeText(value).toLowerCase();
  return text === "true" || text === "1" || text === "yes" || text === "on";
}

function normalizeIncludeAll(value: unknown): boolean {
  const text = normalizeText(value).toLowerCase();
  return text === "true";
}

function parseRiskPct(value: unknown): number | null {
  const raw = num(value, Number.NaN);
  if (!Number.isFinite(raw) || raw <= 0) return null;
  const frac = raw > 1 ? raw / 100 : raw;
  if (!Number.isFinite(frac) || frac <= 0 || frac >= 1) return null;
  return frac;
}

function normalizeStartingEquity(value: unknown): number {
  const x = Number(value);
  if (!Number.isFinite(x) || x <= 0) return DEFAULT_STARTING_EQUITY;
  return x;
}

function parseOptionalFinite(value: unknown): number | null {
  const text = normalizeText(value);
  if (!text) return null;
  const parsed = Number(text);
  if (!Number.isFinite(parsed)) return null;
  return parsed;
}

function parseOptionalNonNegativeInteger(value: unknown): number | null {
  const parsed = parseOptionalFinite(value);
  if (parsed == null) return null;
  const integer = Math.floor(parsed);
  if (!Number.isFinite(integer)) return null;
  if (integer < 0) throw new Error("Min trades must be >= 0.");
  return integer;
}

function parseOptionalPositiveInteger(
  value: unknown,
  label: string,
): number | null {
  const parsed = parseOptionalFinite(value);
  if (parsed == null) return null;
  const integer = Math.floor(parsed);
  if (!Number.isFinite(integer) || integer <= 0) {
    throw new Error(`${label} must be a positive integer.`);
  }
  return integer;
}

function parseOptionalNonNegativeNumber(
  value: unknown,
  label: string,
): number | null {
  const parsed = parseOptionalFinite(value);
  if (parsed == null) return null;
  if (parsed < 0) throw new Error(`${label} must be >= 0.`);
  return parsed;
}

function parseOptionalNonNegativeU32(
  value: unknown,
  label: string,
): number | null {
  const parsed = parseOptionalFinite(value);
  if (parsed == null) return null;
  const integer = Math.floor(parsed);
  if (!Number.isFinite(integer) || integer < 0 || integer > 0xffff_ffff) {
    throw new Error(`${label} must be in [0, 4294967295].`);
  }
  return integer;
}

function normalizeMaParams(value: unknown): Record<string, number> | null {
  if (!value || typeof value !== "object") return null;
  const out: Record<string, number> = {};
  for (const [key, raw] of Object.entries(value as Record<string, unknown>)) {
    const name = normalizeText(key);
    if (!name) continue;
    const parsed = Number(raw);
    if (!Number.isFinite(parsed)) continue;
    out[name] = parsed;
  }
  return Object.keys(out).length ? out : null;
}

function normalizeMaParamSweeps(
  value: unknown,
): Record<string, [number, number, number]> | null {
  if (!value || typeof value !== "object") return null;
  const out: Record<string, [number, number, number]> = {};
  for (const [key, raw] of Object.entries(value as Record<string, unknown>)) {
    const name = normalizeText(key);
    if (!name) continue;
    let start = Number.NaN;
    let end = Number.NaN;
    let step = Number.NaN;
    if (Array.isArray(raw)) {
      start = Number(raw[0]);
      end = Number(raw[1]);
      step = Number(raw[2]);
    } else if (raw && typeof raw === "object") {
      const row = raw as Record<string, unknown>;
      start = Number(row.start);
      end = Number(row.end);
      step = Number(row.step);
    }
    if (
      !Number.isFinite(start) ||
      !Number.isFinite(end) ||
      !Number.isFinite(step) ||
      step <= 0
    )
      continue;
    out[name] = [start, end, step];
  }
  return Object.keys(out).length ? out : null;
}

function maSweepVariantCount(
  value: Record<string, [number, number, number]> | null,
): number {
  if (!value) return 1;
  let count = 1;
  for (const [, range] of Object.entries(value)) {
    count *= Math.max(1, countF64RangeValues(range[0], range[1], range[2]));
    if (!Number.isFinite(count) || count <= 0) return 1;
  }
  return Math.max(1, Math.floor(count));
}

type NormalizedBackend = "Auto" | "CpuOnly" | "GpuKernel";

function normalizeBackend(value: unknown): NormalizedBackend {
  const backend = normalizeText(value);
  if (backend === "CpuOnly") return "CpuOnly";
  if (backend === "GpuOnly") return "GpuKernel";
  if (backend === "GpuHybridKernel") return "GpuKernel";
  if (backend === "GpuKernel") return "GpuKernel";
  if (backend === "Auto") return "Auto";
  return "CpuOnly";
}

function isDoubleMaOverlayKind(kind: string): boolean {
  return (
    kind === "DoubleMa" ||
    kind === "DynamicDoubleMa" ||
    kind === "Pit" ||
    kind === "Set" ||
    kind === "Mts" ||
    kind === "Orc" ||
    kind === "Marrt" ||
    kind === "Vfct" ||
    kind === "CompositeTrend" ||
    kind === "RegimeHybrid" ||
    kind === "BreakoutMomentum"
  );
}

function resolveCompositeRoutePreview(
  kind: string,
  backend: NormalizedBackend,
  inputs: Pick<
    DoubleMaOptimizationBuildInputs,
    "trendUseAdx" | "breakoutUseAdx"
  >,
): CompositeProfileRoutePreview | null {
  if (!isCompositeProfileKind(kind)) return null;
  return resolveCompositeProfileRoute(kind, backend, {
    trendUseAdx: normalizeBoolean(inputs.trendUseAdx),
    breakoutUseAdx: normalizeBoolean(inputs.breakoutUseAdx),
  });
}

function resolveDoubleMaBackendPayload(
  kind: string,
  backend: NormalizedBackend,
  strategy: Record<string, unknown>,
  compositeRoute: CompositeProfileRoutePreview | null = null,
): Record<string, unknown> {
  if (compositeRoute) {
    if (compositeRoute.targetBackend === "CpuOnly")
      return { backend: "CpuOnly" };
    return {
      backend: compositeRoute.targetBackend,
      device_id: 0,
    };
  }
  if (backend === "CpuOnly") return { backend: "CpuOnly" };

  const useHybrid =
    kind === "RegimeHybrid" ||
    (kind === "CompositeTrend" && !!strategy.trend_quality_use_adx) ||
    (kind === "BreakoutMomentum" && !!strategy.breakout_use_adx);

  return {
    backend: useHybrid ? "GpuHybridKernel" : "GpuKernel",
    device_id: 0,
  };
}

function resolveDoubleMaGpuExecutionPolicy(
  backendPayload: Record<string, unknown>,
  mode: "Grid" | "CoarseToFine" | "GpuVdps" | "Auto",
): "BestEffort" | "PreferGpuResident" | "StrictGpuResident" {
  const backend = normalizeText(backendPayload.backend);
  if (
    mode === "GpuVdps" ||
    backend === "GpuKernel" ||
    backend === "GpuHybridKernel"
  ) {
    return "PreferGpuResident";
  }
  return "BestEffort";
}

function normalizeMode(
  value: unknown,
  outputs: {
    includeAll: boolean;
    heatmapBins: number;
    exportAllCsvPath: string;
  },
): "Grid" | "CoarseToFine" | "GpuVdps" | "Auto" {
  const mode = normalizeText(value);
  const requestedMode =
    mode === "CoarseToFine"
      ? "CoarseToFine"
      : mode === "GpuVdps"
        ? "GpuVdps"
        : mode === "Auto"
          ? "Auto"
          : "Grid";
  const exactGridOutputsRequested =
    outputs.includeAll ||
    outputs.heatmapBins > 0 ||
    outputs.exportAllCsvPath.length > 0;
  if (requestedMode === "Grid") return "Grid";
  return exactGridOutputsRequested ? "Grid" : requestedMode;
}

function normalizeCompositeSignalMode(
  value: unknown,
): "hard_gate" | "entry_core_exit_overlay" | "trend_pullback_reversion" {
  const mode = normalizeText(value).toLowerCase();
  if (mode === "entry_core_exit_overlay") return "entry_core_exit_overlay";
  if (mode === "trend_pullback_reversion") return "trend_pullback_reversion";
  return "hard_gate";
}

interface MovingAverageSupportEntry {
  supports_cpu?: unknown;
  supports_cuda?: unknown;
  supports_cuda_kernel?: unknown;
}

interface MovingAverageSupportLookup {
  hasAny(): boolean;
  get(id: string): MovingAverageSupportEntry | null;
}

function canonicalizeMovingAverageId(value: unknown): string {
  return normalizeText(value).toLowerCase();
}

function createMovingAverageSupportLookup(
  raw: unknown,
): MovingAverageSupportLookup {
  const map =
    raw && typeof raw === "object" ? (raw as Record<string, unknown>) : null;
  const cache = new Map<string, MovingAverageSupportEntry | null>();
  return {
    hasAny(): boolean {
      return !!map && Object.keys(map).length > 0;
    },
    get(id: string): MovingAverageSupportEntry | null {
      const canonicalId = canonicalizeMovingAverageId(id);
      if (!canonicalId || !map) return null;
      if (cache.has(canonicalId)) return cache.get(canonicalId) ?? null;

      let resolved: MovingAverageSupportEntry | null = null;
      for (const [key, value] of Object.entries(map)) {
        if (canonicalizeMovingAverageId(key) !== canonicalId) continue;
        if (value && typeof value === "object") {
          resolved = value as MovingAverageSupportEntry;
        }
        break;
      }
      cache.set(canonicalId, resolved);
      return resolved;
    },
  };
}

function maSupportsCpu(
  lookup: MovingAverageSupportLookup,
  id: string,
): boolean {
  const row = lookup.get(id);
  return !!row?.supports_cpu;
}

function maSupportsCuda(
  lookup: MovingAverageSupportLookup,
  id: string,
): boolean {
  const row = lookup.get(id);
  return !!row?.supports_cuda;
}

function maSupportsKernel(
  lookup: MovingAverageSupportLookup,
  id: string,
): boolean {
  const row = lookup.get(id);
  return !!row?.supports_cuda_kernel;
}

function multiplyClamped(values: number[], limit: number): number {
  let product = 1;
  for (const value of values) {
    const next = Math.max(1, Math.floor(value));
    if (!Number.isFinite(next)) return limit + 1;
    product *= next;
    if (!Number.isFinite(product) || product > limit) return limit + 1;
  }
  return Math.max(1, Math.floor(product));
}

interface CommonStrategyConfigFields {
  longOnly: boolean;
  allowFlip: boolean;
  costPreset: string;
  startingEquity: number;
  feeBps: number;
  spreadBps: number;
  slippageBps: number;
  commission: number;
  epsRel: number;
  stopLossPct: number | null;
  takeProfitPct: number | null;
  cooldownBars: number;
  signalPersistenceBars: number;
  exitMode: string;
  kaufmanStopPeriod: number;
  kaufmanStopMult: number;
  kaufmanStopMaType: string;
}

function parseCommonStrategyConfigFields(
  inputs: StrategyConfigBuildInputs,
): CommonStrategyConfigFields {
  const feeBps = Math.max(0, num(inputs.feeBps, 0));
  const spreadBps = Math.max(0, num(inputs.spreadBps, 0));
  const slippageBps = Math.max(0, num(inputs.slippageBps, 0));
  const startingEquity = normalizeStartingEquity(inputs.startingEquity);
  const commission = (feeBps + slippageBps + spreadBps / 2) / 10_000;
  const epsRel = num(inputs.epsRel, 0);
  const stopLossPct = parseRiskPct(inputs.stopLossPct);
  const takeProfitPct = parseRiskPct(inputs.takeProfitPct);
  const cooldownBars = Math.max(0, Math.floor(num(inputs.cooldownBars, 0)));
  const signalPersistenceBars = Math.max(
    1,
    Math.floor(num(inputs.signalPersistenceBars, 1)),
  );
  const exitMode = normalizeText(inputs.exitMode || "cross") || "cross";
  const kaufmanStopPeriod = Math.max(
    1,
    Math.floor(num(inputs.kaufmanStopPeriod, 22)),
  );
  const kaufmanStopMult = Math.max(0.0001, num(inputs.kaufmanStopMult, 2.0));
  const kaufmanStopMaType =
    normalizeText(inputs.kaufmanStopMaType || "sma") || "sma";

  if (!Number.isFinite(commission) || commission < 0 || commission >= 1)
    throw new Error("total cost must be in [0, 1).");
  if (!Number.isFinite(epsRel) || epsRel < 0)
    throw new Error("eps_rel must be >= 0.");
  if (
    !Number.isFinite(cooldownBars) ||
    cooldownBars < 0 ||
    cooldownBars > 65535
  ) {
    throw new Error("cooldown bars must be in [0, 65535].");
  }
  if (
    !Number.isFinite(signalPersistenceBars) ||
    signalPersistenceBars < 1 ||
    signalPersistenceBars > 65535
  ) {
    throw new Error("signal persistence bars must be in [1, 65535].");
  }
  if (!(exitMode === "cross" || exitMode === "kaufman_stop")) {
    throw new Error("exit mode must be 'cross' or 'kaufman_stop'.");
  }
  if (!Number.isFinite(kaufmanStopMult) || kaufmanStopMult <= 0) {
    throw new Error("kaufman-stop multiplier must be > 0.");
  }

  return {
    longOnly: normalizeBoolean(inputs.longOnly),
    allowFlip: normalizeBoolean(inputs.allowFlip),
    costPreset: normalizeText(inputs.costPreset || "custom") || "custom",
    startingEquity,
    feeBps,
    spreadBps,
    slippageBps,
    commission,
    epsRel,
    stopLossPct,
    takeProfitPct,
    cooldownBars,
    signalPersistenceBars,
    exitMode,
    kaufmanStopPeriod,
    kaufmanStopMult,
    kaufmanStopMaType,
  };
}

function buildStrategyConfigFromInputs(
  inputs: StrategyConfigBuildInputs,
  kindRaw: unknown,
  common: CommonStrategyConfigFields = parseCommonStrategyConfigFields(inputs),
): Record<string, unknown> {
  const kind = normalizeStrategyKindRaw(kindRaw);

  const trendQualityEnable =
    kind === "CompositeTrend" || kind === "RegimeHybrid";
  const trendQualityLinearregPeriod = Math.max(
    1,
    Math.floor(num(inputs.trendLrPeriod, 34)),
  );
  const trendQualityAngleMinDeg = Math.max(
    0.0001,
    num(inputs.trendLrMinDeg, 2.0),
  );
  const trendQualityBbwPeriod = Math.max(
    1,
    Math.floor(num(inputs.trendBbwPeriod, 20)),
  );
  const trendQualityBbwMult = Math.max(0.0001, num(inputs.trendBbwMult, 2.0));
  const trendQualityBbwMin = Math.max(0, num(inputs.trendBbwMin, 0.04));
  const trendQualityRocPeriod = Math.max(
    1,
    Math.floor(num(inputs.trendRocPeriod, 9)),
  );
  const trendQualityUseRoc = normalizeBoolean(inputs.trendUseRoc);
  const trendQualityUseAdx =
    kind === "CompositeTrend" && normalizeBoolean(inputs.trendUseAdx);
  const trendQualityAdxPeriod = Math.max(
    1,
    Math.floor(num(inputs.trendAdxPeriod, 14)),
  );
  const trendQualityAdxMin = Math.max(0.0001, num(inputs.trendAdxMin, 20.0));

  const regimeHybridEnable = kind === "RegimeHybrid";
  const regimeHybridAdxPeriod = Math.max(
    1,
    Math.floor(num(inputs.regimeAdxPeriod, 14)),
  );
  const regimeHybridAdxEnter = Math.max(
    0.0001,
    num(inputs.regimeAdxEnter, 22.0),
  );
  const regimeHybridAdxExit = Math.max(0.0001, num(inputs.regimeAdxExit, 18.0));
  const regimeHybridReversionEntryRel = Math.max(
    0.000001,
    num(inputs.regimeRevEntryRel, 0.0035),
  );
  const regimeHybridReversionExitRel = Math.max(
    0,
    num(inputs.regimeRevExitRel, 0.0012),
  );

  const breakoutMomentumEnable = kind === "BreakoutMomentum";
  const breakoutBbandsPeriod = Math.max(
    1,
    Math.floor(num(inputs.breakoutBbPeriod, 20)),
  );
  const breakoutBbandsMult = Math.max(0.0001, num(inputs.breakoutBbMult, 2.0));
  const breakoutConfirmBars = Math.max(
    1,
    Math.floor(num(inputs.breakoutConfirmBars, 1)),
  );
  const breakoutMacdFastPeriod = Math.max(
    1,
    Math.floor(num(inputs.breakoutMacdFast, 12)),
  );
  const breakoutMacdSlowPeriod = Math.max(
    1,
    Math.floor(num(inputs.breakoutMacdSlow, 26)),
  );
  const breakoutMacdSignalPeriod = Math.max(
    1,
    Math.floor(num(inputs.breakoutMacdSignal, 9)),
  );
  const breakoutCmoPeriod = Math.max(
    1,
    Math.floor(num(inputs.breakoutCmoPeriod, 14)),
  );
  const breakoutCmoMinAbs = Math.max(0, num(inputs.breakoutCmoMinAbs, 20.0));
  const breakoutAdoscShortPeriod = Math.max(
    1,
    Math.floor(num(inputs.breakoutAdoscShort, 3)),
  );
  const breakoutAdoscLongPeriod = Math.max(
    1,
    Math.floor(num(inputs.breakoutAdoscLong, 10)),
  );
  const breakoutUseAdx =
    kind === "BreakoutMomentum" && normalizeBoolean(inputs.breakoutUseAdx);
  const breakoutAdxPeriod = Math.max(
    1,
    Math.floor(num(inputs.breakoutAdxPeriod, 14)),
  );
  const breakoutAdxMin = Math.max(0.0001, num(inputs.breakoutAdxMin, 20.0));

  const pressureImpulseTrendEnable = kind === "Pit";
  const pitEfiPeriod = Math.max(1, Math.floor(num(inputs.pitEfiPeriod, 13)));
  const pitVoscShortPeriod = Math.max(
    1,
    Math.floor(num(inputs.pitVoscShortPeriod, 5)),
  );
  const pitVoscLongPeriod = Math.max(
    pitVoscShortPeriod + 1,
    Math.floor(num(inputs.pitVoscLongPeriod, 20)),
  );
  const pitCfoPeriod = Math.max(1, Math.floor(num(inputs.pitCfoPeriod, 9)));
  const pitCfoScalar = num(inputs.pitCfoScalar, 100.0);
  const pitConfirmBars = Math.max(1, Math.floor(num(inputs.pitConfirmBars, 2)));

  const volumeFlowConfirmEnable = kind === "Vfct";
  const volumeFlowObvSmoothPeriod = Math.max(
    1,
    Math.floor(num(inputs.vfctObvSmoothPeriod, 21)),
  );
  const volumeFlowMfiPeriod = Math.max(
    1,
    Math.floor(num(inputs.vfctMfiPeriod, 14)),
  );
  const volumeFlowMfiLongMin = num(inputs.vfctMfiLongMin, 55.0);
  const volumeFlowMfiShortMax = num(inputs.vfctMfiShortMax, 45.0);
  const volumeFlowConfirmBars = Math.max(
    1,
    Math.floor(num(inputs.vfctConfirmBars, 1)),
  );

  const slopeEfficiencyTrendEnable = kind === "Set";
  const momentumTripleStackEnable = kind === "Mts";
  const oscillatorReversionConfluenceEnable = kind === "Orc";
  if (!Number.isFinite(trendQualityAngleMinDeg) || trendQualityAngleMinDeg <= 0)
    throw new Error("trend min angle must be > 0.");
  if (!Number.isFinite(trendQualityBbwMult) || trendQualityBbwMult <= 0)
    throw new Error("trend BB width multiplier must be > 0.");
  if (!Number.isFinite(trendQualityBbwMin) || trendQualityBbwMin < 0)
    throw new Error("trend min BB width must be >= 0.");
  if (trendQualityUseAdx) {
    if (!Number.isFinite(trendQualityAdxPeriod) || trendQualityAdxPeriod <= 0) {
      throw new Error("trend ADX period must be > 0.");
    }
    if (!Number.isFinite(trendQualityAdxMin) || trendQualityAdxMin <= 0) {
      throw new Error("trend min ADX must be > 0.");
    }
  }
  if (regimeHybridEnable) {
    if (!Number.isFinite(regimeHybridAdxEnter) || regimeHybridAdxEnter <= 0) {
      throw new Error("regime hybrid ADX enter must be > 0.");
    }
    if (
      !Number.isFinite(regimeHybridAdxExit) ||
      regimeHybridAdxExit <= 0 ||
      regimeHybridAdxExit > regimeHybridAdxEnter
    ) {
      throw new Error("regime hybrid ADX exit must be > 0 and <= ADX enter.");
    }
    if (
      !Number.isFinite(regimeHybridReversionEntryRel) ||
      regimeHybridReversionEntryRel <= 0
    ) {
      throw new Error("regime hybrid reversion entry must be > 0.");
    }
    if (
      !Number.isFinite(regimeHybridReversionExitRel) ||
      regimeHybridReversionExitRel < 0 ||
      regimeHybridReversionExitRel > regimeHybridReversionEntryRel
    ) {
      throw new Error(
        "regime hybrid reversion exit must be >= 0 and <= reversion entry.",
      );
    }
  }
  if (breakoutMomentumEnable) {
    if (!Number.isFinite(breakoutBbandsMult) || breakoutBbandsMult <= 0) {
      throw new Error("breakout BB multiplier must be > 0.");
    }
    if (!Number.isFinite(breakoutCmoMinAbs) || breakoutCmoMinAbs < 0) {
      throw new Error("breakout min |CMO| must be >= 0.");
    }
    if (breakoutMacdFastPeriod >= breakoutMacdSlowPeriod) {
      throw new Error("breakout MACD requires fast period < slow period.");
    }
    if (breakoutAdoscShortPeriod >= breakoutAdoscLongPeriod) {
      throw new Error("breakout ADOSC requires short period < long period.");
    }
    if (
      breakoutUseAdx &&
      (!Number.isFinite(breakoutAdxMin) || breakoutAdxMin <= 0)
    ) {
      throw new Error("breakout ADX min must be > 0.");
    }
  }
  if (
    pressureImpulseTrendEnable &&
    (!Number.isFinite(pitCfoScalar) || pitCfoScalar <= 0)
  ) {
    throw new Error("PIT CFO scalar must be > 0.");
  }
  if (volumeFlowConfirmEnable) {
    if (
      !Number.isFinite(volumeFlowMfiLongMin) ||
      volumeFlowMfiLongMin < 0 ||
      volumeFlowMfiLongMin > 100
    ) {
      throw new Error("VFCT MFI long min must be in [0, 100].");
    }
    if (
      !Number.isFinite(volumeFlowMfiShortMax) ||
      volumeFlowMfiShortMax < 0 ||
      volumeFlowMfiShortMax > 100
    ) {
      throw new Error("VFCT MFI short max must be in [0, 100].");
    }
    if (volumeFlowMfiShortMax > volumeFlowMfiLongMin) {
      throw new Error("VFCT MFI short max must be <= MFI long min.");
    }
  }

  return {
    long_only: common.longOnly,
    allow_flip: common.allowFlip,
        trade_on_next_bar: true,
    cost_preset: common.costPreset,
    starting_equity: common.startingEquity,
    fee_bps: common.feeBps,
    spread_bps: common.spreadBps,
    slippage_bps: common.slippageBps,
    commission: common.commission,
    stop_loss_pct: common.stopLossPct,
    take_profit_pct: common.takeProfitPct,
    cooldown_bars: common.cooldownBars,
    signal_persistence_bars: common.signalPersistenceBars,
    exit_mode: common.exitMode,
    kaufman_stop_period: common.kaufmanStopPeriod,
    kaufman_stop_mult: common.kaufmanStopMult,
    kaufman_stop_ma_type: common.kaufmanStopMaType,
    trend_quality_enable: trendQualityEnable,
    trend_quality_linearreg_period: trendQualityLinearregPeriod,
    trend_quality_angle_min_deg: trendQualityAngleMinDeg,
    trend_quality_bbw_period: trendQualityBbwPeriod,
    trend_quality_bbw_mult: trendQualityBbwMult,
    trend_quality_bbw_min: trendQualityBbwMin,
    trend_quality_roc_period: trendQualityRocPeriod,
    trend_quality_use_roc: trendQualityUseRoc,
    trend_quality_use_adx: trendQualityUseAdx,
    trend_quality_adx_period: trendQualityAdxPeriod,
    trend_quality_adx_min: trendQualityAdxMin,
    regime_hybrid_enable: regimeHybridEnable,
    regime_hybrid_adx_period: regimeHybridAdxPeriod,
    regime_hybrid_adx_enter: regimeHybridAdxEnter,
    regime_hybrid_adx_exit: regimeHybridAdxExit,
    regime_hybrid_reversion_entry_rel: regimeHybridReversionEntryRel,
    regime_hybrid_reversion_exit_rel: regimeHybridReversionExitRel,
    breakout_momentum_enable: breakoutMomentumEnable,
    breakout_bbands_period: breakoutBbandsPeriod,
    breakout_bbands_mult: breakoutBbandsMult,
    breakout_confirm_bars: breakoutConfirmBars,
    breakout_macd_fast_period: breakoutMacdFastPeriod,
    breakout_macd_slow_period: breakoutMacdSlowPeriod,
    breakout_macd_signal_period: breakoutMacdSignalPeriod,
    breakout_cmo_period: breakoutCmoPeriod,
    breakout_cmo_min_abs: breakoutCmoMinAbs,
    breakout_adosc_short_period: breakoutAdoscShortPeriod,
    breakout_adosc_long_period: breakoutAdoscLongPeriod,
    breakout_use_adx: breakoutUseAdx,
    breakout_adx_period: breakoutAdxPeriod,
    breakout_adx_min: breakoutAdxMin,
    volume_flow_confirm_enable: volumeFlowConfirmEnable,
    volume_flow_obv_smooth_period: volumeFlowObvSmoothPeriod,
    volume_flow_mfi_period: volumeFlowMfiPeriod,
    volume_flow_mfi_long_min: volumeFlowMfiLongMin,
    volume_flow_mfi_short_max: volumeFlowMfiShortMax,
    volume_flow_confirm_bars: volumeFlowConfirmBars,
    slope_efficiency_trend_enable: slopeEfficiencyTrendEnable,
    set_linearreg_period: 34,
    set_angle_min_deg: 2.0,
    set_er_period: 14,
    set_er_min: 0.3,
    set_chop_period: 14,
    set_chop_max: 45.0,
    momentum_triple_stack_enable: momentumTripleStackEnable,
    mts_roc_period: 10,
    mts_mom_period: 20,
    mts_ppo_fast_period: 12,
    mts_ppo_slow_period: 26,
    oscillator_reversion_confluence_enable: oscillatorReversionConfluenceEnable,
    pressure_impulse_trend_enable: pressureImpulseTrendEnable,
    pit_efi_period: pitEfiPeriod,
    pit_vosc_short_period: pitVoscShortPeriod,
    pit_vosc_long_period: pitVoscLongPeriod,
    pit_cfo_period: pitCfoPeriod,
    pit_cfo_scalar: pitCfoScalar,
    pit_confirm_bars: pitConfirmBars,
    orc_rsi_period: 14,
    orc_stoch_fastk_period: 5,
    orc_stoch_fastd_period: 3,
    orc_willr_period: 14,
    eps_rel: common.epsRel,
  };
}

function parseOptionalRawNumber(value: unknown): number {
  const text = normalizeText(value);
  if (!text) return Number.NaN;
  const parsed = Number(text);
  return Number.isFinite(parsed) ? parsed : Number.NaN;
}

function defaultCompositeMoSearchSpace(): Record<string, unknown> {
  return {
    params: [
      {
        name: "halftrend_amplitude",
        domain: { IntRange: { low: 2, high: 20, step: 1, log: false } },
      },
      {
        name: "halftrend_channel_deviation",
        domain: { FloatRange: { low: 0.5, high: 5.0, log: false } },
      },
      {
        name: "halftrend_atr_period",
        domain: { IntRange: { low: 20, high: 200, step: 1, log: false } },
      },
      {
        name: "macz_fast_length",
        domain: { IntRange: { low: 5, high: 30, step: 1, log: false } },
      },
      {
        name: "macz_slow_length",
        domain: { IntRange: { low: 10, high: 60, step: 1, log: false } },
      },
      {
        name: "macz_signal_length",
        domain: { IntRange: { low: 3, high: 30, step: 1, log: false } },
      },
      {
        name: "macz_lengthz",
        domain: { IntRange: { low: 5, high: 50, step: 1, log: false } },
      },
      {
        name: "macz_length_stdev",
        domain: { IntRange: { low: 5, high: 80, step: 1, log: false } },
      },
      {
        name: "macz_a",
        domain: { FloatRange: { low: -2.0, high: 2.0, log: false } },
      },
      {
        name: "macz_b",
        domain: { FloatRange: { low: -2.0, high: 2.0, log: false } },
      },
      {
        name: "macz_gamma",
        domain: { FloatRange: { low: 0.0, high: 0.2, log: false } },
      },
      { name: "macz_use_lag", domain: "Bool" },
      { name: "macz_use_volume", domain: "Bool" },
      {
        name: "ott_period",
        domain: { IntRange: { low: 1, high: 50, step: 1, log: false } },
      },
      {
        name: "ott_percent",
        domain: { FloatRange: { low: 0.1, high: 5.0, log: false } },
      },
      {
        name: "ott_ma_type",
        domain: {
          Categorical: {
            choices: [
              "VAR",
              "EMA",
              "SMA",
              "WMA",
              "TMA",
              "WWMA",
              "ZLEMA",
              "TSF",
            ],
          },
        },
      },
      {
        name: "strategy_eps_rel",
        domain: { FloatRange: { low: 0.0, high: 0.005, log: false } },
      },
      { name: "strategy_long_only", domain: "Bool" },
      { name: "strategy_allow_flip", domain: "Bool" },
    ],
  };
}

const COMPOSITE_MO_ALLOWED_PARAMS = new Set([
  "fast_ma_type",
  "slow_ma_type",
  "fast_len",
  "slow_len",
  "halftrend_amplitude",
  "halftrend_channel_deviation",
  "halftrend_atr_period",
  "macz_fast_length",
  "macz_slow_length",
  "macz_signal_length",
  "macz_lengthz",
  "macz_length_stdev",
  "macz_a",
  "macz_b",
  "macz_gamma",
  "macz_use_lag",
  "macz_use_volume",
  "ott_period",
  "ott_percent",
  "ott_ma_type",
  "strategy_eps_rel",
  "strategy_long_only",
  "strategy_allow_flip",
]);

const COMPOSITE_MO_LEGACY_FIXED_PARAMS = new Set(["strategy_commission"]);

interface CompositeMoDiscreteSearchEstimate {
  fullyDiscrete: boolean;
  combos: number | null;
}

function hasExplicitCompositeMoInput(value: unknown): boolean {
  return normalizeText(value).length > 0;
}

function serializeCompositeMoChoiceNumber(value: number): string {
  return String(Number(value.toFixed(10)));
}

function parseCompositeMoSearchSpace(raw: unknown): Record<string, unknown> {
  const text = normalizeText(raw);
  if (!text) return defaultCompositeMoSearchSpace();
  let obj: unknown = null;
  try {
    obj = JSON.parse(text);
  } catch (error) {
    throw new Error(`Invalid Composite MO search space JSON: ${String(error)}`);
  }
  if (!obj || typeof obj !== "object")
    throw new Error("Composite MO search space JSON must be an object.");
  const row = obj as Record<string, unknown>;
  if (!Array.isArray(row.params))
    throw new Error(
      "Composite MO search space JSON must contain a params array.",
    );
  return row;
}

function normalizeCompositeMoSearchSpace(
  space: unknown,
  fastDomain: Record<string, unknown>,
  slowDomain: Record<string, unknown>,
): Record<string, unknown> {
  const paramsIn = Array.isArray(
    (space as Record<string, unknown> | null)?.params,
  )
    ? ((space as Record<string, unknown>).params as unknown[])
    : [];
  const params: Array<{ name: string; domain: unknown }> = [];
  const seen = new Set<string>();
  for (const entry of paramsIn) {
    if (!entry || typeof entry !== "object") {
      throw new Error("Composite MO search space params must be objects.");
    }
    const row = entry as Record<string, unknown>;
    const name = normalizeText(row.name);
    if (!name) throw new Error("Composite MO search space params require names.");
    const key = name.toLowerCase();
    if (COMPOSITE_MO_LEGACY_FIXED_PARAMS.has(key)) {
      // Older persisted search spaces exposed execution cost as an optimizer
      // axis. Migrate that legacy shape while keeping costs fixed in
      // base_strategy.
      continue;
    }
    if (!COMPOSITE_MO_ALLOWED_PARAMS.has(key)) {
      throw new Error(`Unsupported Composite MO search-space param "${name}".`);
    }
    if (key === "fast_len" || key === "slow_len") {
      throw new Error(
        `Composite MO search-space param "${name}" is controlled by the main fast/slow range inputs and should not be included in custom JSON.`,
      );
    }
    if (seen.has(key)) {
      throw new Error(`Duplicate Composite MO search-space param "${name}".`);
    }
    if (row.domain == null) {
      throw new Error(`Composite MO search-space param "${name}" is missing a domain.`);
    }
    params.push({ name, domain: row.domain });
    seen.add(key);
  }
  return {
    params: [
      { name: "fast_len", domain: fastDomain },
      { name: "slow_len", domain: slowDomain },
      ...params,
    ],
  };
}

function estimateCompositeMoDiscreteCombos(
  searchSpace: Record<string, unknown>,
  comboLimit = 200_000,
): CompositeMoDiscreteSearchEstimate {
  const params = Array.isArray(searchSpace.params)
    ? (searchSpace.params as Array<Record<string, unknown>>)
    : [];

  let combos = 1;
  for (const param of params) {
    const domain = param?.domain;
    let axisCount: number | null = null;

    if (domain === "Bool") {
      axisCount = 2;
    } else if (domain && typeof domain === "object") {
      const row = domain as Record<string, unknown>;
      if (row.Categorical && typeof row.Categorical === "object") {
        const choices = (row.Categorical as Record<string, unknown>).choices;
        if (Array.isArray(choices)) axisCount = choices.length;
      } else if (row.IntRange && typeof row.IntRange === "object") {
        const range = row.IntRange as Record<string, unknown>;
        axisCount = countF64RangeValues(
          num(range.low, Number.NaN),
          num(range.high, Number.NaN),
          num(range.step, Number.NaN),
        );
      }
    }

    if (!Number.isFinite(axisCount) || axisCount == null || axisCount < 1) {
      return { fullyDiscrete: false, combos: null };
    }

    combos *= Math.max(1, Math.floor(axisCount));
    if (!Number.isFinite(combos) || combos > comboLimit) {
      return { fullyDiscrete: true, combos: comboLimit + 1 };
    }
  }

  return { fullyDiscrete: true, combos };
}

function extractCategoricalStringChoices(
  searchSpace: Record<string, unknown>,
  paramName: string,
): string[] | null {
  const params = Array.isArray(searchSpace.params)
    ? (searchSpace.params as Array<Record<string, unknown>>)
    : [];
  const target = normalizeText(paramName).toLowerCase();
  if (!target) return null;

  for (const param of params) {
    const name = normalizeText(param.name).toLowerCase();
    if (name !== target) continue;
    const domain = param.domain;
    if (!domain || typeof domain !== "object") return null;
    const row = domain as Record<string, unknown>;
    const cat = row.Categorical;
    if (!cat || typeof cat !== "object") return null;
    const choices = (cat as Record<string, unknown>).choices;
    if (!Array.isArray(choices)) return null;
    const dedup = Array.from(
      new Set(
        choices
          .map((choice) => normalizeText(choice).toLowerCase())
          .filter((choice) => !!choice),
      ),
    );
    return dedup.length ? dedup : null;
  }

  return null;
}

function buildCompositeMoDefaultPrescanPairs(
  searchSpace: Record<string, unknown>,
  fastFallback: string,
  slowFallback: string,
  maxPairs = 8,
): Array<{ fast_ma_type: string; slow_ma_type: string }> | null {
  const fastChoices =
    extractCategoricalStringChoices(searchSpace, "fast_ma_type") ?? [
      fastFallback.toLowerCase(),
    ];
  const slowChoices =
    extractCategoricalStringChoices(searchSpace, "slow_ma_type") ?? [
      slowFallback.toLowerCase(),
    ];

  const out: Array<{ fast_ma_type: string; slow_ma_type: string }> = [];
  for (const fast of fastChoices) {
    for (const slow of slowChoices) {
      out.push({ fast_ma_type: fast, slow_ma_type: slow });
      if (out.length > maxPairs) {
        throw new Error(
          `Composite MO prescan would need more than ${maxPairs} MA pairs. Narrow fast/slow MA type choices or specify prescan pairs explicitly.`,
        );
      }
    }
  }
  return out.length ? out : null;
}

function parseCompositeMoPrescanMaPairs(
  raw: unknown,
): Array<{ fast_ma_type: string; slow_ma_type: string }> | null {
  const text = String(raw ?? "");
  const lines = text.split(/\r?\n/);
  const out: Array<{ fast_ma_type: string; slow_ma_type: string }> = [];
  for (const line of lines) {
    const row = normalizeText(line);
    if (!row || row.startsWith("#")) continue;
    const parts = row
      .split(",")
      .map((item) => normalizeText(item))
      .filter((item) => !!item);
    if (parts.length !== 2) {
      throw new Error(
        `Invalid MA pair line "${line}". Use "fast,slow" (one pair per line).`,
      );
    }
    const fastMaType = parts[0] || "";
    const slowMaType = parts[1] || "";
    out.push({ fast_ma_type: fastMaType, slow_ma_type: slowMaType });
  }
  return out.length ? out : null;
}

function parseMoRangeInt(
  low: unknown,
  high: unknown,
  step: unknown,
  label: string,
): Record<string, unknown> {
  const lowInt = Math.floor(num(low, Number.NaN));
  const highInt = Math.floor(num(high, Number.NaN));
  const stepInt = Math.floor(num(step, Number.NaN));
  if (
    !Number.isFinite(lowInt) ||
    !Number.isFinite(highInt) ||
    !Number.isFinite(stepInt)
  ) {
    throw new Error(`${label}: low/high/step must be finite integers.`);
  }
  if (stepInt < 1) throw new Error(`${label}: step must be >= 1.`);
  if (lowInt > highInt) throw new Error(`${label}: low must be <= high.`);
  return {
    IntRange: { low: lowInt, high: highInt, step: stepInt, log: false },
  };
}

function parseMoRangeFloat(
  low: unknown,
  high: unknown,
  step: unknown,
  label: string,
): Record<string, unknown> {
  const lowNum = num(low, Number.NaN);
  const highNum = num(high, Number.NaN);
  const stepNum = num(step, Number.NaN);
  if (
    !Number.isFinite(lowNum) ||
    !Number.isFinite(highNum) ||
    !Number.isFinite(stepNum)
  ) {
    throw new Error(`${label}: low/high/step must be finite numbers.`);
  }
  if (stepNum <= 0) throw new Error(`${label}: step must be > 0.`);
  if (lowNum > highNum) throw new Error(`${label}: low must be <= high.`);
  if (lowNum === highNum) {
    return {
      Categorical: { choices: [serializeCompositeMoChoiceNumber(lowNum)] },
    };
  }

  const count = Math.floor((highNum - lowNum) / stepNum) + 1;
  if (Number.isFinite(count) && count >= 2 && count <= 400) {
    const choices: string[] = [];
    for (let idx = 0; idx < count; idx += 1) {
      const value = lowNum + idx * stepNum;
      if (value > highNum + stepNum * 1e-9) break;
      choices.push(serializeCompositeMoChoiceNumber(value));
    }
    const last = choices.length ? choices[choices.length - 1] : undefined;
    if (
      choices.length >= 2 &&
      typeof last === "string" &&
      Math.abs(Number(last) - highNum) > stepNum * 0.55
    ) {
      choices.push(serializeCompositeMoChoiceNumber(highNum));
    }
    if (choices.length >= 2) return { Categorical: { choices } };
  }
  return { FloatRange: { low: lowNum, high: highNum, log: false } };
}

function parseMoBoolDomain(
  mode: unknown,
): "Bool" | { Categorical: { choices: string[] } } {
  const normalized = normalizeText(mode).toLowerCase() || "both";
  if (normalized === "true") return { Categorical: { choices: ["true"] } };
  if (normalized === "false") return { Categorical: { choices: ["false"] } };
  return "Bool";
}

function parseMoOttMaTypeDomain(raw: unknown): Record<string, unknown> {
  const choices = normalizeText(raw)
    .split(/[\s,]+/g)
    .map((value) => normalizeText(value).toUpperCase())
    .filter((value) => !!value);
  const dedup: string[] = [];
  const seen = new Set<string>();
  for (const choice of choices) {
    if (seen.has(choice)) continue;
    seen.add(choice);
    dedup.push(choice);
  }
  if (!dedup.length) {
    throw new Error(
      "OTT MA types: enter at least one value (comma-separated).",
    );
  }
  return { Categorical: { choices: dedup } };
}

function buildCompositeMoSearchSpaceFromInputs(
  inputs: CompositeMoOptimizationBuildInputs,
): Record<string, unknown> {
  return {
    params: [
      {
        name: "halftrend_amplitude",
        domain: parseMoRangeInt(
          inputs.moHtAmpLow,
          inputs.moHtAmpHigh,
          inputs.moHtAmpStep,
          "HalfTrend amplitude",
        ),
      },
      {
        name: "halftrend_channel_deviation",
        domain: parseMoRangeFloat(
          inputs.moHtDevLow,
          inputs.moHtDevHigh,
          inputs.moHtDevStep,
          "HalfTrend channel deviation",
        ),
      },
      {
        name: "halftrend_atr_period",
        domain: parseMoRangeInt(
          inputs.moHtAtrLow,
          inputs.moHtAtrHigh,
          inputs.moHtAtrStep,
          "HalfTrend ATR period",
        ),
      },
      {
        name: "macz_fast_length",
        domain: parseMoRangeInt(
          inputs.moMaczFastLow,
          inputs.moMaczFastHigh,
          inputs.moMaczFastStep,
          "MACZ fast length",
        ),
      },
      {
        name: "macz_slow_length",
        domain: parseMoRangeInt(
          inputs.moMaczSlowLow,
          inputs.moMaczSlowHigh,
          inputs.moMaczSlowStep,
          "MACZ slow length",
        ),
      },
      {
        name: "macz_signal_length",
        domain: parseMoRangeInt(
          inputs.moMaczSignalLow,
          inputs.moMaczSignalHigh,
          inputs.moMaczSignalStep,
          "MACZ signal length",
        ),
      },
      {
        name: "macz_lengthz",
        domain: parseMoRangeInt(
          inputs.moMaczLengthZLow,
          inputs.moMaczLengthZHigh,
          inputs.moMaczLengthZStep,
          "MACZ lengthz",
        ),
      },
      {
        name: "macz_length_stdev",
        domain: parseMoRangeInt(
          inputs.moMaczLengthStdevLow,
          inputs.moMaczLengthStdevHigh,
          inputs.moMaczLengthStdevStep,
          "MACZ length stdev",
        ),
      },
      {
        name: "macz_gamma",
        domain: parseMoRangeFloat(
          inputs.moMaczGammaLow,
          inputs.moMaczGammaHigh,
          inputs.moMaczGammaStep,
          "MACZ gamma",
        ),
      },
      {
        name: "macz_a",
        domain: parseMoRangeFloat(
          inputs.moMaczALow,
          inputs.moMaczAHigh,
          inputs.moMaczAStep,
          "MACZ a",
        ),
      },
      {
        name: "macz_b",
        domain: parseMoRangeFloat(
          inputs.moMaczBLow,
          inputs.moMaczBHigh,
          inputs.moMaczBStep,
          "MACZ b",
        ),
      },
      {
        name: "macz_use_lag",
        domain: parseMoBoolDomain(inputs.moMaczUseLagMode),
      },
      {
        name: "macz_use_volume",
        domain: parseMoBoolDomain(inputs.moMaczUseVolumeMode),
      },
      {
        name: "ott_period",
        domain: parseMoRangeInt(
          inputs.moOttPeriodLow,
          inputs.moOttPeriodHigh,
          inputs.moOttPeriodStep,
          "OTT period",
        ),
      },
      {
        name: "ott_percent",
        domain: parseMoRangeFloat(
          inputs.moOttPercentLow,
          inputs.moOttPercentHigh,
          inputs.moOttPercentStep,
          "OTT percent",
        ),
      },
      {
        name: "ott_ma_type",
        domain: parseMoOttMaTypeDomain(inputs.moOttMaTypes),
      },
      {
        name: "strategy_eps_rel",
        domain: { FloatRange: { low: 0.0, high: 0.005, log: false } },
      },
      { name: "strategy_long_only", domain: "Bool" },
      { name: "strategy_allow_flip", domain: "Bool" },
    ],
  };
}

function buildCompositeMoSampler(
  inputs: CompositeMoOptimizationBuildInputs,
  searchStats: CompositeMoDiscreteSearchEstimate,
): Record<string, unknown> {
  const samplerKind = normalizeText(inputs.moSampler || "Nsga2") || "Nsga2";
  const discreteCap =
    searchStats.fullyDiscrete && searchStats.combos != null
      ? Math.max(1, Math.floor(searchStats.combos))
      : null;
  if (samplerKind === "Random") {
    const seed = Math.max(0, Math.floor(num(inputs.moRandomSeed, 1)));
    const defaultTrials =
      discreteCap == null ? 2000 : Math.min(2000, discreteCap);
    const trials = Math.max(
      1,
      Math.floor(num(inputs.moRandomTrials, defaultTrials)),
    );
    if (trials > 200_000)
      throw new Error("Random trials too large (max 200000).");
    return { sampler: "Random", config: { seed, trials } };
  }
  if (samplerKind === "Tpe") {
    const seed = Math.max(0, Math.floor(num(inputs.moTpeSeed, 1)));
    const defaultTrials =
      discreteCap == null ? 5000 : Math.min(5000, discreteCap);
    const trials = Math.max(
      1,
      Math.floor(num(inputs.moTpeTrials, defaultTrials)),
    );
    const defaultStartupTrials = Math.max(0, Math.min(10, trials - 1));
    const startupTrials = Math.max(
      0,
      Math.floor(num(inputs.moTpeStartupTrials, defaultStartupTrials)),
    );
    const defaultEiCandidates =
      discreteCap == null ? 24 : Math.min(24, Math.max(1, discreteCap));
    const eiCandidates = Math.max(
      1,
      Math.floor(num(inputs.moTpeCandidates, defaultEiCandidates)),
    );
    const gamma = num(inputs.moTpeGamma, 0.25);
    const defaultBatchSize = Math.max(1, Math.min(64, trials));
    const batchSize = Math.max(
      1,
      Math.floor(num(inputs.moTpeBatchSize, defaultBatchSize)),
    );
    const maxModelPoints = Math.max(
      32,
      Math.floor(num(inputs.moTpeMaxModelPoints, 2048)),
    );
    if (trials > 200_000) throw new Error("TPE trials too large (max 200000).");
    if (!Number.isFinite(gamma) || gamma <= 0 || gamma >= 1)
      throw new Error("TPE gamma must be in (0, 1).");
    if (eiCandidates > 512)
      throw new Error("TPE EI candidates too large (max 512).");
    return {
      sampler: "Tpe",
      config: {
        seed,
        trials,
        startup_trials: startupTrials,
        n_ei_candidates: eiCandidates,
        gamma,
        batch_size: batchSize,
        max_model_points: maxModelPoints,
      },
    };
  }
  if (samplerKind === "Qmc") {
    const seed = Math.max(0, Math.floor(num(inputs.moQmcSeed, 1)));
    const defaultTrials =
      discreteCap == null ? 5000 : Math.min(5000, discreteCap);
    const trials = Math.max(
      1,
      Math.floor(num(inputs.moQmcTrials, defaultTrials)),
    );
    const sequence =
      normalizeText(inputs.moQmcSequence || "sobol").toLowerCase() || "sobol";
    if (trials > 200_000) throw new Error("QMC trials too large (max 200000).");
    return { sampler: "Qmc", config: { seed, trials, sequence } };
  }
  const seed = Math.max(0, Math.floor(num(inputs.moNsga2Seed, 1)));
  const explicitPopulation = hasExplicitCompositeMoInput(inputs.moNsga2Population);
  const defaultPopulation =
    discreteCap == null
      ? 80
      : Math.min(80, Math.max(2, Math.ceil(discreteCap / 2)));
  const populationSize = Math.max(
    2,
    Math.floor(num(inputs.moNsga2Population, defaultPopulation)),
  );
  const explicitGenerations = hasExplicitCompositeMoInput(inputs.moNsga2Generations);
  const defaultGenerations =
    discreteCap == null
      ? 40
      : Math.max(
          1,
          Math.ceil(Math.max(2, discreteCap) / populationSize) - 1,
        );
  const generations = Math.max(
    1,
    Math.floor(num(inputs.moNsga2Generations, defaultGenerations)),
  );
  const crossoverProb = num(inputs.moNsga2CrossoverProb, 0.9);
  const swappingProb = num(inputs.moNsga2SwappingProb, 0.5);
  const mutRaw = normalizeText(inputs.moNsga2MutationProb);
  const mutationProb = mutRaw
    ? num(inputs.moNsga2MutationProb, Number.NaN)
    : null;
  if (
    !Number.isFinite(crossoverProb) ||
    crossoverProb < 0 ||
    crossoverProb > 1
  ) {
    throw new Error("NSGA-II crossover_prob must be in [0, 1].");
  }
  if (!Number.isFinite(swappingProb) || swappingProb < 0 || swappingProb > 1) {
    throw new Error("NSGA-II swapping_prob must be in [0, 1].");
  }
  if (
    mutationProb != null &&
    (!Number.isFinite(mutationProb) || mutationProb < 0 || mutationProb > 1)
  ) {
    throw new Error("NSGA-II mutation_prob must be blank or in [0, 1].");
  }
  let effectivePopulationSize = populationSize;
  let effectiveGenerations = generations;
  if (
    discreteCap != null &&
    !explicitPopulation &&
    !explicitGenerations &&
    effectivePopulationSize * (effectiveGenerations + 1) > discreteCap
  ) {
    effectivePopulationSize = Math.max(
      2,
      Math.min(effectivePopulationSize, discreteCap),
    );
    effectiveGenerations = Math.max(
      1,
      Math.ceil(Math.max(2, discreteCap) / effectivePopulationSize) - 1,
    );
  }
  const evals = effectivePopulationSize * (effectiveGenerations + 1);
  if (evals > 200_000)
    throw new Error(`NSGA-II budget too large (${evals} evals; max 200000).`);
  return {
    sampler: "Nsga2",
    config: {
      seed,
      population_size: effectivePopulationSize,
      generations: effectiveGenerations,
      crossover_prob: crossoverProb,
      swapping_prob: swappingProb,
      mutation_prob: mutationProb == null ? null : mutationProb,
    },
  };
}

export function buildDoubleMaOptimizationRequest(
  inputs: DoubleMaOptimizationBuildInputs,
): Record<string, unknown> | null {
  const kind = normalizeStrategyKindRaw(inputs.context?.strategy_kind);
  if (!isDoubleMaOverlayKind(kind)) return null;

  const dataId = normalizeText(inputs.context?.data_id);
  if (!dataId) throw new Error("Load CSV first (missing data id).");
  const meta = requireMinimumCandleCount(
    inputs.context?.current_data_meta ?? null,
    2,
    "Optimization",
  );

  const fast: [number, number, number] = [
    Number(inputs.fastStart),
    Number(inputs.fastEnd),
    Number(inputs.fastStep),
  ];
  const slow: [number, number, number] = [
    Number(inputs.slowStart),
    Number(inputs.slowEnd),
    Number(inputs.slowStep),
  ];
  const fastRange = parseU16LikeRange(fast[0], fast[1], fast[2]);
  const slowRange = parseU16LikeRange(slow[0], slow[1], slow[2]);
  const pairs = countPairsFastLtSlowRanges(fastRange, slowRange);
  if (!pairs)
    throw new Error("No valid parameter combinations (fast must be < slow).");

  const fastMaType = normalizeText(inputs.fastMaType);
  const slowMaType = normalizeText(inputs.slowMaType);
  if (!fastMaType) throw new Error("Select a fast MA type.");
  if (!slowMaType) throw new Error("Select a slow MA type.");
  if (kind === "Marrt") {
    const fastAllowed = MARRT_MA_WHITELIST.has(fastMaType.toLowerCase());
    const slowAllowed = MARRT_MA_WHITELIST.has(slowMaType.toLowerCase());
    if (!fastAllowed || !slowAllowed) {
      throw new Error(
        "MARRT currently supports only sma, ema, alma, wma, zlema, and vwma.",
      );
    }
  }
  if (kind === "Vfct") {
    const fastAllowed = VFCT_MA_WHITELIST.has(fastMaType.toLowerCase());
    const slowAllowed = VFCT_MA_WHITELIST.has(slowMaType.toLowerCase());
    if (!fastAllowed || !slowAllowed) {
      throw new Error(
        "VFCT currently supports only vwma fast/slow trend lines.",
      );
    }
  }

  const maSource = normalizeText(inputs.maSource || "close") || "close";
  validateCandleFieldRequirements(
    meta,
    fastMaType,
    slowMaType,
    maSource,
  );
  if (kind === "Set") {
    const record = meta;
    if (!record?.has_high || !record?.has_low) {
      throw new Error("SET requires high and low candle fields.");
    }
  }
  if (kind === "Pit") {
    const record = meta;
    if (
      !record?.has_open ||
      !record?.has_high ||
      !record?.has_low ||
      !record?.has_volume
    ) {
      throw new Error(
        "PIT requires open, high, low, and volume candle fields.",
      );
    }
  }
  if (kind === "Vfct") {
    const record = meta;
    if (!record?.has_high || !record?.has_low || !record?.has_volume) {
      throw new Error("VFCT requires high, low, and volume candle fields.");
    }
  }
  if (kind === "CompositeTrend") {
    const record = meta;
    const trendUseAdx = normalizeBoolean(inputs.trendUseAdx);
    if (trendUseAdx && (!record?.has_high || !record?.has_low)) {
      throw new Error(
        "CompositeTrend with ADX enabled requires high and low candle fields.",
      );
    }
  }
  if (kind === "RegimeHybrid") {
    const record = meta;
    if (!record?.has_high || !record?.has_low) {
      throw new Error("RegimeHybrid requires high and low candle fields.");
    }
  }
  if (kind === "BreakoutMomentum") {
    const record = meta;
    if (!record?.has_high || !record?.has_low || !record?.has_volume) {
      throw new Error(
        "BreakoutMomentum requires high, low, and volume candle fields.",
      );
    }
  }
  const objective = normalizeText(inputs.objective || "Sharpe") || "Sharpe";
  const scoreMode = normalizeDoubleMaScoreMode(inputs.scoreMode);
  const hurdleTau = parseOptionalFinite(inputs.hurdleTau);
  const metricContract = normalizeDoubleMaMetricContract(inputs.metricContract);
  const includeTradeMetricsRequested = normalizeBoolean(inputs.includeTradeMetrics);
  if (metricContract === "full_17" && includeTradeMetricsRequested) {
    throw new Error(
      "metricContract=full_17 conflicts with includeTradeMetrics=true.",
    );
  }
  const includeTradeMetrics =
    metricContract === "trade_22" || includeTradeMetricsRequested;
  const extraShortlistObjectives = normalizeExtraShortlistObjectives(
    inputs.extraShortlistObjectives,
    objective,
  );
  const shortlistConfidenceMode = normalizeShortlistConfidenceMode(
    inputs.shortlistConfidenceMode,
  );
  const shortlistUnionMode = normalizeShortlistUnionMode(
    inputs.shortlistUnionMode,
  );
  const shortlistSliceMode = normalizeShortlistSliceMode(
    inputs.shortlistSliceMode,
  );
  const shortlistSliceBenchmark = normalizeShortlistSliceBenchmark(
    inputs.shortlistSliceBenchmark,
  );
  const shortlistNeighborhoodRadius = parseOptionalPositiveInteger(
    inputs.shortlistNeighborhoodRadius,
    "Shortlist neighborhood radius",
  );
  const shortlistNeighborhoodStep = parseOptionalPositiveInteger(
    inputs.shortlistNeighborhoodStep,
    "Shortlist neighborhood step",
  );
  const backend = normalizeBackend(inputs.backend);
  const compositeRoute = resolveCompositeRoutePreview(kind, backend, inputs);
  const maSupport = createMovingAverageSupportLookup(inputs.maSupportMap);
  const hasMaSupportMap = maSupport.hasAny();
  if (hasMaSupportMap) {
    if (backend === "CpuOnly") {
      if (
        !maSupportsCpu(maSupport, fastMaType) ||
        !maSupportsCpu(maSupport, slowMaType)
      ) {
        throw new Error(
          "CPU backend does not support one of the selected MA types. Choose a supported MA or switch backend.",
        );
      }
    } else if (backend === "GpuKernel") {
      const okFast =
        maSupportsCpu(maSupport, fastMaType) ||
        maSupportsCuda(maSupport, fastMaType) ||
        maSupportsKernel(maSupport, fastMaType);
      const okSlow =
        maSupportsCpu(maSupport, slowMaType) ||
        maSupportsCuda(maSupport, slowMaType) ||
        maSupportsKernel(maSupport, slowMaType);
      if (!okFast || !okSlow) {
        throw new Error(
          "GPU unified backend cannot run one of the selected MA types. Choose a supported MA or switch backend.",
        );
      }
    } else {
      const okFast =
        maSupportsCpu(maSupport, fastMaType) ||
        maSupportsCuda(maSupport, fastMaType) ||
        maSupportsKernel(maSupport, fastMaType);
      const okSlow =
        maSupportsCpu(maSupport, slowMaType) ||
        maSupportsCuda(maSupport, slowMaType) ||
        maSupportsKernel(maSupport, slowMaType);
      if (!okFast || !okSlow) {
        throw new Error(
          "Auto backend cannot run the selected MA(s). Choose a period-based, single-output MA that supports CPU or CUDA.",
        );
      }
    }
  }
  const topK = Math.max(0, Math.floor(num(inputs.topK, 0)));
  const includeAll = normalizeIncludeAll(inputs.includeAll);
  const heatmapBins = Math.max(0, Math.floor(num(inputs.heatmapBins, 0)));
  if (heatmapBins > MAX_HEATMAP_BINS)
    throw new Error(`heatmapBins must be <= ${MAX_HEATMAP_BINS}.`);
  const exportAllCsvPath = normalizeText(inputs.exportAllCsvPath);
  const mode = normalizeMode(inputs.mode, {
    includeAll,
    heatmapBins,
    exportAllCsvPath,
  });
  const common = parseCommonStrategyConfigFields(inputs);

  const fastMaParams = normalizeMaParams(inputs.fastMaParams);
  const slowMaParams = normalizeMaParams(inputs.slowMaParams);
  const fastMaParamSweeps = normalizeMaParamSweeps(inputs.fastMaParamSweeps);
  const slowMaParamSweeps = normalizeMaParamSweeps(inputs.slowMaParamSweeps);
  const fastVariantCount = maSweepVariantCount(fastMaParamSweeps);
  const slowVariantCount = maSweepVariantCount(slowMaParamSweeps);
  const variantPairCount = fastVariantCount * slowVariantCount;
  const combos = multiplyClamped(
    [pairs, fastVariantCount, slowVariantCount],
    MAX_INCLUDE_ALL_ROWS,
  );
  if (includeAll && combos > MAX_INCLUDE_ALL_ROWS) {
    throw new Error(
      `includeAll=true would return ${combos} rows; disable it or reduce ranges/sweeps.`,
    );
  }
  if (shortlistSliceMode && variantPairCount > 1) {
    throw new Error(
      "Shortlist slice mode currently requires a single expanded GPU kernel run.",
    );
  }

  const hasExecutionOverlays =
    common.cooldownBars > 0 || common.signalPersistenceBars > 1;
  const kernelRoute =
    compositeRoute != null
      ? compositeRoute.targetBackend === "GpuKernel"
      : backend === "GpuKernel";
  const exportAllAllowedRoute =
    compositeRoute != null
      ? compositeRoute.supportsExportAll
      : backend === "GpuKernel";

  if (kernelRoute) {
    if (!includeAll && topK > MAX_GPU_KERNEL_TOP_K) {
      throw new Error(
        `GPU (kernel) currently supports topK <= ${MAX_GPU_KERNEL_TOP_K} when includeAll=false.`,
      );
    }
    if (common.exitMode !== "cross") {
      throw new Error(
        "GPU (kernel) currently supports exit mode = cross only.",
      );
    }
    if (hasExecutionOverlays) {
      throw new Error(
        "GPU (kernel) does not support cooldown/persistence overlays.",
      );
    }
  }

  if (extraShortlistObjectives) {
    if (!kernelRoute) {
      throw new Error(
        "Extra shortlist objectives are currently supported only for the GPU kernel route.",
      );
    }
    if (topK <= 0) {
      throw new Error("Extra shortlist objectives require topK > 0.");
    }
    if (includeAll) {
      throw new Error("Extra shortlist objectives require includeAll=false.");
    }
    if (scoreMode) {
      throw new Error(
        "Extra shortlist objectives currently require score mode = objective only.",
      );
    }
  }
  if (shortlistConfidenceMode) {
    if (!kernelRoute) {
      throw new Error(
        "Shortlist confidence mode is currently supported only for the GPU kernel route.",
      );
    }
    if (topK <= 0) {
      throw new Error("Shortlist confidence mode requires topK > 0.");
    }
    if (includeAll) {
      throw new Error("Shortlist confidence mode requires includeAll=false.");
    }
    if (scoreMode) {
      throw new Error(
        "Shortlist confidence mode currently requires score mode = objective only.",
      );
    }
  }
  if (shortlistUnionMode) {
    if (!kernelRoute) {
      throw new Error(
        "Shortlist union mode is currently supported only for the GPU kernel route.",
      );
    }
    if (!extraShortlistObjectives) {
      throw new Error(
        "Shortlist union mode requires extra shortlist objectives.",
      );
    }
    if (topK <= 0) {
      throw new Error("Shortlist union mode requires topK > 0.");
    }
    if (includeAll) {
      throw new Error("Shortlist union mode requires includeAll=false.");
    }
    if (scoreMode) {
      throw new Error(
        "Shortlist union mode currently requires score mode = objective only.",
      );
    }
  }
  if (shortlistSliceMode) {
    if (!kernelRoute) {
      throw new Error(
        "Shortlist slice mode is currently supported only for the GPU kernel route.",
      );
    }
    if (topK <= 0) {
      throw new Error("Shortlist slice mode requires topK > 0.");
    }
    if (includeAll) {
      throw new Error("Shortlist slice mode requires includeAll=false.");
    }
  }
  if (shortlistSliceBenchmark && !shortlistSliceMode) {
    throw new Error("Shortlist slice benchmark requires shortlist slice mode.");
  }
  if (shortlistNeighborhoodRadius != null && !shortlistSliceMode) {
    throw new Error(
      "Shortlist neighborhood radius requires shortlist slice mode.",
    );
  }
  if (
    shortlistNeighborhoodStep != null &&
    shortlistNeighborhoodRadius == null
  ) {
    throw new Error(
      "Shortlist neighborhood step requires shortlist neighborhood radius.",
    );
  }
  if (exportAllCsvPath) {
    if (includeAll)
      throw new Error(
        "exportAllCsvPath requires includeAll=false (stream export avoids huge RAM usage).",
      );
    if (mode !== "Grid")
      throw new Error("exportAllCsvPath requires mode=Grid.");
    if (!exportAllAllowedRoute) {
      throw new Error(
        "exportAllCsvPath is currently supported only for the GPU kernel route.",
      );
    }
    if (multiplyClamped([fastVariantCount, slowVariantCount], 1) > 1) {
      throw new Error(
        "exportAllCsvPath is not supported when optimizing MA parameters. Disable MA param sweeps or export.",
      );
    }
  }
  if (extraShortlistObjectives && exportAllCsvPath) {
    throw new Error(
      "Extra shortlist objectives currently require exportAllCsvPath to be empty.",
    );
  }
  if (shortlistConfidenceMode && exportAllCsvPath) {
    throw new Error(
      "Shortlist confidence mode currently requires exportAllCsvPath to be empty.",
    );
  }
  if (shortlistUnionMode && exportAllCsvPath) {
    throw new Error(
      "Shortlist union mode currently requires exportAllCsvPath to be empty.",
    );
  }
  if (shortlistSliceMode && exportAllCsvPath) {
    throw new Error(
      "Shortlist slice mode currently requires exportAllCsvPath to be empty.",
    );
  }

  let vdps: Record<string, unknown> | null = null;
  void hasExecutionOverlays;

  const minTrades = parseOptionalNonNegativeInteger(
    inputs.dmConstraintMinTrades,
  );
  const maxDrawdown = parseOptionalNonNegativeNumber(
    inputs.dmConstraintMaxDd,
    "Max drawdown constraint",
  );
  const minSharpe = parseOptionalFinite(inputs.dmConstraintMinSharpe);
  const minSortino = parseOptionalFinite(inputs.dmConstraintMinSortino);
  const minCalmar = parseOptionalFinite(inputs.dmConstraintMinCalmar);
  const maxVolatility = parseOptionalNonNegativeNumber(
    inputs.dmConstraintMaxVolatility,
    "Max volatility constraint",
  );
  const minExposure = parseOptionalFinite(inputs.dmConstraintMinExposure);
  const maxExposure = parseOptionalFinite(inputs.dmConstraintMaxExposure);
  const maxAbsNetExposure = parseOptionalFinite(
    inputs.dmConstraintMaxAbsNetExposure,
  );
  const minOmega = parseOptionalNonNegativeNumber(
    inputs.dmConstraintMinOmega,
    "Min omega constraint",
  );
  const maxUlcer = parseOptionalNonNegativeNumber(
    inputs.dmConstraintMaxUlcer,
    "Max ulcer constraint",
  );
  const maxDownsideDeviation = parseOptionalNonNegativeNumber(
    inputs.dmConstraintMaxDownsideDeviation,
    "Max downside deviation constraint",
  );
  if (minExposure != null && (minExposure < 0 || minExposure > 1)) {
    throw new Error("Min exposure constraint must be in [0, 1].");
  }
  if (maxExposure != null && (maxExposure < 0 || maxExposure > 1)) {
    throw new Error("Max exposure constraint must be in [0, 1].");
  }
  if (minExposure != null && maxExposure != null && minExposure > maxExposure) {
    throw new Error(
      "Min exposure constraint must be <= max exposure constraint.",
    );
  }
  if (
    maxAbsNetExposure != null &&
    (maxAbsNetExposure < 0 || maxAbsNetExposure > 1)
  ) {
    throw new Error("Max abs net exposure constraint must be in [0, 1].");
  }
  let constraints: Record<string, unknown> | null = null;
  if (
    minTrades != null ||
    maxDrawdown != null ||
    minSharpe != null ||
    minSortino != null ||
    minCalmar != null ||
    maxVolatility != null ||
    minExposure != null ||
    maxExposure != null ||
    maxAbsNetExposure != null ||
    minOmega != null ||
    maxUlcer != null ||
    maxDownsideDeviation != null
  ) {
    constraints = {};
    if (minTrades != null) constraints.min_trades = minTrades;
    if (maxDrawdown != null) constraints.max_drawdown = maxDrawdown;
    if (minSharpe != null) constraints.min_sharpe = minSharpe;
    if (minSortino != null) constraints.min_sortino = minSortino;
    if (minCalmar != null) constraints.min_calmar = minCalmar;
    if (maxVolatility != null) constraints.max_volatility = maxVolatility;
    if (minExposure != null) constraints.min_exposure = minExposure;
    if (maxExposure != null) constraints.max_exposure = maxExposure;
    if (maxAbsNetExposure != null)
      constraints.max_abs_net_exposure = maxAbsNetExposure;
    if (minOmega != null) constraints.min_omega = minOmega;
    if (maxUlcer != null) constraints.max_ulcer = maxUlcer;
    if (maxDownsideDeviation != null) {
      constraints.max_downside_deviation = maxDownsideDeviation;
    }
  }

  const strategy = buildStrategyConfigFromInputs(inputs, kind, common);
  const backendPayload = resolveDoubleMaBackendPayload(
    kind,
    backend,
    strategy,
    compositeRoute,
  );
  const gpuExecution = resolveDoubleMaGpuExecutionPolicy(backendPayload, mode);
  const dynamicStrategyIr =
    kind === "DynamicDoubleMa" &&
    inputs.dynamicStrategyIr &&
    typeof inputs.dynamicStrategyIr === "object" &&
    !Array.isArray(inputs.dynamicStrategyIr)
      ? (inputs.dynamicStrategyIr as Record<string, unknown>)
      : null;

  return {
    strategy_kind: kind,
    backend: backendPayload,
    data_id: dataId,
    starting_equity: common.startingEquity,
    fast_range: fast,
    slow_range: slow,
    fast_ma_types: [fastMaType],
    slow_ma_types: [slowMaType],
    ma_source: maSource,
    export_all_csv_path: exportAllCsvPath || null,
    fast_ma_params: fastMaParams,
    slow_ma_params: slowMaParams,
    fast_ma_param_sweeps: fastMaParamSweeps,
    slow_ma_param_sweeps: slowMaParamSweeps,
    strategy,
    objective,
    score_mode: scoreMode,
    hurdle_tau: hurdleTau,
    extra_shortlist_objectives: extraShortlistObjectives,
    shortlist_confidence_mode: shortlistConfidenceMode,
    shortlist_union_mode: shortlistUnionMode,
    shortlist_slice_mode: shortlistSliceMode,
    shortlist_slice_benchmark: shortlistSliceBenchmark,
    shortlist_neighborhood_radius: shortlistNeighborhoodRadius,
    shortlist_neighborhood_step: shortlistNeighborhoodStep,
    mode,
    vdps,
    top_k: topK,
    include_all: includeAll,
    heatmap_bins: heatmapBins,
    constraints,
    dynamic_strategy_ir: dynamicStrategyIr,
    gpu_execution: gpuExecution,
    persist_followup_cache: true,
    include_trade_metrics: includeTradeMetrics,
    metric_contract: metricContract === "auto" ? null : metricContract,
  };
}

export function buildCompositeCpuOptimizationRequest(
  inputs: CompositeCpuOptimizationBuildInputs,
): Record<string, unknown> | null {
  const kind = normalizeStrategyKindRaw(inputs.context?.strategy_kind);
  if (
    kind !== "Asr" &&
    kind !== "Vcc" &&
    kind !== "Vemr" &&
    kind !== "Rocb" &&
    kind !== "Vcet" &&
    kind !== "Kqsr" &&
    kind !== "Carm" &&
    kind !== "Kvdf" &&
    kind !== "Mkcs" &&
    kind !== "Cdmc" &&
    kind !== "CompositeCpu"
  ) {
    return null;
  }

  const dataId = normalizeText(inputs.context?.data_id);
  if (!dataId) throw new Error("Load CSV first (missing data id).");

  const dataMeta = requireMinimumCandleCount(
    inputs.context?.current_data_meta ?? null,
    2,
    "Optimization",
  );
  const hasOpen = !!dataMeta.has_open;
  const hasHigh = !!dataMeta.has_high;
  const hasLow = !!dataMeta.has_low;
  const hasClose = !!dataMeta.has_close;
  const hasVolume = !!dataMeta.has_volume;
  if (kind === "Kqsr" && (!hasOpen || !hasHigh || !hasLow || !hasClose)) {
    throw new Error("KQSR requires open/high/low/close candle fields.");
  }
  if (
    (kind === "Carm" ||
      kind === "Mkcs" ||
      kind === "CompositeCpu" ||
      kind === "Asr" ||
      kind === "Vcc" ||
      kind === "Vemr" ||
      kind === "Rocb" ||
      kind === "Vcet") &&
    (!hasHigh || !hasLow || !hasClose)
  ) {
    throw new Error(
      "Composite strategy requires high/low candle fields. Load a CSV with high+low columns.",
    );
  }
  if (kind === "Kvdf" && (!hasHigh || !hasLow || !hasClose || !hasVolume)) {
    throw new Error("KVDF requires high/low/close/volume candle fields.");
  }
  if (kind === "Cdmc" && !hasClose) {
    throw new Error("CDMC requires close candle data.");
  }

  const fast: [number, number, number] = [
    Number(inputs.fastStart),
    Number(inputs.fastEnd),
    Number(inputs.fastStep),
  ];
  const slow: [number, number, number] = [
    Number(inputs.slowStart),
    Number(inputs.slowEnd),
    Number(inputs.slowStep),
  ];
  const fastRange = parseU16LikeRange(fast[0], fast[1], fast[2]);
  const slowRange = parseU16LikeRange(slow[0], slow[1], slow[2]);
  if (
    kind === "Vcet" &&
    (!fastRange || fastRange.low < 2 || fastRange.high > 65535)
  ) {
    throw new Error("VCET Fast range must be in [2, 65535].");
  }
  const pairs = countPairsFastLtSlowRanges(fastRange, slowRange);
  if (!pairs)
    throw new Error("No valid parameter combinations (fast must be < slow).");

  const fastMaType = normalizeText(inputs.fastMaType);
  const slowMaType = normalizeText(inputs.slowMaType);
  if (!fastMaType) throw new Error("Select a fast MA type.");
  if (!slowMaType) throw new Error("Select a slow MA type.");
  const maSupport = createMovingAverageSupportLookup(inputs.maSupportMap);
  if (
    kind !== "Asr" &&
    kind !== "Vcc" &&
    kind !== "Vemr" &&
    kind !== "Rocb" &&
    kind !== "Vcet" &&
    kind !== "Kqsr" &&
    kind !== "Carm" &&
    kind !== "Kvdf" &&
    kind !== "Mkcs" &&
    kind !== "Cdmc" &&
    maSupport.hasAny()
  ) {
    if (
      !maSupportsCpu(maSupport, fastMaType) ||
      !maSupportsCpu(maSupport, slowMaType)
    ) {
      throw new Error(
        "Composite strategy is CPU-only and does not support one of the selected MA types.",
      );
    }
  }

  const maSource = normalizeText(inputs.maSource || "close") || "close";
  validateCandleFieldRequirements(
    inputs.context?.current_data_meta ?? null,
    fastMaType,
    slowMaType,
    maSource,
  );

  const amp = Math.floor(num(inputs.halfTrendAmplitude, 2));
  const atrP = Math.floor(num(inputs.halfTrendAtrPeriod, 100));
  const dev = num(inputs.halfTrendChannelDev, 2.0);
  if (!Number.isFinite(amp) || amp < 1 || amp > 65535)
    throw new Error("HalfTrend amplitude must be in [1, 65535].");
  if (!Number.isFinite(atrP) || atrP < 1 || atrP > 65535)
    throw new Error("HalfTrend ATR period must be in [1, 65535].");
  if (!Number.isFinite(dev) || dev < 0)
    throw new Error("HalfTrend channel deviation must be finite and >= 0.");
  const halftrend = {
    amplitude: amp,
    atr_period: atrP,
    channel_deviation: dev,
  };

  const maczFast = Math.floor(num(inputs.maczFastLength, 12));
  const maczSlow = Math.floor(num(inputs.maczSlowLength, 25));
  const maczSig = Math.floor(num(inputs.maczSignalLength, 9));
  const maczLz = Math.floor(num(inputs.maczLengthZ, 20));
  const maczLsd = Math.floor(num(inputs.maczLengthStdev, 25));
  const maczGamma = num(inputs.maczGamma, 0.02);
  const maczA = num(inputs.maczA, 1.0);
  const maczB = num(inputs.maczB, 1.0);
  const maczUseVolume = normalizeBoolean(inputs.maczUseVolume);
  const maczUseLag = normalizeBoolean(inputs.maczUseLag);
  if (!Number.isFinite(maczFast) || maczFast < 1)
    throw new Error("MACZ fast_length must be >= 1.");
  if (!Number.isFinite(maczSlow) || maczSlow < 1)
    throw new Error("MACZ slow_length must be >= 1.");
  if (maczFast >= maczSlow)
    throw new Error("MACZ requires fast_length < slow_length.");
  if (!Number.isFinite(maczSig) || maczSig < 1)
    throw new Error("MACZ signal_length must be >= 1.");
  if (!Number.isFinite(maczLz) || maczLz < 1)
    throw new Error("MACZ lengthz must be >= 1.");
  if (!Number.isFinite(maczLsd) || maczLsd < 1)
    throw new Error("MACZ length_stdev must be >= 1.");
  if (!Number.isFinite(maczA) || maczA < -2 || maczA > 2)
    throw new Error("MACZ a must be in [-2, 2].");
  if (!Number.isFinite(maczB) || maczB < -2 || maczB > 2)
    throw new Error("MACZ b must be in [-2, 2].");
  if (!Number.isFinite(maczGamma) || maczGamma < 0 || maczGamma >= 1)
    throw new Error("MACZ gamma must be in [0, 1).");
  if (maczUseVolume && !dataMeta.has_volume)
    throw new Error("MACZ use_volume requires volume column in the CSV.");
  const macz = {
    fast_length: maczFast,
    slow_length: maczSlow,
    signal_length: maczSig,
    lengthz: maczLz,
    length_stdev: maczLsd,
    a: maczA,
    b: maczB,
    use_lag: maczUseLag,
    gamma: maczGamma,
    use_volume: maczUseVolume,
  };

  const ottPeriod = Math.floor(num(inputs.ottPeriod, 2));
  const ottPercent = num(inputs.ottPercent, 1.4);
  const ottMaType = normalizeText(inputs.ottMaType || "VAR");
  if (!Number.isFinite(ottPeriod) || ottPeriod < 1)
    throw new Error("OTT period must be >= 1.");
  if (!Number.isFinite(ottPercent) || ottPercent < 0)
    throw new Error("OTT percent must be finite and >= 0.");
  if (!ottMaType) throw new Error("OTT MA type is required.");
  const ott = { period: ottPeriod, percent: ottPercent, ma_type: ottMaType };

  const objective = normalizeText(inputs.objective || "Sharpe") || "Sharpe";
  const metricContract = normalizeDoubleMaMetricContract(inputs.metricContract);
  const includeTradeMetricsRequested = normalizeBoolean(inputs.includeTradeMetrics);
  if (metricContract === "full_17" && includeTradeMetricsRequested) {
    throw new Error(
      "metricContract=full_17 conflicts with includeTradeMetrics=true.",
    );
  }
  const includeTradeMetrics =
    metricContract === "trade_22" || includeTradeMetricsRequested;
  const extraShortlistObjectives = normalizeExtraShortlistObjectives(
    inputs.extraShortlistObjectives,
    objective,
  );
  const shortlistConfidenceMode = normalizeShortlistConfidenceMode(
    inputs.shortlistConfidenceMode,
  );
  const shortlistUnionMode = normalizeShortlistUnionMode(
    inputs.shortlistUnionMode,
  );
  const topK = Math.max(0, Math.floor(num(inputs.topK, 0)));
  const includeAll = normalizeIncludeAll(inputs.includeAll);
  const heatmapBins =
    kind === "CompositeCpu"
      ? 0
      : Math.max(0, Math.floor(num(inputs.heatmapBins, 0)));
  if (heatmapBins > MAX_HEATMAP_BINS) {
    throw new Error(`heatmapBins must be <= ${MAX_HEATMAP_BINS}.`);
  }
  if (includeAll && pairs > MAX_INCLUDE_ALL_ROWS) {
    throw new Error(
      `includeAll=true would return ${pairs} rows; disable it or reduce ranges.`,
    );
  }

  const common = parseCommonStrategyConfigFields(inputs);
  const strategy = buildStrategyConfigFromInputs(inputs, kind, common);
  const signalMode = normalizeCompositeSignalMode(inputs.compositeSignalMode);
  const backend = normalizeBackend(inputs.backend);
  const backendPayload =
    kind === "Asr" ||
    kind === "Vcc" ||
    kind === "Vemr" ||
    kind === "Rocb" ||
    kind === "Vcet" ||
    kind === "Kqsr" ||
    kind === "Carm" ||
    kind === "Kvdf" ||
    kind === "Mkcs" ||
    kind === "Cdmc"
      ? backend === "CpuOnly"
        ? { backend: "CpuOnly" }
        : { backend, device_id: 0 }
      : { backend: "CpuOnly" };
  const standaloneGpuShortlistRoute =
    kind !== "CompositeCpu" && backend === "GpuKernel";

  if (extraShortlistObjectives) {
    if (!standaloneGpuShortlistRoute) {
      throw new Error(
        "Extra shortlist objectives are currently supported only for standalone unified-GPU GpuKernel routes.",
      );
    }
    if (topK <= 0) {
      throw new Error("Extra shortlist objectives require topK > 0.");
    }
    if (!includeAll) {
      throw new Error(
        "Standalone extra shortlist objectives currently require includeAll=true.",
      );
    }
  }
  if (shortlistUnionMode) {
    if (!standaloneGpuShortlistRoute) {
      throw new Error(
        "Shortlist union mode is currently supported only for standalone unified-GPU GpuKernel routes.",
      );
    }
    if (!extraShortlistObjectives) {
      throw new Error(
        "Shortlist union mode requires extra shortlist objectives.",
      );
    }
    if (topK <= 0) {
      throw new Error("Shortlist union mode requires topK > 0.");
    }
    if (!includeAll) {
      throw new Error(
        "Standalone shortlist union mode currently requires includeAll=true.",
      );
    }
  }
  if (shortlistConfidenceMode) {
    if (!standaloneGpuShortlistRoute) {
      throw new Error(
        "Shortlist confidence mode is currently supported only for standalone unified-GPU GpuKernel routes.",
      );
    }
    if (topK <= 0) {
      throw new Error("Shortlist confidence mode requires topK > 0.");
    }
  }

  const asr =
    kind === "Asr"
      ? {
          squeeze_bb_mult: Math.max(0.0001, num(inputs.asrSqueezeBbMult, 2.0)),
          squeeze_kc_mult: Math.max(0.0001, num(inputs.asrSqueezeKcMult, 1.5)),
          angle_deg_min: Math.max(0, num(inputs.asrAngleDegMin, 10.0)),
          angle_deg_mr: Math.max(0, num(inputs.asrAngleDegMr, 6.0)),
          use_er_filter:
            inputs.asrUseErFilter == null
              ? true
              : normalizeBoolean(inputs.asrUseErFilter),
          er_min: Math.max(0, num(inputs.asrErMin, 0.3)),
          er_mr: Math.max(0, num(inputs.asrErMr, 0.25)),
          z_ma_type: normalizeText(inputs.asrZMaType || "sma") || "sma",
          z_nbdev: Math.max(0.0001, num(inputs.asrZNbdev, 1.0)),
          z_devtype: Math.max(0, Math.floor(num(inputs.asrZDevtype, 0))),
          z_entry: Math.max(0, num(inputs.asrZEntry, 2.0)),
          z_exit: Math.max(0, num(inputs.asrZExit, 0.5)),
          confirm_bars: Math.max(0, Math.floor(num(inputs.asrConfirmBars, 0))),
          mom_fade_bars: Math.max(1, Math.floor(num(inputs.asrMomFadeBars, 2))),
          max_trade_bars: Math.max(
            1,
            Math.floor(num(inputs.asrMaxTradeBars, 80)),
          ),
          max_mr_trade_bars: Math.max(
            1,
            Math.floor(num(inputs.asrMaxMrTradeBars, 30)),
          ),
        }
      : undefined;
  let vcc: Record<string, unknown> | undefined;
  if (kind === "Vcc") {
    const mapFastToSqueezePeriod =
      inputs.vccMapFastToSqueezePeriod == null
        ? true
        : normalizeBoolean(inputs.vccMapFastToSqueezePeriod);
    const mapFastToMomentumPeriod =
      inputs.vccMapFastToMomentumPeriod == null
        ? true
        : normalizeBoolean(inputs.vccMapFastToMomentumPeriod);
    const mapSlowToTrendPeriod =
      inputs.vccMapSlowToTrendPeriod == null
        ? true
        : normalizeBoolean(inputs.vccMapSlowToTrendPeriod);
    const squeezePeriod = Math.floor(num(inputs.vccSqueezePeriod, 20));
    const momentumPeriod = Math.floor(num(inputs.vccMomentumPeriod, 20));
    const momentumBbPeriod = Math.floor(num(inputs.vccMomentumBbPeriod, 20));
    const trendPeriod = Math.floor(num(inputs.vccTrendPeriod, 6));
    const squeezeBbMult = num(inputs.vccSqueezeBbMult, 2.0);
    const squeezeKcMult = num(inputs.vccSqueezeKcMult, 1.5);
    const confirmBars = Math.floor(num(inputs.vccConfirmBars, 0));
    const requireMomentumAccel =
      inputs.vccRequireMomentumAccel == null
        ? true
        : normalizeBoolean(inputs.vccRequireMomentumAccel);
    const maxTradeBars = Math.floor(num(inputs.vccMaxTradeBars, 120));

    if (
      !Number.isFinite(squeezePeriod) ||
      squeezePeriod < 1 ||
      squeezePeriod > 65535
    ) {
      throw new Error("VCC squeeze period must be in [1, 65535].");
    }
    if (
      !Number.isFinite(momentumPeriod) ||
      momentumPeriod < 1 ||
      momentumPeriod > 65535
    ) {
      throw new Error("VCC momentum period must be in [1, 65535].");
    }
    if (
      !Number.isFinite(momentumBbPeriod) ||
      momentumBbPeriod < 1 ||
      momentumBbPeriod > 65535
    ) {
      throw new Error("VCC momentum BB period must be in [1, 65535].");
    }
    if (
      !Number.isFinite(trendPeriod) ||
      trendPeriod < 1 ||
      trendPeriod > 65535
    ) {
      throw new Error("VCC trend period must be in [1, 65535].");
    }
    if (!Number.isFinite(squeezeBbMult) || squeezeBbMult <= 0) {
      throw new Error("VCC squeeze BB multiplier must be > 0.");
    }
    if (!Number.isFinite(squeezeKcMult) || squeezeKcMult <= 0) {
      throw new Error("VCC squeeze KC multiplier must be > 0.");
    }
    if (
      !Number.isFinite(confirmBars) ||
      confirmBars < 0 ||
      confirmBars > 65535
    ) {
      throw new Error("VCC confirm bars must be in [0, 65535].");
    }
    if (
      !Number.isFinite(maxTradeBars) ||
      maxTradeBars < 1 ||
      maxTradeBars > 65535
    ) {
      throw new Error("VCC max trade bars must be in [1, 65535].");
    }

    vcc = {
      map_fast_to_squeeze_period: mapFastToSqueezePeriod,
      map_fast_to_momentum_period: mapFastToMomentumPeriod,
      map_slow_to_trend_period: mapSlowToTrendPeriod,
      squeeze_period: squeezePeriod,
      momentum_period: momentumPeriod,
      momentum_bb_period: momentumBbPeriod,
      trend_period: trendPeriod,
      squeeze_bb_mult: squeezeBbMult,
      squeeze_kc_mult: squeezeKcMult,
      confirm_bars: confirmBars,
      require_momentum_accel: requireMomentumAccel,
      max_trade_bars: maxTradeBars,
    };
  }

  let vemr: Record<string, unknown> | undefined;
  if (kind === "Vemr") {
    const mapFastToZPeriod =
      inputs.vemrMapFastToZPeriod == null
        ? true
        : normalizeBoolean(inputs.vemrMapFastToZPeriod);
    const mapFastToBbPeriod =
      inputs.vemrMapFastToBbPeriod == null
        ? true
        : normalizeBoolean(inputs.vemrMapFastToBbPeriod);
    const mapSlowToAtrPeriod =
      inputs.vemrMapSlowToAtrPeriod == null
        ? true
        : normalizeBoolean(inputs.vemrMapSlowToAtrPeriod);

    const zPeriod = Math.floor(num(inputs.vemrZPeriod, 20));
    const zMaType = normalizeText(inputs.vemrZMaType || "sma") || "sma";
    const zNbdev = num(inputs.vemrZNbdev, 1.0);
    const zDevtype = Math.floor(num(inputs.vemrZDevtype, 0));
    const zEntry = num(inputs.vemrZEntry, 2.0);
    const zExit = num(inputs.vemrZExit, 0.5);
    const bbPeriod = Math.floor(num(inputs.vemrBbPeriod, 20));
    const bbMult = num(inputs.vemrBbMult, 2.0);
    const requireBandTouch =
      inputs.vemrRequireBandTouch == null
        ? true
        : normalizeBoolean(inputs.vemrRequireBandTouch);
    const atrPeriod = Math.floor(num(inputs.vemrAtrPeriod, 14));
    const atrStopMult = num(inputs.vemrAtrStopMult, 2.0);
    const confirmBars = Math.floor(num(inputs.vemrConfirmBars, 0));
    const maxTradeBars = Math.floor(num(inputs.vemrMaxTradeBars, 80));

    if (!Number.isFinite(zPeriod) || zPeriod < 1 || zPeriod > 65535) {
      throw new Error("VEMR z period must be in [1, 65535].");
    }
    if (!zMaType) throw new Error("VEMR z MA type is required.");
    if (!Number.isFinite(zNbdev) || zNbdev <= 0)
      throw new Error("VEMR z nbdev must be > 0.");
    if (!Number.isFinite(zDevtype) || zDevtype < 0 || zDevtype > 255) {
      throw new Error("VEMR z devtype must be in [0, 255].");
    }
    if (!Number.isFinite(zEntry) || zEntry <= 0)
      throw new Error("VEMR z entry must be > 0.");
    if (!Number.isFinite(zExit) || zExit < 0)
      throw new Error("VEMR z exit must be >= 0.");
    if (zExit > zEntry) throw new Error("VEMR z exit must be <= z entry.");
    if (!Number.isFinite(bbPeriod) || bbPeriod < 1 || bbPeriod > 65535) {
      throw new Error("VEMR BB period must be in [1, 65535].");
    }
    if (!Number.isFinite(bbMult) || bbMult <= 0)
      throw new Error("VEMR BB multiplier must be > 0.");
    if (!Number.isFinite(atrPeriod) || atrPeriod < 1 || atrPeriod > 65535) {
      throw new Error("VEMR ATR period must be in [1, 65535].");
    }
    if (!Number.isFinite(atrStopMult) || atrStopMult <= 0) {
      throw new Error("VEMR ATR stop multiplier must be > 0.");
    }
    if (
      !Number.isFinite(confirmBars) ||
      confirmBars < 0 ||
      confirmBars > 65535
    ) {
      throw new Error("VEMR confirm bars must be in [0, 65535].");
    }
    if (
      !Number.isFinite(maxTradeBars) ||
      maxTradeBars < 1 ||
      maxTradeBars > 65535
    ) {
      throw new Error("VEMR max trade bars must be in [1, 65535].");
    }

    vemr = {
      map_fast_to_z_period: mapFastToZPeriod,
      map_fast_to_bb_period: mapFastToBbPeriod,
      map_slow_to_atr_period: mapSlowToAtrPeriod,
      z_period: zPeriod,
      z_ma_type: zMaType,
      z_nbdev: zNbdev,
      z_devtype: zDevtype,
      z_entry: zEntry,
      z_exit: zExit,
      bb_period: bbPeriod,
      bb_mult: bbMult,
      require_band_touch: requireBandTouch,
      atr_period: atrPeriod,
      atr_stop_mult: atrStopMult,
      confirm_bars: confirmBars,
      max_trade_bars: maxTradeBars,
    };
  }

  let rocb: Record<string, unknown> | undefined;
  if (kind === "Rocb") {
    const mapFastToChannelPeriod =
      inputs.rocbMapFastToChannelPeriod == null
        ? true
        : normalizeBoolean(inputs.rocbMapFastToChannelPeriod);
    const mapSlowToAtrPeriod =
      inputs.rocbMapSlowToAtrPeriod == null
        ? true
        : normalizeBoolean(inputs.rocbMapSlowToAtrPeriod);
    const channelPeriod = Math.floor(num(inputs.rocbChannelPeriod, 20));
    const atrPeriod = Math.floor(num(inputs.rocbAtrPeriod, 14));
    const atrStopMult = num(inputs.rocbAtrStopMult, 2.0);
    const maxTradeBars = Math.floor(num(inputs.rocbMaxTradeBars, 80));

    if (
      !Number.isFinite(channelPeriod) ||
      channelPeriod < 1 ||
      channelPeriod > 65535
    ) {
      throw new Error("ROCB channel period must be in [1, 65535].");
    }
    if (!Number.isFinite(atrPeriod) || atrPeriod < 1 || atrPeriod > 65535) {
      throw new Error("ROCB ATR period must be in [1, 65535].");
    }
    if (!Number.isFinite(atrStopMult) || atrStopMult <= 0) {
      throw new Error("ROCB ATR stop multiplier must be > 0.");
    }
    if (
      !Number.isFinite(maxTradeBars) ||
      maxTradeBars < 1 ||
      maxTradeBars > 65535
    ) {
      throw new Error("ROCB max trade bars must be in [1, 65535].");
    }

    rocb = {
      map_fast_to_channel_period: mapFastToChannelPeriod,
      map_slow_to_atr_period: mapSlowToAtrPeriod,
      channel_period: channelPeriod,
      atr_period: atrPeriod,
      atr_stop_mult: atrStopMult,
      max_trade_bars: maxTradeBars,
    };
  }

  let vcet: Record<string, unknown> | undefined;
  if (kind === "Vcet") {
    const mapFastToVidyaShortPeriod =
      inputs.vcetMapFastToVidyaShortPeriod == null
        ? true
        : normalizeBoolean(inputs.vcetMapFastToVidyaShortPeriod);
    const mapSlowToVidyaLongPeriod =
      inputs.vcetMapSlowToVidyaLongPeriod == null
        ? true
        : normalizeBoolean(inputs.vcetMapSlowToVidyaLongPeriod);
    const mapSlowToCePeriod =
      inputs.vcetMapSlowToCePeriod == null
        ? true
        : normalizeBoolean(inputs.vcetMapSlowToCePeriod);
    const vidyaShortPeriod = Math.floor(num(inputs.vcetVidyaShortPeriod, 6));
    const vidyaLongPeriod = Math.floor(num(inputs.vcetVidyaLongPeriod, 24));
    const vidyaAlpha = num(inputs.vcetVidyaAlpha, 0.2);
    const cePeriod = Math.floor(num(inputs.vcetCePeriod, 22));
    const ceMult = num(inputs.vcetCeMult, 3.0);
    const ceUseClose =
      inputs.vcetCeUseClose == null
        ? true
        : normalizeBoolean(inputs.vcetCeUseClose);
    const requireVidyaSlope =
      inputs.vcetRequireVidyaSlope == null
        ? true
        : normalizeBoolean(inputs.vcetRequireVidyaSlope);
    const maxTradeBars = Math.floor(num(inputs.vcetMaxTradeBars, 120));

    if (
      !Number.isFinite(vidyaShortPeriod) ||
      vidyaShortPeriod < 2 ||
      vidyaShortPeriod > 65535
    ) {
      throw new Error("VCET VIDYA short period must be in [2, 65535].");
    }
    if (
      !Number.isFinite(vidyaLongPeriod) ||
      vidyaLongPeriod < 1 ||
      vidyaLongPeriod > 65535
    ) {
      throw new Error("VCET VIDYA long period must be in [1, 65535].");
    }
    if (vidyaLongPeriod <= vidyaShortPeriod) {
      throw new Error("VCET VIDYA long period must be > short period.");
    }
    if (!Number.isFinite(vidyaAlpha) || vidyaAlpha < 0 || vidyaAlpha > 1) {
      throw new Error("VCET VIDYA alpha must be in [0, 1].");
    }
    if (!Number.isFinite(cePeriod) || cePeriod < 1 || cePeriod > 65535) {
      throw new Error("VCET Chandelier period must be in [1, 65535].");
    }
    if (!Number.isFinite(ceMult) || ceMult <= 0) {
      throw new Error("VCET Chandelier multiplier must be > 0.");
    }
    if (
      !Number.isFinite(maxTradeBars) ||
      maxTradeBars < 1 ||
      maxTradeBars > 65535
    ) {
      throw new Error("VCET max trade bars must be in [1, 65535].");
    }

    vcet = {
      map_fast_to_vidya_short_period: mapFastToVidyaShortPeriod,
      map_slow_to_vidya_long_period: mapSlowToVidyaLongPeriod,
      map_slow_to_ce_period: mapSlowToCePeriod,
      vidya_short_period: vidyaShortPeriod,
      vidya_long_period: vidyaLongPeriod,
      vidya_alpha: vidyaAlpha,
      ce_period: cePeriod,
      ce_mult: ceMult,
      ce_use_close: ceUseClose,
      require_vidya_slope: requireVidyaSlope,
      max_trade_bars: maxTradeBars,
    };
  }

  const mkcs =
    kind === "Mkcs"
      ? {
          map_fast_to_stddev_period: true,
          map_slow_to_breakout_lookback: true,
          stddev_period: 20,
          breakout_lookback: 20,
          compression_threshold: 0.01,
          reexpansion_threshold: 0.03,
        }
      : undefined;
  const kqsr =
    kind === "Kqsr"
      ? {
          kdj_period: 14,
          srsi_period: 14,
          srsi_smooth: 3,
          ift_rsi_period: 9,
          qqe_period: 14,
          oversold: 20,
          overbought: 80,
          prb_period: 20,
          prb_threshold: 0.2,
        }
      : undefined;
  const carm =
    kind === "Carm"
      ? {
          coppock_long_period: 14,
          coppock_short_period: 11,
          coppock_wma_period: 10,
          ao_fast_period: 5,
          ao_slow_period: 34,
          rocr_period: 10,
          apo_fast_period: 12,
          apo_slow_period: 26,
          otto_period: 20,
          otto_threshold: 0,
        }
      : undefined;
  const kvdf =
    kind === "Kvdf"
      ? {
          keltner_period: 20,
          keltner_mult: 1.5,
          vwap_anchor: "session",
          fvg_lookback: 20,
          devstop_period: 14,
          devstop_mult: 2.0,
          dti_period: 14,
          dti_threshold: 20,
        }
      : undefined;
  const cdmc =
    kind === "Cdmc"
      ? {
          dpo_period: 20,
          correlation_cycle_period: 20,
          mama_fast_limit: 0.5,
          mama_slow_limit: 0.05,
          decycler_period: 20,
          phase_threshold: 0,
        }
      : undefined;

  return {
    strategy_kind: kind,
    backend: backendPayload,
    data_id: dataId,
    starting_equity: common.startingEquity,
    fast_range: fast,
    slow_range: slow,
    fast_ma_types: [fastMaType],
    slow_ma_types: [slowMaType],
    ma_source: maSource,
    fast_ma_params: normalizeMaParams(inputs.fastMaParams),
    slow_ma_params: normalizeMaParams(inputs.slowMaParams),
    strategy,
    signal_mode: signalMode,
    halftrend,
    macz,
    ott,
    asr,
    vcc,
    vemr,
    rocb,
    vcet,
    kqsr,
    carm,
    kvdf,
    mkcs,
    cdmc,
    objective,
    include_trade_metrics: includeTradeMetrics,
    metric_contract: metricContract === "auto" ? null : metricContract,
    extra_shortlist_objectives: extraShortlistObjectives,
    shortlist_confidence_mode: shortlistConfidenceMode,
    shortlist_union_mode: shortlistUnionMode,
            mode: COMPOSITE_CPU_PRODUCT_MODE,
    top_k: topK,
    include_all: includeAll,
    heatmap_bins: heatmapBins,
  };
}

export function buildCompositeMoOptimizationRequest(
  inputs: CompositeMoOptimizationBuildInputs,
): Record<string, unknown> | null {
  const kind = normalizeStrategyKindRaw(inputs.context?.strategy_kind);
  if (kind !== "CompositeMo") return null;

  const dataId = normalizeText(inputs.context?.data_id);
  if (!dataId) throw new Error("Load CSV first (missing data id).");

  const dataMeta = requireMinimumCandleCount(
    inputs.context?.current_data_meta ?? null,
    2,
    "Optimization",
  );
  if (!dataMeta.has_high || !dataMeta.has_low) {
    throw new Error(
      "Composite strategy requires high/low candle fields. Load a CSV with high+low columns.",
    );
  }

  const fast: [number, number, number] = [
    Number(inputs.fastStart),
    Number(inputs.fastEnd),
    Number(inputs.fastStep),
  ];
  const slow: [number, number, number] = [
    Number(inputs.slowStart),
    Number(inputs.slowEnd),
    Number(inputs.slowStep),
  ];
  const fastRange = parseU16LikeRange(fast[0], fast[1], fast[2]);
  const slowRange = parseU16LikeRange(slow[0], slow[1], slow[2]);
  const pairs = countPairsFastLtSlowRanges(fastRange, slowRange);
  if (!pairs)
    throw new Error("No valid parameter combinations (fast must be < slow).");

  const fastMaType = normalizeText(inputs.fastMaType);
  const slowMaType = normalizeText(inputs.slowMaType);
  if (!fastMaType) throw new Error("Select a fast MA type.");
  if (!slowMaType) throw new Error("Select a slow MA type.");
  const maSupport = createMovingAverageSupportLookup(inputs.maSupportMap);
  if (maSupport.hasAny()) {
    if (
      !maSupportsCpu(maSupport, fastMaType) ||
      !maSupportsCpu(maSupport, slowMaType)
    ) {
      throw new Error(
        "Composite strategy is CPU-only and does not support one of the selected MA types.",
      );
    }
  }

  const maSource = normalizeText(inputs.maSource || "close") || "close";
  validateCandleFieldRequirements(dataMeta, fastMaType, slowMaType, maSource);

  const common = parseCommonStrategyConfigFields(inputs);
  const baseStrategy = buildStrategyConfigFromInputs(
    inputs,
    "CompositeMo",
    common,
  );
  const signalMode = normalizeCompositeSignalMode(inputs.compositeSignalMode);
  const maximize = normalizeText(inputs.objective || "Sharpe") || "Sharpe";
  const minimize =
    normalizeText(inputs.moObjectiveMin || "MaxDrawdown") || "MaxDrawdown";

  const minTrades = parseOptionalRawNumber(inputs.moConstraintMinTrades);
  const maxDd = parseOptionalRawNumber(inputs.moConstraintMaxDd);
  const maxAbsExposure = parseOptionalRawNumber(
    inputs.moConstraintMaxAbsExposure,
  );
  let constraints: Record<string, unknown> | null = null;
  if (
    Number.isFinite(minTrades) ||
    Number.isFinite(maxDd) ||
    Number.isFinite(maxAbsExposure)
  ) {
    constraints = {};
    if (Number.isFinite(minTrades)) {
      if (minTrades < 0) throw new Error("Min trades must be >= 0.");
      constraints.min_trades = Math.floor(minTrades);
    }
    if (Number.isFinite(maxDd)) {
      if (maxDd < 0) throw new Error("Max drawdown constraint must be >= 0.");
      constraints.max_drawdown = maxDd;
    }
    if (Number.isFinite(maxAbsExposure)) {
      if (maxAbsExposure < 0)
        throw new Error("Max abs exposure constraint must be >= 0.");
      constraints.max_abs_exposure = maxAbsExposure;
    }
  }

  const useRangeBuilder = normalizeBoolean(inputs.moUseRangeBuilder);
  const space = useRangeBuilder
    ? buildCompositeMoSearchSpaceFromInputs(inputs)
    : parseCompositeMoSearchSpace(inputs.moSearchSpace);

  const fastDomain = {
    IntRange: {
      low: fastRange?.low ?? Math.floor(fast[0]),
      high: fastRange?.high ?? Math.floor(fast[0]),
      step: fastRange?.step ?? 1,
      log: false,
    },
  };
  const slowDomain = {
    IntRange: {
      low: slowRange?.low ?? Math.floor(slow[0]),
      high: slowRange?.high ?? Math.floor(slow[0]),
      step: slowRange?.step ?? 1,
      log: false,
    },
  };
  const searchSpace = normalizeCompositeMoSearchSpace(
    space,
    fastDomain,
    slowDomain,
  );
  const searchStats = estimateCompositeMoDiscreteCombos(searchSpace);
  const sampler = buildCompositeMoSampler(inputs, searchStats);

  const prescanDeviceId = parseOptionalNonNegativeU32(
    inputs.moPrescanDeviceId,
    "Composite MO prescan device id",
  );
  let gpuPrescan: Record<string, unknown> | null = null;
  if (normalizeBoolean(inputs.moPrescanEnable)) {
    const topK = Math.max(1, Math.floor(num(inputs.moPrescanTopK, 128)));
    const window = Math.max(0, Math.floor(num(inputs.moPrescanWindow, 8)));
    const predictiveTopK = Math.max(
      0,
      Math.floor(num(inputs.moPrescanPredictiveTopK, 12)),
    );
    const predictiveStride = Math.max(
      1,
      Math.min(65535, Math.floor(num(inputs.moPrescanPredictiveStride, 8))),
    );
    const objectives =
      normalizeText(inputs.moPrescanObjectives || "both") || "both";
    const mode = normalizeText(inputs.moPrescanMode || "Auto") || "Auto";
    const maPairs =
      parseCompositeMoPrescanMaPairs(inputs.moPrescanMaPairs) ??
      buildCompositeMoDefaultPrescanPairs(
        searchSpace,
        fastMaType,
        slowMaType,
      );
    gpuPrescan = {
      top_k: topK,
      window,
      predictive_top_k: predictiveTopK,
      predictive_stride: predictiveStride,
      ...(prescanDeviceId == null ? {} : { device_id: prescanDeviceId }),
      require_gpu: normalizeBoolean(inputs.moPrescanRequireGpu),
      objectives,
      ma_pairs: maPairs,
      mode,
    };
  }

  return {
    strategy_kind: "CompositeMo",
    data_id: dataId,
    starting_equity: common.startingEquity,
    fast_ma_type: fastMaType,
    slow_ma_type: slowMaType,
    ma_source: maSource,
    fast_ma_params: normalizeMaParams(inputs.fastMaParams),
    slow_ma_params: normalizeMaParams(inputs.slowMaParams),
    base_strategy: baseStrategy,
    signal_mode: signalMode,
    search_space: searchSpace,
    objective: maximize,
    objectives: { maximize, minimize },
    sampler,
    constraints,
    gpu_prescan: gpuPrescan,
    mode: null,
  };
}
