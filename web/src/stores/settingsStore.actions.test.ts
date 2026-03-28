import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { bindRuntimePinia } from './runtimePinia'
import * as dataService from '../services/dataService'
import * as settingsService from '../services/settingsService'
import { useDataStore } from './dataStore'
import { useSettingsStore } from './settingsStore'
import { useWorkflowStore } from './workflowStore'

vi.mock('../services/dataService', () => ({
  browseMarketDataDir: vi.fn(async () => ''),
  getPolygonCacheInfo: vi.fn(async () => ({ cacheDir: 'C:/polygon', hasApiKey: false, datasets: [] })),
  openMarketDataDir: vi.fn(async () => ({ path: 'C:/default/polygon-cache', openedWith: 'open_path' })),
  openPolygon: vi.fn(async () => undefined),
  resetMarketDataDir: vi.fn(async () => ({
    defaultRootDir: 'C:/default',
    configuredRootDir: '',
    effectiveRootDir: 'C:/default',
    polygonCacheDir: 'C:/default/polygon-cache',
    hint: 'Using default root: C:/default · Polygon cache: C:/default/polygon-cache',
  })),
  setMarketDataDir: vi.fn(async () => ({
    defaultRootDir: 'C:/default',
    configuredRootDir: 'C:/custom',
    effectiveRootDir: 'C:/custom',
    polygonCacheDir: 'C:/custom/polygon-cache',
    hint: 'Using custom root: C:/custom · Polygon cache: C:/custom/polygon-cache',
  })),
}))

vi.mock('../services/settingsService', () => ({
  openReportsDir: vi.fn(async () => undefined),
  openRunsDir: vi.fn(async () => undefined),
  setWindowEffect: vi.fn(async () => undefined),
}))

const browseMarketDataDirMock = vi.mocked(dataService.browseMarketDataDir)
const openMarketDataDirMock = vi.mocked(dataService.openMarketDataDir)
const openPolygonMock = vi.mocked(dataService.openPolygon)
const resetMarketDataDirMock = vi.mocked(dataService.resetMarketDataDir)
const setMarketDataDirMock = vi.mocked(dataService.setMarketDataDir)

const openReportsDirMock = vi.mocked(settingsService.openReportsDir)
const openRunsDirMock = vi.mocked(settingsService.openRunsDir)
const setWindowEffectMock = vi.mocked(settingsService.setWindowEffect)

interface LegacyField {
  value?: string
  checked?: boolean
  type?: string
  dispatchEvent?: (event: Event) => boolean
}

interface DocumentHarness {
  dataset: Record<string, string>
  fields: Record<string, LegacyField>
}

function installDocumentHarness(): DocumentHarness {
  const dataset: Record<string, string> = {}
  const fields: Record<string, LegacyField> = {
    marketDataDir: {
      value: '',
      dispatchEvent: () => true,
    },
    marketDataMoveExisting: {
      type: 'checkbox',
      checked: true,
      dispatchEvent: () => true,
    },
  }
  vi.stubGlobal('document', {
    documentElement: { dataset },
    getElementById: (id: string) => fields[id] ?? null,
  })
  return { dataset, fields }
}

const defaultMarketDataDirInfo = {
  defaultRootDir: 'C:/default',
  configuredRootDir: '',
  effectiveRootDir: 'C:/default',
  polygonCacheDir: 'C:/default/polygon-cache',
  hint: 'Using default root: C:/default · Polygon cache: C:/default/polygon-cache',
}

describe('settings store actions', () => {
  let harness: DocumentHarness
  let pinia: ReturnType<typeof createPinia>

  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    vi.clearAllMocks()
    browseMarketDataDirMock.mockResolvedValue('')
    setMarketDataDirMock.mockResolvedValue(defaultMarketDataDirInfo)
    resetMarketDataDirMock.mockResolvedValue(defaultMarketDataDirInfo)
    openMarketDataDirMock.mockResolvedValue({ path: 'C:/default/polygon-cache', openedWith: 'open_path' })
    harness = installDocumentHarness()
  })

  it('applies settings field updates without legacy settings action dispatch', () => {
    const store = useSettingsStore()
    store.setInputField('settingsMarketDataDir', ' C:/cache ')
    store.setSelectField('settingsThemeSelect', 'dark')
    store.setSelectField('settingsEffectSelect', 'mica')
    store.setSelectField('settingsHelpMode', 'off')
    store.setSelectField('settingsDensityMode', 'dense')
    store.setCheckboxField('settingsWorkflowAdvanced', true)
    store.setCheckboxField('settingsControlsCollapsed', true)
    store.setCheckboxField('settingsMoveExisting', false)

    expect(store.marketDataDir).toBe('C:/cache')
    expect(harness.fields.marketDataDir?.value).toBe('')
    expect(store.theme).toBe('dark')
    expect(harness.dataset.themePref).toBe('dark')
    expect(store.helpMode).toBe('off')
    expect(harness.dataset.helpMode).toBe('off')
    expect(store.densityMode).toBe('dense')
    expect(harness.dataset.density).toBe('dense')
    expect(store.workflowAdvanced).toBe(true)
    expect(harness.dataset.workflowAdvanced).toBe('1')
    expect(store.controlsCollapsed).toBe(true)
    expect(harness.dataset.controlsCollapsed).toBe('1')
    expect(store.moveExisting).toBe(false)
    expect(harness.fields.marketDataMoveExisting?.checked).toBe(true)
    expect(setWindowEffectMock).toHaveBeenCalledWith('mica', true)
  })

  it('dispatches market-data directory actions via data service', () => {
    const store = useSettingsStore()
    store.browseMarketDataDir()
    store.setMarketDataDir(' C:/md ', true)
    store.resetMarketDataDir()
    store.openMarketDataDir()

    expect(browseMarketDataDirMock).toHaveBeenCalled()
    expect(setMarketDataDirMock).toHaveBeenCalledWith('C:/md', true)
    expect(resetMarketDataDirMock).toHaveBeenCalled()
    expect(openMarketDataDirMock).toHaveBeenCalled()
  })

  it('dispatches utility and navigation actions', () => {
    const store = useSettingsStore()
    const workflowStore = useWorkflowStore()

    store.openWorkflowData()
    store.openRunsDir()
    store.openReportsDir()
    store.openPolygonModal()
    expect(workflowStore.workspace).toBe('workflow')
    expect(workflowStore.workflow).toBe('data')

    store.gotoLibrary()
    expect(workflowStore.workspace).toBe('library')
    expect(workflowStore.workflow).toBe('history')

    store.gotoAggregate()
    expect(workflowStore.workspace).toBe('aggregate')
    expect(workflowStore.workflow).toBe('settings')

    expect(openPolygonMock).toHaveBeenCalled()
    expect(openRunsDirMock).toHaveBeenCalled()
    expect(openReportsDirMock).toHaveBeenCalled()
  })

  it('creates, applies, duplicates, renames, and deletes custom perspectives', () => {
    const settingsStore = useSettingsStore()
    const workflowStore = useWorkflowStore()

    workflowStore.selectWorkflowStep('analyze')
    workflowStore.selectResultsTab('top')
    workflowStore.setSplitFraction(0.41, { persist: false })
    settingsStore.setSelectField('settingsDensityMode', 'dense')

    settingsStore.createPerspective('Review focus')
    const created = settingsStore.perspectives.find((entry) => entry.name === 'Review focus')
    expect(created).toBeTruthy()
    expect(created?.snapshot.resultsTab).toBe('top')
    expect(created?.snapshot.splitFraction).toBeCloseTo(0.41, 5)
    expect(created?.snapshot.densityMode).toBe('dense')

    workflowStore.selectWorkspace('library')
    settingsStore.setSelectField('settingsDensityMode', 'comfortable')
    settingsStore.applyPerspective(String(created?.id || ''))

    expect(workflowStore.workspace).toBe('workflow')
    expect(workflowStore.resultsTab).toBe('top')
    expect(workflowStore.splitFraction).toBeCloseTo(0.41, 5)
    expect(settingsStore.densityMode).toBe('dense')

    settingsStore.duplicatePerspective(String(created?.id || ''))
    const duplicate = settingsStore.perspectives.find((entry) => entry.name === 'Review focus Copy')
    expect(duplicate).toBeTruthy()

    settingsStore.renamePerspective(String(duplicate?.id || ''), 'Review focus alt')
    expect(settingsStore.perspectives.some((entry) => entry.name === 'Review focus alt')).toBe(true)

    settingsStore.deletePerspective(String(duplicate?.id || ''))
    expect(settingsStore.perspectives.some((entry) => entry.id === duplicate?.id)).toBe(false)
  })

  it('syncs market-data fields from data store and applies runtime mutability flags', () => {
    const settingsStore = useSettingsStore()
    const dataStore = useDataStore()

    dataStore.applyLegacySnapshot({
      marketDataDir: 'C:/market-root',
      marketDataMoveExisting: false,
      marketDataDirHint: 'Using custom root: C:/market-root',
      canBrowseMarketDataDir: true,
      canSetMarketDataDir: true,
      canResetMarketDataDir: true,
      canOpenMarketDataDir: true,
    })

    harness.dataset.running = '1'
    settingsStore.syncFromDataStore()
    expect(settingsStore.marketDataDir).toBe('C:/market-root')
    expect(settingsStore.moveExisting).toBe(false)
    expect(settingsStore.marketDataDirHint).toBe('Using custom root: C:/market-root')
    expect(settingsStore.canBrowseMarketDataDir).toBe(false)
    expect(settingsStore.canOpenRunsDir).toBe(false)
    expect(settingsStore.canOpenReportsDir).toBe(false)

    harness.dataset.running = ''
    settingsStore.syncFromDataStore()
    expect(settingsStore.canBrowseMarketDataDir).toBe(true)
    expect(settingsStore.canSetMarketDataDir).toBe(true)
    expect(settingsStore.canResetMarketDataDir).toBe(true)
    expect(settingsStore.canOpenMarketDataDir).toBe(true)
    expect(settingsStore.canOpenRunsDir).toBe(true)
    expect(settingsStore.canOpenReportsDir).toBe(true)
  })
})
