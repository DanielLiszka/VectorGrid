import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ContextHelpTip from './ContextHelpTip.vue'
import { useSettingsStore } from '../stores/settingsStore'

vi.mock('reka-ui', () => {
  const passthrough = (name: string) => ({
    name,
    template: '<div><slot /></div>',
  })
  return {
    HoverCardContent: passthrough('HoverCardContent'),
    HoverCardPortal: passthrough('HoverCardPortal'),
    HoverCardRoot: passthrough('HoverCardRoot'),
    HoverCardTrigger: passthrough('HoverCardTrigger'),
    TooltipContent: passthrough('TooltipContent'),
    TooltipPortal: passthrough('TooltipPortal'),
    TooltipProvider: passthrough('TooltipProvider'),
    TooltipRoot: passthrough('TooltipRoot'),
    TooltipTrigger: passthrough('TooltipTrigger'),
  }
})

describe('ContextHelpTip', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('renders full help mode with summary and detail copy', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const settingsStore = useSettingsStore(pinia)
    settingsStore.helpMode = 'full'

    const wrapper = mount(ContextHelpTip, {
      props: {
        title: 'Scope',
        summary: 'Local search only.',
        detail: 'Use Ctrl+K for the global command palette.',
      },
      global: { plugins: [pinia] },
    })

    expect(wrapper.attributes('data-help-mode')).toBe('full')
    expect(wrapper.findComponent({ name: 'HoverCardRoot' }).attributes('open-delay')).toBe('1600')
    expect(wrapper.text()).toContain('Scope')
    expect(wrapper.text()).toContain('Local search only.')
    expect(wrapper.text()).toContain('Use Ctrl+K for the global command palette.')
  })

  it('renders concise help mode without the detailed body and hides output when help is off', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const settingsStore = useSettingsStore(pinia)
    settingsStore.helpMode = 'concise'

    const wrapper = mount(ContextHelpTip, {
      props: {
        title: 'Scope',
        summary: 'Local search only.',
        detail: 'Use Ctrl+K for the global command palette.',
      },
      global: { plugins: [pinia] },
    })

    expect(wrapper.attributes('data-help-mode')).toBe('concise')
    expect(wrapper.findComponent({ name: 'TooltipRoot' }).attributes('delay-duration')).toBe('750')
    expect(wrapper.text()).toContain('Local search only.')
    expect(wrapper.text()).not.toContain('Use Ctrl+K for the global command palette.')

    settingsStore.helpMode = 'off'
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.contextHelpRoot').exists()).toBe(false)
  })
})
