// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import GlobalShortcutBridge from './GlobalShortcutBridge.vue'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useCommandPaletteStore } from '../stores/commandPaletteStore'
import { useDataStore } from '../stores/dataStore'
import { useOptimizeStore } from '../stores/optimizeStore'

function dispatchKeydown(options: KeyboardEventInit): void {
  const event = new KeyboardEvent('keydown', { bubbles: true, ...options })
  document.dispatchEvent(event)
}

describe('GlobalShortcutBridge', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = ''
  })

  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('opens the command palette on Ctrl+K', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const paletteStore = useCommandPaletteStore(pinia)
    const openSpy = vi.spyOn(paletteStore, 'openPalette')

    const wrapper = mount(GlobalShortcutBridge, { global: { plugins: [pinia] } })
    dispatchKeydown({ key: 'k', ctrlKey: true })

    expect(openSpy).toHaveBeenCalledTimes(1)
    wrapper.unmount()
  })

  it('closes the command palette on Escape when it is open', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const paletteStore = useCommandPaletteStore(pinia)
    const actionStore = useCommandActionStore(pinia)
    const closeSpy = vi.spyOn(paletteStore, 'closePalette')
    const clearDrillSpy = vi.spyOn(actionStore, 'clearDrillTradeSelection')
    paletteStore.openPalette()

    const wrapper = mount(GlobalShortcutBridge, { global: { plugins: [pinia] } })
    dispatchKeydown({ key: 'Escape' })

    expect(closeSpy).toHaveBeenCalledTimes(1)
    expect(paletteStore.open).toBe(false)
    expect(clearDrillSpy).not.toHaveBeenCalled()
    wrapper.unmount()
  })

  it('closes polygon modal on Escape before drill/cancel actions', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const dataStore = useDataStore(pinia)
    const actionStore = useCommandActionStore(pinia)
    const closePolygonSpy = vi.spyOn(dataStore, 'closePolygon')
    const clearDrillSpy = vi.spyOn(actionStore, 'clearDrillTradeSelection')
    const modal = document.createElement('div')
    modal.id = 'polygonModal'
    modal.className = 'open'
    document.body.appendChild(modal)

    const wrapper = mount(GlobalShortcutBridge, { global: { plugins: [pinia] } })
    dispatchKeydown({ key: 'Escape' })

    expect(closePolygonSpy).toHaveBeenCalledTimes(1)
    expect(clearDrillSpy).not.toHaveBeenCalled()
    wrapper.unmount()
  })

  it('runs drill clear and cancellation on Escape when cancellation is enabled', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const optimizeStore = useOptimizeStore(pinia)
    const actionStore = useCommandActionStore(pinia)
    const clearDrillSpy = vi.spyOn(actionStore, 'clearDrillTradeSelection')
    const cancelSpy = vi.spyOn(actionStore, 'cancelOptimization')
    optimizeStore.canCancel = true

    const wrapper = mount(GlobalShortcutBridge, { global: { plugins: [pinia] } })
    dispatchKeydown({ key: 'Escape' })

    expect(clearDrillSpy).toHaveBeenCalledTimes(1)
    expect(cancelSpy).toHaveBeenCalledTimes(1)
    wrapper.unmount()
  })

  it('guards Ctrl+O browse action when browsing is disabled', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const dataStore = useDataStore(pinia)
    const actionStore = useCommandActionStore(pinia)
    const browseSpy = vi.spyOn(actionStore, 'browseCsv')
    dataStore.canBrowseCsv = false

    const wrapper = mount(GlobalShortcutBridge, { global: { plugins: [pinia] } })
    dispatchKeydown({ key: 'o', ctrlKey: true })

    expect(browseSpy).not.toHaveBeenCalled()
    wrapper.unmount()
  })
})
