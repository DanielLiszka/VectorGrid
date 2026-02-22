import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useStrategyStore } from './strategyStore'

interface LegacyField {
  value: string
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
  const fields: Record<string, LegacyField> = {
    feeBps: makeField('feeBps', '0'),
    exitMode: makeField('exitMode', 'cross'),
    costPreset: makeField('costPreset', 'custom'),
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

  it('applies strategy field updates via legacy DOM events', () => {
    const store = useStrategyStore()
    store.setInputField('feeBps', '1.25')
    store.setSelectField('exitMode', 'cross')
    store.setSelectField('costPreset', 'custom')

    expect(harness.fields.feeBps?.value).toBe('1.25')
    expect(harness.fields.exitMode?.value).toBe('cross')
    expect(harness.fields.costPreset?.value).toBe('custom')
    expect(harness.eventsById.feeBps).toEqual(['input'])
    expect(harness.eventsById.exitMode).toEqual(['change'])
    expect(harness.eventsById.costPreset).toEqual(['change'])
  })
})
