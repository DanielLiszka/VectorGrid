import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { useRuntimeStore } from '../stores/runtimeStore'
import { useSettingsStore } from '../stores/settingsStore'
import { useTaskStore, type TrackedTask } from '../stores/taskStore'
import { initializeDesktopProductization, teardownDesktopProductizationForTests } from './desktopProductization'

const saveWindowStateMock = vi.fn(async () => undefined)
const sendNotificationMock = vi.fn()
const isPermissionGrantedMock = vi.fn(async () => true)
const requestPermissionMock = vi.fn(async () => 'granted')
const attachConsoleMock = vi.fn(async () => () => {})
const infoMock = vi.fn(async (_message: string) => undefined)
let loggerCallback: ((entry: { level: number; message: string }) => void) | null = null
const attachLoggerMock = vi.fn(async (callback: (entry: { level: number; message: string }) => void) => {
  loggerCallback = callback
  return () => {
    loggerCallback = null
  }
})

vi.mock('../tauri/invoke', () => ({
  tauriRuntimeAvailable: () => true,
}))

vi.mock('@tauri-apps/plugin-window-state', () => ({
  StateFlags: { ALL: 63 },
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

describe('initializeDesktopProductization', () => {
  beforeEach(() => {
    const pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    loggerCallback = null
    saveWindowStateMock.mockClear()
    sendNotificationMock.mockClear()
    isPermissionGrantedMock.mockClear()
    requestPermissionMock.mockClear()
    attachConsoleMock.mockClear()
    attachLoggerMock.mockClear()
    infoMock.mockClear()
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

  it('routes important completed tasks to OS notifications when enabled', async () => {
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
})
