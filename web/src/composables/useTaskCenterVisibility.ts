import { shallowRef, type Ref } from 'vue'
import { useStorage, type RemovableRef } from '@vueuse/core'

export const TASK_CENTER_OPEN_STORAGE_KEY = 'VTA_TASK_CENTER_OPEN_V1'
export const TASK_CENTER_PANEL_ID = 'vta-task-center-panel'

const pendingReturnFocus = shallowRef<HTMLElement | null>(null)
let sharedTaskCenterOpen: RemovableRef<boolean> | null = null

function resolveReturnFocusTarget(explicitTarget?: HTMLElement | null): HTMLElement | null {
  if (explicitTarget) return explicitTarget
  if (typeof document === 'undefined') return null
  return document.activeElement instanceof HTMLElement ? document.activeElement : null
}

export function useTaskCenterVisibility(): RemovableRef<boolean> {
  if (!sharedTaskCenterOpen) {
    sharedTaskCenterOpen = useStorage<boolean>(TASK_CENTER_OPEN_STORAGE_KEY, false)
  }
  return sharedTaskCenterOpen
}

export function openTaskCenter(openState: Ref<boolean>, returnFocusTarget?: HTMLElement | null): void {
  pendingReturnFocus.value = resolveReturnFocusTarget(returnFocusTarget)
  openState.value = true
}

export function toggleTaskCenter(openState: Ref<boolean>, returnFocusTarget?: HTMLElement | null): void {
  if (!openState.value) pendingReturnFocus.value = resolveReturnFocusTarget(returnFocusTarget)
  openState.value = !openState.value
}

export function closeTaskCenter(openState: Ref<boolean>): void {
  openState.value = false
}

export function consumeTaskCenterReturnFocus(): HTMLElement | null {
  const target = pendingReturnFocus.value
  pendingReturnFocus.value = null
  return target
}
