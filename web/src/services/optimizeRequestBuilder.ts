import { countF64RangeValues, countPairsFastLtSlow, expandRangeU16Like, num } from '../utils/core/ranges'
import { validateCandleFieldRequirements } from '../utils/core/candleFields'

const DEFAULT_STARTING_EQUITY = 10_000

export interface OptimizeBuildContext {
  strategy_kind?: unknown
  data_id?: unknown
  current_data_meta?: unknown
}

export interface DoubleMaOptimizationBuildInputs {
  context: OptimizeBuildContext
  maSupportMap?: unknown
  backend?: unknown
  fastStart?: unknown
  fastEnd?: unknown
  fastStep?: unknown
  slowStart?: unknown
  slowEnd?: unknown
  slowStep?: unknown
  fastMaType?: unknown
  slowMaType?: unknown
  maSource?: unknown
  objective?: unknown
  mode?: unknown
  topK?: unknown
  includeAll?: unknown
  heatmapBins?: unknown
  exportAllCsvPath?: unknown
  startingEquity?: unknown
  costPreset?: unknown
  feeBps?: unknown
  spreadBps?: unknown
  slippageBps?: unknown
  epsRel?: unknown
  stopLossPct?: unknown
  takeProfitPct?: unknown
  cooldownBars?: unknown
  signalPersistenceBars?: unknown
  exitMode?: unknown
  kaufmanStopPeriod?: unknown
  kaufmanStopMult?: unknown
  kaufmanStopMaType?: unknown
  trendLrPeriod?: unknown
  trendLrMinDeg?: unknown
  trendBbwPeriod?: unknown
  trendBbwMult?: unknown
  trendBbwMin?: unknown
  trendRocPeriod?: unknown
  trendUseRoc?: unknown
  trendUseAdx?: unknown
  trendAdxPeriod?: unknown
  trendAdxMin?: unknown
  regimeAdxPeriod?: unknown
  regimeAdxEnter?: unknown
  regimeAdxExit?: unknown
  regimeRevEntryRel?: unknown
  regimeRevExitRel?: unknown
  breakoutBbPeriod?: unknown
  breakoutBbMult?: unknown
  breakoutConfirmBars?: unknown
  breakoutMacdFast?: unknown
  breakoutMacdSlow?: unknown
  breakoutMacdSignal?: unknown
  breakoutCmoPeriod?: unknown
  breakoutCmoMinAbs?: unknown
  breakoutAdoscShort?: unknown
  breakoutAdoscLong?: unknown
  breakoutUseAdx?: unknown
  breakoutAdxPeriod?: unknown
  breakoutAdxMin?: unknown
  pitEfiPeriod?: unknown
  pitVoscShortPeriod?: unknown
  pitVoscLongPeriod?: unknown
  pitCfoPeriod?: unknown
  pitCfoScalar?: unknown
  pitConfirmBars?: unknown
  vfctObvSmoothPeriod?: unknown
  vfctMfiPeriod?: unknown
  vfctMfiLongMin?: unknown
  vfctMfiShortMax?: unknown
  vfctConfirmBars?: unknown
  longOnly?: unknown
  allowFlip?: unknown
  tradeOnNext?: unknown
  dmConstraintMinTrades?: unknown
  dmConstraintMaxDd?: unknown
  fastMaParams?: unknown
  slowMaParams?: unknown
  fastMaParamSweeps?: unknown
  slowMaParamSweeps?: unknown
  vdpsEta?: unknown
  vdpsRungs?: unknown
  vdpsMinInitialPairs?: unknown
  vdpsMaxSurvivorSeeds?: unknown
  vdpsMaxPeriodCandidates?: unknown
  vdpsRandomInjectionRatio?: unknown
  vdpsEliteMutationRatio?: unknown
  vdpsDifferentialRatio?: unknown
  vdpsDeterministic?: unknown
  vdpsSeed?: unknown
  vdpsQualityCheckMaxPairs?: unknown
  vdpsQualityCheckTopK?: unknown
}

interface StrategyConfigBuildInputs {
  startingEquity?: unknown
  costPreset?: unknown
  feeBps?: unknown
  spreadBps?: unknown
  slippageBps?: unknown
  epsRel?: unknown
  stopLossPct?: unknown
  takeProfitPct?: unknown
  cooldownBars?: unknown
  signalPersistenceBars?: unknown
  exitMode?: unknown
  kaufmanStopPeriod?: unknown
  kaufmanStopMult?: unknown
  kaufmanStopMaType?: unknown
  longOnly?: unknown
  allowFlip?: unknown
  tradeOnNext?: unknown
  trendLrPeriod?: unknown
  trendLrMinDeg?: unknown
  trendBbwPeriod?: unknown
  trendBbwMult?: unknown
  trendBbwMin?: unknown
  trendRocPeriod?: unknown
  trendUseRoc?: unknown
  trendUseAdx?: unknown
  trendAdxPeriod?: unknown
  trendAdxMin?: unknown
  regimeAdxPeriod?: unknown
  regimeAdxEnter?: unknown
  regimeAdxExit?: unknown
  regimeRevEntryRel?: unknown
  regimeRevExitRel?: unknown
  breakoutBbPeriod?: unknown
  breakoutBbMult?: unknown
  breakoutConfirmBars?: unknown
  breakoutMacdFast?: unknown
  breakoutMacdSlow?: unknown
  breakoutMacdSignal?: unknown
  breakoutCmoPeriod?: unknown
  breakoutCmoMinAbs?: unknown
  breakoutAdoscShort?: unknown
  breakoutAdoscLong?: unknown
  breakoutUseAdx?: unknown
  breakoutAdxPeriod?: unknown
  breakoutAdxMin?: unknown
  pitEfiPeriod?: unknown
  pitVoscShortPeriod?: unknown
  pitVoscLongPeriod?: unknown
  pitCfoPeriod?: unknown
  pitCfoScalar?: unknown
  pitConfirmBars?: unknown
  vfctObvSmoothPeriod?: unknown
  vfctMfiPeriod?: unknown
  vfctMfiLongMin?: unknown
  vfctMfiShortMax?: unknown
  vfctConfirmBars?: unknown
}

export interface CompositeCpuOptimizationBuildInputs extends StrategyConfigBuildInputs {
  context: OptimizeBuildContext
  maSupportMap?: unknown
  backend?: unknown
  fastStart?: unknown
  fastEnd?: unknown
  fastStep?: unknown
  slowStart?: unknown
  slowEnd?: unknown
  slowStep?: unknown
  fastMaType?: unknown
  slowMaType?: unknown
  maSource?: unknown
  objective?: unknown
  mode?: unknown
  topK?: unknown
  includeAll?: unknown
  compositeSignalMode?: unknown
  halfTrendAmplitude?: unknown
  halfTrendAtrPeriod?: unknown
  halfTrendChannelDev?: unknown
  maczFastLength?: unknown
  maczSlowLength?: unknown
  maczSignalLength?: unknown
  maczLengthZ?: unknown
  maczLengthStdev?: unknown
  maczGamma?: unknown
  maczA?: unknown
  maczB?: unknown
  maczUseVolume?: unknown
  maczUseLag?: unknown
  ottPeriod?: unknown
  ottPercent?: unknown
  ottMaType?: unknown
  asrSqueezeBbMult?: unknown
  asrSqueezeKcMult?: unknown
  asrAngleDegMin?: unknown
  asrAngleDegMr?: unknown
  asrUseErFilter?: unknown
  asrErMin?: unknown
  asrErMr?: unknown
  asrZMaType?: unknown
  asrZNbdev?: unknown
  asrZDevtype?: unknown
  asrZEntry?: unknown
  asrZExit?: unknown
  asrConfirmBars?: unknown
  asrMomFadeBars?: unknown
  asrMaxTradeBars?: unknown
  asrMaxMrTradeBars?: unknown
  vccMapFastToSqueezePeriod?: unknown
  vccMapFastToMomentumPeriod?: unknown
  vccMapSlowToTrendPeriod?: unknown
  vccSqueezePeriod?: unknown
  vccMomentumPeriod?: unknown
  vccMomentumBbPeriod?: unknown
  vccTrendPeriod?: unknown
  vccSqueezeBbMult?: unknown
  vccSqueezeKcMult?: unknown
  vccConfirmBars?: unknown
  vccRequireMomentumAccel?: unknown
  vccMaxTradeBars?: unknown
  vemrMapFastToZPeriod?: unknown
  vemrMapFastToBbPeriod?: unknown
  vemrMapSlowToAtrPeriod?: unknown
  vemrZPeriod?: unknown
  vemrZMaType?: unknown
  vemrZNbdev?: unknown
  vemrZDevtype?: unknown
  vemrZEntry?: unknown
  vemrZExit?: unknown
  vemrBbPeriod?: unknown
  vemrBbMult?: unknown
  vemrRequireBandTouch?: unknown
  vemrAtrPeriod?: unknown
  vemrAtrStopMult?: unknown
  vemrConfirmBars?: unknown
  vemrMaxTradeBars?: unknown
  rocbMapFastToChannelPeriod?: unknown
  rocbMapSlowToAtrPeriod?: unknown
  rocbChannelPeriod?: unknown
  rocbAtrPeriod?: unknown
  rocbAtrStopMult?: unknown
  rocbMaxTradeBars?: unknown
  fastMaParams?: unknown
  slowMaParams?: unknown
}

export interface CompositeMoOptimizationBuildInputs extends StrategyConfigBuildInputs {
  context: OptimizeBuildContext
  maSupportMap?: unknown
  fastStart?: unknown
  fastEnd?: unknown
  fastStep?: unknown
  slowStart?: unknown
  slowEnd?: unknown
  slowStep?: unknown
  fastMaType?: unknown
  slowMaType?: unknown
  maSource?: unknown
  objective?: unknown
  moObjectiveMin?: unknown
  compositeSignalMode?: unknown
  fastMaParams?: unknown
  slowMaParams?: unknown
  moSampler?: unknown
  moRandomSeed?: unknown
  moRandomTrials?: unknown
  moTpeSeed?: unknown
  moTpeTrials?: unknown
  moTpeStartupTrials?: unknown
  moTpeCandidates?: unknown
  moTpeGamma?: unknown
  moTpeBatchSize?: unknown
  moTpeMaxModelPoints?: unknown
  moQmcSeed?: unknown
  moQmcTrials?: unknown
  moQmcSequence?: unknown
  moNsga2Seed?: unknown
  moNsga2Population?: unknown
  moNsga2Generations?: unknown
  moNsga2CrossoverProb?: unknown
  moNsga2SwappingProb?: unknown
  moNsga2MutationProb?: unknown
  moConstraintMinTrades?: unknown
  moConstraintMaxDd?: unknown
  moConstraintMaxAbsExposure?: unknown
  moPrescanEnable?: unknown
  moPrescanTopK?: unknown
  moPrescanWindow?: unknown
  moPrescanPredictiveTopK?: unknown
  moPrescanPredictiveStride?: unknown
  moPrescanRequireGpu?: unknown
  moPrescanObjectives?: unknown
  moPrescanMode?: unknown
  moPrescanMaPairs?: unknown
  moUseRangeBuilder?: unknown
  moSearchSpace?: unknown
  moHtAmpLow?: unknown
  moHtAmpHigh?: unknown
  moHtAmpStep?: unknown
  moHtDevLow?: unknown
  moHtDevHigh?: unknown
  moHtDevStep?: unknown
  moHtAtrLow?: unknown
  moHtAtrHigh?: unknown
  moHtAtrStep?: unknown
  moMaczFastLow?: unknown
  moMaczFastHigh?: unknown
  moMaczFastStep?: unknown
  moMaczSlowLow?: unknown
  moMaczSlowHigh?: unknown
  moMaczSlowStep?: unknown
  moMaczSignalLow?: unknown
  moMaczSignalHigh?: unknown
  moMaczSignalStep?: unknown
  moMaczLengthZLow?: unknown
  moMaczLengthZHigh?: unknown
  moMaczLengthZStep?: unknown
  moMaczLengthStdevLow?: unknown
  moMaczLengthStdevHigh?: unknown
  moMaczLengthStdevStep?: unknown
  moMaczGammaLow?: unknown
  moMaczGammaHigh?: unknown
  moMaczGammaStep?: unknown
  moMaczALow?: unknown
  moMaczAHigh?: unknown
  moMaczAStep?: unknown
  moMaczBLow?: unknown
  moMaczBHigh?: unknown
  moMaczBStep?: unknown
  moMaczUseLagMode?: unknown
  moMaczUseVolumeMode?: unknown
  moOttPeriodLow?: unknown
  moOttPeriodHigh?: unknown
  moOttPeriodStep?: unknown
  moOttPercentLow?: unknown
  moOttPercentHigh?: unknown
  moOttPercentStep?: unknown
  moOttMaTypes?: unknown
}

export interface MaParamsAndSweeps {
  params: Record<string, number> | null
  sweeps: Record<string, [number, number, number]> | null
}

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizeStrategyKindRaw(kindRaw: unknown): string {
  const kind = normalizeText(kindRaw || 'DoubleMa') || 'DoubleMa'
  if (kind === 'Pit') return 'Pit'
  if (kind === 'Set') return 'Set'
  if (kind === 'Mts') return 'Mts'
  if (kind === 'Orc') return 'Orc'
  if (kind === 'Marrt') return 'Marrt'
  if (kind === 'Vfct') return 'Vfct'
  if (kind === 'Rocb') return 'Rocb'
  if (kind === 'Mkcs') return 'Mkcs'
  if (kind === 'Asr') return 'Asr'
  if (kind === 'Vcc') return 'Vcc'
  if (kind === 'Vemr') return 'Vemr'
  if (kind === 'CompositeCpu') return 'CompositeCpu'
  if (kind === 'CompositeMo') return 'CompositeMo'
  if (kind === 'CompositeTrend') return 'CompositeTrend'
  if (kind === 'RegimeHybrid') return 'RegimeHybrid'
  if (kind === 'BreakoutMomentum') return 'BreakoutMomentum'
  return 'DoubleMa'
}

const MARRT_MA_WHITELIST = new Set(['sma', 'ema', 'alma', 'wma', 'zlema', 'vwma'])
const VFCT_MA_WHITELIST = new Set(['vwma'])

function normalizeBoolean(value: unknown): boolean {
  if (typeof value === 'boolean') return value
  const text = normalizeText(value).toLowerCase()
  return text === 'true' || text === '1' || text === 'yes' || text === 'on'
}

function normalizeIncludeAll(value: unknown): boolean {
  const text = normalizeText(value).toLowerCase()
  return text === 'true'
}

function parseRiskPct(value: unknown): number | null {
  const raw = num(value, Number.NaN)
  if (!Number.isFinite(raw) || raw <= 0) return null
  const frac = raw > 1 ? raw / 100 : raw
  if (!Number.isFinite(frac) || frac <= 0 || frac >= 1) return null
  return frac
}

function normalizeStartingEquity(value: unknown): number {
  const x = Number(value)
  if (!Number.isFinite(x) || x <= 0) return DEFAULT_STARTING_EQUITY
  return x
}

function parseOptionalFinite(value: unknown): number | null {
  const text = normalizeText(value)
  if (!text) return null
  const parsed = Number(text)
  if (!Number.isFinite(parsed)) return null
  return parsed
}

function parseOptionalNonNegativeInteger(value: unknown): number | null {
  const parsed = parseOptionalFinite(value)
  if (parsed == null) return null
  const integer = Math.floor(parsed)
  if (!Number.isFinite(integer)) return null
  if (integer < 0) throw new Error('Min trades must be >= 0.')
  return integer
}

function parseOptionalNonNegativeNumber(value: unknown, label: string): number | null {
  const parsed = parseOptionalFinite(value)
  if (parsed == null) return null
  if (parsed < 0) throw new Error(`${label} must be >= 0.`)
  return parsed
}

function normalizeMaParams(value: unknown): Record<string, number> | null {
  if (!value || typeof value !== 'object') return null
  const out: Record<string, number> = {}
  for (const [key, raw] of Object.entries(value as Record<string, unknown>)) {
    const name = normalizeText(key)
    if (!name) continue
    const parsed = Number(raw)
    if (!Number.isFinite(parsed)) continue
    out[name] = parsed
  }
  return Object.keys(out).length ? out : null
}

function normalizeMaParamSweeps(value: unknown): Record<string, [number, number, number]> | null {
  if (!value || typeof value !== 'object') return null
  const out: Record<string, [number, number, number]> = {}
  for (const [key, raw] of Object.entries(value as Record<string, unknown>)) {
    const name = normalizeText(key)
    if (!name) continue
    let start = Number.NaN
    let end = Number.NaN
    let step = Number.NaN
    if (Array.isArray(raw)) {
      start = Number(raw[0])
      end = Number(raw[1])
      step = Number(raw[2])
    } else if (raw && typeof raw === 'object') {
      const row = raw as Record<string, unknown>
      start = Number(row.start)
      end = Number(row.end)
      step = Number(row.step)
    }
    if (!Number.isFinite(start) || !Number.isFinite(end) || !Number.isFinite(step) || step <= 0) continue
    out[name] = [start, end, step]
  }
  return Object.keys(out).length ? out : null
}

function maSweepVariantCount(value: Record<string, [number, number, number]> | null): number {
  if (!value) return 1
  let count = 1
  for (const [, range] of Object.entries(value)) {
    count *= Math.max(1, countF64RangeValues(range[0], range[1], range[2]))
    if (!Number.isFinite(count) || count <= 0) return 1
  }
  return Math.max(1, Math.floor(count))
}

function normalizeBackend(value: unknown): 'Auto' | 'CpuOnly' | 'GpuKernel' {
  const backend = normalizeText(value)
  if (backend === 'CpuOnly') return 'CpuOnly'
  if (backend === 'GpuOnly') return 'GpuKernel'
  if (backend === 'GpuHybridKernel') return 'GpuKernel'
  if (backend === 'GpuKernel') return 'GpuKernel'
  if (backend === 'Auto') return 'Auto'
  return 'CpuOnly'
}

function isDoubleMaOverlayKind(kind: string): boolean {
  return kind === 'DoubleMa'
    || kind === 'Pit'
    || kind === 'Set'
    || kind === 'Mts'
    || kind === 'Orc'
    || kind === 'Marrt'
    || kind === 'Vfct'
    || kind === 'CompositeTrend'
    || kind === 'RegimeHybrid'
    || kind === 'BreakoutMomentum'
}

function resolveDoubleMaBackendPayload(
  kind: string,
  backend: 'Auto' | 'CpuOnly' | 'GpuKernel',
  strategy: Record<string, unknown>,
): Record<string, unknown> {
  if (backend === 'CpuOnly') return { backend: 'CpuOnly' }

  const useHybrid =
    kind === 'RegimeHybrid'
    || (kind === 'CompositeTrend' && !!strategy.trend_quality_use_adx)
    || (kind === 'BreakoutMomentum' && !!strategy.breakout_use_adx)

  return {
    backend: useHybrid ? 'GpuHybridKernel' : 'GpuKernel',
    device_id: 0,
  }
}

function normalizeMode(value: unknown): 'Grid' {
  const mode = normalizeText(value)
  if (
    mode === 'CoarseToFine'
    || mode === 'GpuProgressive'
    || mode === 'GpuProgressiveSearch'
    || mode === 'GpuVdps'
    || mode === 'Auto'
  ) {
    return 'Grid'
  }
  return 'Grid'
}

function normalizeCompositeSignalMode(value: unknown): 'hard_gate' | 'entry_core_exit_overlay' | 'trend_pullback_reversion' {
  const mode = normalizeText(value).toLowerCase()
  if (mode === 'entry_core_exit_overlay') return 'entry_core_exit_overlay'
  if (mode === 'trend_pullback_reversion') return 'trend_pullback_reversion'
  return 'hard_gate'
}

interface MovingAverageSupportEntry {
  supports_cpu?: unknown
  supports_cuda?: unknown
  supports_cuda_kernel?: unknown
}

function normalizeMovingAverageSupportMap(raw: unknown): Record<string, MovingAverageSupportEntry> {
  if (!raw || typeof raw !== 'object') return {}
  const out: Record<string, MovingAverageSupportEntry> = {}
  for (const [key, value] of Object.entries(raw as Record<string, unknown>)) {
    const id = normalizeText(key)
    if (!id || !value || typeof value !== 'object') continue
    out[id] = value as MovingAverageSupportEntry
  }
  return out
}

function hasMovingAverageSupportData(map: Record<string, MovingAverageSupportEntry>): boolean {
  return Object.keys(map).length > 0
}

function maSupportsCpu(map: Record<string, MovingAverageSupportEntry>, id: string): boolean {
  const row = map[id]
  return !!row?.supports_cpu
}

function maSupportsCuda(map: Record<string, MovingAverageSupportEntry>, id: string): boolean {
  const row = map[id]
  return !!row?.supports_cuda
}

function maSupportsKernel(map: Record<string, MovingAverageSupportEntry>, id: string): boolean {
  const row = map[id]
  return !!row?.supports_cuda_kernel
}

function buildStrategyConfigFromInputs(inputs: StrategyConfigBuildInputs, kindRaw: unknown): Record<string, unknown> {
  const kind = normalizeStrategyKindRaw(kindRaw)
  const feeBps = Math.max(0, num(inputs.feeBps, 0))
  const spreadBps = Math.max(0, num(inputs.spreadBps, 0))
  const slippageBps = Math.max(0, num(inputs.slippageBps, 0))
  const startingEquity = normalizeStartingEquity(inputs.startingEquity)
  const commission = (feeBps + slippageBps + (spreadBps / 2)) / 10_000
  const epsRel = num(inputs.epsRel, 0)
  const stopLossPct = parseRiskPct(inputs.stopLossPct)
  const takeProfitPct = parseRiskPct(inputs.takeProfitPct)
  const cooldownBars = Math.max(0, Math.floor(num(inputs.cooldownBars, 0)))
  const signalPersistenceBars = Math.max(1, Math.floor(num(inputs.signalPersistenceBars, 1)))
  const exitMode = normalizeText(inputs.exitMode || 'cross') || 'cross'
  const kaufmanStopPeriod = Math.max(1, Math.floor(num(inputs.kaufmanStopPeriod, 22)))
  const kaufmanStopMult = Math.max(0.0001, num(inputs.kaufmanStopMult, 2.0))
  const kaufmanStopMaType = normalizeText(inputs.kaufmanStopMaType || 'sma') || 'sma'

  const trendQualityEnable = kind === 'CompositeTrend' || kind === 'RegimeHybrid'
  const trendQualityLinearregPeriod = Math.max(1, Math.floor(num(inputs.trendLrPeriod, 34)))
  const trendQualityAngleMinDeg = Math.max(0.0001, num(inputs.trendLrMinDeg, 2.0))
  const trendQualityBbwPeriod = Math.max(1, Math.floor(num(inputs.trendBbwPeriod, 20)))
  const trendQualityBbwMult = Math.max(0.0001, num(inputs.trendBbwMult, 2.0))
  const trendQualityBbwMin = Math.max(0, num(inputs.trendBbwMin, 0.04))
  const trendQualityRocPeriod = Math.max(1, Math.floor(num(inputs.trendRocPeriod, 9)))
  const trendQualityUseRoc = normalizeBoolean(inputs.trendUseRoc)
  const trendQualityUseAdx = kind === 'CompositeTrend' && normalizeBoolean(inputs.trendUseAdx)
  const trendQualityAdxPeriod = Math.max(1, Math.floor(num(inputs.trendAdxPeriod, 14)))
  const trendQualityAdxMin = Math.max(0.0001, num(inputs.trendAdxMin, 20.0))

  const regimeHybridEnable = kind === 'RegimeHybrid'
  const regimeHybridAdxPeriod = Math.max(1, Math.floor(num(inputs.regimeAdxPeriod, 14)))
  const regimeHybridAdxEnter = Math.max(0.0001, num(inputs.regimeAdxEnter, 22.0))
  const regimeHybridAdxExit = Math.max(0.0001, num(inputs.regimeAdxExit, 18.0))
  const regimeHybridReversionEntryRel = Math.max(0.000001, num(inputs.regimeRevEntryRel, 0.0035))
  const regimeHybridReversionExitRel = Math.max(0, num(inputs.regimeRevExitRel, 0.0012))

  const breakoutMomentumEnable = kind === 'BreakoutMomentum'
  const breakoutBbandsPeriod = Math.max(1, Math.floor(num(inputs.breakoutBbPeriod, 20)))
  const breakoutBbandsMult = Math.max(0.0001, num(inputs.breakoutBbMult, 2.0))
  const breakoutConfirmBars = Math.max(1, Math.floor(num(inputs.breakoutConfirmBars, 1)))
  const breakoutMacdFastPeriod = Math.max(1, Math.floor(num(inputs.breakoutMacdFast, 12)))
  const breakoutMacdSlowPeriod = Math.max(1, Math.floor(num(inputs.breakoutMacdSlow, 26)))
  const breakoutMacdSignalPeriod = Math.max(1, Math.floor(num(inputs.breakoutMacdSignal, 9)))
  const breakoutCmoPeriod = Math.max(1, Math.floor(num(inputs.breakoutCmoPeriod, 14)))
  const breakoutCmoMinAbs = Math.max(0, num(inputs.breakoutCmoMinAbs, 20.0))
  const breakoutAdoscShortPeriod = Math.max(1, Math.floor(num(inputs.breakoutAdoscShort, 3)))
  const breakoutAdoscLongPeriod = Math.max(1, Math.floor(num(inputs.breakoutAdoscLong, 10)))
  const breakoutUseAdx = kind === 'BreakoutMomentum' && normalizeBoolean(inputs.breakoutUseAdx)
  const breakoutAdxPeriod = Math.max(1, Math.floor(num(inputs.breakoutAdxPeriod, 14)))
  const breakoutAdxMin = Math.max(0.0001, num(inputs.breakoutAdxMin, 20.0))

  const pressureImpulseTrendEnable = kind === 'Pit'
  const pitEfiPeriod = Math.max(1, Math.floor(num(inputs.pitEfiPeriod, 13)))
  const pitVoscShortPeriod = Math.max(1, Math.floor(num(inputs.pitVoscShortPeriod, 5)))
  const pitVoscLongPeriod = Math.max(pitVoscShortPeriod + 1, Math.floor(num(inputs.pitVoscLongPeriod, 20)))
  const pitCfoPeriod = Math.max(1, Math.floor(num(inputs.pitCfoPeriod, 9)))
  const pitCfoScalar = num(inputs.pitCfoScalar, 100.0)
  const pitConfirmBars = Math.max(1, Math.floor(num(inputs.pitConfirmBars, 2)))

  const volumeFlowConfirmEnable = kind === 'Vfct'
  const volumeFlowObvSmoothPeriod = Math.max(1, Math.floor(num(inputs.vfctObvSmoothPeriod, 21)))
  const volumeFlowMfiPeriod = Math.max(1, Math.floor(num(inputs.vfctMfiPeriod, 14)))
  const volumeFlowMfiLongMin = num(inputs.vfctMfiLongMin, 55.0)
  const volumeFlowMfiShortMax = num(inputs.vfctMfiShortMax, 45.0)
  const volumeFlowConfirmBars = Math.max(1, Math.floor(num(inputs.vfctConfirmBars, 1)))

  const slopeEfficiencyTrendEnable = kind === 'Set'
  const momentumTripleStackEnable = kind === 'Mts'
  const oscillatorReversionConfluenceEnable = kind === 'Orc'

  if (!Number.isFinite(commission) || commission < 0 || commission >= 1) throw new Error('total cost must be in [0, 1).')
  if (!Number.isFinite(epsRel) || epsRel < 0) throw new Error('eps_rel must be >= 0.')
  if (!Number.isFinite(cooldownBars) || cooldownBars < 0 || cooldownBars > 65535) {
    throw new Error('cooldown bars must be in [0, 65535].')
  }
  if (!Number.isFinite(signalPersistenceBars) || signalPersistenceBars < 1 || signalPersistenceBars > 65535) {
    throw new Error('signal persistence bars must be in [1, 65535].')
  }
  if (!(exitMode === 'cross' || exitMode === 'kaufman_stop')) throw new Error("exit mode must be 'cross' or 'kaufman_stop'.")
  if (!Number.isFinite(kaufmanStopMult) || kaufmanStopMult <= 0) throw new Error('kaufman-stop multiplier must be > 0.')
  if (!Number.isFinite(trendQualityAngleMinDeg) || trendQualityAngleMinDeg <= 0) throw new Error('trend min angle must be > 0.')
  if (!Number.isFinite(trendQualityBbwMult) || trendQualityBbwMult <= 0) throw new Error('trend BB width multiplier must be > 0.')
  if (!Number.isFinite(trendQualityBbwMin) || trendQualityBbwMin < 0) throw new Error('trend min BB width must be >= 0.')
  if (trendQualityUseAdx) {
    if (!Number.isFinite(trendQualityAdxPeriod) || trendQualityAdxPeriod <= 0) {
      throw new Error('trend ADX period must be > 0.')
    }
    if (!Number.isFinite(trendQualityAdxMin) || trendQualityAdxMin <= 0) {
      throw new Error('trend min ADX must be > 0.')
    }
  }
  if (regimeHybridEnable) {
    if (!Number.isFinite(regimeHybridAdxEnter) || regimeHybridAdxEnter <= 0) {
      throw new Error('regime hybrid ADX enter must be > 0.')
    }
    if (!Number.isFinite(regimeHybridAdxExit) || regimeHybridAdxExit <= 0 || regimeHybridAdxExit > regimeHybridAdxEnter) {
      throw new Error('regime hybrid ADX exit must be > 0 and <= ADX enter.')
    }
    if (!Number.isFinite(regimeHybridReversionEntryRel) || regimeHybridReversionEntryRel <= 0) {
      throw new Error('regime hybrid reversion entry must be > 0.')
    }
    if (!Number.isFinite(regimeHybridReversionExitRel)
      || regimeHybridReversionExitRel < 0
      || regimeHybridReversionExitRel > regimeHybridReversionEntryRel
    ) {
      throw new Error('regime hybrid reversion exit must be >= 0 and <= reversion entry.')
    }
  }
  if (breakoutMomentumEnable) {
    if (!Number.isFinite(breakoutBbandsMult) || breakoutBbandsMult <= 0) {
      throw new Error('breakout BB multiplier must be > 0.')
    }
    if (!Number.isFinite(breakoutCmoMinAbs) || breakoutCmoMinAbs < 0) {
      throw new Error('breakout min |CMO| must be >= 0.')
    }
    if (breakoutMacdFastPeriod >= breakoutMacdSlowPeriod) {
      throw new Error('breakout MACD requires fast period < slow period.')
    }
    if (breakoutAdoscShortPeriod >= breakoutAdoscLongPeriod) {
      throw new Error('breakout ADOSC requires short period < long period.')
    }
    if (breakoutUseAdx && (!Number.isFinite(breakoutAdxMin) || breakoutAdxMin <= 0)) {
      throw new Error('breakout ADX min must be > 0.')
    }
  }
  if (pressureImpulseTrendEnable && (!Number.isFinite(pitCfoScalar) || pitCfoScalar <= 0)) {
    throw new Error('PIT CFO scalar must be > 0.')
  }
  if (volumeFlowConfirmEnable) {
    if (!Number.isFinite(volumeFlowMfiLongMin) || volumeFlowMfiLongMin < 0 || volumeFlowMfiLongMin > 100) {
      throw new Error('VFCT MFI long min must be in [0, 100].')
    }
    if (!Number.isFinite(volumeFlowMfiShortMax) || volumeFlowMfiShortMax < 0 || volumeFlowMfiShortMax > 100) {
      throw new Error('VFCT MFI short max must be in [0, 100].')
    }
    if (volumeFlowMfiShortMax > volumeFlowMfiLongMin) {
      throw new Error('VFCT MFI short max must be <= MFI long min.')
    }
  }

  return {
    long_only: normalizeBoolean(inputs.longOnly),
    allow_flip: normalizeBoolean(inputs.allowFlip),
    trade_on_next_bar: true,
    cost_preset: normalizeText(inputs.costPreset || 'custom') || 'custom',
    starting_equity: startingEquity,
    fee_bps: feeBps,
    spread_bps: spreadBps,
    slippage_bps: slippageBps,
    commission,
    stop_loss_pct: stopLossPct,
    take_profit_pct: takeProfitPct,
    cooldown_bars: cooldownBars,
    signal_persistence_bars: signalPersistenceBars,
    exit_mode: exitMode,
    kaufman_stop_period: kaufmanStopPeriod,
    kaufman_stop_mult: kaufmanStopMult,
    kaufman_stop_ma_type: kaufmanStopMaType,
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
    eps_rel: epsRel,
  }
}

function parseOptionalRawNumber(value: unknown): number {
  const text = normalizeText(value)
  if (!text) return Number.NaN
  const parsed = Number(text)
  return Number.isFinite(parsed) ? parsed : Number.NaN
}

function defaultCompositeMoSearchSpace(): Record<string, unknown> {
  return {
    params: [
      { name: 'halftrend_amplitude', domain: { IntRange: { low: 2, high: 20, step: 1, log: false } } },
      { name: 'halftrend_channel_deviation', domain: { FloatRange: { low: 0.5, high: 5.0, log: false } } },
      { name: 'halftrend_atr_period', domain: { IntRange: { low: 20, high: 200, step: 1, log: false } } },
      { name: 'macz_fast_length', domain: { IntRange: { low: 5, high: 30, step: 1, log: false } } },
      { name: 'macz_slow_length', domain: { IntRange: { low: 10, high: 60, step: 1, log: false } } },
      { name: 'macz_signal_length', domain: { IntRange: { low: 3, high: 30, step: 1, log: false } } },
      { name: 'macz_lengthz', domain: { IntRange: { low: 5, high: 50, step: 1, log: false } } },
      { name: 'macz_length_stdev', domain: { IntRange: { low: 5, high: 80, step: 1, log: false } } },
      { name: 'macz_a', domain: { FloatRange: { low: -2.0, high: 2.0, log: false } } },
      { name: 'macz_b', domain: { FloatRange: { low: -2.0, high: 2.0, log: false } } },
      { name: 'macz_gamma', domain: { FloatRange: { low: 0.0, high: 0.2, log: false } } },
      { name: 'macz_use_lag', domain: 'Bool' },
      { name: 'macz_use_volume', domain: 'Bool' },
      { name: 'ott_period', domain: { IntRange: { low: 1, high: 50, step: 1, log: false } } },
      { name: 'ott_percent', domain: { FloatRange: { low: 0.1, high: 5.0, log: false } } },
      {
        name: 'ott_ma_type',
        domain: { Categorical: { choices: ['VAR', 'EMA', 'SMA', 'WMA', 'TMA', 'WWMA', 'ZLEMA', 'TSF'] } },
      },
      { name: 'strategy_commission', domain: { FloatRange: { low: 0.0, high: 0.001, log: false } } },
      { name: 'strategy_eps_rel', domain: { FloatRange: { low: 0.0, high: 0.005, log: false } } },
      { name: 'strategy_long_only', domain: 'Bool' },
      { name: 'strategy_allow_flip', domain: 'Bool' },
    ],
  }
}

function parseCompositeMoSearchSpace(raw: unknown): Record<string, unknown> {
  const text = normalizeText(raw)
  if (!text) return defaultCompositeMoSearchSpace()
  let obj: unknown = null
  try {
    obj = JSON.parse(text)
  } catch (error) {
    throw new Error(`Invalid Composite MO search space JSON: ${String(error)}`)
  }
  if (!obj || typeof obj !== 'object') throw new Error('Composite MO search space JSON must be an object.')
  const row = obj as Record<string, unknown>
  if (!Array.isArray(row.params)) throw new Error('Composite MO search space JSON must contain a params array.')
  return row
}

function normalizeCompositeMoSearchSpace(
  space: unknown,
  fastDomain: Record<string, unknown>,
  slowDomain: Record<string, unknown>,
): Record<string, unknown> {
  const paramsIn = Array.isArray((space as Record<string, unknown> | null)?.params)
    ? ((space as Record<string, unknown>).params as unknown[])
    : []
  const params: Array<{ name: string; domain: unknown }> = []
  const seen = new Set<string>()
  for (const entry of paramsIn) {
    if (!entry || typeof entry !== 'object') continue
    const row = entry as Record<string, unknown>
    const name = normalizeText(row.name)
    if (!name) continue
    const key = name.toLowerCase()
    if (key === 'fast_len' || key === 'slow_len') continue
    if (seen.has(key)) continue
    if (row.domain == null) continue
    params.push({ name, domain: row.domain })
    seen.add(key)
  }
  return {
    params: [
      { name: 'fast_len', domain: fastDomain },
      { name: 'slow_len', domain: slowDomain },
      ...params,
    ],
  }
}

function parseCompositeMoPrescanMaPairs(raw: unknown): Array<{ fast_ma_type: string; slow_ma_type: string }> | null {
  const text = String(raw ?? '')
  const lines = text.split(/\r?\n/)
  const out: Array<{ fast_ma_type: string; slow_ma_type: string }> = []
  for (const line of lines) {
    const row = normalizeText(line)
    if (!row || row.startsWith('#')) continue
    const parts = row.split(',').map((item) => normalizeText(item)).filter((item) => !!item)
    if (parts.length !== 2) {
      throw new Error(`Invalid MA pair line "${line}". Use "fast,slow" (one pair per line).`)
    }
    const fastMaType = parts[0] || ''
    const slowMaType = parts[1] || ''
    out.push({ fast_ma_type: fastMaType, slow_ma_type: slowMaType })
  }
  return out.length ? out : null
}

function parseMoRangeInt(low: unknown, high: unknown, step: unknown, label: string): Record<string, unknown> {
  const lowInt = Math.floor(num(low, Number.NaN))
  const highInt = Math.floor(num(high, Number.NaN))
  const stepInt = Math.floor(num(step, Number.NaN))
  if (!Number.isFinite(lowInt) || !Number.isFinite(highInt) || !Number.isFinite(stepInt)) {
    throw new Error(`${label}: low/high/step must be finite integers.`)
  }
  if (stepInt < 1) throw new Error(`${label}: step must be >= 1.`)
  if (lowInt > highInt) throw new Error(`${label}: low must be <= high.`)
  return { IntRange: { low: lowInt, high: highInt, step: stepInt, log: false } }
}

function parseMoRangeFloat(low: unknown, high: unknown, step: unknown, label: string): Record<string, unknown> {
  const lowNum = num(low, Number.NaN)
  const highNum = num(high, Number.NaN)
  const stepNum = num(step, Number.NaN)
  if (!Number.isFinite(lowNum) || !Number.isFinite(highNum) || !Number.isFinite(stepNum)) {
    throw new Error(`${label}: low/high/step must be finite numbers.`)
  }
  if (stepNum <= 0) throw new Error(`${label}: step must be > 0.`)
  if (lowNum > highNum) throw new Error(`${label}: low must be <= high.`)
  if (lowNum === highNum) return { Categorical: { choices: [lowNum] } }

  const count = Math.floor((highNum - lowNum) / stepNum) + 1
  if (Number.isFinite(count) && count >= 2 && count <= 400) {
    const choices: number[] = []
    for (let idx = 0; idx < count; idx += 1) {
      const value = lowNum + (idx * stepNum)
      if (value > highNum + (stepNum * 1e-9)) break
      choices.push(Number(value.toFixed(10)))
    }
    const last = choices.length ? choices[choices.length - 1] : undefined
    if (choices.length >= 2 && typeof last === 'number' && Number.isFinite(last) && Math.abs(last - highNum) > stepNum * 0.55) {
      choices.push(Number(highNum.toFixed(10)))
    }
    if (choices.length >= 2) return { Categorical: { choices } }
  }
  return { FloatRange: { low: lowNum, high: highNum, log: false } }
}

function parseMoBoolDomain(mode: unknown): 'Bool' | { Categorical: { choices: boolean[] } } {
  const normalized = normalizeText(mode).toLowerCase() || 'both'
  if (normalized === 'true') return { Categorical: { choices: [true] } }
  if (normalized === 'false') return { Categorical: { choices: [false] } }
  return 'Bool'
}

function parseMoOttMaTypeDomain(raw: unknown): Record<string, unknown> {
  const choices = normalizeText(raw)
    .split(/[\s,]+/g)
    .map((value) => normalizeText(value).toUpperCase())
    .filter((value) => !!value)
  const dedup: string[] = []
  const seen = new Set<string>()
  for (const choice of choices) {
    if (seen.has(choice)) continue
    seen.add(choice)
    dedup.push(choice)
  }
  if (!dedup.length) {
    throw new Error('OTT MA types: enter at least one value (comma-separated).')
  }
  return { Categorical: { choices: dedup } }
}

function buildCompositeMoSearchSpaceFromInputs(inputs: CompositeMoOptimizationBuildInputs): Record<string, unknown> {
  return {
    params: [
      { name: 'halftrend_amplitude', domain: parseMoRangeInt(inputs.moHtAmpLow, inputs.moHtAmpHigh, inputs.moHtAmpStep, 'HalfTrend amplitude') },
      { name: 'halftrend_channel_deviation', domain: parseMoRangeFloat(inputs.moHtDevLow, inputs.moHtDevHigh, inputs.moHtDevStep, 'HalfTrend channel deviation') },
      { name: 'halftrend_atr_period', domain: parseMoRangeInt(inputs.moHtAtrLow, inputs.moHtAtrHigh, inputs.moHtAtrStep, 'HalfTrend ATR period') },
      { name: 'macz_fast_length', domain: parseMoRangeInt(inputs.moMaczFastLow, inputs.moMaczFastHigh, inputs.moMaczFastStep, 'MACZ fast length') },
      { name: 'macz_slow_length', domain: parseMoRangeInt(inputs.moMaczSlowLow, inputs.moMaczSlowHigh, inputs.moMaczSlowStep, 'MACZ slow length') },
      { name: 'macz_signal_length', domain: parseMoRangeInt(inputs.moMaczSignalLow, inputs.moMaczSignalHigh, inputs.moMaczSignalStep, 'MACZ signal length') },
      { name: 'macz_lengthz', domain: parseMoRangeInt(inputs.moMaczLengthZLow, inputs.moMaczLengthZHigh, inputs.moMaczLengthZStep, 'MACZ lengthz') },
      { name: 'macz_length_stdev', domain: parseMoRangeInt(inputs.moMaczLengthStdevLow, inputs.moMaczLengthStdevHigh, inputs.moMaczLengthStdevStep, 'MACZ length stdev') },
      { name: 'macz_gamma', domain: parseMoRangeFloat(inputs.moMaczGammaLow, inputs.moMaczGammaHigh, inputs.moMaczGammaStep, 'MACZ gamma') },
      { name: 'macz_a', domain: parseMoRangeFloat(inputs.moMaczALow, inputs.moMaczAHigh, inputs.moMaczAStep, 'MACZ a') },
      { name: 'macz_b', domain: parseMoRangeFloat(inputs.moMaczBLow, inputs.moMaczBHigh, inputs.moMaczBStep, 'MACZ b') },
      { name: 'macz_use_lag', domain: parseMoBoolDomain(inputs.moMaczUseLagMode) },
      { name: 'macz_use_volume', domain: parseMoBoolDomain(inputs.moMaczUseVolumeMode) },
      { name: 'ott_period', domain: parseMoRangeInt(inputs.moOttPeriodLow, inputs.moOttPeriodHigh, inputs.moOttPeriodStep, 'OTT period') },
      { name: 'ott_percent', domain: parseMoRangeFloat(inputs.moOttPercentLow, inputs.moOttPercentHigh, inputs.moOttPercentStep, 'OTT percent') },
      { name: 'ott_ma_type', domain: parseMoOttMaTypeDomain(inputs.moOttMaTypes) },
      { name: 'strategy_commission', domain: { FloatRange: { low: 0.0, high: 0.001, log: false } } },
      { name: 'strategy_eps_rel', domain: { FloatRange: { low: 0.0, high: 0.005, log: false } } },
      { name: 'strategy_long_only', domain: 'Bool' },
      { name: 'strategy_allow_flip', domain: 'Bool' },
    ],
  }
}

function buildCompositeMoSampler(inputs: CompositeMoOptimizationBuildInputs): Record<string, unknown> {
  const samplerKind = normalizeText(inputs.moSampler || 'Nsga2') || 'Nsga2'
  if (samplerKind === 'Random') {
    const seed = Math.max(0, Math.floor(num(inputs.moRandomSeed, 1)))
    const trials = Math.max(1, Math.floor(num(inputs.moRandomTrials, 2000)))
    if (trials > 200_000) throw new Error('Random trials too large (max 200000).')
    return { sampler: 'Random', config: { seed, trials } }
  }
  if (samplerKind === 'Tpe') {
    const seed = Math.max(0, Math.floor(num(inputs.moTpeSeed, 1)))
    const trials = Math.max(1, Math.floor(num(inputs.moTpeTrials, 5000)))
    const startupTrials = Math.max(0, Math.floor(num(inputs.moTpeStartupTrials, 10)))
    const eiCandidates = Math.max(1, Math.floor(num(inputs.moTpeCandidates, 24)))
    const gamma = num(inputs.moTpeGamma, 0.25)
    const batchSize = Math.max(1, Math.floor(num(inputs.moTpeBatchSize, 64)))
    const maxModelPoints = Math.max(32, Math.floor(num(inputs.moTpeMaxModelPoints, 2048)))
    if (trials > 200_000) throw new Error('TPE trials too large (max 200000).')
    if (!Number.isFinite(gamma) || gamma <= 0 || gamma >= 1) throw new Error('TPE gamma must be in (0, 1).')
    if (eiCandidates > 512) throw new Error('TPE EI candidates too large (max 512).')
    return {
      sampler: 'Tpe',
      config: {
        seed,
        trials,
        startup_trials: startupTrials,
        n_ei_candidates: eiCandidates,
        gamma,
        batch_size: batchSize,
        max_model_points: maxModelPoints,
      },
    }
  }
  if (samplerKind === 'Qmc') {
    const seed = Math.max(0, Math.floor(num(inputs.moQmcSeed, 1)))
    const trials = Math.max(1, Math.floor(num(inputs.moQmcTrials, 5000)))
    const sequence = normalizeText(inputs.moQmcSequence || 'sobol').toLowerCase() || 'sobol'
    if (trials > 200_000) throw new Error('QMC trials too large (max 200000).')
    return { sampler: 'Qmc', config: { seed, trials, sequence } }
  }
  const seed = Math.max(0, Math.floor(num(inputs.moNsga2Seed, 1)))
  const populationSize = Math.max(2, Math.floor(num(inputs.moNsga2Population, 80)))
  const generations = Math.max(1, Math.floor(num(inputs.moNsga2Generations, 40)))
  const crossoverProb = num(inputs.moNsga2CrossoverProb, 0.9)
  const swappingProb = num(inputs.moNsga2SwappingProb, 0.5)
  const mutRaw = normalizeText(inputs.moNsga2MutationProb)
  const mutationProb = mutRaw ? num(inputs.moNsga2MutationProb, Number.NaN) : null
  if (!Number.isFinite(crossoverProb) || crossoverProb < 0 || crossoverProb > 1) {
    throw new Error('NSGA-II crossover_prob must be in [0, 1].')
  }
  if (!Number.isFinite(swappingProb) || swappingProb < 0 || swappingProb > 1) {
    throw new Error('NSGA-II swapping_prob must be in [0, 1].')
  }
  if (mutationProb != null && (!Number.isFinite(mutationProb) || mutationProb < 0 || mutationProb > 1)) {
    throw new Error('NSGA-II mutation_prob must be blank or in [0, 1].')
  }
  const evals = populationSize * (generations + 1)
  if (evals > 200_000) throw new Error(`NSGA-II budget too large (${evals} evals; max 200000).`)
  return {
    sampler: 'Nsga2',
    config: {
      seed,
      population_size: populationSize,
      generations,
      crossover_prob: crossoverProb,
      swapping_prob: swappingProb,
      mutation_prob: mutationProb == null ? null : mutationProb,
    },
  }
}

export function buildDoubleMaOptimizationRequest(inputs: DoubleMaOptimizationBuildInputs): Record<string, unknown> | null {
  const kind = normalizeStrategyKindRaw(inputs.context?.strategy_kind)
  if (!isDoubleMaOverlayKind(kind)) return null

  const dataId = normalizeText(inputs.context?.data_id)
  if (!dataId) throw new Error('Load CSV first (missing data id).')

  const fast: [number, number, number] = [
    Number(inputs.fastStart),
    Number(inputs.fastEnd),
    Number(inputs.fastStep),
  ]
  const slow: [number, number, number] = [
    Number(inputs.slowStart),
    Number(inputs.slowEnd),
    Number(inputs.slowStep),
  ]
  const fastPeriods = expandRangeU16Like(fast[0], fast[1], fast[2])
  const slowPeriods = expandRangeU16Like(slow[0], slow[1], slow[2])
  const pairs = countPairsFastLtSlow(fastPeriods, slowPeriods)
  if (!pairs) throw new Error('No valid parameter combinations (fast must be < slow).')

  const fastMaType = normalizeText(inputs.fastMaType)
  const slowMaType = normalizeText(inputs.slowMaType)
  if (!fastMaType) throw new Error('Select a fast MA type.')
  if (!slowMaType) throw new Error('Select a slow MA type.')
  if (kind === 'Marrt') {
    const fastAllowed = MARRT_MA_WHITELIST.has(fastMaType.toLowerCase())
    const slowAllowed = MARRT_MA_WHITELIST.has(slowMaType.toLowerCase())
    if (!fastAllowed || !slowAllowed) {
      throw new Error('MARRT currently supports only sma, ema, alma, wma, zlema, and vwma.')
    }
  }
  if (kind === 'Vfct') {
    const fastAllowed = VFCT_MA_WHITELIST.has(fastMaType.toLowerCase())
    const slowAllowed = VFCT_MA_WHITELIST.has(slowMaType.toLowerCase())
    if (!fastAllowed || !slowAllowed) {
      throw new Error('VFCT currently supports only vwma fast/slow trend lines.')
    }
  }

  const maSource = normalizeText(inputs.maSource || 'close') || 'close'
  validateCandleFieldRequirements(inputs.context?.current_data_meta ?? null, fastMaType, slowMaType, maSource)
  if (kind === 'Set') {
    const meta = inputs.context?.current_data_meta
    const record = meta && typeof meta === 'object' ? meta as Record<string, unknown> : null
    if (!record?.has_high || !record?.has_low) {
      throw new Error('SET requires high and low candle fields.')
    }
  }
  if (kind === 'Pit') {
    const meta = inputs.context?.current_data_meta
    const record = meta && typeof meta === 'object' ? meta as Record<string, unknown> : null
    if (!record?.has_open || !record?.has_high || !record?.has_low || !record?.has_volume) {
      throw new Error('PIT requires open, high, low, and volume candle fields.')
    }
  }
  if (kind === 'Vfct') {
    const meta = inputs.context?.current_data_meta
    const record = meta && typeof meta === 'object' ? meta as Record<string, unknown> : null
    if (!record?.has_high || !record?.has_low || !record?.has_volume) {
      throw new Error('VFCT requires high, low, and volume candle fields.')
    }
  }
  if (kind === 'CompositeTrend') {
    const meta = inputs.context?.current_data_meta
    const record = meta && typeof meta === 'object' ? meta as Record<string, unknown> : null
    const trendUseAdx = normalizeBoolean(inputs.trendUseAdx)
    if (trendUseAdx && (!record?.has_high || !record?.has_low)) {
      throw new Error('CompositeTrend with ADX enabled requires high and low candle fields.')
    }
  }
  if (kind === 'RegimeHybrid') {
    const meta = inputs.context?.current_data_meta
    const record = meta && typeof meta === 'object' ? meta as Record<string, unknown> : null
    if (!record?.has_high || !record?.has_low) {
      throw new Error('RegimeHybrid requires high and low candle fields.')
    }
  }
  if (kind === 'BreakoutMomentum') {
    const meta = inputs.context?.current_data_meta
    const record = meta && typeof meta === 'object' ? meta as Record<string, unknown> : null
    if (!record?.has_high || !record?.has_low || !record?.has_volume) {
      throw new Error('BreakoutMomentum requires high, low, and volume candle fields.')
    }
  }
  const objective = normalizeText(inputs.objective || 'Sharpe') || 'Sharpe'
  const mode = normalizeMode(inputs.mode)
  const backend = normalizeBackend(inputs.backend)
  const maSupportMap = normalizeMovingAverageSupportMap(inputs.maSupportMap)
  const hasMaSupportMap = hasMovingAverageSupportData(maSupportMap)
  if (hasMaSupportMap) {
    if (backend === 'CpuOnly') {
      if (!maSupportsCpu(maSupportMap, fastMaType) || !maSupportsCpu(maSupportMap, slowMaType)) {
        throw new Error('CPU backend does not support one of the selected MA types. Choose a supported MA or switch backend.')
      }
    } else if (backend === 'GpuKernel') {
      const okFast = maSupportsCpu(maSupportMap, fastMaType)
        || maSupportsCuda(maSupportMap, fastMaType)
        || maSupportsKernel(maSupportMap, fastMaType)
      const okSlow = maSupportsCpu(maSupportMap, slowMaType)
        || maSupportsCuda(maSupportMap, slowMaType)
        || maSupportsKernel(maSupportMap, slowMaType)
      if (!okFast || !okSlow) {
        throw new Error('GPU unified backend cannot run one of the selected MA types. Choose a supported MA or switch backend.')
      }
    } else {
      const okFast = maSupportsCpu(maSupportMap, fastMaType)
        || maSupportsCuda(maSupportMap, fastMaType)
        || maSupportsKernel(maSupportMap, fastMaType)
      const okSlow = maSupportsCpu(maSupportMap, slowMaType)
        || maSupportsCuda(maSupportMap, slowMaType)
        || maSupportsKernel(maSupportMap, slowMaType)
      if (!okFast || !okSlow) {
        throw new Error('Auto backend cannot run the selected MA(s). Choose a period-based, single-output MA that supports CPU or CUDA.')
      }
    }
  }
  const topK = Math.max(0, Math.floor(num(inputs.topK, 0)))
  const includeAll = normalizeIncludeAll(inputs.includeAll)
  const heatmapBins = Math.max(0, Math.floor(num(inputs.heatmapBins, 0)))
  if (heatmapBins > 512) throw new Error('heatmapBins must be <= 512.')
  const exportAllCsvPath = normalizeText(inputs.exportAllCsvPath)

  const feeBps = Math.max(0, num(inputs.feeBps, 0))
  const spreadBps = Math.max(0, num(inputs.spreadBps, 0))
  const slippageBps = Math.max(0, num(inputs.slippageBps, 0))
  const perSideBps = feeBps + slippageBps + (spreadBps / 2)
  const commission = perSideBps / 10_000
  if (!Number.isFinite(commission) || commission < 0 || commission >= 1) {
    throw new Error('total cost must be in [0, 1).')
  }
  const startingEquity = normalizeStartingEquity(inputs.startingEquity)
  const cooldownBars = Math.max(0, Math.floor(num(inputs.cooldownBars, 0)))
  if (!Number.isFinite(cooldownBars) || cooldownBars < 0 || cooldownBars > 65535) {
    throw new Error('cooldown bars must be in [0, 65535].')
  }
  const signalPersistenceBars = Math.max(1, Math.floor(num(inputs.signalPersistenceBars, 1)))
  if (!Number.isFinite(signalPersistenceBars) || signalPersistenceBars < 1 || signalPersistenceBars > 65535) {
    throw new Error('signal persistence bars must be in [1, 65535].')
  }
  const exitMode = normalizeText(inputs.exitMode || 'cross') || 'cross'
  if (!(exitMode === 'cross' || exitMode === 'kaufman_stop')) {
    throw new Error("exit mode must be 'cross' or 'kaufman_stop'.")
  }
  const kaufmanStopMult = Math.max(0.0001, num(inputs.kaufmanStopMult, 2.0))
  if (!Number.isFinite(kaufmanStopMult) || kaufmanStopMult <= 0) {
    throw new Error('kaufman-stop multiplier must be > 0.')
  }

  const fastMaParams = normalizeMaParams(inputs.fastMaParams)
  const slowMaParams = normalizeMaParams(inputs.slowMaParams)
  const fastMaParamSweeps = normalizeMaParamSweeps(inputs.fastMaParamSweeps)
  const slowMaParamSweeps = normalizeMaParamSweeps(inputs.slowMaParamSweeps)
  const fastVariantCount = maSweepVariantCount(fastMaParamSweeps)
  const slowVariantCount = maSweepVariantCount(slowMaParamSweeps)
  const combos = pairs * fastVariantCount * slowVariantCount
  if (includeAll && combos > 2_000_000) {
    throw new Error(`includeAll=true would return ${combos} rows; disable it or reduce ranges/sweeps.`)
  }

  const hasExecutionOverlays = cooldownBars > 0 || signalPersistenceBars > 1

  if (backend === 'GpuKernel') {
    if (exitMode !== 'cross') {
      throw new Error('GPU (kernel) currently supports exit mode = cross only.')
    }
    if (hasExecutionOverlays) {
      throw new Error('GPU (kernel) does not support cooldown/persistence overlays.')
    }
  }

  if (exportAllCsvPath) {
    if (includeAll) throw new Error('exportAllCsvPath requires includeAll=false (stream export avoids huge RAM usage).')
    if (mode !== 'Grid') throw new Error('exportAllCsvPath requires mode=Grid.')
    if (backend !== 'GpuKernel') {
      throw new Error('exportAllCsvPath is currently supported only for the GPU unified backend.')
    }
    if ((fastVariantCount * slowVariantCount) > 1) {
      throw new Error('exportAllCsvPath is not supported when optimizing MA parameters. Disable MA param sweeps or export.')
    }
  }

  let vdps: Record<string, unknown> | null = null
  void hasExecutionOverlays

  const minTrades = parseOptionalNonNegativeInteger(inputs.dmConstraintMinTrades)
  const maxDrawdown = parseOptionalNonNegativeNumber(inputs.dmConstraintMaxDd, 'Max drawdown constraint')
  let constraints: Record<string, unknown> | null = null
  if (minTrades != null || maxDrawdown != null) {
    constraints = {}
    if (minTrades != null) constraints.min_trades = minTrades
    if (maxDrawdown != null) constraints.max_drawdown = maxDrawdown
  }

  const strategy = buildStrategyConfigFromInputs(inputs, kind)
  const backendPayload = resolveDoubleMaBackendPayload(kind, backend, strategy)

  return {
    strategy_kind: kind,
    backend: backendPayload,
    data_id: dataId,
    starting_equity: startingEquity,
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
    mode,
    vdps,
    top_k: topK,
    include_all: includeAll,
    heatmap_bins: heatmapBins,
    constraints,
  }
}

export function buildCompositeCpuOptimizationRequest(inputs: CompositeCpuOptimizationBuildInputs): Record<string, unknown> | null {
  const kind = normalizeStrategyKindRaw(inputs.context?.strategy_kind)
  if (
    kind !== 'Asr'
    && kind !== 'Vcc'
    && kind !== 'Vemr'
    && kind !== 'Rocb'
    && kind !== 'Mkcs'
    && kind !== 'CompositeCpu'
  ) {
    return null
  }

  const dataId = normalizeText(inputs.context?.data_id)
  if (!dataId) throw new Error('Load CSV first (missing data id).')

  const meta = inputs.context?.current_data_meta
  if (!meta || typeof meta !== 'object') throw new Error('Load CSV first.')
  const dataMeta = meta as Record<string, unknown>
  if (!dataMeta.has_high || !dataMeta.has_low) {
    throw new Error('Composite strategy requires high/low candle fields. Load a CSV with high+low columns.')
  }

  const fast: [number, number, number] = [
    Number(inputs.fastStart),
    Number(inputs.fastEnd),
    Number(inputs.fastStep),
  ]
  const slow: [number, number, number] = [
    Number(inputs.slowStart),
    Number(inputs.slowEnd),
    Number(inputs.slowStep),
  ]
  const fastPeriods = expandRangeU16Like(fast[0], fast[1], fast[2])
  const slowPeriods = expandRangeU16Like(slow[0], slow[1], slow[2])
  const pairs = countPairsFastLtSlow(fastPeriods, slowPeriods)
  if (!pairs) throw new Error('No valid parameter combinations (fast must be < slow).')

  const fastMaType = normalizeText(inputs.fastMaType)
  const slowMaType = normalizeText(inputs.slowMaType)
  if (!fastMaType) throw new Error('Select a fast MA type.')
  if (!slowMaType) throw new Error('Select a slow MA type.')
  const maSupportMap = normalizeMovingAverageSupportMap(inputs.maSupportMap)
  if (kind !== 'Asr' && kind !== 'Vcc' && kind !== 'Vemr' && kind !== 'Rocb' && kind !== 'Mkcs' && hasMovingAverageSupportData(maSupportMap)) {
    if (!maSupportsCpu(maSupportMap, fastMaType) || !maSupportsCpu(maSupportMap, slowMaType)) {
      throw new Error('Composite strategy is CPU-only and does not support one of the selected MA types.')
    }
  }

  const maSource = normalizeText(inputs.maSource || 'close') || 'close'
  validateCandleFieldRequirements(inputs.context?.current_data_meta ?? null, fastMaType, slowMaType, maSource)

  const amp = Math.floor(num(inputs.halfTrendAmplitude, 2))
  const atrP = Math.floor(num(inputs.halfTrendAtrPeriod, 100))
  const dev = num(inputs.halfTrendChannelDev, 2.0)
  if (!Number.isFinite(amp) || amp < 1 || amp > 65535) throw new Error('HalfTrend amplitude must be in [1, 65535].')
  if (!Number.isFinite(atrP) || atrP < 1 || atrP > 65535) throw new Error('HalfTrend ATR period must be in [1, 65535].')
  if (!Number.isFinite(dev) || dev < 0) throw new Error('HalfTrend channel deviation must be finite and >= 0.')
  const halftrend = { amplitude: amp, atr_period: atrP, channel_deviation: dev }

  const maczFast = Math.floor(num(inputs.maczFastLength, 12))
  const maczSlow = Math.floor(num(inputs.maczSlowLength, 25))
  const maczSig = Math.floor(num(inputs.maczSignalLength, 9))
  const maczLz = Math.floor(num(inputs.maczLengthZ, 20))
  const maczLsd = Math.floor(num(inputs.maczLengthStdev, 25))
  const maczGamma = num(inputs.maczGamma, 0.02)
  const maczA = num(inputs.maczA, 1.0)
  const maczB = num(inputs.maczB, 1.0)
  const maczUseVolume = normalizeBoolean(inputs.maczUseVolume)
  const maczUseLag = normalizeBoolean(inputs.maczUseLag)
  if (!Number.isFinite(maczFast) || maczFast < 1) throw new Error('MACZ fast_length must be >= 1.')
  if (!Number.isFinite(maczSlow) || maczSlow < 1) throw new Error('MACZ slow_length must be >= 1.')
  if (maczFast >= maczSlow) throw new Error('MACZ requires fast_length < slow_length.')
  if (!Number.isFinite(maczSig) || maczSig < 1) throw new Error('MACZ signal_length must be >= 1.')
  if (!Number.isFinite(maczLz) || maczLz < 1) throw new Error('MACZ lengthz must be >= 1.')
  if (!Number.isFinite(maczLsd) || maczLsd < 1) throw new Error('MACZ length_stdev must be >= 1.')
  if (!Number.isFinite(maczA) || maczA < -2 || maczA > 2) throw new Error('MACZ a must be in [-2, 2].')
  if (!Number.isFinite(maczB) || maczB < -2 || maczB > 2) throw new Error('MACZ b must be in [-2, 2].')
  if (!Number.isFinite(maczGamma) || maczGamma < 0 || maczGamma >= 1) throw new Error('MACZ gamma must be in [0, 1).')
  if (maczUseVolume && !dataMeta.has_volume) throw new Error('MACZ use_volume requires volume column in the CSV.')
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
  }

  const ottPeriod = Math.floor(num(inputs.ottPeriod, 2))
  const ottPercent = num(inputs.ottPercent, 1.4)
  const ottMaType = normalizeText(inputs.ottMaType || 'VAR')
  if (!Number.isFinite(ottPeriod) || ottPeriod < 1) throw new Error('OTT period must be >= 1.')
  if (!Number.isFinite(ottPercent) || ottPercent < 0) throw new Error('OTT percent must be finite and >= 0.')
  if (!ottMaType) throw new Error('OTT MA type is required.')
  const ott = { period: ottPeriod, percent: ottPercent, ma_type: ottMaType }

  const objective = normalizeText(inputs.objective || 'Sharpe') || 'Sharpe'
  const topK = Math.max(0, Math.floor(num(inputs.topK, 0)))
  const includeAll = normalizeIncludeAll(inputs.includeAll)
  if (includeAll && pairs > 2_000_000) {
    throw new Error(`includeAll=true would return ${pairs} rows; disable it or reduce ranges.`)
  }

  const startingEquity = normalizeStartingEquity(inputs.startingEquity)
  const strategy = buildStrategyConfigFromInputs(inputs, kind)
  const signalMode = normalizeCompositeSignalMode(inputs.compositeSignalMode)
  const backend = normalizeBackend(inputs.backend)
  const backendPayload = (kind === 'Asr' || kind === 'Vcc' || kind === 'Vemr' || kind === 'Rocb' || kind === 'Mkcs')
    ? (backend === 'CpuOnly' ? { backend: 'CpuOnly' } : { backend, device_id: 0 })
    : { backend: 'CpuOnly' }

  const asr = kind === 'Asr'
    ? {
        squeeze_bb_mult: Math.max(0.0001, num(inputs.asrSqueezeBbMult, 2.0)),
        squeeze_kc_mult: Math.max(0.0001, num(inputs.asrSqueezeKcMult, 1.5)),
        angle_deg_min: Math.max(0, num(inputs.asrAngleDegMin, 10.0)),
        angle_deg_mr: Math.max(0, num(inputs.asrAngleDegMr, 6.0)),
        use_er_filter: inputs.asrUseErFilter == null ? true : normalizeBoolean(inputs.asrUseErFilter),
        er_min: Math.max(0, num(inputs.asrErMin, 0.30)),
        er_mr: Math.max(0, num(inputs.asrErMr, 0.25)),
        z_ma_type: normalizeText(inputs.asrZMaType || 'sma') || 'sma',
        z_nbdev: Math.max(0.0001, num(inputs.asrZNbdev, 1.0)),
        z_devtype: Math.max(0, Math.floor(num(inputs.asrZDevtype, 0))),
        z_entry: Math.max(0, num(inputs.asrZEntry, 2.0)),
        z_exit: Math.max(0, num(inputs.asrZExit, 0.5)),
        confirm_bars: Math.max(0, Math.floor(num(inputs.asrConfirmBars, 0))),
        mom_fade_bars: Math.max(1, Math.floor(num(inputs.asrMomFadeBars, 2))),
        max_trade_bars: Math.max(1, Math.floor(num(inputs.asrMaxTradeBars, 80))),
        max_mr_trade_bars: Math.max(1, Math.floor(num(inputs.asrMaxMrTradeBars, 30))),
      }
    : undefined
  let vcc: Record<string, unknown> | undefined
  if (kind === 'Vcc') {
    const mapFastToSqueezePeriod = inputs.vccMapFastToSqueezePeriod == null
      ? true
      : normalizeBoolean(inputs.vccMapFastToSqueezePeriod)
    const mapFastToMomentumPeriod = inputs.vccMapFastToMomentumPeriod == null
      ? true
      : normalizeBoolean(inputs.vccMapFastToMomentumPeriod)
    const mapSlowToTrendPeriod = inputs.vccMapSlowToTrendPeriod == null
      ? true
      : normalizeBoolean(inputs.vccMapSlowToTrendPeriod)
    const squeezePeriod = Math.floor(num(inputs.vccSqueezePeriod, 20))
    const momentumPeriod = Math.floor(num(inputs.vccMomentumPeriod, 20))
    const momentumBbPeriod = Math.floor(num(inputs.vccMomentumBbPeriod, 20))
    const trendPeriod = Math.floor(num(inputs.vccTrendPeriod, 6))
    const squeezeBbMult = num(inputs.vccSqueezeBbMult, 2.0)
    const squeezeKcMult = num(inputs.vccSqueezeKcMult, 1.5)
    const confirmBars = Math.floor(num(inputs.vccConfirmBars, 0))
    const requireMomentumAccel = inputs.vccRequireMomentumAccel == null
      ? true
      : normalizeBoolean(inputs.vccRequireMomentumAccel)
    const maxTradeBars = Math.floor(num(inputs.vccMaxTradeBars, 120))

    if (!Number.isFinite(squeezePeriod) || squeezePeriod < 1 || squeezePeriod > 65535) {
      throw new Error('VCC squeeze period must be in [1, 65535].')
    }
    if (!Number.isFinite(momentumPeriod) || momentumPeriod < 1 || momentumPeriod > 65535) {
      throw new Error('VCC momentum period must be in [1, 65535].')
    }
    if (!Number.isFinite(momentumBbPeriod) || momentumBbPeriod < 1 || momentumBbPeriod > 65535) {
      throw new Error('VCC momentum BB period must be in [1, 65535].')
    }
    if (!Number.isFinite(trendPeriod) || trendPeriod < 1 || trendPeriod > 65535) {
      throw new Error('VCC trend period must be in [1, 65535].')
    }
    if (!Number.isFinite(squeezeBbMult) || squeezeBbMult <= 0) {
      throw new Error('VCC squeeze BB multiplier must be > 0.')
    }
    if (!Number.isFinite(squeezeKcMult) || squeezeKcMult <= 0) {
      throw new Error('VCC squeeze KC multiplier must be > 0.')
    }
    if (!Number.isFinite(confirmBars) || confirmBars < 0 || confirmBars > 65535) {
      throw new Error('VCC confirm bars must be in [0, 65535].')
    }
    if (!Number.isFinite(maxTradeBars) || maxTradeBars < 1 || maxTradeBars > 65535) {
      throw new Error('VCC max trade bars must be in [1, 65535].')
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
    }
  }

  let vemr: Record<string, unknown> | undefined
  if (kind === 'Vemr') {
    const mapFastToZPeriod = inputs.vemrMapFastToZPeriod == null
      ? true
      : normalizeBoolean(inputs.vemrMapFastToZPeriod)
    const mapFastToBbPeriod = inputs.vemrMapFastToBbPeriod == null
      ? true
      : normalizeBoolean(inputs.vemrMapFastToBbPeriod)
    const mapSlowToAtrPeriod = inputs.vemrMapSlowToAtrPeriod == null
      ? true
      : normalizeBoolean(inputs.vemrMapSlowToAtrPeriod)

    const zPeriod = Math.floor(num(inputs.vemrZPeriod, 20))
    const zMaType = normalizeText(inputs.vemrZMaType || 'sma') || 'sma'
    const zNbdev = num(inputs.vemrZNbdev, 1.0)
    const zDevtype = Math.floor(num(inputs.vemrZDevtype, 0))
    const zEntry = num(inputs.vemrZEntry, 2.0)
    const zExit = num(inputs.vemrZExit, 0.5)
    const bbPeriod = Math.floor(num(inputs.vemrBbPeriod, 20))
    const bbMult = num(inputs.vemrBbMult, 2.0)
    const requireBandTouch = inputs.vemrRequireBandTouch == null
      ? true
      : normalizeBoolean(inputs.vemrRequireBandTouch)
    const atrPeriod = Math.floor(num(inputs.vemrAtrPeriod, 14))
    const atrStopMult = num(inputs.vemrAtrStopMult, 2.0)
    const confirmBars = Math.floor(num(inputs.vemrConfirmBars, 0))
    const maxTradeBars = Math.floor(num(inputs.vemrMaxTradeBars, 80))

    if (!Number.isFinite(zPeriod) || zPeriod < 1 || zPeriod > 65535) {
      throw new Error('VEMR z period must be in [1, 65535].')
    }
    if (!zMaType) throw new Error('VEMR z MA type is required.')
    if (!Number.isFinite(zNbdev) || zNbdev <= 0) throw new Error('VEMR z nbdev must be > 0.')
    if (!Number.isFinite(zDevtype) || zDevtype < 0 || zDevtype > 255) {
      throw new Error('VEMR z devtype must be in [0, 255].')
    }
    if (!Number.isFinite(zEntry) || zEntry <= 0) throw new Error('VEMR z entry must be > 0.')
    if (!Number.isFinite(zExit) || zExit < 0) throw new Error('VEMR z exit must be >= 0.')
    if (zExit > zEntry) throw new Error('VEMR z exit must be <= z entry.')
    if (!Number.isFinite(bbPeriod) || bbPeriod < 1 || bbPeriod > 65535) {
      throw new Error('VEMR BB period must be in [1, 65535].')
    }
    if (!Number.isFinite(bbMult) || bbMult <= 0) throw new Error('VEMR BB multiplier must be > 0.')
    if (!Number.isFinite(atrPeriod) || atrPeriod < 1 || atrPeriod > 65535) {
      throw new Error('VEMR ATR period must be in [1, 65535].')
    }
    if (!Number.isFinite(atrStopMult) || atrStopMult <= 0) {
      throw new Error('VEMR ATR stop multiplier must be > 0.')
    }
    if (!Number.isFinite(confirmBars) || confirmBars < 0 || confirmBars > 65535) {
      throw new Error('VEMR confirm bars must be in [0, 65535].')
    }
    if (!Number.isFinite(maxTradeBars) || maxTradeBars < 1 || maxTradeBars > 65535) {
      throw new Error('VEMR max trade bars must be in [1, 65535].')
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
    }
  }

  let rocb: Record<string, unknown> | undefined
  if (kind === 'Rocb') {
    const mapFastToChannelPeriod = inputs.rocbMapFastToChannelPeriod == null
      ? true
      : normalizeBoolean(inputs.rocbMapFastToChannelPeriod)
    const mapSlowToAtrPeriod = inputs.rocbMapSlowToAtrPeriod == null
      ? true
      : normalizeBoolean(inputs.rocbMapSlowToAtrPeriod)
    const channelPeriod = Math.floor(num(inputs.rocbChannelPeriod, 20))
    const atrPeriod = Math.floor(num(inputs.rocbAtrPeriod, 14))
    const atrStopMult = num(inputs.rocbAtrStopMult, 2.0)
    const maxTradeBars = Math.floor(num(inputs.rocbMaxTradeBars, 80))

    if (!Number.isFinite(channelPeriod) || channelPeriod < 1 || channelPeriod > 65535) {
      throw new Error('ROCB channel period must be in [1, 65535].')
    }
    if (!Number.isFinite(atrPeriod) || atrPeriod < 1 || atrPeriod > 65535) {
      throw new Error('ROCB ATR period must be in [1, 65535].')
    }
    if (!Number.isFinite(atrStopMult) || atrStopMult <= 0) {
      throw new Error('ROCB ATR stop multiplier must be > 0.')
    }
    if (!Number.isFinite(maxTradeBars) || maxTradeBars < 1 || maxTradeBars > 65535) {
      throw new Error('ROCB max trade bars must be in [1, 65535].')
    }

    rocb = {
      map_fast_to_channel_period: mapFastToChannelPeriod,
      map_slow_to_atr_period: mapSlowToAtrPeriod,
      channel_period: channelPeriod,
      atr_period: atrPeriod,
      atr_stop_mult: atrStopMult,
      max_trade_bars: maxTradeBars,
    }
  }

  const mkcs = kind === 'Mkcs'
    ? {
        map_fast_to_stddev_period: true,
        map_slow_to_breakout_lookback: true,
        stddev_period: 20,
        breakout_lookback: 20,
        compression_threshold: 0.01,
        reexpansion_threshold: 0.03,
      }
    : undefined

  return {
    strategy_kind: kind,
    backend: backendPayload,
    data_id: dataId,
    starting_equity: startingEquity,
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
    mkcs,
    objective,
    mode: 'Grid',
    top_k: topK,
    include_all: includeAll,
    heatmap_bins: 0,
  }
}

export function buildCompositeMoOptimizationRequest(inputs: CompositeMoOptimizationBuildInputs): Record<string, unknown> | null {
  const kind = normalizeStrategyKindRaw(inputs.context?.strategy_kind)
  if (kind !== 'CompositeMo') return null

  const dataId = normalizeText(inputs.context?.data_id)
  if (!dataId) throw new Error('Load CSV first (missing data id).')

  const meta = inputs.context?.current_data_meta
  if (!meta || typeof meta !== 'object') throw new Error('Load CSV first.')
  const dataMeta = meta as Record<string, unknown>
  if (!dataMeta.has_high || !dataMeta.has_low) {
    throw new Error('Composite strategy requires high/low candle fields. Load a CSV with high+low columns.')
  }

  const fast: [number, number, number] = [
    Number(inputs.fastStart),
    Number(inputs.fastEnd),
    Number(inputs.fastStep),
  ]
  const slow: [number, number, number] = [
    Number(inputs.slowStart),
    Number(inputs.slowEnd),
    Number(inputs.slowStep),
  ]
  const fastPeriods = expandRangeU16Like(fast[0], fast[1], fast[2])
  const slowPeriods = expandRangeU16Like(slow[0], slow[1], slow[2])
  const pairs = countPairsFastLtSlow(fastPeriods, slowPeriods)
  if (!pairs) throw new Error('No valid parameter combinations (fast must be < slow).')

  const fastMaType = normalizeText(inputs.fastMaType)
  const slowMaType = normalizeText(inputs.slowMaType)
  if (!fastMaType) throw new Error('Select a fast MA type.')
  if (!slowMaType) throw new Error('Select a slow MA type.')
  const maSupportMap = normalizeMovingAverageSupportMap(inputs.maSupportMap)
  if (hasMovingAverageSupportData(maSupportMap)) {
    if (!maSupportsCpu(maSupportMap, fastMaType) || !maSupportsCpu(maSupportMap, slowMaType)) {
      throw new Error('Composite strategy is CPU-only and does not support one of the selected MA types.')
    }
  }

  const maSource = normalizeText(inputs.maSource || 'close') || 'close'
  validateCandleFieldRequirements(meta, fastMaType, slowMaType, maSource)

  const baseStrategy = buildStrategyConfigFromInputs(inputs, 'CompositeMo')
  const signalMode = normalizeCompositeSignalMode(inputs.compositeSignalMode)
  const maximize = normalizeText(inputs.objective || 'Sharpe') || 'Sharpe'
  const minimize = normalizeText(inputs.moObjectiveMin || 'MaxDrawdown') || 'MaxDrawdown'
  const sampler = buildCompositeMoSampler(inputs)

  const minTrades = parseOptionalRawNumber(inputs.moConstraintMinTrades)
  const maxDd = parseOptionalRawNumber(inputs.moConstraintMaxDd)
  const maxAbsExposure = parseOptionalRawNumber(inputs.moConstraintMaxAbsExposure)
  let constraints: Record<string, unknown> | null = null
  if (Number.isFinite(minTrades) || Number.isFinite(maxDd) || Number.isFinite(maxAbsExposure)) {
    constraints = {}
    if (Number.isFinite(minTrades)) {
      if (minTrades < 0) throw new Error('Min trades must be >= 0.')
      constraints.min_trades = Math.floor(minTrades)
    }
    if (Number.isFinite(maxDd)) {
      if (maxDd < 0) throw new Error('Max drawdown constraint must be >= 0.')
      constraints.max_drawdown = maxDd
    }
    if (Number.isFinite(maxAbsExposure)) {
      if (maxAbsExposure < 0) throw new Error('Max abs exposure constraint must be >= 0.')
      constraints.max_abs_exposure = maxAbsExposure
    }
  }

  let gpuPrescan: Record<string, unknown> | null = null
  if (normalizeBoolean(inputs.moPrescanEnable)) {
    const topK = Math.max(1, Math.floor(num(inputs.moPrescanTopK, 128)))
    const window = Math.max(0, Math.floor(num(inputs.moPrescanWindow, 8)))
    const predictiveTopK = Math.max(0, Math.floor(num(inputs.moPrescanPredictiveTopK, 12)))
    const predictiveStride = Math.max(1, Math.min(65535, Math.floor(num(inputs.moPrescanPredictiveStride, 8))))
    const objectives = normalizeText(inputs.moPrescanObjectives || 'both') || 'both'
    const mode = normalizeText(inputs.moPrescanMode || 'Auto') || 'Auto'
    const maPairs = parseCompositeMoPrescanMaPairs(inputs.moPrescanMaPairs)
    gpuPrescan = {
      top_k: topK,
      window,
      predictive_top_k: predictiveTopK,
      predictive_stride: predictiveStride,
      device_id: 0,
      require_gpu: normalizeBoolean(inputs.moPrescanRequireGpu),
      objectives,
      ma_pairs: maPairs,
      mode,
    }
  }

  const useRangeBuilder = normalizeBoolean(inputs.moUseRangeBuilder)
  const space = useRangeBuilder
    ? buildCompositeMoSearchSpaceFromInputs(inputs)
    : parseCompositeMoSearchSpace(inputs.moSearchSpace)

  const rawFastStep = Math.floor(num(inputs.fastStep, 1))
  const rawSlowStep = Math.floor(num(inputs.slowStep, 1))
  const fastStep = Math.max(1, rawFastStep)
  const slowStep = Math.max(1, rawSlowStep)
  const fastLow = Math.floor(fast[0])
  const slowLow = Math.floor(slow[0])
  const fastHigh = rawFastStep === 0 ? fastLow : Math.floor(fast[1])
  const slowHigh = rawSlowStep === 0 ? slowLow : Math.floor(slow[1])
  const fastDomain = { IntRange: { low: fastLow, high: fastHigh, step: fastStep, log: false } }
  const slowDomain = { IntRange: { low: slowLow, high: slowHigh, step: slowStep, log: false } }
  const searchSpace = normalizeCompositeMoSearchSpace(space, fastDomain, slowDomain)

  const startingEquity = normalizeStartingEquity(inputs.startingEquity)
  return {
    strategy_kind: 'CompositeMo',
    data_id: dataId,
    starting_equity: startingEquity,
    fast_ma_type: fastMaType,
    slow_ma_type: slowMaType,
    ma_source: maSource,
    fast_ma_params: normalizeMaParams(inputs.fastMaParams),
    slow_ma_params: normalizeMaParams(inputs.slowMaParams),
    base_strategy: baseStrategy,
    signal_mode: signalMode,
    strategy: baseStrategy,
    search_space: searchSpace,
    objective: maximize,
    objectives: { maximize, minimize },
    sampler,
    constraints,
    gpu_prescan: gpuPrescan,
    mode: null,
  }
}
