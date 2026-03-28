<script setup lang="ts">
import { computed, defineAsyncComponent, reactive, watch } from 'vue'
import { storeToRefs } from 'pinia'
import AppRuntimeBanner from './components/AppRuntimeBanner.vue'
import CommandPaletteOverlay from './components/CommandPaletteOverlay.vue'
import GlobalShortcutBridge from './components/GlobalShortcutBridge.vue'
import RunFooterPanel from './components/RunFooterPanel.vue'
import TaskCenterOverlay from './components/TaskCenterOverlay.vue'
import VueWorkspaceShell from './components/VueWorkspaceShell.vue'
import WorkflowQuickSwitch from './components/WorkflowQuickSwitch.vue'
import { useWorkflowStore } from './stores/workflowStore'

const AdvancedWorkspacePanel = defineAsyncComponent(() => import('./components/AdvancedWorkspacePanel.vue'))
const AnalysisContextDock = defineAsyncComponent(() => import('./components/AnalysisContextDock.vue'))
const DataWorkspacePanel = defineAsyncComponent(() => import('./components/DataWorkspacePanel.vue'))
const DataContextDock = defineAsyncComponent(() => import('./components/DataContextDock.vue'))
const DebugWorkspacePanel = defineAsyncComponent(() => import('./components/DebugWorkspacePanel.vue'))
const DrillWorkspacePanel = defineAsyncComponent(() => import('./components/DrillWorkspacePanel.vue'))
const ExecutionContextDock = defineAsyncComponent(() => import('./components/ExecutionContextDock.vue'))
const ExecutionWorkspacePanel = defineAsyncComponent(() => import('./components/ExecutionWorkspacePanel.vue'))
const HeatmapWorkspacePanel = defineAsyncComponent(() => import('./components/HeatmapWorkspacePanel.vue'))
const HistoryWorkspacePanel = defineAsyncComponent(() => import('./components/HistoryWorkspacePanel.vue'))
const HistoryContextDock = defineAsyncComponent(() => import('./components/HistoryContextDock.vue'))
const MetaWorkspacePanel = defineAsyncComponent(() => import('./components/MetaWorkspacePanel.vue'))
const OptimizeContextDock = defineAsyncComponent(() => import('./components/OptimizeContextDock.vue'))
const OptimizeWorkspacePanel = defineAsyncComponent(() => import('./components/OptimizeWorkspacePanel.vue'))
const OverviewWorkspacePanel = defineAsyncComponent(() => import('./components/OverviewWorkspacePanel.vue'))
const ParetoWorkspacePanel = defineAsyncComponent(() => import('./components/ParetoWorkspacePanel.vue'))
const SettingsContextDock = defineAsyncComponent(() => import('./components/SettingsContextDock.vue'))
const SettingsWorkspacePanel = defineAsyncComponent(() => import('./components/SettingsWorkspacePanel.vue'))
const StrategyWorkspacePanel = defineAsyncComponent(() => import('./components/StrategyWorkspacePanel.vue'))
const TopWorkspacePanel = defineAsyncComponent(() => import('./components/TopWorkspacePanel.vue'))
const ValidationContextDock = defineAsyncComponent(() => import('./components/ValidationContextDock.vue'))
const ValidationWorkspacePanel = defineAsyncComponent(() => import('./components/ValidationWorkspacePanel.vue'))

const workflowStore = useWorkflowStore()
workflowStore.syncRuntimeState()

const { controlsTab, resultsTab, workflow, workspace } = storeToRefs(workflowStore)

const settingsWorkspaceActive = computed(() => String(workspace.value || '').trim() === 'settings')
const workflowWorkspaceActive = computed(() => String(workspace.value || '').trim() === 'workflow')
const activeControlsTab = computed(() => String(controlsTab.value || '').trim() || 'data')
const activeResultsTab = computed(() => String(resultsTab.value || '').trim() || 'overview')
const analysisDockActive = computed(() => {
  const ws = String(workspace.value || '').trim()
  const wf = String(workflow.value || '').trim()
  return ws === 'aggregate' || wf === 'optimize' || wf === 'validate' || wf === 'analyze'
})
const dataDockActive = computed(() => workflowWorkspaceActive.value && activeControlsTab.value === 'data')
const optimizeDockActive = computed(() => workflowWorkspaceActive.value && activeControlsTab.value === 'opt')
const validationDockActive = computed(() => workflowWorkspaceActive.value && activeResultsTab.value === 'validate')
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
const overviewPanelActive = computed(() => activeResultsTab.value === 'overview')
const topPanelActive = computed(() => activeResultsTab.value === 'top')
const paretoPanelActive = computed(() => activeResultsTab.value === 'pareto')
const validationPanelActive = computed(() => activeResultsTab.value === 'validate')
const drillPanelActive = computed(() => activeResultsTab.value === 'drill')
const heatmapPanelActive = computed(() => activeResultsTab.value === 'heatmap')
const metaPanelActive = computed(() => activeResultsTab.value === 'meta')
const debugPanelActive = computed(() => activeResultsTab.value === 'debug')

type LazyPanelKey = 'data' | 'optimize' | 'overview'

const mountedPanels = reactive<Record<LazyPanelKey, boolean>>({
  data: false,
  optimize: false,
  overview: false,
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

mountWhenActive('data', () => dataPanelActive.value)
mountWhenActive('optimize', () => optimizePanelActive.value)
mountWhenActive('overview', () => overviewPanelActive.value)
</script>

<template>
  <VueWorkspaceShell />
  <CommandPaletteOverlay />
  <GlobalShortcutBridge />
  <AppRuntimeBanner />
  <TaskCenterOverlay />
  <AnalysisContextDock v-if="analysisDockActive" />
  <AdvancedWorkspacePanel v-if="advancedPanelActive" />
  <DataContextDock v-if="dataDockActive" />
  <DataWorkspacePanel v-if="mountedPanels.data" />
  <DebugWorkspacePanel v-if="debugPanelActive" />
  <DrillWorkspacePanel v-if="drillPanelActive" />
  <ExecutionContextDock v-if="settingsWorkspaceActive" />
  <ExecutionWorkspacePanel v-if="settingsWorkspaceActive" />
  <HeatmapWorkspacePanel v-if="heatmapPanelActive" />
  <HistoryContextDock v-if="historyPanelActive" />
  <HistoryWorkspacePanel v-if="historyPanelActive" />
  <MetaWorkspacePanel v-if="metaPanelActive" />
  <OverviewWorkspacePanel v-if="mountedPanels.overview" />
  <OptimizeContextDock v-if="optimizeDockActive" />
  <ParetoWorkspacePanel v-if="paretoPanelActive" />
  <OptimizeWorkspacePanel v-if="mountedPanels.optimize" />
  <RunFooterPanel />
  <SettingsContextDock v-if="settingsWorkspaceActive" />
  <SettingsWorkspacePanel v-if="settingsWorkspaceActive" />
  <StrategyWorkspacePanel v-if="strategyPanelActive" />
  <TopWorkspacePanel v-if="topPanelActive" />
  <ValidationContextDock v-if="validationDockActive" />
  <ValidationWorkspacePanel v-if="validationPanelActive" />
  <WorkflowQuickSwitch v-if="showWorkflowQuickSwitch" />
</template>
