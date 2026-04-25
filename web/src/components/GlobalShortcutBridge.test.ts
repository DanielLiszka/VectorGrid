import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import GlobalShortcutBridge from './GlobalShortcutBridge.vue'
import { useAnalysisStore } from '../stores/analysisStore'
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
    dataStore.polygonOpen = true

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

  it('routes secondary shortcut inventory through the existing command actions', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const optimizeStore = useOptimizeStore(pinia)
    const actionStore = useCommandActionStore(pinia)

    analysisStore.applyLegacySnapshot({ canExportCsv: true, canExportJson: true })
    actionStore.applyLegacyQuickActionsSnapshot({
      latestReportPath: 'C:/reports/latest.xlsx',
      latestMetaReportPath: 'C:/reports/meta.json',
    })
    dataStore.canPolygonOpen = true
    optimizeStore.$patch({ runDisabled: false, validateDisabled: false })

    const openPolygonSpy = vi.spyOn(actionStore, 'openPolygon').mockImplementation(() => {})
    const toggleControlsSpy = vi.spyOn(actionStore, 'toggleControls').mockImplementation(() => {})
    const runSpy = vi.spyOn(actionStore, 'runOptimization').mockImplementation(() => {})
    const validateSpy = vi.spyOn(actionStore, 'validateOptimization').mockImplementation(() => {})
    const exportCsvSpy = vi.spyOn(actionStore, 'exportTopCsv').mockImplementation(() => {})
    const exportJsonSpy = vi.spyOn(actionStore, 'exportTopJson').mockImplementation(() => {})
    const openReportSpy = vi.spyOn(actionStore, 'openLatestReport').mockImplementation(() => {})
    const openMetaReportSpy = vi.spyOn(actionStore, 'openLatestMetaReport').mockImplementation(() => {})

    const wrapper = mount(GlobalShortcutBridge, { global: { plugins: [pinia] } })

    dispatchKeydown({ key: 'O', ctrlKey: true, shiftKey: true })
    dispatchKeydown({ key: '\\', ctrlKey: true })
    dispatchKeydown({ key: 'Enter', ctrlKey: true })
    dispatchKeydown({ key: 'V', ctrlKey: true, shiftKey: true })
    dispatchKeydown({ key: 'C', ctrlKey: true, shiftKey: true })
    dispatchKeydown({ key: 'J', ctrlKey: true, shiftKey: true })
    dispatchKeydown({ key: 'R', ctrlKey: true, shiftKey: true })
    dispatchKeydown({ key: 'M', ctrlKey: true, shiftKey: true })

    expect(openPolygonSpy).toHaveBeenCalledTimes(1)
    expect(toggleControlsSpy).toHaveBeenCalledTimes(1)
    expect(runSpy).toHaveBeenCalledTimes(1)
    expect(validateSpy).toHaveBeenCalledTimes(1)
    expect(exportCsvSpy).toHaveBeenCalledTimes(1)
    expect(exportJsonSpy).toHaveBeenCalledTimes(1)
    expect(openReportSpy).toHaveBeenCalledTimes(1)
    expect(openMetaReportSpy).toHaveBeenCalledTimes(1)

    wrapper.unmount()
  })

  it('does not trigger run optimization from Ctrl+Enter while a multiline editor has focus', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const actionStore = useCommandActionStore(pinia)
    const runSpy = vi.spyOn(actionStore, 'runOptimization').mockImplementation(() => {})

    const textarea = document.createElement('textarea')
    document.body.appendChild(textarea)
    textarea.focus()

    const wrapper = mount(GlobalShortcutBridge, { global: { plugins: [pinia] } })
    dispatchKeydown({ key: 'Enter', ctrlKey: true })

    expect(runSpy).not.toHaveBeenCalled()
    wrapper.unmount()
  })
})
