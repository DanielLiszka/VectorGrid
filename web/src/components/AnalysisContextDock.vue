<script setup lang="ts">
import { computed, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useStorage } from '@vueuse/core'
import { useAnchoredOverlay } from '../composables/useAnchoredOverlay'
import { useAnalysisStore } from '../stores/analysisStore'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { ANALYSIS_DOCK_OPEN_STORAGE_KEY, isAnalysisDockVisible } from '../utils/shellDockContract'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const analysis = useAnalysisStore()
const commandActionStore = useCommandActionStore()
const workflowState = useWorkflowStore()
const {
  bestTitle,
  bestMeta,
  topRows,
  topFilterMeta,
  drillStatus,
  stabilityStatus,
  heatmapMeta,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysis)
const { resultsTab, workspace, workflow } = storeToRefs(workflowState)
const dockOpen = useStorage<boolean>(ANALYSIS_DOCK_OPEN_STORAGE_KEY, false)
const dockPanelId = 'vta-analysis-dock-panel'
const { referenceRef, floatingRef, floatingStyles } = useAnchoredOverlay(dockOpen, {
  placement: 'top-end',
  offsetPx: 10,
  paddingPx: 10,
})

const visible = computed(() => {
  return isAnalysisDockVisible({
    mode: workflowState.mode,
    workspace: workspace.value,
    workflow: workflow.value,
  })
})

watch(
  visible,
  (isVisible) => {
    if (isVisible) return
    dockOpen.value = false
  },
  { immediate: true },
)

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

const activeResultsTab = computed(() => String(resultsTab.value || '').trim() || 'overview')
const quietMode = computed(() => !dockOpen.value)

function exportTopCsvAction(): void {
  commandActionStore.exportTopCsv()
}

function runOptimizationAction(): void {
  commandActionStore.runOptimization()
}

function runValidationAction(): void {
  commandActionStore.validateOptimization()
}

function clickResultsTab(tab: string): void {
  workflowState.selectResultsTab(tab)
}

function toggleDock(): void {
  dockOpen.value = !dockOpen.value
}
</script>

<template>
  <div v-if="visible" class="analysisDockRoot">
    <div ref="referenceRef" class="dockAnchor">
      <button
        class="dockToggle"
        :class="{ quiet: quietMode }"
        type="button"
        :aria-expanded="dockOpen ? 'true' : 'false'"
        :aria-controls="dockPanelId"
        @click="toggleDock"
      >
        <span class="dockToggleLabel">{{ quietMode ? 'Insights' : 'Analysis' }}</span>
        <span class="dockToggleState">{{ dockOpen ? activeResultsTab : 'Closed' }}</span>
      </button>
    </div>

    <aside
      v-if="dockOpen"
      :id="dockPanelId"
      ref="floatingRef"
      class="analysisDock"
      aria-label="Analysis context dock"
      :style="floatingStyles"
    >
      <header class="dockHeader">
        <h2 class="title" style="margin: 0;">Analysis Context</h2>
        <div class="stamp">{{ stamp }}</div>
      </header>
      <div class="rows">
        <div class="row"><span>Tab</span><strong class="mono">{{ activeResultsTab }}</strong></div>
        <div class="row"><span>Best</span><strong class="mono">{{ bestTitle }}</strong></div>
        <div class="row"><span>Meta</span><strong class="mono">{{ bestMeta }}</strong></div>
        <div class="row"><span>Top Rows</span><strong class="mono">{{ topRows }} · {{ topFilterMeta }}</strong></div>
        <div class="row"><span>Drill</span><strong class="mono">{{ drillStatus }}</strong></div>
        <div class="row"><span>Stability</span><strong class="mono">{{ stabilityStatus }}</strong></div>
        <div class="row"><span>Heatmap</span><strong class="mono">{{ heatmapMeta }}</strong></div>
        <div class="row"><span>Status</span><strong class="mono">{{ statusLine }}</strong></div>
      </div>
      <div class="actions">
        <button type="button" @click="clickResultsTab('overview')">Overview</button>
        <button type="button" @click="clickResultsTab('top')">Top</button>
        <button type="button" @click="clickResultsTab('drill')">Drill</button>
        <button type="button" @click="runOptimizationAction">Run</button>
        <button type="button" @click="runValidationAction">Validate</button>
        <button type="button" @click="exportTopCsvAction">Export top CSV</button>
      </div>
    </aside>
  </div>
</template>

<style scoped>
.analysisDockRoot {
  --dock-accent: var(--accent);
  --dock-accent-ink: var(--selection-text);
  --dock-accent-soft: var(--accent-soft);
  --dock-surface: linear-gradient(
    158deg,
    color-mix(in oklab, var(--surface-focus) 90%, var(--panel-footer)),
    color-mix(in oklab, var(--accent-soft) 34%, var(--surface-sticky))
  );
  --dock-surface-quiet: color-mix(in oklab, var(--surface-sticky) 94%, var(--panel-footer));
  --dock-border: color-mix(in oklab, var(--dock-accent) 40%, var(--border));
  --dock-border-soft: color-mix(in oklab, var(--dock-accent) 26%, var(--border));
  --dock-badge-border: color-mix(in oklab, var(--dock-accent-ink) 34%, var(--border));
  --dock-badge-surface: color-mix(in oklab, var(--dock-accent-soft) 44%, var(--surface-inline));
  --dock-text: var(--text);
  --dock-title: color-mix(in oklab, var(--dock-accent) 74%, var(--dock-text));
  --dock-stamp: color-mix(in oklab, var(--dock-accent-ink) 76%, var(--dock-text));
  --dock-label: color-mix(in oklab, var(--dock-accent) 62%, var(--dock-text));
  --dock-action-text: color-mix(in oklab, var(--dock-accent-ink) 82%, var(--dock-text));
  position: fixed;
  right: 220px;
  bottom: 18px;
  z-index: 4037;
  pointer-events: none;
}

.analysisDockRoot > * {
  pointer-events: auto;
}

.dockAnchor {
  display: flex;
  justify-content: flex-end;
}

.dockToggle {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  border: 1px solid var(--dock-border);
  border-radius: 999px;
  background: var(--dock-surface-quiet);
  color: var(--dock-action-text);
  padding: 6px 10px;
  font-size: 0.75rem;
  font-weight: 650;
  letter-spacing: 0.04em;
  box-shadow: var(--shadow-inline);
  backdrop-filter: blur(10px);
  transition:
    opacity 140ms ease,
    transform 140ms ease,
    box-shadow 140ms ease,
    background-color 140ms ease;
}

.dockToggle.quiet {
  gap: 6px;
  padding: 5px 9px;
  background: color-mix(in oklab, var(--dock-surface-quiet) 94%, var(--panel));
  border-color: var(--dock-border-soft);
  opacity: 0.82;
  box-shadow: var(--shadow-inline);
}

.dockToggle.quiet:hover {
  opacity: 1;
  transform: translateY(-1px);
}

.dockToggleLabel {
  text-transform: uppercase;
}

.dockToggleState {
  border: 1px solid var(--dock-badge-border);
  border-radius: 999px;
  padding: 1px 6px;
  font-size: 0.66rem;
  color: var(--dock-stamp);
  background: var(--dock-badge-surface);
}

.analysisDock {
  z-index: 4038;
  width: min(430px, calc(100vw - 20px));
  border-radius: 12px;
  border: 1px solid var(--dock-border);
  background: var(--dock-surface);
  color: var(--dock-text);
  box-shadow: var(--shadow-focus);
  padding: 10px;
  display: grid;
  gap: 8px;
  pointer-events: auto;
  backdrop-filter: blur(12px);
}

.dockHeader {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.title {
  font-size: 0.78rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--dock-title);
}

.stamp {
  font-size: 0.72rem;
  color: var(--dock-stamp);
}

.rows {
  display: grid;
  gap: 4px;
}

.row {
  display: grid;
  grid-template-columns: 78px 1fr;
  gap: 6px;
  align-items: baseline;
}

.row span {
  font-size: 0.72rem;
  color: var(--dock-label);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.row strong {
  font-size: 0.78rem;
  color: var(--dock-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.actions button {
  border: 1px solid var(--dock-border);
  border-radius: 8px;
  padding: 4px 8px;
  font-size: 0.72rem;
  background: var(--dock-badge-surface);
  color: var(--dock-action-text);
}

@media (max-width: 760px) {
  .analysisDockRoot {
    right: 150px;
    bottom: 10px;
  }

  .analysisDock {
    width: min(96vw, 440px);
  }
}

@media (max-width: 1180px) {
  .analysisDockRoot {
    display: none;
  }
}
</style>


