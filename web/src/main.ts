import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import './styles/app.css'
import { applyRecommendedOptimizationDefaults } from './services/recommendedOptimizationDefaults'
import { initializeDesktopProductization } from './services/desktopProductization'
import { applySettingsBootstrapFromStorage } from './services/settingsBootstrap'
import { applyWorkflowBootstrapFromStorage } from './services/workflowBootstrap'
import { bindRuntimePinia } from './stores/runtimePinia'
import { BOOT_STAGES, useRuntimeStore } from './stores/runtimeStore'
import { installTauriInstrumentation } from './tauri/instrumentation'
import { runStorageMigrations } from './utils/storage'

function waitForNextPaint(): Promise<void> {
  return new Promise((resolve) => {
    if (typeof requestAnimationFrame !== 'function') {
      setTimeout(() => resolve(), 0)
      return
    }
    requestAnimationFrame(() => resolve())
  })
}

export async function startVectorGridApp(root: HTMLElement): Promise<void> {
  const pinia = createPinia()
  bindRuntimePinia(pinia)
  installTauriInstrumentation()
  const runtimeStore = useRuntimeStore(pinia)
  const app = createApp(App)
  app.use(pinia)
  runtimeStore.markBooting(BOOT_STAGES.RESTORING_WORKSPACE_STATE)
  app.mount(root)

  try {
    await waitForNextPaint()

    runStorageMigrations()
    applySettingsBootstrapFromStorage()
    const startupDataPromise = applyWorkflowBootstrapFromStorage({ includeDeferredData: false })
    const desktopInitializationPromise = initializeDesktopProductization().catch((error) => {
      const message = String(error instanceof Error ? error.message : error ?? 'Unknown desktop startup error')
      runtimeStore.pushDesktopLog('warn', `Desktop startup service failed: ${message}`)
      console.warn(error)
    })
    applyRecommendedOptimizationDefaults()

    await startupDataPromise
    runtimeStore.setBootStage(BOOT_STAGES.INITIALIZING_DESKTOP_SERVICES)
    await desktopInitializationPromise

    runtimeStore.setBootStage(BOOT_STAGES.MOUNTING_WORKSPACE_SHELL)
    await waitForNextPaint()
    runtimeStore.markReady()
  } catch (error) {
    runtimeStore.markError(error)
    console.error(error)
  }
}

const root = document.querySelector('#app') as HTMLElement | null
if (root) {
  void startVectorGridApp(root)
}

