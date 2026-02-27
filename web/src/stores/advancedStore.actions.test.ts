import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import { resetMoSearchSpace } from '../services/advancedService'
import { useAdvancedStore } from './advancedStore'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))
vi.mock('../services/advancedService', () => ({
  resetMoSearchSpace: vi.fn(async () => ({ searchSpace: { params: [] }, json: '{\n  \"params\": []\n}' })),
}))

const invokeCommandMock = vi.mocked(invokeCommand)
const resetMoSearchSpaceMock = vi.mocked(resetMoSearchSpace)

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
    heatmapBins: makeField('heatmapBins', '128'),
    mode: makeField('mode', 'Grid'),
    includeAll: makeField('includeAll', 'false'),
    moSampler: makeField('moSampler', 'random'),
    exportAllCsvPath: makeField('exportAllCsvPath', ''),
    moSearchSpace: makeField('moSearchSpace', ''),
    vdpsDeterministic: makeCheckbox('vdpsDeterministic', true),
    moPrescanEnable: makeCheckbox('moPrescanEnable', false),
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
    resetMoSearchSpaceMock.mockClear()
    resetMoSearchSpaceMock.mockResolvedValue({ searchSpace: { params: [] }, json: '{\n  \"params\": []\n}' })
  })

  it('updates Vue advanced state without legacy DOM mirroring', () => {
    const store = useAdvancedStore()
    store.setInputField('heatmapBins', '256')
    store.setInputField('moSampler', 'tpe')
    store.setSelectField('mode', 'Grid')
    store.setSelectField('includeAll', 'true')
    store.setCheckboxField('vdpsDeterministic', false)
    store.setCheckboxField('moPrescanEnable', true)

    expect(store.heatmapBins).toBe('256')
    expect(store.moSampler).toBe('tpe')
    expect(store.mode).toBe('Grid')
    expect(store.includeAll).toBe('true')
    expect(store.vdpsDeterministic).toBe(false)
    expect(store.moPrescanEnable).toBe(true)
    expect(store.source).toBe('vue')
    expect(harness.fields.heatmapBins?.value).toBe('128')
    expect(harness.fields.moSampler?.value).toBe('random')
    expect(harness.fields.mode?.value).toBe('Grid')
    expect(harness.fields.includeAll?.value).toBe('false')
    expect(harness.fields.vdpsDeterministic?.checked).toBe(true)
    expect(harness.fields.moPrescanEnable?.checked).toBe(false)
    expect(harness.eventsById.heatmapBins).toBeUndefined()
    expect(harness.eventsById.moSampler).toBeUndefined()
    expect(harness.eventsById.mode).toBeUndefined()
    expect(harness.eventsById.includeAll).toBeUndefined()
    expect(harness.eventsById.vdpsDeterministic).toBeUndefined()
    expect(harness.eventsById.moPrescanEnable).toBeUndefined()
  })

  it('dispatches export browse via command picker and updates Vue state', async () => {
    invokeCommandMock.mockResolvedValue('C:/exports/all_pairs.csv')
    const store = useAdvancedStore()
    store.browseExportAll()
    await Promise.resolve()
    await Promise.resolve()

    expect(invokeCommandMock).toHaveBeenCalledWith('pick_save_csv', { defaultName: 'all_pairs.csv' })
    expect(store.exportAllCsvPath).toBe('C:/exports/all_pairs.csv')
    expect(harness.fields.exportAllCsvPath?.value).toBe('')
    expect(harness.eventsById.exportAllCsvPath).toBeUndefined()
  })

  it('resets Composite MO search space via service and updates Vue state', async () => {
    const store = useAdvancedStore()
    store.canResetMoSearchSpace = true
    store.resetMoSearchSpace()
    await Promise.resolve()
    await Promise.resolve()

    expect(resetMoSearchSpaceMock).toHaveBeenCalled()
    expect(store.moSearchSpace).toContain('"params"')
    expect(harness.fields.moSearchSpace?.value).toBe('')
    expect(harness.eventsById.moSearchSpace).toBeUndefined()
    expect(store.statusLine).toBe('Reset Composite MO search space.')
  })
})
