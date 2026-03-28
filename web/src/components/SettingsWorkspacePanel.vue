<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useStorage } from '@vueuse/core'
import { useShellMount } from '../composables/useShellMount'
import { useTaskLifecycleSummary } from '../composables/useTaskLifecycleSummary'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { useDataStore } from '../stores/dataStore'
import { useRuntimeStore } from '../stores/runtimeStore'
import { useSettingsStore } from '../stores/settingsStore'
import { dismissToast, useTaskStore } from '../stores/taskStore'
import { useWorkflowStore } from '../stores/workflowStore'
import {
  describeLatestTaskOutcome,
  describeTaskToast,
  describeTimelineEntry,
  describeRuntimePhase,
  describeTrackedTask,
  formatTaskPhaseLabel,
  formatTaskSourceLabel,
  runtimePhaseLabel as formatRuntimePhaseLabel,
  taskLevelLabel,
  taskLevelTone,
  taskLifecycleLabel,
  taskLifecycleTone,
} from '../utils/statusPresentation'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const settingsStore = useSettingsStore()
const dataStore = useDataStore()
const runtimeStore = useRuntimeStore()
const taskStore = useTaskStore()
const workflowStore = useWorkflowStore()
const {
  theme,
  effect,
  helpMode,
  workflowAdvanced,
  controlsCollapsed,
  marketDataDir,
  moveExisting,
  marketDataDirHint,
  canBrowseMarketDataDir,
  canSetMarketDataDir,
  canResetMarketDataDir,
  canOpenMarketDataDir,
  canOpenWorkflowData,
  canOpenRunsDir,
  canOpenReportsDir,
  canOpenPolygonModal,
  canGotoLibrary,
  canGotoAggregate,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(settingsStore)
const { updatedAtUnixMs: dataUpdatedAtUnixMs } = storeToRefs(dataStore)
const { bootElapsedMs, bootError, bootErrorDetail, bootPhase, bootStage } = storeToRefs(runtimeStore)
const { timeline, toasts } = storeToRefs(taskStore)
const { workspace } = storeToRefs(workflowStore)
const taskCenterOpen = useStorage<boolean>('VTA_TASK_CENTER_OPEN_V1', false)
const {
  activeTaskCount,
  centerStatusLabel,
  taskHeadline,
  latestTask,
  overallProgressLabel: progressLabel,
} = useTaskLifecycleSummary()

const { mountReady } = useShellMount('#settingsVueShellMount', {
  paneSelector: '#settingsShellGrid',
  paneDatasetKey: 'vueSettings',
})
let runtimeObserver: MutationObserver | null = null

const themeDraft = ref('system')
const effectDraft = ref('none')
const helpModeDraft = ref('full')
const marketDataDirDraft = ref('')

const visible = computed(() => String(workspace.value || '').trim() === 'settings')
const settingsSummaryChips = computed(() => [
  `Theme ${String(themeDraft.value || 'system').trim() || 'system'}`,
  `Effect ${String(effectDraft.value || 'none').trim() || 'none'}`,
  `Help ${String(helpModeDraft.value || 'full').trim() || 'full'}`,
  workflowAdvanced.value ? 'Advanced controls on' : 'Advanced controls off',
  controlsCollapsed.value ? 'Controls collapsed first' : 'Controls expanded first',
  marketDataDirDraft.value.trim().length > 0 ? 'Custom data folder' : 'Default data folder',
])
const diagnosticsSummaryChips = computed(() => [
  `Runtime ${runtimePhaseLabel.value}`,
  `Task center ${centerStatusLabel.value}`,
  `${activeTaskCount.value} active`,
  progressLabel.value,
])

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))
const runtimeStamp = computed(() => {
  return formatRuntimePhaseLabel(bootPhase.value)
})
const runtimePhaseLabel = computed(() => {
  return formatRuntimePhaseLabel(bootPhase.value)
})
const runtimeDetail = computed(() => {
  return describeRuntimePhase({
    phase: bootPhase.value,
    stage: bootStage.value,
    elapsedMs: bootElapsedMs.value,
    error: bootError.value,
  })
})
const runtimeErrorBody = computed(() => String(bootErrorDetail.value || '').trim())
const recentTimeline = computed(() => timeline.value.slice(0, 6))
const recentAlerts = computed(() => toasts.value.slice(0, 4))
const latestTaskLabel = computed(() => {
  return describeLatestTaskOutcome(latestTask.value)
})
const latestTaskDetail = computed(() => {
  return latestTask.value ? describeTrackedTask(latestTask.value) : 'No completed or cancelled tasks yet.'
})
const latestTaskTone = computed(() => {
  return latestTask.value ? taskLifecycleTone(latestTask.value.status) : 'idle'
})
const latestTaskStatusLabel = computed(() => {
  return latestTask.value ? taskLifecycleLabel(latestTask.value.status) : 'Idle'
})

function diagnosticsLevelTone(level: string | null | undefined): string {
  return taskLevelTone(level)
}

function diagnosticsLevelLabel(level: string | null | undefined): string {
  return taskLevelLabel(level)
}

function diagnosticsPhaseLabel(phase: string | null | undefined): string {
  return formatTaskPhaseLabel(phase)
}

function diagnosticsSourceLabel(source: string | null | undefined): string {
  return formatTaskSourceLabel(source)
}

function diagnosticsTimelineSummary(index: number): string {
  return describeTimelineEntry(recentTimeline.value[index] ?? null)
}

function diagnosticsAlertSummary(index: number): string {
  return describeTaskToast(recentAlerts.value[index] ?? null)
}

watch(
  () => [
    String(theme.value || 'system').trim() || 'system',
    String(effect.value || 'none').trim() || 'none',
    String(helpMode.value || 'full').trim() || 'full',
    String(marketDataDir.value || ''),
  ] as const,
  ([nextTheme, nextEffect, nextHelpMode, nextMarketDataDir]) => {
    if (themeDraft.value !== nextTheme) themeDraft.value = nextTheme
    if (effectDraft.value !== nextEffect) effectDraft.value = nextEffect
    if (helpModeDraft.value !== nextHelpMode) helpModeDraft.value = nextHelpMode
    if (marketDataDirDraft.value !== nextMarketDataDir) marketDataDirDraft.value = nextMarketDataDir
  },
  { immediate: true },
)

watch(
  dataUpdatedAtUnixMs,
  () => {
    settingsStore.syncFromDataStore()
  },
  { immediate: true },
)

function setMarketDataDirFromDraft(): void {
  settingsStore.setMarketDataDir(String(marketDataDirDraft.value || '').trim(), !!moveExisting.value)
}

function setCheckboxValue(id: string, checked: boolean): void {
  settingsStore.setCheckboxField(id, checked)
}

function setSelectValue(id: string, nextValue: string): void {
  settingsStore.setSelectField(id, nextValue)
}

function setInputValue(id: string, nextValue: string): void {
  settingsStore.setInputField(id, nextValue)
}

function openTaskCenter(): void {
  taskCenterOpen.value = true
}

function clearRecentAlerts(): void {
  for (const toast of recentAlerts.value) dismissToast(toast.id)
}

const { queueInputValue, commitInputValue, flushQueuedInputValuesNow } = useQueuedInputWriter(setInputValue)

onMounted(async () => {
  settingsStore.syncFromDataStore()
  if (typeof document !== 'undefined') {
    runtimeObserver = new MutationObserver(() => settingsStore.syncFromDataStore())
    runtimeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-running'],
    })
  }
})

onBeforeUnmount(() => {
  flushQueuedInputValuesNow()
  if (runtimeObserver) {
    runtimeObserver.disconnect()
    runtimeObserver = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#settingsVueShellMount">
    <section class="settingsPanelVueRoot settingsVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Workspace</div>
          <h2 class="panelTitle">Settings</h2>
          <div class="panelSubtitle">
            Review appearance, storage paths, and local utility shortcuts without leaving the desktop workflow.
          </div>
        </div>
        <div class="panelMeta mono panelNumeric">{{ stamp }}</div>
      </header>
      <div class="panelSummaryChips" aria-label="Settings summary">
        <span v-for="chip in settingsSummaryChips" :key="`settings_summary_${chip}`" class="panelSummaryChip mono">
          {{ chip }}
        </span>
      </div>

      <div class="grid">
        <section class="pane panelProminentPane">
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Appearance</div>
              <h3 class="panelPaneTitle">Appearance</h3>
              <div class="panelSectionLead">
                Keep the shell readable first. Theme and window effects should support dense workflows rather than compete with them.
              </div>
            </div>
          </div>
          <div class="rowTwo">
            <div>
              <label for="vueSettingsTheme">Theme</label>
              <select
                id="vueSettingsTheme"
                :value="themeDraft"
                @change="themeDraft = String(($event.target as HTMLSelectElement).value || 'system'); setSelectValue('settingsThemeSelect', themeDraft)"
              >
                <option value="light">Light</option>
                <option value="dark">Dark</option>
                <option value="system">System</option>
              </select>
            </div>
            <div>
              <label for="vueSettingsEffect">Window effect</label>
              <select
                id="vueSettingsEffect"
                :value="effectDraft"
                @change="effectDraft = String(($event.target as HTMLSelectElement).value || 'none'); setSelectValue('settingsEffectSelect', effectDraft)"
              >
                <option value="none">None</option>
                <option value="mica">Mica</option>
                <option value="acrylic">Acrylic</option>
                <option value="vibrancy">Vibrancy</option>
              </select>
            </div>
          </div>
          <div class="rowSingle">
            <label for="vueSettingsHelpMode">Help and guidance</label>
            <select
              id="vueSettingsHelpMode"
              :value="helpModeDraft"
              @change="helpModeDraft = String(($event.target as HTMLSelectElement).value || 'full'); setSelectValue('settingsHelpMode', helpModeDraft)"
            >
              <option value="full">Full help (tooltips + long-hover callouts)</option>
              <option value="concise">Concise help (tooltips only)</option>
              <option value="off">Off (hide non-essential help overlays)</option>
            </select>
            <div class="panelMuted">
              Use this as the global policy for optional in-app help. Safety notices, errors, and runtime status still stay visible.
            </div>
          </div>
          <div class="checks">
            <label>
              <input
                type="checkbox"
                :checked="workflowAdvanced"
                @change="setCheckboxValue('settingsWorkflowAdvanced', ($event.target as HTMLInputElement).checked)"
              />
              Show advanced controls by default
            </label>
            <label>
              <input
                type="checkbox"
                :checked="controlsCollapsed"
                @change="setCheckboxValue('settingsControlsCollapsed', ($event.target as HTMLInputElement).checked)"
              />
              Start with controls panel collapsed
            </label>
          </div>
        </section>

        <section class="pane">
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Storage</div>
              <h3 class="panelPaneTitle">Data and storage</h3>
              <div class="panelSectionLead">
                Point the app at the right market-data cache and decide whether existing local data should move with it.
              </div>
            </div>
          </div>
          <label for="vueSettingsMarketDataDir">Market data folder</label>
          <div class="inputWithActions">
            <input
              id="vueSettingsMarketDataDir"
              type="text"
              :value="marketDataDirDraft"
              placeholder="Default (AppData)"
              @input="marketDataDirDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('settingsMarketDataDir', marketDataDirDraft)"
              @change="commitInputValue('settingsMarketDataDir', marketDataDirDraft)"
            />
            <div class="actions">
              <button type="button" :disabled="!canBrowseMarketDataDir" @click="settingsStore.browseMarketDataDir()">Browse</button>
              <button type="button" :disabled="!canSetMarketDataDir" @click="setMarketDataDirFromDraft">Set</button>
            </div>
          </div>
          <label class="inlineCheck">
            <input
              type="checkbox"
              :checked="moveExisting"
              @change="setCheckboxValue('settingsMoveExisting', ($event.target as HTMLInputElement).checked)"
            />
            Move existing Polygon cache
          </label>
          <div class="actions">
            <button type="button" :disabled="!canResetMarketDataDir" @click="settingsStore.resetMarketDataDir()">Reset to default</button>
            <button type="button" :disabled="!canOpenMarketDataDir" @click="settingsStore.openMarketDataDir()">Open folder</button>
            <button type="button" :disabled="!canOpenWorkflowData" @click="settingsStore.openWorkflowData()">Open workflow data panel</button>
          </div>
          <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ marketDataDirHint }}</div>
        </section>

        <section class="pane">
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Utilities</div>
              <h3 class="panelPaneTitle">Utilities</h3>
              <div class="panelSectionLead">
                Jump to folders and adjacent workspaces when you need to inspect artifacts outside the main workflow view.
              </div>
            </div>
          </div>
          <div class="actions">
            <button type="button" :disabled="!canOpenRunsDir" @click="settingsStore.openRunsDir()">Open runs folder</button>
            <button type="button" :disabled="!canOpenReportsDir" @click="settingsStore.openReportsDir()">Open reports folder</button>
            <button type="button" :disabled="!canOpenPolygonModal" @click="settingsStore.openPolygonModal()">Open Polygon downloader</button>
          </div>
          <div class="actions">
            <button type="button" :disabled="!canGotoLibrary" @click="settingsStore.gotoLibrary()">Go to Run Library</button>
            <button type="button" :disabled="!canGotoAggregate" @click="settingsStore.gotoAggregate()">Go to Aggregate Analysis</button>
          </div>
        </section>

        <section class="pane">
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Navigation</div>
              <h3 class="panelPaneTitle">Keyboard shortcuts</h3>
              <div class="panelSectionLead">
                These shortcuts cover the highest-frequency commands so the main authoring flow stays keyboard-friendly.
              </div>
            </div>
          </div>
          <div class="shortcutGrid">
            <div class="mono">Ctrl+K</div>
            <div>Command palette</div>
            <div class="mono">Ctrl+Enter</div>
            <div>Run optimization</div>
            <div class="mono">Ctrl+Shift+V</div>
            <div>Run validation</div>
            <div class="mono">Ctrl+Shift+C</div>
            <div>Export top rows CSV</div>
            <div class="mono">Ctrl+O</div>
            <div>Browse CSV file</div>
          </div>
        </section>

        <section class="pane diagnosticsPane">
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Diagnostics</div>
              <h3 class="panelPaneTitle">Runtime and task center</h3>
              <div class="panelSectionLead">
                Review startup state, current task posture, and recent operator-facing alerts from one place before you need external logs.
              </div>
            </div>
          </div>
          <div class="panelSummaryChips" aria-label="Diagnostics summary">
            <span v-for="chip in diagnosticsSummaryChips" :key="`settings_diagnostics_${chip}`" class="panelSummaryChip mono">
              {{ chip }}
            </span>
          </div>
          <div class="actions">
            <button type="button" @click="openTaskCenter">Open task center</button>
            <button type="button" :disabled="recentAlerts.length === 0" @click="clearRecentAlerts">Clear recent alerts</button>
          </div>
          <div class="diagnosticsGrid">
            <section class="diagnosticsCard">
              <header class="diagnosticsHead">
                <div class="diagnosticsLabel">Runtime</div>
                <div class="mono diagnosticsStamp">{{ runtimeStamp }}</div>
              </header>
              <div class="diagnosticsPrimary">{{ runtimeDetail }}</div>
              <details v-if="runtimeErrorBody" class="panelDetails">
                <summary>Runtime error detail</summary>
                <div class="panelDetailsBody">
                  <pre class="mono diagnosticsPre">{{ runtimeErrorBody }}</pre>
                </div>
              </details>
            </section>

            <section class="diagnosticsCard">
              <header class="diagnosticsHead">
                <div class="diagnosticsLabel">Task posture</div>
                <div class="mono diagnosticsStamp">{{ centerStatusLabel }}</div>
              </header>
              <div v-if="latestTask" class="diagnosticsPrimary">
                {{ latestTaskLabel }}
              </div>
              <div v-if="latestTask" class="diagnosticsEntryMeta">
                <span class="mono diagnosticsBadge" :class="latestTaskTone">{{ latestTaskStatusLabel }}</span>
              </div>
              <div class="panelMuted">
                {{ latestTaskDetail }}
              </div>
              <div class="panelMuted">
                {{ taskHeadline }}
              </div>
              <div class="panelMuted">
                {{ progressLabel }}
              </div>
            </section>

            <section class="diagnosticsCard diagnosticsCardWide">
              <header class="diagnosticsHead">
                <div class="diagnosticsLabel">Recent timeline</div>
                <div class="mono diagnosticsStamp">{{ recentTimeline.length }} items</div>
              </header>
              <div v-if="recentTimeline.length === 0" class="panelMuted">No timeline entries yet.</div>
              <ul v-else class="diagnosticsList">
                <li
                  v-for="(entry, index) in recentTimeline"
                  :key="entry.id"
                  class="diagnosticsEntry"
                  :aria-label="diagnosticsTimelineSummary(index)"
                >
                  <div class="diagnosticsEntryMeta">
                    <span class="mono diagnosticsTime">{{ formatUiTimeFromUnixMs(entry.atUnixMs) }}</span>
                    <span class="mono diagnosticsBadge" :class="diagnosticsLevelTone(entry.level)">{{ diagnosticsLevelLabel(entry.level) }}</span>
                    <span class="mono diagnosticsBadge">{{ diagnosticsPhaseLabel(entry.phase) }}</span>
                  </div>
                  <div class="diagnosticsEntryBody">
                    <strong>{{ diagnosticsSourceLabel(entry.source) }}</strong>
                    <span>{{ entry.message }}</span>
                  </div>
                </li>
              </ul>
            </section>

            <section class="diagnosticsCard diagnosticsCardWide">
              <header class="diagnosticsHead">
                <div class="diagnosticsLabel">Recent alerts</div>
                <div class="mono diagnosticsStamp">{{ recentAlerts.length }} items</div>
              </header>
              <div v-if="recentAlerts.length === 0" class="panelMuted">No recent alerts.</div>
              <ul v-else class="diagnosticsList">
                <li
                  v-for="(toast, index) in recentAlerts"
                  :key="toast.id"
                  class="diagnosticsEntry"
                  :aria-label="diagnosticsAlertSummary(index)"
                >
                  <div class="diagnosticsEntryMeta">
                    <span class="mono diagnosticsTime">{{ formatUiTimeFromUnixMs(toast.atUnixMs) }}</span>
                    <span class="mono diagnosticsBadge" :class="diagnosticsLevelTone(toast.level)">{{ diagnosticsLevelLabel(toast.level) }}</span>
                  </div>
                  <div class="diagnosticsEntryBody">
                    <strong>{{ diagnosticsSourceLabel(toast.source) }}</strong>
                    <span>{{ toast.message }}</span>
                  </div>
                </li>
              </ul>
            </section>
          </div>
        </section>
      </div>

      <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.settingsPanelVueRoot {
  display: grid;
  gap: 0.85rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.8rem;
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
}

.rowTwo {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.7rem;
}

.rowSingle {
  margin-top: 0.7rem;
  display: grid;
  gap: 0.35rem;
}

.checks {
  margin-top: 0.65rem;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.inputWithActions {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.45rem;
}

.actions {
  margin-top: 0.6rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.inlineCheck {
  margin-top: 0.6rem;
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
}

.shortcutGrid {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.35rem 0.8rem;
}

.diagnosticsPane {
  grid-column: 1 / -1;
}

.diagnosticsGrid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.7rem;
}

.diagnosticsCard {
  border: 1px solid color-mix(in oklab, var(--border) 84%, var(--surface-working));
  border-radius: 10px;
  background: color-mix(in oklab, var(--surface-inline) 76%, var(--panel-footer));
  padding: 0.7rem;
  display: grid;
  gap: 0.45rem;
}

.diagnosticsCardWide {
  grid-column: span 2;
}

.diagnosticsHead {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.6rem;
}

.diagnosticsLabel {
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--panelEyebrow, var(--muted));
}

.diagnosticsStamp {
  font-size: 0.72rem;
  color: var(--muted);
}

.diagnosticsPrimary {
  font-size: 0.84rem;
  color: var(--text);
}

.diagnosticsList {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 0.45rem;
}

.diagnosticsEntry {
  display: grid;
  gap: 0.2rem;
  padding-top: 0.45rem;
  border-top: 1px solid color-mix(in oklab, var(--border) 78%, transparent);
}

.diagnosticsEntry:first-child {
  padding-top: 0;
  border-top: 0;
}

.diagnosticsEntryMeta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  align-items: center;
}

.diagnosticsEntryBody {
  display: grid;
  gap: 0.14rem;
  font-size: 0.78rem;
}

.diagnosticsTime,
.diagnosticsBadge {
  font-size: 0.7rem;
}

.diagnosticsBadge {
  border: 1px solid color-mix(in oklab, var(--border) 80%, transparent);
  border-radius: 999px;
  padding: 0.08rem 0.38rem;
  background: color-mix(in oklab, var(--surface-sunken) 76%, var(--panel-footer));
}

.diagnosticsBadge.info,
.diagnosticsBadge.running {
  border-color: color-mix(in oklab, var(--accent) 34%, var(--border));
  background: color-mix(in oklab, var(--selection-fill) 84%, var(--surface-inline));
  color: var(--selection-text);
}

.diagnosticsBadge.warn,
.diagnosticsBadge.cancelling,
.diagnosticsBadge.cancelled {
  border-color: color-mix(in oklab, var(--warning) 38%, var(--border));
  background: color-mix(in oklab, var(--surface-warning) 90%, var(--surface-inline));
  color: var(--warning-ink);
}

.diagnosticsBadge.error,
.diagnosticsBadge.failed {
  border-color: color-mix(in oklab, var(--danger) 38%, var(--border));
  background: color-mix(in oklab, var(--surface-danger) 90%, var(--surface-inline));
  color: var(--danger-ink);
}

.diagnosticsBadge.success {
  border-color: color-mix(in oklab, var(--ok) 38%, var(--border));
  background: color-mix(in oklab, var(--surface-success) 90%, var(--surface-inline));
  color: var(--ok-ink);
}

.diagnosticsPre {
  margin: 0;
  max-height: 220px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1040px) {
  .grid {
    grid-template-columns: 1fr;
  }

  .rowTwo {
    grid-template-columns: 1fr;
  }

  .diagnosticsGrid {
    grid-template-columns: 1fr;
  }

  .diagnosticsCardWide {
    grid-column: auto;
  }

  .inputWithActions {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
#settingsShellGrid[data-vue-settings='1'] > :not(#settingsVueShellMount):not(#executionShellPane) {
  display: none !important;
}
</style>



