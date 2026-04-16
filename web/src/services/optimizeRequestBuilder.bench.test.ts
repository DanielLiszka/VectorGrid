import fs from 'node:fs/promises'
import path from 'node:path'
import { Bench } from 'tinybench'
import { describe, expect, test } from 'vitest'
import {
  buildCompositeCpuOptimizationRequest,
  buildCompositeMoOptimizationRequest,
  buildDoubleMaOptimizationRequest,
} from './optimizeRequestBuilder'

const SIMPLE_DATA_META = { has_close: true }
const COMPOSITE_DATA_META = {
  has_open: true,
  has_high: true,
  has_low: true,
  has_close: true,
  has_volume: true,
}

const DEFAULT_OUTPUT_PATH = path.resolve(process.cwd(), '../output/benchmarks/request-builder-bench.json')
const BENCH_TIME_MS = Number(process.env.REQUEST_BUILDER_BENCH_TIME_MS || 500)
const WARMUP_TIME_MS = Number(process.env.REQUEST_BUILDER_BENCH_WARMUP_TIME_MS || 100)

function baseDoubleMaInputs() {
  return {
    context: {
      strategy_kind: 'DoubleMa',
      data_id: 'bench-double-ma',
      current_data_meta: SIMPLE_DATA_META,
    },
    backend: 'Auto',
    fastStart: '5',
    fastEnd: '15',
    fastStep: '5',
    slowStart: '20',
    slowEnd: '40',
    slowStep: '10',
    fastMaType: 'sma',
    slowMaType: 'ema',
    maSource: 'close',
    objective: 'Sharpe',
    mode: 'Grid',
    topK: '20',
    includeAll: 'false',
    heatmapBins: '64',
    exportAllCsvPath: '',
    startingEquity: '10000',
    costPreset: 'custom',
    feeBps: '1',
    spreadBps: '2',
    slippageBps: '0.5',
    epsRel: '0.001',
    stopLossPct: '0.01',
    takeProfitPct: '0.03',
    cooldownBars: '0',
    signalPersistenceBars: '1',
    exitMode: 'cross',
    kaufmanStopPeriod: '22',
    kaufmanStopMult: '2.0',
    kaufmanStopMaType: 'sma',
    longOnly: true,
    allowFlip: true,
    tradeOnNext: false,
    dmConstraintMinTrades: '5',
    dmConstraintMaxDd: '0.25',
    fastMaParams: { alpha: 2 },
    slowMaParams: { alpha: 3 },
    fastMaParamSweeps: { alpha: [1, 3, 1] },
    slowMaParamSweeps: null,
  }
}

function baseCompositeDoubleMaInputs(
  strategyKind: 'CompositeTrend' | 'RegimeHybrid' | 'BreakoutMomentum',
) {
  return {
    context: {
      strategy_kind: strategyKind,
      data_id: `bench-${strategyKind}`,
      current_data_meta: COMPOSITE_DATA_META,
    },
    backend: 'GpuKernel',
    fastStart: '5',
    fastEnd: '15',
    fastStep: '5',
    slowStart: '20',
    slowEnd: '40',
    slowStep: '10',
    fastMaType: 'sma',
    slowMaType: 'ema',
    maSource: 'close',
    objective: 'Sharpe',
    mode: 'Grid',
    topK: '20',
    includeAll: 'false',
    heatmapBins: '64',
    exportAllCsvPath: '',
    trendUseAdx: false,
    breakoutUseAdx: false,
  }
}

function baseCompositeCpuInputs() {
  return {
    context: {
      strategy_kind: 'CompositeCpu',
      data_id: 'bench-composite-cpu',
      current_data_meta: COMPOSITE_DATA_META,
    },
    backend: 'CpuOnly',
    fastStart: '5',
    fastEnd: '35',
    fastStep: '5',
    slowStart: '40',
    slowEnd: '120',
    slowStep: '10',
    fastMaType: 'sma',
    slowMaType: 'ema',
    maSource: 'close',
    objective: 'Sharpe',
    mode: 'Grid',
    topK: '20',
    includeAll: 'false',
    startingEquity: '10000',
  }
}

function baseCompositeMoInputs() {
  return {
    context: {
      strategy_kind: 'CompositeMo',
      data_id: 'bench-composite-mo',
      current_data_meta: COMPOSITE_DATA_META,
    },
    fastStart: '5',
    fastEnd: '25',
    fastStep: '5',
    slowStart: '30',
    slowEnd: '120',
    slowStep: '10',
    fastMaType: 'sma',
    slowMaType: 'ema',
    maSource: 'close',
    objective: 'Sharpe',
    moObjectiveMin: 'MaxDrawdown',
    compositeSignalMode: 'hard_gate',
    moSampler: 'Nsga2',
    moNsga2Seed: '1',
    moNsga2Population: '80',
    moNsga2Generations: '40',
    moNsga2CrossoverProb: '0.9',
    moNsga2SwappingProb: '0.5',
    moNsga2MutationProb: '',
    moUseRangeBuilder: true,
    moSearchSpace: '',
    moHtAmpLow: '1',
    moHtAmpHigh: '3',
    moHtAmpStep: '1',
    moHtDevLow: '0.5',
    moHtDevHigh: '1.5',
    moHtDevStep: '0.5',
    moHtAtrLow: '6',
    moHtAtrHigh: '10',
    moHtAtrStep: '2',
    moMaczFastLow: '6',
    moMaczFastHigh: '10',
    moMaczFastStep: '2',
    moMaczSlowLow: '18',
    moMaczSlowHigh: '30',
    moMaczSlowStep: '6',
    moMaczSignalLow: '4',
    moMaczSignalHigh: '8',
    moMaczSignalStep: '2',
    moMaczLengthZLow: '10',
    moMaczLengthZHigh: '20',
    moMaczLengthZStep: '5',
    moMaczLengthStdevLow: '10',
    moMaczLengthStdevHigh: '20',
    moMaczLengthStdevStep: '5',
    moMaczGammaLow: '0.1',
    moMaczGammaHigh: '0.3',
    moMaczGammaStep: '0.1',
    moMaczALow: '0.5',
    moMaczAHigh: '1.5',
    moMaczAStep: '0.5',
    moMaczBLow: '0.1',
    moMaczBHigh: '0.3',
    moMaczBStep: '0.1',
    moMaczUseLagMode: 'both',
    moMaczUseVolumeMode: 'both',
    moOttPeriodLow: '2',
    moOttPeriodHigh: '6',
    moOttPeriodStep: '2',
    moOttPercentLow: '0.5',
    moOttPercentHigh: '1.5',
    moOttPercentStep: '0.5',
    moOttMaTypes: 'SMA,EMA',
    moPrescanEnable: false,
    startingEquity: '10000',
    costPreset: 'custom',
    feeBps: '1',
    spreadBps: '2',
    slippageBps: '0.5',
    epsRel: '0.001',
    stopLossPct: '',
    takeProfitPct: '',
    cooldownBars: '0',
    signalPersistenceBars: '1',
    exitMode: 'cross',
    kaufmanStopPeriod: '22',
    kaufmanStopMult: '2.0',
    kaufmanStopMaType: 'sma',
    longOnly: true,
    allowFlip: true,
    tradeOnNext: false,
    fastMaParams: { alpha: 2 },
    slowMaParams: { alpha: 3 },
  }
}

function assertBenchmarkFixture(label: string, req: Record<string, unknown> | null) {
  if (req == null) {
    throw new Error(`invalid benchmark fixture for ${label}`)
  }
}

const DOUBLE_MA_BASELINE_INPUT = baseDoubleMaInputs()
const COMPOSITE_TREND_KERNEL_INPUT = {
  ...baseCompositeDoubleMaInputs('CompositeTrend'),
  exportAllCsvPath: 'bench.csv',
}
const COMPOSITE_TREND_HYBRID_INPUT = {
  ...baseCompositeDoubleMaInputs('CompositeTrend'),
  trendUseAdx: true,
  exitMode: 'kaufman_stop',
  cooldownBars: '3',
  signalPersistenceBars: '2',
}
const COMPOSITE_TREND_CPU_SUPPORT_INPUT = {
  ...baseCompositeDoubleMaInputs('CompositeTrend'),
  backend: 'CpuOnly',
  topK: '257',
  includeAll: 'true',
  maSupportMap: {
    SMA: { supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
    ' EMA ': { supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
  },
}
const COMPOSITE_CPU_GRID_INPUT = baseCompositeCpuInputs()

assertBenchmarkFixture('double_ma/grid_baseline', buildDoubleMaOptimizationRequest(DOUBLE_MA_BASELINE_INPUT))
assertBenchmarkFixture(
  'double_ma/composite_trend_kernel_route',
  buildDoubleMaOptimizationRequest(COMPOSITE_TREND_KERNEL_INPUT),
)
assertBenchmarkFixture(
  'double_ma/composite_trend_hybrid_route',
  buildDoubleMaOptimizationRequest(COMPOSITE_TREND_HYBRID_INPUT),
)
assertBenchmarkFixture(
  'double_ma/composite_cpu_route_with_support_lookup',
  buildDoubleMaOptimizationRequest(COMPOSITE_TREND_CPU_SUPPORT_INPUT),
)
assertBenchmarkFixture('composite_cpu/grid_request', buildCompositeCpuOptimizationRequest(COMPOSITE_CPU_GRID_INPUT))
assertBenchmarkFixture('composite_mo/range_builder_request', buildCompositeMoOptimizationRequest(baseCompositeMoInputs()))

type RequestBuilderBenchRow = {
  name: string
  meanMs: number
  hz: number
  rme: number
  sampleCount: number
  minMs: number
  maxMs: number
  p99Ms: number
  totalTimeMs: number
  rank: number
}

type RequestBuilderBenchReport = {
  generatedAt: string
  runner: 'tinybench'
  benchmarkTimeMs: number
  warmupTimeMs: number
  cases: RequestBuilderBenchRow[]
}

function collectCaseRows(bench: Bench): RequestBuilderBenchRow[] {
  const rows = bench.tasks.map((task) => {
    const result = task.result
    if (!result) {
      throw new Error(`Missing benchmark result for ${task.name}.`)
    }
    if (!Number.isFinite(result.mean) || !Number.isFinite(result.hz)) {
      throw new Error(`Incomplete benchmark stats for ${task.name}.`)
    }
    return {
      name: task.name,
      meanMs: result.mean,
      hz: result.hz,
      rme: result.rme,
      sampleCount: result.samples.length,
      minMs: result.min,
      maxMs: result.max,
      p99Ms: result.p99,
      totalTimeMs: result.totalTime,
      rank: 0,
    }
  })

  const bySpeed = [...rows].sort((left, right) => left.meanMs - right.meanMs)
  const rankByName = new Map(bySpeed.map((row, index) => [row.name, index + 1]))
  return rows.map((row) => ({ ...row, rank: rankByName.get(row.name) ?? rows.length }))
}

async function writeReport(report: RequestBuilderBenchReport, outputPath: string) {
  await fs.mkdir(path.dirname(outputPath), { recursive: true })
  await fs.writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8')
}

describe('optimizeRequestBuilder benchmark harness', () => {
  test(
    'writes complete request-builder timing stats',
    async () => {
      const bench = new Bench({
        time: BENCH_TIME_MS,
        warmupTime: WARMUP_TIME_MS,
      })

      bench
        .add('double_ma/grid_baseline', () => {
          const req = buildDoubleMaOptimizationRequest(DOUBLE_MA_BASELINE_INPUT)
          if (req == null) throw new Error('expected double-ma benchmark request')
        })
        .add('double_ma/composite_trend_kernel_route', () => {
          const req = buildDoubleMaOptimizationRequest(COMPOSITE_TREND_KERNEL_INPUT)
          if (req == null) throw new Error('expected composite trend kernel benchmark request')
        })
        .add('double_ma/composite_trend_hybrid_route', () => {
          const req = buildDoubleMaOptimizationRequest(COMPOSITE_TREND_HYBRID_INPUT)
          if (req == null) throw new Error('expected composite trend hybrid benchmark request')
        })
        .add('double_ma/composite_cpu_route_with_support_lookup', () => {
          const req = buildDoubleMaOptimizationRequest(COMPOSITE_TREND_CPU_SUPPORT_INPUT)
          if (req == null) throw new Error('expected composite cpu benchmark request')
        })
        .add('composite_cpu/grid_request', () => {
          const req = buildCompositeCpuOptimizationRequest(COMPOSITE_CPU_GRID_INPUT)
          if (req == null) throw new Error('expected composite cpu benchmark request')
        })
        .add('composite_mo/range_builder_request', () => {
          const req = buildCompositeMoOptimizationRequest(baseCompositeMoInputs())
          if (req == null) throw new Error('expected composite mo benchmark request')
        })

      await bench.warmup()
      await bench.run()

      const rows = collectCaseRows(bench)
      const outputPath = path.resolve(
        process.cwd(),
        process.env.REQUEST_BUILDER_BENCH_OUT || DEFAULT_OUTPUT_PATH,
      )
      await writeReport(
        {
          generatedAt: new Date().toISOString(),
          runner: 'tinybench',
          benchmarkTimeMs: BENCH_TIME_MS,
          warmupTimeMs: WARMUP_TIME_MS,
          cases: rows,
        },
        outputPath,
      )

      expect(rows).toHaveLength(6)
      expect(rows.every((row) => Number.isFinite(row.meanMs) && Number.isFinite(row.hz))).toBe(true)
    },
    60_000,
  )
})
