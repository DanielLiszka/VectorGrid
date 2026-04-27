import { beforeEach, describe, expect, it, vi } from 'vitest'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { tauriRuntimeAvailable } from '../tauri/invoke'
import {
  checkForDesktopUpdate,
  downloadAndInstallDesktopUpdate,
  relaunchAfterDesktopUpdate,
  resetDesktopUpdaterForTests,
} from './desktopUpdater'

vi.mock('@tauri-apps/plugin-updater', () => ({
  check: vi.fn(async () => null),
}))

vi.mock('@tauri-apps/plugin-process', () => ({
  relaunch: vi.fn(async () => undefined),
}))

vi.mock('../tauri/invoke', () => ({
  tauriRuntimeAvailable: vi.fn(() => true),
}))

const checkMock = vi.mocked(check)
const relaunchMock = vi.mocked(relaunch)
const tauriRuntimeAvailableMock = vi.mocked(tauriRuntimeAvailable)

function makeUpdate(): NonNullable<Awaited<ReturnType<typeof check>>> {
  return {
    version: '0.2.0',
    currentVersion: '0.1.0',
    date: '2026-04-27',
    body: 'desktop polish',
    downloadAndInstall: vi.fn(async () => undefined),
  } as unknown as NonNullable<Awaited<ReturnType<typeof check>>>
}

describe('desktopUpdater', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetDesktopUpdaterForTests()
    tauriRuntimeAvailableMock.mockReturnValue(true)
    checkMock.mockResolvedValue(null)
  })

  it('reports unavailable outside Tauri runtime', async () => {
    tauriRuntimeAvailableMock.mockReturnValue(false)

    await expect(checkForDesktopUpdate()).resolves.toEqual({
      status: 'unavailable',
      update: null,
    })
    expect(checkMock).not.toHaveBeenCalled()
  })

  it('normalizes available update metadata', async () => {
    checkMock.mockResolvedValueOnce(makeUpdate())

    await expect(checkForDesktopUpdate()).resolves.toEqual({
      status: 'update-available',
      update: {
        available: true,
        version: '0.2.0',
        currentVersion: '0.1.0',
        date: '2026-04-27',
        body: 'desktop polish',
      },
    })
  })

  it('downloads and installs the pending update', async () => {
    const update = makeUpdate()
    checkMock.mockResolvedValueOnce(update)
    await checkForDesktopUpdate()

    const result = await downloadAndInstallDesktopUpdate()

    expect(update.downloadAndInstall).toHaveBeenCalled()
    expect(result.status).toBe('update-available')
    expect(result.update?.version).toBe('0.2.0')
  })

  it('wraps check errors as status values', async () => {
    checkMock.mockRejectedValueOnce(new Error('missing endpoint'))

    const result = await checkForDesktopUpdate()

    expect(result.status).toBe('error')
    expect(result.error).toContain('missing endpoint')
  })

  it('relaunches through the process plugin only in Tauri runtime', async () => {
    await expect(relaunchAfterDesktopUpdate()).resolves.toBe(true)
    expect(relaunchMock).toHaveBeenCalledTimes(1)

    tauriRuntimeAvailableMock.mockReturnValue(false)
    await expect(relaunchAfterDesktopUpdate()).resolves.toBe(false)
    expect(relaunchMock).toHaveBeenCalledTimes(1)
  })
})
