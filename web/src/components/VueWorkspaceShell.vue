<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useWorkflowStore } from '../stores/workflowStore'

const workflowStore = useWorkflowStore()

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
const shellSubtitle = computed(() => {
  const text = String(workflowSubtitle.value || '').trim()
  return text || '—'
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
      <section class="workspaceGrid" v-show="!settingsWorkspaceActive">
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
  padding: 0.85rem 1rem 0.7rem;
  display: grid;
  gap: 0.45rem;
}

.brandRow {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.7rem;
}

.brandRow h1 {
  margin: 0;
  font-size: 1rem;
  letter-spacing: 0.02em;
}

.subtitle {
  font-size: 0.76rem;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 64%;
}

.tabsRow {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.tabBtn {
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface2);
  color: inherit;
  font-size: 0.74rem;
  padding: 0.34rem 0.62rem;
}

.tabBtn.active {
  border-color: color-mix(in oklab, var(--accent) 64%, var(--border));
  background: color-mix(in oklab, var(--accent-soft) 68%, var(--surface2));
}

.shellMain {
  min-height: 0;
  overflow: hidden;
}

.workspaceGrid {
  height: 100%;
  display: grid;
  grid-template-columns: minmax(340px, 38%) minmax(440px, 1fr);
  gap: 0.8rem;
  padding: 0.8rem;
}

.controlsColumn,
.resultsColumn {
  min-height: 0;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel);
  padding: 0.65rem;
  display: grid;
  align-content: start;
  gap: 0.55rem;
}

.tabsRow.compact .tabBtn {
  font-size: 0.71rem;
  padding: 0.28rem 0.5rem;
}

.panelHost {
  min-height: 0;
}

#settingsShellGrid {
  height: 100%;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel);
  margin: 0.8rem;
  padding: 0.8rem;
  display: grid;
  gap: 0.75rem;
}

#executionShellPane {
  border-top: 1px dashed var(--border);
  padding-top: 0.75rem;
}

#runFooterShell {
  border-top: 1px solid var(--border);
  background: var(--panel-footer);
  padding: 0.55rem 0.8rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1200px) {
  .workspaceGrid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 760px) {
  .shellHeader {
    padding: 0.7rem 0.65rem 0.6rem;
  }

  .workspaceGrid {
    padding: 0.65rem;
    gap: 0.65rem;
  }

  #settingsShellGrid {
    margin: 0.65rem;
    padding: 0.65rem;
  }
}
</style>
