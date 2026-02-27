<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue'
import { storeToRefs } from 'pinia'
import { useAnalysisStore } from '../stores/analysisStore'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useCommandPaletteStore } from '../stores/commandPaletteStore'
import { useDataStore } from '../stores/dataStore'
import { useOptimizeStore } from '../stores/optimizeStore'

const analysisStore = useAnalysisStore()
const commandActionStore = useCommandActionStore()
const commandPaletteStore = useCommandPaletteStore()
const dataStore = useDataStore()
const optimizeStore = useOptimizeStore()

const { canExportCsv, canExportJson } = storeToRefs(analysisStore)
const { canOpenLatestMetaReport, canOpenLatestReport } = storeToRefs(commandActionStore)
const { open: commandPaletteOpen } = storeToRefs(commandPaletteStore)
const { canBrowseCsv, canPolygonOpen } = storeToRefs(dataStore)
const { canCancel, runDisabled, validateDisabled } = storeToRefs(optimizeStore)

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
  if (typeof document === 'undefined') return false
  const modal = document.querySelector('#polygonModal') as HTMLElement | null
  if (!(modal instanceof HTMLElement)) return false
  return modal.classList.contains('open')
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

  if (mod && lower === 'o' && !e.shiftKey) {
    e.preventDefault()
    if (!canBrowseCsv.value) return
    commandActionStore.browseCsv()
    return
  }

  if (mod && lower === 'o' && e.shiftKey) {
    e.preventDefault()
    if (!canPolygonOpen.value) return
    commandActionStore.openPolygon()
    return
  }

  if (mod && key === '\\') {
    e.preventDefault()
    commandActionStore.toggleControls()
    return
  }

  if (mod && key === 'Enter') {
    if (isMultilineEditorTarget(document.activeElement as Element | null) && !e.shiftKey) return
    e.preventDefault()
    if (!canRun.value) return
    commandActionStore.runOptimization()
    return
  }

  if (mod && e.shiftKey && lower === 'v') {
    e.preventDefault()
    if (!canValidate.value) return
    commandActionStore.validateOptimization()
    return
  }

  if (mod && e.shiftKey && lower === 'c') {
    e.preventDefault()
    if (!canExportCsv.value) return
    commandActionStore.exportTopCsv()
    return
  }

  if (mod && e.shiftKey && lower === 'j') {
    e.preventDefault()
    if (!canExportJson.value) return
    commandActionStore.exportTopJson()
    return
  }

  if (mod && e.shiftKey && lower === 'r') {
    e.preventDefault()
    if (!canOpenLatestReport.value) return
    commandActionStore.openLatestReport()
    return
  }

  if (mod && e.shiftKey && lower === 'm') {
    e.preventDefault()
    if (!canOpenLatestMetaReport.value) return
    commandActionStore.openLatestMetaReport()
    return
  }

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

onMounted(() => {
  document.addEventListener('keydown', onKeydown, true)
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydown, true)
})
</script>

<template></template>

