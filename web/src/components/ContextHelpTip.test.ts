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
    PopoverContent: passthrough('PopoverContent'),
    PopoverPortal: passthrough('PopoverPortal'),
    PopoverRoot: passthrough('PopoverRoot'),
    PopoverTrigger: passthrough('PopoverTrigger'),
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
    const trigger = wrapper.get('button.contextHelpTrigger')
    const content = wrapper.findComponent({ name: 'PopoverContent' })
    const title = wrapper.get('.contextHelpTitle')
    const summary = wrapper.get('.contextHelpSummary')
    const detail = wrapper.get('.contextHelpDetail')
    expect(trigger.attributes('aria-expanded')).toBe('false')
    expect(trigger.attributes('aria-controls')).toBe(content.attributes('id'))
    expect(content.attributes('role')).toBe('dialog')
    expect(content.attributes('aria-modal')).toBe('false')
    expect(content.attributes('aria-labelledby')).toBe(title.attributes('id'))
    expect(content.attributes('aria-describedby')).toBe(
      `${summary.attributes('id')} ${detail.attributes('id')}`,
    )
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
    expect(wrapper.findComponent({ name: 'TooltipRoot' }).attributes('delay-duration')).toBe('450')
    const trigger = wrapper.get('button.contextHelpTrigger')
    const tooltip = wrapper.findComponent({ name: 'TooltipContent' })
    expect(trigger.attributes('aria-describedby')).toBe(tooltip.attributes('id'))
    expect(tooltip.attributes('role')).toBe('tooltip')
    expect(wrapper.text()).toContain('Local search only.')
    expect(wrapper.text()).not.toContain('Use Ctrl+K for the global command palette.')

    settingsStore.helpMode = 'off'
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.contextHelpRoot').exists()).toBe(false)
  })
})
