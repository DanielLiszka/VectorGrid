import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  cancel,
  type DistributedWorkerStatusSummary,
  fillFromPolygon,
  generateToken,
  refreshWorkerStatus,
  runAssets,
  runTiles,
  startWorker,
  stopWorker,
  useCurrentCsv,
} from '../services/distributedService'
import { useDataStore } from './dataStore'
import { useDistributedStore } from './distributedStore'

vi.mock('../services/distributedService', () => ({
  cancel: vi.fn(async () => undefined),
  fillFromPolygon: vi.fn(async () => ({ paths: ['C:/polygon/a.csv', 'C:/polygon/b.csv'], count: 2 })),
  generateToken: vi.fn(async () => 'abcdef123456abcdef123456abcdef123456abcdef123456'),
  refreshWorkerStatus: vi.fn(async () => ({ running: true, bindAddr: '127.0.0.1:48991', text: 'Worker: running on 127.0.0.1:48991' })),
  runAssets: vi.fn(async () => ({ outputPreview: 'assets-output', statusLine: 'Distributed assets: done (2 ok).', cancelled: false })),
  runTiles: vi.fn(async () => ({ outputPreview: 'tiles-output', statusLine: 'Distributed tiles: done.', cancelled: false })),
  startWorker: vi.fn(async () => ({ running: true, bindAddr: '127.0.0.1:48991', text: 'Worker: running on 127.0.0.1:48991' })),
  stopWorker: vi.fn(async () => ({ running: false, bindAddr: '', text: 'Worker: stopped' })),
  useCurrentCsv: vi.fn(async () => ({ path: 'C:/data/current.csv' })),
}))

const generateTokenMock = vi.mocked(generateToken)
const startWorkerMock = vi.mocked(startWorker)
const stopWorkerMock = vi.mocked(stopWorker)
const refreshWorkerStatusMock = vi.mocked(refreshWorkerStatus)
const fillFromPolygonMock = vi.mocked(fillFromPolygon)
const useCurrentCsvMock = vi.mocked(useCurrentCsv)
const runAssetsMock = vi.mocked(runAssets)
const runTilesMock = vi.mocked(runTiles)
const cancelMock = vi.mocked(cancel)
const defaultWorkerStatus: DistributedWorkerStatusSummary = {
  running: true,
  bindAddr: '127.0.0.1:48991',
  text: 'Worker: running on 127.0.0.1:48991',
}

describe('distributed store actions', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    generateTokenMock.mockClear()
    generateTokenMock.mockResolvedValue('abcdef123456abcdef123456abcdef123456abcdef123456')
    startWorkerMock.mockClear()
    startWorkerMock.mockResolvedValue(defaultWorkerStatus)
    stopWorkerMock.mockClear()
    stopWorkerMock.mockResolvedValue({ running: false, bindAddr: '', text: 'Worker: stopped' })
    refreshWorkerStatusMock.mockClear()
    refreshWorkerStatusMock.mockResolvedValue(defaultWorkerStatus)
    fillFromPolygonMock.mockClear()
    fillFromPolygonMock.mockResolvedValue({ paths: ['C:/polygon/a.csv', 'C:/polygon/b.csv'], count: 2 })
    useCurrentCsvMock.mockClear()
    useCurrentCsvMock.mockResolvedValue({ path: 'C:/data/current.csv' })
    runAssetsMock.mockClear()
    runAssetsMock.mockResolvedValue({ outputPreview: 'assets-output', statusLine: 'Distributed assets: done (2 ok).', cancelled: false })
    runTilesMock.mockClear()
    runTilesMock.mockResolvedValue({ outputPreview: 'tiles-output', statusLine: 'Distributed tiles: done.', cancelled: false })
    cancelMock.mockClear()
    cancelMock.mockResolvedValue(undefined)
  })

  it('updates Vue distributed state without legacy DOM mirroring', () => {
    const store = useDistributedStore()
    store.setInputField('distWorkerBindAddr', '127.0.0.1:48991')
    store.setSelectField('distLeaderMode', 'tiles')

    expect(store.workerBindAddr).toBe('127.0.0.1:48991')
    expect(store.leaderMode).toBe('tiles')
    expect(store.source).toBe('vue')
  })

  it('dispatches distributed control actions through distributed service', async () => {
    const store = useDistributedStore()
    const dataStore = useDataStore()
    dataStore.csvPath = ' C:/data/current.csv '
    store.setInputField('distLeaderWorkers', 'http://w1:48991')
    store.setInputField('distLeaderToken', 'leader-token')
    store.setInputField('distAssetsCsvs', 'C:/data/a.csv')
    store.setInputField('distTileCsvPath', 'C:/data/tile.csv')
    store.setInputField('distTilesPerWorker', '5')
    store.setInputField('distPathFrom', 'C:/data')
    store.setInputField('distPathTo', '\\\\server\\share')
    store.generateToken()
    store.startWorker()
    store.stopWorker()
    store.refreshWorkerStatus()
    store.fillFromPolygon()
    store.useCurrentCsv()
    store.runAssets()
    store.runTiles()
    store.cancel()
    await Promise.resolve()
    await Promise.resolve()
    await Promise.resolve()
    await Promise.resolve()

    expect(generateTokenMock).toHaveBeenCalled()
    expect(startWorkerMock).toHaveBeenCalledWith('', '')
    expect(stopWorkerMock).toHaveBeenCalled()
    expect(refreshWorkerStatusMock).toHaveBeenCalled()
    expect(fillFromPolygonMock).toHaveBeenCalled()
    expect(useCurrentCsvMock).toHaveBeenCalledWith(' C:/data/current.csv ')
    expect(runAssetsMock).toHaveBeenCalledWith({
      workerUrlsText: 'http://w1:48991',
      authToken: 'leader-token',
      assetsCsvText: 'C:/data/a.csv',
      rewriteFrom: 'C:/data',
      rewriteTo: '\\\\server\\share',
    })
    expect(runTilesMock).toHaveBeenCalledWith({
      workerUrlsText: 'http://w1:48991',
      authToken: 'leader-token',
      tileCsvPath: 'C:/data/tile.csv',
      rewriteFrom: 'C:/data',
      rewriteTo: '\\\\server\\share',
      tilesPerWorker: '5',
    })
    expect(cancelMock).toHaveBeenCalled()
    expect(store.assetPathCount).toBe(2)
    expect(store.assetsCsvText).toBe('C:/polygon/a.csv\nC:/polygon/b.csv')
    expect(store.tileCsvPath).toBe('C:/data/current.csv')
    expect(store.outputPreview).toBe('tiles-output')
  })
})
