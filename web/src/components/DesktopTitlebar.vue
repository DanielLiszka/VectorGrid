<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { Maximize2, Minimize2, Play, Search, Settings2, Square, X } from 'lucide-vue-next'
import { type CloseRequestedEvent, getCurrentWindow } from '@tauri-apps/api/window'
import VectorAlphaLogo from './VectorAlphaLogo.vue'
import ConfirmationDialog from './ConfirmationDialog.vue'
import { useGuidedWorkflowReadiness } from '../composables/useGuidedWorkflowReadiness'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useCommandPaletteStore } from '../stores/commandPaletteStore'
import { useDataStore } from '../stores/dataStore'
import { useExecutionStore } from '../stores/executionStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useSettingsStore } from '../stores/settingsStore'
import { type TrackedTask, shouldGuardDesktopCloseForTask, useTaskStore } from '../stores/taskStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { invokeCommand, tauriRuntimeAvailable } from '../tauri/invoke'
import {
  WORKSPACE_SUBTITLES,
  mlSignalPrimaryActionForStep,
  shellModeLabel,
  normalizeWorkflowStep,
  normalizeWorkspaceTab,
  workflowStepLabelForMode,
  workflowStepSubtitleForMode,
} from '../utils/core/workflowModel'
import { mlSignalScoreCacheDisabledReason } from '../utils/workbenchActionModel'

const commandActionStore = useCommandActionStore()
const commandPaletteStore = useCommandPaletteStore()
const dataStore = useDataStore()
const executionStore = useExecutionStore()
const optimizeStore = useOptimizeStore()
const settingsStore = useSettingsStore()
const taskStore = useTaskStore()
const workflowStore = useWorkflowStore()
const guidedReadiness = useGuidedWorkflowReadiness()

const { runnerActive } = storeToRefs(executionStore)
const { canCancel, runDisabled, validateDisabled } = storeToRefs(optimizeStore)
const { uiMode } = storeToRefs(settingsStore)
const { mode, workflowStep, workspace } = storeToRefs(workflowStore)

const desktopRuntimeAvailable = tauriRuntimeAvailable()
const desktopWindow = desktopRuntimeAvailable ? getCurrentWindow() : null
const isFocused = ref(true)
const isMaximized = ref(false)
const closeConfirmOpen = ref(false)
const unlisteners: Array<() => void> = []
const guidedModeActive = computed(() => uiMode.value === 'guided')
const activeGuidedStep = computed(() => (
  settingsStore.guidedStep === 'trade' ? 'validate' : settingsStore.guidedStep
))
const guidedOptimizationActionDisabled = computed(() =>
  Boolean(runDisabled.value || !guidedReadiness.datasetReady.value || !guidedReadiness.simpleOptimizationSupported.value),
)
const guidedValidationActionDisabled = computed(() =>
  Boolean(validateDisabled.value || !guidedReadiness.optimizationReady.value),
)

const workspaceLabel = computed(() => {
  if (guidedModeActive.value) return 'Simple'
  const normalizedWorkspace = normalizeWorkspaceTab(workspace.value)
  if (normalizedWorkspace === 'library') return 'Library'
  if (normalizedWorkspace === 'aggregate') return 'Aggregate'
  if (normalizedWorkspace === 'portfolio') return 'Portfolio'
  if (normalizedWorkspace === 'pro') return 'Diagnostics'
  if (normalizedWorkspace === 'live') return 'Live'
  if (normalizedWorkspace === 'settings') return 'Settings'
  return 'Research'
})
const showWorkflowStepLabel = computed(() => guidedModeActive.value || normalizeWorkspaceTab(workspace.value) === 'workflow')

const workflowStepLabel = computed(() => {
  if (guidedModeActive.value) {
    const step = activeGuidedStep.value
    if (step === 'strategy') return 'Strategy'
    if (step === 'optimize') return 'Optimize'
    if (step === 'validate') return 'Validate'
    return 'Data'
  }
  const normalizedStep = normalizeWorkflowStep(workflowStep.value) || 'data'
  return workflowStepLabelForMode(normalizedStep, mode.value)
})

const subtitle = computed(() => {
  if (guidedModeActive.value) {
    return 'Simple path for backtest optimization and optional validation.'
  }
  const normalizedWorkspace = normalizeWorkspaceTab(workspace.value)
  if (normalizedWorkspace && normalizedWorkspace !== 'workflow') {
    return WORKSPACE_SUBTITLES[normalizedWorkspace]
  }
  const normalizedStep = normalizeWorkflowStep(workflowStep.value) || 'data'
  return workflowStepSubtitleForMode(normalizedStep, mode.value)
})

const mlSignalPrimaryAction = computed(() =>
  mlSignalPrimaryActionForStep(workflowStep.value),
)
const validateStepActive = computed(() => normalizeWorkflowStep(workflowStep.value) === 'validate')
const mlSignalPrimaryActionDisabled = computed(() => {
  if (canCancel.value) return false
  const dataLoaded = String(dataStore.dataId || '').trim().length > 0
  switch (mlSignalPrimaryAction.value.kind) {
    case 'dataset_export':
      return !dataLoaded
        || !String(optimizeStore.mlSignalDatasetExportPath || '').trim()
        || !!runDisabled.value
    case 'score_cache':
      return !dataLoaded
        || !!mlSignalScoreCacheDisabledReason({
          mlSignalModelBundleId: optimizeStore.mlSignalModelBundleId,
          mlSignalScoreCachePath: optimizeStore.mlSignalScoreCachePath,
          mlSignalScoreInputPath: optimizeStore.mlSignalScoreInputPath,
          mlSignalScoreSource: optimizeStore.mlSignalScoreSource,
        })
        || !!runDisabled.value
    case 'validation':
      return !dataLoaded
        || !String(optimizeStore.mlSignalScoreCachePath || '').trim()
        || !!validateDisabled.value
    case 'policy_sweep':
      return !dataLoaded
        || !String(optimizeStore.mlSignalScoreCachePath || '').trim()
        || !!validateDisabled.value
    case 'artifacts':
      return true
  }
})
const actionLabel = computed(() => {
  if (canCancel.value) return 'Stop run'
  if (guidedModeActive.value) {
    if (activeGuidedStep.value === 'data') return 'Choose data'
    if (activeGuidedStep.value === 'strategy') return 'Pick strategy'
    if (activeGuidedStep.value === 'validate') return 'Run validation'
    return 'Run optimization'
  }
  if (shellModeLabel(mode.value) === 'ML Signal') {
    return mlSignalPrimaryAction.value.label
  }
  if (validateStepActive.value) return 'Run validation'
  return 'Run optimization'
})
const actionShortLabel = computed(() => {
  if (canCancel.value) return 'Stop'
  if (guidedModeActive.value) {
    if (activeGuidedStep.value === 'data') return 'Data'
    if (activeGuidedStep.value === 'strategy') return 'Strategy'
    if (activeGuidedStep.value === 'validate') return 'Validate'
    return 'Run'
  }
  if (shellModeLabel(mode.value) === 'ML Signal') {
    return mlSignalPrimaryAction.value.shortLabel
  }
  if (validateStepActive.value) return 'Validate'
  return 'Run'
})
const actionDisabled = computed(() => {
  if (canCancel.value) return false
  if (guidedModeActive.value) {
    if (activeGuidedStep.value === 'data') return true
    if (activeGuidedStep.value === 'strategy') return true
    if (activeGuidedStep.value === 'validate') return guidedValidationActionDisabled.value
    return guidedOptimizationActionDisabled.value
  }
  if (shellModeLabel(mode.value) === 'ML Signal') {
    return mlSignalPrimaryActionDisabled.value
  }
  if (validateStepActive.value) return !!validateDisabled.value
  return !!runDisabled.value
})
const actionTitle = computed(() => {
  if (!actionDisabled.value) return actionLabel.value
  if (guidedModeActive.value) {
    if (activeGuidedStep.value === 'data') return 'Use the Simple data panel to load CSV, sample, or Polygon candles.'
    if (activeGuidedStep.value === 'strategy') return 'Use the Simple strategy panel to choose a supported strategy.'
    if (activeGuidedStep.value === 'validate' && !guidedReadiness.optimizationReady.value) return 'Run optimization before validation.'
    if (activeGuidedStep.value === 'optimize' && !guidedReadiness.datasetReady.value) return 'Load data before optimization.'
    if (activeGuidedStep.value === 'optimize' && !guidedReadiness.simpleOptimizationSupported.value) return 'Choose Double MA or Composite before optimizing.'
  }
  return actionLabel.value
})
const guardedActiveTasks = computed<TrackedTask[]>(() => {
  const tasks: TrackedTask[] = []
  for (const id of taskStore.activeOrder) {
    const task = taskStore.activeById[id] ?? null
    if (!task || !shouldGuardDesktopCloseForTask(task)) continue
    tasks.push(task)
  }
  return tasks
})
const closeGuardTaskSummary = computed(() => {
  const labels = Array.from(
    new Set(guardedActiveTasks.value.map((task) => String(task.label || '').trim()).filter((value) => value.length > 0)),
  )
  if (labels.length === 0) return ''
  if (labels.length === 1) return `${labels[0]} is still active.`
  return `${labels[0]} and ${labels.length - 1} other long-running tasks are still active.`
})
const closeGuardReasons = computed(() => {
  const reasons: string[] = []
  if (closeGuardTaskSummary.value) reasons.push(closeGuardTaskSummary.value)
  if (runnerActive.value) reasons.push('The live runner is active.')
  return reasons
})
const shouldGuardClose = computed(() => closeGuardReasons.value.length > 0)
const closeGuardBody = computed(() => {
  if (!closeGuardReasons.value.length) return 'VectorGrid will exit immediately.'
  return `${closeGuardReasons.value.join('\n')}\n\nClose exits VectorGrid instead of minimizing it.\nChoose Stay in app to keep the current session alive.`
})

async function syncWindowState(): Promise<void> {
  if (!desktopWindow) return
  try {
    isMaximized.value = !!(await desktopWindow.isMaximized())
  } catch {
    isMaximized.value = false
  }
  try {
    isFocused.value = !!(await desktopWindow.isFocused())
  } catch {
    isFocused.value = true
  }
}

function openPalette(): void {
  commandPaletteStore.openPalette()
}

function openSettings(): void {
  if (guidedModeActive.value) settingsStore.setUiMode('advanced')
  workflowStore.selectWorkspace('settings')
}

function triggerPrimaryAction(): void {
  if (canCancel.value) {
    commandActionStore.cancelOptimization()
    return
  }
  if (guidedModeActive.value) {
    if (activeGuidedStep.value === 'validate' && !validateDisabled.value) {
      if (guidedValidationActionDisabled.value) return
      commandActionStore.validateOptimization()
      return
    }
    if (activeGuidedStep.value === 'optimize' && !guidedOptimizationActionDisabled.value) {
      commandActionStore.runOptimization()
    }
    return
  }
  if (shellModeLabel(mode.value) === 'ML Signal') {
    if (!actionDisabled.value) commandActionStore.runOptimization()
    return
  }
  if (validateStepActive.value && !validateDisabled.value) {
    commandActionStore.validateOptimization()
    return
  }
  if (!runDisabled.value) commandActionStore.runOptimization()
}

async function minimizeWindow(): Promise<void> {
  if (!desktopWindow) return
  await desktopWindow.minimize().catch(() => undefined)
}

async function toggleMaximizeWindow(): Promise<void> {
  if (!desktopWindow) return
  await desktopWindow.toggleMaximize().catch(() => undefined)
  await syncWindowState()
}

async function requestClose(): Promise<void> {
  if (!desktopRuntimeAvailable) return
  await invokeCommand('exit_application').catch(async () => {
    if (!desktopWindow) return
    await desktopWindow.destroy().catch(() => desktopWindow.close().catch(() => undefined))
  })
}

async function closeWindow(): Promise<void> {
  if (!desktopWindow) return
  if (shouldGuardClose.value) {
    closeConfirmOpen.value = true
    return
  }
  await requestClose()
}

async function confirmCloseWindow(): Promise<void> {
  closeConfirmOpen.value = false
  await requestClose()
}

function cancelCloseWindow(): void {
  closeConfirmOpen.value = false
}

async function onWindowCloseRequested(event: CloseRequestedEvent): Promise<void> {
  if (!shouldGuardClose.value) return
  event.preventDefault()
  closeConfirmOpen.value = true
}

async function startWindowDrag(event?: MouseEvent): Promise<void> {
  if (!desktopWindow || event?.button !== 0) return
  await desktopWindow.startDragging().catch(() => undefined)
}

async function handleTitlebarDoubleClick(): Promise<void> {
  if (!desktopWindow) return
  await toggleMaximizeWindow()
}

onMounted(async () => {
  document.documentElement.dataset.desktopChrome = '1'
  await syncWindowState()
  if (!desktopWindow) return
  const resizeOff = await desktopWindow.onResized(() => {
    void syncWindowState()
  })
  const focusOff = await desktopWindow.onFocusChanged(({ payload }) => {
    isFocused.value = !!payload
  })
  const closeOff = await desktopWindow.onCloseRequested(onWindowCloseRequested)
  unlisteners.push(resizeOff, focusOff, closeOff)
})

onBeforeUnmount(() => {
  for (const off of unlisteners.splice(0)) off()
  closeConfirmOpen.value = false
  delete document.documentElement.dataset.desktopChrome
})
</script>

<template>
  <div
    class="desktopTitlebar"
    :class="{ focused: isFocused, maximized: isMaximized }"
    role="region"
    aria-label="Desktop titlebar"
  >
    <div
      class="titlebarIdentity"
      @mousedown.left.prevent="startWindowDrag"
      @dblclick="handleTitlebarDoubleClick"
    >
      <div class="titlebarBrandRow">
        <VectorAlphaLogo class="titlebarLogoMark" decorative :size="20" />
        <div class="titlebarBrand">VectorGrid</div>
        <div class="titlebarWorkspace">{{ workspaceLabel }}<span v-if="showWorkflowStepLabel"> · {{ workflowStepLabel }}</span></div>
      </div>
      <div class="titlebarSubtitle">{{ subtitle }}</div>
    </div>

    <div
      class="titlebarDragFill"
      aria-hidden="true"
      @mousedown.left.prevent="startWindowDrag"
      @dblclick="handleTitlebarDoubleClick"
    ></div>

    <div class="titlebarActions">
      <button
        type="button"
        class="titlebarTool"
        data-qa-action="shell.commandPalette.open"
        aria-label="Open command palette"
        title="Command palette (Ctrl+K)"
        aria-keyshortcuts="Control+K"
        @click="openPalette"
      >
        <Search :size="15" aria-hidden="true" />
        <span class="titlebarToolLabel">Search</span>
      </button>
      <button
        type="button"
        class="titlebarTool"
        data-qa-action="shell.settings.open"
        aria-label="Open settings workspace"
        title="Settings"
        @click="openSettings"
      >
        <Settings2 :size="15" aria-hidden="true" />
        <span class="titlebarToolLabel">Settings</span>
      </button>
      <button
        type="button"
        class="titlebarTool titlebarPrimary"
        data-qa-action="shell.primaryAction.run"
        :class="{ titlebarDanger: canCancel }"
        :disabled="actionDisabled"
        :aria-label="actionLabel"
        :title="actionTitle"
        @click="triggerPrimaryAction"
      >
        <Square v-if="canCancel" :size="15" aria-hidden="true" />
        <Play v-else :size="15" aria-hidden="true" />
        <span class="titlebarToolLabel">{{ actionShortLabel }}</span>
      </button>

      <div class="windowControls" :class="{ unavailable: !desktopRuntimeAvailable }" aria-label="Window controls">
        <button
          type="button"
          class="windowControl"
          data-qa-action="shell.window.minimize"
          :disabled="!desktopRuntimeAvailable"
          aria-label="Minimize window"
          title="Minimize"
          @click="minimizeWindow"
        >
          <Minimize2 :size="15" aria-hidden="true" />
        </button>
        <button
          type="button"
          class="windowControl"
          data-qa-action="shell.window.toggleMaximize"
          :disabled="!desktopRuntimeAvailable"
          :aria-label="isMaximized ? 'Restore window' : 'Maximize window'"
          :title="isMaximized ? 'Restore' : 'Maximize'"
          @click="toggleMaximizeWindow"
        >
          <Square v-if="isMaximized" :size="13" aria-hidden="true" />
          <Maximize2 v-else :size="15" aria-hidden="true" />
        </button>
        <button
          type="button"
          class="windowControl windowControlClose"
          data-qa-action="shell.window.close"
          :disabled="!desktopRuntimeAvailable"
          aria-label="Close window"
          title="Close"
          @click="closeWindow"
        >
          <X :size="15" aria-hidden="true" />
        </button>
      </div>
    </div>
    <ConfirmationDialog
      id-base="desktop_close_confirm"
      :open="closeConfirmOpen"
      title="Exit VectorGrid now?"
      :body="closeGuardBody"
      confirm-label="Exit app"
      cancel-label="Stay in app"
      danger
      @cancel="cancelCloseWindow"
      @confirm="confirmCloseWindow"
    />
  </div>
</template>

<style scoped>
.desktopTitlebar {
  position: fixed;
  inset: 0 0 auto 0;
  z-index: 4300;
  height: var(--desktop-titlebar-h, 52px);
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: stretch;
  gap: 0.42rem;
  padding: 0.34rem 0.62rem 0.32rem;
  border-bottom: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  background: color-mix(in oklab, var(--surface-shell) 90%, var(--header-bg));
  backdrop-filter: blur(8px);
  box-shadow: 0 8px 18px color-mix(in srgb, rgb(var(--shadow-overflow-rgb) / 0.16) 72%, transparent);
}

.desktopTitlebar.focused {
  border-bottom-color: color-mix(in oklab, var(--panel-focus-edge) 44%, var(--border));
}

.desktopTitlebar.maximized {
  border-radius: 0;
}

.titlebarIdentity,
.titlebarDragFill {
  min-width: 0;
  display: flex;
  align-items: center;
  user-select: none;
  cursor: grab;
}

.titlebarIdentity {
  flex-direction: column;
  align-items: flex-start;
  justify-content: center;
  gap: 0.04rem;
  min-width: 260px;
}

.titlebarBrandRow {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.titlebarLogoMark {
  filter: drop-shadow(0 1px 4px color-mix(in srgb, var(--accent-soft) 18%, transparent));
}

.titlebarBrand {
  font-size: 0.88rem;
  font-weight: 700;
  letter-spacing: 0.01em;
}

.titlebarWorkspace {
  font-size: 0.74rem;
  color: var(--muted-strong);
}

.titlebarSubtitle {
  max-width: 56ch;
  font-size: 0.68rem;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.titlebarDragFill {
  width: 100%;
}

.titlebarActions {
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.34rem;
}

.titlebarTool,
.windowControl {
  min-height: 32px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.42rem;
  border-radius: 10px;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  background: color-mix(in oklab, var(--surface-inline) 88%, var(--panel));
  box-shadow: none;
}

.titlebarTool {
  padding: 0.34rem 0.56rem;
  font-size: 0.72rem;
  white-space: nowrap;
}

.titlebarTool:hover:not(:disabled),
.windowControl:hover:not(:disabled) {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 34%, var(--border));
  background: color-mix(in oklab, var(--surface-working) 92%, var(--surface-inline));
}

.titlebarTool:focus-visible,
.windowControl:focus-visible {
  outline: none;
  border-color: var(--focus-border);
  box-shadow: var(--focus-ring);
}

.titlebarPrimary {
  border-color: color-mix(in oklab, var(--accent) 58%, var(--border));
  background: color-mix(in oklab, var(--selection-fill) 86%, var(--surface-working));
  color: var(--selection-text);
  box-shadow: inset 0 -2px 0 var(--selection-rail);
}

.titlebarPrimary:hover:not(:disabled) {
  background: color-mix(in oklab, var(--selection-fill-strong) 88%, var(--surface-working));
}

.titlebarDanger {
  border-color: color-mix(in oklab, var(--danger) 48%, var(--border));
  background: color-mix(in oklab, var(--surface-danger) 92%, var(--surface-working));
  color: var(--danger-ink);
}

.windowControls {
  display: inline-flex;
  align-items: center;
  gap: 0.22rem;
  margin-left: 0.16rem;
}

.windowControls.unavailable {
  opacity: 0.68;
}

.windowControl {
  width: 32px;
  padding: 0;
}

.windowControlClose:hover:not(:disabled) {
  border-color: color-mix(in oklab, var(--danger) 44%, var(--border));
  background: color-mix(in oklab, var(--surface-danger) 92%, var(--surface-working));
  color: var(--danger-ink);
}

.titlebarToolLabel {
  display: inline;
}

@media (max-width: 980px) {
  .desktopTitlebar {
    grid-template-columns: minmax(0, 1fr) auto;
    padding-inline: 0.55rem;
  }

  .titlebarIdentity {
    min-width: 0;
  }

  .titlebarDragFill {
    display: none;
  }
}

@media (max-width: 760px) {
  .desktopTitlebar {
    height: 44px;
    gap: 0.26rem;
    padding-inline: 0.42rem;
  }

  .titlebarSubtitle,
  .titlebarWorkspace,
  .titlebarToolLabel {
    display: none;
  }

  .titlebarIdentity {
    min-width: 0;
  }

  .titlebarTool {
    width: 30px;
    padding: 0;
  }

  .windowControl {
    width: 30px;
    min-height: 30px;
  }
}

.mono {
  font-family: var(--mono);
}
</style>
