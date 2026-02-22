import { describe, expect, it } from 'vitest'
import { invokeByName } from './invoke'
import { TauriRuntimeError, normalizeTauriRuntimeError } from './runtimeError'

describe('tauri runtime errors', () => {
  it('normalizes non-error causes into a typed runtime error', () => {
    const err = normalizeTauriRuntimeError({
      code: 'TAURI_INVOKE_FAILED',
      operation: 'invoke:run_double_ma_optimization',
      fallbackMessage: 'Invoke failed.',
      causeValue: { reason: 'network' },
    })
    expect(err).toBeInstanceOf(TauriRuntimeError)
    expect(err.code).toBe('TAURI_INVOKE_FAILED')
    expect(err.operation).toBe('invoke:run_double_ma_optimization')
    expect(err.message).toContain('Invoke failed.')
    expect(err.message).toContain('network')
  })

  it('throws typed error for unknown commands before invoke', async () => {
    await expect(invokeByName('not_real_command')).rejects.toMatchObject({
      name: 'TauriRuntimeError',
      code: 'TAURI_COMMAND_UNKNOWN',
    })
  })
})
