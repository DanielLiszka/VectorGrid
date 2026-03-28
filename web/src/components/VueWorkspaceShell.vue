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

type TabKeyItem = { key: string }

const shellMainPanelId = 'shellMainPanel'
const shellSelectionStatusId = 'shellSelectionStatus'

function workspaceTabId(key: string): string {
  return `workspace_tab_${key}`
}

function workflowStepTabId(key: string): string {
  return `workflow_step_tab_${key}`
}

function controlTabId(key: string): string {
  return `controls_tab_${key}`
}

function resultTabId(key: string): string {
  return `results_tab_${key}`
}

const activeControlsTab = computed(() => String(controlsTab.value || '').trim() || 'data')
const activeResultsTab = computed(() => String(resultsTab.value || '').trim() || 'overview')
const currentWorkspace = computed(() => String(workspace.value || '').trim() || 'workflow')
const currentWorkflowStep = computed(() => String(workflowStep.value || '').trim() || 'data')
const settingsWorkspaceActive = computed(() => currentWorkspace.value === 'settings')
const currentWorkspaceLabel = computed(() => workspaceItems.find((item) => item.key === currentWorkspace.value)?.label || 'Workflow')
const currentWorkflowStepLabel = computed(
  () => workflowStepItems.find((item) => item.key === currentWorkflowStep.value)?.label || '1 Data Source',
)
const currentControlsLabel = computed(
  () => visibleControlsItems.value.find((item) => item.key === activeControlsTab.value)?.label || 'No setup panel',
)
const currentResultsLabel = computed(
  () => visibleResultsItems.value.find((item) => item.key === activeResultsTab.value)?.label || 'No result panel',
)
const selectedShellTabId = computed(() =>
  currentWorkspace.value === 'workflow'
    ? workflowStepTabId(currentWorkflowStep.value)
    : workspaceTabId(currentWorkspace.value),
)
const shellSelectionAnnouncement = computed(() => {
  const parts = [`Workspace ${currentWorkspaceLabel.value}.`]
  if (currentWorkspace.value === 'workflow') parts.push(`Workflow step ${currentWorkflowStepLabel.value}.`)
  if (visibleControlsItems.value.length > 0) parts.push(`Setup panel ${currentControlsLabel.value}.`)
  if (visibleResultsItems.value.length > 0) parts.push(`Results panel ${currentResultsLabel.value}.`)
  return parts.join(' ')
})
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
const splitValuePercent = computed(() => Math.round(clampSplitFraction(splitFraction.value) * 100))
const splitMinPercent = computed(() => Math.round(clampSplitFraction(0) * 100))
const splitMaxPercent = computed(() => Math.round(clampSplitFraction(1) * 100))

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

function focusSelector(selector: string): void {
  const element = document.querySelector<HTMLElement>(selector)
  element?.focus()
}

function controlsPanelId(key: string): string {
  return `controls_panel_${key}`
}

function resultsPanelId(key: string): string {
  return `results_panel_${key}`
}

function visibleItemKeys(items: readonly TabKeyItem[]): string[] {
  return items.map((item) => item.key)
}

function resolveFocusedNavigationKey(event: KeyboardEvent): string | null {
  const target =
    event.target instanceof Element
      ? (event.target.closest('[data-workspace], [data-workflow-step], [data-tab]') as HTMLElement | null)
      : null
  if (!target) return null
  const focusedKey =
    String(target.dataset.workspace || '').trim() ||
    String(target.dataset.workflowStep || '').trim() ||
    String(target.dataset.tab || '').trim()
  return focusedKey || null
}

function isControlPanelVisible(key: string): boolean {
  return visibleItemKeys(visibleControlsItems.value).includes(key) && activeControlsTab.value === key
}

function isResultsPanelVisible(key: string): boolean {
  return visibleItemKeys(visibleResultsItems.value).includes(key) && activeResultsTab.value === key
}

function moveTabFocus(
  event: KeyboardEvent,
  items: readonly TabKeyItem[],
  currentKey: string,
  selectTab: (key: string) => void,
  selectorForKey: (key: string) => string,
): void {
  if (items.length <= 1) return
  const originKey = resolveFocusedNavigationKey(event) ?? currentKey
  const currentIndex = items.findIndex((item) => item.key === originKey)
  if (currentIndex < 0) return

  let nextIndex = currentIndex
  if (event.key === 'ArrowRight' || event.key === 'ArrowDown') nextIndex = (currentIndex + 1) % items.length
  else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') nextIndex = (currentIndex - 1 + items.length) % items.length
  else if (event.key === 'Home') nextIndex = 0
  else if (event.key === 'End') nextIndex = items.length - 1
  else return

  event.preventDefault()
  const nextKey = items[nextIndex]?.key
  if (!nextKey) return
  selectTab(nextKey)
  void nextTick(() => focusSelector(selectorForKey(nextKey)))
}

function onWorkspaceKeydown(event: KeyboardEvent): void {
  moveTabFocus(event, workspaceItems, currentWorkspace.value, selectWorkspace, (key) => `#workspaceBar [data-workspace="${key}"]`)
}

function onWorkflowStepKeydown(event: KeyboardEvent): void {
  moveTabFocus(
    event,
    workflowStepItems,
    currentWorkflowStep.value,
    selectWorkflowStep,
    (key) => `#workflowStepBar [data-workflow-step="${key}"]`,
  )
}

function onControlsTabKeydown(event: KeyboardEvent): void {
  moveTabFocus(
    event,
    visibleControlsItems.value,
    activeControlsTab.value,
    selectControlsTab,
    (key) => `#controlsTabBar [data-tab="${key}"]`,
  )
}

function onResultsTabKeydown(event: KeyboardEvent): void {
  moveTabFocus(
    event,
    visibleResultsItems.value,
    activeResultsTab.value,
    selectResultsTab,
    (key) => `#resultsTabBar [data-tab="${key}"]`,
  )
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
          <h1>VectorGrid</h1>
          <div class="headerCaption">Guided default workflow with power tools on demand.</div>
        </div>
        <div class="subtitle mono">{{ shellSubtitle }}</div>
      </div>

      <nav
        id="workspaceBar"
        class="tabsRow workspaceBar"
        aria-label="Workspace sections"
        role="tablist"
        aria-orientation="horizontal"
        @keydown="onWorkspaceKeydown"
      >
        <button
          v-for="item in workspaceItems"
          :key="`ws_${item.key}`"
          :id="workspaceTabId(item.key)"
          type="button"
          class="tabBtn"
          :data-workspace="item.key"
          :class="{ active: currentWorkspace === item.key }"
          role="tab"
          :aria-selected="currentWorkspace === item.key"
          :aria-controls="shellMainPanelId"
          :tabindex="currentWorkspace === item.key ? 0 : -1"
          @click="selectWorkspace(item.key)"
        >
          {{ item.label }}
        </button>
      </nav>

      <nav
        id="workflowStepBar"
        class="tabsRow workflowStepBar"
        v-if="currentWorkspace === 'workflow'"
        aria-label="Workflow steps"
        role="tablist"
        aria-orientation="horizontal"
        @keydown="onWorkflowStepKeydown"
      >
        <button
          v-for="item in workflowStepItems"
          :key="`step_${item.key}`"
          :id="workflowStepTabId(item.key)"
          type="button"
          class="tabBtn"
          :data-workflow-step="item.key"
          :class="{ active: currentWorkflowStep === item.key }"
          role="tab"
          :aria-selected="currentWorkflowStep === item.key"
          :aria-controls="shellMainPanelId"
          :tabindex="currentWorkflowStep === item.key ? 0 : -1"
          @click="selectWorkflowStep(item.key)"
        >
          {{ item.label }}
        </button>
      </nav>

      <div
        :id="shellSelectionStatusId"
        class="srOnly"
        role="status"
        aria-live="polite"
        aria-atomic="true"
      >
        {{ shellSelectionAnnouncement }}
      </div>

      <section class="contextStrip" aria-label="Run context">
        <article v-for="item in contextItems" :key="`ctx_${item.key}`" class="contextCard">
          <div class="contextKey">{{ item.key }}</div>
          <div class="contextValue mono">{{ item.value }}</div>
        </article>
      </section>
    </header>

    <main class="shellMain">
      <section
        :id="shellMainPanelId"
        class="shellMainPanel"
        role="tabpanel"
        :aria-labelledby="selectedShellTabId"
        :aria-describedby="shellSelectionStatusId"
      >
        <section
          ref="workspaceGridRef"
          class="workspaceGrid"
          :class="workspaceGridClass"
          :style="workspaceGridStyle"
          v-show="!settingsWorkspaceActive"
        >
        <aside id="shellControlsColumn" v-show="controlsColumnVisible" class="controlsColumn">
          <nav
            v-if="controlsTabsVisible"
            id="controlsTabBar"
            class="tabsRow compact"
            aria-label="Setup panels"
            role="tablist"
            aria-orientation="horizontal"
            @keydown="onControlsTabKeydown"
          >
            <button
              v-for="item in visibleControlsItems"
              :key="`controls_${item.key}`"
              :id="controlTabId(item.key)"
              type="button"
              class="tabBtn"
              :data-tab="item.key"
              :class="{ active: activeControlsTab === item.key }"
              role="tab"
              :aria-selected="activeControlsTab === item.key"
              :aria-controls="controlsPanelId(item.key)"
              :tabindex="activeControlsTab === item.key ? 0 : -1"
              @click="selectControlsTab(item.key)"
            >
              {{ item.label }}
            </button>
          </nav>

          <div
            :id="controlsPanelId('data')"
            class="panelHost"
            data-vue-panel="data"
            :role="controlsTabsVisible ? 'tabpanel' : 'region'"
            :aria-labelledby="controlsTabsVisible ? controlTabId('data') : undefined"
            :aria-label="controlsTabsVisible ? undefined : 'Data'"
            :aria-hidden="!isControlPanelVisible('data')"
            v-show="isControlPanelVisible('data')"
          >
            <div id="dataVueShellMount"></div>
          </div>
          <div
            :id="controlsPanelId('opt')"
            class="panelHost"
            data-vue-panel="opt"
            :role="controlsTabsVisible ? 'tabpanel' : 'region'"
            :aria-labelledby="controlsTabsVisible ? controlTabId('opt') : undefined"
            :aria-label="controlsTabsVisible ? undefined : 'Optimize'"
            :aria-hidden="!isControlPanelVisible('opt')"
            v-show="isControlPanelVisible('opt')"
          >
            <div id="optVueShellMount"></div>
          </div>
          <div
            :id="controlsPanelId('strategy')"
            class="panelHost"
            data-vue-panel="strategy"
            :role="controlsTabsVisible ? 'tabpanel' : 'region'"
            :aria-labelledby="controlsTabsVisible ? controlTabId('strategy') : undefined"
            :aria-label="controlsTabsVisible ? undefined : 'Strategy'"
            :aria-hidden="!isControlPanelVisible('strategy')"
            v-show="isControlPanelVisible('strategy')"
          >
            <div id="strategyVueShellMount"></div>
          </div>
          <div
            :id="controlsPanelId('advanced')"
            class="panelHost"
            data-vue-panel="advanced"
            :role="controlsTabsVisible ? 'tabpanel' : 'region'"
            :aria-labelledby="controlsTabsVisible ? controlTabId('advanced') : undefined"
            :aria-label="controlsTabsVisible ? undefined : 'Advanced'"
            :aria-hidden="!isControlPanelVisible('advanced')"
            v-show="isControlPanelVisible('advanced')"
          >
            <div id="advancedVueShellMount"></div>
          </div>
        </aside>

        <button
          v-if="splitLayoutActive"
          type="button"
          class="workspaceDivider"
          aria-label="Resize setup and results panels"
          aria-controls="shellControlsColumn shellResultsColumn"
          aria-orientation="vertical"
          :aria-valuemin="splitMinPercent"
          :aria-valuemax="splitMaxPercent"
          :aria-valuenow="splitValuePercent"
          :aria-valuetext="`${splitValuePercent}% setup width`"
          role="separator"
          @pointerdown="onDividerPointerDown"
          @pointermove="onDividerPointerMove"
          @pointerup="onDividerPointerUp"
          @pointercancel="onDividerPointerCancel"
          @keydown="onDividerKeydown"
        >
          <span class="workspaceDividerGrip" aria-hidden="true"></span>
        </button>

        <section id="shellResultsColumn" class="resultsColumn">
          <nav
            v-if="resultsTabsVisible"
            id="resultsTabBar"
            class="tabsRow compact"
            aria-label="Result panels"
            role="tablist"
            aria-orientation="horizontal"
            @keydown="onResultsTabKeydown"
          >
            <button
              v-for="item in visibleResultsItems"
              :key="`results_${item.key}`"
              :id="resultTabId(item.key)"
              type="button"
              class="tabBtn"
              :data-tab="item.key"
              :class="{ active: activeResultsTab === item.key }"
              role="tab"
              :aria-selected="activeResultsTab === item.key"
              :aria-controls="resultsPanelId(item.key)"
              :tabindex="activeResultsTab === item.key ? 0 : -1"
              @click="selectResultsTab(item.key)"
            >
              {{ item.label }}
            </button>
          </nav>

          <div
            :id="resultsPanelId('overview')"
            class="panelHost"
            data-vue-panel="overview"
            :role="resultsTabsVisible ? 'tabpanel' : 'region'"
            :aria-labelledby="resultsTabsVisible ? resultTabId('overview') : undefined"
            :aria-label="resultsTabsVisible ? undefined : 'Overview'"
            :aria-hidden="!isResultsPanelVisible('overview')"
            v-show="isResultsPanelVisible('overview')"
          >
            <div id="overviewVueShellMount"></div>
          </div>
          <div
            :id="resultsPanelId('top')"
            class="panelHost"
            data-vue-panel="top"
            :role="resultsTabsVisible ? 'tabpanel' : 'region'"
            :aria-labelledby="resultsTabsVisible ? resultTabId('top') : undefined"
            :aria-label="resultsTabsVisible ? undefined : 'Top'"
            :aria-hidden="!isResultsPanelVisible('top')"
            v-show="isResultsPanelVisible('top')"
          >
            <div id="topVueShellMount"></div>
          </div>
          <div
            :id="resultsPanelId('pareto')"
            class="panelHost"
            data-vue-panel="pareto"
            :role="resultsTabsVisible ? 'tabpanel' : 'region'"
            :aria-labelledby="resultsTabsVisible ? resultTabId('pareto') : undefined"
            :aria-label="resultsTabsVisible ? undefined : 'Pareto'"
            :aria-hidden="!isResultsPanelVisible('pareto')"
            v-show="isResultsPanelVisible('pareto')"
          >
            <div id="paretoVueShellMount"></div>
          </div>
          <div
            :id="resultsPanelId('validate')"
            class="panelHost"
            data-vue-panel="validate"
            :role="resultsTabsVisible ? 'tabpanel' : 'region'"
            :aria-labelledby="resultsTabsVisible ? resultTabId('validate') : undefined"
            :aria-label="resultsTabsVisible ? undefined : 'Validate'"
            :aria-hidden="!isResultsPanelVisible('validate')"
            v-show="isResultsPanelVisible('validate')"
          >
            <div id="validationVueShellMount"></div>
          </div>
          <div
            :id="resultsPanelId('drill')"
            class="panelHost"
            data-vue-panel="drill"
            :role="resultsTabsVisible ? 'tabpanel' : 'region'"
            :aria-labelledby="resultsTabsVisible ? resultTabId('drill') : undefined"
            :aria-label="resultsTabsVisible ? undefined : 'Drilldown'"
            :aria-hidden="!isResultsPanelVisible('drill')"
            v-show="isResultsPanelVisible('drill')"
          >
            <div id="drillVueShellMount"></div>
          </div>
          <div
            :id="resultsPanelId('heatmap')"
            class="panelHost"
            data-vue-panel="heatmap"
            :role="resultsTabsVisible ? 'tabpanel' : 'region'"
            :aria-labelledby="resultsTabsVisible ? resultTabId('heatmap') : undefined"
            :aria-label="resultsTabsVisible ? undefined : 'Heatmap'"
            :aria-hidden="!isResultsPanelVisible('heatmap')"
            v-show="isResultsPanelVisible('heatmap')"
          >
            <div id="heatmapVueShellMount"></div>
          </div>
          <div
            :id="resultsPanelId('history')"
            class="panelHost"
            data-vue-panel="history"
            :role="resultsTabsVisible ? 'tabpanel' : 'region'"
            :aria-labelledby="resultsTabsVisible ? resultTabId('history') : undefined"
            :aria-label="resultsTabsVisible ? undefined : 'History'"
            :aria-hidden="!isResultsPanelVisible('history')"
            v-show="isResultsPanelVisible('history')"
          >
            <div id="historyVueShellMount"></div>
          </div>
          <div
            :id="resultsPanelId('meta')"
            class="panelHost"
            data-vue-panel="meta"
            :role="resultsTabsVisible ? 'tabpanel' : 'region'"
            :aria-labelledby="resultsTabsVisible ? resultTabId('meta') : undefined"
            :aria-label="resultsTabsVisible ? undefined : 'Meta'"
            :aria-hidden="!isResultsPanelVisible('meta')"
            v-show="isResultsPanelVisible('meta')"
          >
            <div id="metaVueShellMount"></div>
          </div>
          <div
            :id="resultsPanelId('debug')"
            class="panelHost"
            data-vue-panel="debug"
            :role="resultsTabsVisible ? 'tabpanel' : 'region'"
            :aria-labelledby="resultsTabsVisible ? resultTabId('debug') : undefined"
            :aria-label="resultsTabsVisible ? undefined : 'Debug'"
            :aria-hidden="!isResultsPanelVisible('debug')"
            v-show="isResultsPanelVisible('debug')"
          >
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

.shellMainPanel {
  flex: 1 1 auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
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
