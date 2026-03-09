<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useUiReadiness } from '../composables/useUiReadiness'
import { useVirtualTableRows } from '../composables/useVirtualTableRows'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'

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
  batchAutoMeta,
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
  canRevealMetaStage4,
  metaStage4Status,
  metaStage4OutputPreview,
  canRunMeta,
  canOpenMetaReport,
  canRevealMetaFiles,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { aggregateTab, resultsTab } = storeToRefs(workflowStore)

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)

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
const batchAutoMetaDraft = ref(true)
const batchSkipDoneDraft = ref(true)
const metaSavedSearchDraft = ref('')
const metaSavedSelectDraft = ref('')
const metaSavedTagsLocalDraft = ref('')
const stage4SavedSearchDraft = ref('')
const stage4SavedSelectDraft = ref('')
const stage4SavedTagsLocalDraft = ref('')
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
const renderedBatchDatasets = computed<BatchDatasetItem[]>(() => batchDatasets.value as BatchDatasetItem[])
const {
  scrollElementRef: tableScrollElementRef,
  virtualRows,
  topSpacerPx: virtualTopSpacerPx,
  bottomSpacerPx: virtualBottomSpacerPx,
  scrollToTop,
} = useVirtualTableRows(renderedRows)

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
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
  return [{ value: '', label: 'No saved Stage 4', disabled: false }]
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
    Boolean(batchAutoMeta.value),
    Boolean(batchSkipDone.value),
    String(metaSavedSearch.value || '').trim(),
    String(metaSavedSelected.value || '').trim(),
    String(metaSavedTagsDraft.value || '').trim(),
    String(stage4SavedSearch.value || '').trim(),
    String(stage4SavedSelected.value || '').trim(),
    String(stage4SavedTagsDraft.value || '').trim(),
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
    nextBatchAutoMeta,
    nextBatchSkipDone,
    nextMetaSavedSearch,
    nextMetaSavedSelect,
    nextMetaSavedTags,
    nextStage4SavedSearch,
    nextStage4SavedSelect,
    nextStage4SavedTags,
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
    if (batchAutoMetaDraft.value !== nextBatchAutoMeta) batchAutoMetaDraft.value = nextBatchAutoMeta
    if (batchSkipDoneDraft.value !== nextBatchSkipDone) batchSkipDoneDraft.value = nextBatchSkipDone
    if (metaSavedSearchDraft.value !== nextMetaSavedSearch) metaSavedSearchDraft.value = nextMetaSavedSearch
    if (metaSavedSelectDraft.value !== nextMetaSavedSelect) metaSavedSelectDraft.value = nextMetaSavedSelect
    if (metaSavedTagsLocalDraft.value !== nextMetaSavedTags) metaSavedTagsLocalDraft.value = nextMetaSavedTags
    if (stage4SavedSearchDraft.value !== nextStage4SavedSearch) stage4SavedSearchDraft.value = nextStage4SavedSearch
    if (stage4SavedSelectDraft.value !== nextStage4SavedSelect) stage4SavedSelectDraft.value = nextStage4SavedSelect
    if (stage4SavedTagsLocalDraft.value !== nextStage4SavedTags) stage4SavedTagsLocalDraft.value = nextStage4SavedTags
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

function clickAggregateTab(tab: string): void {
  const aggregateTab = String(tab || '').trim()
  if (!aggregateTab) return
  workflowStore.selectAggregateTab(aggregateTab)
}

function toggleBatchDataset(index: number): void {
  const entry = renderedBatchDatasets.value[index]
  if (!entry || entry.disabled) return
  const path = String(entry.path || '').trim()
  if (!path) return
  analysisStore.toggleBatchDataset(path, !entry.checked)
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="meta"]') as HTMLElement | null
  const mount = document.querySelector('#metaVueShellMount') as HTMLElement | null
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueMeta = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
  scrollToTop()
})

onBeforeUnmount(() => {
  flushQueuedInputValuesNow()
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueMeta
    panelRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#metaVueShellMount">
    <section class="metaPanelVueRoot metaVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Meta Analysis</div>
        <div class="meta mono">{{ activeAggregateTab }} · {{ stamp }}</div>
      </header>

      <section v-if="metaStage === 'no_data'" class="emptyState">
        <div class="emptyTitle">Meta Analysis needs saved runs first.</div>
        <div class="emptyBody">Build a run library by saving optimizations, then use this workspace to generate meta artifacts, compare groups, and rank robust candidates.</div>
        <div class="muted mono">{{ statusLine }}</div>
      </section>

      <section v-else-if="metaStage === 'data_loaded_no_run' && activeAggregateTab === 'meta'" class="emptyState">
        <div class="emptyTitle">Saved runs are available, but no meta artifact is active yet.</div>
        <div class="emptyBody">Configure the selection criteria below, then run meta analysis to generate grouped recommendations and reports.</div>
        <div class="muted mono">{{ aggregateSummarySavedRuns }} saved runs available</div>
      </section>

      <section v-else-if="metaCompareStage === 'results_available_no_selection' && compareTabActive" class="emptyState">
        <div class="emptyTitle">Select a baseline meta artifact to compare.</div>
        <div class="emptyBody">Comparison results appear here once you choose a saved meta artifact from the compare selector.</div>
        <div class="muted mono">{{ statusLine }}</div>
      </section>

      <section v-if="showMetaTooling" class="pane">
        <div class="subtabs">
          <button type="button" :class="{ active: activeAggregateTab === 'meta' }" @click="clickAggregateTab('meta')">Meta Selection</button>
          <button type="button" :class="{ active: activeAggregateTab === 'batch' }" @click="clickAggregateTab('batch')">Batch Generation</button>
          <button type="button" :class="{ active: activeAggregateTab === 'saved' }" @click="clickAggregateTab('saved')">Saved Meta</button>
          <button type="button" :class="{ active: activeAggregateTab === 'compare' }" @click="clickAggregateTab('compare')">Compare</button>
        </div>
        <div v-if="metaTabActive" class="summaryGrid">
          <article>
            <div class="k">Saved runs</div>
            <div class="v mono">{{ aggregateSummarySavedRuns }}</div>
          </article>
          <article>
            <div class="k">Saved meta</div>
            <div class="v mono">{{ aggregateSummarySavedMeta }}</div>
          </article>
          <article>
            <div class="k">Latest groups</div>
            <div class="v mono">{{ aggregateSummaryLatestGroups }}</div>
          </article>
          <article>
            <div class="k">Latest artifacts</div>
            <div class="v mono">{{ aggregateSummaryLatestArtifacts }}</div>
          </article>
          <article>
            <div class="k">Latest holdout</div>
            <div class="v mono">{{ aggregateSummaryLatestHoldout }}</div>
          </article>
          <article>
            <div class="k">Latest runtime</div>
            <div class="v mono">{{ aggregateSummaryLatestRuntime }}</div>
          </article>
        </div>
      </section>

      <section v-if="showMetaTooling && activeAggregateTab === 'batch'" class="pane">
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
        </div>

        <div class="checks">
          <label>
            <input
              type="checkbox"
              :checked="batchAutoMetaDraft"
              @change="batchAutoMetaDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('batchAutoMeta', batchAutoMetaDraft)"
            />
            Auto-run meta after batch
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

        <div class="actions">
          <button type="button" class="primary" :disabled="!canRunBatch" @click="runAction(() => analysisStore.runBatch())">Run batch</button>
          <button type="button" :disabled="!canBatchStop" @click="runAction(() => analysisStore.batchStop())">Stop after current</button>
          <button type="button" :disabled="!canBatchRefresh" @click="runAction(() => analysisStore.batchRefresh())">Refresh</button>
          <button type="button" :disabled="!canBatchSelectAll" @click="runAction(() => analysisStore.batchSelectAll())">All</button>
          <button type="button" :disabled="!canBatchSelectNone" @click="runAction(() => analysisStore.batchSelectNone())">None</button>
          <button type="button" :disabled="!canBatchRetryFailed" @click="runAction(() => analysisStore.batchRetryFailed())">Retry failed</button>
          <button type="button" :disabled="!canBatchRevealSummary" @click="runAction(() => analysisStore.batchRevealSummary())">Reveal summary</button>
        </div>

        <div class="metaLine mono">{{ batchMeta }}</div>
        <div class="metaLine mono">Selected {{ batchSelectedCount }}/{{ batchTotalCount }}</div>

        <div class="batchList">
          <div
            v-for="(item, index) in renderedBatchDatasets"
            :key="`batch_item_${item.path || item.label}_${index}`"
            class="batchItem"
            @click="toggleBatchDataset(index)"
          >
            <div class="left">
              <div class="mono">{{ item.label }}</div>
              <div class="muted">{{ item.meta }}</div>
            </div>
            <input type="checkbox" :checked="item.checked" :disabled="item.disabled" @click.prevent="toggleBatchDataset(index)" />
          </div>
        </div>

        <pre class="mono preview">{{ batchOutputPreview }}</pre>
      </section>

      <section v-if="activeAggregateTab === 'saved'" class="pane">
        <h3>Saved Meta</h3>
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
        <div class="actions">
          <button type="button" :disabled="!canRefreshMetaSaved" @click="runAction(() => analysisStore.refreshSaved())">Refresh</button>
          <button type="button" :disabled="!canLoadMetaSaved" @click="runAction(() => analysisStore.loadSaved())">Load</button>
          <button type="button" :disabled="!canOpenMetaSavedReport" @click="runAction(() => analysisStore.openSavedReport())">Open report</button>
          <button type="button" :disabled="!canRevealMetaSaved" @click="runAction(() => analysisStore.revealSaved())">Reveal file</button>
          <button type="button" :disabled="!canDeleteMetaSaved" @click="runAction(() => analysisStore.deleteSaved())">Delete</button>
        </div>
        <div class="metaLine mono">{{ metaSavedMeta }}</div>
      </section>

      <section v-if="activeAggregateTab === 'saved'" class="pane">
        <h3>Saved Stage 4</h3>
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
            <label for="vueStage4SavedSelect">Saved Stage 4</label>
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
              <button type="button" :disabled="!canSaveStage4SavedTags" @click="runAction(() => analysisStore.saveStage4SavedTags())">Save</button>
            </div>
          </div>
        </div>
        <div class="actions">
          <button type="button" :disabled="!canRefreshStage4Saved" @click="runAction(() => analysisStore.refreshStage4Saved())">Refresh</button>
          <button type="button" :disabled="!canLoadStage4Saved" @click="runAction(() => analysisStore.loadStage4Saved())">Load</button>
          <button type="button" :disabled="!canRevealStage4Saved" @click="runAction(() => analysisStore.revealStage4Saved())">Reveal file</button>
          <button type="button" :disabled="!canDeleteStage4Saved" @click="runAction(() => analysisStore.deleteStage4Saved())">Delete</button>
        </div>
        <div class="metaLine mono">{{ stage4SavedMeta }}</div>
      </section>

      <section v-if="showMetaTooling && metaTabActive" class="pane">
        <h3>ML Confidence + Stage 4</h3>
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
            <button type="button" :disabled="!canRunMetaStage4" @click="runAction(() => analysisStore.runStage4())">Run Stage 4</button>
            <button type="button" :disabled="!canRevealMetaStage4" @click="runAction(() => analysisStore.revealStage4())">Reveal Stage 4 file</button>
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
        <pre class="mono preview">{{ metaStage4OutputPreview }}</pre>
      </section>

      <section v-if="showMetaTooling && metaTabActive" class="pane">
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

      <section v-if="showMetaTooling && metaTabActive" class="pane">
        <h3>Holdout + Grouping</h3>
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
            <label for="vueMetaHoldoutReps">Holdout reps / cluster</label>
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
            <label for="vueMetaHoldoutExploreN">Explore N (optional)</label>
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
            <label for="vueMetaClusterIntBucket">Int bucket</label>
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
            <label for="vueMetaClusterFloatBucket">Float bucket</label>
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

      <section v-if="metaTabActive" class="pane">
        <div class="metaLine mono">{{ metaMeta }}</div>
        <div ref="tableScrollElementRef" class="tableWrap">
          <table class="metaTableVue">
            <thead>
              <tr>
                <th
                  v-for="(column, columnIndex) in renderedColumns"
                  :key="`meta_col_${columnIndex}_${column.label}`"
                  :class="{ left: column.alignLeft }"
                >
                  {{ column.label }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="virtualTopSpacerPx > 0" class="spacerRow" aria-hidden="true">
                <td :colspan="Math.max(1, renderedColumns.length)" :style="{ height: `${virtualTopSpacerPx}px` }"></td>
              </tr>
              <tr v-for="virtualRow in virtualRows" :key="`meta_row_${virtualRow.index}`">
                <template v-for="cell in virtualRow.row.cells" :key="`meta_row_${virtualRow.index}_${cell.text}_${cell.className}`">
                  <td
                    :class="cell.className"
                    :title="cell.title || undefined"
                    :colspan="virtualRow.row.cells.length === 1 && renderedColumns.length > 1 ? renderedColumns.length : undefined"
                  >
                    {{ cell.text }}
                  </td>
                </template>
              </tr>
              <tr v-if="virtualBottomSpacerPx > 0" class="spacerRow" aria-hidden="true">
                <td :colspan="Math.max(1, renderedColumns.length)" :style="{ height: `${virtualBottomSpacerPx}px` }"></td>
              </tr>
            </tbody>
          </table>
        </div>
        <pre class="mono preview">{{ metaOutputPreview }}</pre>
      </section>

      <section v-if="showMetaTooling && compareTabActive" class="pane">
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
        <pre class="mono preview">{{ metaCompareSummary }}</pre>
      </section>

      <div class="status mono">{{ statusLine }}</div>
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
  align-items: center;
  gap: 0.75rem;
}

.title {
  font-size: 0.95rem;
  font-weight: 650;
}

.meta {
  font-size: 0.78rem;
  color: var(--muted);
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.pane h3 {
  margin: 0 0 0.55rem;
  font-size: 0.86rem;
}

.subtabs {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.subtabs button {
  padding: 0.4rem 0.58rem;
  font-size: 0.75rem;
}

.subtabs button.active {
  border-color: color-mix(in oklab, var(--accent) 65%, var(--border));
}

.summaryGrid {
  margin-top: 0.6rem;
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

.summaryGrid .k {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--muted);
}

.summaryGrid .v {
  margin-top: 0.2rem;
  font-size: 0.82rem;
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

.metaTableVue {
  width: 100%;
  border-collapse: collapse;
  min-width: 1080px;
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
}

.batchItem:hover {
  border-color: var(--border);
  background: color-mix(in oklab, var(--panel) 92%, var(--panel-footer));
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

.status {
  font-size: 0.78rem;
  color: var(--muted);
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
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
}

@media (max-width: 740px) {
  .summaryGrid,
  .gridFive,
  .gridFour,
  .gridThree {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
[data-vue-panel='meta'][data-vue-meta='1'] > :not(#metaVueShellMount) {
  display: none !important;
}
</style>

