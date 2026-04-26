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
  emptyResultsWorkspaceIllustration,
} from '../utils/appSvgAssets'
import { describeExecutionRisk } from '../utils/executionRisk'
import {
  buildRouteProvenanceFromRun,
  formatRouteProvenanceCompact,
} from '../utils/core/runArtifactReview'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

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
  overviewEmptyVisible,
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
const showPendingOverview = computed(() => !overviewEmptyVisible.value && overviewStage.value === 'data_loaded_no_run')
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
  if (overviewEmptyVisible.value) return 'Awaiting results'
  if (showPendingOverview.value) return 'Ready for review'
  return showReadyOverview.value ? 'Winner ready' : 'Overview'
})

const overviewSummaryChips = computed(() => {
  const chips = [overviewStageLabel.value]
  if (showReadyOverview.value && String(bestKicker.value || '').trim()) chips.push(String(bestKicker.value || '').trim())
  if (showReadyOverview.value && executionRisk.value) chips.push('Execution review advised')
  if (compareOptions.value.length > 1) chips.push(`Compare ${compareOptions.value.length - 1} baselines`)
  return chips
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

      <section v-if="overviewEmptyVisible" class="emptyState">
        <img class="emptyStateIllustration" :src="emptyResultsWorkspaceIllustration" alt="" aria-hidden="true" draggable="false" />
        <div class="emptyTitle">{{ overviewEmptyTitle }}</div>
        <div class="emptyBody">{{ overviewEmptyBody }}</div>
        <div class="emptyActions">
          <button
            class="primary"
            type="button"
            data-qa-action="overview.empty.primary"
            :disabled="overviewPrimaryDisabled"
            @click="analysisStore.overviewEmptyPrimary()"
          >
            {{ overviewPrimaryLabel }}
          </button>
          <button
            v-if="overviewSecondaryVisible"
            type="button"
            data-qa-action="overview.empty.secondary"
            :disabled="overviewSecondaryDisabled"
            @click="analysisStore.overviewEmptySecondary()"
          >
            {{ overviewSecondaryLabel }}
          </button>
        </div>
        <div class="panelMuted" style="margin-top: 0.65rem; font-size: 0.85rem;">{{ overviewEmptyHint }}</div>
      </section>

      <section v-else-if="showPendingOverview" class="emptyState overviewPendingState">
        <img class="emptyStateIllustration" :src="emptyResultsWorkspaceIllustration" alt="" aria-hidden="true" draggable="false" />
        <div class="emptyTitle">No analyzed winner yet.</div>
        <div class="emptyBody">
          Results will appear here after an optimization run is scored or an existing run library result is loaded.
        </div>
        <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
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
          <button type="button" :disabled="!canSetBaseline" @click="analysisStore.setBaseline()">Set baseline = current</button>
          <button type="button" :disabled="!canClearBaseline" @click="analysisStore.clearBaseline()">Clear baseline</button>
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

.emptyStateIllustration {
  width: clamp(88px, 18vw, 124px);
  height: auto;
  margin-bottom: 0.2rem;
  opacity: 0.92;
}

.riskNotice {
  border: 1px solid color-mix(in oklab, var(--warning-ink) 34%, var(--border));
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-warning) 86%, var(--panel-footer));
  padding: 0.8rem 0.9rem;
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




