<script setup lang="ts">
import {
  computed,
  onBeforeUnmount,
  ref,
  watch,
} from "vue";
import { storeToRefs } from "pinia";
import { useShellMount } from "../composables/useShellMount";
import { useQueuedInputWriter } from "../composables/useQueuedInputWriter";
import { useStrategyStore, type StrategyOption } from "../stores/strategyStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useWorkflowStore } from "../stores/workflowStore";
import { describeExecutionRisk } from "../utils/executionRisk";
import { formatUiTimeFromUnixMs } from "../utils/uiTime";

const strategyStore = useStrategyStore();
const optimizeStore = useOptimizeStore();
const workflowStore = useWorkflowStore();
const {
  costPreset,
  costPresetOptions,
  feeBps,
  spreadBps,
  slippageBps,
  commission,
  epsRel,
  costHint,
  startingEquity,
  stopLossPct,
  takeProfitPct,
  cooldownBars,
  signalPersistenceBars,
  dmConstraintMinTrades,
  dmConstraintMaxDd,
  dmConstraintMinSharpe,
  dmConstraintMinSortino,
  dmConstraintMinCalmar,
  dmConstraintMaxVolatility,
  dmConstraintMinExposure,
  dmConstraintMaxExposure,
  dmConstraintMaxAbsNetExposure,
  dmConstraintMinOmega,
  dmConstraintMaxUlcer,
  dmConstraintMaxDownsideDeviation,
  exitMode,
  exitModeOptions,
  kaufmanStopPeriod,
  kaufmanStopMult,
  kaufmanStopMaType,
  asrUseErFilter,
  asrSqueezeBbMult,
  asrSqueezeKcMult,
  asrAngleDegMin,
  asrAngleDegMr,
  asrErMin,
  asrErMr,
  asrZMaType,
  asrZNbdev,
  asrZDevtype,
  asrZEntry,
  asrZExit,
  asrConfirmBars,
  asrMomFadeBars,
  asrMaxTradeBars,
  asrMaxMrTradeBars,
  vccSqueezePeriod,
  vccMomentumPeriod,
  vccMomentumBbPeriod,
  vccTrendPeriod,
  vccSqueezeBbMult,
  vccSqueezeKcMult,
  vccConfirmBars,
  vccMaxTradeBars,
  pitEfiPeriod,
  pitVoscShortPeriod,
  pitVoscLongPeriod,
  pitCfoPeriod,
  pitCfoScalar,
  pitConfirmBars,
  vfctObvSmoothPeriod,
  vfctMfiPeriod,
  vfctMfiLongMin,
  vfctMfiShortMax,
  vfctConfirmBars,
  rocbChannelPeriod,
  rocbAtrPeriod,
  rocbAtrStopMult,
  rocbMaxTradeBars,
  vcetVidyaShortPeriod,
  vcetVidyaLongPeriod,
  vcetVidyaAlpha,
  vcetCePeriod,
  vcetCeMult,
  vcetMaxTradeBars,
  vemrZPeriod,
  vemrZMaType,
  vemrZNbdev,
  vemrZDevtype,
  vemrZEntry,
  vemrZExit,
  vemrBbPeriod,
  vemrBbMult,
  vemrAtrPeriod,
  vemrAtrStopMult,
  vemrConfirmBars,
  vemrMaxTradeBars,
  vccMapFastToSqueezePeriod,
  vccMapFastToMomentumPeriod,
  vccMapSlowToTrendPeriod,
  vccRequireMomentumAccel,
  rocbMapFastToChannelPeriod,
  rocbMapSlowToAtrPeriod,
  vcetMapFastToVidyaShortPeriod,
  vcetMapSlowToVidyaLongPeriod,
  vcetMapSlowToCePeriod,
  vcetCeUseClose,
  vcetRequireVidyaSlope,
  vemrMapFastToZPeriod,
  vemrMapFastToBbPeriod,
  vemrMapSlowToAtrPeriod,
  vemrRequireBandTouch,
  longOnly,
  tradeOnNext,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(strategyStore);
const { strategyKind } = storeToRefs(optimizeStore);
const { controlsTab } = storeToRefs(workflowStore);

const { mountReady } = useShellMount("#strategyVueShellMount", {
  paneSelector: '[data-vue-panel="strategy"]',
  paneDatasetKey: "vueStrategy",
});

const costPresetDraft = ref("custom");
const feeDraft = ref("0");
const spreadDraft = ref("0");
const slippageDraft = ref("0");
const epsDraft = ref("0");
const equityDraft = ref("10000");
const slDraft = ref("");
const tpDraft = ref("");
const cooldownDraft = ref("0");
const persistenceDraft = ref("1");
const minTradesDraft = ref("");
const maxDdDraft = ref("");
const minSharpeDraft = ref("");
const minSortinoDraft = ref("");
const minCalmarDraft = ref("");
const maxVolatilityDraft = ref("");
const minExposureDraft = ref("");
const maxExposureDraft = ref("");
const maxAbsNetExposureDraft = ref("");
const minOmegaDraft = ref("");
const maxUlcerDraft = ref("");
const maxDownsideDeviationDraft = ref("");
const exitModeDraft = ref("cross");
const kaufmanPeriodDraft = ref("22");
const kaufmanMultDraft = ref("2.0");
const kaufmanTypeDraft = ref("sma");
const asrSqueezeBbMultDraft = ref("2.0");
const asrSqueezeKcMultDraft = ref("1.5");
const asrAngleDegMinDraft = ref("10.0");
const asrAngleDegMrDraft = ref("6.0");
const asrErMinDraft = ref("0.30");
const asrErMrDraft = ref("0.25");
const asrZMaTypeDraft = ref("sma");
const asrZNbdevDraft = ref("1.0");
const asrZDevtypeDraft = ref("0");
const asrZEntryDraft = ref("2.0");
const asrZExitDraft = ref("0.5");
const asrConfirmBarsDraft = ref("0");
const asrMomFadeBarsDraft = ref("2");
const asrMaxTradeBarsDraft = ref("80");
const asrMaxMrTradeBarsDraft = ref("30");
const vccSqueezePeriodDraft = ref("20");
const vccMomentumPeriodDraft = ref("20");
const vccMomentumBbPeriodDraft = ref("20");
const vccTrendPeriodDraft = ref("6");
const vccSqueezeBbMultDraft = ref("2.0");
const vccSqueezeKcMultDraft = ref("1.5");
const vccConfirmBarsDraft = ref("0");
const vccMaxTradeBarsDraft = ref("120");
const pitEfiPeriodDraft = ref("13");
const pitVoscShortPeriodDraft = ref("5");
const pitVoscLongPeriodDraft = ref("20");
const pitCfoPeriodDraft = ref("9");
const pitCfoScalarDraft = ref("100.0");
const pitConfirmBarsDraft = ref("2");
const vfctObvSmoothPeriodDraft = ref("21");
const vfctMfiPeriodDraft = ref("14");
const vfctMfiLongMinDraft = ref("55");
const vfctMfiShortMaxDraft = ref("45");
const vfctConfirmBarsDraft = ref("1");
const rocbChannelPeriodDraft = ref("20");
const rocbAtrPeriodDraft = ref("14");
const rocbAtrStopMultDraft = ref("2.0");
const rocbMaxTradeBarsDraft = ref("80");
const vcetVidyaShortPeriodDraft = ref("6");
const vcetVidyaLongPeriodDraft = ref("24");
const vcetVidyaAlphaDraft = ref("0.2");
const vcetCePeriodDraft = ref("22");
const vcetCeMultDraft = ref("3.0");
const vcetMaxTradeBarsDraft = ref("120");
const vemrZPeriodDraft = ref("20");
const vemrZMaTypeDraft = ref("sma");
const vemrZNbdevDraft = ref("1.0");
const vemrZDevtypeDraft = ref("0");
const vemrZEntryDraft = ref("2.0");
const vemrZExitDraft = ref("0.5");
const vemrBbPeriodDraft = ref("20");
const vemrBbMultDraft = ref("2.0");
const vemrAtrPeriodDraft = ref("14");
const vemrAtrStopMultDraft = ref("2.0");
const vemrConfirmBarsDraft = ref("0");
const vemrMaxTradeBarsDraft = ref("80");

const visible = computed(
  () => String(controlsTab.value || "").trim() === "strategy",
);
const strategyKindNormalized = computed(
  () => String(strategyKind.value || "DoubleMa").trim() || "DoubleMa",
);
const isDoubleLikeKind = computed(() => {
  const kind = strategyKindNormalized.value;
  return (
    kind === "DoubleMa" ||
    kind === "Pit" ||
    kind === "Set" ||
    kind === "Mts" ||
    kind === "Orc" ||
    kind === "Marrt" ||
    kind === "Vfct" ||
    kind === "Asr" ||
    kind === "Vemr" ||
    kind === "Rocb" ||
    kind === "Vcet" ||
    kind === "Kqsr" ||
    kind === "Carm" ||
    kind === "Kvdf" ||
    kind === "Mkcs" ||
    kind === "Cdmc" ||
    kind === "CompositeTrend" ||
    kind === "RegimeHybrid" ||
    kind === "BreakoutMomentum"
  );
});
const showDmConstraints = computed(() => isDoubleLikeKind.value);
const showAsrControls = computed(() => strategyKindNormalized.value === "Asr");
const showVccControls = computed(() => strategyKindNormalized.value === "Vcc");
const showPitControls = computed(() => strategyKindNormalized.value === "Pit");
const showVfctControls = computed(
  () => strategyKindNormalized.value === "Vfct",
);
const showRocbControls = computed(
  () => strategyKindNormalized.value === "Rocb",
);
const showVcetControls = computed(
  () => strategyKindNormalized.value === "Vcet",
);
const showVemrControls = computed(
  () => strategyKindNormalized.value === "Vemr",
);
const showKaufmanFields = computed(() => {
  const mode = String(exitMode.value || "cross").trim() || "cross";
  return (
    showAsrControls.value || (isDoubleLikeKind.value && mode === "kaufman_stop")
  );
});
const executionRisk = computed(() =>
  describeExecutionRisk({
    longOnly: longOnly.value,
    tradeOnNext: tradeOnNext.value,
  }),
);

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value));

function resolvedOptions(
  options: StrategyOption[],
  fallbackValue: string,
  fallbackLabel: string,
): StrategyOption[] {
  if (options.length > 0) return options;
  const value = String(fallbackValue || "").trim();
  if (!value) return [];
  return [
    {
      value,
      label: String(fallbackLabel || value).trim() || value,
      disabled: false,
    },
  ];
}

const costPresetOptionItems = computed(() =>
  resolvedOptions(
    costPresetOptions.value,
    costPreset.value,
    costPreset.value || "custom",
  ),
);
const exitModeOptionItems = computed(() =>
  resolvedOptions(
    exitModeOptions.value,
    exitMode.value,
    exitMode.value || "cross",
  ),
);
function optionLabel(
  options: StrategyOption[],
  nextValue: string,
  fallback: string,
): string {
  const match = options.find(
    (option) => String(option.value || "").trim() === String(nextValue || "").trim(),
  );
  return String(match?.label || fallback || nextValue || "").trim() || fallback;
}
const selectedCostPresetLabel = computed(() =>
  optionLabel(costPresetOptionItems.value, costPresetDraft.value, "Custom"),
);
const selectedExitModeLabel = computed(() =>
  optionLabel(exitModeOptionItems.value, exitModeDraft.value, "Cross"),
);
function hasDraftValue(nextValue: string): boolean {
  return String(nextValue || "").trim().length > 0;
}
const coreConstraintActiveCount = computed(() =>
  [
    minTradesDraft.value,
    maxDdDraft.value,
    minSharpeDraft.value,
    minSortinoDraft.value,
  ].filter(hasDraftValue).length,
);
const additionalConstraintActiveCount = computed(() =>
  [
    minCalmarDraft.value,
    maxVolatilityDraft.value,
    minExposureDraft.value,
    maxExposureDraft.value,
    maxAbsNetExposureDraft.value,
    minOmegaDraft.value,
    maxUlcerDraft.value,
    maxDownsideDeviationDraft.value,
  ].filter(hasDraftValue).length,
);
const hasAdditionalConstraints = computed(
  () => additionalConstraintActiveCount.value > 0,
);
const additionalConstraintSummary = computed(() =>
  hasAdditionalConstraints.value
    ? `${additionalConstraintActiveCount.value} active`
    : "Optional long-tail guards",
);
const strategySummaryChips = computed(() => {
  const chips = [
    `Strategy ${strategyKindNormalized.value}`,
    `Costs ${selectedCostPresetLabel.value}`,
    `Exit ${selectedExitModeLabel.value}`,
    longOnly.value ? "Long only" : "Long + short",
    tradeOnNext.value ? "Trade on next bar" : "Trade on signal bar",
  ];
  if (showDmConstraints.value) {
    const totalConstraints =
      coreConstraintActiveCount.value + additionalConstraintActiveCount.value;
    if (totalConstraints > 0) {
      chips.push(`Constraints ${totalConstraints} active`);
    }
  }
  return chips;
});

watch(
  () =>
    [
      String(costPreset.value || "custom"),
      String(feeBps.value || "0"),
      String(spreadBps.value || "0"),
      String(slippageBps.value || "0"),
      String(epsRel.value || "0"),
      String(startingEquity.value || "10000"),
      String(stopLossPct.value || ""),
      String(takeProfitPct.value || ""),
      String(cooldownBars.value || "0"),
      String(signalPersistenceBars.value || "1"),
      String(dmConstraintMinTrades.value || ""),
      String(dmConstraintMaxDd.value || ""),
      String(dmConstraintMinSharpe.value || ""),
      String(dmConstraintMinSortino.value || ""),
      String(dmConstraintMinCalmar.value || ""),
      String(dmConstraintMaxVolatility.value || ""),
      String(dmConstraintMinExposure.value || ""),
      String(dmConstraintMaxExposure.value || ""),
      String(dmConstraintMaxAbsNetExposure.value || ""),
      String(dmConstraintMinOmega.value || ""),
      String(dmConstraintMaxUlcer.value || ""),
      String(dmConstraintMaxDownsideDeviation.value || ""),
      String(kaufmanStopPeriod.value || "22"),
      String(kaufmanStopMult.value || "2.0"),
      String(kaufmanStopMaType.value || "sma"),
      String(asrSqueezeBbMult.value || "2.0"),
      String(asrSqueezeKcMult.value || "1.5"),
      String(asrAngleDegMin.value || "10.0"),
      String(asrAngleDegMr.value || "6.0"),
      String(asrErMin.value || "0.30"),
      String(asrErMr.value || "0.25"),
      String(asrZMaType.value || "sma"),
      String(asrZNbdev.value || "1.0"),
      String(asrZDevtype.value || "0"),
      String(asrZEntry.value || "2.0"),
      String(asrZExit.value || "0.5"),
      String(asrConfirmBars.value || "0"),
      String(asrMomFadeBars.value || "2"),
      String(asrMaxTradeBars.value || "80"),
      String(asrMaxMrTradeBars.value || "30"),
      String(vccSqueezePeriod.value || "20"),
      String(vccMomentumPeriod.value || "20"),
      String(vccMomentumBbPeriod.value || "20"),
      String(vccTrendPeriod.value || "6"),
      String(vccSqueezeBbMult.value || "2.0"),
      String(vccSqueezeKcMult.value || "1.5"),
      String(vccConfirmBars.value || "0"),
      String(vccMaxTradeBars.value || "120"),
      String(pitEfiPeriod.value || "13"),
      String(pitVoscShortPeriod.value || "5"),
      String(pitVoscLongPeriod.value || "20"),
      String(pitCfoPeriod.value || "9"),
      String(pitCfoScalar.value || "100.0"),
      String(pitConfirmBars.value || "2"),
      String(vfctObvSmoothPeriod.value || "21"),
      String(vfctMfiPeriod.value || "14"),
      String(vfctMfiLongMin.value || "55"),
      String(vfctMfiShortMax.value || "45"),
      String(vfctConfirmBars.value || "1"),
      String(rocbChannelPeriod.value || "20"),
      String(rocbAtrPeriod.value || "14"),
      String(rocbAtrStopMult.value || "2.0"),
      String(rocbMaxTradeBars.value || "80"),
      String(vcetVidyaShortPeriod.value || "6"),
      String(vcetVidyaLongPeriod.value || "24"),
      String(vcetVidyaAlpha.value || "0.2"),
      String(vcetCePeriod.value || "22"),
      String(vcetCeMult.value || "3.0"),
      String(vcetMaxTradeBars.value || "120"),
      String(vemrZPeriod.value || "20"),
      String(vemrZMaType.value || "sma"),
      String(vemrZNbdev.value || "1.0"),
      String(vemrZDevtype.value || "0"),
      String(vemrZEntry.value || "2.0"),
      String(vemrZExit.value || "0.5"),
      String(vemrBbPeriod.value || "20"),
      String(vemrBbMult.value || "2.0"),
      String(vemrAtrPeriod.value || "14"),
      String(vemrAtrStopMult.value || "2.0"),
      String(vemrConfirmBars.value || "0"),
      String(vemrMaxTradeBars.value || "80"),
    ] as const,
  ([
    nextCostPreset,
    nextFeeBps,
    nextSpreadBps,
    nextSlippageBps,
    nextEpsRel,
    nextStartingEquity,
    nextStopLossPct,
    nextTakeProfitPct,
    nextCooldownBars,
    nextSignalPersistenceBars,
    nextDmConstraintMinTrades,
    nextDmConstraintMaxDd,
    nextDmConstraintMinSharpe,
    nextDmConstraintMinSortino,
    nextDmConstraintMinCalmar,
    nextDmConstraintMaxVolatility,
    nextDmConstraintMinExposure,
    nextDmConstraintMaxExposure,
    nextDmConstraintMaxAbsNetExposure,
    nextDmConstraintMinOmega,
    nextDmConstraintMaxUlcer,
    nextDmConstraintMaxDownsideDeviation,
    nextKaufmanStopPeriod,
    nextKaufmanStopMult,
    nextKaufmanStopMaType,
    nextAsrSqueezeBbMult,
    nextAsrSqueezeKcMult,
    nextAsrAngleDegMin,
    nextAsrAngleDegMr,
    nextAsrErMin,
    nextAsrErMr,
    nextAsrZMaType,
    nextAsrZNbdev,
    nextAsrZDevtype,
    nextAsrZEntry,
    nextAsrZExit,
    nextAsrConfirmBars,
    nextAsrMomFadeBars,
    nextAsrMaxTradeBars,
    nextAsrMaxMrTradeBars,
    nextVccSqueezePeriod,
    nextVccMomentumPeriod,
    nextVccMomentumBbPeriod,
    nextVccTrendPeriod,
    nextVccSqueezeBbMult,
    nextVccSqueezeKcMult,
    nextVccConfirmBars,
    nextVccMaxTradeBars,
    nextPitEfiPeriod,
    nextPitVoscShortPeriod,
    nextPitVoscLongPeriod,
    nextPitCfoPeriod,
    nextPitCfoScalar,
    nextPitConfirmBars,
    nextVfctObvSmoothPeriod,
    nextVfctMfiPeriod,
    nextVfctMfiLongMin,
    nextVfctMfiShortMax,
    nextVfctConfirmBars,
    nextRocbChannelPeriod,
    nextRocbAtrPeriod,
    nextRocbAtrStopMult,
    nextRocbMaxTradeBars,
    nextVcetVidyaShortPeriod,
    nextVcetVidyaLongPeriod,
    nextVcetVidyaAlpha,
    nextVcetCePeriod,
    nextVcetCeMult,
    nextVcetMaxTradeBars,
    nextVemrZPeriod,
    nextVemrZMaType,
    nextVemrZNbdev,
    nextVemrZDevtype,
    nextVemrZEntry,
    nextVemrZExit,
    nextVemrBbPeriod,
    nextVemrBbMult,
    nextVemrAtrPeriod,
    nextVemrAtrStopMult,
    nextVemrConfirmBars,
    nextVemrMaxTradeBars,
  ]) => {
    if (costPresetDraft.value !== nextCostPreset)
      costPresetDraft.value = nextCostPreset;
    if (feeDraft.value !== nextFeeBps) feeDraft.value = nextFeeBps;
    if (spreadDraft.value !== nextSpreadBps) spreadDraft.value = nextSpreadBps;
    if (slippageDraft.value !== nextSlippageBps)
      slippageDraft.value = nextSlippageBps;
    if (epsDraft.value !== nextEpsRel) epsDraft.value = nextEpsRel;
    if (equityDraft.value !== nextStartingEquity)
      equityDraft.value = nextStartingEquity;
    if (slDraft.value !== nextStopLossPct) slDraft.value = nextStopLossPct;
    if (tpDraft.value !== nextTakeProfitPct) tpDraft.value = nextTakeProfitPct;
    if (cooldownDraft.value !== nextCooldownBars)
      cooldownDraft.value = nextCooldownBars;
    if (persistenceDraft.value !== nextSignalPersistenceBars)
      persistenceDraft.value = nextSignalPersistenceBars;
    if (minTradesDraft.value !== nextDmConstraintMinTrades)
      minTradesDraft.value = nextDmConstraintMinTrades;
    if (maxDdDraft.value !== nextDmConstraintMaxDd)
      maxDdDraft.value = nextDmConstraintMaxDd;
    if (minSharpeDraft.value !== nextDmConstraintMinSharpe)
      minSharpeDraft.value = nextDmConstraintMinSharpe;
    if (minSortinoDraft.value !== nextDmConstraintMinSortino)
      minSortinoDraft.value = nextDmConstraintMinSortino;
    if (minCalmarDraft.value !== nextDmConstraintMinCalmar)
      minCalmarDraft.value = nextDmConstraintMinCalmar;
    if (maxVolatilityDraft.value !== nextDmConstraintMaxVolatility)
      maxVolatilityDraft.value = nextDmConstraintMaxVolatility;
    if (minExposureDraft.value !== nextDmConstraintMinExposure)
      minExposureDraft.value = nextDmConstraintMinExposure;
    if (maxExposureDraft.value !== nextDmConstraintMaxExposure)
      maxExposureDraft.value = nextDmConstraintMaxExposure;
    if (maxAbsNetExposureDraft.value !== nextDmConstraintMaxAbsNetExposure)
      maxAbsNetExposureDraft.value = nextDmConstraintMaxAbsNetExposure;
    if (minOmegaDraft.value !== nextDmConstraintMinOmega)
      minOmegaDraft.value = nextDmConstraintMinOmega;
    if (maxUlcerDraft.value !== nextDmConstraintMaxUlcer)
      maxUlcerDraft.value = nextDmConstraintMaxUlcer;
    if (
      maxDownsideDeviationDraft.value !== nextDmConstraintMaxDownsideDeviation
    )
      maxDownsideDeviationDraft.value = nextDmConstraintMaxDownsideDeviation;
    if (kaufmanPeriodDraft.value !== nextKaufmanStopPeriod)
      kaufmanPeriodDraft.value = nextKaufmanStopPeriod;
    if (kaufmanMultDraft.value !== nextKaufmanStopMult)
      kaufmanMultDraft.value = nextKaufmanStopMult;
    if (kaufmanTypeDraft.value !== nextKaufmanStopMaType)
      kaufmanTypeDraft.value = nextKaufmanStopMaType;
    if (asrSqueezeBbMultDraft.value !== nextAsrSqueezeBbMult)
      asrSqueezeBbMultDraft.value = nextAsrSqueezeBbMult;
    if (asrSqueezeKcMultDraft.value !== nextAsrSqueezeKcMult)
      asrSqueezeKcMultDraft.value = nextAsrSqueezeKcMult;
    if (asrAngleDegMinDraft.value !== nextAsrAngleDegMin)
      asrAngleDegMinDraft.value = nextAsrAngleDegMin;
    if (asrAngleDegMrDraft.value !== nextAsrAngleDegMr)
      asrAngleDegMrDraft.value = nextAsrAngleDegMr;
    if (asrErMinDraft.value !== nextAsrErMin)
      asrErMinDraft.value = nextAsrErMin;
    if (asrErMrDraft.value !== nextAsrErMr) asrErMrDraft.value = nextAsrErMr;
    if (asrZMaTypeDraft.value !== nextAsrZMaType)
      asrZMaTypeDraft.value = nextAsrZMaType;
    if (asrZNbdevDraft.value !== nextAsrZNbdev)
      asrZNbdevDraft.value = nextAsrZNbdev;
    if (asrZDevtypeDraft.value !== nextAsrZDevtype)
      asrZDevtypeDraft.value = nextAsrZDevtype;
    if (asrZEntryDraft.value !== nextAsrZEntry)
      asrZEntryDraft.value = nextAsrZEntry;
    if (asrZExitDraft.value !== nextAsrZExit)
      asrZExitDraft.value = nextAsrZExit;
    if (asrConfirmBarsDraft.value !== nextAsrConfirmBars)
      asrConfirmBarsDraft.value = nextAsrConfirmBars;
    if (asrMomFadeBarsDraft.value !== nextAsrMomFadeBars)
      asrMomFadeBarsDraft.value = nextAsrMomFadeBars;
    if (asrMaxTradeBarsDraft.value !== nextAsrMaxTradeBars)
      asrMaxTradeBarsDraft.value = nextAsrMaxTradeBars;
    if (asrMaxMrTradeBarsDraft.value !== nextAsrMaxMrTradeBars)
      asrMaxMrTradeBarsDraft.value = nextAsrMaxMrTradeBars;
    if (vccSqueezePeriodDraft.value !== nextVccSqueezePeriod)
      vccSqueezePeriodDraft.value = nextVccSqueezePeriod;
    if (vccMomentumPeriodDraft.value !== nextVccMomentumPeriod)
      vccMomentumPeriodDraft.value = nextVccMomentumPeriod;
    if (vccMomentumBbPeriodDraft.value !== nextVccMomentumBbPeriod)
      vccMomentumBbPeriodDraft.value = nextVccMomentumBbPeriod;
    if (vccTrendPeriodDraft.value !== nextVccTrendPeriod)
      vccTrendPeriodDraft.value = nextVccTrendPeriod;
    if (vccSqueezeBbMultDraft.value !== nextVccSqueezeBbMult)
      vccSqueezeBbMultDraft.value = nextVccSqueezeBbMult;
    if (vccSqueezeKcMultDraft.value !== nextVccSqueezeKcMult)
      vccSqueezeKcMultDraft.value = nextVccSqueezeKcMult;
    if (vccConfirmBarsDraft.value !== nextVccConfirmBars)
      vccConfirmBarsDraft.value = nextVccConfirmBars;
    if (vccMaxTradeBarsDraft.value !== nextVccMaxTradeBars)
      vccMaxTradeBarsDraft.value = nextVccMaxTradeBars;
    if (pitEfiPeriodDraft.value !== nextPitEfiPeriod)
      pitEfiPeriodDraft.value = nextPitEfiPeriod;
    if (pitVoscShortPeriodDraft.value !== nextPitVoscShortPeriod)
      pitVoscShortPeriodDraft.value = nextPitVoscShortPeriod;
    if (pitVoscLongPeriodDraft.value !== nextPitVoscLongPeriod)
      pitVoscLongPeriodDraft.value = nextPitVoscLongPeriod;
    if (pitCfoPeriodDraft.value !== nextPitCfoPeriod)
      pitCfoPeriodDraft.value = nextPitCfoPeriod;
    if (pitCfoScalarDraft.value !== nextPitCfoScalar)
      pitCfoScalarDraft.value = nextPitCfoScalar;
    if (pitConfirmBarsDraft.value !== nextPitConfirmBars)
      pitConfirmBarsDraft.value = nextPitConfirmBars;
    if (vfctObvSmoothPeriodDraft.value !== nextVfctObvSmoothPeriod)
      vfctObvSmoothPeriodDraft.value = nextVfctObvSmoothPeriod;
    if (vfctMfiPeriodDraft.value !== nextVfctMfiPeriod)
      vfctMfiPeriodDraft.value = nextVfctMfiPeriod;
    if (vfctMfiLongMinDraft.value !== nextVfctMfiLongMin)
      vfctMfiLongMinDraft.value = nextVfctMfiLongMin;
    if (vfctMfiShortMaxDraft.value !== nextVfctMfiShortMax)
      vfctMfiShortMaxDraft.value = nextVfctMfiShortMax;
    if (vfctConfirmBarsDraft.value !== nextVfctConfirmBars)
      vfctConfirmBarsDraft.value = nextVfctConfirmBars;
    if (rocbChannelPeriodDraft.value !== nextRocbChannelPeriod)
      rocbChannelPeriodDraft.value = nextRocbChannelPeriod;
    if (rocbAtrPeriodDraft.value !== nextRocbAtrPeriod)
      rocbAtrPeriodDraft.value = nextRocbAtrPeriod;
    if (rocbAtrStopMultDraft.value !== nextRocbAtrStopMult)
      rocbAtrStopMultDraft.value = nextRocbAtrStopMult;
    if (rocbMaxTradeBarsDraft.value !== nextRocbMaxTradeBars)
      rocbMaxTradeBarsDraft.value = nextRocbMaxTradeBars;
    if (vcetVidyaShortPeriodDraft.value !== nextVcetVidyaShortPeriod)
      vcetVidyaShortPeriodDraft.value = nextVcetVidyaShortPeriod;
    if (vcetVidyaLongPeriodDraft.value !== nextVcetVidyaLongPeriod)
      vcetVidyaLongPeriodDraft.value = nextVcetVidyaLongPeriod;
    if (vcetVidyaAlphaDraft.value !== nextVcetVidyaAlpha)
      vcetVidyaAlphaDraft.value = nextVcetVidyaAlpha;
    if (vcetCePeriodDraft.value !== nextVcetCePeriod)
      vcetCePeriodDraft.value = nextVcetCePeriod;
    if (vcetCeMultDraft.value !== nextVcetCeMult)
      vcetCeMultDraft.value = nextVcetCeMult;
    if (vcetMaxTradeBarsDraft.value !== nextVcetMaxTradeBars)
      vcetMaxTradeBarsDraft.value = nextVcetMaxTradeBars;
    if (vemrZPeriodDraft.value !== nextVemrZPeriod)
      vemrZPeriodDraft.value = nextVemrZPeriod;
    if (vemrZMaTypeDraft.value !== nextVemrZMaType)
      vemrZMaTypeDraft.value = nextVemrZMaType;
    if (vemrZNbdevDraft.value !== nextVemrZNbdev)
      vemrZNbdevDraft.value = nextVemrZNbdev;
    if (vemrZDevtypeDraft.value !== nextVemrZDevtype)
      vemrZDevtypeDraft.value = nextVemrZDevtype;
    if (vemrZEntryDraft.value !== nextVemrZEntry)
      vemrZEntryDraft.value = nextVemrZEntry;
    if (vemrZExitDraft.value !== nextVemrZExit)
      vemrZExitDraft.value = nextVemrZExit;
    if (vemrBbPeriodDraft.value !== nextVemrBbPeriod)
      vemrBbPeriodDraft.value = nextVemrBbPeriod;
    if (vemrBbMultDraft.value !== nextVemrBbMult)
      vemrBbMultDraft.value = nextVemrBbMult;
    if (vemrAtrPeriodDraft.value !== nextVemrAtrPeriod)
      vemrAtrPeriodDraft.value = nextVemrAtrPeriod;
    if (vemrAtrStopMultDraft.value !== nextVemrAtrStopMult)
      vemrAtrStopMultDraft.value = nextVemrAtrStopMult;
    if (vemrConfirmBarsDraft.value !== nextVemrConfirmBars)
      vemrConfirmBarsDraft.value = nextVemrConfirmBars;
    if (vemrMaxTradeBarsDraft.value !== nextVemrMaxTradeBars)
      vemrMaxTradeBarsDraft.value = nextVemrMaxTradeBars;
  },
  { immediate: true },
);
watch(
  exitMode,
  (v) => {
    const next = String(v || "cross");
    if (exitModeDraft.value !== next) exitModeDraft.value = next;
  },
  { immediate: true },
);

function setInputValue(id: string, nextValue: string): void {
  strategyStore.setInputField(id, nextValue);
}

const { queueInputValue, commitInputValue, flushQueuedInputValuesNow } =
  useQueuedInputWriter(setInputValue);

function setSelectValue(id: string, nextValue: string): void {
  strategyStore.setSelectField(id, nextValue);
}

function setSaferExecutionDefaults(): void {
  strategyStore.setCheckboxField("tradeOnNext", true);
  strategyStore.setCheckboxField("longOnly", true);
}

onBeforeUnmount(() => {
  flushQueuedInputValuesNow();
});
</script>

<template>
  <Teleport v-if="mountReady" to="#strategyVueShellMount">
    <section
      class="strategyPanelVueRoot strategyVueShellMountHost"
      v-show="visible"
    >
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Authoring</div>
          <h2 class="panelTitle">Strategy Controls</h2>
          <div class="panelSubtitle">
            Set execution costs, shared risk guardrails, and the active strategy module before widening the optimization search.
          </div>
        </div>
        <div class="meta mono">{{ stamp }}</div>
      </header>
      <div class="panelSummaryChips" aria-label="Strategy summary">
        <span
          v-for="chip in strategySummaryChips"
          :key="`strategy_summary_${chip}`"
          class="panelSummaryChip mono"
        >
          {{ chip }}
        </span>
      </div>

      <section v-if="executionRisk" class="riskNotice">
        <div class="riskTitle">{{ executionRisk.title }}</div>
        <div class="riskBody">{{ executionRisk.body }}</div>
        <div class="actions">
          <button
            type="button"
            class="primary"
            @click="setSaferExecutionDefaults()"
          >
            Use safer execution defaults
          </button>
        </div>
      </section>

      <section class="pane panelProminentPane">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Setup</div>
            <h3 class="panelPaneTitle">Execution cost model</h3>
            <div class="panelSectionLead">
              Keep cost inputs aligned with the broker, slippage, and validation assumptions you expect to carry through later execution work.
            </div>
          </div>
        </div>
        <label for="vueCostPreset">Cost preset</label>
        <select
          id="vueCostPreset"
          :value="costPresetDraft"
          @change="
            costPresetDraft = String(
              ($event.target as HTMLSelectElement).value || 'custom',
            );
            setSelectValue('costPreset', costPresetDraft);
          "
        >
          <option
            v-for="option in costPresetOptionItems"
            :key="`cost_${option.value}_${option.label}`"
            :value="option.value"
            :disabled="option.disabled"
          >
            {{ option.label }}
          </option>
        </select>

        <div class="rowThree">
          <div>
            <label for="vueFeeBps">Fee (bps/side)</label>
            <input
              id="vueFeeBps"
              type="number"
              min="0"
              step="0.01"
              :value="feeDraft"
              @input="
                feeDraft = String(
                  ($event.target as HTMLInputElement).value || '0',
                );
                queueInputValue('feeBps', feeDraft);
              "
              @change="commitInputValue('feeBps', feeDraft)"
            />
          </div>
          <div>
            <label for="vueSpreadBps">Spread (bps full)</label>
            <input
              id="vueSpreadBps"
              type="number"
              min="0"
              step="0.01"
              :value="spreadDraft"
              @input="
                spreadDraft = String(
                  ($event.target as HTMLInputElement).value || '0',
                );
                queueInputValue('spreadBps', spreadDraft);
              "
              @change="commitInputValue('spreadBps', spreadDraft)"
            />
          </div>
          <div>
            <label for="vueSlippageBps">Slippage (bps/side)</label>
            <input
              id="vueSlippageBps"
              type="number"
              min="0"
              step="0.01"
              :value="slippageDraft"
              @input="
                slippageDraft = String(
                  ($event.target as HTMLInputElement).value || '0',
                );
                queueInputValue('slippageBps', slippageDraft);
              "
              @change="commitInputValue('slippageBps', slippageDraft)"
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueCommission">Total cost (fraction/side)</label>
            <input
              id="vueCommission"
              type="text"
              :value="commission"
              readonly
            />
          </div>
          <div>
            <label for="vueEpsRel">Epsilon band (eps_rel)</label>
            <input
              id="vueEpsRel"
              type="number"
              min="0"
              step="0.0001"
              :value="epsDraft"
              @input="
                epsDraft = String(
                  ($event.target as HTMLInputElement).value || '0',
                );
                queueInputValue('epsRel', epsDraft);
              "
              @change="commitInputValue('epsRel', epsDraft)"
            />
          </div>
          <div class="muted inlineMeta mono">{{ costHint }}</div>
        </div>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Guardrails</div>
            <h3 class="panelPaneTitle">Risk and sizing</h3>
            <div class="panelSectionLead">
              Set portfolio-level equity, stop management, and signal timing before moving into strategy-specific parameter families.
            </div>
          </div>
        </div>
        <div>
          <label for="vueStartingEquity">Starting equity</label>
          <input
            id="vueStartingEquity"
            type="number"
            min="0.01"
            step="1"
            :value="equityDraft"
            @input="
              equityDraft = String(
                ($event.target as HTMLInputElement).value || '10000',
              );
              queueInputValue('startingEquity', equityDraft);
            "
            @change="commitInputValue('startingEquity', equityDraft)"
          />
        </div>

        <div class="rowThree">
          <div>
            <label for="vueStopLossPct">Stop-loss (optional)</label>
            <input
              id="vueStopLossPct"
              type="number"
              min="0"
              step="0.0001"
              :value="slDraft"
              @input="
                slDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueInputValue('stopLossPct', slDraft);
              "
              @change="commitInputValue('stopLossPct', slDraft)"
            />
          </div>
          <div>
            <label for="vueTakeProfitPct">Take-profit (optional)</label>
            <input
              id="vueTakeProfitPct"
              type="number"
              min="0"
              step="0.0001"
              :value="tpDraft"
              @input="
                tpDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueInputValue('takeProfitPct', tpDraft);
              "
              @change="commitInputValue('takeProfitPct', tpDraft)"
            />
          </div>
        </div>

        <div class="rowThree">
          <div>
            <label for="vueCooldownBars">Cooldown bars</label>
            <input
              id="vueCooldownBars"
              type="number"
              min="0"
              step="1"
              :value="cooldownDraft"
              @input="
                cooldownDraft = String(
                  ($event.target as HTMLInputElement).value || '0',
                );
                queueInputValue('cooldownBars', cooldownDraft);
              "
              @change="commitInputValue('cooldownBars', cooldownDraft)"
            />
          </div>
          <div>
            <label for="vueSignalPersistenceBars"
              >Signal persistence bars</label
            >
            <input
              id="vueSignalPersistenceBars"
              type="number"
              min="1"
              step="1"
              :value="persistenceDraft"
              @input="
                persistenceDraft = String(
                  ($event.target as HTMLInputElement).value || '1',
                );
                queueInputValue('signalPersistenceBars', persistenceDraft);
              "
              @change="
                commitInputValue('signalPersistenceBars', persistenceDraft)
              "
            />
          </div>
        </div>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Risk policy</div>
            <h3 class="panelPaneTitle">Constraints and exits</h3>
            <div class="panelSectionLead">
              Use shortlist constraints to reject brittle runs early, then choose the exit behavior that should persist into validation and live review.
            </div>
          </div>
        </div>
        <div class="rowTwo" v-if="showDmConstraints">
          <div>
            <label for="vueDmMinTrades">Min trades (optional)</label>
            <input
              id="vueDmMinTrades"
              type="number"
              min="0"
              step="1"
              :value="minTradesDraft"
              @input="
                minTradesDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueInputValue('dmConstraintMinTrades', minTradesDraft);
              "
              @change="
                commitInputValue('dmConstraintMinTrades', minTradesDraft)
              "
            />
          </div>
          <div>
            <label for="vueDmMaxDd">Max drawdown (optional)</label>
            <input
              id="vueDmMaxDd"
              type="number"
              min="0"
              step="0.0001"
              :value="maxDdDraft"
              @input="
                maxDdDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueInputValue('dmConstraintMaxDd', maxDdDraft);
              "
              @change="commitInputValue('dmConstraintMaxDd', maxDdDraft)"
            />
          </div>
        </div>

        <div class="rowTwo" v-if="showDmConstraints">
          <div>
            <label for="vueDmMinSharpe">Min Sharpe (optional)</label>
            <input
              id="vueDmMinSharpe"
              type="number"
              step="0.01"
              :value="minSharpeDraft"
              @input="
                minSharpeDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueInputValue('dmConstraintMinSharpe', minSharpeDraft);
              "
              @change="
                commitInputValue('dmConstraintMinSharpe', minSharpeDraft)
              "
            />
          </div>
          <div>
            <label for="vueDmMinSortino">Min Sortino (optional)</label>
            <input
              id="vueDmMinSortino"
              type="number"
              step="0.01"
              :value="minSortinoDraft"
              @input="
                minSortinoDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueInputValue('dmConstraintMinSortino', minSortinoDraft);
              "
              @change="
                commitInputValue('dmConstraintMinSortino', minSortinoDraft)
              "
            />
          </div>
        </div>

        <details
          v-if="showDmConstraints"
          class="panelDetails strategyConstraintDetails"
          :open="hasAdditionalConstraints"
        >
          <summary>
            <span>More constraints</span>
            <span class="mono">{{ additionalConstraintSummary }}</span>
          </summary>
          <div class="panelDetailsBody">
            <div class="panelMuted">
              Keep the most frequently used shortlist gates visible and place lower-frequency exposure, downside, and robustness guards here.
            </div>
            <div class="rowThree">
              <div>
                <label for="vueDmMinCalmar">Min Calmar (optional)</label>
                <input
                  id="vueDmMinCalmar"
                  type="number"
                  step="0.01"
                  :value="minCalmarDraft"
                  @input="
                    minCalmarDraft = String(
                      ($event.target as HTMLInputElement).value || '',
                    );
                    queueInputValue('dmConstraintMinCalmar', minCalmarDraft);
                  "
                  @change="
                    commitInputValue('dmConstraintMinCalmar', minCalmarDraft)
                  "
                />
              </div>
              <div>
                <label for="vueDmMaxVolatility">Max volatility (optional)</label>
                <input
                  id="vueDmMaxVolatility"
                  type="number"
                  min="0"
                  step="0.0001"
                  :value="maxVolatilityDraft"
                  @input="
                    maxVolatilityDraft = String(
                      ($event.target as HTMLInputElement).value || '',
                    );
                    queueInputValue(
                      'dmConstraintMaxVolatility',
                      maxVolatilityDraft,
                    );
                  "
                  @change="
                    commitInputValue(
                      'dmConstraintMaxVolatility',
                      maxVolatilityDraft,
                    )
                  "
                />
              </div>
              <div>
                <label for="vueDmMinExposure">Min exposure (optional)</label>
                <input
                  id="vueDmMinExposure"
                  type="number"
                  min="0"
                  max="1"
                  step="0.0001"
                  :value="minExposureDraft"
                  @input="
                    minExposureDraft = String(
                      ($event.target as HTMLInputElement).value || '',
                    );
                    queueInputValue('dmConstraintMinExposure', minExposureDraft);
                  "
                  @change="
                    commitInputValue('dmConstraintMinExposure', minExposureDraft)
                  "
                />
              </div>
            </div>

            <div class="rowThree">
              <div>
                <label for="vueDmMaxExposure">Max exposure (optional)</label>
                <input
                  id="vueDmMaxExposure"
                  type="number"
                  min="0"
                  max="1"
                  step="0.0001"
                  :value="maxExposureDraft"
                  @input="
                    maxExposureDraft = String(
                      ($event.target as HTMLInputElement).value || '',
                    );
                    queueInputValue('dmConstraintMaxExposure', maxExposureDraft);
                  "
                  @change="
                    commitInputValue('dmConstraintMaxExposure', maxExposureDraft)
                  "
                />
              </div>
              <div>
                <label for="vueDmMaxAbsNetExposure"
                  >Max abs net exposure (optional)</label
                >
                <input
                  id="vueDmMaxAbsNetExposure"
                  type="number"
                  min="0"
                  max="1"
                  step="0.0001"
                  :value="maxAbsNetExposureDraft"
                  @input="
                    maxAbsNetExposureDraft = String(
                      ($event.target as HTMLInputElement).value || '',
                    );
                    queueInputValue(
                      'dmConstraintMaxAbsNetExposure',
                      maxAbsNetExposureDraft,
                    );
                  "
                  @change="
                    commitInputValue(
                      'dmConstraintMaxAbsNetExposure',
                      maxAbsNetExposureDraft,
                    )
                  "
                />
              </div>
              <div>
                <label for="vueDmMinOmega">Min Omega (optional)</label>
                <input
                  id="vueDmMinOmega"
                  type="number"
                  min="0"
                  step="0.01"
                  :value="minOmegaDraft"
                  @input="
                    minOmegaDraft = String(
                      ($event.target as HTMLInputElement).value || '',
                    );
                    queueInputValue('dmConstraintMinOmega', minOmegaDraft);
                  "
                  @change="
                    commitInputValue('dmConstraintMinOmega', minOmegaDraft)
                  "
                />
              </div>
            </div>

            <div class="rowTwo">
              <div>
                <label for="vueDmMaxUlcer">Max ulcer index (optional)</label>
                <input
                  id="vueDmMaxUlcer"
                  type="number"
                  min="0"
                  step="0.0001"
                  :value="maxUlcerDraft"
                  @input="
                    maxUlcerDraft = String(
                      ($event.target as HTMLInputElement).value || '',
                    );
                    queueInputValue('dmConstraintMaxUlcer', maxUlcerDraft);
                  "
                  @change="
                    commitInputValue('dmConstraintMaxUlcer', maxUlcerDraft)
                  "
                />
              </div>
              <div>
                <label for="vueDmMaxDownsideDeviation"
                  >Max downside deviation (optional)</label
                >
                <input
                  id="vueDmMaxDownsideDeviation"
                  type="number"
                  min="0"
                  step="0.0001"
                  :value="maxDownsideDeviationDraft"
                  @input="
                    maxDownsideDeviationDraft = String(
                      ($event.target as HTMLInputElement).value || '',
                    );
                    queueInputValue(
                      'dmConstraintMaxDownsideDeviation',
                      maxDownsideDeviationDraft,
                    );
                  "
                  @change="
                    commitInputValue(
                      'dmConstraintMaxDownsideDeviation',
                      maxDownsideDeviationDraft,
                    )
                  "
                />
              </div>
            </div>
          </div>
        </details>

        <div class="rowTwo">
          <div>
            <label for="vueExitMode">Exit mode</label>
            <select
              id="vueExitMode"
              :value="exitModeDraft"
              @change="
                exitModeDraft = String(
                  ($event.target as HTMLSelectElement).value || 'cross',
                );
                setSelectValue('exitMode', exitModeDraft);
              "
            >
              <option
                v-for="option in exitModeOptionItems"
                :key="`exit_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>

        <div class="rowThree">
          <div v-if="showKaufmanFields">
            <label for="vueKaufmanPeriod">Kaufman period</label>
            <input
              id="vueKaufmanPeriod"
              type="number"
              min="1"
              step="1"
              :value="kaufmanPeriodDraft"
              @input="
                kaufmanPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '22',
                );
                queueInputValue('kaufmanStopPeriod', kaufmanPeriodDraft);
              "
              @change="
                commitInputValue('kaufmanStopPeriod', kaufmanPeriodDraft)
              "
            />
          </div>
          <div v-if="showKaufmanFields">
            <label for="vueKaufmanMult">Kaufman multiplier</label>
            <input
              id="vueKaufmanMult"
              type="number"
              min="0.0001"
              step="0.01"
              :value="kaufmanMultDraft"
              @input="
                kaufmanMultDraft = String(
                  ($event.target as HTMLInputElement).value || '2.0',
                );
                queueInputValue('kaufmanStopMult', kaufmanMultDraft);
              "
              @change="commitInputValue('kaufmanStopMult', kaufmanMultDraft)"
            />
          </div>
          <div v-if="showKaufmanFields">
            <label for="vueKaufmanType">Kaufman MA type</label>
            <input
              id="vueKaufmanType"
              type="text"
              :value="kaufmanTypeDraft"
              @input="
                kaufmanTypeDraft = String(
                  ($event.target as HTMLInputElement).value || 'sma',
                );
                queueInputValue('kaufmanStopMaType', kaufmanTypeDraft);
              "
              @change="commitInputValue('kaufmanStopMaType', kaufmanTypeDraft)"
            />
          </div>
        </div>
      </section>

      <section class="pane" v-if="showAsrControls">
        <h3 class="panelPaneTitle">ASR parameters</h3>
        <div class="rowThree">
          <div>
            <label for="vueAsrSqueezeBbMult">Squeeze BB mult</label>
            <input
              id="vueAsrSqueezeBbMult"
              type="number"
              min="0.0001"
              step="0.01"
              :value="asrSqueezeBbMultDraft"
              @input="
                asrSqueezeBbMultDraft = String(
                  ($event.target as HTMLInputElement).value || '2.0',
                );
                queueInputValue('asrSqueezeBbMult', asrSqueezeBbMultDraft);
              "
              @change="
                commitInputValue('asrSqueezeBbMult', asrSqueezeBbMultDraft)
              "
            />
          </div>
          <div>
            <label for="vueAsrSqueezeKcMult">Squeeze KC mult</label>
            <input
              id="vueAsrSqueezeKcMult"
              type="number"
              min="0.0001"
              step="0.01"
              :value="asrSqueezeKcMultDraft"
              @input="
                asrSqueezeKcMultDraft = String(
                  ($event.target as HTMLInputElement).value || '1.5',
                );
                queueInputValue('asrSqueezeKcMult', asrSqueezeKcMultDraft);
              "
              @change="
                commitInputValue('asrSqueezeKcMult', asrSqueezeKcMultDraft)
              "
            />
          </div>
          <div>
            <label for="vueAsrUseErFilter">Use ER filter</label>
            <input
              id="vueAsrUseErFilter"
              type="checkbox"
              :checked="Boolean(asrUseErFilter)"
              @change="
                strategyStore.setCheckboxField(
                  'asrUseErFilter',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueAsrAngleMin">Angle min (deg)</label>
            <input
              id="vueAsrAngleMin"
              type="number"
              min="0"
              step="0.1"
              :value="asrAngleDegMinDraft"
              @input="
                asrAngleDegMinDraft = String(
                  ($event.target as HTMLInputElement).value || '10.0',
                );
                queueInputValue('asrAngleDegMin', asrAngleDegMinDraft);
              "
              @change="commitInputValue('asrAngleDegMin', asrAngleDegMinDraft)"
            />
          </div>
          <div>
            <label for="vueAsrAngleMr">Angle MR (deg)</label>
            <input
              id="vueAsrAngleMr"
              type="number"
              min="0"
              step="0.1"
              :value="asrAngleDegMrDraft"
              @input="
                asrAngleDegMrDraft = String(
                  ($event.target as HTMLInputElement).value || '6.0',
                );
                queueInputValue('asrAngleDegMr', asrAngleDegMrDraft);
              "
              @change="commitInputValue('asrAngleDegMr', asrAngleDegMrDraft)"
            />
          </div>
          <div>
            <label for="vueAsrErMin">ER min</label>
            <input
              id="vueAsrErMin"
              type="number"
              min="0"
              step="0.01"
              :value="asrErMinDraft"
              @input="
                asrErMinDraft = String(
                  ($event.target as HTMLInputElement).value || '0.30',
                );
                queueInputValue('asrErMin', asrErMinDraft);
              "
              @change="commitInputValue('asrErMin', asrErMinDraft)"
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueAsrErMr">ER MR</label>
            <input
              id="vueAsrErMr"
              type="number"
              min="0"
              step="0.01"
              :value="asrErMrDraft"
              @input="
                asrErMrDraft = String(
                  ($event.target as HTMLInputElement).value || '0.25',
                );
                queueInputValue('asrErMr', asrErMrDraft);
              "
              @change="commitInputValue('asrErMr', asrErMrDraft)"
            />
          </div>
          <div>
            <label for="vueAsrZEntry">Z entry</label>
            <input
              id="vueAsrZEntry"
              type="number"
              min="0"
              step="0.1"
              :value="asrZEntryDraft"
              @input="
                asrZEntryDraft = String(
                  ($event.target as HTMLInputElement).value || '2.0',
                );
                queueInputValue('asrZEntry', asrZEntryDraft);
              "
              @change="commitInputValue('asrZEntry', asrZEntryDraft)"
            />
          </div>
          <div>
            <label for="vueAsrZExit">Z exit</label>
            <input
              id="vueAsrZExit"
              type="number"
              min="0"
              step="0.1"
              :value="asrZExitDraft"
              @input="
                asrZExitDraft = String(
                  ($event.target as HTMLInputElement).value || '0.5',
                );
                queueInputValue('asrZExit', asrZExitDraft);
              "
              @change="commitInputValue('asrZExit', asrZExitDraft)"
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueAsrConfirmBars">Confirm bars</label>
            <input
              id="vueAsrConfirmBars"
              type="number"
              min="0"
              step="1"
              :value="asrConfirmBarsDraft"
              @input="
                asrConfirmBarsDraft = String(
                  ($event.target as HTMLInputElement).value || '0',
                );
                queueInputValue('asrConfirmBars', asrConfirmBarsDraft);
              "
              @change="commitInputValue('asrConfirmBars', asrConfirmBarsDraft)"
            />
          </div>
          <div>
            <label for="vueAsrMomFadeBars">Momentum fade bars</label>
            <input
              id="vueAsrMomFadeBars"
              type="number"
              min="1"
              step="1"
              :value="asrMomFadeBarsDraft"
              @input="
                asrMomFadeBarsDraft = String(
                  ($event.target as HTMLInputElement).value || '2',
                );
                queueInputValue('asrMomFadeBars', asrMomFadeBarsDraft);
              "
              @change="commitInputValue('asrMomFadeBars', asrMomFadeBarsDraft)"
            />
          </div>
          <div>
            <label for="vueAsrMaxTradeBars">Max trend bars</label>
            <input
              id="vueAsrMaxTradeBars"
              type="number"
              min="1"
              step="1"
              :value="asrMaxTradeBarsDraft"
              @input="
                asrMaxTradeBarsDraft = String(
                  ($event.target as HTMLInputElement).value || '80',
                );
                queueInputValue('asrMaxTradeBars', asrMaxTradeBarsDraft);
              "
              @change="
                commitInputValue('asrMaxTradeBars', asrMaxTradeBarsDraft)
              "
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueAsrMaxMrTradeBars">Max MR bars</label>
            <input
              id="vueAsrMaxMrTradeBars"
              type="number"
              min="1"
              step="1"
              :value="asrMaxMrTradeBarsDraft"
              @input="
                asrMaxMrTradeBarsDraft = String(
                  ($event.target as HTMLInputElement).value || '30',
                );
                queueInputValue('asrMaxMrTradeBars', asrMaxMrTradeBarsDraft);
              "
              @change="
                commitInputValue('asrMaxMrTradeBars', asrMaxMrTradeBarsDraft)
              "
            />
          </div>
          <div>
            <label for="vueAsrZMaType">Z MA type</label>
            <input
              id="vueAsrZMaType"
              type="text"
              :value="asrZMaTypeDraft"
              @input="
                asrZMaTypeDraft = String(
                  ($event.target as HTMLInputElement).value || 'sma',
                );
                queueInputValue('asrZMaType', asrZMaTypeDraft);
              "
              @change="commitInputValue('asrZMaType', asrZMaTypeDraft)"
            />
          </div>
          <div>
            <label for="vueAsrZNbdev">Z nbdev</label>
            <input
              id="vueAsrZNbdev"
              type="number"
              min="0.0001"
              step="0.1"
              :value="asrZNbdevDraft"
              @input="
                asrZNbdevDraft = String(
                  ($event.target as HTMLInputElement).value || '1.0',
                );
                queueInputValue('asrZNbdev', asrZNbdevDraft);
              "
              @change="commitInputValue('asrZNbdev', asrZNbdevDraft)"
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueAsrZDevtype">Z devtype</label>
            <input
              id="vueAsrZDevtype"
              type="number"
              min="0"
              step="1"
              :value="asrZDevtypeDraft"
              @input="
                asrZDevtypeDraft = String(
                  ($event.target as HTMLInputElement).value || '0',
                );
                queueInputValue('asrZDevtype', asrZDevtypeDraft);
              "
              @change="commitInputValue('asrZDevtype', asrZDevtypeDraft)"
            />
          </div>
        </div>
      </section>

      <section class="pane" v-if="showVccControls">
        <h3 class="panelPaneTitle">VCC parameters</h3>
        <div class="rowTwo">
          <label class="checkboxRow" for="vueVccMapFastToSqueeze">
            <input
              id="vueVccMapFastToSqueeze"
              type="checkbox"
              :checked="Boolean(vccMapFastToSqueezePeriod)"
              @change="
                strategyStore.setCheckboxField(
                  'vccMapFastToSqueezePeriod',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Map fast length to squeeze period</span>
          </label>
          <label class="checkboxRow" for="vueVccMapFastToMomentum">
            <input
              id="vueVccMapFastToMomentum"
              type="checkbox"
              :checked="Boolean(vccMapFastToMomentumPeriod)"
              @change="
                strategyStore.setCheckboxField(
                  'vccMapFastToMomentumPeriod',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Map fast length to momentum period</span>
          </label>
        </div>
        <div class="rowTwo">
          <label class="checkboxRow" for="vueVccMapSlowToTrend">
            <input
              id="vueVccMapSlowToTrend"
              type="checkbox"
              :checked="Boolean(vccMapSlowToTrendPeriod)"
              @change="
                strategyStore.setCheckboxField(
                  'vccMapSlowToTrendPeriod',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Map slow length to trend period</span>
          </label>
          <label class="checkboxRow" for="vueVccRequireMomentumAccel">
            <input
              id="vueVccRequireMomentumAccel"
              type="checkbox"
              :checked="Boolean(vccRequireMomentumAccel)"
              @change="
                strategyStore.setCheckboxField(
                  'vccRequireMomentumAccel',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Require momentum acceleration</span>
          </label>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueVccSqueezePeriod">Squeeze period</label>
            <input
              id="vueVccSqueezePeriod"
              type="number"
              min="1"
              step="1"
              :disabled="Boolean(vccMapFastToSqueezePeriod)"
              :value="vccSqueezePeriodDraft"
              @input="
                vccSqueezePeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '20',
                );
                queueInputValue('vccSqueezePeriod', vccSqueezePeriodDraft);
              "
              @change="
                commitInputValue('vccSqueezePeriod', vccSqueezePeriodDraft)
              "
            />
          </div>
          <div>
            <label for="vueVccMomentumPeriod">Momentum period</label>
            <input
              id="vueVccMomentumPeriod"
              type="number"
              min="1"
              step="1"
              :disabled="Boolean(vccMapFastToMomentumPeriod)"
              :value="vccMomentumPeriodDraft"
              @input="
                vccMomentumPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '20',
                );
                queueInputValue('vccMomentumPeriod', vccMomentumPeriodDraft);
              "
              @change="
                commitInputValue('vccMomentumPeriod', vccMomentumPeriodDraft)
              "
            />
          </div>
          <div>
            <label for="vueVccMomentumBbPeriod">Momentum BB period</label>
            <input
              id="vueVccMomentumBbPeriod"
              type="number"
              min="1"
              step="1"
              :value="vccMomentumBbPeriodDraft"
              @input="
                vccMomentumBbPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '20',
                );
                queueInputValue(
                  'vccMomentumBbPeriod',
                  vccMomentumBbPeriodDraft,
                );
              "
              @change="
                commitInputValue(
                  'vccMomentumBbPeriod',
                  vccMomentumBbPeriodDraft,
                )
              "
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueVccTrendPeriod">Trend period</label>
            <input
              id="vueVccTrendPeriod"
              type="number"
              min="1"
              step="1"
              :disabled="Boolean(vccMapSlowToTrendPeriod)"
              :value="vccTrendPeriodDraft"
              @input="
                vccTrendPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '6',
                );
                queueInputValue('vccTrendPeriod', vccTrendPeriodDraft);
              "
              @change="commitInputValue('vccTrendPeriod', vccTrendPeriodDraft)"
            />
          </div>
          <div>
            <label for="vueVccSqueezeBbMult">Squeeze BB mult</label>
            <input
              id="vueVccSqueezeBbMult"
              type="number"
              min="0.0001"
              step="0.01"
              :value="vccSqueezeBbMultDraft"
              @input="
                vccSqueezeBbMultDraft = String(
                  ($event.target as HTMLInputElement).value || '2.0',
                );
                queueInputValue('vccSqueezeBbMult', vccSqueezeBbMultDraft);
              "
              @change="
                commitInputValue('vccSqueezeBbMult', vccSqueezeBbMultDraft)
              "
            />
          </div>
          <div>
            <label for="vueVccSqueezeKcMult">Squeeze KC mult</label>
            <input
              id="vueVccSqueezeKcMult"
              type="number"
              min="0.0001"
              step="0.01"
              :value="vccSqueezeKcMultDraft"
              @input="
                vccSqueezeKcMultDraft = String(
                  ($event.target as HTMLInputElement).value || '1.5',
                );
                queueInputValue('vccSqueezeKcMult', vccSqueezeKcMultDraft);
              "
              @change="
                commitInputValue('vccSqueezeKcMult', vccSqueezeKcMultDraft)
              "
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueVccConfirmBars">Confirm bars</label>
            <input
              id="vueVccConfirmBars"
              type="number"
              min="0"
              step="1"
              :value="vccConfirmBarsDraft"
              @input="
                vccConfirmBarsDraft = String(
                  ($event.target as HTMLInputElement).value || '0',
                );
                queueInputValue('vccConfirmBars', vccConfirmBarsDraft);
              "
              @change="commitInputValue('vccConfirmBars', vccConfirmBarsDraft)"
            />
          </div>
          <div>
            <label for="vueVccMaxTradeBars">Max trade bars</label>
            <input
              id="vueVccMaxTradeBars"
              type="number"
              min="1"
              step="1"
              :value="vccMaxTradeBarsDraft"
              @input="
                vccMaxTradeBarsDraft = String(
                  ($event.target as HTMLInputElement).value || '120',
                );
                queueInputValue('vccMaxTradeBars', vccMaxTradeBarsDraft);
              "
              @change="
                commitInputValue('vccMaxTradeBars', vccMaxTradeBarsDraft)
              "
            />
          </div>
        </div>
      </section>

      <section class="pane" v-if="showPitControls">
        <h3 class="panelPaneTitle">PIT parameters</h3>
        <p class="muted">
          PIT builds a pressure-impulse regime from EFI, BOP, VOSC, ACOSC, and
          CFO before feeding the VRAM-resident optimizer.
        </p>
        <div class="rowThree">
          <div>
            <label for="vuePitEfiPeriod">EFI period</label>
            <input
              id="vuePitEfiPeriod"
              type="number"
              min="1"
              step="1"
              :value="pitEfiPeriodDraft"
              @input="
                pitEfiPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '13',
                );
                queueInputValue('pitEfiPeriod', pitEfiPeriodDraft);
              "
              @change="commitInputValue('pitEfiPeriod', pitEfiPeriodDraft)"
            />
          </div>
          <div>
            <label for="vuePitVoscShortPeriod">VOSC short period</label>
            <input
              id="vuePitVoscShortPeriod"
              type="number"
              min="1"
              step="1"
              :value="pitVoscShortPeriodDraft"
              @input="
                pitVoscShortPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '5',
                );
                queueInputValue('pitVoscShortPeriod', pitVoscShortPeriodDraft);
              "
              @change="
                commitInputValue('pitVoscShortPeriod', pitVoscShortPeriodDraft)
              "
            />
          </div>
          <div>
            <label for="vuePitVoscLongPeriod">VOSC long period</label>
            <input
              id="vuePitVoscLongPeriod"
              type="number"
              min="2"
              step="1"
              :value="pitVoscLongPeriodDraft"
              @input="
                pitVoscLongPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '20',
                );
                queueInputValue('pitVoscLongPeriod', pitVoscLongPeriodDraft);
              "
              @change="
                commitInputValue('pitVoscLongPeriod', pitVoscLongPeriodDraft)
              "
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vuePitCfoPeriod">CFO period</label>
            <input
              id="vuePitCfoPeriod"
              type="number"
              min="1"
              step="1"
              :value="pitCfoPeriodDraft"
              @input="
                pitCfoPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '9',
                );
                queueInputValue('pitCfoPeriod', pitCfoPeriodDraft);
              "
              @change="commitInputValue('pitCfoPeriod', pitCfoPeriodDraft)"
            />
          </div>
          <div>
            <label for="vuePitCfoScalar">CFO scalar</label>
            <input
              id="vuePitCfoScalar"
              type="number"
              min="0.0001"
              step="0.1"
              :value="pitCfoScalarDraft"
              @input="
                pitCfoScalarDraft = String(
                  ($event.target as HTMLInputElement).value || '100.0',
                );
                queueInputValue('pitCfoScalar', pitCfoScalarDraft);
              "
              @change="commitInputValue('pitCfoScalar', pitCfoScalarDraft)"
            />
          </div>
          <div>
            <label for="vuePitConfirmBars">Confirm bars</label>
            <input
              id="vuePitConfirmBars"
              type="number"
              min="1"
              step="1"
              :value="pitConfirmBarsDraft"
              @input="
                pitConfirmBarsDraft = String(
                  ($event.target as HTMLInputElement).value || '2',
                );
                queueInputValue('pitConfirmBars', pitConfirmBarsDraft);
              "
              @change="commitInputValue('pitConfirmBars', pitConfirmBarsDraft)"
            />
          </div>
        </div>
      </section>

      <section class="pane" v-if="showVfctControls">
        <h3 class="panelPaneTitle">VFCT parameters</h3>
        <p class="muted">
          VFCT currently uses `vwma` trend lines plus OBV/MFI confirmation.
          Volume-backed OHLCV data is required.
        </p>
        <div class="rowThree">
          <div>
            <label for="vueVfctObvSmoothPeriod">OBV smooth period</label>
            <input
              id="vueVfctObvSmoothPeriod"
              type="number"
              min="1"
              step="1"
              :value="vfctObvSmoothPeriodDraft"
              @input="
                vfctObvSmoothPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '21',
                );
                queueInputValue(
                  'vfctObvSmoothPeriod',
                  vfctObvSmoothPeriodDraft,
                );
              "
              @change="
                commitInputValue(
                  'vfctObvSmoothPeriod',
                  vfctObvSmoothPeriodDraft,
                )
              "
            />
          </div>
          <div>
            <label for="vueVfctMfiPeriod">MFI period</label>
            <input
              id="vueVfctMfiPeriod"
              type="number"
              min="1"
              step="1"
              :value="vfctMfiPeriodDraft"
              @input="
                vfctMfiPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '14',
                );
                queueInputValue('vfctMfiPeriod', vfctMfiPeriodDraft);
              "
              @change="commitInputValue('vfctMfiPeriod', vfctMfiPeriodDraft)"
            />
          </div>
          <div>
            <label for="vueVfctConfirmBars">Confirm bars</label>
            <input
              id="vueVfctConfirmBars"
              type="number"
              min="1"
              step="1"
              :value="vfctConfirmBarsDraft"
              @input="
                vfctConfirmBarsDraft = String(
                  ($event.target as HTMLInputElement).value || '1',
                );
                queueInputValue('vfctConfirmBars', vfctConfirmBarsDraft);
              "
              @change="
                commitInputValue('vfctConfirmBars', vfctConfirmBarsDraft)
              "
            />
          </div>
        </div>
        <div class="rowTwo">
          <div>
            <label for="vueVfctMfiLongMin">MFI long min</label>
            <input
              id="vueVfctMfiLongMin"
              type="number"
              min="0"
              max="100"
              step="0.5"
              :value="vfctMfiLongMinDraft"
              @input="
                vfctMfiLongMinDraft = String(
                  ($event.target as HTMLInputElement).value || '55',
                );
                queueInputValue('vfctMfiLongMin', vfctMfiLongMinDraft);
              "
              @change="commitInputValue('vfctMfiLongMin', vfctMfiLongMinDraft)"
            />
          </div>
          <div>
            <label for="vueVfctMfiShortMax">MFI short max</label>
            <input
              id="vueVfctMfiShortMax"
              type="number"
              min="0"
              max="100"
              step="0.5"
              :value="vfctMfiShortMaxDraft"
              @input="
                vfctMfiShortMaxDraft = String(
                  ($event.target as HTMLInputElement).value || '45',
                );
                queueInputValue('vfctMfiShortMax', vfctMfiShortMaxDraft);
              "
              @change="
                commitInputValue('vfctMfiShortMax', vfctMfiShortMaxDraft)
              "
            />
          </div>
        </div>
      </section>

      <section class="pane" v-if="showRocbControls">
        <h3 class="panelPaneTitle">ROCB parameters</h3>
        <p class="muted">
          ROCB builds an N-bar channel breakout and ATR risk overlay on the GPU,
          then feeds that regime into the existing optimizer.
        </p>
        <div class="rowTwo">
          <label class="checkboxRow" for="vueRocbMapFastToChannelPeriod">
            <input
              id="vueRocbMapFastToChannelPeriod"
              type="checkbox"
              :checked="Boolean(rocbMapFastToChannelPeriod)"
              @change="
                strategyStore.setCheckboxField(
                  'rocbMapFastToChannelPeriod',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Map fast length to channel period</span>
          </label>
          <label class="checkboxRow" for="vueRocbMapSlowToAtrPeriod">
            <input
              id="vueRocbMapSlowToAtrPeriod"
              type="checkbox"
              :checked="Boolean(rocbMapSlowToAtrPeriod)"
              @change="
                strategyStore.setCheckboxField(
                  'rocbMapSlowToAtrPeriod',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Map slow length to ATR period</span>
          </label>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueRocbChannelPeriod">Channel period</label>
            <input
              id="vueRocbChannelPeriod"
              type="number"
              min="1"
              step="1"
              :disabled="Boolean(rocbMapFastToChannelPeriod)"
              :value="rocbChannelPeriodDraft"
              @input="
                rocbChannelPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '20',
                );
                queueInputValue('rocbChannelPeriod', rocbChannelPeriodDraft);
              "
              @change="
                commitInputValue('rocbChannelPeriod', rocbChannelPeriodDraft)
              "
            />
          </div>
          <div>
            <label for="vueRocbAtrPeriod">ATR period</label>
            <input
              id="vueRocbAtrPeriod"
              type="number"
              min="1"
              step="1"
              :disabled="Boolean(rocbMapSlowToAtrPeriod)"
              :value="rocbAtrPeriodDraft"
              @input="
                rocbAtrPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '14',
                );
                queueInputValue('rocbAtrPeriod', rocbAtrPeriodDraft);
              "
              @change="commitInputValue('rocbAtrPeriod', rocbAtrPeriodDraft)"
            />
          </div>
          <div>
            <label for="vueRocbAtrStopMult">ATR stop mult</label>
            <input
              id="vueRocbAtrStopMult"
              type="number"
              min="0.0001"
              step="0.1"
              :value="rocbAtrStopMultDraft"
              @input="
                rocbAtrStopMultDraft = String(
                  ($event.target as HTMLInputElement).value || '2.0',
                );
                queueInputValue('rocbAtrStopMult', rocbAtrStopMultDraft);
              "
              @change="
                commitInputValue('rocbAtrStopMult', rocbAtrStopMultDraft)
              "
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueRocbMaxTradeBars">Max trade bars</label>
            <input
              id="vueRocbMaxTradeBars"
              type="number"
              min="1"
              step="1"
              :value="rocbMaxTradeBarsDraft"
              @input="
                rocbMaxTradeBarsDraft = String(
                  ($event.target as HTMLInputElement).value || '80',
                );
                queueInputValue('rocbMaxTradeBars', rocbMaxTradeBarsDraft);
              "
              @change="
                commitInputValue('rocbMaxTradeBars', rocbMaxTradeBarsDraft)
              "
            />
          </div>
        </div>
      </section>

      <section class="pane" v-if="showVcetControls">
        <h3 class="panelPaneTitle">VCET parameters</h3>
        <p class="muted">
          VCET combines a mapped VIDYA trend anchor with a Chandelier Exit
          regime filter. It now supports unified GPU execution with CPU fallback
          while optimizing over the global fast/slow lattice.
        </p>
        <div class="rowThree">
          <label class="checkboxRow" for="vueVcetMapFastToVidyaShortPeriod">
            <input
              id="vueVcetMapFastToVidyaShortPeriod"
              type="checkbox"
              :checked="Boolean(vcetMapFastToVidyaShortPeriod)"
              @change="
                strategyStore.setCheckboxField(
                  'vcetMapFastToVidyaShortPeriod',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Map fast length to VIDYA short</span>
          </label>
          <label class="checkboxRow" for="vueVcetMapSlowToVidyaLongPeriod">
            <input
              id="vueVcetMapSlowToVidyaLongPeriod"
              type="checkbox"
              :checked="Boolean(vcetMapSlowToVidyaLongPeriod)"
              @change="
                strategyStore.setCheckboxField(
                  'vcetMapSlowToVidyaLongPeriod',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Map slow length to VIDYA long</span>
          </label>
          <label class="checkboxRow" for="vueVcetMapSlowToCePeriod">
            <input
              id="vueVcetMapSlowToCePeriod"
              type="checkbox"
              :checked="Boolean(vcetMapSlowToCePeriod)"
              @change="
                strategyStore.setCheckboxField(
                  'vcetMapSlowToCePeriod',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Map slow length to Chandelier period</span>
          </label>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueVcetVidyaShortPeriod">VIDYA short</label>
            <input
              id="vueVcetVidyaShortPeriod"
              type="number"
              min="1"
              step="1"
              :disabled="Boolean(vcetMapFastToVidyaShortPeriod)"
              :value="vcetVidyaShortPeriodDraft"
              @input="
                vcetVidyaShortPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '6',
                );
                queueInputValue(
                  'vcetVidyaShortPeriod',
                  vcetVidyaShortPeriodDraft,
                );
              "
              @change="
                commitInputValue(
                  'vcetVidyaShortPeriod',
                  vcetVidyaShortPeriodDraft,
                )
              "
            />
          </div>
          <div>
            <label for="vueVcetVidyaLongPeriod">VIDYA long</label>
            <input
              id="vueVcetVidyaLongPeriod"
              type="number"
              min="2"
              step="1"
              :disabled="Boolean(vcetMapSlowToVidyaLongPeriod)"
              :value="vcetVidyaLongPeriodDraft"
              @input="
                vcetVidyaLongPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '24',
                );
                queueInputValue(
                  'vcetVidyaLongPeriod',
                  vcetVidyaLongPeriodDraft,
                );
              "
              @change="
                commitInputValue(
                  'vcetVidyaLongPeriod',
                  vcetVidyaLongPeriodDraft,
                )
              "
            />
          </div>
          <div>
            <label for="vueVcetVidyaAlpha">VIDYA alpha</label>
            <input
              id="vueVcetVidyaAlpha"
              type="number"
              min="0.0001"
              step="0.01"
              :value="vcetVidyaAlphaDraft"
              @input="
                vcetVidyaAlphaDraft = String(
                  ($event.target as HTMLInputElement).value || '0.2',
                );
                queueInputValue('vcetVidyaAlpha', vcetVidyaAlphaDraft);
              "
              @change="commitInputValue('vcetVidyaAlpha', vcetVidyaAlphaDraft)"
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueVcetCePeriod">Chandelier period</label>
            <input
              id="vueVcetCePeriod"
              type="number"
              min="1"
              step="1"
              :disabled="Boolean(vcetMapSlowToCePeriod)"
              :value="vcetCePeriodDraft"
              @input="
                vcetCePeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '22',
                );
                queueInputValue('vcetCePeriod', vcetCePeriodDraft);
              "
              @change="commitInputValue('vcetCePeriod', vcetCePeriodDraft)"
            />
          </div>
          <div>
            <label for="vueVcetCeMult">Chandelier mult</label>
            <input
              id="vueVcetCeMult"
              type="number"
              min="0.0001"
              step="0.1"
              :value="vcetCeMultDraft"
              @input="
                vcetCeMultDraft = String(
                  ($event.target as HTMLInputElement).value || '3.0',
                );
                queueInputValue('vcetCeMult', vcetCeMultDraft);
              "
              @change="commitInputValue('vcetCeMult', vcetCeMultDraft)"
            />
          </div>
          <div>
            <label for="vueVcetMaxTradeBars">Max trade bars</label>
            <input
              id="vueVcetMaxTradeBars"
              type="number"
              min="1"
              step="1"
              :value="vcetMaxTradeBarsDraft"
              @input="
                vcetMaxTradeBarsDraft = String(
                  ($event.target as HTMLInputElement).value || '120',
                );
                queueInputValue('vcetMaxTradeBars', vcetMaxTradeBarsDraft);
              "
              @change="
                commitInputValue('vcetMaxTradeBars', vcetMaxTradeBarsDraft)
              "
            />
          </div>
        </div>
        <div class="rowTwo">
          <label class="checkboxRow" for="vueVcetCeUseClose">
            <input
              id="vueVcetCeUseClose"
              type="checkbox"
              :checked="Boolean(vcetCeUseClose)"
              @change="
                strategyStore.setCheckboxField(
                  'vcetCeUseClose',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Use close in Chandelier Exit</span>
          </label>
          <label class="checkboxRow" for="vueVcetRequireVidyaSlope">
            <input
              id="vueVcetRequireVidyaSlope"
              type="checkbox"
              :checked="Boolean(vcetRequireVidyaSlope)"
              @change="
                strategyStore.setCheckboxField(
                  'vcetRequireVidyaSlope',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Require VIDYA slope confirmation</span>
          </label>
        </div>
      </section>

      <section class="pane" v-if="showVemrControls">
        <h3 class="panelPaneTitle">VEMR parameters</h3>
        <div class="rowTwo">
          <label class="checkboxRow" for="vueVemrMapFastToZ">
            <input
              id="vueVemrMapFastToZ"
              type="checkbox"
              :checked="Boolean(vemrMapFastToZPeriod)"
              @change="
                strategyStore.setCheckboxField(
                  'vemrMapFastToZPeriod',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Map fast length to Z period</span>
          </label>
          <label class="checkboxRow" for="vueVemrMapFastToBb">
            <input
              id="vueVemrMapFastToBb"
              type="checkbox"
              :checked="Boolean(vemrMapFastToBbPeriod)"
              @change="
                strategyStore.setCheckboxField(
                  'vemrMapFastToBbPeriod',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Map fast length to BB period</span>
          </label>
        </div>
        <div class="rowTwo">
          <label class="checkboxRow" for="vueVemrMapSlowToAtr">
            <input
              id="vueVemrMapSlowToAtr"
              type="checkbox"
              :checked="Boolean(vemrMapSlowToAtrPeriod)"
              @change="
                strategyStore.setCheckboxField(
                  'vemrMapSlowToAtrPeriod',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Map slow length to ATR period</span>
          </label>
          <label class="checkboxRow" for="vueVemrRequireBandTouch">
            <input
              id="vueVemrRequireBandTouch"
              type="checkbox"
              :checked="Boolean(vemrRequireBandTouch)"
              @change="
                strategyStore.setCheckboxField(
                  'vemrRequireBandTouch',
                  !!($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>Require BB band touch on entry</span>
          </label>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueVemrZPeriod">Z period</label>
            <input
              id="vueVemrZPeriod"
              type="number"
              min="1"
              step="1"
              :disabled="Boolean(vemrMapFastToZPeriod)"
              :value="vemrZPeriodDraft"
              @input="
                vemrZPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '20',
                );
                queueInputValue('vemrZPeriod', vemrZPeriodDraft);
              "
              @change="commitInputValue('vemrZPeriod', vemrZPeriodDraft)"
            />
          </div>
          <div>
            <label for="vueVemrZEntry">Z entry</label>
            <input
              id="vueVemrZEntry"
              type="number"
              min="0.0001"
              step="0.1"
              :value="vemrZEntryDraft"
              @input="
                vemrZEntryDraft = String(
                  ($event.target as HTMLInputElement).value || '2.0',
                );
                queueInputValue('vemrZEntry', vemrZEntryDraft);
              "
              @change="commitInputValue('vemrZEntry', vemrZEntryDraft)"
            />
          </div>
          <div>
            <label for="vueVemrZExit">Z exit</label>
            <input
              id="vueVemrZExit"
              type="number"
              min="0"
              step="0.1"
              :value="vemrZExitDraft"
              @input="
                vemrZExitDraft = String(
                  ($event.target as HTMLInputElement).value || '0.5',
                );
                queueInputValue('vemrZExit', vemrZExitDraft);
              "
              @change="commitInputValue('vemrZExit', vemrZExitDraft)"
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueVemrZMaType">Z MA type</label>
            <input
              id="vueVemrZMaType"
              type="text"
              :value="vemrZMaTypeDraft"
              @input="
                vemrZMaTypeDraft = String(
                  ($event.target as HTMLInputElement).value || 'sma',
                );
                queueInputValue('vemrZMaType', vemrZMaTypeDraft);
              "
              @change="commitInputValue('vemrZMaType', vemrZMaTypeDraft)"
            />
          </div>
          <div>
            <label for="vueVemrZNbdev">Z nbdev</label>
            <input
              id="vueVemrZNbdev"
              type="number"
              min="0.0001"
              step="0.1"
              :value="vemrZNbdevDraft"
              @input="
                vemrZNbdevDraft = String(
                  ($event.target as HTMLInputElement).value || '1.0',
                );
                queueInputValue('vemrZNbdev', vemrZNbdevDraft);
              "
              @change="commitInputValue('vemrZNbdev', vemrZNbdevDraft)"
            />
          </div>
          <div>
            <label for="vueVemrZDevtype">Z devtype</label>
            <input
              id="vueVemrZDevtype"
              type="number"
              min="0"
              step="1"
              :value="vemrZDevtypeDraft"
              @input="
                vemrZDevtypeDraft = String(
                  ($event.target as HTMLInputElement).value || '0',
                );
                queueInputValue('vemrZDevtype', vemrZDevtypeDraft);
              "
              @change="commitInputValue('vemrZDevtype', vemrZDevtypeDraft)"
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueVemrBbPeriod">BB period</label>
            <input
              id="vueVemrBbPeriod"
              type="number"
              min="1"
              step="1"
              :disabled="Boolean(vemrMapFastToBbPeriod)"
              :value="vemrBbPeriodDraft"
              @input="
                vemrBbPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '20',
                );
                queueInputValue('vemrBbPeriod', vemrBbPeriodDraft);
              "
              @change="commitInputValue('vemrBbPeriod', vemrBbPeriodDraft)"
            />
          </div>
          <div>
            <label for="vueVemrBbMult">BB multiplier</label>
            <input
              id="vueVemrBbMult"
              type="number"
              min="0.0001"
              step="0.1"
              :value="vemrBbMultDraft"
              @input="
                vemrBbMultDraft = String(
                  ($event.target as HTMLInputElement).value || '2.0',
                );
                queueInputValue('vemrBbMult', vemrBbMultDraft);
              "
              @change="commitInputValue('vemrBbMult', vemrBbMultDraft)"
            />
          </div>
          <div>
            <label for="vueVemrAtrPeriod">ATR period</label>
            <input
              id="vueVemrAtrPeriod"
              type="number"
              min="1"
              step="1"
              :disabled="Boolean(vemrMapSlowToAtrPeriod)"
              :value="vemrAtrPeriodDraft"
              @input="
                vemrAtrPeriodDraft = String(
                  ($event.target as HTMLInputElement).value || '14',
                );
                queueInputValue('vemrAtrPeriod', vemrAtrPeriodDraft);
              "
              @change="commitInputValue('vemrAtrPeriod', vemrAtrPeriodDraft)"
            />
          </div>
        </div>
        <div class="rowThree">
          <div>
            <label for="vueVemrAtrStopMult">ATR stop mult</label>
            <input
              id="vueVemrAtrStopMult"
              type="number"
              min="0.0001"
              step="0.1"
              :value="vemrAtrStopMultDraft"
              @input="
                vemrAtrStopMultDraft = String(
                  ($event.target as HTMLInputElement).value || '2.0',
                );
                queueInputValue('vemrAtrStopMult', vemrAtrStopMultDraft);
              "
              @change="
                commitInputValue('vemrAtrStopMult', vemrAtrStopMultDraft)
              "
            />
          </div>
          <div>
            <label for="vueVemrConfirmBars">Confirm bars</label>
            <input
              id="vueVemrConfirmBars"
              type="number"
              min="0"
              step="1"
              :value="vemrConfirmBarsDraft"
              @input="
                vemrConfirmBarsDraft = String(
                  ($event.target as HTMLInputElement).value || '0',
                );
                queueInputValue('vemrConfirmBars', vemrConfirmBarsDraft);
              "
              @change="
                commitInputValue('vemrConfirmBars', vemrConfirmBarsDraft)
              "
            />
          </div>
          <div>
            <label for="vueVemrMaxTradeBars">Max trade bars</label>
            <input
              id="vueVemrMaxTradeBars"
              type="number"
              min="1"
              step="1"
              :value="vemrMaxTradeBarsDraft"
              @input="
                vemrMaxTradeBarsDraft = String(
                  ($event.target as HTMLInputElement).value || '80',
                );
                queueInputValue('vemrMaxTradeBars', vemrMaxTradeBarsDraft);
              "
              @change="
                commitInputValue('vemrMaxTradeBars', vemrMaxTradeBarsDraft)
              "
            />
          </div>
        </div>
      </section>

      <div class="status mono" role="status" aria-live="polite">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.strategyPanelVueRoot {
  display: grid;
  gap: 0.85rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.riskNotice {
  border: 1px solid color-mix(in oklab, var(--warning-ink) 34%, var(--border));
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-warning) 86%, var(--panel-footer));
  padding: 0.8rem 0.9rem;
}

.riskTitle {
  font-size: 0.86rem;
  font-weight: 650;
  color: var(--warning-ink);
}

.riskBody {
  margin-top: 0.35rem;
  color: var(--text);
  font-size: 0.82rem;
  line-height: 1.45;
}

.pane :is(h3, h4) {
  margin: 0.1rem 0 0;
}

.rowThree {
  margin-top: 0.6rem;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.6rem;
}

.rowTwo {
  margin-top: 0.6rem;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.6rem;
}

.inlineMeta {
  align-self: end;
}

.actions {
  margin-top: 0.75rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.checkboxRow {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.82rem;
}

.status {
  font-size: 0.78rem;
  color: var(--muted);
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 980px) {
  .rowThree,
  .rowTwo {
    grid-template-columns: 1fr;
  }
}
</style>
