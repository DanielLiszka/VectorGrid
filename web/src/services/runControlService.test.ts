import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { beginTrackedCommand, useTaskStore } from '../stores/taskStore'
import { invokeCommand } from '../tauri/invoke'
import {
  requestCancellationForMappedCommand,
  requestCancelCurrentRun,
} from './runControlService'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

const invokeCommandMock = vi.mocked(invokeCommand)

function deferred<T>(): {
  promise: Promise<T>
  resolve: (value: T) => void
  reject: (error?: unknown) => void
} {
  let resolve!: (value: T) => void
  let reject!: (error?: unknown) => void
  const promise = new Promise<T>((innerResolve, innerReject) => {
    resolve = innerResolve
    reject = innerReject
  })
  return { promise, resolve, reject }
}

async function flushMicrotasks(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
}

describe('runControlService', () => {
  let pinia: Pinia

  beforeEach(() => {
    vi.clearAllMocks()
    pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    useTaskStore(pinia).$reset()
  })

  afterEach(() => {
    bindRuntimePinia(null)
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

  it('marks current-run tasks cancelling immediately while cancellation is pending', async () => {
    const cancelCurrentRun = deferred<void>()
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'cancel_current_run') {
        return cancelCurrentRun.promise
      }
      return undefined
    })

    const store = useTaskStore(pinia)
    const taskId = beginTrackedCommand('run_meta_selection')

    const requestPromise = requestCancelCurrentRun()
    await flushMicrotasks()

    expect(store.activeById[taskId]?.status).toBe('cancelling')
    expect(store.centerStatus).toBe('cancelling')

    cancelCurrentRun.resolve(undefined)
    await expect(requestPromise).resolves.toEqual({
      requested: true,
      usedLegacyFallback: false,
      backgroundTaskRequested: true,
    })
  })

  it('restores current-run tasks to running when no foreground cancellation command succeeds', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      throw new Error(`unavailable:${command}`)
    })

    const store = useTaskStore(pinia)
    const taskId = beginTrackedCommand('run_meta_selection')

    await expect(requestCancelCurrentRun()).rejects.toThrow('Cancellation command is unavailable.')

    expect(store.activeById[taskId]?.status).toBe('running')
    expect(store.activeById[taskId]?.message).toBe('Cancellation failed. Task still running.')
  })

  it('restores mapped background tasks when background cancellation fails', async () => {
    invokeCommandMock.mockRejectedValue(new Error('background unavailable'))

    const store = useTaskStore(pinia)
    const taskId = beginTrackedCommand('polygon_download_1m_and_load')

    await expect(requestCancellationForMappedCommand('cancel_background_task')).rejects.toThrow(
      'Cancellation command is unavailable.',
    )

    expect(store.activeById[taskId]?.status).toBe('running')
    expect(store.activeById[taskId]?.message).toBe('Cancellation failed. Task still running.')
  })
})
