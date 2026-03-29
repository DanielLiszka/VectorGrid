import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import './styles/app.css'
import { applyRecommendedOptimizationDefaults } from './services/recommendedOptimizationDefaults'
import { initializeDesktopProductization } from './services/desktopProductization'
import { applySettingsBootstrapFromStorage } from './services/settingsBootstrap'
import { applyWorkflowBootstrapFromStorage } from './services/workflowBootstrap'
import { bindRuntimePinia } from './stores/runtimePinia'
import { runStorageMigrations } from './utils/storage'

runStorageMigrations()

const root = document.querySelector('#app') as HTMLElement | null
if (root) {
  const pinia = createPinia()
  bindRuntimePinia(pinia)
  const app = createApp(App)
  app.use(pinia)
  applySettingsBootstrapFromStorage()
  applyWorkflowBootstrapFromStorage()
  applyRecommendedOptimizationDefaults()
  void initializeDesktopProductization()
  app.mount(root)
}

