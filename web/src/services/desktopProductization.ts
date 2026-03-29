import { watch, type WatchStopHandle } from 'vue'
import { useDocumentVisibility } from '@vueuse/core'
import { attachConsole, attachLogger, info as writeDesktopInfoLog, LogLevel } from '@tauri-apps/plugin-log'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import { filename as windowStateFilename, saveWindowState, StateFlags } from '@tauri-apps/plugin-window-state'
import { resolveRuntimePinia } from '../stores/runtimePinia'
import { useRuntimeStore, type RuntimeLogLevel } from '../stores/runtimeStore'
import { useSettingsStore } from '../stores/settingsStore'
import { useTaskStore } from '../stores/taskStore'
import { desktopNotificationIntentForTask } from '../utils/desktopNotificationPolicy'
import { tauriRuntimeAvailable } from '../tauri/invoke'

let initialized = false
let consoleDetach: (() => void) | null = null
let loggerDetach: (() => void) | null = null
let taskWatchStop: WatchStopHandle | null = null
let saveWindowStateBound = false
const notifiedTaskIds = new Set<string>()
const documentVisibility = typeof document === 'undefined' ? null : useDocumentVisibility()

function runtimeStoreOrNull() {
  const pinia = resolveRuntimePinia()
  if (!pinia) return null
  return useRuntimeStore(pinia)
}

function mapLogLevel(level: LogLevel): RuntimeLogLevel {
  if (level === LogLevel.Trace) return 'trace'
  if (level === LogLevel.Debug) return 'debug'
  if (level === LogLevel.Warn) return 'warn'
  if (level === LogLevel.Error) return 'error'
  return 'info'
}

async function bindWindowStateSaving(): Promise<void> {
  const runtimeStore = runtimeStoreOrNull()
  if (!runtimeStore) return

  try {
    const file = await windowStateFilename()
    runtimeStore.setDesktopWindowState(file, true)
    runtimeStore.pushDesktopLog('info', `Window state persistence active (${file}).`)
  } catch (error) {
    runtimeStore.setDesktopWindowState('', false)
    runtimeStore.pushDesktopLog(
      'warn',
      `Window state metadata unavailable: ${String(error instanceof Error ? error.message : error)}`,
    )
  }

  if (typeof window === 'undefined' || saveWindowStateBound) return
  const persist = () => {
    void saveWindowState(StateFlags.ALL).catch((error) => {
      runtimeStore.pushDesktopLog(
        'warn',
        `Window state save failed: ${String(error instanceof Error ? error.message : error)}`,
      )
    })
  }
  window.addEventListener('pagehide', persist)
  window.addEventListener('beforeunload', persist)
  saveWindowStateBound = true
}

async function refreshDesktopNotificationPermission(): Promise<void> {
  const runtimeStore = runtimeStoreOrNull()
  if (!runtimeStore) return
  try {
    runtimeStore.setDesktopNotificationPermission((await isPermissionGranted()) ? 'granted' : 'default')
  } catch (error) {
    runtimeStore.setDesktopNotificationPermission('unsupported')
    runtimeStore.pushDesktopLog(
      'warn',
      `OS notifications unavailable: ${String(error instanceof Error ? error.message : error)}`,
    )
  }
}

async function installDesktopLogging(): Promise<void> {
  const pinia = resolveRuntimePinia()
  if (!pinia) return
  const runtimeStore = useRuntimeStore(pinia)

  try {
    loggerDetach = await attachLogger((entry) => {
      runtimeStore.pushDesktopLog(mapLogLevel(entry.level), entry.message)
    })
    runtimeStore.pushDesktopLog('info', 'Desktop log bridge attached.')
  } catch (error) {
    runtimeStore.pushDesktopLog('warn', `Desktop log stream unavailable: ${String(error instanceof Error ? error.message : error)}`)
  }

  if (!import.meta.env.DEV) return

  try {
    consoleDetach = await attachConsole()
    runtimeStore.setDesktopLogConsoleAttached(true)
    runtimeStore.pushDesktopLog('debug', 'Desktop log console mirroring attached.')
  } catch (error) {
    runtimeStore.pushDesktopLog('warn', `Console log bridge unavailable: ${String(error instanceof Error ? error.message : error)}`)
  }
}

function installTaskNotificationRouting(): void {
  const pinia = resolveRuntimePinia()
  if (!pinia) return
  const settingsStore = useSettingsStore(pinia)
  const taskStore = useTaskStore(pinia)
  const runtimeStore = useRuntimeStore(pinia)

  taskWatchStop = watch(
    () => taskStore.history[0]?.id ?? '',
    async (taskId, previousTaskId) => {
      if (!taskId || taskId === previousTaskId || notifiedTaskIds.has(taskId)) return
      const task = taskStore.history.find((entry) => entry.id === taskId) ?? null
      const intent = desktopNotificationIntentForTask(task, settingsStore.desktopNotificationMode, {
        documentVisibility: documentVisibility?.value,
      })
      if (!intent) return

      if (runtimeStore.desktopNotificationPermission !== 'granted') {
        return
      }

      notifiedTaskIds.add(taskId)
      try {
        sendNotification({
          title: intent.title,
          body: intent.body,
        })
      } catch (error) {
        runtimeStore.pushDesktopLog('warn', `OS notification delivery failed: ${String(error instanceof Error ? error.message : error)}`)
      }
    },
    { flush: 'post' },
  )
}

export async function initializeDesktopProductization(): Promise<void> {
  if (initialized || !tauriRuntimeAvailable()) return
  initialized = true

  await bindWindowStateSaving()
  await installDesktopLogging()
  await refreshDesktopNotificationPermission()
  installTaskNotificationRouting()
  void writeDesktopInfoLog('Desktop productization bridges ready.').catch(() => undefined)
}

export async function requestDesktopNotificationPermission(): Promise<void> {
  if (!tauriRuntimeAvailable()) return
  const runtimeStore = runtimeStoreOrNull()
  if (!runtimeStore) return

  try {
    const permission = await requestPermission()
    runtimeStore.setDesktopNotificationPermission(
      permission === 'granted' ? 'granted' : permission === 'denied' ? 'denied' : 'default',
    )
    runtimeStore.pushDesktopLog(
      permission === 'granted' ? 'info' : 'warn',
      permission === 'granted'
        ? 'OS notification permission granted.'
        : `OS notification permission ${permission}.`,
    )
  } catch (error) {
    runtimeStore.setDesktopNotificationPermission('unsupported')
    runtimeStore.pushDesktopLog(
      'warn',
      `OS notification permission request failed: ${String(error instanceof Error ? error.message : error)}`,
    )
  }
}

export function teardownDesktopProductizationForTests(): void {
  if (consoleDetach) {
    consoleDetach()
    consoleDetach = null
  }
  if (loggerDetach) {
    loggerDetach()
    loggerDetach = null
  }
  if (taskWatchStop) {
    taskWatchStop()
    taskWatchStop = null
  }
  const runtimeStore = runtimeStoreOrNull()
  runtimeStore?.setDesktopLogConsoleAttached(false)
  initialized = false
  saveWindowStateBound = false
  notifiedTaskIds.clear()
}
