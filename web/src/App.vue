<script setup lang="ts">
import { computed, defineAsyncComponent, reactive, watch } from 'vue'
import { storeToRefs } from 'pinia'
import AppRuntimeBanner from './components/AppRuntimeBanner.vue'
import DesktopTitlebar from './components/DesktopTitlebar.vue'
import GlobalShortcutBridge from './components/GlobalShortcutBridge.vue'
import RunFooterPanel from './components/RunFooterPanel.vue'
import StartupAnimationOverlay from './components/StartupAnimationOverlay.vue'
import VueWorkspaceShell from './components/VueWorkspaceShell.vue'
import { useWorkflowStore } from './stores/workflowStore'
import {
  isAnalysisDockVisible,
  isHistoryDockVisible,
  isOptimizeDockVisible,
  isValidationDockVisible,
} from './utils/shellDockContract'

const AdvancedWorkspacePanel = defineAsyncComponent(() => import('./components/AdvancedWorkspacePanel.vue'))
const AnalysisContextDock = defineAsyncComponent(() => import('./components/AnalysisContextDock.vue'))
const CommandPaletteOverlay = defineAsyncComponent(() => import('./components/CommandPaletteOverlay.vue'))
const DataWorkspacePanel = defineAsyncComponent(() => import('./components/DataWorkspacePanel.vue'))
const DebugWorkspacePanel = defineAsyncComponent(() => import('./components/DebugWorkspacePanel.vue'))
const DrillWorkspacePanel = defineAsyncComponent(() => import('./components/DrillWorkspacePanel.vue'))
const ExecutionWorkspacePanel = defineAsyncComponent(() => import('./components/ExecutionWorkspacePanel.vue'))
const HeatmapWorkspacePanel = defineAsyncComponent(() => import('./components/HeatmapWorkspacePanel.vue'))
const HistoryWorkspacePanel = defineAsyncComponent(() => import('./components/HistoryWorkspacePanel.vue'))
const HistoryContextDock = defineAsyncComponent(() => import('./components/HistoryContextDock.vue'))
const MetaWorkspacePanel = defineAsyncComponent(() => import('./components/MetaWorkspacePanel.vue'))
const MlSignalShellPanel = defineAsyncComponent(() => import('./components/MlSignalShellPanel.vue'))
const OptimizeContextDock = defineAsyncComponent(() => import('./components/OptimizeContextDock.vue'))
const OptimizeWorkspacePanel = defineAsyncComponent(() => import('./components/OptimizeWorkspacePanel.vue'))
const OverviewWorkspacePanel = defineAsyncComponent(() => import('./components/OverviewWorkspacePanel.vue'))
const ParetoWorkspacePanel = defineAsyncComponent(() => import('./components/ParetoWorkspacePanel.vue'))
const SettingsWorkspacePanel = defineAsyncComponent(() => import('./components/SettingsWorkspacePanel.vue'))
const StrategyWorkspacePanel = defineAsyncComponent(() => import('./components/StrategyWorkspacePanel.vue'))
const TaskCenterOverlay = defineAsyncComponent(() => import('./components/TaskCenterOverlay.vue'))
const TopWorkspacePanel = defineAsyncComponent(() => import('./components/TopWorkspacePanel.vue'))
const ValidationContextDock = defineAsyncComponent(() => import('./components/ValidationContextDock.vue'))
const ValidationWorkspacePanel = defineAsyncComponent(() => import('./components/ValidationWorkspacePanel.vue'))
const WorkflowQuickSwitch = defineAsyncComponent(() => import('./components/WorkflowQuickSwitch.vue'))

const workflowStore = useWorkflowStore()
workflowStore.syncRuntimeState()

const { controlsTab, mode, resultsTab, workflow, workspace } = storeToRefs(workflowStore)

const settingsWorkspaceActive = computed(() => String(workspace.value || '').trim() === 'settings')
const currentWorkspace = computed(() => String(workspace.value || '').trim() || 'workflow')
const mlSignalModeActive = computed(() => String(mode.value || '').trim() === 'ml-signal')
const activeControlsTab = computed(() => String(controlsTab.value || '').trim() || 'data')
const activeResultsTab = computed(() => String(resultsTab.value || '').trim() || 'overview')
const analysisDockActive = computed(() =>
  isAnalysisDockVisible({
    mode: mode.value,
    workspace: workspace.value,
    workflow: workflow.value,
  }),
)
const optimizeDockActive = computed(() =>
  isOptimizeDockVisible({
    mode: mode.value,
    workspace: workspace.value,
    controlsTab: controlsTab.value,
  }),
)
const validationDockActive = computed(() =>
  isValidationDockVisible({
    mode: mode.value,
    workspace: workspace.value,
    resultsTab: resultsTab.value,
  }),
)
const historyPanelActive = computed(() =>
  isHistoryDockVisible({
    workspace: workspace.value,
    workflow: workflow.value,
  }),
)
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
const optimizePanelActive = computed(() => activeControlsTab.value === 'opt' && !mlSignalModeActive.value)
const mlSignalPanelActive = computed(() => activeControlsTab.value === 'opt' && mlSignalModeActive.value)
const strategyPanelActive = computed(() => activeControlsTab.value === 'strategy')
const advancedPanelActive = computed(() => activeControlsTab.value === 'advanced')
const overviewPanelActive = computed(() => activeResultsTab.value === 'overview' && !mlSignalModeActive.value)
const topPanelActive = computed(() => activeResultsTab.value === 'top' && !mlSignalModeActive.value)
const paretoPanelActive = computed(() => activeResultsTab.value === 'pareto' && !mlSignalModeActive.value)
const validationPanelActive = computed(() => activeResultsTab.value === 'validate' && !mlSignalModeActive.value)
const drillPanelActive = computed(() => activeResultsTab.value === 'drill' && !mlSignalModeActive.value)
const heatmapPanelActive = computed(() => activeResultsTab.value === 'heatmap' && !mlSignalModeActive.value)
const metaPanelActive = computed(() => activeResultsTab.value === 'meta' && (!mlSignalModeActive.value || currentWorkspace.value !== 'workflow'))
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
  <StartupAnimationOverlay />
  <DesktopTitlebar />
  <VueWorkspaceShell />
  <CommandPaletteOverlay />
  <GlobalShortcutBridge />
  <AppRuntimeBanner />
  <TaskCenterOverlay />
  <AnalysisContextDock v-if="analysisDockActive" />
  <AdvancedWorkspacePanel v-if="advancedPanelActive" />
  <DataWorkspacePanel v-if="mountedPanels.data" />
  <DebugWorkspacePanel v-if="debugPanelActive" />
  <DrillWorkspacePanel v-if="drillPanelActive" />
  <ExecutionWorkspacePanel v-if="settingsWorkspaceActive" />
  <HeatmapWorkspacePanel v-if="heatmapPanelActive" />
  <HistoryContextDock v-if="historyPanelActive" />
  <HistoryWorkspacePanel v-if="historyPanelActive" />
  <MetaWorkspacePanel v-if="metaPanelActive" />
  <MlSignalShellPanel v-if="mlSignalPanelActive" />
  <OverviewWorkspacePanel v-if="mountedPanels.overview" />
  <OptimizeContextDock v-if="optimizeDockActive" />
  <ParetoWorkspacePanel v-if="paretoPanelActive" />
  <OptimizeWorkspacePanel v-if="mountedPanels.optimize && !mlSignalModeActive" />
  <RunFooterPanel />
  <SettingsWorkspacePanel v-if="settingsWorkspaceActive" />
  <StrategyWorkspacePanel v-if="strategyPanelActive" />
  <TopWorkspacePanel v-if="topPanelActive" />
  <ValidationContextDock v-if="validationDockActive" />
  <ValidationWorkspacePanel v-if="validationPanelActive" />
  <WorkflowQuickSwitch v-if="showWorkflowQuickSwitch" />
</template>
