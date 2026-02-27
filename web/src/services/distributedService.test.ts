import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  cancel,
  fillFromPolygon,
  generateToken,
  refreshWorkerStatus,
  runAssets,
  runTiles,
  startWorker,
  stopWorker,
  useCurrentCsv,
} from './distributedService'
import { requestCancelCurrentRun } from './runControlService'
import { invokeCommand } from '../tauri/invoke'
import { buildDoubleMaRequestFromVueStores } from './optimizeService'

vi.mock('./runControlService', () => ({
  requestCancelCurrentRun: vi.fn(async () => ({
    requested: true,
    usedLegacyFallback: false,
    backgroundTaskRequested: true,
  })),
}))

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => ({ running: true, bind_addr: '127.0.0.1:48991' })),
}))

vi.mock('./optimizeService', () => ({
  buildDoubleMaRequestFromVueStores: vi.fn(async () => ({
    strategy_kind: 'DoubleMa',
    objective: 'Sharpe',
    data_id: 'data-1',
  })),
}))

const requestCancelCurrentRunMock = vi.mocked(requestCancelCurrentRun)
const invokeCommandMock = vi.mocked(invokeCommand)
const buildDoubleMaRequestFromVueStoresMock = vi.mocked(buildDoubleMaRequestFromVueStores)

describe('distributedService', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    buildDoubleMaRequestFromVueStoresMock.mockResolvedValue({
      strategy_kind: 'DoubleMa',
      objective: 'Sharpe',
      data_id: 'data-1',
    })
  })

  it('dispatches direct polygon fill + current csv normalization and distributed run commands', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'polygon_get_cache_info') {
        return {
          datasets: [{ csv_path: 'C:/data/a.csv' }, { csv_path: ' C:/data/b.csv ' }, { csv_path: '' }],
        }
      }
      if (command === 'distributed_run_double_ma_assets') {
        return {
          cancelled: false,
          total: 2,
          completed: 2,
          successes: 2,
          failures: 0,
          results: [
            {
              csv_path: 'C:/data/a.csv',
              result: {
                best_params: { fast_ma_type: 'sma', fast_len: 5, slow_ma_type: 'alma', slow_len: 20 },
                best_metrics: { sharpe: 1.2, pnl: 0.3, max_dd: 0.1, trades: 20 },
              },
            },
          ],
        }
      }
      if (command === 'distributed_run_double_ma_tiled') {
        return {
          cancelled: false,
          tiles_total: 8,
          tiles_completed: 8,
          errors: [],
          result: {
            best_params: { fast_ma_type: 'sma', fast_len: 5, slow_ma_type: 'alma', slow_len: 20 },
            best_metrics: { sharpe: 1.1, pnl: 0.25, max_dd: 0.11, trades: 18 },
            top: [],
          },
        }
      }
      return { running: true, bind_addr: '127.0.0.1:48991' }
    })

    const fill = await fillFromPolygon()
    const current = await useCurrentCsv(' C:/data/current.csv ')
    const assets = await runAssets({
      workerUrlsText: 'http://w1:48991\nhttp://w2:48991',
      authToken: 'abc123',
      assetsCsvText: 'C:/data/a.csv\nC:/data/b.csv',
      rewriteFrom: 'C:/data',
      rewriteTo: '\\\\server\\share',
    })
    const tiles = await runTiles({
      workerUrlsText: 'http://w1:48991\nhttp://w2:48991',
      authToken: 'abc123',
      tileCsvPath: 'C:/data/tile.csv',
      rewriteFrom: 'C:/data',
      rewriteTo: '\\\\server\\share',
      tilesPerWorker: '6',
    })

    expect(invokeCommandMock).toHaveBeenCalledWith('polygon_get_cache_info')
    expect(fill).toEqual({
      paths: ['C:/data/a.csv', 'C:/data/b.csv'],
      count: 2,
    })
    expect(current).toEqual({ path: 'C:/data/current.csv' })
    expect(buildDoubleMaRequestFromVueStoresMock).toHaveBeenCalledTimes(2)
    expect(invokeCommandMock).toHaveBeenCalledWith('distributed_run_double_ma_assets', {
      dist_req: expect.objectContaining({
        auth_token: 'abc123',
        worker_urls: ['http://w1:48991', 'http://w2:48991'],
        csv_paths: ['\\\\server\\share/a.csv', '\\\\server\\share/b.csv'],
      }),
    })
    expect(invokeCommandMock).toHaveBeenCalledWith('distributed_run_double_ma_tiled', {
      dist_req: expect.objectContaining({
        auth_token: 'abc123',
        worker_urls: ['http://w1:48991', 'http://w2:48991'],
        csv_path: '\\\\server\\share/tile.csv',
        tiles_per_worker: 6,
      }),
    })
    expect(assets.statusLine).toContain('Distributed assets: done')
    expect(assets.outputPreview).toContain('top (by objective)')
    expect(tiles.statusLine).toBe('Distributed tiles: done.')
    expect(tiles.outputPreview).toContain('Tiled run complete')
  })

  it('normalizes token and worker status command paths', async () => {
    const token = await generateToken()
    const startStatus = await startWorker('127.0.0.1:48991', 'abc123')
    const status = await refreshWorkerStatus()
    const stopStatus = await stopWorker()

    expect(token).toHaveLength(48)
    expect(invokeCommandMock).toHaveBeenCalledWith('distributed_start_worker_server', {
      cfg: { bind_addr: '127.0.0.1:48991', auth_token: 'abc123' },
    })
    expect(invokeCommandMock).toHaveBeenCalledWith('distributed_worker_server_status')
    expect(invokeCommandMock).toHaveBeenCalledWith('distributed_stop_worker_server')
    expect(startStatus.text).toContain('Worker: running on')
    expect(status.text).toContain('Worker: running on')
    expect(stopStatus.text).toContain('Worker: running on')
  })

  it('requests cancellation via runControlService', async () => {
    await cancel()
    expect(requestCancelCurrentRunMock).toHaveBeenCalled()
  })
})
