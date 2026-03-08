<script setup lang="ts">
import { useStorage } from '@vueuse/core'
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useWorkflowStore } from '../stores/workflowStore'

const workflowStore = useWorkflowStore()
const workspaceGridRef = ref<HTMLElement | null>(null)
const viewportWidth = ref(1440)
const splitFraction = useStorage<number>('VTA_WORKSPACE_SPLIT_V1', 0.34)
const dividerDrag = ref<{ pointerId: number; startClientX: number; startFraction: number } | null>(null)

const { controlsTab, resultsTab, workspace, workflow, workflowStep, workflowSubtitle } = storeToRefs(workflowStore)

const workspaceItems = [
  { key: 'workflow', label: 'Workflow' },
  { key: 'library', label: 'Run Library' },
  { key: 'aggregate', label: 'Aggregate Analysis' },
  { key: 'pro', label: 'Pro Tools' },
  { key: 'settings', label: 'Settings' },
] as const

const workflowItems = [
  { key: 'data', label: 'Data' },
  { key: 'optimize', label: 'Optimize' },
  { key: 'validate', label: 'Validate' },
  { key: 'analyze', label: 'Analyze' },
  { key: 'history', label: 'History' },
  { key: 'settings', label: 'Settings' },
] as const

const workflowStepItems = [
  { key: 'data', label: '1 Data Source' },
  { key: 'optimize', label: '2 Strategy Setup' },
  { key: 'validate', label: '3 Validation' },
  { key: 'analyze', label: '4 Summary' },
] as const

const controlsItems = [
  { key: 'data', label: 'Data' },
  { key: 'opt', label: 'Optimize' },
  { key: 'strategy', label: 'Strategy' },
  { key: 'advanced', label: 'Advanced' },
  { key: 'distributed', label: 'Distributed' },
] as const

const resultsItems = [
  { key: 'overview', label: 'Overview' },
  { key: 'top', label: 'Top' },
  { key: 'pareto', label: 'Pareto' },
  { key: 'validate', label: 'Validate' },
  { key: 'drill', label: 'Drilldown' },
  { key: 'heatmap', label: 'Heatmap' },
  { key: 'history', label: 'History' },
  { key: 'meta', label: 'Meta' },
  { key: 'debug', label: 'Debug' },
] as const

const effectiveControlsTab = computed(() => String(controlsTab.value || '').trim() || 'data')
const effectiveResultsTab = computed(() => String(resultsTab.value || '').trim() || 'overview')

const settingsWorkspaceActive = computed(() => String(workspace.value || '').trim() === 'settings')
const splitLayoutActive = computed(() => !settingsWorkspaceActive.value && viewportWidth.value > 1200)
const shellSubtitle = computed(() => {
  const text = String(workflowSubtitle.value || '').trim()
  return text || '—'
})
const workspaceGridStyle = computed(() => {
  if (!splitLayoutActive.value) return undefined
  const fraction = clampSplitFraction(splitFraction.value)
  return {
    gridTemplateColumns: `${(fraction * 100).toFixed(3)}% 14px minmax(0, 1fr)`,
  }
})

function selectWorkspace(key: string): void {
  if (String(workspace.value || '').trim() === key) return
  workflowStore.selectWorkspace(key)
}

function selectWorkflow(key: string): void {
  if (String(workflow.value || '').trim() === key) return
  workflowStore.selectWorkflow(key)
}

function selectWorkflowStep(key: string): void {
  if (String(workspace.value || '').trim() === 'workflow' && String(workflowStep.value || '').trim() === key) return
  workflowStore.selectWorkflowStep(key)
}

function selectControlsTab(key: string): void {
  if (String(effectiveControlsTab.value || '').trim() === key) return
  workflowStore.selectControlsTab(key)
}

function selectResultsTab(key: string): void {
  if (String(effectiveResultsTab.value || '').trim() === key) return
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
        <h1>VectorTA Optimizer</h1>
        <div class="subtitle mono">{{ shellSubtitle }}</div>
      </div>

      <nav id="workspaceBar" class="tabsRow workspaceBar">
        <button
          v-for="item in workspaceItems"
          :key="`ws_${item.key}`"
          type="button"
          class="tabBtn"
          :data-workspace="item.key"
          :class="{ active: workspace === item.key }"
          @click="selectWorkspace(item.key)"
        >
          {{ item.label }}
        </button>
      </nav>

      <nav id="workflowStepBar" class="tabsRow workflowStepBar" v-if="workspace === 'workflow'">
        <button
          v-for="item in workflowStepItems"
          :key="`step_${item.key}`"
          type="button"
          class="tabBtn"
          :data-workflow-step="item.key"
          :class="{ active: workflowStep === item.key }"
          @click="selectWorkflowStep(item.key)"
        >
          {{ item.label }}
        </button>
      </nav>

      <nav id="workflowTabBar" class="tabsRow workflowBar">
        <button
          v-for="item in workflowItems"
          :key="`wf_${item.key}`"
          type="button"
          class="tabBtn"
          :data-workflow="item.key"
          :class="{ active: workflow === item.key }"
          @click="selectWorkflow(item.key)"
        >
          {{ item.label }}
        </button>
      </nav>
    </header>

    <main class="shellMain">
      <section ref="workspaceGridRef" class="workspaceGrid" :style="workspaceGridStyle" v-show="!settingsWorkspaceActive">
        <aside class="controlsColumn">
          <nav id="controlsTabBar" class="tabsRow compact">
            <button
              v-for="item in controlsItems"
              :key="`controls_${item.key}`"
              type="button"
              class="tabBtn"
              :data-tab="item.key"
              :class="{ active: effectiveControlsTab === item.key }"
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
          <div class="panelHost" data-vue-panel="distributed">
            <div id="distributedVueShellMount"></div>
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
          <nav id="resultsTabBar" class="tabsRow compact">
            <button
              v-for="item in resultsItems"
              :key="`results_${item.key}`"
              type="button"
              class="tabBtn"
              :data-tab="item.key"
              :class="{ active: effectiveResultsTab === item.key }"
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
  padding: 0.95rem 1.1rem 0.85rem;
  display: grid;
  gap: 0.55rem;
  box-shadow: 0 10px 28px color-mix(in srgb, var(--accent-soft) 38%, transparent);
}

.brandRow {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.9rem;
  padding-bottom: 0.15rem;
}

.brandRow h1 {
  margin: 0;
  font-size: 1.05rem;
  letter-spacing: 0.01em;
}

.subtitle {
  font-size: 0.78rem;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: min(48rem, 58%);
  padding: 0.38rem 0.72rem;
  border: 1px solid color-mix(in srgb, var(--border) 82%, transparent);
  border-radius: 999px;
  background: color-mix(in srgb, var(--surface) 88%, transparent);
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

.controlsColumn,
.resultsColumn {
  min-height: 0;
  min-width: 0;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 18px;
  background: var(--panel);
  padding: 0.8rem;
  display: grid;
  align-content: start;
  gap: 0.7rem;
  box-shadow: var(--shadow-md);
}

.controlsColumn {
  padding: 0.95rem;
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
  background: var(--panel);
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
  background: color-mix(in srgb, var(--panel-footer) 92%, var(--surface));
  padding: 0.7rem 1rem;
  box-shadow: 0 -10px 24px color-mix(in srgb, var(--accent-soft) 18%, transparent);
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1200px) {
  .workspaceGrid {
    grid-template-columns: 1fr;
    grid-template-rows: auto auto;
  }

  .subtitle {
    max-width: 100%;
  }
}

@media (max-width: 900px) {
  .workspaceBar,
  .workflowStepBar,
  .workflowBar,
  .tabsRow.compact {
    flex-wrap: nowrap;
    overflow-x: auto;
    padding-bottom: 0.15rem;
    scrollbar-width: thin;
  }

  .workspaceBar .tabBtn,
  .workflowStepBar .tabBtn,
  .workflowBar .tabBtn,
  .tabsRow.compact .tabBtn {
    flex: 0 0 auto;
  }
}

@media (max-width: 760px) {
  .shellHeader {
    padding: 0.8rem 0.75rem 0.7rem;
  }

  .brandRow {
    align-items: flex-start;
    flex-direction: column;
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
