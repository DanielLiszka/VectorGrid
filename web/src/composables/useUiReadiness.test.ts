import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useUiReadiness } from './useUiReadiness'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useHistoryStore } from '../stores/historyStore'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'

describe('useUiReadiness', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('starts in a no-data state across the shared result surfaces', () => {
    const readiness = useUiReadiness()

    expect(readiness.overviewStage.value).toBe('no_data')
    expect(readiness.topStage.value).toBe('no_data')
    expect(readiness.paretoStage.value).toBe('no_data')
    expect(readiness.validationStage.value).toBe('no_data')
    expect(readiness.historyStage.value).toBe('no_data')
    expect(readiness.metaStage.value).toBe('no_data')
  })

  it('distinguishes loaded-data/no-run from result-available/no-selection states', () => {
    const dataStore = useDataStore()
    const analysisStore = useAnalysisStore()
    const validationStore = useValidationStore()
    const historyStore = useHistoryStore()
    const readiness = useUiReadiness()

    dataStore.dataId = 'dataset_1'
    expect(readiness.overviewStage.value).toBe('data_loaded_no_run')
    expect(readiness.topStage.value).toBe('data_loaded_no_run')
    expect(readiness.paretoStage.value).toBe('data_loaded_no_run')
    expect(readiness.validationStage.value).toBe('data_loaded_no_run')

    analysisStore.bestTitle = 'Rank #1 · Winner'
    expect(readiness.overviewStage.value).toBe('ready')
    expect(readiness.topStage.value).toBe('ready')
    expect(readiness.paretoStage.value).toBe('ready')
    expect(readiness.validationStage.value).toBe('results_available_no_selection')

    validationStore.tableVisible = true
    validationStore.tableRowCount = 2
    expect(readiness.validationStage.value).toBe('ready')

    historyStore.savedRunCount = 2
    expect(readiness.historyStage.value).toBe('results_available_no_selection')
    historyStore.savedSelectedValue = 'C:/runs/demo.json'
    expect(readiness.historyStage.value).toBe('ready')
  })

  it('tracks meta artifact readiness and compare selection separately', () => {
    const historyStore = useHistoryStore()
    const analysisStore = useAnalysisStore()
    const workflowArtifactsStore = useWorkflowArtifactsStore()
    const readiness = useUiReadiness()

    historyStore.savedRunCount = 1
    expect(readiness.metaStage.value).toBe('data_loaded_no_run')

    analysisStore.metaOutputPreview = '{"summary":"ok"}'
    expect(readiness.metaStage.value).toBe('ready')
    expect(readiness.metaCompareStage.value).toBe('results_available_no_selection')

    analysisStore.metaCompareSelected = 'baseline_1'
    expect(readiness.metaCompareStage.value).toBe('ready')

    workflowArtifactsStore.clearArtifacts()
  })
})
