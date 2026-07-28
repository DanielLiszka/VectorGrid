import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import { mount } from '@vue/test-utils'

vi.mock('../services/workflowBootstrap', () => ({
  retryLatestWorkflowArtifactHydration: vi.fn(async () => true),
}))

import AppRuntimeBanner from './AppRuntimeBanner.vue'
import { retryLatestWorkflowArtifactHydration } from '../services/workflowBootstrap'
import { BOOT_STAGES, useRuntimeStore } from '../stores/runtimeStore'

describe('AppRuntimeBanner', () => {
  let pinia: Pinia

  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    vi.clearAllMocks()
  })

  it('renders the current boot stage while booting', () => {
    const store = useRuntimeStore()
    store.markBooting(BOOT_STAGES.INITIALIZING_DESKTOP_SERVICES)

    const wrapper = mount(AppRuntimeBanner, {
      global: { plugins: [pinia] },
    })

    expect(wrapper.find('.runtimeBanner.booting').exists()).toBe(true)
    expect(wrapper.text()).toContain('Booting')
    expect(wrapper.text()).toContain(BOOT_STAGES.INITIALIZING_DESKTOP_SERVICES)
    expect(wrapper.find('[role="status"]').attributes('aria-live')).toBe('polite')
  })

  it('renders the error banner and stack-trace details on fatal boot errors', () => {
    const store = useRuntimeStore()
    const error = new Error('boot exploded')
    error.stack = 'Error: boot exploded\n    at runtime'
    store.markError(error)

    const wrapper = mount(AppRuntimeBanner, {
      global: { plugins: [pinia] },
    })

    expect(wrapper.find('.runtimeBanner.error').exists()).toBe(true)
    expect(wrapper.text()).toContain('Runtime Error')
    expect(wrapper.text()).toContain('boot exploded')
    expect(wrapper.find('.runtimeErrorTrace').exists()).toBe(true)
    expect(wrapper.find('[role="status"]').attributes('aria-live')).toBe('assertive')
  })

  it('stays hidden once the runtime is ready', () => {
    const store = useRuntimeStore()
    store.markBooting(BOOT_STAGES.MOUNTING_WORKSPACE_SHELL)
    store.markReady()

    const wrapper = mount(AppRuntimeBanner, {
      global: { plugins: [pinia] },
    })

    expect(wrapper.find('.runtimeBanner').exists()).toBe(false)
    expect(wrapper.text()).toBe('')
  })

  it('does not keep the banner visible for recoverable desktop warnings after ready', () => {
    const store = useRuntimeStore()
    store.markBooting(BOOT_STAGES.INITIALIZING_DESKTOP_SERVICES)
    store.pushDesktopLog('warn', 'Desktop startup service failed: simulated warning')
    store.markReady()

    const wrapper = mount(AppRuntimeBanner, {
      global: { plugins: [pinia] },
    })

    expect(wrapper.find('.runtimeBanner').exists()).toBe(false)
    expect(store.desktopLogs[0]?.message).toContain('simulated warning')
  })

  it('keeps the ready shell available while offering an actionable latest-artifact retry', async () => {
    const store = useRuntimeStore()
    store.markBooting(BOOT_STAGES.MOUNTING_WORKSPACE_SHELL)
    store.markReady()
    store.markLatestArtifactHydrationStarted()
    store.markLatestArtifactHydrationError(new Error('artifact store unavailable'))

    const wrapper = mount(AppRuntimeBanner, {
      global: { plugins: [pinia] },
    })

    expect(store.bootPhase).toBe('ready')
    expect(wrapper.find('.runtimeBanner.restoreWarning').exists()).toBe(true)
    expect(wrapper.text()).toContain('Latest saved artifact could not be restored')
    expect(wrapper.text()).toContain('The workspace remains usable')
    const retry = wrapper.get('button.runtimeAction')
    expect(retry.text()).toContain('Retry latest artifact')

    await retry.trigger('click')

    expect(vi.mocked(retryLatestWorkflowArtifactHydration)).toHaveBeenCalledTimes(1)
  })
})
