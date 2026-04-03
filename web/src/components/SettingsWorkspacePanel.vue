<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import { openTaskCenter as requestTaskCenterOpen, useTaskCenterVisibility } from '../composables/useTaskCenterVisibility'
import { useTaskLifecycleSummary } from '../composables/useTaskLifecycleSummary'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { requestDesktopNotificationPermission } from '../services/desktopProductization'
import { useDataStore } from '../stores/dataStore'
import { useRuntimeStore } from '../stores/runtimeStore'
import { useSettingsStore } from '../stores/settingsStore'
import { dismissToast, taskPostureSnapshot, useTaskStore } from '../stores/taskStore'
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
import { taskLevelIcon, taskStatusIcon } from '../utils/iconography'
import { searchPerspectives } from '../utils/perspectiveSearch'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'
import VectorAlphaLogo from './VectorAlphaLogo.vue'

type DesktopLogFilter = 'all' | 'attention' | 'errors'

const settingsStore = useSettingsStore()
const dataStore = useDataStore()
const runtimeStore = useRuntimeStore()
const taskStore = useTaskStore()
const workflowStore = useWorkflowStore()
const {
  densityMode,
  theme,
  effect,
  helpMode,
  perspectives,
  selectedPerspectiveId,
  workflowAdvanced,
  controlsCollapsed,
  desktopNotificationMode,
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
const {
  bootElapsedMs,
  bootError,
  bootErrorDetail,
  bootPhase,
  bootStage,
  desktopLogs,
  desktopLogConsoleAttached,
  desktopNotificationPermission,
  desktopWindowStateFile,
  desktopWindowStateReady,
} = storeToRefs(runtimeStore)
const { timeline, toasts } = storeToRefs(taskStore)
const { workspace } = storeToRefs(workflowStore)
const taskCenterOpen = useTaskCenterVisibility()
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
const densityModeDraft = ref('standard')
const desktopNotificationModeDraft = ref('off')
const marketDataDirDraft = ref('')
const perspectiveNameDraft = ref('')
const perspectiveSearchDraft = ref('')
const desktopLogFilterDraft = ref<DesktopLogFilter>('all')
const diagnosticsCopied = ref(false)
let diagnosticsCopiedTimer: ReturnType<typeof setTimeout> | null = null

const visible = computed(() => String(workspace.value || '').trim() === 'settings')
const settingsSummaryChips = computed(() => [
  `Theme ${String(themeDraft.value || 'system').trim() || 'system'}`,
  `Effect ${String(effectDraft.value || 'none').trim() || 'none'}`,
  `Help ${String(helpModeDraft.value || 'full').trim() || 'full'}`,
  `Density ${String(densityModeDraft.value || 'standard').trim() || 'standard'}`,
  `Desktop alerts ${String(desktopNotificationModeDraft.value || 'off').trim() || 'off'}`,
  workflowAdvanced.value ? 'Advanced controls on' : 'Advanced controls off',
  controlsCollapsed.value ? 'Controls collapsed first' : 'Controls expanded first',
  marketDataDirDraft.value.trim().length > 0 ? 'Custom data folder' : 'Default data folder',
])
const selectedPerspective = computed(() =>
  perspectives.value.find((entry) => entry.id === selectedPerspectiveId.value) ?? perspectives.value[0] ?? null,
)
const selectedPerspectiveLabel = computed(() => selectedPerspective.value?.name || 'No perspective selected')
const canDeletePerspective = computed(() => !!selectedPerspective.value && !selectedPerspective.value.builtIn)
const canRenamePerspective = computed(() => !!selectedPerspective.value && !selectedPerspective.value.builtIn)
const perspectiveSummaryChips = computed(() => {
  const current = selectedPerspective.value
  if (!current) return ['No saved perspective']
  return [
    current.builtIn ? 'Built-in' : 'Custom',
    `Split ${Math.round(Number(current.snapshot.splitFraction || 0.34) * 100)}%`,
    `Density ${String(current.snapshot.densityMode || 'standard').trim() || 'standard'}`,
  ]
})
const filteredPerspectives = computed(() => searchPerspectives(perspectives.value, perspectiveSearchDraft.value))
const visiblePerspectiveOptions = computed(() => {
  const selected = selectedPerspective.value
  const filtered = filteredPerspectives.value
  if (!selected) return filtered
  if (filtered.some((entry) => entry.id === selected.id)) return filtered
  return [selected, ...filtered]
})
const perspectiveSearchSummary = computed(() => {
  const query = String(perspectiveSearchDraft.value || '').trim()
  const matchCount = filteredPerspectives.value.length
  const total = perspectives.value.length
  const localHelp = 'Search stays local to saved perspectives; use Ctrl+K for global actions.'
  if (!query) return `${total} saved perspectives. ${localHelp}`
  const label = matchCount === 1 ? 'match' : 'matches'
  const selectedHidden =
    !!selectedPerspective.value && !filteredPerspectives.value.some((entry) => entry.id === selectedPerspective.value?.id)
  if (selectedHidden) {
    return `${matchCount} ${label} for “${query}”. Current selection stays visible while filtered. ${localHelp}`
  }
  return `${matchCount} ${label} for “${query}”. ${localHelp}`
})
const diagnosticsSummaryChips = computed(() => [
  `Runtime ${runtimePhaseLabel.value}`,
  `Task center ${centerStatusLabel.value}`,
  `${activeTaskCount.value} active`,
  progressLabel.value,
  desktopLogs.value.length ? `${desktopLogs.value.length} desktop logs` : 'No desktop logs',
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
const filteredDesktopLogs = computed(() => {
  if (desktopLogFilterDraft.value === 'errors') {
    return desktopLogs.value.filter((entry) => entry.level === 'error')
  }
  if (desktopLogFilterDraft.value === 'attention') {
    return desktopLogs.value.filter((entry) => entry.level === 'warn' || entry.level === 'error')
  }
  return desktopLogs.value
})
const recentDesktopLogs = computed(() => filteredDesktopLogs.value.slice(0, 6))
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
const taskPosture = computed(() => taskPostureSnapshot(taskStore.$state, 8))
const diagnosticsTaskChips = computed(() => {
  const counts = taskPosture.value.counts
  return [
    counts.running ? `${counts.running} running` : '',
    counts.queued ? `${counts.queued} queued` : '',
    counts.cancelling ? `${counts.cancelling} cancelling` : '',
    counts.failed ? `${counts.failed} failed` : '',
    counts.cancelled ? `${counts.cancelled} cancelled` : '',
    counts.completed ? `${counts.completed} completed` : '',
  ].filter((chip) => chip.length > 0)
})
const taskAttentionLine = computed(() => {
  const failed = taskPosture.value.failed[0] ?? null
  if (failed) return `${failed.label} failed${String(failed.error || '').trim() ? `: ${String(failed.error || '').trim()}` : '.'}`
  const cancelled = taskPosture.value.cancelled[0] ?? null
  if (cancelled) return `${cancelled.label} was cancelled.`
  return 'No recent failed or cancelled tasks.'
})
const taskCompletedLine = computed(() => {
  const completed = taskPosture.value.completed[0] ?? null
  if (!completed) return 'No recently completed tasks.'
  return `${completed.label} completed.`
})
const desktopNotificationPermissionLabel = computed(() => {
  if (desktopNotificationPermission.value === 'granted') return 'Granted'
  if (desktopNotificationPermission.value === 'denied') return 'Denied'
  if (desktopNotificationPermission.value === 'unsupported') return 'Unavailable'
  if (desktopNotificationPermission.value === 'default') return 'Not granted'
  return 'Unknown'
})
const canRequestDesktopNotifications = computed(() => {
  return desktopNotificationPermission.value !== 'granted' && desktopNotificationPermission.value !== 'unsupported'
})
const desktopWindowStateStamp = computed(() => {
  return desktopWindowStateReady.value ? 'State ready' : 'State unavailable'
})
const desktopWindowStateDetail = computed(() => {
  if (!desktopWindowStateReady.value) {
    return 'Window state persistence is not available in the current desktop runtime.'
  }
  const file = String(desktopWindowStateFile.value || '').trim()
  return file ? `Window geometry persists to ${file}.` : 'Window geometry persists between launches.'
})
const desktopLogFilterLabel = computed(() => {
  if (desktopLogFilterDraft.value === 'errors') return 'Errors only'
  if (desktopLogFilterDraft.value === 'attention') return 'Warnings and errors'
  return 'All levels'
})
const desktopLogStamp = computed(() => {
  const total = desktopLogs.value.length
  const visibleCount = recentDesktopLogs.value.length
  if (desktopLogFilterDraft.value === 'all') {
    return `${visibleCount} items`
  }
  return `${visibleCount} shown of ${total}`
})
const desktopLogEmptyLabel = computed(() => {
  if (desktopLogs.value.length === 0) return 'No desktop log entries yet.'
  return 'No desktop log entries match the current filter.'
})
const desktopDiagnosticsSnapshot = computed(() => {
  const lines = [
    'VectorAlpha desktop diagnostics snapshot',
    `Captured: ${formatUiTimeFromUnixMs(Date.now())}`,
    '',
    'Runtime',
    `- phase: ${runtimePhaseLabel.value}`,
    `- detail: ${runtimeDetail.value}`,
    runtimeErrorBody.value ? `- error: ${runtimeErrorBody.value}` : '',
    '',
    'Desktop',
    `- notifications: ${desktopNotificationPermissionLabel.value}`,
    `- window state: ${desktopWindowStateDetail.value}`,
    `- console mirroring: ${desktopLogConsoleAttached.value ? 'attached in dev' : 'buffered in-app only'}`,
    '',
    'Task posture',
    `- center: ${centerStatusLabel.value}`,
    `- summary: ${taskHeadline.value}`,
    `- progress: ${progressLabel.value}`,
    diagnosticsTaskChips.value.length ? `- counts: ${diagnosticsTaskChips.value.join(', ')}` : '',
    `- latest: ${latestTaskLabel.value}`,
    `- latest detail: ${latestTaskDetail.value}`,
    `- attention: ${taskAttentionLine.value}`,
    `- completed: ${taskCompletedLine.value}`,
    '',
    'Recent desktop logs',
    `- filter: ${desktopLogFilterLabel.value}`,
    ...(recentDesktopLogs.value.length
      ? recentDesktopLogs.value.map(
          (entry) =>
            `- ${formatUiTimeFromUnixMs(entry.atUnixMs)} [${String(entry.level || 'info').toUpperCase()}] ${entry.message}`,
        )
      : ['- none']),
    '',
    'Recent timeline',
    ...(recentTimeline.value.length
      ? recentTimeline.value.map(
          (entry) =>
            `- ${formatUiTimeFromUnixMs(entry.atUnixMs)} [${diagnosticsLevelLabel(entry.level)} / ${diagnosticsPhaseLabel(entry.phase)}] ${diagnosticsSourceLabel(entry.source)}: ${entry.message}`,
        )
      : ['- none']),
    '',
    'Recent alerts',
    ...(recentAlerts.value.length
      ? recentAlerts.value.map(
          (toast) =>
            `- ${formatUiTimeFromUnixMs(toast.atUnixMs)} [${diagnosticsLevelLabel(toast.level)}] ${diagnosticsSourceLabel(toast.source)}: ${toast.message}`,
        )
      : ['- none']),
  ]
  return lines.filter((line) => String(line || '').trim().length > 0).join('\n')
})

function diagnosticsLevelTone(level: string | null | undefined): string {
  return taskLevelTone(level)
}

function diagnosticsLevelLabel(level: string | null | undefined): string {
  return taskLevelLabel(level)
}

function diagnosticsLevelIcon(level: string | null | undefined) {
  return taskLevelIcon(level)
}

function diagnosticsPhaseLabel(phase: string | null | undefined): string {
  return formatTaskPhaseLabel(phase)
}

function diagnosticsStatusIcon(status: string | null | undefined) {
  return taskStatusIcon(status)
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

function diagnosticsDesktopLogSummary(index: number): string {
  const entry = recentDesktopLogs.value[index]
  if (!entry) return 'No desktop log entry.'
  return `${String(entry.level || 'info').toUpperCase()} · ${String(entry.message || '').trim()}`
}

watch(
  () => [
    String(theme.value || 'system').trim() || 'system',
    String(effect.value || 'none').trim() || 'none',
    String(helpMode.value || 'full').trim() || 'full',
    String(densityMode.value || 'standard').trim() || 'standard',
    String(desktopNotificationMode.value || 'off').trim() || 'off',
    String(marketDataDir.value || ''),
    String(selectedPerspectiveId.value || ''),
  ] as const,
  ([nextTheme, nextEffect, nextHelpMode, nextDensityMode, nextDesktopNotificationMode, nextMarketDataDir, nextPerspectiveId]) => {
    if (themeDraft.value !== nextTheme) themeDraft.value = nextTheme
    if (effectDraft.value !== nextEffect) effectDraft.value = nextEffect
    if (helpModeDraft.value !== nextHelpMode) helpModeDraft.value = nextHelpMode
    if (densityModeDraft.value !== nextDensityMode) densityModeDraft.value = nextDensityMode
    if (desktopNotificationModeDraft.value !== nextDesktopNotificationMode) {
      desktopNotificationModeDraft.value = nextDesktopNotificationMode
    }
    if (marketDataDirDraft.value !== nextMarketDataDir) marketDataDirDraft.value = nextMarketDataDir
    if (perspectiveNameDraft.value !== (selectedPerspective.value?.name || '') && nextPerspectiveId) {
      perspectiveNameDraft.value = selectedPerspective.value?.name || ''
    }
  },
  { immediate: true },
)

watch(
  selectedPerspective,
  (next) => {
    const nextName = next?.name || ''
    if (perspectiveNameDraft.value !== nextName) perspectiveNameDraft.value = nextName
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

function clearRecentAlerts(): void {
  for (const toast of recentAlerts.value) dismissToast(toast.id)
}

function clearDesktopLogs(): void {
  runtimeStore.clearDesktopLogs()
}

function openTaskCenter(): void {
  requestTaskCenterOpen(taskCenterOpen)
}

async function copyDesktopDiagnostics(): Promise<void> {
  const copied = await writeClipboardText(desktopDiagnosticsSnapshot.value)
  markDiagnosticsCopied(copied)
}

function createPerspectiveFromDraft(): void {
  settingsStore.createPerspective(String(perspectiveNameDraft.value || '').trim())
  perspectiveNameDraft.value = selectedPerspective.value?.name || ''
}

function applySelectedPerspective(): void {
  settingsStore.applyPerspective(String(selectedPerspectiveId.value || '').trim())
}

function overwriteSelectedPerspective(): void {
  settingsStore.overwritePerspective(String(selectedPerspectiveId.value || '').trim())
}

function renameSelectedPerspective(): void {
  settingsStore.renamePerspective(String(selectedPerspectiveId.value || '').trim(), String(perspectiveNameDraft.value || '').trim())
}

function duplicateSelectedPerspective(): void {
  settingsStore.duplicatePerspective(String(selectedPerspectiveId.value || '').trim())
  perspectiveNameDraft.value = selectedPerspective.value?.name || ''
}

function deleteSelectedPerspective(): void {
  settingsStore.deletePerspective(String(selectedPerspectiveId.value || '').trim())
  perspectiveNameDraft.value = selectedPerspective.value?.name || ''
}

async function requestDesktopNotifications(): Promise<void> {
  await requestDesktopNotificationPermission()
}

async function writeClipboardText(value: string): Promise<boolean> {
  const text = String(value || '')
  const clipboard = typeof navigator === 'undefined' ? null : navigator.clipboard
  if (clipboard?.writeText) {
    await clipboard.writeText(text)
    return true
  }
  if (typeof document !== 'undefined' && typeof document.execCommand === 'function') {
    const textarea = document.createElement('textarea')
    textarea.value = text
    textarea.setAttribute('readonly', 'true')
    textarea.style.position = 'fixed'
    textarea.style.opacity = '0'
    document.body.appendChild(textarea)
    textarea.select()
    const copied = document.execCommand('copy')
    document.body.removeChild(textarea)
    return copied
  }
  return false
}

function markDiagnosticsCopied(active: boolean): void {
  diagnosticsCopied.value = active
  if (diagnosticsCopiedTimer) {
    clearTimeout(diagnosticsCopiedTimer)
    diagnosticsCopiedTimer = null
  }
  if (active) {
    diagnosticsCopiedTimer = setTimeout(() => {
      diagnosticsCopied.value = false
      diagnosticsCopiedTimer = null
    }, 1800)
  }
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
  if (diagnosticsCopiedTimer) {
    clearTimeout(diagnosticsCopiedTimer)
    diagnosticsCopiedTimer = null
  }
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
        <div class="settingsHeaderMeta">
          <div class="panelMeta mono panelNumeric">{{ stamp }}</div>
          <div class="settingsBrandCard" aria-label="VectorAlpha brand mark">
            <VectorAlphaLogo class="settingsBrandLogo" decorative hover-animate :size="30" />
            <div class="settingsBrandCopy">
              <div class="settingsBrandLabel">VectorAlpha</div>
              <div class="settingsBrandDetail">Desktop runtime</div>
            </div>
          </div>
        </div>
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
          <div class="rowSingle">
            <label for="vueSettingsDensityMode">Density</label>
            <select
              id="vueSettingsDensityMode"
              :value="densityModeDraft"
              @change="densityModeDraft = String(($event.target as HTMLSelectElement).value || 'standard'); setSelectValue('settingsDensityMode', densityModeDraft)"
            >
              <option value="comfortable">Comfortable</option>
              <option value="standard">Standard</option>
              <option value="dense">Dense</option>
            </select>
            <div class="panelMuted">
              Apply a consistent row, chip, and spacing density across the shell instead of relying on per-panel compaction.
            </div>
          </div>
          <div class="rowSingle">
            <label for="vueSettingsDesktopNotifications">Desktop notifications</label>
            <select
              id="vueSettingsDesktopNotifications"
              :value="desktopNotificationModeDraft"
              @change="
                desktopNotificationModeDraft = String(($event.target as HTMLSelectElement).value || 'off');
                setSelectValue('settingsDesktopNotifications', desktopNotificationModeDraft)
              "
            >
              <option value="off">Off</option>
              <option value="failures">Failures and cancellations</option>
              <option value="important">Important completions and failures</option>
            </select>
            <div class="panelMuted">
              OS notifications complement the in-app task center. They stay focused on high-signal outcomes and only route when the desktop window is not visible, so background work stays visible without routine spam.
            </div>
            <div class="actions">
              <button type="button" :disabled="!canRequestDesktopNotifications" @click="requestDesktopNotifications">
                Request OS notification permission
              </button>
            </div>
            <div class="panelMuted">Permission: {{ desktopNotificationPermissionLabel }}</div>
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
              <div class="panelEyebrow">Workstation</div>
              <h3 class="panelPaneTitle">Saved perspectives</h3>
              <div class="panelSectionLead">
                Save named shell layouts so the app restores a working posture instead of one generic remembered state.
              </div>
            </div>
          </div>
          <div class="panelSummaryChips" aria-label="Selected perspective summary">
            <span v-for="chip in perspectiveSummaryChips" :key="`settings_perspective_${chip}`" class="panelSummaryChip mono">
              {{ chip }}
            </span>
          </div>
          <div class="rowSingle">
            <label for="vueSettingsPerspectiveSearch">Search saved perspectives</label>
            <input
              id="vueSettingsPerspectiveSearch"
              type="text"
              :value="perspectiveSearchDraft"
              placeholder="name, workflow, density…"
              @input="perspectiveSearchDraft = String(($event.target as HTMLInputElement).value || '')"
            />
            <div class="panelMuted">
              {{ perspectiveSearchSummary }}
            </div>
          </div>
          <div class="rowTwo">
            <div>
              <label for="vueSettingsPerspectiveSelect">Perspective</label>
              <select
                id="vueSettingsPerspectiveSelect"
                :value="selectedPerspectiveId"
                @change="settingsStore.selectPerspective(String(($event.target as HTMLSelectElement).value || ''))"
              >
                <option v-for="entry in visiblePerspectiveOptions" :key="entry.id" :value="entry.id">
                  {{ entry.name }}{{ entry.builtIn ? ' (built-in)' : '' }}
                </option>
              </select>
            </div>
            <div>
              <label for="vueSettingsPerspectiveName">Name</label>
              <input
                id="vueSettingsPerspectiveName"
                type="text"
                :value="perspectiveNameDraft"
                placeholder="Perspective name"
                @input="perspectiveNameDraft = String(($event.target as HTMLInputElement).value || '')"
              />
            </div>
          </div>
          <div class="actions">
            <button type="button" @click="applySelectedPerspective">Apply</button>
            <button type="button" @click="overwriteSelectedPerspective">Save current to selected</button>
            <button type="button" @click="createPerspectiveFromDraft">Save as new</button>
          </div>
          <div class="actions">
            <button type="button" :disabled="!canRenamePerspective" @click="renameSelectedPerspective">Rename</button>
            <button type="button" @click="duplicateSelectedPerspective">Duplicate</button>
            <button type="button" class="danger-btn" :disabled="!canDeletePerspective" @click="deleteSelectedPerspective">Delete</button>
          </div>
          <div class="panelMuted">
            {{ selectedPerspectiveLabel }}
          </div>
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
            <button type="button" @click="copyDesktopDiagnostics">
              {{ diagnosticsCopied ? 'Copied diagnostics snapshot' : 'Copy diagnostics snapshot' }}
            </button>
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
                <div class="diagnosticsLabel">Native desktop</div>
                <div class="mono diagnosticsStamp">{{ desktopWindowStateStamp }}</div>
              </header>
              <div class="diagnosticsPrimary">{{ desktopWindowStateDetail }}</div>
              <div class="panelMuted">Notifications: {{ desktopNotificationPermissionLabel }}</div>
              <div class="panelMuted">
                {{ desktopLogConsoleAttached ? 'Desktop log mirror is attached to the browser console in dev.' : 'Desktop log output stays buffered in-app outside dev console mirroring.' }}
              </div>
            </section>

            <section class="diagnosticsCard">
              <header class="diagnosticsHead">
                <div class="diagnosticsLabel">Task posture</div>
                <div class="mono diagnosticsStamp">{{ centerStatusLabel }}</div>
              </header>
              <div v-if="diagnosticsTaskChips.length > 0" class="diagnosticsEntryMeta">
                <span
                  v-for="chip in diagnosticsTaskChips"
                  :key="`diagnostics_task_${chip}`"
                  class="mono diagnosticsBadge"
                >
                  {{ chip }}
                </span>
              </div>
              <div v-if="latestTask" class="diagnosticsPrimary">
                {{ latestTaskLabel }}
              </div>
              <div v-if="latestTask" class="diagnosticsEntryMeta">
                <span class="mono diagnosticsBadge" :class="latestTaskTone">
                  <component :is="diagnosticsStatusIcon(latestTask?.status)" class="diagnosticsBadgeIcon" aria-hidden="true" />
                  <span>{{ latestTaskStatusLabel }}</span>
                </span>
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
              <div class="panelMuted">
                {{ taskAttentionLine }}
              </div>
              <div class="panelMuted">
                {{ taskCompletedLine }}
              </div>
            </section>

            <section class="diagnosticsCard">
              <header class="diagnosticsHead">
                <div class="diagnosticsLabel">Desktop log bridge</div>
                <div class="mono diagnosticsStamp">{{ desktopLogConsoleAttached ? 'Console attached' : 'Buffered only' }}</div>
              </header>
              <div class="diagnosticsPrimary">
                {{ recentDesktopLogs.length > 0 ? recentDesktopLogs[0]?.message : 'No desktop log entries yet.' }}
              </div>
              <div class="panelMuted">
                Runtime log targets stream into the app diagnostics surface, and the browser console mirrors them only in dev.
              </div>
            </section>

            <section class="diagnosticsCard diagnosticsCardWide">
              <header class="diagnosticsHead">
                <div class="diagnosticsLabel">Desktop logs</div>
                <div class="mono diagnosticsStamp">{{ desktopLogStamp }}</div>
              </header>
              <div class="diagnosticsControls">
                <label class="diagnosticsControl" for="settingsDesktopLogFilter">
                  <span class="panelMuted">Filter</span>
                  <select
                    id="settingsDesktopLogFilter"
                    :value="desktopLogFilterDraft"
                    @change="desktopLogFilterDraft = String(($event.target as HTMLSelectElement).value || 'all') as DesktopLogFilter"
                  >
                    <option value="all">All levels</option>
                    <option value="attention">Warnings and errors</option>
                    <option value="errors">Errors only</option>
                  </select>
                </label>
                <button type="button" :disabled="desktopLogs.length === 0" @click="clearDesktopLogs">Clear desktop logs</button>
              </div>
              <div class="panelMuted">
                {{ desktopLogFilterDraft === 'all' ? 'Use the filter to isolate warnings or errors before copying a diagnostics snapshot.' : `Showing ${desktopLogFilterLabel.toLowerCase()}.` }}
              </div>
              <div v-if="recentDesktopLogs.length === 0" class="panelMuted">{{ desktopLogEmptyLabel }}</div>
              <ul v-else class="diagnosticsList">
                <li
                  v-for="(entry, index) in recentDesktopLogs"
                  :key="entry.id"
                  class="diagnosticsEntry"
                  :aria-label="diagnosticsDesktopLogSummary(index)"
                >
                  <div class="diagnosticsEntryMeta">
                    <span class="mono diagnosticsTime">{{ formatUiTimeFromUnixMs(entry.atUnixMs) }}</span>
                    <span class="mono diagnosticsBadge" :class="diagnosticsLevelTone(entry.level)">
                      <component :is="diagnosticsLevelIcon(entry.level)" class="diagnosticsBadgeIcon" aria-hidden="true" />
                      <span>{{ diagnosticsLevelLabel(entry.level) }}</span>
                    </span>
                  </div>
                  <div class="diagnosticsEntryBody">
                    <strong>Desktop runtime</strong>
                    <span>{{ entry.message }}</span>
                  </div>
                </li>
              </ul>
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
                    <span class="mono diagnosticsBadge" :class="diagnosticsLevelTone(entry.level)">
                      <component :is="diagnosticsLevelIcon(entry.level)" class="diagnosticsBadgeIcon" aria-hidden="true" />
                      <span>{{ diagnosticsLevelLabel(entry.level) }}</span>
                    </span>
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
                    <span class="mono diagnosticsBadge" :class="diagnosticsLevelTone(toast.level)">
                      <component :is="diagnosticsLevelIcon(toast.level)" class="diagnosticsBadgeIcon" aria-hidden="true" />
                      <span>{{ diagnosticsLevelLabel(toast.level) }}</span>
                    </span>
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

.settingsHeaderMeta {
  display: grid;
  justify-items: end;
  gap: 0.42rem;
  flex: 0 0 auto;
}

.settingsBrandCard {
  display: inline-flex;
  align-items: center;
  gap: 0.52rem;
  padding: 0.34rem 0.56rem;
  border: 1px solid color-mix(in oklab, var(--border) 84%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-inline) 86%, var(--panel-footer));
  box-shadow: var(--shadow-inline);
  color: var(--muted-strong);
}

.settingsBrandCard:hover {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 48%, var(--border));
  background: color-mix(in oklab, var(--surface-focus) 88%, var(--panel-footer));
}

.settingsBrandLogo {
  filter: drop-shadow(0 3px 10px color-mix(in srgb, var(--accent-soft) 30%, transparent));
}

.settingsBrandCopy {
  display: grid;
  gap: 0.02rem;
}

.settingsBrandLabel {
  font-size: 0.76rem;
  font-weight: 700;
  letter-spacing: 0.02em;
  color: var(--text);
}

.settingsBrandDetail {
  font-size: 0.66rem;
  color: var(--muted);
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

.diagnosticsControls {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: end;
}

.diagnosticsControl {
  display: grid;
  gap: 0.2rem;
  min-width: 0;
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
  display: inline-flex;
  align-items: center;
  gap: 0.28rem;
  border: 1px solid color-mix(in oklab, var(--border) 80%, transparent);
  border-radius: 999px;
  padding: 0.08rem 0.38rem;
  background: color-mix(in oklab, var(--surface-sunken) 76%, var(--panel-footer));
}

.diagnosticsBadgeIcon {
  width: 0.8rem;
  height: 0.8rem;
  flex: 0 0 auto;
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
  .head {
    flex-direction: column;
  }

  .settingsHeaderMeta {
    width: 100%;
    justify-items: start;
  }

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



