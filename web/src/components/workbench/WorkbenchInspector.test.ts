import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { WorkbenchInspectorState } from '../../utils/inspectorModel'
import WorkbenchInspector from './WorkbenchInspector.vue'

function state(): WorkbenchInspectorState {
  return {
    activeMode: 'selection',
    attention: false,
    modes: [
      { id: 'selection', label: 'Selection', tone: 'accent', current: true, actionId: 'workbench.inspector.mode.selection' },
      { id: 'validation', label: 'Validation', tone: 'neutral', current: false, actionId: 'workbench.inspector.mode.validation' },
      { id: 'history', label: 'History', tone: 'neutral', current: false, actionId: 'workbench.inspector.mode.history' },
      { id: 'tasks', label: 'Tasks', tone: 'neutral', current: false, actionId: 'workbench.inspector.mode.tasks' },
      { id: 'diagnostics', label: 'Diagnostics', tone: 'neutral', current: false, actionId: 'workbench.inspector.mode.diagnostics' },
    ],
    sections: [
      { id: 'dataset', title: 'Dataset', value: 'SPY', detail: '1m', tone: 'accent' },
    ],
    actions: [
      {
        id: 'selection.openOptimize',
        label: 'Open optimize',
        actionId: 'workbench.inspector.selection.openOptimize',
        disabled: false,
        tone: 'accent',
      },
    ],
  }
}

describe('WorkbenchInspector', () => {
  it('renders sections and emits action requests', async () => {
    const wrapper = mount(WorkbenchInspector, {
      props: {
        state: state(),
        collapsed: false,
      },
      attachTo: document.body,
    })

    expect(wrapper.get('[data-qa-state="workbench.inspector.section.dataset"]').text()).toContain('SPY')
    await wrapper.get('[data-qa-action="workbench.inspector.selection.openOptimize"]').trigger('click')
    expect(wrapper.emitted('runAction')?.[0]).toEqual(['selection.openOptimize'])

    wrapper.unmount()
  })

  it('supports roving keyboard selection between inspector modes', async () => {
    const wrapper = mount(WorkbenchInspector, {
      props: {
        state: state(),
        collapsed: false,
      },
      attachTo: document.body,
    })

    await wrapper.get('#workbench_inspector_mode_selection').trigger('keydown', { key: 'ArrowRight' })
    expect(wrapper.emitted('selectMode')?.[0]).toEqual(['validation'])

    wrapper.unmount()
  })
})
