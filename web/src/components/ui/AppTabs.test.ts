
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it, vi } from 'vitest'
import AppTabs from './AppTabs.vue'

describe('AppTabs', () => {
  it('emits model updates for clicks and keyboard movement', async () => {
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      callback(0)
      return 1
    })
    const wrapper = mount(AppTabs, {
      props: {
        idBase: 'test_tab',
        ariaLabel: 'Test tabs',
        modelValue: 'a',
        items: [
          { key: 'a', label: 'Alpha', panelId: 'panel-a' },
          { key: 'b', label: 'Beta', panelId: 'panel-b' },
        ],
      },
      attachTo: document.body,
    })

    await wrapper.get('#test_tab_b').trigger('click')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['b'])
    expect(wrapper.get('#test_tab_b').attributes('aria-controls')).toBe('panel-b')

    await wrapper.setProps({ modelValue: 'a' })
    wrapper.get('#test_tab_a').element.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }))
    await nextTick()

    expect(wrapper.emitted('update:modelValue')?.[1]).toEqual(['b'])
    expect(document.activeElement).toBe(document.getElementById('test_tab_b'))

    wrapper.unmount()
    vi.unstubAllGlobals()
  })

  it('wraps across enabled tabs and skips disabled tabs', async () => {
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      callback(0)
      return 1
    })
    const wrapper = mount(AppTabs, {
      props: {
        idBase: 'wrap_tab',
        ariaLabel: 'Wrapped tabs',
        modelValue: 'c',
        items: [
          { key: 'a', label: 'Alpha' },
          { key: 'b', label: 'Beta', disabled: true },
          { key: 'c', label: 'Gamma' },
        ],
      },
      attachTo: document.body,
    })

    wrapper.get('#wrap_tab_c').element.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }))
    await nextTick()
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['a'])

    await wrapper.setProps({ modelValue: 'a' })
    wrapper.get('#wrap_tab_a').element.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }))
    await nextTick()
    expect(wrapper.emitted('update:modelValue')?.[1]).toEqual(['c'])

    wrapper.unmount()
    vi.unstubAllGlobals()
  })
})
