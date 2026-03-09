import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useHistoryStore } from '../stores/historyStore'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'

export type UiReadinessStage = 'no_data' | 'data_loaded_no_run' | 'results_available_no_selection' | 'ready'

const PLACEHOLDER_VALUES = new Set(['', '-', '—'])

function hasMeaningfulText(value: unknown): boolean {
  const next = String(value || '').trim()
  return !PLACEHOLDER_VALUES.has(next)
}

function hasArrayValues<T>(value: T[] | readonly T[] | undefined | null): boolean {
  return Array.isArray(value) && value.length > 0
}

export function useUiReadiness() {
  const analysisStore = useAnalysisStore()
  const dataStore = useDataStore()
  const historyStore = useHistoryStore()
  const validationStore = useValidationStore()
  const workflowArtifactsStore = useWorkflowArtifactsStore()

  const {
    bestMeta,
    bestStats,
    bestTitle,
    metaCompareSelected,
    metaOutputPreview,
    metaTableRows,
    topRows,
  } = storeToRefs(analysisStore)
  const { dataId, datasetLabel } = storeToRefs(dataStore)
  const { savedRunCount, savedSelectedValue } = storeToRefs(historyStore)
  const { tableRowCount, tableVisible, validationOutputPreview } = storeToRefs(validationStore)
  const { currentRun } = storeToRefs(workflowArtifactsStore)

  const datasetReady = computed(() => {
    const id = String(dataId.value || '').trim()
    const label = String(datasetLabel.value || '').trim()
    return hasMeaningfulText(id) || hasMeaningfulText(label)
  })

  const hasOptimizationResults = computed(() => {
    if (currentRun.value?.res) return true
    if (Number(topRows.value || 0) > 0) return true
    if (hasArrayValues(bestStats.value)) return true
    return hasMeaningfulText(bestTitle.value) || hasMeaningfulText(bestMeta.value)
  })

  const hasValidationResults = computed(() => {
    if (Boolean(tableVisible.value)) return true
    if (Number(tableRowCount.value || 0) > 0) return true
    return hasMeaningfulText(validationOutputPreview.value)
  })

  const hasSavedRuns = computed(() => Number(savedRunCount.value || 0) > 0)
  const hasSavedRunSelection = computed(() => hasMeaningfulText(savedSelectedValue.value))
  const hasMetaArtifacts = computed(() => hasArrayValues(metaTableRows.value) || hasMeaningfulText(metaOutputPreview.value))
  const hasMetaComparisonSelection = computed(() => hasMeaningfulText(metaCompareSelected.value))

  const overviewStage = computed<UiReadinessStage>(() => {
    if (!datasetReady.value) return 'no_data'
    if (!hasOptimizationResults.value) return 'data_loaded_no_run'
    return 'ready'
  })

  const topStage = computed<UiReadinessStage>(() => {
    if (!datasetReady.value) return 'no_data'
    if (!hasOptimizationResults.value) return 'data_loaded_no_run'
    return 'ready'
  })

  const paretoStage = computed<UiReadinessStage>(() => {
    if (!datasetReady.value) return 'no_data'
    if (!hasOptimizationResults.value) return 'data_loaded_no_run'
    return 'ready'
  })

  const validationStage = computed<UiReadinessStage>(() => {
    if (!datasetReady.value) return 'no_data'
    if (!hasOptimizationResults.value) return 'data_loaded_no_run'
    if (!hasValidationResults.value) return 'results_available_no_selection'
    return 'ready'
  })

  const historyStage = computed<UiReadinessStage>(() => {
    if (!hasSavedRuns.value) return 'no_data'
    if (!hasSavedRunSelection.value) return 'results_available_no_selection'
    return 'ready'
  })

  const metaStage = computed<UiReadinessStage>(() => {
    if (!hasSavedRuns.value) return 'no_data'
    if (!hasMetaArtifacts.value) return 'data_loaded_no_run'
    return 'ready'
  })

  const metaCompareStage = computed<UiReadinessStage>(() => {
    if (metaStage.value !== 'ready') return metaStage.value
    if (!hasMetaComparisonSelection.value) return 'results_available_no_selection'
    return 'ready'
  })

  return {
    datasetReady,
    hasOptimizationResults,
    hasValidationResults,
    hasSavedRuns,
    hasSavedRunSelection,
    hasMetaArtifacts,
    hasMetaComparisonSelection,
    overviewStage,
    topStage,
    paretoStage,
    validationStage,
    historyStage,
    metaStage,
    metaCompareStage,
  }
}
