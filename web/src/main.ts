import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import './styles/app.css'
import { applyRecommendedOptimizationDefaults } from './services/recommendedOptimizationDefaults'
import { initializeDesktopProductization } from './services/desktopProductization'
import { applySettingsBootstrapFromStorage } from './services/settingsBootstrap'
import { applyWorkflowBootstrapFromStorage } from './services/workflowBootstrap'
import { bindRuntimePinia } from './stores/runtimePinia'
import { useRuntimeStore } from './stores/runtimeStore'
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
  const runtimeStore = useRuntimeStore(pinia)
  const app = createApp(App)
  app.use(pinia)
  runtimeStore.markBooting('Restoring workspace state')
  app.mount(root)

  try {
    runStorageMigrations()
    applySettingsBootstrapFromStorage()
    applyWorkflowBootstrapFromStorage()
    applyRecommendedOptimizationDefaults()

    runtimeStore.setBootStage('Initializing desktop services')
    await initializeDesktopProductization().catch((error) => {
      const message = String(error instanceof Error ? error.message : error ?? 'Unknown desktop startup error')
      runtimeStore.pushDesktopLog('warn', `Desktop startup service failed: ${message}`)
      console.warn(error)
    })

    runtimeStore.setBootStage('Mounting workspace shell')
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

