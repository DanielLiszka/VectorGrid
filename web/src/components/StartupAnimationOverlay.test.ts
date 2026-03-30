// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import StartupAnimationOverlay from './StartupAnimationOverlay.vue'
import { useRuntimeStore } from '../stores/runtimeStore'

const MIN_VISIBLE_MS = 360
const SIGNATURE_MS = 1400
const EXIT_MS = 180

function stubMatchMedia(reducedMotion = false): void {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: reducedMotion && query === '(prefers-reduced-motion: reduce)',
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  })
}

async function flushTimers(ms = 0): Promise<void> {
  vi.advanceTimersByTime(ms)
  await nextTick()
  await nextTick()
}

describe('StartupAnimationOverlay', () => {
  let pinia: Pinia

  beforeEach(() => {
    vi.useFakeTimers()
    localStorage.clear()
    document.body.innerHTML = ''
    stubMatchMedia(false)
    pinia = createPinia()
    setActivePinia(pinia)
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  it('shows during boot and exits after readiness once minimum visibility elapses', async () => {
    const runtimeStore = useRuntimeStore(pinia)
    runtimeStore.markBooting('Restoring workspace state')

    const wrapper = mount(StartupAnimationOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect(document.querySelector('#startupAnimationOverlay')?.getAttribute('data-stage')).toBe('intro')
    expect(wrapper.text()).toContain('Restoring workspace state')

    runtimeStore.markReady()
    await nextTick()

    expect(document.querySelector('#startupAnimationOverlay')?.getAttribute('data-stage')).toBe('intro')
    await flushTimers(MIN_VISIBLE_MS - 1)
    expect(document.querySelector('#startupAnimationOverlay')?.getAttribute('data-stage')).toBe('intro')
    await flushTimers(1)
    expect(document.querySelector('#startupAnimationOverlay')?.getAttribute('data-stage')).toBe('exiting')
    await flushTimers(EXIT_MS)
    expect(document.querySelector('#startupAnimationOverlay')).toBeNull()

    wrapper.unmount()
  })

  it('hands off after the signature duration if startup is still booting', async () => {
    const runtimeStore = useRuntimeStore(pinia)
    runtimeStore.markBooting('Initializing desktop services')

    const wrapper = mount(StartupAnimationOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    await flushTimers(SIGNATURE_MS)
    expect(document.querySelector('#startupAnimationOverlay')?.getAttribute('data-stage')).toBe('exiting')
    await flushTimers(EXIT_MS)
    expect(document.querySelector('#startupAnimationOverlay')).toBeNull()
    expect(runtimeStore.bootPhase).toBe('booting')

    wrapper.unmount()
  })

  it('allows skipping after the minimum threshold', async () => {
    const runtimeStore = useRuntimeStore(pinia)
    runtimeStore.markBooting('Restoring workspace state')

    const wrapper = mount(StartupAnimationOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect(wrapper.find('.startupSkipButton').exists()).toBe(false)
    await flushTimers(MIN_VISIBLE_MS)
    expect(wrapper.find('.startupSkipButton').exists()).toBe(true)

    await wrapper.find('.startupSkipButton').trigger('click')
    await nextTick()
    expect(document.querySelector('#startupAnimationOverlay')?.getAttribute('data-stage')).toBe('exiting')
    await flushTimers(EXIT_MS)
    expect(document.querySelector('#startupAnimationOverlay')).toBeNull()

    wrapper.unmount()
  })

  it('exits quickly if startup errors', async () => {
    const runtimeStore = useRuntimeStore(pinia)
    runtimeStore.markBooting('Initializing desktop services')

    const wrapper = mount(StartupAnimationOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    runtimeStore.markError(new Error('Desktop bridge failed'))
    await nextTick()

    expect(document.querySelector('#startupAnimationOverlay')?.getAttribute('data-stage')).toBe('exiting')
    await flushTimers(EXIT_MS)
    expect(document.querySelector('#startupAnimationOverlay')).toBeNull()

    wrapper.unmount()
  })

  it('switches to the static mark when reduced motion is preferred', async () => {
    stubMatchMedia(true)
    const runtimeStore = useRuntimeStore(pinia)
    runtimeStore.markBooting('Restoring workspace state')

    const wrapper = mount(StartupAnimationOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect(document.querySelector('.startupLogoStatic')).not.toBeNull()
    expect(document.querySelector('.startupLogoAnimated')).toBeNull()

    wrapper.unmount()
  })

  it('honors the debug hold flag until the operator skips', async () => {
    localStorage.setItem('VTA_STARTUP_OVERLAY_HOLD', '1')
    const runtimeStore = useRuntimeStore(pinia)
    runtimeStore.markBooting('Restoring workspace state')

    const wrapper = mount(StartupAnimationOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    runtimeStore.markReady()
    await nextTick()
    await flushTimers(SIGNATURE_MS + MIN_VISIBLE_MS + EXIT_MS)

    expect(document.querySelector('#startupAnimationOverlay')?.getAttribute('data-stage')).toBe('intro')
    expect(wrapper.find('.startupSkipButton').exists()).toBe(true)

    await wrapper.find('.startupSkipButton').trigger('click')
    await nextTick()
    await flushTimers(EXIT_MS)
    expect(document.querySelector('#startupAnimationOverlay')).toBeNull()

    wrapper.unmount()
  })
})
