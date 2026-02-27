import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invokeCommand } from '../tauri/invoke'
import { requestCancelCurrentRun } from './runControlService'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

const invokeCommandMock = vi.mocked(invokeCommand)

describe('runControlService', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('requests cancel_current_run and background cancellation when available', async () => {
    invokeCommandMock.mockResolvedValue(undefined)

    const result = await requestCancelCurrentRun()

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, 'cancel_current_run')
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, 'cancel_background_task')
    expect(result).toEqual({
      requested: true,
      usedLegacyFallback: false,
      backgroundTaskRequested: true,
    })
  })

  it('falls back to cancel_double_ma_optimization when cancel_current_run fails', async () => {
    invokeCommandMock
      .mockRejectedValueOnce(new Error('missing'))
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(undefined)

    const result = await requestCancelCurrentRun()

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, 'cancel_current_run')
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, 'cancel_double_ma_optimization')
    expect(invokeCommandMock).toHaveBeenNthCalledWith(3, 'cancel_background_task')
    expect(result).toEqual({
      requested: true,
      usedLegacyFallback: true,
      backgroundTaskRequested: true,
    })
  })

  it('throws when no cancellation command can be requested', async () => {
    invokeCommandMock
      .mockRejectedValueOnce(new Error('missing'))
      .mockRejectedValueOnce(new Error('missing'))
      .mockRejectedValueOnce(new Error('missing'))

    await expect(requestCancelCurrentRun()).rejects.toThrow('Cancellation command is unavailable.')
  })
})
