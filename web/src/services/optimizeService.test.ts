import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  applyPreset,
  cancelOptimization,
  presetKernel,
  refreshGpuStatus,
  runGpuParityCheck,
  runOptimization,
  runValidation,
} from './optimizeService'
import { requestCancelCurrentRun } from './runControlService'
import { invokeCommand } from '../tauri/invoke'
import { runValidation as runValidationAction } from './validationService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useOptimizeStore } from '../stores/optimizeStore'

vi.mock('./runControlService', () => ({
  requestCancelCurrentRun: vi.fn(async () => ({
    requested: true,
    usedLegacyFallback: false,
    backgroundTaskRequested: true,
  })),
}))

vi.mock('./validationService', () => ({
  runValidation: vi.fn(async () => undefined),
}))

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => ({
    built_with_cuda: true,
    built_with_cuda_backtest_kernel: true,
    cuda_available: true,
    auto_device_name: 'NVIDIA RTX 4090',
    auto_device_memory_bytes: 24 * 1024 * 1024 * 1024,
  })),
}))
const runValidationActionMock = vi.mocked(runValidationAction)
const requestCancelCurrentRunMock = vi.mocked(requestCancelCurrentRun)
const invokeCommandMock = vi.mocked(invokeCommand)
let pinia: ReturnType<typeof createPinia>

function installDefaultInvokeCommandMock(): void {
  invokeCommandMock.mockImplementation(async (command: string) => {
    if (command === 'get_gpu_status') {
      return {
        built_with_cuda: true,
        built_with_cuda_backtest_kernel: true,
        cuda_available: true,
        auto_device_name: 'NVIDIA RTX 4090',
        auto_device_memory_bytes: 24 * 1024 * 1024 * 1024,
      }
    }
    if (command === 'list_moving_averages') {
      return [
        { id: 'sma', supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
        { id: 'ema', supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
        { id: 'alma', supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
      ]
    }
    if (command === 'run_double_ma_optimization') {
      return {
        best_metrics: { pnl: 1, sharpe: 0.5, max_dd: 0.2 },
        best_params: { fast_len: 5, slow_len: 20 },
      }
    }
    if (command === 'run_composite_cpu_optimization') {
      return {
        top: [],
        best_metrics: { pnl: 1, sharpe: 0.5, max_dd: 0.2 },
        best_params: { fast_len: 5, slow_len: 20 },
      }
    }
    if (command === 'run_composite_mo_optimization') {
      return {
        top: [],
        best_metrics: { pnl: 1, sharpe: 0.5, max_dd: 0.2 },
        best_params: { fast_len: 5, slow_len: 20 },
      }
    }
    if (command === 'save_run_artifact') {
      return {
        path: 'runs/vectorgrid_run_test.json',
        bytes: 128,
      }
    }
    return {}
  })
}

function firstOptimizationRunCall(): [string, Record<string, unknown>] {
  const runCall = invokeCommandMock.mock.calls.find(([command]) =>
    command === 'run_double_ma_optimization'
    || command === 'run_composite_cpu_optimization'
    || command === 'run_composite_mo_optimization',
  )
  expect(runCall).toBeTruthy()
  return [String(runCall?.[0] || ''), (runCall?.[1] ?? {}) as Record<string, unknown>]
}

function firstOptimizationRunRequest(): Record<string, unknown> {
  const [, args] = firstOptimizationRunCall()
  return (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
}

describe('optimizeService', () => {
  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    vi.clearAllMocks()
    installDefaultInvokeCommandMock()
    if (typeof window !== 'undefined') {
      window.__VTA_LEGACY_BOOTED__ = false
    }
  })

  it('applies preset profile into Vue stores without legacy optimize runtime wrappers', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      presetProfile: 'Deep',
      canCancel: false,
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      topK: '20',
      includeAll: 'false',
      heatmapBins: '64',
    }
    ;(pinia.state.value as Record<string, unknown>).validation = {
      crossDatasets: [],
      crossSelectedCount: 0,
    }

    await applyPreset()

    const advanced = (pinia.state.value as Record<string, unknown>).advanced as Record<string, unknown>
    const validation = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    const optimize = (pinia.state.value as Record<string, unknown>).optimize as Record<string, unknown>
    expect(advanced.topK).toBe('50')
    expect(advanced.includeAll).toBe('false')
    expect(advanced.heatmapBins).toBe('256')
    expect(validation.crossEnable).toBe(false)
    expect(optimize.statusLine).toContain('Applied preset: Deep')
    expect(optimize.statusLine).toContain('cross-ticker skipped')
  })

  it('applies GPU-kernel preset into Vue stores and refreshes GPU status', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      canCancel: false,
      fastMaType: 'ema',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'hlc3',
    }

    await presetKernel()

    const optimize = (pinia.state.value as Record<string, unknown>).optimize as Record<string, unknown>
    const advanced = (pinia.state.value as Record<string, unknown>).advanced as Record<string, unknown>
    expect(optimize.backend).toBe('GpuKernel')
    expect(optimize.fastMaType).toBe('sma')
    expect(optimize.slowMaType).toBe('alma')
    expect(optimize.statusLine).toBe('Applied preset: GPU kernel (SMA/ALMA)')
    expect(optimize.gpuStatus).toContain('build=cuda')
    expect(optimize.gpuDeviceMeta).toContain('Automatic CUDA device')
    expect(advanced.maSource).toBe('close')
    expect(invokeCommandMock).toHaveBeenCalledWith('get_gpu_status')
  })

  it('refreshes GPU status with automatic CUDA device selection', async () => {
    const summary = await refreshGpuStatus()

    expect(summary.autoDeviceName).toBe('NVIDIA RTX 4090')
    expect(summary.autoDeviceMemoryBytes).toBe(24 * 1024 * 1024 * 1024)
    expect(summary.deviceMeta).toContain('Automatic CUDA device')
  })

  it('runs CPU↔GPU parity check via Tauri commands', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'DoubleMa',
      canCancel: false,
      backend: 'Auto',
      fastStart: '5',
      fastEnd: '9',
      fastStep: '2',
      slowStart: '20',
      slowEnd: '30',
      slowStep: '5',
      fastMaType: 'sma',
      slowMaType: 'alma',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'Grid',
      topK: '20',
      includeAll: 'false',
      heatmapBins: '64',
      exportAllCsvPath: '',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
      startingEquity: '10000',
      feeBps: '0',
      spreadBps: '0',
      slippageBps: '0',
      epsRel: '0',
      cooldownBars: '0',
      signalPersistenceBars: '1',
      exitMode: 'cross',
      kaufmanStopPeriod: '22',
      kaufmanStopMult: '2.0',
      kaufmanStopMaType: 'sma',
      longOnly: true,
      allowFlip: true,
      tradeOnNext: false,
      costPreset: 'custom',
      stopLossPct: '',
      takeProfitPct: '',
      dmConstraintMinTrades: '',
      dmConstraintMaxDd: '',
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-parity',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '200000',
      dataTimeframe: '1m',
    }

    invokeCommandMock.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
      if (command === 'get_gpu_status') {
        return {
          built_with_cuda: true,
          built_with_cuda_backtest_kernel: true,
          cuda_available: true,
          auto_device_name: 'NVIDIA RTX 4090',
          auto_device_memory_bytes: 24 * 1024 * 1024 * 1024,
        }
      }
      if (command === 'list_moving_averages') {
        return [
          { id: 'sma', supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
          { id: 'alma', supports_cpu: true, supports_cuda: true, supports_cuda_kernel: true },
        ]
      }
      if (command === 'run_double_ma_optimization') {
        const req = args?.req as Record<string, unknown>
        const backend = (req?.backend as Record<string, unknown> | null)?.backend
        if (backend === 'CpuOnly') {
          return {
            best_metrics: { pnl: 100, sharpe: 1.2345, max_dd: 12.5 },
            best_params: { fast_len: 5, slow_len: 20 },
          }
        }
        return {
          best_metrics: { pnl: 100, sharpe: 1.2345, max_dd: 12.5 },
          best_params: { fast_len: 5, slow_len: 20 },
        }
      }
      return {}
    })

    await runGpuParityCheck()

    const optimize = (pinia.state.value as Record<string, unknown>).optimize as Record<string, unknown>
    expect(optimize.gpuParityMeta).toContain('Parity: OK')
    expect(optimize.gpuParityMeta).toContain('gpu=GpuKernel')
    expect(optimize.statusLine).toBe('Parity check OK.')
    expect(optimize.gpuDeviceMeta).toContain('Automatic CUDA device')
    expect(invokeCommandMock).toHaveBeenCalledWith('run_double_ma_optimization', expect.any(Object))
  })

  it('runs optimization command path and validation service dispatch', async () => {
    const analysis = useAnalysisStore()
    await runOptimization({
      request: { strategy_kind: 'DoubleMa', data_id: 'data-1', top_k: 20, heatmap_bins: 32 },
    })
    await runValidation()

    const [command, args] = firstOptimizationRunCall()
    expect(command).toBe('run_double_ma_optimization')
    expect((args.req as { strategy_kind?: string } | undefined)?.strategy_kind).toBe('DoubleMa')
    expect(invokeCommandMock).toHaveBeenCalledWith('save_run_artifact', expect.any(Object))
    const saveCall = invokeCommandMock.mock.calls.find(([nextCommand]) => nextCommand === 'save_run_artifact')
    const savedArtifact = (saveCall?.[1] as { artifact?: Record<string, unknown> } | undefined)?.artifact
    expect(savedArtifact?.kind).toBe('vectorgrid_run_artifact')
    expect(analysis.topRows).toBeGreaterThan(0)
    expect(String(analysis.bestTitle || '')).toContain('vs')
    expect(runValidationActionMock).toHaveBeenCalled()
  })

  it('includes backend execution route details in status line when available', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      statusLine: '',
      progressMeta: '',
      runDisabled: false,
      canCancel: false,
      validateDisabled: false,
    }

    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'run_double_ma_optimization') {
        return {
          best_metrics: { pnl: 1, sharpe: 0.5, max_dd: 0.2 },
          best_params: { fast_len: 5, slow_len: 20 },
          backend_used: 'Cpu',
          backend_route: 'GpuHybridKernel->CpuOnly',
          backend_fallback_reason: 'Hybrid phase-1 fallback to CpuOnly.',
          indicator_route_plan: {
            fast: {
              indicator_id: 'sma',
              capability: 'gpu_kernel',
              route: 'gpu_kernel',
              gpu_preferred_by_speedup: true,
            },
            slow: {
              indicator_id: 'kama',
              capability: 'gpu_kernel',
              route: 'cpu_staged',
              gpu_preferred_by_speedup: false,
            },
          },
        }
      }
      if (command === 'save_run_artifact') {
        return { path: 'runs/vectorgrid_run_test.json', bytes: 128 }
      }
      if (command === 'get_gpu_status') {
        return {
          built_with_cuda: true,
          built_with_cuda_backtest_kernel: true,
          cuda_available: true,
          auto_device_name: 'NVIDIA RTX 4090',
          auto_device_memory_bytes: 24 * 1024 * 1024 * 1024,
        }
      }
      return {}
    })

    await runOptimization({
      request: { strategy_kind: 'DoubleMa', data_id: 'data-1' },
      switchTab: false,
    })

    const optimize = (pinia.state.value as Record<string, unknown>).optimize as Record<string, unknown>
    const statusLine = String(optimize.statusLine || '')
    expect(statusLine).toContain('backend=Cpu')
    expect(statusLine).toContain('route=GpuHybridKernel->CpuOnly')
    expect(statusLine).toContain('indicators=fast:sma->gpu_kernel, slow:kama->cpu_staged')
    expect(statusLine).toContain('policy_cpu_route=slow:kama (gpu_speedups.csv)')
    expect(statusLine).toContain('fallback=Hybrid phase-1 fallback to CpuOnly.')
  })

  it('surfaces backend route metadata for CompositeTrend, RegimeHybrid, and BreakoutMomentum', async () => {
    const optimizeStore = useOptimizeStore()
    const cases = [
      {
        strategy_kind: 'CompositeTrend',
        backend_used: 'Gpu',
        backend_route: 'GpuKernel',
        backend_fallback_reason: null,
      },
      {
        strategy_kind: 'RegimeHybrid',
        backend_used: 'Gpu',
        backend_route: 'GpuHybridKernel',
        backend_fallback_reason: null,
      },
      {
        strategy_kind: 'BreakoutMomentum',
        backend_used: 'Cpu',
        backend_route: 'GpuHybridKernel->CpuOnly',
        backend_fallback_reason: 'Hybrid profile fallback to CpuOnly.',
      },
    ]

    for (const row of cases) {
      invokeCommandMock.mockClear()
      installDefaultInvokeCommandMock()
      invokeCommandMock.mockImplementation(async (command: string) => {
        if (command === 'run_double_ma_optimization') {
          return {
            best_metrics: { pnl: 1, sharpe: 0.5, max_dd: 0.2 },
            best_params: { fast_len: 5, slow_len: 20 },
            backend_used: row.backend_used,
            backend_route: row.backend_route,
            backend_fallback_reason: row.backend_fallback_reason,
          }
        }
        if (command === 'save_run_artifact') {
          return { path: 'runs/vectorgrid_run_test.json', bytes: 128 }
        }
        if (command === 'get_gpu_status') {
          return {
            built_with_cuda: true,
            built_with_cuda_backtest_kernel: true,
            cuda_available: true,
            auto_device_name: 'NVIDIA RTX 4090',
            auto_device_memory_bytes: 24 * 1024 * 1024 * 1024,
          }
        }
        return {}
      })

      await runOptimization({
        request: { strategy_kind: row.strategy_kind, data_id: `route-${row.strategy_kind}` },
        switchTab: false,
      })

      const statusLine = String(optimizeStore.statusLine || '')
      expect(statusLine).toContain(`backend=${row.backend_used}`)
      expect(statusLine).toContain(`route=${row.backend_route}`)
      if (row.backend_fallback_reason) {
        expect(statusLine).toContain(`fallback=${row.backend_fallback_reason}`)
      }
    }
  })

  it('runs optimization via injected request when request payload is provided', async () => {
    await runOptimization({
      request: { strategy_kind: 'DoubleMa', data_id: 'data-123' },
      switchTab: false,
    })

    const [command, args] = firstOptimizationRunCall()
    expect(command).toBe('run_double_ma_optimization')
    expect(args.req).toMatchObject({ strategy_kind: 'DoubleMa', data_id: 'data-123' })
  })

  it('auto-builds DoubleMa optimization request from Vue stores', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'DoubleMa',
      backend: 'Auto',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'Grid',
      topK: '20',
      includeAll: 'false',
      heatmapBins: '64',
      exportAllCsvPath: '',
      vdpsDeterministic: true,
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
      startingEquity: '10000',
      costPreset: 'custom',
      feeBps: '1',
      spreadBps: '2',
      slippageBps: '0',
      epsRel: '0',
      stopLossPct: '',
      takeProfitPct: '',
      cooldownBars: '0',
      signalPersistenceBars: '1',
      exitMode: 'cross',
      kaufmanStopPeriod: '22',
      kaufmanStopMult: '2.0',
      kaufmanStopMaType: 'sma',
      dmConstraintMinTrades: '',
      dmConstraintMaxDd: '',
      longOnly: true,
      allowFlip: true,
      tradeOnNext: false,
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-456',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '200000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const [command, args] = firstOptimizationRunCall()
    const req = (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
    expect(command).toBe('run_double_ma_optimization')
    expect(req.strategy_kind).toBe('DoubleMa')
    expect(req.data_id).toBe('data-456')
    expect(req.fast_ma_types).toEqual(['sma'])
    expect(req.slow_ma_types).toEqual(['ema'])
  })

  it('auto-builds PIT request parameters from Vue strategy store fields', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'Pit',
      backend: 'GpuKernel',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'Grid',
      topK: '20',
      includeAll: 'false',
      heatmapBins: '0',
      exportAllCsvPath: '',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
      startingEquity: '10000',
      costPreset: 'custom',
      feeBps: '1',
      spreadBps: '2',
      slippageBps: '0',
      epsRel: '0',
      stopLossPct: '',
      takeProfitPct: '',
      cooldownBars: '0',
      signalPersistenceBars: '1',
      exitMode: 'cross',
      kaufmanStopPeriod: '22',
      kaufmanStopMult: '2.0',
      kaufmanStopMaType: 'sma',
      pitEfiPeriod: '21',
      pitVoscShortPeriod: '6',
      pitVoscLongPeriod: '27',
      pitCfoPeriod: '11',
      pitCfoScalar: '150.0',
      pitConfirmBars: '3',
      dmConstraintMinTrades: '',
      dmConstraintMaxDd: '',
      longOnly: true,
      allowFlip: true,
      tradeOnNext: false,
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-pit',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '200000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const req = firstOptimizationRunRequest()
    const strategy = (req.strategy && typeof req.strategy === 'object') ? req.strategy as Record<string, unknown> : {}
    expect(req.strategy_kind).toBe('Pit')
    expect(strategy.pressure_impulse_trend_enable).toBe(true)
    expect(strategy.pit_efi_period).toBe(21)
    expect(strategy.pit_vosc_short_period).toBe(6)
    expect(strategy.pit_vosc_long_period).toBe(27)
    expect(strategy.pit_cfo_period).toBe(11)
    expect(strategy.pit_cfo_scalar).toBe(150)
    expect(strategy.pit_confirm_bars).toBe(3)
  })

  it('auto-builds CompositeCpu optimization request from Vue stores', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'CompositeCpu',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      topK: '20',
      includeAll: 'false',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
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
      slippageBps: '0',
      epsRel: '0',
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
      vccMapFastToMomentumPeriod: false,
      vccMapSlowToTrendPeriod: true,
      vccSqueezePeriod: '16',
      vccMomentumPeriod: '18',
      vccMomentumBbPeriod: '21',
      vccTrendPeriod: '7',
      vccSqueezeBbMult: '2.3',
      vccSqueezeKcMult: '1.8',
      vccConfirmBars: '2',
      vccRequireMomentumAccel: false,
      vccMaxTradeBars: '150',
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-789',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '10000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const [command, args] = firstOptimizationRunCall()
    const req = (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
    expect(command).toBe('run_composite_cpu_optimization')
    expect(req.strategy_kind).toBe('CompositeCpu')
    expect(req.data_id).toBe('data-789')
    expect(req.backend).toEqual({ backend: 'CpuOnly' })
  })

  it('routes CompositeTrend through the double-MA optimization command', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'CompositeTrend',
      backend: 'GpuKernel',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      topK: '20',
      includeAll: 'false',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
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
      slippageBps: '0',
      epsRel: '0',
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
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-trend-1',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '10000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const [command, args] = firstOptimizationRunCall()
    const req = (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
    expect(command).toBe('run_double_ma_optimization')
    expect(req.strategy_kind).toBe('CompositeTrend')
    expect(req.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
  })

  it('routes RegimeHybrid through the double-MA optimization command with GPU hybrid', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'RegimeHybrid',
      backend: 'GpuKernel',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'Grid',
      topK: '20',
      includeAll: 'false',
      heatmapBins: '64',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
      startingEquity: '10000',
      costPreset: 'custom',
      feeBps: '1',
      spreadBps: '2',
      slippageBps: '0',
      epsRel: '0',
      stopLossPct: '',
      takeProfitPct: '',
      cooldownBars: '0',
      signalPersistenceBars: '1',
      exitMode: 'cross',
      kaufmanStopPeriod: '22',
      kaufmanStopMult: '2.0',
      kaufmanStopMaType: 'sma',
      longOnly: false,
      allowFlip: true,
      tradeOnNext: false,
      regimeAdxPeriod: '14',
      regimeAdxEnter: '22',
      regimeAdxExit: '18',
      regimeRevEntryRel: '0.0035',
      regimeRevExitRel: '0.0012',
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-regime-1',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '10000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const [command, args] = firstOptimizationRunCall()
    const req = (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
    expect(command).toBe('run_double_ma_optimization')
    expect(req.strategy_kind).toBe('RegimeHybrid')
    expect(req.backend).toEqual({ backend: 'GpuHybridKernel', device_id: 0 })
  })

  it('routes BreakoutMomentum through the double-MA optimization command and promotes ADX-on requests to GPU hybrid', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'BreakoutMomentum',
      backend: 'GpuKernel',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'Grid',
      topK: '20',
      includeAll: 'false',
      heatmapBins: '64',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
      startingEquity: '10000',
      costPreset: 'custom',
      feeBps: '1',
      spreadBps: '2',
      slippageBps: '0',
      epsRel: '0',
      stopLossPct: '',
      takeProfitPct: '',
      cooldownBars: '0',
      signalPersistenceBars: '1',
      exitMode: 'cross',
      kaufmanStopPeriod: '22',
      kaufmanStopMult: '2.0',
      kaufmanStopMaType: 'sma',
      longOnly: false,
      allowFlip: true,
      tradeOnNext: false,
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
      breakoutUseAdx: true,
      breakoutAdxPeriod: '14',
      breakoutAdxMin: '20',
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-breakout-1',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '10000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const [command, args] = firstOptimizationRunCall()
    const req = (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
    expect(command).toBe('run_double_ma_optimization')
    expect(req.strategy_kind).toBe('BreakoutMomentum')
    expect(req.backend).toEqual({ backend: 'GpuHybridKernel', device_id: 0 })
  })

  it('keeps CompositeTrend on the double-MA optimization command when CpuOnly is requested', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'CompositeTrend',
      backend: 'CpuOnly',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'Grid',
      topK: '20',
      includeAll: 'false',
      heatmapBins: '64',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
      startingEquity: '10000',
      costPreset: 'custom',
      feeBps: '1',
      spreadBps: '2',
      slippageBps: '0',
      epsRel: '0',
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
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-trend-cpu-1',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '10000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const [command, args] = firstOptimizationRunCall()
    const req = (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
    expect(command).toBe('run_double_ma_optimization')
    expect(req.strategy_kind).toBe('CompositeTrend')
    expect(req.backend).toEqual({ backend: 'CpuOnly' })
  })

  it('routes VCC through the composite CPU optimization command', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'Vcc',
      backend: 'GpuKernel',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'CoarseToFine',
      topK: '20',
      includeAll: 'false',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
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
      slippageBps: '0',
      epsRel: '0',
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
      vccMapFastToMomentumPeriod: false,
      vccMapSlowToTrendPeriod: true,
      vccSqueezePeriod: '16',
      vccMomentumPeriod: '18',
      vccMomentumBbPeriod: '21',
      vccTrendPeriod: '7',
      vccSqueezeBbMult: '2.3',
      vccSqueezeKcMult: '1.8',
      vccConfirmBars: '2',
      vccRequireMomentumAccel: false,
      vccMaxTradeBars: '150',
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-vcc-1',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '10000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const [command, args] = firstOptimizationRunCall()
    const req = (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
    expect(command).toBe('run_composite_cpu_optimization')
    expect(req.strategy_kind).toBe('Vcc')
    expect(req.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
    expect(req.mode).toBe('Grid')
    expect(req.vcc).toMatchObject({
      map_fast_to_squeeze_period: false,
      map_fast_to_momentum_period: false,
      map_slow_to_trend_period: true,
      squeeze_period: 16,
      momentum_period: 18,
      momentum_bb_period: 21,
      trend_period: 7,
      squeeze_bb_mult: 2.3,
      squeeze_kc_mult: 1.8,
      confirm_bars: 2,
      require_momentum_accel: false,
      max_trade_bars: 150,
    })
  })

  it('routes VEMR through the composite CPU optimization command', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'Vemr',
      backend: 'GpuKernel',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'CoarseToFine',
      topK: '20',
      includeAll: 'false',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
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
      slippageBps: '0',
      epsRel: '0',
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
      vemrRequireBandTouch: true,
      vemrZPeriod: '18',
      vemrZMaType: 'sma',
      vemrZNbdev: '1.2',
      vemrZDevtype: '0',
      vemrZEntry: '2.4',
      vemrZExit: '0.7',
      vemrBbPeriod: '21',
      vemrBbMult: '2.3',
      vemrAtrPeriod: '17',
      vemrAtrStopMult: '2.1',
      vemrConfirmBars: '2',
      vemrMaxTradeBars: '130',
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-vemr-1',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '10000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const [command, args] = firstOptimizationRunCall()
    const req = (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
    expect(command).toBe('run_composite_cpu_optimization')
    expect(req.strategy_kind).toBe('Vemr')
    expect(req.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
    expect(req.mode).toBe('Grid')
    expect(req.vemr).toMatchObject({
      map_fast_to_z_period: false,
      map_fast_to_bb_period: true,
      map_slow_to_atr_period: false,
      require_band_touch: true,
      z_period: 18,
      z_ma_type: 'sma',
      z_nbdev: 1.2,
      z_devtype: 0,
      z_entry: 2.4,
      z_exit: 0.7,
      bb_period: 21,
      bb_mult: 2.3,
      atr_period: 17,
      atr_stop_mult: 2.1,
      confirm_bars: 2,
      max_trade_bars: 130,
    })
  })

  it('routes MKCS through the composite CPU optimization command', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'Mkcs',
      backend: 'GpuKernel',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'Grid',
      topK: '20',
      includeAll: 'false',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
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
      slippageBps: '0',
      epsRel: '0',
      stopLossPct: '',
      takeProfitPct: '',
      cooldownBars: '0',
      signalPersistenceBars: '1',
      exitMode: 'cross',
      kaufmanStopPeriod: '22',
      kaufmanStopMult: '2.0',
      kaufmanStopMaType: 'sma',
      longOnly: false,
      allowFlip: true,
      tradeOnNext: false,
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-mkcs-1',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '10000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const [command, args] = firstOptimizationRunCall()
    const req = (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
    expect(command).toBe('run_composite_cpu_optimization')
    expect(req.strategy_kind).toBe('Mkcs')
    expect(req.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
    expect(req.mkcs).toMatchObject({
      map_fast_to_stddev_period: true,
      map_slow_to_breakout_lookback: true,
      stddev_period: 20,
      breakout_lookback: 20,
      compression_threshold: 0.01,
      reexpansion_threshold: 0.03,
    })
  })

  it('routes the staged GPU-native expansion kinds through the composite CPU optimization command', async () => {
    const cases = [
      { kind: 'Kqsr', field: 'kqsr' },
      { kind: 'Carm', field: 'carm' },
      { kind: 'Kvdf', field: 'kvdf' },
      { kind: 'Cdmc', field: 'cdmc' },
    ] as const

    for (const row of cases) {
      invokeCommandMock.mockClear()
      installDefaultInvokeCommandMock()
      ;(pinia.state.value as Record<string, unknown>).optimize = {
        strategyKind: row.kind,
        backend: 'GpuKernel',
        fastStart: '5',
        fastEnd: '15',
        fastStep: '5',
        slowStart: '20',
        slowEnd: '40',
        slowStep: '10',
        fastMaType: 'sma',
        slowMaType: 'ema',
      }
      ;(pinia.state.value as Record<string, unknown>).advanced = {
        maSource: 'close',
        objective: 'Sharpe',
        mode: 'Grid',
        topK: '20',
        includeAll: 'false',
      }
      ;(pinia.state.value as Record<string, unknown>).strategy = {
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
        slippageBps: '0',
        epsRel: '0',
        stopLossPct: '',
        takeProfitPct: '',
        cooldownBars: '0',
        signalPersistenceBars: '1',
        exitMode: 'cross',
        kaufmanStopPeriod: '22',
        kaufmanStopMult: '2.0',
        kaufmanStopMaType: 'sma',
        longOnly: false,
        allowFlip: true,
        tradeOnNext: false,
      }
      ;(pinia.state.value as Record<string, unknown>).data = {
        dataId: `data-${row.kind.toLowerCase()}`,
        dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
        dataLen: '10000',
        dataTimeframe: '1m',
      }

      await runOptimization()

      const [command, args] = firstOptimizationRunCall()
      const req = (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
      expect(command).toBe('run_composite_cpu_optimization')
      expect(req.strategy_kind).toBe(row.kind)
      expect(req.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
      expect(req[row.field]).toBeTruthy()
    }
  })

  it('auto-builds CompositeMo optimization request from Vue stores', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'CompositeMo',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      moObjectiveMin: 'MaxDrawdown',
      moSampler: 'Nsga2',
      moNsga2Seed: '1',
      moNsga2Population: '80',
      moNsga2Generations: '40',
      moNsga2CrossoverProb: '0.9',
      moNsga2SwappingProb: '0.5',
      moNsga2MutationProb: '',
      moUseRangeBuilder: false,
      moSearchSpace: '',
      moPrescanEnable: false,
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
      compositeSignalMode: 'hard_gate',
      startingEquity: '10000',
      costPreset: 'custom',
      feeBps: '1',
      spreadBps: '2',
      slippageBps: '0',
      epsRel: '0',
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
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-mo-1',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '10000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const [command, args] = firstOptimizationRunCall()
    const req = (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
    const sampler = (req.sampler && typeof req.sampler === 'object')
      ? req.sampler as Record<string, unknown>
      : {}
    expect(command).toBe('run_composite_mo_optimization')
    expect(req.strategy_kind).toBe('CompositeMo')
    expect(req.data_id).toBe('data-mo-1')
    expect(sampler.sampler).toBe('Nsga2')
  })

  it('builds request from Vue store build context when available', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'DoubleMa',
      backend: 'Auto',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'Grid',
      topK: '20',
      includeAll: 'false',
      heatmapBins: '64',
      exportAllCsvPath: '',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
      startingEquity: '10000',
      costPreset: 'custom',
      feeBps: '1',
      spreadBps: '2',
      slippageBps: '0',
      epsRel: '0',
      stopLossPct: '',
      takeProfitPct: '',
      cooldownBars: '0',
      signalPersistenceBars: '1',
      exitMode: 'cross',
      kaufmanStopPeriod: '22',
      kaufmanStopMult: '2.0',
      kaufmanStopMaType: 'sma',
      dmConstraintMinTrades: '',
      dmConstraintMaxDd: '',
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-store-ctx',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '200000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const req = firstOptimizationRunRequest()
    expect(req.strategy_kind).toBe('DoubleMa')
    expect(req.data_id).toBe('data-store-ctx')
    expect(req.ma_source).toBe('close')
  })

  it('uses Vue store checkbox strategy flags when auto-building', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'DoubleMa',
      backend: 'Auto',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'Grid',
      topK: '20',
      includeAll: 'false',
      heatmapBins: '64',
      exportAllCsvPath: '',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
      startingEquity: '10000',
      costPreset: 'custom',
      feeBps: '1',
      spreadBps: '2',
      slippageBps: '0',
      epsRel: '0',
      stopLossPct: '',
      takeProfitPct: '',
      cooldownBars: '0',
      signalPersistenceBars: '1',
      exitMode: 'cross',
      kaufmanStopPeriod: '22',
      kaufmanStopMult: '2.0',
      kaufmanStopMaType: 'sma',
      dmConstraintMinTrades: '',
      dmConstraintMaxDd: '',
      longOnly: false,
      allowFlip: false,
      tradeOnNext: true,
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-store-bool',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '10000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const req = firstOptimizationRunRequest()
    const strategy = (req.strategy && typeof req.strategy === 'object')
      ? req.strategy as Record<string, unknown>
      : {}
    expect(strategy).toMatchObject({
      long_only: false,
      allow_flip: false,
      trade_on_next_bar: true,
    })
  })

  it('uses Vue store MA params and sweeps when auto-building', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'DoubleMa',
      backend: 'Auto',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
      fastMaParams: { alpha: 7 },
      slowMaParams: { alpha: 11 },
      fastMaParamSweeps: { alpha: [2, 8, 2] },
      slowMaParamSweeps: { alpha: [3, 15, 3] },
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'Grid',
      topK: '20',
      includeAll: 'false',
      heatmapBins: '64',
      exportAllCsvPath: '',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
      startingEquity: '10000',
      costPreset: 'custom',
      feeBps: '1',
      spreadBps: '2',
      slippageBps: '0',
      epsRel: '0',
      stopLossPct: '',
      takeProfitPct: '',
      cooldownBars: '0',
      signalPersistenceBars: '1',
      exitMode: 'cross',
      kaufmanStopPeriod: '22',
      kaufmanStopMult: '2.0',
      kaufmanStopMaType: 'sma',
      dmConstraintMinTrades: '',
      dmConstraintMaxDd: '',
      longOnly: true,
      allowFlip: true,
      tradeOnNext: false,
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-store-ma',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '200000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const req = firstOptimizationRunRequest()
    expect(req.fast_ma_params).toEqual({ alpha: 7 })
    expect(req.slow_ma_params).toEqual({ alpha: 11 })
    expect(req.fast_ma_param_sweeps).toEqual({ alpha: [2, 8, 2] })
    expect(req.slow_ma_param_sweeps).toEqual({ alpha: [3, 15, 3] })
  })

  it('keeps missing per-side MA params null when not present in store', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'DoubleMa',
      backend: 'Auto',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
      fastMaParams: { alpha: 9 },
      fastMaParamSweeps: { alpha: [3, 15, 3] },
      slowMaParams: null,
      slowMaParamSweeps: null,
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      mode: 'Grid',
      topK: '20',
      includeAll: 'false',
      heatmapBins: '64',
      exportAllCsvPath: '',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
      startingEquity: '10000',
      costPreset: 'custom',
      feeBps: '1',
      spreadBps: '2',
      slippageBps: '0',
      epsRel: '0',
      stopLossPct: '',
      takeProfitPct: '',
      cooldownBars: '0',
      signalPersistenceBars: '1',
      exitMode: 'cross',
      kaufmanStopPeriod: '22',
      kaufmanStopMult: '2.0',
      kaufmanStopMaType: 'sma',
      dmConstraintMinTrades: '',
      dmConstraintMaxDd: '',
      longOnly: true,
      allowFlip: true,
      tradeOnNext: false,
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-store-ma-side-fallback',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '200000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const req = firstOptimizationRunRequest()
    expect(req.fast_ma_params).toEqual({ alpha: 9 })
    expect(req.fast_ma_param_sweeps).toEqual({ alpha: [3, 15, 3] })
    expect(req.slow_ma_params).toBeNull()
    expect(req.slow_ma_param_sweeps).toBeNull()
  })

  it('uses Vue store composite strategy text fields when auto-building', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'CompositeCpu',
      backend: 'Auto',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      topK: '20',
      includeAll: 'false',
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
      compositeSignalMode: 'entry_core_exit_overlay',
      halfTrendAmplitude: '7',
      halfTrendAtrPeriod: '88',
      halfTrendChannelDev: '1.7',
      maczFastLength: '11',
      maczSlowLength: '28',
      maczSignalLength: '8',
      maczLengthZ: '19',
      maczLengthStdev: '24',
      maczGamma: '0.03',
      maczA: '1.1',
      maczB: '0.9',
      ottPeriod: '5',
      ottPercent: '2.2',
      ottMaType: 'EMA',
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-store-composite',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '10000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const req = firstOptimizationRunRequest()
    const halftrend = (req.halftrend && typeof req.halftrend === 'object')
      ? req.halftrend as Record<string, unknown>
      : {}
    const macz = (req.macz && typeof req.macz === 'object')
      ? req.macz as Record<string, unknown>
      : {}
    const ott = (req.ott && typeof req.ott === 'object')
      ? req.ott as Record<string, unknown>
      : {}
    expect(req.signal_mode).toBe('entry_core_exit_overlay')
    expect(halftrend).toMatchObject({
      amplitude: 7,
      atr_period: 88,
      channel_deviation: 1.7,
    })
    expect(macz).toMatchObject({
      fast_length: 11,
      slow_length: 28,
      signal_length: 8,
    })
    expect(ott).toMatchObject({
      period: 5,
      percent: 2.2,
      ma_type: 'EMA',
    })
  })

  it('uses Vue advanced CompositeMo tuning fields when auto-building', async () => {
    ;(pinia.state.value as Record<string, unknown>).optimize = {
      strategyKind: 'CompositeMo',
      backend: 'Auto',
      fastStart: '5',
      fastEnd: '15',
      fastStep: '5',
      slowStart: '20',
      slowEnd: '40',
      slowStep: '10',
      fastMaType: 'sma',
      slowMaType: 'ema',
    }
    ;(pinia.state.value as Record<string, unknown>).advanced = {
      maSource: 'close',
      objective: 'Sharpe',
      moObjectiveMin: 'UlcerIndex',
      moSampler: 'Random',
      moRandomSeed: '11',
      moRandomTrials: '333',
      moPrescanEnable: true,
      moPrescanRequireGpu: true,
      moPrescanTopK: '77',
      moPrescanWindow: '9',
      moPrescanPredictiveTopK: '13',
      moPrescanPredictiveStride: '6',
      moPrescanObjectives: 'both',
      moPrescanMode: 'Auto',
      moPrescanMaPairs: '',
      moUseRangeBuilder: false,
    }
    ;(pinia.state.value as Record<string, unknown>).strategy = {
      compositeSignalMode: 'hard_gate',
      startingEquity: '10000',
      costPreset: 'custom',
      feeBps: '1',
      spreadBps: '2',
      slippageBps: '0',
      epsRel: '0',
      stopLossPct: '',
      takeProfitPct: '',
      cooldownBars: '0',
      signalPersistenceBars: '1',
      exitMode: 'cross',
      kaufmanStopPeriod: '22',
      kaufmanStopMult: '2.0',
      kaufmanStopMaType: 'sma',
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      dataId: 'data-store-mo',
      dataFields: ['timestamp', 'open', 'high', 'low', 'close', 'volume'],
      dataLen: '10000',
      dataTimeframe: '1m',
    }

    await runOptimization()

    const req = firstOptimizationRunRequest()
    const objectives = (req.objectives && typeof req.objectives === 'object')
      ? req.objectives as Record<string, unknown>
      : {}
    const sampler = (req.sampler && typeof req.sampler === 'object')
      ? req.sampler as Record<string, unknown>
      : {}
    const gpuPrescan = (req.gpu_prescan && typeof req.gpu_prescan === 'object')
      ? req.gpu_prescan as Record<string, unknown>
      : {}
    expect(objectives.minimize).toBe('UlcerIndex')
    expect(sampler).toMatchObject({
      sampler: 'Random',
      config: { seed: 11, trials: 333 },
    })
    expect(gpuPrescan).toMatchObject({
      top_k: 77,
      window: 9,
      predictive_top_k: 13,
      predictive_stride: 6,
      require_gpu: true,
    })
  })

  it('requests cancellation via runControlService', async () => {
    await cancelOptimization()
    expect(requestCancelCurrentRunMock).toHaveBeenCalled()
  })

  it('normalizes GPU status via direct invoke command', async () => {
    const status = await refreshGpuStatus()

    expect(invokeCommandMock).toHaveBeenCalledWith('get_gpu_status')
    expect(status.builtWithCuda).toBe(true)
    expect(status.builtWithCudaBacktestKernel).toBe(true)
    expect(status.cudaAvailable).toBe(true)
    expect(status.autoDeviceName).toBe('NVIDIA RTX 4090')
    expect(status.autoDeviceMemoryBytes).toBe(24 * 1024 * 1024 * 1024)
    expect(status.text).toContain('build=cuda')
    expect(status.text).toContain('kernel=on')
    expect(status.deviceMeta).toContain('Automatic CUDA device')
  })
})
