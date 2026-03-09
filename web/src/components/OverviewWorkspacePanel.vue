<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useUiReadiness } from '../composables/useUiReadiness'
import { useAnalysisStore } from '../stores/analysisStore'
import { useHistoryStore } from '../stores/historyStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { describeExecutionRisk } from '../utils/executionRisk'

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

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)

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

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
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

function reviewExecutionSettings(): void {
  workflowStore.selectWorkspace('workflow', 'optimize')
  workflowStore.selectControlsTab('strategy', { persist: false })
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="overview"]') as HTMLElement | null
  const mount = document.querySelector('#overviewVueShellMount') as HTMLElement | null
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueOverview = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
})

onBeforeUnmount(() => {
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueOverview
    panelRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#overviewVueShellMount">
    <section class="overviewPanelVueRoot overviewVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Overview</div>
        <div class="meta mono">{{ stamp }}</div>
      </header>

      <section v-if="overviewEmptyVisible" class="emptyState">
        <div class="emptyTitle">{{ overviewEmptyTitle }}</div>
        <div class="emptyBody">{{ overviewEmptyBody }}</div>
        <div class="emptyActions">
          <button
            class="primary"
            type="button"
            :disabled="overviewPrimaryDisabled"
            @click="analysisStore.overviewEmptyPrimary()"
          >
            {{ overviewPrimaryLabel }}
          </button>
          <button
            v-if="overviewSecondaryVisible"
            type="button"
            :disabled="overviewSecondaryDisabled"
            @click="analysisStore.overviewEmptySecondary()"
          >
            {{ overviewSecondaryLabel }}
          </button>
        </div>
        <div class="muted" style="margin-top: 0.65rem; font-size: 0.85rem;">{{ overviewEmptyHint }}</div>
      </section>

      <section v-else-if="showPendingOverview" class="emptyState overviewPendingState">
        <div class="emptyTitle">No analyzed winner yet.</div>
        <div class="emptyBody">
          Results will appear here after an optimization run is scored or an existing run library result is loaded.
        </div>
        <div class="muted mono">{{ statusLine }}</div>
      </section>

      <section v-if="showReadyOverview && executionRisk" class="riskNotice">
        <div class="riskTitle">{{ executionRisk.title }}</div>
        <div class="riskBody">{{ executionRisk.body }}</div>
        <div class="actions" style="margin-top: 0.75rem;">
          <button type="button" class="primary" @click="reviewExecutionSettings()">Review execution settings</button>
        </div>
      </section>

      <section v-if="showReadyOverview" class="bestCard">
        <div class="bestHeader">
          <div>
            <div class="bestKicker">{{ bestKicker }}</div>
            <div class="bestTitle mono">{{ bestTitle }}</div>
            <div class="muted" style="margin-top: 0.25rem; font-size: 0.85rem;">{{ bestMeta }}</div>
          </div>
          <div class="actions" style="margin-top: 0;">
            <button type="button" :disabled="!canCopyBest" @click="analysisStore.copyBest()">Copy</button>
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
          <div v-if="bestStats.length === 0" class="muted">No metrics yet.</div>
        </div>
        <details class="bestDetails">
          <summary>Details</summary>
          <div class="mono summaryDetails">{{ bestSummary }}</div>
        </details>
        <div class="actions" style="margin-top: 0.9rem;">
          <button type="button" :disabled="!canExportCsv" @click="analysisStore.exportTopCsv()">Export top CSV</button>
          <button type="button" :disabled="!canExportJson" @click="analysisStore.exportTopJson()">Export JSON</button>
        </div>
      </section>

      <section v-if="showReadyOverview" class="pane">
        <h3>Compare</h3>
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
        <pre class="mono summary">{{ compareSummary }}</pre>
      </section>

      <div class="status mono">{{ statusLine }}</div>
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

.riskNotice {
  border: 1px solid color-mix(in oklab, var(--warning) 45%, var(--border));
  border-radius: 12px;
  background: color-mix(in oklab, var(--warning) 12%, var(--panel-footer));
  padding: 0.8rem 0.9rem;
}

.riskTitle {
  font-size: 0.86rem;
  font-weight: 650;
  color: var(--warning);
}

.riskBody {
  margin-top: 0.35rem;
  color: var(--text);
  font-size: 0.82rem;
  line-height: 1.45;
}

.pane h3 {
  margin: 0 0 0.55rem;
  font-size: 0.86rem;
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
</style>

<style>
[data-vue-panel='overview'][data-vue-overview='1'] > :not(#overviewVueShellMount) {
  display: none !important;
}
</style>



