// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from 'vitest'
import {
  TASK_CENTER_OPEN_STORAGE_KEY,
  closeTaskCenter,
  consumeTaskCenterReturnFocus,
  openTaskCenter,
  toggleTaskCenter,
  useTaskCenterVisibility,
} from './useTaskCenterVisibility'

describe('useTaskCenterVisibility', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = ''
    useTaskCenterVisibility().value = false
    consumeTaskCenterReturnFocus()
  })

  it('hydrates the shared open state from localStorage and persists toggle changes', () => {
    localStorage.setItem(TASK_CENTER_OPEN_STORAGE_KEY, 'true')

    const openState = useTaskCenterVisibility()
    expect(openState.value).toBe(true)

    closeTaskCenter(openState)
    expect(openState.value).toBe(false)
    expect(localStorage.getItem(TASK_CENTER_OPEN_STORAGE_KEY)).toBe('false')

    toggleTaskCenter(openState)
    expect(openState.value).toBe(true)
    expect(localStorage.getItem(TASK_CENTER_OPEN_STORAGE_KEY)).toBe('true')
  })

  it('captures and consumes return focus targets when opening the task center', () => {
    const opener = document.createElement('button')
    opener.type = 'button'
    opener.textContent = 'Open task center'
    document.body.appendChild(opener)
    opener.focus()

    const openState = useTaskCenterVisibility()
    openTaskCenter(openState)

    expect(openState.value).toBe(true)
    expect(consumeTaskCenterReturnFocus()).toBe(opener)
    expect(consumeTaskCenterReturnFocus()).toBeNull()
  })

  it('captures an explicit focus target only when toggling from closed to open', () => {
    const opener = document.createElement('button')
    opener.type = 'button'
    opener.textContent = 'Toggle task center'
    document.body.appendChild(opener)

    const openState = useTaskCenterVisibility()
    toggleTaskCenter(openState, opener)
    expect(openState.value).toBe(true)
    expect(consumeTaskCenterReturnFocus()).toBe(opener)

    toggleTaskCenter(openState, opener)
    expect(openState.value).toBe(false)
    expect(consumeTaskCenterReturnFocus()).toBeNull()
  })
})
