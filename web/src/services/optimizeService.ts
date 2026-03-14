import { getActivePinia } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import { requestCancelCurrentRun } from './runControlService'
import { runValidation as runValidationAction } from './validationService'
import { refreshAnalysisFromWorkflowRun } from './analysisService'
import { applyWorkflowArtifactsPayload } from '../stores/workflowArtifactsStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { computeConfigFingerprint } from '../utils/core/artifactIdentity'
import { buildDatasetMetaForArtifact } from '../utils/core/datasetMeta'
import {
  buildCompositeMoOptimizationRequest,
  buildCompositeCpuOptimizationRequest,
  buildDoubleMaOptimizationRequest,
  type MaParamsAndSweeps,
} from './optimizeRequestBuilder'

interface GpuStatusResponse {
  built_with_cuda?: unknown
  built_with_cuda_backtest_kernel?: unknown
  cuda_available?: unknown
  auto_device_name?: unknown
  auto_device_memory_bytes?: unknown
  auto_device_error?: unknown
}

export interface GpuStatusSummary {
  builtWithCuda: boolean
  builtWithCudaBacktestKernel: boolean
  cudaAvailable: boolean
  autoDeviceName: string
  autoDeviceMemoryBytes: number | null
  deviceMeta: string
  text: string
}

export interface OptimizeRunOptions {
  request?: unknown
  switchTab?: boolean
}

interface SaveRunArtifactResponse {
  path?: unknown
}

interface LegacyOptimizeBuildContext {
  strategy_kind?: unknown
  data_id?: unknown
  current_data_meta?: unknown
}

type VueStoreId = 'data' | 'optimize' | 'advanced' | 'strategy'
type PiniaStoreId = VueStoreId | 'validation'

interface VueStoreFieldBinding {
  storeId: VueStoreId
  key: string
}

interface MovingAverageDescriptor {
  id?: unknown
  supports_cpu?: unknown
  supports_cuda?: unknown
  supports_cuda_kernel?: unknown
}

interface ValidationCrossDatasetEntry {
  checked?: unknown
}

interface PresetProfile {
  name: string
  topK: number
  includeAll: boolean
  heatmapBins: number
  valAutoRun: boolean
  valAutoExportXlsx: boolean
  valAutoRevealReport: boolean
  valAutoExportTopN: number
  valCandidates: number
  valWfEnable: boolean
  valWfWindow: number
  valWfStep: number
  valPerturbEnable: boolean
  valPerturbIters: number
  valPerturbMin: number
  valPerturbMax: number
  valMcEnable: boolean
  valMcSims: number
  valCscvEnable: boolean
  valCscvSlices: number
  valCrossEnable: boolean
  valCrossMax: number
}

type DoubleMaParityBackend = 'GpuKernel'

interface DoubleMaRequestOverrides {
  backend?: unknown
  fastStart?: unknown
  fastEnd?: unknown
  fastStep?: unknown
  slowStart?: unknown
  slowEnd?: unknown
  slowStep?: unknown
  mode?: unknown
  topK?: unknown
  includeAll?: unknown
  heatmapBins?: unknown
  exportAllCsvPath?: unknown
  fastMaParamSweeps?: unknown
  slowMaParamSweeps?: unknown
}

let movingAverageSupportMapCache: Record<string, Record<string, unknown>> | null = null

const BALANCED_PRESET_PROFILE: PresetProfile = {
  name: 'Balanced',
  topK: 20,
  includeAll: false,
  heatmapBins: 128,
  valAutoRun: true,
  valAutoExportXlsx: false,
  valAutoRevealReport: false,
  valAutoExportTopN: 20,
  valCandidates: 32,
  valWfEnable: false,
  valWfWindow: 20000,
  valWfStep: 20000,
  valPerturbEnable: true,
  valPerturbIters: 8,
  valPerturbMin: 0.015,
  valPerturbMax: 0.06,
  valMcEnable: true,
  valMcSims: 500,
  valCscvEnable: false,
  valCscvSlices: 16,
  valCrossEnable: false,
  valCrossMax: 4,
}

const PRESET_PROFILES: Record<string, PresetProfile> = {
  Quick: {
    name: 'Quick',
    topK: 20,
    includeAll: false,
    heatmapBins: 0,
    valAutoRun: true,
    valAutoExportXlsx: false,
    valAutoRevealReport: false,
    valAutoExportTopN: 20,
    valCandidates: 16,
    valWfEnable: false,
    valWfWindow: 20000,
    valWfStep: 20000,
    valPerturbEnable: false,
    valPerturbIters: 8,
    valPerturbMin: 0.015,
    valPerturbMax: 0.06,
    valMcEnable: false,
    valMcSims: 500,
    valCscvEnable: false,
    valCscvSlices: 16,
    valCrossEnable: false,
    valCrossMax: 4,
  },
  Balanced: BALANCED_PRESET_PROFILE,
  Deep: {
    name: 'Deep',
    topK: 50,
    includeAll: false,
    heatmapBins: 256,
    valAutoRun: true,
    valAutoExportXlsx: false,
    valAutoRevealReport: false,
    valAutoExportTopN: 50,
    valCandidates: 64,
    valWfEnable: true,
    valWfWindow: 20000,
    valWfStep: 20000,
    valPerturbEnable: true,
    valPerturbIters: 16,
    valPerturbMin: 0.01,
    valPerturbMax: 0.08,
    valMcEnable: true,
    valMcSims: 2000,
    valCscvEnable: true,
    valCscvSlices: 16,
    valCrossEnable: true,
    valCrossMax: 8,
  },
}

const VUE_TEXT_FIELD_BINDINGS: Record<string, VueStoreFieldBinding> = {
  backend: { storeId: 'optimize', key: 'backend' },
  fastStart: { storeId: 'optimize', key: 'fastStart' },
  fastEnd: { storeId: 'optimize', key: 'fastEnd' },
  fastStep: { storeId: 'optimize', key: 'fastStep' },
  slowStart: { storeId: 'optimize', key: 'slowStart' },
  slowEnd: { storeId: 'optimize', key: 'slowEnd' },
  slowStep: { storeId: 'optimize', key: 'slowStep' },
  fastMaType: { storeId: 'optimize', key: 'fastMaType' },
  slowMaType: { storeId: 'optimize', key: 'slowMaType' },
  maSource: { storeId: 'advanced', key: 'maSource' },
  objective: { storeId: 'advanced', key: 'objective' },
  mode: { storeId: 'advanced', key: 'mode' },
  topK: { storeId: 'advanced', key: 'topK' },
  includeAll: { storeId: 'advanced', key: 'includeAll' },
  heatmapBins: { storeId: 'advanced', key: 'heatmapBins' },
  exportAllCsvPath: { storeId: 'advanced', key: 'exportAllCsvPath' },
  vdpsEta: { storeId: 'advanced', key: 'vdpsEta' },
  vdpsRungs: { storeId: 'advanced', key: 'vdpsRungs' },
  vdpsMinInitialPairs: { storeId: 'advanced', key: 'vdpsMinInitialPairs' },
  vdpsMaxSurvivorSeeds: { storeId: 'advanced', key: 'vdpsMaxSurvivorSeeds' },
  vdpsMaxPeriodCandidates: { storeId: 'advanced', key: 'vdpsMaxPeriodCandidates' },
  vdpsRandomInjectionRatio: { storeId: 'advanced', key: 'vdpsRandomInjectionRatio' },
  vdpsEliteMutationRatio: { storeId: 'advanced', key: 'vdpsEliteMutationRatio' },
  vdpsDifferentialRatio: { storeId: 'advanced', key: 'vdpsDifferentialRatio' },
  vdpsSeed: { storeId: 'advanced', key: 'vdpsSeed' },
  vdpsQualityCheckMaxPairs: { storeId: 'advanced', key: 'vdpsQualityCheckMaxPairs' },
  vdpsQualityCheckTopK: { storeId: 'advanced', key: 'vdpsQualityCheckTopK' },
  moObjectiveMin: { storeId: 'advanced', key: 'moObjectiveMin' },
  moSampler: { storeId: 'advanced', key: 'moSampler' },
  moRandomSeed: { storeId: 'advanced', key: 'moRandomSeed' },
  moRandomTrials: { storeId: 'advanced', key: 'moRandomTrials' },
  moTpeSeed: { storeId: 'advanced', key: 'moTpeSeed' },
  moTpeTrials: { storeId: 'advanced', key: 'moTpeTrials' },
  moTpeStartupTrials: { storeId: 'advanced', key: 'moTpeStartupTrials' },
  moTpeCandidates: { storeId: 'advanced', key: 'moTpeCandidates' },
  moTpeGamma: { storeId: 'advanced', key: 'moTpeGamma' },
  moTpeBatchSize: { storeId: 'advanced', key: 'moTpeBatchSize' },
  moTpeMaxModelPoints: { storeId: 'advanced', key: 'moTpeMaxModelPoints' },
  moQmcSeed: { storeId: 'advanced', key: 'moQmcSeed' },
  moQmcTrials: { storeId: 'advanced', key: 'moQmcTrials' },
  moQmcSequence: { storeId: 'advanced', key: 'moQmcSequence' },
  moNsga2Seed: { storeId: 'advanced', key: 'moNsga2Seed' },
  moNsga2Population: { storeId: 'advanced', key: 'moNsga2Population' },
  moNsga2Generations: { storeId: 'advanced', key: 'moNsga2Generations' },
  moNsga2CrossoverProb: { storeId: 'advanced', key: 'moNsga2CrossoverProb' },
  moNsga2SwappingProb: { storeId: 'advanced', key: 'moNsga2SwappingProb' },
  moNsga2MutationProb: { storeId: 'advanced', key: 'moNsga2MutationProb' },
  moConstraintMinTrades: { storeId: 'advanced', key: 'moConstraintMinTrades' },
  moConstraintMaxDd: { storeId: 'advanced', key: 'moConstraintMaxDd' },
  moConstraintMaxAbsExposure: { storeId: 'advanced', key: 'moConstraintMaxAbsExposure' },
  moPrescanTopK: { storeId: 'advanced', key: 'moPrescanTopK' },
  moPrescanWindow: { storeId: 'advanced', key: 'moPrescanWindow' },
  moPrescanPredictiveTopK: { storeId: 'advanced', key: 'moPrescanPredictiveTopK' },
  moPrescanPredictiveStride: { storeId: 'advanced', key: 'moPrescanPredictiveStride' },
  moPrescanObjectives: { storeId: 'advanced', key: 'moPrescanObjectives' },
  moPrescanMode: { storeId: 'advanced', key: 'moPrescanMode' },
  moPrescanMaPairs: { storeId: 'advanced', key: 'moPrescanMaPairs' },
  moSearchSpace: { storeId: 'advanced', key: 'moSearchSpace' },
  moHtAmpLow: { storeId: 'advanced', key: 'moHtAmpLow' },
  moHtAmpHigh: { storeId: 'advanced', key: 'moHtAmpHigh' },
  moHtAmpStep: { storeId: 'advanced', key: 'moHtAmpStep' },
  moHtDevLow: { storeId: 'advanced', key: 'moHtDevLow' },
  moHtDevHigh: { storeId: 'advanced', key: 'moHtDevHigh' },
  moHtDevStep: { storeId: 'advanced', key: 'moHtDevStep' },
  moHtAtrLow: { storeId: 'advanced', key: 'moHtAtrLow' },
  moHtAtrHigh: { storeId: 'advanced', key: 'moHtAtrHigh' },
  moHtAtrStep: { storeId: 'advanced', key: 'moHtAtrStep' },
  moMaczFastLow: { storeId: 'advanced', key: 'moMaczFastLow' },
  moMaczFastHigh: { storeId: 'advanced', key: 'moMaczFastHigh' },
  moMaczFastStep: { storeId: 'advanced', key: 'moMaczFastStep' },
  moMaczSlowLow: { storeId: 'advanced', key: 'moMaczSlowLow' },
  moMaczSlowHigh: { storeId: 'advanced', key: 'moMaczSlowHigh' },
  moMaczSlowStep: { storeId: 'advanced', key: 'moMaczSlowStep' },
  moMaczSignalLow: { storeId: 'advanced', key: 'moMaczSignalLow' },
  moMaczSignalHigh: { storeId: 'advanced', key: 'moMaczSignalHigh' },
  moMaczSignalStep: { storeId: 'advanced', key: 'moMaczSignalStep' },
  moMaczLengthZLow: { storeId: 'advanced', key: 'moMaczLengthZLow' },
  moMaczLengthZHigh: { storeId: 'advanced', key: 'moMaczLengthZHigh' },
  moMaczLengthZStep: { storeId: 'advanced', key: 'moMaczLengthZStep' },
  moMaczLengthStdevLow: { storeId: 'advanced', key: 'moMaczLengthStdevLow' },
  moMaczLengthStdevHigh: { storeId: 'advanced', key: 'moMaczLengthStdevHigh' },
  moMaczLengthStdevStep: { storeId: 'advanced', key: 'moMaczLengthStdevStep' },
  moMaczGammaLow: { storeId: 'advanced', key: 'moMaczGammaLow' },
  moMaczGammaHigh: { storeId: 'advanced', key: 'moMaczGammaHigh' },
  moMaczGammaStep: { storeId: 'advanced', key: 'moMaczGammaStep' },
  moMaczALow: { storeId: 'advanced', key: 'moMaczALow' },
  moMaczAHigh: { storeId: 'advanced', key: 'moMaczAHigh' },
  moMaczAStep: { storeId: 'advanced', key: 'moMaczAStep' },
  moMaczBLow: { storeId: 'advanced', key: 'moMaczBLow' },
  moMaczBHigh: { storeId: 'advanced', key: 'moMaczBHigh' },
  moMaczBStep: { storeId: 'advanced', key: 'moMaczBStep' },
  moMaczUseLagMode: { storeId: 'advanced', key: 'moMaczUseLagMode' },
  moMaczUseVolumeMode: { storeId: 'advanced', key: 'moMaczUseVolumeMode' },
  moOttPeriodLow: { storeId: 'advanced', key: 'moOttPeriodLow' },
  moOttPeriodHigh: { storeId: 'advanced', key: 'moOttPeriodHigh' },
  moOttPeriodStep: { storeId: 'advanced', key: 'moOttPeriodStep' },
  moOttPercentLow: { storeId: 'advanced', key: 'moOttPercentLow' },
  moOttPercentHigh: { storeId: 'advanced', key: 'moOttPercentHigh' },
  moOttPercentStep: { storeId: 'advanced', key: 'moOttPercentStep' },
  moOttMaTypes: { storeId: 'advanced', key: 'moOttMaTypes' },
  startingEquity: { storeId: 'strategy', key: 'startingEquity' },
  costPreset: { storeId: 'strategy', key: 'costPreset' },
  feeBps: { storeId: 'strategy', key: 'feeBps' },
  spreadBps: { storeId: 'strategy', key: 'spreadBps' },
  slippageBps: { storeId: 'strategy', key: 'slippageBps' },
  epsRel: { storeId: 'strategy', key: 'epsRel' },
  stopLossPct: { storeId: 'strategy', key: 'stopLossPct' },
  takeProfitPct: { storeId: 'strategy', key: 'takeProfitPct' },
  cooldownBars: { storeId: 'strategy', key: 'cooldownBars' },
  signalPersistenceBars: { storeId: 'strategy', key: 'signalPersistenceBars' },
  compositeSignalMode: { storeId: 'strategy', key: 'compositeSignalMode' },
  halfTrendAmplitude: { storeId: 'strategy', key: 'halfTrendAmplitude' },
  halfTrendAtrPeriod: { storeId: 'strategy', key: 'halfTrendAtrPeriod' },
  halfTrendChannelDev: { storeId: 'strategy', key: 'halfTrendChannelDev' },
  maczFastLength: { storeId: 'strategy', key: 'maczFastLength' },
  maczSlowLength: { storeId: 'strategy', key: 'maczSlowLength' },
  maczSignalLength: { storeId: 'strategy', key: 'maczSignalLength' },
  maczLengthZ: { storeId: 'strategy', key: 'maczLengthZ' },
  maczLengthStdev: { storeId: 'strategy', key: 'maczLengthStdev' },
  maczGamma: { storeId: 'strategy', key: 'maczGamma' },
  maczA: { storeId: 'strategy', key: 'maczA' },
  maczB: { storeId: 'strategy', key: 'maczB' },
  ottPeriod: { storeId: 'strategy', key: 'ottPeriod' },
  ottPercent: { storeId: 'strategy', key: 'ottPercent' },
  ottMaType: { storeId: 'strategy', key: 'ottMaType' },
  trendLrPeriod: { storeId: 'strategy', key: 'trendLrPeriod' },
  trendLrMinDeg: { storeId: 'strategy', key: 'trendLrMinDeg' },
  trendBbwPeriod: { storeId: 'strategy', key: 'trendBbwPeriod' },
  trendBbwMult: { storeId: 'strategy', key: 'trendBbwMult' },
  trendBbwMin: { storeId: 'strategy', key: 'trendBbwMin' },
  trendRocPeriod: { storeId: 'strategy', key: 'trendRocPeriod' },
  trendAdxPeriod: { storeId: 'strategy', key: 'trendAdxPeriod' },
  trendAdxMin: { storeId: 'strategy', key: 'trendAdxMin' },
  regimeAdxPeriod: { storeId: 'strategy', key: 'regimeAdxPeriod' },
  regimeAdxEnter: { storeId: 'strategy', key: 'regimeAdxEnter' },
  regimeAdxExit: { storeId: 'strategy', key: 'regimeAdxExit' },
  regimeRevEntryRel: { storeId: 'strategy', key: 'regimeRevEntryRel' },
  regimeRevExitRel: { storeId: 'strategy', key: 'regimeRevExitRel' },
  breakoutBbPeriod: { storeId: 'strategy', key: 'breakoutBbPeriod' },
  breakoutBbMult: { storeId: 'strategy', key: 'breakoutBbMult' },
  breakoutConfirmBars: { storeId: 'strategy', key: 'breakoutConfirmBars' },
  breakoutMacdFast: { storeId: 'strategy', key: 'breakoutMacdFast' },
  breakoutMacdSlow: { storeId: 'strategy', key: 'breakoutMacdSlow' },
  breakoutMacdSignal: { storeId: 'strategy', key: 'breakoutMacdSignal' },
  breakoutCmoPeriod: { storeId: 'strategy', key: 'breakoutCmoPeriod' },
  breakoutCmoMinAbs: { storeId: 'strategy', key: 'breakoutCmoMinAbs' },
  breakoutAdoscShort: { storeId: 'strategy', key: 'breakoutAdoscShort' },
  breakoutAdoscLong: { storeId: 'strategy', key: 'breakoutAdoscLong' },
  breakoutAdxPeriod: { storeId: 'strategy', key: 'breakoutAdxPeriod' },
  breakoutAdxMin: { storeId: 'strategy', key: 'breakoutAdxMin' },
  asrSqueezeBbMult: { storeId: 'strategy', key: 'asrSqueezeBbMult' },
  asrSqueezeKcMult: { storeId: 'strategy', key: 'asrSqueezeKcMult' },
  asrAngleDegMin: { storeId: 'strategy', key: 'asrAngleDegMin' },
  asrAngleDegMr: { storeId: 'strategy', key: 'asrAngleDegMr' },
  asrErMin: { storeId: 'strategy', key: 'asrErMin' },
  asrErMr: { storeId: 'strategy', key: 'asrErMr' },
  asrZMaType: { storeId: 'strategy', key: 'asrZMaType' },
  asrZNbdev: { storeId: 'strategy', key: 'asrZNbdev' },
  asrZDevtype: { storeId: 'strategy', key: 'asrZDevtype' },
  asrZEntry: { storeId: 'strategy', key: 'asrZEntry' },
  asrZExit: { storeId: 'strategy', key: 'asrZExit' },
  asrConfirmBars: { storeId: 'strategy', key: 'asrConfirmBars' },
  asrMomFadeBars: { storeId: 'strategy', key: 'asrMomFadeBars' },
  asrMaxTradeBars: { storeId: 'strategy', key: 'asrMaxTradeBars' },
  asrMaxMrTradeBars: { storeId: 'strategy', key: 'asrMaxMrTradeBars' },
  vccSqueezePeriod: { storeId: 'strategy', key: 'vccSqueezePeriod' },
  vccMomentumPeriod: { storeId: 'strategy', key: 'vccMomentumPeriod' },
  vccMomentumBbPeriod: { storeId: 'strategy', key: 'vccMomentumBbPeriod' },
  vccTrendPeriod: { storeId: 'strategy', key: 'vccTrendPeriod' },
  vccSqueezeBbMult: { storeId: 'strategy', key: 'vccSqueezeBbMult' },
  vccSqueezeKcMult: { storeId: 'strategy', key: 'vccSqueezeKcMult' },
  vccConfirmBars: { storeId: 'strategy', key: 'vccConfirmBars' },
  vccMaxTradeBars: { storeId: 'strategy', key: 'vccMaxTradeBars' },
  pitEfiPeriod: { storeId: 'strategy', key: 'pitEfiPeriod' },
  pitVoscShortPeriod: { storeId: 'strategy', key: 'pitVoscShortPeriod' },
  pitVoscLongPeriod: { storeId: 'strategy', key: 'pitVoscLongPeriod' },
  pitCfoPeriod: { storeId: 'strategy', key: 'pitCfoPeriod' },
  pitCfoScalar: { storeId: 'strategy', key: 'pitCfoScalar' },
  pitConfirmBars: { storeId: 'strategy', key: 'pitConfirmBars' },
  vfctObvSmoothPeriod: { storeId: 'strategy', key: 'vfctObvSmoothPeriod' },
  vfctMfiPeriod: { storeId: 'strategy', key: 'vfctMfiPeriod' },
  vfctMfiLongMin: { storeId: 'strategy', key: 'vfctMfiLongMin' },
  vfctMfiShortMax: { storeId: 'strategy', key: 'vfctMfiShortMax' },
  vfctConfirmBars: { storeId: 'strategy', key: 'vfctConfirmBars' },
  rocbChannelPeriod: { storeId: 'strategy', key: 'rocbChannelPeriod' },
  rocbAtrPeriod: { storeId: 'strategy', key: 'rocbAtrPeriod' },
  rocbAtrStopMult: { storeId: 'strategy', key: 'rocbAtrStopMult' },
  rocbMaxTradeBars: { storeId: 'strategy', key: 'rocbMaxTradeBars' },
  vemrZPeriod: { storeId: 'strategy', key: 'vemrZPeriod' },
  vemrZMaType: { storeId: 'strategy', key: 'vemrZMaType' },
  vemrZNbdev: { storeId: 'strategy', key: 'vemrZNbdev' },
  vemrZDevtype: { storeId: 'strategy', key: 'vemrZDevtype' },
  vemrZEntry: { storeId: 'strategy', key: 'vemrZEntry' },
  vemrZExit: { storeId: 'strategy', key: 'vemrZExit' },
  vemrBbPeriod: { storeId: 'strategy', key: 'vemrBbPeriod' },
  vemrBbMult: { storeId: 'strategy', key: 'vemrBbMult' },
  vemrAtrPeriod: { storeId: 'strategy', key: 'vemrAtrPeriod' },
  vemrAtrStopMult: { storeId: 'strategy', key: 'vemrAtrStopMult' },
  vemrConfirmBars: { storeId: 'strategy', key: 'vemrConfirmBars' },
  vemrMaxTradeBars: { storeId: 'strategy', key: 'vemrMaxTradeBars' },
  exitMode: { storeId: 'strategy', key: 'exitMode' },
  kaufmanStopPeriod: { storeId: 'strategy', key: 'kaufmanStopPeriod' },
  kaufmanStopMult: { storeId: 'strategy', key: 'kaufmanStopMult' },
  kaufmanStopMaType: { storeId: 'strategy', key: 'kaufmanStopMaType' },
  dmConstraintMinTrades: { storeId: 'strategy', key: 'dmConstraintMinTrades' },
  dmConstraintMaxDd: { storeId: 'strategy', key: 'dmConstraintMaxDd' },
}

const VUE_CHECKBOX_FIELD_BINDINGS: Record<string, VueStoreFieldBinding> = {
  longOnly: { storeId: 'strategy', key: 'longOnly' },
  allowFlip: { storeId: 'strategy', key: 'allowFlip' },
  tradeOnNext: { storeId: 'strategy', key: 'tradeOnNext' },
  trendUseRoc: { storeId: 'strategy', key: 'trendUseRoc' },
  trendUseAdx: { storeId: 'strategy', key: 'trendUseAdx' },
  breakoutUseAdx: { storeId: 'strategy', key: 'breakoutUseAdx' },
  maczUseVolume: { storeId: 'strategy', key: 'maczUseVolume' },
  maczUseLag: { storeId: 'strategy', key: 'maczUseLag' },
  asrUseErFilter: { storeId: 'strategy', key: 'asrUseErFilter' },
  vccMapFastToSqueezePeriod: { storeId: 'strategy', key: 'vccMapFastToSqueezePeriod' },
  vccMapFastToMomentumPeriod: { storeId: 'strategy', key: 'vccMapFastToMomentumPeriod' },
  vccMapSlowToTrendPeriod: { storeId: 'strategy', key: 'vccMapSlowToTrendPeriod' },
  vccRequireMomentumAccel: { storeId: 'strategy', key: 'vccRequireMomentumAccel' },
  vemrMapFastToZPeriod: { storeId: 'strategy', key: 'vemrMapFastToZPeriod' },
  vemrMapFastToBbPeriod: { storeId: 'strategy', key: 'vemrMapFastToBbPeriod' },
  vemrMapSlowToAtrPeriod: { storeId: 'strategy', key: 'vemrMapSlowToAtrPeriod' },
  vemrRequireBandTouch: { storeId: 'strategy', key: 'vemrRequireBandTouch' },
  vdpsDeterministic: { storeId: 'advanced', key: 'vdpsDeterministic' },
  moPrescanEnable: { storeId: 'advanced', key: 'moPrescanEnable' },
  moPrescanRequireGpu: { storeId: 'advanced', key: 'moPrescanRequireGpu' },
  moUseRangeBuilder: { storeId: 'advanced', key: 'moUseRangeBuilder' },
}

function normalizeStatusText(raw: unknown): GpuStatusSummary {
  const status = (raw && typeof raw === 'object' ? raw : {}) as GpuStatusResponse
  const builtWithCuda = !!status.built_with_cuda
  const builtWithCudaBacktestKernel = !!status.built_with_cuda_backtest_kernel
  const cudaAvailable = !!status.cuda_available
  const text =
    `GPU: build=${builtWithCuda ? 'cuda' : 'cpu-only'} | ` +
    `kernel=${builtWithCudaBacktestKernel ? 'on' : 'off'} | ` +
    `cuda_available=${cudaAvailable ? 'true' : 'false'}`
  const autoDeviceName = normalizeText(status.auto_device_name)
  const autoDeviceMemoryBytes = Number(status.auto_device_memory_bytes)
  const autoDeviceError = normalizeText(status.auto_device_error)
  const hasAutoDeviceMemory = Number.isFinite(autoDeviceMemoryBytes) && autoDeviceMemoryBytes > 0

  return {
    builtWithCuda,
    builtWithCudaBacktestKernel,
    cudaAvailable,
    autoDeviceName,
    autoDeviceMemoryBytes: hasAutoDeviceMemory ? autoDeviceMemoryBytes : null,
    deviceMeta: builtWithCuda && cudaAvailable
      ? (
          autoDeviceName
            ? `Automatic CUDA device: ${autoDeviceName}${hasAutoDeviceMemory ? ` (${formatMemoryLabel(autoDeviceMemoryBytes)})` : ''}.`
            : (autoDeviceError || 'CUDA is available; the app will choose a single local device automatically.')
        )
      : 'CUDA device selection is unavailable; the app will use CPU-safe routing.',
    text,
  }
}

function formatMemoryLabel(bytesValue: unknown): string {
  const bytes = Number(bytesValue)
  if (!Number.isFinite(bytes) || bytes <= 0) return ''
  const gib = bytes / (1024 ** 3)
  if (gib >= 10) return `${gib.toFixed(0)} GB`
  if (gib >= 1) return `${gib.toFixed(1)} GB`
  const mib = bytes / (1024 ** 2)
  return `${mib.toFixed(0)} MB`
}

function parseNonNegativeInt(value: unknown, fallback: number): number {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) return fallback
  return Math.max(0, Math.floor(parsed))
}

function dispatchInputLikeEvents(target: EventTarget | null): void {
  if (!target) return
  target.dispatchEvent(new Event('input', { bubbles: true }))
  target.dispatchEvent(new Event('change', { bubbles: true }))
}

function queryDomById<T extends Element>(id: string): T | null {
  if (typeof document === 'undefined') return null
  const byQuery = typeof document.querySelector === 'function'
    ? document.querySelector(`[id="${id}"]`)
    : null
  if (byQuery) return byQuery as T
  const getById = (document as unknown as { getElementById?: (nextId: string) => Element | null }).getElementById
  if (typeof getById !== 'function') return null
  return (getById.call(document, id) as T | null) ?? null
}

function writeDomInputValue(id: string, rawValue: string): void {
  const el = queryDomById<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>(id)
  if (!el) return
  const value = String(rawValue || '')
  if (el.value === value) return
  el.value = value
  dispatchInputLikeEvents(el)
}

function writeDomCheckboxValue(id: string, checked: boolean): void {
  const el = queryDomById<HTMLInputElement>(id)
  if (!el) return
  const next = !!checked
  if (el.checked === next) return
  el.checked = next
  dispatchInputLikeEvents(el)
}

function writeDomTextContent(id: string, value: string): void {
  const el = queryDomById<HTMLElement>(id)
  if (!el) return
  el.textContent = value
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') return null
  return value as Record<string, unknown>
}

function markStoreVueUpdated(store: Record<string, unknown>): void {
  store.source = 'vue'
  store.updatedAtUnixMs = Date.now()
}

function patchStoreState(storeId: PiniaStoreId, patch: Record<string, unknown>): void {
  const store = readPiniaStoreState(storeId)
  if (!store) return
  Object.assign(store, patch)
  markStoreVueUpdated(store)
}

function countCheckedCrossDatasets(validationStore: Record<string, unknown> | null): number {
  if (!validationStore) return 0
  const rows = validationStore.crossDatasets
  if (Array.isArray(rows)) {
    let count = 0
    for (const entry of rows) {
      const row = entry as ValidationCrossDatasetEntry | null
      if (normalizeBoolean(row?.checked)) count += 1
    }
    return count
  }
  const selected = Number(validationStore.crossSelectedCount)
  if (Number.isFinite(selected) && selected > 0) return Math.floor(selected)
  return 0
}

function resolvePresetProfile(profileId: unknown): PresetProfile {
  const id = normalizeText(profileId)
  return PRESET_PROFILES[id] ?? BALANCED_PRESET_PROFILE
}

function maSupportsFlag(
  map: Record<string, Record<string, unknown>> | null,
  maType: string,
  flag: 'supports_cuda' | 'supports_cuda_kernel',
): boolean {
  if (!map) return false
  const row = map[maType]
  return !!row?.[flag]
}

function isCancellationError(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error || '')
  return /cancel/i.test(message)
}

function formatFixed(value: number, digits = 6): string {
  if (!Number.isFinite(value)) return 'NaN'
  return value.toFixed(digits)
}

function readBestMetric(result: unknown, key: 'pnl' | 'sharpe' | 'max_dd'): number {
  const row = asRecord(result)
  const metrics = asRecord(row?.best_metrics)
  return Number(metrics?.[key] ?? 0)
}

function readBestParams(result: unknown): Record<string, unknown> {
  const row = asRecord(result)
  const params = asRecord(row?.best_params)
  return params ?? {}
}

function summarizeBackendExecution(result: unknown): string {
  const row = asRecord(result)
  if (!row) return ''
  const backendUsed = normalizeText(row.backend_used)
  const route = normalizeText(row.backend_route)
  const fallbackReason = normalizeText(row.backend_fallback_reason)
  const indicatorPlan = asRecord(row.indicator_route_plan)
  const fastPlan = asRecord(indicatorPlan?.fast)
  const slowPlan = asRecord(indicatorPlan?.slow)
  const fastId = normalizeText(fastPlan?.indicator_id)
  const fastRoute = normalizeText(fastPlan?.route)
  const fastCapability = normalizeText(fastPlan?.capability)
  const fastGpuPreferred = normalizeBoolean(fastPlan?.gpu_preferred_by_speedup)
  const slowId = normalizeText(slowPlan?.indicator_id)
  const slowRoute = normalizeText(slowPlan?.route)
  const slowCapability = normalizeText(slowPlan?.capability)
  const slowGpuPreferred = normalizeBoolean(slowPlan?.gpu_preferred_by_speedup)
  const parts: string[] = []
  const policyCpuRouted: string[] = []
  if (backendUsed) parts.push(`backend=${backendUsed}`)
  if (route) parts.push(`route=${route}`)
  if (fastId || slowId) {
    const segs: string[] = []
    if (fastId || fastRoute) segs.push(`fast:${fastId || '?'}->${fastRoute || '?'}`)
    if (slowId || slowRoute) segs.push(`slow:${slowId || '?'}->${slowRoute || '?'}`)
    if (segs.length > 0) parts.push(`indicators=${segs.join(', ')}`)
  }
  if (
    fastRoute === 'cpu_staged'
    && fastCapability === 'gpu_kernel'
    && !fastGpuPreferred
    && fastId
  ) {
    policyCpuRouted.push(`fast:${fastId}`)
  }
  if (
    slowRoute === 'cpu_staged'
    && slowCapability === 'gpu_kernel'
    && !slowGpuPreferred
    && slowId
  ) {
    policyCpuRouted.push(`slow:${slowId}`)
  }
  if (policyCpuRouted.length > 0) {
    parts.push(`policy_cpu_route=${policyCpuRouted.join(', ')} (gpu_speedups.csv)`)
  }
  if (fallbackReason) parts.push(`fallback=${fallbackReason}`)
  return parts.join(' • ')
}

function updateParityStatus(meta: string, statusLine: string): void {
  patchStoreState('optimize', {
    gpuParityMeta: meta,
    statusLine,
  })
  writeDomTextContent('gpuParityMeta', meta)
  writeDomTextContent('status', statusLine)
}

function applyGpuSummaryToOptimizeStore(summary: GpuStatusSummary): void {
  patchStoreState('optimize', {
    gpuStatus: summary.text,
    gpuDeviceMeta: summary.deviceMeta,
  })
  writeDomTextContent('gpuStatus', summary.text)
}

export async function applyPreset(): Promise<void> {
  const optimizeStore = readPiniaStoreState('optimize')
  if (normalizeBoolean(optimizeStore?.canCancel)) return

  const preset = resolvePresetProfile(optimizeStore?.presetProfile)
  const validationStore = readPiniaStoreState('validation')
  const crossSelectedCount = countCheckedCrossDatasets(validationStore)
  const crossEnabled = preset.valCrossEnable && crossSelectedCount > 0
  const crossNote = preset.valCrossEnable && !crossEnabled ? ' (cross-ticker skipped; no datasets selected)' : ''
  const statusLine = `Applied preset: ${preset.name}${crossNote}`

  patchStoreState('advanced', {
    topK: String(preset.topK),
    includeAll: preset.includeAll ? 'true' : 'false',
    heatmapBins: String(preset.heatmapBins),
  })
  patchStoreState('validation', {
    autoRun: preset.valAutoRun,
    autoExportXlsx: preset.valAutoExportXlsx,
    autoRevealReport: preset.valAutoRevealReport,
    autoExportTopN: String(preset.valAutoExportTopN),
    candidates: String(preset.valCandidates),
    wfEnable: preset.valWfEnable,
    wfWindow: String(preset.valWfWindow),
    wfStep: String(preset.valWfStep),
    perturbEnable: preset.valPerturbEnable,
    perturbIters: String(preset.valPerturbIters),
    perturbMin: String(preset.valPerturbMin),
    perturbMax: String(preset.valPerturbMax),
    mcEnable: preset.valMcEnable,
    mcSims: String(preset.valMcSims),
    cscvEnable: preset.valCscvEnable,
    cscvSlices: String(preset.valCscvSlices),
    crossEnable: crossEnabled,
    crossMax: String(preset.valCrossMax),
    statusLine,
  })
  patchStoreState('optimize', { statusLine })

  writeDomInputValue('topK', String(preset.topK))
  writeDomInputValue('includeAll', preset.includeAll ? 'true' : 'false')
  writeDomInputValue('heatmapBins', String(preset.heatmapBins))
  writeDomCheckboxValue('valAutoRun', preset.valAutoRun)
  writeDomCheckboxValue('valAutoExportXlsx', preset.valAutoExportXlsx)
  writeDomCheckboxValue('valAutoRevealReport', preset.valAutoRevealReport)
  writeDomInputValue('valAutoExportTopN', String(preset.valAutoExportTopN))
  writeDomInputValue('valCandidates', String(preset.valCandidates))
  writeDomCheckboxValue('valWfEnable', preset.valWfEnable)
  writeDomInputValue('valWfWindow', String(preset.valWfWindow))
  writeDomInputValue('valWfStep', String(preset.valWfStep))
  writeDomCheckboxValue('valPerturbEnable', preset.valPerturbEnable)
  writeDomInputValue('valPerturbIters', String(preset.valPerturbIters))
  writeDomInputValue('valPerturbMin', String(preset.valPerturbMin))
  writeDomInputValue('valPerturbMax', String(preset.valPerturbMax))
  writeDomCheckboxValue('valMcEnable', preset.valMcEnable)
  writeDomInputValue('valMcSims', String(preset.valMcSims))
  writeDomCheckboxValue('valCscvEnable', preset.valCscvEnable)
  writeDomInputValue('valCscvSlices', String(preset.valCscvSlices))
  writeDomCheckboxValue('valCrossEnable', crossEnabled)
  writeDomInputValue('valCrossMax', String(preset.valCrossMax))
  writeDomTextContent('status', statusLine)
}

export async function presetKernel(): Promise<void> {
  const optimizeStore = readPiniaStoreState('optimize')
  if (normalizeBoolean(optimizeStore?.canCancel)) return

  let maSupportMap: Record<string, Record<string, unknown>> | null = null
  try {
    maSupportMap = await getMovingAverageSupportMap()
  } catch {
    maSupportMap = null
  }

  const hasSma = !maSupportMap || Object.prototype.hasOwnProperty.call(maSupportMap, 'sma')
  const hasAlma = !maSupportMap || Object.prototype.hasOwnProperty.call(maSupportMap, 'alma')
  const fastMaType = hasSma ? 'sma' : (normalizeText(optimizeStore?.fastMaType) || 'sma')
  const slowMaType = hasAlma ? 'alma' : (normalizeText(optimizeStore?.slowMaType) || 'alma')
  const statusLine = 'Applied preset: GPU kernel (SMA/ALMA)'

  patchStoreState('optimize', {
    backend: 'GpuKernel',
    fastMaType,
    slowMaType,
    statusLine,
  })
  patchStoreState('advanced', { maSource: 'close' })

  writeDomInputValue('backend', 'GpuKernel')
  writeDomInputValue('maSource', 'close')
  writeDomInputValue('fastMaType', fastMaType)
  writeDomInputValue('slowMaType', slowMaType)
  writeDomTextContent('status', statusLine)

  try {
    const summary = await refreshGpuStatus()
    applyGpuSummaryToOptimizeStore(summary)
  } catch {
    // keep preset application non-throwing (legacy-compatible behavior)
  }
}

export async function refreshGpuStatus(): Promise<GpuStatusSummary> {
  const status = await invokeCommand('get_gpu_status')
  return normalizeStatusText(status)
}

export async function runGpuParityCheck(): Promise<void> {
  const optimizeStore = readPiniaStoreState('optimize')
  if (normalizeBoolean(optimizeStore?.canCancel)) return

  const strategyKind = normalizeText(optimizeStore?.strategyKind || 'DoubleMa') || 'DoubleMa'
  if (strategyKind !== 'DoubleMa' && strategyKind !== 'Pit' && strategyKind !== 'Set' && strategyKind !== 'Mts' && strategyKind !== 'Orc' && strategyKind !== 'Marrt' && strategyKind !== 'Vfct') {
    updateParityStatus('Parity: switch Strategy to Double MA, MARRT, or VFCT to run the check.', 'Parity unavailable: switch strategy.')
    return
  }

  updateParityStatus('Parity: checking GPU availability...', 'Checking GPU status for parity...')

  let statusSummary: GpuStatusSummary
  try {
    statusSummary = await refreshGpuStatus()
    applyGpuSummaryToOptimizeStore(statusSummary)
  } catch (error) {
    updateParityStatus(`Parity: failed (${String(error || '')})`, 'Parity check failed.')
    return
  }

  if (!statusSummary.builtWithCuda) {
    updateParityStatus('Parity: unavailable (app not built with CUDA).', 'Parity unavailable: app not built with CUDA.')
    return
  }
  if (!statusSummary.cudaAvailable) {
    updateParityStatus('Parity: unavailable (CUDA runtime not available).', 'Parity unavailable: CUDA runtime not available.')
    return
  }

  const maSupportMap = await getMovingAverageSupportMap()
  const fastMaType = readStoreTextInputValue('fastMaType')
  const slowMaType = readStoreTextInputValue('slowMaType')
  const exitMode = readStoreTextInputValue('exitMode') || 'cross'
  const cooldownBars = parseNonNegativeInt(readStoreTextInputValue('cooldownBars'), 0)
  const signalPersistenceBars = Math.max(1, parseNonNegativeInt(readStoreTextInputValue('signalPersistenceBars'), 1))
  const hasExecutionOverlays = cooldownBars > 0 || signalPersistenceBars > 1

  let gpuBackend: DoubleMaParityBackend | null = null
  const supportsKernel =
    statusSummary.builtWithCudaBacktestKernel &&
    exitMode === 'cross' &&
    !hasExecutionOverlays &&
    maSupportsFlag(maSupportMap, fastMaType, 'supports_cuda_kernel') &&
    maSupportsFlag(maSupportMap, slowMaType, 'supports_cuda_kernel')
  const supportsCudaSweep =
    maSupportsFlag(maSupportMap, fastMaType, 'supports_cuda') &&
    maSupportsFlag(maSupportMap, slowMaType, 'supports_cuda')
  if (supportsKernel || supportsCudaSweep) {
    gpuBackend = 'GpuKernel'
  }

  if (!gpuBackend) {
    updateParityStatus(
      'Parity: unavailable (selected MAs do not support unified GPU path).',
      'Parity unavailable: selected MAs do not support unified GPU.',
    )
    return
  }

  const context = readOptimizeBuildContextFromVueStores()
  if (!context) {
    updateParityStatus('Parity: failed (Load CSV first (missing data id).)', 'Parity check failed.')
    return
  }

  const f0 = Math.max(1, parseNonNegativeInt(readStoreTextInputValue('fastStart'), 5))
  const s0raw = Math.max(1, parseNonNegativeInt(readStoreTextInputValue('slowStart'), 20))
  const s0 = Math.max(s0raw, f0 + 1)
  const f1 = f0 + 2
  const s1 = s0 + 3
  let cpuReq: Record<string, unknown> | null = null
  let gpuReq: Record<string, unknown> | null = null
  try {
    const tinyRangeOverrides: DoubleMaRequestOverrides = {
      fastStart: String(f0),
      fastEnd: String(f1),
      fastStep: '1',
      slowStart: String(s0),
      slowEnd: String(s1),
      slowStep: '1',
      mode: 'Grid',
      topK: '8',
      includeAll: 'false',
      heatmapBins: '0',
      exportAllCsvPath: '',
      fastMaParamSweeps: null,
      slowMaParamSweeps: null,
    }
    cpuReq = await buildAutoDoubleMaRequest(context, {
      ...tinyRangeOverrides,
      backend: 'CpuOnly',
    })
    gpuReq = await buildAutoDoubleMaRequest(context, {
      ...tinyRangeOverrides,
      backend: gpuBackend,
    })
  } catch (error) {
    updateParityStatus(`Parity: failed (${String(error || '')})`, 'Parity check failed.')
    return
  }

  if (!cpuReq || !gpuReq) {
    updateParityStatus('Parity: failed (Unable to build tiny parity requests.)', 'Parity check failed.')
    return
  }

  updateParityStatus('Parity: running CPU ↔ GPU tiny-grid checks...', 'Running CPU ↔ GPU parity check (tiny grid)...')

  try {
    const cpuRes = await invokeCommand('run_double_ma_optimization', { req: cpuReq })
    const gpuRes = await invokeCommand('run_double_ma_optimization', { req: gpuReq })

    const dp = Math.abs(readBestMetric(cpuRes, 'pnl') - readBestMetric(gpuRes, 'pnl'))
    const ds = Math.abs(readBestMetric(cpuRes, 'sharpe') - readBestMetric(gpuRes, 'sharpe'))
    const dd = Math.abs(readBestMetric(cpuRes, 'max_dd') - readBestMetric(gpuRes, 'max_dd'))
    const sameParams = JSON.stringify(readBestParams(cpuRes)) === JSON.stringify(readBestParams(gpuRes))
    const tol = 1e-4
    const ok = dp <= tol && ds <= tol && dd <= tol
    const paramsHint = sameParams ? '' : ' (best_params differ; may be tie)'
    const meta =
      `Parity: ${ok ? 'OK' : 'CHECK'} • gpu=${gpuBackend} • ` +
      `|Δpnl|=${formatFixed(dp, 6)} |Δsharpe|=${formatFixed(ds, 6)} |Δdd|=${formatFixed(dd, 6)} tol=${tol}${paramsHint}`
    const statusLine = ok ? 'Parity check OK.' : 'Parity check completed (review diffs).'
    updateParityStatus(meta, statusLine)
  } catch (error) {
    if (isCancellationError(error)) {
      updateParityStatus('Parity: cancelled.', 'Parity cancelled.')
      return
    }
    updateParityStatus(`Parity: failed (${String(error || '')})`, 'Parity check failed.')
    return
  }

  try {
    const summary = await refreshGpuStatus()
    applyGpuSummaryToOptimizeStore(summary)
  } catch {
    // keep parity action completion non-throwing
  }
}

function normalizeRequestPayload(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') return null
  return value as Record<string, unknown>
}

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizeBoolean(value: unknown): boolean {
  if (typeof value === 'boolean') return value
  const text = normalizeText(value).toLowerCase()
  return text === 'true' || text === '1' || text === 'yes' || text === 'on'
}

function normalizeDataId(value: unknown): string {
  const id = normalizeText(value)
  if (!id || id === '—' || id === '-') return ''
  return id
}

type OptimizeStrategyKind =
  | 'DoubleMa'
  | 'Pit'
  | 'Set'
  | 'Mts'
  | 'Orc'
  | 'Marrt'
  | 'Vfct'
  | 'Rocb'
  | 'Kqsr'
  | 'Carm'
  | 'Kvdf'
  | 'Mkcs'
  | 'Cdmc'
  | 'Asr'
  | 'Vcc'
  | 'Vemr'
  | 'CompositeCpu'
  | 'CompositeMo'
  | 'CompositeTrend'
  | 'RegimeHybrid'
  | 'BreakoutMomentum'

function normalizeStrategyKindRaw(kindRaw: unknown): OptimizeStrategyKind {
  const kind = normalizeText(kindRaw || 'DoubleMa') || 'DoubleMa'
  if (kind === 'Pit') return 'Pit'
  if (kind === 'Set') return 'Set'
  if (kind === 'Mts') return 'Mts'
  if (kind === 'Orc') return 'Orc'
  if (kind === 'Marrt') return 'Marrt'
  if (kind === 'Vfct') return 'Vfct'
  if (kind === 'Rocb') return 'Rocb'
  if (kind === 'Kqsr') return 'Kqsr'
  if (kind === 'Carm') return 'Carm'
  if (kind === 'Kvdf') return 'Kvdf'
  if (kind === 'Mkcs') return 'Mkcs'
  if (kind === 'Cdmc') return 'Cdmc'
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

function resolveOptimizationCommand(kindRaw: unknown): 'run_double_ma_optimization' | 'run_composite_cpu_optimization' | 'run_composite_mo_optimization' {
  const kind = normalizeStrategyKindRaw(kindRaw)
  if (kind === 'CompositeMo') return 'run_composite_mo_optimization'
  if (
    kind === 'Asr'
    || kind === 'Vcc'
    || kind === 'Vemr'
    || kind === 'Rocb'
    || kind === 'Kqsr'
    || kind === 'Carm'
    || kind === 'Kvdf'
    || kind === 'Mkcs'
    || kind === 'Cdmc'
    || kind === 'CompositeCpu'
  ) {
    return 'run_composite_cpu_optimization'
  }
  return 'run_double_ma_optimization'
}

function readPiniaStoreState(storeId: PiniaStoreId): Record<string, unknown> | null {
  const pinia = getActivePinia()
  if (!pinia) return null
  const root = pinia.state.value as Record<string, unknown>
  const store = root[storeId]
  return store && typeof store === 'object' ? store as Record<string, unknown> : null
}

function readVueTextFieldValue(fieldId: string): string | null {
  const binding = VUE_TEXT_FIELD_BINDINGS[fieldId]
  if (!binding) return null
  const store = readPiniaStoreState(binding.storeId)
  if (!store) return null
  if (!Object.prototype.hasOwnProperty.call(store, binding.key)) return null
  return normalizeText(store[binding.key])
}

function readVueCheckboxFieldValue(fieldId: string): boolean | null {
  const binding = VUE_CHECKBOX_FIELD_BINDINGS[fieldId]
  if (!binding) return null
  const store = readPiniaStoreState(binding.storeId)
  if (!store) return null
  if (!Object.prototype.hasOwnProperty.call(store, binding.key)) return null
  return normalizeBoolean(store[binding.key])
}

function normalizeDataFields(value: unknown): Set<string> {
  const out = new Set<string>()
  if (!Array.isArray(value)) return out
  for (const entry of value) {
    const field = normalizeText(entry).toLowerCase()
    if (!field) continue
    out.add(field)
  }
  return out
}

function deriveCurrentDataMetaFromDataStore(): Record<string, unknown> | null {
  const dataStore = readPiniaStoreState('data')
  if (!dataStore) return null
  const fields = normalizeDataFields(dataStore.dataFields)
  const len = Number(dataStore.dataLen)
  const baseMeta = asRecord(dataStore.baseDataMeta)
  const timeframe = normalizeText(dataStore.dataTimeframe || baseMeta?.timeframe)
  const hasMeta = fields.size > 0 || (Number.isFinite(len) && len >= 0) || !!timeframe
  if (!hasMeta) return null

  const meta: Record<string, unknown> = {
    has_open: fields.has('open'),
    has_high: fields.has('high'),
    has_low: fields.has('low'),
    has_close: fields.has('close'),
    has_volume: fields.has('volume'),
  }
  if (Number.isFinite(len) && len >= 0) meta.len = Math.floor(len)
  if (timeframe) meta.timeframe = timeframe
  return meta
}

function readOptimizeBuildContextFromVueStores(): LegacyOptimizeBuildContext | null {
  const optimizeStore = readPiniaStoreState('optimize')
  const dataStore = readPiniaStoreState('data')
  const strategy_kind = normalizeText(optimizeStore?.strategyKind)
  const data_id = normalizeDataId(dataStore?.dataId)
  const current_data_meta = deriveCurrentDataMetaFromDataStore()
  if (!strategy_kind && !data_id && !current_data_meta) return null
  return {
    strategy_kind: strategy_kind || undefined,
    data_id: data_id || undefined,
    current_data_meta: current_data_meta || null,
  }
}

function normalizeMovingAverageSupportMap(value: unknown): Record<string, Record<string, unknown>> | null {
  if (!Array.isArray(value)) return null
  const out: Record<string, Record<string, unknown>> = {}
  for (const entry of value) {
    if (!entry || typeof entry !== 'object') continue
    const row = entry as MovingAverageDescriptor
    const id = normalizeText(row.id)
    if (!id) continue
    out[id] = {
      supports_cpu: row.supports_cpu,
      supports_cuda: row.supports_cuda,
      supports_cuda_kernel: row.supports_cuda_kernel,
    }
  }
  return Object.keys(out).length ? out : null
}

async function getMovingAverageSupportMap(): Promise<Record<string, Record<string, unknown>> | null> {
  if (movingAverageSupportMapCache) return movingAverageSupportMapCache
  try {
    const list = await invokeCommand('list_moving_averages')
    const map = normalizeMovingAverageSupportMap(list)
    if (map) movingAverageSupportMapCache = map
    return map
  } catch {
    return null
  }
}

function readStoreTextInputValue(id: string): string {
  const key = normalizeText(id)
  if (!key) return ''
  const vueValue = readVueTextFieldValue(key)
  return vueValue != null ? vueValue : ''
}

function readStoreCheckboxValue(id: string): boolean {
  const key = normalizeText(id)
  if (!key) return false
  const vueValue = readVueCheckboxFieldValue(key)
  return vueValue != null ? vueValue : false
}

function normalizeMaParamsFromStore(value: unknown): Record<string, number> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  const out: Record<string, number> = {}
  for (const [key, raw] of Object.entries(value as Record<string, unknown>)) {
    const name = normalizeText(key)
    if (!name) continue
    const parsed = Number(raw)
    if (!Number.isFinite(parsed)) continue
    out[name] = parsed
  }
  return Object.keys(out).length > 0 ? out : null
}

function normalizeMaParamSweepsFromStore(value: unknown): Record<string, [number, number, number]> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
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
  return Object.keys(out).length > 0 ? out : null
}

function readOptimizeStoreMaParamState(): { fast: MaParamsAndSweeps; slow: MaParamsAndSweeps } | null {
  const optimizeStore = readPiniaStoreState('optimize')
  if (!optimizeStore) return null
  const fast: MaParamsAndSweeps = {
    params: normalizeMaParamsFromStore(optimizeStore.fastMaParams),
    sweeps: normalizeMaParamSweepsFromStore(optimizeStore.fastMaParamSweeps),
  }
  const slow: MaParamsAndSweeps = {
    params: normalizeMaParamsFromStore(optimizeStore.slowMaParams),
    sweeps: normalizeMaParamSweepsFromStore(optimizeStore.slowMaParamSweeps),
  }
  return { fast, slow }
}

async function buildAutoDoubleMaRequest(
  context: LegacyOptimizeBuildContext,
  overrides: DoubleMaRequestOverrides = {},
): Promise<Record<string, unknown> | null> {
  const maSupportMap = await getMovingAverageSupportMap()
  const storeMaParamState = readOptimizeStoreMaParamState()
  const emptyParamState: MaParamsAndSweeps = { params: null, sweeps: null }
  const fastParamState: MaParamsAndSweeps = storeMaParamState?.fast ?? emptyParamState
  const slowParamState: MaParamsAndSweeps = storeMaParamState?.slow ?? emptyParamState

  return buildDoubleMaOptimizationRequest({
    context,
    maSupportMap,
    backend: overrides.backend ?? readStoreTextInputValue('backend'),
    fastStart: overrides.fastStart ?? readStoreTextInputValue('fastStart'),
    fastEnd: overrides.fastEnd ?? readStoreTextInputValue('fastEnd'),
    fastStep: overrides.fastStep ?? readStoreTextInputValue('fastStep'),
    slowStart: overrides.slowStart ?? readStoreTextInputValue('slowStart'),
    slowEnd: overrides.slowEnd ?? readStoreTextInputValue('slowEnd'),
    slowStep: overrides.slowStep ?? readStoreTextInputValue('slowStep'),
    fastMaType: readStoreTextInputValue('fastMaType'),
    slowMaType: readStoreTextInputValue('slowMaType'),
    maSource: readStoreTextInputValue('maSource'),
    objective: readStoreTextInputValue('objective'),
    mode: overrides.mode ?? readStoreTextInputValue('mode'),
    topK: overrides.topK ?? readStoreTextInputValue('topK'),
    includeAll: overrides.includeAll ?? readStoreTextInputValue('includeAll'),
    heatmapBins: overrides.heatmapBins ?? readStoreTextInputValue('heatmapBins'),
    exportAllCsvPath: overrides.exportAllCsvPath ?? readStoreTextInputValue('exportAllCsvPath'),
    startingEquity: readStoreTextInputValue('startingEquity'),
    costPreset: readStoreTextInputValue('costPreset'),
    feeBps: readStoreTextInputValue('feeBps'),
    spreadBps: readStoreTextInputValue('spreadBps'),
    slippageBps: readStoreTextInputValue('slippageBps'),
    epsRel: readStoreTextInputValue('epsRel'),
    stopLossPct: readStoreTextInputValue('stopLossPct'),
    takeProfitPct: readStoreTextInputValue('takeProfitPct'),
    cooldownBars: readStoreTextInputValue('cooldownBars'),
    signalPersistenceBars: readStoreTextInputValue('signalPersistenceBars'),
    exitMode: readStoreTextInputValue('exitMode'),
    kaufmanStopPeriod: readStoreTextInputValue('kaufmanStopPeriod'),
    kaufmanStopMult: readStoreTextInputValue('kaufmanStopMult'),
    kaufmanStopMaType: readStoreTextInputValue('kaufmanStopMaType'),
    trendLrPeriod: readStoreTextInputValue('trendLrPeriod'),
    trendLrMinDeg: readStoreTextInputValue('trendLrMinDeg'),
    trendBbwPeriod: readStoreTextInputValue('trendBbwPeriod'),
    trendBbwMult: readStoreTextInputValue('trendBbwMult'),
    trendBbwMin: readStoreTextInputValue('trendBbwMin'),
    trendRocPeriod: readStoreTextInputValue('trendRocPeriod'),
    trendUseRoc: readStoreCheckboxValue('trendUseRoc'),
    trendUseAdx: readStoreCheckboxValue('trendUseAdx'),
    trendAdxPeriod: readStoreTextInputValue('trendAdxPeriod'),
    trendAdxMin: readStoreTextInputValue('trendAdxMin'),
    regimeAdxPeriod: readStoreTextInputValue('regimeAdxPeriod'),
    regimeAdxEnter: readStoreTextInputValue('regimeAdxEnter'),
    regimeAdxExit: readStoreTextInputValue('regimeAdxExit'),
    regimeRevEntryRel: readStoreTextInputValue('regimeRevEntryRel'),
    regimeRevExitRel: readStoreTextInputValue('regimeRevExitRel'),
    breakoutBbPeriod: readStoreTextInputValue('breakoutBbPeriod'),
    breakoutBbMult: readStoreTextInputValue('breakoutBbMult'),
    breakoutConfirmBars: readStoreTextInputValue('breakoutConfirmBars'),
    breakoutMacdFast: readStoreTextInputValue('breakoutMacdFast'),
    breakoutMacdSlow: readStoreTextInputValue('breakoutMacdSlow'),
    breakoutMacdSignal: readStoreTextInputValue('breakoutMacdSignal'),
    breakoutCmoPeriod: readStoreTextInputValue('breakoutCmoPeriod'),
    breakoutCmoMinAbs: readStoreTextInputValue('breakoutCmoMinAbs'),
    breakoutAdoscShort: readStoreTextInputValue('breakoutAdoscShort'),
    breakoutAdoscLong: readStoreTextInputValue('breakoutAdoscLong'),
    breakoutUseAdx: readStoreCheckboxValue('breakoutUseAdx'),
    breakoutAdxPeriod: readStoreTextInputValue('breakoutAdxPeriod'),
    breakoutAdxMin: readStoreTextInputValue('breakoutAdxMin'),
    pitEfiPeriod: readStoreTextInputValue('pitEfiPeriod'),
    pitVoscShortPeriod: readStoreTextInputValue('pitVoscShortPeriod'),
    pitVoscLongPeriod: readStoreTextInputValue('pitVoscLongPeriod'),
    pitCfoPeriod: readStoreTextInputValue('pitCfoPeriod'),
    pitCfoScalar: readStoreTextInputValue('pitCfoScalar'),
    pitConfirmBars: readStoreTextInputValue('pitConfirmBars'),
    vfctObvSmoothPeriod: readStoreTextInputValue('vfctObvSmoothPeriod'),
    vfctMfiPeriod: readStoreTextInputValue('vfctMfiPeriod'),
    vfctMfiLongMin: readStoreTextInputValue('vfctMfiLongMin'),
    vfctMfiShortMax: readStoreTextInputValue('vfctMfiShortMax'),
    vfctConfirmBars: readStoreTextInputValue('vfctConfirmBars'),
    longOnly: readStoreCheckboxValue('longOnly'),
    allowFlip: readStoreCheckboxValue('allowFlip'),
    tradeOnNext: readStoreCheckboxValue('tradeOnNext'),
    dmConstraintMinTrades: readStoreTextInputValue('dmConstraintMinTrades'),
    dmConstraintMaxDd: readStoreTextInputValue('dmConstraintMaxDd'),
    fastMaParams: fastParamState.params,
    slowMaParams: slowParamState.params,
    fastMaParamSweeps: overrides.fastMaParamSweeps ?? fastParamState.sweeps,
    slowMaParamSweeps: overrides.slowMaParamSweeps ?? slowParamState.sweeps,
    vdpsEta: readStoreTextInputValue('vdpsEta'),
    vdpsRungs: readStoreTextInputValue('vdpsRungs'),
    vdpsMinInitialPairs: readStoreTextInputValue('vdpsMinInitialPairs'),
    vdpsMaxSurvivorSeeds: readStoreTextInputValue('vdpsMaxSurvivorSeeds'),
    vdpsMaxPeriodCandidates: readStoreTextInputValue('vdpsMaxPeriodCandidates'),
    vdpsRandomInjectionRatio: readStoreTextInputValue('vdpsRandomInjectionRatio'),
    vdpsEliteMutationRatio: readStoreTextInputValue('vdpsEliteMutationRatio'),
    vdpsDifferentialRatio: readStoreTextInputValue('vdpsDifferentialRatio'),
    vdpsDeterministic: readStoreCheckboxValue('vdpsDeterministic'),
    vdpsSeed: readStoreTextInputValue('vdpsSeed'),
    vdpsQualityCheckMaxPairs: readStoreTextInputValue('vdpsQualityCheckMaxPairs'),
    vdpsQualityCheckTopK: readStoreTextInputValue('vdpsQualityCheckTopK'),
  })
}

export async function buildOptimizationRequestFromVueStores(): Promise<Record<string, unknown>> {
  const req = await tryBuildAutoOptimizationRequest()
  if (!req) {
    throw new Error('Load data first, then configure strategy/optimization settings.')
  }
  return req
}

async function tryBuildAutoOptimizationRequest(): Promise<Record<string, unknown> | null> {
  const context = readOptimizeBuildContextFromVueStores()
  if (!context) return null
  const hasStrategy = !!normalizeText(context.strategy_kind)
  const hasDataId = !!normalizeDataId(context.data_id)
  const hasDataMeta = !!(context.current_data_meta && typeof context.current_data_meta === 'object')
  if (!hasStrategy || !hasDataId || !hasDataMeta) return null

  const kind = normalizeText(context.strategy_kind)
  const fastMaType = readStoreTextInputValue('fastMaType')
  const slowMaType = readStoreTextInputValue('slowMaType')
  const storeMaParamState = readOptimizeStoreMaParamState()
  const emptyParamState: MaParamsAndSweeps = { params: null, sweeps: null }
  const fastParamState: MaParamsAndSweeps = storeMaParamState?.fast ?? emptyParamState
  const slowParamState: MaParamsAndSweeps = storeMaParamState?.slow ?? emptyParamState

  if (kind === 'DoubleMa' || kind === 'Pit' || kind === 'Set' || kind === 'Mts' || kind === 'Orc' || kind === 'Marrt' || kind === 'Vfct' || kind === 'CompositeTrend' || kind === 'RegimeHybrid' || kind === 'BreakoutMomentum') {
    return buildAutoDoubleMaRequest(context)
  }

  if (
    kind === 'Asr'
    || kind === 'Vcc'
    || kind === 'Vemr'
    || kind === 'Rocb'
    || kind === 'Kqsr'
    || kind === 'Carm'
    || kind === 'Kvdf'
    || kind === 'Mkcs'
    || kind === 'Cdmc'
    || kind === 'CompositeCpu'
  ) {
    const maSupportMap = await getMovingAverageSupportMap()
    return buildCompositeCpuOptimizationRequest({
      context,
      maSupportMap,
      backend: readStoreTextInputValue('backend'),
      fastStart: readStoreTextInputValue('fastStart'),
      fastEnd: readStoreTextInputValue('fastEnd'),
      fastStep: readStoreTextInputValue('fastStep'),
      slowStart: readStoreTextInputValue('slowStart'),
      slowEnd: readStoreTextInputValue('slowEnd'),
      slowStep: readStoreTextInputValue('slowStep'),
      fastMaType,
      slowMaType,
      maSource: readStoreTextInputValue('maSource'),
      objective: readStoreTextInputValue('objective'),
      topK: readStoreTextInputValue('topK'),
      includeAll: readStoreTextInputValue('includeAll'),
      compositeSignalMode: readStoreTextInputValue('compositeSignalMode'),
      halfTrendAmplitude: readStoreTextInputValue('halfTrendAmplitude'),
      halfTrendAtrPeriod: readStoreTextInputValue('halfTrendAtrPeriod'),
      halfTrendChannelDev: readStoreTextInputValue('halfTrendChannelDev'),
      maczFastLength: readStoreTextInputValue('maczFastLength'),
      maczSlowLength: readStoreTextInputValue('maczSlowLength'),
      maczSignalLength: readStoreTextInputValue('maczSignalLength'),
      maczLengthZ: readStoreTextInputValue('maczLengthZ'),
      maczLengthStdev: readStoreTextInputValue('maczLengthStdev'),
      maczGamma: readStoreTextInputValue('maczGamma'),
      maczA: readStoreTextInputValue('maczA'),
      maczB: readStoreTextInputValue('maczB'),
      maczUseVolume: readStoreCheckboxValue('maczUseVolume'),
      maczUseLag: readStoreCheckboxValue('maczUseLag'),
      ottPeriod: readStoreTextInputValue('ottPeriod'),
      ottPercent: readStoreTextInputValue('ottPercent'),
      ottMaType: readStoreTextInputValue('ottMaType'),
      startingEquity: readStoreTextInputValue('startingEquity'),
      costPreset: readStoreTextInputValue('costPreset'),
      feeBps: readStoreTextInputValue('feeBps'),
      spreadBps: readStoreTextInputValue('spreadBps'),
      slippageBps: readStoreTextInputValue('slippageBps'),
      epsRel: readStoreTextInputValue('epsRel'),
      stopLossPct: readStoreTextInputValue('stopLossPct'),
      takeProfitPct: readStoreTextInputValue('takeProfitPct'),
      cooldownBars: readStoreTextInputValue('cooldownBars'),
      signalPersistenceBars: readStoreTextInputValue('signalPersistenceBars'),
      exitMode: readStoreTextInputValue('exitMode'),
      kaufmanStopPeriod: readStoreTextInputValue('kaufmanStopPeriod'),
      kaufmanStopMult: readStoreTextInputValue('kaufmanStopMult'),
      kaufmanStopMaType: readStoreTextInputValue('kaufmanStopMaType'),
      longOnly: readStoreCheckboxValue('longOnly'),
      allowFlip: readStoreCheckboxValue('allowFlip'),
      tradeOnNext: readStoreCheckboxValue('tradeOnNext'),
      trendLrPeriod: readStoreTextInputValue('trendLrPeriod'),
      trendLrMinDeg: readStoreTextInputValue('trendLrMinDeg'),
      trendBbwPeriod: readStoreTextInputValue('trendBbwPeriod'),
      trendBbwMult: readStoreTextInputValue('trendBbwMult'),
      trendBbwMin: readStoreTextInputValue('trendBbwMin'),
      trendRocPeriod: readStoreTextInputValue('trendRocPeriod'),
      trendUseRoc: readStoreCheckboxValue('trendUseRoc'),
      trendUseAdx: readStoreCheckboxValue('trendUseAdx'),
      trendAdxPeriod: readStoreTextInputValue('trendAdxPeriod'),
      trendAdxMin: readStoreTextInputValue('trendAdxMin'),
      regimeAdxPeriod: readStoreTextInputValue('regimeAdxPeriod'),
      regimeAdxEnter: readStoreTextInputValue('regimeAdxEnter'),
      regimeAdxExit: readStoreTextInputValue('regimeAdxExit'),
      regimeRevEntryRel: readStoreTextInputValue('regimeRevEntryRel'),
      regimeRevExitRel: readStoreTextInputValue('regimeRevExitRel'),
      breakoutBbPeriod: readStoreTextInputValue('breakoutBbPeriod'),
      breakoutBbMult: readStoreTextInputValue('breakoutBbMult'),
      breakoutConfirmBars: readStoreTextInputValue('breakoutConfirmBars'),
      breakoutMacdFast: readStoreTextInputValue('breakoutMacdFast'),
      breakoutMacdSlow: readStoreTextInputValue('breakoutMacdSlow'),
      breakoutMacdSignal: readStoreTextInputValue('breakoutMacdSignal'),
      breakoutCmoPeriod: readStoreTextInputValue('breakoutCmoPeriod'),
      breakoutCmoMinAbs: readStoreTextInputValue('breakoutCmoMinAbs'),
      breakoutAdoscShort: readStoreTextInputValue('breakoutAdoscShort'),
      breakoutAdoscLong: readStoreTextInputValue('breakoutAdoscLong'),
      breakoutUseAdx: readStoreCheckboxValue('breakoutUseAdx'),
      breakoutAdxPeriod: readStoreTextInputValue('breakoutAdxPeriod'),
      breakoutAdxMin: readStoreTextInputValue('breakoutAdxMin'),
      asrSqueezeBbMult: readStoreTextInputValue('asrSqueezeBbMult'),
      asrSqueezeKcMult: readStoreTextInputValue('asrSqueezeKcMult'),
      asrAngleDegMin: readStoreTextInputValue('asrAngleDegMin'),
      asrAngleDegMr: readStoreTextInputValue('asrAngleDegMr'),
      asrUseErFilter: readStoreCheckboxValue('asrUseErFilter'),
      asrErMin: readStoreTextInputValue('asrErMin'),
      asrErMr: readStoreTextInputValue('asrErMr'),
      asrZMaType: readStoreTextInputValue('asrZMaType'),
      asrZNbdev: readStoreTextInputValue('asrZNbdev'),
      asrZDevtype: readStoreTextInputValue('asrZDevtype'),
      asrZEntry: readStoreTextInputValue('asrZEntry'),
      asrZExit: readStoreTextInputValue('asrZExit'),
      asrConfirmBars: readStoreTextInputValue('asrConfirmBars'),
      asrMomFadeBars: readStoreTextInputValue('asrMomFadeBars'),
      asrMaxTradeBars: readStoreTextInputValue('asrMaxTradeBars'),
      asrMaxMrTradeBars: readStoreTextInputValue('asrMaxMrTradeBars'),
      vccMapFastToSqueezePeriod: readStoreCheckboxValue('vccMapFastToSqueezePeriod'),
      vccMapFastToMomentumPeriod: readStoreCheckboxValue('vccMapFastToMomentumPeriod'),
      vccMapSlowToTrendPeriod: readStoreCheckboxValue('vccMapSlowToTrendPeriod'),
      vccSqueezePeriod: readStoreTextInputValue('vccSqueezePeriod'),
      vccMomentumPeriod: readStoreTextInputValue('vccMomentumPeriod'),
      vccMomentumBbPeriod: readStoreTextInputValue('vccMomentumBbPeriod'),
      vccTrendPeriod: readStoreTextInputValue('vccTrendPeriod'),
      vccSqueezeBbMult: readStoreTextInputValue('vccSqueezeBbMult'),
      vccSqueezeKcMult: readStoreTextInputValue('vccSqueezeKcMult'),
      vccConfirmBars: readStoreTextInputValue('vccConfirmBars'),
      vccRequireMomentumAccel: readStoreCheckboxValue('vccRequireMomentumAccel'),
      vccMaxTradeBars: readStoreTextInputValue('vccMaxTradeBars'),
      vemrMapFastToZPeriod: readStoreCheckboxValue('vemrMapFastToZPeriod'),
      vemrMapFastToBbPeriod: readStoreCheckboxValue('vemrMapFastToBbPeriod'),
      vemrMapSlowToAtrPeriod: readStoreCheckboxValue('vemrMapSlowToAtrPeriod'),
      vemrRequireBandTouch: readStoreCheckboxValue('vemrRequireBandTouch'),
      vemrZPeriod: readStoreTextInputValue('vemrZPeriod'),
      vemrZMaType: readStoreTextInputValue('vemrZMaType'),
      vemrZNbdev: readStoreTextInputValue('vemrZNbdev'),
      vemrZDevtype: readStoreTextInputValue('vemrZDevtype'),
      vemrZEntry: readStoreTextInputValue('vemrZEntry'),
      vemrZExit: readStoreTextInputValue('vemrZExit'),
      vemrBbPeriod: readStoreTextInputValue('vemrBbPeriod'),
      vemrBbMult: readStoreTextInputValue('vemrBbMult'),
      vemrAtrPeriod: readStoreTextInputValue('vemrAtrPeriod'),
      vemrAtrStopMult: readStoreTextInputValue('vemrAtrStopMult'),
      vemrConfirmBars: readStoreTextInputValue('vemrConfirmBars'),
      vemrMaxTradeBars: readStoreTextInputValue('vemrMaxTradeBars'),
      fastMaParams: fastParamState.params,
      slowMaParams: slowParamState.params,
    })
  }

  if (kind === 'CompositeMo') {
    const maSupportMap = await getMovingAverageSupportMap()
    return buildCompositeMoOptimizationRequest({
      context,
      maSupportMap,
      fastStart: readStoreTextInputValue('fastStart'),
      fastEnd: readStoreTextInputValue('fastEnd'),
      fastStep: readStoreTextInputValue('fastStep'),
      slowStart: readStoreTextInputValue('slowStart'),
      slowEnd: readStoreTextInputValue('slowEnd'),
      slowStep: readStoreTextInputValue('slowStep'),
      fastMaType,
      slowMaType,
      maSource: readStoreTextInputValue('maSource'),
      objective: readStoreTextInputValue('objective'),
      moObjectiveMin: readStoreTextInputValue('moObjectiveMin'),
      compositeSignalMode: readStoreTextInputValue('compositeSignalMode'),
      fastMaParams: fastParamState.params,
      slowMaParams: slowParamState.params,
      startingEquity: readStoreTextInputValue('startingEquity'),
      costPreset: readStoreTextInputValue('costPreset'),
      feeBps: readStoreTextInputValue('feeBps'),
      spreadBps: readStoreTextInputValue('spreadBps'),
      slippageBps: readStoreTextInputValue('slippageBps'),
      epsRel: readStoreTextInputValue('epsRel'),
      stopLossPct: readStoreTextInputValue('stopLossPct'),
      takeProfitPct: readStoreTextInputValue('takeProfitPct'),
      cooldownBars: readStoreTextInputValue('cooldownBars'),
      signalPersistenceBars: readStoreTextInputValue('signalPersistenceBars'),
      exitMode: readStoreTextInputValue('exitMode'),
      kaufmanStopPeriod: readStoreTextInputValue('kaufmanStopPeriod'),
      kaufmanStopMult: readStoreTextInputValue('kaufmanStopMult'),
      kaufmanStopMaType: readStoreTextInputValue('kaufmanStopMaType'),
      longOnly: readStoreCheckboxValue('longOnly'),
      allowFlip: readStoreCheckboxValue('allowFlip'),
      tradeOnNext: readStoreCheckboxValue('tradeOnNext'),
      moSampler: readStoreTextInputValue('moSampler'),
      moRandomSeed: readStoreTextInputValue('moRandomSeed'),
      moRandomTrials: readStoreTextInputValue('moRandomTrials'),
      moTpeSeed: readStoreTextInputValue('moTpeSeed'),
      moTpeTrials: readStoreTextInputValue('moTpeTrials'),
      moTpeStartupTrials: readStoreTextInputValue('moTpeStartupTrials'),
      moTpeCandidates: readStoreTextInputValue('moTpeCandidates'),
      moTpeGamma: readStoreTextInputValue('moTpeGamma'),
      moTpeBatchSize: readStoreTextInputValue('moTpeBatchSize'),
      moTpeMaxModelPoints: readStoreTextInputValue('moTpeMaxModelPoints'),
      moQmcSeed: readStoreTextInputValue('moQmcSeed'),
      moQmcTrials: readStoreTextInputValue('moQmcTrials'),
      moQmcSequence: readStoreTextInputValue('moQmcSequence'),
      moNsga2Seed: readStoreTextInputValue('moNsga2Seed'),
      moNsga2Population: readStoreTextInputValue('moNsga2Population'),
      moNsga2Generations: readStoreTextInputValue('moNsga2Generations'),
      moNsga2CrossoverProb: readStoreTextInputValue('moNsga2CrossoverProb'),
      moNsga2SwappingProb: readStoreTextInputValue('moNsga2SwappingProb'),
      moNsga2MutationProb: readStoreTextInputValue('moNsga2MutationProb'),
      moConstraintMinTrades: readStoreTextInputValue('moConstraintMinTrades'),
      moConstraintMaxDd: readStoreTextInputValue('moConstraintMaxDd'),
      moConstraintMaxAbsExposure: readStoreTextInputValue('moConstraintMaxAbsExposure'),
      moPrescanEnable: readStoreCheckboxValue('moPrescanEnable'),
      moPrescanTopK: readStoreTextInputValue('moPrescanTopK'),
      moPrescanWindow: readStoreTextInputValue('moPrescanWindow'),
      moPrescanPredictiveTopK: readStoreTextInputValue('moPrescanPredictiveTopK'),
      moPrescanPredictiveStride: readStoreTextInputValue('moPrescanPredictiveStride'),
      moPrescanRequireGpu: readStoreCheckboxValue('moPrescanRequireGpu'),
      moPrescanObjectives: readStoreTextInputValue('moPrescanObjectives'),
      moPrescanMode: readStoreTextInputValue('moPrescanMode'),
      moPrescanMaPairs: readStoreTextInputValue('moPrescanMaPairs'),
      moUseRangeBuilder: readStoreCheckboxValue('moUseRangeBuilder'),
      moSearchSpace: readStoreTextInputValue('moSearchSpace'),
      moHtAmpLow: readStoreTextInputValue('moHtAmpLow'),
      moHtAmpHigh: readStoreTextInputValue('moHtAmpHigh'),
      moHtAmpStep: readStoreTextInputValue('moHtAmpStep'),
      moHtDevLow: readStoreTextInputValue('moHtDevLow'),
      moHtDevHigh: readStoreTextInputValue('moHtDevHigh'),
      moHtDevStep: readStoreTextInputValue('moHtDevStep'),
      moHtAtrLow: readStoreTextInputValue('moHtAtrLow'),
      moHtAtrHigh: readStoreTextInputValue('moHtAtrHigh'),
      moHtAtrStep: readStoreTextInputValue('moHtAtrStep'),
      moMaczFastLow: readStoreTextInputValue('moMaczFastLow'),
      moMaczFastHigh: readStoreTextInputValue('moMaczFastHigh'),
      moMaczFastStep: readStoreTextInputValue('moMaczFastStep'),
      moMaczSlowLow: readStoreTextInputValue('moMaczSlowLow'),
      moMaczSlowHigh: readStoreTextInputValue('moMaczSlowHigh'),
      moMaczSlowStep: readStoreTextInputValue('moMaczSlowStep'),
      moMaczSignalLow: readStoreTextInputValue('moMaczSignalLow'),
      moMaczSignalHigh: readStoreTextInputValue('moMaczSignalHigh'),
      moMaczSignalStep: readStoreTextInputValue('moMaczSignalStep'),
      moMaczLengthZLow: readStoreTextInputValue('moMaczLengthZLow'),
      moMaczLengthZHigh: readStoreTextInputValue('moMaczLengthZHigh'),
      moMaczLengthZStep: readStoreTextInputValue('moMaczLengthZStep'),
      moMaczLengthStdevLow: readStoreTextInputValue('moMaczLengthStdevLow'),
      moMaczLengthStdevHigh: readStoreTextInputValue('moMaczLengthStdevHigh'),
      moMaczLengthStdevStep: readStoreTextInputValue('moMaczLengthStdevStep'),
      moMaczGammaLow: readStoreTextInputValue('moMaczGammaLow'),
      moMaczGammaHigh: readStoreTextInputValue('moMaczGammaHigh'),
      moMaczGammaStep: readStoreTextInputValue('moMaczGammaStep'),
      moMaczALow: readStoreTextInputValue('moMaczALow'),
      moMaczAHigh: readStoreTextInputValue('moMaczAHigh'),
      moMaczAStep: readStoreTextInputValue('moMaczAStep'),
      moMaczBLow: readStoreTextInputValue('moMaczBLow'),
      moMaczBHigh: readStoreTextInputValue('moMaczBHigh'),
      moMaczBStep: readStoreTextInputValue('moMaczBStep'),
      moMaczUseLagMode: readStoreTextInputValue('moMaczUseLagMode'),
      moMaczUseVolumeMode: readStoreTextInputValue('moMaczUseVolumeMode'),
      moOttPeriodLow: readStoreTextInputValue('moOttPeriodLow'),
      moOttPeriodHigh: readStoreTextInputValue('moOttPeriodHigh'),
      moOttPeriodStep: readStoreTextInputValue('moOttPeriodStep'),
      moOttPercentLow: readStoreTextInputValue('moOttPercentLow'),
      moOttPercentHigh: readStoreTextInputValue('moOttPercentHigh'),
      moOttPercentStep: readStoreTextInputValue('moOttPercentStep'),
      moOttMaTypes: readStoreTextInputValue('moOttMaTypes'),
    })
  }

  return null
}

function safePrettyJson(value: unknown): string {
  try {
    return JSON.stringify(value ?? null, null, 2)
  } catch {
    return String(value ?? '')
  }
}

function selectWorkflowResultsTab(tab: 'top' | 'overview', persist: boolean): void {
  const pinia = getActivePinia()
  if (!pinia) return
  const workflowStore = useWorkflowStore(pinia)
  workflowStore.selectResultsTab(tab, { persist })
}

function readValidationAutoRunEnabled(): boolean {
  const validationStore = readPiniaStoreState('validation')
  if (!validationStore) return false
  return normalizeBoolean(validationStore.autoRun)
}

function buildRunDatasetContextState(): {
  currentDataMeta: Record<string, unknown> | null
  currentBaseDataMeta: Record<string, unknown> | null
  currentBaseDatasetCtx: Record<string, unknown>
  currentDatasetCtx: Record<string, unknown>
} {
  const dataStore = readPiniaStoreState('data')
  const dataMeta = deriveCurrentDataMetaFromDataStore()
  const label = normalizeText(dataStore?.datasetLabel)
  const baseMeta = asRecord(dataStore?.baseDataMeta)
  const timeframe = normalizeText(dataStore?.dataTimeframe || baseMeta?.timeframe)
  const source = normalizeText(dataStore?.datasetContext)
  const symbol = normalizeText(dataStore?.polygonSymbol || label)

  const datasetCtx: Record<string, unknown> = {}
  if (label && label !== '—') datasetCtx.label = label
  if (symbol && symbol !== '—') datasetCtx.symbol = symbol
  if (timeframe) {
    datasetCtx.timeframe = timeframe
    datasetCtx.base_timeframe = timeframe
    datasetCtx.derived_timeframe = timeframe
    datasetCtx.resample_applied = false
  }
  if (source && source !== '—') datasetCtx.source = source

  return {
    currentDataMeta: dataMeta,
    currentBaseDataMeta: dataMeta,
    currentBaseDatasetCtx: datasetCtx,
    currentDatasetCtx: datasetCtx,
  }
}

function buildRunArtifactPayload(req: Record<string, unknown>, res: unknown, ts: string): {
  artifact: Record<string, unknown>
  csvPath: string
} {
  const dataStore = readPiniaStoreState('data')
  const csvPath = normalizeText(dataStore?.csvPath)
  const datasetMeta = buildDatasetMetaForArtifact(csvPath, buildRunDatasetContextState())
  const configFingerprint = computeConfigFingerprint(req, null)
  const artifact: Record<string, unknown> = {
    version: 1,
    kind: 'vectorgrid_run_artifact',
    ts,
    config_fingerprint: configFingerprint,
    run: { req, res },
    validation: null,
    report_xlsx_path: '',
    data: { csv_path: csvPath },
    dataset_meta: datasetMeta,
    tags: [],
  }
  return { artifact, csvPath }
}

async function persistRunArtifact(
  req: Record<string, unknown>,
  res: unknown,
  ts: string,
): Promise<{ path: string; csvPath: string }> {
  const { artifact, csvPath } = buildRunArtifactPayload(req, res, ts)
  try {
    const saved = (await invokeCommand('save_run_artifact', { artifact })) as SaveRunArtifactResponse
    return {
      path: normalizeText(saved?.path),
      csvPath,
    }
  } catch {
    return {
      path: '',
      csvPath,
    }
  }
}

function applyVueRunArtifacts(
  ts: string,
  req: Record<string, unknown>,
  res: unknown,
  artifactPath: string,
  csvPath: string,
): void {
  const datasetMeta = buildDatasetMetaForArtifact(csvPath, buildRunDatasetContextState())
  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ts,
        artifact_path: artifactPath,
        report_xlsx_path: '',
        csv_path: csvPath,
        dataset_meta: datasetMeta,
        tags: [],
        req,
        res,
      },
      currentValidation: null,
    },
    'vue',
  )
}

function markOptimizationRunning(running: boolean): void {
  patchStoreState('optimize', {
    runDisabled: running,
    canCancel: running,
    validateDisabled: running,
  })
}

export async function runOptimization(options: OptimizeRunOptions = {}): Promise<void> {
  let req = normalizeRequestPayload(options.request)
  if (!req) {
    try {
      req = await tryBuildAutoOptimizationRequest()
    } catch {
      req = null
    }
  }
  if (!req) {
    const message = 'Load data first, then configure strategy/optimization settings.'
    patchStoreState('optimize', { statusLine: message })
    writeDomTextContent('status', message)
    return
  }

  const command = resolveOptimizationCommand(req.strategy_kind ?? req.strategyKind)
  const switchTab = options.switchTab !== false
  const runTs = new Date().toISOString()

  markOptimizationRunning(true)
  patchStoreState('optimize', {
    progressMeta: 'Running optimization...',
    statusLine: 'Running optimization...',
  })
  writeDomTextContent('progressMeta', 'Running optimization...')
  writeDomTextContent('status', 'Running optimization...')

  try {
    const startedAt = typeof performance !== 'undefined' ? performance.now() : Date.now()
    const res = await invokeCommand(command, { req })
    const finishedAt = typeof performance !== 'undefined' ? performance.now() : Date.now()
    const elapsedMs = Math.max(0, finishedAt - startedAt)
    const strategyKind = normalizeStrategyKindRaw(req.strategy_kind ?? req.strategyKind)
    const progressMeta = `${strategyKind} • ${elapsedMs.toFixed(0)}ms`
    const backendSummary = summarizeBackendExecution(res)
    const statusLine = backendSummary
      ? `Done in ${elapsedMs.toFixed(0)}ms (frontend) • ${backendSummary}`
      : `Done in ${elapsedMs.toFixed(0)}ms (frontend)`

    patchStoreState('optimize', {
      progressMeta,
      statusLine,
    })
    writeDomTextContent('progressMeta', progressMeta)
    writeDomTextContent('status', statusLine)
    writeDomTextContent('output', safePrettyJson(res))

    const persisted = await persistRunArtifact(req, res, runTs)
    applyVueRunArtifacts(runTs, req, res, persisted.path, persisted.csvPath)
    refreshAnalysisFromWorkflowRun()

    if (switchTab) selectWorkflowResultsTab('top', false)

    if (readValidationAutoRunEnabled()) {
      try {
        await runValidationAction({ useFiltered: false, switchTab: false })
        if (switchTab) selectWorkflowResultsTab('overview', true)
      } catch (error) {
        const message = isCancellationError(error)
          ? 'Auto-validation cancelled.'
          : `Auto-validation failed: ${String(error || '')}`
        patchStoreState('optimize', { statusLine: message })
        writeDomTextContent('status', message)
      }
    }

    try {
      const summary = await refreshGpuStatus()
      applyGpuSummaryToOptimizeStore(summary)
    } catch {
      // Keep run completion non-throwing if status refresh is unavailable.
    }
  } catch (error) {
    const statusLine = isCancellationError(error) ? 'Cancelled.' : String(error || '')
    patchStoreState('optimize', { statusLine })
    writeDomTextContent('status', statusLine)
    throw error
  } finally {
    markOptimizationRunning(false)
  }
}

export async function cancelOptimization(): Promise<void> {
  await requestCancelCurrentRun()
}

export async function runValidation(): Promise<void> {
  await runValidationAction()
}
