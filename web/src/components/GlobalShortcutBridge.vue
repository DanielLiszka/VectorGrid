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
import {
  createSharedShellShortcutCommandEnabledStates,
  createSharedShellShortcutCommands,
  dispatchSharedShellShortcut,
  type SharedShellShortcutContext,
} from '../utils/shellShortcutRegistry'
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

const { canExportCsv, canExportJson } = storeToRefs(analysisStore)
const { canOpenLatestMetaReport, canOpenLatestReport } = storeToRefs(commandActionStore)
const { open: commandPaletteOpen } = storeToRefs(commandPaletteStore)
const { canBrowseCsv, canPolygonOpen, polygonOpen } = storeToRefs(dataStore)
const { canCancel, runDisabled, validateDisabled } = storeToRefs(optimizeStore)
const { controlsCollapsed } = storeToRefs(settingsStore)

const canRun = computed(() => !runDisabled.value)
const canValidate = computed(() => !validateDisabled.value)

function isMultilineEditorTarget(el: Element | null): boolean {
  const tag = String((el as HTMLElement | null)?.tagName || '').toLowerCase()
  if (tag === 'textarea') return true
  return !!(el as HTMLElement | null)?.isContentEditable
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
    canRun: canRun.value,
    canCancel: canCancel.value,
    canValidate: canValidate.value,
    canBrowseCsv: canBrowseCsv.value,
    canPolygonOpen: canPolygonOpen.value,
    canExportCsv: canExportCsv.value,
    canExportJson: canExportJson.value,
    canOpenLatestReport: canOpenLatestReport.value,
    canOpenLatestMetaReport: canOpenLatestMetaReport.value,
    controlsPanelVisible: !controlsCollapsed.value,
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
  const key = String(e.key || '')
  const lower = key.toLowerCase()
  const mod = e.ctrlKey || e.metaKey

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
    if (canCancel.value) {
      commandActionStore.cancelOptimization()
    }
    return
  }

  if (isCommandPaletteOpen()) return

  if (mod && lower === 'k') {
    e.preventDefault()
    commandPaletteStore.openPalette()
    return
  }

  if (mod && key === 'Enter') {
    if (isMultilineEditorTarget(document.activeElement as Element | null) && !e.shiftKey) return
  }

  if (dispatchSharedShellShortcut(e, sharedShellShortcutContext())) return

  if (!mod) {
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

