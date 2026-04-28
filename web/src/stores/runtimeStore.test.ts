import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { BOOT_STAGES, useRuntimeStore } from './runtimeStore'

describe('runtimeStore', () => {
  beforeEach(() => {
    const pinia = createPinia()
    setActivePinia(pinia)
  })

  it('promotes idle state into booting and captures fatal error detail', () => {
    const store = useRuntimeStore()
    const nowSpy = vi.spyOn(Date, 'now').mockReturnValue(1_700_000_000_000)

    store.setBootStage(BOOT_STAGES.RESTORING_WORKSPACE_STATE)

    expect(store.bootPhase).toBe('booting')
    expect(store.bootStage).toBe(BOOT_STAGES.RESTORING_WORKSPACE_STATE)
    expect(store.bootStartedAt).toBe(1_700_000_000_000)

    const error = new Error('runtime boot failed')
    error.stack = 'Error: runtime boot failed\n    at runtime'
    store.markError(error)

    expect(store.bootPhase).toBe('error')
    expect(store.bootStage).toBe(BOOT_STAGES.FRONTEND_RUNTIME_FAILED)
    expect(store.bootError).toBe('runtime boot failed')
    expect(store.bootErrorDetail).toContain('at runtime')

    nowSpy.mockRestore()
  })

  it('caps desktop log retention and clears logs explicitly', () => {
    const store = useRuntimeStore()

    for (let index = 0; index < 30; index += 1) {
      store.pushDesktopLog('info', `desktop log ${index}`)
    }

    expect(store.desktopLogs).toHaveLength(24)
    expect(store.desktopLogs[0]?.message).toBe('desktop log 29')
    expect(store.desktopLogs[store.desktopLogs.length - 1]?.message).toBe('desktop log 6')

    store.clearDesktopLogs()
    expect(store.desktopLogs).toEqual([])
  })

  it('redacts secret-looking values from runtime errors and logs', () => {
    const store = useRuntimeStore()
    const error = new Error('boot failed apiSecret=alpaca_secret_123')
    error.stack = 'Error: boot failed apiSecret=alpaca_secret_123'

    store.markError(error)
    store.pushDesktopLog('warn', 'Polygon status apiKey=poly_secret_123')

    const serialized = JSON.stringify({
      bootError: store.bootError,
      bootErrorDetail: store.bootErrorDetail,
      logs: store.desktopLogs,
    })
    expect(serialized).toContain('[redacted]')
    expect(serialized).not.toContain('alpaca_secret_123')
    expect(serialized).not.toContain('poly_secret_123')
  })

  it('tracks desktop bridge readiness flags and ready-state timing', () => {
    const store = useRuntimeStore()
    const nowSpy = vi.spyOn(Date, 'now')
    nowSpy.mockReturnValueOnce(100).mockReturnValueOnce(160)

    store.markBooting()
    store.markReady()
    store.setLegacyMounted(true)
    store.setDesktopLogConsoleAttached(true)
    store.setDesktopWindowState('vectorgrid-window-state.json', true)
    store.setDesktopNotificationPermission('granted')

    expect(store.bootPhase).toBe('ready')
    expect(store.bootStage).toBe(BOOT_STAGES.FRONTEND_RUNTIME_READY)
    expect(store.bootElapsedMs).toBe(60)
    expect(store.legacyMounted).toBe(true)
    expect(store.desktopLogConsoleAttached).toBe(true)
    expect(store.desktopWindowStateFile).toBe('vectorgrid-window-state.json')
    expect(store.desktopWindowStateReady).toBe(true)
    expect(store.desktopNotificationPermission).toBe('granted')

    nowSpy.mockRestore()
  })
})
