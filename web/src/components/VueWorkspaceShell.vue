<script setup lang="ts">
import { useStorage } from '@vueuse/core'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useDataStore } from '../stores/dataStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useSettingsStore } from '../stores/settingsStore'
import { useWorkflowStore } from '../stores/workflowStore'

const workflowStore = useWorkflowStore()
const dataStore = useDataStore()
const optimizeStore = useOptimizeStore()
const settingsStore = useSettingsStore()

const workspaceGridRef = ref<HTMLElement | null>(null)
const viewportWidth = ref(1440)
const splitFraction = useStorage<number>('VTA_WORKSPACE_SPLIT_V1', 0.34)
const dividerDrag = ref<{ pointerId: number; startClientX: number; startFraction: number } | null>(null)

const { controlsTab, resultsTab, workspace, workflowStep, workflowSubtitle } = storeToRefs(workflowStore)
const { dataId, dataLen, datasetLabel, datasetRange } = storeToRefs(dataStore)
const { backend, progressMeta, runReadinessHint, strategyKind } = storeToRefs(optimizeStore)
const { controlsCollapsed } = storeToRefs(settingsStore)

const workspaceItems = [
  { key: 'workflow', label: 'Workflow' },
  { key: 'library', label: 'Run Library' },
  { key: 'aggregate', label: 'Aggregate Analysis' },
  { key: 'pro', label: 'Pro Tools' },
  { key: 'settings', label: 'Settings' },
] as const

const workflowStepItems = [
  { key: 'data', label: '1 Data Source' },
  { key: 'optimize', label: '2 Strategy Setup' },
  { key: 'validate', label: '3 Validation' },
  { key: 'analyze', label: '4 Summary' },
  { key: 'finalize', label: '5 Finalizer' },
] as const

const controlsItems = {
  data: { key: 'data', label: 'Data' },
  opt: { key: 'opt', label: 'Optimize' },
  strategy: { key: 'strategy', label: 'Strategy' },
  advanced: { key: 'advanced', label: 'Advanced' },
} as const

const resultsItems = {
  overview: { key: 'overview', label: 'Overview' },
  top: { key: 'top', label: 'Top' },
  pareto: { key: 'pareto', label: 'Pareto' },
  validate: { key: 'validate', label: 'Validate' },
  drill: { key: 'drill', label: 'Drilldown' },
  heatmap: { key: 'heatmap', label: 'Heatmap' },
  history: { key: 'history', label: 'History' },
  meta: { key: 'meta', label: 'Meta' },
  debug: { key: 'debug', label: 'Debug' },
} as const

const activeControlsTab = computed(() => String(controlsTab.value || '').trim() || 'data')
const activeResultsTab = computed(() => String(resultsTab.value || '').trim() || 'overview')
const currentWorkspace = computed(() => String(workspace.value || '').trim() || 'workflow')
const currentWorkflowStep = computed(() => String(workflowStep.value || '').trim() || 'data')
const settingsWorkspaceActive = computed(() => currentWorkspace.value === 'settings')
const shellSubtitle = computed(() => {
  const text = String(workflowSubtitle.value || '').trim()
  return text || '—'
})
const layoutKey = computed(() => `${currentWorkspace.value}:${currentWorkflowStep.value}`)

const visibleControlsItems = computed(() => {
  if (currentWorkspace.value === 'workflow') {
    if (currentWorkflowStep.value === 'data') return [controlsItems.data]
    if (currentWorkflowStep.value === 'optimize') return [controlsItems.opt, controlsItems.strategy, controlsItems.advanced]
    if (currentWorkflowStep.value === 'finalize') return [controlsItems.advanced]
    return []
  }
  if (currentWorkspace.value === 'pro') {
    return [controlsItems.advanced]
  }
  return []
})

const visibleResultsItems = computed(() => {
  if (currentWorkspace.value === 'workflow') {
    if (currentWorkflowStep.value === 'data') return [resultsItems.overview]
    if (currentWorkflowStep.value === 'optimize') {
      return [resultsItems.overview, resultsItems.top, resultsItems.pareto, resultsItems.heatmap]
    }
    if (currentWorkflowStep.value === 'validate') return [resultsItems.validate]
    if (currentWorkflowStep.value === 'analyze') {
      return [resultsItems.overview, resultsItems.top, resultsItems.pareto, resultsItems.drill, resultsItems.heatmap]
    }
    if (currentWorkflowStep.value === 'finalize') return [resultsItems.meta]
  }
  if (currentWorkspace.value === 'library') return [resultsItems.history]
  if (currentWorkspace.value === 'aggregate') return [resultsItems.meta]
  if (currentWorkspace.value === 'pro') return [resultsItems.debug]
  return []
})

const controlsColumnEligible = computed(() => visibleControlsItems.value.length > 0 && !settingsWorkspaceActive.value)
const controlsColumnVisible = computed(() => controlsColumnEligible.value && !controlsCollapsed.value)
const resultsTabsVisible = computed(() => visibleResultsItems.value.length > 1)
const controlsTabsVisible = computed(() => controlsColumnVisible.value && visibleControlsItems.value.length > 1)
const shellUsesSplitLayout = computed(() => {
  if (!controlsColumnVisible.value) return false
  return layoutKey.value === 'workflow:data' || layoutKey.value === 'workflow:optimize' || currentWorkspace.value === 'pro'
})
const splitLayoutActive = computed(() => shellUsesSplitLayout.value && viewportWidth.value > 1200)
const stackedLayoutActive = computed(() => shellUsesSplitLayout.value && controlsColumnVisible.value && !splitLayoutActive.value)
const workspaceGridClass = computed(() => ({
  singleColumn: !controlsColumnVisible.value || !shellUsesSplitLayout.value,
  stacked: stackedLayoutActive.value,
}))
const workspaceGridStyle = computed(() => {
  if (!splitLayoutActive.value) return undefined
  const fraction = clampSplitFraction(splitFraction.value)
  return {
    gridTemplateColumns: `${(fraction * 100).toFixed(3)}% 14px minmax(0, 1fr)`,
  }
})

const contextItems = computed(() => {
  const dataset = String(datasetLabel.value || '').trim()
  const range = String(datasetRange.value || '').trim()
  const rows = String(dataLen.value || '').trim()
  const strategy = String(strategyKind.value || '').trim()
  const backendLabel = String(backend.value || '').trim()
  const status = String(progressMeta.value || '').trim() || String(runReadinessHint.value || '').trim()
  const loaded = String(dataId.value || '').trim()
  return [
    { key: 'Dataset', value: dataset && dataset !== '—' ? dataset : 'No dataset' },
    { key: 'Range', value: range && range !== '—' ? range : 'No range' },
    { key: 'Rows', value: rows && rows !== '—' ? rows : '—' },
    { key: 'Strategy', value: strategy || '—' },
    { key: 'Backend', value: backendLabel || '—' },
    { key: 'Status', value: status || 'Ready.' },
    { key: 'Loaded ID', value: loaded && loaded !== '—' ? loaded : '—' },
  ]
})

watch(
  [currentWorkspace, currentWorkflowStep, visibleControlsItems, visibleResultsItems],
  () => {
    const controlsKeys = visibleControlsItems.value.map((item) => String(item.key))
    const resultsKeys = visibleResultsItems.value.map((item) => String(item.key))

    if (controlsKeys.length > 0 && !controlsKeys.includes(activeControlsTab.value)) {
      const nextControls = controlsKeys[0]
      if (nextControls) workflowStore.selectControlsTab(nextControls, { persist: false })
    }
    if (resultsKeys.length > 0 && !resultsKeys.includes(activeResultsTab.value)) {
      const nextResults = resultsKeys[0]
      if (nextResults) workflowStore.selectResultsTab(nextResults, { persist: false })
    }
  },
  { immediate: true },
)

function selectWorkspace(key: string): void {
  if (currentWorkspace.value === key) return
  workflowStore.selectWorkspace(key)
}

function selectWorkflowStep(key: string): void {
  if (currentWorkspace.value === 'workflow' && currentWorkflowStep.value === key) return
  workflowStore.selectWorkflowStep(key)
}

function selectControlsTab(key: string): void {
  if (activeControlsTab.value === key) return
  workflowStore.selectControlsTab(key)
}

function selectResultsTab(key: string): void {
  if (activeResultsTab.value === key) return
  workflowStore.selectResultsTab(key)
}

function currentGridWidth(): number {
  const fallback = Math.max(960, viewportWidth.value - 48)
  return Math.max(1, workspaceGridRef.value?.clientWidth ?? fallback)
}

function clampSplitFraction(raw: number): number {
  const totalWidth = currentGridWidth()
  const minControlsPx = 420
  const maxControlsPx = Math.min(760, Math.max(minControlsPx, totalWidth - 520))
  const minFraction = Math.min(0.6, minControlsPx / totalWidth)
  const maxFraction = Math.max(minFraction, Math.min(0.6, maxControlsPx / totalWidth))
  if (!Number.isFinite(raw)) return Math.min(maxFraction, Math.max(minFraction, 0.34))
  return Math.min(maxFraction, Math.max(minFraction, raw))
}

function syncViewportMetrics(): void {
  viewportWidth.value = window.innerWidth
  splitFraction.value = clampSplitFraction(splitFraction.value)
}

function onWindowResize(): void {
  syncViewportMetrics()
}

function onDividerPointerDown(event: PointerEvent): void {
  if (!splitLayoutActive.value || event.button !== 0) return
  const target = event.currentTarget as HTMLElement | null
  target?.setPointerCapture?.(event.pointerId)
  dividerDrag.value = {
    pointerId: event.pointerId,
    startClientX: event.clientX,
    startFraction: clampSplitFraction(splitFraction.value),
  }
}

function onDividerPointerMove(event: PointerEvent): void {
  const drag = dividerDrag.value
  if (!drag || drag.pointerId !== event.pointerId) return
  const deltaX = event.clientX - drag.startClientX
  splitFraction.value = clampSplitFraction(drag.startFraction + deltaX / currentGridWidth())
  event.preventDefault()
}

function releaseDividerPointer(event: PointerEvent): void {
  const drag = dividerDrag.value
  if (!drag || drag.pointerId !== event.pointerId) return
  const target = event.currentTarget as HTMLElement | null
  if (target?.hasPointerCapture?.(event.pointerId)) target.releasePointerCapture(event.pointerId)
  dividerDrag.value = null
}

function onDividerPointerUp(event: PointerEvent): void {
  releaseDividerPointer(event)
}

function onDividerPointerCancel(event: PointerEvent): void {
  releaseDividerPointer(event)
}

function onDividerKeydown(event: KeyboardEvent): void {
  if (!splitLayoutActive.value) return
  if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight' && event.key !== 'Home' && event.key !== 'End') return
  event.preventDefault()
  if (event.key === 'Home') {
    splitFraction.value = clampSplitFraction(0)
    return
  }
  if (event.key === 'End') {
    splitFraction.value = clampSplitFraction(1)
    return
  }
  const delta = event.key === 'ArrowLeft' ? -0.02 : 0.02
  splitFraction.value = clampSplitFraction(splitFraction.value + delta)
}

onMounted(async () => {
  await nextTick()
  syncViewportMetrics()
  window.addEventListener('resize', onWindowResize, { passive: true })
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', onWindowResize)
})
</script>

<template>
  <section class="shellRoot">
    <header class="shellHeader">
      <div class="brandRow">
        <div class="brandBlock">
          <h1>VectorTA Optimizer</h1>
          <div class="headerCaption">Guided default workflow with power tools on demand.</div>
        </div>
        <div class="subtitle mono">{{ shellSubtitle }}</div>
      </div>

      <nav id="workspaceBar" class="tabsRow workspaceBar">
        <button
          v-for="item in workspaceItems"
          :key="`ws_${item.key}`"
          type="button"
          class="tabBtn"
          :data-workspace="item.key"
          :class="{ active: currentWorkspace === item.key }"
          @click="selectWorkspace(item.key)"
        >
          {{ item.label }}
        </button>
      </nav>

      <nav id="workflowStepBar" class="tabsRow workflowStepBar" v-if="currentWorkspace === 'workflow'">
        <button
          v-for="item in workflowStepItems"
          :key="`step_${item.key}`"
          type="button"
          class="tabBtn"
          :data-workflow-step="item.key"
          :class="{ active: currentWorkflowStep === item.key }"
          @click="selectWorkflowStep(item.key)"
        >
          {{ item.label }}
        </button>
      </nav>

      <section class="contextStrip" aria-label="Run context">
        <article v-for="item in contextItems" :key="`ctx_${item.key}`" class="contextCard">
          <div class="contextKey">{{ item.key }}</div>
          <div class="contextValue mono">{{ item.value }}</div>
        </article>
      </section>
    </header>

    <main class="shellMain">
      <section
        ref="workspaceGridRef"
        class="workspaceGrid"
        :class="workspaceGridClass"
        :style="workspaceGridStyle"
        v-show="!settingsWorkspaceActive"
      >
        <aside v-show="controlsColumnVisible" class="controlsColumn">
          <nav v-if="controlsTabsVisible" id="controlsTabBar" class="tabsRow compact">
            <button
              v-for="item in visibleControlsItems"
              :key="`controls_${item.key}`"
              type="button"
              class="tabBtn"
              :data-tab="item.key"
              :class="{ active: activeControlsTab === item.key }"
              @click="selectControlsTab(item.key)"
            >
              {{ item.label }}
            </button>
          </nav>

          <div class="panelHost" data-vue-panel="data">
            <div id="dataVueShellMount"></div>
          </div>
          <div class="panelHost" data-vue-panel="opt">
            <div id="optVueShellMount"></div>
          </div>
          <div class="panelHost" data-vue-panel="strategy">
            <div id="strategyVueShellMount"></div>
          </div>
          <div class="panelHost" data-vue-panel="advanced">
            <div id="advancedVueShellMount"></div>
          </div>
        </aside>

        <button
          v-if="splitLayoutActive"
          type="button"
          class="workspaceDivider"
          aria-label="Resize setup and results panels"
          @pointerdown="onDividerPointerDown"
          @pointermove="onDividerPointerMove"
          @pointerup="onDividerPointerUp"
          @pointercancel="onDividerPointerCancel"
          @keydown="onDividerKeydown"
        >
          <span class="workspaceDividerGrip" aria-hidden="true"></span>
        </button>

        <section class="resultsColumn">
          <nav v-if="resultsTabsVisible" id="resultsTabBar" class="tabsRow compact">
            <button
              v-for="item in visibleResultsItems"
              :key="`results_${item.key}`"
              type="button"
              class="tabBtn"
              :data-tab="item.key"
              :class="{ active: activeResultsTab === item.key }"
              @click="selectResultsTab(item.key)"
            >
              {{ item.label }}
            </button>
          </nav>

          <div class="panelHost" data-vue-panel="overview">
            <div id="overviewVueShellMount"></div>
          </div>
          <div class="panelHost" data-vue-panel="top">
            <div id="topVueShellMount"></div>
          </div>
          <div class="panelHost" data-vue-panel="pareto">
            <div id="paretoVueShellMount"></div>
          </div>
          <div class="panelHost" data-vue-panel="validate">
            <div id="validationVueShellMount"></div>
          </div>
          <div class="panelHost" data-vue-panel="drill">
            <div id="drillVueShellMount"></div>
          </div>
          <div class="panelHost" data-vue-panel="heatmap">
            <div id="heatmapVueShellMount"></div>
          </div>
          <div class="panelHost" data-vue-panel="history">
            <div id="historyVueShellMount"></div>
          </div>
          <div class="panelHost" data-vue-panel="meta">
            <div id="metaVueShellMount"></div>
          </div>
          <div class="panelHost" data-vue-panel="debug">
            <div id="debugVueShellMount"></div>
          </div>
        </section>
      </section>

      <section v-show="settingsWorkspaceActive" id="settingsShellGrid">
        <div id="settingsVueShellMount"></div>
        <div id="executionShellPane">
          <div id="executionVueShellMount"></div>
        </div>
      </section>
    </main>

    <footer id="runFooterShell">
      <div id="runFooterVueShellMount"></div>
    </footer>
  </section>
</template>

<style scoped>
.shellRoot {
  position: fixed;
  inset: 0;
  z-index: 1200;
  background: var(--app-bg);
  color: var(--text);
  display: grid;
  grid-template-rows: auto 1fr auto;
  pointer-events: auto;
}

.shellHeader {
  border-bottom: 1px solid var(--border);
  background: var(--header-bg);
  padding: 0.85rem 1rem 0.8rem;
  display: grid;
  gap: 0.5rem;
  box-shadow: 0 10px 28px color-mix(in srgb, var(--accent-soft) 38%, transparent);
}

.brandRow {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
}

.brandBlock {
  display: grid;
  gap: 0.18rem;
}

.brandRow h1 {
  margin: 0;
  font-size: 1.05rem;
  letter-spacing: 0.01em;
}

.headerCaption {
  font-size: 0.74rem;
  color: var(--muted);
}

.subtitle {
  font-size: 0.74rem;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: min(42rem, 56%);
  padding: 0.28rem 0.62rem;
  border: 1px solid color-mix(in srgb, var(--border) 82%, transparent);
  border-radius: 999px;
  background: color-mix(in srgb, var(--surface) 96%, transparent);
}

.tabsRow {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.tabBtn {
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--surface2);
  color: inherit;
  font-size: 0.76rem;
  padding: 0.38rem 0.7rem;
  font-weight: 600;
}

.tabBtn.active {
  border-color: color-mix(in oklab, var(--accent) 64%, var(--border));
  background: color-mix(in oklab, var(--accent-soft) 68%, var(--surface2));
  box-shadow: 0 8px 18px color-mix(in srgb, var(--accent-soft) 28%, transparent);
}

.contextStrip {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 0.32rem;
}

.contextCard {
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in oklab, var(--surface) 98%, var(--panel));
  padding: 0.34rem 0.48rem;
  display: grid;
  gap: 0.08rem;
}

.contextKey {
  font-size: 0.62rem;
  letter-spacing: 0.08em;
  color: var(--muted);
  text-transform: uppercase;
}

.contextValue {
  font-size: 0.73rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.shellMain {
  min-height: 0;
  overflow: auto;
  display: flex;
  flex-direction: column;
  width: 100%;
  padding: 0;
}

.workspaceGrid {
  flex: 1 1 auto;
  height: 100%;
  width: 100%;
  min-width: 0;
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(420px, 34%) 14px minmax(0, 1fr);
  grid-template-rows: minmax(0, 1fr);
  gap: 1rem;
  padding: 1rem;
  align-items: stretch;
  align-content: stretch;
  box-sizing: border-box;
}

.workspaceGrid.singleColumn {
  grid-template-columns: minmax(0, 1fr);
}

.workspaceGrid.stacked {
  grid-template-columns: minmax(0, 1fr);
  grid-template-rows: auto minmax(0, 1fr);
}

.controlsColumn,
.resultsColumn {
  min-height: 0;
  min-width: 0;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 18px;
  background: var(--panel);
  display: grid;
  align-content: start;
  gap: 0.7rem;
  box-shadow: var(--shadow-md);
}

.controlsColumn {
  padding: 0.95rem;
  background:
    linear-gradient(
      180deg,
      color-mix(in oklab, var(--accent-soft) 30%, var(--panel)),
      color-mix(in oklab, var(--panel) 98%, var(--surface))
    );
}

.resultsColumn {
  padding: 0.8rem;
  background:
    linear-gradient(
      180deg,
      color-mix(in oklab, var(--surface) 97%, var(--panel)),
      color-mix(in oklab, var(--panel) 98%, var(--surface2))
    );
}

.workspaceDivider {
  position: relative;
  align-self: stretch;
  width: 14px;
  min-width: 14px;
  padding: 0;
  border: 0;
  border-radius: 999px;
  background: transparent;
  cursor: col-resize;
  touch-action: none;
}

.workspaceDivider::before {
  content: '';
  position: absolute;
  left: 50%;
  top: 0;
  bottom: 0;
  width: 1px;
  transform: translateX(-50%);
  background: color-mix(in oklab, var(--border) 88%, transparent);
}

.workspaceDividerGrip {
  position: absolute;
  left: 50%;
  top: 50%;
  width: 8px;
  height: 88px;
  transform: translate(-50%, -50%);
  border-radius: 999px;
  background:
    radial-gradient(circle at center, color-mix(in oklab, var(--accent) 42%, var(--border)) 0 1px, transparent 1.5px) center / 8px 12px repeat-y,
    color-mix(in oklab, var(--surface2) 86%, transparent);
  border: 1px solid color-mix(in oklab, var(--border) 80%, transparent);
  box-shadow: 0 8px 18px color-mix(in srgb, var(--accent-soft) 22%, transparent);
}

.workspaceDivider:hover .workspaceDividerGrip,
.workspaceDivider:focus-visible .workspaceDividerGrip {
  background:
    radial-gradient(circle at center, color-mix(in oklab, var(--accent) 62%, var(--border)) 0 1px, transparent 1.5px) center / 8px 12px repeat-y,
    color-mix(in oklab, var(--surface) 92%, transparent);
  border-color: color-mix(in oklab, var(--accent) 34%, var(--border));
}

.tabsRow.compact .tabBtn {
  font-size: 0.73rem;
  padding: 0.32rem 0.58rem;
}

.panelHost {
  min-height: 0;
  min-width: 0;
}

#settingsShellGrid {
  flex: 1 1 auto;
  height: 100%;
  width: 100%;
  min-width: 0;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 18px;
  background: color-mix(in oklab, var(--panel) 98%, var(--surface));
  margin: 1rem;
  padding: 1rem;
  display: grid;
  gap: 0.9rem;
  box-shadow: var(--shadow-md);
}

#executionShellPane {
  border-top: 1px dashed var(--border);
  padding-top: 0.9rem;
}

#runFooterShell {
  border-top: 1px solid var(--border);
  background: color-mix(in srgb, var(--panel-footer) 98%, var(--surface));
  padding: 0.55rem 0.9rem;
  box-shadow: 0 -10px 24px color-mix(in srgb, var(--accent-soft) 18%, transparent);
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1400px) {
  .contextStrip {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
}

@media (max-width: 1200px) {
  .contextStrip {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .workspaceGrid {
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr);
  }
}

@media (max-width: 900px) {
  .workspaceBar,
  .workflowStepBar,
  .tabsRow.compact {
    flex-wrap: nowrap;
    overflow-x: auto;
    padding-bottom: 0.15rem;
    scrollbar-width: thin;
  }

  .workspaceBar .tabBtn,
  .workflowStepBar .tabBtn,
  .tabsRow.compact .tabBtn {
    flex: 0 0 auto;
  }

  .contextStrip {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 760px) {
  .shellHeader {
    padding: 0.8rem 0.75rem 0.7rem;
  }

  .brandRow {
    flex-direction: column;
  }

  .subtitle {
    max-width: 100%;
  }

  .contextStrip {
    grid-template-columns: 1fr;
  }

  .workspaceGrid {
    padding: 0.75rem;
    gap: 0.75rem;
  }

  #settingsShellGrid {
    margin: 0.75rem;
    padding: 0.75rem;
  }

  #runFooterShell {
    padding: 0.7rem 0.75rem;
  }
}
</style>
