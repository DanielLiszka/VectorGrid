<script setup lang="ts">
import { computed, defineAsyncComponent, reactive, watch } from 'vue'
import { storeToRefs } from 'pinia'
import AnalysisContextDock from './components/AnalysisContextDock.vue'
import AppRuntimeBanner from './components/AppRuntimeBanner.vue'
import CommandPaletteOverlay from './components/CommandPaletteOverlay.vue'
import GlobalShortcutBridge from './components/GlobalShortcutBridge.vue'
import RunFooterPanel from './components/RunFooterPanel.vue'
import TaskCenterOverlay from './components/TaskCenterOverlay.vue'
import VueWorkspaceShell from './components/VueWorkspaceShell.vue'
import WorkflowQuickSwitch from './components/WorkflowQuickSwitch.vue'
import { useDistributedStore } from './stores/distributedStore'
import { useWorkflowStore } from './stores/workflowStore'

const AdvancedWorkspacePanel = defineAsyncComponent(() => import('./components/AdvancedWorkspacePanel.vue'))
const DataWorkspacePanel = defineAsyncComponent(() => import('./components/DataWorkspacePanel.vue'))
const DebugWorkspacePanel = defineAsyncComponent(() => import('./components/DebugWorkspacePanel.vue'))
const DistributedWorkspacePanel = defineAsyncComponent(() => import('./components/DistributedWorkspacePanel.vue'))
const DrillWorkspacePanel = defineAsyncComponent(() => import('./components/DrillWorkspacePanel.vue'))
const ExecutionWorkspacePanel = defineAsyncComponent(() => import('./components/ExecutionWorkspacePanel.vue'))
const HeatmapWorkspacePanel = defineAsyncComponent(() => import('./components/HeatmapWorkspacePanel.vue'))
const HistoryWorkspacePanel = defineAsyncComponent(() => import('./components/HistoryWorkspacePanel.vue'))
const MetaWorkspacePanel = defineAsyncComponent(() => import('./components/MetaWorkspacePanel.vue'))
const OptimizeWorkspacePanel = defineAsyncComponent(() => import('./components/OptimizeWorkspacePanel.vue'))
const OverviewWorkspacePanel = defineAsyncComponent(() => import('./components/OverviewWorkspacePanel.vue'))
const ParetoWorkspacePanel = defineAsyncComponent(() => import('./components/ParetoWorkspacePanel.vue'))
const SettingsWorkspacePanel = defineAsyncComponent(() => import('./components/SettingsWorkspacePanel.vue'))
const StrategyWorkspacePanel = defineAsyncComponent(() => import('./components/StrategyWorkspacePanel.vue'))
const TopWorkspacePanel = defineAsyncComponent(() => import('./components/TopWorkspacePanel.vue'))
const ValidationWorkspacePanel = defineAsyncComponent(() => import('./components/ValidationWorkspacePanel.vue'))

const distributedStore = useDistributedStore()
const workflowStore = useWorkflowStore()
workflowStore.syncRuntimeState()

const { isRunning: distributedIsRunning } = storeToRefs(distributedStore)
const { controlsTab, resultsTab, workflow, workspace } = storeToRefs(workflowStore)

const settingsWorkspaceActive = computed(() => String(workspace.value || '').trim() === 'settings')
const activeControlsTab = computed(() => String(controlsTab.value || '').trim() || 'data')
const activeResultsTab = computed(() => String(resultsTab.value || '').trim() || 'overview')
const historyPanelActive = computed(() => {
  const ws = String(workspace.value || '').trim()
  const wf = String(workflow.value || '').trim()
  return ws === 'library' || wf === 'history'
})
function shouldShowWorkflowQuickSwitch(): boolean {
  if (!import.meta.env.DEV) return false
  try {
    const params = new URLSearchParams(window.location.search)
    if (params.get('workflow_nav') === '1') return true
    return localStorage.getItem('VTA_SHOW_WORKFLOW_NAV') === '1'
  } catch {
    return false
  }
}

const showWorkflowQuickSwitch = shouldShowWorkflowQuickSwitch()
const dataPanelActive = computed(() => activeControlsTab.value === 'data')
const optimizePanelActive = computed(() => activeControlsTab.value === 'opt')
const strategyPanelActive = computed(() => activeControlsTab.value === 'strategy')
const advancedPanelActive = computed(() => activeControlsTab.value === 'advanced')
const distributedPanelActive = computed(() => {
  const ws = String(workspace.value || '').trim()
  const wf = String(workflow.value || '').trim()
  return ws === 'workflow' && wf === 'optimize' && (activeControlsTab.value === 'distributed' || distributedIsRunning.value)
})
const overviewPanelActive = computed(() => activeResultsTab.value === 'overview')
const topPanelActive = computed(() => activeResultsTab.value === 'top')
const paretoPanelActive = computed(() => activeResultsTab.value === 'pareto')
const validationPanelActive = computed(() => activeResultsTab.value === 'validate')
const drillPanelActive = computed(() => activeResultsTab.value === 'drill')
const heatmapPanelActive = computed(() => activeResultsTab.value === 'heatmap')
const metaPanelActive = computed(() => activeResultsTab.value === 'meta')
const debugPanelActive = computed(() => activeResultsTab.value === 'debug')

type LazyPanelKey =
  | 'advanced'
  | 'data'
  | 'debug'
  | 'distributed'
  | 'drill'
  | 'execution'
  | 'heatmap'
  | 'history'
  | 'meta'
  | 'optimize'
  | 'overview'
  | 'pareto'
  | 'settings'
  | 'strategy'
  | 'top'
  | 'validation'

const mountedPanels = reactive<Record<LazyPanelKey, boolean>>({
  advanced: false,
  data: false,
  debug: false,
  distributed: false,
  drill: false,
  execution: false,
  heatmap: false,
  history: false,
  meta: false,
  optimize: false,
  overview: false,
  pareto: false,
  settings: false,
  strategy: false,
  top: false,
  validation: false,
})

function mountWhenActive(panel: LazyPanelKey, source: () => boolean): void {
  watch(
    source,
    (isActive) => {
      if (isActive) mountedPanels[panel] = true
    },
    { immediate: true },
  )
}

mountWhenActive('advanced', () => advancedPanelActive.value)
mountWhenActive('data', () => dataPanelActive.value)
mountWhenActive('debug', () => debugPanelActive.value)
mountWhenActive('distributed', () => distributedPanelActive.value)
mountWhenActive('drill', () => drillPanelActive.value)
mountWhenActive('execution', () => settingsWorkspaceActive.value)
mountWhenActive('heatmap', () => heatmapPanelActive.value)
mountWhenActive('history', () => historyPanelActive.value)
mountWhenActive('meta', () => metaPanelActive.value)
mountWhenActive('optimize', () => optimizePanelActive.value)
mountWhenActive('overview', () => overviewPanelActive.value)
mountWhenActive('pareto', () => paretoPanelActive.value)
mountWhenActive('settings', () => settingsWorkspaceActive.value)
mountWhenActive('strategy', () => strategyPanelActive.value)
mountWhenActive('top', () => topPanelActive.value)
mountWhenActive('validation', () => validationPanelActive.value)
</script>

<template>
  <VueWorkspaceShell />
  <CommandPaletteOverlay />
  <GlobalShortcutBridge />
  <TaskCenterOverlay />
  <AppRuntimeBanner />
  <AnalysisContextDock />
  <AdvancedWorkspacePanel v-if="mountedPanels.advanced" />
  <DataWorkspacePanel v-if="mountedPanels.data" />
  <DebugWorkspacePanel v-if="mountedPanels.debug" />
  <DistributedWorkspacePanel v-if="mountedPanels.distributed" />
  <DrillWorkspacePanel v-if="mountedPanels.drill" />
  <ExecutionWorkspacePanel v-if="mountedPanels.execution" />
  <HeatmapWorkspacePanel v-if="mountedPanels.heatmap" />
  <HistoryWorkspacePanel v-if="mountedPanels.history" />
  <MetaWorkspacePanel v-if="mountedPanels.meta" />
  <OverviewWorkspacePanel v-if="mountedPanels.overview" />
  <ParetoWorkspacePanel v-if="mountedPanels.pareto" />
  <OptimizeWorkspacePanel v-if="mountedPanels.optimize" />
  <RunFooterPanel />
  <SettingsWorkspacePanel v-if="mountedPanels.settings" />
  <StrategyWorkspacePanel v-if="mountedPanels.strategy" />
  <TopWorkspacePanel v-if="mountedPanels.top" />
  <ValidationWorkspacePanel v-if="mountedPanels.validation" />
  <WorkflowQuickSwitch v-if="showWorkflowQuickSwitch" />
</template>
