<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useEventListener } from '@vueuse/core'
import { useAnalysisStore } from '../stores/analysisStore'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useCommandPaletteStore } from '../stores/commandPaletteStore'
import { useDataStore } from '../stores/dataStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useSettingsStore } from '../stores/settingsStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { openTaskCenter, useTaskCenterVisibility } from '../composables/useTaskCenterVisibility'
import { useUiReadiness } from '../composables/useUiReadiness'
import {
  createSharedShellShortcutCommandEnabledStates,
  createSharedShellShortcutCommands,
  dispatchSharedShellShortcut,
  type SharedShellShortcutContext,
} from '../utils/shellShortcutRegistry'
import { resolveShellResultCommandReadiness } from '../utils/shellCommandReadiness'
import { listenEvent } from '../tauri/events'
import { invokeCommand, tauriRuntimeAvailable } from '../tauri/invoke'

interface NativeShellCommandPayload {
  commandId?: unknown
  command_id?: unknown
  source?: unknown
}

const analysisStore = useAnalysisStore()
const commandActionStore = useCommandActionStore()
const commandPaletteStore = useCommandPaletteStore()
const dataStore = useDataStore()
const optimizeStore = useOptimizeStore()
const settingsStore = useSettingsStore()
const workflowArtifactsStore = useWorkflowArtifactsStore()
const workflowStore = useWorkflowStore()
const taskCenterOpen = useTaskCenterVisibility()

const { canExportCsv, canExportJson } = storeToRefs(analysisStore)
const { canOpenLatestMetaReport, canOpenLatestReport } = storeToRefs(commandActionStore)
const { open: commandPaletteOpen } = storeToRefs(commandPaletteStore)
const { baseCsvPath, canBrowseCsv, canPolygonOpen, csvPath, dataId, dataTimeframe, polygonOpen } = storeToRefs(dataStore)
const { canCancel, runDisabled, validateDisabled } = storeToRefs(optimizeStore)
const { controlsCollapsed } = storeToRefs(settingsStore)
const { currentRun } = storeToRefs(workflowArtifactsStore)
const { mode, workspace } = storeToRefs(workflowStore)
const { datasetReady } = useUiReadiness()

const resultCommandReadiness = computed(() =>
  resolveShellResultCommandReadiness({
    workspace: workspace.value,
    mode: mode.value,
    datasetReady: datasetReady.value,
    activeDataId: dataId.value,
    activeDatasetPath: baseCsvPath.value || csvPath.value,
    activeTimeframe: dataTimeframe.value,
    currentRun: currentRun.value,
    runDisabled: runDisabled.value,
    validateDisabled: validateDisabled.value,
    canExportCsv: canExportCsv.value,
    canExportJson: canExportJson.value,
  }),
)

function isMultilineEditorTarget(el: Element | null): boolean {
  const tag = String((el as HTMLElement | null)?.tagName || '').toLowerCase()
  if (tag === 'textarea') return true
  return !!(el as HTMLElement | null)?.isContentEditable
}

function isInteractiveShortcutTarget(el: Element | null): boolean {
  const target = el as HTMLElement | null
  if (!target) return false
  if (target.isContentEditable) return true
  return !!target.closest(
    'input, textarea, select, button, a[href], [role="button"], [role="textbox"], [role="combobox"], [role="listbox"], [role="menuitem"], [role="option"]',
  )
}

function shortcutEventTarget(e: KeyboardEvent): Element | null {
  const pathTarget = e.composedPath().find((entry): entry is Element => entry instanceof Element)
  if (pathTarget) return pathTarget
  if (e.target instanceof Element) return e.target
  return document.activeElement instanceof Element ? document.activeElement : null
}

function isCommandPaletteOpen(): boolean {
  return commandPaletteOpen.value
}

function closeCommandPalette(): void {
  commandPaletteStore.closePalette()
}

function isPolygonModalOpen(): boolean {
  return polygonOpen.value
}

function sharedShellShortcutContext(): SharedShellShortcutContext {
  return {
    canRun: resultCommandReadiness.value.canRun,
    canCancel: canCancel.value,
    canValidate: resultCommandReadiness.value.canValidate,
    canBrowseCsv: canBrowseCsv.value,
    canPolygonOpen: canPolygonOpen.value,
    canExportCsv: resultCommandReadiness.value.canExportCsv,
    canExportJson: resultCommandReadiness.value.canExportJson,
    canOpenLatestReport: canOpenLatestReport.value,
    canOpenLatestMetaReport: canOpenLatestMetaReport.value,
    controlsPanelVisible: !controlsCollapsed.value,
    disabledReasons: resultCommandReadiness.value.reasons,
    browseCsv: () => {
      commandActionStore.browseCsv()
    },
    openPolygon: () => {
      commandActionStore.openPolygon()
    },
    toggleControls: () => {
      commandActionStore.toggleControls()
    },
    runOptimization: () => {
      commandActionStore.runOptimization()
    },
    cancelOptimization: () => {
      commandActionStore.cancelOptimization()
    },
    validateOptimization: () => {
      commandActionStore.validateOptimization()
    },
    exportTopCsv: () => {
      commandActionStore.exportTopCsv()
    },
    exportTopJson: () => {
      commandActionStore.exportTopJson()
    },
    openLatestReport: () => {
      commandActionStore.openLatestReport()
    },
    openLatestMetaReport: () => {
      commandActionStore.openLatestMetaReport()
    },
    openTaskCenter: () => {
      openTaskCenter(taskCenterOpen, document.activeElement instanceof HTMLElement ? document.activeElement : null)
    },
  }
}

function nativeShellCommandId(payload: NativeShellCommandPayload): string {
  return String(payload.commandId ?? payload.command_id ?? '').trim()
}

function runNativeShellCommand(payload: NativeShellCommandPayload): void {
  const commandId = nativeShellCommandId(payload)
  if (!commandId) return
  if (commandId === 'open_command_palette') {
    commandPaletteStore.openPalette()
    return
  }
  const command = createSharedShellShortcutCommands(sharedShellShortcutContext())
    .find((item) => item.id === commandId)
  if (!command?.enabled) return
  command.run()
}

const nativeMenuCommandStates = computed(() =>
  createSharedShellShortcutCommandEnabledStates(sharedShellShortcutContext()),
)

let lastNativeMenuStateSignature = ''

function nativeMenuStateSignature(
  states: Array<{ commandId: string; enabled: boolean; checked?: boolean }>,
): string {
  return states
    .map((state) => `${state.commandId}:${state.enabled ? '1' : '0'}:${state.checked === true ? '1' : state.checked === false ? '0' : '-'}`)
    .join('|')
}

async function syncNativeMenuCommandStates(
  states: Array<{ commandId: string; enabled: boolean; checked?: boolean }>,
): Promise<void> {
  if (!tauriRuntimeAvailable()) return
  const signature = nativeMenuStateSignature(states)
  if (signature === lastNativeMenuStateSignature) return
  lastNativeMenuStateSignature = signature
  try {
    await invokeCommand('set_shell_menu_command_states', { states })
  } catch {
    // Native menus are a desktop enhancement; shortcut routing remains authoritative.
  }
}

function onKeydown(e: KeyboardEvent): void {
  if (e.defaultPrevented) return
  if (e.isComposing || e.key === 'Process') return
  const key = String(e.key || '')
  const lower = key.toLowerCase()
  const mod = e.ctrlKey || e.metaKey
  const shortcutTarget = shortcutEventTarget(e)

  if (key === 'Escape') {
    if (isCommandPaletteOpen()) {
      e.preventDefault()
      closeCommandPalette()
      return
    }
    if (isPolygonModalOpen()) {
      e.preventDefault()
      dataStore.closePolygon()
      return
    }
    commandActionStore.clearDrillTradeSelection()
    return
  }

  if (isCommandPaletteOpen()) return

  if (mod && lower === 'k') {
    e.preventDefault()
    commandPaletteStore.openPalette()
    return
  }

  if (mod && key === 'Enter') {
    if (isMultilineEditorTarget(shortcutTarget) && !e.shiftKey) return
  }

  if (dispatchSharedShellShortcut(e, sharedShellShortcutContext())) return

  if (!mod) {
    if (isInteractiveShortcutTarget(shortcutTarget)) return
    const isPrev = lower === 'k' || key === 'ArrowUp'
    const isNext = lower === 'j' || key === 'ArrowDown'
    if (isPrev) {
      e.preventDefault()
      commandActionStore.drillPrevTrade()
      return
    }
    if (isNext) {
      e.preventDefault()
      commandActionStore.drillNextTrade()
      return
    }
  }
}

if (typeof document !== 'undefined') {
  useEventListener(document, 'keydown', onKeydown, { capture: true })
}

watch(
  nativeMenuCommandStates,
  (states) => {
    void syncNativeMenuCommandStates(states)
  },
  { immediate: true, deep: true },
)

let nativeShellCommandOff: (() => void) | null = null

onMounted(() => {
  void listenEvent<NativeShellCommandPayload>('shell-command', (payload) => {
    runNativeShellCommand(payload || {})
  })
    .then((off) => {
      nativeShellCommandOff = off
    })
    .catch(() => {
      nativeShellCommandOff = null
    })
})

onBeforeUnmount(() => {
  nativeShellCommandOff?.()
  nativeShellCommandOff = null
})
</script>

<template></template>

