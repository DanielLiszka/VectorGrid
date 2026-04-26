// @vitest-environment jsdom

import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { useResearchCockpitModel } from './useResearchCockpitModel'

describe('useResearchCockpitModel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
  })

  it('describes an empty research cockpit without leaking undefined values', () => {
    const { model } = useResearchCockpitModel()

    expect(model.value.title).toBe('Research Cockpit')
    expect(model.value.cards.find((card) => card.id === 'dataset')).toMatchObject({
      value: 'No dataset',
      tone: 'warning',
    })
    expect(model.value.primaryAction.disabled).toBe(true)
    expect(JSON.stringify(model.value)).not.toContain('undefined')
  })

  it('updates readiness as data and optimization results become available', () => {
    const dataStore = useDataStore()
    const analysisStore = useAnalysisStore()
    const { model } = useResearchCockpitModel()

    dataStore.dataId = 'dataset-1'
    dataStore.datasetLabel = 'SPY'
    dataStore.dataTimeframe = '1m'
    dataStore.dataLen = '1000'
    dataStore.datasetRange = '2024-01-01 -> 2024-01-31'
    expect(model.value.steps.find((step) => step.id === 'data')?.state).toBe('active')
    expect(model.value.cards.find((card) => card.id === 'dataset')?.tone).toBe('success')

    analysisStore.applyLegacySnapshot({
      topRows: 8,
      bestTitle: 'Rank 1 Sharpe 2.1',
      bestMeta: '8 candidates',
    })
    expect(model.value.steps.find((step) => step.id === 'optimize')?.state).toBe('complete')
    expect(model.value.cards.find((card) => card.id === 'results')?.value).toContain('Rank 1')
  })

  it('routes cockpit step actions through workflow state', () => {
    const workflowStore = useWorkflowStore()
    const { openStep } = useResearchCockpitModel()

    openStep('configure')
    expect(workflowStore.workflowStep).toBe('optimize')
    expect(workflowStore.controlsTab).toBe('strategy')

    openStep('analyze')
    expect(workflowStore.workflowStep).toBe('analyze')
  })
})
