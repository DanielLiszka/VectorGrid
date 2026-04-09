import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useWorkflowStore } from './workflowStore'

function ensureDocumentDataset(): Record<string, string> {
  const existing = (globalThis as { document?: { documentElement?: { dataset?: Record<string, string> } } }).document
  if (existing?.documentElement?.dataset) return existing.documentElement.dataset
  const dataset: Record<string, string> = {}
  vi.stubGlobal('document', { documentElement: { dataset } })
  return dataset
}

describe('workflow store actions', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    if (typeof localStorage !== 'undefined') localStorage.clear()
    const dataset = ensureDocumentDataset()
    for (const key of Object.keys(dataset)) {
      delete dataset[key]
    }
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

  it('selects the finalizer workflow step with meta results', () => {
    const store = useWorkflowStore()

    store.selectWorkflowStep('finalize')

    expect(store.workspace).toBe('workflow')
    expect(store.workflowStep).toBe('finalize')
    expect(store.workflow).toBe('finalize')
    expect(store.controlsTab).toBe('advanced')
    expect(store.resultsTab).toBe('meta')
  })

  it('keeps controls and results tab changes local to the current workflow', () => {
    const store = useWorkflowStore()
    store.selectWorkflowStep('optimize')
    store.selectResultsTab('top')
    expect(store.workflow).toBe('optimize')
    expect(store.workspace).toBe('workflow')
    expect(store.resultsTab).toBe('top')

    store.selectControlsTab('strategy')
    expect(store.workflow).toBe('optimize')
    expect(store.workspace).toBe('workflow')
    expect(store.controlsTab).toBe('strategy')
  })

  it('ignores tabs that are not allowed in the current workflow', () => {
    const store = useWorkflowStore()
    store.selectWorkflowStep('data')

    store.selectResultsTab('top')
    store.selectControlsTab('strategy')

    expect(store.resultsTab).toBe('overview')
    expect(store.controlsTab).toBe('data')
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

  it('stores shell split fraction as workflow layout state', () => {
    const store = useWorkflowStore()
    store.setSplitFraction(0.48, { persist: false })

    expect(store.splitFraction).toBe(0.48)
    expect(store.source).toBe('vue')
  })

  it('switches into MlSignal mode without reusing conventional optimize tabs', () => {
    const store = useWorkflowStore()
    store.selectWorkflowStep('optimize', { persist: false })
    store.selectControlsTab('strategy', { persist: false })
    store.selectResultsTab('top', { persist: false })

    store.selectMode('ml-signal', { persist: false })

    expect(store.mode).toBe('ml-signal')
    expect(store.workflow).toBe('optimize')
    expect(store.workflowStep).toBe('optimize')
    expect(store.controlsTab).toBe('opt')
    expect(store.resultsTab).toBe('overview')
    expect(store.workflowSubtitle).toContain('score cache')
  })

  it('keeps mode-specific tab memory separate between conventional and MlSignal', () => {
    const store = useWorkflowStore()
    store.selectWorkflowStep('optimize', { persist: false })
    store.selectControlsTab('strategy', { persist: false })
    store.selectResultsTab('top', { persist: false })
    store.selectMode('ml-signal', { persist: false })
    store.selectWorkflowStep('validate', { persist: false })
    store.selectWorkflowStep('optimize', { persist: false })
    expect(store.controlsTab).toBe('opt')
    expect(store.resultsTab).toBe('overview')

    store.selectMode('conventional', { persist: false })

    expect(store.controlsTab).toBe('strategy')
    expect(store.resultsTab).toBe('top')
  })

  it('ignores invalid mode changes and legacy mode snapshots', () => {
    const store = useWorkflowStore()
    store.selectMode('ml-signal', { persist: false })

    store.selectMode('invalid', { persist: false })
    expect(store.mode).toBe('ml-signal')

    store.applyLegacySnapshot({ mode: 'invalid' as never })
    expect(store.mode).toBe('ml-signal')
  })

  it('syncs runtime dataset attributes and subtitle from workflow posture changes', () => {
    const dataset = ensureDocumentDataset()
    const store = useWorkflowStore()

    store.selectWorkflowStep('optimize', { persist: false })
    store.selectControlsTab('strategy', { persist: false })
    store.selectResultsTab('top', { persist: false })
    store.setSplitFraction(0.52, { persist: false })

    expect(dataset.mode).toBe('conventional')
    expect(dataset.workspace).toBe('workflow')
    expect(dataset.workflow).toBe('optimize')
    expect(dataset.workflowStep).toBe('optimize')
    expect(dataset.controlsTab).toBe('strategy')
    expect(dataset.resultsTab).toBe('top')
    expect(dataset.aggregateTab).toBe('meta')
    expect(dataset.splitFraction).toBe('0.52')
    expect(dataset.workflowSubtitle).toBe(store.workflowSubtitle)
    expect(store.workflowSubtitle).toContain('Strategy setup')
  })

  it('infers settings and pro workspace posture from legacy tab snapshots', () => {
    const dataset = ensureDocumentDataset()
    const store = useWorkflowStore()

    store.applyLegacySnapshot({
      controlsTab: 'advanced',
      resultsTab: 'debug',
      splitFraction: 0.61,
    })

    expect(store.workspace).toBe('pro')
    expect(store.workflow).toBe('settings')
    expect(store.workflowStep).toBe('data')
    expect(store.controlsTab).toBe('advanced')
    expect(store.resultsTab).toBe('debug')
    expect(store.splitFraction).toBe(0.61)
    expect(dataset.workspace).toBe('pro')
    expect(dataset.workflow).toBe('settings')
    expect(dataset.resultsTab).toBe('debug')
    expect(dataset.workflowSubtitle).toBe(store.workflowSubtitle)
  })

  it('ignores invalid values', () => {
    const store = useWorkflowStore()
    const snapshot = {
      mode: store.mode,
      workspace: store.workspace,
      workflow: store.workflow,
      workflowStep: store.workflowStep,
      controlsTab: store.controlsTab,
      resultsTab: store.resultsTab,
      aggregateTab: store.aggregateTab,
    }

    store.selectMode('ml-signal', { persist: false })
    store.selectWorkspace('invalid')
    store.selectWorkflow('invalid')
    store.selectWorkflowStep('invalid')
    store.selectMode('invalid', { persist: false })
    store.selectControlsTab('invalid')
    store.selectResultsTab('invalid')
    store.selectAggregateTab('invalid')

    expect(store.mode).toBe('ml-signal')
    expect(store.workspace).toBe(snapshot.workspace)
    expect(store.workflow).toBe(snapshot.workflow)
    expect(store.workflowStep).toBe(snapshot.workflowStep)
    expect(store.controlsTab).toBe(snapshot.controlsTab)
    expect(store.resultsTab).toBe(snapshot.resultsTab)
    expect(store.aggregateTab).toBe(snapshot.aggregateTab)
  })
})
