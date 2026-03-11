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
        trade_on_next_bar: true,
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

  it('normalizes removed approximate-mode aliases back to exact Grid mode', () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'DoubleMa',
        data_id: 'data-123',
        current_data_meta: { has_close: true },
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
      mode: 'GpuProgressive',
      topK: '20',
      includeAll: 'false',
      heatmapBins: '0',
      exportAllCsvPath: '',
      vdpsEta: '4',
      vdpsRungs: '0.125,0.25,0.5,1.0',
      vdpsMinInitialPairs: '1024',
      vdpsMaxSurvivorSeeds: '128',
      vdpsMaxPeriodCandidates: '256',
      vdpsRandomInjectionRatio: '0.1',
      vdpsEliteMutationRatio: '0.5',
      vdpsDifferentialRatio: '0.4',
      vdpsDeterministic: true,
      vdpsSeed: '7',
      vdpsQualityCheckMaxPairs: '0',
      vdpsQualityCheckTopK: '20',
    })

    expect(req).toBeTruthy()
    expect(req?.mode).toBe('Grid')
  })

  it('preserves Marrt strategy kind on the double-ma request path', () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'Marrt',
        data_id: 'data-ribbon',
        current_data_meta: { has_close: true, has_volume: true },
      },
      maSupportMap: {
        alma: { supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
        vwma: { supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
      },
      backend: 'GpuKernel',
      fastStart: '5',
      fastEnd: '10',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '20',
      fastMaType: 'alma',
      slowMaType: 'vwma',
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'Grid',
      topK: '10',
      includeAll: 'false',
      heatmapBins: '0',
      exitMode: 'cross',
      cooldownBars: '0',
      signalPersistenceBars: '1',
    })

    expect(req).toBeTruthy()
    expect(req?.strategy_kind).toBe('Marrt')
  })

  it('preserves Set strategy kind and emits slope-efficiency config on the double-ma path', () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'Set',
        data_id: 'data-set',
        current_data_meta: { has_close: true, has_high: true, has_low: true },
      },
      maSupportMap: {
        sma: { supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
      },
      backend: 'GpuKernel',
      fastStart: '5',
      fastEnd: '10',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '20',
      fastMaType: 'sma',
      slowMaType: 'sma',
      maSource: 'close',
      exitMode: 'cross',
      cooldownBars: '0',
      signalPersistenceBars: '1',
    })

    expect(req?.strategy_kind).toBe('Set')
    expect(req?.strategy).toMatchObject({
      slope_efficiency_trend_enable: true,
      set_linearreg_period: 34,
      set_angle_min_deg: 2.0,
      set_er_period: 14,
      set_er_min: 0.3,
      set_chop_period: 14,
      set_chop_max: 45.0,
    })
  })

  it('preserves Pit strategy kind and emits pressure-impulse config on the double-ma path', () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'Pit',
        data_id: 'data-pit',
        current_data_meta: { has_open: true, has_close: true, has_high: true, has_low: true, has_volume: true },
      },
      maSupportMap: {
        sma: { supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
      },
      backend: 'GpuKernel',
      fastStart: '5',
      fastEnd: '10',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '20',
      fastMaType: 'sma',
      slowMaType: 'sma',
      maSource: 'close',
      pitEfiPeriod: '11',
      pitVoscShortPeriod: '4',
      pitVoscLongPeriod: '12',
      pitCfoPeriod: '7',
      pitCfoScalar: '80',
      pitConfirmBars: '3',
      exitMode: 'cross',
      cooldownBars: '0',
      signalPersistenceBars: '1',
    })

    expect(req?.strategy_kind).toBe('Pit')
    expect(req?.strategy).toMatchObject({
      pressure_impulse_trend_enable: true,
      pit_efi_period: 11,
      pit_vosc_short_period: 4,
      pit_vosc_long_period: 12,
      pit_cfo_period: 7,
      pit_cfo_scalar: 80,
      pit_confirm_bars: 3,
    })
  })

  it('preserves Mts strategy kind and emits momentum-stack config on the double-ma path', () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'Mts',
        data_id: 'data-mts',
        current_data_meta: { has_close: true },
      },
      maSupportMap: {
        sma: { supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
      },
      backend: 'GpuKernel',
      fastStart: '5',
      fastEnd: '10',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '20',
      fastMaType: 'sma',
      slowMaType: 'sma',
      maSource: 'close',
      exitMode: 'cross',
      cooldownBars: '0',
      signalPersistenceBars: '1',
    })

    expect(req?.strategy_kind).toBe('Mts')
    expect(req?.strategy).toMatchObject({
      momentum_triple_stack_enable: true,
      mts_roc_period: 10,
      mts_mom_period: 20,
      mts_ppo_fast_period: 12,
      mts_ppo_slow_period: 26,
    })
  })

  it('preserves Orc strategy kind and emits oscillator-reversion config on the double-ma path', () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'Orc',
        data_id: 'data-orc',
        current_data_meta: { has_close: true, has_high: true, has_low: true },
      },
      backend: 'GpuKernel',
      fastStart: '5',
      fastEnd: '9',
      fastStep: '2',
      slowStart: '20',
      slowEnd: '30',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
      maSource: 'close',
    })

    expect(req?.strategy_kind).toBe('Orc')
    expect(req?.strategy).toMatchObject({
      oscillator_reversion_confluence_enable: true,
      orc_rsi_period: 14,
      orc_stoch_fastk_period: 5,
      orc_stoch_fastd_period: 3,
      orc_willr_period: 14,
    })
  })

  it('rejects unsupported MA types for Marrt', () => {
    expect(() => buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'Marrt',
        data_id: 'data-ribbon',
        current_data_meta: { has_close: true },
      },
      fastStart: '5',
      fastEnd: '10',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '20',
      fastMaType: 'kama',
      slowMaType: 'alma',
      maSource: 'close',
    })).toThrow('MARRT currently supports only sma, ema, alma, wma, zlema, and vwma.')
  })

  it('preserves Vfct strategy kind and emits volume-flow config on the double-ma path', () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'Vfct',
        data_id: 'data-vfct',
        current_data_meta: { has_close: true, has_high: true, has_low: true, has_volume: true },
      },
      maSupportMap: {
        vwma: { supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
      },
      backend: 'GpuKernel',
      fastStart: '5',
      fastEnd: '10',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '20',
      fastMaType: 'vwma',
      slowMaType: 'vwma',
      maSource: 'close',
      vfctObvSmoothPeriod: '21',
      vfctMfiPeriod: '14',
      vfctMfiLongMin: '55',
      vfctMfiShortMax: '45',
      vfctConfirmBars: '2',
      exitMode: 'cross',
      cooldownBars: '0',
      signalPersistenceBars: '1',
    })

    expect(req?.strategy_kind).toBe('Vfct')
    expect(req?.strategy).toMatchObject({
      volume_flow_confirm_enable: true,
      volume_flow_obv_smooth_period: 21,
      volume_flow_mfi_period: 14,
      volume_flow_mfi_long_min: 55,
      volume_flow_mfi_short_max: 45,
      volume_flow_confirm_bars: 2,
    })
  })

  it('rejects unsupported MA types for Vfct', () => {
    expect(() => buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'Vfct',
        data_id: 'data-vfct',
        current_data_meta: { has_close: true, has_high: true, has_low: true, has_volume: true },
      },
      fastStart: '5',
      fastEnd: '10',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '20',
      fastMaType: 'ema',
      slowMaType: 'vwma',
      maSource: 'close',
    })).toThrow('VFCT currently supports only vwma fast/slow trend lines.')
  })

  it('normalizes legacy GpuHybridKernel backend to unified GpuKernel payload', () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'DoubleMa',
        data_id: 'data-hybrid',
        current_data_meta: { has_close: true },
      },
      maSupportMap: {
        sma: { supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
        alma: { supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
      },
      backend: 'GpuHybridKernel',
      fastStart: '5',
      fastEnd: '7',
      fastStep: '1',
      slowStart: '10',
      slowEnd: '12',
      slowStep: '1',
      fastMaType: 'sma',
      slowMaType: 'alma',
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'Grid',
      topK: '10',
      includeAll: 'false',
      heatmapBins: '0',
      exportAllCsvPath: '',
      exitMode: 'cross',
      cooldownBars: '0',
      signalPersistenceBars: '1',
    })
    expect(req).toBeTruthy()
    expect(req?.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
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
      mode: 'CoarseToFine',
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
        trade_on_next_bar: true,
      },
    })
  })

  it('builds a CompositeTrend request through the double-MA builder path with GPU kernel when ADX is off', () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'CompositeTrend',
        data_id: 'data-trend',
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
      backend: 'GpuKernel',
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
      trendUseRoc: true,
      trendUseAdx: false,
      trendLrPeriod: '34',
      trendLrMinDeg: '2.0',
      trendBbwPeriod: '20',
      trendBbwMult: '2.0',
      trendBbwMin: '0.04',
      trendRocPeriod: '9',
      trendAdxPeriod: '14',
      trendAdxMin: '20',
      fastMaParams: { alpha: 2 },
      slowMaParams: { alpha: 3 },
    })

    const strategy = (req?.strategy && typeof req.strategy === 'object')
      ? req.strategy as Record<string, unknown>
      : {}
    expect(req).toBeTruthy()
    expect(req?.strategy_kind).toBe('CompositeTrend')
    expect((req as Record<string, unknown>)?.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
    expect(strategy.trend_quality_enable).toBe(true)
    expect(strategy.regime_hybrid_enable).toBe(false)
    expect(strategy.breakout_momentum_enable).toBe(false)
  })

  it('builds a CompositeTrend request through the double-MA builder path with GPU hybrid when ADX is on', () => {
    const req = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'CompositeTrend',
        data_id: 'data-trend',
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
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
      trendUseRoc: true,
      trendUseAdx: true,
      trendLrPeriod: '34',
      trendLrMinDeg: '2.0',
      trendBbwPeriod: '20',
      trendBbwMult: '2.0',
      trendBbwMin: '0.04',
      trendRocPeriod: '9',
      trendAdxPeriod: '14',
      trendAdxMin: '20',
    })

    expect(req).toBeTruthy()
    expect((req as Record<string, unknown>)?.backend).toEqual({ backend: 'GpuHybridKernel', device_id: 0 })
  })

  it('builds RegimeHybrid and BreakoutMomentum through the double-MA builder path', () => {
    const regimeReq = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'RegimeHybrid',
        data_id: 'data-regime',
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
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
      regimeAdxPeriod: '14',
      regimeAdxEnter: '22',
      regimeAdxExit: '18',
      regimeRevEntryRel: '0.0035',
      regimeRevExitRel: '0.0012',
    })

    const breakoutReq = buildDoubleMaOptimizationRequest({
      context: {
        strategy_kind: 'BreakoutMomentum',
        data_id: 'data-breakout',
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
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
      breakoutBbPeriod: '20',
      breakoutBbMult: '2.0',
      breakoutConfirmBars: '1',
      breakoutMacdFast: '12',
      breakoutMacdSlow: '26',
      breakoutMacdSignal: '9',
      breakoutCmoPeriod: '14',
      breakoutCmoMinAbs: '20',
      breakoutAdoscShort: '3',
      breakoutAdoscLong: '10',
      breakoutUseAdx: false,
      breakoutAdxPeriod: '14',
      breakoutAdxMin: '20',
    })

    expect(regimeReq).toBeTruthy()
    expect((regimeReq as Record<string, unknown>)?.backend).toEqual({ backend: 'GpuHybridKernel', device_id: 0 })
    expect((regimeReq as Record<string, unknown>)?.strategy_kind).toBe('RegimeHybrid')

    expect(breakoutReq).toBeTruthy()
    expect((breakoutReq as Record<string, unknown>)?.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
    expect((breakoutReq as Record<string, unknown>)?.strategy_kind).toBe('BreakoutMomentum')
  })

  it('keeps CompositeTrend, RegimeHybrid, and BreakoutMomentum on the double-MA path with CpuOnly fallback when requested', () => {
    const baseContext = {
      current_data_meta: {
        has_open: true,
        has_high: true,
        has_low: true,
        has_close: true,
        has_volume: true,
      },
    }

    const trendReq = buildDoubleMaOptimizationRequest({
      context: { strategy_kind: 'CompositeTrend', data_id: 'data-trend-cpu', ...baseContext },
      backend: 'CpuOnly',
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
      trendUseAdx: false,
    })

    const regimeReq = buildDoubleMaOptimizationRequest({
      context: { strategy_kind: 'RegimeHybrid', data_id: 'data-regime-cpu', ...baseContext },
      backend: 'CpuOnly',
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
    })

    const breakoutReq = buildDoubleMaOptimizationRequest({
      context: { strategy_kind: 'BreakoutMomentum', data_id: 'data-breakout-cpu', ...baseContext },
      backend: 'CpuOnly',
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
      breakoutUseAdx: false,
    })

    expect((trendReq as Record<string, unknown>).backend).toEqual({ backend: 'CpuOnly' })
    expect((regimeReq as Record<string, unknown>).backend).toEqual({ backend: 'CpuOnly' })
    expect((breakoutReq as Record<string, unknown>).backend).toEqual({ backend: 'CpuOnly' })
  })

  it('builds a VCC request through the composite CPU builder path', () => {
    const req = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: 'Vcc',
        data_id: 'data-vcc',
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
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
      cooldownBars: '0',
      signalPersistenceBars: '1',
      exitMode: 'cross',
      kaufmanStopPeriod: '22',
      kaufmanStopMult: '2.0',
      kaufmanStopMaType: 'sma',
      longOnly: true,
      allowFlip: true,
      tradeOnNext: false,
      vccMapFastToSqueezePeriod: false,
      vccMapFastToMomentumPeriod: true,
      vccMapSlowToTrendPeriod: false,
      vccSqueezePeriod: '18',
      vccMomentumPeriod: '22',
      vccMomentumBbPeriod: '24',
      vccTrendPeriod: '9',
      vccSqueezeBbMult: '2.2',
      vccSqueezeKcMult: '1.7',
      vccConfirmBars: '3',
      vccRequireMomentumAccel: false,
      vccMaxTradeBars: '140',
      fastMaParams: { alpha: 2 },
      slowMaParams: { alpha: 3 },
    })

    expect(req).toBeTruthy()
    expect(req?.strategy_kind).toBe('Vcc')
    expect(req?.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
    expect(req?.mode).toBe('Grid')
    expect(req?.vcc).toMatchObject({
      map_fast_to_squeeze_period: false,
      map_fast_to_momentum_period: true,
      map_slow_to_trend_period: false,
      squeeze_period: 18,
      momentum_period: 22,
      momentum_bb_period: 24,
      trend_period: 9,
      squeeze_bb_mult: 2.2,
      squeeze_kc_mult: 1.7,
      confirm_bars: 3,
      require_momentum_accel: false,
      max_trade_bars: 140,
    })
  })

  it('builds a VEMR request through the composite CPU builder path', () => {
    const req = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: 'Vemr',
        data_id: 'data-vemr',
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
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
      mode: 'CoarseToFine',
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
      cooldownBars: '0',
      signalPersistenceBars: '1',
      exitMode: 'cross',
      kaufmanStopPeriod: '22',
      kaufmanStopMult: '2.0',
      kaufmanStopMaType: 'sma',
      longOnly: true,
      allowFlip: true,
      tradeOnNext: false,
      vemrMapFastToZPeriod: false,
      vemrMapFastToBbPeriod: true,
      vemrMapSlowToAtrPeriod: false,
      vemrZPeriod: '18',
      vemrZMaType: 'sma',
      vemrZNbdev: '1.2',
      vemrZDevtype: '0',
      vemrZEntry: '2.4',
      vemrZExit: '0.7',
      vemrBbPeriod: '21',
      vemrBbMult: '2.3',
      vemrRequireBandTouch: true,
      vemrAtrPeriod: '17',
      vemrAtrStopMult: '2.1',
      vemrConfirmBars: '2',
      vemrMaxTradeBars: '130',
      fastMaParams: { alpha: 2 },
      slowMaParams: { alpha: 3 },
    })

    expect(req).toBeTruthy()
    expect(req?.strategy_kind).toBe('Vemr')
    expect(req?.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
    expect(req?.mode).toBe('Grid')
    expect(req?.vemr).toMatchObject({
      map_fast_to_z_period: false,
      map_fast_to_bb_period: true,
      map_slow_to_atr_period: false,
      z_period: 18,
      z_ma_type: 'sma',
      z_nbdev: 1.2,
      z_devtype: 0,
      z_entry: 2.4,
      z_exit: 0.7,
      bb_period: 21,
      bb_mult: 2.3,
      require_band_touch: true,
      atr_period: 17,
      atr_stop_mult: 2.1,
      confirm_bars: 2,
      max_trade_bars: 130,
    })
  })

  it('builds a ROCB request through the composite CPU builder path', () => {
    const req = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: 'Rocb',
        data_id: 'data-rocb',
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
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
      rocbMapFastToChannelPeriod: false,
      rocbMapSlowToAtrPeriod: false,
      rocbChannelPeriod: '22',
      rocbAtrPeriod: '16',
      rocbAtrStopMult: '2.3',
      rocbMaxTradeBars: '90',
    })

    expect(req).toBeTruthy()
    expect(req?.strategy_kind).toBe('Rocb')
    expect(req?.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
    expect(req?.rocb).toMatchObject({
      map_fast_to_channel_period: false,
      map_slow_to_atr_period: false,
      channel_period: 22,
      atr_period: 16,
      atr_stop_mult: 2.3,
      max_trade_bars: 90,
    })
  })

  it('builds an MKCS request through the composite CPU builder path', () => {
    const req = buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: 'Mkcs',
        data_id: 'data-mkcs',
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
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
    })

    expect(req).toBeTruthy()
    expect(req?.strategy_kind).toBe('Mkcs')
    expect(req?.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
    expect(req?.mkcs).toMatchObject({
      map_fast_to_stddev_period: true,
      map_slow_to_breakout_lookback: true,
      stddev_period: 20,
      breakout_lookback: 20,
      compression_threshold: 0.01,
      reexpansion_threshold: 0.03,
    })
  })

  it('builds the staged GPU-native expansion requests through the composite CPU builder path', () => {
    const cases = [
      {
        kind: 'Kqsr',
        field: 'kqsr',
        expected: {
          kdj_period: 14,
          srsi_period: 14,
          srsi_smooth: 3,
          ift_rsi_period: 9,
          qqe_period: 14,
          oversold: 20,
          overbought: 80,
          prb_period: 20,
          prb_threshold: 0.2,
        },
      },
      {
        kind: 'Carm',
        field: 'carm',
        expected: {
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
        },
      },
      {
        kind: 'Kvdf',
        field: 'kvdf',
        expected: {
          keltner_period: 20,
          keltner_mult: 1.5,
          vwap_anchor: 'session',
          fvg_lookback: 20,
          devstop_period: 14,
          devstop_mult: 2,
          dti_period: 14,
          dti_threshold: 20,
        },
      },
      {
        kind: 'Cdmc',
        field: 'cdmc',
        expected: {
          dpo_period: 20,
          correlation_cycle_period: 20,
          mama_fast_limit: 0.5,
          mama_slow_limit: 0.05,
          decycler_period: 20,
          phase_threshold: 0,
        },
      },
    ] as const

    for (const row of cases) {
      const req = buildCompositeCpuOptimizationRequest({
        context: {
          strategy_kind: row.kind,
          data_id: `data-${row.kind.toLowerCase()}`,
          current_data_meta: {
            has_open: true,
            has_high: true,
            has_low: true,
            has_close: true,
            has_volume: true,
          },
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
      })

      expect(req).toBeTruthy()
      expect(req?.strategy_kind).toBe(row.kind)
      expect(req?.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
      expect((req as Record<string, unknown>)[row.field]).toMatchObject(row.expected)
    }
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
        trade_on_next_bar: true,
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
