<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { CheckCircle2, Circle, CircleAlert, CircleDot, Play, ShieldCheck } from 'lucide-vue-next'
import { useShellMount } from '../composables/useShellMount'
import { useGuidedWorkflowReadiness, type GuidedChecklistItem, type GuidedStatusTone } from '../composables/useGuidedWorkflowReadiness'
import { useDataStore } from '../stores/dataStore'
import { useExecutionStore } from '../stores/executionStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { type GuidedStep, useSettingsStore } from '../stores/settingsStore'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const dataStore = useDataStore()
const executionStore = useExecutionStore()
const optimizeStore = useOptimizeStore()
const settingsStore = useSettingsStore()
const validationStore = useValidationStore()
const workflowStore = useWorkflowStore()
const readiness = useGuidedWorkflowReadiness()

const { mountReady } = useShellMount('#guidedVueShellMount', {
  paneSelector: '#guidedShellGrid',
  paneDatasetKey: 'vueGuided',
})

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
  validationOutputPreview,
  validationRouteSummary,
  validationShortlistMeta,
  validationWarningSummary,
} = storeToRefs(validationStore)
const {
  canRunOnce,
  canStartLive,
  canStartRunner,
  canStopLive,
  canStopRunner,
  dryRun,
  feedStateLabel,
  hasExecutionCredentials,
  keyStateDetail,
  keyStateLabel,
  liveStatusHint,
  mode: executionMode,
  qty,
  requestedSymbolLabel,
  runnerStateLabel,
  runnerStatusHint,
  statusLine: executionStatusLine,
  symbol,
} = storeToRefs(executionStore)
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

const stepMeta: GuidedStepMeta[] = [
  { key: 'data', label: 'Data', subtitle: 'Load candles' },
  { key: 'strategy', label: 'Strategy', subtitle: 'Pick template' },
  { key: 'optimize', label: 'Optimize', subtitle: 'Find candidates' },
  { key: 'validate', label: 'Validate', subtitle: 'Check robustness' },
  { key: 'trade', label: 'Trade Ready', subtitle: 'Dry-run first' },
]

const sampleIndexDraft = ref(0)
const timeframeDraft = ref('')

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
const activeStep = computed<GuidedStep>(() => guidedStep.value)
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
const selectedCandidateTitle = computed(() => {
  const title = String(readiness.selectedTopTitle.value || '').trim()
  if (!title || title === 'No ranked candidate selected.' || title === '—') return 'Best available candidate'
  return title
})
const selectedCandidateSummary = computed(() => {
  const summary = String(readiness.selectedTopSummary.value || '').trim()
  if (!summary || summary === 'No ranked candidates are available.' || summary === '—') {
    return 'Run optimization to populate the shortlist.'
  }
  return summary
})
const validationPreviewText = computed(() => {
  const parts = [
    String(validationShortlistMeta.value || '').trim(),
    String(validationRouteSummary.value || '').trim(),
    String(validationWarningSummary.value || '').trim(),
  ].filter((part) => part && part !== '-')
  return parts.join(' ') || String(validationOutputPreview.value || '').trim() || 'No validation output yet.'
})
const guidedRunDisabled = computed(() => Boolean(runDisabled.value || !readiness.liveStrategySupported.value))
const guidedCanStartRunner = computed(() => Boolean(dryRun.value && canStartRunner.value))

const stepStatuses = computed<Record<GuidedStep, GuidedStatusTone>>(() => ({
  data: readiness.datasetReady.value ? 'done' : 'blocked',
  strategy: readiness.liveStrategySupported.value ? 'done' : 'warning',
  optimize: readiness.optimizationReady.value ? 'done' : readiness.datasetReady.value && readiness.liveStrategySupported.value ? 'ready' : 'blocked',
  validate: readiness.validationReady.value ? readiness.validationVerdictTone.value : readiness.optimizationReady.value ? 'ready' : 'blocked',
  trade: readiness.runnerReady.value ? 'ready' : readiness.validationReady.value ? 'warning' : 'blocked',
}))

function stepStatusLabel(tone: GuidedStatusTone): string {
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

function openAdvanced(area: 'data' | 'optimize' | 'validate' | 'aggregate' | 'live'): void {
  settingsStore.setUiMode('advanced')
  if (area === 'data') workflowStore.selectWorkspace('workflow', 'data')
  else if (area === 'optimize') workflowStore.selectWorkflowStep('optimize')
  else if (area === 'validate') workflowStore.selectWorkflowStep('validate')
  else if (area === 'aggregate') workflowStore.selectWorkspace('aggregate')
  else workflowStore.selectWorkspace('live')
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
    <section class="guidedRoot" aria-label="Guided workflow">
      <header class="guidedHero">
        <div class="guidedHeroCopy">
          <div class="guidedEyebrow">Guided Mode</div>
          <h2>Optimize, validate, then dry-run.</h2>
          <p>
            This view keeps the beginner path narrow while Advanced Mode keeps
            every research, ML, options, diagnostic, and artifact workflow.
          </p>
        </div>
        <div class="guidedHeroActions">
          <button type="button" class="guidedSecondary" @click="settingsStore.setUiMode('advanced')">
            Advanced Mode
          </button>
        </div>
      </header>

      <nav class="guidedSteps" aria-label="Guided workflow steps">
        <button
          v-for="step in stepMeta"
          :key="step.key"
          type="button"
          class="guidedStepButton"
          :class="[stepStatuses[step.key], { active: activeStep === step.key }]"
          :aria-current="activeStep === step.key ? 'step' : undefined"
          @click="selectStep(step.key)"
        >
          <component :is="checklistIcon({ label: step.label, detail: step.subtitle, complete: stepStatuses[step.key] === 'done', tone: stepStatuses[step.key] })" :size="16" aria-hidden="true" />
          <span class="guidedStepText">
            <span class="guidedStepLabel">{{ step.label }}</span>
            <span class="guidedStepSubtitle">{{ step.subtitle }}</span>
          </span>
          <span class="guidedStepState">{{ stepStatusLabel(stepStatuses[step.key]) }}</span>
        </button>
      </nav>

      <section class="guidedLayout">
        <aside class="guidedRail" aria-label="Guided checklist">
          <div class="guidedRailHeader">
            <div class="guidedEyebrow">Readiness</div>
            <h3>Current path</h3>
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
            <strong>Advanced Mode stays complete.</strong>
            Meta analysis, Stage 5 ML confidence, ML Signal/CatBoost, options
            optimization, campaigns, diagnostics, and artifact replay remain
            there.
          </div>
          <button type="button" class="guidedSecondary fullWidth" @click="openAdvanced('aggregate')">
            Open advanced research tools
          </button>
        </aside>

        <main class="guidedMain">
          <section v-if="activeStep === 'data'" class="guidedPane" aria-label="Guided data setup">
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

            <div class="guidedActions">
              <button type="button" class="guidedPrimary" :disabled="!canBrowseCsv" @click="dataStore.browseCsv()">
                Browse CSV
              </button>
              <button type="button" class="guidedSecondary" :disabled="!canLoadCsv" @click="dataStore.loadCsv()">
                Load CSV
              </button>
              <button type="button" class="guidedSecondary" :disabled="!selectedSample || selectedSample.disabled" @click="loadSelectedSample">
                Load sample
              </button>
              <button type="button" class="guidedSecondary" :disabled="!canPolygonOpen" @click="dataStore.openPolygon()">
                Polygon data
              </button>
              <button type="button" class="guidedSecondary" :disabled="!canApplyTimeframe" @click="applyTimeframe">
                Apply timeframe
              </button>
              <button type="button" class="guidedSecondary" :disabled="!canClearData" @click="dataStore.clearData()">
                Clear
              </button>
            </div>

            <p class="guidedMuted">{{ dataStatusLine }}</p>
            <p v-if="dataFieldHint" class="guidedWarning">{{ dataFieldHint }}</p>
            <p class="guidedMuted">{{ dataTimeframeHint }}</p>
            <p class="guidedMuted mono">{{ datasetContext }}</p>
            <button type="button" class="guidedTextLink" :disabled="!canOpenMarketDataDir" @click="dataStore.openMarketDataDir()">
              Open market-data folder
            </button>
          </section>

          <section v-else-if="activeStep === 'strategy'" class="guidedPane" aria-label="Guided strategy setup">
            <div class="guidedPaneHeader">
              <div>
                <div class="guidedEyebrow">Step 2</div>
                <h3>Pick a conventional strategy</h3>
                <p>Guided trading readiness starts with live-supported strategies.</p>
              </div>
            </div>

            <div class="guidedTemplateGrid">
              <article class="guidedTemplate">
                <div class="guidedTemplateHeader">
                  <ShieldCheck :size="18" aria-hidden="true" />
                  <h4>Double MA crossover</h4>
                </div>
                <p>Simple trend-following baseline with fast and slow moving averages.</p>
                <button type="button" class="guidedPrimary" @click="setTemplate('DoubleMa')">
                  Use Double MA
                </button>
              </article>
              <article class="guidedTemplate">
                <div class="guidedTemplateHeader">
                  <ShieldCheck :size="18" aria-hidden="true" />
                  <h4>Composite signal</h4>
                </div>
                <p>Conventional composite signal path that can still move into live execution.</p>
                <button type="button" class="guidedPrimary" @click="setTemplate('CompositeCpu')">
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
                Backend
                <select :value="backend" @change="optimizeStore.setSelectField('backend', ($event.target as HTMLSelectElement).value)">
                  <option value="Auto">Auto</option>
                  <option value="CpuOnly">CPU only</option>
                </select>
              </label>
            </div>
            <p class="guidedMuted">
              Advanced Mode keeps the full backend route controls, including GPU and hybrid routes.
            </p>

            <div class="guidedActions">
              <button type="button" class="guidedPrimary" @click="selectStep('optimize')">
                Continue to optimize
              </button>
              <button type="button" class="guidedSecondary" @click="openAdvanced('optimize')">
                Full strategy controls
              </button>
            </div>
          </section>

          <section v-else-if="activeStep === 'optimize'" class="guidedPane" aria-label="Guided optimization">
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
                <span>Backend</span>
                <strong>{{ backend }}</strong>
              </article>
              <article>
                <span>Status</span>
                <strong>{{ progressMeta || runReadinessHint }}</strong>
              </article>
            </div>

            <p class="guidedMuted mono">{{ runPlanMeta }}</p>
            <div class="guidedActions">
              <button v-if="canCancel" type="button" class="guidedDanger" @click="optimizeStore.cancelOptimization()">
                Stop run
              </button>
              <button v-else type="button" class="guidedPrimary" :disabled="guidedRunDisabled" @click="runOptimization">
                <Play :size="15" aria-hidden="true" />
                Run optimization
              </button>
              <button type="button" class="guidedSecondary" :disabled="!readiness.optimizationReady.value" @click="selectStep('validate')">
                Continue to validate
              </button>
              <button type="button" class="guidedSecondary" @click="openAdvanced('optimize')">
                Open results in Advanced
              </button>
            </div>

            <section class="guidedCandidate" aria-label="Selected candidate summary">
              <div class="guidedEyebrow">Candidate</div>
              <h4>{{ selectedCandidateTitle }}</h4>
              <p>{{ selectedCandidateSummary }}</p>
            </section>
          </section>

          <section v-else-if="activeStep === 'validate'" class="guidedPane" aria-label="Guided validation">
            <div class="guidedPaneHeader">
              <div>
                <div class="guidedEyebrow">Step 4</div>
                <h3>{{ readiness.validationVerdictTitle.value }}</h3>
                <p>{{ readiness.validationVerdictBody.value }}</p>
              </div>
              <span class="guidedStatusPill" :class="readiness.validationVerdictTone.value">
                {{ stepStatusLabel(readiness.validationVerdictTone.value) }}
              </span>
            </div>

            <div class="guidedSummaryGrid">
              <article>
                <span>Rows</span>
                <strong>{{ tableRowCount }}</strong>
              </article>
              <article>
                <span>Candidate</span>
                <strong>{{ selectedCandidateTitle }}</strong>
              </article>
              <article>
                <span>Warnings</span>
                <strong>{{ validationWarningSummary || 'None reported' }}</strong>
              </article>
            </div>

            <p class="guidedMuted">{{ validationPreviewText }}</p>
            <div class="guidedActions">
              <button type="button" class="guidedPrimary" :disabled="validateDisabled" @click="runValidation">
                Run validation
              </button>
              <button type="button" class="guidedSecondary" :disabled="!readiness.validationReady.value" @click="selectStep('trade')">
                Continue to trade readiness
              </button>
              <button type="button" class="guidedSecondary" @click="openAdvanced('validate')">
                Full validation report
              </button>
            </div>
          </section>

          <section v-else class="guidedPane" aria-label="Guided paper and live readiness">
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
              Dry-run is off. Guided Mode will not start the auto-runner in this state; open Advanced Mode for broker-armed launch after manual review.
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
                <strong>{{ hasExecutionCredentials || dryRun ? keyStateLabel : 'Missing' }}</strong>
              </article>
            </div>

            <div class="guidedActions">
              <button type="button" class="guidedSecondary" :disabled="!canRunOnce" @click="executionStore.runOnce()">
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

            <p class="guidedMuted">{{ liveStatusHint }}</p>
            <p class="guidedMuted">{{ runnerStatusHint }}</p>
            <p class="guidedMuted">{{ executionStatusLine }}</p>
            <p class="guidedMuted">{{ keyStateDetail }}</p>
          </section>
        </main>
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
.guidedLayout,
.guidedPane,
.guidedRail {
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 14px;
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
}

.guidedHero p,
.guidedPane p,
.guidedTemplate p {
  margin: 0;
  color: var(--muted-strong);
  line-height: 1.45;
}

.guidedHeroActions {
  display: flex;
  align-items: flex-start;
}

.guidedEyebrow {
  font-size: 0.64rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.guidedSteps {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
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
  border-radius: 12px;
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
  padding: 1rem;
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
  border-radius: 10px;
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
  border-radius: 10px;
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
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-working) 96%, var(--panel));
  padding: 0.75rem;
  display: grid;
  gap: 0.2rem;
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
  min-height: 2.25rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.42rem;
  border-radius: 10px;
  padding: 0.42rem 0.72rem;
  font-size: 0.76rem;
  font-weight: 700;
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

  .guidedSteps,
  .guidedSummaryGrid,
  .guidedFormGrid,
  .guidedFormGrid.compact,
  .guidedTemplateGrid {
    grid-template-columns: 1fr;
  }
}
</style>
