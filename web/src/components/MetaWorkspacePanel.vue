<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { type Cell, type Header } from '@tanstack/vue-table'
import {
  useHeadlessMatrixTable,
  type MatrixColumnMeta,
} from '../composables/useHeadlessMatrixTable'
import { usePersistedMatrixColumnState } from '../composables/usePersistedMatrixColumnState'
import { useShellMount } from '../composables/useShellMount'
import { useUiReadiness } from '../composables/useUiReadiness'
import { useVirtualTableRows } from '../composables/useVirtualTableRows'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import ContextHelpTip from './ContextHelpTip.vue'
import ContextActionMenu from './ContextActionMenu.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { META_RESULTS_TABLE_STATE_KEY } from '../utils/core/storageKeys'
import { artifactActionIcon, type AppIcon } from '../utils/iconography'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

interface OptionItem {
  value: string
  label: string
  disabled: boolean
}

interface MetaColumn {
  label: string
  alignLeft: boolean
}

interface MetaCell {
  text: string
  className: string
  title: string
}

interface MetaRow {
  cells: MetaCell[]
}

interface BatchDatasetItem {
  path: string
  label: string
  meta: string
  checked: boolean
  disabled: boolean
}

interface ContextActionMenuItem {
  key: string
  label: string
  icon?: AppIcon
  disabled: boolean
  destructive?: boolean
  separatorBefore?: boolean
}

const aggregateTabItems = [
  { key: 'meta', label: 'Meta Selection' },
  { key: 'batch', label: 'Batch Generation' },
  { key: 'saved', label: 'Saved Meta' },
  { key: 'compare', label: 'Compare' },
] as const

type AggregateTabKey = (typeof aggregateTabItems)[number]['key']

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const { metaCompareStage, metaStage } = useUiReadiness()
const {
  metaStrategyKind,
  metaStrategyOptions,
  metaTimeframe,
  metaRequiredTag,
  metaMinSamples,
  metaTopK,
  metaHoldoutTag,
  metaEvalHoldout,
  metaSaveHoldoutRows,
  metaHoldoutMode,
  metaHoldoutReps,
  metaHoldoutEvalTopN,
  metaHoldoutEvalRankBy,
  metaHoldoutExploreN,
  metaGroupingMode,
  metaClusterIntBucket,
  metaClusterFloatBucket,
  metaHoldoutNote,
  aggregateSummarySavedRuns,
  aggregateSummarySavedMeta,
  aggregateSummaryLatestGroups,
  aggregateSummaryLatestArtifacts,
  aggregateSummaryLatestHoldout,
  aggregateSummaryLatestRuntime,
  metaMeta,
  metaOutputPreview,
  metaTableColumns,
  metaTableRows,
  metaCompareOptions,
  metaCompareSelected,
  metaCompareSummary,
  batchDatasetSearch,
  batchTags,
  batchHoldoutPct,
  batchHoldoutTag,
  batchExecutionMode,
  batchPreloadPolicy,
  batchAutoMeta,
  batchAutoStage5,
  batchSkipDone,
  batchDatasets,
  batchSelectedCount,
  batchTotalCount,
  batchMeta,
  batchOutputPreview,
  canRunBatch,
  canBatchStop,
  canBatchRefresh,
  canBatchSelectAll,
  canBatchSelectNone,
  canBatchRetryFailed,
  canBatchResumeIncomplete,
  canBatchRevealSummary,
  metaSavedSearch,
  metaSavedOptions,
  metaSavedSelected,
  metaSavedTagsDraft,
  metaSavedMeta,
  canSaveMetaSavedTags,
  canRefreshMetaSaved,
  canLoadMetaSaved,
  canOpenMetaSavedReport,
  canRevealMetaSaved,
  canDeleteMetaSaved,
  stage4SavedSearch,
  stage4SavedOptions,
  stage4SavedSelected,
  stage4SavedTagsDraft,
  stage4SavedMeta,
  canSaveStage4SavedTags,
  canRefreshStage4Saved,
  canLoadStage4Saved,
  canRevealStage4Saved,
  canDeleteStage4Saved,
  policyEvalSavedSearch,
  policyEvalSavedOptions,
  policyEvalSavedSelected,
  policyEvalSavedTagsDraft,
  policyEvalSavedMeta,
  canSavePolicyEvalSavedTags,
  canRefreshPolicyEvalSaved,
  canLoadPolicyEvalSaved,
  canRevealPolicyEvalSaved,
  canDeletePolicyEvalSaved,
  metaUseMlConfidence,
  metaMlAutopilot,
  canMetaMlRefresh,
  canMetaMlTrain,
  canMetaMlApply,
  canMetaMlDelete,
  metaMlStatus,
  metaStage4TopK,
  metaStage4MinConf,
  metaStage4MetaWeight,
  metaStage4MlWeight,
  metaStage4UncertaintyWeight,
  metaStage4AcceptConf,
  metaStage4ReviewConf,
  metaStage4MaxAcceptUncertainty,
  metaStage4AutoTrain,
  metaStage4RequireHealthy,
  metaStage4FallbackToMeta,
  canRunMetaStage4,
  canRunMetaPolicyEval,
  canRevealMetaStage4,
  metaStage4Status,
  metaStage4OutputPreview,
  metaPolicyEvalStatus,
  metaPolicyEvalOutputPreview,
  canRunMeta,
  canOpenMetaReport,
  canRevealMetaFiles,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { aggregateTab, resultsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#metaVueShellMount', {
  paneSelector: '[data-vue-panel="meta"]',
  paneDatasetKey: 'vueMeta',
})

const strategyDraft = ref('CompositeMo')
const timeframeDraft = ref('')
const requiredTagDraft = ref('')
const minSamplesDraft = ref('30')
const topKDraft = ref('25')
const holdoutTagDraft = ref('holdout')
const holdoutEvalDraft = ref(true)
const holdoutSaveRowsDraft = ref(false)
const holdoutModeDraft = ref('light')
const holdoutRepsDraft = ref('2')
const holdoutEvalTopNDraft = ref('0')
const holdoutEvalRankByDraft = ref('auto')
const holdoutExploreNDraft = ref('0')
const groupingModeDraft = ref('clustered')
const clusterIntBucketDraft = ref('5')
const clusterFloatBucketDraft = ref('0.25')
const compareDraft = ref('')
const batchSearchDraft = ref('')
const batchTagsDraft = ref('')
const batchHoldoutPctDraft = ref('15')
const batchHoldoutTagDraft = ref('holdout')
const batchExecutionModeDraft = ref('full')
const batchPreloadPolicyDraft = ref('host_only')
const batchAutoMetaDraft = ref(true)
const batchAutoStage5Draft = ref(false)
const batchSkipDoneDraft = ref(true)
const metaSavedSearchDraft = ref('')
const metaSavedSelectDraft = ref('')
const metaSavedTagsLocalDraft = ref('')
const stage4SavedSearchDraft = ref('')
const stage4SavedSelectDraft = ref('')
const stage4SavedTagsLocalDraft = ref('')
const policyEvalSavedSearchDraft = ref('')
const policyEvalSavedSelectDraft = ref('')
const policyEvalSavedTagsLocalDraft = ref('')
const metaUseMlConfidenceDraft = ref(false)
const metaMlAutopilotDraft = ref(true)
const metaStage4TopKDraft = ref('10')
const metaStage4MinConfDraft = ref('0.55')
const metaStage4MetaWeightDraft = ref('0.60')
const metaStage4MlWeightDraft = ref('0.35')
const metaStage4UncertaintyWeightDraft = ref('0.05')
const metaStage4AcceptConfDraft = ref('0.70')
const metaStage4ReviewConfDraft = ref('0.55')
const metaStage4MaxAcceptUncertaintyDraft = ref('0.45')
const metaStage4AutoTrainDraft = ref(true)
const metaStage4RequireHealthyDraft = ref(true)
const metaStage4FallbackToMetaDraft = ref(true)

const visible = computed(() => String(resultsTab.value || '').trim() === 'meta')
const activeAggregateTab = computed(() => String(aggregateTab.value || '').trim() || 'meta')
const metaTabActive = computed(() => activeAggregateTab.value === 'meta')
const compareTabActive = computed(() => activeAggregateTab.value === 'compare')
const showMetaTooling = computed(() => metaStage.value !== 'no_data')
const renderedColumns = computed<MetaColumn[]>(() => metaTableColumns.value as MetaColumn[])
const renderedRows = computed<MetaRow[]>(() => metaTableRows.value as MetaRow[])
const {
  columnOptions,
  columnVisibility,
  visibleCount: visibleColumnCount,
  hiddenCount: hiddenColumnCount,
  canRestoreDefaults,
  toggleColumnVisibility,
  restoreDefaultColumns,
  canHideColumn,
} = usePersistedMatrixColumnState(renderedColumns, META_RESULTS_TABLE_STATE_KEY)
const { headerGroups, rowModelRows, visibleLeafColumns } = useHeadlessMatrixTable(
  renderedColumns,
  renderedRows,
  { columnVisibility },
)
const renderedBatchDatasets = computed<BatchDatasetItem[]>(() => batchDatasets.value as BatchDatasetItem[])
const {
  scrollElementRef: tableScrollElementRef,
  virtualRows,
  topSpacerPx: virtualTopSpacerPx,
  bottomSpacerPx: virtualBottomSpacerPx,
  scrollToTop,
} = useVirtualTableRows(rowModelRows)

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

const activeAggregateTabLabel = computed(
  () => aggregateTabItems.find((item) => item.key === activeAggregateTab.value)?.label || 'Meta Selection',
)

const metaStageLabel = computed(() => {
  if (metaStage.value === 'no_data') return 'Awaiting saved runs'
  if (metaStage.value === 'data_loaded_no_run') return 'Ready for meta run'
  if (compareTabActive.value && metaCompareStage.value === 'results_available_no_selection') return 'Awaiting compare baseline'
  return 'Results available'
})

const metaSummaryChips = computed(() => {
  return [
    metaStageLabel.value,
    activeAggregateTabLabel.value,
    String(aggregateSummarySavedRuns.value || '').trim() ? `Runs ${aggregateSummarySavedRuns.value}` : '',
    String(aggregateSummarySavedMeta.value || '').trim() ? `Meta ${aggregateSummarySavedMeta.value}` : '',
  ].filter((chip) => String(chip || '').trim().length > 0)
})
const columnVisibilitySummary = computed(
  () => `Columns ${visibleColumnCount.value}/${renderedColumns.value.length}`,
)
const columnVisibilityHint = computed(() => {
  if (hiddenColumnCount.value > 0) {
    return `${hiddenColumnCount.value} hidden columns. Restore defaults to recover the full meta review table.`
  }
  return 'Hide lower-signal grouping columns when you want a tighter meta selection review.'
})

const strategyOptions = computed<OptionItem[]>(() => {
  const options = metaStrategyOptions.value as OptionItem[]
  if (options.length > 0) return options
  const value = String(metaStrategyKind.value || 'CompositeMo').trim() || 'CompositeMo'
  return [{ value, label: value, disabled: false }]
})

const compareOptions = computed<OptionItem[]>(() => {
  const options = metaCompareOptions.value as OptionItem[]
  if (options.length > 0) return options
  return [{ value: '', label: 'Select baseline meta', disabled: false }]
})

const metaSavedSelectOptions = computed<OptionItem[]>(() => {
  const options = metaSavedOptions.value as OptionItem[]
  if (options.length > 0) return options
  return [{ value: '', label: 'No saved meta', disabled: false }]
})

const stage4SavedSelectOptions = computed<OptionItem[]>(() => {
  const options = stage4SavedOptions.value as OptionItem[]
  if (options.length > 0) return options
  return [{ value: '', label: 'No saved Stage 5', disabled: false }]
})

const policyEvalSavedSelectOptions = computed<OptionItem[]>(() => {
  const options = policyEvalSavedOptions.value as OptionItem[]
  if (options.length > 0) return options
  return [{ value: '', label: 'No saved policy evaluation', disabled: false }]
})

const holdoutModeOptions: OptionItem[] = [
  { value: 'light', label: 'Light (fast)', disabled: false },
  { value: 'heavy', label: 'Heavy (walk-forward + optional perturb/MC)', disabled: false },
]

const holdoutRepsOptions: OptionItem[] = [
  { value: '1', label: '1 (fastest)', disabled: false },
  { value: '2', label: '2 (recommended)', disabled: false },
  { value: '3', label: '3 (more robust)', disabled: false },
]

const holdoutRankByOptions: OptionItem[] = [
  { value: 'auto', label: 'Auto (ML if healthy else meta)', disabled: false },
  { value: 'meta', label: 'Meta score', disabled: false },
  { value: 'ml', label: 'ML confidence', disabled: false },
]

const groupingModeOptions: OptionItem[] = [
  { value: 'clustered', label: 'Clustered (recommended)', disabled: false },
  { value: 'exact', label: 'Exact (strict)', disabled: false },
]

const batchPreloadPolicyOptions: OptionItem[] = [
  { value: 'host_only', label: 'Host preload (recommended)', disabled: false },
  { value: 'disabled', label: 'Disabled', disabled: false },
  { value: 'host_plus_doublema_vram', label: 'Host + DoubleMa VRAM staging (experimental)', disabled: false },
]

const batchExecutionModeOptions: OptionItem[] = [
  { value: 'full', label: 'Full campaign', disabled: false },
  { value: 'optimize_only', label: 'Optimize only', disabled: false },
  { value: 'validate_only', label: 'Validate only', disabled: false },
]

const batchRunButtonLabel = computed(() => {
  switch (String(batchExecutionModeDraft.value || 'full').trim()) {
    case 'optimize_only':
      return 'Run optimize-only'
    case 'validate_only':
      return 'Run validate-only'
    default:
      return 'Run full campaign'
  }
})

watch(
  () => [
    String(metaStrategyKind.value || 'CompositeMo'),
    String(metaTimeframe.value || ''),
    String(metaRequiredTag.value || ''),
    String(metaMinSamples.value || '30'),
    String(metaTopK.value || '25'),
    String(metaHoldoutTag.value || 'holdout').trim() || 'holdout',
    Boolean(metaEvalHoldout.value),
    Boolean(metaSaveHoldoutRows.value),
    String(metaHoldoutMode.value || 'light').trim() || 'light',
    String(metaHoldoutReps.value || '2').trim() || '2',
    String(metaHoldoutEvalTopN.value || '0').trim() || '0',
    String(metaHoldoutEvalRankBy.value || 'auto').trim() || 'auto',
    String(metaHoldoutExploreN.value || '0').trim() || '0',
    String(metaGroupingMode.value || 'clustered').trim() || 'clustered',
    String(metaClusterIntBucket.value || '5').trim() || '5',
    String(metaClusterFloatBucket.value || '0.25').trim() || '0.25',
    String(metaCompareSelected.value || '').trim(),
    String(batchDatasetSearch.value || '').trim(),
    String(batchTags.value || '').trim(),
    String(batchHoldoutPct.value || '15').trim() || '15',
    String(batchHoldoutTag.value || 'holdout').trim() || 'holdout',
    String(batchExecutionMode.value || 'full').trim() || 'full',
    String(batchPreloadPolicy.value || 'host_only').trim() || 'host_only',
    Boolean(batchAutoMeta.value),
    Boolean(batchAutoStage5.value),
    Boolean(batchSkipDone.value),
    String(metaSavedSearch.value || '').trim(),
    String(metaSavedSelected.value || '').trim(),
    String(metaSavedTagsDraft.value || '').trim(),
    String(stage4SavedSearch.value || '').trim(),
    String(stage4SavedSelected.value || '').trim(),
    String(stage4SavedTagsDraft.value || '').trim(),
    String(policyEvalSavedSearch.value || '').trim(),
    String(policyEvalSavedSelected.value || '').trim(),
    String(policyEvalSavedTagsDraft.value || '').trim(),
    Boolean(metaUseMlConfidence.value),
    Boolean(metaMlAutopilot.value),
    String(metaStage4TopK.value || '10').trim() || '10',
    String(metaStage4MinConf.value || '0.55').trim() || '0.55',
    String(metaStage4MetaWeight.value || '0.60').trim() || '0.60',
    String(metaStage4MlWeight.value || '0.35').trim() || '0.35',
    String(metaStage4UncertaintyWeight.value || '0.05').trim() || '0.05',
    String(metaStage4AcceptConf.value || '0.70').trim() || '0.70',
    String(metaStage4ReviewConf.value || '0.55').trim() || '0.55',
    String(metaStage4MaxAcceptUncertainty.value || '0.45').trim() || '0.45',
    Boolean(metaStage4AutoTrain.value),
    Boolean(metaStage4RequireHealthy.value),
    Boolean(metaStage4FallbackToMeta.value),
  ] as const,
  ([
    nextStrategy,
    nextTimeframe,
    nextRequiredTag,
    nextMinSamples,
    nextTopK,
    nextHoldoutTag,
    nextHoldoutEval,
    nextHoldoutSaveRows,
    nextHoldoutMode,
    nextHoldoutReps,
    nextHoldoutEvalTopN,
    nextHoldoutEvalRankBy,
    nextHoldoutExploreN,
    nextGroupingMode,
    nextClusterIntBucket,
    nextClusterFloatBucket,
    nextCompare,
    nextBatchSearch,
    nextBatchTags,
    nextBatchHoldoutPct,
    nextBatchHoldoutTag,
    nextBatchExecutionMode,
    nextBatchPreloadPolicy,
    nextBatchAutoMeta,
    nextBatchAutoStage5,
    nextBatchSkipDone,
    nextMetaSavedSearch,
    nextMetaSavedSelect,
    nextMetaSavedTags,
    nextStage4SavedSearch,
    nextStage4SavedSelect,
    nextStage4SavedTags,
    nextPolicyEvalSavedSearch,
    nextPolicyEvalSavedSelect,
    nextPolicyEvalSavedTags,
    nextUseMlConfidence,
    nextMlAutopilot,
    nextStage4TopK,
    nextStage4MinConf,
    nextStage4MetaWeight,
    nextStage4MlWeight,
    nextStage4UncertaintyWeight,
    nextStage4AcceptConf,
    nextStage4ReviewConf,
    nextStage4MaxAcceptUncertainty,
    nextStage4AutoTrain,
    nextStage4RequireHealthy,
    nextStage4FallbackToMeta,
  ]) => {
    if (strategyDraft.value !== nextStrategy) strategyDraft.value = nextStrategy
    if (timeframeDraft.value !== nextTimeframe) timeframeDraft.value = nextTimeframe
    if (requiredTagDraft.value !== nextRequiredTag) requiredTagDraft.value = nextRequiredTag
    if (minSamplesDraft.value !== nextMinSamples) minSamplesDraft.value = nextMinSamples
    if (topKDraft.value !== nextTopK) topKDraft.value = nextTopK
    if (holdoutTagDraft.value !== nextHoldoutTag) holdoutTagDraft.value = nextHoldoutTag
    if (holdoutEvalDraft.value !== nextHoldoutEval) holdoutEvalDraft.value = nextHoldoutEval
    if (holdoutSaveRowsDraft.value !== nextHoldoutSaveRows) holdoutSaveRowsDraft.value = nextHoldoutSaveRows
    if (holdoutModeDraft.value !== nextHoldoutMode) holdoutModeDraft.value = nextHoldoutMode
    if (holdoutRepsDraft.value !== nextHoldoutReps) holdoutRepsDraft.value = nextHoldoutReps
    if (holdoutEvalTopNDraft.value !== nextHoldoutEvalTopN) holdoutEvalTopNDraft.value = nextHoldoutEvalTopN
    if (holdoutEvalRankByDraft.value !== nextHoldoutEvalRankBy) holdoutEvalRankByDraft.value = nextHoldoutEvalRankBy
    if (holdoutExploreNDraft.value !== nextHoldoutExploreN) holdoutExploreNDraft.value = nextHoldoutExploreN
    if (groupingModeDraft.value !== nextGroupingMode) groupingModeDraft.value = nextGroupingMode
    if (clusterIntBucketDraft.value !== nextClusterIntBucket) clusterIntBucketDraft.value = nextClusterIntBucket
    if (clusterFloatBucketDraft.value !== nextClusterFloatBucket) clusterFloatBucketDraft.value = nextClusterFloatBucket
    if (compareDraft.value !== nextCompare) compareDraft.value = nextCompare
    if (batchSearchDraft.value !== nextBatchSearch) batchSearchDraft.value = nextBatchSearch
    if (batchTagsDraft.value !== nextBatchTags) batchTagsDraft.value = nextBatchTags
    if (batchHoldoutPctDraft.value !== nextBatchHoldoutPct) batchHoldoutPctDraft.value = nextBatchHoldoutPct
    if (batchHoldoutTagDraft.value !== nextBatchHoldoutTag) batchHoldoutTagDraft.value = nextBatchHoldoutTag
    if (batchExecutionModeDraft.value !== nextBatchExecutionMode) batchExecutionModeDraft.value = nextBatchExecutionMode
    if (batchPreloadPolicyDraft.value !== nextBatchPreloadPolicy) batchPreloadPolicyDraft.value = nextBatchPreloadPolicy
    if (batchAutoMetaDraft.value !== nextBatchAutoMeta) batchAutoMetaDraft.value = nextBatchAutoMeta
    if (batchAutoStage5Draft.value !== nextBatchAutoStage5) batchAutoStage5Draft.value = nextBatchAutoStage5
    if (batchSkipDoneDraft.value !== nextBatchSkipDone) batchSkipDoneDraft.value = nextBatchSkipDone
    if (metaSavedSearchDraft.value !== nextMetaSavedSearch) metaSavedSearchDraft.value = nextMetaSavedSearch
    if (metaSavedSelectDraft.value !== nextMetaSavedSelect) metaSavedSelectDraft.value = nextMetaSavedSelect
    if (metaSavedTagsLocalDraft.value !== nextMetaSavedTags) metaSavedTagsLocalDraft.value = nextMetaSavedTags
    if (stage4SavedSearchDraft.value !== nextStage4SavedSearch) stage4SavedSearchDraft.value = nextStage4SavedSearch
    if (stage4SavedSelectDraft.value !== nextStage4SavedSelect) stage4SavedSelectDraft.value = nextStage4SavedSelect
    if (stage4SavedTagsLocalDraft.value !== nextStage4SavedTags) stage4SavedTagsLocalDraft.value = nextStage4SavedTags
    if (policyEvalSavedSearchDraft.value !== nextPolicyEvalSavedSearch) policyEvalSavedSearchDraft.value = nextPolicyEvalSavedSearch
    if (policyEvalSavedSelectDraft.value !== nextPolicyEvalSavedSelect) policyEvalSavedSelectDraft.value = nextPolicyEvalSavedSelect
    if (policyEvalSavedTagsLocalDraft.value !== nextPolicyEvalSavedTags) policyEvalSavedTagsLocalDraft.value = nextPolicyEvalSavedTags
    if (metaUseMlConfidenceDraft.value !== nextUseMlConfidence) metaUseMlConfidenceDraft.value = nextUseMlConfidence
    if (metaMlAutopilotDraft.value !== nextMlAutopilot) metaMlAutopilotDraft.value = nextMlAutopilot
    if (metaStage4TopKDraft.value !== nextStage4TopK) metaStage4TopKDraft.value = nextStage4TopK
    if (metaStage4MinConfDraft.value !== nextStage4MinConf) metaStage4MinConfDraft.value = nextStage4MinConf
    if (metaStage4MetaWeightDraft.value !== nextStage4MetaWeight) metaStage4MetaWeightDraft.value = nextStage4MetaWeight
    if (metaStage4MlWeightDraft.value !== nextStage4MlWeight) metaStage4MlWeightDraft.value = nextStage4MlWeight
    if (metaStage4UncertaintyWeightDraft.value !== nextStage4UncertaintyWeight) metaStage4UncertaintyWeightDraft.value = nextStage4UncertaintyWeight
    if (metaStage4AcceptConfDraft.value !== nextStage4AcceptConf) metaStage4AcceptConfDraft.value = nextStage4AcceptConf
    if (metaStage4ReviewConfDraft.value !== nextStage4ReviewConf) metaStage4ReviewConfDraft.value = nextStage4ReviewConf
    if (metaStage4MaxAcceptUncertaintyDraft.value !== nextStage4MaxAcceptUncertainty) metaStage4MaxAcceptUncertaintyDraft.value = nextStage4MaxAcceptUncertainty
    if (metaStage4AutoTrainDraft.value !== nextStage4AutoTrain) metaStage4AutoTrainDraft.value = nextStage4AutoTrain
    if (metaStage4RequireHealthyDraft.value !== nextStage4RequireHealthy) metaStage4RequireHealthyDraft.value = nextStage4RequireHealthy
    if (metaStage4FallbackToMetaDraft.value !== nextStage4FallbackToMeta) metaStage4FallbackToMetaDraft.value = nextStage4FallbackToMeta
  },
  { immediate: true },
)

function setInputValue(id: string, nextValue: string): void {
  analysisStore.setMetaInputField(id, nextValue)
}

const { queueInputValue, commitInputValue, flushQueuedInputValuesNow } = useQueuedInputWriter(setInputValue)

function setSelectValue(id: string, nextValue: string): void {
  analysisStore.setMetaSelectField(id, nextValue)
}

function setCheckedValue(id: string, nextValue: boolean): void {
  analysisStore.setMetaCheckboxField(id, nextValue)
}

function runAction(action: () => void): void {
  flushQueuedInputValuesNow()
  action()
}

const savedMetaSecondaryActions = computed<ContextActionMenuItem[]>(() => [
  { key: 'refresh', label: 'Refresh', icon: artifactActionIcon('refresh'), disabled: !canRefreshMetaSaved.value },
  { key: 'report', label: 'Open report', icon: artifactActionIcon('report'), disabled: !canOpenMetaSavedReport.value },
  { key: 'reveal', label: 'Reveal file', icon: artifactActionIcon('reveal'), disabled: !canRevealMetaSaved.value },
  {
    key: 'delete',
    label: 'Delete',
    icon: artifactActionIcon('delete'),
    disabled: !canDeleteMetaSaved.value,
    destructive: true,
    separatorBefore: true,
  },
])

const savedStage5SecondaryActions = computed<ContextActionMenuItem[]>(() => [
  { key: 'refresh', label: 'Refresh', icon: artifactActionIcon('refresh'), disabled: !canRefreshStage4Saved.value },
  { key: 'reveal', label: 'Reveal file', icon: artifactActionIcon('reveal'), disabled: !canRevealStage4Saved.value },
  {
    key: 'delete',
    label: 'Delete',
    icon: artifactActionIcon('delete'),
    disabled: !canDeleteStage4Saved.value,
    destructive: true,
    separatorBefore: true,
  },
])

const savedPolicyEvalSecondaryActions = computed<ContextActionMenuItem[]>(() => [
  { key: 'refresh', label: 'Refresh', icon: artifactActionIcon('refresh'), disabled: !canRefreshPolicyEvalSaved.value },
  { key: 'reveal', label: 'Reveal file', icon: artifactActionIcon('reveal'), disabled: !canRevealPolicyEvalSaved.value },
  {
    key: 'delete',
    label: 'Delete',
    icon: artifactActionIcon('delete'),
    disabled: !canDeletePolicyEvalSaved.value,
    destructive: true,
    separatorBefore: true,
  },
])

function onSavedMetaActionSelect(actionKey: string): void {
  if (actionKey === 'refresh') {
    runAction(() => analysisStore.refreshSaved())
    return
  }
  if (actionKey === 'report') {
    runAction(() => analysisStore.openSavedReport())
    return
  }
  if (actionKey === 'reveal') {
    runAction(() => analysisStore.revealSaved())
    return
  }
  if (actionKey === 'delete') {
    runAction(() => analysisStore.deleteSaved())
  }
}

function onSavedStage5ActionSelect(actionKey: string): void {
  if (actionKey === 'refresh') {
    runAction(() => analysisStore.refreshStage5Saved())
    return
  }
  if (actionKey === 'reveal') {
    runAction(() => analysisStore.revealStage5Saved())
    return
  }
  if (actionKey === 'delete') {
    runAction(() => analysisStore.deleteStage5Saved())
  }
}

function onSavedPolicyEvalActionSelect(actionKey: string): void {
  if (actionKey === 'refresh') {
    runAction(() => analysisStore.refreshPolicyEvalSaved())
    return
  }
  if (actionKey === 'reveal') {
    runAction(() => analysisStore.revealPolicyEvalSaved())
    return
  }
  if (actionKey === 'delete') {
    runAction(() => analysisStore.deletePolicyEvalSaved())
  }
}

function aggregateTabId(key: string): string {
  return `meta_aggregate_tab_${key}`
}

function aggregatePanelId(key: AggregateTabKey): string {
  if (key === 'meta') {
    return metaStage.value === 'data_loaded_no_run'
      ? 'meta_aggregate_panel_meta_empty'
      : 'meta_aggregate_panel_meta_main'
  }
  if (key === 'compare') {
    return metaCompareStage.value === 'results_available_no_selection'
      ? 'meta_aggregate_panel_compare_empty'
      : 'meta_aggregate_panel_compare_main'
  }
  return `meta_aggregate_panel_${key}`
}

function headerMeta(header: Header<MetaRow, unknown>): MatrixColumnMeta {
  return header.column.columnDef.meta as MatrixColumnMeta
}

function metaCellPayload(cell: Cell<MetaRow, unknown>): MetaCell {
  return cell.getValue() as MetaCell
}

function metaColumnToggleId(columnKey: string): string {
  return `metaColumnToggle_${String(columnKey || '').replace(/[^a-zA-Z0-9_-]+/g, '_')}`
}

function moveAggregateTabFocus(event: KeyboardEvent): void {
  if (aggregateTabItems.length <= 1) return
  const currentIndex = aggregateTabItems.findIndex((item) => item.key === activeAggregateTab.value)
  if (currentIndex < 0) return

  let nextIndex = currentIndex
  if (event.key === 'ArrowRight' || event.key === 'ArrowDown') nextIndex = (currentIndex + 1) % aggregateTabItems.length
  else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') nextIndex = (currentIndex - 1 + aggregateTabItems.length) % aggregateTabItems.length
  else if (event.key === 'Home') nextIndex = 0
  else if (event.key === 'End') nextIndex = aggregateTabItems.length - 1
  else return

  event.preventDefault()
  const nextKey = aggregateTabItems[nextIndex]?.key
  if (!nextKey) return
  clickAggregateTab(nextKey)
  void nextTick(() => {
    document.querySelector<HTMLElement>(`#${aggregateTabId(nextKey)}`)?.focus()
  })
}

function clickAggregateTab(tab: AggregateTabKey): void {
  const aggregateTab = String(tab || '').trim() as AggregateTabKey
  if (!aggregateTab || activeAggregateTab.value === aggregateTab) return
  workflowStore.selectAggregateTab(aggregateTab)
}

function toggleBatchDatasetByPath(pathValue: string, nextChecked: boolean): void {
  const path = String(pathValue || '').trim()
  if (!path) return
  analysisStore.toggleBatchDataset(path, nextChecked)
}

function batchDatasetInputId(index: number): string {
  return `meta_batch_dataset_${index}`
}

function batchDatasetDescriptionId(index: number): string {
  return `meta_batch_dataset_meta_${index}`
}

onMounted(async () => {
  await nextTick()
  scrollToTop()
})

onBeforeUnmount(() => {
  flushQueuedInputValuesNow()
})
</script>

<template>
  <Teleport v-if="mountReady" to="#metaVueShellMount">
    <section class="metaPanelVueRoot metaVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Aggregate</div>
          <h2 class="panelTitle">Meta Analysis</h2>
          <div class="panelSubtitle">
            Group saved runs, generate meta artifacts, review saved aggregate outputs, and compare baselines before promotion.
          </div>
        </div>
        <div class="panelMeta mono panelNumeric">{{ activeAggregateTabLabel }} · {{ stamp }}</div>
      </header>

      <div class="panelSummaryChips" aria-label="Meta analysis summary">
        <div v-for="chip in metaSummaryChips" :key="chip" class="panelSummaryChip">{{ chip }}</div>
      </div>

      <section v-if="metaStage === 'no_data'" class="emptyState">
        <div class="emptyTitle">Meta Analysis needs saved runs first.</div>
        <div class="emptyBody">Build a run library by saving optimizations, then use this workspace to generate meta artifacts, compare groups, and rank robust candidates.</div>
        <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
      </section>

      <section
        v-else-if="metaStage === 'data_loaded_no_run' && activeAggregateTab === 'meta'"
        id="meta_aggregate_panel_meta_empty"
        class="emptyState"
        role="tabpanel"
        :aria-labelledby="aggregateTabId('meta')"
      >
        <div class="emptyTitle">Saved runs are available, but no meta artifact is active yet.</div>
        <div class="emptyBody">Configure the selection criteria below, then run meta analysis to generate grouped recommendations and reports.</div>
        <div class="panelMuted mono panelNumeric">{{ aggregateSummarySavedRuns }} saved runs available</div>
      </section>

      <section
        v-else-if="metaCompareStage === 'results_available_no_selection' && compareTabActive"
        id="meta_aggregate_panel_compare_empty"
        class="emptyState"
        role="tabpanel"
        :aria-labelledby="aggregateTabId('compare')"
      >
        <div class="emptyTitle">Select a baseline meta artifact to compare.</div>
        <div class="emptyBody">Comparison results appear here once you choose a saved meta artifact from the compare selector.</div>
        <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
      </section>

      <section v-if="showMetaTooling" id="metaAggregateSurface" class="pane">
        <div
          class="subtabs"
          aria-label="Meta analysis sections"
          role="tablist"
          aria-orientation="horizontal"
          @keydown="moveAggregateTabFocus"
        >
          <button
            v-for="item in aggregateTabItems"
            :id="aggregateTabId(item.key)"
            :key="`meta_aggregate_tab_${item.key}`"
            type="button"
            :class="{ active: activeAggregateTab === item.key }"
            role="tab"
            :aria-selected="activeAggregateTab === item.key"
            :data-aggregate-tab="item.key"
            :aria-controls="aggregatePanelId(item.key)"
            :tabindex="activeAggregateTab === item.key ? 0 : -1"
            @click="clickAggregateTab(item.key)"
          >
            {{ item.label }}
          </button>
        </div>
      </section>

      <section
        v-if="showMetaTooling && activeAggregateTab === 'batch'"
        :id="aggregatePanelId('batch')"
        class="pane panelProminentPane"
        role="tabpanel"
        :aria-labelledby="aggregateTabId('batch')"
      >
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Batch</div>
            <h3 class="panelPaneTitle">Dataset generation queue</h3>
            <div class="panelSectionLead">
              Choose the dataset set, holdout settings, and retry behavior for the next batch-generation sweep.
            </div>
          </div>
          <ContextHelpTip
            title="Batch generation queue"
            summary="Use this batch surface for dataset selection, holdout policy, and retry behavior before launching a sweep."
            detail="The queue is designed to expose the next run's selection, holdout tag, and auto-meta policy without opening another panel. Long-running output stays behind disclosure so the dataset list remains the dominant surface."
            aria-label="Batch generation queue help"
          />
        </div>
        <div class="gridFour">
          <div>
            <label for="vueBatchDatasetSearch">Search</label>
            <input
              id="vueBatchDatasetSearch"
              type="text"
              :value="batchSearchDraft"
              @input="batchSearchDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('batchDatasetSearch', batchSearchDraft)"
              @change="commitInputValue('batchDatasetSearch', batchSearchDraft)"
            />
          </div>
          <div>
            <label for="vueBatchTags">Tags</label>
            <input
              id="vueBatchTags"
              type="text"
              :value="batchTagsDraft"
              @input="batchTagsDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('batchTags', batchTagsDraft)"
              @change="commitInputValue('batchTags', batchTagsDraft)"
            />
          </div>
          <div>
            <label for="vueBatchHoldoutPct">Holdout %</label>
            <input
              id="vueBatchHoldoutPct"
              type="number"
              min="0"
              max="50"
              step="1"
              :value="batchHoldoutPctDraft"
              @input="batchHoldoutPctDraft = String(($event.target as HTMLInputElement).value || '15'); queueInputValue('batchHoldoutPct', batchHoldoutPctDraft)"
              @change="commitInputValue('batchHoldoutPct', batchHoldoutPctDraft)"
            />
          </div>
          <div>
            <label for="vueBatchHoldoutTag">Holdout tag</label>
            <input
              id="vueBatchHoldoutTag"
              type="text"
              :value="batchHoldoutTagDraft"
              @input="batchHoldoutTagDraft = String(($event.target as HTMLInputElement).value || 'holdout'); queueInputValue('batchHoldoutTag', batchHoldoutTagDraft)"
              @change="commitInputValue('batchHoldoutTag', batchHoldoutTagDraft)"
            />
          </div>
          <div>
            <label for="vueBatchExecutionMode">Execution mode</label>
            <select
              id="vueBatchExecutionMode"
              :value="batchExecutionModeDraft"
              @change="batchExecutionModeDraft = String(($event.target as HTMLSelectElement).value || 'full'); setSelectValue('batchExecutionMode', batchExecutionModeDraft); if (batchExecutionModeDraft === 'optimize_only') { batchAutoMetaDraft = false; batchAutoStage5Draft = false; setCheckedValue('batchAutoMeta', false); setCheckedValue('batchAutoStage5', false) }"
            >
              <option
                v-for="option in batchExecutionModeOptions"
                :key="`batch_mode_${option.value}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
          <div>
            <label for="vueBatchPreloadPolicy">Preload policy</label>
            <select
              id="vueBatchPreloadPolicy"
              :value="batchPreloadPolicyDraft"
              @change="batchPreloadPolicyDraft = String(($event.target as HTMLSelectElement).value || 'host_only'); setSelectValue('batchPreloadPolicy', batchPreloadPolicyDraft)"
            >
              <option
                v-for="option in batchPreloadPolicyOptions"
                :key="`batch_preload_${option.value}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>

        <div class="checks">
          <label>
            <input
              type="checkbox"
              :checked="batchAutoMetaDraft"
              :disabled="batchExecutionModeDraft === 'optimize_only'"
              @change="batchAutoMetaDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('batchAutoMeta', batchAutoMetaDraft); if (!batchAutoMetaDraft) { batchAutoStage5Draft = false; setCheckedValue('batchAutoStage5', false) }"
            />
            Auto-run meta after batch
          </label>
          <label>
            <input
              type="checkbox"
              :checked="batchAutoStage5Draft"
              :disabled="batchExecutionModeDraft === 'optimize_only' || !batchAutoMetaDraft"
              @change="batchAutoStage5Draft = ($event.target as HTMLInputElement).checked; setCheckedValue('batchAutoStage5', batchAutoStage5Draft)"
            />
            Run Stage 5 after meta
          </label>
          <label>
            <input
              type="checkbox"
              :checked="batchSkipDoneDraft"
              @change="batchSkipDoneDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('batchSkipDone', batchSkipDoneDraft)"
            />
            Skip completed (same batch tag)
          </label>
        </div>
        <div class="panelMuted">
          Host preload is the default campaign path. The VRAM staging mode is experimental and only applies to the plain DoubleMa exact GpuKernel route; other paths fall back to host preload. `Optimize only` skips meta and Stage 5, while `Validate only` reuses existing optimization artifacts instead of rerunning search.
        </div>

        <div class="actions">
          <button type="button" class="primary" :disabled="!canRunBatch" @click="runAction(() => analysisStore.runBatch())">{{ batchRunButtonLabel }}</button>
          <button type="button" :disabled="!canBatchStop" @click="runAction(() => analysisStore.batchStop())">Stop after current</button>
          <button type="button" :disabled="!canBatchRefresh" @click="runAction(() => analysisStore.batchRefresh())">Refresh</button>
          <button type="button" :disabled="!canBatchSelectAll" @click="runAction(() => analysisStore.batchSelectAll())">All</button>
          <button type="button" :disabled="!canBatchSelectNone" @click="runAction(() => analysisStore.batchSelectNone())">None</button>
          <button type="button" :disabled="!canBatchRetryFailed" @click="runAction(() => analysisStore.batchRetryFailed())">Retry failed</button>
          <button type="button" :disabled="!canBatchResumeIncomplete" @click="runAction(() => analysisStore.batchResumeIncomplete())">Resume incomplete</button>
          <button type="button" :disabled="!canBatchRevealSummary" @click="runAction(() => analysisStore.batchRevealSummary())">Reveal summary</button>
        </div>

        <div class="metaLine mono">{{ batchMeta }}</div>
        <div class="metaLine mono" role="status" aria-live="polite">
          Selected {{ batchSelectedCount }}/{{ batchTotalCount }}
        </div>

        <div class="batchList" role="group" aria-label="Batch generation datasets">
          <label
            v-for="(item, index) in renderedBatchDatasets"
            :key="`batch_item_${item.path || item.label}_${index}`"
            class="batchItem"
            :class="{ disabled: item.disabled }"
            :for="batchDatasetInputId(index)"
          >
            <div class="left">
              <div class="mono">{{ item.label }}</div>
              <div :id="batchDatasetDescriptionId(index)" class="panelMuted">{{ item.meta }}</div>
            </div>
            <input
              :id="batchDatasetInputId(index)"
              type="checkbox"
              :checked="item.checked"
              :disabled="item.disabled"
              :aria-describedby="batchDatasetDescriptionId(index)"
              @change="toggleBatchDatasetByPath(item.path, ($event.target as HTMLInputElement).checked)"
            />
          </label>
        </div>

        <details class="panelDetails">
          <summary>Batch output preview</summary>
          <div class="panelDetailsBody">
            <div class="panelMuted">Keep the raw batch summary close at hand for troubleshooting without letting it dominate the dataset-selection flow.</div>
            <pre class="mono preview">{{ batchOutputPreview }}</pre>
          </div>
        </details>
      </section>

      <div
        v-if="showMetaTooling && activeAggregateTab === 'saved'"
        :id="aggregatePanelId('saved')"
        class="aggregatePanelStack"
        role="tabpanel"
        :aria-labelledby="aggregateTabId('saved')"
      >
      <section class="pane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Saved</div>
            <h3 class="panelPaneTitle">Saved Meta</h3>
            <div class="panelSectionLead">
              Search, tag, load, and reveal previously generated meta-analysis artifacts.
            </div>
          </div>
        </div>
        <div class="gridThree">
          <div>
            <label for="vueMetaSavedSearch">Search</label>
            <input
              id="vueMetaSavedSearch"
              type="text"
              :value="metaSavedSearchDraft"
              @input="metaSavedSearchDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('metaSavedSearch', metaSavedSearchDraft)"
              @change="commitInputValue('metaSavedSearch', metaSavedSearchDraft)"
            />
          </div>
          <div>
            <label for="vueMetaSavedSelect">Saved meta</label>
            <select
              id="vueMetaSavedSelect"
              :value="metaSavedSelectDraft"
              @change="metaSavedSelectDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('metaSavedSelect', metaSavedSelectDraft)"
            >
              <option
                v-for="option in metaSavedSelectOptions"
                :key="`saved_meta_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
          <div>
            <label for="vueMetaSavedTags">Tags</label>
            <div class="rowAuto">
              <input
                id="vueMetaSavedTags"
                type="text"
                :value="metaSavedTagsLocalDraft"
                @input="metaSavedTagsLocalDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('metaSavedTags', metaSavedTagsLocalDraft)"
                @change="commitInputValue('metaSavedTags', metaSavedTagsLocalDraft)"
              />
              <button type="button" :disabled="!canSaveMetaSavedTags" @click="runAction(() => analysisStore.saveSavedTags())">Save</button>
            </div>
          </div>
        </div>
        <div class="actions savedArtifactActions">
          <button type="button" :disabled="!canLoadMetaSaved" @click="runAction(() => analysisStore.loadSaved())">Load</button>
          <ContextActionMenu
            aria-label="Saved meta artifact actions"
            :items="savedMetaSecondaryActions"
            @select="onSavedMetaActionSelect"
          />
        </div>
        <div class="metaLine mono">{{ metaSavedMeta }}</div>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Saved</div>
            <h3 class="panelPaneTitle">Saved Stage 5</h3>
            <div class="panelSectionLead">
              Manage saved Stage 5 confidence outputs alongside the base meta artifacts.
            </div>
          </div>
        </div>
        <div class="gridThree">
          <div>
            <label for="vueStage4SavedSearch">Search</label>
            <input
              id="vueStage4SavedSearch"
              type="text"
              :value="stage4SavedSearchDraft"
              @input="stage4SavedSearchDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('stage4SavedSearch', stage4SavedSearchDraft)"
              @change="commitInputValue('stage4SavedSearch', stage4SavedSearchDraft)"
            />
          </div>
          <div>
            <label for="vueStage4SavedSelect">Saved Stage 5</label>
            <select
              id="vueStage4SavedSelect"
              :value="stage4SavedSelectDraft"
              @change="stage4SavedSelectDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('stage4SavedSelect', stage4SavedSelectDraft)"
            >
              <option
                v-for="option in stage4SavedSelectOptions"
                :key="`saved_stage4_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
          <div>
            <label for="vueStage4SavedTags">Tags</label>
            <div class="rowAuto">
              <input
                id="vueStage4SavedTags"
                type="text"
                :value="stage4SavedTagsLocalDraft"
                @input="stage4SavedTagsLocalDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('stage4SavedTags', stage4SavedTagsLocalDraft)"
                @change="commitInputValue('stage4SavedTags', stage4SavedTagsLocalDraft)"
              />
              <button type="button" :disabled="!canSaveStage4SavedTags" @click="runAction(() => analysisStore.saveStage5SavedTags())">Save</button>
            </div>
          </div>
        </div>
        <div class="actions savedArtifactActions">
          <button type="button" :disabled="!canLoadStage4Saved" @click="runAction(() => analysisStore.loadStage5Saved())">Load</button>
          <ContextActionMenu
            aria-label="Saved Stage 5 artifact actions"
            :items="savedStage5SecondaryActions"
            @select="onSavedStage5ActionSelect"
          />
        </div>
        <div class="metaLine mono">{{ stage4SavedMeta }}</div>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Saved</div>
            <h3 class="panelPaneTitle">Saved Policy Evaluation</h3>
            <div class="panelSectionLead">
              Review and reopen saved policy-evaluation outputs without leaving the aggregate workspace.
            </div>
          </div>
        </div>
        <div class="gridThree">
          <div>
            <label for="vuePolicyEvalSavedSearch">Search</label>
            <input
              id="vuePolicyEvalSavedSearch"
              type="text"
              :value="policyEvalSavedSearchDraft"
              @input="policyEvalSavedSearchDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('policyEvalSavedSearch', policyEvalSavedSearchDraft)"
              @change="commitInputValue('policyEvalSavedSearch', policyEvalSavedSearchDraft)"
            />
          </div>
          <div>
            <label for="vuePolicyEvalSavedSelect">Saved policy evaluation</label>
            <select
              id="vuePolicyEvalSavedSelect"
              :value="policyEvalSavedSelectDraft"
              @change="policyEvalSavedSelectDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('policyEvalSavedSelect', policyEvalSavedSelectDraft)"
            >
              <option
                v-for="option in policyEvalSavedSelectOptions"
                :key="`saved_policy_eval_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
          <div>
            <label for="vuePolicyEvalSavedTags">Tags</label>
            <div class="rowAuto">
              <input
                id="vuePolicyEvalSavedTags"
                type="text"
                :value="policyEvalSavedTagsLocalDraft"
                @input="policyEvalSavedTagsLocalDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('policyEvalSavedTags', policyEvalSavedTagsLocalDraft)"
                @change="commitInputValue('policyEvalSavedTags', policyEvalSavedTagsLocalDraft)"
              />
              <button type="button" :disabled="!canSavePolicyEvalSavedTags" @click="runAction(() => analysisStore.savePolicyEvalSavedTags())">Save</button>
            </div>
          </div>
        </div>
        <div class="actions savedArtifactActions">
          <button type="button" :disabled="!canLoadPolicyEvalSaved" @click="runAction(() => analysisStore.loadPolicyEvalSaved())">Load</button>
          <ContextActionMenu
            aria-label="Saved policy evaluation artifact actions"
            :items="savedPolicyEvalSecondaryActions"
            @select="onSavedPolicyEvalActionSelect"
          />
        </div>
        <div class="metaLine mono">{{ policyEvalSavedMeta }}</div>
      </section>
      </div>

      <div
        v-if="showMetaTooling && metaTabActive"
        id="meta_aggregate_panel_meta_main"
        class="aggregatePanelStack"
        role="tabpanel"
        :aria-labelledby="aggregateTabId('meta')"
      >
      <section class="pane panelProminentPane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Snapshot</div>
            <h3 class="panelPaneTitle">Aggregate inventory</h3>
            <div class="panelSectionLead">
              Confirm how many saved runs and meta artifacts are in play before starting a new aggregate selection pass.
            </div>
          </div>
        </div>
        <div class="summaryGrid">
          <article>
            <div class="panelMetricKey">Saved runs</div>
            <div class="panelMetricValue mono panelNumeric">{{ aggregateSummarySavedRuns }}</div>
          </article>
          <article>
            <div class="panelMetricKey">Saved meta</div>
            <div class="panelMetricValue mono panelNumeric">{{ aggregateSummarySavedMeta }}</div>
          </article>
          <article>
            <div class="panelMetricKey">Latest groups</div>
            <div class="panelMetricValue mono panelNumeric">{{ aggregateSummaryLatestGroups }}</div>
          </article>
          <article>
            <div class="panelMetricKey">Latest artifacts</div>
            <div class="panelMetricValue mono panelNumeric">{{ aggregateSummaryLatestArtifacts }}</div>
          </article>
          <article>
            <div class="panelMetricKey">Latest holdout</div>
            <div class="panelMetricValue mono panelNumeric">{{ aggregateSummaryLatestHoldout }}</div>
          </article>
          <article>
            <div class="panelMetricKey">Latest runtime</div>
            <div class="panelMetricValue mono panelNumeric">{{ aggregateSummaryLatestRuntime }}</div>
          </article>
        </div>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">ML</div>
            <h3 class="panelPaneTitle">ML Confidence + Stage 5</h3>
            <div class="panelSectionLead">
              Refresh the confidence model, tune Stage 5 weighting, and decide when aggregate ranking should trust ML outputs.
            </div>
          </div>
        </div>
        <div class="checks">
          <label>
            <input
              type="checkbox"
              :checked="metaUseMlConfidenceDraft"
              @change="metaUseMlConfidenceDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('metaUseMlConfidence', metaUseMlConfidenceDraft)"
            />
            Enable ML confidence
          </label>
          <label>
            <input
              type="checkbox"
              :checked="metaMlAutopilotDraft"
              @change="metaMlAutopilotDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('metaMlAutopilot', metaMlAutopilotDraft)"
            />
            Autopilot
          </label>
        </div>
        <div class="actions">
          <button type="button" :disabled="!canMetaMlRefresh" @click="runAction(() => analysisStore.mlRefresh())">Refresh</button>
          <button type="button" :disabled="!canMetaMlTrain" @click="runAction(() => analysisStore.mlTrain())">Train / update</button>
          <button type="button" :disabled="!canMetaMlApply" @click="runAction(() => analysisStore.mlApply())">Apply to current meta</button>
          <button type="button" :disabled="!canMetaMlDelete" @click="runAction(() => analysisStore.mlDelete())">Delete model</button>
        </div>
        <div class="metaLine mono">{{ metaMlStatus }}</div>

        <div class="gridThree" style="margin-top: 0.6rem;">
          <div>
            <label for="vueMetaStage4TopK">Top K</label>
            <input
              id="vueMetaStage4TopK"
              type="number"
              min="1"
              step="1"
              :value="metaStage4TopKDraft"
              @input="metaStage4TopKDraft = String(($event.target as HTMLInputElement).value || '10'); queueInputValue('metaStage4TopK', metaStage4TopKDraft)"
              @change="commitInputValue('metaStage4TopK', metaStage4TopKDraft)"
            />
          </div>
          <div>
            <label for="vueMetaStage4MinConf">Min ML confidence</label>
            <input
              id="vueMetaStage4MinConf"
              type="number"
              min="0"
              max="1"
              step="0.01"
              :value="metaStage4MinConfDraft"
              @input="metaStage4MinConfDraft = String(($event.target as HTMLInputElement).value || '0.55'); queueInputValue('metaStage4MinConf', metaStage4MinConfDraft)"
              @change="commitInputValue('metaStage4MinConf', metaStage4MinConfDraft)"
            />
          </div>
          <div class="actions" style="margin-top: 1.4rem;">
            <button type="button" :disabled="!canRunMetaStage4" @click="runAction(() => analysisStore.runStage5())">Run Stage 5</button>
            <button type="button" :disabled="!canRunMetaPolicyEval" @click="runAction(() => analysisStore.runMlPolicyEvaluation())">Evaluate policy</button>
            <button type="button" :disabled="!canRevealMetaStage4" @click="runAction(() => analysisStore.revealStage5())">Reveal Stage 5 file</button>
          </div>
        </div>

        <div class="gridThree" style="margin-top: 0.6rem;">
          <div>
            <label for="vueMetaStage4MetaWeight">Weight: Meta score</label>
            <input
              id="vueMetaStage4MetaWeight"
              type="number"
              min="0"
              step="0.01"
              :value="metaStage4MetaWeightDraft"
              @input="metaStage4MetaWeightDraft = String(($event.target as HTMLInputElement).value || '0.60'); queueInputValue('metaStage4MetaWeight', metaStage4MetaWeightDraft)"
              @change="commitInputValue('metaStage4MetaWeight', metaStage4MetaWeightDraft)"
            />
          </div>
          <div>
            <label for="vueMetaStage4MlWeight">Weight: ML confidence</label>
            <input
              id="vueMetaStage4MlWeight"
              type="number"
              min="0"
              step="0.01"
              :value="metaStage4MlWeightDraft"
              @input="metaStage4MlWeightDraft = String(($event.target as HTMLInputElement).value || '0.35'); queueInputValue('metaStage4MlWeight', metaStage4MlWeightDraft)"
              @change="commitInputValue('metaStage4MlWeight', metaStage4MlWeightDraft)"
            />
          </div>
          <div>
            <label for="vueMetaStage4UncertaintyWeight">Weight: uncertainty penalty</label>
            <input
              id="vueMetaStage4UncertaintyWeight"
              type="number"
              min="0"
              step="0.01"
              :value="metaStage4UncertaintyWeightDraft"
              @input="metaStage4UncertaintyWeightDraft = String(($event.target as HTMLInputElement).value || '0.05'); queueInputValue('metaStage4UncertaintyWeight', metaStage4UncertaintyWeightDraft)"
              @change="commitInputValue('metaStage4UncertaintyWeight', metaStage4UncertaintyWeightDraft)"
            />
          </div>
        </div>

        <div class="gridThree" style="margin-top: 0.6rem;">
          <div>
            <label for="vueMetaStage4AcceptConf">Accept confidence</label>
            <input
              id="vueMetaStage4AcceptConf"
              type="number"
              min="0"
              max="1"
              step="0.01"
              :value="metaStage4AcceptConfDraft"
              @input="metaStage4AcceptConfDraft = String(($event.target as HTMLInputElement).value || '0.70'); queueInputValue('metaStage4AcceptConf', metaStage4AcceptConfDraft)"
              @change="commitInputValue('metaStage4AcceptConf', metaStage4AcceptConfDraft)"
            />
          </div>
          <div>
            <label for="vueMetaStage4ReviewConf">Review confidence</label>
            <input
              id="vueMetaStage4ReviewConf"
              type="number"
              min="0"
              max="1"
              step="0.01"
              :value="metaStage4ReviewConfDraft"
              @input="metaStage4ReviewConfDraft = String(($event.target as HTMLInputElement).value || '0.55'); queueInputValue('metaStage4ReviewConf', metaStage4ReviewConfDraft)"
              @change="commitInputValue('metaStage4ReviewConf', metaStage4ReviewConfDraft)"
            />
          </div>
          <div>
            <label for="vueMetaStage4MaxAcceptUncertainty">Max accept uncertainty</label>
            <input
              id="vueMetaStage4MaxAcceptUncertainty"
              type="number"
              min="0"
              max="1"
              step="0.01"
              :value="metaStage4MaxAcceptUncertaintyDraft"
              @input="metaStage4MaxAcceptUncertaintyDraft = String(($event.target as HTMLInputElement).value || '0.45'); queueInputValue('metaStage4MaxAcceptUncertainty', metaStage4MaxAcceptUncertaintyDraft)"
              @change="commitInputValue('metaStage4MaxAcceptUncertainty', metaStage4MaxAcceptUncertaintyDraft)"
            />
          </div>
        </div>

        <div class="checks">
          <label>
            <input
              type="checkbox"
              :checked="metaStage4AutoTrainDraft"
              @change="metaStage4AutoTrainDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('metaStage4AutoTrain', metaStage4AutoTrainDraft)"
            />
            Auto-train / retrain model
          </label>
          <label>
            <input
              type="checkbox"
              :checked="metaStage4RequireHealthyDraft"
              @change="metaStage4RequireHealthyDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('metaStage4RequireHealthy', metaStage4RequireHealthyDraft)"
            />
            Require healthy model
          </label>
          <label>
            <input
              type="checkbox"
              :checked="metaStage4FallbackToMetaDraft"
              @change="metaStage4FallbackToMetaDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('metaStage4FallbackToMeta', metaStage4FallbackToMetaDraft)"
            />
            Fallback to meta if empty
          </label>
        </div>

        <div class="metaLine mono">{{ metaStage4Status }}</div>
        <details class="panelDetails">
          <summary>Stage 5 output preview</summary>
          <div class="panelDetailsBody">
            <pre class="mono preview">{{ metaStage4OutputPreview }}</pre>
          </div>
        </details>
        <div class="metaLine mono">{{ metaPolicyEvalStatus }}</div>
        <details class="panelDetails">
          <summary>Policy evaluation preview</summary>
          <div class="panelDetailsBody">
            <pre class="mono preview">{{ metaPolicyEvalOutputPreview }}</pre>
          </div>
        </details>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Selection</div>
            <h3 class="panelPaneTitle">Meta run configuration</h3>
            <div class="panelSectionLead">
              Set the strategy scope, run filters, and top-k size for the next meta-analysis execution.
            </div>
          </div>
        </div>
        <div class="gridFive">
          <div>
            <label for="vueMetaStrategy">Strategy</label>
            <select
              id="vueMetaStrategy"
              :value="strategyDraft"
              @change="strategyDraft = String(($event.target as HTMLSelectElement).value || 'CompositeMo'); setSelectValue('metaStrategyKind', strategyDraft)"
            >
              <option
                v-for="option in strategyOptions"
                :key="`meta_strategy_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
          <div>
            <label for="vueMetaTimeframe">Timeframe filter</label>
            <input
              id="vueMetaTimeframe"
              type="text"
              :value="timeframeDraft"
              placeholder="1m"
              @input="timeframeDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('metaTimeframe', timeframeDraft)"
              @change="commitInputValue('metaTimeframe', timeframeDraft)"
            />
          </div>
          <div>
            <label for="vueMetaRequiredTag">Required tag</label>
            <input
              id="vueMetaRequiredTag"
              type="text"
              :value="requiredTagDraft"
              placeholder="e.g. 1m, forex, top50"
              @input="requiredTagDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('metaRequiredTag', requiredTagDraft)"
              @change="commitInputValue('metaRequiredTag', requiredTagDraft)"
            />
          </div>
          <div>
            <label for="vueMetaMinSamples">Min samples</label>
            <input
              id="vueMetaMinSamples"
              type="number"
              min="1"
              step="1"
              :value="minSamplesDraft"
              @input="minSamplesDraft = String(($event.target as HTMLInputElement).value || '30'); queueInputValue('metaMinSamples', minSamplesDraft)"
              @change="commitInputValue('metaMinSamples', minSamplesDraft)"
            />
          </div>
          <div>
            <label for="vueMetaTopK">Top K</label>
            <input
              id="vueMetaTopK"
              type="number"
              min="1"
              step="1"
              :value="topKDraft"
              @input="topKDraft = String(($event.target as HTMLInputElement).value || '25'); queueInputValue('metaTopK', topKDraft)"
              @change="commitInputValue('metaTopK', topKDraft)"
            />
          </div>
        </div>
        <div class="actions">
          <button type="button" class="primary" :disabled="!canRunMeta" @click="runAction(() => analysisStore.runMeta())">Run meta analysis</button>
          <button type="button" :disabled="!canOpenMetaReport" @click="runAction(() => analysisStore.openMetaReport())">Open meta report (XLSX)</button>
          <button type="button" :disabled="!canRevealMetaFiles" @click="runAction(() => analysisStore.revealMetaFiles())">Reveal files</button>
        </div>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Grouping</div>
            <h3 class="panelPaneTitle">Holdout + Grouping</h3>
            <div class="panelSectionLead">
              Control holdout evaluation, grouping strictness, and clustering so the aggregate selection logic stays comparable.
            </div>
          </div>
        </div>
        <div class="gridThree">
          <div>
            <label for="vueMetaHoldoutTag">Holdout tag</label>
            <input
              id="vueMetaHoldoutTag"
              type="text"
              :value="holdoutTagDraft"
              @input="holdoutTagDraft = String(($event.target as HTMLInputElement).value || 'holdout'); queueInputValue('metaHoldoutTag', holdoutTagDraft)"
              @change="commitInputValue('metaHoldoutTag', holdoutTagDraft)"
            />
          </div>
          <div>
            <label for="vueMetaHoldoutMode">Holdout mode</label>
            <select
              id="vueMetaHoldoutMode"
              :value="holdoutModeDraft"
              @change="holdoutModeDraft = String(($event.target as HTMLSelectElement).value || 'light'); setSelectValue('metaHoldoutMode', holdoutModeDraft)"
            >
              <option
                v-for="option in holdoutModeOptions"
                :key="`meta_holdout_mode_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
          <div>
            <label for="vueMetaHoldoutReps">Holdout reps per cluster</label>
            <select
              id="vueMetaHoldoutReps"
              :value="holdoutRepsDraft"
              @change="holdoutRepsDraft = String(($event.target as HTMLSelectElement).value || '2'); setSelectValue('metaHoldoutReps', holdoutRepsDraft)"
            >
              <option
                v-for="option in holdoutRepsOptions"
                :key="`meta_holdout_reps_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>

        <div class="checks">
          <label>
            <input
              type="checkbox"
              :checked="holdoutEvalDraft"
              @change="holdoutEvalDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('metaEvalHoldout', holdoutEvalDraft)"
            />
            Evaluate holdout (recommended)
          </label>
          <label>
            <input
              type="checkbox"
              :checked="holdoutSaveRowsDraft"
              @change="holdoutSaveRowsDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('metaSaveHoldoutRows', holdoutSaveRowsDraft)"
            />
            Save per-dataset holdout rows (for ML)
          </label>
        </div>

        <div class="gridThree" style="margin-top: 0.6rem;">
          <div>
            <label for="vueMetaHoldoutEvalTopN">Holdout eval top N (0 = all)</label>
            <input
              id="vueMetaHoldoutEvalTopN"
              type="number"
              min="0"
              step="1"
              :value="holdoutEvalTopNDraft"
              @input="holdoutEvalTopNDraft = String(($event.target as HTMLInputElement).value || '0'); queueInputValue('metaHoldoutEvalTopN', holdoutEvalTopNDraft)"
              @change="commitInputValue('metaHoldoutEvalTopN', holdoutEvalTopNDraft)"
            />
          </div>
          <div>
            <label for="vueMetaHoldoutEvalRankBy">Holdout rank by</label>
            <select
              id="vueMetaHoldoutEvalRankBy"
              :value="holdoutEvalRankByDraft"
              @change="holdoutEvalRankByDraft = String(($event.target as HTMLSelectElement).value || 'auto'); setSelectValue('metaHoldoutEvalRankBy', holdoutEvalRankByDraft)"
            >
              <option
                v-for="option in holdoutRankByOptions"
                :key="`meta_holdout_rank_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
          <div>
            <label for="vueMetaHoldoutExploreN">Explore additional finalists (optional)</label>
            <input
              id="vueMetaHoldoutExploreN"
              type="number"
              min="0"
              step="1"
              :value="holdoutExploreNDraft"
              @input="holdoutExploreNDraft = String(($event.target as HTMLInputElement).value || '0'); queueInputValue('metaHoldoutExploreN', holdoutExploreNDraft)"
              @change="commitInputValue('metaHoldoutExploreN', holdoutExploreNDraft)"
            />
          </div>
        </div>

        <div class="gridThree" style="margin-top: 0.6rem;">
          <div>
            <label for="vueMetaGroupingMode">Grouping</label>
            <select
              id="vueMetaGroupingMode"
              :value="groupingModeDraft"
              @change="groupingModeDraft = String(($event.target as HTMLSelectElement).value || 'clustered'); setSelectValue('metaGroupingMode', groupingModeDraft)"
            >
              <option
                v-for="option in groupingModeOptions"
                :key="`meta_grouping_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
          <div>
            <label for="vueMetaClusterIntBucket">Integer bucket size</label>
            <input
              id="vueMetaClusterIntBucket"
              type="number"
              min="1"
              step="1"
              :value="clusterIntBucketDraft"
              @input="clusterIntBucketDraft = String(($event.target as HTMLInputElement).value || '5'); queueInputValue('metaClusterIntBucket', clusterIntBucketDraft)"
              @change="commitInputValue('metaClusterIntBucket', clusterIntBucketDraft)"
            />
          </div>
          <div>
            <label for="vueMetaClusterFloatBucket">Float bucket size</label>
            <input
              id="vueMetaClusterFloatBucket"
              type="number"
              min="0"
              step="0.01"
              :value="clusterFloatBucketDraft"
              @input="clusterFloatBucketDraft = String(($event.target as HTMLInputElement).value || '0.25'); queueInputValue('metaClusterFloatBucket', clusterFloatBucketDraft)"
              @change="commitInputValue('metaClusterFloatBucket', clusterFloatBucketDraft)"
            />
          </div>
        </div>
        <div class="metaLine">{{ metaHoldoutNote }}</div>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Results</div>
            <h3 class="panelPaneTitle">Meta selection table</h3>
            <div class="panelSectionLead">
              Review grouped aggregate recommendations, holdout outcomes, and selection notes for the current meta run.
            </div>
          </div>
        </div>
        <div class="metaLine mono" role="status" aria-live="polite">{{ metaMeta }}</div>
        <details v-if="renderedColumns.length > 0" class="columnDetails">
          <summary class="columnDetailsSummary">
            <span>Visible columns</span>
            <span class="mono panelNumeric">{{ columnVisibilitySummary }}</span>
          </summary>
          <div class="columnDetailsBody">
            <div class="panelMuted">{{ columnVisibilityHint }}</div>
            <div class="columnToggleGrid" role="group" aria-label="Visible meta result columns">
              <label
                v-for="column in columnOptions"
                :key="column.key"
                class="columnToggle"
                :class="{ disabled: !canHideColumn(column.key) && column.visible }"
                :for="metaColumnToggleId(column.key)"
              >
                <input
                  :id="metaColumnToggleId(column.key)"
                  type="checkbox"
                  :checked="column.visible"
                  :disabled="!canHideColumn(column.key) && column.visible"
                  @change="toggleColumnVisibility(column.key)"
                />
                <span>{{ column.label }}</span>
              </label>
            </div>
            <div class="actions columnDetailsActions">
              <button type="button" :disabled="!canRestoreDefaults" @click="restoreDefaultColumns()">
                Restore defaults
              </button>
            </div>
          </div>
        </details>
        <div
          ref="tableScrollElementRef"
          class="tableWrap overflowShadowY overflowShadowX surfaceWorking"
          tabindex="0"
          aria-label="Meta analysis results table"
        >
          <table class="metaTableVue">
            <caption class="srOnly">
              Meta analysis selection results grouped by the configured strategy, holdout, and clustering rules.
            </caption>
            <thead>
              <tr v-for="headerGroup in headerGroups" :key="headerGroup.id">
                <th
                  v-for="header in headerGroup.headers"
                  :key="header.id"
                  :class="{ left: headerMeta(header).alignLeft }"
                  scope="col"
                >
                  {{ headerMeta(header).columnLabel }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="virtualTopSpacerPx > 0" class="spacerRow" aria-hidden="true">
                <td :colspan="Math.max(1, visibleLeafColumns.length)" :style="{ height: `${virtualTopSpacerPx}px` }"></td>
              </tr>
              <tr v-for="virtualRow in virtualRows" :key="virtualRow.row.id">
                <template v-for="cell in virtualRow.row.getVisibleCells()" :key="cell.id">
                  <td
                    :class="metaCellPayload(cell).className"
                    :title="metaCellPayload(cell).title || undefined"
                  >
                    {{ metaCellPayload(cell).text }}
                  </td>
                </template>
              </tr>
              <tr v-if="virtualBottomSpacerPx > 0" class="spacerRow" aria-hidden="true">
                <td :colspan="Math.max(1, visibleLeafColumns.length)" :style="{ height: `${virtualBottomSpacerPx}px` }"></td>
              </tr>
            </tbody>
          </table>
        </div>
        <details class="panelDetails">
          <summary>Meta output preview</summary>
          <div class="panelDetailsBody">
            <pre class="mono preview">{{ metaOutputPreview }}</pre>
          </div>
        </details>
      </section>
      </div>

      <section
        v-if="showMetaTooling && compareTabActive"
        id="meta_aggregate_panel_compare_main"
        class="pane"
        role="tabpanel"
        :aria-labelledby="aggregateTabId('compare')"
      >
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Comparison</div>
            <h3 class="panelPaneTitle">Compare meta baseline</h3>
            <div class="panelSectionLead">
              Choose a saved meta artifact and inspect the difference summary before accepting a new aggregate baseline.
            </div>
          </div>
        </div>
        <label for="vueMetaCompareSelect">Compare meta baseline</label>
        <select
          id="vueMetaCompareSelect"
          :value="compareDraft"
          @change="compareDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('metaCompareSelect', compareDraft)"
        >
          <option
            v-for="option in compareOptions"
            :key="`meta_compare_${option.value}_${option.label}`"
            :value="option.value"
            :disabled="option.disabled"
          >
            {{ option.label }}
          </option>
        </select>
        <details class="panelDetails">
          <summary>Compare summary</summary>
          <div class="panelDetailsBody">
            <pre class="mono preview">{{ metaCompareSummary }}</pre>
          </div>
        </details>
      </section>

      <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.metaPanelVueRoot {
  display: grid;
  gap: 0.85rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.subtabs {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.subtabs button {
  padding: 0.4rem 0.58rem;
  font-size: 0.75rem;
  cursor: pointer;
}

.subtabs button.active {
  border-color: color-mix(in oklab, var(--accent) 65%, var(--border));
}

.subtabs button:focus-visible {
  outline: none;
  box-shadow: var(--focus-ring);
}

.aggregatePanelStack {
  display: grid;
  gap: 0.85rem;
}

.summaryGrid {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 0.55rem;
}

.summaryGrid article {
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 0.5rem 0.55rem;
  background: color-mix(in oklab, var(--panel) 88%, var(--panel-footer));
}

.gridFive {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 0.6rem;
}

.gridFour {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.6rem;
}

.gridThree {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.6rem;
}

.actions {
  margin-top: 0.65rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.actions .primary {
  border-color: color-mix(in oklab, var(--accent) 70%, var(--border));
}

.checks {
  margin-top: 0.6rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.7rem;
}

.metaLine {
  font-size: 0.78rem;
  color: var(--muted);
}

.tableWrap {
  margin-top: 0.55rem;
  overflow: auto;
}

.columnDetails {
  margin-top: 0.55rem;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface-sunken);
}

.columnDetailsSummary {
  list-style: none;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
  cursor: pointer;
  padding: calc(var(--density-pane-y) - 0.12rem) calc(var(--density-pane-x) - 0.02rem);
  font-size: 0.78rem;
  font-weight: 600;
}

.columnDetailsSummary::-webkit-details-marker {
  display: none;
}

.columnDetailsBody {
  border-top: 1px solid var(--border);
  padding: calc(var(--density-pane-y) - 0.05rem) calc(var(--density-pane-x) - 0.02rem);
  display: grid;
  gap: 0.65rem;
}

.columnToggleGrid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.45rem 0.75rem;
}

.columnToggle {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  min-width: 0;
  font-size: 0.76rem;
}

.columnToggle.disabled {
  opacity: 0.62;
}

.columnToggle input {
  margin: 0;
}

.columnDetailsActions {
  margin-top: 0;
}

.metaTableVue {
  width: 100%;
  border-collapse: collapse;
  min-width: 1080px;
}

.srOnly {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

.metaTableVue th,
.metaTableVue td {
  border-bottom: 1px solid var(--border);
  padding: 0.34rem 0.42rem;
  text-align: right;
  vertical-align: top;
}

.metaTableVue th.left,
.metaTableVue td.left {
  text-align: left;
}

.metaTableVue tbody tr.spacerRow {
  pointer-events: none;
}

.metaTableVue tbody tr.spacerRow td {
  border-bottom: none;
  padding: 0;
}

.batchList {
  margin-top: 0.55rem;
  max-height: 300px;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 0.25rem;
}

.batchItem {
  border: 1px solid transparent;
  border-radius: 8px;
  padding: 0.42rem 0.5rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.7rem;
  cursor: pointer;
}

.batchItem:hover {
  border-color: var(--border);
  background: color-mix(in oklab, var(--panel) 92%, var(--panel-footer));
}

.batchItem:focus-within {
  border-color: color-mix(in oklab, var(--accent) 62%, var(--border));
  box-shadow: var(--focus-ring);
  background: color-mix(in oklab, var(--panel) 90%, var(--panel-footer));
}

.batchItem.disabled {
  cursor: not-allowed;
  opacity: 0.64;
}

.rowAuto {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.45rem;
}

.preview {
  margin-top: 0.45rem;
  max-height: 220px;
  overflow: auto;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1220px) {
  .summaryGrid {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .gridFive,
  .gridFour,
  .gridThree {
    grid-template-columns: 1fr 1fr;
  }

  .columnToggleGrid {
    grid-template-columns: 1fr 1fr;
  }
}

@media (max-width: 740px) {
  .summaryGrid,
  .gridFive,
  .gridFour,
  .gridThree {
    grid-template-columns: 1fr;
  }

  .columnToggleGrid {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
[data-vue-panel='meta'][data-vue-meta='1'] > :not(#metaVueShellMount) {
  display: none !important;
}
</style>

