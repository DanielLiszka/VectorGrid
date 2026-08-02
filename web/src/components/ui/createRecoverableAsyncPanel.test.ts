import { defineComponent } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { createRecoverableAsyncPanel } from './createRecoverableAsyncPanel'

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise
    reject = rejectPromise
  })
  return { promise, reject, resolve }
}

const LoadedPanel = defineComponent({
  name: 'LoadedPanel',
  inheritAttrs: false,
  template: '<div data-testid="loaded-panel"><slot /></div>',
})

describe('createRecoverableAsyncPanel', () => {
  it('exposes an accessible loading state while a deferred panel is pending', async () => {
    const pendingPanel = deferred<{ default: typeof LoadedPanel }>()
    const RecoverablePanel = createRecoverableAsyncPanel({
      label: 'Data workspace',
      loader: () => pendingPanel.promise,
      loadingDelayMs: 0,
    })
    const wrapper = mount(RecoverablePanel, {
      slots: { default: 'Loaded content' },
    })

    const status = wrapper.get('[role="status"]')
    expect(status.attributes('aria-live')).toBe('polite')
    expect(status.attributes('aria-busy')).toBe('true')
    expect(status.text()).toContain('Loading Data workspace')

    pendingPanel.resolve({ default: LoadedPanel })
    await flushPromises()

    expect(wrapper.find('[role="status"]').exists()).toBe(false)
    expect(wrapper.get('[data-testid="loaded-panel"]').text()).toBe('Loaded content')
  })

  it('offers an accessible retry after a rejected chunk and recovers in place', async () => {
    const loader = vi
      .fn<() => Promise<{ default: typeof LoadedPanel }>>()
      .mockRejectedValueOnce(new Error('Failed to fetch dynamically imported module'))
      .mockResolvedValueOnce({ default: LoadedPanel })
    const RecoverablePanel = createRecoverableAsyncPanel({
      label: 'Validation workspace',
      loader,
      loadingDelayMs: 0,
    })
    const wrapper = mount(RecoverablePanel)

    await flushPromises()

    const alert = wrapper.get('[role="alert"]')
    expect(alert.attributes('aria-live')).toBe('assertive')
    expect(alert.text()).toContain('Validation workspace did not load')
    expect(alert.text()).toContain('Your current workflow is unchanged')

    await wrapper.get('button[aria-label="Retry loading Validation workspace"]').trigger('click')
    await flushPromises()

    expect(loader).toHaveBeenCalledTimes(2)
    expect(wrapper.find('[role="alert"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="loaded-panel"]').exists()).toBe(true)
  })
})
