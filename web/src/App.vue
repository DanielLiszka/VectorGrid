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
import { useSettingsStore } from './stores/settingsStore'
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
const GuidedWorkspacePanel = defineAsyncComponent(() => import('./components/GuidedWorkspacePanel.vue'))
const OverviewWorkspacePanel = defineAsyncComponent(() => import('./components/OverviewWorkspacePanel.vue'))
const ParetoWorkspacePanel = defineAsyncComponent(() => import('./components/ParetoWorkspacePanel.vue'))
const SettingsWorkspacePanel = defineAsyncComponent(() => import('./components/SettingsWorkspacePanel.vue'))
const TaskCenterOverlay = defineAsyncComponent(() => import('./components/TaskCenterOverlay.vue'))
const TopWorkspacePanel = defineAsyncComponent(() => import('./components/TopWorkspacePanel.vue'))
const ValidationWorkspacePanel = defineAsyncComponent(() => import('./components/ValidationWorkspacePanel.vue'))
const WorkflowQuickSwitch = defineAsyncComponent(() => import('./components/WorkflowQuickSwitch.vue'))

const workflowStore = useWorkflowStore()
const settingsStore = useSettingsStore()
workflowStore.syncRuntimeState()

const { controlsTab, mode, resultsTab, workflow, workspace } = storeToRefs(workflowStore)
const { uiMode } = storeToRefs(settingsStore)

const guidedModeActive = computed(() => uiMode.value === 'guided')
const settingsWorkspaceActive = computed(() => !guidedModeActive.value && String(workspace.value || '').trim() === 'settings')
const liveWorkspaceActive = computed(() => !guidedModeActive.value && String(workspace.value || '').trim() === 'live')
const currentWorkspace = computed(() => String(workspace.value || '').trim() || 'workflow')
const mlSignalModeActive = computed(() => String(mode.value || '').trim() === 'ml-signal')
const activeControlsTab = computed(() => String(controlsTab.value || '').trim() || 'data')
const activeResultsTab = computed(() => String(resultsTab.value || '').trim() || 'overview')
const historyPanelActive = computed(() =>
  !guidedModeActive.value
  && (String(workspace.value || '').trim() === 'library' || String(workflow.value || '').trim() === 'history'),
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
const dataPanelActive = computed(() => !guidedModeActive.value && activeControlsTab.value === 'data')
const optimizePanelActive = computed(() => !guidedModeActive.value && activeControlsTab.value === 'opt' && !mlSignalModeActive.value)
const mlSignalPanelActive = computed(() => !guidedModeActive.value && activeControlsTab.value === 'opt' && mlSignalModeActive.value)
const strategyPanelActive = computed(() => !guidedModeActive.value && activeControlsTab.value === 'strategy')
const advancedPanelActive = computed(() => !guidedModeActive.value && activeControlsTab.value === 'advanced')
const overviewPanelActive = computed(() => !guidedModeActive.value && activeResultsTab.value === 'overview' && !mlSignalModeActive.value)
const topPanelActive = computed(() => !guidedModeActive.value && activeResultsTab.value === 'top' && !mlSignalModeActive.value)
const paretoPanelActive = computed(() => !guidedModeActive.value && activeResultsTab.value === 'pareto' && !mlSignalModeActive.value)
const validationPanelActive = computed(() => !guidedModeActive.value && activeResultsTab.value === 'validate' && !mlSignalModeActive.value)
const drillPanelActive = computed(() => !guidedModeActive.value && activeResultsTab.value === 'drill' && !mlSignalModeActive.value)
const heatmapPanelActive = computed(() => !guidedModeActive.value && activeResultsTab.value === 'heatmap' && !mlSignalModeActive.value)
const metaPanelActive = computed(() => !guidedModeActive.value && activeResultsTab.value === 'meta' && (!mlSignalModeActive.value || currentWorkspace.value !== 'workflow'))
const debugPanelActive = computed(() => !guidedModeActive.value && activeResultsTab.value === 'debug')

type LazyPanelKey = 'data' | 'live' | 'optimize' | 'overview'

const mountedPanels = reactive<Record<LazyPanelKey, boolean>>({
  data: false,
  live: false,
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
mountWhenActive('live', () => liveWorkspaceActive.value)
mountWhenActive('optimize', () => optimizePanelActive.value)
mountWhenActive('overview', () => overviewPanelActive.value)
</script>

<template>
  <DesktopTitlebar />
  <WorkbenchShell />
  <GuidedWorkspacePanel v-if="guidedModeActive" />
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
  <OptimizeWorkspacePanel v-if="mountedPanels.optimize && !mlSignalModeActive" />
  <ActionStatusBar v-if="!guidedModeActive" />
  <SettingsWorkspacePanel v-if="settingsWorkspaceActive" />
  <StrategyWorkspacePanel v-if="strategyPanelActive" />
  <TopWorkspacePanel v-if="topPanelActive" />
  <ValidationWorkspacePanel v-if="validationPanelActive" />
  <WorkflowQuickSwitch v-if="showWorkflowQuickSwitch" />
</template>
