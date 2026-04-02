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

    const commands = invokeCommandMock.mock.calls.map(([command]) => command)
    expect(commands).toContain('cancel_current_run')
    expect(commands).toContain('cancel_background_task')
    expect(result).toEqual({
      requested: true,
      usedLegacyFallback: false,
      backgroundTaskRequested: true,
    })
  })

  it('falls back to cancel_double_ma_optimization when cancel_current_run fails', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'cancel_current_run') {
        throw new Error('missing')
      }
      return undefined
    })

    const result = await requestCancelCurrentRun()

    const commands = invokeCommandMock.mock.calls.map(([command]) => command)
    expect(commands).toContain('cancel_current_run')
    expect(commands).toContain('cancel_double_ma_optimization')
    expect(commands).toContain('cancel_background_task')
    expect(result).toEqual({
      requested: true,
      usedLegacyFallback: true,
      backgroundTaskRequested: true,
    })
  })

  it('keeps a successful cancellation request when the background-task probe fails', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'cancel_background_task') {
        throw new Error('background unavailable')
      }
      return undefined
    })

    const result = await requestCancelCurrentRun()

    const commands = invokeCommandMock.mock.calls.map(([command]) => command)
    expect(commands).toContain('cancel_current_run')
    expect(commands).toContain('cancel_background_task')
    expect(result).toEqual({
      requested: true,
      usedLegacyFallback: false,
      backgroundTaskRequested: false,
    })
  })

  it('throws when no cancellation command can be requested', async () => {
    invokeCommandMock.mockRejectedValue(new Error('missing'))

    await expect(requestCancelCurrentRun()).rejects.toThrow('Cancellation command is unavailable.')
  })
})
