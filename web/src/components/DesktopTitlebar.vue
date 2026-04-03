<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { Maximize2, Minimize2, Play, Search, Settings2, Square, X } from 'lucide-vue-next'
import { getCurrentWindow } from '@tauri-apps/api/window'
import VectorAlphaLogo from './VectorAlphaLogo.vue'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useCommandPaletteStore } from '../stores/commandPaletteStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { tauriRuntimeAvailable } from '../tauri/invoke'
import {
  WORKFLOW_STEP_SUBTITLES,
  WORKSPACE_SUBTITLES,
  normalizeWorkflowStep,
  normalizeWorkspaceTab,
} from '../utils/core/workflowModel'

const commandActionStore = useCommandActionStore()
const commandPaletteStore = useCommandPaletteStore()
const optimizeStore = useOptimizeStore()
const workflowStore = useWorkflowStore()

const { canCancel, runDisabled } = storeToRefs(optimizeStore)
const { workflowStep, workspace } = storeToRefs(workflowStore)

const desktopRuntimeAvailable = tauriRuntimeAvailable()
const desktopWindow = desktopRuntimeAvailable ? getCurrentWindow() : null
const isFocused = ref(true)
const isMaximized = ref(false)
const unlisteners: Array<() => void> = []

const workspaceLabel = computed(() => {
  const normalizedWorkspace = normalizeWorkspaceTab(workspace.value)
  if (normalizedWorkspace === 'library') return 'Run Library'
  if (normalizedWorkspace === 'aggregate') return 'Aggregate Analysis'
  if (normalizedWorkspace === 'pro') return 'Pro Tools'
  if (normalizedWorkspace === 'settings') return 'Settings'
  return 'Workflow'
})

const workflowStepLabel = computed(() => {
  const normalizedStep = normalizeWorkflowStep(workflowStep.value)
  if (normalizedStep === 'optimize') return 'Strategy Setup'
  if (normalizedStep === 'validate') return 'Validation'
  if (normalizedStep === 'analyze') return 'Summary'
  if (normalizedStep === 'finalize') return 'Finalizer'
  return 'Data Source'
})

const subtitle = computed(() => {
  const normalizedWorkspace = normalizeWorkspaceTab(workspace.value)
  if (normalizedWorkspace && normalizedWorkspace !== 'workflow') {
    return WORKSPACE_SUBTITLES[normalizedWorkspace]
  }
  const normalizedStep = normalizeWorkflowStep(workflowStep.value) || 'data'
  return WORKFLOW_STEP_SUBTITLES[normalizedStep]
})

const actionLabel = computed(() => (canCancel.value ? 'Stop run' : 'Run optimization'))
const actionDisabled = computed(() => !canCancel.value && !!runDisabled.value)

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
  workflowStore.selectWorkspace('settings')
}

function triggerPrimaryAction(): void {
  if (canCancel.value) {
    commandActionStore.cancelOptimization()
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

async function closeWindow(): Promise<void> {
  if (!desktopWindow) return
  await desktopWindow.close().catch(() => undefined)
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
  unlisteners.push(resizeOff, focusOff)
})

onBeforeUnmount(() => {
  for (const off of unlisteners.splice(0)) off()
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
        <div class="titlebarWorkspace">{{ workspaceLabel }}<span v-if="workspaceLabel === 'Workflow'"> · {{ workflowStepLabel }}</span></div>
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
        :class="{ titlebarDanger: canCancel }"
        :disabled="actionDisabled"
        :aria-label="actionLabel"
        :title="actionLabel"
        @click="triggerPrimaryAction"
      >
        <Square v-if="canCancel" :size="15" aria-hidden="true" />
        <Play v-else :size="15" aria-hidden="true" />
        <span class="titlebarToolLabel">{{ canCancel ? 'Stop' : 'Run' }}</span>
      </button>

      <div class="windowControls" :class="{ unavailable: !desktopRuntimeAvailable }" aria-label="Window controls">
        <button
          type="button"
          class="windowControl"
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
          :disabled="!desktopRuntimeAvailable"
          aria-label="Close window"
          title="Close"
          @click="closeWindow"
        >
          <X :size="15" aria-hidden="true" />
        </button>
      </div>
    </div>
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
  gap: 0.5rem;
  padding: 0.45rem 0.7rem 0.4rem;
  border-bottom: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  background: color-mix(in oklab, var(--surface-sticky) 94%, var(--header-bg));
  backdrop-filter: blur(12px);
  box-shadow: 0 10px 28px color-mix(in srgb, var(--accent-soft) 16%, transparent);
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
  gap: 0.08rem;
  min-width: 260px;
}

.titlebarBrandRow {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.titlebarLogoMark {
  filter: drop-shadow(0 2px 6px color-mix(in srgb, var(--accent-soft) 28%, transparent));
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
  font-size: 0.7rem;
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
  gap: 0.42rem;
}

.titlebarTool,
.windowControl {
  min-height: 34px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.42rem;
  border-radius: 10px;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  background: color-mix(in oklab, var(--surface-working) 94%, var(--panel));
  box-shadow: var(--shadow-inline);
}

.titlebarTool {
  padding: 0.42rem 0.62rem;
  font-size: 0.74rem;
  white-space: nowrap;
}

.titlebarPrimary {
  border-color: color-mix(in oklab, var(--accent) 58%, var(--border));
  background: color-mix(in oklab, var(--selection-fill) 86%, var(--surface-working));
  color: var(--selection-text);
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
  gap: 0.28rem;
  margin-left: 0.2rem;
}

.windowControls.unavailable {
  opacity: 0.68;
}

.windowControl {
  width: 34px;
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
    height: 48px;
    gap: 0.35rem;
  }

  .titlebarSubtitle,
  .titlebarToolLabel {
    display: none;
  }

  .titlebarIdentity {
    min-width: 0;
  }

  .titlebarTool {
    width: 34px;
    padding: 0;
  }
}

.mono {
  font-family: var(--mono);
}
</style>
