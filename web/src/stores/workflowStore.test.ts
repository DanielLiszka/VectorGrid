import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useWorkflowStore } from './workflowStore'

describe('workflow store actions', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('selects library workspace with history workflow defaults', () => {
    const store = useWorkflowStore()
    store.selectWorkspace('library')

    expect(store.workspace).toBe('library')
    expect(store.workflow).toBe('history')
    expect(store.controlsTab).toBe('data')
    expect(store.resultsTab).toBe('history')
    expect(store.source).toBe('vue')
  })

  it('selects workflow and applies workflow defaults', () => {
    const store = useWorkflowStore()
    store.selectWorkflow('validate')

    expect(store.workflow).toBe('validate')
    expect(store.workflowStep).toBe('validate')
    expect(store.workspace).toBe('workflow')
    expect(store.controlsTab).toBe('opt')
    expect(store.resultsTab).toBe('validate')
  })

  it('selects workflow step and forces workflow workspace with step tabs', () => {
    const store = useWorkflowStore()
    store.selectWorkspace('aggregate')

    store.selectWorkflowStep('analyze')

    expect(store.workspace).toBe('workflow')
    expect(store.workflowStep).toBe('analyze')
    expect(store.workflow).toBe('analyze')
    expect(store.resultsTab).toBe('drill')
  })

  it('infers workflow from tab selection changes', () => {
    const store = useWorkflowStore()
    store.selectResultsTab('validate')
    expect(store.workflow).toBe('validate')
    expect(store.workspace).toBe('workflow')

    store.selectResultsTab('history')
    expect(store.workflow).toBe('history')
    expect(store.workspace).toBe('library')

    store.selectResultsTab('meta')
    expect(store.workflow).toBe('settings')
    expect(store.workspace).toBe('aggregate')
  })

  it('moves into aggregate workspace when aggregate tab is selected', () => {
    const store = useWorkflowStore()
    store.selectAggregateTab('saved')
    expect(store.workspace).toBe('aggregate')
    expect(store.workflow).toBe('settings')
    expect(store.controlsTab).toBe('advanced')
    expect(store.resultsTab).toBe('meta')
    expect(store.aggregateTab).toBe('saved')
  })

  it('ignores invalid values', () => {
    const store = useWorkflowStore()
    const snapshot = {
      workspace: store.workspace,
      workflow: store.workflow,
      workflowStep: store.workflowStep,
      controlsTab: store.controlsTab,
      resultsTab: store.resultsTab,
      aggregateTab: store.aggregateTab,
    }

    store.selectWorkspace('invalid')
    store.selectWorkflow('invalid')
    store.selectWorkflowStep('invalid')
    store.selectControlsTab('invalid')
    store.selectResultsTab('invalid')
    store.selectAggregateTab('invalid')

    expect(store.workspace).toBe(snapshot.workspace)
    expect(store.workflow).toBe(snapshot.workflow)
    expect(store.workflowStep).toBe(snapshot.workflowStep)
    expect(store.controlsTab).toBe(snapshot.controlsTab)
    expect(store.resultsTab).toBe(snapshot.resultsTab)
    expect(store.aggregateTab).toBe(snapshot.aggregateTab)
  })
})
