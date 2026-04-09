// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { invokeByName, invokeCommand, resetInvokeLifecycleHooksForTests, setInvokeLifecycleHooks } from './invoke'
import { TauriRuntimeError } from './runtimeError'
import { installMockIpc, resetTauriApiMocks } from '../test/tauriMocks'

describe('invoke mockIPC bridge', () => {
  beforeEach(() => {
    resetTauriApiMocks()
    resetInvokeLifecycleHooksForTests()
  })

  afterEach(() => {
    resetInvokeLifecycleHooksForTests()
    resetTauriApiMocks()
  })

  it('returns the mocked payload on success', async () => {
    const harness = installMockIpc({
      cancel_current_run: { ok: true, requested: true },
    })

    await expect(invokeCommand('cancel_current_run', { scope: 'desktop' })).resolves.toEqual({
      ok: true,
      requested: true,
    })
    harness.expectCalled('cancel_current_run', { scope: 'desktop' })
  })

  it('normalizes async rejected IPC promises into TauriRuntimeError', async () => {
    installMockIpc({
      cancel_current_run: async () => Promise.reject(new Error('async unavailable')),
    })

    await expect(invokeCommand('cancel_current_run')).rejects.toMatchObject({
      name: 'TauriRuntimeError',
      code: 'TAURI_INVOKE_FAILED',
      operation: 'invoke:cancel_current_run',
    })
  })

  it('normalizes thrown string errors into TauriRuntimeError', async () => {
    installMockIpc({
      cancel_current_run: () => {
        throw 'missing runtime'
      },
    })

    const error = await invokeCommand('cancel_current_run').catch((cause) => cause as TauriRuntimeError)
    expect(error).toBeInstanceOf(TauriRuntimeError)
    expect((error as TauriRuntimeError).code).toBe('TAURI_INVOKE_FAILED')
    expect((error as TauriRuntimeError).message).toContain('missing runtime')
  })

  it('normalizes thrown Error objects into TauriRuntimeError', async () => {
    installMockIpc({
      cancel_current_run: () => {
        throw new Error('backend panic')
      },
    })

    const error = await invokeCommand('cancel_current_run').catch((cause) => cause as TauriRuntimeError)
    expect(error).toBeInstanceOf(TauriRuntimeError)
    expect((error as TauriRuntimeError).code).toBe('TAURI_INVOKE_FAILED')
    expect((error as TauriRuntimeError).message).toContain('backend panic')
  })

  it('rejects unknown invokeByName commands before IPC dispatch', async () => {
    const harness = installMockIpc({}, { allowUnknown: false })

    await expect(invokeByName('not_real_command')).rejects.toMatchObject({
      code: 'TAURI_COMMAND_UNKNOWN',
      operation: 'invoke:not_real_command',
    })
    expect(harness.calls).toHaveLength(0)
  })

  it('forwards command args unchanged to the mock IPC layer', async () => {
    const harness = installMockIpc({
      cancel_current_run: { ok: true },
    })
    const args = { nested: { mode: 'safe' }, retry: 2 }

    await invokeCommand('cancel_current_run', args)

    expect(harness.calls[0]?.args).toEqual(args)
    harness.expectCallCount('cancel_current_run', 1)
  })

  it('keeps invoke lifecycle hooks best-effort when hook callbacks throw', async () => {
    const harness = installMockIpc({
      cancel_current_run: { ok: true },
    })
    const beforeInvoke = vi.fn(() => {
      throw new Error('before hook failed')
    })
    const afterSuccess = vi.fn(() => {
      throw new Error('after success failed')
    })

    setInvokeLifecycleHooks({ beforeInvoke, afterSuccess })

    await expect(invokeCommand('cancel_current_run', { scope: 'desktop' })).resolves.toEqual({
      ok: true,
    })
    expect(beforeInvoke).toHaveBeenCalledTimes(1)
    expect(afterSuccess).toHaveBeenCalledTimes(1)
    harness.expectCallCount('cancel_current_run', 1)
  })
})
