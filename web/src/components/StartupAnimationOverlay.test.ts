// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { defineComponent, nextTick } from 'vue'
import StartupAnimationOverlay from './StartupAnimationOverlay.vue'
import AppRuntimeBanner from './AppRuntimeBanner.vue'
import { BOOT_STAGES, useRuntimeStore } from '../stores/runtimeStore'

const MIN_VISIBLE_MS = 360
const SIGNATURE_MS = 900
const EXIT_MS = 180
const SKIP_UNLOCK_MS = MIN_VISIBLE_MS

const StartupHarness = defineComponent({
  components: {
    AppRuntimeBanner,
    StartupAnimationOverlay,
  },
  template: `
    <div>
      <StartupAnimationOverlay />
      <AppRuntimeBanner />
    </div>
  `,
})

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
    runtimeStore.markBooting(BOOT_STAGES.RESTORING_WORKSPACE_STATE)

    const wrapper = mount(StartupAnimationOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect(document.querySelector('#startupAnimationOverlay')?.getAttribute('data-stage')).toBe('intro')
    expect(wrapper.text()).toContain('Restoring workspace state')
    expect(wrapper.find('.startupRail').exists()).toBe(true)
    expect(wrapper.find('.startupPhaseTag').text()).toBe('Booting')
    expect(wrapper.find('.startupGridField').exists()).toBe(false)
    expect(wrapper.find('.startupResultRail').exists()).toBe(false)

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
    runtimeStore.markBooting(BOOT_STAGES.INITIALIZING_DESKTOP_SERVICES)

    const wrapper = mount(StartupHarness, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    await flushTimers(SIGNATURE_MS)
    expect(document.querySelector('#startupAnimationOverlay')?.getAttribute('data-stage')).toBe('exiting')
    await flushTimers(EXIT_MS)
    expect(document.querySelector('#startupAnimationOverlay')).toBeNull()
    expect(runtimeStore.bootPhase).toBe('booting')
    expect(wrapper.find('.runtimeBanner').exists()).toBe(true)
    expect(wrapper.text()).toContain(BOOT_STAGES.INITIALIZING_DESKTOP_SERVICES)

    wrapper.unmount()
  })

  it('allows skipping after the minimum threshold', async () => {
    const runtimeStore = useRuntimeStore(pinia)
    runtimeStore.markBooting(BOOT_STAGES.RESTORING_WORKSPACE_STATE)

    const wrapper = mount(StartupAnimationOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect(wrapper.find('.startupSkipButton').exists()).toBe(false)
    await flushTimers(SKIP_UNLOCK_MS)
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
    runtimeStore.markBooting(BOOT_STAGES.INITIALIZING_DESKTOP_SERVICES)

    const wrapper = mount(StartupHarness, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    runtimeStore.markError(new Error('Desktop bridge failed'))
    await nextTick()

    expect(document.querySelector('#startupAnimationOverlay')?.getAttribute('data-stage')).toBe('exiting')
    await flushTimers(EXIT_MS)
    expect(document.querySelector('#startupAnimationOverlay')).toBeNull()
    expect(wrapper.find('.runtimeBanner.error').exists()).toBe(true)
    expect(wrapper.text()).toContain('Runtime Error')

    wrapper.unmount()
  })

  it('applies the reduced-motion posture when reduced motion is preferred', async () => {
    stubMatchMedia(true)
    const runtimeStore = useRuntimeStore(pinia)
    runtimeStore.markBooting(BOOT_STAGES.RESTORING_WORKSPACE_STATE)

    const wrapper = mount(StartupAnimationOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect(document.querySelector('#startupAnimationOverlay')?.classList.contains('reduced')).toBe(true)
    expect(wrapper.find('.startupMarkLogo').exists()).toBe(true)

    wrapper.unmount()
  })

  it('honors the debug hold flag until the operator skips', async () => {
    localStorage.setItem('VTA_STARTUP_OVERLAY_HOLD', '1')
    const runtimeStore = useRuntimeStore(pinia)
    runtimeStore.markBooting(BOOT_STAGES.RESTORING_WORKSPACE_STATE)

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
