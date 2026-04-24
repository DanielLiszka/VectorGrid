<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { ListTodo } from 'lucide-vue-next'
import AppVisualIcon from './AppVisualIcon.vue'
import RunContextBar from './RunContextBar.vue'
import { TASK_CENTER_PANEL_ID, openTaskCenter as requestTaskCenterOpen, useTaskCenterVisibility } from '../composables/useTaskCenterVisibility'
import { useTaskLifecycleSummary } from '../composables/useTaskLifecycleSummary'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useSettingsStore } from '../stores/settingsStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { shellModeLabel, workflowStepLabelForMode } from '../utils/core/workflowModel'
import { shellPostureIcon, shellPostureLabel } from '../utils/appSvgAssets'

const workflowStore = useWorkflowStore()
const analysisStore = useAnalysisStore()
const dataStore = useDataStore()
const optimizeStore = useOptimizeStore()
const settingsStore = useSettingsStore()
const { activeTaskCount, centerStatusLabel } = useTaskLifecycleSummary()
const taskCenterOpen = useTaskCenterVisibility()

const workspaceGridRef = ref<HTMLElement | null>(null)
const viewportWidth = ref(1440)
const dividerDrag = ref<{ pointerId: number; startClientX: number; startFraction: number } | null>(null)

const { controlsTab, mode, resultsTab, splitFraction, workspace, workflowStep, workflowSubtitle } = storeToRefs(workflowStore)
const { dataId, dataLen, dataTimeframe, datasetLabel, datasetRange } = storeToRefs(dataStore)
const { selectedTopTitle } = storeToRefs(analysisStore)
const { backend, objective, progressMeta, runReadinessHint, strategyKind } = storeToRefs(optimizeStore)
const { controlsCollapsed } = storeToRefs(settingsStore)

const workspaceItems = [
  { key: 'workflow', label: 'Research' },
  { key: 'library', label: 'Library' },
  { key: 'aggregate', label: 'Aggregate' },
  { key: 'pro', label: 'Diagnostics' },
  { key: 'live', label: 'Live' },
  { key: 'settings', label: 'Settings' },
] as const

const modeItems = [
  { key: 'conventional', label: 'Conventional' },
  { key: 'ml-signal', label: 'ML Signal' },
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
type ContextItem = {
  key: string
  value: string
  current: boolean
  defaultVisible: boolean
}

const shellMainPanelId = 'shellMainPanel'
const shellSelectionStatusId = 'shellSelectionStatus'

function workspaceTabId(key: string): string {
  return `workspace_tab_${key}`
}

function modeTabId(key: string): string {
  return `mode_tab_${key}`
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
const currentMode = computed(() => String(mode.value || '').trim() || 'conventional')
const currentWorkspace = computed(() => String(workspace.value || '').trim() || 'workflow')
const currentWorkflowStep = computed(() => String(workflowStep.value || '').trim() || 'data')
const workflowStepItems = computed(() => [
  { key: 'data', label: `1 ${workflowStepLabelForMode('data', currentMode.value)}` },
  { key: 'optimize', label: `2 ${workflowStepLabelForMode('optimize', currentMode.value)}` },
  { key: 'validate', label: `3 ${workflowStepLabelForMode('validate', currentMode.value)}` },
  { key: 'analyze', label: `4 ${workflowStepLabelForMode('analyze', currentMode.value)}` },
  { key: 'finalize', label: `5 ${workflowStepLabelForMode('finalize', currentMode.value)}` },
])
const settingsWorkspaceActive = computed(() => currentWorkspace.value === 'settings')
const liveWorkspaceActive = computed(() => currentWorkspace.value === 'live')
const standardWorkspaceActive = computed(() => !settingsWorkspaceActive.value && !liveWorkspaceActive.value)
const currentModeLabel = computed(() => shellModeLabel(currentMode.value))
const currentWorkspaceLabel = computed(() => workspaceItems.find((item) => item.key === currentWorkspace.value)?.label || 'Research')
const currentWorkflowStepLabel = computed(
  () => workflowStepItems.value.find((item) => item.key === currentWorkflowStep.value)?.label || `1 ${workflowStepLabelForMode('data', currentMode.value)}`,
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
  if (currentWorkspace.value === 'workflow') {
    parts.push(`Mode ${currentModeLabel.value}.`)
    parts.push(`Workflow step ${currentWorkflowStepLabel.value}.`)
  }
  if (visibleControlsItems.value.length > 0) parts.push(`Setup panel ${currentControlsLabel.value}.`)
  if (visibleResultsItems.value.length > 0) parts.push(`Results panel ${currentResultsLabel.value}.`)
  return parts.join(' ')
})
const shellSubtitle = computed(() => {
  const text = String(workflowSubtitle.value || '').trim()
  return text || '—'
})
const shellPostureVisual = computed(() => shellPostureIcon(currentWorkspace.value, currentWorkflowStep.value))
const shellPostureText = computed(() => shellPostureLabel(currentWorkspace.value, currentWorkflowStep.value))
const layoutKey = computed(() => `${currentMode.value}:${currentWorkspace.value}:${currentWorkflowStep.value}`)

const visibleControlsItems = computed(() => {
  if (currentWorkspace.value === 'workflow') {
    if (currentMode.value === 'ml-signal') {
      if (currentWorkflowStep.value === 'data') return [controlsItems.data]
      return [controlsItems.opt]
    }
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
    if (currentMode.value === 'ml-signal') return []
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

const controlsColumnEligible = computed(() => visibleControlsItems.value.length > 0 && standardWorkspaceActive.value)
const controlsColumnVisible = computed(() => controlsColumnEligible.value && !controlsCollapsed.value)
const resultsColumnVisible = computed(() => visibleResultsItems.value.length > 0 && standardWorkspaceActive.value)
const resultsTabsVisible = computed(() => resultsColumnVisible.value && visibleResultsItems.value.length > 1)
const controlsTabsVisible = computed(() => controlsColumnVisible.value && visibleControlsItems.value.length > 1)
const shellUsesSplitLayout = computed(() => {
  if (!controlsColumnVisible.value || !resultsColumnVisible.value) return false
  if (currentWorkspace.value === 'pro') return true
  return currentMode.value === 'conventional'
    && (layoutKey.value === 'conventional:workflow:data' || layoutKey.value === 'conventional:workflow:optimize')
})
const splitLayoutActive = computed(() => shellUsesSplitLayout.value && viewportWidth.value > 1200)
const stackedLayoutActive = computed(() => shellUsesSplitLayout.value && controlsColumnVisible.value && !splitLayoutActive.value)
const workspaceGridClass = computed(() => ({
  singleColumn: !controlsColumnVisible.value || !resultsColumnVisible.value || !shellUsesSplitLayout.value,
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

const allContextItems = computed<ContextItem[]>(() => {
  const dataset = String(datasetLabel.value || '').trim()
  const range = String(datasetRange.value || '').trim()
  const rows = String(dataLen.value || '').trim()
  const strategy = String(strategyKind.value || '').trim()
  const objectiveLabel = String(objective.value || '').trim()
  const backendLabel = String(backend.value || '').trim()
  const timeframe = String(dataTimeframe.value || '').trim()
  const status = String(progressMeta.value || '').trim() || String(runReadinessHint.value || '').trim()
  const loaded = String(dataId.value || '').trim()
  const candidate = String(selectedTopTitle.value || '').trim()
  const modeContextItems = currentWorkspace.value !== 'workflow'
    ? [
        {
          key: 'Workspace',
          value: currentWorkspaceLabel.value,
          current: true,
          defaultVisible: true,
        },
        {
          key: 'Mode',
          value: currentModeLabel.value,
          current: currentMode.value === 'ml-signal',
          defaultVisible: true,
        },
      ]
    : currentMode.value === 'ml-signal'
      ? [
          {
            key: 'Workflow',
            value: 'ML Signal',
            current: true,
            defaultVisible: true,
          },
          {
            key: 'Stage',
            value: workflowStepLabelForMode(currentWorkflowStep.value, currentMode.value),
            current: true,
            defaultVisible: true,
          },
        ]
      : [
          {
            key: 'Strategy',
            value: [strategy, objectiveLabel].filter((part) => part.length > 0 && part !== '—').join(' · ') || '—',
            current: strategy.length > 0 || objectiveLabel.length > 0,
            defaultVisible: true,
          },
          {
            key: 'Backend',
            value: backendLabel || '—',
            current: false,
            defaultVisible: true,
          },
      ]
  const liveSupportItems = currentWorkspace.value === 'live'
    ? [
        {
          key: 'Support',
          value: 'Single symbol · Alpaca · Polygon',
          current: false,
          defaultVisible: true,
        },
      ]
    : []
  return [
    {
      key: 'Dataset',
      value: dataset && dataset !== '—'
        ? `${dataset}${timeframe && timeframe !== '—' ? ` · ${timeframe}` : ''}`
        : 'No dataset',
      current: !dataset || dataset === '—',
      defaultVisible: true,
    },
    { key: 'Range', value: range && range !== '—' ? range : 'No range', current: false, defaultVisible: true },
    { key: 'Rows', value: rows && rows !== '—' ? rows : '—', current: false, defaultVisible: false },
    ...modeContextItems,
    ...liveSupportItems,
    {
      key: 'Candidate',
      value: candidate && candidate !== '—' && candidate !== 'No ranked candidate selected.' ? candidate : 'No candidate',
      current: candidate.length > 0 && candidate !== '—' && candidate !== 'No ranked candidate selected.',
      defaultVisible: currentWorkspace.value === 'workflow' && currentMode.value === 'conventional' && currentWorkflowStep.value !== 'data',
    },
    { key: 'Status', value: status || 'Ready.', current: statusCardCurrent.value, defaultVisible: true },
    { key: 'Loaded ID', value: loaded && loaded !== '—' ? loaded : '—', current: false, defaultVisible: false },
  ]
})
const contextItems = computed(() => allContextItems.value.filter((item) => item.defaultVisible))
const secondaryContextItems = computed(() => allContextItems.value.filter((item) => !item.defaultVisible))
const secondaryContextSummary = computed(() => secondaryContextItems.value.map((item) => item.key).join(' · '))
const taskCenterLauncherStatus = computed(() => {
  if (activeTaskCount.value > 0) return `${activeTaskCount.value} active`
  return centerStatusLabel.value
})
const taskCenterLauncherLabel = computed(() => `Open task center. ${taskCenterLauncherStatus.value}.`)
const statusCardCurrent = computed(() => {
  const progress = String(progressMeta.value || '').trim()
  return progress.length > 0 || activeTaskCount.value > 0 || taskCenterOpen.value
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

function selectMode(key: string): void {
  if (currentMode.value === key) return
  workflowStore.selectMode(key)
}

function openTaskCenter(): void {
  requestTaskCenterOpen(taskCenterOpen)
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
      ? (event.target.closest('[data-mode], [data-workspace], [data-workflow-step], [data-tab]') as HTMLElement | null)
      : null
  if (!target) return null
  const focusedKey =
    String(target.dataset.mode || '').trim() ||
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

function onModeKeydown(event: KeyboardEvent): void {
  moveTabFocus(event, modeItems, currentMode.value, selectMode, (key) => `#modeBar [data-mode="${key}"]`)
}

function onWorkflowStepKeydown(event: KeyboardEvent): void {
  moveTabFocus(
    event,
    workflowStepItems.value,
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
  workflowStore.setSplitFraction(clampSplitFraction(splitFraction.value), { persist: false })
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
  workflowStore.setSplitFraction(clampSplitFraction(drag.startFraction + deltaX / currentGridWidth()))
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
    workflowStore.setSplitFraction(clampSplitFraction(0))
    return
  }
  if (event.key === 'End') {
    workflowStore.setSplitFraction(clampSplitFraction(1))
    return
  }
  const delta = event.key === 'ArrowLeft' ? -0.02 : 0.02
  workflowStore.setSplitFraction(clampSplitFraction(splitFraction.value + delta))
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
    <div class="shellHeader" role="region" aria-label="Workspace shell header">
      <div class="brandRow">
        <div class="brandBlock">
          <h1>VectorGrid</h1>
        </div>
        <div class="brandMeta">
          <div class="subtitle mono">{{ shellSubtitle }}</div>
          <div class="shellPostureChip" :aria-label="shellPostureText">
            <AppVisualIcon :icon="shellPostureVisual" class="shellPostureIcon" />
            <span>{{ shellPostureText }}</span>
          </div>
        </div>
      </div>

      <div class="workspaceBarRow">
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
          id="modeBar"
          class="tabsRow modeBar"
          aria-label="Workflow mode"
          role="tablist"
          aria-orientation="horizontal"
          @keydown="onModeKeydown"
        >
          <button
            v-for="item in modeItems"
            :key="`mode_${item.key}`"
            :id="modeTabId(item.key)"
            type="button"
            class="tabBtn modeBtn"
            :data-mode="item.key"
            :class="{ active: currentMode === item.key }"
            role="tab"
            :aria-selected="currentMode === item.key"
            :aria-controls="shellMainPanelId"
            :tabindex="currentMode === item.key ? 0 : -1"
            @click="selectMode(item.key)"
          >
            {{ item.label }}
          </button>
        </nav>

        <button
          type="button"
          class="taskCenterLauncher"
          :class="{ active: activeTaskCount > 0, open: taskCenterOpen }"
          :aria-controls="TASK_CENTER_PANEL_ID"
          :aria-expanded="taskCenterOpen ? 'true' : 'false'"
          aria-haspopup="dialog"
          :aria-label="taskCenterLauncherLabel"
          title="Open task center"
          @click="openTaskCenter"
        >
          <ListTodo :size="16" aria-hidden="true" />
          <span class="taskCenterLauncherBody">
            <span class="taskCenterLauncherText">Task Center</span>
            <span class="taskCenterLauncherValue mono">{{ taskCenterLauncherStatus }}</span>
          </span>
          <span v-if="activeTaskCount > 0" class="taskCenterLauncherBadge mono">{{ activeTaskCount }}</span>
        </button>
      </div>

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

      <RunContextBar
        :items="contextItems"
        :secondary-items="secondaryContextItems"
        :secondary-summary="secondaryContextSummary"
      />
    </div>

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
          v-show="standardWorkspaceActive"
        >
        <section
          id="shellControlsColumn"
          v-show="controlsColumnVisible"
          class="controlsColumn"
          aria-label="Setup panels"
        >
          <nav
            v-if="controlsTabsVisible"
            id="controlsTabBar"
            class="tabsRow compact shellStickyTabs surfaceSticky"
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
        </section>

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

        <section
          id="shellResultsColumn"
          v-show="resultsColumnVisible"
          class="resultsColumn"
        >
          <nav
            v-if="resultsTabsVisible"
            id="resultsTabBar"
            class="tabsRow compact shellStickyTabs surfaceSticky"
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

        <section v-show="liveWorkspaceActive" id="liveShellGrid" aria-label="Live workspace shell">
          <section class="liveSupportPane surfaceSunken" aria-label="Current live support envelope">
            <div class="liveSupportCopy">
              <div class="contextKey">Current live support envelope</div>
              <div class="liveSupportLead">
                Live mode stays narrow and depends on the currently loaded dataset and strategy context.
              </div>
            </div>
            <div class="liveSupportGrid">
              <article class="liveSupportCard">
                <div class="contextKey">Strategies</div>
                <div class="contextValue mono">DoubleMa · CompositeCpu</div>
              </article>
              <article class="liveSupportCard">
                <div class="contextKey">Broker</div>
                <div class="contextValue mono">Alpaca REST</div>
              </article>
              <article class="liveSupportCard">
                <div class="contextKey">Feed</div>
                <div class="contextValue mono">Polygon stocks</div>
              </article>
              <article class="liveSupportCard">
                <div class="contextKey">Limits</div>
                <div class="contextValue mono">One symbol · one runner · one stream</div>
              </article>
            </div>
          </section>
          <div id="executionShellPane">
            <div id="executionVueShellMount"></div>
          </div>
        </section>

        <section v-show="settingsWorkspaceActive" id="settingsShellGrid">
          <div id="settingsVueShellMount"></div>
        </section>
      </section>
    </main>

    <footer id="runFooterShell" role="contentinfo">
      <div id="runFooterVueShellMount"></div>
    </footer>
  </section>
</template>

<style scoped>
.shellRoot {
  position: fixed;
  inset: var(--desktop-titlebar-h, 0px) 0 0 0;
  z-index: 1200;
  background: var(--app-bg);
  color: var(--text);
  display: grid;
  grid-template-rows: auto 1fr auto;
  pointer-events: auto;
}

.shellHeader {
  border-bottom: 1px solid var(--border);
  background: color-mix(in oklab, var(--header-bg) 92%, var(--surface-shell));
  padding: 0.85rem 1rem 0.8rem;
  display: grid;
  gap: 0.5rem;
  box-shadow: 0 8px 18px color-mix(in srgb, rgb(var(--shadow-overflow-rgb) / 0.14) 72%, transparent);
}

.brandRow {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.35rem 0.95rem;
}

.brandBlock {
  display: grid;
  gap: 0.12rem;
}

.brandMeta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.35rem 0.65rem;
  min-width: 0;
}

.brandRow h1 {
  margin: 0;
  font-size: 1.05rem;
  letter-spacing: 0.01em;
}

.workspaceBarRow {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.6rem;
}

.workspaceBar {
  flex: 1 1 42rem;
  min-width: 0;
  padding: 0.24rem 0.28rem 0.18rem;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-sunken) 94%, var(--panel));
}

.modeBar {
  flex: 0 0 auto;
  padding: 0.24rem 0.28rem 0.18rem;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-sunken) 94%, var(--panel));
}

.workflowStepBar {
  padding: 0.22rem 0.28rem 0.16rem;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-sunken) 92%, var(--panel));
}

.subtitle {
  align-self: baseline;
  font-size: 0.68rem;
  line-height: 1.3;
  color: color-mix(in oklab, var(--muted) 88%, var(--text));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: min(48rem, 100%);
  padding: 0.08rem 0;
}

.shellPostureChip {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  border: 1px solid color-mix(in oklab, var(--panel-focus-edge) 42%, var(--border));
  border-radius: 999px;
  background: color-mix(in oklab, var(--surface-working) 88%, var(--panel));
  padding: 0.2rem 0.55rem;
  font-size: 0.7rem;
  line-height: 1.2;
  color: var(--muted-strong);
}

.shellPostureIcon {
  width: 0.92rem;
  height: 0.92rem;
  flex: 0 0 auto;
}

.taskCenterLauncher {
  margin-left: auto;
  min-height: 38px;
  display: inline-flex;
  align-items: center;
  gap: 0.58rem;
  padding: 0.38rem 0.66rem;
  border-radius: 10px;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  background: color-mix(in oklab, var(--surface-shell) 88%, var(--panel));
  transition:
    background-color var(--motion-fast) var(--ease-standard),
    border-color var(--motion-fast) var(--ease-standard),
    box-shadow var(--motion-fast) var(--ease-standard),
    transform var(--motion-fast) var(--ease-standard);
}

.taskCenterLauncher:hover {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 42%, var(--border));
  background: color-mix(in oklab, var(--surface-working) 88%, var(--surface-shell));
  transform: translateY(-1px);
  box-shadow: var(--shadow-inline);
}

.taskCenterLauncher.active,
.taskCenterLauncher.open {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 70%, var(--accent));
  background: color-mix(in oklab, var(--selection-fill) 82%, var(--surface-working));
  box-shadow: inset 0 -2px 0 var(--selection-rail), 0 0 0 1px color-mix(in oklab, var(--accent) 14%, transparent);
}

.taskCenterLauncherBody {
  min-width: 0;
  display: grid;
  gap: 0.04rem;
  text-align: left;
}

.taskCenterLauncherText {
  font-size: 0.64rem;
  line-height: 1;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}

.taskCenterLauncherValue {
  font-size: 0.74rem;
  color: var(--muted-strong);
  white-space: nowrap;
}

.taskCenterLauncherBadge {
  min-width: 1.55rem;
  padding: 0.16rem 0.38rem;
  border-radius: 999px;
  background: color-mix(in oklab, var(--surface) 92%, transparent);
  font-size: 0.69rem;
  text-align: center;
}

.tabsRow {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.tabBtn {
  border: 1px solid transparent;
  border-radius: 10px;
  background: transparent;
  color: inherit;
  font-size: 0.76rem;
  padding: 0.38rem 0.7rem;
  font-weight: 600;
  transition:
    background-color var(--motion-fast) var(--ease-standard),
    border-color var(--motion-fast) var(--ease-standard),
    color var(--motion-fast) var(--ease-standard),
    box-shadow var(--motion-fast) var(--ease-standard),
    transform var(--motion-fast) var(--ease-standard);
}

.modeBtn {
  min-width: 7.25rem;
}

.tabBtn.active {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 76%, var(--accent));
  background: color-mix(in oklab, var(--selection-fill) 84%, var(--surface-working));
  box-shadow: inset 0 -2px 0 var(--selection-rail);
}

.tabBtn:hover:not(:disabled) {
  background: color-mix(in oklab, var(--surface-working) 88%, var(--panel));
  border-color: color-mix(in oklab, var(--border) 82%, transparent);
  transform: translateY(-1px);
}

.contextStrip {
  position: relative;
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 0.32rem;
  padding: 0.24rem;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-sunken) 94%, var(--surface-shell));
}

.contextCard {
  min-width: 0;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 10px;
  background: color-mix(in oklab, var(--surface-working) 96%, var(--panel));
  padding: 0.34rem 0.48rem;
  display: grid;
  gap: 0.08rem;
  transition:
    background-color var(--motion-fast) var(--ease-standard),
    border-color var(--motion-fast) var(--ease-standard),
    box-shadow var(--motion-fast) var(--ease-standard),
    transform var(--motion-fast) var(--ease-standard);
}

.contextCard.current {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 74%, var(--border));
  background: linear-gradient(
    145deg,
    color-mix(in oklab, var(--surface-focus) 90%, var(--panel-footer)),
    color-mix(in oklab, var(--panel-footer) 92%, var(--surface-working))
  );
  box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--panel-focus-edge) 16%, transparent);
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

.contextOverflow {
  position: relative;
}

.contextOverflowSummary {
  min-height: 100%;
  list-style: none;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 10px;
  background: color-mix(in oklab, var(--surface-working) 94%, var(--panel));
  padding: 0.34rem 0.48rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  cursor: pointer;
}

.contextOverflowSummary::-webkit-details-marker {
  display: none;
}

.contextOverflowBody {
  min-width: 0;
  display: grid;
  gap: 0.08rem;
}

.contextOverflowIcon {
  flex: 0 0 auto;
  color: var(--muted);
  transition: transform var(--motion-fast) var(--ease-standard);
}

.contextOverflow[open] .contextOverflowSummary {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 70%, var(--border));
  background: color-mix(in oklab, var(--surface-focus) 88%, var(--panel));
}

.contextOverflow[open] .contextOverflowIcon {
  transform: rotate(180deg);
}

.contextOverflowPanel {
  position: absolute;
  top: calc(100% + 0.35rem);
  right: 0;
  z-index: 6;
  min-width: min(22rem, calc(100vw - 2rem));
  display: grid;
  gap: 0.32rem;
  padding: 0.35rem;
  border: 1px solid color-mix(in oklab, var(--border) 90%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-overlay) 94%, var(--panel));
  box-shadow: var(--shadow-md);
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
  border-radius: 16px;
  background: var(--panel);
  display: grid;
  align-content: start;
  gap: 0.7rem;
  box-shadow: var(--shadow-sm);
}

.controlsColumn {
  padding: 0.95rem;
  background:
    linear-gradient(
      180deg,
      color-mix(in oklab, var(--surface-shell) 86%, var(--panel)),
      color-mix(in oklab, var(--panel) 98%, var(--surface-working))
    );
}

.resultsColumn {
  padding: 0.8rem;
  background:
    linear-gradient(
      180deg,
      color-mix(in oklab, var(--surface-working) 96%, var(--panel)),
      color-mix(in oklab, var(--panel) 98%, var(--surface-inline))
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

.shellStickyTabs {
  position: sticky;
  top: 0;
  z-index: 6;
  margin: -0.15rem -0.15rem 0.1rem;
  padding: 0.25rem 0.15rem 0.55rem;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 12px 12px 10px 10px;
  box-shadow: 0 10px 14px -18px rgb(var(--shadow-overflow-rgb) / 0.8);
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
  border-radius: 10px;
  background:
    radial-gradient(circle at center, color-mix(in oklab, var(--accent) 42%, var(--border)) 0 1px, transparent 1.5px) center / 8px 12px repeat-y,
    color-mix(in oklab, var(--surface2) 86%, transparent);
  border: 1px solid color-mix(in oklab, var(--border) 80%, transparent);
  box-shadow: 0 6px 14px color-mix(in srgb, var(--accent-soft) 14%, transparent);
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

#settingsShellGrid,
#liveShellGrid {
  flex: 1 1 auto;
  height: 100%;
  width: 100%;
  min-width: 0;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 16px;
  background: color-mix(in oklab, var(--panel) 98%, var(--surface));
  margin: 1rem;
  padding: 1rem;
  display: grid;
  gap: 0.9rem;
  box-shadow: var(--shadow-sm);
}

#liveShellGrid {
  align-content: start;
}

.liveSupportPane {
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 14px;
  padding: 0.8rem 0.9rem;
  display: grid;
  gap: 0.7rem;
  background: color-mix(in oklab, var(--surface-working) 94%, var(--surface-shell));
}

.liveSupportCopy {
  display: grid;
  gap: 0.18rem;
}

.liveSupportLead {
  font-size: 0.8rem;
  line-height: 1.45;
  color: var(--muted-strong);
}

.liveSupportGrid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.5rem;
}

.liveSupportCard {
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface) 95%, var(--panel));
  padding: 0.6rem 0.65rem;
  display: grid;
  gap: 0.14rem;
}

#executionShellPane {
  border-top: 1px dashed var(--border);
  padding-top: 0.9rem;
}

#runFooterShell {
  border-top: 1px solid var(--border);
  background: color-mix(in srgb, var(--panel-footer) 98%, var(--surface));
  padding: 0.55rem 0.9rem;
  box-shadow: 0 -8px 20px color-mix(in srgb, var(--accent-soft) 10%, transparent);
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
  .workspaceBarRow {
    align-items: stretch;
  }

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

  .taskCenterLauncher {
    margin-left: 0;
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

  .taskCenterLauncher {
    width: 100%;
    justify-content: space-between;
  }

  .contextStrip {
    grid-template-columns: 1fr;
  }

  .contextOverflowPanel {
    left: 0;
    right: auto;
    min-width: min(100%, 22rem);
  }

  .workspaceGrid {
    padding: 0.75rem;
    gap: 0.75rem;
  }

  #settingsShellGrid,
  #liveShellGrid {
    margin: 0.75rem;
    padding: 0.75rem;
  }

  .liveSupportGrid {
    grid-template-columns: 1fr;
  }

  #runFooterShell {
    padding: 0.7rem 0.75rem;
  }
}
</style>
