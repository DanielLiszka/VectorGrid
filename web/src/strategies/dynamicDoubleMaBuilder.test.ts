import { describe, expect, it } from 'vitest'
import type { DynamicIndicatorCatalogEntry } from '../services/dynamicIndicatorCatalogService'
import {
  builderUsesConstantOperands,
  builderEligibleDynamicIndicators,
  buildDynamicDoubleMaStrategyIrFromBuilder,
  createDefaultDynamicDoubleMaBuilderState,
  setDynamicDoubleMaBuilderLongOnly,
} from './dynamicDoubleMaBuilder'

const sampleCatalog: DynamicIndicatorCatalogEntry[] = [
  {
    id: 'rsi',
    label: 'RSI',
    category: 'momentum',
    dynamicStrategyEligible: true,
    inputKind: 'slice',
    requiresCandles: false,
    supportsCpuSingle: true,
    supportsCpuBatch: true,
    supportsCudaSingle: true,
    supportsCudaSweep: true,
    supportsCudaVram: true,
    supportsGpuKernel: true,
    prefersGpuBySpeedup: true,
    outputCount: 1,
    outputs: [{ id: 'value', label: 'Value', valueType: 'f64' }],
    params: [
      {
        key: 'length',
        label: 'Length',
        kind: 'int',
        required: true,
        defaultValue: { kind: 'int', value: 14 },
        min: 1,
        max: 100,
        step: 1,
        enumValues: [],
        notes: '',
      },
    ],
    notes: '',
  },
]

describe('dynamicDoubleMaBuilder', () => {
  it('filters builder-eligible indicators to floating-output defaultable entries', () => {
    expect(builderEligibleDynamicIndicators(sampleCatalog)).toHaveLength(1)
    expect(builderEligibleDynamicIndicators(sampleCatalog)[0]?.id).toBe('rsi')
  })

  it('builds a constrained DoubleMa strategy_ir with static fast/slow indicators plus builder slots', () => {
    const builder = createDefaultDynamicDoubleMaBuilderState(sampleCatalog)
    const ir = buildDynamicDoubleMaStrategyIrFromBuilder({
      fastMaType: 'sma',
      slowMaType: 'alma',
      fastMaParams: null,
      slowMaParams: null,
      builder,
    })

    expect(ir.profile).toBe('double_ma')
    expect(ir.fast_ma.ma_type).toBe('sma')
    expect(ir.slow_ma.ma_type).toBe('alma')
    expect(ir.execution.strategy.trade_on_next_bar).toBe(true)
    expect(ir.program.indicators[0]).toMatchObject({ id: 'fast_ma', kind: 'fast_ma' })
    expect(ir.program.indicators[1]).toMatchObject({ id: 'slow_ma', kind: 'slow_ma' })
    expect(ir.program.indicators.some((row) => row.id === 'indicator_1')).toBe(true)
    expect(ir.program.signals.long_entry_group).toBe('long_entry_group')
    expect(ir.program.signals.short_entry_group).toBeNull()
    expect(ir.program.groups).toHaveLength(2)
    expect(ir.program.conditions).toHaveLength(2)
    expect(ir.execution.strategy.allow_flip).toBe(false)
  })

  it('normalizes percent-style risk overlay fields before building strategy_ir', () => {
    const builder = createDefaultDynamicDoubleMaBuilderState(sampleCatalog)
    builder.stopLossPct = '5'
    builder.takeProfitPct = '0.025'

    const ir = buildDynamicDoubleMaStrategyIrFromBuilder({
      fastMaType: 'sma',
      slowMaType: 'alma',
      fastMaParams: null,
      slowMaParams: null,
      builder,
    })

    expect(ir.execution.strategy.stop_loss_pct).toBe(0.05)
    expect(ir.execution.strategy.take_profit_pct).toBe(0.025)
  })

  it('rejects invalid risk overlay fields and malformed indicator params', () => {
    const builder = createDefaultDynamicDoubleMaBuilderState(sampleCatalog)
    builder.stopLossPct = '100'

    expect(() =>
      buildDynamicDoubleMaStrategyIrFromBuilder({
        fastMaType: 'sma',
        slowMaType: 'alma',
        fastMaParams: null,
        slowMaParams: null,
        builder,
      }),
    ).toThrow(/less than 100%/)

    const malformed = createDefaultDynamicDoubleMaBuilderState(sampleCatalog)
    malformed.indicators[0]!.params.length = { kind: 'int', value: Number.NaN }

    expect(() =>
      buildDynamicDoubleMaStrategyIrFromBuilder({
        fastMaType: 'sma',
        slowMaType: 'alma',
        fastMaParams: null,
        slowMaParams: null,
        builder: malformed,
      }),
    ).toThrow(/not a finite number/)
  })

  it('restores default short-side bindings when long-only is disabled', () => {
    const builder = createDefaultDynamicDoubleMaBuilderState(sampleCatalog)
    setDynamicDoubleMaBuilderLongOnly(builder, false)

    expect(builder.longOnly).toBe(false)
    expect(builder.signals.shortEntryGroup).toBe('short_entry_group')
    expect(builder.signals.shortExitGroup).toBe('short_exit_group')
    expect(builder.groups.some((group) => group.id === 'short_entry_group')).toBe(true)
    expect(builder.conditions.some((condition) => condition.id === 'short_entry_cond')).toBe(true)
  })

  it('detects when the builder uses a constant operand', () => {
    const builder = createDefaultDynamicDoubleMaBuilderState(sampleCatalog)
    const first = builder.conditions[0]
    builder.conditions[0] = {
      id: first?.id || 'long_entry_cond',
      op: 'gt',
      lhs: first?.lhs || { kind: 'indicator', value: 'fast_ma' },
      rhs: { kind: 'constant', value: '0.5' },
    }

    expect(builderUsesConstantOperands(builder)).toBe(true)
  })
})
