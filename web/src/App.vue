<script setup lang="ts">
import { computed, defineAsyncComponent, reactive, watch } from 'vue'
import { storeToRefs } from 'pinia'
import AppRuntimeBanner from './components/AppRuntimeBanner.vue'
import DesktopTitlebar from './components/DesktopTitlebar.vue'
import GlobalShortcutBridge from './components/GlobalShortcutBridge.vue'
import OptimizeWorkspacePanel from './components/OptimizeWorkspacePanel.vue'
import ActionStatusBar from './components/ActionStatusBar.vue'
import StrategyWorkspacePanel from './components/StrategyWorkspacePanel.vue'
import WorkbenchShell from './components/workbench/WorkbenchShell.vue'
import { useWorkflowStore } from './stores/workflowStore'

const AdvancedWorkspacePanel = defineAsyncComponent(() => import('./components/AdvancedWorkspacePanel.vue'))
const CommandPaletteOverlay = defineAsyncComponent(() => import('./components/CommandPaletteOverlay.vue'))
const DataWorkspacePanel = defineAsyncComponent(() => import('./components/DataWorkspacePanel.vue'))
const DebugWorkspacePanel = defineAsyncComponent(() => import('./components/DebugWorkspacePanel.vue'))
const CandidateDetailWorkspacePanel = defineAsyncComponent(() => import('./components/CandidateDetailWorkspacePanel.vue'))
const ExecutionWorkspacePanel = defineAsyncComponent(() => import('./components/ExecutionWorkspacePanel.vue'))
const HeatmapWorkspacePanel = defineAsyncComponent(() => import('./components/HeatmapWorkspacePanel.vue'))
const HistoryWorkspacePanel = defineAsyncComponent(() => import('./components/HistoryWorkspacePanel.vue'))
const MetaWorkspacePanel = defineAsyncComponent(() => import('./components/MetaWorkspacePanel.vue'))
const MlSignalShellPanel = defineAsyncComponent(() => import('./components/MlSignalShellPanel.vue'))
const OverviewWorkspacePanel = defineAsyncComponent(() => import('./components/OverviewWorkspacePanel.vue'))
const ParetoWorkspacePanel = defineAsyncComponent(() => import('./components/ParetoWorkspacePanel.vue'))
const PortfolioFeasibilityWorkspacePanel = defineAsyncComponent(() => import('./components/PortfolioFeasibilityWorkspacePanel.vue'))
const SettingsWorkspacePanel = defineAsyncComponent(() => import('./components/SettingsWorkspacePanel.vue'))
const TaskCenterOverlay = defineAsyncComponent(() => import('./components/TaskCenterOverlay.vue'))
const TopWorkspacePanel = defineAsyncComponent(() => import('./components/TopWorkspacePanel.vue'))
const ValidationWorkspacePanel = defineAsyncComponent(() => import('./components/ValidationWorkspacePanel.vue'))
const WorkflowQuickSwitch = defineAsyncComponent(() => import('./components/WorkflowQuickSwitch.vue'))

const workflowStore = useWorkflowStore()
workflowStore.syncRuntimeState()

const { controlsTab, mode, resultsTab, workflow, workspace } = storeToRefs(workflowStore)

const settingsWorkspaceActive = computed(() => String(workspace.value || '').trim() === 'settings')
const liveWorkspaceActive = computed(() => String(workspace.value || '').trim() === 'live')
const portfolioWorkspaceActive = computed(() => String(workspace.value || '').trim() === 'portfolio')
const currentWorkspace = computed(() => String(workspace.value || '').trim() || 'workflow')
const mlSignalModeActive = computed(() => String(mode.value || '').trim() === 'ml-signal')
const activeControlsTab = computed(() => String(controlsTab.value || '').trim() || 'data')
const activeResultsTab = computed(() => String(resultsTab.value || '').trim() || 'overview')
const historyPanelActive = computed(() =>
  String(workspace.value || '').trim() === 'library' || String(workflow.value || '').trim() === 'history',
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

type LazyPanelKey = 'data' | 'live' | 'optimize' | 'overview' | 'portfolio'

const mountedPanels = reactive<Record<LazyPanelKey, boolean>>({
  data: false,
  live: false,
  optimize: false,
  overview: false,
  portfolio: false,
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
mountWhenActive('live', () => liveWorkspaceActive.value)
mountWhenActive('optimize', () => optimizePanelActive.value)
mountWhenActive('overview', () => overviewPanelActive.value)
mountWhenActive('portfolio', () => portfolioWorkspaceActive.value)
</script>

<template>
  <DesktopTitlebar />
  <WorkbenchShell />
  <CommandPaletteOverlay />
  <GlobalShortcutBridge />
  <AppRuntimeBanner />
  <TaskCenterOverlay />
  <AdvancedWorkspacePanel v-if="advancedPanelActive" />
  <DataWorkspacePanel v-if="mountedPanels.data" />
  <DebugWorkspacePanel v-if="debugPanelActive" />
  <CandidateDetailWorkspacePanel v-if="drillPanelActive" />
  <ExecutionWorkspacePanel v-if="mountedPanels.live" />
  <HeatmapWorkspacePanel v-if="heatmapPanelActive" />
  <HistoryWorkspacePanel v-if="historyPanelActive" />
  <MetaWorkspacePanel v-if="metaPanelActive" />
  <MlSignalShellPanel v-if="mlSignalPanelActive" />
  <OverviewWorkspacePanel v-if="mountedPanels.overview" />
  <ParetoWorkspacePanel v-if="paretoPanelActive" />
  <PortfolioFeasibilityWorkspacePanel v-if="mountedPanels.portfolio" />
  <OptimizeWorkspacePanel v-if="mountedPanels.optimize && !mlSignalModeActive" />
  <ActionStatusBar />
  <SettingsWorkspacePanel v-if="settingsWorkspaceActive" />
  <StrategyWorkspacePanel v-if="strategyPanelActive" />
  <TopWorkspacePanel v-if="topPanelActive" />
  <ValidationWorkspacePanel v-if="validationPanelActive" />
  <WorkflowQuickSwitch v-if="showWorkflowQuickSwitch" />
</template>
