import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { bindRuntimePinia } from './runtimePinia'
import {
  crossRefresh,
  exportReportJson,
  exportReportXlsx,
  exportShortlistCsv,
  exportValidationJson,
  openReportsDir,
  runValidation,
  toggleCrossDataset,
} from '../services/validationService'
import { useDataStore } from './dataStore'
import { useValidationStore } from './validationStore'

vi.mock('../services/validationService', () => ({
  crossRefresh: vi.fn(async () => ({ datasets: [] })),
  exportReportJson: vi.fn(async () => undefined),
  exportReportXlsx: vi.fn(async () => undefined),
  exportShortlistCsv: vi.fn(async () => undefined),
  exportValidationJson: vi.fn(async () => undefined),
  openReportsDir: vi.fn(async () => undefined),
  runValidation: vi.fn(async () => undefined),
  toggleCrossDataset: vi.fn(async () => undefined),
}))

const runValidationMock = vi.mocked(runValidation)
const exportShortlistCsvMock = vi.mocked(exportShortlistCsv)
const exportValidationJsonMock = vi.mocked(exportValidationJson)
const exportReportJsonMock = vi.mocked(exportReportJson)
const exportReportXlsxMock = vi.mocked(exportReportXlsx)
const crossRefreshMock = vi.mocked(crossRefresh)
const toggleCrossDatasetMock = vi.mocked(toggleCrossDataset)
const openReportsDirMock = vi.mocked(openReportsDir)

interface LegacyTextField {
  value: string
  disabled?: boolean
  type?: string
  dispatchEvent: (event: Event) => boolean
}

interface LegacyCheckboxField {
  checked: boolean
  disabled?: boolean
  type: 'checkbox'
  dispatchEvent: (event: Event) => boolean
}

interface DocumentHarness {
  fields: Record<string, LegacyTextField | LegacyCheckboxField>
  eventsById: Record<string, string[]>
}

function installDocumentHarness(): DocumentHarness {
  const eventsById: Record<string, string[]> = {}
  const trackEvent = (id: string, event: Event): boolean => {
    if (!eventsById[id]) eventsById[id] = []
    eventsById[id].push(String(event.type || '').trim())
    return true
  }
  const fields: Record<string, LegacyTextField | LegacyCheckboxField> = {
    valCandidates: {
      value: '32',
      dispatchEvent: (event: Event) => trackEvent('valCandidates', event),
    },
    valAutoRun: {
      checked: false,
      type: 'checkbox',
      dispatchEvent: (event: Event) => trackEvent('valAutoRun', event),
    },
  }
  vi.stubGlobal('document', {
    getElementById: (id: string) => fields[id] ?? null,
  })
  return { fields, eventsById }
}

describe('validation store actions', () => {
  let harness: DocumentHarness
  let pinia: ReturnType<typeof createPinia>

  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    harness = installDocumentHarness()
    runValidationMock.mockClear()
    runValidationMock.mockResolvedValue(undefined)
    exportShortlistCsvMock.mockClear()
    exportShortlistCsvMock.mockResolvedValue(undefined)
    exportValidationJsonMock.mockClear()
    exportValidationJsonMock.mockResolvedValue(undefined)
    exportReportJsonMock.mockClear()
    exportReportJsonMock.mockResolvedValue(undefined)
    exportReportXlsxMock.mockClear()
    exportReportXlsxMock.mockResolvedValue(undefined)
    crossRefreshMock.mockClear()
    crossRefreshMock.mockResolvedValue({ datasets: [] })
    toggleCrossDatasetMock.mockClear()
    toggleCrossDatasetMock.mockResolvedValue(undefined)
    openReportsDirMock.mockClear()
    openReportsDirMock.mockResolvedValue(undefined)
  })

  async function flushServiceImports(): Promise<void> {
    await vi.dynamicImportSettled()
    await Promise.resolve()
    await Promise.resolve()
  }

  it('updates Vue validation state without requiring legacy DOM mirroring', () => {
    const store = useValidationStore()
    store.setInputField('valCandidates', '64')
    store.setInputField('valAdmissionPolicy', 'scalar_union')
    store.setCheckboxField('valAutoRun', true)

    const autoRun = harness.fields.valAutoRun as LegacyCheckboxField
    expect((harness.fields.valCandidates as LegacyTextField).value).toBe('32')
    expect(autoRun.checked).toBe(false)
    expect(harness.eventsById.valCandidates).toBeUndefined()
    expect(harness.eventsById.valAutoRun).toBeUndefined()
    expect(store.candidates).toBe('64')
    expect(store.admissionPolicy).toBe('scalar_union')
    expect(store.autoRun).toBe(true)
    expect(store.source).toBe('vue')
    expect(store.updatedAtUnixMs).toBeGreaterThan(0)
  })

  it('starts with stricter validation defaults enabled', () => {
    const store = useValidationStore()

    expect(store.minPsr).toBe('0.80')
    expect(store.minDsr).toBe('0.60')
    expect(store.minCscvRank).toBe('0.55')
    expect(store.maxPbo).toBe('0.20')
    expect(store.wfEnable).toBe(true)
    expect(store.wfWindow).toBe('20000')
    expect(store.wfStep).toBe('5000')
    expect(store.cscvEnable).toBe(true)
    expect(store.cscvSlices).toBe('16')
    expect(store.spaEnable).toBe(false)
    expect(store.reoptWfEnable).toBe(false)
  })

  it('dispatches validate and export actions', async () => {
    const store = useValidationStore()
    store.runValidation()
    store.exportShortlistCsv()
    store.exportValidationJson()
    store.exportReportJson()
    store.exportReportXlsx()
    store.openReportsDir()
    await flushServiceImports()

    expect(runValidationMock).toHaveBeenCalled()
    expect(exportShortlistCsvMock).toHaveBeenCalled()
    expect(exportValidationJsonMock).toHaveBeenCalled()
    expect(exportReportJsonMock).toHaveBeenCalled()
    expect(exportReportXlsxMock).toHaveBeenCalled()
    expect(openReportsDirMock).toHaveBeenCalled()
  })

  it('keeps validation service rejections inside store state', async () => {
    const store = useValidationStore()
    runValidationMock.mockRejectedValueOnce(new Error('dataset mismatch'))

    store.runValidation()
    await flushServiceImports()

    expect(runValidationMock).toHaveBeenCalled()
    expect(store.statusLine).toBe('Validation failed: Error: dataset mismatch')
    expect(store.validationHint).toBe('Validation failed: Error: dataset mismatch')
    expect(store.source).toBe('vue')
  })

  it('dispatches cross-dataset actions', async () => {
    const store = useValidationStore()
    store.crossSearch = 'stocks'
    store.crossDatasets = [
      { path: 'C:/data/aapl.csv', label: 'AAPL', meta: 'stocks (adj) • tf=1m', checked: false, disabled: false },
      { path: 'C:/data/msft.csv', label: 'MSFT', meta: 'stocks (adj) • tf=1m', checked: true, disabled: false },
      { path: 'C:/data/btc.csv', label: 'BTCUSD', meta: 'crypto • tf=1m', checked: false, disabled: false },
    ]
    store.crossSelectAll()
    store.crossSelectNone()
    store.toggleCrossDataset(' C:/data/aapl.csv ', true)
    store.toggleCrossDataset('', false)
    await flushServiceImports()

    expect(toggleCrossDatasetMock).toHaveBeenNthCalledWith(1, 'C:/data/aapl.csv', true)
    expect(toggleCrossDatasetMock).toHaveBeenNthCalledWith(2, 'C:/data/aapl.csv', false)
    expect(toggleCrossDatasetMock).toHaveBeenNthCalledWith(3, 'C:/data/msft.csv', false)
    expect(toggleCrossDatasetMock).toHaveBeenNthCalledWith(4, 'C:/data/aapl.csv', true)
  })

  it('hydrates cross datasets from refresh and clears stale legacy selections', async () => {
    const store = useValidationStore()
    store.crossSearch = 'stocks'
    store.crossDatasets = [
      { path: 'C:/data/aapl.csv', label: 'AAPL', meta: 'stocks (adj) • tf=1m', checked: true, disabled: false },
      { path: 'C:/data/stale.csv', label: 'STALE', meta: 'stocks (adj) • tf=1m', checked: true, disabled: false },
    ]
    crossRefreshMock.mockResolvedValue({
      datasets: [
        {
          path: 'C:/data/aapl.csv',
          label: 'AAPL',
          meta: 'stocks (adj) • tf=1m • 2025-01-01 → 2025-01-31 • rows=10,000 • updated=2025-01-31',
          market: 'stocks',
          timeframe: '1m',
          adjusted: true,
        },
        {
          path: 'C:/data/msft.csv',
          label: 'MSFT',
          meta: 'stocks (adj) • tf=1m • 2025-01-01 → 2025-01-31 • rows=9,500 • updated=2025-01-31',
          market: 'stocks',
          timeframe: '1m',
          adjusted: true,
        },
      ],
    })

    store.crossRefresh()
    await flushServiceImports()

    expect(crossRefreshMock).toHaveBeenCalled()
    expect(store.crossDatasets).toEqual([
      {
        path: 'C:/data/aapl.csv',
        label: 'AAPL',
        meta: 'stocks (adj) • tf=1m • 2025-01-01 → 2025-01-31 • rows=10,000 • updated=2025-01-31',
        checked: true,
        disabled: false,
        market: 'stocks',
        timeframe: '1m',
        adjusted: true,
      },
      {
        path: 'C:/data/msft.csv',
        label: 'MSFT',
        meta: 'stocks (adj) • tf=1m • 2025-01-01 → 2025-01-31 • rows=9,500 • updated=2025-01-31',
        checked: false,
        disabled: false,
        market: 'stocks',
        timeframe: '1m',
        adjusted: true,
      },
    ])
    expect(store.crossSelectedCount).toBe(1)
    expect(store.crossTotalCount).toBe(2)
    expect(store.crossMeta).toContain('selected 1/2')
    expect(store.statusLine).toBe('Refreshed cross-ticker list.')
    expect(toggleCrossDatasetMock).toHaveBeenCalledWith('C:/data/stale.csv', false)
  })

  it('selects matching cross datasets in Vue path when current dataset context is known', async () => {
    const dataStore = useDataStore()
    dataStore.csvPath = 'C:/data/aapl.csv'
    dataStore.dataTimeframe = '1m'
    dataStore.datasetContext = 'source=polygon • market=stocks (adj) • symbol=AAPL • file=aapl.csv'

    const store = useValidationStore()
    store.crossDatasets = [
      {
        path: 'C:/data/aapl.csv',
        label: 'AAPL',
        meta: 'stocks (adj) • tf=1m',
        checked: true,
        disabled: false,
        market: 'stocks',
        timeframe: '1m',
        adjusted: true,
      },
      {
        path: 'C:/data/msft.csv',
        label: 'MSFT',
        meta: 'stocks (adj) • tf=1m',
        checked: false,
        disabled: false,
        market: 'stocks',
        timeframe: '1m',
        adjusted: true,
      },
      {
        path: 'C:/data/spy.csv',
        label: 'SPY',
        meta: 'stocks (raw) • tf=1m',
        checked: true,
        disabled: false,
        market: 'stocks',
        timeframe: '1m',
        adjusted: false,
      },
      {
        path: 'C:/data/btc.csv',
        label: 'BTCUSD',
        meta: 'crypto • tf=1m',
        checked: true,
        disabled: false,
        market: 'crypto',
        timeframe: '1m',
        adjusted: null,
      },
    ]

    store.crossSelectMatch()
    await flushServiceImports()

    expect(store.crossDatasets.map((entry) => [entry.path, entry.checked])).toEqual([
      ['C:/data/aapl.csv', false],
      ['C:/data/msft.csv', true],
      ['C:/data/spy.csv', false],
      ['C:/data/btc.csv', false],
    ])
    expect(store.crossSelectedCount).toBe(1)
    expect(store.crossTotalCount).toBe(4)
    expect(store.statusLine).toBe('Cross-ticker: selected datasets matching current dataset.')
    expect(toggleCrossDatasetMock).toHaveBeenNthCalledWith(1, 'C:/data/aapl.csv', false)
    expect(toggleCrossDatasetMock).toHaveBeenNthCalledWith(2, 'C:/data/msft.csv', true)
    expect(toggleCrossDatasetMock).toHaveBeenNthCalledWith(3, 'C:/data/spy.csv', false)
    expect(toggleCrossDatasetMock).toHaveBeenNthCalledWith(4, 'C:/data/btc.csv', false)
  })

  it('skips match-current when dataset context is unavailable', async () => {
    const store = useValidationStore()
    store.crossDatasets = [
      {
        path: 'C:/data/msft.csv',
        label: 'MSFT',
        meta: 'stocks (adj) • tf=1m',
        checked: false,
        disabled: false,
        market: 'stocks',
        timeframe: '1m',
        adjusted: true,
      },
    ]

    store.crossSelectMatch()
    await flushServiceImports()

    expect(store.statusLine).toBe('Cross-ticker: unable to match current dataset (load data + timeframe first).')
    expect(toggleCrossDatasetMock).not.toHaveBeenCalled()
  })

  it('recomputes cross meta immediately when Vue cross search input changes', () => {
    const store = useValidationStore()
    store.crossDatasets = [
      { path: 'C:/data/aapl.csv', label: 'AAPL', meta: 'stocks (adj) • tf=1m', checked: true, disabled: false },
      { path: 'C:/data/msft.csv', label: 'MSFT', meta: 'stocks (adj) • tf=1m', checked: false, disabled: false },
      { path: 'C:/data/btc.csv', label: 'BTCUSD', meta: 'crypto • tf=1m', checked: true, disabled: false },
    ]

    store.setInputField('valCrossSearch', 'stocks')

    expect(store.crossSearch).toBe('stocks')
    expect(store.crossMeta).toContain('selected 2/3 (showing 2)')
    expect(store.crossMeta).toContain('filter=stocks')
    expect(store.source).toBe('vue')
  })
})
