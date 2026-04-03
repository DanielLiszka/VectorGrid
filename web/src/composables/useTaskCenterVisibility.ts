import { ref, type Ref } from 'vue'

export const TASK_CENTER_OPEN_STORAGE_KEY = 'VTA_TASK_CENTER_OPEN_V1'
export const TASK_CENTER_PANEL_ID = 'vta-task-center-panel'

type TaskCenterWindow = Window & {
  __VTA_TASK_CENTER_OPEN__?: Ref<boolean>
  __VTA_TASK_CENTER_RETURN_FOCUS__?: HTMLElement | null
}

const fallbackTaskCenterOpen = ref(false)
let fallbackReturnFocus: HTMLElement | null = null

function taskCenterWindow(): TaskCenterWindow | null {
  return typeof window === 'undefined' ? null : (window as TaskCenterWindow)
}

function readPersistedTaskCenterState(): boolean {
  const browserWindow = taskCenterWindow()
  if (!browserWindow) return fallbackTaskCenterOpen.value
  return browserWindow.localStorage.getItem(TASK_CENTER_OPEN_STORAGE_KEY) === 'true'
}

function writePersistedTaskCenterState(value: boolean): void {
  const browserWindow = taskCenterWindow()
  if (!browserWindow) return
  browserWindow.localStorage.setItem(TASK_CENTER_OPEN_STORAGE_KEY, value ? 'true' : 'false')
}

function sharedTaskCenterOpenRef(): Ref<boolean> {
  const browserWindow = taskCenterWindow()
  if (!browserWindow) return fallbackTaskCenterOpen
  if (!browserWindow.__VTA_TASK_CENTER_OPEN__) {
    browserWindow.__VTA_TASK_CENTER_OPEN__ = ref(readPersistedTaskCenterState())
  }
  return browserWindow.__VTA_TASK_CENTER_OPEN__
}

function setTaskCenterReturnFocus(target: HTMLElement | null): void {
  const browserWindow = taskCenterWindow()
  if (!browserWindow) {
    fallbackReturnFocus = target
    return
  }
  browserWindow.__VTA_TASK_CENTER_RETURN_FOCUS__ = target
}

function takeTaskCenterReturnFocus(): HTMLElement | null {
  const browserWindow = taskCenterWindow()
  if (!browserWindow) {
    const target = fallbackReturnFocus
    fallbackReturnFocus = null
    return target
  }
  const target = browserWindow.__VTA_TASK_CENTER_RETURN_FOCUS__ ?? null
  browserWindow.__VTA_TASK_CENTER_RETURN_FOCUS__ = null
  return target
}

function resolveReturnFocusTarget(explicitTarget?: HTMLElement | null): HTMLElement | null {
  if (explicitTarget) return explicitTarget
  if (typeof document === 'undefined') return null
  return document.activeElement instanceof HTMLElement ? document.activeElement : null
}

export function useTaskCenterVisibility(): Ref<boolean> {
  const openState = sharedTaskCenterOpenRef()
  openState.value = readPersistedTaskCenterState()
  return openState
}

export function openTaskCenter(openState: Ref<boolean>, returnFocusTarget?: HTMLElement | null): void {
  setTaskCenterReturnFocus(resolveReturnFocusTarget(returnFocusTarget))
  openState.value = true
  writePersistedTaskCenterState(true)
}

export function toggleTaskCenter(openState: Ref<boolean>, returnFocusTarget?: HTMLElement | null): void {
  if (!openState.value) setTaskCenterReturnFocus(resolveReturnFocusTarget(returnFocusTarget))
  openState.value = !openState.value
  writePersistedTaskCenterState(openState.value)
}

export function closeTaskCenter(openState: Ref<boolean>): void {
  openState.value = false
  writePersistedTaskCenterState(false)
}

export function consumeTaskCenterReturnFocus(): HTMLElement | null {
  return takeTaskCenterReturnFocus()
}
