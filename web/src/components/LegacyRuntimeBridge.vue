<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted } from 'vue'
import { installTauriInstrumentation } from '../tauri/instrumentation'
import { attachDataContextBridge } from '../legacy/bridges/dataContextBridge'
import { attachAdvancedContextBridge } from '../legacy/bridges/advancedContextBridge'
import {
  attachAnalysisContextBridge,
  attachHistoryContextBridge,
  attachSettingsContextBridge,
} from '../legacy/bridges/analysisHistorySettingsBridge'
import {
  attachDistributedContextBridge,
  attachExecutionContextBridge,
} from '../legacy/bridges/executionDistributedBridge'
import {
  attachOptimizeContextBridge,
  attachValidationContextBridge,
} from '../legacy/bridges/optimizeValidationBridge'
import { attachQuickActionsBridge } from '../legacy/bridges/quickActionsBridge'
import { attachStrategyContextBridge } from '../legacy/bridges/strategyContextBridge'
import { attachWorkflowBridge } from '../legacy/bridges/workflowBridge'
import { applySettingsBootstrapFromStorage } from '../services/settingsBootstrap'
import { applyWorkflowBootstrapFromStorage } from '../services/workflowBootstrap'
import { useRuntimeStore } from '../stores/runtimeStore'

const runtime = useRuntimeStore()
type LegacyRuntimeModule = typeof import('../legacy/app.js')
let detachDataContextBridge: (() => void) | null = null
let detachAdvancedContextBridge: (() => void) | null = null
let detachOptimizeContextBridge: (() => void) | null = null
let detachStrategyContextBridge: (() => void) | null = null
let detachValidationContextBridge: (() => void) | null = null
let detachWorkflowBridge: (() => void) | null = null
let detachAnalysisContextBridge: (() => void) | null = null
let detachHistoryContextBridge: (() => void) | null = null
let detachSettingsContextBridge: (() => void) | null = null
let detachDistributedContextBridge: (() => void) | null = null
let detachExecutionContextBridge: (() => void) | null = null
let detachQuickActionsBridge: (() => void) | null = null

function attachCompatibilityBridges(): void {
  runtime.setBootStage('event subscription: data')
  detachDataContextBridge = attachDataContextBridge()
  runtime.setBootStage('event subscription: advanced/optimize/validation/strategy')
  detachAdvancedContextBridge = attachAdvancedContextBridge()
  detachOptimizeContextBridge = attachOptimizeContextBridge()
  detachStrategyContextBridge = attachStrategyContextBridge()
  detachValidationContextBridge = attachValidationContextBridge()
  runtime.setBootStage('event subscription: workflow')
  detachWorkflowBridge = attachWorkflowBridge()
  runtime.setBootStage('event subscription: analysis/history/settings')
  detachAnalysisContextBridge = attachAnalysisContextBridge()
  detachHistoryContextBridge = attachHistoryContextBridge()
  detachSettingsContextBridge = attachSettingsContextBridge()
  runtime.setBootStage('event subscription: distributed/execution')
  detachDistributedContextBridge = attachDistributedContextBridge()
  detachExecutionContextBridge = attachExecutionContextBridge()
}

function applyBootstrapSync(): void {
  runtime.setBootStage('ui sync: workflow bootstrap')
  applyWorkflowBootstrapFromStorage()
  runtime.setBootStage('ui sync: settings bootstrap')
  applySettingsBootstrapFromStorage()
}

async function bootLegacyOnce(): Promise<void> {
  if (window.__VTA_LEGACY_BOOTED__) return
  window.__VTA_LEGACY_BOOTED__ = true
  runtime.markBooting('runtime init: tauri instrumentation')
  try {
    installTauriInstrumentation()
    runtime.setBootStage('runtime init: compatibility roots')
    await nextTick()
    runtime.setBootStage('runtime init: load legacy module')
    const legacy: LegacyRuntimeModule = await import('../legacy/app.js')
    runtime.setBootStage('runtime init: quick-actions bridge')
    detachQuickActionsBridge = attachQuickActionsBridge()
    runtime.setBootStage('runtime init: bind legacy handlers')
    legacy.initLegacyApp()
    runtime.setBootStage('domain init: startup bootstrap')
    if (typeof legacy.runLegacyPostBootstrapStartup === 'function') {
      legacy.runLegacyPostBootstrapStartup()
    }
    attachCompatibilityBridges()
    applyBootstrapSync()
    runtime.markReady()
  } catch (err) {
    runtime.markError(err)
    console.error('[LegacyRuntimeBridge] boot failed', err)
  }
}

onMounted(() => {
  runtime.setLegacyMounted(true)
  void bootLegacyOnce()
})

onBeforeUnmount(() => {
  if (detachDataContextBridge) {
    detachDataContextBridge()
    detachDataContextBridge = null
  }
  if (detachAdvancedContextBridge) {
    detachAdvancedContextBridge()
    detachAdvancedContextBridge = null
  }
  if (detachOptimizeContextBridge) {
    detachOptimizeContextBridge()
    detachOptimizeContextBridge = null
  }
  if (detachStrategyContextBridge) {
    detachStrategyContextBridge()
    detachStrategyContextBridge = null
  }
  if (detachValidationContextBridge) {
    detachValidationContextBridge()
    detachValidationContextBridge = null
  }
  if (detachWorkflowBridge) {
    detachWorkflowBridge()
    detachWorkflowBridge = null
  }
  if (detachAnalysisContextBridge) {
    detachAnalysisContextBridge()
    detachAnalysisContextBridge = null
  }
  if (detachHistoryContextBridge) {
    detachHistoryContextBridge()
    detachHistoryContextBridge = null
  }
  if (detachSettingsContextBridge) {
    detachSettingsContextBridge()
    detachSettingsContextBridge = null
  }
  if (detachDistributedContextBridge) {
    detachDistributedContextBridge()
    detachDistributedContextBridge = null
  }
  if (detachExecutionContextBridge) {
    detachExecutionContextBridge()
    detachExecutionContextBridge = null
  }
  if (detachQuickActionsBridge) {
    detachQuickActionsBridge()
    detachQuickActionsBridge = null
  }
  runtime.setLegacyMounted(false)
})
</script>

<template></template>
