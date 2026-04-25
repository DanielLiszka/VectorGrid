import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import OptimizeContextDock from './OptimizeContextDock.vue'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { OPTIMIZE_DOCK_OPEN_STORAGE_KEY } from '../utils/shellDockContract'

function dispatchPointer(target: EventTarget, type: string, coords: { x: number; y: number }, pointerId = 1): void {
  const event = new MouseEvent(type, {
    bubbles: true,
    cancelable: true,
    clientX: coords.x,
    clientY: coords.y,
    button: 0,
  })
  Object.defineProperty(event, 'pointerId', { value: pointerId })
  target.dispatchEvent(event)
}

describe('OptimizeContextDock', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = ''
    window.innerWidth = 1280
    window.innerHeight = 900
  })

  it('renders optimize summary fields and routes actions through the command action store', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const optimizeStore = useOptimizeStore(pinia)
    optimizeStore.applyLegacySnapshot({
      strategyKind: 'DoubleMa',
      backend: 'GpuKernel',
      objective: 'Net PnL',
      mode: 'max',
      fastStart: '5',
      fastEnd: '20',
      slowStart: '40',
      slowEnd: '100',
      fastMaType: 'ema',
      slowMaType: 'alma',
      pairsPreview: '12 pairs',
      gpuStatus: 'Ready',
      progressMeta: 'Ready to run',
    })

    const commandActionStore = useCommandActionStore(pinia)
    const runSpy = vi.spyOn(commandActionStore, 'runOptimization').mockImplementation(() => {})
    const validateSpy = vi.spyOn(commandActionStore, 'validateOptimization').mockImplementation(() => {})

    const wrapper = mount(OptimizeContextDock, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await wrapper.get('.dockToggle').trigger('click')
    await nextTick()

    expect(wrapper.text()).toContain('Optimize Context')
    expect(wrapper.text()).toContain('DoubleMa')
    expect(wrapper.text()).toContain('GpuKernel')
    expect(wrapper.text()).toContain('Ready')

    const runButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Run'),
    )
    const validateButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Validate'),
    )
    runButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    validateButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await nextTick()

    expect(runSpy).toHaveBeenCalledTimes(1)
    expect(validateSpy).toHaveBeenCalledTimes(1)
  })

  it('allows the optimize context dock to be minimized and reopened via a persisted toggle', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const optimizeStore = useOptimizeStore(pinia)
    optimizeStore.applyLegacySnapshot({
      strategyKind: 'DoubleMa',
      progressMeta: 'Ready to run',
    })

    const wrapper = mount(OptimizeContextDock, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect(document.querySelector('.optDock')).toBeNull()

    await wrapper.get('.dockToggle').trigger('click')
    await nextTick()
    expect(document.querySelector('.optDock')).not.toBeNull()
    expect(localStorage.getItem(OPTIMIZE_DOCK_OPEN_STORAGE_KEY)).toBe('true')

    const minimizeButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.getAttribute('aria-label') || '').includes('Minimize optimize context'),
    )
    minimizeButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await nextTick()

    expect(document.querySelector('.optDock')).toBeNull()
    expect(localStorage.getItem(OPTIMIZE_DOCK_OPEN_STORAGE_KEY)).toBe('false')
  })

  it('persists drag repositioning for the optimize dock header', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const optimizeStore = useOptimizeStore(pinia)
    optimizeStore.applyLegacySnapshot({
      strategyKind: 'DoubleMa',
      progressMeta: 'Ready to run',
    })

    mount(OptimizeContextDock, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()
    const toggle = document.querySelector<HTMLButtonElement>('.dockToggle')
    toggle?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await nextTick()

    const header = document.querySelector<HTMLElement>('.dockHeader')
    expect(header).not.toBeNull()

    const before = JSON.parse(String(localStorage.getItem('VTA_OPTIMIZE_DOCK_POS_V1') || '{}')) as { x?: number; y?: number }
    dispatchPointer(header as HTMLElement, 'pointerdown', { x: 16, y: 16 })
    dispatchPointer(header as HTMLElement, 'pointermove', { x: 76, y: 64 })
    dispatchPointer(header as HTMLElement, 'pointerup', { x: 76, y: 64 })
    await nextTick()
    await nextTick()

    const after = JSON.parse(String(localStorage.getItem('VTA_OPTIMIZE_DOCK_POS_V1') || '{}')) as { x?: number; y?: number }
    expect(typeof before.x).toBe('number')
    expect(typeof before.y).toBe('number')
    expect(typeof after.x).toBe('number')
    expect(typeof after.y).toBe('number')
    expect(Number(after.x)).toBeGreaterThan(Number(before.x))
    expect(Number(after.y)).toBeGreaterThan(Number(before.y))
  })
})
