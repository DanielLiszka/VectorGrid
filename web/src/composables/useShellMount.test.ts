import { mount } from '@vue/test-utils'
import { defineComponent, nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import { useShellMount } from './useShellMount'

const ShellMountHarness = defineComponent({
  setup() {
    return useShellMount('#shellMountHost', {
      paneSelector: '#shellPane',
      paneDatasetKey: 'vueHarness',
    })
  },
  template: `
    <section id="shellPane">
      <div v-if="mountReady" class="readyFlag">ready</div>
    </section>
  `,
})

async function flushShellMount(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('useShellMount', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
    localStorage.clear()
  })

  it('marks the pane ready and tags the pane root when the host exists', async () => {
    document.body.innerHTML = '<div id="shellMountHost"></div>'

    const wrapper = mount(ShellMountHarness, {
      attachTo: document.body,
    })

    await flushShellMount()

    expect(wrapper.find('.readyFlag').exists()).toBe(true)
    expect((document.querySelector('#shellPane') as HTMLElement | null)?.dataset.vueHarness).toBe('1')

    wrapper.unmount()

    expect((document.querySelector('#shellPane') as HTMLElement | null)?.dataset.vueHarness).toBeUndefined()
  })

  it('stays unready when the shell mount host is absent', async () => {
    const wrapper = mount(ShellMountHarness, {
      attachTo: document.body,
    })

    await flushShellMount()

    expect(wrapper.find('.readyFlag').exists()).toBe(false)
    expect((document.querySelector('#shellPane') as HTMLElement | null)?.dataset.vueHarness).toBeUndefined()
  })
})
