import { defineStore } from "pinia";

export interface StrategyOption {
  value: string;
  label: string;
  disabled: boolean;
}

const COST_PRESET_OPTIONS_DEFAULT: StrategyOption[] = [
  { value: "custom", label: "Custom", disabled: false },
  { value: "none", label: "None (0 bps)", disabled: false },
  {
    value: "stocks_liquid",
    label: "Stocks (liquid, conservative)",
    disabled: false,
  },
  { value: "fx_majors", label: "FX (majors, conservative)", disabled: false },
  {
    value: "crypto_spot_maker",
    label: "Crypto spot (maker-ish)",
    disabled: false,
  },
  {
    value: "crypto_spot_taker",
    label: "Crypto spot (taker, conservative)",
    disabled: false,
  },
];

const EXIT_MODE_OPTIONS_DEFAULT: StrategyOption[] = [
  { value: "cross", label: "Cross signal", disabled: false },
  { value: "kaufman_stop", label: "Kaufman-stop", disabled: false },
];

const COST_PRESETS_DEFAULT: Record<
  string,
  {
    feeBps: string | null;
    spreadBps: string | null;
    slippageBps: string | null;
  }
> = {
  custom: { feeBps: null, spreadBps: null, slippageBps: null },
  none: { feeBps: "0", spreadBps: "0", slippageBps: "0" },
  stocks_liquid: { feeBps: "0.10", spreadBps: "1.00", slippageBps: "0.50" },
  fx_majors: { feeBps: "0.00", spreadBps: "1.00", slippageBps: "0.50" },
  crypto_spot_maker: { feeBps: "2.00", spreadBps: "5.00", slippageBps: "1.50" },
  crypto_spot_taker: { feeBps: "7.50", spreadBps: "5.00", slippageBps: "2.50" },
};

function normalizedOptions(values: unknown): StrategyOption[] {
  if (!Array.isArray(values)) return [];
  const entries: StrategyOption[] = [];
  for (const item of values) {
    if (!item || typeof item !== "object") continue;
    const typed = item as Partial<StrategyOption>;
    const value = String(typed.value || "").trim();
    const label = String(typed.label || typed.value || "").trim();
    if (!value && !label) continue;
    entries.push({
      value: value || label,
      label: label || value,
      disabled: Boolean(typed.disabled),
    });
  }
  return entries;
}

function parseNonNegativeNumber(value: string): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed < 0) return 0;
  return parsed;
}

function syncDerivedCostFields(state: StrategyState): void {
  const feeBps = parseNonNegativeNumber(state.feeBps);
  const spreadBps = parseNonNegativeNumber(state.spreadBps);
  const slippageBps = parseNonNegativeNumber(state.slippageBps);
  const perSideBps = feeBps + slippageBps + spreadBps / 2;
  const roundTripBps = perSideBps * 2;
  const commissionFraction = Math.max(
    0,
    Math.min(0.999999, perSideBps / 10_000),
  );

  state.commission = String(commissionFraction);
  state.costHint = `Total: ${perSideBps.toFixed(2)} bps/side (${roundTripBps.toFixed(2)} bps round-trip)`;
}

function applyCostPresetToState(state: StrategyState, presetId: string): void {
  const presetKey = String(presetId || "custom").trim() || "custom";
  const preset = COST_PRESETS_DEFAULT[presetKey] ??
    COST_PRESETS_DEFAULT.custom ?? {
      feeBps: null,
      spreadBps: null,
      slippageBps: null,
    };
  if (preset.feeBps != null) state.feeBps = preset.feeBps;
  if (preset.spreadBps != null) state.spreadBps = preset.spreadBps;
  if (preset.slippageBps != null) state.slippageBps = preset.slippageBps;
  syncDerivedCostFields(state);
}

interface StrategyState {
  activeControlsTab: string;
  costPreset: string;
  costPresetOptions: StrategyOption[];
  feeBps: string;
  spreadBps: string;
  slippageBps: string;
  commission: string;
  epsRel: string;
  costHint: string;
  startingEquity: string;
  stopLossPct: string;
  takeProfitPct: string;
  cooldownBars: string;
  signalPersistenceBars: string;
  compositeSignalMode: string;
  halfTrendAmplitude: string;
  halfTrendAtrPeriod: string;
  halfTrendChannelDev: string;
  maczFastLength: string;
  maczSlowLength: string;
  maczSignalLength: string;
  maczLengthZ: string;
  maczLengthStdev: string;
  maczGamma: string;
  maczA: string;
  maczB: string;
  ottPeriod: string;
  ottPercent: string;
  ottMaType: string;
  trendLrPeriod: string;
  trendLrMinDeg: string;
  trendBbwPeriod: string;
  trendBbwMult: string;
  trendBbwMin: string;
  trendRocPeriod: string;
  trendAdxPeriod: string;
  trendAdxMin: string;
  regimeAdxPeriod: string;
  regimeAdxEnter: string;
  regimeAdxExit: string;
  regimeRevEntryRel: string;
  regimeRevExitRel: string;
  breakoutBbPeriod: string;
  breakoutBbMult: string;
  breakoutConfirmBars: string;
  breakoutMacdFast: string;
  breakoutMacdSlow: string;
  breakoutMacdSignal: string;
  breakoutCmoPeriod: string;
  breakoutCmoMinAbs: string;
  breakoutAdoscShort: string;
  breakoutAdoscLong: string;
  breakoutAdxPeriod: string;
  breakoutAdxMin: string;
  asrSqueezeBbMult: string;
  asrSqueezeKcMult: string;
  asrAngleDegMin: string;
  asrAngleDegMr: string;
  asrErMin: string;
  asrErMr: string;
  asrZMaType: string;
  asrZNbdev: string;
  asrZDevtype: string;
  asrZEntry: string;
  asrZExit: string;
  asrConfirmBars: string;
  asrMomFadeBars: string;
  asrMaxTradeBars: string;
  asrMaxMrTradeBars: string;
  vccSqueezePeriod: string;
  vccMomentumPeriod: string;
  vccMomentumBbPeriod: string;
  vccTrendPeriod: string;
  vccSqueezeBbMult: string;
  vccSqueezeKcMult: string;
  vccConfirmBars: string;
  vccMaxTradeBars: string;
  pitEfiPeriod: string;
  pitVoscShortPeriod: string;
  pitVoscLongPeriod: string;
  pitCfoPeriod: string;
  pitCfoScalar: string;
  pitConfirmBars: string;
  vfctObvSmoothPeriod: string;
  vfctMfiPeriod: string;
  vfctMfiLongMin: string;
  vfctMfiShortMax: string;
  vfctConfirmBars: string;
  rocbChannelPeriod: string;
  rocbAtrPeriod: string;
  rocbAtrStopMult: string;
  rocbMaxTradeBars: string;
  vcetVidyaShortPeriod: string;
  vcetVidyaLongPeriod: string;
  vcetVidyaAlpha: string;
  vcetCePeriod: string;
  vcetCeMult: string;
  vcetMaxTradeBars: string;
  vemrZPeriod: string;
  vemrZMaType: string;
  vemrZNbdev: string;
  vemrZDevtype: string;
  vemrZEntry: string;
  vemrZExit: string;
  vemrBbPeriod: string;
  vemrBbMult: string;
  vemrAtrPeriod: string;
  vemrAtrStopMult: string;
  vemrConfirmBars: string;
  vemrMaxTradeBars: string;
  dmConstraintMinTrades: string;
  dmConstraintMaxDd: string;
  dmConstraintMinSharpe: string;
  dmConstraintMinCalmar: string;
  dmConstraintMaxVolatility: string;
  dmConstraintMinExposure: string;
  dmConstraintMaxExposure: string;
  dmConstraintMaxAbsNetExposure: string;
  dmConstraintMinOmega: string;
  dmConstraintMaxUlcer: string;
  dmConstraintMaxDownsideDeviation: string;
  dmConstraintsVisible: boolean;
  exitMode: string;
  exitModeOptions: StrategyOption[];
  kaufmanStopPeriod: string;
  kaufmanStopMult: string;
  kaufmanStopMaType: string;
  kaufmanStopPeriodVisible: boolean;
  kaufmanStopMultVisible: boolean;
  kaufmanStopMaTypeVisible: boolean;
  longOnly: boolean;
  allowFlip: boolean;
  tradeOnNext: boolean;
  trendUseRoc: boolean;
  trendUseAdx: boolean;
  breakoutUseAdx: boolean;
  maczUseVolume: boolean;
  maczUseLag: boolean;
  asrUseErFilter: boolean;
  vccMapFastToSqueezePeriod: boolean;
  vccMapFastToMomentumPeriod: boolean;
  vccMapSlowToTrendPeriod: boolean;
  vccRequireMomentumAccel: boolean;
  rocbMapFastToChannelPeriod: boolean;
  rocbMapSlowToAtrPeriod: boolean;
  vcetMapFastToVidyaShortPeriod: boolean;
  vcetMapSlowToVidyaLongPeriod: boolean;
  vcetMapSlowToCePeriod: boolean;
  vcetCeUseClose: boolean;
  vcetRequireVidyaSlope: boolean;
  vemrMapFastToZPeriod: boolean;
  vemrMapFastToBbPeriod: boolean;
  vemrMapSlowToAtrPeriod: boolean;
  vemrRequireBandTouch: boolean;
  statusLine: string;
  source: "vue" | "unknown";
  updatedAtUnixMs: number;
}

const STRATEGY_TEXT_FIELDS = [
  "costPreset",
  "feeBps",
  "spreadBps",
  "slippageBps",
  "commission",
  "epsRel",
  "startingEquity",
  "stopLossPct",
  "takeProfitPct",
  "cooldownBars",
  "signalPersistenceBars",
  "compositeSignalMode",
  "halfTrendAmplitude",
  "halfTrendAtrPeriod",
  "halfTrendChannelDev",
  "maczFastLength",
  "maczSlowLength",
  "maczSignalLength",
  "maczLengthZ",
  "maczLengthStdev",
  "maczGamma",
  "maczA",
  "maczB",
  "ottPeriod",
  "ottPercent",
  "ottMaType",
  "trendLrPeriod",
  "trendLrMinDeg",
  "trendBbwPeriod",
  "trendBbwMult",
  "trendBbwMin",
  "trendRocPeriod",
  "trendAdxPeriod",
  "trendAdxMin",
  "regimeAdxPeriod",
  "regimeAdxEnter",
  "regimeAdxExit",
  "regimeRevEntryRel",
  "regimeRevExitRel",
  "breakoutBbPeriod",
  "breakoutBbMult",
  "breakoutConfirmBars",
  "breakoutMacdFast",
  "breakoutMacdSlow",
  "breakoutMacdSignal",
  "breakoutCmoPeriod",
  "breakoutCmoMinAbs",
  "breakoutAdoscShort",
  "breakoutAdoscLong",
  "breakoutAdxPeriod",
  "breakoutAdxMin",
  "asrSqueezeBbMult",
  "asrSqueezeKcMult",
  "asrAngleDegMin",
  "asrAngleDegMr",
  "asrErMin",
  "asrErMr",
  "asrZMaType",
  "asrZNbdev",
  "asrZDevtype",
  "asrZEntry",
  "asrZExit",
  "asrConfirmBars",
  "asrMomFadeBars",
  "asrMaxTradeBars",
  "asrMaxMrTradeBars",
  "vccSqueezePeriod",
  "vccMomentumPeriod",
  "vccMomentumBbPeriod",
  "vccTrendPeriod",
  "vccSqueezeBbMult",
  "vccSqueezeKcMult",
  "vccConfirmBars",
  "vccMaxTradeBars",
  "pitEfiPeriod",
  "pitVoscShortPeriod",
  "pitVoscLongPeriod",
  "pitCfoPeriod",
  "pitCfoScalar",
  "pitConfirmBars",
  "vfctObvSmoothPeriod",
  "vfctMfiPeriod",
  "vfctMfiLongMin",
  "vfctMfiShortMax",
  "vfctConfirmBars",
  "rocbChannelPeriod",
  "rocbAtrPeriod",
  "rocbAtrStopMult",
  "rocbMaxTradeBars",
  "vcetVidyaShortPeriod",
  "vcetVidyaLongPeriod",
  "vcetVidyaAlpha",
  "vcetCePeriod",
  "vcetCeMult",
  "vcetMaxTradeBars",
  "vemrZPeriod",
  "vemrZMaType",
  "vemrZNbdev",
  "vemrZDevtype",
  "vemrZEntry",
  "vemrZExit",
  "vemrBbPeriod",
  "vemrBbMult",
  "vemrAtrPeriod",
  "vemrAtrStopMult",
  "vemrConfirmBars",
  "vemrMaxTradeBars",
  "dmConstraintMinTrades",
  "dmConstraintMaxDd",
  "dmConstraintMinSharpe",
  "dmConstraintMinCalmar",
  "dmConstraintMaxVolatility",
  "dmConstraintMinExposure",
  "dmConstraintMaxExposure",
  "dmConstraintMaxAbsNetExposure",
  "dmConstraintMinOmega",
  "dmConstraintMaxUlcer",
  "dmConstraintMaxDownsideDeviation",
  "exitMode",
  "kaufmanStopPeriod",
  "kaufmanStopMult",
  "kaufmanStopMaType",
] as const;

type StrategyTextField = (typeof STRATEGY_TEXT_FIELDS)[number];
const STRATEGY_TEXT_FIELD_SET: ReadonlySet<string> = new Set(
  STRATEGY_TEXT_FIELDS,
);

const STRATEGY_CHECKBOX_FIELDS = [
  "dmConstraintsVisible",
  "longOnly",
  "allowFlip",
  "tradeOnNext",
  "trendUseRoc",
  "trendUseAdx",
  "breakoutUseAdx",
  "maczUseVolume",
  "maczUseLag",
  "asrUseErFilter",
  "vccMapFastToSqueezePeriod",
  "vccMapFastToMomentumPeriod",
  "vccMapSlowToTrendPeriod",
  "vccRequireMomentumAccel",
  "rocbMapFastToChannelPeriod",
  "rocbMapSlowToAtrPeriod",
  "vcetMapFastToVidyaShortPeriod",
  "vcetMapSlowToVidyaLongPeriod",
  "vcetMapSlowToCePeriod",
  "vcetCeUseClose",
  "vcetRequireVidyaSlope",
  "vemrMapFastToZPeriod",
  "vemrMapFastToBbPeriod",
  "vemrMapSlowToAtrPeriod",
  "vemrRequireBandTouch",
] as const;

type StrategyCheckboxField = (typeof STRATEGY_CHECKBOX_FIELDS)[number];
const STRATEGY_CHECKBOX_FIELD_SET: ReadonlySet<string> = new Set(
  STRATEGY_CHECKBOX_FIELDS,
);

function applyVueTextFieldUpdate(
  state: StrategyState,
  field: string,
  value: string,
): boolean {
  const key = String(field || "").trim();
  if (!STRATEGY_TEXT_FIELD_SET.has(key)) return false;
  const typedKey = key as StrategyTextField;
  (state as unknown as Record<StrategyTextField, string>)[typedKey] = String(
    value || "",
  );
  return true;
}

function applyVueCheckboxFieldUpdate(
  state: StrategyState,
  field: string,
  checked: boolean,
): boolean {
  const key = String(field || "").trim();
  if (!STRATEGY_CHECKBOX_FIELD_SET.has(key)) return false;
  const typedKey = key as StrategyCheckboxField;
  if (typedKey === "tradeOnNext") {
    (state as unknown as Record<StrategyCheckboxField, boolean>)[typedKey] =
      true;
    return true;
  }
  (state as unknown as Record<StrategyCheckboxField, boolean>)[typedKey] =
    !!checked;
  return true;
}

export const useStrategyStore = defineStore("strategy", {
  state: (): StrategyState => ({
    activeControlsTab: "data",
    costPreset: "custom",
    costPresetOptions: [...COST_PRESET_OPTIONS_DEFAULT],
    feeBps: "0",
    spreadBps: "0",
    slippageBps: "0",
    commission: "0",
    epsRel: "0",
    costHint: "Total: 0.00 bps/side (0.00 bps round-trip)",
    startingEquity: "10000",
    stopLossPct: "",
    takeProfitPct: "",
    cooldownBars: "0",
    signalPersistenceBars: "1",
    compositeSignalMode: "",
    halfTrendAmplitude: "",
    halfTrendAtrPeriod: "",
    halfTrendChannelDev: "",
    maczFastLength: "",
    maczSlowLength: "",
    maczSignalLength: "",
    maczLengthZ: "",
    maczLengthStdev: "",
    maczGamma: "",
    maczA: "",
    maczB: "",
    ottPeriod: "",
    ottPercent: "",
    ottMaType: "",
    trendLrPeriod: "",
    trendLrMinDeg: "",
    trendBbwPeriod: "",
    trendBbwMult: "",
    trendBbwMin: "",
    trendRocPeriod: "",
    trendAdxPeriod: "",
    trendAdxMin: "",
    regimeAdxPeriod: "",
    regimeAdxEnter: "",
    regimeAdxExit: "",
    regimeRevEntryRel: "",
    regimeRevExitRel: "",
    breakoutBbPeriod: "",
    breakoutBbMult: "",
    breakoutConfirmBars: "",
    breakoutMacdFast: "",
    breakoutMacdSlow: "",
    breakoutMacdSignal: "",
    breakoutCmoPeriod: "",
    breakoutCmoMinAbs: "",
    breakoutAdoscShort: "",
    breakoutAdoscLong: "",
    breakoutAdxPeriod: "",
    breakoutAdxMin: "",
    asrSqueezeBbMult: "2.0",
    asrSqueezeKcMult: "1.5",
    asrAngleDegMin: "10.0",
    asrAngleDegMr: "6.0",
    asrErMin: "0.30",
    asrErMr: "0.25",
    asrZMaType: "sma",
    asrZNbdev: "1.0",
    asrZDevtype: "0",
    asrZEntry: "2.0",
    asrZExit: "0.5",
    asrConfirmBars: "0",
    asrMomFadeBars: "2",
    asrMaxTradeBars: "80",
    asrMaxMrTradeBars: "30",
    vccSqueezePeriod: "20",
    vccMomentumPeriod: "20",
    vccMomentumBbPeriod: "20",
    vccTrendPeriod: "6",
    vccSqueezeBbMult: "2.0",
    vccSqueezeKcMult: "1.5",
    vccConfirmBars: "0",
    vccMaxTradeBars: "120",
    pitEfiPeriod: "13",
    pitVoscShortPeriod: "5",
    pitVoscLongPeriod: "20",
    pitCfoPeriod: "9",
    pitCfoScalar: "100.0",
    pitConfirmBars: "2",
    vfctObvSmoothPeriod: "21",
    vfctMfiPeriod: "14",
    vfctMfiLongMin: "55",
    vfctMfiShortMax: "45",
    vfctConfirmBars: "1",
    rocbChannelPeriod: "20",
    rocbAtrPeriod: "14",
    rocbAtrStopMult: "2.0",
    rocbMaxTradeBars: "80",
    vcetVidyaShortPeriod: "6",
    vcetVidyaLongPeriod: "24",
    vcetVidyaAlpha: "0.2",
    vcetCePeriod: "22",
    vcetCeMult: "3.0",
    vcetMaxTradeBars: "120",
    vemrZPeriod: "20",
    vemrZMaType: "sma",
    vemrZNbdev: "1.0",
    vemrZDevtype: "0",
    vemrZEntry: "2.0",
    vemrZExit: "0.5",
    vemrBbPeriod: "20",
    vemrBbMult: "2.0",
    vemrAtrPeriod: "14",
    vemrAtrStopMult: "2.0",
    vemrConfirmBars: "0",
    vemrMaxTradeBars: "80",
    dmConstraintMinTrades: "",
    dmConstraintMaxDd: "",
    dmConstraintMinSharpe: "",
    dmConstraintMinCalmar: "",
    dmConstraintMaxVolatility: "",
    dmConstraintMinExposure: "",
    dmConstraintMaxExposure: "",
    dmConstraintMaxAbsNetExposure: "",
    dmConstraintMinOmega: "",
    dmConstraintMaxUlcer: "",
    dmConstraintMaxDownsideDeviation: "",
    dmConstraintsVisible: true,
    exitMode: "cross",
    exitModeOptions: [...EXIT_MODE_OPTIONS_DEFAULT],
    kaufmanStopPeriod: "22",
    kaufmanStopMult: "2.0",
    kaufmanStopMaType: "sma",
    kaufmanStopPeriodVisible: true,
    kaufmanStopMultVisible: true,
    kaufmanStopMaTypeVisible: true,
    longOnly: true,
    allowFlip: true,
    tradeOnNext: true,
    trendUseRoc: true,
    trendUseAdx: false,
    breakoutUseAdx: false,
    maczUseVolume: false,
    maczUseLag: false,
    asrUseErFilter: true,
    vccMapFastToSqueezePeriod: true,
    vccMapFastToMomentumPeriod: true,
    vccMapSlowToTrendPeriod: true,
    vccRequireMomentumAccel: true,
    rocbMapFastToChannelPeriod: true,
    rocbMapSlowToAtrPeriod: true,
    vcetMapFastToVidyaShortPeriod: true,
    vcetMapSlowToVidyaLongPeriod: true,
    vcetMapSlowToCePeriod: true,
    vcetCeUseClose: true,
    vcetRequireVidyaSlope: true,
    vemrMapFastToZPeriod: true,
    vemrMapFastToBbPeriod: true,
    vemrMapSlowToAtrPeriod: true,
    vemrRequireBandTouch: true,
    statusLine: "Ready.",
    source: "unknown",
    updatedAtUnixMs: 0,
  }),

  actions: {
    markVueUpdate(): void {
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();
    },

    applyLegacySnapshot(snapshot: Partial<StrategyState>): void {
      this.activeControlsTab =
        String(
          snapshot.activeControlsTab ?? this.activeControlsTab ?? "data",
        ).trim() || "data";
      this.costPreset =
        String(snapshot.costPreset ?? this.costPreset ?? "custom").trim() ||
        "custom";
      this.costPresetOptions = normalizedOptions(
        snapshot.costPresetOptions ?? this.costPresetOptions ?? [],
      );
      this.feeBps = String(snapshot.feeBps ?? this.feeBps ?? "0").trim() || "0";
      this.spreadBps =
        String(snapshot.spreadBps ?? this.spreadBps ?? "0").trim() || "0";
      this.slippageBps =
        String(snapshot.slippageBps ?? this.slippageBps ?? "0").trim() || "0";
      this.commission =
        String(snapshot.commission ?? this.commission ?? "0").trim() || "0";
      this.epsRel = String(snapshot.epsRel ?? this.epsRel ?? "0").trim() || "0";
      this.costHint =
        String(snapshot.costHint ?? this.costHint ?? "Total: —").trim() ||
        "Total: —";
      this.startingEquity =
        String(
          snapshot.startingEquity ?? this.startingEquity ?? "10000",
        ).trim() || "10000";
      this.stopLossPct = String(
        snapshot.stopLossPct ?? this.stopLossPct ?? "",
      ).trim();
      this.takeProfitPct = String(
        snapshot.takeProfitPct ?? this.takeProfitPct ?? "",
      ).trim();
      this.cooldownBars =
        String(snapshot.cooldownBars ?? this.cooldownBars ?? "0").trim() || "0";
      this.signalPersistenceBars =
        String(
          snapshot.signalPersistenceBars ?? this.signalPersistenceBars ?? "1",
        ).trim() || "1";
      this.compositeSignalMode = String(
        snapshot.compositeSignalMode ?? this.compositeSignalMode ?? "",
      ).trim();
      this.halfTrendAmplitude = String(
        snapshot.halfTrendAmplitude ?? this.halfTrendAmplitude ?? "",
      ).trim();
      this.halfTrendAtrPeriod = String(
        snapshot.halfTrendAtrPeriod ?? this.halfTrendAtrPeriod ?? "",
      ).trim();
      this.halfTrendChannelDev = String(
        snapshot.halfTrendChannelDev ?? this.halfTrendChannelDev ?? "",
      ).trim();
      this.maczFastLength = String(
        snapshot.maczFastLength ?? this.maczFastLength ?? "",
      ).trim();
      this.maczSlowLength = String(
        snapshot.maczSlowLength ?? this.maczSlowLength ?? "",
      ).trim();
      this.maczSignalLength = String(
        snapshot.maczSignalLength ?? this.maczSignalLength ?? "",
      ).trim();
      this.maczLengthZ = String(
        snapshot.maczLengthZ ?? this.maczLengthZ ?? "",
      ).trim();
      this.maczLengthStdev = String(
        snapshot.maczLengthStdev ?? this.maczLengthStdev ?? "",
      ).trim();
      this.maczGamma = String(
        snapshot.maczGamma ?? this.maczGamma ?? "",
      ).trim();
      this.maczA = String(snapshot.maczA ?? this.maczA ?? "").trim();
      this.maczB = String(snapshot.maczB ?? this.maczB ?? "").trim();
      this.ottPeriod = String(
        snapshot.ottPeriod ?? this.ottPeriod ?? "",
      ).trim();
      this.ottPercent = String(
        snapshot.ottPercent ?? this.ottPercent ?? "",
      ).trim();
      this.ottMaType = String(
        snapshot.ottMaType ?? this.ottMaType ?? "",
      ).trim();
      this.trendLrPeriod = String(
        snapshot.trendLrPeriod ?? this.trendLrPeriod ?? "",
      ).trim();
      this.trendLrMinDeg = String(
        snapshot.trendLrMinDeg ?? this.trendLrMinDeg ?? "",
      ).trim();
      this.trendBbwPeriod = String(
        snapshot.trendBbwPeriod ?? this.trendBbwPeriod ?? "",
      ).trim();
      this.trendBbwMult = String(
        snapshot.trendBbwMult ?? this.trendBbwMult ?? "",
      ).trim();
      this.trendBbwMin = String(
        snapshot.trendBbwMin ?? this.trendBbwMin ?? "",
      ).trim();
      this.trendRocPeriod = String(
        snapshot.trendRocPeriod ?? this.trendRocPeriod ?? "",
      ).trim();
      this.trendAdxPeriod = String(
        snapshot.trendAdxPeriod ?? this.trendAdxPeriod ?? "",
      ).trim();
      this.trendAdxMin = String(
        snapshot.trendAdxMin ?? this.trendAdxMin ?? "",
      ).trim();
      this.regimeAdxPeriod = String(
        snapshot.regimeAdxPeriod ?? this.regimeAdxPeriod ?? "",
      ).trim();
      this.regimeAdxEnter = String(
        snapshot.regimeAdxEnter ?? this.regimeAdxEnter ?? "",
      ).trim();
      this.regimeAdxExit = String(
        snapshot.regimeAdxExit ?? this.regimeAdxExit ?? "",
      ).trim();
      this.regimeRevEntryRel = String(
        snapshot.regimeRevEntryRel ?? this.regimeRevEntryRel ?? "",
      ).trim();
      this.regimeRevExitRel = String(
        snapshot.regimeRevExitRel ?? this.regimeRevExitRel ?? "",
      ).trim();
      this.breakoutBbPeriod = String(
        snapshot.breakoutBbPeriod ?? this.breakoutBbPeriod ?? "",
      ).trim();
      this.breakoutBbMult = String(
        snapshot.breakoutBbMult ?? this.breakoutBbMult ?? "",
      ).trim();
      this.breakoutConfirmBars = String(
        snapshot.breakoutConfirmBars ?? this.breakoutConfirmBars ?? "",
      ).trim();
      this.breakoutMacdFast = String(
        snapshot.breakoutMacdFast ?? this.breakoutMacdFast ?? "",
      ).trim();
      this.breakoutMacdSlow = String(
        snapshot.breakoutMacdSlow ?? this.breakoutMacdSlow ?? "",
      ).trim();
      this.breakoutMacdSignal = String(
        snapshot.breakoutMacdSignal ?? this.breakoutMacdSignal ?? "",
      ).trim();
      this.breakoutCmoPeriod = String(
        snapshot.breakoutCmoPeriod ?? this.breakoutCmoPeriod ?? "",
      ).trim();
      this.breakoutCmoMinAbs = String(
        snapshot.breakoutCmoMinAbs ?? this.breakoutCmoMinAbs ?? "",
      ).trim();
      this.breakoutAdoscShort = String(
        snapshot.breakoutAdoscShort ?? this.breakoutAdoscShort ?? "",
      ).trim();
      this.breakoutAdoscLong = String(
        snapshot.breakoutAdoscLong ?? this.breakoutAdoscLong ?? "",
      ).trim();
      this.breakoutAdxPeriod = String(
        snapshot.breakoutAdxPeriod ?? this.breakoutAdxPeriod ?? "",
      ).trim();
      this.breakoutAdxMin = String(
        snapshot.breakoutAdxMin ?? this.breakoutAdxMin ?? "",
      ).trim();
      this.asrSqueezeBbMult =
        String(
          snapshot.asrSqueezeBbMult ?? this.asrSqueezeBbMult ?? "2.0",
        ).trim() || "2.0";
      this.asrSqueezeKcMult =
        String(
          snapshot.asrSqueezeKcMult ?? this.asrSqueezeKcMult ?? "1.5",
        ).trim() || "1.5";
      this.asrAngleDegMin =
        String(
          snapshot.asrAngleDegMin ?? this.asrAngleDegMin ?? "10.0",
        ).trim() || "10.0";
      this.asrAngleDegMr =
        String(snapshot.asrAngleDegMr ?? this.asrAngleDegMr ?? "6.0").trim() ||
        "6.0";
      this.asrErMin =
        String(snapshot.asrErMin ?? this.asrErMin ?? "0.30").trim() || "0.30";
      this.asrErMr =
        String(snapshot.asrErMr ?? this.asrErMr ?? "0.25").trim() || "0.25";
      this.asrZMaType =
        String(snapshot.asrZMaType ?? this.asrZMaType ?? "sma").trim() || "sma";
      this.asrZNbdev =
        String(snapshot.asrZNbdev ?? this.asrZNbdev ?? "1.0").trim() || "1.0";
      this.asrZDevtype =
        String(snapshot.asrZDevtype ?? this.asrZDevtype ?? "0").trim() || "0";
      this.asrZEntry =
        String(snapshot.asrZEntry ?? this.asrZEntry ?? "2.0").trim() || "2.0";
      this.asrZExit =
        String(snapshot.asrZExit ?? this.asrZExit ?? "0.5").trim() || "0.5";
      this.asrConfirmBars =
        String(snapshot.asrConfirmBars ?? this.asrConfirmBars ?? "0").trim() ||
        "0";
      this.asrMomFadeBars =
        String(snapshot.asrMomFadeBars ?? this.asrMomFadeBars ?? "2").trim() ||
        "2";
      this.asrMaxTradeBars =
        String(
          snapshot.asrMaxTradeBars ?? this.asrMaxTradeBars ?? "80",
        ).trim() || "80";
      this.asrMaxMrTradeBars =
        String(
          snapshot.asrMaxMrTradeBars ?? this.asrMaxMrTradeBars ?? "30",
        ).trim() || "30";
      this.vccSqueezePeriod =
        String(
          snapshot.vccSqueezePeriod ?? this.vccSqueezePeriod ?? "20",
        ).trim() || "20";
      this.vccMomentumPeriod =
        String(
          snapshot.vccMomentumPeriod ?? this.vccMomentumPeriod ?? "20",
        ).trim() || "20";
      this.vccMomentumBbPeriod =
        String(
          snapshot.vccMomentumBbPeriod ?? this.vccMomentumBbPeriod ?? "20",
        ).trim() || "20";
      this.vccTrendPeriod =
        String(snapshot.vccTrendPeriod ?? this.vccTrendPeriod ?? "6").trim() ||
        "6";
      this.vccSqueezeBbMult =
        String(
          snapshot.vccSqueezeBbMult ?? this.vccSqueezeBbMult ?? "2.0",
        ).trim() || "2.0";
      this.vccSqueezeKcMult =
        String(
          snapshot.vccSqueezeKcMult ?? this.vccSqueezeKcMult ?? "1.5",
        ).trim() || "1.5";
      this.vccConfirmBars =
        String(snapshot.vccConfirmBars ?? this.vccConfirmBars ?? "0").trim() ||
        "0";
      this.vccMaxTradeBars =
        String(
          snapshot.vccMaxTradeBars ?? this.vccMaxTradeBars ?? "120",
        ).trim() || "120";
      this.pitEfiPeriod =
        String(snapshot.pitEfiPeriod ?? this.pitEfiPeriod ?? "13").trim() ||
        "13";
      this.pitVoscShortPeriod =
        String(
          snapshot.pitVoscShortPeriod ?? this.pitVoscShortPeriod ?? "5",
        ).trim() || "5";
      this.pitVoscLongPeriod =
        String(
          snapshot.pitVoscLongPeriod ?? this.pitVoscLongPeriod ?? "20",
        ).trim() || "20";
      this.pitCfoPeriod =
        String(snapshot.pitCfoPeriod ?? this.pitCfoPeriod ?? "9").trim() || "9";
      this.pitCfoScalar =
        String(snapshot.pitCfoScalar ?? this.pitCfoScalar ?? "100.0").trim() ||
        "100.0";
      this.pitConfirmBars =
        String(snapshot.pitConfirmBars ?? this.pitConfirmBars ?? "2").trim() ||
        "2";
      this.vfctObvSmoothPeriod =
        String(
          snapshot.vfctObvSmoothPeriod ?? this.vfctObvSmoothPeriod ?? "21",
        ).trim() || "21";
      this.vfctMfiPeriod =
        String(snapshot.vfctMfiPeriod ?? this.vfctMfiPeriod ?? "14").trim() ||
        "14";
      this.vfctMfiLongMin =
        String(snapshot.vfctMfiLongMin ?? this.vfctMfiLongMin ?? "55").trim() ||
        "55";
      this.vfctMfiShortMax =
        String(
          snapshot.vfctMfiShortMax ?? this.vfctMfiShortMax ?? "45",
        ).trim() || "45";
      this.vfctConfirmBars =
        String(
          snapshot.vfctConfirmBars ?? this.vfctConfirmBars ?? "1",
        ).trim() || "1";
      this.rocbChannelPeriod =
        String(
          snapshot.rocbChannelPeriod ?? this.rocbChannelPeriod ?? "20",
        ).trim() || "20";
      this.rocbAtrPeriod =
        String(snapshot.rocbAtrPeriod ?? this.rocbAtrPeriod ?? "14").trim() ||
        "14";
      this.rocbAtrStopMult =
        String(
          snapshot.rocbAtrStopMult ?? this.rocbAtrStopMult ?? "2.0",
        ).trim() || "2.0";
      this.rocbMaxTradeBars =
        String(
          snapshot.rocbMaxTradeBars ?? this.rocbMaxTradeBars ?? "80",
        ).trim() || "80";
      this.vcetVidyaShortPeriod =
        String(
          snapshot.vcetVidyaShortPeriod ?? this.vcetVidyaShortPeriod ?? "6",
        ).trim() || "6";
      this.vcetVidyaLongPeriod =
        String(
          snapshot.vcetVidyaLongPeriod ?? this.vcetVidyaLongPeriod ?? "24",
        ).trim() || "24";
      this.vcetVidyaAlpha =
        String(
          snapshot.vcetVidyaAlpha ?? this.vcetVidyaAlpha ?? "0.2",
        ).trim() || "0.2";
      this.vcetCePeriod =
        String(snapshot.vcetCePeriod ?? this.vcetCePeriod ?? "22").trim() ||
        "22";
      this.vcetCeMult =
        String(snapshot.vcetCeMult ?? this.vcetCeMult ?? "3.0").trim() || "3.0";
      this.vcetMaxTradeBars =
        String(
          snapshot.vcetMaxTradeBars ?? this.vcetMaxTradeBars ?? "120",
        ).trim() || "120";
      this.vemrZPeriod =
        String(snapshot.vemrZPeriod ?? this.vemrZPeriod ?? "20").trim() || "20";
      this.vemrZMaType =
        String(snapshot.vemrZMaType ?? this.vemrZMaType ?? "sma").trim() ||
        "sma";
      this.vemrZNbdev =
        String(snapshot.vemrZNbdev ?? this.vemrZNbdev ?? "1.0").trim() || "1.0";
      this.vemrZDevtype =
        String(snapshot.vemrZDevtype ?? this.vemrZDevtype ?? "0").trim() || "0";
      this.vemrZEntry =
        String(snapshot.vemrZEntry ?? this.vemrZEntry ?? "2.0").trim() || "2.0";
      this.vemrZExit =
        String(snapshot.vemrZExit ?? this.vemrZExit ?? "0.5").trim() || "0.5";
      this.vemrBbPeriod =
        String(snapshot.vemrBbPeriod ?? this.vemrBbPeriod ?? "20").trim() ||
        "20";
      this.vemrBbMult =
        String(snapshot.vemrBbMult ?? this.vemrBbMult ?? "2.0").trim() || "2.0";
      this.vemrAtrPeriod =
        String(snapshot.vemrAtrPeriod ?? this.vemrAtrPeriod ?? "14").trim() ||
        "14";
      this.vemrAtrStopMult =
        String(
          snapshot.vemrAtrStopMult ?? this.vemrAtrStopMult ?? "2.0",
        ).trim() || "2.0";
      this.vemrConfirmBars =
        String(
          snapshot.vemrConfirmBars ?? this.vemrConfirmBars ?? "0",
        ).trim() || "0";
      this.vemrMaxTradeBars =
        String(
          snapshot.vemrMaxTradeBars ?? this.vemrMaxTradeBars ?? "80",
        ).trim() || "80";
      this.dmConstraintMinTrades = String(
        snapshot.dmConstraintMinTrades ?? this.dmConstraintMinTrades ?? "",
      ).trim();
      this.dmConstraintMaxDd = String(
        snapshot.dmConstraintMaxDd ?? this.dmConstraintMaxDd ?? "",
      ).trim();
      this.dmConstraintMinSharpe = String(
        snapshot.dmConstraintMinSharpe ?? this.dmConstraintMinSharpe ?? "",
      ).trim();
      this.dmConstraintMinCalmar = String(
        snapshot.dmConstraintMinCalmar ?? this.dmConstraintMinCalmar ?? "",
      ).trim();
      this.dmConstraintMaxVolatility = String(
        snapshot.dmConstraintMaxVolatility ??
          this.dmConstraintMaxVolatility ??
          "",
      ).trim();
      this.dmConstraintMinExposure = String(
        snapshot.dmConstraintMinExposure ?? this.dmConstraintMinExposure ?? "",
      ).trim();
      this.dmConstraintMaxExposure = String(
        snapshot.dmConstraintMaxExposure ?? this.dmConstraintMaxExposure ?? "",
      ).trim();
      this.dmConstraintMaxAbsNetExposure = String(
        snapshot.dmConstraintMaxAbsNetExposure ??
          this.dmConstraintMaxAbsNetExposure ??
          "",
      ).trim();
      this.dmConstraintMinOmega = String(
        snapshot.dmConstraintMinOmega ?? this.dmConstraintMinOmega ?? "",
      ).trim();
      this.dmConstraintMaxUlcer = String(
        snapshot.dmConstraintMaxUlcer ?? this.dmConstraintMaxUlcer ?? "",
      ).trim();
      this.dmConstraintMaxDownsideDeviation = String(
        snapshot.dmConstraintMaxDownsideDeviation ??
          this.dmConstraintMaxDownsideDeviation ??
          "",
      ).trim();
      this.dmConstraintsVisible = Boolean(
        snapshot.dmConstraintsVisible ?? this.dmConstraintsVisible,
      );
      this.exitMode =
        String(snapshot.exitMode ?? this.exitMode ?? "cross").trim() || "cross";
      this.exitModeOptions = normalizedOptions(
        snapshot.exitModeOptions ?? this.exitModeOptions ?? [],
      );
      this.kaufmanStopPeriod =
        String(
          snapshot.kaufmanStopPeriod ?? this.kaufmanStopPeriod ?? "22",
        ).trim() || "22";
      this.kaufmanStopMult =
        String(
          snapshot.kaufmanStopMult ?? this.kaufmanStopMult ?? "2.0",
        ).trim() || "2.0";
      this.kaufmanStopMaType =
        String(
          snapshot.kaufmanStopMaType ?? this.kaufmanStopMaType ?? "sma",
        ).trim() || "sma";
      this.kaufmanStopPeriodVisible = Boolean(
        snapshot.kaufmanStopPeriodVisible ?? this.kaufmanStopPeriodVisible,
      );
      this.kaufmanStopMultVisible = Boolean(
        snapshot.kaufmanStopMultVisible ?? this.kaufmanStopMultVisible,
      );
      this.kaufmanStopMaTypeVisible = Boolean(
        snapshot.kaufmanStopMaTypeVisible ?? this.kaufmanStopMaTypeVisible,
      );
      this.longOnly = Boolean(snapshot.longOnly ?? this.longOnly);
      this.allowFlip = Boolean(snapshot.allowFlip ?? this.allowFlip);
      this.tradeOnNext = true;
      this.trendUseRoc = Boolean(snapshot.trendUseRoc ?? this.trendUseRoc);
      this.trendUseAdx = Boolean(snapshot.trendUseAdx ?? this.trendUseAdx);
      this.breakoutUseAdx = Boolean(
        snapshot.breakoutUseAdx ?? this.breakoutUseAdx,
      );
      this.maczUseVolume = Boolean(
        snapshot.maczUseVolume ?? this.maczUseVolume,
      );
      this.maczUseLag = Boolean(snapshot.maczUseLag ?? this.maczUseLag);
      this.asrUseErFilter = Boolean(
        snapshot.asrUseErFilter ?? this.asrUseErFilter,
      );
      this.vccMapFastToSqueezePeriod = Boolean(
        snapshot.vccMapFastToSqueezePeriod ?? this.vccMapFastToSqueezePeriod,
      );
      this.vccMapFastToMomentumPeriod = Boolean(
        snapshot.vccMapFastToMomentumPeriod ?? this.vccMapFastToMomentumPeriod,
      );
      this.vccMapSlowToTrendPeriod = Boolean(
        snapshot.vccMapSlowToTrendPeriod ?? this.vccMapSlowToTrendPeriod,
      );
      this.vccRequireMomentumAccel = Boolean(
        snapshot.vccRequireMomentumAccel ?? this.vccRequireMomentumAccel,
      );
      this.rocbMapFastToChannelPeriod = Boolean(
        snapshot.rocbMapFastToChannelPeriod ?? this.rocbMapFastToChannelPeriod,
      );
      this.rocbMapSlowToAtrPeriod = Boolean(
        snapshot.rocbMapSlowToAtrPeriod ?? this.rocbMapSlowToAtrPeriod,
      );
      this.vcetMapFastToVidyaShortPeriod = Boolean(
        snapshot.vcetMapFastToVidyaShortPeriod ??
        this.vcetMapFastToVidyaShortPeriod,
      );
      this.vcetMapSlowToVidyaLongPeriod = Boolean(
        snapshot.vcetMapSlowToVidyaLongPeriod ??
        this.vcetMapSlowToVidyaLongPeriod,
      );
      this.vcetMapSlowToCePeriod = Boolean(
        snapshot.vcetMapSlowToCePeriod ?? this.vcetMapSlowToCePeriod,
      );
      this.vcetCeUseClose = Boolean(
        snapshot.vcetCeUseClose ?? this.vcetCeUseClose,
      );
      this.vcetRequireVidyaSlope = Boolean(
        snapshot.vcetRequireVidyaSlope ?? this.vcetRequireVidyaSlope,
      );
      this.vemrMapFastToZPeriod = Boolean(
        snapshot.vemrMapFastToZPeriod ?? this.vemrMapFastToZPeriod,
      );
      this.vemrMapFastToBbPeriod = Boolean(
        snapshot.vemrMapFastToBbPeriod ?? this.vemrMapFastToBbPeriod,
      );
      this.vemrMapSlowToAtrPeriod = Boolean(
        snapshot.vemrMapSlowToAtrPeriod ?? this.vemrMapSlowToAtrPeriod,
      );
      this.vemrRequireBandTouch = Boolean(
        snapshot.vemrRequireBandTouch ?? this.vemrRequireBandTouch,
      );
      this.statusLine =
        String(snapshot.statusLine ?? this.statusLine ?? "Ready.").trim() ||
        "Ready.";
      this.source = "vue";
      this.updatedAtUnixMs = Date.now();
    },

    setInputField(field: string, value: string): void {
      const key = String(field || "").trim();
      const next = String(value || "");
      if (!applyVueTextFieldUpdate(this, key, next)) return;
      if (key === "feeBps" || key === "spreadBps" || key === "slippageBps") {
        syncDerivedCostFields(this);
      }
      this.markVueUpdate();
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || "").trim();
      const next = String(value || "");
      if (!applyVueTextFieldUpdate(this, key, next)) return;
      if (key === "costPreset") {
        applyCostPresetToState(this, next);
      }
      this.markVueUpdate();
    },

    setCheckboxField(field: string, checked: boolean): void {
      const key = String(field || "").trim();
      const next = !!checked;
      if (applyVueCheckboxFieldUpdate(this, key, next)) this.markVueUpdate();
    },
  },
});
