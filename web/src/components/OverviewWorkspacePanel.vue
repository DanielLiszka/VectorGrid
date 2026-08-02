<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import AppVisualIcon from './AppVisualIcon.vue'
import { useShellMount } from '../composables/useShellMount'
import { useUiReadiness } from '../composables/useUiReadiness'
import { useAnalysisStore } from '../stores/analysisStore'
import { useHistoryStore } from '../stores/historyStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'
import { useWorkflowStore } from '../stores/workflowStore'
import {
  artifactExportSnapshotIcon,
  compareCandidatesIcon,
} from '../utils/appSvgAssets'
import { describeExecutionRisk } from '../utils/executionRisk'
import {
  buildRouteProvenanceFromRun,
  formatRouteProvenanceCompact,
} from '../utils/core/runArtifactReview'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'
import AppButton from './ui/AppButton.vue'
import AppEmptyState from './ui/AppEmptyState.vue'
import AppMetricStrip, { type AppMetricItem } from './ui/AppMetricStrip.vue'

const analysisStore = useAnalysisStore()
const historyStore = useHistoryStore()
const workflowArtifactsStore = useWorkflowArtifactsStore()
const workflowStore = useWorkflowStore()
const { overviewStage } = useUiReadiness()
const {
  bestKicker,
  bestTitle,
  bestMeta,
  bestSummary,
  bestStats,
  overviewEmptyTitle,
  overviewEmptyBody,
  overviewEmptyHint,
  overviewPrimaryLabel,
  overviewPrimaryDisabled,
  overviewSecondaryVisible,
  overviewSecondaryLabel,
  overviewSecondaryDisabled,
  compareSummary,
  canCopyBest,
  canExportCsv,
  canExportJson,
  canSetBaseline,
  canClearBaseline,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { compareOptions, compareSelectedValue } = storeToRefs(historyStore)
const { currentRun } = storeToRefs(workflowArtifactsStore)
const { resultsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#overviewVueShellMount')

const compareDraft = ref('')

const visible = computed(() => String(resultsTab.value || '').trim() === 'overview')
const showNoDataOverview = computed(() => overviewStage.value === 'no_data')
const showPendingOverview = computed(() => overviewStage.value === 'data_loaded_no_run')
const showReadyOverview = computed(() => overviewStage.value === 'ready')
const executionRisk = computed(() => {
  const req = currentRun.value?.req
  const strategy = req && typeof req === 'object'
    ? (req as { strategy?: unknown }).strategy ?? null
    : null
  return describeExecutionRisk(strategy)
})
const routeProvenanceSummary = computed(() => {
  const run = currentRun.value
  if (!run) return ''
  return formatRouteProvenanceCompact(
    run.routeProvenance ?? buildRouteProvenanceFromRun(run.req, run.res),
  )
})

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

const overviewStageLabel = computed(() => {
  if (showNoDataOverview.value) return 'Awaiting data'
  if (showPendingOverview.value) return 'Ready to run'
  return showReadyOverview.value ? 'Winner ready' : 'Overview'
})

const overviewSummaryChips = computed(() => {
  const chips = [overviewStageLabel.value]
  if (showReadyOverview.value && String(bestKicker.value || '').trim()) chips.push(String(bestKicker.value || '').trim())
  if (showReadyOverview.value && executionRisk.value) chips.push('Execution review advised')
  if (compareOptions.value.length > 1) chips.push(`Compare ${compareOptions.value.length - 1} baselines`)
  return chips
})

const overviewHealthCards = computed<AppMetricItem[]>(() => {
  const baselineCount = Math.max(0, compareOptions.value.length - 1)
  const exportTargets = [
    canCopyBest.value ? 'Copy' : '',
    canExportCsv.value ? 'CSV' : '',
    canExportJson.value ? 'JSON' : '',
  ].filter((item) => item.length > 0)
  return [
    {
      key: 'run-state',
      label: 'Run state',
      value: overviewStageLabel.value,
      detail: String(statusLine.value || '').trim() || 'No run status yet.',
      tone: showReadyOverview.value ? 'success' : 'neutral',
    },
    {
      key: 'winner',
      label: 'Winner',
      value: String(bestKicker.value || '').trim() || 'Awaiting winner',
      detail: String(bestTitle.value || bestMeta.value || '').trim() || 'Run optimization to score candidates.',
      tone: showReadyOverview.value ? 'accent' : 'neutral',
    },
    {
      key: 'baselines',
      label: 'Baselines',
      value: baselineCount ? `${baselineCount} available` : 'None selected',
      detail: baselineCount ? 'Use baseline review before promotion.' : 'Set a baseline from a completed run when needed.',
      tone: baselineCount ? 'accent' : 'neutral',
    },
    {
      key: 'artifacts',
      label: 'Artifacts',
      value: exportTargets.length ? exportTargets.join(' · ') : 'No exports',
      detail: routeProvenanceSummary.value || 'Route provenance appears after a run artifact is loaded.',
      tone: exportTargets.length ? 'success' : 'neutral',
    },
  ]
})

watch(
  compareSelectedValue,
  (value) => {
    const next = String(value || '').trim()
    if (compareDraft.value !== next) compareDraft.value = next
  },
  { immediate: true },
)

function setSelectValue(id: string, nextValue: string): void {
  analysisStore.setSelectField(id, nextValue)
}

function reviewExecutionSurface(): void {
  workflowStore.selectWorkspace('live')
}

function reviewTopResults(): void {
  workflowStore.selectResultsTab('top')
}

function openOptimizationWorkspace(): void {
  workflowStore.selectWorkspace('workflow', 'optimize')
  workflowStore.selectControlsTab('opt')
}

</script>

<template>
  <Teleport v-if="mountReady" to="#overviewVueShellMount">
    <section class="overviewPanelVueRoot overviewVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Winner</div>
          <h2 class="panelTitle">Overview</h2>
          <div class="panelSubtitle">
            Review the leading candidate, compare it against saved baselines, and decide whether it is ready for execution review.
          </div>
        </div>
        <div class="panelMeta mono panelNumeric">{{ stamp }}</div>
      </header>

      <div class="panelSummaryChips" aria-label="Overview summary">
        <div v-for="chip in overviewSummaryChips" :key="chip" class="panelSummaryChip">{{ chip }}</div>
      </div>

      <AppEmptyState
        v-if="showNoDataOverview"
        :title="overviewEmptyTitle"
        :body="overviewEmptyBody"
        :hint="overviewEmptyHint"
      >
        <template #actions>
          <AppButton
            variant="primary"
            type="button"
            data-qa-action="overview.empty.primary"
            :disabled="overviewPrimaryDisabled"
            @click="analysisStore.overviewEmptyPrimary()"
          >
            {{ overviewPrimaryLabel }}
          </AppButton>
          <AppButton
            v-if="overviewSecondaryVisible"
            type="button"
            data-qa-action="overview.empty.secondary"
            :disabled="overviewSecondaryDisabled"
            @click="analysisStore.overviewEmptySecondary()"
          >
            {{ overviewSecondaryLabel }}
          </AppButton>
        </template>
      </AppEmptyState>

      <AppEmptyState
        v-else-if="showPendingOverview"
        class="overviewPendingState"
        title="No analyzed winner yet."
        body="Results will appear here after an optimization run is scored or an existing run library result is loaded."
        hint="Next: open Optimize to configure and run a strategy, or load a completed run from the Library."
      >
        <template #actions>
          <AppButton variant="primary" type="button" @click="openOptimizationWorkspace">
            Open Optimize
          </AppButton>
        </template>
      </AppEmptyState>

      <section v-if="showReadyOverview" class="overviewHealthGrid" aria-label="Post-run health summary">
        <h3 class="overviewHealthTitle">Run health</h3>
        <AppMetricStrip
          :items="overviewHealthCards"
          aria-label="Post-run health metrics"
          min-item-width="10rem"
        />
      </section>

      <section v-if="showReadyOverview && executionRisk" class="riskNotice">
        <div class="riskTitle">{{ executionRisk.title }}</div>
        <div class="riskBody">{{ executionRisk.body }}</div>
        <div class="actions" style="margin-top: 0.75rem;">
          <button type="button" class="primary" data-qa-action="overview.live.review" @click="reviewExecutionSurface()">Review live execution</button>
        </div>
      </section>

      <section v-if="showReadyOverview" class="bestCard panelProminentPane">
        <div class="bestHeader">
          <div>
            <div class="bestKicker">{{ bestKicker }}</div>
            <div class="bestTitle mono">{{ bestTitle }}</div>
            <div class="panelMuted" style="margin-top: 0.25rem; font-size: 0.85rem;">{{ bestMeta }}</div>
            <div
              v-if="routeProvenanceSummary"
              class="panelMuted mono"
              style="margin-top: 0.35rem; font-size: 0.78rem;"
            >
              Route: {{ routeProvenanceSummary }}
            </div>
          </div>
        </div>
        <div class="bestGrid">
          <div
            v-for="(stat, statIndex) in bestStats"
            :key="`best_stat_${stat.label}_${stat.value}_${statIndex}`"
            class="bestStat"
            :class="stat.tone"
          >
            <div class="bestStatK">{{ stat.label }}</div>
            <div class="bestStatV" :title="stat.value">{{ stat.value || '—' }}</div>
          </div>
          <div v-if="bestStats.length === 0" class="panelMuted">No metrics yet.</div>
        </div>
        <details class="bestDetails">
          <summary>Details</summary>
          <div class="mono summaryDetails">{{ bestSummary }}</div>
        </details>
        <div class="bestUtilityBand surfaceSunken">
          <div class="bestUtilityCopy">
            <div class="bestUtilityTitle">Next step</div>
            <div class="panelMuted">
              Use Top Results for shortlist triage and candidate switching. Overview stays focused on winner framing and baseline review.
            </div>
          </div>
          <div class="actions bestUtilityActions">
            <button type="button" class="primary" data-qa-action="overview.top.review" @click="reviewTopResults">Review Top results</button>
            <button type="button" data-qa-action="overview.best.copy" :disabled="!canCopyBest" @click="analysisStore.copyBest()">Copy</button>
            <button type="button" class="buttonWithIcon" data-qa-action="overview.top.exportCsv" :disabled="!canExportCsv" @click="analysisStore.exportTopCsv()">
              <AppVisualIcon :icon="artifactExportSnapshotIcon" class="actionIcon" />
              <span>Export top CSV</span>
            </button>
            <button type="button" class="buttonWithIcon" data-qa-action="overview.top.exportJson" :disabled="!canExportJson" @click="analysisStore.exportTopJson()">
              <AppVisualIcon :icon="artifactExportSnapshotIcon" class="actionIcon" />
              <span>Export JSON</span>
            </button>
          </div>
        </div>
      </section>

      <section v-if="showReadyOverview" class="pane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Comparison</div>
            <h3 class="panelPaneTitle sectionHeadingWithIcon">
              <AppVisualIcon :icon="compareCandidatesIcon" class="sectionGlyph" />
              <span>Baseline review</span>
            </h3>
            <div class="panelSectionLead">
              Compare the current winner against a saved run before promoting it to execution review.
            </div>
          </div>
        </div>
        <label for="vueCompareSelect">Baseline run</label>
        <select
          id="vueCompareSelect"
          :value="compareDraft"
          @change="compareDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('compareSelect', compareDraft)"
        >
          <option
            v-for="option in compareOptions"
            :key="`compare_${option.value}_${option.label}`"
            :value="option.value"
            :disabled="option.disabled"
          >
            {{ option.label }}
          </option>
        </select>
        <div class="actions">
          <button type="button" :disabled="!canSetBaseline" data-qa-action="overview.baseline.set" @click="analysisStore.setBaseline()">Set baseline = current</button>
          <button type="button" :disabled="!canClearBaseline" data-qa-action="overview.baseline.clear" @click="analysisStore.clearBaseline()">Clear baseline</button>
        </div>
        <details class="panelDetails">
          <summary>Comparison details</summary>
          <div class="panelDetailsBody">
            <pre class="mono summary">{{ compareSummary }}</pre>
          </div>
        </details>
        <div class="panelMuted baselineOwnershipNote">
          Baseline comparison stays here. Use Top Results when you need to change the working candidate rather than just compare the current winner.
        </div>
      </section>

      <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.overviewPanelVueRoot {
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

.riskNotice {
  border: 1px solid color-mix(in oklab, var(--warning-ink) 34%, var(--border));
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-warning) 86%, var(--panel-footer));
  padding: 0.8rem 0.9rem;
}

.overviewHealthGrid {
  display: grid;
  gap: 0.6rem;
}

.overviewHealthTitle {
  margin: 0;
  font-size: 0.82rem;
  font-weight: 700;
  color: var(--muted-strong);
}

.riskTitle {
  font-size: 0.86rem;
  font-weight: 650;
  color: var(--warning-ink);
}

.riskBody {
  margin-top: 0.35rem;
  color: var(--text);
  font-size: 0.82rem;
  line-height: 1.45;
}

.kicker {
  font-size: 0.78rem;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--muted);
}

.bestTitle {
  margin-top: 0.2rem;
  font-size: 0.9rem;
}

.bestUtilityBand {
  margin-top: 0.9rem;
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 12px;
  padding: 0.75rem 0.8rem;
  display: grid;
  gap: 0.65rem;
}

.bestUtilityTitle {
  font-size: 0.78rem;
  font-weight: 650;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--muted-soft);
}

.bestUtilityActions {
  margin-top: 0;
}

.buttonWithIcon,
.sectionHeadingWithIcon {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.actionIcon,
.sectionGlyph {
  width: 1rem;
  height: 1rem;
  flex: 0 0 auto;
}

.summaryDetails {
  margin-top: 0.65rem;
  white-space: pre-wrap;
}

.summary {
  margin-top: 0.45rem;
  max-height: 200px;
  overflow: auto;
}

.overviewPendingState {
  min-height: 220px;
  display: grid;
  align-content: center;
  gap: 0.65rem;
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

.baselineOwnershipNote {
  margin-top: 0.75rem;
}

.mono {
  font-family: var(--mono);
}
</style>



