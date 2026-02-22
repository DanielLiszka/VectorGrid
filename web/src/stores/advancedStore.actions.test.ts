import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import { useAdvancedStore } from './advancedStore'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

const invokeCommandMock = vi.mocked(invokeCommand)

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
    heatmapBins: makeField('heatmapBins', '128'),
    mode: makeField('mode', 'Grid'),
    includeAll: makeField('includeAll', 'false'),
    exportAllCsvPath: makeField('exportAllCsvPath', ''),
  }
  vi.stubGlobal('document', {
    getElementById: (id: string) => fields[id] ?? null,
  })
  return { fields, eventsById }
}

describe('advanced store actions', () => {
  let harness: DocumentHarness

  beforeEach(() => {
    setActivePinia(createPinia())
    harness = installDocumentHarness()
    invokeCommandMock.mockClear()
    invokeCommandMock.mockResolvedValue(undefined)
  })

  it('dispatches advanced field updates via legacy DOM events', () => {
    const store = useAdvancedStore()
    store.setInputField('heatmapBins', '256')
    store.setSelectField('mode', 'Grid')
    store.setSelectField('includeAll', 'true')

    expect(harness.fields.heatmapBins?.value).toBe('256')
    expect(harness.fields.mode?.value).toBe('Grid')
    expect(harness.fields.includeAll?.value).toBe('true')
    expect(harness.eventsById.heatmapBins).toEqual(['input'])
    expect(harness.eventsById.mode).toEqual(['change'])
    expect(harness.eventsById.includeAll).toEqual(['change'])
  })

  it('dispatches export browse via command picker and legacy DOM event', async () => {
    invokeCommandMock.mockResolvedValue('C:/exports/all_pairs.csv')
    const store = useAdvancedStore()
    store.browseExportAll()
    await Promise.resolve()
    await Promise.resolve()

    expect(invokeCommandMock).toHaveBeenCalledWith('pick_save_csv', { defaultName: 'all_pairs.csv' })
    expect(harness.fields.exportAllCsvPath?.value).toBe('C:/exports/all_pairs.csv')
    expect(harness.eventsById.exportAllCsvPath).toEqual(['input'])
  })
})
