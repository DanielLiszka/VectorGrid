// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { useRuntimeStore } from '../stores/runtimeStore'
import { useSettingsStore } from '../stores/settingsStore'
import { useTaskStore, type TrackedTask } from '../stores/taskStore'
import {
  initializeDesktopProductization,
  requestDesktopNotificationPermission,
  teardownDesktopProductizationForTests,
} from './desktopProductization'

const saveWindowStateMock = vi.fn(async () => undefined)
const filenameMock = vi.fn(async () => 'vectorgrid-window-state.json')
const sendNotificationMock = vi.fn((_payload: unknown) => undefined)
const isPermissionGrantedMock = vi.fn(async () => true)
const requestPermissionMock = vi.fn(async () => 'granted')
const attachConsoleMock = vi.fn(async () => () => {})
const infoMock = vi.fn(async (_message: string) => undefined)
const tauriRuntimeAvailableMock = vi.fn(() => true)
let loggerCallback: ((entry: { level: number; message: string }) => void) | null = null
const attachLoggerMock = vi.fn(async (callback: (entry: { level: number; message: string }) => void) => {
  loggerCallback = callback
  return () => {
    loggerCallback = null
  }
})
vi.mock('../tauri/invoke', () => ({
  tauriRuntimeAvailable: () => tauriRuntimeAvailableMock(),
}))

vi.mock('@tauri-apps/plugin-window-state', () => ({
  StateFlags: { ALL: 63 },
  filename: () => filenameMock(),
  saveWindowState: () => saveWindowStateMock(),
}))

vi.mock('@tauri-apps/plugin-log', () => ({
  LogLevel: {
    Trace: 1,
    Debug: 2,
    Info: 3,
    Warn: 4,
    Error: 5,
  },
  attachConsole: () => attachConsoleMock(),
  attachLogger: (callback: (entry: { level: number; message: string }) => void) => attachLoggerMock(callback),
  info: (message: string) => infoMock(message),
}))

vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: () => isPermissionGrantedMock(),
  requestPermission: () => requestPermissionMock(),
  sendNotification: (payload: unknown) => sendNotificationMock(payload),
}))

function makeTrackedTask(partial: Partial<TrackedTask>): TrackedTask {
  return {
    id: 'task_1',
    command: 'run_double_ma_optimization',
    label: 'Double Ma Optimization',
    status: 'success',
    startedAt: Date.now(),
    ...partial,
  }
}

function setDocumentVisibilityState(state: 'hidden' | 'visible'): void {
  Object.defineProperty(document, 'visibilityState', {
    configurable: true,
    value: state,
  })
  document.dispatchEvent(new Event('visibilitychange'))
}

describe('initializeDesktopProductization', () => {
  beforeEach(() => {
    const pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    localStorage.clear()
    loggerCallback = null
    saveWindowStateMock.mockClear()
    sendNotificationMock.mockClear()
    isPermissionGrantedMock.mockClear()
    requestPermissionMock.mockClear()
    attachConsoleMock.mockClear()
    attachLoggerMock.mockClear()
    infoMock.mockClear()
    filenameMock.mockReset()
    filenameMock.mockResolvedValue('vectorgrid-window-state.json')
    tauriRuntimeAvailableMock.mockReset()
    tauriRuntimeAvailableMock.mockReturnValue(true)
    setDocumentVisibilityState('hidden')
  })

  afterEach(() => {
    teardownDesktopProductizationForTests()
  })

  it('captures desktop log events into the runtime diagnostics store', async () => {
    await initializeDesktopProductization()
    const runtimeStore = useRuntimeStore()

    loggerCallback?.({ level: 3, message: 'desktop log ready' })

    expect(runtimeStore.desktopLogs[0]).toMatchObject({
      level: 'info',
      message: 'desktop log ready',
    })
  })

  it('is a no-op when the Tauri runtime is unavailable', async () => {
    tauriRuntimeAvailableMock.mockReturnValue(false)
    const runtimeStore = useRuntimeStore()

    await initializeDesktopProductization()
    await requestDesktopNotificationPermission()

    expect(attachLoggerMock).not.toHaveBeenCalled()
    expect(attachConsoleMock).not.toHaveBeenCalled()
    expect(isPermissionGrantedMock).not.toHaveBeenCalled()
    expect(requestPermissionMock).not.toHaveBeenCalled()
    expect(infoMock).not.toHaveBeenCalled()
    expect(runtimeStore.desktopLogs).toEqual([])
    expect(runtimeStore.desktopNotificationPermission).toBe('unknown')
    expect(runtimeStore.desktopWindowStateReady).toBe(false)
  })

  it('records window-state metadata and binds manual saves when the plugin is available', async () => {
    await initializeDesktopProductization()
    const runtimeStore = useRuntimeStore()

    expect(runtimeStore.desktopWindowStateFile).toBe('vectorgrid-window-state.json')
    expect(runtimeStore.desktopWindowStateReady).toBe(true)

    window.dispatchEvent(new Event('pagehide'))
    expect(saveWindowStateMock).toHaveBeenCalledTimes(1)
  })

  it('downgrades window-state metadata failures into diagnostics warnings', async () => {
    filenameMock.mockRejectedValueOnce(new Error('window-state capability denied'))
    await initializeDesktopProductization()
    const runtimeStore = useRuntimeStore()

    expect(runtimeStore.desktopWindowStateFile).toBe('')
    expect(runtimeStore.desktopWindowStateReady).toBe(false)
    expect(runtimeStore.desktopLogs.some((entry) => entry.message.includes('Window state metadata unavailable'))).toBe(true)
  })

  it('marks desktop notifications unsupported when permission refresh fails', async () => {
    isPermissionGrantedMock.mockRejectedValueOnce(new Error('notifications unavailable'))
    await initializeDesktopProductization()
    const runtimeStore = useRuntimeStore()

    expect(runtimeStore.desktopNotificationPermission).toBe('unsupported')
    expect(runtimeStore.desktopLogs.some((entry) => entry.message.includes('OS notifications unavailable'))).toBe(true)
  })

  it('routes important completed tasks to OS notifications when enabled', async () => {
    setDocumentVisibilityState('hidden')
    await initializeDesktopProductization()
    const settingsStore = useSettingsStore()
    const taskStore = useTaskStore()
    settingsStore.setSelectField('settingsDesktopNotifications', 'important')

    taskStore.$patch({
      history: [
        makeTrackedTask({
          id: 'task_notify_success',
          status: 'success',
          message: 'Finished with 20 shortlisted candidates.',
        }),
      ],
    })

    await nextTick()
    await Promise.resolve()

    expect(sendNotificationMock).toHaveBeenCalledWith({
      title: 'Double Ma Optimization complete',
      body: 'Finished with 20 shortlisted candidates.',
    })
  })

  it('suppresses OS notifications while the desktop surface is visible', async () => {
    setDocumentVisibilityState('visible')
    await initializeDesktopProductization()
    const settingsStore = useSettingsStore()
    const taskStore = useTaskStore()
    settingsStore.setSelectField('settingsDesktopNotifications', 'important')

    taskStore.$patch({
      history: [
        makeTrackedTask({
          id: 'task_notify_visible',
          status: 'success',
          message: 'Finished with 12 shortlisted candidates.',
        }),
      ],
    })

    await nextTick()
    await Promise.resolve()

    expect(sendNotificationMock).not.toHaveBeenCalled()
  })

  it('suppresses OS notifications when the preference is off', async () => {
    await initializeDesktopProductization()
    const taskStore = useTaskStore()

    taskStore.$patch({
      history: [
        makeTrackedTask({
          id: 'task_notify_off',
          status: 'failed',
          error: 'Backend panic.',
        }),
      ],
    })

    await nextTick()
    await Promise.resolve()

    expect(sendNotificationMock).not.toHaveBeenCalled()
  })

  it('records a warning when notification delivery throws', async () => {
    sendNotificationMock.mockImplementationOnce(() => {
      throw new Error('delivery failed')
    })
    await initializeDesktopProductization()
    const settingsStore = useSettingsStore()
    const taskStore = useTaskStore()
    const runtimeStore = useRuntimeStore()
    settingsStore.setSelectField('settingsDesktopNotifications', 'important')

    taskStore.$patch({
      history: [
        makeTrackedTask({
          id: 'task_notify_delivery_failure',
          status: 'failed',
          error: 'Desktop notification should fail.',
        }),
      ],
    })

    await nextTick()
    await Promise.resolve()

    expect(runtimeStore.desktopLogs.some((entry) => entry.message.includes('OS notification delivery failed'))).toBe(true)
  })

  it('requests notification permission through the runtime diagnostics bridge', async () => {
    const runtimeStore = useRuntimeStore()

    await requestDesktopNotificationPermission()

    expect(requestPermissionMock).toHaveBeenCalledTimes(1)
    expect(runtimeStore.desktopNotificationPermission).toBe('granted')
    expect(runtimeStore.desktopLogs.some((entry) => entry.message.includes('OS notification permission granted'))).toBe(true)
  })

  it('downgrades notification permission request failures into unsupported diagnostics', async () => {
    requestPermissionMock.mockRejectedValueOnce(new Error('permission request unavailable'))
    const runtimeStore = useRuntimeStore()

    await requestDesktopNotificationPermission()

    expect(runtimeStore.desktopNotificationPermission).toBe('unsupported')
    expect(runtimeStore.desktopLogs.some((entry) => entry.message.includes('OS notification permission request failed'))).toBe(
      true,
    )
  })

})
