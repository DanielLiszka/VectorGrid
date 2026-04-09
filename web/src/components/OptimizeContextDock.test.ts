// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it, vi } from 'vitest'
import OptimizeContextDock from './OptimizeContextDock.vue'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useOptimizeStore } from '../stores/optimizeStore'

describe('OptimizeContextDock', () => {
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

    expect(wrapper.text()).toContain('Optimize Context')
    expect(wrapper.text()).toContain('DoubleMa')
    expect(wrapper.text()).toContain('GpuKernel')
    expect(wrapper.text()).toContain('Ready')

    const buttons = wrapper.findAll('button')
    await buttons[0]?.trigger('click')
    await buttons[1]?.trigger('click')

    expect(runSpy).toHaveBeenCalledTimes(1)
    expect(validateSpy).toHaveBeenCalledTimes(1)
  })
})
