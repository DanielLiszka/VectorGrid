<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { ArrowRight, CheckCircle2, Circle, CircleAlert, CircleDot, Clipboard, FileChartColumn, ListTodo, Play, Save, ShieldCheck } from 'lucide-vue-next'
import { useShellMount } from '../composables/useShellMount'
import { useGuidedWorkflowReadiness, type GuidedChecklistItem, type GuidedStatusTone } from '../composables/useGuidedWorkflowReadiness'
import {
  TASK_CENTER_PANEL_ID,
  openTaskCenter as requestTaskCenterOpen,
  useTaskCenterVisibility,
} from '../composables/useTaskCenterVisibility'
import { useTaskLifecycleSummary } from '../composables/useTaskLifecycleSummary'
import AppMetricStrip from './ui/AppMetricStrip.vue'
import { copyTextToDesktopClipboard } from '../services/desktopClipboard'
import { buildSimpleSummaryDefaultName, saveSimpleSummaryMarkdown } from '../services/simpleSummaryService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useExecutionStore } from '../stores/executionStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { type GuidedStep, useSettingsStore } from '../stores/settingsStore'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { buildSimpleRunSummary, formatSimpleRunSummaryMarkdown } from '../utils/simpleWorkflowModel'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const analysisStore = useAnalysisStore()
const dataStore = useDataStore()
const executionStore = useExecutionStore()
const optimizeStore = useOptimizeStore()
const settingsStore = useSettingsStore()
const validationStore = useValidationStore()
const workflowStore = useWorkflowStore()
const readiness = useGuidedWorkflowReadiness()
const taskCenterOpen = useTaskCenterVisibility()
const { activeTaskCount, centerStatusLabel, taskHeadline } = useTaskLifecycleSummary()

const { mountReady } = useShellMount('#guidedVueShellMount', {
  paneSelector: '#guidedShellGrid',
  paneDatasetKey: 'vueGuided',
})

const {
  bestStats,
  topRows,
} = storeToRefs(analysisStore)
const {
  canApplyTimeframe,
  canBrowseCsv,
  canClearData,
  canLoadCsv,
  canOpenMarketDataDir,
  canPolygonOpen,
  csvPath,
  dataFields,
  dataFieldHint,
  dataId,
  dataLen,
  dataTimeframe,
  dataTimeframeHint,
  datasetContext,
  datasetLabel,
  datasetRange,
  sampleCsvOptions,
  statusLine: dataStatusLine,
  updatedAtUnixMs: dataUpdatedAtUnixMs,
} = storeToRefs(dataStore)
const {
  backend,
  canCancel,
  objective,
  presetProfile,
  progressMeta,
  runDisabled,
  runPlanMeta,
  runReadinessHint,
  strategyKind,
  validateDisabled,
} = storeToRefs(optimizeStore)
const {
  tableRowCount,
  validationAccelerationStatus,
  validationOutputPreview,
  validationRouteSummary,
  validationShortlistMeta,
  validationWarningSummary,
} = storeToRefs(validationStore)
const canRunOnce = computed(() => executionStore.canRunOnce)
const canStartLive = computed(() => executionStore.canStartLive)
const canStartRunner = computed(() => executionStore.canStartRunner)
const canStopLive = computed(() => executionStore.canStopLive)
const canStopRunner = computed(() => executionStore.canStopRunner)
const dryRun = computed(() => executionStore.dryRun)
const feedStateLabel = computed(() => executionStore.feedStateLabel)
const hasExecutionCredentials = computed(() => executionStore.hasExecutionCredentials)
const keyStateDetail = computed(() => executionStore.keyStateDetail)
const keyStateLabel = computed(() => executionStore.keyStateLabel)
const liveStatusHint = computed(() => executionStore.liveStatusHint)
const executionMode = computed(() => executionStore.mode)
const qty = computed(() => executionStore.qty)
const requestedSymbolLabel = computed(() => executionStore.requestedSymbolLabel)
const runnerStateLabel = computed(() => executionStore.runnerStateLabel)
const runnerStatusHint = computed(() => executionStore.runnerStatusHint)
const executionStatusLine = computed(() => executionStore.statusLine)
const symbol = computed(() => executionStore.symbol)
const { guidedStep } = storeToRefs(settingsStore)

onMounted(async () => {
  await nextTick()
  dataStore.hydrateStartupState()
})

type GuidedStepMeta = {
  key: GuidedStep
  label: string
  subtitle: string
}

type GuidedPrimaryAction = 'data' | 'sample' | 'strategy' | 'optimize' | 'validate' | 'results' | 'runner'

const stepMeta: GuidedStepMeta[] = [
  { key: 'data', label: 'Data', subtitle: 'Load candles' },
  { key: 'strategy', label: 'Strategy', subtitle: 'Pick template' },
  { key: 'optimize', label: 'Optimize', subtitle: 'Find candidates' },
  { key: 'validate', label: 'Validate', subtitle: 'Optional evidence' },
]
const visibleGuidedStepKeys = new Set<GuidedStep>(stepMeta.map((step) => step.key))

const sampleIndexDraft = ref(0)
const timeframeDraft = ref('')
const copySummaryStatus = ref('')

watch(
  sampleCsvOptions,
  (options) => {
    const index = options.findIndex((option) => !option.disabled)
    sampleIndexDraft.value = Math.max(0, index)
  },
  { immediate: true },
)

watch(
  dataTimeframe,
  (value) => {
    timeframeDraft.value = String(value || '').trim()
  },
  { immediate: true },
)

const stamp = computed(() => formatUiTimeFromUnixMs(dataUpdatedAtUnixMs.value))
const activeStep = computed<GuidedStep>(() =>
  visibleGuidedStepKeys.has(guidedStep.value) ? guidedStep.value : 'validate',
)
const currentStrategyLabel = computed(() => {
  const value = String(strategyKind.value || '').trim() || 'DoubleMa'
  if (value === 'DoubleMa') return 'Double MA crossover'
  if (value === 'CompositeCpu') return 'Composite signal'
  return value
})
const datasetSummary = computed(() => {
  const label = String(datasetLabel.value || dataId.value || '').trim()
  if (!label) return 'No dataset loaded.'
  const parts = [label]
  if (String(dataTimeframe.value || '').trim()) parts.push(String(dataTimeframe.value).trim())
  if (String(dataLen.value || '').trim()) parts.push(`${dataLen.value} rows`)
  return parts.join(' · ')
})
const selectedSample = computed(() => sampleCsvOptions.value[sampleIndexDraft.value] ?? null)
const selectedSampleReady = computed(() => Boolean(selectedSample.value && !selectedSample.value.disabled))
const selectedSampleDetail = computed(() => {
  const sample = selectedSample.value
  if (!sample) return ''
  return String(
    sample.detail ||
      [
        sample.description,
        sample.timeframe ? `${sample.timeframe} candles` : '',
        sample.rowCount != null ? `${sample.rowCount.toLocaleString()} rows` : '',
        sample.range,
        sample.researchNote,
      ].filter(Boolean).join(' · '),
  ).trim()
})
const selectedCandidateTitle = computed(() => {
  if (!readiness.optimizationReady.value) return 'No ranked candidate yet'
  const title = String(readiness.selectedTopTitle.value || '').trim()
  if (!title || title === 'No ranked candidate selected.' || title === '—') return 'Best available candidate'
  return title
})
const validationPreviewText = computed(() => {
  const parts = [
    String(validationShortlistMeta.value || '').trim(),
    String(validationRouteSummary.value || '').trim(),
    String(validationWarningSummary.value || '').trim(),
  ].filter((part) => part && part !== '-')
  return parts.join(' ') || String(validationOutputPreview.value || '').trim() || 'No validation output yet.'
})
const simpleRunSummary = computed(() => buildSimpleRunSummary({
  datasetLabel: datasetLabel.value,
  dataId: dataId.value,
  dataTimeframe: dataTimeframe.value,
  dataLen: dataLen.value,
  datasetRange: datasetRange.value,
  strategyKind: strategyKind.value,
  objective: objective.value,
  presetProfile: presetProfile.value,
  backend: backend.value,
  optimizationReady: readiness.optimizationReady.value,
  selectedCandidateTitle: readiness.selectedTopTitle.value,
  selectedCandidateSummary: readiness.selectedTopSummary.value,
  topRows: topRows.value,
  bestStats: bestStats.value,
  validationReady: readiness.validationReady.value,
  validationRows: tableRowCount.value,
  validationWarnings: validationWarningSummary.value,
  validationVerdictTitle: readiness.validationVerdictTitle.value,
  validationVerdictBody: readiness.validationVerdictBody.value,
  validationVerdictTone: readiness.validationVerdictTone.value,
  validationPreview: validationPreviewText.value,
  validationAccelerationStatus: validationAccelerationStatus.value,
}))
const validationRecommendation = computed(() => simpleRunSummary.value.validation.recommendation)
const validationCandidateLabel = computed(() => (readiness.optimizationReady.value ? selectedCandidateTitle.value : 'Run optimization first'))
const validationWarningsLabel = computed(() => (
  readiness.validationReady.value
    ? String(validationWarningSummary.value || 'None reported')
    : 'Run validation after optimization'
))
const executionCredentialSummary = computed(() => {
  if (dryRun.value && !hasExecutionCredentials.value) return 'Not required for dry-run'
  return hasExecutionCredentials.value || dryRun.value ? keyStateLabel.value : 'Missing'
})
const advancedModeLabel = computed(() => {
  if (activeStep.value === 'data') return 'Open full data tools'
  if (activeStep.value === 'validate') return 'Open full validation report'
  return 'Open full strategy controls'
})
const guidedRunDisabled = computed(() =>
  Boolean(runDisabled.value || !readiness.datasetReady.value || !readiness.simpleOptimizationSupported.value),
)
const guidedValidateDisabled = computed(() =>
  Boolean(validateDisabled.value || !readiness.optimizationReady.value),
)
const guidedCanRunOnce = computed(() => Boolean(dryRun.value && canRunOnce.value))
const guidedCanStartRunner = computed(() => Boolean(dryRun.value && canStartRunner.value))
const guidedRunDisabledReason = computed(() => {
  if (!readiness.datasetReady.value) return 'Load data before optimization.'
  if (!readiness.simpleOptimizationSupported.value) return 'Choose Double MA or Composite before optimizing.'
  if (runDisabled.value) return String(runReadinessHint.value || 'Optimization is not ready yet.').trim()
  return ''
})
const guidedValidateDisabledReason = computed(() => {
  if (!readiness.optimizationReady.value) return 'Run optimization before validation.'
  if (validateDisabled.value) return 'Validation is not ready yet.'
  return ''
})
const optimizeStatusLabel = computed(() => (
  String(progressMeta.value || '').trim()
  || guidedRunDisabledReason.value
  || (readiness.optimizationReady.value ? 'Results ready.' : String(runReadinessHint.value || '').trim())
  || 'Ready to optimize.'
))
const primaryGuidedAction = computed<{
  kind: GuidedPrimaryAction
  label: string
  detail: string
}>(() => {
  if (!readiness.datasetReady.value) {
    if (selectedSampleReady.value) {
      return {
        kind: 'sample',
        label: 'Load sample data',
        detail: selectedSampleDetail.value || `${selectedSample.value?.label || 'Sample candles'} is ready for a first run.`,
      }
    }
    return {
      kind: 'data',
      label: 'Choose data',
      detail: 'Start by loading CSV, sample, or Polygon candles.',
    }
  }
  if (!readiness.simpleOptimizationSupported.value) {
    return {
      kind: 'strategy',
      label: 'Pick strategy',
      detail: 'Use Double MA or Composite for the streamlined Simple path.',
    }
  }
  if (!readiness.optimizationReady.value) {
    return {
      kind: 'optimize',
      label: 'Run optimization',
      detail: 'Generate candidate settings for the loaded dataset.',
    }
  }
  if (!readiness.validationReady.value) {
    return {
      kind: 'validate',
      label: 'Optional validation',
      detail: 'Optimization results are ready. Validation adds robustness evidence when you need it.',
    }
  }
  return {
    kind: 'results',
    label: 'Review results',
    detail: 'Optimization and validation outputs are ready for inspection or export.',
  }
})
const primaryGuidedActionDisabled = computed(() => {
  if (primaryGuidedAction.value.kind === 'sample') return !selectedSampleReady.value
  if (primaryGuidedAction.value.kind === 'optimize') return guidedRunDisabled.value
  if (primaryGuidedAction.value.kind === 'validate') return guidedValidateDisabled.value
  if (primaryGuidedAction.value.kind === 'runner') return !guidedCanStartRunner.value
  return false
})
const primaryGuidedActionTitle = computed(() => {
  if (!primaryGuidedActionDisabled.value) return primaryGuidedAction.value.label
  if (primaryGuidedAction.value.kind === 'sample') return 'No enabled sample dataset is available.'
  if (primaryGuidedAction.value.kind === 'optimize') return guidedRunDisabledReason.value
  if (primaryGuidedAction.value.kind === 'validate') return guidedValidateDisabledReason.value
  if (primaryGuidedAction.value.kind === 'results') return 'Review the full optimization and validation outputs.'
  if (primaryGuidedAction.value.kind === 'runner') return 'Confirm dry-run readiness and live candles before starting the runner.'
  return primaryGuidedAction.value.detail
})

const stepStatuses = computed<Record<GuidedStep, GuidedStatusTone>>(() => ({
  data: readiness.datasetReady.value ? 'done' : 'blocked',
  strategy: readiness.simpleOptimizationSupported.value ? 'done' : 'warning',
  optimize: readiness.optimizationReady.value ? 'done' : readiness.datasetReady.value && readiness.simpleOptimizationSupported.value ? 'ready' : 'blocked',
  validate: readiness.validationReady.value ? readiness.validationVerdictTone.value : readiness.optimizationReady.value ? 'ready' : 'blocked',
  trade: readiness.runnerReady.value ? 'ready' : readiness.validationReady.value ? 'warning' : 'blocked',
}))

function stepStatusLabel(tone: GuidedStatusTone, step?: GuidedStep): string {
  if (step === 'validate' && tone === 'blocked' && !readiness.optimizationReady.value) return 'Later'
  if (tone === 'done') return 'Done'
  if (tone === 'ready') return 'Ready'
  if (tone === 'warning') return 'Review'
  return 'Blocked'
}

function selectStep(step: GuidedStep): void {
  settingsStore.setGuidedStep(step)
  if (step === 'data') workflowStore.selectWorkflowStep('data', { persist: false })
  else if (step === 'strategy' || step === 'optimize') workflowStore.selectWorkflowStep('optimize', { persist: false })
  else if (step === 'validate') workflowStore.selectWorkflowStep('validate', { persist: false })
  else workflowStore.selectWorkflowStep('finalize', { persist: false })
}

function setTemplate(kind: 'DoubleMa' | 'CompositeCpu'): void {
  optimizeStore.setSelectField('strategyKind', kind)
  optimizeStore.setSelectField('presetProfile', 'Balanced')
  optimizeStore.setSelectField('backend', 'Auto')
  optimizeStore.setSelectField('objective', 'Sharpe')
  optimizeStore.setSelectField('mode', 'Grid')
  settingsStore.setGuidedStep('optimize')
  workflowStore.selectWorkflowStep('optimize', { persist: false })
}

function loadSelectedSample(): void {
  const option = selectedSample.value
  if (!option || option.disabled) return
  dataStore.loadSampleCsv(sampleIndexDraft.value, option.path)
}

function applyTimeframe(): void {
  dataStore.applyTimeframe(timeframeDraft.value)
}

function runOptimization(): void {
  optimizeStore.runOptimization()
}

function runValidation(): void {
  optimizeStore.runValidation()
}

async function copySimpleSummary(): Promise<void> {
  copySummaryStatus.value = 'Copying summary...'
  const result = await copyTextToDesktopClipboard(
    formatSimpleRunSummaryMarkdown(simpleRunSummary.value),
  )
  copySummaryStatus.value = result.ok
    ? 'Copied Simple summary.'
    : `Copy failed${result.error ? `: ${result.error}` : '.'}`
}

async function saveSimpleSummary(): Promise<void> {
  copySummaryStatus.value = 'Saving summary...'
  try {
    const saved = await saveSimpleSummaryMarkdown(
      formatSimpleRunSummaryMarkdown(simpleRunSummary.value),
      buildSimpleSummaryDefaultName(simpleRunSummary.value.dataset.label),
    )
    copySummaryStatus.value = saved
      ? `Saved Simple summary: ${saved.path}`
      : 'Save canceled.'
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error || '')
    copySummaryStatus.value = `Save failed${message ? `: ${message}` : '.'}`
  }
}

function runPrimaryGuidedAction(): void {
  const action = primaryGuidedAction.value.kind
  if (action === 'data') selectStep('data')
  else if (action === 'sample') {
    selectStep('data')
    loadSelectedSample()
  }
  else if (action === 'strategy') selectStep('strategy')
  else if (action === 'optimize') runOptimization()
  else if (action === 'validate') runValidation()
  else if (action === 'results') openAdvanced(readiness.validationReady.value ? 'validate' : 'optimize')
  else executionStore.startRunner()
}

function openAdvanced(area: 'data' | 'optimize' | 'validate' | 'aggregate' | 'live'): void {
  settingsStore.setUiMode('advanced')
  if (area === 'data') workflowStore.selectWorkspace('workflow', 'data')
  else if (area === 'optimize') workflowStore.selectWorkflowStep('optimize')
  else if (area === 'validate') workflowStore.selectWorkflowStep('validate')
  else if (area === 'aggregate') workflowStore.selectWorkspace('aggregate')
  else workflowStore.selectWorkspace('live')
}

function openAdvancedForActiveStep(): void {
  if (activeStep.value === 'data') openAdvanced('data')
  else if (activeStep.value === 'validate') openAdvanced('validate')
  else openAdvanced('optimize')
}

function openTaskCenter(): void {
  requestTaskCenterOpen(taskCenterOpen)
}

function checklistIcon(item: GuidedChecklistItem): typeof CheckCircle2 {
  if (item.tone === 'done') return CheckCircle2
  if (item.tone === 'ready') return CircleDot
  if (item.tone === 'warning') return CircleAlert
  return Circle
}
</script>

<template>
  <Teleport v-if="mountReady" to="#guidedVueShellMount">
    <section class="guidedRoot" aria-label="Simple workflow" data-qa-panel="simple.workflow">
      <header class="guidedHero">
        <div class="guidedHeroCopy">
          <div class="guidedEyebrow">Simple Mode</div>
          <h2>Optimize backtests. Validate when needed.</h2>
          <p>
            Simple view keeps the standard workflow focused: load candles,
            choose a strategy, run a backtest optimization, then optionally
            validate the leading candidate.
          </p>
        </div>
        <div class="guidedHeroActions">
          <div class="guidedNextAction" aria-label="Recommended next action">
            <span>Next</span>
            <strong>{{ primaryGuidedAction.label }}</strong>
            <small>{{ primaryGuidedAction.detail }}</small>
            <button type="button" class="guidedPrimary" data-qa-action="guided.primaryAction.run" :disabled="primaryGuidedActionDisabled" :title="primaryGuidedActionTitle" @click="runPrimaryGuidedAction">
              <ArrowRight :size="15" aria-hidden="true" />
              {{ primaryGuidedAction.label }}
            </button>
          </div>
          <button
            type="button"
            class="guidedSecondary"
            :class="{ active: activeTaskCount > 0 || taskCenterOpen }"
            :aria-controls="TASK_CENTER_PANEL_ID"
            :aria-expanded="taskCenterOpen ? 'true' : 'false'"
            aria-haspopup="dialog"
            :aria-label="`Open task center. ${centerStatusLabel}. ${taskHeadline}`"
            @click="openTaskCenter"
          >
            <ListTodo :size="15" aria-hidden="true" />
            {{ centerStatusLabel }}
          </button>
          <button type="button" class="guidedSecondary" data-qa-action="guided.advancedMode.open" :aria-label="advancedModeLabel" @click="openAdvancedForActiveStep">
            {{ advancedModeLabel }}
          </button>
        </div>
      </header>

      <nav class="guidedSteps" aria-label="Simple workflow steps">
          <button
            v-for="step in stepMeta"
            :key="step.key"
            type="button"
            class="guidedStepButton"
            data-qa-action="guided.step.select"
          :class="[stepStatuses[step.key], { active: activeStep === step.key }]"
          :aria-current="activeStep === step.key ? 'step' : undefined"
          @click="selectStep(step.key)"
        >
          <component :is="checklistIcon({ label: step.label, detail: step.subtitle, complete: stepStatuses[step.key] === 'done', tone: stepStatuses[step.key] })" :size="16" aria-hidden="true" />
          <span class="guidedStepText">
            <span class="guidedStepLabel">{{ step.label }}</span>
            <span class="guidedStepSubtitle">{{ step.subtitle }}</span>
          </span>
          <span class="guidedStepState">{{ stepStatusLabel(stepStatuses[step.key], step.key) }}</span>
        </button>
      </nav>

      <section class="guidedLayout">
        <section class="guidedRail" aria-label="Simple checklist">
          <div class="guidedRailHeader">
            <div class="guidedEyebrow">Readiness</div>
            <h3>Backtest path</h3>
          </div>
          <ol class="guidedChecklist">
            <li v-for="item in readiness.guidedChecklist.value" :key="item.label" :class="item.tone">
              <component :is="checklistIcon(item)" :size="16" aria-hidden="true" />
              <span>
                <strong>{{ item.label }}</strong>
                <small>{{ item.detail }}</small>
              </span>
            </li>
          </ol>
          <div class="guidedAdvancedNote">
            <strong>Need the full workstation?</strong>
            Advanced Mode keeps live trading, ML, options, campaign,
            diagnostic, and artifact replay tools available without crowding
            this path.
          </div>
          <button type="button" class="guidedSecondary fullWidth" data-qa-action="guided.advancedResearch.open" @click="openAdvanced('aggregate')">
            Open advanced research tools
          </button>
        </section>

        <section class="guidedMain" aria-label="Simple step detail">
          <section v-if="activeStep === 'data'" class="guidedPane" aria-label="Simple data setup" data-qa-panel="simple.data">
            <div class="guidedPaneHeader">
              <div>
                <div class="guidedEyebrow">Step 1</div>
                <h3>Choose data</h3>
                <p>Load candles first. Everything downstream uses this dataset.</p>
              </div>
              <span class="guidedStamp mono">{{ stamp }}</span>
            </div>

            <div class="guidedSummaryGrid">
              <article>
                <span>Dataset</span>
                <strong>{{ datasetSummary }}</strong>
              </article>
              <article>
                <span>Range</span>
                <strong>{{ datasetRange || 'No range yet' }}</strong>
              </article>
              <article>
                <span>Fields</span>
                <strong>{{ dataFields.length ? dataFields.join(', ') : 'No fields loaded' }}</strong>
              </article>
            </div>

            <div class="guidedFormGrid">
              <label>
                CSV path
                <input
                  type="text"
                  :value="csvPath"
                  placeholder="Browse or paste a CSV path"
                  @input="dataStore.setInputField('csvPath', ($event.target as HTMLInputElement).value)"
                />
              </label>
              <label>
                Sample data
                <select v-model.number="sampleIndexDraft">
                  <option
                    v-for="(option, index) in sampleCsvOptions"
                    :key="`${option.path}_${index}`"
                    :value="index"
                    :disabled="option.disabled"
                  >
                    {{ option.label }}
                  </option>
                </select>
              </label>
              <label>
                Timeframe
                <input
                  type="text"
                  v-model="timeframeDraft"
                  placeholder="blank keeps base timeframe"
                />
              </label>
            </div>

            <p v-if="selectedSampleDetail" class="guidedMuted">{{ selectedSampleDetail }}</p>

            <div class="guidedActions">
              <button type="button" class="guidedPrimary" data-qa-action="guided.data.sample.load" :disabled="!selectedSampleReady" @click="loadSelectedSample">
                Load sample
              </button>
              <button type="button" class="guidedSecondary" data-qa-action="guided.data.csv.browse" :disabled="!canBrowseCsv" @click="dataStore.browseCsv()">
                Browse CSV
              </button>
              <button type="button" class="guidedSecondary" data-qa-action="guided.data.csv.load" :disabled="!canLoadCsv" @click="dataStore.loadCsv()">
                Load CSV
              </button>
              <button type="button" class="guidedSecondary" data-qa-action="guided.data.polygon.open" :disabled="!canPolygonOpen" @click="dataStore.openPolygon()">
                Polygon data
              </button>
              <button type="button" class="guidedSecondary" data-qa-action="guided.data.timeframe.apply" :disabled="!canApplyTimeframe" @click="applyTimeframe">
                Apply timeframe
              </button>
              <button type="button" class="guidedSecondary" data-qa-action="guided.data.clear" :disabled="!canClearData" @click="dataStore.clearData()">
                Clear
              </button>
            </div>

            <p class="guidedMuted" role="status" aria-live="polite">{{ dataStatusLine }}</p>
            <p v-if="dataFieldHint" class="guidedWarning">{{ dataFieldHint }}</p>
            <p class="guidedMuted">{{ dataTimeframeHint }}</p>
            <p class="guidedMuted mono">{{ datasetContext }}</p>
            <button type="button" class="guidedTextLink" :disabled="!canOpenMarketDataDir" @click="dataStore.openMarketDataDir()">
              Open market-data folder
            </button>
          </section>

          <section v-else-if="activeStep === 'strategy'" class="guidedPane" aria-label="Simple strategy setup" data-qa-panel="simple.strategy">
            <div class="guidedPaneHeader">
              <div>
                <div class="guidedEyebrow">Step 2</div>
                <h3>Pick a conventional strategy</h3>
                <p>Simple mode starts with stable backtest templates and sensible defaults.</p>
              </div>
            </div>

            <div class="guidedTemplateGrid">
              <article class="guidedTemplate">
                <div class="guidedTemplateHeader">
                  <ShieldCheck :size="18" aria-hidden="true" />
                  <h4>Double MA crossover</h4>
                </div>
                <p>Simple trend-following baseline with fast and slow moving averages.</p>
                <button type="button" class="guidedPrimary" data-qa-action="guided.strategy.template.doubleMa" @click="setTemplate('DoubleMa')">
                  Use Double MA
                </button>
              </article>
              <article class="guidedTemplate">
                <div class="guidedTemplateHeader">
                  <ShieldCheck :size="18" aria-hidden="true" />
                  <h4>Composite signal</h4>
                </div>
                <p>Conventional multi-signal template for optimization and validation.</p>
                <button type="button" class="guidedPrimary" data-qa-action="guided.strategy.template.compositeCpu" @click="setTemplate('CompositeCpu')">
                  Use Composite
                </button>
              </article>
            </div>

            <div class="guidedSummaryGrid">
              <article>
                <span>Current strategy</span>
                <strong>{{ currentStrategyLabel }}</strong>
              </article>
              <article>
                <span>Objective</span>
                <strong>{{ objective }}</strong>
              </article>
              <article>
                <span>Preset</span>
                <strong>{{ presetProfile }}</strong>
              </article>
            </div>

            <div class="guidedFormGrid compact">
              <label>
                Objective
                <select :value="objective" @change="optimizeStore.setSelectField('objective', ($event.target as HTMLSelectElement).value)">
                  <option value="Sharpe">Sharpe</option>
                  <option value="Sortino">Sortino</option>
                  <option value="Return">Return</option>
                  <option value="Calmar">Calmar</option>
                </select>
              </label>
              <label>
                Search depth
                <select :value="presetProfile" @change="optimizeStore.setSelectField('presetProfile', ($event.target as HTMLSelectElement).value)">
                  <option value="Quick">Quick</option>
                  <option value="Balanced">Balanced</option>
                  <option value="Deep">Deep</option>
                </select>
              </label>
              <label>
                Engine
                <select :value="backend" @change="optimizeStore.setSelectField('backend', ($event.target as HTMLSelectElement).value)">
                  <option value="Auto">Auto</option>
                  <option value="CpuOnly">CPU only</option>
                </select>
              </label>
            </div>
            <p class="guidedMuted">
              Advanced Mode keeps the full engine and route controls, including GPU and hybrid routes.
            </p>

            <div class="guidedActions">
              <button type="button" class="guidedPrimary" data-qa-action="guided.strategy.continueToOptimize" @click="selectStep('optimize')">
                Continue to optimize
              </button>
              <button type="button" class="guidedSecondary" data-qa-action="guided.optimize.advanced.open" @click="openAdvanced('optimize')">
                Full strategy controls
              </button>
            </div>
          </section>

          <section v-else-if="activeStep === 'optimize'" class="guidedPane" aria-label="Simple optimization" data-qa-panel="simple.optimize">
            <div class="guidedPaneHeader">
              <div>
                <div class="guidedEyebrow">Step 3</div>
                <h3>Run optimization</h3>
                <p>Score candidate settings against the loaded dataset.</p>
              </div>
              <span class="guidedStatusPill" :class="readiness.optimizationReady.value ? 'done' : 'blocked'">
                {{ readiness.optimizationReady.value ? 'Results ready' : 'Needs run' }}
              </span>
            </div>

            <div class="guidedSummaryGrid">
              <article>
                <span>Strategy</span>
                <strong>{{ currentStrategyLabel }}</strong>
              </article>
              <article>
                <span>Engine</span>
                <strong>{{ simpleRunSummary.strategy.engineLabel }}</strong>
              </article>
              <article>
                <span>Status</span>
                <strong>{{ optimizeStatusLabel }}</strong>
              </article>
            </div>

            <p class="guidedMuted mono">{{ runPlanMeta }}</p>
            <div class="guidedActions">
              <button v-if="canCancel" type="button" class="guidedDanger" data-qa-action="guided.optimize.cancel" @click="optimizeStore.cancelOptimization()">
                Stop run
              </button>
              <button v-else type="button" class="guidedPrimary" data-qa-action="guided.optimize.run" :disabled="guidedRunDisabled" :title="guidedRunDisabledReason || 'Run optimization'" @click="runOptimization">
                <Play :size="15" aria-hidden="true" />
                Run optimization
              </button>
              <button type="button" class="guidedSecondary" data-qa-action="guided.optimize.continueToValidate" :disabled="!readiness.optimizationReady.value" @click="selectStep('validate')">
                Continue to validate
              </button>
              <button type="button" class="guidedSecondary" @click="openAdvanced('optimize')">
                Open results in Advanced
              </button>
            </div>
            <p v-if="guidedRunDisabledReason" class="guidedMuted" role="status" aria-live="polite">{{ guidedRunDisabledReason }}</p>

            <section class="guidedCandidate guidedRunSummary" aria-label="Simple run summary" data-qa-state="guided.optimization.summary">
              <div class="guidedEyebrow">Run summary</div>
              <h4>{{ simpleRunSummary.optimization.candidateTitle }}</h4>
              <p>{{ simpleRunSummary.optimization.candidateSummary }}</p>
              <AppMetricStrip
                v-if="simpleRunSummary.optimization.metricItems.length"
                :items="simpleRunSummary.optimization.metricItems"
                aria-label="Simple optimization metrics"
              />
              <div class="guidedActions">
                <button type="button" class="guidedSecondary" data-qa-action="guided.summary.copy" :disabled="!readiness.optimizationReady.value" @click="copySimpleSummary">
                  <Clipboard :size="15" aria-hidden="true" />
                  Copy summary
                </button>
                <button type="button" class="guidedSecondary" data-qa-action="guided.summary.save" :disabled="!readiness.optimizationReady.value" @click="saveSimpleSummary">
                  <Save :size="15" aria-hidden="true" />
                  Save summary
                </button>
              </div>
              <p v-if="copySummaryStatus" class="guidedMuted" role="status" aria-live="polite">{{ copySummaryStatus }}</p>
            </section>
          </section>

          <section v-else-if="activeStep === 'validate'" class="guidedPane" aria-label="Simple validation" data-qa-panel="simple.validate">
            <div class="guidedPaneHeader">
              <div>
                <div class="guidedEyebrow">Step 4</div>
                <h3>{{ validationRecommendation.title }}</h3>
                <p>{{ validationRecommendation.body }}</p>
              </div>
              <span class="guidedStatusPill" :class="validationRecommendation.tone">
                {{ stepStatusLabel(validationRecommendation.tone, 'validate') }}
              </span>
            </div>

            <div class="guidedSummaryGrid">
              <article>
                <span>Evidence</span>
                <strong>{{ simpleRunSummary.validation.rowsLabel }}</strong>
              </article>
              <article>
                <span>Candidate</span>
                <strong>{{ validationCandidateLabel }}</strong>
              </article>
              <article>
                <span>Warnings</span>
                <strong>{{ validationWarningsLabel }}</strong>
              </article>
              <article>
                <span>Engine</span>
                <strong>{{ simpleRunSummary.validation.accelerationLabel }}</strong>
              </article>
            </div>

            <section class="guidedCandidate guidedRecommendation" :class="validationRecommendation.tone" aria-label="Validation recommendation" data-qa-state="guided.validation.recommendation">
              <div class="guidedEyebrow">Recommendation</div>
              <h4>{{ validationRecommendation.title }}</h4>
              <p>{{ validationRecommendation.body }}</p>
              <AppMetricStrip
                :items="simpleRunSummary.validation.metricItems"
                aria-label="Simple validation metrics"
              />
            </section>
            <p class="guidedMuted" role="status" aria-live="polite">{{ validationPreviewText }}</p>
            <div class="guidedActions">
              <button type="button" class="guidedPrimary" data-qa-action="guided.validation.run" :disabled="guidedValidateDisabled" :title="guidedValidateDisabledReason || 'Run validation'" @click="runValidation">
                Run validation
              </button>
              <button type="button" class="guidedSecondary" @click="openAdvanced('optimize')">
                Review optimization results
              </button>
              <button type="button" class="guidedSecondary" data-qa-action="guided.validation.advanced.open" @click="openAdvanced('validate')">
                Full validation report
              </button>
              <button type="button" class="guidedSecondary" data-qa-action="guided.summary.copy" :disabled="!readiness.optimizationReady.value" @click="copySimpleSummary">
                <Clipboard :size="15" aria-hidden="true" />
                Copy summary
              </button>
              <button type="button" class="guidedSecondary" data-qa-action="guided.summary.save" :disabled="!readiness.optimizationReady.value" @click="saveSimpleSummary">
                <Save :size="15" aria-hidden="true" />
                Save summary
              </button>
            </div>
            <p v-if="guidedValidateDisabledReason" class="guidedMuted" role="status" aria-live="polite">{{ guidedValidateDisabledReason }}</p>
            <p v-if="copySummaryStatus" class="guidedMuted" role="status" aria-live="polite">{{ copySummaryStatus }}</p>
            <section class="guidedCandidate" aria-label="Optional live execution handoff">
              <div class="guidedEyebrow">Optional next step</div>
              <h4>Paper/live trading stays in Advanced Mode</h4>
              <p>Simple Mode stops at optimization and optional validation. Use Advanced Mode when you are ready to review dry-run, broker, and feed controls.</p>
              <button type="button" class="guidedSecondary" data-qa-action="guided.validation.continueToTrade" @click="openAdvanced('live')">
                <FileChartColumn :size="15" aria-hidden="true" />
                Advanced live tools
              </button>
            </section>
          </section>

          <section v-else class="guidedPane" aria-label="Simple paper and live readiness">
            <div class="guidedPaneHeader">
              <div>
                <div class="guidedEyebrow">Step 5</div>
                <h3>Prepare paper/live trading</h3>
                <p>Start with dry-run. Broker-armed trading should stay gated by credentials, live candles, and explicit review.</p>
              </div>
            </div>

            <ol class="guidedTradeChecklist">
              <li v-for="item in readiness.tradeChecklist.value" :key="item.label" :class="item.tone">
                <component :is="checklistIcon(item)" :size="16" aria-hidden="true" />
                <span>
                  <strong>{{ item.label }}</strong>
                  <small>{{ item.detail }}</small>
                </span>
              </li>
            </ol>

            <div class="guidedFormGrid compact">
              <label>
                Symbol
                <input
                  type="text"
                  :value="symbol"
                  placeholder="AAPL"
                  @input="executionStore.setInputField('execSymbol', ($event.target as HTMLInputElement).value)"
                />
              </label>
              <label>
                Quantity
                <input
                  type="text"
                  :value="qty"
                  placeholder="1"
                  @input="executionStore.setInputField('execQty', ($event.target as HTMLInputElement).value)"
                />
              </label>
              <label>
                Broker mode
                <select :value="executionMode" @change="executionStore.setSelectField('execMode', ($event.target as HTMLSelectElement).value)">
                  <option value="paper">Paper</option>
                  <option value="live">Live</option>
                </select>
              </label>
            </div>

            <label class="guidedCheck">
              <input
                type="checkbox"
                :checked="dryRun"
                @change="executionStore.setCheckboxField('execDryRun', ($event.target as HTMLInputElement).checked)"
              />
              Dry-run only
            </label>
            <p v-if="!dryRun" class="guidedWarning">
              Dry-run is off. Simple Mode will not start the auto-runner in this state; open Advanced Mode for broker-armed launch after manual review.
            </p>

            <div class="guidedSummaryGrid">
              <article>
                <span>Symbol</span>
                <strong>{{ requestedSymbolLabel }}</strong>
              </article>
              <article>
                <span>Live feed</span>
                <strong>{{ feedStateLabel }}</strong>
              </article>
              <article>
                <span>Runner</span>
                <strong>{{ runnerStateLabel }}</strong>
              </article>
              <article>
                <span>Credentials</span>
                <strong>{{ executionCredentialSummary }}</strong>
              </article>
            </div>

            <div class="guidedActions">
              <button type="button" class="guidedSecondary" :disabled="!guidedCanRunOnce" @click="executionStore.runOnce()">
                Simulate once
              </button>
              <button v-if="canStopLive" type="button" class="guidedSecondary" @click="executionStore.stopLive()">
                Stop live candles
              </button>
              <button v-else type="button" class="guidedPrimary" :disabled="!canStartLive" @click="executionStore.startLive()">
                Start live candles
              </button>
              <button v-if="canStopRunner" type="button" class="guidedDanger" @click="executionStore.stopRunner()">
                Stop auto-runner
              </button>
              <button v-else type="button" class="guidedPrimary" :disabled="!guidedCanStartRunner" @click="executionStore.startRunner()">
                Start dry-run runner
              </button>
              <button type="button" class="guidedSecondary" @click="openAdvanced('live')">
                Full live workspace
              </button>
            </div>

            <p class="guidedMuted" role="status" aria-live="polite">{{ liveStatusHint }}</p>
            <p class="guidedMuted">{{ runnerStatusHint }}</p>
            <p class="guidedMuted">{{ executionStatusLine }}</p>
            <p class="guidedMuted">{{ keyStateDetail }}</p>
          </section>
        </section>
      </section>
    </section>
  </Teleport>
</template>

<style scoped>
.guidedRoot {
  display: grid;
  gap: 1rem;
  min-width: 0;
}

.guidedHero,
.guidedPane,
.guidedRail {
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--panel) 98%, var(--surface));
  box-shadow: var(--shadow-sm);
}

.guidedHero {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  padding: 1rem 1.1rem;
}

.guidedHeroCopy,
.guidedRailHeader,
.guidedPaneHeader > div {
  display: grid;
  gap: 0.18rem;
}

.guidedHero h2,
.guidedPane h3,
.guidedRail h3,
.guidedCandidate h4,
.guidedTemplate h4 {
  margin: 0;
  overflow-wrap: anywhere;
}

.guidedHero p,
.guidedPane p,
.guidedTemplate p {
  margin: 0;
  color: var(--muted-strong);
  line-height: 1.45;
}

.guidedHeroActions {
  display: grid;
  justify-items: end;
  align-content: start;
  gap: 0.65rem;
  min-width: min(20rem, 42vw);
}

.guidedNextAction {
  display: grid;
  justify-items: end;
  gap: 0.2rem;
  text-align: right;
}

.guidedNextAction span {
  font-size: 0.64rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.guidedNextAction strong {
  font-size: 1rem;
}

.guidedNextAction small {
  max-width: 22rem;
  color: var(--muted-strong);
  line-height: 1.35;
  overflow-wrap: anywhere;
}

.guidedEyebrow {
  font-size: 0.64rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.guidedSteps {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.55rem;
}

.guidedStepButton {
  min-width: 0;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 0.55rem;
  text-align: left;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-working) 96%, var(--panel));
  padding: 0.7rem 0.75rem;
}

.guidedStepButton.active {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 70%, var(--accent));
  background: color-mix(in oklab, var(--selection-fill) 82%, var(--surface-working));
  box-shadow: inset 0 -2px 0 var(--selection-rail);
}

.guidedStepButton.done {
  color: var(--success-ink, var(--text));
}

.guidedStepButton.warning {
  color: var(--warning-ink, var(--text));
}

.guidedStepButton:focus-visible,
.guidedPrimary:focus-visible,
.guidedSecondary:focus-visible,
.guidedDanger:focus-visible,
.guidedTextLink:focus-visible {
  outline: none;
  border-color: var(--focus-border);
  box-shadow: var(--focus-ring);
}

.guidedStepText {
  min-width: 0;
  display: grid;
  gap: 0.05rem;
}

.guidedStepLabel,
.guidedStepState {
  font-size: 0.76rem;
  font-weight: 700;
}

.guidedStepSubtitle {
  color: var(--muted);
  font-size: 0.68rem;
}

.guidedStepState {
  color: var(--muted-strong);
}

.guidedLayout {
  display: grid;
  grid-template-columns: minmax(17rem, 0.32fr) minmax(0, 1fr);
  gap: 1rem;
}

.guidedRail,
.guidedPane {
  padding: 1rem;
}

.guidedRail {
  display: grid;
  align-content: start;
  gap: 0.9rem;
  background: color-mix(in oklab, var(--surface-shell) 88%, var(--panel));
}

.guidedChecklist,
.guidedTradeChecklist {
  margin: 0;
  padding: 0;
  list-style: none;
  display: grid;
  gap: 0.55rem;
}

.guidedChecklist li,
.guidedTradeChecklist li {
  min-width: 0;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.55rem;
  align-items: start;
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface) 96%, var(--panel));
  padding: 0.62rem 0.68rem;
}

.guidedChecklist li.done,
.guidedTradeChecklist li.done,
.guidedChecklist li.ready,
.guidedTradeChecklist li.ready {
  border-color: color-mix(in oklab, var(--success, var(--accent)) 34%, var(--border));
}

.guidedChecklist li.warning,
.guidedTradeChecklist li.warning {
  border-color: color-mix(in oklab, var(--warning, var(--accent)) 38%, var(--border));
}

.guidedChecklist strong,
.guidedTradeChecklist strong {
  display: block;
  font-size: 0.78rem;
}

.guidedChecklist small,
.guidedTradeChecklist small {
  display: block;
  color: var(--muted-strong);
  line-height: 1.35;
}

.guidedAdvancedNote {
  border-top: 1px dashed var(--border);
  padding-top: 0.85rem;
  color: var(--muted-strong);
  font-size: 0.78rem;
  line-height: 1.45;
}

.guidedAdvancedNote strong {
  display: block;
  color: var(--text);
  margin-bottom: 0.18rem;
}

.guidedMain {
  min-width: 0;
}

.guidedPane {
  display: grid;
  gap: 1rem;
  min-height: 100%;
}

.guidedPaneHeader {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
}

.guidedStamp,
.guidedMuted,
.guidedWarning {
  font-size: 0.78rem;
}

.guidedMuted {
  color: var(--muted-strong);
}

.guidedWarning {
  border: 1px solid color-mix(in oklab, var(--warning, var(--accent)) 42%, var(--border));
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-warning, var(--surface-working)) 86%, var(--panel));
  color: var(--warning-ink, var(--text));
  padding: 0.62rem 0.7rem;
}

.guidedSummaryGrid,
.guidedFormGrid,
.guidedTemplateGrid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.65rem;
}

.guidedSummaryGrid article,
.guidedTemplate,
.guidedCandidate {
  min-width: 0;
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-working) 96%, var(--panel));
  padding: 0.75rem;
  display: grid;
  gap: 0.2rem;
}

.guidedRunSummary,
.guidedRecommendation {
  gap: 0.6rem;
}

.guidedCandidate.done {
  border-color: color-mix(in oklab, var(--success, var(--accent)) 34%, var(--border));
}

.guidedCandidate.ready {
  border-color: color-mix(in oklab, var(--accent) 34%, var(--border));
}

.guidedCandidate.warning {
  border-color: color-mix(in oklab, var(--warning, var(--accent)) 40%, var(--border));
}

.guidedSummaryGrid span {
  font-size: 0.64rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.guidedSummaryGrid strong {
  min-width: 0;
  font-size: 0.82rem;
  overflow-wrap: anywhere;
}

.guidedFormGrid label {
  min-width: 0;
  display: grid;
  gap: 0.28rem;
  font-size: 0.72rem;
  color: var(--muted-strong);
}

.guidedFormGrid input,
.guidedFormGrid select {
  width: 100%;
}

.guidedFormGrid.compact {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.guidedTemplateGrid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.guidedTemplateHeader {
  display: flex;
  align-items: center;
  gap: 0.48rem;
}

.guidedActions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
  align-items: center;
}

.guidedPrimary,
.guidedSecondary,
.guidedDanger,
.guidedTextLink {
  max-width: 100%;
  min-height: 2.25rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.42rem;
  border-radius: 8px;
  padding: 0.42rem 0.72rem;
  font-size: 0.76rem;
  font-weight: 700;
  text-align: center;
}

.guidedPrimary {
  border: 1px solid color-mix(in oklab, var(--accent) 58%, var(--border));
  background: color-mix(in oklab, var(--selection-fill) 86%, var(--surface-working));
  color: var(--selection-text);
  box-shadow: inset 0 -2px 0 var(--selection-rail);
}

.guidedSecondary,
.guidedTextLink {
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  background: color-mix(in oklab, var(--surface-inline) 88%, var(--panel));
  color: var(--text);
}

.guidedSecondary.active {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 42%, var(--border));
  background: color-mix(in oklab, var(--selection-fill) 84%, var(--surface-working));
}

.guidedDanger {
  border: 1px solid color-mix(in oklab, var(--danger) 48%, var(--border));
  background: color-mix(in oklab, var(--surface-danger) 92%, var(--surface-working));
  color: var(--danger-ink);
}

.guidedPrimary:disabled,
.guidedSecondary:disabled,
.guidedDanger:disabled,
.guidedTextLink:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.guidedTextLink {
  width: fit-content;
  min-height: 2rem;
}

.fullWidth {
  width: 100%;
}

.guidedStatusPill {
  flex: 0 0 auto;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 999px;
  padding: 0.24rem 0.58rem;
  font-size: 0.7rem;
  font-weight: 700;
  color: var(--muted-strong);
}

.guidedStatusPill.done,
.guidedStatusPill.ready {
  border-color: color-mix(in oklab, var(--success, var(--accent)) 34%, var(--border));
}

.guidedStatusPill.warning {
  border-color: color-mix(in oklab, var(--warning, var(--accent)) 42%, var(--border));
}

.guidedCheck {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  width: fit-content;
  font-weight: 700;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1180px) {
  .guidedLayout {
    grid-template-columns: 1fr;
  }

  .guidedSteps,
  .guidedSummaryGrid,
  .guidedFormGrid,
  .guidedFormGrid.compact {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 760px) {
  .guidedHero,
  .guidedPaneHeader {
    flex-direction: column;
  }

  .guidedHeroActions,
  .guidedNextAction {
    justify-items: start;
    text-align: left;
    min-width: 0;
  }

  .guidedSteps,
  .guidedSummaryGrid,
  .guidedFormGrid,
  .guidedFormGrid.compact,
  .guidedTemplateGrid {
    grid-template-columns: 1fr;
  }

  .guidedMain {
    order: 1;
  }

  .guidedRail {
    order: 2;
  }

  .guidedActions > button {
    width: 100%;
  }
}
</style>
