import { expect, test, type Page } from '@playwright/test'
import fs from 'node:fs/promises'
import path from 'node:path'
import { emptyAppScenario, type MockTauriScenario } from '../fixtures'
import { mockTauriSequence } from '../support/mockTauri'
import { expectNoHorizontalDocumentOverflow, expectNoStuckBusyState, installQaGuards } from '../support/qaAssertions'
import { gotoMockedShellWithScenario, readTauriInvokeLog } from '../support/workflowRunner'

type OptimizationCommand =
  | 'run_double_ma_optimization'
  | 'run_composite_cpu_optimization'
  | 'run_composite_mo_optimization'

type ValidationCommand = 'run_double_ma_validation' | 'run_composite_validation'

interface ConventionalStrategyCase {
  kind: string
  label: string
  optimizationCommand: OptimizationCommand
  validationCommand: ValidationCommand
  expectsHeatmap: boolean
  expectsPareto?: boolean
  expectedValidationBlock?: RegExp
  setup?: (page: Page) => Promise<void>
}

const DATASET_META = {
  csv_path: 'C:/fixtures/aapl_1m.csv',
  timeframe: '1m',
  len: 5000,
  symbol: 'AAPL',
  label: 'AAPL fixture',
  has_open: true,
  has_high: true,
  has_low: true,
  has_close: true,
  has_volume: true,
  min_ts: 1904164200000,
  max_ts: 1904250600000,
}

const DOUBLE_PARAMS = {
  fast_len: 8,
  slow_len: 32,
  fast_ma_type: 'sma',
  slow_ma_type: 'sma',
}

const COMPOSITE_PARAMS = {
  fast_len: 9,
  slow_len: 34,
  fast_ma_type: 'ema',
  slow_ma_type: 'sma',
}

const BASE_METRICS = {
  pnl: 0.12,
  sharpe: 1.4,
  max_dd: -0.05,
  trades: 18,
  exposure: 0.52,
  net_exposure: 0.08,
}

const DRILL_BAR_COUNT = 72
const DRILL_START_SEC = 1_744_500_000

const PRICE_BARS = Array.from({ length: DRILL_BAR_COUNT }, (_, index) => {
  const time = DRILL_START_SEC + index * 60
  const base = 100 + Math.sin(index / 5) * 1.3 + index * 0.015
  const open = Number((base + Math.sin(index / 3) * 0.16).toFixed(3))
  const close = Number((base + Math.cos(index / 4) * 0.2).toFixed(3))
  return {
    time,
    open,
    high: Number((Math.max(open, close) + 0.42).toFixed(3)),
    low: Number((Math.min(open, close) - 0.4).toFixed(3)),
    close,
    from_index: index + 1,
    to_index: index + 1,
  }
})

const BASE_DRILL_CURVES = {
  equity: {
    start_t: 1,
    end_t: 72,
    bins: 6,
    minmax: [1, 1.01, 1.01, 1.02, 1.02, 1.035, 1.03, 1.05, 1.045, 1.065, 1.06, 1.08],
  },
  drawdown: {
    start_t: 1,
    end_t: 72,
    bins: 6,
    minmax: [0, -0.005, -0.005, -0.01, -0.01, -0.02, -0.01, -0.015, -0.015, -0.025, -0.02, -0.03],
  },
  trades: [
    { entry_t: 4, exit_t: 14, direction: 1, pnl: 0.03, bars: 10, open: false },
    { entry_t: 22, exit_t: 33, direction: -1, pnl: -0.01, bars: 11, open: false },
    { entry_t: 45, exit_t: 62, direction: 1, pnl: 0.018, bars: 17, open: false },
  ],
  trade_stats: {
    closed_trades: 3,
    open_trades: 0,
    wins: 2,
    losses: 1,
    win_rate: 0.667,
    avg_win: 0.024,
    avg_loss: -0.01,
    profit_factor: 4.8,
    expectancy: 0.0127,
  },
}

const DOUBLE_MA_DRILL_CURVES = {
  ...BASE_DRILL_CURVES,
  indicator_bars: PRICE_BARS.map((bar, index) => ({
    time: bar.time,
    from_index: index + 1,
    to_index: index + 1,
    fast: Number((bar.close + Math.sin(index / 5) * 0.2).toFixed(3)),
    slow: Number((bar.close + Math.cos(index / 8) * 0.3 - 0.08).toFixed(3)),
  })),
}

const LEGACY_COMPOSITE_DRILL_CURVES = {
  ...DOUBLE_MA_DRILL_CURVES,
  composite_indicator_bars: PRICE_BARS.map((bar, index) => ({
    time: bar.time,
    from_index: index + 1,
    to_index: index + 1,
    halftrend: index % 10 < 5 ? 0 : 1,
    macz_hist: Number((Math.sin(index / 6) * 0.58).toFixed(3)),
    ott_line: Number((bar.close - 0.45 + Math.cos(index / 9) * 0.16).toFixed(3)),
  })),
}

type StrategySeriesKind = 'line' | 'histogram'
type StrategySeriesUnit =
  | 'price'
  | 'price_delta'
  | 'number'
  | 'ratio'
  | 'degrees'
  | 'state'
  | 'z_score'
  | 'oscillator'

interface StrategyReferenceLineFixture {
  value: number
  label: string
}

interface StrategySeriesFixtureDefinition {
  id: string
  label: string
  kind: StrategySeriesKind
  pane_id: string
  scale_id: string
  unit: StrategySeriesUnit
  default_visible: boolean
  reference_lines?: StrategyReferenceLineFixture[]
}

interface StandaloneVisualizationFixture {
  fastAxisLabel: string
  slowAxisLabel: string
  representativeLabels: [string, string]
  series: StrategySeriesFixtureDefinition[]
}

const STANDALONE_VISUALIZATIONS: Record<string, StandaloneVisualizationFixture> = {
  Asr: {
    fastAxisLabel: 'Squeeze period',
    slowAxisLabel: 'Trend / ER / Z-score periods',
    representativeLabels: ['Kaufman long stop', 'Z-score'],
    series: [
      {
        id: 'asr_kaufman_long_stop',
        label: 'Kaufman long stop',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'asr_kaufman_short_stop',
        label: 'Kaufman short stop',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'asr_squeeze_state',
        label: 'Squeeze state',
        kind: 'histogram',
        pane_id: 'asr_squeeze',
        scale_id: 'asr_state',
        unit: 'state',
        default_visible: true,
        reference_lines: [{ value: 0.5, label: 'Squeeze on' }],
      },
      {
        id: 'asr_squeeze_momentum',
        label: 'Squeeze momentum',
        kind: 'histogram',
        pane_id: 'asr_squeeze',
        scale_id: 'asr_momentum',
        unit: 'price_delta',
        default_visible: true,
        reference_lines: [{ value: 0, label: 'Zero' }],
      },
      {
        id: 'asr_linearreg_angle',
        label: 'Linear-regression angle',
        kind: 'line',
        pane_id: 'asr_angle',
        scale_id: 'asr_angle',
        unit: 'degrees',
        default_visible: false,
        reference_lines: [
          { value: 15, label: 'Long trend minimum' },
          { value: -15, label: 'Short trend minimum' },
          { value: 5, label: 'Mean-reversion upper' },
          { value: -5, label: 'Mean-reversion lower' },
        ],
      },
      {
        id: 'asr_efficiency_ratio',
        label: 'Efficiency ratio',
        kind: 'line',
        pane_id: 'asr_efficiency',
        scale_id: 'asr_efficiency',
        unit: 'ratio',
        default_visible: false,
        reference_lines: [
          { value: 0.35, label: 'Trend minimum' },
          { value: 0.22, label: 'Mean-reversion maximum' },
        ],
      },
      {
        id: 'asr_zscore',
        label: 'Z-score',
        kind: 'line',
        pane_id: 'asr_zscore',
        scale_id: 'asr_zscore',
        unit: 'z_score',
        default_visible: true,
        reference_lines: [
          { value: -2, label: 'Long entry' },
          { value: 2, label: 'Short entry' },
          { value: -0.5, label: 'Long exit' },
          { value: 0.5, label: 'Short exit' },
        ],
      },
    ],
  },
  Vcc: {
    fastAxisLabel: 'Squeeze / momentum periods',
    slowAxisLabel: 'Trend period',
    representativeLabels: ['Squeeze state', 'Squeeze momentum'],
    series: [
      {
        id: 'vcc_squeeze_state',
        label: 'Squeeze state',
        kind: 'histogram',
        pane_id: 'vcc_state',
        scale_id: 'vcc_state',
        unit: 'state',
        default_visible: true,
        reference_lines: [{ value: 0.5, label: 'Squeeze on' }],
      },
      {
        id: 'vcc_trend_state',
        label: 'Trend state',
        kind: 'line',
        pane_id: 'vcc_state',
        scale_id: 'vcc_state',
        unit: 'state',
        default_visible: true,
        reference_lines: [{ value: 0.5, label: 'Bullish threshold' }],
      },
      {
        id: 'vcc_squeeze_momentum',
        label: 'Squeeze momentum',
        kind: 'histogram',
        pane_id: 'vcc_momentum',
        scale_id: 'vcc_momentum',
        unit: 'price_delta',
        default_visible: true,
        reference_lines: [{ value: 0, label: 'Zero' }],
      },
    ],
  },
  Vemr: {
    fastAxisLabel: 'Z-score / BB periods',
    slowAxisLabel: 'ATR period',
    representativeLabels: ['Bollinger upper', 'Z-score'],
    series: [
      {
        id: 'vemr_bb_upper',
        label: 'Bollinger upper',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'vemr_bb_middle',
        label: 'Bollinger middle',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'vemr_bb_lower',
        label: 'Bollinger lower',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'vemr_zscore',
        label: 'Z-score',
        kind: 'line',
        pane_id: 'vemr_zscore',
        scale_id: 'vemr_zscore',
        unit: 'z_score',
        default_visible: true,
        reference_lines: [
          { value: -2, label: 'Long entry' },
          { value: 2, label: 'Short entry' },
          { value: -0.5, label: 'Long exit' },
          { value: 0.5, label: 'Short exit' },
        ],
      },
      {
        id: 'vemr_atr',
        label: 'Average true range',
        kind: 'line',
        pane_id: 'vemr_volatility',
        scale_id: 'vemr_atr',
        unit: 'price_delta',
        default_visible: false,
      },
    ],
  },
  Rocb: {
    fastAxisLabel: 'Channel period',
    slowAxisLabel: 'ATR period',
    representativeLabels: ['Donchian upper', 'Donchian lower'],
    series: [
      {
        id: 'rocb_donchian_upper',
        label: 'Donchian upper',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'rocb_donchian_lower',
        label: 'Donchian lower',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'rocb_atr',
        label: 'Average true range',
        kind: 'line',
        pane_id: 'rocb_volatility',
        scale_id: 'rocb_atr',
        unit: 'price_delta',
        default_visible: false,
      },
    ],
  },
  Vcet: {
    fastAxisLabel: 'VIDYA short period',
    slowAxisLabel: 'VIDYA long / Chandelier periods',
    representativeLabels: ['VIDYA', 'Chandelier long stop'],
    series: [
      {
        id: 'vcet_vidya',
        label: 'VIDYA',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'vcet_chandelier_long',
        label: 'Chandelier long stop',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'vcet_chandelier_short',
        label: 'Chandelier short stop',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
    ],
  },
  Mkcs: {
    fastAxisLabel: 'StdDev period',
    slowAxisLabel: 'Breakout lookback',
    representativeLabels: ['Donchian upper', 'Compression ratio'],
    series: [
      {
        id: 'mkcs_donchian_upper',
        label: 'Donchian upper',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'mkcs_donchian_lower',
        label: 'Donchian lower',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'mkcs_compression_ratio',
        label: 'Compression ratio',
        kind: 'line',
        pane_id: 'mkcs_compression',
        scale_id: 'mkcs_compression',
        unit: 'ratio',
        default_visible: true,
        reference_lines: [
          { value: 0.018, label: 'Compression threshold' },
          { value: 0.026, label: 'Re-expansion threshold' },
        ],
      },
    ],
  },
  Kqsr: {
    fastAxisLabel: 'Fast oscillator period floor',
    slowAxisLabel: 'Slow oscillator period floor',
    representativeLabels: ['PRB upper', 'KDJ J'],
    series: [
      {
        id: 'kqsr_prb_upper',
        label: 'PRB upper',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'kqsr_prb_lower',
        label: 'PRB lower',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'kqsr_kdj_j',
        label: 'KDJ J',
        kind: 'line',
        pane_id: 'kqsr_bounded',
        scale_id: 'kqsr_bounded',
        unit: 'oscillator',
        default_visible: true,
        reference_lines: [
          { value: 20, label: 'Oversold' },
          { value: 50, label: 'Neutral' },
          { value: 80, label: 'Overbought' },
        ],
      },
      {
        id: 'kqsr_srsi_k',
        label: 'Stochastic RSI K',
        kind: 'line',
        pane_id: 'kqsr_bounded',
        scale_id: 'kqsr_bounded',
        unit: 'oscillator',
        default_visible: true,
      },
      {
        id: 'kqsr_srsi_d',
        label: 'Stochastic RSI D',
        kind: 'line',
        pane_id: 'kqsr_bounded',
        scale_id: 'kqsr_bounded',
        unit: 'oscillator',
        default_visible: true,
      },
      {
        id: 'kqsr_qqe_fast',
        label: 'QQE fast',
        kind: 'line',
        pane_id: 'kqsr_bounded',
        scale_id: 'kqsr_bounded',
        unit: 'oscillator',
        default_visible: true,
      },
      {
        id: 'kqsr_qqe_slow',
        label: 'QQE slow',
        kind: 'line',
        pane_id: 'kqsr_bounded',
        scale_id: 'kqsr_bounded',
        unit: 'oscillator',
        default_visible: true,
      },
      {
        id: 'kqsr_ift_rsi',
        label: 'Inverse Fisher RSI',
        kind: 'line',
        pane_id: 'kqsr_ift',
        scale_id: 'kqsr_ift',
        unit: 'ratio',
        default_visible: true,
        reference_lines: [{ value: 0, label: 'Zero' }],
      },
      {
        id: 'kqsr_prb_fraction',
        label: 'PRB band fraction',
        kind: 'line',
        pane_id: 'kqsr_prb_fraction',
        scale_id: 'kqsr_prb_fraction',
        unit: 'ratio',
        default_visible: false,
        reference_lines: [
          { value: 0.2, label: 'Long gate' },
          { value: 0.8, label: 'Short gate' },
        ],
      },
    ],
  },
  Carm: {
    fastAxisLabel: 'Fast indicator period floor',
    slowAxisLabel: 'Slow indicator period floor',
    representativeLabels: ['OTTO HOTT', 'OTTO regime spread'],
    series: [
      {
        id: 'carm_otto_hott',
        label: 'OTTO HOTT',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'carm_otto_lott',
        label: 'OTTO LOTT',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'carm_coppock',
        label: 'Coppock curve',
        kind: 'line',
        pane_id: 'carm_coppock',
        scale_id: 'carm_coppock',
        unit: 'oscillator',
        default_visible: false,
        reference_lines: [{ value: 0, label: 'Zero' }],
      },
      {
        id: 'carm_ao',
        label: 'Awesome oscillator',
        kind: 'histogram',
        pane_id: 'carm_price_delta',
        scale_id: 'carm_price_delta',
        unit: 'price_delta',
        default_visible: false,
        reference_lines: [{ value: 0, label: 'Zero' }],
      },
      {
        id: 'carm_apo',
        label: 'Absolute price oscillator',
        kind: 'line',
        pane_id: 'carm_price_delta',
        scale_id: 'carm_price_delta',
        unit: 'price_delta',
        default_visible: false,
      },
      {
        id: 'carm_rocr',
        label: 'Rate of change ratio',
        kind: 'line',
        pane_id: 'carm_rocr',
        scale_id: 'carm_rocr',
        unit: 'ratio',
        default_visible: false,
        reference_lines: [{ value: 1, label: 'Neutral' }],
      },
      {
        id: 'carm_otto_regime_spread',
        label: 'OTTO regime spread',
        kind: 'line',
        pane_id: 'carm_otto_spread',
        scale_id: 'carm_otto_spread',
        unit: 'ratio',
        default_visible: true,
        reference_lines: [
          { value: 0.012, label: 'Bullish threshold' },
          { value: -0.012, label: 'Bearish threshold' },
        ],
      },
    ],
  },
  Kvdf: {
    fastAxisLabel: 'FVG / fast DTI period floors',
    slowAxisLabel: 'Keltner / DevStop / DTI period floors',
    representativeLabels: ['Keltner upper', 'Directional trend index'],
    series: [
      {
        id: 'kvdf_keltner_upper',
        label: 'Keltner upper',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'kvdf_keltner_middle',
        label: 'Keltner middle',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'kvdf_keltner_lower',
        label: 'Keltner lower',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'kvdf_vwap',
        label: 'VWAP',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'kvdf_devstop_long',
        label: 'DevStop long',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: false,
      },
      {
        id: 'kvdf_devstop_short',
        label: 'DevStop short',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: false,
      },
      {
        id: 'kvdf_fvg_upper',
        label: 'FVG upper trail',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: false,
      },
      {
        id: 'kvdf_fvg_lower',
        label: 'FVG lower trail',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: false,
      },
      {
        id: 'kvdf_dti',
        label: 'Directional trend index',
        kind: 'histogram',
        pane_id: 'kvdf_dti',
        scale_id: 'kvdf_dti',
        unit: 'oscillator',
        default_visible: true,
        reference_lines: [
          { value: -20, label: 'Bearish threshold' },
          { value: 0, label: 'Zero' },
          { value: 20, label: 'Bullish threshold' },
        ],
      },
    ],
  },
  Cdmc: {
    fastAxisLabel: 'DPO period floor',
    slowAxisLabel: 'Cycle / decycler period floors',
    representativeLabels: ['MAMA', 'Detrended price oscillator'],
    series: [
      {
        id: 'cdmc_mama',
        label: 'MAMA',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'cdmc_fama',
        label: 'FAMA',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'cdmc_decycler',
        label: 'Decycler',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
      },
      {
        id: 'cdmc_dpo',
        label: 'Detrended price oscillator',
        kind: 'histogram',
        pane_id: 'cdmc_dpo',
        scale_id: 'cdmc_dpo',
        unit: 'price_delta',
        default_visible: true,
        reference_lines: [{ value: 0, label: 'Zero' }],
      },
      {
        id: 'cdmc_cycle_angle',
        label: 'Correlation-cycle angle',
        kind: 'line',
        pane_id: 'cdmc_cycle',
        scale_id: 'cdmc_angle',
        unit: 'degrees',
        default_visible: true,
        reference_lines: [
          { value: -25, label: 'Bearish phase' },
          { value: 25, label: 'Bullish phase' },
        ],
      },
      {
        id: 'cdmc_cycle_state',
        label: 'Correlation-cycle state',
        kind: 'histogram',
        pane_id: 'cdmc_cycle',
        scale_id: 'cdmc_state',
        unit: 'state',
        default_visible: true,
        reference_lines: [{ value: 0, label: 'Regime boundary' }],
      },
    ],
  },
}

const STRATEGIES: ConventionalStrategyCase[] = [
  {
    kind: 'DoubleMa',
    label: 'Double MA',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'DynamicDoubleMa',
    label: 'Dynamic Double MA',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
    setup: async (page) => {
      await page.locator('#vueBackend').selectOption('GpuKernel')
      await page.locator('#vueFastType').selectOption('sma')
      await page.locator('#vueSlowType').selectOption('ema')
      await page.locator('#vueDynamicStrategyIrJson').fill('{"profile":"double_ma"}')
    },
  },
  {
    kind: 'Pit',
    label: 'Pressure Impulse Trend',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Set',
    label: 'Slope + Efficiency Trend',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Mts',
    label: 'Momentum Triple Stack',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Orc',
    label: 'Oscillator Reversion Confluence',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Marrt',
    label: 'MA Ribbon Regime Trend',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Vfct',
    label: 'Volume Flow Confirmation Trend',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
    setup: async (page) => {
      await page.locator('#vueFastType').selectOption('vwma')
      await page.locator('#vueSlowType').selectOption('vwma')
    },
  },
  {
    kind: 'CompositeTrend',
    label: 'Composite Trend',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'RegimeHybrid',
    label: 'Regime Hybrid',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'BreakoutMomentum',
    label: 'Breakout Momentum',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Rocb',
    label: 'Risk Overlay Channel Breakout',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Vcet',
    label: 'VIDYA Chandelier Exit Trend',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Kqsr',
    label: 'KDJ QQE Stochastic Reversion',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Carm',
    label: 'Coppock AO Relative Momentum',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Kvdf',
    label: 'Keltner VWAP Directional Flow',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Mkcs',
    label: 'Market Kurtosis Compression Setup',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Cdmc',
    label: 'Cycle Decycler MAMA Composite',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Asr',
    label: 'Adaptive Squeeze Regime',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Vcc',
    label: 'Volatility Compression Confirmation',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Vemr',
    label: 'Volatility Expansion Mean Reversion',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'CompositeCpu',
    label: 'Composite CPU',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: false,
  },
  {
    kind: 'CompositeMo',
    label: 'Composite MO',
    optimizationCommand: 'run_composite_mo_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: false,
    expectsPareto: true,
  },
]

function action(page: Page, id: string) {
  return page.locator(`[data-qa-action="${id}"]`)
}

function sanitizeFilePart(value: string): string {
  return String(value || '').replace(/[^a-z0-9_-]+/gi, '_').toLowerCase()
}

const CONVENTIONAL_HEATMAP_BINS = 16
const STRATEGY_BUNDLE_START_INDEX = 6

function heatmapPeriodAt(min: number, max: number, index: number, bins: number): number {
  if (bins <= 1) return min
  return Math.round(min + ((max - min) * index) / (bins - 1))
}

function buildHeatmapFixture(bins = CONVENTIONAL_HEATMAP_BINS): Record<string, unknown> {
  const fastMin = 5
  const fastMax = 20
  const slowMin = 20
  const slowMax = 35
  const values: Array<number | null> = Array.from({ length: bins * bins }, (_, index) => {
    const logicalRow = Math.floor(index / bins)
    const col = index % bins
    const fast = heatmapPeriodAt(fastMin, fastMax, col, bins)
    const slow = heatmapPeriodAt(slowMin, slowMax, logicalRow, bins)
    if (fast >= slow) return null
    const ridge = Math.cos((logicalRow - col) / 4) * 0.16
    const slope = (logicalRow / Math.max(1, bins - 1)) * 0.11 - (col / Math.max(1, bins - 1)) * 0.05
    return Number((ridge + slope).toFixed(4))
  })
  const meanValues = values.map((value, index) => (
    value == null
      ? null
      : Number((value - Math.abs(Math.sin(index / 7)) * 0.012).toFixed(4))
  ))
  const sampleCounts = values.map((value, index) => (
    value == null ? 0 : 1 + (index % 3)
  ))
  return {
    bins_fast: bins,
    bins_slow: bins,
    fast_min: fastMin,
    fast_max: fastMax,
    slow_min: slowMin,
    slow_max: slowMax,
    cell_order: 'slow_fast_row_major_v1',
    values,
    mean_values: meanValues,
    sample_counts: sampleCounts,
  }
}

function strategySeriesFixtureValue(
  definition: StrategySeriesFixtureDefinition,
  index: number,
  seriesIndex: number,
): number | null {
  const warmup = STRATEGY_BUNDLE_START_INDEX + (seriesIndex % 4)
  if (index < warmup) return null
  const bar = PRICE_BARS[index]
  if (!bar) return null
  const phase = index / (4.5 + (seriesIndex % 4)) + seriesIndex * 0.37
  const wave = Math.sin(phase)
  const label = definition.label.toLowerCase()
  switch (definition.unit) {
    case 'price': {
      const directionalOffset =
        label.includes('upper') || label.includes('short stop') || label.includes('lott')
          ? 0.72
          : label.includes('lower') || label.includes('long stop') || label.includes('hott')
            ? -0.72
            : 0
      return Number((bar.close + directionalOffset + wave * 0.16).toFixed(4))
    }
    case 'price_delta':
      if (definition.id.endsWith('_atr')) {
        return Number((0.48 + Math.abs(wave) * 0.74).toFixed(4))
      }
      return Number((wave * 0.82).toFixed(4))
    case 'degrees':
      return Number((wave * 38).toFixed(3))
    case 'state':
      if (definition.id === 'cdmc_cycle_state') return index % 14 < 7 ? 1 : -1
      return index % 12 < 6 ? 1 : 0
    case 'z_score':
      return Number((wave * 2.35).toFixed(4))
    case 'oscillator':
      if (definition.id.startsWith('kqsr_')) {
        return Number((50 + wave * 36).toFixed(3))
      }
      return Number((wave * 34).toFixed(3))
    case 'ratio':
      if (definition.id === 'carm_rocr') return Number((1 + wave * 0.045).toFixed(5))
      if (definition.id === 'mkcs_compression_ratio') {
        return Number((0.021 + wave * 0.008).toFixed(5))
      }
      if (definition.id === 'kqsr_prb_fraction') {
        return Number((0.5 + wave * 0.42).toFixed(4))
      }
      if (definition.id === 'kqsr_ift_rsi') return Number((wave * 0.86).toFixed(4))
      if (definition.id === 'carm_otto_regime_spread') {
        return Number((wave * 0.021).toFixed(5))
      }
      return Number((0.48 + wave * 0.32).toFixed(4))
    case 'number':
      return Number((wave * 10).toFixed(4))
  }
  return null
}

function buildStandaloneDrillCurves(strategyKind: string): Record<string, unknown> {
  const fixture = STANDALONE_VISUALIZATIONS[strategyKind]
  if (!fixture) throw new Error(`Missing standalone visualization fixture for ${strategyKind}`)
  return {
    ...BASE_DRILL_CURVES,
    strategy_indicator_bundle: {
      strategy_kind: strategyKind,
      aggregation: 'last_finite',
      bars: PRICE_BARS.slice(STRATEGY_BUNDLE_START_INDEX).map((bar, index) => ({
        time: bar.time * 1000,
        from_index: index + STRATEGY_BUNDLE_START_INDEX,
        to_index: index + STRATEGY_BUNDLE_START_INDEX,
      })),
      series: fixture.series.map((definition, seriesIndex) => ({
        id: definition.id,
        label: definition.label,
        kind: definition.kind,
        pane_id: definition.pane_id,
        scale_id: definition.scale_id,
        unit: definition.unit,
        default_visible: definition.default_visible,
        values: PRICE_BARS.slice(STRATEGY_BUNDLE_START_INDEX).map((_, index) =>
          strategySeriesFixtureValue(
            definition,
            index + STRATEGY_BUNDLE_START_INDEX,
            seriesIndex,
          )),
        ...(definition.reference_lines?.length
          ? { reference_lines: definition.reference_lines.map((line) => ({ ...line })) }
          : {}),
      })),
    },
  }
}

function buildHeatmapSliceRows(): { all: unknown[]; all_scores: number[] } {
  const all: unknown[] = []
  const all_scores: number[] = []
  for (const offset of [0.75, 0.85]) {
    for (let fast = 5; fast <= 18; fast += 1) {
      for (let slow = Math.max(20, fast + 1); slow <= 35; slow += 2) {
        const score = Number((
          Math.cos((slow - fast) / 5) * 0.16
          + (offset === 0.75 ? 0.08 : -0.03)
          + slow / 360
          - fast / 520
        ).toFixed(4))
        all.push([
          {
            fast_len: fast,
            slow_len: slow,
            fast_ma_type: 'alma',
            slow_ma_type: 'sma',
            fast_ma_params: { offset, sigma: 6 },
            slow_ma_params: null,
          },
          {
            pnl: score * 1000,
            sharpe: score,
            max_dd: Math.max(0.01, 0.12 - score / 10),
            trades: 24,
            exposure: 0.5,
            net_exposure: 0.1,
          },
        ])
        all_scores.push(score)
      }
    }
  }
  return { all, all_scores }
}

function buildDoubleMaRunResult(): Record<string, unknown> {
  const sliceRows = buildHeatmapSliceRows()
  return {
    best_params: DOUBLE_PARAMS,
    best_metrics: BASE_METRICS,
    top: [
      [DOUBLE_PARAMS, BASE_METRICS],
      [
        { fast_len: 10, slow_len: 36, fast_ma_type: 'ema', slow_ma_type: 'sma' },
        { ...BASE_METRICS, pnl: 0.1, sharpe: 1.22, max_dd: -0.06, trades: 16 },
      ],
    ],
    heatmap: buildHeatmapFixture(16),
    all: sliceRows.all,
    all_scores: sliceRows.all_scores,
    runtime_ms: 84,
    backend_used: 'cpu',
    backend_route: 'CpuOnly',
    followup_cache_key: 'qa-double-family-cache',
  }
}

function buildCompositeCpuRunResult(strategyKind = 'CompositeCpu'): Record<string, unknown> {
  const standalone = STANDALONE_VISUALIZATIONS[strategyKind]
  const strategyCacheKey = sanitizeFilePart(strategyKind)
  return {
    best_params: COMPOSITE_PARAMS,
    best_metrics: BASE_METRICS,
    top: [
      [COMPOSITE_PARAMS, BASE_METRICS],
      [
        { fast_len: 11, slow_len: 39, fast_ma_type: 'ema', slow_ma_type: 'sma' },
        { ...BASE_METRICS, pnl: 0.09, sharpe: 1.18, max_dd: -0.07, trades: 15 },
      ],
    ],
    runtime_ms: 96,
    backend_used: 'Cpu',
    backend_route: standalone ? 'CpuOnly' : 'CompositeCpu',
    followup_cache_key: `qa-${strategyCacheKey}-cache`,
    ...(standalone ? { heatmap: buildHeatmapFixture() } : {}),
  }
}

function buildCompositeMoRunResult(): Record<string, unknown> {
  return {
    strategy_kind: 'CompositeMo',
    backend_route: 'CompositeMo',
    runtime_ms: 112,
    pareto: [
      {
        params: {
          fast_len: 8,
          slow_len: 31,
          fast_ma_type: 'ema',
          slow_ma_type: 'sma',
          signal_mode: 'entry_core_exit_overlay',
          halftrend: { amplitude: 9, atr_period: 21, channel_deviation: 1.6 },
          macz: { fast_length: 12, slow_length: 26, signal_length: 9 },
          ott: { period: 6, percent: 1.3, ma_type: 'EMA' },
          strategy: { long_only: false, allow_flip: false },
        },
        metrics: BASE_METRICS,
        maximize_value: 1.4,
        minimize_value: 0.05,
        violation: 0,
      },
    ],
  }
}

function buildValidationResult(route: string): Record<string, unknown> {
  return {
    backend_used: 'Cpu',
    backend_route: route,
    backend_fallback_reason: '',
    runtime_ms: 42,
    candidates: [
      {
        params: DOUBLE_PARAMS,
        in_sample: BASE_METRICS,
        robust_score: 1.21,
        psr: 0.91,
        dsr: 0.84,
        robust_pareto: true,
        validation_verdict: {
          passed: true,
          approved: true,
          final_recommendation: 'approved',
        },
      },
    ],
    cross_ticker: { datasets: [] },
    run_verdict: {
      passed: true,
      approved: true,
      candidates_tested: 1,
      candidates_survived_hard_gates: 1,
      approved_candidate_count: 1,
      first_approved_rank: 1,
      final_recommendation: 'approved',
    },
  }
}

function conventionalScenario(strategy: ConventionalStrategyCase): MockTauriScenario {
  const standalone = STANDALONE_VISUALIZATIONS[strategy.kind]
  return {
    ...emptyAppScenario,
    name: `conventional-strategy-workflow-${sanitizeFilePart(strategy.kind)}`,
    family: 'optimization',
    description: `${strategy.label} can load data, optimize, inspect contract-faithful visuals, validate, and enter analysis.`,
    commandResults: {
      ...emptyAppScenario.commandResults,
      load_price_data: {
        id: 'fixture_aapl_1m',
        meta: DATASET_META,
        cache_hit: true,
        cache_path: 'C:/VectorGridData/cache/aapl_1m.vta',
        load_source: 'binary_cache',
      },
      preview_dynamic_double_ma_strategy_ir_json: {
        schema_compatibility: 'Compatible',
        ir: { profile: 'double_ma' },
        pretty_json: '{\n  "profile": "double_ma"\n}',
        preview: {
          requested_backend: 'gpu_unified',
          resolved_backend: 'cpu',
          plan_fingerprint_hex: 'qa-dynamic-plan',
          indicator_count: 2,
          condition_count: 2,
          group_count: 2,
          capability: {
            gpu_dynamic_supported: false,
            cpu_dynamic_supported: true,
            reasons: ['browser QA fixture uses CPU dynamic route'],
          },
        },
      },
      run_double_ma_optimization: buildDoubleMaRunResult(),
      run_composite_cpu_optimization: buildCompositeCpuRunResult(strategy.kind),
      run_composite_mo_optimization: buildCompositeMoRunResult(),
      compute_double_ma_drilldown: DOUBLE_MA_DRILL_CURVES,
      compute_composite_cpu_drilldown: standalone
        ? buildStandaloneDrillCurves(strategy.kind)
        : LEGACY_COMPOSITE_DRILL_CURVES,
      compute_double_ma_local_stability: {
        base: DOUBLE_PARAMS,
        objective: 'Sharpe',
        radius: 1,
        step: 1,
        start_t: 1,
        points: [],
        summary: { base: 0, min: 0, median: 0, max: 0 },
      },
      compute_composite_cpu_local_stability: {
        base: COMPOSITE_PARAMS,
        objective: 'Sharpe',
        radius: 1,
        step: 1,
        start_t: 1,
        points: [],
        summary: { base: 0, min: 0, median: 0, max: 0 },
      },
      get_price_bars: PRICE_BARS,
      run_double_ma_validation: buildValidationResult('CpuOnly'),
      run_composite_validation: buildValidationResult(standalone ? 'CpuOnly' : 'CompositeCpu'),
      save_run_artifact: {
        path: 'C:/runs/vectorgrid_run_conventional_strategy_qa.json',
        bytes: 73728,
        artifact_updated: true,
      },
    },
  }
}

function requireStrategyCase(strategyKind: string): ConventionalStrategyCase {
  const strategy = STRATEGIES.find((candidate) => candidate.kind === strategyKind)
  if (!strategy) throw new Error(`Missing conventional strategy case for ${strategyKind}`)
  return strategy
}

function standaloneStrategySwitchScenario(): MockTauriScenario {
  const base = conventionalScenario(requireStrategyCase('Vcc'))
  return {
    ...base,
    name: 'standalone-native-series-switch-vcc-vemr',
    description:
      'A CPU-fallback browser fixture switches between two GPU-capable standalone strategy contracts without retaining stale indicator panes.',
    commandResults: {
      ...base.commandResults,
      run_composite_cpu_optimization: mockTauriSequence(
        buildCompositeCpuRunResult('Vcc'),
        buildCompositeCpuRunResult('Vemr'),
      ),
      compute_composite_cpu_drilldown: mockTauriSequence(
        buildStandaloneDrillCurves('Vcc'),
        buildStandaloneDrillCurves('Vemr'),
      ),
    },
  }
}

async function loadData(page: Page): Promise<void> {
  await action(page, 'shell.workflowStep.data').click()
  await action(page, 'data.sample.load').first().click()
  await expect
    .poll(async () => (await readTauriInvokeLog(page)).some((entry) => entry.command === 'load_price_data'))
    .toBe(true)
}

async function configureHeatmapBins(page: Page, bins: number): Promise<void> {
  await action(page, 'shell.controlsTab.advanced').click()
  await expect(page.locator('#controls_panel_advanced')).toBeVisible()
  const input = page.locator('#vueHeatmapBins')
  await expect(input).toBeVisible()
  await input.fill(String(bins))
  await input.blur()
  await expect(input).toHaveValue(String(bins))
  await action(page, 'shell.controlsTab.optimize').click()
  await expect(page.locator('#controls_panel_opt')).toBeVisible()
}

async function canvasPaintSummary(page: Page, selector: string): Promise<{
  canvasCount: number
  paintedCanvasCount: number
  paintedSamples: number
}> {
  return page.locator(selector).evaluate((host) => {
    const canvases = Array.from(host.querySelectorAll('canvas')) as HTMLCanvasElement[]
    let paintedCanvasCount = 0
    let paintedSamples = 0
    for (const canvas of canvases) {
      if (canvas.width <= 0 || canvas.height <= 0) continue
      const ctx = canvas.getContext('2d', { willReadFrequently: true })
      if (!ctx) continue
      const xStep = Math.max(1, Math.floor(canvas.width / 20))
      const yStep = Math.max(1, Math.floor(canvas.height / 16))
      let canvasSamples = 0
      for (let y = 0; y < canvas.height; y += yStep) {
        for (let x = 0; x < canvas.width; x += xStep) {
          const pixel = ctx.getImageData(x, y, 1, 1).data
          const alpha = pixel[3] ?? 0
          const rgb = (pixel[0] ?? 0) + (pixel[1] ?? 0) + (pixel[2] ?? 0)
          if (alpha > 0 && rgb > 0) {
            canvasSamples += 1
            paintedSamples += 1
          }
        }
      }
      if (canvasSamples > 4) paintedCanvasCount += 1
    }
    return { canvasCount: canvases.length, paintedCanvasCount, paintedSamples }
  })
}

async function heatmapPainted(page: Page): Promise<boolean> {
  return page.locator('#heatmapCanvasVue').evaluate((node) => {
    const canvas = node as HTMLCanvasElement
    const ctx = canvas.getContext('2d', { willReadFrequently: true })
    if (!ctx || canvas.width <= 0 || canvas.height <= 0) return false
    const image = ctx.getImageData(0, 0, canvas.width, canvas.height)
    let paintedSamples = 0
    for (let index = 3; index < image.data.length; index += 128) {
      if (image.data[index] > 0) paintedSamples += 1
      if (paintedSamples > 24) return true
    }
    return false
  })
}

async function ensureEvidenceDir(): Promise<string> {
  const evidenceDir = path.resolve(process.cwd(), '../output/playwright/conventional-strategy-workflow')
  await fs.mkdir(evidenceDir, { recursive: true })
  return evidenceDir
}

async function writeEvidence(kind: string, evidence: Record<string, unknown>): Promise<void> {
  const evidenceDir = await ensureEvidenceDir()
  await fs.writeFile(
    path.join(evidenceDir, `${sanitizeFilePart(kind)}.json`),
    `${JSON.stringify(evidence, null, 2)}\n`,
    'utf8',
  )
}

test.describe('conventional strategy workflow matrix', () => {
  for (const strategy of STRATEGIES) {
    test(`runs ${strategy.label} through optimization, visual review, validation, and analysis @workflow @visual`, async ({
      page,
      context,
    }) => {
      const guard = installQaGuards(page)
      const evidenceDir = await ensureEvidenceDir()
      const standaloneVisualization = STANDALONE_VISUALIZATIONS[strategy.kind]
      await page.setViewportSize({ width: 1600, height: 1000 })
      await gotoMockedShellWithScenario(page, context, conventionalScenario(strategy))

      await loadData(page)
      await action(page, 'shell.workflowStep.optimize').click()
      await page.locator('#vueStrategyKind').selectOption(strategy.kind)
      await strategy.setup?.(page)
      if (strategy.expectsHeatmap) {
        await configureHeatmapBins(page, CONVENTIONAL_HEATMAP_BINS)
      }

      await expect(action(page, 'optimize.run')).toBeEnabled()
      await action(page, 'optimize.run').click()
      await expect
        .poll(async () => (await readTauriInvokeLog(page)).some((entry) => entry.command === strategy.optimizationCommand))
        .toBe(true)

      const optimizationEntry = (await readTauriInvokeLog(page)).find((entry) => entry.command === strategy.optimizationCommand)
      expect(optimizationEntry?.args?.req, `${strategy.kind} optimization request`).toMatchObject({
        strategy_kind: strategy.kind,
      })
      if (strategy.expectsHeatmap) {
        expect(
          (optimizationEntry?.args?.req as Record<string, unknown> | undefined)?.heatmap_bins,
          `${strategy.kind} requested heatmap bins`,
        ).toBe(CONVENTIONAL_HEATMAP_BINS)
      }
      if (strategy.kind === 'DynamicDoubleMa') {
        expect((await readTauriInvokeLog(page)).some((entry) => entry.command === 'preview_dynamic_double_ma_strategy_ir_json')).toBe(true)
      }

      await action(page, 'shell.resultsTab.top').click()
      await expect(page.locator('#results_panel_top')).toBeVisible()
      await expect(page.locator('.topTableVue [data-top-row="1"]').first()).toBeVisible()
      await expect(action(page, 'top.selected.openCandidateDetail')).toBeEnabled()

      if (strategy.expectsPareto) {
        await action(page, 'shell.resultsTab.pareto').click()
        await expect(page.locator('#results_panel_pareto')).toBeVisible()
        await expect(page.getByText(/Pareto/i).first()).toBeVisible()
        await action(page, 'shell.resultsTab.top').click()
      }

      let heatmapSummary: Record<string, unknown> = { expected: strategy.expectsHeatmap }
      if (strategy.expectsHeatmap) {
        await action(page, 'shell.resultsTab.heatmap').click()
        await expect(page.locator('#results_panel_heatmap')).toBeVisible()
        const heatmapStack = page.locator('#heatmapCanvasStackVue')
        await expect(heatmapStack).toBeVisible()
        await expect(heatmapStack).toHaveAttribute('data-render-ready', 'true')
        await expect(heatmapStack).toHaveAttribute(
          'data-total-cells',
          String(CONVENTIONAL_HEATMAP_BINS * CONVENTIONAL_HEATMAP_BINS),
        )
        await expect(heatmapStack).toHaveAttribute('data-finite-cells', /^[1-9]\d*$/)
        if (standaloneVisualization) {
          await expect(page.locator('#heatmapMetaVue')).toContainText(
            `bins=${CONVENTIONAL_HEATMAP_BINS}x${CONVENTIONAL_HEATMAP_BINS}`,
          )
          const expectedAxisSummary =
            `${standaloneVisualization.fastAxisLabel} 5-20; ${standaloneVisualization.slowAxisLabel} 20-35`
          await expect(heatmapStack).toHaveAttribute('data-axis-summary', expectedAxisSummary)
          await expect(page.locator('#heatmapMetaVue')).toContainText(
            standaloneVisualization.fastAxisLabel,
          )
          await expect(page.locator('#heatmapMetaVue')).toContainText(
            standaloneVisualization.slowAxisLabel,
          )
          await expect(page.locator('#vueHeatmapViewMode option[value="3d"]')).toBeDisabled()
          await expect(page.locator('#heatmap3dAvailabilityReasonVue')).toContainText(
            /3D slices are unavailable for standalone strategy heatmaps/i,
          )
          await expect(page.locator('#vueHeatmap2dSource')).toHaveCount(0)
        }
        await expect.poll(async () => heatmapPainted(page)).toBe(true)
        heatmapSummary = {
          expected: true,
          painted: await heatmapPainted(page),
          meta: await page.locator('#heatmapMetaVue').innerText(),
          axisSummary: await heatmapStack.getAttribute('data-axis-summary'),
          finiteCells: await heatmapStack.getAttribute('data-finite-cells'),
        }
        await page.locator('#results_panel_heatmap').screenshot({
          path: path.join(evidenceDir, `${sanitizeFilePart(strategy.kind)}-heatmap.png`),
        })
      }

      await action(page, 'shell.resultsTab.top').click()
      await action(page, 'top.selected.openCandidateDetail').click()
      await expect(page.locator('#results_panel_drill')).toBeVisible()
      await expect(page.locator('#drillTvChartVue')).toBeVisible()
      const chartMeta = page.locator('#drillTvChartMetaVue')
      const chartFrame = page.locator('#results_panel_drill .tvChartFrame')
      const chartLegend = page.locator('#results_panel_drill .chartLegendOverlay')
      await expect(chartMeta).toContainText(/candles=72/)
      await expect(chartMeta).toContainText(/entries=3/)
      await expect(chartMeta).toContainText(/exits=3/)
      await expect(chartMeta).toContainText(/markers=6/)
      if (standaloneVisualization) {
        const defaultVisibleSeries = standaloneVisualization.series.filter(
          (series) => series.default_visible,
        )
        const activeNativePaneCount =
          1 + new Set(
            defaultVisibleSeries
              .map((series) => series.pane_id)
              .filter((paneId) => paneId !== 'price'),
          ).size
        const controls = page.locator(
          `details[data-testid="native-indicator-controls"][data-native-strategy="${strategy.kind}"]`,
        )
        await expect(controls).toBeVisible()
        await expect(chartFrame).toHaveAttribute('data-native-strategy', strategy.kind)
        await expect(chartFrame).toHaveAttribute(
          'data-native-series-visible',
          String(defaultVisibleSeries.length),
        )
        await expect(chartFrame).toHaveAttribute(
          'data-native-series-total',
          String(standaloneVisualization.series.length),
        )
        await expect(chartFrame).toHaveAttribute(
          'data-native-pane-count',
          String(activeNativePaneCount),
        )
        const checkboxes = controls.locator(
          'input[type="checkbox"][data-indicator-series-id]',
        )
        await expect(checkboxes).toHaveCount(standaloneVisualization.series.length)
        await expect(
          controls.locator('input[type="checkbox"][data-indicator-series-id]:checked'),
        ).toHaveCount(defaultVisibleSeries.length)
        const firstRepresentativeDefinition = standaloneVisualization.series.find(
          (series) => series.label === standaloneVisualization.representativeLabels[0],
        )
        const secondRepresentativeDefinition = standaloneVisualization.series.find(
          (series) => series.label === standaloneVisualization.representativeLabels[1],
        )
        expect(firstRepresentativeDefinition).toBeTruthy()
        expect(secondRepresentativeDefinition).toBeTruthy()
        const firstRepresentative = controls.locator(
          `[data-indicator-series-id="${firstRepresentativeDefinition?.id || ''}"]`,
        )
        if (!(await firstRepresentative.isVisible())) {
          await controls.locator('summary').click()
        }
        await expect(firstRepresentative).toBeVisible()
        await expect(firstRepresentative).toHaveAttribute('aria-controls', 'drillTvChartVue')
        await expect(
          controls.getByRole('checkbox', {
            name: standaloneVisualization.representativeLabels[0],
            exact: true,
          }),
        ).toHaveAttribute(
          'data-indicator-series-id',
          firstRepresentativeDefinition?.id || '',
        )
        await expect(
          controls.locator(
            `[data-indicator-series-id="${secondRepresentativeDefinition?.id || ''}"]`,
          ),
        ).toBeVisible()
        await expect(
          controls.getByRole('checkbox', {
            name: standaloneVisualization.representativeLabels[1],
            exact: true,
          }),
        ).toHaveAttribute(
          'data-indicator-series-id',
          secondRepresentativeDefinition?.id || '',
        )
        await expect(
          chartLegend.getByText(standaloneVisualization.representativeLabels[0], {
            exact: true,
          }),
        ).toBeVisible()
        await expect(chartMeta).not.toContainText(/fast=|slow=|halftrend=|ott=|macz=/i)
        await expect(chartLegend).not.toContainText(
          /Fast MA|Slow MA|HalfTrend|MAC-Z|\bOTT\b/i,
        )
      } else {
        await expect(chartMeta).toContainText(/fast=72/)
        await expect(chartMeta).toContainText(/slow=72/)
        if (strategy.optimizationCommand !== 'run_double_ma_optimization' || strategy.expectsPareto) {
          await expect(chartMeta).toContainText(/halftrend=72/)
          await expect(chartMeta).toContainText(/ott=72/)
          await expect(chartMeta).toContainText(/macz=72/)
          await expect(chartLegend.getByText(/HalfTrend/)).toBeVisible()
          await expect(chartLegend.getByText('OTT')).toBeVisible()
          await expect(chartLegend.getByText('MAC-Z')).toBeVisible()
        }
      }
      await expect
        .poll(async () => {
          const summary = await canvasPaintSummary(page, '#drillTvChartVue')
          return summary.canvasCount > 0 && summary.paintedCanvasCount > 0 && summary.paintedSamples > 16
        })
        .toBe(true)
      const initialChartView = await page.locator('.tvChartFrame[data-chart-ready="true"]').getAttribute('data-chart-view')
      await action(page, 'candidateChart.panLeft').click()
      await expect
        .poll(async () => page.locator('.tvChartFrame[data-chart-ready="true"]').getAttribute('data-chart-view'))
        .not.toBe(initialChartView)
      const pannedLeftChartView = await page.locator('.tvChartFrame[data-chart-ready="true"]').getAttribute('data-chart-view')
      await action(page, 'candidateChart.panRight').click()
      await expect
        .poll(async () => page.locator('.tvChartFrame[data-chart-ready="true"]').getAttribute('data-chart-view'))
        .not.toBe(pannedLeftChartView)
      await action(page, 'candidateChart.fitContent').click()
      await expect(page.locator('#drillTvChartMetaVue')).toContainText(/view=/)
      const chartSummary = await canvasPaintSummary(page, '#drillTvChartVue')
      const chartView = await page.locator('.tvChartFrame[data-chart-ready="true"]').getAttribute('data-chart-view')
      await page.locator('.tvChartFrame').screenshot({
        path: path.join(evidenceDir, `${sanitizeFilePart(strategy.kind)}-candidate-detail-chart.png`),
      })

      await action(page, 'shell.workflowStep.validate').click()
      await expect(action(page, 'validation.run')).toBeEnabled()
      await action(page, 'validation.run').click()
      let validationEntry = (await readTauriInvokeLog(page)).find((entry) => entry.command === strategy.validationCommand)
      let validationBlocked = false
      let validationStatusText = ''
      if (strategy.expectedValidationBlock) {
        await expect(page.getByText(strategy.expectedValidationBlock).first()).toBeVisible()
        validationBlocked = true
        validationStatusText = await page.getByText(strategy.expectedValidationBlock).first().innerText()
      } else {
        await expect
          .poll(async () => (await readTauriInvokeLog(page)).some((entry) => entry.command === strategy.validationCommand))
          .toBe(true)
        validationEntry = (await readTauriInvokeLog(page)).find((entry) => entry.command === strategy.validationCommand)
        expect(validationEntry?.args?.req, `${strategy.kind} validation request`).toBeTruthy()
        if (strategy.kind === 'DynamicDoubleMa') {
          expect((validationEntry?.args?.req as Record<string, unknown>).dynamic_strategy_ir).toEqual({
            profile: 'double_ma',
          })
        }
        await expect(page.getByText(/Validation completed in/i).first()).toBeVisible()
        await expect(page.locator('.validationTableVue tbody tr').first()).toBeVisible()
      }

      await action(page, 'shell.workflowStep.analyze').click()
      await expect(page.locator('#resultsTabBar')).toBeVisible()
      await expect(action(page, 'shell.resultsTab.drill')).toBeVisible()
      if (strategy.expectsHeatmap) {
        await expect(action(page, 'shell.resultsTab.heatmap')).toBeVisible()
      }
      await expectNoHorizontalDocumentOverflow(page)
      await expectNoStuckBusyState(page)

      const invokes = await readTauriInvokeLog(page)
      await writeEvidence(strategy.kind, {
        strategy: strategy.kind,
        optimizationCommand: strategy.optimizationCommand,
        validationCommand: strategy.validationCommand,
        optimizationStrategyKind: (optimizationEntry?.args?.req as Record<string, unknown> | undefined)?.strategy_kind,
        optimizationInvokeCount: invokes.filter((entry) => entry.command === strategy.optimizationCommand).length,
        validationInvokeCount: invokes.filter((entry) => entry.command === strategy.validationCommand).length,
        validationBlocked,
        validationStatusText,
        chartSummary,
        chartView,
        heatmapSummary,
      })

      await guard.assertClean()
    })
  }

  test('switches VCC to VEMR without stale native panes and toggles the hidden ATR pane @workflow @visual', async ({
    page,
    context,
  }) => {
    const guard = installQaGuards(page)
    await page.setViewportSize({ width: 1600, height: 1000 })
    await gotoMockedShellWithScenario(page, context, standaloneStrategySwitchScenario())
    await loadData(page)

    await action(page, 'shell.workflowStep.optimize').click()
    await page.locator('#vueStrategyKind').selectOption('Vcc')
    await configureHeatmapBins(page, CONVENTIONAL_HEATMAP_BINS)
    await action(page, 'optimize.run').click()
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).filter(
          (entry) => entry.command === 'run_composite_cpu_optimization',
        ).length)
      .toBe(1)
    await action(page, 'shell.resultsTab.top').click()
    await action(page, 'top.selected.openCandidateDetail').click()

    const chartFrame = page.locator('#results_panel_drill .tvChartFrame')
    await expect(chartFrame).toHaveAttribute('data-chart-ready', 'true')
    await expect(chartFrame).toHaveAttribute('data-native-strategy', 'Vcc')
    await expect(chartFrame).toHaveAttribute('data-native-series-visible', '3')
    await expect(chartFrame).toHaveAttribute('data-native-series-total', '3')
    await expect(chartFrame).toHaveAttribute('data-native-pane-count', '3')
    await expect(
      page.locator(
        'details[data-testid="native-indicator-controls"][data-native-strategy="Vcc"]',
      ),
    ).toBeVisible()

    await action(page, 'shell.workflowStep.optimize').click()
    await page.locator('#vueStrategyKind').selectOption('Vemr')
    await action(page, 'optimize.run').click()
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).filter(
          (entry) => entry.command === 'run_composite_cpu_optimization',
        ).length)
      .toBe(2)
    await action(page, 'shell.resultsTab.top').click()
    await expect(action(page, 'top.selected.openCandidateDetail')).toBeEnabled()
    await action(page, 'top.selected.openCandidateDetail').click()

    await expect(chartFrame).toHaveAttribute('data-chart-ready', 'true')
    await expect(chartFrame).toHaveAttribute('data-native-strategy', 'Vemr')
    await expect(chartFrame).toHaveAttribute('data-native-series-visible', '4')
    await expect(chartFrame).toHaveAttribute('data-native-series-total', '5')
    await expect(chartFrame).toHaveAttribute('data-native-pane-count', '2')
    await expect(
      page.locator(
        'details[data-testid="native-indicator-controls"][data-native-strategy="Vcc"]',
      ),
    ).toHaveCount(0)
    await expect(page.locator('#results_panel_drill .chartLegendOverlay')).not.toContainText(
      /Squeeze state|Squeeze momentum|Trend state/,
    )

    const vemrControls = page.locator(
      'details[data-testid="native-indicator-controls"][data-native-strategy="Vemr"]',
    )
    await expect(vemrControls).toBeVisible()
    const atrToggle = vemrControls.locator(
      '[data-indicator-series-id="vemr_atr"]',
    )
    if (!(await atrToggle.isVisible())) {
      await vemrControls.locator('summary').click()
    }
    await expect(atrToggle).toBeVisible()
    await expect(atrToggle).not.toBeChecked()
    await atrToggle.check()
    await expect(atrToggle).toBeChecked()
    await expect(chartFrame).toHaveAttribute('data-native-series-visible', '5')
    await expect(chartFrame).toHaveAttribute('data-native-pane-count', '3')
    await expect(chartFrame).toHaveAttribute('data-chart-ready', 'true')
    await expect(
      page.locator('#results_panel_drill .chartLegendOverlay').getByText(
        'Average true range',
        { exact: true },
      ),
    ).toBeVisible()
    await expect
      .poll(async () => {
        const summary = await canvasPaintSummary(page, '#drillTvChartVue')
        return summary.canvasCount > 0 && summary.paintedCanvasCount > 0
      })
      .toBe(true)

    await atrToggle.uncheck()
    await expect(atrToggle).not.toBeChecked()
    await expect(chartFrame).toHaveAttribute('data-native-series-visible', '4')
    await expect(chartFrame).toHaveAttribute('data-native-pane-count', '2')
    await expect(chartFrame).toHaveAttribute('data-chart-ready', 'true')
    await expect(
      page.locator('#results_panel_drill .chartLegendOverlay'),
    ).not.toContainText('Average true range')
    await expectNoHorizontalDocumentOverflow(page)
    await expectNoStuckBusyState(page)
    await guard.assertClean()
  })
})
