import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  applySavedRun,
  applySession,
  clearSession,
  deleteSavedRun,
  exportPreset,
  importPreset,
  loadSavedRun,
  loadSavedRunDetail,
  openRunsDir,
  openSavedRunReport,
  readSessionHistory,
  refreshSavedRuns,
  rerunSavedRun,
  revealSavedRun,
  saveSavedRunTags,
} from '../services/historyService'
import { useHistoryStore } from './historyStore'

vi.mock('../services/historyService', () => ({
  applySavedRun: vi.fn(async () => undefined),
  applySession: vi.fn(async () => undefined),
  clearSession: vi.fn(async () => undefined),
  deleteSavedRun: vi.fn(async () => undefined),
  exportPreset: vi.fn(async () => undefined),
  importPreset: vi.fn(async () => undefined),
  loadSavedRun: vi.fn(async () => undefined),
  loadSavedRunDetail: vi.fn(async () => ({
    strategyKind: 'DoubleMA',
    timeframe: '1m',
    csvPath: 'C:/data/aapl.csv',
    reportPath: 'C:/runs/aapl.xlsx',
    hasValidation: true,
    tags: ['aapl'],
    paramsText: 'strategy: DoubleMA',
  })),
  openRunsDir: vi.fn(async () => undefined),
  openSavedRunReport: vi.fn(async () => undefined),
  readSessionHistory: vi.fn(() => []),
  refreshSavedRuns: vi.fn(async () => ({ runs: [] })),
  rerunSavedRun: vi.fn(async () => undefined),
  revealSavedRun: vi.fn(async () => undefined),
  saveSavedRunTags: vi.fn(async () => undefined),
}))

const applySessionMock = vi.mocked(applySession)
const exportPresetMock = vi.mocked(exportPreset)
const importPresetMock = vi.mocked(importPreset)
const clearSessionMock = vi.mocked(clearSession)
const saveSavedRunTagsMock = vi.mocked(saveSavedRunTags)
const refreshSavedRunsMock = vi.mocked(refreshSavedRuns)
const loadSavedRunMock = vi.mocked(loadSavedRun)
const loadSavedRunDetailMock = vi.mocked(loadSavedRunDetail)
const applySavedRunMock = vi.mocked(applySavedRun)
const rerunSavedRunMock = vi.mocked(rerunSavedRun)
const openSavedRunReportMock = vi.mocked(openSavedRunReport)
const revealSavedRunMock = vi.mocked(revealSavedRun)
const deleteSavedRunMock = vi.mocked(deleteSavedRun)
const openRunsDirMock = vi.mocked(openRunsDir)
const readSessionHistoryMock = vi.mocked(readSessionHistory)

describe('history store actions', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    applySessionMock.mockClear()
    applySessionMock.mockResolvedValue(undefined)
    exportPresetMock.mockClear()
    exportPresetMock.mockResolvedValue(undefined)
    importPresetMock.mockClear()
    importPresetMock.mockResolvedValue(undefined)
    clearSessionMock.mockClear()
    clearSessionMock.mockResolvedValue(undefined)
    saveSavedRunTagsMock.mockClear()
    saveSavedRunTagsMock.mockResolvedValue(undefined)
    refreshSavedRunsMock.mockClear()
    refreshSavedRunsMock.mockResolvedValue({ runs: [] })
    loadSavedRunMock.mockClear()
    loadSavedRunMock.mockResolvedValue(undefined)
    loadSavedRunDetailMock.mockClear()
    loadSavedRunDetailMock.mockResolvedValue({
      strategyKind: 'DoubleMA',
      timeframe: '1m',
      csvPath: 'C:/data/aapl.csv',
      reportPath: 'C:/runs/aapl.xlsx',
      hasValidation: true,
      tags: ['aapl'],
      paramsText: 'strategy: DoubleMA',
    })
    applySavedRunMock.mockClear()
    applySavedRunMock.mockResolvedValue(undefined)
    rerunSavedRunMock.mockClear()
    rerunSavedRunMock.mockResolvedValue(undefined)
    openSavedRunReportMock.mockClear()
    openSavedRunReportMock.mockResolvedValue(undefined)
    revealSavedRunMock.mockClear()
    revealSavedRunMock.mockResolvedValue(undefined)
    deleteSavedRunMock.mockClear()
    deleteSavedRunMock.mockResolvedValue(undefined)
    openRunsDirMock.mockClear()
    openRunsDirMock.mockResolvedValue(undefined)
    readSessionHistoryMock.mockClear()
    readSessionHistoryMock.mockReturnValue([])
  })

  it('updates Vue history state without legacy DOM mirroring', () => {
    const store = useHistoryStore()
    store.setInputField('savedRunsSearch', 'aapl')
    store.setSelectField('historySelect', 'run-1')

    expect(store.savedRunsSearch).toBe('aapl')
    expect(store.sessionSelectedValue).toBe('run-1')
    expect(store.source).toBe('vue')
  })

  it('dispatches history session/actions and runs-directory action through history service', () => {
    const store = useHistoryStore()
    readSessionHistoryMock.mockReturnValue([
      { ts: '2026-02-25T00:00:00Z', backend: 'CpuOnly', pairs: 123 },
    ])
    store.refreshSessionHistory('2026-02-25T00:00:00Z')
    store.applySession()
    store.exportPreset()
    store.importPreset()
    store.clearSession()
    store.refreshSavedRuns()
    store.openRunsDir()

    expect(applySessionMock).toHaveBeenCalledWith('2026-02-25T00:00:00Z')
    expect(exportPresetMock).toHaveBeenCalled()
    expect(importPresetMock).toHaveBeenCalled()
    expect(clearSessionMock).toHaveBeenCalled()
    expect(refreshSavedRunsMock).toHaveBeenCalled()
    expect(openRunsDirMock).toHaveBeenCalled()
  })

  it('dispatches saved-run actions through history service', async () => {
    const store = useHistoryStore()
    store.savedRunsRaw = [
      {
        ts: '2026-02-25T00:00:00Z',
        path: 'C:/runs/run1.json',
        bytes: 1024,
        strategyKind: 'DoubleMA',
        hasValidation: true,
        reportXlsxPath: 'C:/runs/run1.xlsx',
        csvPath: 'C:/data/aapl.csv',
        tags: ['holdout'],
      },
    ]
    store.syncSavedRunsView('C:/runs/run1.json')
    await Promise.resolve()
    await Promise.resolve()
    store.saveSavedRunTags(' C:/runs/run1.json ', 'holdout,1m')
    store.loadSavedRun(' C:/runs/run1.json ')
    store.applySavedRun(' C:/runs/run1.json ')
    store.rerunSavedRun(' C:/runs/run1.json ')
    store.openSavedRunReport(' C:/runs/run1.json ')
    store.revealSavedRun(' C:/runs/run1.json ')
    store.deleteSavedRun(' C:/runs/run1.json ')
    await Promise.resolve()
    await Promise.resolve()
    await Promise.resolve()

    expect(saveSavedRunTagsMock).toHaveBeenCalledWith('C:/runs/run1.json', 'holdout,1m')
    expect(loadSavedRunDetailMock).toHaveBeenCalledWith('C:/runs/run1.json')
    expect(loadSavedRunMock).toHaveBeenCalledWith('C:/runs/run1.json')
    expect(applySavedRunMock).toHaveBeenCalledWith('C:/runs/run1.json')
    expect(rerunSavedRunMock).toHaveBeenCalledWith('C:/runs/run1.json')
    expect(openSavedRunReportMock).toHaveBeenCalledWith('C:/runs/run1.json')
    expect(revealSavedRunMock).toHaveBeenCalledWith('C:/runs/run1.json')
    expect(deleteSavedRunMock).toHaveBeenCalledWith('C:/runs/run1.json')
    expect(refreshSavedRunsMock).toHaveBeenCalledTimes(2)
  })

  it('hydrates and filters saved runs via Vue-owned refresh/search flow', async () => {
    const store = useHistoryStore()
    refreshSavedRunsMock.mockResolvedValue({
      runs: [
        {
          ts: '2026-02-25T00:00:00Z',
          path: 'C:/runs/aapl.json',
          bytes: 2048,
          strategyKind: 'DoubleMA',
          hasValidation: true,
          reportXlsxPath: 'C:/runs/aapl.xlsx',
          csvPath: 'C:/data/aapl.csv',
          tags: ['aapl', '1m'],
        },
        {
          ts: '2026-02-24T00:00:00Z',
          path: 'C:/runs/msft.json',
          bytes: 1024,
          strategyKind: 'CompositeCpu',
          hasValidation: false,
          reportXlsxPath: '',
          csvPath: 'C:/data/msft.csv',
          tags: ['msft'],
        },
      ],
    })

    store.setSavedRunsSearch('aapl')
    store.refreshSavedRuns()
    await Promise.resolve()
    await Promise.resolve()
    await Promise.resolve()

    expect(refreshSavedRunsMock).toHaveBeenCalledTimes(1)
    expect(store.savedRunCount).toBe(2)
    expect(store.savedOptions.length).toBe(2)
    expect(store.savedOptions[0]?.label).toContain('Select a saved run')
    expect(store.savedOptions[1]?.value).toBe('C:/runs/aapl.json')
    expect(store.savedRunsMeta).toBe('saved runs: 1/2')
    expect(store.statusLine).toBe('Refreshed saved runs.')
  })

  it('uses fuzzy ranking for saved runs while keeping short-query filtering stable', () => {
    const store = useHistoryStore()
    store.savedRunsRaw = [
      {
        ts: '2026-02-25T00:00:00Z',
        path: 'C:/runs/aapl.json',
        bytes: 2048,
        strategyKind: 'DoubleMA',
        hasValidation: true,
        reportXlsxPath: 'C:/runs/aapl.xlsx',
        csvPath: 'C:/data/aapl.csv',
        tags: ['aapl', '1m'],
      },
      {
        ts: '2026-02-24T00:00:00Z',
        path: 'C:/runs/msft.json',
        bytes: 1024,
        strategyKind: 'CompositeCpu',
        hasValidation: false,
        reportXlsxPath: '',
        csvPath: 'C:/data/msft.csv',
        tags: ['msft', 'swing'],
      },
    ]

    store.setSavedRunsSearch('appl')
    expect(store.savedRunsVisible.map((entry) => entry.path)).toEqual(['C:/runs/aapl.json'])

    store.setSavedRunsSearch('ms')
    expect(store.savedRunsVisible.map((entry) => entry.path)).toEqual(['C:/runs/msft.json'])

    store.setSavedRunsSearch('aapl 1m')
    expect(store.savedRunsVisible.map((entry) => entry.path)).toEqual(['C:/runs/aapl.json'])
  })
})
