import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useStrategyStore } from './strategyStore'

interface LegacyField {
  value: string
  checked?: boolean
  type?: string
  disabled?: boolean
  dispatchEvent: (event: Event) => boolean
}

interface DocumentHarness {
  fields: Record<string, LegacyField>
  eventsById: Record<string, string[]>
}

function installDocumentHarness(): DocumentHarness {
  const eventsById: Record<string, string[]> = {}
  const makeField = (id: string, value = ''): LegacyField => ({
    value,
    dispatchEvent: (event: Event) => {
      if (!eventsById[id]) eventsById[id] = []
      eventsById[id].push(String(event.type || '').trim())
      return true
    },
  })
  const makeCheckbox = (id: string, checked: boolean): LegacyField => ({
    value: checked ? 'on' : '',
    checked,
    type: 'checkbox',
    dispatchEvent: (event: Event) => {
      if (!eventsById[id]) eventsById[id] = []
      eventsById[id].push(String(event.type || '').trim())
      return true
    },
  })
  const fields: Record<string, LegacyField> = {
    feeBps: makeField('feeBps', '0'),
    exitMode: makeField('exitMode', 'cross'),
    costPreset: makeField('costPreset', 'custom'),
    breakoutMacdFast: makeField('breakoutMacdFast', ''),
    trendUseAdx: makeCheckbox('trendUseAdx', false),
    longOnly: makeCheckbox('longOnly', true),
  }
  vi.stubGlobal('document', {
    getElementById: (id: string) => fields[id] ?? null,
  })
  return { fields, eventsById }
}

describe('strategy store actions', () => {
  let harness: DocumentHarness

  beforeEach(() => {
    setActivePinia(createPinia())
    harness = installDocumentHarness()
  })

  it('applies strategy field updates via Vue-first state sync without legacy DOM mirroring', () => {
    const store = useStrategyStore()
    store.setInputField('feeBps', '1.25')
    store.setInputField('spreadBps', '1.00')
    store.setInputField('slippageBps', '0.50')
    store.setSelectField('exitMode', 'cross')
    store.setSelectField('costPreset', 'stocks_liquid')
    store.setInputField('breakoutMacdFast', '12')
    store.setCheckboxField('trendUseAdx', true)
    store.setCheckboxField('longOnly', false)
    store.setCheckboxField('tradeOnNext', false)

    expect(harness.fields.feeBps?.value).toBe('0')
    expect(harness.fields.exitMode?.value).toBe('cross')
    expect(harness.fields.costPreset?.value).toBe('custom')
    expect(harness.fields.breakoutMacdFast?.value).toBe('')
    expect(harness.fields.trendUseAdx?.checked).toBe(false)
    expect(harness.fields.longOnly?.checked).toBe(true)
    expect(store.feeBps).toBe('0.10')
    expect(store.spreadBps).toBe('1.00')
    expect(store.slippageBps).toBe('0.50')
    expect(store.commission).toBe('0.00011')
    expect(store.costHint).toBe('Total: 1.10 bps/side (2.20 bps round-trip)')
    expect(store.breakoutMacdFast).toBe('12')
    expect(store.trendUseAdx).toBe(true)
    expect(store.longOnly).toBe(false)
    expect(store.tradeOnNext).toBe(true)
    expect(store.source).toBe('vue')
    expect(store.updatedAtUnixMs).toBeGreaterThan(0)
    expect(harness.eventsById.feeBps).toBeUndefined()
    expect(harness.eventsById.exitMode).toBeUndefined()
    expect(harness.eventsById.costPreset).toBeUndefined()
    expect(harness.eventsById.breakoutMacdFast).toBeUndefined()
    expect(harness.eventsById.trendUseAdx).toBeUndefined()
    expect(harness.eventsById.longOnly).toBeUndefined()
  })
})
