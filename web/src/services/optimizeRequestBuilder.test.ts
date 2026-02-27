import { describe, expect, it } from 'vitest'
import {
  buildCompositeCpuOptimizationRequest,
  buildCompositeMoOptimizationRequest,
  buildDoubleMaOptimizationRequest,
} from './optimizeRequestBuilder'

describe('optimizeRequestBuilder', () => {
  it('builds a DoubleMa optimization request from typed inputs', () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'DoubleMa',
        data_id: 'data-123',
        current_data_meta: { has_close: true },
      },
      backend: 'Auto',
      deviceId: '0',
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
      trailingStopPct: '',
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
    })

    expect(req).toBeTruthy()
    expect(req).toMatchObject({
      strategy_kind: 'DoubleMa',
      data_id: 'data-123',
      objective: 'Sharpe',
      mode: 'Grid',
      top_k: 20,
      include_all: false,
      fast_ma_types: ['sma'],
      slow_ma_types: ['ema'],
      fast_ma_params: { alpha: 2 },
      slow_ma_params: { alpha: 3 },
      fast_ma_param_sweeps: { alpha: [1, 3, 1] },
      strategy: {
        long_only: true,
        allow_flip: true,
        trade_on_next_bar: false,
      },
      constraints: {
        min_trades: 5,
        max_drawdown: 0.25,
      },
    })
  })

  it('returns null for non-DoubleMa strategy kind', () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'CompositeMo',
        data_id: 'data-123',
        current_data_meta: { has_close: true },
      },
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    })
    expect(req).toBeNull()
  })

  it('enforces includeAll combo guardrail', () => {
    expect(() => buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'DoubleMa',
        data_id: 'data-123',
        current_data_meta: { has_close: true },
      },
      backend: 'Auto',
      fastStart: '1',
      fastEnd: '2000',
      fastStep: '1',
      slowStart: '2',
      slowEnd: '4000',
      slowStep: '1',
      fastMaType: 'sma',
      slowMaType: 'ema',
      maSource: 'close',
      includeAll: 'true',
      fastMaParamSweeps: { alpha: [1, 100, 1] },
      slowMaParamSweeps: { beta: [1, 100, 1] },
    })).toThrow('includeAll=true would return')
  })

  it('builds a CompositeCpu optimization request from typed inputs', () => {
    const req = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: 'CompositeCpu',
        data_id: 'data-abc',
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
      },
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
      topK: '20',
      includeAll: 'false',
      compositeSignalMode: 'hard_gate',
      halfTrendAmplitude: '2',
      halfTrendAtrPeriod: '100',
      halfTrendChannelDev: '2.0',
      maczFastLength: '12',
      maczSlowLength: '25',
      maczSignalLength: '9',
      maczLengthZ: '20',
      maczLengthStdev: '25',
      maczGamma: '0.02',
      maczA: '1',
      maczB: '1',
      maczUseVolume: false,
      maczUseLag: false,
      ottPeriod: '2',
      ottPercent: '1.4',
      ottMaType: 'VAR',
      startingEquity: '10000',
      costPreset: 'custom',
      feeBps: '1',
      spreadBps: '2',
      slippageBps: '0.5',
      epsRel: '0.001',
      stopLossPct: '',
      takeProfitPct: '',
      trailingStopPct: '',
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
    })

    expect(req).toBeTruthy()
    expect(req).toMatchObject({
      strategy_kind: 'CompositeCpu',
      data_id: 'data-abc',
      backend: { backend: 'CpuOnly' },
      mode: 'Grid',
      top_k: 20,
      include_all: false,
      heatmap_bins: 0,
      signal_mode: 'hard_gate',
      fast_ma_types: ['sma'],
      slow_ma_types: ['ema'],
      halftrend: {
        amplitude: 2,
        atr_period: 100,
        channel_deviation: 2,
      },
      strategy: {
        long_only: true,
        allow_flip: true,
        trade_on_next_bar: false,
      },
    })
  })

  it('returns null for non-CompositeCpu strategy kind', () => {
    const req = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: 'DoubleMa',
        data_id: 'data-123',
        current_data_meta: { has_high: true, has_low: true, has_close: true },
      },
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    })
    expect(req).toBeNull()
  })

  it('builds a CompositeMo optimization request from typed inputs', () => {
    const req = buildCompositeMoOptimizationRequest({
      context: {
        strategy_kind: 'CompositeMo',
        data_id: 'data-mo',
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
      },
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
      moObjectiveMin: 'MaxDrawdown',
      compositeSignalMode: 'hard_gate',
      moSampler: 'Nsga2',
      moNsga2Seed: '1',
      moNsga2Population: '80',
      moNsga2Generations: '40',
      moNsga2CrossoverProb: '0.9',
      moNsga2SwappingProb: '0.5',
      moNsga2MutationProb: '',
      moUseRangeBuilder: false,
      moSearchSpace: '',
      startingEquity: '10000',
      costPreset: 'custom',
      feeBps: '1',
      spreadBps: '2',
      slippageBps: '0.5',
      epsRel: '0.001',
      stopLossPct: '',
      takeProfitPct: '',
      trailingStopPct: '',
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
    })

    expect(req).toBeTruthy()
    expect(req).toMatchObject({
      strategy_kind: 'CompositeMo',
      data_id: 'data-mo',
      objective: 'Sharpe',
      objectives: { maximize: 'Sharpe', minimize: 'MaxDrawdown' },
      fast_ma_type: 'sma',
      slow_ma_type: 'ema',
      signal_mode: 'hard_gate',
      sampler: {
        sampler: 'Nsga2',
      },
      strategy: {
        long_only: true,
        allow_flip: true,
        trade_on_next_bar: false,
      },
    })
    expect(Array.isArray((req as { search_space?: { params?: unknown[] } }).search_space?.params)).toBe(true)
  })

  it('returns null for non-CompositeMo strategy kind', () => {
    const req = buildCompositeMoOptimizationRequest({
      context: {
        strategy_kind: 'DoubleMa',
        data_id: 'data-123',
        current_data_meta: { has_high: true, has_low: true, has_close: true },
      },
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    })
    expect(req).toBeNull()
  })
})
