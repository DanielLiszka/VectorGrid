// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it } from 'vitest'
import ValidationContextDock from './ValidationContextDock.vue'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowStore } from '../stores/workflowStore'

describe('ValidationContextDock', () => {
  it('renders validation summary state and opens the validation workspace action', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const validationStore = useValidationStore(pinia)
    validationStore.applyLegacySnapshot({
      validationMeta: '32 candidates',
      validationOutputPreview: 'report.xlsx',
      crossMeta: 'selected 2/6',
      tableRowCount: 12,
      minPsr: '0.80',
      minDsr: '0.60',
      cscvSlices: '16',
      statusLine: 'Ready.',
    })

    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflowStep('optimize', { persist: false })

    const wrapper = mount(ValidationContextDock, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect(wrapper.text()).toContain('Validation Context')
    expect(wrapper.text()).toContain('32 candidates')
    expect(wrapper.text()).toContain('PSR≥0.80')
    expect(wrapper.text()).toContain('DSR≥0.60')
    expect(wrapper.text()).toContain('CSCV=16')

    await wrapper.get('button').trigger('click')
    expect(workflowStore.workflowStep).toBe('validate')
  })
})
